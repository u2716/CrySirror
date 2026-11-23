// SPDX-License-Identifier: MPL-2.0
//! `Manager.Sound` / `AL.Utils.Sound` over the host's audio channels:
//! `AL.ContentProvider.DicBGM`/`DicSystemSE` from the `sd/lis` tables,
//! `Sound.Loader` (bundle, asset) resolution to AudioClips, FSB5 banks to
//! shared decoder streams. Vorbis setup headers come from the installed
//! `UnityPlayer.dll` (FMOD's table), read once on first use.
use crate::{tables::Table, unity::Unity};
use anyhow::{Context, Result};
use crysirror_audio::fsb5;
use crysirror_core::{
    service::{AudioParameters, AudioPreparePhase, AudioRequest, EncodedAudio, HostServices, PcmBus},
    state::ClientId,
};
use std::collections::HashMap;

/// Personalities with a `Voice/cNN` config volume.
const PERSONALITIES: i64 = 12;

/// A sound request of scene logic, executed by the adapter.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// `PlayBGM(PropertyData._bgmID.*)`.
    Bgm(i32),
    /// `Sound.Play(PropertyData._sseID.*)`.
    SystemSe(i32),
    /// A random AudioClip of the bundles under `lib/<prefix>` as system SE
    /// (`LogoScene` brand call).
    RandomClip(String),
    StopAll,
}

/// `Manager.Sound.Type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bgm,
    Env,
    SystemSe,
    GameSe2d,
    GameSe3d,
    /// ADV/character voices (`Manager.Voice`).
    Voice,
}

impl Kind {
    fn channel(self) -> u32 {
        match self {
            Kind::Bgm => 0,
            Kind::Env => 1,
            Kind::SystemSe => 2,
            Kind::GameSe2d => 3,
            Kind::GameSe3d => 4,
            Kind::Voice => 5,
        }
    }

    /// The config mix group (`Manager.Sound` mixer groups).
    fn group(self) -> &'static str {
        match self {
            Kind::Bgm => "bgm",
            Kind::Env => "env",
            Kind::SystemSe => "system_se",
            Kind::GameSe2d | Kind::GameSe3d => "game_se",
            Kind::Voice => "voice",
        }
    }
}

#[derive(Default)]
pub struct Sound {
    bgm: HashMap<i32, (String, String)>,
    system_se: HashMap<i32, (String, String)>,
    setups: HashMap<u32, Option<Vec<u8>>>,
    player: Option<Vec<u8>>,
    current_bgm: Option<(String, String)>,
    pending: Vec<(u32, u64)>,
    /// The config mix groups exist (`apply_volumes` succeeded).
    groups: bool,
    pub log: Vec<String>,
}

impl Sound {
    /// The `sd/lis` tables (`bgm[…]`, `sse[…]`).
    pub fn load(unity: &mut Unity) -> Result<Self> {
        let mut s = Sound::default();
        let bundles: Vec<String> = unity.bundle_files().into_iter().filter(|k| k.starts_with("bundle:lib/sd/lis/")).collect();
        for key in bundles {
            let file = unity.file(&key)?;
            let texts: Vec<i64> = file.objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::TEXT_ASSET).map(|o| o.path_id).collect();
            for id in texts {
                let name = unity.decode(&key, id)?.string("m_Name").unwrap_or("").to_owned();
                let table = Table::parse(&unity.text_asset(&key, id)?).with_context(|| format!("sound table {name}"))?;
                let dict = if name.starts_with("bgm[") {
                    &mut s.bgm
                } else if name.starts_with("sse[") {
                    &mut s.system_se
                } else {
                    continue;
                };
                for (id, bundle, asset) in table.assets() {
                    dict.insert(id, (bundle, asset));
                }
            }
        }
        Ok(s)
    }

    fn stream(&mut self, unity: &mut Unity, bundle: &str, asset: &str) -> Result<Vec<u8>> {
        let (key, id) = unity.bundle_asset(&format!("lib/{bundle}"), asset, crysirror_unity::class_id::AUDIO_CLIP)?.with_context(|| format!("AudioClip {asset} not in {bundle}"))?;
        let clip = unity.decode(&key, id)?;
        let bank = unity.audio_clip_bytes(&key, &clip)?;
        let b = fsb5::Bank::parse(&bank)?;
        if let Some(crc) = b.samples.first().and_then(|s| s.vorbis_crc) {
            if !self.setups.contains_key(&crc) {
                if self.player.is_none() {
                    // SATD-amanatsu-port: read on a worker (ADR 0050).
                    let r = unity.reader("UnityPlayer.dll")?;
                    let len = r.len()? as usize;
                    anyhow::ensure!(len <= 256 << 20, "UnityPlayer.dll size");
                    let mut data = vec![0u8; len];
                    let mut at = 0;
                    while at < len {
                        let n = r.read_at(at as u64, &mut data[at..])?;
                        anyhow::ensure!(n > 0, "UnityPlayer.dll truncated");
                        at += n;
                    }
                    self.player = Some(data);
                }
                let found = fsb5::find_vorbis_setup(self.player.as_deref().unwrap_or(&[]), crc);
                self.setups.insert(crc, found);
            }
        }
        let setups = &self.setups;
        fsb5::to_stream(&bank, &|crc| setups.get(&crc).cloned().flatten())
    }

    /// `Manager.Sound.Play(Loader)` of a bundle asset.
    pub fn play(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, kind: Kind, bundle: &str, asset: &str, gain: f32) -> Result<()> {
        self.play_in(host, client, unity, kind, kind.channel(), bundle, asset, gain, kind.group().to_owned())
    }

    /// A character voice in its personality's config volume
    /// (`Voice/cNN`, under the voice master).
    pub fn play_voice(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, bundle: &str, asset: &str, personality: Option<i64>) -> Result<()> {
        self.play_adv_voice(host, client, unity, 0, bundle, asset, personality)
    }

    // Channel 5 remains the talking heroine / normal map voice. The player
    // and additional scenario actors have separate playback and waits.
    fn actor_channel(actor: i32) -> Result<u32> {
        match actor {
            0 => Ok(Kind::Voice.channel()),
            -1 => Ok(6),
            n if n > 0 => Ok(6 + n as u32),
            _ => anyhow::bail!("unknown ADV voice actor {actor}"),
        }
    }

    pub(crate) fn voice_playing(&self, host: &dyn HostServices, client: ClientId, actor: i32) -> Result<bool> {
        let channel = Self::actor_channel(actor)?;
        Ok(self.pending.iter().any(|(c, _)| *c == channel) || host.audio_playing(client, channel)?)
    }

    pub(crate) fn stop_voice(&mut self, host: &mut dyn HostServices, client: ClientId, actor: i32) -> Result<()> {
        let channel = Self::actor_channel(actor)?;
        while let Some(at) = self.pending.iter().position(|(c, _)| *c == channel) {
            host.release_audio(client, self.pending[at].1)?;
            self.pending.remove(at);
        }
        host.submit_audio(client, AudioRequest::Stop { channel, fade_ms: 0. })?;
        Ok(())
    }

    pub(crate) fn play_adv_voice(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, actor: i32, bundle: &str, asset: &str, personality: Option<i64>) -> Result<()> {
        let group = personality.filter(|p| (0..PERSONALITIES).contains(p)).map_or_else(|| Kind::Voice.group().to_owned(), |p| format!("voice_c{p:02}"));
        self.play_in(host, client, unity, Kind::Voice, Self::actor_channel(actor)?, bundle, asset, 1., group)
    }

    fn play_in(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, kind: Kind, channel: u32, bundle: &str, asset: &str, gain: f32, group: String) -> Result<()> {
        let bytes = self.stream(unity, bundle, asset)?;
        let parameters = AudioParameters {
            gain,
            looping: kind == Kind::Bgm,
            bus: match kind {
                Kind::Bgm => PcmBus::Music,
                Kind::Env => PcmBus::Ambient,
                Kind::SystemSe => PcmBus::Ui,
                Kind::Voice => PcmBus::Voice,
                _ => PcmBus::Sfx,
            },
            mix_group: self.groups.then_some(group),
            ..Default::default()
        };
        let ticket = host.begin_audio(client, AudioRequest::Play { channel, source: EncodedAudio { bytes: bytes.into(), frame_limit: None }, intro: None, parameters })?;
        self.pending.push((channel, ticket));
        Ok(())
    }

    /// `AL.Utils.Sound.PlayBGM(bgmID)`; the same track keeps playing.
    pub fn play_bgm(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, id: i32) -> Result<()> {
        let Some(entry) = self.bgm.get(&id).cloned() else {
            self.log.push(format!("BGM {id} not in the tables"));
            return Ok(());
        };
        if self.current_bgm.as_ref() == Some(&entry) {
            return Ok(());
        }
        self.current_bgm = Some(entry.clone());
        // SATD-amanatsu-port: the 0.8 s cross-fade and config volumes.
        self.play(host, client, unity, Kind::Bgm, &entry.0, &entry.1, 1.)
    }

    /// `AL.Utils.Sound.Play(int se)` (`DicSystemSE`).
    pub fn play_system_se(&mut self, host: &mut dyn HostServices, client: ClientId, unity: &mut Unity, id: i32) -> Result<()> {
        let Some((bundle, asset)) = self.system_se.get(&id).cloned() else {
            self.log.push(format!("system SE {id} not in the tables"));
            return Ok(());
        };
        self.play(host, client, unity, Kind::SystemSe, &bundle, &asset, 1.)
    }

    /// The config volumes (`Sound` Master/BGM/ENV/SystemSE/GameSE, the
    /// `Voice` Master and each personality's `cNN`, each
    /// `Volume × Switch`) as mix groups under `master`.
    pub fn apply_volumes(&mut self, host: &mut dyn HostServices, client: ClientId, cfg: &crate::config::Config) -> Result<()> {
        use crysirror_core::service::AudioMixGroupSettings as G;
        let mut groups: Vec<(String, Option<&str>, f32)> = vec![
            ("master".into(), None, cfg.gain("Sound", "Master")),
            ("bgm".into(), Some("master"), cfg.gain("Sound", "BGM")),
            ("env".into(), Some("master"), cfg.gain("Sound", "ENV")),
            ("system_se".into(), Some("master"), cfg.gain("Sound", "SystemSE")),
            ("game_se".into(), Some("master"), cfg.gain("Sound", "GameSE")),
            ("voice".into(), Some("master"), cfg.gain("Voice", "Master")),
        ];
        for p in 0..PERSONALITIES {
            groups.push((format!("voice_c{p:02}"), Some("voice"), cfg.gain("Voice", &format!("c{p:02}"))));
        }
        for (name, parent, gain) in groups {
            let settings = G { parent: parent.map(str::to_owned), gain, muted: false };
            host.submit_audio(client, AudioRequest::SetMixGroup { name, settings, fade_ms: 50. })?;
        }
        self.groups = true;
        Ok(())
    }

    /// An AudioClip's `m_Length` in seconds.
    pub fn clip_length(unity: &mut Unity, bundle: &str, asset: &str) -> Option<f32> {
        let (key, id) = unity.bundle_asset(&format!("lib/{bundle}"), asset, crysirror_unity::class_id::AUDIO_CLIP).ok()??;
        unity.decode(&key, id).ok()?.float("m_Length").map(|v| v as f32)
    }

    /// Every AudioClip name of the bundles under `lib/<prefix>`, sorted.
    pub fn clips_under(unity: &mut Unity, prefix: &str) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        let keys: Vec<String> = unity.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:lib/{prefix}"))).collect();
        for key in keys {
            let file = unity.file(&key)?;
            let bundle = key.trim_start_matches("bundle:lib/").split('#').next().unwrap_or("").to_owned();
            let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::AUDIO_CLIP).map(|o| o.path_id).collect();
            for id in ids {
                let name = unity.decode(&key, id)?.string("m_Name").unwrap_or("").to_owned();
                out.push((bundle.clone(), name));
            }
        }
        out.sort();
        Ok(out)
    }

    pub fn stop_all(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        for k in [Kind::Bgm, Kind::Env, Kind::SystemSe, Kind::GameSe2d, Kind::GameSe3d] {
            host.submit_audio(client, AudioRequest::Stop { channel: k.channel(), fade_ms: 0. })?;
        }
        self.current_bgm = None;
        Ok(())
    }

    /// Release finished preparations.
    pub fn poll(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let mut keep = Vec::new();
        for (channel, t) in std::mem::take(&mut self.pending) {
            match host.audio_preparation(client, t)?.phase {
                AudioPreparePhase::Ready => host.release_audio(client, t)?,
                AudioPreparePhase::Failed => {
                    let p = host.audio_preparation(client, t)?;
                    self.log.push(format!("audio preparation failed: {}", p.error.unwrap_or_default()));
                    host.release_audio(client, t)?;
                }
                AudioPreparePhase::Pending => keep.push((channel, t)),
            }
        }
        self.pending = keep;
        Ok(())
    }
}

#[cfg(test)]
mod adv_voice_tests {
    use super::*;
    use crysirror_core::{path::VfsPath, service::FileSystem, state::Command};

    #[derive(Default)]
    struct AudioHost {
        playing: Vec<u32>,
        released: Vec<u64>,
    }

    impl FileSystem for AudioHost {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> { unreachable!() }
        fn write_atomic(&mut self, _: ClientId, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> { unreachable!() }
    }

    impl HostServices for AudioHost {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> { unreachable!() }
        fn now_ms(&self) -> u64 { 0 }
        fn audio_playing(&self, _: ClientId, channel: u32) -> crysirror_core::Result<bool> {
            Ok(self.playing.contains(&channel))
        }
        fn release_audio(&mut self, _: ClientId, ticket: u64) -> crysirror_core::Result<()> {
            self.released.push(ticket);
            Ok(())
        }
        fn submit_audio(&mut self, _: ClientId, request: AudioRequest) -> crysirror_core::Result<()> {
            let AudioRequest::Stop { channel, .. } = request else { unreachable!() };
            self.playing.retain(|c| *c != channel);
            Ok(())
        }
    }

    #[test]
    fn waits_for_pending_or_published_actor_voice_and_cancels_only_that_actor() {
        let client = ClientId(1);
        let mut host = AudioHost { playing: vec![5, 7], ..Default::default() };
        let mut sound = Sound { pending: vec![(6, 100), (7, 101)], ..Default::default() };
        assert!(sound.voice_playing(&host, client, -1).unwrap());
        assert!(sound.voice_playing(&host, client, 0).unwrap());
        sound.stop_voice(&mut host, client, -1).unwrap();
        assert_eq!(host.released, [100]);
        assert!(!sound.voice_playing(&host, client, -1).unwrap());
        sound.stop_voice(&mut host, client, 0).unwrap();
        assert!(!sound.voice_playing(&host, client, 0).unwrap());
        assert!(sound.voice_playing(&host, client, 1).unwrap());
        assert_eq!(sound.pending, [(7, 101)]);
        assert_eq!(host.playing, [7]);
    }
}
