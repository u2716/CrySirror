//! Device-independent pointer displacement in explicit owner-supplied ticks.
use crate::{Error, Result};
/// Incremental float32 interpolation with a per-axis one-step snap window.
/// A caller advances it only at its admitted input boundary; it samples no
/// clock, owns no OS pointer, and emits logical coordinates for a host to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct SteppedPointerMotion {
    position: [f32; 2],
    target: [f32; 2],
    increment: [f32; 2],
    active: bool,
}
impl SteppedPointerMotion {
    pub fn new(start: [i32; 2], target: [i32; 2], steps: u16) -> Result<Self> {
        if steps == 0
            || start
                .iter()
                .chain(&target)
                .any(|v| v.unsigned_abs() > 1_000_000)
        {
            return Err(Error::Invalid("stepped pointer motion domain"));
        }
        let position = start.map(|v| v as f32);
        let target = target.map(|v| v as f32);
        Ok(Self {
            position,
            target,
            increment: core::array::from_fn(|axis| {
                (target[axis] - position[axis]) / f32::from(steps)
            }),
            active: true,
        })
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn advance(&mut self) -> Option<[i32; 2]> {
        if !self.active {
            return None;
        }
        for axis in 0..2 {
            self.position[axis] += self.increment[axis];
            let tolerance = self.increment[axis].abs();
            if self.position[axis] >= self.target[axis] - tolerance
                && self.position[axis] <= self.target[axis] + tolerance
            {
                self.position[axis] = self.target[axis];
            }
        }
        self.active = self.position != self.target;
        Some(self.position.map(|v| v as i32))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_owner_tick_adds_float_delta_and_snaps_within_one_step() {
        let mut motion = SteppedPointerMotion::new([0, 100], [100, 0], 10).unwrap();
        assert_eq!(motion.advance(), Some([10, 90]));
        for _ in 0..7 {
            motion.advance();
        }
        assert!(motion.active());
        assert_eq!(motion.advance(), Some([100, 0]));
        assert!(!motion.active());
        assert_eq!(motion.advance(), None);
    }
    #[test]
    fn signed_fractional_steps_truncate_coordinates_and_zero_axes_finish() {
        let mut motion = SteppedPointerMotion::new([-10, 7], [0, 7], 3).unwrap();
        assert_eq!(motion.advance(), Some([-6, 7]));
        assert_eq!(motion.advance(), Some([0, 7]));
        assert!(!motion.active());
        assert!(SteppedPointerMotion::new([0; 2], [0; 2], 0).is_err());
        let mut stationary = SteppedPointerMotion::new([2; 2], [2; 2], 10).unwrap();
        assert_eq!(stationary.advance(), Some([2; 2]));
        assert!(!stationary.active());
    }
}
