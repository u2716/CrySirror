// SPDX-License-Identifier: MPL-2.0
//! Bounded scoped resource work. AOS2, Huffman SCR and ABM are existing shared
//! formats; this VM has no archive, decompression, pixel or audio algorithm.
use crate::{Config, program::Program};
use anyhow::{Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_resource::{DecodeContext, FileRegistry};
use crysirror_tasks::Cancellation;
use std::{collections::BTreeMap, sync::Arc};
pub(crate) struct Boot {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub settings: crate::native_storage::SystemSettings,
    pub globals: Vec<crate::native_storage::Binding>,
    pub read_flags: Vec<crate::native_storage::ReadFlag>,
}
pub(crate) fn boot(
    storage: Arc<dyn ClientStorage>,
    config: Config,
    client: ClientId,
    cancel: &Cancellation,
) -> Result<Boot> {
    use crate::native_storage as ns;
    let settings = ns::read_file(
        storage.clone(),
        &VfsPath::new(Mount::Save, "system.dat")?,
        cancel,
    )?
    .map(|b| ns::SystemSettings::decode(&b))
    .transpose()?
    .unwrap_or_default();
    let globals = ns::read_file(
        storage.clone(),
        &VfsPath::new(Mount::Save, "global.dat")?,
        cancel,
    )?
    .map(|b| ns::decode_globals(&b))
    .transpose()?
    .unwrap_or_default();
    let read_flags = ns::read_file(
        storage.clone(),
        &VfsPath::new(Mount::Save, "readal.dat")?,
        cancel,
    )?
    .map(|b| ns::decode_read_flags(&b))
    .transpose()?
    .unwrap_or_default();
    Ok(Boot {
        programs: programs(storage, config, client, cancel)?,
        settings,
        globals,
        read_flags,
    })
}
pub fn programs(
    storage: Arc<dyn ClientStorage>,
    config: Config,
    client: ClientId,
    cancel: &Cancellation,
) -> Result<BTreeMap<String, Arc<Program>>> {
    let mut registry = FileRegistry::default();
    crysirror_formats_arc::register_files(&mut registry)?;
    let mut context = DecodeContext::new(client).with_cancellation(cancel.clone());
    context.limits.max_entry_size = 4 * 1024 * 1024;
    context.limits.max_resident_bytes = 64 * 1024 * 1024;
    let mut out = BTreeMap::new();
    let mut total = 0usize;
    for (key, path) in &config.assets {
        let Some(name) = key.strip_prefix("scr/") else {
            continue;
        };
        if !name.ends_with(".scr") {
            continue;
        }
        cancel.check()?;
        let source = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
        let (_, decoded) = registry.decode(key, source, Some("lilim/scr"), &context)?;
        let bytes = crysirror_resource::read_all(decoded.reader.as_ref(), 4 * 1024 * 1024)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| anyhow::anyhow!("SugarPot script total overflow"))?;
        ensure!(
            total <= 64 * 1024 * 1024 && out.len() < 1024,
            "SugarPot script resident budget"
        );
        out.insert(name.to_owned(), Arc::new(Program::parse(name, &bytes)?));
    }
    cancel.check()?;
    Ok(out)
}
pub fn read(
    storage: Arc<dyn ClientStorage>,
    path: String,
    cancel: &Cancellation,
) -> Result<Arc<[u8]>> {
    cancel.check()?;
    let source = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
    let bytes = crysirror_resource::read_all(source.as_ref(), 64 * 1024 * 1024)?;
    cancel.check()?;
    Ok(bytes.into())
}

pub(crate) struct PreparedAnimation {
    pub programs: Vec<Vec<u16>>,
    pub frames: Vec<Arc<[u8]>>,
}
pub(crate) fn animation(
    bytes: Arc<[u8]>,
    client: ClientId,
    cancel: &Cancellation,
) -> Result<PreparedAnimation> {
    use crysirror_resource::ArchiveFormat;
    let context = DecodeContext::new(client).with_cancellation(cancel.clone());
    let archive = crysirror_formats_arc::lilim_animation::AbmFrames.open(
        Arc::new(crysirror_core::service::MemoryAssetReader(bytes)),
        &context,
    )?;
    let timeline = archive.open(0, &context)?;
    let timeline = crysirror_resource::read_all(timeline.as_ref(), 1024 * 1024)?;
    let metadata: serde_json::Value = serde_json::from_slice(&timeline)?;
    let programs = metadata["sequences"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("SugarPot shared animation timeline"))?
        .iter()
        .map(|sequence| {
            ensure!(
                sequence["flags"].as_u64() == Some(0),
                "SugarPot unsupported native animation sequence flags"
            );
            sequence["frames"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("SugarPot frame indices"))?
                .iter()
                .map(|v| {
                    Ok(u16::try_from(
                        v.as_u64()
                            .ok_or_else(|| anyhow::anyhow!("SugarPot frame index"))?,
                    )?)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let frames = archive
        .entries()
        .iter()
        .filter(|entry| entry.id != 0)
        .map(|entry| {
            cancel.check()?;
            let reader = archive.open(entry.id, &context)?;
            Ok(Arc::from(crysirror_resource::read_all(
                reader.as_ref(),
                context.limits.max_entry_size,
            )?))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PreparedAnimation { programs, frames })
}

/// 411190's four initialized 1024-entry tables (VA 4A72B8, 4A82B8, 4A92B8,
/// 4AA2B8 of this 2012 profile): disc source x/y and destination x/y.
pub(crate) fn disc_tables(
    storage: Arc<dyn ClientStorage>,
    path: String,
    cancel: &Cancellation,
) -> Result<[Vec<i32>; 4]> {
    use crysirror_resource::pe::PeData;
    let bytes = read(storage, path, cancel)?;
    let pe = PeData::parse(&bytes)?;
    let table = |va: u64| -> Result<Vec<i32>> {
        Ok(pe
            .va_slice(va, 1024 * 4)?
            .chunks_exact(4)
            .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
            .collect())
    };
    let tables = [table(0x4a72b8)?, table(0x4a82b8)?, table(0x4a92b8)?, table(0x4aa2b8)?];
    ensure!(
        tables[0].iter().chain(&tables[2]).all(|x| (0..1024).contains(x))
            && tables[1].iter().chain(&tables[3]).all(|y| (0..600).contains(y)),
        "SugarPot UD39 disc table profile"
    );
    cancel.check()?;
    Ok(tables)
}

pub(crate) struct FallAssets {
    pub images: Vec<Arc<[u8]>>,
    pub phases: Vec<u32>,
}
///414B50 resources and414FC0's initialized data table. The executable is only
///a capability-scoped data source; no image is copied into distributable code.
pub(crate) fn fall_assets(
    storage: Arc<dyn ClientStorage>,
    path: String,
    cancel: &Cancellation,
) -> Result<FallAssets> {
    use anyhow::Context;
    use crysirror_resource::pe::{PeData, PeResourceName};
    let bytes = read(storage, path, cancel)?;
    let pe = PeData::parse(&bytes)?;
    let rows = pe.raw_resources(&PeResourceName::Name("IMG".into()), &|| cancel.check())?;
    let mut images = vec![];
    for (index, name) in ["SNOW01", "SNOW02", "RAIN01", "RAIN02", "HANA01", "HANA02"]
        .into_iter()
        .enumerate()
    {
        cancel.check()?;
        let variants = rows
            .iter()
            .filter(|r| matches!(&r.name,PeResourceName::Name(key) if key==name))
            .collect::<Vec<_>>();
        let row = variants
            .first()
            .with_context(|| format!("SugarPot PE IMG {name} missing"))?;
        ensure!(
            variants.iter().all(|v| v.bytes == row.bytes),
            "SugarPot PE IMG {name} ambiguous language payloads"
        );
        let b = &row.bytes;
        let size = match index {
            0 => [16, 16],
            1 => [24, 24],
            2 => [16, 64],
            3 => [24, 128],
            4 => [16, 256],
            _ => [24, 384],
        };
        ensure!(
            b.len() >= 54
                && &b[..2] == b"BM"
                && i32::from_le_bytes(b[18..22].try_into()?) == size[0]
                && i32::from_le_bytes(b[22..26].try_into()?) == size[1]
                && u16::from_le_bytes(b[28..30].try_into()?) == 32,
            "SugarPot embedded {name} image profile"
        );
        images.push(Arc::from(b.clone()));
    }
    //Preferred VA492F18 is an initialized64-DWORD constant bank in this
    //explicit2012 profile, not an executable address or a runtime RNG oracle.
    let phases = pe
        .va_slice(0x492f18, 64 * 4)?
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    ensure!(
        phases.iter().all(|n| *n <= 0x7fff),
        "SugarPot particle seed-table profile"
    );
    cancel.check()?;
    Ok(FallAssets { images, phases })
}
