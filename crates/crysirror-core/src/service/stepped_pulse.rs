//! Clock-free float32 triangular pulses for explicitly stepped visual owners.
use crate::{Error, Result};
/// A sequence of triangular pulses. The peak transition occurs after an
/// increment, so float32 accumulation and threshold timing remain observable.
#[derive(Clone, Debug, PartialEq)]
pub struct SteppedPulse {
    value: f32,
    increment: f32,
    ascending: bool,
    remaining: u32,
}
impl SteppedPulse {
    pub fn new(rise_ticks: u32, pulses: u32) -> Result<Self> {
        if rise_ticks == 0 || pulses == 0 {
            return Err(Error::Invalid("stepped pulse domain"));
        }
        Ok(Self {
            value: 0.,
            increment: 1. / rise_ticks as f32,
            ascending: true,
            remaining: pulses,
        })
    }
    pub fn active(&self) -> bool {
        self.remaining != 0
    }
    ///Each admitted tick adds the same float32 delta. The returned value is
    ///clamped for presentation; the internal accumulator retains overshoot.
    pub fn advance(&mut self) -> Option<f32> {
        self.advance_by(1, false).expect("one pulse step")
    }
    ///Apply repeated float32 additions before testing the phase boundary.
    ///Forcing a boundary preserves the current peak value on the rising half.
    pub fn advance_by(&mut self, steps: u8, force_boundary: bool) -> Result<Option<f32>> {
        if steps == 0 {
            return Err(Error::Invalid("stepped pulse zero rate"));
        }
        if !self.active() {
            return Ok(None);
        }
        for _ in 0..steps {
            if self.ascending {
                self.value += self.increment;
            } else {
                self.value -= self.increment;
            }
        }
        if self.ascending {
            if self.value >= 1. || force_boundary {
                self.ascending = false;
            }
        } else if self.value <= 0. || force_boundary {
            self.remaining -= 1;
            self.value = 0.;
            self.ascending = true;
        }
        Ok(Some(self.value.clamp(0., 1.)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_ticks_rise_and_float32_fall_are_not_normalized_tweens() {
        let mut p = SteppedPulse::new(5, 1).unwrap();
        let values = (0..5)
            .map(|_| p.advance().unwrap())
            .collect::<alloc::vec::Vec<_>>();
        assert_eq!(values, alloc::vec![0.2, 0.4, 0.6, 0.8, 1.]);
        assert_eq!(p.advance(), Some(0.8));
        for _ in 0..5 {
            p.advance();
        }
        assert!(!p.active());
        assert_eq!(p.advance(), None);
    }
    #[test]
    fn repeated_pulses_start_from_zero_on_the_next_tick() {
        let mut p = SteppedPulse::new(2, 2).unwrap();
        assert_eq!(
            (0..8)
                .map(|_| p.advance().unwrap())
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![0.5, 1., 0.5, 0., 0.5, 1., 0.5, 0.]
        );
        assert!(!p.active());
        assert!(SteppedPulse::new(0, 1).is_err());
    }
}

/// An explicitly clocked signed float32 ramp. Negative durations deliberately
/// move away from the completion boundary instead of being normalized.
#[derive(Clone, Debug, PartialEq)]
pub struct SteppedRamp {
    value: f32,
    target: f32,
    increment: f32,
    rising: bool,
    active: bool,
}
impl SteppedRamp {
    pub fn new(start: f32, target: f32, ticks: i32) -> Result<Self> {
        if !start.is_finite() || !target.is_finite() || ticks == 0 {
            return Err(Error::Invalid("stepped ramp domain"));
        }
        Ok(Self {
            value: start,
            target,
            increment: ((f64::from(target) - f64::from(start)) / f64::from(ticks)) as f32,
            rising: target >= start,
            active: true,
        })
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn advance_by(&mut self, steps: u8, finish: bool) -> Result<Option<f32>> {
        if steps == 0 {
            return Err(Error::Invalid("stepped ramp zero rate"));
        }
        if !self.active {
            return Ok(None);
        }
        for _ in 0..steps {
            self.value += self.increment;
        }
        self.active = !(finish
            || if self.rising {
                self.value >= self.target
            } else {
                self.value <= self.target
            });
        Ok(Some(self.value))
    }
}
#[cfg(test)]
mod ramp_tests {
    use super::*;
    #[test]
    fn signed_duration_and_double_steps_preserve_accumulator() {
        let mut r = SteppedRamp::new(-255., 255., 4).unwrap();
        assert_eq!(r.advance_by(2, false).unwrap(), Some(0.));
        assert_eq!(r.advance_by(2, false).unwrap(), Some(255.));
        assert!(!r.active());
        let mut r = SteppedRamp::new(-255., 255., -4).unwrap();
        assert_eq!(r.advance_by(1, false).unwrap(), Some(-382.5));
        assert!(r.active());
        assert!(SteppedRamp::new(0., 1., 0).is_err());
    }
}
