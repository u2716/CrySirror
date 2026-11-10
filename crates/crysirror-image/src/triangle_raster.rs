// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Device-free projective byte triangles. Coverage includes all three edges;
//! attributes use homogeneous inverse coefficients at integer pixel positions.
//! Sampling and colour conversion retain the stored-float2^23 bias operation.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrianglePaint {
    /// Texture coordinates are source pixels; nearest samples clamp to edges.
    Texture {
        uv: [[f32; 2]; 3],
        source_alpha: bool,
        #[serde(default)]
        feedback: bool,
    },
    /// Signed corner colours interpolate before byte conversion and clamping.
    Color { rgba: [[i32; 4]; 3] },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriangleRaster {
    /// Screen x/y and the homogeneous scale of each corner, not a depth buffer.
    pub positions: [[f32; 3]; 3],
    pub paint: TrianglePaint,
}
fn native_integer(value: f64) -> i32 {
    ((value + 8388608.) as f32)
        .to_bits()
        .wrapping_sub(0x4b000000) as i32
}
impl TriangleRaster {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.positions
                .iter()
                .flatten()
                .all(|n| n.is_finite() && n.abs() <= 1_000_000.),
            "triangle finite position budget"
        );
        if let TrianglePaint::Texture { uv, .. } = &self.paint {
            ensure!(
                uv.iter()
                    .flatten()
                    .all(|n| n.is_finite() && n.abs() <= 1_000_000.),
                "triangle finite texture coordinate budget"
            );
        }
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
        check()?;
        ensure!(
            u64::from(destination.width) * u64::from(destination.height) <= MAX_EFFECT_PIXELS,
            "triangle destination pixel budget"
        );
        if matches!(self.paint, TrianglePaint::Texture { feedback: true, .. }) {
            ensure!(
                (destination.width, destination.height) == (source.width, source.height),
                "triangle feedback image sizes differ"
            );
        }
        let mut out = destination.clone();
        let Some(coefficients) = self.coefficients() else {
            return Ok(out);
        };
        let [min_x, min_y, max_x, max_y] = self.bounds();
        let left = min_x.max(0);
        let top = min_y.max(0);
        let right = max_x.min(destination.width as i32 - 1);
        let bottom = max_y.min(destination.height as i32 - 1);
        if left > right || top > bottom {
            return Ok(out);
        }
        let opaque = matches!(
            self.paint,
            TrianglePaint::Texture {
                source_alpha: false,
                ..
            }
        );
        for y in top..=bottom {
            check()?;
            // Two native row products are stored to float before visiting x;
            // the third stays in the intermediate precision of the expression.
            let rows = [
                f64::from((f64::from(coefficients[0][1]) * f64::from(y)) as f32),
                f64::from((f64::from(coefficients[1][1]) * f64::from(y)) as f32),
                f64::from(coefficients[2][1]) * f64::from(y),
            ];
            for x in left..=right {
                if (x - left) & 255 == 0 {
                    check()?;
                }
                let mut b = [0f64; 3];
                for i in 0..3 {
                    let xpart = f64::from(coefficients[i][0]) * f64::from(x);
                    b[i] = f64::from(
                        (if opaque {
                            xpart + f64::from(coefficients[i][2]) + rows[i]
                        } else {
                            xpart + rows[i] + f64::from(coefficients[i][2])
                        }) as f32,
                    );
                }
                if b.iter().any(|n| *n < 0.) {
                    continue;
                }
                let sum = (b[2] + b[1]) + b[0];
                // No meaningful sample exists at a zero homogeneous pole.
                // Define that otherwise undefined conversion as no coverage.
                if sum <= 0. || !sum.is_finite() {
                    continue;
                }
                let reciprocal = 1. / sum;
                for n in &mut b {
                    *n *= reciprocal;
                }
                let pixel = match &self.paint {
                    TrianglePaint::Texture {
                        uv,
                        source_alpha,
                        feedback,
                    } => {
                        let mut pos = [0i32; 2];
                        for axis in 0..2 {
                            pos[axis] = native_integer(
                                (b[0] * f64::from(uv[0][axis]) + b[1] * f64::from(uv[1][axis]))
                                    + b[2] * f64::from(uv[2][axis]),
                            );
                        }
                        let sx = pos[0].clamp(0, source.width as i32 - 1) as usize;
                        let sy = pos[1].clamp(0, source.height as i32 - 1) as usize;
                        let at = (sy * source.width as usize + sx) * 4;
                        let samples = if *feedback { &out.rgba } else { &source.rgba };
                        [
                            samples[at],
                            samples[at + 1],
                            samples[at + 2],
                            if *source_alpha { samples[at + 3] } else { 255 },
                        ]
                    }
                    TrianglePaint::Color { rgba } => std::array::from_fn(|channel| {
                        let a = rgba.map(|color| f64::from(color[channel] as f32));
                        native_integer((a[2] * b[2] + b[1] * a[1]) + b[0] * a[0]).clamp(0, 255)
                            as u8
                    }),
                };
                let at = (y as usize * destination.width as usize + x as usize) * 4;
                out.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(out)
    }
    fn bounds(&self) -> [i32; 4] {
        let min = |axis| {
            self.positions
                .iter()
                .map(|p| p[axis])
                .fold(f32::INFINITY, f32::min)
        };
        let max = |axis| {
            self.positions
                .iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        [
            native_integer(f64::from(min(0))),
            native_integer(f64::from(min(1))),
            native_integer(f64::from(max(0))),
            native_integer(f64::from(max(1))),
        ]
    }
    fn coefficients(&self) -> Option<[[f32; 3]; 3]> {
        let p = self.positions.map(|p| p.map(f64::from));
        let [x0, y0, z0] = p[0];
        let [x1, y1, z1] = p[1];
        let [x2, y2, z2] = p[2];
        let x0 = x0 * z0;
        let y0 = y0 * z0;
        let x1 = x1 * z1;
        let y1 = y1 * z1;
        let x2 = f64::from((x2 * z2) as f32);
        let y2 = f64::from((y2 * z2) as f32);
        let minor = |v: f64| f64::from(v as f32);
        let b1 = [
            minor(y2 * z0 - y0 * z2),
            minor(x0 * z2 - x2 * z0),
            minor(y0 * x2 - y2 * x0),
        ];
        let b2 = [
            minor(y0 * z1 - y1 * z0),
            minor(x1 * z0 - x0 * z1),
            x0 * y1 - y0 * x1,
        ];
        let determinant = (y2 * b2[1] + x2 * b2[0]) + b2[2] * z2;
        if determinant.abs() < 0.0001 || !determinant.is_finite() {
            return None;
        }
        let b0 = [
            minor(z2 * y1 - y2 * z1),
            minor(x2 * z1 - x1 * z2),
            minor(y2 * x1 - y1 * x2),
        ];
        let inverse = 1. / determinant;
        Some([b0, b1, b2].map(|row| row.map(|v| (v * inverse) as f32)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn positions() -> [[f32; 3]; 3] {
        [[0., 0., 1.], [2., 0., 1.], [0., 2., 1.]]
    }
    #[test]
    fn inclusive_edges_copy_hidden_rgb_and_alpha_without_source_over() {
        let d = ImageSurface::filled(3, 3, [10, 20, 30, 70]).unwrap();
        let s = ImageSurface::filled(1, 1, [40, 50, 60, 0]).unwrap();
        for alpha in [false, true] {
            let spec = TriangleRaster {
                positions: positions(),
                paint: TrianglePaint::Texture {
                    uv: [[0.; 2]; 3],
                    source_alpha: alpha,
                    feedback: false,
                },
            };
            let out = spec.apply(&d, &s, &|| Ok(())).unwrap();
            for (x, y) in [(0, 0), (2, 0), (0, 2), (1, 1)] {
                let at = (y * 3 + x) * 4;
                assert_eq!(
                    &out.rgba[at..at + 4],
                    &[40, 50, 60, if alpha { 0 } else { 255 }]
                );
            }
            assert_eq!(&out.rgba[32..36], &[10, 20, 30, 70]);
        }
    }
    #[test]
    fn projective_texture_weights_clamp_and_color_bytes_are_direct() {
        let d = ImageSurface::filled(5, 5, [10, 20, 30, 70]).unwrap();
        let s = ImageSurface {
            width: 5,
            height: 1,
            rgba: (0..5).flat_map(|i| [i * 40, 0, 0, 255]).collect(),
        };
        let mut spec = TriangleRaster {
            positions: [[0., 0., 1.], [4., 0., 2.], [0., 4., 1.]],
            paint: TrianglePaint::Texture {
                uv: [[0., 0.], [4., 0.], [0., 0.]],
                source_alpha: true,
                feedback: false,
            },
        };
        let out = spec.apply(&d, &s, &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[8..12], &[40, 0, 0, 255]);
        spec.paint = TrianglePaint::Color {
            rgba: [[300, -1, 50, 128]; 3],
        };
        let out = spec.apply(&d, &s, &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[..4], &[255, 0, 50, 128]);
        spec.positions = [[0., 0., 1.]; 3];
        assert_eq!(spec.apply(&d, &s, &|| Ok(())).unwrap().rgba, d.rgba);
        assert_eq!(native_integer(-1.), -2);
        assert_eq!(native_integer(0.5), 0);
        assert!(spec.apply(&d, &s, &|| anyhow::bail!("cancelled")).is_err());
    }
    #[test]
    fn feedback_visits_the_live_destination_in_scan_order_and_winding_keeps_coverage() {
        let d = ImageSurface {
            width: 3,
            height: 3,
            rgba: (0..9).flat_map(|i| [i as u8, 0, 0, 255]).collect(),
        };
        let mut spec = TriangleRaster {
            positions: positions(),
            paint: TrianglePaint::Texture {
                uv: [[-1., 0.], [1., 0.], [-1., 2.]],
                source_alpha: true,
                feedback: true,
            },
        };
        let out = spec.apply(&d, &d, &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[8..12], &[0, 0, 0, 255]);
        spec.paint = TrianglePaint::Color {
            rgba: [[90, 80, 70, 60]; 3],
        };
        let a = spec.apply(&d, &d, &|| Ok(())).unwrap();
        spec.positions.swap(1, 2);
        let b = spec.apply(&d, &d, &|| Ok(())).unwrap();
        assert_eq!(a.rgba, b.rgba);
    }
}
