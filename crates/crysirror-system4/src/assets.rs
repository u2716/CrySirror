// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d src/asset_manager.c native policy.
//! Native resource class, one-based ID and archive-name policy only. ALD
//! parsing/streams are provided by CrySirror's shared formats archive API.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_formats_arc::ald_catalog::{AldCatalog, AldVolume};
use crysirror_resource::{Archive, DecodeContext};
use std::{collections::BTreeMap, sync::Arc};
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Music,
    Sound,
    Cg,
    Data,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    pub class: Class,
    pub volume: u8,
    pub source: String,
    pub mount: String,
}
impl Volume {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.volume < 26, "System4 ALD volume index");
        for path in [&self.source, &self.mount] {
            ensure!(
                !path.is_empty() && VfsPath::new(Mount::Asset, path)?.relative() == path,
                "System4 archive capability path"
            );
        }
        Ok(())
    }
}
/// Native ALD classification. The original AIN basename, class letter and
/// A..Z volume letter carry meaning; arbitrary directory order does not.
pub fn classify(program: &str, file: &str) -> Option<(Class, u8)> {
    let base = program.rsplit_once('.')?.0;
    let prefix = file.get(..base.len())?;
    if !prefix.eq_ignore_ascii_case(base) {
        return None;
    }
    let suffix = file.get(base.len()..)?.as_bytes();
    if suffix.len() != 6 || suffix[2] != b'.' || !suffix[3..].eq_ignore_ascii_case(b"ald") {
        return None;
    }
    let volume = suffix[1].to_ascii_uppercase();
    if !volume.is_ascii_uppercase() {
        return None;
    }
    let class = match suffix[0].to_ascii_uppercase() {
        b'B' => Class::Music,
        b'W' => Class::Sound,
        b'G' => Class::Cg,
        b'D' => Class::Data,
        _ => return None,
    };
    Some((class, volume - b'A'))
}
struct Group {
    catalog: AldCatalog,
    mounts: BTreeMap<u8, String>,
}
pub struct Assets {
    groups: BTreeMap<Class, Group>,
}
impl Assets {
    /// CPU/IO operation: caller runs it on an admitted resource worker and
    /// transfers the immutable result back to the VM owner.
    pub fn load(
        client: ClientId,
        storage: &dyn ClientStorage,
        volumes: &[Volume],
        cancel: crysirror_tasks::Cancellation,
    ) -> Result<Self> {
        ensure!(volumes.len() <= 104, "System4 archive volume count budget");
        let mut grouped: BTreeMap<Class, Vec<&Volume>> = BTreeMap::new();
        for volume in volumes {
            volume.validate()?;
            grouped.entry(volume.class).or_default().push(volume);
        }
        let mut groups = BTreeMap::new();
        let mut cx = DecodeContext::new(client).with_cancellation(cancel.clone());
        cx.limits.max_entry_size = 1024 * 1024 * 1024;
        for (class, group) in grouped {
            let mut readers = Vec::new();
            let mut mounts = BTreeMap::new();
            for volume in group {
                cancel.check()?;
                ensure!(
                    mounts.insert(volume.volume, volume.mount.clone()).is_none(),
                    "duplicate System4 class/volume"
                );
                readers.push(AldVolume {
                    number: volume.volume,
                    source: storage.open_asset(&VfsPath::new(Mount::Asset, &volume.source)?)?,
                });
            }
            groups.insert(
                class,
                Group {
                    catalog: AldCatalog::load(readers, &cx)?,
                    mounts,
                },
            );
        }
        Ok(Self { groups })
    }
    /// One-based AIN resource number -> shared mount-relative asset path.
    /// True holes are absent; references to missing volumes are errors.
    pub fn path(&self, class: Class, native_id: i32) -> Result<Option<String>> {
        if native_id <= 0 {
            return Ok(None);
        }
        let Some(group) = self.groups.get(&class) else {
            return Ok(None);
        };
        let Some(entry) = group.catalog.entry(native_id as u32 - 1)? else {
            return Ok(None);
        };
        let volume = u8::try_from(
            entry.metadata["volume"]
                .as_u64()
                .context("System4 shared ALD volume metadata")?,
        )?;
        let prefix = group
            .mounts
            .get(&volume)
            .context("System4 shared ALD mount absent")?;
        let path = format!("{prefix}/{}", entry.path);
        ensure!(
            VfsPath::new(Mount::Asset, &path)?.relative() == path,
            "System4 ALD member capability path"
        );
        Ok(Some(path))
    }
    /// Streaming access uses the very same shared catalogue. This is useful
    /// for native numbered binary tables without decoding the entire archive.
    pub fn open(
        &self,
        class: Class,
        native_id: i32,
        cx: &DecodeContext,
    ) -> Result<Option<Arc<dyn crysirror_core::service::AssetReader>>> {
        if native_id <= 0 {
            return Ok(None);
        }
        let Some(group) = self.groups.get(&class) else {
            return Ok(None);
        };
        if group.catalog.entry(native_id as u32 - 1)?.is_none() {
            return Ok(None);
        }
        Ok(Some(group.catalog.open(native_id as u32 - 1, cx)?))
    }
}

impl std::fmt::Debug for Assets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System4Assets")
            .field("classes", &self.groups.keys().collect::<Vec<_>>())
            .finish()
    }
}
