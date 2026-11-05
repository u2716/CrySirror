use std::collections::BTreeMap;
use std::io::Cursor;

mod mpeg;
mod audio;
use audio::MovieAudio;
use crate::native_vm::audio::{AudioSystem, PalSoundGroup};
use mpeg::MpegDecoder;
use wmv_decoder::{AsfWmv2Decoder, YuvFrame};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MSpriteHandle(pub u32);

#[derive(Debug)]
pub struct MSpriteSystem {
    next_handle: u32,
    entries: BTreeMap<MSpriteHandle, MSpriteEntry>,
    movie: Option<MoviePlayback>,
    audio_master: i32,
    audio_group4: i32,
}

impl Default for MSpriteSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MSpriteSystem {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            entries: BTreeMap::new(),
            movie: None,
            audio_master: 10000,
            audio_group4: 10000,
        }
    }

    pub fn load_wmv(
        &mut self,
        name: impl Into<String>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<LoadedMSprite> {
        let decoder = AsfWmv2Decoder::open(Cursor::new(bytes.clone()))?;
        let info = decoder.video_stream_info().clone();
        let mut source = FrameSource::Wmv(decoder);
        let first = next_presented(&mut source)?;
        let (width, height, rgba, pts_ms) = match first {
            Some(frame) => (frame.width, frame.height, frame.rgba, frame.pts_ms),
            None => {
                let rgba = vec![0; info.width.max(1) as usize * info.height.max(1) as usize * 4];
                (info.width.max(1), info.height.max(1), rgba, 0)
            }
        };
        let audio = MovieAudio::decode(&bytes, self.initial_gain());
        let loaded = self.insert_loaded(name.into(), bytes, source, width, height, rgba, pts_ms);
        self.entries.get_mut(&loaded.handle).unwrap().audio = audio;
        Ok(loaded)
    }

    pub fn load_movie(
        &mut self,
        name: impl Into<String>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<LoadedMSprite> {
        match movie_container(&bytes) {
            MovieContainer::Mpeg => self.load_mpeg(name, bytes),
            MovieContainer::Mp4 => {
                anyhow::bail!("mp4 playback is not supported by the verified PAL profiles")
            }
            MovieContainer::Wmv | MovieContainer::Unknown => self.load_wmv(name, bytes),
        }
    }

    fn load_mpeg(
        &mut self,
        name: impl Into<String>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<LoadedMSprite> {
        let audio = MovieAudio::decode(&bytes, self.initial_gain());
        let mut decoder = MpegDecoder::open(bytes).map_err(|err| anyhow::anyhow!(err))?;
        let Some(frame) = decoder.next_frame().map_err(|err| anyhow::anyhow!(err))? else {
            anyhow::bail!("mpeg stream produced no video frame");
        };
        let loaded = self.insert_loaded(
            name.into(),
            Vec::new(),
            FrameSource::Mpeg(decoder),
            frame.width,
            frame.height,
            frame.rgba,
            frame.pts_ms,
        );
        self.entries.get_mut(&loaded.handle).unwrap().audio = audio;
        Ok(loaded)
    }

    fn insert_loaded(
        &mut self,
        name: String,
        bytes: Vec<u8>,
        decoder: FrameSource,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        pts_ms: u32,
    ) -> LoadedMSprite {
        let handle = self.allocate_handle();
        self.entries.insert(
            handle,
            MSpriteEntry {
                name: name.clone(),
                bytes,
                decoder,
                width,
                height,
                current_rgba: rgba.clone(),
                current_pts_ms: pts_ms,
                playback_pts_ms: pts_ms,
                pending_frame: None,
                source_ended: false,
                frame_duration_ms: 1,
                playing: false,
                locked: false,
                loop_mode: 0,
                loop_start: 0,
                loop_end: 0,
                finished: false,
                state_bits: 0,
                loop_generation: 0,
                active_end_ms: None,
                audio: MovieAudio::new(None, self.initial_gain()),
            },
        );
        LoadedMSprite {
            handle,
            width,
            height,
            rgba,
            name,
        }
    }

    pub fn release(&mut self, handle: MSpriteHandle) -> bool {
        self.entries.remove(&handle).is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Snapshot the native master and group gains before a load or setter.
    pub fn set_audio_levels(&mut self, audio: &AudioSystem) {
        self.audio_master = audio.primary_volume().raw();
        self.audio_group4 = audio.group_volume(PalSoundGroup::GROUP4).raw();
    }

    fn initial_gain(&self) -> i32 {
        self.audio_group4.wrapping_mul(self.audio_master) / 10000
    }

    pub fn set_volume(&mut self, handle: MSpriteHandle, percent: i32, bypass_group: bool) -> bool {
        let raw = if bypass_group { percent.wrapping_mul(100) } else {
            percent.wrapping_mul(self.audio_group4) / 100
        };
        self.set_raw_audio_gain(handle, raw.wrapping_mul(self.audio_master) / 10000)
    }

    pub fn set_raw_audio_gain(&mut self, handle: MSpriteHandle, gain: i32) -> bool {
        self.entries.get_mut(&handle).is_some_and(|entry| entry.audio.set_gain(gain))
    }

    pub fn sync_audio(&mut self, audio: &mut AudioSystem) {
        for entry in self.entries.values_mut() {
            entry.audio.sync(audio, entry.playing && !entry.finished, entry.playback_pts_ms, entry.loop_generation);
        }
    }

    pub fn check(&self, handle: MSpriteHandle) -> bool {
        self.entries.contains_key(&handle)
    }

    pub fn play(&mut self, handle: MSpriteHandle, loop_mode: i32) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        entry.playing = true;
        entry.finished = false;
        entry.loop_mode = loop_mode;
        entry.state_bits = (entry.state_bits & MSPRITE_STATE_LOOP) | MSPRITE_STATE_PLAYING;
        true
    }

    pub fn stop(&mut self, handle: MSpriteHandle) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        if entry.playing {
            entry.state_bits = (entry.state_bits & MSPRITE_STATE_LOOP) | MSPRITE_STATE_STOPPED;
        }
        entry.playing = false;
        entry.finished = false;
        entry.audio.stop();
        true
    }

    /// Native PalMSpritePause accepts a nonnull sprite and otherwise does nothing.
    /// Portable movie handles represent only sprites with a movie worker.
    pub fn pause(&mut self, handle: MSpriteHandle) -> bool {
        self.entries.contains_key(&handle)
    }

    pub fn lock(&mut self, handle: MSpriteHandle) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        entry.locked = true;
        true
    }

    pub fn unlock(&mut self, handle: MSpriteHandle) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        entry.locked = false;
        true
    }

    pub fn set_loop(&mut self, handle: MSpriteHandle, loop_mode: i32) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        entry.loop_mode = loop_mode;
        true
    }

    pub fn set_loop_point(&mut self, handle: MSpriteHandle, start: i32, end: i32) -> bool {
        let Some(entry) = self.entries.get_mut(&handle) else {
            return false;
        };
        entry.loop_start = start;
        if end != 0 {
            entry.loop_end = end;
        }
        true
    }

    /// PalMSpriteIsLoop tests bit 31 of the native state word, independently
    /// of the loop counter set by PalMSpriteSetLoop and PalMSpritePlay.
    pub fn is_loop(&self, handle: MSpriteHandle) -> bool {
        self.state(handle) & MSPRITE_STATE_LOOP != 0
    }

    pub fn state(&self, handle: MSpriteHandle) -> u32 {
        self.entries.get(&handle).map_or(0, |entry| entry.state_bits)
    }

    pub fn advance(&mut self, delta_ms: u32) -> Vec<MSpriteFrameUpdate> {
        let mut updates = Vec::new();
        let handles = self.entries.keys().copied().collect::<Vec<_>>();
        for handle in handles {
            let Some(entry) = self.entries.get_mut(&handle) else {
                continue;
            };
            if !entry.playing || entry.finished {
                continue;
            }
            if delta_ms == 0 {
                continue;
            }
            // The presentation clock must advance by elapsed time, not by the
            // timestamp of the last decoded frame. Otherwise every renderer
            // tick consumes a frame even when the movie runs at a lower rate.
            let mut target_pts = entry.playback_pts_ms.saturating_add(delta_ms);
            let mut latest = None;
            let start = entry.loop_start.max(0) as u32;
            // Native SetLoopPoint stores a future seek range. Run does not seek:
            // the first pass ends at stream EOF, and only a loop restart installs
            // the requested stop position on the graph.
            let explicit_end = entry.active_end_ms;
            let mut finish_at_end = false;
            if let Some(end) = explicit_end.filter(|end| target_pts >= *end) {
                if entry.loop_mode != 0 {
                    // Stored points can change while the previous seek range
                    // is still active. Carry time into the new range only.
                    let next_end = entry.loop_end.max(0) as u32;
                    if next_end <= start {
                        log::warn!("invalid movie loop range: {start}..{next_end}");
                        finish_entry(entry);
                        continue;
                    }
                    target_pts = start + (target_pts - end) % (next_end - start);
                    if let Err(err) = rewind_entry(entry) {
                        log::warn!("[trace-msprite] restart {:?} failed: {err}", entry.name);
                        finish_entry(entry);
                        continue;
                    }
                } else {
                    target_pts = end.saturating_sub(1);
                    finish_at_end = true;
                }
            }
            let mut restarts = 0;
            loop {
                if entry.source_ended {
                    // Keep even a one-picture stream visible for its declared
                    // duration. There is no observed PTS interval in that case.
                    let end = explicit_end.unwrap_or_else(|| match &entry.decoder {
                        FrameSource::Wmv(decoder) => decoder
                            .duration_ms()
                            .and_then(|end| u32::try_from(end).ok())
                            .filter(|&end| end > entry.current_pts_ms)
                            .unwrap_or_else(|| {
                                entry.current_pts_ms.saturating_add(entry.frame_duration_ms)
                            }),
                        FrameSource::Mpeg(decoder) => entry
                            .current_pts_ms
                            .saturating_add(decoder.frame_duration_ms()),
                        #[cfg(test)]
                        FrameSource::Frames { .. } => {
                            entry.current_pts_ms.saturating_add(entry.frame_duration_ms)
                        }
                    });
                    if target_pts < end {
                        break;
                    }
                    if entry.loop_mode == 0 || restarts >= 1 {
                        finish_entry(entry);
                        break;
                    }
                    let next_end = if entry.loop_end > 0 {
                        entry.loop_end as u32
                    } else {
                        end
                    };
                    if next_end <= start {
                        log::warn!("invalid movie loop range: {start}..{next_end}");
                        finish_entry(entry);
                        break;
                    }
                    target_pts = start + (target_pts - end) % (next_end - start);
                    restarts += 1;
                    if let Err(err) = rewind_entry(entry) {
                        log::warn!("[trace-msprite] restart {:?} failed: {err}", entry.name);
                        finish_entry(entry);
                        break;
                    }
                }
                let pulled = match entry.pending_frame.take() {
                    Some(frame) => Ok(Some(frame)),
                    None => next_presented(&mut entry.decoder),
                };
                let frame = match pulled {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        entry.source_ended = true;
                        continue;
                    }
                    Err(err) => {
                        log::warn!("[trace-msprite] decode {:?} failed: {err}", entry.name);
                        finish_entry(entry);
                        break;
                    }
                };
                if frame.pts_ms > target_pts {
                    entry.pending_frame = Some(frame);
                    break;
                }
                apply_presented(entry, handle, frame, &mut latest);
            }
            entry.playback_pts_ms = if finish_at_end {
                explicit_end.unwrap()
            } else {
                target_pts
            };
            if finish_at_end {
                finish_entry(entry);
            }
            if let Some(update) = latest {
                updates.push(update);
            }
        }
        if let Some(movie) = self.movie.as_mut() {
            if let Some(handle) = movie.handle {
                if let Some(entry) = self.entries.get(&handle) {
                    movie.playing = entry.playing && !entry.finished;
                    movie.elapsed_ms = entry.playback_pts_ms;
                } else {
                    movie.playing = false;
                }
            }
        }
        updates
    }

    pub fn start_movie(&mut self, name: impl Into<String>, flags: u16, handle: MSpriteHandle) {
        self.movie = Some(MoviePlayback {
            name: name.into(),
            flags,
            playing: true,
            elapsed_ms: 0,
            handle: Some(handle),
        });
    }

    pub fn stop_movie(&mut self) {
        self.movie = None;
    }

    pub fn is_movie(&self) -> bool {
        self.movie.as_ref().is_some_and(|movie| movie.playing)
    }

    pub fn movie(&self) -> Option<&MoviePlayback> {
        self.movie.as_ref()
    }

    fn allocate_handle(&mut self) -> MSpriteHandle {
        let handle = MSpriteHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .expect("MSprite handle space exhausted");
        handle
    }
}

#[derive(Clone, Debug)]
pub struct LoadedMSprite {
    pub handle: MSpriteHandle,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct MSpriteFrameUpdate {
    pub handle: MSpriteHandle,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub source_name: String,
}

#[derive(Clone, Debug)]
pub struct MoviePlayback {
    pub name: String,
    pub flags: u16,
    pub playing: bool,
    pub elapsed_ms: u32,
    pub handle: Option<MSpriteHandle>,
}

struct MSpriteEntry {
    name: String,
    bytes: Vec<u8>,
    decoder: FrameSource,
    width: u32,
    height: u32,
    current_rgba: Vec<u8>,
    current_pts_ms: u32,
    playback_pts_ms: u32,
    pending_frame: Option<PresentedFrame>,
    source_ended: bool,
    frame_duration_ms: u32,
    playing: bool,
    locked: bool,
    loop_mode: i32,
    loop_start: i32,
    loop_end: i32,
    finished: bool,
    state_bits: u32,
    loop_generation: u64,
    active_end_ms: Option<u32>,
    audio: MovieAudio,
}

impl std::fmt::Debug for MSpriteEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MSpriteEntry")
            .field("name", &self.name)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("current_pts_ms", &self.current_pts_ms)
            .field("playing", &self.playing)
            .field("locked", &self.locked)
            .field("loop_mode", &self.loop_mode)
            .field("loop_start", &self.loop_start)
            .field("loop_end", &self.loop_end)
            .field("finished", &self.finished)
            .field("state_bits", &self.state_bits)
            .finish_non_exhaustive()
    }
}

enum FrameSource {
    Wmv(AsfWmv2Decoder<Cursor<Vec<u8>>>),
    Mpeg(MpegDecoder),
    #[cfg(test)]
    Frames {
        frames: Vec<PresentedFrame>,
        next: usize,
    },
}

#[derive(Clone)]
struct PresentedFrame {
    pts_ms: u32,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MovieContainer {
    Mpeg,
    Wmv,
    Mp4,
    Unknown,
}

fn movie_container(bytes: &[u8]) -> MovieContainer {
    if mpeg::is_mpeg_packet(bytes) {
        MovieContainer::Mpeg
    } else if bytes.len() >= 4 && bytes[..4] == [0x30, 0x26, 0xB2, 0x75] {
        MovieContainer::Wmv
    } else if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
        MovieContainer::Mp4
    } else {
        MovieContainer::Unknown
    }
}

fn finish_entry(entry: &mut MSpriteEntry) {
    entry.audio.stop();
    entry.playing = false;
    entry.finished = true;
    entry.state_bits = (entry.state_bits & MSPRITE_STATE_LOOP)
        | MSPRITE_STATE_STOPPED | MSPRITE_STATE_FINISHED;
}

fn apply_presented(
    entry: &mut MSpriteEntry,
    handle: MSpriteHandle,
    frame: PresentedFrame,
    latest: &mut Option<MSpriteFrameUpdate>,
) {
    entry.width = frame.width;
    entry.height = frame.height;
    if frame.pts_ms > entry.current_pts_ms {
        entry.frame_duration_ms = frame.pts_ms - entry.current_pts_ms;
    }
    entry.current_pts_ms = frame.pts_ms;
    entry.current_rgba = frame.rgba.clone();
    *latest = Some(MSpriteFrameUpdate {
        handle,
        width: frame.width,
        height: frame.height,
        rgba: frame.rgba,
        source_name: entry.name.clone(),
    });
}

fn rewind_entry(entry: &mut MSpriteEntry) -> anyhow::Result<()> {
    // Native retries a rejected DirectShow seek indefinitely. Reject the invalid
    // range instead of reproducing that hang on a portable backend.
    anyhow::ensure!(entry.loop_start >= 0 && entry.loop_end >= 0, "negative movie loop point");
    entry.active_end_ms = (entry.loop_end > 0).then_some(entry.loop_end as u32);
    entry.pending_frame = restart_source(&mut entry.decoder, &entry.bytes, entry.loop_start)?;
    entry.source_ended = false;
    entry.loop_generation = entry.loop_generation.wrapping_add(1);
    Ok(())
}

fn next_presented(source: &mut FrameSource) -> anyhow::Result<Option<PresentedFrame>> {
    match source {
        #[cfg(test)]
        FrameSource::Frames { frames, next } => {
            let frame = frames.get(*next).cloned();
            *next += usize::from(frame.is_some());
            Ok(frame)
        }
        FrameSource::Wmv(decoder) => match decoder.next_frame()? {
            Some(frame) => Ok(Some(PresentedFrame {
                pts_ms: frame.pts_ms,
                width: frame.frame.width,
                height: frame.frame.height,
                rgba: yuv420_to_rgba(&frame.frame),
            })),
            None => Ok(None),
        },
        FrameSource::Mpeg(decoder) => {
            match decoder.next_frame().map_err(|err| anyhow::anyhow!(err))? {
                Some(frame) => Ok(Some(PresentedFrame {
                    pts_ms: frame.pts_ms,
                    width: frame.width,
                    height: frame.height,
                    rgba: frame.rgba,
                })),
                None => Ok(None),
            }
        }
    }
}

fn restart_source(
    source: &mut FrameSource,
    bytes: &[u8],
    loop_start: i32,
) -> anyhow::Result<Option<PresentedFrame>> {
    let _ = loop_start;
    match source {
        #[cfg(test)]
        FrameSource::Frames { next, .. } => {
            *next = 0;
            Ok(None)
        }
        FrameSource::Wmv(decoder) => {
            *decoder = AsfWmv2Decoder::open(Cursor::new(bytes.to_vec()))?;
            // Decode forward on the presentation clock. Preroll must retain
            // the frame before loop_start until the next frame is due.
            Ok(None)
        }
        FrameSource::Mpeg(decoder) => {
            // Rebuild reference pictures from the beginning; advance() prerolls
            // on its presentation clock and keeps the preceding frame at start.
            decoder.rewind();
            Ok(None)
        }
    }
}

pub const MSPRITE_STATE_PLAYING: u32 = 0x0000_0001;
pub const MSPRITE_STATE_STOPPED: u32 = 0x0000_0002;
pub const MSPRITE_STATE_FINISHED: u32 = 0x0000_0004;
pub const MSPRITE_STATE_LOOP: u32 = 0x8000_0000;

fn yuv420_to_rgba(frame: &YuvFrame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let chroma_width = (width / 2).max(1);
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let yy = frame.y[y * width + x] as i32;
            let uv_idx = (y / 2) * chroma_width + (x / 2);
            let cb = frame.cb.get(uv_idx).copied().unwrap_or(128) as i32;
            let cr = frame.cr.get(uv_idx).copied().unwrap_or(128) as i32;
            let c = yy - 16;
            let d = cb - 128;
            let e = cr - 128;
            let r = ((298 * c + 409 * e + 128) >> 8).clamp(0, 255) as u8;
            let g = ((298 * c - 100 * d - 208 * e + 128) >> 8).clamp(0, 255) as u8;
            let b = ((298 * c + 516 * d + 128) >> 8).clamp(0, 255) as u8;
            let out = (y * width + x) * 4;
            rgba[out] = r;
            rgba[out + 1] = g;
            rgba[out + 2] = b;
            rgba[out + 3] = 255;
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuv420_black_frame_converts_to_opaque_rgba() {
        let frame = YuvFrame::new(2, 2);
        let rgba = yuv420_to_rgba(&frame);
        assert_eq!(rgba.len(), 16);
        assert!(rgba.chunks_exact(4).all(|px| px[3] == 255));
        assert!(rgba
            .chunks_exact(4)
            .all(|px| px[0] <= 1 && px[1] <= 1 && px[2] <= 1));
    }

    fn timed_movie() -> (MSpriteSystem, MSpriteHandle) {
        let mut system = MSpriteSystem::new();
        let frames = [0, 40, 80]
            .into_iter()
            .map(|pts_ms| PresentedFrame {
                pts_ms,
                width: 1,
                height: 1,
                rgba: vec![pts_ms as u8, 0, 0, 255],
            })
            .collect();
        let loaded = system.insert_loaded(
            "timed".into(),
            Vec::new(),
            FrameSource::Frames { frames, next: 1 },
            1,
            1,
            vec![0, 0, 0, 255],
            0,
        );
        system.play(loaded.handle, 0);
        (system, loaded.handle)
    }

    #[cfg(feature = "host-audio")]
    #[test]
    fn decoded_wmv_and_mpeg_tracks_reach_the_host_sink_and_stop_with_the_movie() {
        for bytes in [
            include_bytes!("../tests/fixtures/movie_audio_delayed.mpg").as_slice(),
            include_bytes!("../tests/fixtures/movie_audio_delayed.wmv").as_slice(),
        ] {
            let mut audio = crate::native_vm::audio::AudioSystem::new(crate::native_vm::audio::AudioConfig { enabled: true }).unwrap();
            let sink = audio.host_sink();
            let mut system = MSpriteSystem::new();
            let movie = system.load_movie("soundtrack", bytes.to_vec()).unwrap();
            system.play(movie.handle, 0);
            system.sync_audio(&mut audio);
            let mut output = vec![0; 48000];
            sink.render_i16(&mut output);
            assert!(output.iter().any(|sample| sample.abs() > 50));
            system.stop(movie.handle);
            sink.render_i16(&mut output);
            assert!(output.iter().all(|&sample| sample == 0));
            system.play(movie.handle, 0);
            system.sync_audio(&mut audio);
            sink.render_i16(&mut output);
            assert!(output.iter().any(|sample| sample.abs() > 50));
            system.release(movie.handle);
            sink.render_i16(&mut output);
            assert!(output.iter().all(|&sample| sample == 0));
        }
    }

    #[test]
    fn movie_sprite_gain_is_a_load_and_set_snapshot() {
        let mut audio = crate::native_vm::audio::AudioSystem::new(crate::native_vm::audio::AudioConfig { enabled: false }).unwrap();
        audio.set_primary_volume(crate::native_vm::audio::PalVolume::from_raw(5000)).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP4, crate::native_vm::audio::PalVolume::from_raw(4000)).unwrap();
        let mut system = MSpriteSystem::new();
        system.set_audio_levels(&audio);
        let loaded = system.load_movie("audio", include_bytes!("../tests/fixtures/movie_audio_delayed.mpg").to_vec()).unwrap();
        assert_eq!(system.entries[&loaded.handle].audio.gain(), 2000);
        audio.set_primary_volume(crate::native_vm::audio::PalVolume::from_raw(2500)).unwrap();
        system.set_audio_levels(&audio);
        system.play(loaded.handle, 1);
        system.sync_audio(&mut audio);
        assert_eq!(system.entries[&loaded.handle].audio.gain(), 2000);
        assert!(system.set_volume(loaded.handle, 50, false));
        assert_eq!(system.entries[&loaded.handle].audio.gain(), 500);
        assert!(system.set_volume(loaded.handle, 50, true));
        assert_eq!(system.entries[&loaded.handle].audio.gain(), 1250);
    }

    #[test]
    fn presentation_waits_for_frame_pts_and_accumulates_small_ticks() {
        let (mut system, handle) = timed_movie();
        assert!(system.advance(16).is_empty());
        assert!(system.advance(16).is_empty());
        assert_eq!(system.advance(16)[0].rgba[0], 40);
        assert!(system.advance(16).is_empty());
        assert_eq!(system.advance(16)[0].rgba[0], 80);
        assert!(system.advance(39).is_empty());
        assert_ne!(system.state(handle) & MSPRITE_STATE_PLAYING, 0);
        assert!(system.advance(1).is_empty());
        assert_ne!(system.state(handle) & MSPRITE_STATE_FINISHED, 0);
    }

    #[test]
    fn native_pause_and_lock_do_not_stop_the_graph_clock() {
        let (mut system, handle) = timed_movie();
        assert!(system.advance(0).is_empty());
        assert!(system.pause(handle));
        assert_eq!(system.advance(40)[0].rgba[0], 40);
        assert!(system.lock(handle));
        assert_eq!(system.state(handle), 1);
        assert_eq!(system.advance(40)[0].rgba[0], 80);
        assert!(system.unlock(handle));
        system.stop(handle);
        assert_eq!(system.state(handle), 2);
        assert!(system.advance(1_000).is_empty());
        system.play(handle, 1);
        assert_eq!(system.state(handle), 1);
        assert!(!system.is_loop(handle));
    }

    #[test]
    fn loop_wraps_clock_and_keeps_remainder_without_running_to_eof_again() {
        let (mut system, handle) = timed_movie();
        system.set_loop(handle, 1);
        system.start_movie("timed", 0, handle);
        // Eight complete 120ms loops plus 40ms into the ninth.
        assert_eq!(system.advance(1_000)[0].rgba[0], 40);
        assert_eq!(system.movie().unwrap().elapsed_ms, 40);
        assert!(system.advance(39).is_empty());
        assert_eq!(system.advance(1)[0].rgba[0], 80);
        assert_eq!(system.advance(40)[0].rgba[0], 0);
        assert_ne!(system.state(handle) & MSPRITE_STATE_PLAYING, 0);
    }

    #[test]
    fn explicit_loop_points_apply_on_restart_and_stop_at_end() {
        let (mut system, handle) = timed_movie();
        system.set_loop_point(handle, 40, 100);
        system.set_loop(handle, 1);
        // The intro still plays; loop_start only chooses the restart point.
        assert!(system.advance(16).is_empty());
        assert_eq!(system.advance(104)[0].rgba[0], 40);
        assert_eq!(system.entries[&handle].playback_pts_ms, 40);
        assert!(system.advance(20).is_empty());
        assert_eq!(system.advance(20)[0].rgba[0], 80);
        assert_eq!(system.advance(20)[0].rgba[0], 40);

        let (mut system, handle) = timed_movie();
        system.set_loop_point(handle, 40, 100);
        assert_eq!(system.advance(1_000)[0].rgba[0], 80);
        assert_ne!(system.state(handle) & MSPRITE_STATE_FINISHED, 0);
    }

    #[test]
    fn changed_loop_points_replace_the_next_seek_not_the_active_end() {
        let (mut system, handle) = timed_movie();
        system.set_loop_point(handle, 0, 40);
        system.set_loop(handle, 1);
        system.advance(120);
        assert_eq!(system.entries[&handle].playback_pts_ms, 0);
        system.set_loop_point(handle, 80, 120);
        system.advance(45);
        assert_eq!(system.entries[&handle].playback_pts_ms, 85);
        assert_eq!(system.entries[&handle].active_end_ms, Some(120));

        system.set_loop_point(handle, 120, 100);
        system.advance(40);
        assert_eq!(system.state(handle), MSPRITE_STATE_STOPPED | MSPRITE_STATE_FINISHED);
    }

    #[test]
    fn movie_container_detects_mpeg_wmv_and_mp4() {
        assert_eq!(
            movie_container(&[0x00, 0x00, 0x01, 0xBA, 0x21]),
            MovieContainer::Mpeg
        );
        assert_eq!(
            movie_container(&[0x30, 0x26, 0xB2, 0x75]),
            MovieContainer::Wmv
        );
        let mut mp4 = vec![0, 0, 0, 0x18];
        mp4.extend_from_slice(b"ftyp");
        assert_eq!(movie_container(&mp4), MovieContainer::Mp4);
    }
}
