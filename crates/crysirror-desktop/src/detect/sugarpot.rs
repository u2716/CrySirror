//! AOS2 plus the native Tsukumo text language identify this profile. Archives
//! remain mounted in place; each native resource bank has its own namespace.
use super::*;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
    state::ClientId,
};
use crysirror_resource::{DecodeContext, FileRegistry};
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("SugarPot reader lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("SugarPot reader lock"))?;
        file.seek(SeekFrom::Start(at))
            .and_then(|_| file.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    if find(names, "scr.aos").is_none() || find(names, "grp.aos").is_none() {
        return Ok(None);
    }
    let root = dir.canonicalize()?;
    let format = crysirror_formats_arc::legacy_archive("lilim/aos2")
        .context("SugarPot shared AOS2 format unavailable")?;
    let mut context = DecodeContext::new(ClientId(1));
    context.limits.max_entry_size = 256 * 1024 * 1024;
    let mut files = FileRegistry::default();
    crysirror_formats_arc::register_files(&mut files)?;
    let mut assets = std::collections::BTreeMap::new();
    let mut mounts = Vec::new();
    let mut native_script = false;
    for bank in ["scr", "grp", "bgm", "se", "cv"] {
        let Some(file) = find(names, &format!("{bank}.aos")) else {
            continue;
        };
        let full = root.join(file).canonicalize()?;
        ensure!(
            full.starts_with(&root),
            "SugarPot archive escapes selected game"
        );
        let source = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(
            full,
        )?)));
        if format.probe(source.as_ref())?.confidence == 0 {
            return Ok(None);
        }
        let archive = format.open(source, &context)?;
        for entry in archive.entries() {
            let path = entry.path.replace('\\', "/");
            // Native AOS resources are addressed by filename, within a bank.
            let logical = VfsPath::new(Mount::Asset, &path)?
                .relative()
                .to_ascii_lowercase();
            ensure!(
                !logical.contains('/'),
                "SugarPot nested AOS entry is not a native basename"
            );
            ensure!(assets.len() < 100000, "SugarPot asset catalog budget");
            ensure!(
                assets
                    .insert(format!("{bank}/{logical}"), format!("{bank}/{path}"))
                    .is_none(),
                "SugarPot duplicate case-insensitive resource in {bank}: {logical}"
            );
            if bank == "scr" && logical == "start.scr" {
                let source = archive.open(entry.id, &context)?;
                let (_, decoded) = files.decode(&path, source, Some("lilim/scr"), &context)?;
                let bytes = crysirror_resource::read_all(decoded.reader.as_ref(), 4 * 1024 * 1024)?;
                let program = crysirror_sugarpot::program::Program::parse("start.scr", &bytes)?;
                native_script = program.sections.contains_key("START")
                    && program.sections.contains_key("END")
                    && program
                        .instructions
                        .iter()
                        .any(|i| i.source.starts_with("spr("))
                    && program
                        .instructions
                        .iter()
                        .any(|i| i.source.starts_with("^gsb("));
            }
        }
        mounts.push(ArchiveMount {
            fallback: false,
            source: file.clone(),
            target: bank.into(),
            options: json!({"format":"lilim/aos2","profile":"sugarpot"}),
        });
    }
    // TITLE is a native cross-file target (title.scr), not a section of start.
    if !native_script || !assets.contains_key("scr/title.scr") {
        return Ok(None);
    }
    let executable = executable_resource_profile(&root, names)?;
    Ok(Some(Plan {
        engine: "SugarPot",
        project: project(
            name,
            [1024, 600],
            "sugarpot",
            json!({"script":"start.scr","canvas":[1024,600],"assets":assets,"font":"default","executable":executable}),
        ),
        profiles: std::collections::BTreeMap::from([(
            "sugarpot".into(),
            json!({"limits":{"max_entry_size":256u64*1024*1024}}),
        )]),
        mounts,
        evidence: Vec::new(),
    }))
}

/// The game EXE is a read-only source of embedded particle images. Installers
/// and uninstallers are not selected by extension/order; the original IMG
/// family identifies the relevant data-bearing module without executing it.
fn executable_resource_profile(root: &SourcePath, names: &[String]) -> Result<Option<String>> {
    use crysirror_resource::pe::{PeData, PeResourceName, MAX_PE_BYTES};
    let candidates = names
        .iter()
        .filter(|name| name.to_ascii_lowercase().ends_with(".exe"))
        .collect::<Vec<_>>();
    ensure!(
        candidates.len() <= 128,
        "SugarPot executable candidate budget"
    );
    let mut result = None;
    let mut used = 0u64;
    for name in candidates {
        let size = root.join(name).metadata()?.len();
        if size > MAX_PE_BYTES as u64 {
            continue;
        }
        used = used
            .checked_add(size)
            .context("SugarPot executable metadata budget")?;
        ensure!(
            used <= 256 * 1024 * 1024,
            "SugarPot executable metadata byte budget"
        );
        let bytes = read_bounded(root, name, MAX_PE_BYTES)?;
        let Ok(image) = PeData::parse(&bytes) else {
            continue;
        };
        let Ok(resources) = image.raw_resources(&PeResourceName::Name("IMG".into()), &|| Ok(()))
        else {
            continue;
        };
        let family = ["SNOW01", "SNOW02", "RAIN01", "RAIN02", "HANA01", "HANA02"];
        if !family.iter().all(|expected| {
            resources.iter().any(
                |resource| matches!(&resource.name, PeResourceName::Name(name) if name == expected),
            )
        }) {
            continue;
        }
        ensure!(
            result.is_none(),
            "ambiguous SugarPot embedded resource executables"
        );
        result = Some(name.clone());
    }
    Ok(result)
}
