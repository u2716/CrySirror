//! Constant-space stepped logarithmic envelopes on the mixer output clock.
use crysirror_core::service::{AudioGainEnvelopeInfo as Info, AudioGainEnvelopeSpec as Spec};
#[derive(Clone, Copy)]
pub(crate) struct Envelope {
    spec: Spec,
    count: u32,
    phase: u64,
    rate: u32,
    db100: i32,
    advanced: bool,
    pub gain: f32,
}
impl Envelope {
    pub fn new(spec: Spec, rate: u32, current_db100: i32, current_gain: f32) -> Self {
        let db100 = if spec.apply_initial {
            spec.db100_at(spec.elapsed_steps)
        } else {
            current_db100
        };
        Self {
            spec,
            count: spec.elapsed_steps,
            phase: u64::from(spec.phase_ms) * u64::from(rate),
            rate,
            db100,
            advanced: false,
            gain: if spec.apply_initial {
                gain(db100)
            } else {
                current_gain
            },
        }
    }
    pub fn info(self) -> Info {
        Info {
            spec: self.spec,
            elapsed_steps: self.count,
            phase_ms: (self.phase / u64::from(self.rate)) as u32,
            current_db100: self.db100,
            active: self.count < self.spec.steps || (!self.spec.apply_initial && !self.advanced),
        }
    }
    pub fn stop(self) -> bool {
        self.spec.stop_at_end
            && self.count >= self.spec.steps
            && (self.spec.apply_initial || self.advanced)
    }
    /// Audio frames are rational milliseconds. No per-frame division or
    /// logarithm is needed until crossing an actual step boundary.
    pub fn advance(&mut self) {
        self.phase += 1000;
        let interval = u64::from(self.spec.interval_ms) * u64::from(self.rate);
        if self.phase >= interval {
            self.advanced = true;
            let steps = self.phase / interval;
            self.phase %= interval;
            self.count = self
                .count
                .saturating_add(steps.min(u64::from(u32::MAX)) as u32)
                .min(self.spec.steps);
            let db100 = self.spec.db100_at(self.count);
            if db100 != self.db100 {
                self.db100 = db100;
                self.gain = gain(db100);
            }
        }
    }
}
fn gain(db100: i32) -> f32 {
    if db100 <= -10000 {
        0.
    } else {
        10f64.powf(f64::from(db100) / 2000.) as f32
    }
}
