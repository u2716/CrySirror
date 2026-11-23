// SPDX-License-Identifier: MPL-2.0
//! AiFS v5 numeric Ogg directory and its sibling GRP data volume.
use crate::{
    Archive, ArchiveEntry, ArchiveFormat, DecodeContext, Probe, read_all, read_exact_at, subrange,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::AssetReader;
use std::sync::Arc;

pub struct ExhibitVoice;
struct Voices {
    entries: Vec<ArchiveEntry>,
    offsets: Vec<u64>,
    volume: Arc<dyn AssetReader>,
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .context("truncated AiFS directory")?
            .try_into()?,
    ))
}

impl ArchiveFormat for ExhibitVoice {
    fn id(&self) -> &'static str {
        "exhibit/aifs-voice"
    }
    fn probe(&self, source: &dyn AssetReader) -> Result<Probe> {
        if source.len()? < 32 {
            return Ok(Probe::NO);
        }
        let mut h = [0; 16];
        read_exact_at(source, 0, &mut h)?;
        Ok(
            if &h[..4] == b"AiFS"
                && u32_at(&h, 4)? == 0x50000
                && u32_at(&h, 8)? == 32
                && u32_at(&h, 12)? == 1
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
            self.probe(&*source)?.confidence != 0,
            "unsupported AiFS directory profile"
        );
        let bytes = read_all(&*source, context.limits.max_index_size)?;
        let mut at = 16;
        if u32_at(&bytes, at)? == 0x0100_0000 {
            at += 4;
        }
        ensure!(u32_at(&bytes, at)? == 1, "unsupported AiFS volume index");
        let first = u32_at(&bytes, at + 4)?;
        let last = u32_at(&bytes, at + 8)?;
        let count = u32_at(&bytes, at + 12)? as usize;
        at += 16;
        ensure!(
            count > 0
                && count <= context.limits.max_entries
                && last.checked_sub(first).and_then(|n| n.checked_add(1)) == Some(count as u32),
            "invalid AiFS numeric range"
        );
        let rows = bytes.get(at..).context("missing AiFS entries")?;
        ensure!(
            rows.len() / 8 == count && rows.len() % 8 == 0,
            "invalid AiFS directory length"
        );
        let name = context
            .source_name
            .as_deref()
            .and_then(|n| n.rsplit('/').next())
            .context("AiFS needs its logical source name")?;
        ensure!(
            name.eq_ignore_ascii_case("res2000.grp"),
            "unsupported AiFS resource bank"
        );
        let volume = context.open_sibling("res2001.grp")?;
        let limit = volume.len()?;
        let mut entries = Vec::new();
        let mut offsets = Vec::new();
        for (i, row) in rows.chunks_exact(8).enumerate() {
            if i % 1024 == 0 {
                context.check_cancelled()?
            }
            let offset = u64::from(u32_at(row, 0)?);
            let size = u64::from(u32_at(row, 4)?);
            if size == 0 {
                continue;
            }
            ensure!(
                offset + size <= limit && size <= context.limits.max_entry_size,
                "AiFS voice outside volume or budget"
            );
            let id = first + i as u32;
            entries.push(ArchiveEntry {
                id: entries.len() as u32,
                path: format!("{id:05}.ogg"),
                size,
                stored_size: size,
                metadata: serde_json::Value::Null,
            });
            offsets.push(offset);
        }
        Ok(Box::new(Voices {
            entries,
            offsets,
            volume,
        }))
    }
}
impl Archive for Voices {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn open(&self, id: u32, context: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        context.check_cancelled()?;
        let i = id as usize;
        let entry = self.entries.get(i).context("unknown AiFS voice")?;
        ensure!(
            entry.size <= context.limits.max_entry_size,
            "AiFS voice exceeds budget"
        );
        let stream = subrange(self.volume.clone(), self.offsets[i], entry.size)?;
        let mut signature = [0u8; 4];
        read_exact_at(&*stream, 0, &mut signature)?;
        ensure!(&signature == b"OggS", "invalid AiFS voice stream");
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::state::ClientId;

    #[test]
    fn sparse_ids_use_sibling_offsets_and_reject_cross_volume_ranges() -> Result<()> {
        for padded in [false, true] {
            let mut cx = DecodeContext::new(ClientId(1));
            cx.source_name = Some("res/res2000.grp".into());
            let volume = cx.memory(b"xxOggS1__OggS2".to_vec())?;
            cx.resolver = Some(Arc::new(move |path: &str| {
                ensure!(path == "res/res2001.grp", "unexpected sibling");
                Ok(volume.clone())
            }));
            let mut bytes = b"AiFS".to_vec();
            let mut header = vec![0x50000u32, 32, 1];
            if padded {
                header.push(0x01000000)
            }
            header.extend([1, 7, 9, 3, 2, 5, 0, 0, 9, 5]);
            for word in header {
                bytes.extend(word.to_le_bytes());
            }
            let archive = ExhibitVoice.open(cx.memory(bytes.clone())?, &cx)?;
            assert_eq!(
                archive
                    .entries()
                    .iter()
                    .map(|e| e.path.as_str())
                    .collect::<Vec<_>>(),
                ["00007.ogg", "00009.ogg"]
            );
            assert_eq!(read_all(&*archive.open(1, &cx)?, 5)?, b"OggS2");
            let end = bytes.len();
            bytes[end - 4..].copy_from_slice(&6u32.to_le_bytes());
            assert!(ExhibitVoice.open(cx.memory(bytes)?, &cx).is_err());
        }
        Ok(())
    }
}
