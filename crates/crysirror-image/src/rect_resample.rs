//! In-place-extent rectangle resampling over an immutable source. Coordinates
//! use10 fractional bits and byte mixtures use the two native7-bit passes.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RectGeometry {
    Rectangle { bounds: [i32; 4] },
    CenteredDestination { center: [i32; 2], scale: [f32; 2] },
    SourcePivot { pivot: [i32; 2], scale: [f32; 2] },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectResample1024 {
    pub geometry: RectGeometry,
    /// Clear the old canvas when the clipped destination is inset farther
    /// than these left/top/right/bottom margins; otherwise retain old pixels.
    pub retained_border: [u16; 4],
}
impl RectResample1024 {
    pub fn validate(&self) -> Result<()> {
        let check = |point: &[i32]| point.iter().all(|v| v.unsigned_abs() <= 1_000_000);
        match &self.geometry {
            RectGeometry::Rectangle { bounds } => {
                ensure!(check(bounds), "resample rectangle bounds")
            }
            RectGeometry::CenteredDestination { center, scale }
            | RectGeometry::SourcePivot {
                pivot: center,
                scale,
            } => {
                ensure!(check(center), "resample centre bounds");
                ensure!(
                    scale.iter().all(|v| v.is_finite() && v.abs() <= 4096.),
                    "resample scale bounds"
                );
            }
        }
        Ok(())
    }
    fn bounds(&self, size: [u32; 2]) -> Result<[i64; 4]> {
        let mut bounds = [0i64; 4];
        match self.geometry {
            RectGeometry::Rectangle { bounds: value } => bounds = value.map(i64::from),
            RectGeometry::CenteredDestination { center, scale } => {
                for axis in 0..2 {
                    let start = i64::from(center[axis])
                        + ((-f64::from(size[axis]) / 2. * f64::from(scale[axis])) as f32).round()
                            as i64;
                    bounds[axis] = start;
                    bounds[axis + 2] = start
                        + ((f64::from(size[axis]) * f64::from(scale[axis])) as f32).round() as i64;
                }
            }
            RectGeometry::SourcePivot { pivot, scale } => {
                for axis in 0..2 {
                    let center = size[axis] as f32 / 2.;
                    let start = -(pivot[axis] as f32);
                    let end = (f64::from(size[axis]) - f64::from(pivot[axis])) as f32;
                    bounds[axis] = ((f64::from(start) * f64::from(scale[axis]) + f64::from(center))
                        as f32)
                        .round() as i64;
                    bounds[axis + 2] = ((f64::from(end) * f64::from(scale[axis])
                        + f64::from(center)) as f32)
                        .round() as i64;
                }
            }
        }
        ensure!(
            bounds.iter().all(|v| v.unsigned_abs() <= 1_000_000),
            "derived resample rectangle bounds"
        );
        for axis in 0..2 {
            if bounds[axis] > bounds[axis + 2] {
                bounds.swap(axis, axis + 2);
            }
        }
        Ok(bounds)
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "rectangle resample pixel budget"
        );
        check()?;
        let size = [i64::from(input.width), i64::from(input.height)];
        let bounds = self.bounds([input.width, input.height])?;
        let mut destination = [0; 4];
        let mut source = [0; 4];
        for axis in 0..2 {
            let (start, end) = (bounds[axis], bounds[axis + 2]);
            if end <= start {
                return Ok(input.clone());
            }
            let (low, high) = (start.max(0), end.min(size[axis]));
            if high <= low {
                return Ok(input.clone());
            }
            destination[axis] = low;
            destination[axis + 2] = high;
            source[axis] = size[axis] * (low - start) / (end - start);
            source[axis + 2] = size[axis] * (high - start) / (end - start);
            if source[axis + 2] <= source[axis] {
                return Ok(input.clone());
            }
        }
        let mut out = input.clone();
        let border = self.retained_border.map(i64::from);
        let clear = destination[0] > border[0]
            || destination[1] > border[1]
            || destination[2] < size[0] - border[2]
            || destination[3] < size[1] - border[3];
        if clear {
            for row in out.rgba.chunks_exact_mut(input.width as usize * 4) {
                check()?;
                row.fill(0);
            }
        }
        let step = std::array::from_fn::<_, 2, _>(|i| {
            ((source[i + 2] - source[i]) << 10) / (destination[i + 2] - destination[i])
        });
        let phase_weight = |coordinate: i64| {
            let p = (coordinate & 1023) as u32;
            (p >> 3) + (p >> 9)
        };
        let mix = |a: [u8; 4], b: [u8; 4], weight: u32| {
            std::array::from_fn(|c| {
                ((u32::from(a[c]) * (128 - weight) + u32::from(b[c]) * weight) >> 7) as u8
            })
        };
        let tap = |x: i64, y: i64| -> [u8; 4] {
            let x = x.clamp(0, size[0] - 1);
            let y = y.clamp(0, size[1] - 1);
            let at = ((y * size[0] + x) * 4) as usize;
            input.rgba[at..at + 4].try_into().unwrap()
        };
        for y in destination[1]..destination[3] {
            check()?;
            let qy = (source[1] << 10) + (y - destination[1]) * step[1];
            let sy = qy >> 10;
            let wy = phase_weight(qy);
            for x in destination[0]..destination[2] {
                let qx = (source[0] << 10) + (x - destination[0]) * step[0];
                let sx = qx >> 10;
                let wx = phase_weight(qx);
                let pixel = mix(
                    mix(tap(sx, sy), tap(sx + 1, sy), wx),
                    mix(tap(sx, sy + 1), tap(sx + 1, sy + 1), wx),
                    wy,
                );
                let at = ((y * size[0] + x) * 4) as usize;
                out.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(out)
    }
}
