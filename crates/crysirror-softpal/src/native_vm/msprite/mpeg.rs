//! Incremental adapter for the siglus_rs MPEG decoder.
use std::{collections::VecDeque, sync::Arc};

use na_mpeg2_decoder::{frame_to_rgba_bt601_limited, Frame, MpegVideoPipeline};

use super::PresentedFrame;

pub(super) struct MpegDecoder {
    bytes: Vec<u8>,
    cursor: usize,
    pipeline: MpegVideoPipeline,
    frames: VecDeque<Arc<Frame>>,
    flushed: bool,
    frame_index: u64,
    frame_ticks: u64,
    tick_divisor: u64,
    origin: Option<i64>,
    last_pts: Option<u32>,
}

impl MpegDecoder {
    pub(super) fn open(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let (frame_ticks, tick_divisor) = sequence_frame_ticks(&bytes)
            .ok_or_else(|| anyhow::anyhow!("MPEG sequence header has no valid frame rate"))?;
        Ok(Self {
            bytes,
            cursor: 0,
            pipeline: MpegVideoPipeline::new(),
            frames: VecDeque::new(),
            flushed: false,
            frame_index: 0,
            frame_ticks,
            tick_divisor,
            origin: None,
            last_pts: None,
        })
    }

    pub(super) fn rewind(&mut self) {
        self.cursor = 0;
        self.pipeline = MpegVideoPipeline::new();
        self.frames.clear();
        self.flushed = false;
        self.frame_index = 0;
        self.origin = None;
        self.last_pts = None;
    }

    pub(super) fn frame_duration_ms(&self) -> u32 {
        let end = self.frame_index.saturating_mul(self.frame_ticks) / self.tick_divisor;
        let start = self
            .frame_index
            .saturating_sub(1)
            .saturating_mul(self.frame_ticks)
            / self.tick_divisor;
        end.saturating_sub(start).max(1).min(u64::from(u32::MAX)) as u32
    }

    pub(super) fn next_frame(&mut self) -> anyhow::Result<Option<PresentedFrame>> {
        while self.frames.is_empty() && !self.flushed {
            if self.cursor < self.bytes.len() {
                // Keep only one input chunk's decoded frames, not the entire movie.
                let end = (self.cursor + 16 * 1024).min(self.bytes.len());
                let queue = &mut self.frames;
                self.pipeline
                    .push_with(&self.bytes[self.cursor..end], None, |f| queue.push_back(f))?;
                self.cursor = end;
            } else {
                let queue = &mut self.frames;
                self.pipeline.flush_with(|f| queue.push_back(f))?;
                self.flushed = true;
            }
        }
        let Some(frame) = self.frames.pop_front() else {
            return Ok(None);
        };
        let nominal = self.frame_index.saturating_mul(self.frame_ticks) / self.tick_divisor;
        let anchored = frame.pts_90k.and_then(|pts| {
            let origin = *self.origin.get_or_insert(pts - (nominal as i64 * 90));
            // PTS has a 33-bit wrap and is relative to the first displayed frame.
            let ticks = (pts - origin).rem_euclid(1_i64 << 33);
            (ticks <= (1_i64 << 32)).then_some((ticks / 90) as u64)
        });
        // A PES can contain several pictures sharing the same coarse PTS.
        // Preserve valid anchors and use the sequence cadence between them.
        let fallback = self.last_pts.map_or(nominal, |last| {
            u64::from(last)
                + (self.frame_index * self.frame_ticks / self.tick_divisor
                    - self.frame_index.saturating_sub(1) * self.frame_ticks / self.tick_divisor)
        });
        let pts_ms = anchored
            .filter(|&pts| self.last_pts.is_none_or(|last| pts > u64::from(last)))
            .unwrap_or(fallback)
            .min(u64::from(u32::MAX)) as u32;
        self.frame_index += 1;
        self.last_pts = Some(pts_ms);
        let mut rgba = vec![0; frame.width * frame.height * 4];
        frame_to_rgba_bt601_limited(&frame, &mut rgba);
        Ok(Some(PresentedFrame {
            pts_ms,
            width: frame.width as u32,
            height: frame.height as u32,
            rgba,
        }))
    }
}

pub(super) fn is_mpeg_packet(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0, 0, 1, 0xba]) || bytes.starts_with(&[0, 0, 1, 0xb3])
}

fn sequence_frame_ticks(bytes: &[u8]) -> Option<(u64, u64)> {
    let start = bytes.windows(4).position(|w| w == [0, 0, 1, 0xb3])?;
    let code = bytes.get(start + 7)? & 15;
    // 360 kHz represents all eight rates exactly, including 24000/1001.
    let ticks = match code {
        1 => 15015,
        2 => 15000,
        3 => 14400,
        4 => 12012,
        5 => 12000,
        6 => 7200,
        7 => 6006,
        8 => 6000,
        _ => return None,
    };
    for window in bytes[start + 8..].windows(10) {
        if window[..4] == [0, 0, 1, 0] {
            break;
        }
        if window[..4] == [0, 0, 1, 0xb5] && window[4] >> 4 == 1 {
            let n = u64::from((window[9] >> 5) & 3) + 1;
            let d = u64::from(window[9] & 31) + 1;
            return Some((ticks * d, 360 * n));
        }
    }
    Some((ticks, 360))
}
