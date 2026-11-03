// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/avg32/src/movie.rs (MPL-2.0).
//! AVG32 FMV playback.
//!
//! AVG32 titles ship cutscenes as Video-for-Windows AVI files (Cinepak video,
//! optionally uncompressed PCM audio). CrySirror: the RIFF/AVI demuxer and
//! the Cinepak decoder are the shared engine video service
//! (`crysirror_video::avi`); this module keeps the original playback policy:
//! one linear play-through (optionally looped) stretched into a rectangle of
//! display buffer 0, paced by the engine clock instead of `Instant`.
use anyhow::{Context, Result};
use crysirror_image::avg32::buffer::PdtBuffer;
use crysirror_image::avg32::{PdtManager, Rect};
use crysirror_video::avi::CinepakRgba;

pub use crysirror_video::avi::{AviAudio, AviMovie};

/// Plays an [`AviMovie`]'s video track into a rectangle of display buffer
/// 0 (`0x0e:50..55` stretch the movie to the requested area).
pub struct MoviePlayer {
    frames: Vec<Vec<u8>>,
    decoder: CinepakRgba,
    next_frame: usize,
    /// Frame interval in microseconds.
    interval_us: u64,
    /// Engine-clock due time of the next frame, in microseconds.
    due_us: u64,
    looped: bool,
    active: bool,
    rect: Rect,
}

impl std::fmt::Debug for MoviePlayer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MoviePlayer")
            .finish_non_exhaustive()
    }
}

impl MoviePlayer {
    /// CrySirror: `now` is the engine clock in milliseconds.
    pub fn start(
        movie: &AviMovie,
        looped: bool,
        rect: Rect,
        gfx: &mut PdtManager,
        now: u64,
    ) -> Result<Option<Self>> {
        if movie.video_frames.is_empty() {
            return Ok(None);
        }
        movie.require_cinepak()?;
        let rect = if rect.x2 <= rect.x1 || rect.y2 <= rect.y1 {
            Rect::full()
        } else {
            rect
        };
        let mut player = Self {
            frames: movie.video_frames.clone(),
            decoder: CinepakRgba::new(),
            next_frame: 0,
            interval_us: u64::from(movie.micro_sec_per_frame.max(1)),
            due_us: 0,
            looped,
            active: true,
            rect,
        };
        player.show_next(gfx)?;
        player.due_us = now.saturating_mul(1000) + player.interval_us;
        Ok(Some(player))
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn tick(&mut self, gfx: &mut PdtManager, now: u64) -> Result<()> {
        let now_us = now.saturating_mul(1000);
        if !self.active || now_us < self.due_us {
            return Ok(());
        }
        self.show_next(gfx)?;
        self.due_us += self.interval_us;
        if self.due_us < now_us {
            self.due_us = now_us + self.interval_us;
        }
        Ok(())
    }

    fn show_next(&mut self, gfx: &mut PdtManager) -> Result<()> {
        if self.next_frame >= self.frames.len() {
            if self.looped {
                self.next_frame = 0;
                self.decoder.reset();
            } else {
                self.active = false;
                return Ok(());
            }
        }
        let (width, height, rgba) = self
            .decoder
            .decode(&self.frames[self.next_frame])
            .with_context(|| format!("FMV frame {}", self.next_frame))?;
        let mut buffer = PdtBuffer::new(width as usize, height as usize);
        for (index, pixel) in rgba.chunks_exact(4).enumerate() {
            buffer.set_pixel(index, [pixel[0], pixel[1], pixel[2]]);
        }
        gfx.stretch_from(
            &buffer,
            Rect::new(0, 0, width as i32 - 1, height as i32 - 1),
            self.rect,
            0,
        );
        self.next_frame += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two real Cinepak frames (a solid colour) in a minimal movie.
    fn movie(rgb: [u8; 3]) -> AviMovie {
        let (w, h) = (32u32, 16u32);
        let pixels: Vec<u8> = (0..w * h).flat_map(|_| rgb).collect();
        let frame =
            oxideav_cinepak::encode_rgb24(&pixels, w, h, Default::default()).expect("encode");
        AviMovie {
            micro_sec_per_frame: 100_000,
            video_frames: vec![frame.clone(), frame],
            audio: None,
            video_codec: Some(*b"cvid"),
        }
    }

    #[test]
    fn plays_cinepak_into_buffer_zero_on_the_engine_clock() {
        let mut gfx = PdtManager::new();
        gfx.ensure(0);
        let mut player = MoviePlayer::start(&movie([200, 40, 40]), false, Rect::full(), &mut gfx, 1000)
            .unwrap()
            .expect("frames");
        let screen = gfx.get(0).unwrap();
        let pixel = screen.pixel(screen.at(320, 240));
        assert!(pixel[0] > 150 && pixel[1] < 90 && pixel[2] < 90, "{pixel:?}");
        // The second frame is due 100 ms later on the engine clock.
        player.tick(&mut gfx, 1050).unwrap();
        assert_eq!(player.next_frame, 1);
        player.tick(&mut gfx, 1100).unwrap();
        assert_eq!(player.next_frame, 2);
        assert!(player.active());
        player.tick(&mut gfx, 1200).unwrap();
        assert!(!player.active());
    }

    #[test]
    fn non_cinepak_video_fails_visibly() {
        let mut gfx = PdtManager::new();
        let mut other = movie([0, 0, 0]);
        other.video_codec = Some(*b"MJPG");
        assert!(MoviePlayer::start(&other, false, Rect::full(), &mut gfx, 0).is_err());
    }
}
