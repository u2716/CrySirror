// SPDX-License-Identifier: MPL-2.0
//! Checked stored-byte drawing on immutable image pairs. Geometry sampling is
//! independent of native sprite handles, windows, text and output devices.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ByteRasterMath {
    /// Independent saturated stored-RGB addition or subtraction; no coverage
    /// modulation and no alpha writes.
    SaturatedRgb { subtract: bool },
    /// Scale stored coverage by weight/256. Zero coverage skips, 255 copies
    /// RGB; partial contributions are summed before one /256 truncation.
    /// Destination alpha is retained independently of the coverage source.
    AlphaWeightedRgb256 { weight: u16 },
    /// Scale source coverage by weight/256, then floor the two RGB products
    /// separately. Only source alpha255 plus weight256 copies RGB directly.
    /// Destination alpha is retained; weight is0..=256.
    AlphaWeightedSeparateRgb256 { weight: u16 },
    /// Piecewise stored-mask blend with explicit low-word arithmetic. Mask0
    /// skips; mask255 uses(weight,256-weight). Partial masks use the high byte
    /// of the wrapping mask*weight product and its255 complement. All RGB
    /// products/sums wrap to16bits before /256; destination alpha is retained.
    MaskPiecewiseWrapping256 { weight: u16 },
    CopyRgb,
    /// Copy RGB when the stored source coverage reaches the inclusive threshold.
    AlphaThresholdRgb {
        threshold: u8,
    },
    CopyAlpha,
    CopyRgba,
    /// Two separate floor products /255; source coverage is ignored.
    ConstantRgb255 {
        opacity: u8,
    },
    /// Two separate floor products /255, using stored source coverage.
    AlphaRgb255,
}
impl ByteRasterMath {
    pub(crate) fn validate(self) -> Result<()> {
        if let Self::AlphaWeightedRgb256 { weight } | Self::AlphaWeightedSeparateRgb256 { weight } = self {
            ensure!(weight <= 256, "byte coverage weight bounds");
        }
        Ok(())
    }
    pub(crate) fn pixel(self, d: &mut [u8], s: &[u8]) {
        match self {
            Self::SaturatedRgb { subtract } => {
                for c in 0..3 {
                    d[c] = if subtract { d[c].saturating_sub(s[c]) }
                        else { d[c].saturating_add(s[c]) };
                }
            }

            Self::AlphaWeightedSeparateRgb256 { weight } => {
                if s[3] == 255 && weight == 256 {
                    d[..3].copy_from_slice(&s[..3]);
                } else {
                    let a = u32::from(s[3]) * u32::from(weight) >> 8;
                    for c in 0..3 {
                        d[c] = ((u32::from(s[c])*a >> 8)
                            + (u32::from(d[c])*(256-a) >> 8)) as u8;
                    }
                }
            }
            Self::MaskPiecewiseWrapping256 { weight } => {
                if s[3] == 0 { return; }
                let (a,b) = if s[3] == 255 {
                    (weight, 256u16.wrapping_sub(weight))
                } else {
                    let a = u16::from(s[3]).wrapping_mul(weight) >> 8;
                    (a, 255-a)
                };
                for c in 0..3 {
                    d[c] = (u16::from(s[c]).wrapping_mul(a)
                        .wrapping_add(u16::from(d[c]).wrapping_mul(b)) >> 8) as u8;
                }
            }
            Self::AlphaWeightedRgb256 { weight } => {
                let a = u32::from(s[3]) * u32::from(weight) >> 8;
                if a == 255 { d[..3].copy_from_slice(&s[..3]); }
                else if a != 0 {
                    for c in 0..3 {
                        d[c] = ((u32::from(s[c])*a + u32::from(d[c])*(256-a)) >> 8) as u8;
                    }
                }
            }
            Self::CopyRgb => d[..3].copy_from_slice(&s[..3]),
            Self::AlphaThresholdRgb { threshold } => {
                if s[3] >= threshold {
                    d[..3].copy_from_slice(&s[..3]);
                }
            }
            Self::CopyAlpha => d[3] = s[3],
            Self::CopyRgba => d.copy_from_slice(s),
            Self::ConstantRgb255 { opacity } => {
                for c in 0..3 {
                    d[c] = ((u32::from(s[c]) * u32::from(opacity)) / 255
                        + (u32::from(d[c]) * u32::from(255 - opacity)) / 255)
                        as u8;
                }
            }
            Self::AlphaRgb255 => Self::ConstantRgb255 { opacity: s[3] }.pixel(d, s),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ByteRaster {
    RowTintBlend{spec:crate::row_tint_blend::RowTintBlend},
    MaskedBackdropShift{spec:crate::masked_backdrop_shift::MaskedBackdropShift},
    /// Clipped box sums with an explicit packed-byte normalisation profile.
    NativeBoxBlur { spec: crate::native_box_blur::NativeBoxBlur },
    FixedScanlines { spec: crate::fixed_scanlines::FixedScanlines },
    ClippedScanlineSample { spec: crate::clipped_scanline_sample::ClippedScanlineSample },
    RowBlit { spec: crate::row_blit::RowBlit },
    MaskedAddSubtract256 {spec:crate::masked_add::MaskedAddSubtract256},
    RandomWipe {
        spec: crate::integer_wipe::random::RandomWipe,
    },
    PolygonFragments {
        spec: crate::integer_wipe::fragments::PolygonFragments,
    },
    IntegerWipe {
        spec: crate::integer_wipe::IntegerWipe,
    },
    AxisProjection {
        spec: crate::axis_projection::AxisProjection,
    },
    Triangle {
        spec: crate::triangle_raster::TriangleRaster,
    },
    /// Inclusive line with sequential integer edge clipping, followed by a
    /// truncating8.8 DDA. The major axis is ordered ascending before drawing.
    Line {
        points: [[i32; 2]; 2],
        color: [u8; 4],
        write_mask: [bool; 4],
    },
    /// Inverse centred rotation with per-row truncating16.16 starts and steps.
    /// Invalid samples clear coverage and retain destination hidden RGB.
    RotateScale {
        source: [u32; 4],
        radians: f32,
        scale: f32,
        math: ByteRasterMath,
        opaque: bool,
    },
    /// Scalar inverse rotation with centres, byte256 bilinear weights and
    /// repeated floating increments. All-zero source coverage retains hidden RGB.
    BilinearRotation {
        destination_center: [f32; 2],
        source_center: [f32; 2],
        degrees: f32,
        scale: f32,
    },
    /// Repeat a source window. `first_run`, when present, selects an absolute
    /// first source x and its pixel count before ordinary window repetition.
    /// Clipping the destination retains the original row/column phase.
    Tile {
        source: [u32; 4],
        destination: [i32; 4],
        phase: [i32; 2],
        first_run: Option<[u32; 2]>,
        math: ByteRasterMath,
    },
}
impl ByteRaster {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::FixedScanlines {spec}=>spec.validate()?,
            Self::ClippedScanlineSample {spec}=>spec.validate()?,
            Self::RowBlit { spec } => spec.validate()?,
            Self::NativeBoxBlur { spec } => spec.validate()?,
            Self::RowTintBlend{spec}=>spec.validate()?,
            Self::MaskedBackdropShift{spec}=>spec.validate()?,
            Self::MaskedAddSubtract256{spec}=>spec.validate()?,
            Self::RandomWipe { spec } => spec.validate()?,
            Self::PolygonFragments { spec } => spec.validate()?,
            Self::IntegerWipe { spec } => spec.validate()?,
            Self::AxisProjection { spec } => spec.validate()?,
            Self::Triangle { spec } => spec.validate()?,
            Self::Line { points, .. } => ensure!(
                points
                    .iter()
                    .flatten()
                    .all(|n| n.unsigned_abs() <= 1_000_000),
                "byte line coordinate budget"
            ),
            Self::RotateScale {
                source,
                radians,
                scale,
                ..
            } => {
                ensure!(
                    source[2] > 0 && source[3] > 0 && source.iter().all(|n| *n <= 16384),
                    "byte affine crop bounds"
                );
                ensure!(
                    radians.is_finite()
                        && radians.abs() <= 1_000_000.
                        && scale.is_finite()
                        && scale.abs() <= 1_000_000.,
                    "byte affine finite parameters"
                );
            }
            Self::BilinearRotation {
                destination_center,
                source_center,
                degrees,
                scale,
            } => {
                ensure!(
                    destination_center
                        .iter()
                        .chain(source_center)
                        .chain([degrees, scale])
                        .all(|n| n.is_finite() && n.abs() <= 1_000_000.),
                    "byte bilinear finite parameters"
                );
            }
            Self::Tile {
                source,
                destination,
                phase,
                first_run,
                ..
            } => {
                ensure!(
                    source[2] > 0 && source[3] > 0 && source.iter().all(|n| *n <= 16384),
                    "byte tile source bounds"
                );
                ensure!(
                    destination
                        .iter()
                        .chain(phase)
                        .all(|n| n.unsigned_abs() <= 1_000_000)
                        && destination[2] >= 0
                        && destination[3] >= 0,
                    "byte tile destination bounds"
                );
                ensure!(
                    u64::try_from(destination[2])? * u64::try_from(destination[3])?
                        <= MAX_EFFECT_PIXELS * 4,
                    "byte tile work budget"
                );
                if let Some([x, n]) = first_run {
                    ensure!(
                        *n > 0 && *n <= source[2] && *x <= 16384,
                        "byte tile initial run"
                    );
                }
            }
        }
        if let Self::RotateScale { math, .. } | Self::Tile { math, .. } = self {
            math.validate()?;
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
        if let Self::FixedScanlines{spec}=self{return spec.apply(destination,source,check);}
        if let Self::ClippedScanlineSample{spec}=self{return spec.apply(destination,source,check);}
        if let Self::RowBlit { spec } = self { return spec.apply(destination,source,check); }
        if let Self::NativeBoxBlur { spec } = self { return spec.apply(destination,source,check); }
        if let Self::RowTintBlend{spec}=self{return spec.apply(destination,source,check);}
        if let Self::MaskedBackdropShift{spec}=self{return spec.apply(destination,source,check);}
        if let Self::MaskedAddSubtract256{spec}=self{return spec.apply(destination,source,check);}
        if let Self::RandomWipe { spec } = self {
            return spec.apply(destination, source, check);
        }
        if let Self::PolygonFragments { spec } = self {
            return spec.apply(destination, source, check);
        }
        if let Self::IntegerWipe { spec } = self {
            return spec.apply(destination, source, check);
        }
        if let Self::AxisProjection { spec } = self {
            return spec.apply(destination, source, check);
        }
        if let Self::Triangle { spec } = self {
            return spec.apply(destination, source, check);
        }
        destination.validate()?;
        source.validate()?;
        for image in [destination, source] {
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "byte raster pixel budget"
            );
        }
        check()?;
        let mut output = destination.clone();
        match *self {
            Self::FixedScanlines{..}|Self::ClippedScanlineSample{..}=>unreachable!(),
            Self::RowBlit { .. } | Self::NativeBoxBlur {..} | Self::RowTintBlend{..} | Self::MaskedBackdropShift{..} => unreachable!(),
            Self::MaskedAddSubtract256{..}=>unreachable!(),
            Self::RandomWipe { .. }
            | Self::PolygonFragments { .. }
            | Self::IntegerWipe { .. }
            | Self::AxisProjection { .. }
            | Self::Triangle { .. } => unreachable!(),
            Self::Line {
                mut points,
                color,
                write_mask,
            } => {
                if clip_line(&mut points, [destination.width, destination.height]) {
                    let [[mut x0, mut y0], [mut x1, mut y1]] = points.map(|p| p.map(i64::from));
                    let dx = x1 - x0;
                    let dy = y1 - y0;
                    let mut put = |x: i64, y: i64| {
                        let at = ((y * i64::from(output.width) + x) * 4) as usize;
                        for c in 0..4 {
                            if write_mask[c] {
                                output.rgba[at + c] = color[c];
                            }
                        }
                    };
                    if dx.abs() > dy.abs() {
                        if x0 > x1 {
                            std::mem::swap(&mut x0, &mut x1);
                            std::mem::swap(&mut y0, &mut y1);
                        }
                        let step = if dx == 0 { 0 } else { dy * 256 / dx };
                        let mut phase = y0 * 256;
                        for x in x0..=x1 {
                            check()?;
                            put(x, phase >> 8);
                            phase += step;
                        }
                    } else {
                        if y0 > y1 {
                            std::mem::swap(&mut x0, &mut x1);
                            std::mem::swap(&mut y0, &mut y1);
                        }
                        let step = if dy == 0 { 0 } else { dx * 256 / dy };
                        let mut phase = x0 * 256;
                        for y in y0..=y1 {
                            check()?;
                            put(phase >> 8, y);
                            phase += step;
                        }
                    }
                }
            }
            Self::RotateScale {
                source: crop,
                radians,
                scale,
                math,
                opaque,
            } => {
                ensure!(
                    u64::from(crop[0]) + u64::from(crop[2]) <= u64::from(source.width)
                        && u64::from(crop[1]) + u64::from(crop[3]) <= u64::from(source.height),
                    "byte affine crop outside image"
                );
                // This profile quantizes each row start separately. A single
                // floating affine or a global integer start is not equivalent.
                let c = f64::from(radians).cos();
                let s = f64::from(radians).sin();
                let mag = f64::from(scale);
                let step = [
                    ((1. / mag * c * 65536.).trunc() as i64 as i32) as i64,
                    ((1. / mag * s * 65536.).trunc() as i64 as i32) as i64,
                ];
                let half = [
                    f64::from(output.width) * 0.5,
                    f64::from(output.height) * 0.5,
                ];
                for y in 0..output.height {
                    check()?;
                    let yy = f64::from(y) - half[1];
                    let mut u = (((-c * half[0] - s * yy) / mag + f64::from(crop[2] / 2)) * 65536.)
                        .trunc() as i64 as i32 as i64;
                    let mut v = (((c * yy - half[0] * s) / mag + f64::from(crop[3] / 2)) * 65536.)
                        .trunc() as i64 as i32 as i64;
                    for x in 0..output.width {
                        let to = ((y * output.width + x) * 4) as usize;
                        output.rgba[to + 3] = 0;
                        if scale >= 0.01
                            && (0..i64::from(crop[2])).contains(&(u >> 16))
                            && (0..i64::from(crop[3])).contains(&(v >> 16))
                        {
                            let from = (((i64::from(crop[1]) + (v >> 16))
                                * i64::from(source.width)
                                + i64::from(crop[0])
                                + (u >> 16))
                                * 4) as usize;
                            math.pixel(&mut output.rgba[to..to + 4], &source.rgba[from..from + 4]);
                            if opaque {
                                output.rgba[to + 3] = 255;
                            }
                        }
                        u = (u as i32).wrapping_add(step[0] as i32) as i64;
                        v = (v as i32).wrapping_add(step[1] as i32) as i64;
                    }
                }
            }
            Self::BilinearRotation {
                destination_center,
                source_center,
                degrees,
                scale,
            } => {
                if scale >= 0.001 {
                    let inv = f64::from(1f32 / scale);
                    let radians = -f64::from(degrees) * f64::from(0.017453292f32);
                    let c = f64::from(radians.cos() as f32);
                    let s = f64::from(radians.sin() as f32);
                    let left = -(inv * f64::from(destination_center[0]));
                    let horizontal = (c * left) as f32;
                    let vertical = s * left;
                    let du = (c * inv) as f32;
                    let dv = (-s * inv) as f32;
                    let du256 = (f64::from(du) * 256.) as f32;
                    let dv256 = s * inv * -256.;
                    let mix = |a: u8, b: u8, t: i32| {
                        i32::from(a) + ((t * (i32::from(b) - i32::from(a))) >> 8)
                    };
                    for y in 0..output.height {
                        check()?;
                        let yy = (f64::from(destination_center[1]) - f64::from(y)) * inv;
                        let stored_yy = yy as f32;
                        let mut u = f64::from(horizontal) - yy * s + f64::from(source_center[0]);
                        let mut v =
                            f64::from(source_center[1]) - (f64::from(stored_yy) * c + vertical);
                        let mut u256 = u * 256.;
                        let mut v256 = v * 256.;
                        for x in 0..output.width {
                            let to = ((y * output.width + x) * 4) as usize;
                            let xx = u.trunc() as i64;
                            let yy = v.trunc() as i64;
                            if (0..i64::from(source.width)).contains(&xx)
                                && (0..i64::from(source.height)).contains(&yy)
                            {
                                let x1 = (xx + 1).min(i64::from(source.width) - 1);
                                let y1 = (yy + 1).min(i64::from(source.height) - 1);
                                let at = |x: i64, y: i64| {
                                    ((y * i64::from(source.width) + x) * 4) as usize
                                };
                                let indices = [at(xx, yy), at(x1, yy), at(xx, y1), at(x1, y1)];
                                if indices.iter().any(|i| source.rgba[i + 3] != 0) {
                                    let fx = (u256.trunc() as i64 as u8) as i32;
                                    let fy = (v256.trunc() as i64 as u8) as i32;
                                    for ch in 0..4 {
                                        let upper = mix(
                                            source.rgba[indices[0] + ch],
                                            source.rgba[indices[1] + ch],
                                            fx,
                                        );
                                        let lower = mix(
                                            source.rgba[indices[2] + ch],
                                            source.rgba[indices[3] + ch],
                                            fx,
                                        );
                                        output.rgba[to + ch] =
                                            (upper + ((fy * (lower - upper)) >> 8)) as u8;
                                    }
                                } else {
                                    output.rgba[to + 3] = 0;
                                }
                            } else {
                                output.rgba[to + 3] = 0;
                            }
                            u += f64::from(du);
                            v += f64::from(dv);
                            u256 += f64::from(du256);
                            v256 += dv256;
                        }
                    }
                }
            }
            Self::Tile {
                source: crop,
                destination: rect,
                phase,
                first_run,
                math,
            } => {
                ensure!(
                    u64::from(crop[0]) + u64::from(crop[2]) <= u64::from(source.width)
                        && u64::from(crop[1]) + u64::from(crop[3]) <= u64::from(source.height),
                    "byte tile source outside image"
                );
                if let Some([x, n]) = first_run {
                    ensure!(
                        u64::from(x) + u64::from(n) <= u64::from(source.width),
                        "byte tile initial source outside image"
                    );
                }
                let [dx, dy, w, h] = rect.map(i64::from);
                let left = dx.max(0);
                let top = dy.max(0);
                let right = (dx + w).min(i64::from(output.width));
                let bottom = (dy + h).min(i64::from(output.height));
                for y in top..bottom {
                    check()?;
                    let sy = i64::from(crop[1])
                        + (y - dy + i64::from(phase[1])).rem_euclid(i64::from(crop[3]));
                    for x in left..right {
                        let at = x - dx;
                        let sx = if let Some([first, count]) = first_run {
                            if at < i64::from(count) {
                                i64::from(first) + at
                            } else {
                                i64::from(crop[0])
                                    + (at - i64::from(count)).rem_euclid(i64::from(crop[2]))
                            }
                        } else {
                            i64::from(crop[0])
                                + (at + i64::from(phase[0])).rem_euclid(i64::from(crop[2]))
                        };
                        let from = ((sy * i64::from(source.width) + sx) * 4) as usize;
                        let to = ((y * i64::from(output.width) + x) * 4) as usize;
                        math.pixel(&mut output.rgba[to..to + 4], &source.rgba[from..from + 4]);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}
// Ordered endpoint clipping matters for integer division rounding. Avoid
// clipping a completed DDA or reordering endpoints before these operations.
fn clip_line(points: &mut [[i32; 2]; 2], size: [u32; 2]) -> bool {
    let [[mut x0, mut y0], [mut x1, mut y1]] = points.map(|p| p.map(i64::from));
    let [w, h] = size.map(i64::from);
    if (x0 < 0 && x1 < 0) || (x0 >= w && x1 >= w) || (y0 < 0 && y1 < 0) || (y0 >= h && y1 >= h) {
        return false;
    }
    if x1 != x0 {
        if x0 < 0 {
            y0 -= x0 * (y1 - y0) / (x1 - x0);
            x0 = 0;
        }
        if x1 < 0 {
            y1 = y0 - x0 * (y1 - y0) / (x1 - x0);
            x1 = 0;
        }
        if x0 >= w {
            y0 += (y1 - y0) * (w - x0 - 1) / (x1 - x0);
            x0 = w - 1;
        }
        if x1 >= w {
            y1 = y0 + (y1 - y0) * (w - x0 - 1) / (x1 - x0);
            x1 = w - 1;
        }
    }
    if y1 != y0 {
        if y0 < 0 {
            x0 -= y0 * (x1 - x0) / (y1 - y0);
            y0 = 0;
        }
        if y1 < 0 {
            x1 = x0 - y0 * (x1 - x0) / (y1 - y0);
            y1 = 0;
        }
        if y0 >= h {
            x0 += (x1 - x0) * (h - y0 - 1) / (y1 - y0);
            y0 = h - 1;
        }
        if y1 >= h {
            x1 = x0 + (x1 - x0) * (h - y0 - 1) / (y1 - y0);
            y1 = h - 1;
        }
    }
    if x0 < 0 || x1 < 0 || x0 >= w || x1 >= w || y0 < 0 || y1 < 0 || y0 >= h || y1 >= h {
        return false;
    }
    *points = [[x0 as i32, y0 as i32], [x1 as i32, y1 as i32]];
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image(w: u32, h: u32, p: [u8; 4]) -> ImageSurface {
        ImageSurface {
            width: w,
            height: h,
            rgba: p.repeat((w * h) as usize),
        }
    }
    #[test]
    fn line_clipping_tile_phase_and_channel_masks() {
        let base = image(4, 4, [10, 20, 30, 40]);
        let source = ImageSurface {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 0, 11, 12, 13, 128],
        };
        let line = ByteRaster::Line {
            points: [[-1, -1], [4, 4]],
            color: [70, 80, 90, 255],
            write_mask: [true, true, true, false],
        }
        .apply(&base, &source, &|| Ok(()))
        .unwrap();
        for y in 0..4 {
            for x in 0..4 {
                let at = ((y * 4 + x) * 4) as usize;
                assert_eq!(
                    &line.rgba[at..at + 4],
                    if x == y {
                        &[70, 80, 90, 40]
                    } else {
                        &[10, 20, 30, 40]
                    }
                );
            }
        }
        let tiled = ByteRaster::Tile {
            source: [0, 0, 2, 1],
            destination: [-1, 0, 5, 1],
            phase: [0; 2],
            first_run: None,
            math: ByteRasterMath::AlphaThresholdRgb { threshold: 128 },
        }
        .apply(&base, &source, &|| Ok(()))
        .unwrap();
        assert_eq!(
            &tiled.rgba[..16],
            &[
                11, 12, 13, 40, 10, 20, 30, 40, 11, 12, 13, 40, 10, 20, 30, 40
            ]
        );
        for x0 in [-9, -1, 0, 2, 9] {
            for y0 in [-9, -1, 0, 2, 9] {
                for x1 in [-9, -1, 0, 2, 9] {
                    for y1 in [-9, -1, 0, 2, 9] {
                        ByteRaster::Line {
                            points: [[x0, y0], [x1, y1]],
                            color: [0; 4],
                            write_mask: [true; 4],
                        }
                        .apply(&base, &source, &|| Ok(()))
                        .unwrap();
                    }
                }
            }
        }
    }
    #[test]
    fn centered_affine_retains_hidden_bytes_and_clears_outside_alpha() {
        let base = image(2, 2, [90, 80, 70, 60]);
        let source = ImageSurface {
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3, 0, 11, 12, 13, 128, 21, 22, 23, 64, 31, 32, 33, 255],
        };
        let mut spec = ByteRaster::RotateScale {
            source: [0, 0, 2, 2],
            radians: 0.,
            scale: 1.,
            math: ByteRasterMath::CopyRgba,
            opaque: false,
        };
        assert_eq!(
            spec.apply(&base, &source, &|| Ok(())).unwrap().rgba,
            source.rgba
        );
        if let ByteRaster::RotateScale { scale, .. } = &mut spec {
            *scale = 0.;
        }
        assert_eq!(
            spec.apply(&base, &source, &|| Ok(())).unwrap().rgba,
            [90, 80, 70, 0].repeat(4)
        );
        assert!(
            spec.apply(&base, &source, &|| anyhow::bail!("cancel"))
                .is_err()
        );
    }
}
