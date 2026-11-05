//! Saved integer pixel curtains for retained rows. Fonts, row layout, refresh
//! rate, input policy and clip publication are deliberately caller supplied.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelCurtain {
    widths: Vec<u32>,
    pixels_per_tick: u32,
    row: usize,
    visible: u32,
}
impl PixelCurtain {
    pub fn new(widths: Vec<u32>, pixels_per_tick: u32) -> Result<Self> {
        let value = Self {
            widths,
            pixels_per_tick,
            row: 0,
            visible: 0,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.widths.len() <= 4096 && self.widths.iter().all(|v| *v <= 1024 * 1024),
            "pixel curtain row budget"
        );
        ensure!(
            (1..=1024 * 1024).contains(&self.pixels_per_tick),
            "pixel curtain step"
        );
        ensure!(self.row <= self.widths.len(), "pixel curtain row cursor");
        if self.row == self.widths.len() {
            ensure!(self.visible == 0, "completed pixel curtain cursor");
        } else {
            ensure!(
                self.visible <= self.widths[self.row],
                "pixel curtain row extent"
            );
        }
        Ok(())
    }
    pub fn complete(&self) -> bool {
        self.row == self.widths.len()
    }
    pub fn finish(&mut self) {
        self.row = self.widths.len();
        self.visible = 0;
    }
    /// Each tick advances the current row only. Overflow is intentionally not
    /// carried to the next row; a zero-width row also consumes a tick.
    pub fn advance_ticks(&mut self, mut ticks: u64) -> Result<()> {
        self.validate()?;
        while ticks > 0 && !self.complete() {
            let remaining = self.widths[self.row] - self.visible;
            let needed = u64::from(remaining)
                .div_ceil(u64::from(self.pixels_per_tick))
                .max(1);
            if ticks >= needed {
                ticks -= needed;
                self.row += 1;
                self.visible = 0;
            } else {
                self.visible += u32::try_from(ticks * u64::from(self.pixels_per_tick))?;
                ticks = 0;
            }
        }
        Ok(())
    }
    pub fn visible_widths(&self) -> Vec<u32> {
        self.widths
            .iter()
            .enumerate()
            .map(|(n, width)| {
                if n < self.row {
                    *width
                } else if n == self.row {
                    self.visible
                } else {
                    0
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_discard_overflow_and_saved_state_continues_exactly() {
        let mut curtain = PixelCurtain::new(vec![11, 0, 25], 8).unwrap();
        curtain.advance_ticks(1).unwrap();
        assert_eq!(curtain.visible_widths(), [8, 0, 0]);
        curtain.advance_ticks(1).unwrap();
        assert_eq!(curtain.visible_widths(), [11, 0, 0]);
        let mut restored = curtain.clone();
        restored.validate().unwrap();
        restored.advance_ticks(2).unwrap();
        assert_eq!(restored.visible_widths(), [11, 0, 8]);
        restored.advance_ticks(u64::MAX).unwrap();
        assert!(restored.complete());
    }
    #[test]
    fn malformed_saved_cursor_fails_before_advancing() {
        let mut curtain = PixelCurtain::new(vec![3], 1).unwrap();
        curtain.visible = 4;
        assert!(curtain.advance_ticks(1).is_err());
        assert!(PixelCurtain::new(vec![1], 0).is_err());
        let mut empty = PixelCurtain::new(vec![], 1).unwrap();
        assert!(empty.complete());
        empty.finish();
        assert!(empty.validate().is_ok());
    }
}
