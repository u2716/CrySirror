// SPDX-License-Identifier: MPL-2.0
//! Per-character animation sets, `dc1.psarc/anim-sets.bin`.
//!
//! An `anim-set` is `{array, count}` of `{logical StringId, clip StringId}`
//! pairs. The `anim-npc` blend trees name logical animations (`0x950f8fb1`
//! in `s_ambient-idle`) that each character's set maps to its clips (the
//! `*flynn-anim-set*` maps it to `look-add--up-down` in `anim-flynn.pak`).
//! No module or the EBOOT stores which set a character uses; the set names
//! follow `*NAME-anim-set*` for 11 spawn-character and art-group names
//! (`flynn`, `flynn-heist`, `elena`, `chloe`, `sullivan`, `lazarevic`, …),
//! which is how a character's sets are chosen here, the art group's
//! overriding the character's (inferred,
//! SATD-nd-retail-runtime). The other sets (and the small `0x1a117490`
//! lists of sets) are unnamed.
use crate::dc::{Dc, sid};
use anyhow::{Result, ensure};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct AnimSets {
    sets: HashMap<u32, HashMap<u32, u32>>,
}

impl AnimSets {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut dc = Dc::default();
        let module = dc.load(sid("anim-sets"), bytes)?.clone();
        let m = &dc.mem;
        let mut sets = HashMap::new();
        for (name, a) in &module.entries {
            if m.type_of(*a).ok() != Some(sid("anim-set")) {
                continue;
            }
            let (array, count) = (m.u32(*a)?, m.u32(a + 4)?);
            ensure!(count <= 4096, "ND anim-set size");
            let map = (0..count).map(|i| Ok((m.u32(array + 8 * i)?, m.u32(array + 8 * i + 4)?))).collect::<Result<HashMap<_, _>>>()?;
            sets.insert(*name, map);
        }
        Ok(Self { sets })
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The sets named `*NAME-anim-set*` for `names` that have one, most
    /// specific first (`flynn-heist` overrides 4 of `flynn`'s 276).
    pub fn sets_for<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for s in names.into_iter().map(|n| sid(&format!("*{n}-anim-set*"))) {
            if self.sets.contains_key(&s) && !out.contains(&s) {
                out.push(s);
            }
        }
        out
    }

    /// The clip the first of `sets` mapping logical animation `logical`
    /// plays (itself when none maps it).
    pub fn resolve(&self, sets: &[u32], logical: u32) -> u32 {
        sets.iter().find_map(|s| self.sets.get(s).and_then(|m| m.get(&logical))).copied().unwrap_or(logical)
    }
}
