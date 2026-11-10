// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded stored-channel extent query for decoded CPU images. Callers choose
//! native padding/trim policy separately; this query never changes pixels.
use crate::effects::MAX_EFFECT_PIXELS;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageBounds {
    pub channel: u8,
    pub not_equal: u8,
}
impl CoverageBounds {
    pub fn query(
        &self,
        size: [u32; 2],
        rgba: &[u8],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Option<[u32; 4]>> {
        ensure!(self.channel < 4, "coverage bounds channel");
        ensure!(
            rgba.len() == crate::rgba_len(size[0], size[1])?,
            "coverage bounds storage"
        );
        ensure!(
            u64::from(size[0]) * u64::from(size[1]) <= MAX_EFFECT_PIXELS,
            "coverage bounds pixel budget"
        );
        check()?;
        let mut left = size[0];
        let mut top = size[1];
        let mut right = 0;
        let mut bottom = 0;
        for y in 0..size[1] {
            check()?;
            let offset = y as usize * size[0] as usize * 4;
            for x in 0..size[0] {
                if rgba[offset + x as usize * 4 + usize::from(self.channel)] != self.not_equal {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x + 1);
                    bottom = bottom.max(y + 1);
                }
            }
        }
        check()?;
        Ok(if right > left && bottom > top {
            Some([left, top, right - left, bottom - top])
        } else {
            None
        })
    }
}

/// Intersect a source rectangle with explicit half-open left/top/right/bottom
/// edges, retaining source-to-destination phase. Physical image/canvas clipping
/// can follow independently; no image storage or native semantic state is read.
pub fn intersect_source_window(
    source: [i32; 4],
    destination: [i32; 2],
    bounds: [i32; 4],
) -> Result<Option<([i32; 4], [i32; 2])>> {
    ensure!(
        source
            .iter()
            .chain(&destination)
            .chain(&bounds)
            .all(|n| n.unsigned_abs() <= 1_000_000),
        "source window coordinate budget"
    );
    if source[2] <= 0 || source[3] <= 0 {
        return Ok(None);
    }
    let [sx, sy, w, h] = source.map(i64::from);
    let left = sx.max(i64::from(bounds[0]));
    let top = sy.max(i64::from(bounds[1]));
    let right = (sx + w).min(i64::from(bounds[2]));
    let bottom = (sy + h).min(i64::from(bounds[3]));
    if right <= left || bottom <= top {
        return Ok(None);
    }
    Ok(Some((
        [
            left as i32,
            top as i32,
            (right - left) as i32,
            (bottom - top) as i32,
        ],
        [
            (i64::from(destination[0]) + left - sx) as i32,
            (i64::from(destination[1]) + top - sy) as i32,
        ],
    )))
}
