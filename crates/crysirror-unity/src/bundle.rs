// SPDX-License-Identifier: MPL-2.0
//! UnityFS asset bundles (`UnityFS`, archive format 6–8): a header, a
//! (compressed) block/directory table and the data stream split into
//! independently compressed blocks (none, LZMA, LZ4/LZ4HC). Each directory
//! node (serialized file, `.resS`, `.resource`) is exposed as a random-access
//! [`AssetReader`] that decompresses only the blocks it touches, with a small
//! per-bundle cache.
//!
//! Layout (big-endian): `"UnityFS\0"`, u32 format, NUL-terminated player
//! version (`5.x.x`), NUL-terminated engine version, i64 total size, u32
//! compressed and u32 uncompressed size of the block info, u32 flags
//! (bits 0–5 compression, 0x40 directory combined with the blocks, 0x80
//! block info at the end, 0x200 block info padded to 16 bytes). Format ≥ 7
//! aligns the block info to 16 bytes after the header. Block info: 16-byte
//! hash, i32 block count, blocks `{u32 uncompressed, u32 compressed, u16
//! flags}`, i32 node count, nodes `{i64 offset, i64 size, u32 flags,
//! NUL-terminated path}`.
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::service::AssetReader;
use std::sync::{Arc, Mutex};

/// Bound of the decompressed block table.
const MAX_BLOCK_INFO: u32 = 64 * 1024 * 1024;
/// Bound of one decompressed block.
const MAX_BLOCK: u32 = 64 * 1024 * 1024;
const MAX_BLOCKS: usize = 1 << 20;
/// Decompressed blocks kept per bundle.
const CACHE_BLOCKS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    None,
    Lzma,
    Lz4,
}

fn compression(flags: u32) -> Result<Compression> {
    Ok(match flags & 0x3f {
        0 => Compression::None,
        1 => Compression::Lzma,
        2 | 3 => Compression::Lz4,
        other => bail!("unsupported UnityFS compression {other}"),
    })
}

#[derive(Clone, Copy, Debug)]
struct Block {
    uncompressed: u32,
    compressed: u32,
    compression: Compression,
    /// Offset of the compressed bytes in the bundle file.
    file_offset: u64,
    /// Offset of the decompressed bytes in the data stream.
    data_offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub flags: u32,
}

struct Inner {
    source: Arc<dyn AssetReader>,
    blocks: Vec<Block>,
    cache: Mutex<Vec<(usize, Arc<Vec<u8>>)>>,
}

pub struct Bundle {
    pub player_version: String,
    pub engine_version: String,
    pub nodes: Vec<Node>,
    inner: Arc<Inner>,
}

fn read_exact(source: &dyn AssetReader, at: u64, out: &mut [u8]) -> Result<()> {
    let mut done = 0;
    while done < out.len() {
        let n = source.read_at(at + done as u64, &mut out[done..]).map_err(|e| anyhow::anyhow!("UnityFS read: {e}"))?;
        ensure!(n > 0 && n <= out.len() - done, "truncated UnityFS bundle");
        done += n;
    }
    Ok(())
}

fn decompress(c: Compression, input: &[u8], size: usize) -> Result<Vec<u8>> {
    let out = match c {
        Compression::None => input.to_vec(),
        Compression::Lz4 => lz4_flex::block::decompress(input, size).map_err(|e| anyhow::anyhow!("UnityFS LZ4: {e}"))?,
        Compression::Lzma => {
            // Five property bytes, then raw LZMA of the known size.
            let mut out = Vec::with_capacity(size);
            let options = lzma_rs::decompress::Options {
                unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(size as u64)),
                ..Default::default()
            };
            lzma_rs::lzma_decompress_with_options(&mut std::io::Cursor::new(input), &mut out, &options).map_err(|e| anyhow::anyhow!("UnityFS LZMA: {e:?}"))?;
            out
        }
    };
    ensure!(out.len() == size, "UnityFS block decompressed to {} bytes, expected {size}", out.len());
    Ok(out)
}

/// Big-endian cursor over the header and block info.
struct Be<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Be<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.b.len()).context("truncated UnityFS table")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into()?))
    }
    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into()?))
    }
    fn cstring(&mut self) -> Result<String> {
        let rest = &self.b[self.at..];
        let n = rest.iter().position(|c| *c == 0).context("unterminated UnityFS string")?;
        let s = String::from_utf8_lossy(&rest[..n]).into_owned();
        self.at += n + 1;
        Ok(s)
    }
}

impl Bundle {
    pub fn open(source: Arc<dyn AssetReader>) -> Result<Self> {
        let length = source.len().map_err(|e| anyhow::anyhow!("UnityFS length: {e}"))?;
        let mut head = vec![0u8; (length.min(512)) as usize];
        read_exact(source.as_ref(), 0, &mut head)?;
        let mut r = Be { b: &head, at: 0 };
        ensure!(r.cstring()? == "UnityFS", "not a UnityFS bundle");
        let format = r.u32()?;
        ensure!((6..=8).contains(&format), "unsupported UnityFS format {format}");
        let player_version = r.cstring()?;
        let engine_version = r.cstring()?;
        let size = r.i64()?;
        ensure!(size as u64 == length, "UnityFS size {size} does not match the file ({length})");
        let (info_c, info_u, flags) = (r.u32()?, r.u32()?, r.u32()?);
        ensure!(info_u <= MAX_BLOCK_INFO && info_c <= MAX_BLOCK_INFO, "UnityFS block info size");
        let mut at = r.at as u64;
        if format >= 7 {
            at = at.div_ceil(16) * 16;
        }
        let info_at = if flags & 0x80 != 0 { length.checked_sub(u64::from(info_c)).context("UnityFS block info offset")? } else { at };
        let mut packed = vec![0u8; info_c as usize];
        read_exact(source.as_ref(), info_at, &mut packed)?;
        let info = decompress(compression(flags)?, &packed, info_u as usize)?;
        let mut data_at = if flags & 0x80 != 0 { at } else { at + u64::from(info_c) };
        if flags & 0x200 != 0 {
            data_at = data_at.div_ceil(16) * 16;
        }
        let mut t = Be { b: &info, at: 16 };
        let count = t.u32()? as usize;
        ensure!(count <= MAX_BLOCKS, "UnityFS block count");
        let mut blocks = Vec::with_capacity(count);
        let (mut file_offset, mut data_offset) = (data_at, 0u64);
        for _ in 0..count {
            let (uncompressed, compressed, bflags) = (t.u32()?, t.u32()?, t.u16()?);
            let c = compression(u32::from(bflags))?;
            // Stored blocks are read in place (uncompressed bundles keep
            // their whole data in one block).
            ensure!(c == Compression::None || (uncompressed <= MAX_BLOCK && compressed <= MAX_BLOCK), "UnityFS block size");
            ensure!(c != Compression::None || uncompressed == compressed, "UnityFS stored block sizes differ");
            blocks.push(Block { uncompressed, compressed, compression: c, file_offset, data_offset });
            file_offset += u64::from(compressed);
            data_offset += u64::from(uncompressed);
        }
        ensure!(file_offset <= length, "UnityFS blocks outside the file");
        let node_count = t.u32()? as usize;
        ensure!(node_count <= MAX_BLOCKS, "UnityFS node count");
        let mut nodes = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            let (offset, size, nflags) = (t.i64()?, t.i64()?, t.u32()?);
            let path = t.cstring()?;
            ensure!(offset >= 0 && size >= 0 && (offset as u64).checked_add(size as u64).is_some_and(|e| e <= data_offset), "UnityFS node {path} outside the data");
            nodes.push(Node { path, offset: offset as u64, size: size as u64, flags: nflags });
        }
        Ok(Self { player_version, engine_version, nodes, inner: Arc::new(Inner { source, blocks, cache: Mutex::new(Vec::new()) }) })
    }

    pub fn node(&self, path: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.path == path)
    }

    /// Random-access reader over one directory node.
    pub fn reader(&self, node: &Node) -> Arc<dyn AssetReader> {
        Arc::new(NodeReader { inner: self.inner.clone(), offset: node.offset, size: node.size })
    }
}

impl Inner {
    fn block(&self, i: usize) -> Result<Arc<Vec<u8>>> {
        {
            let cache = self.cache.lock().map_err(|_| anyhow::anyhow!("UnityFS cache lock"))?;
            if let Some((_, b)) = cache.iter().find(|(k, _)| *k == i) {
                return Ok(b.clone());
            }
        }
        let b = &self.blocks[i];
        let mut packed = vec![0u8; b.compressed as usize];
        read_exact(self.source.as_ref(), b.file_offset, &mut packed)?;
        let data = Arc::new(decompress(b.compression, &packed, b.uncompressed as usize)?);
        let mut cache = self.cache.lock().map_err(|_| anyhow::anyhow!("UnityFS cache lock"))?;
        cache.retain(|(k, _)| *k != i);
        if cache.len() >= CACHE_BLOCKS {
            cache.remove(0);
        }
        cache.push((i, data.clone()));
        Ok(data)
    }

    fn read(&self, at: u64, out: &mut [u8]) -> Result<usize> {
        let i = self.blocks.partition_point(|b| b.data_offset + u64::from(b.uncompressed) <= at);
        let Some(b) = self.blocks.get(i).copied() else { return Ok(0) };
        if b.compression == Compression::None {
            let start = at - b.data_offset;
            let n = (out.len() as u64).min(u64::from(b.uncompressed) - start) as usize;
            read_exact(self.source.as_ref(), b.file_offset + start, &mut out[..n])?;
            return Ok(n);
        }
        let data = self.block(i)?;
        let start = (at - b.data_offset) as usize;
        let n = out.len().min(data.len() - start);
        out[..n].copy_from_slice(&data[start..start + n]);
        Ok(n)
    }
}

struct NodeReader {
    inner: Arc<Inner>,
    offset: u64,
    size: u64,
}

impl AssetReader for NodeReader {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.size)
    }
    fn read_at(&self, offset: u64, output: &mut [u8]) -> crysirror_core::Result<usize> {
        if offset >= self.size {
            return Ok(0);
        }
        let n = (output.len() as u64).min(self.size - offset) as usize;
        self.inner.read(self.offset + offset, &mut output[..n]).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
