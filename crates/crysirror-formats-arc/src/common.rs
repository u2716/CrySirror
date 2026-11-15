// SPDX-License-Identifier: GPL-3.0-or-later
// Archive layouts adapted from arc_unpacker, Copyright (C) 2016 rr-.
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::service::AssetReader;
use crysirror_resource::{
    concat, read_all, read_exact_at, subrange, Archive, ArchiveEntry, DecodeContext, TransformSpec,
};
use std::sync::Arc;

pub(crate) fn length(source: &dyn AssetReader) -> Result<u64> {
    source.len().map_err(|e| anyhow::anyhow!("{e}"))
}
pub(crate) fn bytes(
    source: &dyn AssetReader,
    offset: u64,
    size: u64,
    limit: u64,
) -> Result<Vec<u8>> {
    ensure!(size <= limit, "archive index/read byte limit");
    let source_size = length(source)?;
    ensure!(
        offset
            .checked_add(size)
            .is_some_and(|end| end <= source_size),
        "archive data range outside source"
    );
    let mut out = vec![0; usize::try_from(size).context("archive range exceeds address space")?];
    read_exact_at(source, offset, &mut out)?;
    Ok(out)
}
pub(crate) fn range(source_size: u64, offset: u64, size: u64) -> Result<()> {
    ensure!(
        offset
            .checked_add(size)
            .is_some_and(|end| end <= source_size),
        "archive entry range outside source"
    );
    Ok(())
}
pub(crate) struct Cursor<'a> {
    pub data: &'a [u8],
    pub at: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
    pub fn left(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).context("archive cursor overflow")?;
        ensure!(end <= self.data.len(), "truncated archive table");
        let out = &self.data[self.at..end];
        self.at = end;
        Ok(out)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn le16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn le32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn be32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn le64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn zero(&mut self) -> Result<&'a [u8]> {
        let size = self
            .data
            .get(self.at..)
            .context("archive cursor outside table")?
            .iter()
            .position(|b| *b == 0)
            .context("unterminated archive filename")?;
        ensure!(size <= 16 * 1024, "archive filename length limit");
        let out = self.take(size)?;
        self.take(1)?;
        Ok(out)
    }
}
pub(crate) fn sjis(bytes: &[u8]) -> Result<String> {
    let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!errors, "invalid CP932 archive filename");
    Ok(text.into_owned())
}
pub(crate) fn utf16(bytes: &[u8]) -> Result<String> {
    ensure!(bytes.len().is_multiple_of(2), "odd UTF16 filename size");
    let units: Vec<_> = bytes
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    String::from_utf16(&units).context("invalid UTF16 archive filename")
}
pub(crate) fn limits(context: &DecodeContext, count: usize) -> Result<()> {
    ensure!(
        count <= context.limits.max_entries && count <= u32::MAX as usize,
        "archive entry count limit"
    );
    Ok(())
}
#[derive(Clone)]
pub(crate) struct Segment {
    pub offset: u64,
    pub stored: u64,
    pub size: u64,
    pub compressed: bool,
}
pub(crate) enum EntryPlan {
    Raw {
        offset: u64,
    },
    Segments {
        segments: Vec<Segment>,
        encrypted: bool,
        facts: Xp3Facts,
    },
    NsaLzss {
        offset: u64,
    },
    /// Stored bytes run through shared engine transforms (zlib, lzss, ...).
    Transform {
        offset: u64,
        specs: Vec<TransformSpec>,
    },
    /// Format-specific byte decode whose output size is the entry size.
    Decode {
        offset: u64,
        decode: DecodeFn,
    },
    Profile {
        offset: u64,
    },
    Unsupported(&'static str),
}
pub(crate) type DecodeFn =
    Arc<dyn Fn(&[u8], &ArchiveEntry, &DecodeContext) -> Result<Vec<u8>> + Send + Sync>;
/// XP3 index facts of one entry, kept typed: a JSON object per entry costs
/// about 1.9 KiB, which made a 45k-entry voice index exceed 100 MiB.
/// [`Xp3Facts::metadata`] builds the published JSON view on request.
#[derive(Clone, Default)]
pub(crate) struct Xp3Facts {
    pub version: u8,
    pub flags: u32,
    pub adler32: u32,
    pub timestamp: Option<u64>,
    pub declared_size: u64,
    pub declared_stored_size: u64,
    /// Original `info` name when the published path came from an inline
    /// (M2/KrkrZ) filename record.
    pub info_name: Option<Box<str>>,
    /// Name the plain index exposed before CXDEC filename recovery renamed it.
    pub visible_name: Option<Box<str>>,
    /// Authenticated HXV4 hashed-path record id.
    pub hxv4_id: Option<u64>,
}
impl Xp3Facts {
    pub fn metadata(&self, entry: &ArchiveEntry, segment_count: usize) -> serde_json::Value {
        let mut out = serde_json::json!({"format":"kirikiri/xp3","version":self.version,"flags":self.flags,"adler32":self.adler32,"timestamp":self.timestamp,"segment_count":segment_count,"declared_size":self.declared_size,"declared_stored_size":self.declared_stored_size});
        if let Some(info) = &self.info_name {
            out["xp3_inline_name"] = serde_json::json!(entry.path);
            out["xp3_info_name"] = serde_json::json!(info);
        }
        if let Some(name) = &self.visible_name {
            out["visible_name"] = serde_json::json!(name);
        }
        if let Some(id) = self.hxv4_id {
            out["hxv4"] = serde_json::json!({"authenticated":true,"name":"hashed","id":id});
        }
        if let Some(extra) = entry.metadata.as_object() {
            for (k, v) in extra {
                out[k] = v.clone();
            }
        }
        out
    }
}
pub(crate) struct ParsedArchive {
    pub format: &'static str,
    pub source: Arc<dyn AssetReader>,
    pub entries: Vec<ArchiveEntry>,
    pub plans: Vec<EntryPlan>,
}
impl ParsedArchive {
    /// Typed XP3 facts of entry `id`.
    pub fn xp3(&self, id: u32) -> Option<&Xp3Facts> {
        match self.plans.get(id as usize)? {
            EntryPlan::Segments { facts, .. } => Some(facts),
            _ => None,
        }
    }
}
impl Archive for ParsedArchive {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn entry_metadata(&self, id: u32) -> serde_json::Value {
        let Some(entry) = self.entries.get(id as usize).filter(|e| e.id == id) else {
            return serde_json::Value::Null;
        };
        match self.plans.get(id as usize) {
            Some(EntryPlan::Segments { facts, segments, .. }) => facts.metadata(entry, segments.len()),
            _ => entry.metadata.clone(),
        }
    }
    fn open(&self, id: u32, context: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        let entry = self
            .entries
            .get(id as usize)
            .filter(|e| e.id == id)
            .context("archive entry id does not exist")?;
        ensure!(
            entry.size <= context.limits.max_entry_size,
            "archive entry decoded byte limit"
        );
        let (data, required) = match &self.plans[id as usize] {
            EntryPlan::Profile { offset } => (
                subrange(self.source.clone(), *offset, entry.stored_size)?,
                true,
            ),
            EntryPlan::Raw { offset } => (
                subrange(self.source.clone(), *offset, entry.stored_size)?,
                false,
            ),
            EntryPlan::Segments {
                segments,
                encrypted,
                ..
            } => {
                let mut readers = Vec::with_capacity(segments.len());
                for segment in segments {
                    let source = subrange(self.source.clone(), segment.offset, segment.stored)?;
                    let reader = if segment.compressed {
                        context.apply(
                            &TransformSpec {
                                algorithm: "zlib".into(),
                                key_id: None,
                                parameters: serde_json::Value::Null,
                                expected_size: Some(segment.size),
                            },
                            source,
                            &entry.metadata,
                        )?
                    } else {
                        ensure!(
                            segment.stored == segment.size,
                            "plain segment size mismatch"
                        );
                        source
                    };
                    readers.push(reader);
                }
                (concat(readers)?, *encrypted)
            }
            EntryPlan::NsaLzss { offset } => {
                let input = read_all(
                    subrange(self.source.clone(), *offset, entry.stored_size)?.as_ref(),
                    context.limits.max_resident_bytes,
                )?;
                (
                    context.decode_with(entry.size, || nsa_lzss(&input, entry.size))?,
                    false,
                )
            }
            EntryPlan::Transform { offset, specs } => {
                let mut reader = subrange(self.source.clone(), *offset, entry.stored_size)?;
                for spec in specs {
                    reader = context.apply(spec, reader, &entry.metadata)?;
                }
                (reader, false)
            }
            EntryPlan::Decode { offset, decode } => {
                let input = read_all(
                    subrange(self.source.clone(), *offset, entry.stored_size)?.as_ref(),
                    context.limits.max_resident_bytes,
                )?;
                (
                    context.decode_with(entry.size, || decode(&input, entry, context))?,
                    false,
                )
            }
            EntryPlan::Unsupported(reason) => bail!("{reason}"),
        };
        context.apply_pipeline(self.format, entry, data, required)
    }
}
/// NScripter's MSB-first bitwise LZSS: 8-bit dictionary address, 4-bit length+2,
/// zero-filled 256-byte ring, initial write position 239. Overlap is intentional.
fn nsa_lzss(input: &[u8], size: u64) -> Result<Vec<u8>> {
    let n = usize::try_from(size).context("LZSS output size overflow")?;
    let mut out = Vec::with_capacity(n);
    let mut ring = [0u8; 256];
    let mut write = 239usize;
    let mut bit = 0usize;
    let mut get = |count: usize| -> Result<u32> {
        ensure!(
            bit.checked_add(count)
                .is_some_and(|end| end <= input.len().saturating_mul(8)),
            "truncated NScripter LZSS bitstream"
        );
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from((input[bit / 8] >> (7 - bit % 8)) & 1);
            bit += 1;
        }
        Ok(value)
    };
    while out.len() < n {
        if get(1)? != 0 {
            let value = get(8)? as u8;
            out.push(value);
            ring[write] = value;
            write = (write + 1) & 255;
        } else {
            let mut read = get(8)? as usize;
            let count = get(4)? as usize + 2;
            for _ in 0..count.min(n - out.len()) {
                let value = ring[read];
                read = (read + 1) & 255;
                out.push(value);
                ring[write] = value;
                write = (write + 1) & 255;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod active_decode_context_tests {
    use super::*;
    use crysirror_core::{service::MemoryAssetReader, state::ClientId};
    #[test]
    fn entry_cpu_decoder_receives_the_active_scoped_open_context() -> Result<()> {
        let source = Arc::new(MemoryAssetReader(Arc::from([17u8])));
        let archive = ParsedArchive {
            format: "authored-context-forwarding",
            source,
            entries: vec![ArchiveEntry {
                id: 0,
                path: "entry.bin".into(),
                size: 1,
                stored_size: 1,
                metadata: serde_json::Value::Null,
            }],
            plans: vec![EntryPlan::Decode {
                offset: 0,
                decode: Arc::new(|bytes, entry, context| {
                    ensure!(context.client == ClientId(7), "stale decode client context");
                    ensure!(
                        context.parameters.get("active") == Some(&serde_json::json!(true)),
                        "stale decode options"
                    );
                    context.check_cancelled()?;
                    ensure!(entry.id == 0, "wrong native entry");
                    Ok(bytes.to_vec())
                }),
            }],
        };
        let mut current = DecodeContext::new(ClientId(7));
        current
            .parameters
            .insert("active".into(), serde_json::json!(true));
        let reader = archive.open(0, &current)?;
        assert_eq!(read_all(reader.as_ref(), 16)?, [17]);
        assert!(archive.open(0, &DecodeContext::new(ClientId(8))).is_err());
        Ok(())
    }
}
