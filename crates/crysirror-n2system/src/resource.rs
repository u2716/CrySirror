// SPDX-License-Identifier: MPL-2.0
//! Scoped NSS preparation. Files and archives stay behind ClientStorage.
use crate::{program::Module, source, vm::module_name};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::Cancellation;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub type Catalog = BTreeMap<String, String>;
const FILES: usize = 200_000;
const SOURCE_BYTES: usize = 8 * 1024 * 1024;
const PREPARE_BYTES: usize = 64 * 1024 * 1024;

pub fn asset_key(path: &str) -> Result<String> {
    let path = path.replace('\\', "/");
    let path = VfsPath::new(Mount::Asset, path.trim_start_matches("./"))?;
    Ok(path.relative().to_ascii_lowercase())
}

pub fn catalog(storage: &dyn ClientStorage, cancel: &Cancellation) -> Result<Catalog> {
    let mut result = BTreeMap::new();
    let mut todo = vec![(String::new(), 0)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = todo.pop() {
        cancel.check()?;
        ensure!(depth <= 32, "N2System resource directory depth");
        for name in storage.list(Mount::Asset, &dir)? {
            cancel.check()?;
            visited += 1;
            ensure!(visited <= FILES, "N2System resource catalog capacity");
            ensure!(
                !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\']),
                "N2System invalid directory entry"
            );
            let path = if dir.is_empty() {
                name
            } else {
                format!("{dir}/{name}")
            };
            let path = VfsPath::new(Mount::Asset, &path)?;
            let Some(metadata) = storage.metadata(Mount::Asset, path.relative())? else {
                continue;
            };
            if metadata.is_directory {
                todo.push((path.relative().into(), depth + 1));
            } else {
                ensure!(
                    result
                        .insert(asset_key(path.relative())?, path.relative().into())
                        .is_none(),
                    "N2System ambiguous case-insensitive resource: {}",
                    path.relative()
                );
            }
        }
    }
    Ok(result)
}

pub fn resolve(catalog: &Catalog, path: &str) -> Result<String> {
    let key = asset_key(path)?;
    // Rebuild Builtins.LoadAudio/OpenAudioStream 77a93db4 (MIT): preserve
    // explicit names, then native extension priority for an audio stem.
    if !key.rsplit('/').next().unwrap_or("").contains('.')
        && (key.starts_with("sound/") || key.starts_with("voice/"))
    {
        if let Some(path) = catalog.get(&key) {
            return Ok(path.clone());
        }
        for extension in [".ogg", ".wav", ".mp3", ".flac", ".opus"] {
            if let Some(path) = catalog.get(&format!("{key}{extension}")) {
                return Ok(path.clone());
            }
        }
    }
    catalog
        .get(&key)
        .cloned()
        .with_context(|| format!("N2System resource not found: {path}"))
}

/// Preserve the original source encoding unless a BOM or explicit profile
/// selects Unicode. No replacement characters may silently change identifiers.
pub(crate) fn decode(bytes: &[u8], encoding: &str) -> Result<String> {
    let (encoding, offset) = encoding_rs::Encoding::for_bom(bytes).unwrap_or_else(|| {
        (
            if encoding == "utf-8" {
                encoding_rs::UTF_8
            } else {
                encoding_rs::SHIFT_JIS
            },
            0,
        )
    });
    let (text, errors) = encoding.decode_without_bom_handling(&bytes[offset..]);
    ensure!(!errors, "N2System malformed NSS source encoding");
    Ok(text.into_owned())
}

/// One worker owns reading, decoding and compilation of a dependency closure.
/// A caller publishes all modules only after the complete closure succeeds.
pub fn modules(
    storage: Arc<dyn ClientStorage>,
    catalog: Arc<Catalog>,
    script_root: &str,
    entry: &str,
    encoding: &str,
    existing: BTreeSet<String>,
    cancel: &Cancellation,
) -> Result<Vec<(String, Module)>> {
    let mut todo = vec![module_name(entry)?];
    let mut seen = existing;
    let mut result = Vec::new();
    let mut bytes_total = 0usize;
    while let Some(name) = todo.pop() {
        cancel.check()?;
        if !seen.insert(name.clone()) {
            continue;
        }
        ensure!(seen.len() <= 4096, "N2System script module capacity");
        let source = if script_root.is_empty() {
            format!("{name}.nss")
        } else {
            format!("{script_root}/{name}.nss")
        };
        let source = resolve(&catalog, &source)?;
        let reader = storage.open_read(&VfsPath::new(Mount::Asset, &source)?)?;
        let len = usize::try_from(reader.len()?).context("N2System NSS source length")?;
        ensure!(
            len <= SOURCE_BYTES && len <= PREPARE_BYTES.saturating_sub(bytes_total),
            "N2System source preparation byte budget"
        );
        bytes_total += len;
        let bytes = crysirror_resource::read_all(reader.as_ref(), len as u64)?;
        cancel.check()?;
        let text = decode(&bytes, encoding)?;
        let parsed =
            source::Source::parse(&text).with_context(|| format!("N2System parse {source}"))?;
        let module =
            Module::compile(parsed).with_context(|| format!("N2System compile {source}"))?;
        for import in &module.imports {
            todo.push(module_name(import)?);
        }
        result.push((name, module));
    }
    Ok(result)
}
