//! Read-only native format detection; no VM, audio backend or script execution.
use crate::{
    native_asset::{AssetError, Nls, ResourceManager},
    native_script::{PointTable, ScriptImage},
};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

pub fn detect_game(root: &Path) -> Result<bool> {
    if !root.is_dir() {
        return Ok(false);
    }
    let mut candidate = false;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "archive.dat" | "script.src" | "data.pac" | "csv.pac"
        ) {
            candidate = true;
            break;
        }
    }
    if !candidate {
        return Ok(false);
    }
    let mut resources = match ResourceManager::bootstrap(root, Nls::ShiftJis) {
        Ok(resources) => resources,
        Err(AssetError::AssetNotFound { .. }) => return Ok(false),
        Err(error) => return Err(error).context("SoftPAL archive bootstrap"),
    };
    let script = match resources.open("Script.src") {
        Ok(script) => script,
        Err(AssetError::AssetNotFound { .. }) => return Ok(false),
        Err(error) => return Err(error).context("SoftPAL Script.src"),
    };
    // A similarly named foreign file must not claim this adapter.
    if script.bytes.get(..4) != Some(b"Sv20".as_slice()) {
        return Ok(false);
    }
    ScriptImage::parse(&script.bytes).context("SoftPAL Sv20 structure")?;
    for name in ["File.dat", "Text.dat", "Mem.dat"] {
        let asset = resources
            .open_decrypting(name)
            .with_context(|| format!("SoftPAL required {name}"))?;
        ensure!(!asset.bytes.is_empty(), "SoftPAL required {name} is empty");
    }
    let points = resources
        .open_decrypting("Point.dat")
        .context("SoftPAL required Point.dat")?;
    PointTable::parse(&points.bytes).context("SoftPAL Point.dat structure")?;
    Ok(true)
}
