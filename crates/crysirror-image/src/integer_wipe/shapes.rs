// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded integer scanline shapes. Inclusive edge interpolation, asymmetric
//! run lengths and angular quantization are explicit raster profiles.
use crate::ImageSurface;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShapeWipe {
    Star { reverse: bool },
    Heart { reverse: bool },
    RotatingStar { reverse: bool, slow: bool },
}
impl ShapeWipe {
    pub(super) fn apply(
        self,
        old: &ImageSurface,
        new: &ImageSurface,
        count: i64,
        duration: i64,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        let reverse = match self {
            Self::Star { reverse }
            | Self::Heart { reverse }
            | Self::RotatingStar { reverse, .. } => reverse,
        };
        let (old, new, count) = if reverse {
            (new, old, duration - count)
        } else {
            (old, new, count)
        };
        let w = i64::from(old.width);
        let h = i64::from(old.height);
        let cx = w / 2;
        let cy = h / 2;
        let mut rows = vec![[w, 0, 0, 0, 0, 0]; h as usize];
        match self {
            Self::Star { .. } => {
                let r = w as f64 * 1.8 * count as f64 / duration as f64;
                let r2 = r * (3. - 5f64.sqrt()) / 2.;
                let a = 18f64 / 180. * 3.14159;
                let b = 36f64 / 180. * 3.14159;
                let y0 = (-r + 0.5) as i64;
                let x1 = (-r2 * b.sin() + 0.5) as i64;
                let y1 = (-r * a.sin() + 0.5) as i64;
                let x2 = (-r * a.cos()) as i64;
                let x3 = (-r2 * a.cos() + 0.5) as i64;
                let y3 = (r2 * a.sin() + 0.5) as i64;
                let y4 = (r2 + 0.5) as i64;
                let x5 = (-r * b.sin() + 0.5) as i64;
                let y5 = (r * b.cos() + 0.5) as i64;
                for y in 0..h {
                    check()?;
                    let p = y - cy;
                    if (y0..=y1 - 1).contains(&p) {
                        let x = interpolate(0, x1, p - y0, y1 - 1 - y0);
                        rows[y as usize] = single(w, cx + x, cx - x);
                    }
                    if (y1..=y3 - 1).contains(&p) {
                        let x = interpolate(x2, x3, p - y1, y3 - 1 - y1);
                        rows[y as usize] = single(w, cx + x, cx - x);
                    }
                    if (y3..=y4 - 1).contains(&p) {
                        let x = interpolate(x3, x5, p - y3, y5 - y3);
                        rows[y as usize] = single(w, cx + x, cx - x);
                    }
                    if (y4..=y5 - 1).contains(&p) {
                        let outer = interpolate(x3, x5, p - y3, y5 - 1 - y3);
                        let inner = interpolate(0, x5, p - y4, y5 - 1 - y4);
                        rows[y as usize] =
                            double(w, cx + outer, cx + inner, cx - inner, cx - outer);
                    }
                }
            }
            Self::Heart { .. } => {
                let r = w as f64 * 0.6 * count as f64 / duration as f64;
                let y0 = (-r - r * 2f64.sqrt() / 2. + 0.5) as i64;
                let y1 = (-r * 2f64.sqrt() / 2. + 0.5) as i64;
                let x2 = (-r - r * 2f64.sqrt() / 2. + 0.5) as i64;
                let y3 = (r * 2. + 0.5) as i64;
                for y in 0..h {
                    check()?;
                    let p = y - cy;
                    if (y0..=y1).contains(&p) {
                        let yy = (y1 - p) as f64;
                        let d = (r * r - yy * yy).max(0.).sqrt();
                        rows[y as usize] = double(
                            w,
                            (cx as f64 - r - d + 0.5) as i64,
                            (cx as f64 - r + d + 0.5) as i64,
                            (cx as f64 + r - d + 0.5) as i64,
                            (cx as f64 + r + d + 0.5) as i64,
                        );
                    }
                    if (y1 + 1..=0).contains(&p) {
                        let yy = (p - y1 - 1) as f64;
                        let d = (r * r - yy * yy).max(0.).sqrt();
                        rows[y as usize] = single(
                            w,
                            (cx as f64 - r - d + 0.5) as i64,
                            (cx as f64 + r + d + 0.5) as i64,
                        );
                    }
                    if (1..=y3).contains(&p) {
                        let x = interpolate(x2, 0, p - 1, y3 - 1);
                        rows[y as usize] = single(w, cx + x, cx - x);
                    }
                }
            }
            Self::RotatingStar { slow, .. } => {
                let r = w as f64 * 1.8 * count as f64 / duration as f64;
                let r2 = r * (3. - 5f64.sqrt()) / 2.;
                let phase = count * (if slow { 216 } else { 720 }) / duration % 72;
                let mut outer = [[0; 2]; 5];
                let mut inner = [[0; 2]; 5];
                for i in 0..5 {
                    let a = ((phase + 198 + i as i64 * 72) % 360) as f64 * (3.14159 / 180.);
                    let b = ((phase + 234 + i as i64 * 72) % 360) as f64 * (3.14159 / 180.);
                    outer[i] = [
                        (cx as f64 + r * a.cos() + 0.5) as i64,
                        (cy as f64 + r * a.sin() + 0.5) as i64,
                    ];
                    inner[i] = [
                        (cx as f64 + r2 * b.cos() + 0.5) as i64,
                        (cy as f64 + r2 * b.sin() + 0.5) as i64,
                    ];
                }
                let mut dots = vec![[0i64; 5]; h as usize];
                for (side, a, b) in [
                    (0, outer[0], inner[4]),
                    (1, outer[0], inner[0]),
                    (0, inner[4], outer[4]),
                    (1, inner[3], outer[4]),
                    (0, outer[1], inner[0]),
                    (1, outer[1], inner[1]),
                    (0, inner[3], outer[3]),
                    (1, inner[2], outer[3]),
                    (1, outer[2], inner[2]),
                    (1, inner[1], outer[2]),
                ] {
                    let (side, a, b) = if b[1] < a[1] {
                        (1 - side, b, a)
                    } else {
                        (side, a, b)
                    };
                    // The original inclusive end could write scratch row h.
                    // Clip to the allocated rows; no native RAM overrun survives.
                    for y in a[1].max(0)..=b[1].min(h - 1) {
                        check()?;
                        let x = interpolate(a[0], b[0], y - a[1], b[1] - a[1]);
                        let row = &mut dots[y as usize];
                        match (side, row[0]) {
                            (0, 0) => {
                                row[0] = 1;
                                row[1] = x;
                            }
                            (0, 2) => {
                                row[0] = 3;
                                row[3] = x;
                            }
                            (1, 1) => {
                                row[0] = 2;
                                row[2] = x;
                            }
                            (1, 2) => row[2] = row[2].max(x),
                            (1, 3) => {
                                row[0] = 4;
                                row[4] = x;
                            }
                            _ => (),
                        }
                    }
                }
                for (row, dot) in rows.iter_mut().zip(dots) {
                    let [n, l1, mut r1, mut l2, mut r2] = dot;
                    if n == 0 {
                        continue;
                    }
                    match n {
                        1 => {
                            r1 = w + 1;
                            l2 = r1;
                            r2 = l2 - 1;
                        }
                        2 => {
                            l2 = w;
                            r2 = w;
                        }
                        3 => r2 = w + 1,
                        _ => (),
                    }
                    *row = [
                        l1.clamp(0, w),
                        (r1.clamp(0, w) - l1.clamp(0, w)).max(0),
                        (l2.clamp(0, w) - r1.clamp(0, w)).max(0),
                        (r2.clamp(0, w) - l2.clamp(0, w)).max(0),
                        (w - r2).clamp(0, w),
                        0,
                    ];
                }
            }
        }
        let mut output = old.clone();
        for (y, row) in rows.iter().enumerate() {
            check()?;
            let mut x = 0i64;
            for (index, len) in row.iter().enumerate() {
                let end = (x + (*len).max(0)).min(w);
                if index % 2 == 1 && end > x {
                    let start = (y * old.width as usize + x as usize) * 4;
                    let end = (y * old.width as usize + end as usize) * 4;
                    output.rgba[start..end].copy_from_slice(&new.rgba[start..end]);
                }
                x = end;
            }
        }
        Ok(output)
    }
}
fn interpolate(start: i64, end: i64, at: i64, duration: i64) -> i64 {
    start + (end - start) * at / duration.max(1)
}
fn single(w: i64, mut left: i64, mut right: i64) -> [i64; 6] {
    left = left.max(0);
    right = right.min(w - 1);
    [
        left.clamp(0, w),
        if left >= w {
            0
        } else {
            (right + 1 - left).clamp(0, w)
        },
        (w - right - 1).clamp(0, w),
        0,
        0,
        0,
    ]
}
fn double(w: i64, mut left: i64, mut right: i64, left2: i64, right2: i64) -> [i64; 6] {
    left = left.max(0);
    right = right.min(w - 1);
    // Retain the source's asymmetric inclusive/exclusive run arithmetic. The
    // final emission clips summed runs instead of writing beyond a scanline.
    [
        left.clamp(0, w),
        if left >= w {
            0
        } else {
            (right + 1 - left).clamp(0, w)
        },
        if right >= w {
            0
        } else {
            (left2.min(w) + 1 - right.max(0)).clamp(0, w)
        },
        if left2 >= w {
            0
        } else {
            (right2 - left2).max(0)
        },
        (w - right2 - 1).clamp(0, w),
        0,
    ]
}
