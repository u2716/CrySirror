// SPDX-License-Identifier: MPL-2.0
//! Plain PSARC 1.3, 64 KiB zlib profile. All multi-byte fields are big-endian.
//! The first record is the file-name manifest. Its names identify subsequent
//! records in table order; their MD5s are over the original UTF-8 path bytes.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
};
use crysirror_resource::{
    Archive, ArchiveEntry, ArchiveFormat, DecodeContext, Probe, read_exact_at,
};
use flate2::{Decompress, FlushDecompress, Status};
use std::{collections::HashSet, sync::Arc};

const BLOCK: u64 = 65536;
const HEADER: usize = 32;
const STRIDE: usize = 30;

pub(super) struct Psarc;

fn word(b: &[u8]) -> u32 {
    u32::from_be_bytes(b[..4].try_into().unwrap())
}
fn uint40(b: &[u8]) -> u64 {
    (u64::from(b[0]) << 32) | u64::from(word(&b[1..]))
}

#[derive(Clone, Copy)]
struct FileSpan {
    block: usize,
    size: u64,
}
struct Index {
    source: Arc<dyn AssetReader>,
    offsets: Vec<u64>,
}
struct FileView {
    index: Arc<Index>,
    file: FileSpan,
    context: DecodeContext,
}

impl FileView {
    fn read(&self, pos: u64, out: &mut [u8]) -> Result<usize> {
        let length = out
            .len()
            .min(usize::try_from(self.file.size.saturating_sub(pos)).unwrap_or(usize::MAX));
        let mut done = 0;
        while done < length {
            self.context.check_cancelled()?;
            let pos = pos + done as u64;
            let inner = (pos % BLOCK) as usize;
            let decoded = (self.file.size - (pos - inner as u64)).min(BLOCK) as usize;
            let block = self.file.block + (pos / BLOCK) as usize;
            let at = self.index.offsets[block];
            let stored = (self.index.offsets[block + 1] - at) as usize;
            let take = (decoded - inner).min(length - done);
            let dst = &mut out[done..done + take];
            if stored == decoded {
                read_exact_at(self.index.source.as_ref(), at + inner as u64, dst)?;
            } else {
                // The extra output byte distinguishes an oversized stream from a valid one.
                self.context
                    .with_temporary_bytes((stored + decoded + 1) as u64, || {
                        let mut input = vec![0; stored];
                        read_exact_at(self.index.source.as_ref(), at, &mut input)?;
                        let mut output = vec![0; decoded + 1];
                        let mut z = Decompress::new(true);
                        let end = z.decompress(&input, &mut output, FlushDecompress::Finish)?;
                        ensure!(
                            end == Status::StreamEnd
                                && z.total_in() == stored as u64
                                && z.total_out() == decoded as u64,
                            "invalid PSARC zlib block length"
                        );
                        dst.copy_from_slice(&output[inner..inner + take]);
                        Ok(())
                    })?;
            }
            done += take;
        }
        Ok(length)
    }
}
impl AssetReader for FileView {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.file.size)
    }
    fn read_at(&self, pos: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        self.read(pos, out)
            .map_err(|e| crysirror_core::Error::Io(format!("PSARC read: {e:#}")))
    }
}

struct Files {
    index: Arc<Index>,
    spans: Vec<FileSpan>,
    entries: Vec<ArchiveEntry>,
}
impl Archive for Files {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn open(&self, id: u32, context: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        context.check_cancelled()?;
        ensure!(id != 0, "PSARC internal manifest");
        let &file = self.spans.get(id as usize).context("missing PSARC entry")?;
        ensure!(
            file.size <= context.limits.max_entry_size,
            "PSARC entry exceeds configured limit"
        );
        Ok(Arc::new(FileView {
            index: self.index.clone(),
            file,
            context: context.clone(),
        }))
    }
}

impl ArchiveFormat for Psarc {
    fn id(&self) -> &'static str {
        "sony/psarc"
    }
    fn probe(&self, source: &dyn AssetReader) -> Result<Probe> {
        if source.len()? < HEADER as u64 {
            return Ok(Probe::NO);
        }
        let mut h = [0; HEADER];
        read_exact_at(source, 0, &mut h)?;
        Ok(
            if &h[..12] == b"PSAR\0\x01\0\x03zlib"
                && word(&h[16..]) == STRIDE as u32
                && word(&h[24..]) == BLOCK as u32
                && word(&h[28..]) == 0
            {
                Probe::YES
            } else {
                Probe::NO
            },
        )
    }

    fn open(
        &self,
        source: Arc<dyn AssetReader>,
        context: &DecodeContext,
    ) -> Result<Box<dyn Archive>> {
        ensure!(
            self.probe(source.as_ref())? == Probe::YES,
            "unsupported PSARC profile"
        );
        let mut h = [0; HEADER];
        read_exact_at(source.as_ref(), 0, &mut h)?;
        let toc = word(&h[12..]) as usize;
        let count = word(&h[20..]) as usize;
        ensure!(
            count > 0 && count - 1 <= context.limits.max_entries,
            "PSARC entry count"
        );
        ensure!(
            (HEADER..).contains(&toc)
                && toc as u64 <= context.limits.max_index_size
                && toc as u64 <= source.len()?,
            "PSARC table size"
        );
        let table_size = count
            .checked_mul(STRIDE)
            .context("PSARC file table overflow")?;
        let total = toc - HEADER;
        ensure!(
            table_size <= total && (total - table_size) % 2 == 0,
            "PSARC block table bounds"
        );
        let mut table = vec![0; total];
        read_exact_at(source.as_ref(), HEADER as u64, &mut table)?;
        let (records, stored) = table.split_at(table_size);
        let mut offset = toc as u64;
        let mut offsets = Vec::with_capacity(stored.len() / 2 + 1);
        offsets.push(offset);
        for bytes in stored.chunks_exact(2) {
            let len = u16::from_be_bytes(bytes.try_into()?);
            offset += if len == 0 { BLOCK } else { u64::from(len) };
            offsets.push(offset);
        }
        ensure!(offset <= source.len()?, "PSARC stored data outside source");
        let mut spans = Vec::with_capacity(count);
        let mut next_block = 0;
        for record in records.chunks_exact(STRIDE) {
            context.check_cancelled()?;
            let block = word(&record[16..]) as usize;
            let size = uint40(&record[20..25]);
            let start = uint40(&record[25..30]);
            ensure!(
                block == next_block && offsets.get(block) == Some(&start),
                "PSARC file/block offset mismatch"
            );
            let blocks = usize::try_from(size.div_ceil(BLOCK))?;
            next_block = block
                .checked_add(blocks)
                .context("PSARC block range overflow")?;
            ensure!(next_block < offsets.len(), "PSARC file exceeds block table");
            if blocks > 0 {
                ensure!(
                    offsets[next_block] - offsets[next_block - 1] <= (size - 1) % BLOCK + 1,
                    "oversized PSARC final block"
                );
            }
            spans.push(FileSpan { block, size });
        }
        ensure!(
            next_block == offsets.len() - 1 && records[..16] == [0; 16],
            "PSARC manifest or block count mismatch"
        );
        let index = Arc::new(Index { source, offsets });
        let manifest = FileView {
            index: index.clone(),
            file: spans[0],
            context: context.clone(),
        };
        ensure!(
            manifest.file.size <= context.limits.max_index_size,
            "PSARC manifest byte limit"
        );
        let mut bytes = vec![0; usize::try_from(manifest.file.size)?];
        manifest.read(0, &mut bytes)?;
        let mut lines = std::str::from_utf8(&bytes)?.lines();
        let mut entries = Vec::with_capacity(count - 1);
        let mut seen = HashSet::new();
        for id in 1..count {
            context.check_cancelled()?;
            let name = lines.next().context("short PSARC manifest")?;
            let path = VfsPath::new(Mount::Asset, name)?;
            ensure!(
                path.relative() == name && seen.insert(name),
                "duplicate or noncanonical PSARC name"
            );
            ensure!(
                crysirror_resource::checksum::md5_legacy(name.as_bytes())
                    == records[id * STRIDE..id * STRIDE + 16],
                "PSARC manifest name hash mismatch"
            );
            let f = spans[id];
            let end = f.block + f.size.div_ceil(BLOCK) as usize;
            entries.push(ArchiveEntry {
                id: id as u32,
                path: name.into(),
                size: f.size,
                stored_size: index.offsets[end] - index.offsets[f.block],
                metadata: Default::default(),
            });
        }
        ensure!(lines.next().is_none(), "extra PSARC manifest names");
        Ok(Box::new(Files {
            index,
            spans,
            entries,
        }))
    }
}
