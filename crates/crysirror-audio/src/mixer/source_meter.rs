// SPDX-License-Identifier: MPL-2.0

use crate::decoder::StereoFrame;
use anyhow::{ensure, Result};
use crysirror_core::service::AudioSourceMeterSpec;

const MAX_HISTORY: usize = 8;

pub(crate) struct SourceMeter {
    step: u64,
    window: u64,
    phase: u64,
    totals: [u64; 2],
    frames: u64,
    history: [[u32; 2]; MAX_HISTORY],
    length: usize,
    next: usize,
}

impl SourceMeter {
    pub fn new(spec: AudioSourceMeterSpec, mixer_rate: u32) -> Result<Self> {
        let step = u64::from(spec.reference_rate);
        let window = u64::from(spec.window_frames) * u64::from(mixer_rate);
        let length = usize::from(spec.history_windows);
        ensure!(
            (8_000..=192_000).contains(&spec.reference_rate)
                && spec.window_frames <= spec.reference_rate
                && window >= step
                && (1..=MAX_HISTORY).contains(&length),
            "audio source meter configuration out of range"
        );
        Ok(Self {
            step,
            window,
            phase: 0,
            totals: [0; 2],
            frames: 0,
            history: [[0; 2]; MAX_HISTORY],
            length,
            next: 0,
        })
    }

    pub fn record(&mut self, frame: StereoFrame) {
        for (total, sample) in self.totals.iter_mut().zip(frame) {
            *total += u64::from(crate::levels::to_pcm16(sample).unsigned_abs());
        }
        self.frames += 1;
        self.phase += self.step;
        if self.phase >= self.window {
            self.history[self.next] = self.totals.map(|total| (total / self.frames) as u32);
            self.next = (self.next + 1) % self.length;
            self.phase -= self.window;
            self.frames = 0;
            self.totals = [0; 2];
        }
    }

    pub fn level(&self) -> [u32; 2] {
        self.history[self.next]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_meter_selects_the_next_write_slot_at_fractional_window_boundaries() {
        let mut meter = SourceMeter::new(
            AudioSourceMeterSpec {
                reference_rate: 32_000,
                window_frames: 1,
                history_windows: 3,
            },
            48_000,
        )
        .unwrap();

        // 1.5 output frames per window. Window boundaries occur after
        // 2, 3, 5, and 6 frames and use independent stereo PCM16 means.
        for frame in [[-1., 1.], [0., 0.25], [0.5, -0.5], [0.25, 0.25]] {
            meter.record(frame);
            assert_eq!(meter.level(), [0; 2]);
        }
        meter.record([0.75, -0.5]);
        assert_eq!(meter.level(), [16_384, 20_479]);
        meter.record([0.25, 0.]);
        assert_eq!(meter.level(), [16_384, 16_384]);
    }
}
