// SPDX-License-Identifier: MPL-2.0
//! Relocated PAK 0xa79 contents and background prototype meshes
//! (`PM_COLLECTION_2`).
//!
//! Pointers inside pages are page-relative words listed by the header's
//! fix-up table (`page, target page, offset`). [`Loaded`] resolves them to
//! file offsets and refuses reads of words that are not listed pointers.
//!
//! Mesh layout (offsets into the relocated item payloads):
//! - collection: `u32 count`, then `count` element pointers; element word 0
//!   points to the mesh header;
//! - mesh header: `u16` group and fragment counts at +4/+6, fragment
//!   pointer array at +0x14;
//! - fragment: vertex count block at +0x24, stream descriptor at +0x28,
//!   attribute count byte at +0x2f, triangle count `u16` at +0x40, index
//!   blob at +0x48, per-attribute stream pointers at +0x50 (stride 8);
//! - descriptor: 8-byte attributes from +8 (`id`, kind << 4 | components,
//!   `u16` parameter offset, bit depth per component). Kind 4 stores
//!   `max`, `min` and maps `q / (2ⁿ − 1)` across them; kind 6 adds three
//!   `u32` grid offsets and maps `(q + offset) / (2ᵍ − 1)`. On the installed
//!   levels `g` is the mesh's kind-4 position depth plus two (15 → 17,
//!   18 → 20): no kind-6 value exceeds that grid. Meshes without a kind-4
//!   fragment use 17 (SATD-nd-retail-runtime).
use crate::{geometry::decode_indices, package::Directory};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::service::AssetReader;
use crysirror_resource::read_exact_at;
use std::collections::HashSet;

pub const ATTR_POSITION: u8 = 1;
/// Kind-6 grid bits for a mesh without kind-4 positions.
const DEFAULT_GRID_BITS: u8 = 17;

pub struct Loaded {
    pub data: Vec<u8>,
    pub directory: Directory,
    pointers: HashSet<usize>,
}

fn be32(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(b.get(at..at + 4).context("ND read outside package")?.try_into()?))
}

impl Loaded {
    /// Read a package of at most `limit` bytes and resolve its pointers.
    pub fn read(source: &dyn AssetReader, limit: u64) -> Result<Self> {
        let directory = Directory::read(source)?;
        let len = source.len()?;
        ensure!(len <= limit, "ND package exceeds the load limit");
        let mut data = vec![0; len as usize];
        read_exact_at(source, 0, &mut data)?;
        let pages = (0..directory.page_count)
            .map(|i| be32(&data, 44 + 12 * i).map(|v| v as usize))
            .collect::<Result<Vec<_>>>()?;
        let (groups, table) = (be32(&data, 24)? as usize, be32(&data, 28)? as usize);
        ensure!(groups <= 64, "ND fix-up groups");
        let mut entries = 0usize;
        for g in 0..groups {
            entries += be32(&data, table + 12 * g + 8)? as usize;
        }
        let first = be32(&data, table + 4)? as usize;
        ensure!(entries <= 4 << 20, "ND fix-up count");
        let mut pointers = HashSet::with_capacity(entries);
        for e in 0..entries {
            let at = first + 8 * e;
            let row = data.get(at..at + 8).context("ND fix-up outside package")?;
            let (from, to) = (u16::from_be_bytes([row[0], row[1]]) as usize, u16::from_be_bytes([row[2], row[3]]) as usize);
            let offset = u32::from_be_bytes(row[4..8].try_into()?) as usize;
            let slot = pages.get(from).context("ND fix-up page")? + offset;
            let target = pages.get(to).context("ND fix-up target page")? + be32(&data, slot)? as usize;
            ensure!(target < directory.texture_base as usize, "ND pointer outside pages");
            data[slot..slot + 4].copy_from_slice(&u32::try_from(target)?.to_be_bytes());
            pointers.insert(slot);
        }
        Ok(Self { data, directory, pointers })
    }

    pub fn ptr(&self, at: usize) -> Result<usize> {
        ensure!(self.pointers.contains(&at), "ND word at {at:#x} is not a pointer");
        Ok(be32(&self.data, at)? as usize)
    }
    pub fn u32(&self, at: usize) -> Result<u32> {
        be32(&self.data, at)
    }
    pub fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_be_bytes(self.data.get(at..at + 2).context("ND read outside package")?.try_into()?))
    }
    pub fn f32(&self, at: usize) -> Result<f32> {
        Ok(f32::from_bits(be32(&self.data, at)?))
    }
}

#[derive(Debug)]
pub struct Fragment {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u16; 3]>,
}

/// Unpack `count` records of MSB-first unsigned fields.
fn unpack(data: &[u8], at: usize, count: usize, depths: &[u8]) -> Result<Vec<[u32; 4]>> {
    let bits: usize = depths.iter().map(|d| *d as usize).sum();
    let bytes = data.get(at..at + (count * bits).div_ceil(8)).context("ND vertex stream outside package")?;
    let mut pos = 0usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let mut row = [0u32; 4];
        for (k, &d) in depths.iter().enumerate() {
            let mut v = 0u32;
            for _ in 0..d {
                v = v << 1 | u32::from(bytes[pos >> 3] >> (7 - (pos & 7)) & 1);
                pos += 1;
            }
            row[k] = v;
        }
        out.push(row);
    }
    Ok(out)
}

/// The position attribute row of a fragment.
fn position_row(p: &Loaded, f: usize) -> Result<(usize, usize, [u8; 8])> {
    let desc = p.ptr(f + 0x28)?;
    let attributes = p.data[f + 0x2f] as usize;
    ensure!((1..=16).contains(&attributes), "ND fragment attribute count");
    for a in 0..attributes {
        let at = desc + 8 + 8 * a;
        let row: [u8; 8] = p.data.get(at..at + 8).context("ND descriptor outside package")?.try_into()?;
        if row[0] == ATTR_POSITION {
            return Ok((desc, a, row));
        }
    }
    bail!("ND fragment without positions")
}

fn fragment(p: &Loaded, f: usize, grid_bits: u8) -> Result<Fragment> {
    let vertices = p.u32(p.ptr(f + 0x24)?)? as usize;
    ensure!(vertices <= 0x10000, "ND fragment vertex count");
    let (desc, a, row) = position_row(p, f)?;
    let (kind, n) = (row[1] >> 4, (row[1] & 15) as usize);
    ensure!(n == 3, "ND position components");
    let depths = &row[4..7];
    ensure!(depths.iter().all(|d| *d <= 24), "ND position bit depth");
    let param = desc + usize::from(u16::from_be_bytes([row[2], row[3]]));
    let max = [p.f32(param)?, p.f32(param + 4)?, p.f32(param + 8)?];
    let min = [p.f32(param + 12)?, p.f32(param + 16)?, p.f32(param + 20)?];
    let q = unpack(&p.data, p.ptr(f + 0x50 + 8 * a)?, vertices, depths)?;
    let map = |k: usize, v: f64| min[k] + (v as f32) * (max[k] - min[k]);
    let positions: Vec<[f32; 3]> = match kind {
        4 => q
            .iter()
            .map(|r| std::array::from_fn(|k| map(k, if depths[k] == 0 { 0. } else { f64::from(r[k]) / f64::from((1u32 << depths[k]) - 1) })))
            .collect(),
        6 => {
            let o = [p.u32(param + 24)?, p.u32(param + 28)?, p.u32(param + 32)?];
            let grid = f64::from((1u32 << grid_bits) - 1);
            let mut out = Vec::with_capacity(q.len());
            for r in &q {
                let g: [u64; 3] = std::array::from_fn(|k| u64::from(r[k]) + u64::from(o[k]));
                ensure!(g.iter().all(|v| *v as f64 <= grid), "ND kind-6 position outside the mesh grid");
                out.push(std::array::from_fn(|k| map(k, g[k] as f64 / grid)));
            }
            out
        }
        k => bail!("ND position kind {k}"),
    };
    let triangles = decode_indices(&p.data[p.ptr(f + 0x48)?..], usize::from(p.u16(f + 0x40)?), vertices)?;
    Ok(Fragment { positions, triangles })
}

/// Every mesh of a `PM_COLLECTION_2` payload, as fragments.
pub fn prototype_meshes(p: &Loaded, payload: usize) -> Result<Vec<Vec<Fragment>>> {
    let count = p.u32(payload)? as usize;
    ensure!(count <= 4096, "ND prototype count");
    let mut meshes = Vec::with_capacity(count);
    for e in 0..count {
        let header = p.ptr(p.ptr(payload + 4 + 4 * e)?)?;
        let fragments = usize::from(p.u16(header + 6)?);
        let array = p.ptr(header + 0x14)?;
        let frags = (0..fragments).map(|i| p.ptr(array + 4 * i)).collect::<Result<Vec<_>>>()?;
        let mut grid_bits = None;
        for &f in &frags {
            let (_, _, row) = position_row(p, f)?;
            if row[1] >> 4 == 4 {
                grid_bits = grid_bits.max(row[4..7].iter().max().map(|d| d + 2));
            }
        }
        let grid_bits = grid_bits.unwrap_or(DEFAULT_GRID_BITS);
        ensure!(grid_bits <= 26, "ND position grid");
        meshes.push(frags.iter().map(|&f| fragment(p, f, grid_bits)).collect::<Result<Vec<_>>>()?);
    }
    Ok(meshes)
}
