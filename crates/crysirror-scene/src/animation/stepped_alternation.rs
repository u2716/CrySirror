//! Alternating integer offsets over a Q16 frame counter. Envelope tables use
//! sequential binary64 progress accumulation; offset sampling precedes clock
//! advancement, and the toggle interval comparison is strictly greater-than.
use crate::layer::Easing;
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SteppedAlternation {
    samples: Arc<[[i32; 2]]>,
    frame_ms: u32,
    interval_ms: u32,
    phase: u32,
    last_ms: u32,
    last_toggle_ms: u32,
    sign: i32,
    offset: [i32; 2],
    pub complete: bool,
}
impl SteppedAlternation {
    pub fn new(
        start: [i32; 2],
        end: [i32; 2],
        frames: u32,
        frame_ms: u32,
        interval_ms: u32,
        easing: Easing,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        ensure!(
            (1..=65535).contains(&frames) && (1..=1000).contains(&frame_ms),
            "stepped alternation frame bounds"
        );
        easing.validate()?;
        check()?;
        let mut samples = Vec::new();
        samples.try_reserve_exact(frames as usize + 1)?;
        let step = 1. / f64::from(frames);
        let mut progress = 0.;
        for index in 0..=frames {
            if index & 255 == 0 {
                check()?;
            }
            let eased = easing.apply_f64(progress);
            let mut value = [0; 2];
            for axis in 0..2 {
                let delta = i64::from(end[axis]) - i64::from(start[axis]);
                let magnitude = (delta.unsigned_abs() as f64 * eased).trunc();
                let result = i64::from(start[axis])
                    + if delta < 0 {
                        -(magnitude as i64)
                    } else {
                        magnitude as i64
                    };
                value[axis] = i32::try_from(result)?;
            }
            samples.push(value);
            progress += step;
        }
        check()?;
        Ok(Self {
            samples: Arc::from(samples),
            frame_ms,
            interval_ms,
            phase: 1,
            last_ms: 0,
            last_toggle_ms: 0,
            sign: 1,
            offset: [0; 2],
            complete: false,
        })
    }
    pub fn restart(&mut self, now_ms: u32) {
        self.phase = 1;
        self.last_ms = now_ms;
        self.last_toggle_ms = now_ms;
        self.sign = 1;
        self.offset = [0; 2];
        self.complete = false;
    }
    pub fn resident_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<[i32; 2]>()
    }
    pub fn advance(&mut self, now_ms: u32, speed: f64) -> Result<Option<[i32; 2]>> {
        ensure!(
            speed.is_finite() && (0. ..=1024.).contains(&speed),
            "stepped alternation speed"
        );
        if self.complete {
            return Ok(None);
        }
        let index = (self.phase >> 16) as usize;
        if index >= self.samples.len() - 1 {
            self.complete = true;
            self.offset = [0; 2];
            return Ok(Some(self.offset));
        }
        let elapsed = now_ms.wrapping_sub(self.last_ms);
        self.last_ms = now_ms;
        // Preserve the native 32-bit shift-before-division arithmetic.
        let scaled = (f64::from(elapsed) * speed).trunc() as u64 as u32;
        self.phase = self
            .phase
            .wrapping_add(scaled.wrapping_shl(16) / self.frame_ms);
        if now_ms.wrapping_sub(self.last_toggle_ms) <= self.interval_ms {
            return Ok(None);
        }
        self.offset = [
            self.samples[index][0]
                .checked_mul(self.sign)
                .context("alternating x offset overflow")?,
            self.samples[index][1]
                .checked_mul(self.sign)
                .context("alternating y offset overflow")?,
        ];
        self.sign = -self.sign;
        self.last_toggle_ms = now_ms;
        Ok(Some(self.offset))
    }
}
