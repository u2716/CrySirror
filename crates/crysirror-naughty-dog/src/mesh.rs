// SPDX-License-Identifier: MPL-2.0
//! Relocated PAK 0xa79 contents and background prototype meshes
//! (`PM_COLLECTION_2`).
//!
//! Pointers inside pages are page-relative words listed by the header's
//! fix-up table (`page, target page, offset`). [`Loaded`] resolves them to
//! file offsets and refuses reads of words that are not listed pointers.
//!
//! Mesh layout (offsets into the relocated item payloads):
//! - collection: `u32 count`, then `count` record pointers. A record holds
//!   the mesh header, a pointer to its instance-pointer list and the
//!   instance count (twice). An instance carries a culling sphere at +0x30
//!   and a row-major 4×4 world matrix in metres at +0x40 (translation in
//!   the last row); the mesh's native units are centimetres;
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
//!   Texture coordinates (attribute 0x11) use the same two kinds. Normals
//!   (attribute 0x02) of kind 0 are three unsigned bytes mapped by
//!   `q / 127.5 − 1`: on `menu-temple` they are unit length and agree with
//!   the triangle winding. Other normal kinds are not decoded.
//! - mesh groups at header +0x10 (count `u16` at +4), 16 bytes each:
//!   pointer to a group whose word 0 is the material, then `u16` first
//!   fragment and fragment count;
//! - material: name, pass count at +0xc, pass pointers at +0x10. Pass
//!   data (pass +0x10) holds the sampler count at +0xc and samplers at +0x18,
//!   16 bytes each, whose word 0 points to a texture ID equal to word 3 of
//!   the matching `VRAM_DESC`. Sampler 1 of pass 0 is the base colour on the
//!   installed materials; slot 0 and the last slot are runtime
//!   placeholders.
use crate::{geometry::decode_indices, package::Directory};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::AssetReader;
use crysirror_resource::read_exact_at;
use std::collections::HashSet;

pub const ATTR_POSITION: u8 = 1;
pub const ATTR_UV: u8 = 0x11;
pub const ATTR_NORMAL: u8 = 0x02;
/// Three unsigned bytes per vertex with 7–24 distinct values per fragment on
/// `menu-temple`: layer blend masks of two-layer materials.
pub const ATTR_LAYER_MASK: u8 = 0x15;
/// Texture coordinates are stored in 1/256 of the texture: uniquely
/// mapped statue fragments of `menu-temple` span at most 256 units.
const UV_UNIT: f32 = 256.;

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

    /// Offsets of the relocated pointer words.
    pub fn pointers(&self) -> impl Iterator<Item = usize> + '_ {
        self.pointers.iter().copied()
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
    /// Unit normals, or empty when the fragment's normal kind is unknown.
    pub normals: Vec<[f32; 3]>,
    /// Layer masks (attribute 0x15) in 0..1, or empty when not stored per
    /// vertex.
    pub masks: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u16; 3]>,
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    /// Texture IDs of pass 0's samplers (`VRAM_DESC` word 3).
    pub samplers: Vec<u32>,
}

impl Material {
    pub fn base_color(&self) -> Option<u32> {
        self.samplers.get(1).copied()
    }
}

#[derive(Debug)]
pub struct Group {
    pub material: Material,
    pub fragments: std::ops::Range<usize>,
}

#[derive(Debug)]
pub struct Mesh {
    pub fragments: Vec<Fragment>,
    pub groups: Vec<Group>,
    /// Row-major world matrices in metres (`p' = p · M`, translation in
    /// row 3), one per placed instance.
    pub instances: Vec<[f32; 16]>,
    /// Each instance's culling sphere (centre in metres, radius).
    pub spheres: Vec<[f32; 4]>,
}

/// One quantized attribute before its mesh grid is known.
struct Raw<const N: usize> {
    kind: u8,
    depths: [u8; N],
    min: [f32; N],
    max: [f32; N],
    values: Vec<[u64; N]>,
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

/// Decode attribute `id` with `N` components of fragment `f`, if present.
fn raw_attribute<const N: usize>(p: &Loaded, f: usize, id: u8, vertices: usize) -> Result<Option<Raw<N>>> {
    let desc = p.ptr(f + 0x28)?;
    let attributes = p.data[f + 0x2f] as usize;
    ensure!((1..=32).contains(&attributes), "ND fragment attribute count");
    for a in 0..attributes {
        let at = desc + 8 + 8 * a;
        let row: [u8; 8] = p.data.get(at..at + 8).context("ND descriptor outside package")?.try_into()?;
        if row[0] != id {
            continue;
        }
        let kind = row[1] >> 4;
        ensure!((row[1] & 15) as usize == N && matches!(kind, 4 | 6), "ND attribute {id:#x} format {:#x}", row[1]);
        let depths: [u8; N] = std::array::from_fn(|k| row[4 + k]);
        ensure!(depths.iter().all(|d| *d <= 24), "ND attribute bit depth");
        let param = desc + usize::from(u16::from_be_bytes([row[2], row[3]]));
        let max = (0..N).map(|k| p.f32(param + 4 * k)).collect::<Result<Vec<_>>>()?;
        let min = (0..N).map(|k| p.f32(param + 4 * (N + k))).collect::<Result<Vec<_>>>()?;
        let offset = (0..N).map(|k| if kind == 6 { p.u32(param + 4 * (2 * N + k)) } else { Ok(0) }).collect::<Result<Vec<_>>>()?;
        let values = unpack(&p.data, p.ptr(f + 0x50 + 8 * a)?, vertices, &depths)?
            .iter()
            .map(|r| std::array::from_fn(|k| if kind == 6 { (u64::from(r[k]) << 2) + u64::from(offset[k]) } else { u64::from(r[k]) }))
            .collect();
        return Ok(Some(Raw { kind, depths, min: std::array::from_fn(|k| min[k]), max: std::array::from_fn(|k| max[k]), values }));
    }
    Ok(None)
}

/// Kind-0 byte normals of fragment `f`, if present.
fn byte_normals(p: &Loaded, f: usize, vertices: usize) -> Result<Option<Vec<[f32; 3]>>> {
    Ok(bytes3(p, f, vertices, ATTR_NORMAL)?.map(|v| v.iter().map(|v| v.map(|x| x / 127.5 - 1.)).collect()))
}

/// Kind-0 layer masks of fragment `f` in 0..1, if present.
fn layer_masks(p: &Loaded, f: usize, vertices: usize) -> Result<Option<Vec<[f32; 3]>>> {
    Ok(bytes3(p, f, vertices, ATTR_LAYER_MASK)?.map(|v| v.iter().map(|v| v.map(|x| x / 255.)).collect()))
}

/// Attribute `id` stored as kind 0 with three 8-bit components, as raw
/// byte values.
fn bytes3(p: &Loaded, f: usize, vertices: usize, id: u8) -> Result<Option<Vec<[f32; 3]>>> {
    let desc = p.ptr(f + 0x28)?;
    for a in 0..p.data[f + 0x2f] as usize {
        let at = desc + 8 + 8 * a;
        let row: [u8; 8] = p.data.get(at..at + 8).context("ND descriptor outside package")?.try_into()?;
        if row[0] != id {
            continue;
        }
        if row[1] != 0x03 || row[4..7] != [8, 8, 8] {
            return Ok(None);
        }
        let values = unpack(&p.data, p.ptr(f + 0x50 + 8 * a)?, vertices, &row[4..7])?;
        return Ok(Some(values.iter().map(|v| std::array::from_fn(|k| v[k] as f32)).collect()));
    }
    Ok(None)
}

/// The mesh grid: the largest depth, raised until every offset value fits.
fn grid_bits<const N: usize>(raws: &[&Raw<N>]) -> Result<u32> {
    let mut bits = raws.iter().flat_map(|r| r.depths).max().unwrap_or(0) as u32;
    let top = raws.iter().filter(|r| r.kind == 6).flat_map(|r| r.values.iter().flatten()).copied().max().unwrap_or(0);
    while 4 * ((1u64 << bits) - 1) < top {
        bits += 1;
    }
    ensure!(bits <= 26, "ND attribute grid");
    Ok(bits)
}

fn resolve<const N: usize>(raw: &Raw<N>, grid_bits: u32, scale: f32) -> Vec<[f32; N]> {
    let grid = 4. * ((1u64 << grid_bits) - 1) as f64;
    raw.values
        .iter()
        .map(|v| {
            std::array::from_fn(|k| {
                let t = match (raw.kind, raw.depths[k]) {
                    (6, _) => v[k] as f64 / grid,
                    (_, 0) => 0.,
                    (_, d) => v[k] as f64 / ((1u64 << d) - 1) as f64,
                };
                (raw.min[k] + t as f32 * (raw.max[k] - raw.min[k])) * scale
            })
        })
        .collect()
}

fn cstring(p: &Loaded, at: usize) -> Result<String> {
    let bytes = p.data.get(at..).context("ND string outside package")?;
    let end = bytes.iter().take(1024).position(|b| *b == 0).context("unterminated ND string")?;
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn material(p: &Loaded, m: usize) -> Result<Material> {
    let name = cstring(p, p.ptr(m)?)?;
    let passes = p.u32(m + 0xc)?;
    ensure!((1..=64).contains(&passes), "ND material pass count");
    let data = p.ptr(p.ptr(p.ptr(m + 0x10)?)? + 0x10)?;
    let count = usize::from(p.u16(data + 0xc)?);
    ensure!(count <= 16, "ND sampler count");
    let samplers = if count == 0 {
        Vec::new()
    } else {
        let table = p.ptr(data + 0x18)?;
        (0..count).map(|s| p.u32(p.ptr(table + 16 * s)?)).collect::<Result<Vec<_>>>()?
    };
    Ok(Material { name, samplers })
}

/// First word of the collision-mesh collections in actor packages (the
/// foreground login code tests the same value); these hold no render meshes.
pub const COLLISION_COLLECTION: usize = 0x1001;

/// Every mesh of a `PM_COLLECTION_2` payload. Fragments without stored
/// positions (two-triangle quads with only texture coordinates in the
/// installed `post-56` package) decode as empty fragments so group ranges
/// keep their native indices.
pub fn prototype_meshes(p: &Loaded, payload: usize) -> Result<Vec<Mesh>> {
    let count = p.u32(payload)? as usize;
    if count == COLLISION_COLLECTION {
        return Ok(Vec::new());
    }
    ensure!(count <= 4096, "ND prototype count");
    let mut meshes = Vec::with_capacity(count);
    for e in 0..count {
        let record = p.ptr(payload + 4 + 4 * e)?;
        let header = p.ptr(record)?;
        let placed = p.u32(record + 8)? as usize;
        ensure!(placed <= 65536, "ND instance count");
        let (mut instances, mut spheres) = (Vec::with_capacity(placed), Vec::with_capacity(placed));
        if placed > 0 {
            let list = p.ptr(record + 4)?;
            for i in 0..placed {
                let at = p.ptr(list + 4 * i)?;
                let m: [f32; 16] = std::array::from_fn(|k| p.f32(at + 0x40 + 4 * k).unwrap_or(f32::NAN));
                let s: [f32; 4] = std::array::from_fn(|k| p.f32(at + 0x30 + 4 * k).unwrap_or(f32::NAN));
                ensure!(m.iter().chain(&s).all(|v| v.is_finite()) && (m[15] - 1.).abs() < 1e-3, "ND instance matrix");
                instances.push(m);
                spheres.push(s);
            }
        }
        let (group_count, fragments) = (usize::from(p.u16(header + 4)?), usize::from(p.u16(header + 6)?));
        let array = p.ptr(header + 0x14)?;
        let mut raws = Vec::with_capacity(fragments);
        for i in 0..fragments {
            let f = p.ptr(array + 4 * i)?;
            let vertices = p.u32(p.ptr(f + 0x24)?)? as usize;
            ensure!(vertices <= 0x10000, "ND fragment vertex count");
            let Some(pos) = raw_attribute::<3>(p, f, ATTR_POSITION, vertices)? else {
                raws.push(None);
                continue;
            };
            // One installed prop (`ice-t-prayer-wheel-gears`) stores
            // coordinates in format 0xb8, not yet decoded: it keeps no UVs.
            let uv = raw_attribute::<2>(p, f, ATTR_UV, vertices).ok().flatten();
            let normals = byte_normals(p, f, vertices)?.unwrap_or_default();
            let masks = layer_masks(p, f, vertices)?.unwrap_or_default();
            let triangles = decode_indices(&p.data[p.ptr(f + 0x48)?..], usize::from(p.u16(f + 0x40)?), vertices)?;
            raws.push(Some((pos, uv, normals, masks, triangles)));
        }
        let pos_grid = grid_bits(&raws.iter().flatten().map(|r| &r.0).collect::<Vec<_>>())?;
        let uv_grid = grid_bits(&raws.iter().flatten().filter_map(|r| r.1.as_ref()).collect::<Vec<_>>())?;
        let fragments = raws
            .into_iter()
            .map(|r| match r {
                None => Fragment { positions: Vec::new(), normals: Vec::new(), masks: Vec::new(), uvs: Vec::new(), triangles: Vec::new() },
                Some((pos, uv, normals, masks, triangles)) => Fragment {
                    masks,
                    positions: resolve(&pos, pos_grid, 1.),
                    normals,
                    uvs: uv.map(|u| resolve(&u, uv_grid, 1. / UV_UNIT)).unwrap_or_default(),
                    triangles,
                },
            })
            .collect::<Vec<_>>();
        let groups_at = p.ptr(header + 0x10)?;
        let mut groups = Vec::with_capacity(group_count);
        for g in 0..group_count {
            let row = groups_at + 16 * g;
            let (first, n) = (usize::from(p.u16(row + 4)?), usize::from(p.u16(row + 6)?));
            ensure!(first + n <= fragments.len(), "ND mesh group range");
            groups.push(Group { material: material(p, p.ptr(p.ptr(row)?)?)?, fragments: first..first + n });
        }
        meshes.push(Mesh { fragments, groups, instances, spheres });
    }
    Ok(meshes)
}
