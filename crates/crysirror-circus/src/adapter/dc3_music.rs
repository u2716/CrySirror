//!DC3WY432F20/431DC0's sole streamed music channel. The adapter keeps only
//!the deferred replacement and60-native-frame release policy; shared Audio
//!owns seekable decoding, PCM, loop transport, gain math and release actors.
use super::*;
use crate::machine::MusicAsset;
use crysirror_core::service::{AudioAsset, AudioReleaseHandle, AudioReleasePhase, AudioReleaseSpec};

#[derive(Clone)]
struct Request { asset: MusicAsset, looping: bool }
impl Request {
    fn matches(&self, other: &Self) -> bool {
        self.asset.name == other.asset.name && self.looping == other.looping
            && self.asset.loop_start == other.asset.loop_start && self.asset.loop_end == other.asset.loop_end
    }
}
struct Starting {
    request: Request,
    path: Option<Receiver<std::result::Result<VfsPath, String>>>,
    ticket: Option<u64>,
}
#[derive(Default)]
pub(super) struct Music {
    queued: Option<Request>,
    release: Option<(AudioReleaseHandle, u64)>,
    starting: Option<Starting>,
    ///4324F0(0) with this[139] (a replacement queued) sets this[138] instead
    ///of restarting the release; 4330D0 then starts the queued music and
    ///immediately releases it again over 60 frames.
    stop_after: bool,
}
impl Bridge<'_> {
    pub(in crate::adapter) fn play_dc3_music(&mut self, asset: &MusicAsset, looping: bool) -> Result<bool> {
        ensure!(self.services.native_profile == Some(crate::Profile::Dc3WithYou), "DC3 native music profile");
        ensure!(asset.loop_start == 0 && asset.loop_end.is_none(), "DC3 native music whole-source loop");
        if asset.name.is_empty() || !self.services.mixer_controls.context("DC3 music mixer owner")?.enabled[0] {
            return Ok(true); //432F20's original disabled/empty native resource branch.
        }
        let mut state = std::mem::take(&mut self.services.dc3_music);
        if crate::trace_enabled() { eprintln!("DC3 music play {} queued={} release={}", asset.name, state.queued.is_some(), state.release.is_some()); }
        let request = Request {asset: asset.clone(), looping};
        //A new start replacing one that 0x12 already released: the original
        //had started and was fading the old track, so this one just plays.
        if state.queued.is_none() && state.starting.as_ref().is_some_and(|starting| !starting.request.matches(&request)) {
            state.stop_after = false;
        }
        let result = (|| {
            if state.queued.is_some() || state.release.is_some() {
                //432F20 overwrites its native288-byte pending name, retaining
                //the running release. This is queue admission, not playback.
                state.queued = Some(request);
                return Ok(true);
            }
            if state.starting.is_none() && self.host.audio_info(self.client, 0)?.is_some_and(|info| info.source_playing) {
                state.release = Some((self.begin_dc3_music_release()?, 0));
                state.queued = Some(request);
                return Ok(true);
            }
            let started = self.poll_dc3_music_start(&mut state, &request)?;
            if started && std::mem::take(&mut state.stop_after) {
                state.release = Some((self.begin_dc3_music_release()?, 0));
            }
            Ok(started)
        })();
        self.services.dc3_music = state;
        result
    }
    ///431DC0(0): the 60-frame release of the streamed music to -10000.
    fn begin_dc3_music_release(&mut self) -> Result<AudioReleaseHandle> {
        self.cancel_native_duck()?;
        self.host.begin_audio_release(self.client, AudioReleaseSpec {
            source_channel: 0, tail_channel: 0, threshold_db100: -10000,
            target_above_db100: -10000, target_below_db100: -10000,
            steps: 60, pan: None, playback_rate: None,
            freeze_group_gain: true, release_tail_when_source_missing: true,
        }).map_err(Into::into)
    }
    ///0x12 / 4324F0(0), the sole owner of channel 0's release:
    ///* a replacement queued (this[139]): only mark this[138];
    ///* a start still being prepared: 432F20 starts synchronously in the
    ///  original, so the release follows the start;
    ///* a release already running: 431DC0 restarts it from the current
    ///  level toward the same -10000 target; the running actor is kept;
    ///* music playing: 431DC0's 60-frame release.
    pub(in crate::adapter) fn stop_dc3_music(&mut self) -> Result<()> {
        let mut state = std::mem::take(&mut self.services.dc3_music);
        let result = (|| {
            if state.queued.is_some() || state.starting.is_some() {
                state.stop_after = true;
            } else if state.release.is_none()
                && self.host.audio_info(self.client, 0)?.is_some_and(|info| info.source_playing)
            {
                state.release = Some((self.begin_dc3_music_release()?, 0));
            }
            Ok(())
        })();
        self.services.dc3_music = state;
        result
    }
    fn poll_dc3_music_start(&mut self, state: &mut Music, request: &Request) -> Result<bool> {
        if state.starting.as_ref().is_some_and(|starting| !starting.request.matches(request)) {
            if let Some(ticket) = state.starting.take().and_then(|starting| starting.ticket) {
                self.host.release_audio(self.client, ticket)?;
            }
        }
        if state.starting.is_none() {
            self.cancel_native_duck()?;
            let storage = self.services.storage.as_ref().context("DC3 music scoped storage")?.clone();
            let name = request.asset.name.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            std::thread::Builder::new().name("circus-dc3-music-path".into()).spawn(move || {
                let result = music_path(storage.as_ref(), &name);
                let _ = sender.send(result.map_err(|error| format!("{error:#}")));
            })?;
            state.starting = Some(Starting {request: request.clone(), path: Some(receiver), ticket: None});
            return Ok(false);
        }
        let starting = state.starting.as_mut().unwrap();
        if let Some(receiver) = &starting.path {
            let path = match receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(false),
                Err(TryRecvError::Disconnected) => bail!("DC3 music path worker disconnected"),
                Ok(result) => result.map_err(anyhow::Error::msg)?,
            };
            let ticket = self.host.begin_audio(self.client, AudioRequest::PlayAsset {
                channel: 0, source: AudioAsset {path, frame_limit: None}, intro: None,
                parameters: AudioParameters {
                    bus: PcmBus::Music, mix_group: Some("circus/music".into()),
                    looping: request.looping, loop_start: 0, loop_end: None,
                    ..Default::default()
                },
            })?;
            starting.path = None;
            starting.ticket = Some(ticket);
            return Ok(false);
        }
        let ticket = starting.ticket.context("DC3 music preparation ticket")?;
        let receipt = self.host.audio_preparation(self.client, ticket)?;
        match receipt.phase {
            AudioPreparePhase::Pending => Ok(false),
            AudioPreparePhase::Failed => bail!("DC3 music preparation: {}", receipt.error.unwrap_or_else(|| "unknown decoder failure".into())),
            AudioPreparePhase::Ready => {
                self.host.release_audio(self.client, ticket)?;
                state.starting = None;
                Ok(true)
            }
        }
    }
    ///Called once per admitted native frame, independently of script/fence
    ///retries. Waiting inside publish_audio for60 frames would deadlock the
    ///same frame clock that owns the original deferred release.
    pub(in crate::adapter) fn advance_dc3_music(&mut self) -> Result<()> {
        let mut state = std::mem::take(&mut self.services.dc3_music);
        let result = (|| {
            if let Some((handle, updates)) = state.release {
                match self.host.audio_release_info(self.client, handle)?.phase {
                    AudioReleasePhase::Pending => return Ok(()),
                    AudioReleasePhase::Active => {
                        let next = updates.checked_add(1).context("DC3 music release clock overflow")?;
                        self.host.advance_audio_release(self.client, handle, next)?;
                        state.release = Some((handle, next));
                        return Ok(());
                    }
                    AudioReleasePhase::Complete => {
                        self.host.release_audio_release(self.client, handle)?;
                        state.release = None;
                    }
                    //A later 431DC0 restart (0x12 after a stop_after release)
                    //supersedes a release that has nothing queued behind it.
                    AudioReleasePhase::Cancelled if state.queued.is_none() => {
                        self.host.release_audio_release(self.client, handle)?;
                        state.release = None;
                    }
                    AudioReleasePhase::Cancelled => bail!("DC3 music replacement lost its release owner"),
                }
            }
            if let Some(request) = state.queued.clone() {
                if self.poll_dc3_music_start(&mut state, &request)? {
                    state.queued = None;
                    if std::mem::take(&mut state.stop_after) {
                        state.release = Some((self.begin_dc3_music_release()?, 0));
                    }
                }
            }
            Ok(())
        })();
        self.services.dc3_music = state;
        result
    }
    pub(in crate::adapter) fn dc3_music_moving(&self) -> bool {
        self.services.dc3_music.release.is_some() || self.services.dc3_music.queued.is_some()
    }
    pub(in crate::adapter) fn dc3_music_volume(&mut self, controls: crate::machine::MixerControls) -> Result<()> {
        ensure!(self.services.native_profile == Some(crate::Profile::Dc3WithYou), "DC3 music-volume profile");
        let has_source = self.host.audio_info(self.client, 0)?.is_some();
        let target = if controls.gain[0] <= 0.00001 { -10000 } else {
            (2000. * f64::from(controls.gain[0]).log10()).round().clamp(-10000.,0.) as i32
        };
        //Store future-buffer preferences while keeping the live volume. Like
        //431E60, an existing native release/volume actor is not restarted.
        self.sync_audio_controls_forced(controls, [false;3], [false;3])?;
        if has_source && self.services.dc3_music.release.is_none() {
            self.declare_native_music_level(target)?;
        }
        Ok(())
    }
    pub(in crate::adapter) fn close_dc3_music(&mut self) -> Result<()> {
        let state = std::mem::take(&mut self.services.dc3_music);
        if let Some(ticket) = state.starting.and_then(|starting| starting.ticket) {
            self.host.release_audio(self.client, ticket)?;
        }
        if let Some((handle, _)) = state.release {
            //An immediate channel-0 stop ends the running 60-frame release
            //(432490); the shared actor then reports Cancelled.
            if matches!(self.host.audio_release_info(self.client, handle)?.phase, AudioReleasePhase::Pending | AudioReleasePhase::Active) {
                self.host.submit_audio(self.client, AudioRequest::Stop { channel: 0, fade_ms: 0. })?;
            }
            self.host.release_audio_release(self.client, handle)?;
        }
        Ok(())
    }
}
fn music_path(storage: &dyn ClientStorage, name: &str) -> Result<VfsPath> {
    //443BA0 replaces the final extension and opens PCM, then WAV, then OGG.
    //It supplies no external native loop table or per-track intro positions.
    let native = name.replace('\\', "/");
    let stem = native.rfind(['/', ':', '.']).filter(|&at| native.as_bytes()[at] == b'.').unwrap_or(native.len());
    for extension in ["PCM", "WAV", "OGG"] {
        let path = VfsPath::new(Mount::Asset, &format!("Advdata/{}.{}", &native[..stem], extension))?;
        if storage.metadata(Mount::Asset, path.relative())?.is_some_and(|info| !info.is_directory) { return Ok(path); }
    }
    bail!("DC3 native music resource not found: Advdata/{native} (PCM/WAV/OGG)")
}
