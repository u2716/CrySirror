// SPDX-License-Identifier: MPL-2.0
//! Vertical gradient streaks stored directly (not blended) into a canvas, one
//! per start point: pixel `(x, y+k)` becomes `color` with alpha `ramp[k]`.
//! This is the N-System Rain drop footprint (438E90); stores are clipped.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreakField {
    pub streaks: Vec<[i32; 2]>,
    pub ramp: Vec<u8>,
    pub color: [u8; 3],
}
impl StreakField {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.streaks.len() <= 4096
                && (1..=1024).contains(&self.ramp.len())
                && self.streaks.iter().all(|p| p.iter().all(|v| v.unsigned_abs() <= 1 << 20)),
            "streak field bounds"
        );
        Ok(())
    }
    pub fn apply(&self, input: &ImageSurface, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        let mut out = input.clone();
        let (w, h) = (i64::from(input.width), i64::from(input.height));
        for (n, &[x, y]) in self.streaks.iter().enumerate() {
            if n % 64 == 0 {
                check()?;
            }
            let x = i64::from(x);
            if x < 0 || x >= w {
                continue;
            }
            for (k, &alpha) in self.ramp.iter().enumerate() {
                let py = i64::from(y) + k as i64;
                if py < 0 || py >= h {
                    continue;
                }
                let at = ((py * w + x) * 4) as usize;
                out.rgba[at..at + 3].copy_from_slice(&self.color);
                out.rgba[at + 3] = alpha;
            }
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streak_is_clipped_and_overwrites() {
        let img = ImageSurface { width: 2, height: 3, rgba: vec![9; 24] };
        let s = StreakField { streaks: vec![[1, -1]], ramp: vec![10, 20, 30], color: [1, 2, 3] };
        let out = s.apply(&img, &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[4..8], &[1, 2, 3, 20]);
        assert_eq!(&out.rgba[12..16], &[1, 2, 3, 30]);
        assert_eq!(&out.rgba[0..4], &[9, 9, 9, 9]);
    }
}
