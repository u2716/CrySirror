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
}

#[derive(Default)]
pub struct Sound {
    bgm: HashMap<i32, (String, String)>,
    system_se: HashMap<i32, (String, String)>,
    setups: HashMap<u32, Option<Vec<u8>>>,
    player: Option<Vec<u8>>,
    current_bgm: Option<(String, String)>,
    pending: Vec<u64>,
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
            ..Default::default()
        };
        let ticket = host.begin_audio(client, AudioRequest::Play { channel: kind.channel(), source: EncodedAudio { bytes: bytes.into(), frame_limit: None }, intro: None, parameters })?;
        self.pending.push(ticket);
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
        for t in std::mem::take(&mut self.pending) {
            match host.audio_preparation(client, t)?.phase {
                AudioPreparePhase::Ready => host.release_audio(client, t)?,
                AudioPreparePhase::Failed => {
                    let p = host.audio_preparation(client, t)?;
                    self.log.push(format!("audio preparation failed: {}", p.error.unwrap_or_default()));
                    host.release_audio(client, t)?;
                }
                AudioPreparePhase::Pending => keep.push(t),
            }
        }
        self.pending = keep;
        Ok(())
    }
}
