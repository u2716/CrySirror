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
//!   `max`, `min` and maps `q / (2ⁿ − 1)` across them. Kind 6 adds three
//!   `u32` offsets and maps `((q << 2) + offset) / (4 · (2ᴺ − 1))`: the
//!   mesh's N-bit grid with two placement bits. Neighbouring kind-6
//!   fragments share seam vertices exactly only with the two-bit shift.
//!   N is the largest position depth of the mesh, raised when a kind-6
//!   value needs more (one installed mesh reaches `4 · (2²⁰ − 1)` exactly
//!   with depths up to 19). Inferred from data (SATD-nd-retail-runtime).
use crate::{geometry::decode_indices, package::Directory};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::service::AssetReader;
use crysirror_resource::read_exact_at;
use std::collections::HashSet;

pub const ATTR_POSITION: u8 = 1;

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

/// Quantized positions before the mesh grid is known.
struct Raw {
    kind: u8,
    depths: [u8; 3],
    min: [f32; 3],
    max: [f32; 3],
    values: Vec<[u64; 3]>,
    triangles: Vec<[u16; 3]>,
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

/// The position attribute row of a fragment, if it stores positions.
fn position_row(p: &Loaded, f: usize) -> Result<Option<(usize, usize, [u8; 8])>> {
    let desc = p.ptr(f + 0x28)?;
    let attributes = p.data[f + 0x2f] as usize;
    ensure!((1..=16).contains(&attributes), "ND fragment attribute count");
    for a in 0..attributes {
        let at = desc + 8 + 8 * a;
        let row: [u8; 8] = p.data.get(at..at + 8).context("ND descriptor outside package")?.try_into()?;
        if row[0] == ATTR_POSITION {
            return Ok(Some((desc, a, row)));
        }
    }
    Ok(None)
}

/// `None` for fragments without stored positions: the installed `post-56`
/// package has two-triangle quads that carry only texture coordinates.
fn raw_fragment(p: &Loaded, f: usize) -> Result<Option<Raw>> {
    let vertices = p.u32(p.ptr(f + 0x24)?)? as usize;
    ensure!(vertices <= 0x10000, "ND fragment vertex count");
    let Some((desc, a, row)) = position_row(p, f)? else { return Ok(None) };
    let (kind, n) = (row[1] >> 4, (row[1] & 15) as usize);
    ensure!(n == 3 && matches!(kind, 4 | 6), "ND position format {:#x}", row[1]);
    let depths = [row[4], row[5], row[6]];
    ensure!(depths.iter().all(|d| *d <= 24), "ND position bit depth");
    let param = desc + usize::from(u16::from_be_bytes([row[2], row[3]]));
    let max = [p.f32(param)?, p.f32(param + 4)?, p.f32(param + 8)?];
    let min = [p.f32(param + 12)?, p.f32(param + 16)?, p.f32(param + 20)?];
    let offset = if kind == 6 { [p.u32(param + 24)?, p.u32(param + 28)?, p.u32(param + 32)?] } else { [0; 3] };
    let values = unpack(&p.data, p.ptr(f + 0x50 + 8 * a)?, vertices, &depths)?
        .iter()
        .map(|r| std::array::from_fn(|k| if kind == 6 { (u64::from(r[k]) << 2) + u64::from(offset[k]) } else { u64::from(r[k]) }))
        .collect();
    let triangles = decode_indices(&p.data[p.ptr(f + 0x48)?..], usize::from(p.u16(f + 0x40)?), vertices)?;
    Ok(Some(Raw { kind, depths, min, max, values, triangles }))
}

fn resolve(raw: Raw, grid_bits: u32) -> Fragment {
    let grid = 4. * ((1u64 << grid_bits) - 1) as f64;
    let positions = raw
        .values
        .iter()
        .map(|v| {
            std::array::from_fn(|k| {
                let t = match (raw.kind, raw.depths[k]) {
                    (6, _) => v[k] as f64 / grid,
                    (_, 0) => 0.,
                    (_, d) => v[k] as f64 / ((1u64 << d) - 1) as f64,
                };
                raw.min[k] + t as f32 * (raw.max[k] - raw.min[k])
            })
        })
        .collect();
    Fragment { positions, triangles: raw.triangles }
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
        let raws = (0..fragments)
            .map(|i| raw_fragment(p, p.ptr(array + 4 * i)?))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>>>()?;
        let mut grid_bits = raws.iter().flat_map(|r| r.depths).max().unwrap_or(0) as u32;
        let top = raws.iter().filter(|r| r.kind == 6).flat_map(|r| r.values.iter().flatten()).copied().max().unwrap_or(0);
        while 4 * ((1u64 << grid_bits) - 1) < top {
            grid_bits += 1;
        }
        ensure!(grid_bits <= 26, "ND position grid");
        meshes.push(raws.into_iter().map(|r| resolve(r, grid_bits)).collect());
    }
    Ok(meshes)
}
