// SPDX-License-Identifier: MPL-2.0
//! Blend trees of the player's animation states (`anim-player.bin`).
//!
//! A state's node (state record +0x10) is one of:
//! - a blend node (type `StringId` `0xb249ac72`): `{left, right, flags,
//!   weight lambda, f32 1.0}`; the lambda gets `(node, info, phase,
//!   instance)` and returns the right child's weight;
//! - a select node (`0x1c3f471c`): `{default, alternatives array, flags,
//!   selector lambda, 0}`; the lambda gets `(node, info, instance)` and
//!   returns the child to evaluate (`default` or an element of the array);
//! - `anim-node-animation`: `{clip StringId, …}`.
//!
//! The lambdas only read two blocks the PPU fills per actor. `info` holds
//! the state's floats: for the locomotion states (from the clips their
//! blends lead to) +28/+32 lean right/left, +40 the walk blend, +44 the
//! downhill spine, +48/+52 stairs up/down; `+0x228` is a cycle phase.
//! `instance` holds enums at +36: +48 the movement type (the index of the
//! `#b35a5e92` table, `walk-hurt` 14), +52 the weapon (the `weapon-base`
//! overlay list, `no-weapon` 1, `long-gun` 3). A blend whose right side is
//! an additive clip (`…-add`, local deltas) adds it on top of the left
//! side. The flags words are not interpreted. Inputs the adapter does not
//! produce stay zero, which selects the default branches
//! (SATD-nd-retail-runtime).
use crate::dc::{Dc, sid};
use crate::script::{MAX_ARGS, Natives, Outcome, Status, Thread};
use anyhow::{Context, Result, bail, ensure};

pub const TYPE_BLEND: u32 = 0xb249_ac72;
pub const TYPE_SELECT: u32 = 0x1c3f_471c;
pub const TYPE_ANIMATION: u32 = sid("anim-node-animation");
/// The movement-type table: `{array, count}` of 0x6c-byte records with the
/// type's `StringId` at +0x40, its name at +0x48 and its index at +0x4c.
const MOVEMENT_TYPES: u32 = 0xb35a_5e92;
const INFO_BYTES: usize = 0x400;
const INSTANCE_BYTES: usize = 0x200;
const MAX_DEPTH: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    pub clip: u32,
    pub weight: f32,
    pub additive: bool,
}

/// Per-frame inputs: `(byte offset, value)` writes into the two blocks.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub info: Vec<(u32, f32)>,
    pub instance: Vec<(u32, u32)>,
    pub phase: f32,
}

pub struct Graph {
    pub dc: Dc,
    info: u32,
    instance: u32,
    /// Movement-type `StringId` by index.
    pub movement_types: Vec<u32>,
    /// Weapon `StringId` by index.
    pub weapons: Vec<u32>,
}

/// `lerp-scale` and `minmax`, the only natives the tree lambdas call
/// (PPU 0x1416cc and 0x14ab64).
struct TreeNatives {
    errors: Vec<String>,
}

const LERP_SCALE: u32 = 0;
const MINMAX: u32 = 1;

impl Natives for TreeNatives {
    fn call(&mut self, _dc: &mut Dc, id: u32, a: &[u32; MAX_ARGS], argc: u32) -> Result<(u32, Status)> {
        let f = |i: usize| f32::from_bits(a[i]);
        let v = match (id, argc) {
            // `t = clamp((x − a) / (b − a), 0, 1)`, `c (1 − t) + d t`.
            (LERP_SCALE, 5) => {
                let t = ((f(4) - f(0)) / (f(1) - f(0))).max(0.);
                let t = if t.is_nan() { 0. } else { t.min(1.) };
                (1. - t) * f(2) + t * f(3)
            }
            // `min(max(x, lo), hi)`.
            (MINMAX, 3) => f(0).max(f(1)).min(f(2)),
            _ => bail!("anim tree native {id} with {argc} arguments"),
        };
        Ok((v.to_bits(), Status::Continue))
    }
    fn lookup(&self, s: u32) -> Option<u32> {
        if s == sid("lerp-scale") {
            Some(LERP_SCALE)
        } else if s == sid("minmax") {
            Some(MINMAX)
        } else {
            None
        }
    }
    fn error(&mut self, message: String) {
        self.errors.push(message);
    }
}

impl Graph {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let mut dc = Dc::default();
        let module = dc.load(sid("anim-player"), bytes)?.clone();
        let entry = |name: u32| module.entries.iter().find(|(n, _)| *n == name).map(|(_, a)| *a);
        let m = &dc.mem;
        let table = entry(MOVEMENT_TYPES).context("anim-player.bin without its movement-type table")?;
        let (array, count) = (m.u32(table)?, m.u32(table + 4)?);
        ensure!(count <= 256, "ND movement-type count");
        let mut movement_types = vec![0; count as usize];
        for i in 0..count {
            let r = array + 0x6c * i;
            let index = m.u32(r + 0x4c)? as usize;
            ensure!(index < movement_types.len(), "ND movement-type index");
            movement_types[index] = m.u32(r + 0x40)?;
        }
        let mut weapons = Vec::new();
        for (_, a) in &module.entries {
            if m.type_of(*a).ok() == Some(sid("player-overlay-info")) && m.u32(a + 4).ok() == Some(sid("weapon-base")) {
                let n = m.u32(a + 8)?;
                ensure!(n <= 64, "ND weapon count");
                let list = m.u32(a + 0xc)?;
                weapons = (0..n).map(|i| m.u32(list + 8 * i + 4)).collect::<Result<_>>()?;
            }
        }
        let info = dc.mem.alloc(INFO_BYTES)?;
        let instance = dc.mem.alloc(INSTANCE_BYTES)?;
        Ok(Self { dc, info, instance, movement_types, weapons })
    }

    pub fn movement_index(&self, name: u32) -> Option<u32> {
        self.movement_types.iter().position(|s| *s == name).map(|i| i as u32)
    }

    pub fn weapon_index(&self, name: u32) -> Option<u32> {
        self.weapons.iter().position(|s| *s == name).map(|i| i as u32)
    }

    /// Every clip a tree can reach, for loading.
    pub fn clips(&self, node: u32) -> Vec<u32> {
        let mut out = Vec::new();
        self.collect(node, 0, &mut out);
        out.sort_unstable();
        out.dedup();
        out
    }

    fn collect(&self, node: u32, depth: usize, out: &mut Vec<u32>) {
        let m = &self.dc.mem;
        if node == 0 || depth > MAX_DEPTH {
            return;
        }
        match m.type_of(node) {
            Ok(TYPE_BLEND) => {
                for at in [node, node + 4] {
                    if let Ok(c) = m.u32(at) {
                        self.collect(c, depth + 1, out);
                    }
                }
            }
            Ok(TYPE_SELECT) => {
                if let Ok(c) = m.u32(node) {
                    self.collect(c, depth + 1, out);
                }
                if let Ok(array) = m.u32(node + 4).and_then(|a| if a == 0 { Ok(Vec::new()) } else { self.array(a) }) {
                    for c in array {
                        self.collect(c, depth + 1, out);
                    }
                }
            }
            Ok(TYPE_ANIMATION) => {
                if let Ok(c) = m.u32(node) {
                    out.push(c);
                }
            }
            _ => {}
        }
    }

    /// Alternatives of a select node. DC arrays carry no length; the
    /// elements are taken while they point at tree nodes.
    fn array(&self, at: u32) -> Result<Vec<u32>> {
        let m = &self.dc.mem;
        let mut out = Vec::new();
        for i in 0..64 {
            let Ok(v) = m.u32(at + 4 * i) else { break };
            if !matches!(m.type_of(v), Ok(TYPE_BLEND | TYPE_SELECT | TYPE_ANIMATION)) {
                break;
            }
            out.push(v);
        }
        Ok(out)
    }

    fn call(&mut self, lambda: u32, args: &[u32]) -> Result<u32> {
        let mut natives = TreeNatives { errors: Vec::new() };
        let mut thread = Thread::new(&self.dc, lambda, args)?;
        match thread.run(&mut self.dc, &mut natives)? {
            Outcome::Returned(v) if natives.errors.is_empty() => Ok(v),
            Outcome::Returned(_) => bail!("anim tree lambda: {}", natives.errors.join("; ")),
            other => bail!("anim tree lambda ended with {other:?}"),
        }
    }

    /// Evaluate a state's tree into weighted clips (`additive` tells which
    /// clips hold deltas). Leaves of the same clip are merged.
    pub fn evaluate(&mut self, node: u32, inputs: &Inputs, additive: &dyn Fn(u32) -> bool) -> Result<Vec<Leaf>> {
        let zero = vec![0u8; INFO_BYTES];
        for (block, len) in [(self.info, INFO_BYTES), (self.instance, INSTANCE_BYTES)] {
            for o in (0..len as u32).step_by(4) {
                self.dc.mem.write_u32(block + o, u32::from_be_bytes(zero[..4].try_into()?))?;
            }
        }
        for (o, v) in &inputs.info {
            ensure!((*o as usize) + 4 <= INFO_BYTES, "anim info offset");
            self.dc.mem.write_u32(self.info + o, v.to_bits())?;
        }
        for (o, v) in &inputs.instance {
            ensure!((*o as usize) + 4 <= INSTANCE_BYTES, "anim instance offset");
            self.dc.mem.write_u32(self.instance + o, *v)?;
        }
        let mut out = Vec::new();
        self.node(node, 1., inputs.phase, additive, 0, &mut out)?;
        let mut merged: Vec<Leaf> = Vec::with_capacity(out.len());
        for l in out {
            match merged.iter_mut().find(|m| m.clip == l.clip && m.additive == l.additive) {
                Some(m) => m.weight += l.weight,
                None => merged.push(l),
            }
        }
        merged.retain(|l| l.weight > 1e-4);
        Ok(merged)
    }

    fn node(&mut self, node: u32, weight: f32, phase: f32, additive: &dyn Fn(u32) -> bool, depth: usize, out: &mut Vec<Leaf>) -> Result<()> {
        ensure!(depth <= MAX_DEPTH, "ND anim tree depth");
        if node == 0 || weight <= 1e-4 {
            return Ok(());
        }
        let m = &self.dc.mem;
        match m.type_of(node)? {
            TYPE_BLEND => {
                let (left, right, lambda) = (m.u32(node)?, m.u32(node + 4)?, m.u32(node + 0xc)?);
                let w = if lambda == 0 { m.f32(node + 0x10)? } else { f32::from_bits(self.call(lambda, &[node, self.info, phase.to_bits(), self.instance])?) };
                let w = if w.is_finite() { w.clamp(0., 1.) } else { 0. };
                let mut right_leaves = Vec::new();
                self.node(right, 1., phase, additive, depth + 1, &mut right_leaves)?;
                let delta = !right_leaves.is_empty() && right_leaves.iter().all(|l| additive(l.clip));
                self.node(left, if delta { weight } else { weight * (1. - w) }, phase, additive, depth + 1, out)?;
                out.extend(right_leaves.into_iter().map(|l| Leaf { weight: l.weight * weight * w, additive: delta || l.additive, ..l }));
            }
            TYPE_SELECT => {
                let (default, lambda) = (m.u32(node)?, m.u32(node + 0xc)?);
                let target = if lambda == 0 { default } else { self.call(lambda, &[node, self.info, self.instance])? };
                self.node(if target == 0 { default } else { target }, weight, phase, additive, depth + 1, out)?;
            }
            TYPE_ANIMATION => out.push(Leaf { clip: m.u32(node)?, weight, additive: false }),
            other => bail!("ND anim node type {other:08x}"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(id: u32, args: &[f32]) -> f32 {
        let mut a = [0u32; MAX_ARGS];
        for (i, v) in args.iter().enumerate() {
            a[i] = v.to_bits();
        }
        let mut n = TreeNatives { errors: Vec::new() };
        f32::from_bits(n.call(&mut Dc::default(), id, &a, args.len() as u32).unwrap().0)
    }

    #[test]
    fn lerp_scale_and_minmax_follow_the_ppu_natives() {
        assert!((call(LERP_SCALE, &[0.4, 1., 0., 1., 0.7]) - 0.5).abs() < 1e-6);
        assert_eq!(call(LERP_SCALE, &[0.4, 1., 0., 1., 0.1]), 0.);
        assert_eq!(call(LERP_SCALE, &[0.4, 1., 2., 4., 5.]), 4.);
        assert_eq!(call(MINMAX, &[1.5, 0., 1.]), 1.);
        assert_eq!(call(MINMAX, &[-0.5, 0., 1.]), 0.);
        let n = TreeNatives { errors: Vec::new() };
        assert_eq!(n.lookup(sid("lerp-scale")), Some(LERP_SCALE));
        assert_eq!(n.lookup(sid("set-walk-type")), None);
    }
}
