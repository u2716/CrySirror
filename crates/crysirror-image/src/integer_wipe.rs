// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Integer two-image reveals and translated panels. Parameters describe the
//! geometry; the caller owns clocks, asset selection and transition policy.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
pub mod fragments;
pub mod random;
pub mod scale;
pub mod scanline;
pub mod shapes;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum IntegerWipeGeometry {
    Scale {
        spec: scale::ScaleWipe,
    },
    Shape {
        spec: shapes::ShapeWipe,
    },
    Scanline {
        spec: scanline::ScanlineWipe,
    },
    /// Groups of equal bars begin in sequence; coverage grows by one texel in
    /// each bar per stage. `rounded_groups` permits a clipped final group.
    StaggeredBars {
        rows: bool,
        bar: u16,
        repeat: u16,
        rounded_groups: bool,
    },
    /// Alternating rows reveal from opposite ends, using strict comparisons
    /// against a descending pair counter, rather than a normalized gradient.
    InterleavedRows,
    /// Expand a circle whose maximum radius is the half-diagonal rounded up.
    Circular {
        reverse: bool,
    },
    /// Expand a circular centre using packed source strips, retaining the
    /// old image's outer strips as a continuous source sequence.
    PackedCircular,
    /// Sweep a sampled edge across the old image, then uncover the new image
    /// behind that moving edge. Sampling uses the adjacent texel, not a blur.
    EdgeSweep,
    /// Saturating old + scaled-new, followed by saturating new + scaled-old.
    AdditiveCross,
    /// Independently delayed strips roll through a periodic old/new pair;
    /// each strip uses a squared countdown and an odd turn multiplier.
    StripRoll {
        columns: bool,
        strips: u8,
        turns: [u8; 5],
    },
    /// A translated panel with three ballistic bounce phases. The trailing
    /// strip can retain its screen-relative samples or restart at source row 0.
    BouncingPanel {
        trailing_restarts: bool,
    },
    /// Clockwise rotating two-interval reveal with rounded scanline cuts.
    Clock {
        pi: f64,
    },
    /// A diagonal seam: x < clamp(start-y,0,width) retains the old image.
    /// Translating the old strip takes its right edge instead of the left.
    Diagonal {
        translate_old: bool,
    },
    /// Translate the new panel over the stationary old image. Directions are
    /// up, right, down, left; reverse exchanges images and time before drawing.
    Panel {
        direction: u8,
        reverse: bool,
    },
    /// Two halves translate from opposite edges around a stationary centre.
    SplitPanels {
        columns: bool,
        reverse: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerWipe {
    pub geometry: IntegerWipeGeometry,
    pub count: u32,
    pub duration: u32,
}
impl IntegerWipe {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.duration <= 864000 && self.count <= 864000,
            "integer wipe clock budget"
        );
        match self.geometry {
            IntegerWipeGeometry::Scale { spec } => spec.validate()?,
            IntegerWipeGeometry::Scanline { spec } => spec.validate()?,
            IntegerWipeGeometry::StaggeredBars { bar, repeat, .. } => ensure!(
                (2..=1024).contains(&bar) && (1..=64).contains(&repeat),
                "integer wipe bar budget"
            ),
            IntegerWipeGeometry::Panel { direction, .. } => {
                ensure!(direction < 4, "integer wipe panel direction")
            }
            IntegerWipeGeometry::StripRoll { strips, turns, .. } => ensure!(
                (2..=5).contains(&strips) && turns.iter().all(|n| *n <= 31),
                "integer wipe roll budget"
            ),
            IntegerWipeGeometry::Clock { pi } => ensure!(
                pi.is_finite() && (3.0..=3.2).contains(&pi),
                "integer wipe angular constant"
            ),
            _ => (),
        }
        Ok(())
    }
    pub fn apply(
        &self,
        old: &ImageSurface,
        new: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        old.validate()?;
        new.validate()?;
        ensure!(
            [old.width, old.height] == [new.width, new.height]
                && u64::from(old.width) * u64::from(old.height) <= MAX_EFFECT_PIXELS,
            "integer wipe image budget/extent"
        );
        check()?;
        let duration = i64::from(self.duration.max(1));
        let mut count = i64::from(self.count).min(duration);
        if let IntegerWipeGeometry::Scale { spec } = self.geometry {
            return spec.apply(old, new, count, duration, check);
        }
        if let IntegerWipeGeometry::Shape { spec } = self.geometry {
            return spec.apply(old, new, count, duration, check);
        }
        if let IntegerWipeGeometry::Scanline { spec } = self.geometry {
            return spec.apply(old, new, count, duration, check);
        }
        if matches!(self.geometry, IntegerWipeGeometry::AdditiveCross) {
            let half = (duration / 2).max(1);
            let (left, right, c, divisor) = if count >= half {
                (new, old, duration - count, (duration - half).max(1))
            } else {
                (old, new, count, half)
            };
            let weight = (c * 256 / divisor).clamp(0, 256) as u32;
            let mut out = left.clone();
            for (row, right) in out
                .rgba
                .chunks_exact_mut(old.width as usize * 4)
                .zip(right.rgba.chunks_exact(old.width as usize * 4))
            {
                check()?;
                for (pixel, source) in row.chunks_exact_mut(4).zip(right.chunks_exact(4)) {
                    for channel in 0..3 {
                        pixel[channel] = pixel[channel]
                            .saturating_add(((u32::from(source[channel]) * weight) >> 8) as u8);
                    }
                    pixel[3] = 255;
                }
            }
            check()?;
            return Ok(out);
        }
        let reverse = matches!(
            self.geometry,
            IntegerWipeGeometry::Panel { reverse: true, .. }
                | IntegerWipeGeometry::SplitPanels { reverse: true, .. }
                | IntegerWipeGeometry::Circular { reverse: true }
        );
        let (old, new) = if reverse {
            count = duration - count;
            (new, old)
        } else {
            (old, new)
        };
        let w = i64::from(old.width);
        let h = i64::from(old.height);
        let mut output = old.clone();
        // All coordinates below are bounded by the checked surface dimensions.
        // Invalid projected samples retain the destination, never read padding.
        let mut copy = |image: &ImageSurface,
                        sx: i64,
                        sy: i64,
                        dx: i64,
                        dy: i64,
                        width: i64,
                        height: i64|
         -> Result<()> {
            for y in 0..height.max(0) {
                check()?;
                if !(0..h).contains(&(dy + y)) || !(0..h).contains(&(sy + y)) {
                    continue;
                }
                let lo = 0.max(-dx).max(-sx);
                let hi = width.min(w - dx).min(w - sx);
                if hi <= lo {
                    continue;
                }
                let from = (((sy + y) * w + sx + lo) * 4) as usize;
                let to = (((dy + y) * w + dx + lo) * 4) as usize;
                let n = ((hi - lo) * 4) as usize;
                output.rgba[to..to + n].copy_from_slice(&image.rgba[from..from + n]);
            }
            Ok(())
        };
        match self.geometry {
            IntegerWipeGeometry::Scale { .. }
            | IntegerWipeGeometry::Scanline { .. }
            | IntegerWipeGeometry::Shape { .. } => unreachable!(),
            IntegerWipeGeometry::StaggeredBars {
                rows,
                bar,
                repeat,
                rounded_groups,
            } => {
                let bar = i64::from(bar);
                let repeat = i64::from(repeat);
                let axis = if rows { h } else { w };
                let block = bar * repeat;
                ensure!(
                    rounded_groups || axis % block == 0,
                    "integer wipe incomplete group"
                );
                let groups = if rounded_groups {
                    (axis + block - 1) / block
                } else {
                    axis / block
                };
                let stage = count * (groups + bar) / duration;
                let complete = (stage - (bar - 1)).clamp(0, groups);
                let pending = (groups - stage).max(0);
                let partial = (groups - complete - pending).max(0);
                let first = (stage + 1).min(bar - 1);
                let extent = complete * block;
                if rows {
                    copy(new, 0, 0, 0, 0, w, extent)?;
                } else {
                    copy(new, 0, 0, 0, 0, extent, h)?;
                }
                for group in 0..partial {
                    let covered = (first - group).max(0);
                    for i in 0..repeat {
                        let position = extent + group * block + i * bar;
                        if rows {
                            copy(new, 0, position, 0, position, w, covered)?;
                        } else {
                            copy(new, position, 0, position, 0, covered, h)?;
                        }
                    }
                }
            }
            IntegerWipeGeometry::InterleavedRows => {
                ensure!(h % 2 == 0, "integer wipe interleaved row pairs");
                let pairs = h / 2;
                let first = count * pairs / duration;
                let second = pairs - first;
                for pair in 0..pairs {
                    let descending = pairs - pair;
                    if descending < first {
                        copy(new, 0, pair * 2, 0, pair * 2, w, 1)?;
                    }
                    if descending >= second {
                        copy(new, 0, pair * 2 + 1, 0, pair * 2 + 1, w, 1)?;
                    }
                }
            }
            IntegerWipeGeometry::AdditiveCross => unreachable!(),
            IntegerWipeGeometry::StripRoll {
                columns,
                strips,
                turns,
            } => {
                if count >= duration - 1 {
                    copy(new, 0, 0, 0, 0, w, h)?;
                } else {
                    let strips = i64::from(strips);
                    let minor = if columns { w } else { h };
                    let cross = if columns { h } else { w };
                    for index in 0..strips {
                        let at = index * (minor / strips);
                        let extent = if index == strips - 1 {
                            minor - at
                        } else {
                            minor / strips
                        };
                        let dv = (duration - 1 - strips + index).max(1);
                        let c = (dv - count).clamp(0, dv);
                        let turns = i64::from(turns[index as usize]) * 2 + 1;
                        let mut k = c * c * turns * cross / (dv * dv) % (cross * 2);
                        if index % 2 != 0 {
                            k = (cross * 2 - k) % (cross * 2);
                        }
                        k = (k + cross) % (cross * 2);
                        for (image, position) in [
                            (old, k - cross * 2),
                            (new, k - cross),
                            (old, k),
                            (new, k + cross),
                        ] {
                            if columns {
                                copy(image, at, 0, at, position, extent, cross)?;
                            } else {
                                copy(image, 0, at, position, at, cross, extent)?;
                            }
                        }
                    }
                }
            }
            IntegerWipeGeometry::EdgeSweep => {
                let delta = w * 2 * count / duration;
                for y in 0..h {
                    if delta < w {
                        if delta > 0 {
                            // Repeated one-pixel crops keep the operation byte
                            // exact. Total work is bounded by the output plane.
                            for x in 0..delta {
                                copy(old, delta - 1, y, x, y, 1, 1)?;
                            }
                        }
                    } else {
                        let edge = delta - w;
                        copy(new, 0, y, 0, y, edge, 1)?;
                        for x in edge..w {
                            copy(new, edge, y, x, y, 1, 1)?;
                        }
                    }
                }
            }
            IntegerWipeGeometry::BouncingPanel { trailing_restarts } => {
                let t1 = (duration * 9 / 17).max(1);
                let t2 = (duration * 6 / 17).max(1);
                let t3 = (duration - t1 - t2).max(0);
                let g = 2. * h as f64 / (t1 * t1) as f64;
                let v2 = g * t1 as f64 / 3.;
                let delta = if count < t1 {
                    let t = count as f64 / t1 as f64;
                    h as f64 * (1. - t * t)
                } else if count < t1 + t2 {
                    let t = (count - t1) as f64;
                    v2 * t - g * t * t / 2.
                } else if count < t1 + t2 + t3 {
                    let t = (count - t1 - t2) as f64;
                    v2 / 3. * t - g * t * t / 2.
                } else {
                    0.
                };
                let delta = ((delta + 0.5) as i64).max(0);
                if trailing_restarts {
                    copy(old, 0, 0, 0, h - delta, w, h)?;
                    copy(new, 0, 0, 0, 0, w, h - delta)?;
                } else {
                    copy(new, 0, 0, 0, -delta, w, h)?;
                }
            }
            IntegerWipeGeometry::Circular { .. } | IntegerWipeGeometry::PackedCircular => {
                ensure!(w % 2 == 0, "integer circular wipe paired scanline extent");
                let radius_max =
                    ((((w / 2) * (w / 2) + (h / 2) * (h / 2)) as f64).sqrt() + 1.) as i64;
                let radius = count * radius_max / duration;
                for y in 0..h {
                    let dy = y - h / 2;
                    let square = radius * radius - dy * dy;
                    let half = if square <= 0 {
                        0
                    } else {
                        ((square as f64).sqrt() + 0.5) as i64
                    }
                    .min(w / 2);
                    let start = w / 2 - half;
                    if matches!(self.geometry, IntegerWipeGeometry::PackedCircular) {
                        copy(old, 0, y, 0, y, start, 1)?;
                        copy(new, 0, y, start, y, half * 2, 1)?;
                        copy(old, start, y, start + half * 2, y, start, 1)?;
                    } else {
                        copy(new, start, y, start, y, half * 2, 1)?;
                    }
                }
            }
            IntegerWipeGeometry::Clock { pi } => {
                if count == duration {
                    copy(new, 0, 0, 0, 0, w, h)?;
                } else if count > 0 {
                    ensure!(w % 2 == 0, "integer clock wipe paired scanline extent");
                    let cx = w / 2;
                    let cy = h / 2;
                    let slope = if count == duration / 4 {
                        (cx + 1) as f64
                    } else if count == duration * 3 / 4 {
                        -(cx + 1) as f64
                    } else {
                        (count as f64 * pi * 2. / duration as f64).tan()
                    };
                    for y in 0..h {
                        let x = (((y - cy) as f64 * slope).abs() + 0.5).min(cx as f64) as i64;
                        let (start, len, second) = if count <= duration / 4 {
                            if y <= cy { (cx, x, 0) } else { (w, 0, 0) }
                        } else if count <= duration / 2 {
                            if y <= cy {
                                (cx, cx, 0)
                            } else {
                                (cx + x, cx - x, 0)
                            }
                        } else if count <= duration * 3 / 4 {
                            if y <= cy {
                                (cx, cx, 0)
                            } else {
                                (cx - x, cx + x, 0)
                            }
                        } else if y <= cy {
                            (0, cx - x, cx)
                        } else {
                            (0, w, 0)
                        };
                        copy(new, start, y, start, y, len, 1)?;
                        if second > 0 {
                            copy(new, cx, y, cx, y, second, 1)?;
                        }
                    }
                }
            }
            IntegerWipeGeometry::Diagonal { translate_old } => {
                let start = (duration - count) * (w + h) / duration;
                for y in 0..h {
                    let cut = (start - y).clamp(0, w);
                    if translate_old {
                        copy(old, w - cut, y, 0, y, cut, 1)?;
                        copy(new, 0, y, cut, y, w - cut, 1)?;
                    } else {
                        copy(new, cut, y, cut, y, w - cut, 1)?;
                    }
                }
            }
            IntegerWipeGeometry::Panel { direction, .. } => {
                let dx = w * (duration - count) / duration;
                let dy = h * (duration - count) / duration;
                match direction {
                    0 => copy(new, 0, 0, 0, -dy, w, h)?,
                    1 => copy(new, 0, 0, dx, 0, w, h)?,
                    2 => copy(new, 0, 0, 0, dy, w, h)?,
                    3 => copy(new, 0, 0, -dx, 0, w, h)?,
                    _ => unreachable!(),
                }
            }
            IntegerWipeGeometry::SplitPanels { columns, .. } => {
                let axis = if columns { w } else { h };
                let half = axis / 2;
                let delta = half * (duration - count) / duration;
                if columns {
                    copy(new, 0, 0, -delta, 0, half, h)?;
                    copy(new, half, 0, half + delta, 0, w - half, h)?;
                } else {
                    copy(new, 0, 0, 0, -delta, w, half)?;
                    copy(new, 0, half, 0, half + delta, w, h - half)?;
                }
            }
        }
        check()?;
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image(w: u32, h: u32, base: u8) -> ImageSurface {
        ImageSurface {
            width: w,
            height: h,
            rgba: (0..w * h)
                .flat_map(|n| [base + n as u8, 0, 0, 255])
                .collect(),
        }
    }
    #[test]
    fn exact_integer_seams_clipped_panels_and_interleaved_strict_edges() {
        let old = image(4, 4, 0);
        let new = image(4, 4, 100);
        let apply = |geometry, count, duration| {
            IntegerWipe {
                geometry,
                count,
                duration,
            }
            .apply(&old, &new, &|| Ok(()))
            .unwrap()
        };
        let red =
            |image: ImageSurface| image.rgba.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>();
        assert_eq!(
            red(apply(
                IntegerWipeGeometry::Diagonal {
                    translate_old: false
                },
                1,
                2
            )),
            vec![0, 1, 2, 3, 4, 5, 6, 107, 8, 9, 110, 111, 12, 113, 114, 115]
        );
        assert_eq!(
            red(apply(
                IntegerWipeGeometry::Diagonal {
                    translate_old: true
                },
                1,
                2
            )),
            vec![
                0, 1, 2, 3, 5, 6, 7, 104, 10, 11, 108, 109, 15, 112, 113, 114
            ]
        );
        assert_eq!(
            red(apply(IntegerWipeGeometry::InterleavedRows, 0, 2)),
            vec![0, 1, 2, 3, 104, 105, 106, 107, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(
            red(apply(
                IntegerWipeGeometry::Panel {
                    direction: 1,
                    reverse: false
                },
                1,
                2
            )),
            vec![
                0, 1, 100, 101, 4, 5, 104, 105, 8, 9, 108, 109, 12, 13, 112, 113
            ]
        );
        assert_eq!(
            red(apply(
                IntegerWipeGeometry::SplitPanels {
                    columns: true,
                    reverse: false
                },
                1,
                2
            )),
            vec![
                101, 1, 2, 102, 105, 5, 6, 106, 109, 9, 10, 110, 113, 13, 14, 114
            ]
        );
        assert_eq!(
            red(apply(
                IntegerWipeGeometry::Circular { reverse: false },
                1,
                3
            )),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 109, 110, 11, 12, 13, 14, 15]
        );
        assert_eq!(
            red(apply(IntegerWipeGeometry::PackedCircular, 1, 3)),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 108, 109, 9, 12, 13, 14, 15]
        );
        assert_eq!(
            red(apply(IntegerWipeGeometry::Clock { pi: 3.14159 }, 1, 4)),
            vec![0, 1, 102, 103, 4, 5, 106, 107, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(apply(IntegerWipeGeometry::Clock { pi: 3.14159 }, 4, 4), new);
        assert_eq!(
            red(apply(IntegerWipeGeometry::EdgeSweep, 1, 4)),
            vec![1, 1, 2, 3, 5, 5, 6, 7, 9, 9, 10, 11, 13, 13, 14, 15]
        );
        assert_eq!(
            red(apply(IntegerWipeGeometry::EdgeSweep, 3, 4)),
            vec![
                100, 101, 102, 102, 104, 105, 106, 106, 108, 109, 110, 110, 112, 113, 114, 114
            ]
        );
        for direction in 0..4 {
            assert_eq!(
                apply(
                    IntegerWipeGeometry::Panel {
                        direction,
                        reverse: false
                    },
                    0,
                    4
                ),
                old
            );
            assert_eq!(
                apply(
                    IntegerWipeGeometry::Panel {
                        direction,
                        reverse: false
                    },
                    4,
                    4
                ),
                new
            );
            assert_eq!(
                apply(
                    IntegerWipeGeometry::Panel {
                        direction,
                        reverse: true
                    },
                    0,
                    4
                ),
                old
            );
            assert_eq!(
                apply(
                    IntegerWipeGeometry::Panel {
                        direction,
                        reverse: true
                    },
                    4,
                    4
                ),
                new
            );
        }
        for columns in [false, true] {
            for strips in 2..=5 {
                let geometry = IntegerWipeGeometry::StripRoll {
                    columns,
                    strips,
                    turns: [2, 3, 5, 4, 1],
                };
                assert_eq!(apply(geometry, 0, 20), old);
                assert_eq!(apply(geometry, 19, 20), new);
            }
        }
        assert_eq!(apply(IntegerWipeGeometry::AdditiveCross, 0, 4), old);
        assert_eq!(apply(IntegerWipeGeometry::AdditiveCross, 4, 4), new);
        let left = ImageSurface {
            width: 1,
            height: 1,
            rgba: vec![200, 9, 100, 255],
        };
        let right = ImageSurface {
            width: 1,
            height: 1,
            rgba: vec![199, 101, 7, 255],
        };
        let output = IntegerWipe {
            geometry: IntegerWipeGeometry::AdditiveCross,
            count: 1,
            duration: 4,
        }
        .apply(&left, &right, &|| Ok(()))
        .unwrap();
        assert_eq!(output.rgba, vec![255, 59, 103, 255]);
        assert!(
            IntegerWipe {
                geometry: IntegerWipeGeometry::Panel {
                    direction: 0,
                    reverse: false
                },
                count: 1,
                duration: 2
            }
            .apply(&old, &new, &|| anyhow::bail!("cancelled"))
            .is_err()
        );
        let old = image(64, 2, 0);
        let new = image(64, 2, 100);
        let output = IntegerWipe {
            geometry: IntegerWipeGeometry::StaggeredBars {
                rows: false,
                bar: 16,
                repeat: 2,
                rounded_groups: false,
            },
            count: 0,
            duration: 10,
        }
        .apply(&old, &new, &|| Ok(()))
        .unwrap();
        assert_eq!(output, old);
        let output = IntegerWipe {
            geometry: IntegerWipeGeometry::StaggeredBars {
                rows: false,
                bar: 16,
                repeat: 2,
                rounded_groups: false,
            },
            count: 10,
            duration: 10,
        }
        .apply(&old, &new, &|| Ok(()))
        .unwrap();
        assert_eq!(output, new);
    }
}
