// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Integer scanline reveals, bounded row remapping and encoded-byte waves.
//! The caller supplies time; no native selectors or device state live here.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScanlineWipe {
    WaveEdge { reverse: bool, gradient: u16 },
    ChevronEdge { inverted: bool, gradient: u16 },
    DoubleChevron { reverse: bool },
    SlantEdge { orientation: u8, gradient: u16 },
    AcceleratingRows { origin: u8, reverse: bool },
    PushPanels { direction: u8 },
    CosineWarp { black_edges: bool },
    MultiplyCross,
    FoldRows,
}
impl ScanlineWipe {
    pub fn validate(&self) -> Result<()> {
        match *self {
            Self::WaveEdge { gradient, .. }
            | Self::ChevronEdge { gradient, .. }
            | Self::SlantEdge { gradient, .. } => {
                ensure!(gradient <= 256, "scanline gradient width")
            }
            _ => (),
        }
        if let Self::SlantEdge { orientation, .. }
        | Self::PushPanels {
            direction: orientation,
        } = *self
        {
            ensure!(orientation < 4, "scanline direction");
        }
        if let Self::AcceleratingRows { origin, .. } = *self {
            ensure!(origin < 3, "scanline shutter origin");
        }
        Ok(())
    }
    pub(super) fn apply(
        &self,
        old: &ImageSurface,
        new: &ImageSurface,
        count: i64,
        duration: i64,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        let reverse = matches!(
            *self,
            Self::WaveEdge { reverse: true, .. }
                | Self::ChevronEdge { inverted: true, .. }
                | Self::DoubleChevron { reverse: true }
                | Self::SlantEdge {
                    orientation: 2 | 3,
                    ..
                }
                | Self::AcceleratingRows { reverse: true, .. }
        );
        let (old, new, count) = if reverse {
            (new, old, duration - count)
        } else {
            (old, new, count)
        };
        let w = i64::from(old.width);
        let h = i64::from(old.height);
        let mut output = old.clone();
        match *self {
            Self::WaveEdge { gradient, .. }
            | Self::ChevronEdge { gradient, .. }
            | Self::SlantEdge { gradient, .. } => {
                let gradient = i64::from(gradient);
                let delta = if gradient > 0 { 256 / gradient } else { 0 };
                let mut radius = [8., 5., 2.];
                for (i, r) in radius.iter_mut().enumerate() {
                    let angle = ((count * [35, 53, 21][i]) % 360) as f64 * (3.14159 / 180.);
                    *r = *r * w as f64 / 100. * (angle.sin() + 1.5) / 2.5;
                }
                for y in 0..h {
                    check()?;
                    let edge = match *self {
                        Self::WaveEdge { .. } => {
                            let mut x = 0.;
                            for i in 0..3 {
                                let angle = (([0, 70, 250][i] + count * 17 + [2, 3, 7][i] * y)
                                    % 360) as f64
                                    * (3.14159 / 180.);
                                x += radius[i] * angle.sin() * 0.5;
                            }
                            x += (count * w * 13 / (10 * duration) - w * 3 / 20) as f64;
                            // C++ truncates toward zero after adding 0.5 even
                            // for negative values; round() is not equivalent.
                            (x + 0.5) as i64
                        }
                        Self::ChevronEdge { inverted, .. } => {
                            let diagonal = h / 2 * 2;
                            let dx = if y >= h / 2 { (h - y) * 2 } else { y * 2 };
                            (if inverted { -dx } else { -diagonal + dx })
                                + (w + diagonal + gradient) * count / duration
                                - gradient
                        }
                        Self::SlantEdge { orientation, .. } => {
                            -gradient
                                + (if matches!(orientation, 0 | 3) {
                                    -y
                                } else {
                                    y - h
                                })
                                + count * (gradient + w + h) / duration
                        }
                        _ => unreachable!(),
                    };
                    let start = edge.clamp(0, w);
                    let end = (edge + gradient).clamp(0, w);
                    for x in 0..w {
                        let at = ((y * w + x) * 4) as usize;
                        if x < start {
                            output.rgba[at..at + 4].copy_from_slice(&new.rgba[at..at + 4]);
                        } else if x < end {
                            let weight = (256 + edge.min(0) * delta - (x - start) * delta)
                                .clamp(0, 256) as u32;
                            for c in 0..3 {
                                output.rgba[at + c] =
                                    (((u32::from(new.rgba[at + c]) * weight) >> 8)
                                        + ((u32::from(old.rgba[at + c]) * (256 - weight)) >> 8))
                                        .min(255) as u8;
                            }
                            output.rgba[at + 3] = 255;
                        }
                    }
                }
            }
            Self::DoubleChevron { .. } => {
                let delta = (w / 2 + h / 2) * count / duration;
                for y in 0..h {
                    check()?;
                    let dy = if y >= h / 2 { h - y } else { y };
                    let left = -h / 2 + dy + delta;
                    let right = w + h / 2 - dy - delta;
                    for x in 0..w {
                        if left >= right || x < left.max(0) || x >= right.min(w) {
                            let at = ((y * w + x) * 4) as usize;
                            output.rgba[at..at + 4].copy_from_slice(&new.rgba[at..at + 4]);
                        }
                    }
                }
            }
            Self::MultiplyCross => {
                let limit = (duration / 2).max(1);
                let (old, new, count) = if count > limit {
                    (new, old, duration - count)
                } else {
                    (old, new, count)
                };
                let weight = (count * 256 / limit).clamp(0, 256) as u32;
                for y in 0..h {
                    check()?;
                    for x in 0..w {
                        let at = ((y * w + x) * 4) as usize;
                        for c in 0..3 {
                            let a = u32::from(old.rgba[at + c]);
                            let b = u32::from(new.rgba[at + c]);
                            output.rgba[at + c] =
                                ((a * (256 - weight) + ((a * b) >> 8) * weight) >> 8) as u8;
                        }
                        output.rgba[at + 3] = 255;
                    }
                }
            }
            Self::CosineWarp { black_edges } => {
                let source = if count >= duration / 2 { new } else { old };
                let mut amplitude = w as f64 * 0.9;
                amplitude *= if count < duration / 2 {
                    count as f64
                } else {
                    (duration - count) as f64
                };
                amplitude /= duration as f64 / 2.;
                if count >= duration {
                    amplitude = 0.;
                }
                for y in 0..h {
                    check()?;
                    let angle = (y + count * 10) as f64 * (3.14159 * 2. / h as f64 * 4.);
                    let shift = ((angle.cos() * amplitude + 0.5) as i64).clamp(-(w - 1), w - 1);
                    for x in 0..w {
                        let sx = x + shift;
                        let at = ((y * w + x) * 4) as usize;
                        if black_edges && !(0..w).contains(&sx) {
                            output.rgba[at..at + 4].copy_from_slice(&[0, 0, 0, 255]);
                        } else {
                            let src = ((y * w + sx.clamp(0, w - 1)) * 4) as usize;
                            output.rgba[at..at + 4].copy_from_slice(&source.rgba[src..src + 4]);
                        }
                    }
                }
            }
            Self::FoldRows => {
                let first = (count * h / duration).clamp(0, h);
                let second = (first - 1).max(0).min(h - first);
                for y in 0..h {
                    check()?;
                    let (source, sy) = if y < first {
                        (new, y)
                    } else if y < first + second {
                        (new, first - 2 - (y - first))
                    } else {
                        (old, y)
                    };
                    copy_row(&mut output, source, sy, y, w);
                }
            }
            Self::AcceleratingRows { origin, .. } => {
                let mut rows = vec![-1i64; h as usize];
                let travel = h + h / 8 + 28;
                match origin {
                    0 => {
                        let start = h - travel * count / duration;
                        for y in 0..start.min(h).max(0) {
                            rows[y as usize] = y;
                        }
                        run_rows(&mut rows, start, 1, 0, h);
                    }
                    1 => {
                        let start = travel * count / duration;
                        for y in start.max(0)..h {
                            rows[y as usize] = y;
                        }
                        run_rows(&mut rows, start, -1, 0, h);
                    }
                    2 => {
                        let upper = travel / 2 * count / duration;
                        for y in upper.max(0)..h / 2 {
                            rows[y as usize] = y;
                        }
                        run_rows(&mut rows, upper, -1, 0, h / 2);
                        let lower = h - upper;
                        for y in h / 2..lower.min(h) {
                            rows[y as usize] = y;
                        }
                        run_rows(&mut rows, lower, 1, h / 2, h);
                    }
                    _ => unreachable!(),
                }
                // Every entry starts initialized to -1. This replaces the
                // source's scratch allocation while preserving its overwrite
                // order and accelerating destination-position increments.
                for y in 0..h {
                    check()?;
                    let sy = rows[y as usize];
                    copy_row(
                        &mut output,
                        if sy < 0 { new } else { old },
                        if sy < 0 { y } else { sy },
                        y,
                        w,
                    );
                }
            }
            Self::PushPanels { direction } => {
                let [dx, dy] = [[0, 1], [-1, 0], [0, -1], [1, 0]][direction as usize];
                let ox = w * dx * count / duration;
                let oy = h * dy * count / duration;
                for y in 0..h {
                    check()?;
                    for x in 0..w {
                        let nx = x - ox + w * dx;
                        let ny = y - oy + h * dy;
                        let (source, sx, sy) = if (0..w).contains(&nx) && (0..h).contains(&ny) {
                            (new, nx, ny)
                        } else {
                            (old, x - ox, y - oy)
                        };
                        if (0..w).contains(&sx) && (0..h).contains(&sy) {
                            let at = ((y * w + x) * 4) as usize;
                            let from = ((sy * w + sx) * 4) as usize;
                            output.rgba[at..at + 4].copy_from_slice(&source.rgba[from..from + 4]);
                        }
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}
fn run_rows(rows: &mut [i64], mut destination: i64, step: i64, lo: i64, hi: i64) {
    let mut speed = 0;
    let mut source = destination;
    while if step > 0 { source < hi } else { source >= lo } {
        if (lo..hi).contains(&destination) && (lo..hi).contains(&source) {
            rows[destination as usize] = source;
        }
        speed = (speed + 1).min(8);
        destination += step * speed;
        source += step;
    }
}
fn copy_row(output: &mut ImageSurface, source: &ImageSurface, sy: i64, dy: i64, width: i64) {
    if !(0..i64::from(source.height)).contains(&sy) {
        return;
    }
    let src = (sy * width * 4) as usize;
    let dst = (dy * width * 4) as usize;
    let n = (width * 4) as usize;
    output.rgba[dst..dst + n].copy_from_slice(&source.rgba[src..src + n]);
}
