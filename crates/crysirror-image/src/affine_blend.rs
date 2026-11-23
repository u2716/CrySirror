//! Two immutable images, centered affine geometry and stored-byte composition.
//! Geometry is snapped to1/256 pixels, scanline UVs advance in16.16 and linear
//! sampling truncates two7-bit mixtures. The destination's alpha is retained.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, rgb_lut::ConstantByteBlend};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteMipProfile {
    pub levels: u8,
    pub min_extent: u16,
    pub average_x: bool,
    pub repeat_last_row: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffineBlendByte128 {
    /// Offset from the destination's integer half-size.
    pub position: [i32; 2],
    pub scale: [f32; 2],
    pub rotation_turns: f32,
    /// Explicit angle profile, independent of the platform's pi constant.
    pub angle_pi: f64,
    pub center_offset: [f32; 2],
    pub odd_source_offset: [f32; 2],
    /// Independent RGBA /128 tint, followed by saturating RGB addition.
    pub multiply: [u8; 4],
    pub add: [u8; 3],
    pub blend: ConstantByteBlend,
    #[serde(default)]
    pub mip: Option<ByteMipProfile>,
}
#[derive(Clone, Copy)]
struct End {
    x: f32,
    u: f32,
    v: f32,
}
#[derive(Clone, Copy)]
struct Span {
    left: End,
    right: End,
}
impl AffineBlendByte128 {
    pub fn validate(&self) -> Result<()> {
        if let Some(mip) = &self.mip {
            ensure!(
                mip.levels <= 4 && mip.min_extent >= 2,
                "affine mip profile bounds"
            );
        }
        ensure!(
            self.position.iter().all(|v| v.unsigned_abs() <= 1_000_000),
            "affine position bounds"
        );
        ensure!(
            self.scale.iter().all(|v| v.is_finite() && v.abs() <= 4096.),
            "affine scale bounds"
        );
        ensure!(
            self.rotation_turns.is_finite() && self.rotation_turns.abs() <= 1_000_000.,
            "affine rotation bounds"
        );
        ensure!(
            self.angle_pi.is_finite() && (3.0..=3.2).contains(&self.angle_pi),
            "affine angle profile"
        );
        ensure!(
            self.center_offset
                .iter()
                .chain(&self.odd_source_offset)
                .all(|v| v.is_finite() && v.abs() <= 8192.),
            "affine center bounds"
        );
        Ok(())
    }
    pub fn auxiliary_bytes(size: [u32; 2]) -> usize {
        size[1] as usize * std::mem::size_of::<Span>()
    }
    fn quad(&self, destination: [u32; 2], source: [u32; 2], snap: bool) -> Result<[[f32; 2]; 4]> {
        let angle = f64::from(self.rotation_turns) * self.angle_pi * 2.;
        let (s, c) = (angle.sin() as f32, angle.cos() as f32);
        let center = std::array::from_fn::<_, 2, _>(|i| {
            let offset = if source[i] & 1 != 0 {
                self.odd_source_offset[i]
            } else {
                0.
            };
            let base = (f64::from(destination[i] / 2)
                + f64::from(self.position[i])
                + f64::from(self.center_offset[i])) as f32;
            (f64::from(base) + f64::from(offset)) as f32
        });
        let half = std::array::from_fn::<_, 2, _>(|i| {
            (f64::from(source[i]) * f64::from(self.scale[i]) * 0.5) as f32
        });
        let mut points = [[0.; 2]; 4];
        for (p, sign) in points
            .iter_mut()
            .zip([[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]])
        {
            let (x, y) = (half[0] * sign[0], half[1] * sign[1]);
            let rotated = [
                (f64::from(x) * f64::from(c) - f64::from(y) * f64::from(s)) as f32,
                (f64::from(x) * f64::from(s) + f64::from(y) * f64::from(c)) as f32,
            ];
            for i in 0..2 {
                let position = (f64::from(rotated[i]) + f64::from(center[i])) as f32;
                ensure!(
                    position.is_finite() && position.abs() <= 1_000_000.,
                    "affine derived vertex bounds"
                );
                // The original snap helper stores its multiply/add as f32
                //before applying its negative-coordinate floor convention.
                p[i] = if snap {
                    let snapped = (f64::from(position) * 256. + 0.5) as f32;
                    floor_native(f64::from(snapped)) as f32 / 256.
                } else {
                    position
                };
            }
        }
        Ok(points)
    }
    fn spans(
        &self,
        size: [u32; 2],
        source: [u32; 2],
        sampled: [u32; 2],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<Span>> {
        let points = self.quad(size, source, true)?;
        let edge = End {
            x: 1_000_001.,
            u: 0.,
            v: 0.,
        };
        let mut spans = vec![
            Span {
                left: edge,
                right: End {
                    x: -1_000_001.,
                    ..edge
                }
            };
            size[1] as usize
        ];
        //55A908 normalizes UVs, then539638 scales them back by the source
        //extent, with a single-precision boundary at each operation.
        let end = std::array::from_fn::<_, 2, _>(|i| {
            let v = source[i];
            let reciprocal = (1. / f64::from(v)) as f32;
            let unit = (f64::from(v) * f64::from(reciprocal)) as f32;
            unit * sampled[i] as f32
        });
        let uv = [[0., 0.], [end[0], 0.], [end[0], end[1]], [0., end[1]]];
        for i in 0..4 {
            check()?;
            let (mut a, mut b) = (i, (i + 1) % 4);
            if points[a][1] > points[b][1] {
                std::mem::swap(&mut a, &mut b);
            }
            let (low, high) = (points[a], points[b]);
            let height = (f64::from(high[1]) - f64::from(low[1])) as f32;
            if height <= 0. {
                continue;
            }
            let first = floor_native(f64::from(low[1]) + 0.99999237060546875).max(0);
            let last =
                floor_native(f64::from(high[1]) - 0.00000762939453125).min(i64::from(size[1]) - 1);
            if last < first {
                continue;
            }
            let slope = [
                ((f64::from(high[0]) - f64::from(low[0])) / f64::from(height)) as f32,
                ((f64::from(uv[b][0]) - f64::from(uv[a][0])) / f64::from(height)) as f32,
                ((f64::from(uv[b][1]) - f64::from(uv[a][1])) / f64::from(height)) as f32,
            ];
            let dy = (first as f64 - f64::from(low[1])).max(0.) as f32;
            let start = [low[0], uv[a][0], uv[a][1]];
            let mut sample = std::array::from_fn::<_, 3, _>(|j| {
                (f64::from(slope[j]) * f64::from(dy) + f64::from(start[j])) as f32
            });
            for row in first..=last {
                check()?;
                let span = &mut spans[row as usize];
                let point = End {
                    x: sample[0],
                    u: sample[1],
                    v: sample[2],
                };
                if point.x < span.left.x {
                    span.left = point;
                }
                if point.x > span.right.x {
                    span.right = point;
                }
                for j in 0..3 {
                    sample[j] = (f64::from(sample[j]) + f64::from(slope[j])) as f32;
                }
            }
        }
        Ok(spans)
    }
    fn mip_level(&self, destination: [u32; 2], source: [u32; 2]) -> Result<u8> {
        let Some(profile) = &self.mip else {
            return Ok(0);
        };
        let mut size = source;
        let mut available = 0;
        while available < profile.levels && size.iter().all(|v| *v > u32::from(profile.min_extent))
        {
            available += 1;
            size = size.map(|v| v / 2);
        }
        if available == 0 {
            return Ok(0);
        }
        let points = self.quad(destination, source, false)?;
        if (points[0][0] - points[2][0]).abs() > 10000.
            || (points[0][1] - points[2][1]).abs() > 10000.
            || (points[0][0] - points[1][0]).abs() > 10000.
            || (points[0][1] - points[1][1]).abs() > 10000.
        {
            return Ok(0);
        }
        let uv = [
            [0., 0.],
            [source[0] as f32, 0.],
            [source[0] as f32, source[1] as f32],
            [0., source[1] as f32],
        ];
        let mut lengths = [1f32; 2];
        for i in 0..4 {
            let j = (i + 1) % 4;
            for (at, quad) in [points, uv].iter().enumerate() {
                let dx = (f64::from(quad[i][0]) - f64::from(quad[j][0])) as f32;
                let dy = (f64::from(quad[i][1]) - f64::from(quad[j][1])) as f32;
                let square = (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)) as f32;
                lengths[at] = (f64::from(lengths[at]) + f64::from(square)) as f32;
            }
        }
        let denominator = (lengths[0] * 0.25).max(1.);
        let numerator = lengths[1] * 0.25;
        let ratio = numerator / denominator;
        let rho = f64::from(ratio).sqrt() as f32;
        let mut level = 0;
        let mut threshold = 3f32;
        while level < available && rho >= threshold {
            level += 1;
            threshold *= 2.;
        }
        Ok(level)
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
        for image in [destination, source] {
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "affine image pixel budget"
            );
        }
        check()?;
        let mut out = destination.clone();
        if self.multiply[3] == 0 || self.scale.contains(&0.) {
            return Ok(out);
        }
        let original_size = [source.width, source.height];
        let level = self.mip_level([out.width, out.height], original_size)?;
        let mut reduced = None;
        for _ in 0..level {
            let input = reduced.as_ref().unwrap_or(source);
            reduced = Some(half(input, self.mip.as_ref().unwrap(), check)?);
        }
        let source = reduced.as_ref().unwrap_or(source);
        let spans = self.spans(
            [out.width, out.height],
            original_size,
            [source.width, source.height],
            check,
        )?;
        let tint = self
            .multiply
            .map(|v| (u32::from(v) >> 1) + (u32::from(v) >> 7));
        for (row, span) in spans.iter().enumerate() {
            check()?;
            let width = (f64::from(span.right.x) - f64::from(span.left.x)) as f32;
            if width < 0.00000762939453125 {
                continue;
            }
            let first = floor_native(f64::from(span.left.x) + 0.99999237060546875).max(0);
            let last = floor_native(f64::from(span.right.x) - 0.00000762939453125)
                .min(i64::from(out.width) - 1);
            if last < first {
                continue;
            }
            let width = width.max(1.);
            let derivative = [
                ((f64::from(span.right.u) - f64::from(span.left.u)) / f64::from(width)) as f32,
                ((f64::from(span.right.v) - f64::from(span.left.v)) / f64::from(width)) as f32,
            ];
            let distance = (first as f64 - f64::from(span.left.x)).max(0.) as f32;
            let left = [span.left.u, span.left.v];
            let mut q = std::array::from_fn::<_, 2, _>(|i| {
                let start =
                    (f64::from(derivative[i]) * f64::from(distance) + f64::from(left[i])) as f32;
                floor_native(f64::from(start) * 65536. + 0.5)
            });
            let step = derivative.map(|v| floor_native(f64::from(v) * 65536. + 0.5));
            for x in first..=last {
                let mut pixel = sample(source, q);
                for c in 0..4 {
                    pixel[c] = ((u32::from(pixel[c]) * tint[c]) >> 7) as u8;
                }
                for c in 0..3 {
                    pixel[c] = pixel[c].saturating_add(self.add[c]);
                }
                let at = (row * out.width as usize + x as usize) * 4;
                for c in 0..3 {
                    out.rgba[at + c] = self.blend.channel(out.rgba[at + c], pixel[c], pixel[3]);
                }
                for i in 0..2 {
                    q[i] += step[i];
                }
            }
        }
        check()?;
        Ok(out)
    }
}
fn half(
    input: &ImageSurface,
    profile: &ByteMipProfile,
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    let size = [input.width.div_ceil(2), input.height.div_ceil(2)];
    let mut output = ImageSurface {
        width: size[0],
        height: size[1],
        rgba: vec![0; crate::rgba_len(size[0], size[1])?],
    };
    let tap = |x: u32, y: u32, c: usize| {
        input.rgba
            [((y.min(input.height - 1) * input.width + x.min(input.width - 1)) * 4) as usize + c]
    };
    for y in 0..size[1] {
        check()?;
        let bottom = if profile.repeat_last_row && y == size[1] - 1 {
            y * 2
        } else {
            y * 2 + 1
        };
        for x in 0..size[0] {
            for c in 0..4 {
                let horizontal = |row| {
                    let a = u16::from(tap(x * 2, row, c));
                    if profile.average_x {
                        (a + u16::from(tap(x * 2 + 1, row, c))) / 2
                    } else {
                        a
                    }
                };
                output.rgba[((y * size[0] + x) * 4) as usize + c] =
                    ((horizontal(y * 2) + horizontal(bottom)) / 2) as u8;
            }
        }
    }
    Ok(output)
}
fn floor_native(value: f64) -> i64 {
    if value < 0. {
        value.trunc() as i64 - 1
    } else {
        value.trunc() as i64
    }
}
fn sample(image: &ImageSurface, q: [i64; 2]) -> [u8; 4] {
    let [u, v] = q.map(|v| v - 32768);
    let (x, y) = (u >> 16, v >> 16);
    let (wx, wy) = (
        (((u & 65535) + 1) >> 9) as i32,
        (((v & 65535) + 1) >> 9) as i32,
    );
    let tap = |x: i64, y: i64| -> [u8; 4] {
        let x = x.clamp(0, i64::from(image.width) - 1);
        let y = y.clamp(0, i64::from(image.height) - 1);
        let at = ((y * i64::from(image.width) + x) * 4) as usize;
        image.rgba[at..at + 4].try_into().unwrap()
    };
    let mix = |a: [u8; 4], b: [u8; 4], w: i32| {
        std::array::from_fn(|i| {
            (i32::from(a[i]) + (((i32::from(b[i]) - i32::from(a[i])) * w) >> 7)) as u8
        })
    };
    mix(
        mix(tap(x, y), tap(x + 1, y), wx),
        mix(tap(x, y + 1), tap(x + 1, y + 1), wx),
        wy,
    )
}
