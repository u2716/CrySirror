// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Offset stored RGB through two independent linear coverage planes. Explicit
//! strides permit valid packed buffers whose row pitch differs from image width.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearMaskOffset {
    pub destination: [i32; 4],
    pub origins: [[i32; 2]; 2],
    pub strides: [i32; 2],
    pub add: [u8; 3],
    pub subtract: [u8; 3],
}
impl LinearMaskOffset {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.destination
                .iter()
                .chain(self.origins.iter().flatten())
                .chain(&self.strides)
                .all(|n| n.unsigned_abs() <= 1_000_000),
            "linear mask offset coordinates"
        );
        ensure!(
            self.destination[2] >= 0
                && self.destination[3] >= 0
                && self.destination[2] as u64 * self.destination[3] as u64 <= MAX_EFFECT_PIXELS * 4,
            "linear mask offset work budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, first, second] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "linear mask offset image budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        let [dx, dy, w, h] = self.destination.map(i64::from);
        let left = 0.max(-dx);
        let right = w.min(i64::from(destination.width) - dx);
        let top = 0.max(-dy);
        let bottom = h.min(i64::from(destination.height) - dy);
        let read = |image: &ImageSurface, index: usize, x: i64, y: i64| -> u16 {
            let linear = (i64::from(self.origins[index][1]) + y) * i64::from(self.strides[index])
                + i64::from(self.origins[index][0])
                + x;
            // Only formerly invalid native buffer reads become zero. A valid
            // linear address may cross the resource's nominal row boundary.
            usize::try_from(linear)
                .ok()
                .and_then(|p| p.checked_mul(4))
                .and_then(|p| p.checked_add(3))
                .and_then(|p| image.rgba.get(p))
                .copied()
                .map(u16::from)
                .unwrap_or(0)
        };
        for y in top..bottom {
            check()?;
            for x in left..right {
                if (x - left) % 4096 == 0 {
                    check()?;
                }
                let a = read(first, 0, x, y);
                let b = read(second, 1, x, y);
                if a == 0 || b == 0 {
                    continue;
                }
                let coverage = if a == 255 {
                    b
                } else if b == 255 {
                    a
                } else {
                    a * b >> 8
                };
                let at = (((dy + y) * i64::from(destination.width) + dx + x) * 4) as usize;
                for c in 0..3 {
                    let scale = |n: u8| {
                        if coverage == 255 {
                            u16::from(n)
                        } else {
                            u16::from(n) * coverage >> 8
                        }
                    };
                    output.rgba[at + c] = u16::from(output.rgba[at + c])
                        .saturating_add(scale(self.add[c]))
                        .saturating_sub(scale(self.subtract[c]))
                        .min(255) as u8;
                }
            }
        }
        check()?;
        Ok(output)
    }
}
