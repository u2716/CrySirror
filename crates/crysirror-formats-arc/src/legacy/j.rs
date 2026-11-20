// SPDX-License-Identifier: MPL-2.0
//! KISS `warc` archives of Custom Maid 3D 2 (`GameData/*.arc`).
//!
//! Recovered from the reader in `cm3d2_x64.dll` (the function that checks the
//! version against 1000) and its hash-tree walker `0x18001bef0`:
//!
//! - header: magic `warc` `ff aa 45 f1`, i32 version (>= 1000), i32 block count;
//! - blocks: `{u32 id, u64 size}` followed by the block bytes. Block 2 holds
//!   the file data, block 3 a chunk with the name table, and blocks 0 and 1
//!   two directory hash trees of identical shape (block 1 uses a second hash
//!   family for lookups; listing only needs block 0);
//! - directory node: u32 entry-table offset, u32 entry size (16), u64 name
//!   hash, i32 directory count, i32 file count, u64; then sorted
//!   `{u64 hash, u64 offset}` entries, directories first. Directory offsets are
//!   relative to the current node, file offsets to the start of block 2;
//! - chunk (`0x180018ec0`): u32 method (1 = zlib), u32, u32 decoded size,
//!   u32 stored size, then the stored bytes;
//! - name table: repeated `{u64 hash, u32 UTF-16 unit count, units}`.

use super::*;
use anyhow::bail;
use std::collections::{BTreeMap, HashMap};

pub(crate) static FORMATS: &[Legacy] = &[Legacy {
    id: "kiss/warc",
    probe: Some(|s| magic(s, 0, b"warc\xff\xaa\x45\xf1")),
    open: warc,
}];

const CHUNK_HEADER: u64 = 16;
/// Directory nesting accepted before the tree is treated as cyclic.
const MAX_DEPTH: usize = 64;

struct Chunk {
    zlib: bool,
    decoded: u64,
    stored: u64,
}

fn chunk(source: &dyn AssetReader, context: &DecodeContext, at: u64) -> Result<Chunk> {
    let header = index(source, context, at, CHUNK_HEADER)?;
    let method = le32(&header, 0)?;
    ensure!(method <= 1, "unknown warc chunk method {method}");
    Ok(Chunk {
        zlib: method == 1,
        decoded: u64::from(le32(&header, 8)?),
        stored: u64::from(le32(&header, 12)?),
    })
}

fn le64(b: &[u8], at: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(at..at + 8).context("truncated warc field")?.try_into()?,
    ))
}

fn warc(source: &dyn AssetReader, context: &DecodeContext) -> Result<Vec<Row>> {
    let size = length(source)?;
    let header = index(source, context, 8, 8)?;
    let version = le32(&header, 0)? as i32;
    let count = le32(&header, 4)? as i32;
    ensure!(version >= 1000, "unsupported warc version {version}");
    ensure!((0..=64).contains(&count), "warc block count");
    let mut blocks = BTreeMap::new();
    let mut at = 16u64;
    for _ in 0..count {
        let record = index(source, context, at, 12)?;
        let id = le32(&record, 0)?;
        let length = le64(&record, 4)?;
        let start = at + 12;
        let end = start.checked_add(length).filter(|end| *end <= size).context("warc block outside archive")?;
        blocks.insert(id, (start, length));
        at = end;
    }
    let block = |id: u32| blocks.get(&id).copied().with_context(|| format!("warc block {id} missing"));
    let (data, data_size) = block(2)?;

    // Block 3: the name table chunk.
    let (names_at, _) = block(3)?;
    let names_chunk = chunk(source, context, names_at)?;
    let stored = index(source, context, names_at + CHUNK_HEADER, names_chunk.stored)?;
    let table = if names_chunk.zlib {
        let table = inflate_index(&stored, names_chunk.decoded.min(context.limits.max_index_size))?;
        ensure!(table.len() as u64 == names_chunk.decoded, "warc name table size");
        table
    } else {
        stored
    };
    let mut names = HashMap::new();
    let mut p = 0;
    while p < table.len() {
        let hash = le64(&table, p)?;
        let units = le32(&table, p + 8)? as usize;
        let raw = table.get(p + 12..p + 12 + 2 * units).context("truncated warc name")?;
        let units: Vec<u16> = raw.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        names.insert(hash, String::from_utf16(&units).context("invalid warc name")?);
        p += 12 + raw.len();
    }

    // Block 0: the directory tree.
    let (tree_at, tree_size) = block(0)?;
    let tree = index(source, context, tree_at, tree_size)?;
    let name = |hash: u64| names.get(&hash).cloned().with_context(|| format!("warc name hash {hash:#018x} missing"));
    let mut files = Vec::new();
    let mut pending = vec![(0usize, String::new(), 0usize)];
    while let Some((node, prefix, depth)) = pending.pop() {
        ensure!(depth <= MAX_DEPTH, "warc directory depth limit");
        let entries = le32(&tree, node)? as usize;
        ensure!(le32(&tree, node + 4)? == 16, "warc entry size");
        let directories = le32(&tree, node + 16)? as i32;
        let file_count = le32(&tree, node + 20)? as i32;
        ensure!(directories >= 0 && file_count >= 0, "warc node counts");
        let base = node.checked_add(entries).context("warc node offset")?;
        for index in 0..directories as usize + file_count as usize {
            let at = base + 16 * index;
            let hash = le64(&tree, at)?;
            let offset = le64(&tree, at + 8)?;
            let path = format!("{prefix}{}", name(hash)?);
            if index < directories as usize {
                let child = usize::try_from(offset)
                    .ok()
                    .and_then(|offset| node.checked_add(offset))
                    .filter(|child| *child < tree.len() && *child != node)
                    .context("warc directory outside tree")?;
                pending.push((child, format!("{path}/"), depth + 1));
            } else {
                ensure!(offset.checked_add(CHUNK_HEADER).is_some_and(|end| end <= data_size), "warc file outside data block");
                files.push((path, data + offset));
                limits(context, files.len())?;
            }
        }
    }
    files.sort();
    let mut rows = Vec::with_capacity(files.len());
    for (path, at) in files {
        let entry = chunk(source, context, at)?;
        let start = at + CHUNK_HEADER;
        if start.checked_add(entry.stored).is_none_or(|end| end > data + data_size) {
            bail!("warc file {path} outside data block");
        }
        rows.push(if entry.zlib {
            Row::packed(path, start, entry.stored, entry.decoded, Plan::Zlib)
        } else {
            ensure!(entry.decoded <= entry.stored, "warc stored entry size");
            Row::raw(path, start, entry.decoded)
        });
    }
    Ok(rows)
}
