// Copyright (C) 1997-1998 Masaki Chikama; (C) 2019 Nunuhara Cabbage.
// Rust implementation (C) 2026 CrySirror contributors. GPL-3.0-or-later.
//! Shared ALD volume/link-table catalogue. Format reference: libsys4
//! b14839f23af0963abbdb17f7cac57a36fd8e9bd2 src/ald.c (GPL-2.0-or-later).
//! Native resource numbers are deliberately not inferred from filenames or row
//! order. Callers supply capability-authorized readers and explicit zero-based
//! volume numbers; this CPU operation belongs on a resource worker.
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{Archive, ArchiveEntry, DecodeContext, read_exact_at, subrange};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct AldVolume {
    pub number: u8,
    pub source: Arc<dyn AssetReader>,
}
struct VolumeIndex {
    source: Arc<dyn AssetReader>,
    pointers: Vec<u8>,
    links: Vec<u8>,
    source_size: u64,
}
/// An immutable archive using the original zero-based link-table IDs. Missing
/// volumes remain explicit unresolved entries; they never alias a nearby file.
pub struct AldCatalog {
    owner: ClientId,
    volumes: BTreeMap<u8, Arc<dyn AssetReader>>,
    entries: Vec<ArchiveEntry>,
    by_id: BTreeMap<u32, usize>,
    missing: BTreeMap<u32, u8>,
}
fn sector(bytes: &[u8]) -> u64 {
    (u64::from(bytes[0]) | u64::from(bytes[1]) << 8 | u64::from(bytes[2]) << 16) << 8
}
fn bytes(source: &dyn AssetReader, offset: u64, count: u64, limit: u64) -> Result<Vec<u8>> {
    ensure!(count <= limit, "ALD index/header byte budget");
    let end = offset.checked_add(count).context("ALD range overflow")?;
    ensure!(
        end <= source.len().map_err(|e| anyhow::anyhow!("{e}"))?,
        "ALD range outside volume"
    );
    let mut output = vec![0; usize::try_from(count)?];
    read_exact_at(source, offset, &mut output)?;
    Ok(output)
}
fn index(
    source: Arc<dyn AssetReader>,
    with_links: bool,
    cx: &DecodeContext,
) -> Result<VolumeIndex> {
    cx.check_cancelled()?;
    let header = bytes(source.as_ref(), 0, 6, 6)?;
    let pointer_end = sector(&header[..3]);
    let link_end = sector(&header[3..]);
    ensure!(
        pointer_end >= 256 && pointer_end <= link_end,
        "ALD pointer/link table order or unsupported scrambled header"
    );
    let source_size = source.len().map_err(|e| anyhow::anyhow!("{e}"))?;
    ensure!(
        link_end <= source_size && link_end <= cx.limits.max_index_size,
        "ALD table extent/budget"
    );
    let pointers = bytes(source.as_ref(), 0, pointer_end, cx.limits.max_index_size)?;
    let links = if with_links {
        bytes(
            source.as_ref(),
            pointer_end,
            link_end - pointer_end,
            cx.limits.max_index_size,
        )?
    } else {
        Vec::new()
    };
    Ok(VolumeIndex {
        source,
        pointers,
        links,
        source_size,
    })
}
/// The same bounded physical header reader is used by filename enumeration and
/// the numbered catalogue. Payloads stay in the shared range reader.
pub(crate) fn file_header(
    source: &dyn AssetReader,
    offset: u64,
    end: u64,
    cx: &DecodeContext,
) -> Result<(String, u64, u64)> {
    ensure!(
        offset.checked_add(16).is_some_and(|n| n <= end),
        "ALD file header extent"
    );
    let first = bytes(source, offset, 16, 16)?;
    let header_size = u32::from_le_bytes(first[..4].try_into()?) as u64;
    let size = u32::from_le_bytes(first[4..8].try_into()?) as u64;
    ensure!(
        (17..=16 * 1024).contains(&header_size),
        "ALD filename/header budget"
    );
    ensure!(size <= cx.limits.max_entry_size, "ALD entry byte budget");
    let payload = offset
        .checked_add(header_size)
        .context("ALD payload offset overflow")?;
    ensure!(
        payload.checked_add(size).is_some_and(|n| n <= end),
        "ALD payload exceeds file interval"
    );
    let name = bytes(source, offset + 16, header_size - 16, 16 * 1024)?;
    let stop = name
        .iter()
        .position(|v| *v == 0)
        .context("ALD filename is not terminated")?;
    let text = encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(&name[..stop])
        .context("ALD filename encoding")?;
    ensure!(!text.is_empty(), "ALD empty filename");
    Ok((text.into_owned(), payload, size))
}
/// Filename-oriented inspection of one physical volume. Numeric lookup must
/// use AldCatalog's link table instead. The reviewed footer count excludes its
/// own terminal pointer (GARbro b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0,
/// ArcFormats/AliceSoft/ArcALD.cs, MIT; see ald_catalog.NOTICE).
pub(crate) fn physical_files(
    source: &dyn AssetReader,
    cx: &DecodeContext,
) -> Result<Vec<ArchiveEntry>> {
    cx.check_cancelled()?;
    let size = source.len().map_err(|e| anyhow::anyhow!("{e}"))?;
    let first = bytes(source, 0, 3, 3)?;
    let pointer_bytes = sector(&first);
    ensure!(
        pointer_bytes >= 256 && pointer_bytes <= cx.limits.max_index_size,
        "ALD physical pointer table budget"
    );
    let pointers = bytes(source, 0, pointer_bytes, cx.limits.max_index_size)?;
    let mut count = pointers.len().saturating_sub(3) / 3;
    if size >= 16 {
        let footer = bytes(source, size - 16, 16, 16)?;
        if matches!(
            u32::from_le_bytes(footer[..4].try_into()?),
            0x014c4e | 0x012020
        ) && u32::from_le_bytes(footer[4..8].try_into()?) == 16
        {
            count = usize::from(u16::from_le_bytes([footer[9], footer[10]]));
            ensure!(
                count <= pointers.len().saturating_sub(3) / 3,
                "ALD footer count exceeds pointer table"
            );
        }
    }
    let mut files = Vec::new();
    for ordinal in 0..count {
        if ordinal % 256 == 0 {
            cx.check_cancelled()?;
        }
        let at = (ordinal + 1) * 3;
        let offset = sector(&pointers[at..at + 3]);
        if offset == 0 {
            break;
        }
        ensure!(
            files.len() < cx.limits.max_entries,
            "ALD physical entry count budget"
        );
        let next = pointers
            .get(at + 3..at + 6)
            .map(sector)
            .filter(|n| *n != 0)
            .unwrap_or(size);
        ensure!(offset < next && next <= size, "ALD physical file interval");
        let (path, payload, size) = file_header(source, offset, next, cx)?;
        files.push(ArchiveEntry {
            id: ordinal as u32,
            path,
            size,
            stored_size: size,
            metadata: serde_json::json!({"offset":payload,"ordinal":ordinal + 1}),
        });
    }
    Ok(files)
}
impl AldCatalog {
    pub fn load(volumes: Vec<AldVolume>, cx: &DecodeContext) -> Result<Self> {
        ensure!(
            !volumes.is_empty() && volumes.len() <= 26,
            "ALD volume count"
        );
        let mut sorted = BTreeMap::new();
        for volume in volumes {
            ensure!(
                volume.number < 26 && sorted.insert(volume.number, volume.source).is_none(),
                "ALD duplicate/invalid volume number"
            );
        }
        let mut indices = BTreeMap::new();
        let mut links = Vec::new();
        let mut encoded_bytes = 0u64;
        for (number, source) in &sorted {
            let mut read = index(source.clone(), indices.is_empty(), cx)?;
            encoded_bytes = encoded_bytes
                .checked_add(read.pointers.len() as u64 + read.links.len() as u64)
                .context("ALD index sum overflow")?;
            ensure!(
                encoded_bytes <= cx.limits.max_index_size,
                "ALD cumulative index budget"
            );
            if indices.is_empty() {
                links = std::mem::take(&mut read.links);
            }
            indices.insert(*number, read);
        }
        ensure!(
            links.len() / 3 <= cx.limits.max_entries,
            "ALD logical entry count budget"
        );
        let mut catalog = Self {
            owner: cx.client,
            volumes: sorted,
            entries: Vec::new(),
            by_id: BTreeMap::new(),
            missing: BTreeMap::new(),
        };
        let mut name_bytes = 0u64;
        for (id, row) in links.chunks_exact(3).enumerate() {
            if id % 256 == 0 {
                cx.check_cancelled()?;
            }
            let ordinal = u16::from_le_bytes([row[1], row[2]]);
            if row[0] == 0 || ordinal == 0 {
                continue;
            }
            let number = row[0] - 1;
            ensure!(number < 26, "ALD link has invalid volume");
            let id = u32::try_from(id)?;
            let Some(volume) = indices.get(&number) else {
                catalog.missing.insert(id, number);
                continue;
            };
            // Pointer #0 is the table extent. Native map_ptr is ordinal-1,
            // and fileptr[ordinal-1] reads physical pointer #ordinal.
            let at = usize::from(ordinal)
                .checked_mul(3)
                .context("ALD pointer ordinal overflow")?;
            let pointer = volume
                .pointers
                .get(at..at + 6)
                .context("ALD link pointer outside volume table")?;
            let (start, end) = (sector(&pointer[..3]), sector(&pointer[3..]));
            if start == 0 || end == 0 {
                continue;
            } // Original _ald_get: absent file.
            ensure!(
                start < end && end <= volume.source_size,
                "ALD file interval outside volume"
            );
            let (path, offset, size) = file_header(volume.source.as_ref(), start, end, cx)?;
            name_bytes = name_bytes
                .checked_add(path.len() as u64)
                .context("ALD name sum overflow")?;
            ensure!(
                name_bytes <= cx.limits.max_index_size,
                "ALD aggregate filename budget"
            );
            catalog.by_id.insert(id, catalog.entries.len());
            catalog.entries.push(ArchiveEntry { id, path, size, stored_size: size,
                metadata: serde_json::json!({"format":"alice-soft/ald","original_id":id,"volume":number,"ordinal":ordinal,"offset":offset}) });
        }
        cx.check_cancelled()?;
        Ok(catalog)
    }
    pub fn entry(&self, id: u32) -> Result<Option<&ArchiveEntry>> {
        if let Some(volume) = self.missing.get(&id) {
            bail!("ALD logical entry {id} needs missing volume {volume}");
        }
        Ok(self.by_id.get(&id).map(|index| &self.entries[*index]))
    }
}
impl Archive for AldCatalog {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.entries.iter().find(|e| e.path == path).map(|e| e.id)
    }
    fn open(&self, id: u32, cx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        ensure!(cx.client == self.owner, "foreign ALD catalogue owner");
        cx.check_cancelled()?;
        let entry = self.entry(id)?.context("ALD logical resource not found")?;
        ensure!(
            entry.size <= cx.limits.max_entry_size,
            "ALD read byte budget"
        );
        let volume = entry.metadata["volume"]
            .as_u64()
            .context("ALD volume metadata")? as u8;
        let offset = entry.metadata["offset"]
            .as_u64()
            .context("ALD offset metadata")?;
        subrange(self.volumes[&volume].clone(), offset, entry.size)
    }
}
