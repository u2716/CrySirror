// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Inclusive f32 paired-edge fragment walkers with retained linear source
//! addressing and low-word RGB mixing. Native partition/motion stays caller data.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, integer_wipe::fragments::ImageFragment};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatingPolygonFragments {
    pub fragments: Vec<ImageFragment>,
    pub weight256: i32,
}
fn edge(points: &[[i32; 2]], at: usize, step: isize) -> Option<(f32, f32, i32)> {
    let a = points.get(at)?;
    let b = points.get(at.checked_add_signed(step)?)?;
    Some((a[0] as f32, b[0] as f32, b[1] - a[1]))
}
impl FloatingPolygonFragments {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.fragments.len() <= 4096, "floating fragment count");
        let mut edges = 0;
        for polygon in &self.fragments {
            edges += polygon.vertices.len();
            ensure!(
                (3..=15).contains(&polygon.vertices.len())
                    && edges <= 65536
                    && polygon
                        .vertices
                        .iter()
                        .flatten()
                        .chain(&polygon.translation)
                        .all(|v| v.unsigned_abs() <= 1_000_000),
                "floating fragment coordinates/edges"
            );
            let min = polygon.vertices.iter().map(|v| v[1]).min().unwrap();
            let max = polygon.vertices.iter().map(|v| v[1]).max().unwrap();
            ensure!(max - min < 16384, "floating fragment row span");
        }
        Ok(())
    }
    pub fn scratch_bytes(&self) -> Result<usize> {
        self.validate()?;
        Ok(self
            .fragments
            .iter()
            .map(|p| {
                let min = p.vertices.iter().map(|v| v[1]).min().unwrap();
                let max = p.vertices.iter().map(|v| v[1]).max().unwrap();
                (max - min + 1) as usize * std::mem::size_of::<[i32; 2]>()
            })
            .max()
            .unwrap_or(0))
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
                .all(|image| u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS),
            "floating fragment image budget"
        );
        check()?;
        let mut out = destination.clone();
        let mut work = 0u64;
        let mut edge_work = 0u64;
        let mut spans = Vec::<[i32; 2]>::new();
        for fragment in &self.fragments {
            check()?;
            let [tx, ty] = fragment.translation;
            let n = fragment.vertices.len();
            let mut points = fragment.vertices.clone();
            points.push(points[0]);
            let min = points.iter().map(|v| v[1]).min().unwrap();
            let max = points.iter().map(|v| v[1]).max().unwrap();
            let height = max - min + 1;
            let y0 = points[0][1];
            let mut y = y0 + ty;
            if y >= out.height as i32 || y + i32::try_from(height).unwrap() < 0 {
                continue;
            }
            let start_y = y.max(0);
            let source_y = y0 + (0 - y).max(0);
            let mut left = 0;
            let mut right = n;
            if points[right][1] == points[right - 1][1] {
                right -= 1;
            }
            let mut lc = 0;
            let mut rc = 0;
            spans.clear();
            for row in 0..height {
                check()?;
                edge_work += 1;
                ensure!(
                    edge_work <= MAX_EFFECT_PIXELS * 16,
                    "floating fragment edge work budget"
                );
                let (Some(le), Some(re)) = (edge(&points, left, 1), edge(&points, right, -1))
                else {
                    break;
                };
                if y >= 0 {
                    let lx = (le.0
                        + ((le.1 - le.0) * (lc as f32)) * (1.0 / le.2.max(1) as f32)
                        + 0.5) as i32;
                    let rx = (re.0
                        + ((re.1 - re.0) * (rc as f32)) * (1.0 / re.2.max(1) as f32)
                        + 0.5) as i32;
                    let mut from = lx;
                    let mut begin = tx + lx;
                    let mut end = tx + rx;
                    if begin >= out.width as i32 || end < 0 || rx - lx <= 0 {
                        spans.push([0, 0]);
                    } else {
                        if begin < 0 {
                            from -= begin;
                            begin = 0;
                        }
                        end = end.min(out.width as i32 - 1);
                        spans.push([(end - begin + 1).max(0), from]);
                    }
                }
                y += 1;
                if y >= out.height as i32 {
                    break;
                }
                lc += 1;
                if lc >= le.2 {
                    left += 1;
                    lc = 0;
                }
                rc += 1;
                if rc >= re.2 {
                    right = right.saturating_sub(1);
                    rc = 0;
                }
                if row == height - 2 {
                    let l = edge(&points, left, 1);
                    let r = edge(&points, right, -1);
                    if l.is_some_and(|v| v.2 == 0) || r.is_some_and(|v| v.2 == 0) {
                        left = left.saturating_sub(1);
                        right = (right + 1).min(n);
                        lc = edge(&points, left, 1).map_or(0, |v| v.2);
                        rc = edge(&points, right, -1).map_or(0, |v| v.2);
                    }
                }
            }
            let rows = spans
                .len()
                .min((source.height as i64 - i64::from(source_y)).max(0) as usize);
            for (row, [count, x]) in spans.iter().take(rows).enumerate() {
                check()?;
                work += *count as u64;
                ensure!(
                    work <= MAX_EFFECT_PIXELS * 16,
                    "floating fragment pixel work budget"
                );
                let source_start =
                    (i64::from(source_y) + row as i64) * i64::from(source.width) + i64::from(*x);
                let target_start = (i64::from(start_y) + row as i64) * i64::from(out.width)
                    + i64::from(*x)
                    + i64::from(tx);
                for col in 0..*count {
                    if col % 4096 == 0 {
                        check()?;
                    }
                    let from = source_start + i64::from(col);
                    let to = target_start + i64::from(col);
                    // Original scratch and source-pointer overruns are bounded
                    // to no-write; valid cross-row LINEAR reads stay intact.
                    if from < 0
                        || to < 0
                        || from >= i64::from(source.width) * i64::from(source.height)
                        || to >= i64::from(out.width) * i64::from(out.height)
                    {
                        continue;
                    }
                    let from = from as usize * 4;
                    let to = to as usize * 4;
                    for c in 0..3 {
                        out.rgba[to + c] = if self.weight256 == 256 {
                            source.rgba[from + c]
                        } else {
                            let a =
                                (source.rgba[from + c] as u16).wrapping_mul(self.weight256 as u16);
                            let b = (out.rgba[to + c] as u16)
                                .wrapping_mul(256i32.wrapping_sub(self.weight256) as u16);
                            (a.wrapping_add(b) >> 8) as u8
                        };
                    }
                }
            }
        }
        check()?;
        Ok(out)
    }
}
