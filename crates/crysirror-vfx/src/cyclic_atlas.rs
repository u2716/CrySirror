// SPDX-License-Identifier: MPL-2.0
//! Bounded integer atlas pools with fixed homes, cyclic drift and explicit ticks.
//! Resource loading, native command policy and rendering are separate owners.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtlasVariant {
    pub cell: [u32; 2],
    pub frames: u32,
    pub vertical_step: i32,
    pub horizontal_step: i32,
    ///One integer displacement per phase; empty disables drift/phase advance.
    pub drift: Vec<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CyclicAtlasSpec {
    pub canvas: [u32; 2],
    pub variants: Vec<AtlasVariant>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtlasNode {
    pub position: [i32; 2],
    pub home: [i32; 2],
    pub variant: usize,
    pub frame: u32,
    pub phase: usize,
    pub tick_phase: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CyclicAtlasPool {
    pub spec: CyclicAtlasSpec,
    pub nodes: Vec<AtlasNode>,
}
#[derive(Clone, Debug)]
pub struct AtlasProjection {
    pub position: [i32; 2],
    pub variant: usize,
    pub source: [i32; 4],
}
impl CyclicAtlasPool {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.spec.canvas.iter().all(|n| (1..=16384).contains(n))
                && (1..=32).contains(&self.spec.variants.len())
                && self.nodes.len() <= 32768,
            "cyclic atlas pool geometry/count"
        );
        for v in &self.spec.variants {
            ensure!(
                v.cell.iter().all(|n| (1..=16384).contains(n))
                    && (1..=1024).contains(&v.frames)
                    && (1..=16384).contains(&v.vertical_step)
                    && v.horizontal_step.unsigned_abs() <= 16384
                    && v.drift.len() <= 4096
                    && v.drift.iter().all(|n| n.unsigned_abs() <= 16384),
                "cyclic atlas variant bounds"
            );
        }
        for n in &self.nodes {
            ensure!(
                n.variant < self.spec.variants.len()
                    && n.position
                        .iter()
                        .chain(&n.home)
                        .all(|n| n.unsigned_abs() <= 1_000_000),
                "cyclic atlas node bounds"
            );
        }
        Ok(())
    }
    ///Reset positions and assign frames in per-variant native list order.
    ///Drift phases retain their initialized state across material changes.
    pub fn reset(&mut self) -> Result<()> {
        self.validate()?;
        let mut frame = vec![0; self.spec.variants.len()];
        for n in &mut self.nodes {
            n.position = n.home;
            n.frame = frame[n.variant];
            frame[n.variant] = (frame[n.variant] + 1) % self.spec.variants[n.variant].frames;
        }
        Ok(())
    }
    ///One caller-admitted update. Cancellation discards the worker's candidate
    ///pool; the owning adapter commits it only with its image completion.
    pub fn step(&mut self, check: &dyn Fn() -> Result<()>) -> Result<Vec<AtlasProjection>> {
        self.validate()?;
        let mut projected = Vec::with_capacity(self.nodes.len());
        let [width, height] = self.spec.canvas.map(|n| n as i32);
        for n in &mut self.nodes {
            check()?;
            let v = &self.spec.variants[n.variant];
            let mut p = n.position;
            if p[1] > 0 {
                p[0] = p[0]
                    .wrapping_add(v.horizontal_step)
                    .wrapping_add(if v.drift.is_empty() {
                        0
                    } else {
                        v.drift[n.phase % v.drift.len()]
                    });
            }
            p[1] = p[1].wrapping_add(v.vertical_step);
            if !v.drift.is_empty() {
                n.phase = (n.phase + 1) % v.drift.len();
            }
            n.tick_phase = (n.tick_phase + 1) % 16;
            n.frame = (n.frame + 1) % v.frames;
            if p[0] >= width {
                p[0] = p[0].wrapping_sub(width + v.cell[0] as i32);
            }
            n.position = p;
            if p[1] >= height {
                if p[1] >= height + v.vertical_step {
                    n.position = n.home;
                }
                continue;
            }
            if p[0] >= -(v.cell[0] as i32) && p[1] >= -(v.cell[1] as i32) {
                projected.push(AtlasProjection {
                    position: p,
                    variant: n.variant,
                    source: [
                        0,
                        (n.frame * v.cell[1]) as i32,
                        v.cell[0] as i32,
                        v.cell[1] as i32,
                    ],
                });
            }
        }
        check()?;
        Ok(projected)
    }
}
