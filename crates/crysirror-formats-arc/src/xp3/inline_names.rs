// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Publish the native inline preferred-name namespace while retaining info-token
//! aliases and the IDs of shadowed plain-index records. Entry::preferred_name
//! follows xp3-brute c82cb14302138125640478556723705aa720007b (MIT).
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// `info_names` maps each entry published under an inline filename to its
/// original `info` name (typed XP3 facts, see `Xp3Facts`).
pub(super) fn publish(
    inner: Box<dyn Archive>,
    info_names: &BTreeMap<u32, Box<str>>,
) -> Result<Box<dyn Archive>> {
    if info_names.is_empty() {
        return Ok(inner);
    }
    let mut names: BTreeMap<String, Vec<&ArchiveEntry>> = BTreeMap::new();
    let mut aliases = BTreeMap::new();
    for entry in inner.entries() {
        let path = crysirror_core::path::VfsPath::new(
            crysirror_core::path::Mount::Asset,
            &entry.path.replace('\\', "/"),
        )
        .map_err(|e| anyhow::anyhow!("XP3 entry path: {e}"))?;
        names
            .entry(path.relative().to_owned())
            .or_default()
            .push(entry);
        if let Some(native) = info_names.get(&entry.id) {
            let native = native.replace('\\', "/");
            let native =
                crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &native)
                    .map_err(|e| anyhow::anyhow!("XP3 native info alias: {e}"))?;
            ensure!(
                aliases
                    .insert(native.relative().to_owned(), entry.id)
                    .is_none(),
                "ambiguous XP3 native info alias"
            );
        }
    }
    let mut hidden = BTreeSet::new();
    let mut shadows = BTreeMap::new();
    for (name, entries) in &names {
        if entries.len() < 2 {
            continue;
        }
        let preferred: Vec<_> = entries
            .iter()
            .filter(|e| info_names.contains_key(&e.id))
            .collect();
        ensure!(
            preferred.len() == 1,
            "ambiguous XP3 inline preferred name: {name}"
        );
        let winner = preferred[0].id;
        let shadowed: Vec<_> = entries
            .iter()
            .filter(|e| e.id != winner)
            .map(|e| e.id)
            .collect();
        hidden.extend(shadowed.iter().copied());
        shadows.insert(winner, shadowed);
    }
    // Native info aliases must not silently override another exposed name.
    for (alias, id) in &aliases {
        if let Some(entries) = names.get(alias) {
            ensure!(
                entries
                    .iter()
                    .filter(|e| !hidden.contains(&e.id))
                    .all(|e| e.id == *id),
                "XP3 info alias conflicts with an exposed resource"
            );
        }
    }
    let visible = if hidden.is_empty() {
        None
    } else {
        Some(
            inner
                .entries()
                .iter()
                .filter(|e| !hidden.contains(&e.id))
                .cloned()
                .collect(),
        )
    };
    Ok(Box::new(InlineArchive {
        inner,
        visible,
        aliases,
        shadows,
    }))
}
struct InlineArchive {
    inner: Box<dyn Archive>,
    visible: Option<Vec<ArchiveEntry>>,
    aliases: BTreeMap<String, u32>,
    /// Preferred inline entry id -> shadowed plain-index ids.
    shadows: BTreeMap<u32, Vec<u32>>,
}
impl Archive for InlineArchive {
    fn entries(&self) -> &[ArchiveEntry] {
        self.visible
            .as_deref()
            .unwrap_or_else(|| self.inner.entries())
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.aliases
            .get(path)
            .copied()
            .or_else(|| self.inner.resolve(path))
    }
    fn open(&self, id: u32, ctx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        self.inner.open(id, ctx)
    }
    fn entry_metadata(&self, id: u32) -> serde_json::Value {
        let mut out = self.inner.entry_metadata(id);
        if let Some(ids) = self.shadows.get(&id) {
            if !out.is_object() {
                out = serde_json::json!({});
            }
            out["xp3_shadowed_native_ids"] = serde_json::json!(ids);
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{service::MemoryAssetReader, state::ClientId};
    struct Source(Vec<ArchiveEntry>);
    impl Archive for Source {
        fn entries(&self) -> &[ArchiveEntry] {
            &self.0
        }
        fn open(&self, id: u32, _: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
            Ok(Arc::new(MemoryAssetReader(Arc::from([id as u8]))))
        }
    }
    fn entry(id: u32, path: &str, _inline: bool) -> ArchiveEntry {
        ArchiveEntry {
            id,
            path: path.into(),
            size: 1,
            stored_size: 1,
            metadata: serde_json::json!({}),
        }
    }
    fn info(entries: &[ArchiveEntry], inline: &[u32]) -> BTreeMap<u32, Box<str>> {
        entries
            .iter()
            .filter(|e| inline.contains(&e.id))
            .map(|e| (e.id, format!("native_token_{}", e.id).into_boxed_str()))
            .collect()
    }
    fn publish_with(entries: Vec<ArchiveEntry>, inline: &[u32]) -> Result<Box<dyn Archive>> {
        let names = info(&entries, inline);
        publish(Box::new(Source(entries)), &names)
    }
    #[test]
    fn explicit_inline_namespace_wins_over_fallback_without_losing_original_record_or_info_alias() {
        let a = publish_with(
            vec![
                entry(0, "startup.tjs", false),
                entry(1, "startup.tjs", true),
                entry(2, "scene.ks", true),
            ],
            &[1, 2],
        )
        .unwrap();
        assert_eq!(a.entries().len(), 2);
        assert_eq!(a.entries()[0].id, 1);
        assert_eq!(
            a.entry_metadata(1)["xp3_shadowed_native_ids"],
            serde_json::json!([0])
        );
        assert_eq!(a.resolve("native_token_1"), Some(1));
        let cx = DecodeContext::new(ClientId(1));
        assert_eq!(read_all(a.open(0, &cx).unwrap().as_ref(), 10).unwrap(), [0]);
        assert_eq!(read_all(a.open(1, &cx).unwrap().as_ref(), 10).unwrap(), [1]);
        let mount = crysirror_resource::ArchiveMount::new(a, cx).unwrap();
        assert_eq!(
            read_all(mount.open_entry("startup.tjs").unwrap().as_ref(), 10).unwrap(),
            [1]
        );
        assert_eq!(
            read_all(mount.open_entry("native_token_1").unwrap().as_ref(), 10).unwrap(),
            [1]
        );
    }
    #[test]
    fn multiple_inline_records_or_token_conflicts_are_not_guessed() {
        assert!(publish_with(vec![entry(0, "same", true), entry(1, "same", true)], &[0, 1]).is_err());
        assert!(publish_with(vec![entry(0, "native_token_1", false), entry(1, "other", true)], &[1]).is_err());
    }
}
