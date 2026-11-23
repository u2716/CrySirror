// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Quantized outward channel lifts from independently positioned row edges.
//! Row boundaries are data; clipping, pair quantization and pixels are shared.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowEdgeLift {
    pub row_start: i32,
    /// Absolute column GROUP indices. Left contributes (left-group)*step for
    /// groups<=left; right contributes (group-right)*step for groups>=right.
    /// Each edge may be independently absent; overlapping lifts saturate.
    pub edge_groups: Vec<[Option<i32>; 2]>,
    pub group_width: u16,
    pub step: u8,
    /// Exclude the final incomplete group from the right-side lift only.
    pub right_full_groups_only: bool,
    pub write_mask: [bool; 4],
}
impl RowEdgeLift {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=256).contains(&self.group_width) && self.edge_groups.len() <= 16384,
            "row edge lift work bounds"
        );
        ensure!(
            self.row_start.unsigned_abs() <= 1_000_000
                && self
                    .edge_groups
                    .iter()
                    .flatten()
                    .flatten()
                    .all(|n| n.unsigned_abs() <= 1_000_000),
            "row edge lift coordinate bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "row edge lift pixel budget"
        );
        check()?;
        let mut output = input.clone();
        let quantum = i64::from(self.group_width);
        let full = i64::from(input.width) / quantum;
        for (row, edges) in self.edge_groups.iter().enumerate() {
            check()?;
            let y = i64::from(self.row_start) + row as i64;
            if y < 0 || y >= i64::from(input.height) {
                continue;
            }
            for x in 0..input.width {
                let group = i64::from(x) / quantum;
                let left = edges[0].map_or(0, |edge| {
                    (i64::from(edge) - group).max(0) * i64::from(self.step)
                });
                let right = if self.right_full_groups_only && group >= full {
                    0
                } else {
                    edges[1].map_or(0, |edge| {
                        (group - i64::from(edge)).max(0) * i64::from(self.step)
                    })
                };
                let lift = (left + right).min(255) as u8;
                let at = (y as usize * input.width as usize + x as usize) * 4;
                for c in 0..4 {
                    if self.write_mask[c] {
                        output.rgba[at + c] = output.rgba[at + c].saturating_add(lift);
                    }
                }
            }
        }
        check()?;
        Ok(output)
    }
}
