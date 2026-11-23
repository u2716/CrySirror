// SPDX-License-Identifier: MPL-2.0
//! CMVS movie-slot lifetime and queries over the CrySirror media owner.

use crate::vm::CmvsVm;
use anyhow::{bail, Result};
use crysirror_core::{
    service::{
        AssetReader, AudioPcm16Gain, HostServices, MediaPhase, MediaRequest, MediaSourceOptions,
    },
    state::ClientId,
};
use serde_json::json;
use std::sync::Arc;

const FULLSCREEN: u32 = 40;
const FULLSCREEN_LAYER: u32 = 3;

#[derive(Default)]
pub(super) struct Movies {
    fullscreen: Option<Fullscreen>,
    scene: [Option<u32>; 3],
    scene_sources: [Option<Arc<dyn AssetReader>>; 3],
    players: [Option<ScenePlayer>; 3],
}

const SCENE_BASE: u32 = 41;

struct Fullscreen {
    loading: bool,
    layer: bool,
    asset: String,
}

impl Movies {
    pub fn begin(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        source: Option<Arc<dyn AssetReader>>,
        gain: f32,
        frame_control: Option<u32>,
    ) -> Result<()> {
        self.stop(host, client)?;
        if let Some(source) = source {
            let options = MediaSourceOptions { looping: false, audio: true, gain };
            let fader = AudioPcm16Gain::default();
            let path = if let Some(mode) = frame_control {
                use crysirror_core::service::IndexedMoviePacing::*;
                let pacing = match mode % 3 { 1 => HalfRate30, 2 => Adaptive30, _ => NativeRate };
                host.indexed_movie_reader(client, FULLSCREEN, source, options, fader, pacing)?
            } else {
                host.media_reader_with_pcm16_gain(client, FULLSCREEN, source, options, fader)?
            };
            self.fullscreen = Some(Fullscreen {
                loading: true,
                layer: false,
                asset: path.relative().into(),
            });
        } else {
            vm.finish_owner_call(1, Some(0), vm.profile() == crate::vm::VmProfile::Cvns20040515X86)?;
        }
        Ok(())
    }

    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let Some(movie) = &mut self.fullscreen else {
            return Ok(false);
        };
        if !movie.loading {
            return Ok(false);
        }
        let info = host.media_info(client, FULLSCREEN)?;
        if info.phase == MediaPhase::Failed {
            bail!("CMVS fullscreen movie decoder failed");
        }
        if host.media_frame(client, FULLSCREEN)?.is_none() {
            return Ok(true);
        }
        let layer = json!({"op":"put","layer":{
            "id":FULLSCREEN_LAYER,"rect":[0,0,canvas[0],canvas[1]],"order":[i32::MAX,0],
            "paint":{"kind":"image","asset":movie.asset},
        }});
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[layer]}))?)?;
        movie.layer = true;
        vm.finish_owner_call(1, Some(1), vm.profile() == crate::vm::VmProfile::Cvns20040515X86)?;
        movie.loading = false;
        Ok(false)
    }

    pub fn frame(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<u32> {
        if self.fullscreen.is_none() {
            return Ok(u32::MAX);
        }
        let info = host.media_info(client, FULLSCREEN)?;
        match info.phase {
            MediaPhase::Failed => bail!("CMVS fullscreen movie playback failed"),
            MediaPhase::Ended | MediaPhase::Stopped => {
                self.stop(host, client)?;
                Ok(u32::MAX)
            }
            _ => Ok(info
                .published_frames
                .saturating_sub(1)
                .min(u64::from(u32::MAX - 1)) as u32),
        }
    }

    pub fn stop(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        if let Some(movie) = &self.fullscreen {
            if movie.layer {
                let commands = json!({"layers":[{"op":"remove","id":FULLSCREEN_LAYER}]});
                host.submit_engine(client, 1, &serde_json::to_vec(&commands)?)?;
            }
            host.media_request(client, MediaRequest::Remove { id: FULLSCREEN })?;
            self.fullscreen = None;
        }
        Ok(())
    }

    /// `0x2308`/`0x2309`: (re)starts a scene movie and returns its image asset.
    pub fn play_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        source: Option<Arc<dyn AssetReader>>,
        looping: bool,
    ) -> Result<Option<crysirror_core::path::VfsPath>> {
        use anyhow::Context;
        if let Some(source) = source {
            *self
                .scene_sources
                .get_mut(slot)
                .context("CMVS scene movie slot outside 0..2")? = Some(source);
        }
        let Some(source) = self.scene_sources.get(slot).cloned().flatten() else {
            return Ok(None);
        };
        self.remove_scene(host, client, slot)?;
        let id = SCENE_BASE + slot as u32;
        let path = host.media_reader_with_pcm16_gain(
            client,
            id,
            source,
            MediaSourceOptions {
                looping,
                audio: false,
                gain: 1.0,
            },
            AudioPcm16Gain::default(),
        )?;
        self.scene[slot] = Some(id);
        Ok(Some(path))
    }

    pub fn remove_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
    ) -> Result<()> {
        use anyhow::Context;
        let entry = self
            .scene
            .get_mut(slot)
            .context("CMVS scene movie slot outside 0..2")?;
        if let Some(id) = *entry {
            host.media_request(client, MediaRequest::Remove { id })?;
            *entry = None;
        }
        self.players[slot] = None;
        Ok(())
    }

    /// Scene movies stay out of checkpoints: their objects carry the visible state.
    pub fn live(&self) -> bool {
        self.fullscreen.is_some()
    }

    pub fn gain_targets(&self) -> impl Iterator<Item = u32> + '_ {
        self.fullscreen
            .iter()
            .map(|_| FULLSCREEN)
            .chain(self.scene.iter().flatten().copied())
    }
}

/// One `0x2309` section record (+88 + 12·n): loop flag, first and last frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Section {
    looping: bool,
    start: i32,
    end: u32,
}

/// Frame clock of a scene movie object (engine +4504, class 0x14004a7d0). Scripts start
/// sections, jump between them and read the frame; the media service only presents the
/// picture this clock selects.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ScenePlayer {
    frames: u32,
    rate: u32,
    scale: u32,
    sections: [Section; 16],
    /// +72 and +76.
    current: u32,
    pending: Option<u32>,
    /// +292.
    frame: u32,
    /// +80: stepping through a section.
    playing: bool,
    /// +84: frozen by `0x230c` or at the end of a section that does not loop.
    held: bool,
    /// +60: 0 sections, 10 single steps up to `stop`, 15 the audio clock.
    mode: u32,
    /// +68 (`0x231d`).
    stop: u32,
    /// +296 and +300.
    clock: u64,
    carry: u64,
}

impl ScenePlayer {
    /// 0x14004c090: section 0 runs from `start` to the last frame; nothing plays yet.
    pub fn new(frames: u32, rate: u32, scale: u32, start: u32) -> Self {
        let mut sections = [Section::default(); 16];
        sections[0] = Section { looping: false, start: start as i32, end: frames.saturating_sub(1) };
        Self {
            frames,
            rate: rate.max(1),
            scale: scale.max(1),
            sections,
            current: 0,
            pending: None,
            frame: start,
            playing: false,
            held: false,
            mode: 0,
            stop: 0,
            clock: 0,
            carry: 0,
        }
    }

    /// Stream position of a frame for the presenting decoder.
    pub fn position_ms(&self, frame: u32) -> u64 {
        u64::from(frame) * 1000 * u64::from(self.scale) / u64::from(self.rate)
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    pub fn running(&self) -> bool {
        self.playing && !self.held
    }

    /// `0x2309` (0x14004b910): a running movie queues the section; otherwise it starts.
    pub fn play(&mut self, section: u32, start: i32, end: i32, looping: bool, now: u64) {
        let Some(record) = self.sections.get_mut(section as usize) else {
            return;
        };
        let end = if end == -1 { self.frames.saturating_sub(1) } else { end as u32 };
        *record = Section { looping, start, end };
        if self.playing {
            self.pending = Some(section);
        } else {
            self.clock = now;
            self.carry = 0;
            self.current = section;
            self.pending = None;
            self.playing = true;
        }
    }

    /// `0x230b` (0x14004ba90): releases the hold, restarting the step clock.
    pub fn release(&mut self, now: u64) {
        if self.playing {
            self.clock = now;
            self.carry = 0;
        }
        self.held = false;
    }

    /// `0x230c` (0x14004be20).
    pub fn hold(&mut self) {
        self.held = true;
    }

    pub fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }

    pub fn set_stop(&mut self, frame: u32) {
        self.stop = frame;
    }

    /// `0x230f`: current section and frame.
    pub fn position(&self) -> (u32, u32) {
        (self.current, self.frame)
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    /// `0x231b` (0x14004b800): shows `frame` and stops stepping.
    pub fn seek(&mut self, frame: u32) -> bool {
        self.playing = false;
        if self.frame == frame || frame >= self.frames {
            if self.frame != frame {
                self.frame = frame;
            }
            return false;
        }
        self.frame = frame;
        true
    }

    /// `0x231c` (0x14004b9c0): starts the queued section at once. False without one.
    pub fn take_pending(&mut self) -> Option<bool> {
        let section = self.pending.take()?;
        self.current = section;
        let start = self.sections[section as usize].start;
        self.frame = if start == -1 { self.frame.wrapping_add(1) } else { start as u32 };
        Some(self.frame < self.frames)
    }

    fn steps(&mut self, elapsed: u64) -> u64 {
        self.carry += elapsed;
        let steps = self.carry * u64::from(self.rate) / 1000 / u64::from(self.scale);
        if steps != 0 {
            self.carry -= 1000 * steps * u64::from(self.scale) / u64::from(self.rate);
        }
        steps
    }

    /// 0x14004bad0: advances the clock and returns the frame to show when it changed.
    /// `audio` is the position of the sound channel mode 15 follows, when it plays.
    pub fn advance(&mut self, now: u64, audio: Option<u64>) -> Option<u32> {
        if self.held || !self.playing {
            return None;
        }
        let before = self.frame;
        match self.mode {
            0 => {
                let elapsed = now.saturating_sub(self.clock).min(1000);
                self.clock = now;
                for _ in 0..self.steps(elapsed) {
                    let section = self.sections[self.current as usize];
                    if self.frame < section.end {
                        self.frame += 1;
                    } else if let Some(next) = self.pending.take() {
                        self.current = next;
                        let start = self.sections[next as usize].start;
                        self.frame = match start {
                            -1 => self.frame + 1,
                            start if start as u32 == self.frame => self.frame + 1,
                            start => start as u32,
                        };
                    } else if section.looping {
                        self.frame = section.start as u32;
                    } else {
                        self.held = true;
                        self.playing = false;
                        break;
                    }
                }
            }
            10 => {
                let elapsed = now.saturating_sub(self.clock);
                self.clock = now;
                let end = self.sections[self.current as usize].end;
                for _ in 0..self.steps(elapsed) {
                    if self.frame >= end {
                        self.held = true;
                        self.playing = false;
                        break;
                    }
                    if self.stop == 0 || self.frame != self.stop {
                        self.frame += 1;
                    }
                }
            }
            15 => {
                let total = 1000 * u64::from(self.frames) * u64::from(self.scale) / u64::from(self.rate);
                let at = audio.unwrap_or_else(|| now.saturating_sub(self.clock));
                if total != 0 {
                    self.frame = (u64::from(self.rate) * (at % total) / 1000 / u64::from(self.scale)) as u32;
                }
            }
            _ => {}
        }
        (self.frame != before).then_some(self.frame)
    }

    /// Restored clocks restart from the load.
    pub fn rebase(&mut self, now: u64) {
        self.clock = now;
        self.carry = 0;
    }
}

impl Movies {
    /// `0x2308`/`0x230e` loaded: the media decoder stays paused on the first frame.
    pub fn load_scene(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        source: Arc<dyn AssetReader>,
        player: ScenePlayer,
    ) -> Result<crysirror_core::path::VfsPath> {
        use anyhow::Context;
        let path = self.play_scene(host, client, slot, Some(source), false)?.context("CMVS scene movie source")?;
        let id = SCENE_BASE + slot as u32;
        host.media_request(client, MediaRequest::Pause { id })?;
        host.media_request(client, MediaRequest::Seek { id, position_ms: player.position_ms(player.frame()) })?;
        if player.running() {
            host.media_request(client, MediaRequest::Resume { id })?;
        }
        self.players[slot] = Some(player);
        Ok(path)
    }

    pub fn player(&mut self, slot: usize) -> Option<&mut ScenePlayer> {
        self.players.get_mut(slot).and_then(Option::as_mut)
    }

    pub fn players(&self) -> &[Option<ScenePlayer>; 3] {
        &self.players
    }

    /// Brings the decoder to the player's frame and run state after a script command.
    pub fn sync_scene(&mut self, host: &mut dyn HostServices, client: ClientId, slot: usize, seek: bool) -> Result<()> {
        let (Some(player), Some(id)) = (self.players[slot].as_ref(), self.scene[slot]) else {
            return Ok(());
        };
        if seek {
            host.media_request(client, MediaRequest::Seek { id, position_ms: player.position_ms(player.frame()) })?;
        }
        let request = if player.running() { MediaRequest::Resume { id } } else { MediaRequest::Pause { id } };
        host.media_request(client, request)?;
        Ok(())
    }

    /// Steps every player; consecutive frames follow the running decoder, jumps seek it.
    pub fn advance_scenes(&mut self, host: &mut dyn HostServices, client: ClientId, now: u64) -> Result<()> {
        for slot in 0..3 {
            let Some(player) = self.players[slot].as_mut() else { continue };
            let before = player.frame();
            let was_running = player.running();
            let Some(frame) = player.advance(now, None) else {
                if was_running != player.running() {
                    self.sync_scene(host, client, slot, true)?;
                }
                continue;
            };
            let consecutive = frame > before && player.running() && player.mode != 15;
            if !consecutive || was_running != player.running() {
                self.sync_scene(host, client, slot, true)?;
            }
        }
        Ok(())
    }
}

/// CMV header words +16, +20 and +24: frame count, rate and scale.
pub(super) fn timing(head: &[u8; 36]) -> [u32; 3] {
    [16, 20, 24].map(|at| u32::from_le_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_loop_queue_and_hold_at_their_end() {
        // 10 frames per second.
        let mut player = ScenePlayer::new(30, 10, 1, 0);
        assert_eq!(player.advance(100, None), None);
        player.play(1, 5, 7, true, 0);
        assert_eq!(player.advance(250, None), Some(2));
        player.frame = 7;
        assert_eq!(player.advance(350, None), Some(5));
        player.play(2, 20, -1, false, 350);
        assert_eq!(player.advance(450, None), Some(6));
        player.frame = 7;
        assert_eq!(player.advance(550, None), Some(20));
        player.frame = 29;
        assert_eq!(player.advance(650, None), None);
        assert!(!player.playing() && player.held);
    }
}
