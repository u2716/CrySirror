// Copyright (C) 2021 kichikuou <KichikuouChrome@gmail.com>.
// Rust (C) 2026 CrySirror contributors. GPL-3.0-or-later.
// Format reference: libsys4 b14839f23af0963abbdb17f7cac57a36fd8e9bd2
// src/alk.c and include/system4/alk.h, GPL-2.0-or-later (used under GPLv3+).
//! Numeric ALK0 container, independent of any VM. Empty original slots remain
//! absent IDs; payloads use shared immutable subrange readers without unpacking.
use crate::common::{bytes, length, limits, range};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{Archive, ArchiveEntry, DecodeContext, subrange};
use std::{collections::BTreeMap, sync::Arc};
pub struct AlkCatalog {
    owner: ClientId,
    source: Arc<dyn AssetReader>,
    entries: Vec<ArchiveEntry>,
    locations: BTreeMap<u32, (usize, u64)>,
}
impl Archive for AlkCatalog {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.entries.iter().find(|e| e.path == path).map(|e| e.id)
    }
    fn open(&self, id: u32, cx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        ensure!(cx.client == self.owner, "foreign ALK archive owner");
        cx.check_cancelled()?;
        let (entry, offset) = self
            .locations
            .get(&id)
            .context("ALK original resource ID absent")?;
        let size = self.entries[*entry].size;
        ensure!(size <= cx.limits.max_entry_size, "ALK resource byte budget");
        subrange(self.source.clone(), *offset, size)
    }
}
/// Parse the original numeric directory once. The legacy filename inspector
/// delegates this routine and retains its historical sequential aliases; the
/// catalogue retains original IDs. Neither view renumbers native IDs in secret.
pub(crate) fn physical_files(
    source: &dyn AssetReader,
    cx: &DecodeContext,
) -> Result<Vec<ArchiveEntry>> {
    cx.check_cancelled()?;
    let header = bytes(source, 0, 8, 8)?;
    ensure!(&header[..4] == b"ALK0", "ALK signature");
    let count = u32::from_le_bytes(header[4..8].try_into()?) as usize;
    limits(cx, count)?;
    let table_size = (count as u64)
        .checked_mul(8)
        .context("ALK table size overflow")?;
    let table = bytes(source, 8, table_size, cx.limits.max_index_size)?;
    let source_size = length(source)?;
    let mut entries = Vec::new();
    for (id, row) in table.chunks_exact(8).enumerate() {
        if id % 256 == 0 {
            cx.check_cancelled()?;
        }
        let offset = u64::from(u32::from_le_bytes(row[..4].try_into()?));
        let size = u64::from(u32::from_le_bytes(row[4..].try_into()?));
        if size == 0 {
            continue;
        }
        ensure!(offset >= 8 + table_size, "ALK resource overlaps directory");
        range(source_size, offset, size)?;
        ensure!(size <= cx.limits.max_entry_size, "ALK resource byte budget");
        let id = id as u32;
        entries.push(ArchiveEntry{id,path:id.to_string(),size,stored_size:size,metadata:serde_json::json!({"format":"alice-soft/alk","original_id":id,"physical_ordinal":entries.len(),"offset":offset})});
    }
    cx.check_cancelled()?;
    Ok(entries)
}
impl AlkCatalog {
    /// Caller supplies a capability-authorized reader on its resource worker.
    /// The original `alice-soft/alk` registry entry remains the legacy filename
    /// view; this explicit numeric catalogue is not a duplicate registration.
    pub fn load(source: Arc<dyn AssetReader>, cx: &DecodeContext) -> Result<Self> {
        let entries = physical_files(source.as_ref(), cx)?;
        let locations = entries
            .iter()
            .enumerate()
            .map(|(index, e)| {
                Ok((
                    e.id,
                    (
                        index,
                        e.metadata["offset"].as_u64().context("ALK source offset")?,
                    ),
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Self {
            owner: cx.client,
            source,
            entries,
            locations,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;
    fn source(rows: &[(u32, u32)], data: &[u8]) -> Arc<dyn AssetReader> {
        let mut bytes = b"ALK0".to_vec();
        bytes.extend((rows.len() as u32).to_le_bytes());
        for (offset, size) in rows {
            bytes.extend(offset.to_le_bytes());
            bytes.extend(size.to_le_bytes());
        }
        bytes.extend(data);
        Arc::new(MemoryAssetReader(bytes.into()))
    }
    #[test]
    fn original_numeric_holes_and_foreign_owner_are_retained() {
        let cx = DecodeContext::new(ClientId(1));
        let ar = AlkCatalog::load(source(&[(32, 1), (0, 0), (33, 2)], b"ABC"), &cx).unwrap();
        assert_eq!(
            ar.entries().iter().map(|e| e.id).collect::<Vec<_>>(),
            [0, 2]
        );
        assert!(ar.open(1, &cx).is_err());
        assert_eq!(
            crysirror_resource::read_all(ar.open(2, &cx).unwrap().as_ref(), 8).unwrap(),
            b"BC"
        );
        assert!(ar.open(2, &DecodeContext::new(ClientId(2))).is_err());
    }
    #[test]
    fn bounded_directory_and_payload_fail_before_publication() {
        let cx = DecodeContext::new(ClientId(1));
        for rows in [[(8, 1)], [(16, 100)], [(u32::MAX, 1)]] {
            assert!(AlkCatalog::load(source(&rows, b"A"), &cx).is_err());
        }
        let mut tiny = cx.clone();
        tiny.limits.max_index_size = 7;
        assert!(AlkCatalog::load(source(&[(16, 1)], b"A"), &tiny).is_err());
        tiny = cx.clone();
        tiny.limits.max_entries = 0;
        assert!(AlkCatalog::load(source(&[(16, 1)], b"A"), &tiny).is_err());
    }
    #[test]
    fn single_registered_legacy_view_uses_the_same_original_id_metadata() {
        let mut registry = crysirror_resource::FormatRegistry::default();
        crate::register(&mut registry).unwrap();
        assert_eq!(
            registry
                .names()
                .iter()
                .filter(|id| **id == "alice-soft/alk")
                .count(),
            1
        );
        let cx = DecodeContext::new(ClientId(1));
        let (_, legacy) = registry
            .open(
                source(&[(32, 1), (0, 0), (33, 2)], b"ABC"),
                Some("alice-soft/alk"),
                &cx,
            )
            .unwrap();
        assert_eq!(
            legacy
                .entries()
                .iter()
                .map(|e| e.path.as_str())
                .collect::<Vec<_>>(),
            ["000.dat", "001.dat"]
        );
        assert_eq!(legacy.entries()[1].metadata["original_id"], 2);
        assert_eq!(
            crysirror_resource::read_all(legacy.open(1, &cx).unwrap().as_ref(), 8).unwrap(),
            b"BC"
        );
    }
    #[test]
    #[ignore = "reads owner AliceLogo2.alk and decodes its original QNT images without publishing game data"]
    fn owner_rance7_logo_archive_and_all_numbered_qnt_images_decode() {
        let root = std::path::PathBuf::from(std::env::var("CRYSIRROR_SYSTEM4_ROOT").unwrap());
        let bytes = std::fs::read(root.join("AliceLogo2.alk")).unwrap();
        let cx = DecodeContext::new(ClientId(1));
        let archive = AlkCatalog::load(Arc::new(MemoryAssetReader(bytes.into())), &cx).unwrap();
        let mut registry = crysirror_image::DecoderRegistry::default();
        crate::register_images(&mut registry).unwrap();
        let mut versions = std::collections::BTreeMap::new();
        let mut pixels = 0u64;
        for entry in archive.entries() {
            let encoded = crysirror_resource::read_all(
                archive.open(entry.id, &cx).unwrap().as_ref(),
                64 * 1024 * 1024,
            )
            .unwrap();
            assert!(encoded.starts_with(b"QNT\0"));
            let version = u32::from_le_bytes(encoded[4..8].try_into().unwrap());
            *versions.entry(version).or_insert(0usize) += 1;
            let image = registry.decode(&entry.path, &encoded).unwrap();
            image.validate().unwrap();
            pixels += u64::from(image.width) * u64::from(image.height);
        }
        println!(
            "owner logo ALK original IDs={} QNT_versions={versions:?} decoded_pixels={pixels}",
            archive.entries().len()
        );
        assert_eq!(archive.entries().len(), 37);
        assert!(pixels > 0);
    }
}
