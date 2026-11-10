// SPDX-License-Identifier: MPL-2.0
//! Stored-byte trapezoidal projection with integer edge interpolation and
//! separately quantized16.16 cross-axis sampling. No scene or native handles.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionAnchor {
    Center,
    Start,
    End,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisProjection {
    pub source: [u32; 4],
    /// false projects columns (rotation around Y); true projects rows.
    pub rows: bool,
    pub anchor: ProjectionAnchor,
    pub degrees: f32,
    pub scale: f32,
    pub source_alpha: bool,
}
struct Geometry {
    projected: i64,
    start: i64,
    end: i64,
    visible: bool,
    back: bool,
}
impl AxisProjection {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source.iter().all(|n| *n <= 16384) && self.source[2] > 0 && self.source[3] > 0,
            "axis projection crop bounds"
        );
        ensure!(
            self.degrees.is_finite()
                && self.degrees.abs() <= 360000.
                && self.scale.is_finite()
                && (0.0..=64.).contains(&self.scale),
            "axis projection parameter budget"
        );
        Ok(())
    }
    fn geometry(&self) -> Result<Geometry> {
        self.validate()?;
        let degree = if self.anchor == ProjectionAnchor::End {
            -self.degrees
        } else {
            self.degrees
        };
        let angle = f64::from(f64::from(degree).rem_euclid(360.) as f32);
        let factor = f64::from(0.017453292f32);
        let radians = angle * factor;
        let major = self.source[if self.rows { 3 } else { 2 }];
        let cross = self.source[if self.rows { 2 } else { 3 }];
        let scale = f64::from(self.scale);
        let projected = (radians.cos() * f64::from(major) * scale).trunc() as i64;
        let (mut start, mut end) = if self.anchor == ProjectionAnchor::Center {
            let mut total = f64::from(cross) * scale * 2.;
            if angle < 180. && !self.source_alpha {
                total = f64::from(total as f32);
            }
            let small = (total
                / (((if angle < 180. {
                    if self.source_alpha {
                        f64::from(radians as f32)
                    } else {
                        radians
                    }
                } else {
                    (angle - 180.) * factor
                })
                .sin())
                    * 0.5
                    + 2.))
                .trunc() as i64;
            let large = (total - small as f64).trunc() as i64;
            if angle < 180. {
                (large, small)
            } else {
                (small, large)
            }
        } else {
            let base = f64::from(cross) * scale;
            let fixed =
                if !self.source_alpha && (self.anchor == ProjectionAnchor::Start || self.rows) {
                    f64::from(base as f32).trunc() as i64
                } else {
                    base.trunc() as i64
                };
            let sine_angle =
                if self.source_alpha && (self.anchor == ProjectionAnchor::Start || self.rows) {
                    f64::from(radians as f32)
                } else {
                    radians
                };
            let perspective = (base * 2. / (sine_angle.sin() + 2.)).trunc() as i64;
            if self.anchor == ProjectionAnchor::Start {
                (fixed, perspective)
            } else {
                (perspective, fixed)
            }
        };
        if projected < 0 {
            std::mem::swap(&mut start, &mut end);
        }
        Ok(Geometry {
            projected: projected.abs(),
            start,
            end,
            visible: self.anchor == ProjectionAnchor::Center || !(90.0..270.).contains(&angle),
            back: projected < 0,
        })
    }
    /// Select a retained front/back input before submitting immutable worker work.
    pub fn uses_back(&self) -> Result<bool> {
        Ok(self.geometry()?.back)
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        let geometry = self.geometry()?;
        destination.validate()?;
        source.validate()?;
        for image in [destination, source] {
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "axis projection pixel budget"
            );
        }
        ensure!(
            u64::from(self.source[0]) + u64::from(self.source[2]) <= u64::from(source.width)
                && u64::from(self.source[1]) + u64::from(self.source[3])
                    <= u64::from(source.height),
            "axis projection crop outside source"
        );
        let major = if self.rows {
            destination.height
        } else {
            destination.width
        } as i64;
        let cross = if self.rows {
            destination.width
        } else {
            destination.height
        } as i64;
        let source_major = i64::from(self.source[if self.rows { 3 } else { 2 }]);
        let source_cross = i64::from(self.source[if self.rows { 2 } else { 3 }]);
        // Native row kernels centre/end against the crop's height. Column kernels
        // use output width. They are distinct when crop and output dimensions differ.
        let anchor_extent = if self.rows { source_major } else { major };
        let offset = match self.anchor {
            ProjectionAnchor::Center => (anchor_extent - geometry.projected) / 2,
            ProjectionAnchor::Start => 0,
            ProjectionAnchor::End => anchor_extent - geometry.projected,
        };
        let mut output = destination.clone();
        check()?;
        if !geometry.visible || geometry.projected == 0 {
            for row in output.rgba.chunks_mut(output.width as usize * 4) {
                check()?;
                for p in row.chunks_exact_mut(4) {
                    p[3] = 0;
                }
            }
            return Ok(output);
        }
        let index = |m: i64, c: i64, w: u32| {
            if self.rows {
                ((m * i64::from(w) + c) * 4) as usize
            } else {
                ((c * i64::from(w) + m) * 4) as usize
            }
        };
        for m in 0..major {
            check()?;
            if m < offset || m >= offset + geometry.projected {
                // Major-axis margins always clear RGB and coverage, including alpha copies.
                for c in 0..cross {
                    let at = index(m, c, output.width);
                    output.rgba[at..at + 4].fill(0);
                }
                continue;
            }
            let relative = m - offset;
            let raw =
                geometry.start + (geometry.end - geometry.start) * relative / geometry.projected;
            if raw <= 0 {
                continue;
            }
            let span = raw & !1;
            let source_m = source_major * relative / geometry.projected;
            let (top, count, mut phase) = if span <= cross {
                ((cross - span) / 2, span, 0)
            } else {
                // Original over-height/width branch writes source_cross pixels, not
                // destination cross extent. Reject its out-of-buffer native profile.
                ensure!(
                    source_cross <= cross,
                    "axis projection native oversized write extent"
                );
                (0, source_cross, 0)
            };
            if span > cross {
                phase = q16_ratio(source_cross * ((span - cross) / 2), span);
            }
            let step = if span > 0 {
                q16_ratio(source_cross, span)
            } else {
                0
            };
            for c in 0..cross {
                let at = index(m, c, output.width);
                if c >= top && c < top + count {
                    let source_c = phase >> 16;
                    phase += step;
                    ensure!(
                        (0..source_major).contains(&source_m)
                            && (0..source_cross).contains(&source_c),
                        "axis projection sample outside crop"
                    );
                    let [sx, sy] = if self.rows {
                        [
                            i64::from(self.source[0]) + source_c,
                            i64::from(self.source[1]) + source_m,
                        ]
                    } else {
                        [
                            i64::from(self.source[0]) + source_m,
                            i64::from(self.source[1]) + source_c,
                        ]
                    };
                    let from = ((sy * i64::from(source.width) + sx) * 4) as usize;
                    output.rgba[at..at + 3].copy_from_slice(&source.rgba[from..from + 3]);
                    output.rgba[at + 3] = if self.source_alpha {
                        source.rgba[from + 3]
                    } else {
                        255
                    };
                } else if span <= cross {
                    if self.source_alpha {
                        output.rgba[at + 3] = 0;
                    } else {
                        output.rgba[at..at + 4].fill(0);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}
fn q16_ratio(n: i64, d: i64) -> i64 {
    let value = n as f64 / d as f64;
    let whole = value.round_ties_even();
    (whole as i64) * 65536 + ((value - whole) * 65536.).trunc() as i64
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axes_preserve_alpha_margins_and_back_selection() {
        let source = ImageSurface {
            width: 4,
            height: 4,
            rgba: [1, 2, 3, 64].repeat(16),
        };
        let base = ImageSurface {
            width: 4,
            height: 4,
            rgba: [90, 80, 70, 60].repeat(16),
        };
        for rows in [false, true] {
            for anchor in [
                ProjectionAnchor::Center,
                ProjectionAnchor::Start,
                ProjectionAnchor::End,
            ] {
                let mut spec = AxisProjection {
                    source: [0, 0, 4, 4],
                    rows,
                    anchor,
                    degrees: 0.,
                    scale: 1.,
                    source_alpha: true,
                };
                assert_eq!(
                    spec.apply(&base, &source, &|| Ok(())).unwrap().rgba,
                    source.rgba
                );
                spec.degrees = 180.;
                if anchor == ProjectionAnchor::Center {
                    assert!(spec.uses_back().unwrap());
                } else {
                    assert_eq!(
                        spec.apply(&base, &source, &|| Ok(())).unwrap().rgba,
                        [90, 80, 70, 0].repeat(16)
                    );
                }
            }
        }
        let spec = AxisProjection {
            source: [0, 0, 4, 4],
            rows: false,
            anchor: ProjectionAnchor::Center,
            degrees: 30.,
            scale: 1.,
            source_alpha: false,
        };
        let result = spec.apply(&base, &source, &|| Ok(())).unwrap();
        for y in 0..4 {
            assert_eq!(&result.rgba[((y * 4 + 3) * 4)..((y * 4 + 4) * 4)], &[0; 4]);
        }
    }
}
