// SPDX-License-Identifier: MPL-2.0
//! Positioned byte-matte copies/products over immutable planes. Pair fast paths
//! belong to the declared arithmetic profile; odd tails still divide by256.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatteSpan {
    pub destination: [i32; 2],
    pub multiply: bool,
    /// Packed delta interpolation using the current row-prefix pair as its
    /// basis. The odd tail uses its own destination pixel instead.
    #[serde(default)]
    pub interpolate_row_prefix: bool,
}
impl MatteSpan {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !(self.multiply && self.interpolate_row_prefix),
            "conflicting matte arithmetic profiles"
        );
        ensure!(
            self.destination
                .iter()
                .all(|v| v.unsigned_abs() <= 1_000_000),
            "matte origin bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        source.validate()?;
        ensure!(
            [destination, source]
                .iter()
                .all(|i| u64::from(i.width) * u64::from(i.height) <= MAX_EFFECT_PIXELS),
            "matte pixel budget"
        );
        check()?;
        let mut out = destination.clone();
        let [x, y] = self.destination.map(i64::from);
        let left = x.max(0).min(i64::from(out.width));
        let right = (x + i64::from(source.width))
            .max(0)
            .min(i64::from(out.width));
        let top = y.max(0).min(i64::from(out.height));
        let bottom = (y + i64::from(source.height))
            .max(0)
            .min(i64::from(out.height));
        if right <= left || bottom <= top {
            return Ok(out);
        }
        for row in top..bottom {
            check()?;
            let begin = ((row * out.width as i64 + left) * 4) as usize;
            let from = (((row - y) * source.width as i64 + left - x) * 4) as usize;
            let len = ((right - left) * 4) as usize;
            if self.interpolate_row_prefix {
                let pair_end = len / 8 * 8;
                let row_start = (row * out.width as i64 * 4) as usize;
                for offset in (0..pair_end).step_by(8) {
                    let prefix: [u8; 8] = out.rgba[row_start..row_start + 8].try_into().unwrap();
                    let target = &mut out.rgba[begin + offset..begin + offset + 8];
                    let input = &source.rgba[from + offset..from + offset + 8];
                    let white = prefix[..3].iter().chain(&prefix[4..7]).all(|v| *v == 255);
                    if white {
                        target.copy_from_slice(input);
                    } else if prefix[..3].iter().chain(&prefix[4..7]).any(|v| *v != 0) {
                        for index in 0..8 {
                            let base = prefix[index];
                            let delta =
                                (i32::from(input[index]) - i32::from(base)) * i32::from(base);
                            target[index] = base.wrapping_add(((delta as u16) >> 8) as u8);
                        }
                    }
                }
                for index in pair_end..len {
                    let base = out.rgba[begin + index];
                    let delta =
                        (i32::from(source.rgba[from + index]) - i32::from(base)) * i32::from(base);
                    out.rgba[begin + index] = base.wrapping_add(((delta as u16) >> 8) as u8);
                }
                continue;
            }
            let d = &mut out.rgba[begin..begin + len];
            let s = &source.rgba[from..from + len];
            if !self.multiply {
                d.copy_from_slice(s);
                continue;
            }
            let pairs = len / 8 * 8;
            for (d, s) in d[..pairs]
                .chunks_exact_mut(8)
                .zip(s[..pairs].chunks_exact(8))
            {
                // The original packed path examines RGB, not the alpha byte.
                let first = d[..3].iter().all(|v| *v == 255);
                let second = d[4..7].iter().all(|v| *v == 255);
                if first && second {
                    d.copy_from_slice(s);
                } else if d[..3].iter().chain(&d[4..7]).any(|v| *v != 0) {
                    for (d, s) in d.iter_mut().zip(s) {
                        *d = (u16::from(*d) * u16::from(*s) >> 8) as u8;
                    }
                }
            }
            for (d, s) in d[pairs..].iter_mut().zip(&s[pairs..]) {
                *d = (u16::from(*d) * u16::from(*s) >> 8) as u8;
            }
        }
        check()?;
        Ok(out)
    }
}
