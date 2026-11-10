// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Clipped translated y-monotone polygons with paired floating edge walkers.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageFragment {
    pub vertices: Vec<[i32; 2]>,
    pub translation: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolygonFragments {
    pub fragments: Vec<ImageFragment>,
}
impl PolygonFragments {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.fragments.len() <= 256, "fragment count budget");
        let mut edges = 0;
        for fragment in &self.fragments {
            edges += fragment.vertices.len();
            ensure!(
                (3..=32).contains(&fragment.vertices.len()) && edges <= 4096,
                "fragment edge budget"
            );
            ensure!(
                fragment
                    .vertices
                    .iter()
                    .flatten()
                    .chain(fragment.translation.iter())
                    .all(|v| v.unsigned_abs() <= 1_000_000),
                "fragment coordinate bounds"
            );
            let min = fragment.vertices.iter().map(|v| v[1]).min().unwrap();
            let max = fragment.vertices.iter().map(|v| v[1]).max().unwrap();
            ensure!(
                fragment.vertices[0][1] == min && max - min <= 16384,
                "fragment scanline extent/top vertex"
            );
            let bottom = fragment.vertices.iter().position(|v| v[1] == max).unwrap();
            ensure!(
                fragment.vertices[..=bottom]
                    .windows(2)
                    .all(|p| p[0][1] <= p[1][1])
                    && fragment.vertices[bottom..]
                        .windows(2)
                        .all(|p| p[0][1] >= p[1][1]),
                "fragment polygon must be Y-monotone"
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
        ensure!(
            [source.width, source.height] == [destination.width, destination.height]
                && u64::from(source.width) * u64::from(source.height) <= MAX_EFFECT_PIXELS,
            "fragment image budget/extent"
        );
        let mut output = destination.clone();
        let w = i64::from(source.width);
        let h = i64::from(source.height);
        let mut work = 0u64;
        for fragment in &self.fragments {
            check()?;
            let n = fragment.vertices.len();
            let mut polygon = fragment.vertices.clone();
            polygon.push(polygon[0]);
            let [tx, ty] = fragment.translation.map(i64::from);
            let y0 = i64::from(polygon[0][1]);
            let height = i64::from(polygon.iter().map(|v| v[1]).max().unwrap()) - y0 + 1;
            let mut left = 0usize;
            let mut right = n;
            if polygon[right][1] == polygon[right - 1][1] {
                right -= 1;
            }
            let mut lc = 0i64;
            let mut rc = 0i64;
            for row in 0..height {
                check()?;
                let y = y0 + row;
                let dy = y + ty;
                let Some(le) = edge(&polygon, left, 1) else {
                    break;
                };
                let Some(re) = edge(&polygon, right, -1) else {
                    break;
                };
                let lx =
                    (le.0 + (le.1 - le.0) * (lc as f32) * (1. / le.2.max(1) as f32) + 0.5) as i64;
                let rx =
                    (re.0 + (re.1 - re.0) * (rc as f32) * (1. / re.2.max(1) as f32) + 0.5) as i64;
                if (0..h).contains(&dy) && (0..h).contains(&y) && rx > lx {
                    let begin = lx.max(-tx).max(0);
                    let end = rx.min(w - 1 - tx).min(w - 1);
                    if end >= begin {
                        work += (end - begin + 1) as u64;
                        ensure!(
                            work <= MAX_EFFECT_PIXELS * 16,
                            "fragment aggregate raster budget"
                        );
                        let src = ((y * w + begin) * 4) as usize;
                        let dst = ((dy * w + begin + tx) * 4) as usize;
                        let size = ((end - begin + 1) * 4) as usize;
                        output.rgba[dst..dst + size].copy_from_slice(&source.rgba[src..src + size]);
                    }
                }
                if dy + 1 >= h {
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
                    let l = edge(&polygon, left, 1);
                    let r = edge(&polygon, right, -1);
                    if l.is_some_and(|e| e.2 == 0) || r.is_some_and(|e| e.2 == 0) {
                        left = left.saturating_sub(1);
                        right = (right + 1).min(n);
                        lc = edge(&polygon, left, 1).map_or(0, |e| e.2);
                        rc = edge(&polygon, right, -1).map_or(0, |e| e.2);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}
fn edge(points: &[[i32; 2]], at: usize, step: isize) -> Option<(f32, f32, i64)> {
    let next = at.checked_add_signed(step)?;
    let a = points.get(at)?;
    let b = points.get(next)?;
    // Original edge walkers could step beyond the final flat edge. Checked
    // termination replaces that scratch-memory read without exposing host RAM.
    Some((a[0] as f32, b[0] as f32, i64::from(b[1]) - i64::from(a[1])))
}
