//! BP bootstrap selection for the integrated BGI runtime.
use super::*;
use crysirror_core::service::AssetReader;
use crysirror_resource::{ArchiveFormat, DecodeContext};
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};
struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Io("BGI detection file lock".into()))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, offset: u64, output: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Io("BGI detection file lock".into()))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))?;
        file.read(output)
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    if let Some(plan) = detect_root(dir, names, name)? {
        return Ok(Some(plan));
    }
    // Disc layouts keep the actual game and original archives under Install/.
    // Select that resource root directly; never execute the Windows installer.
    if let Some(folder) = find(names, "Install").filter(|folder| dir.join(folder).is_dir()) {
        let root = dir.join(folder).canonicalize()?;
        ensure!(
            root.starts_with(dir.canonicalize()?),
            "BGI Install root escapes game folder"
        );
        if let Some(mut plan) = detect_root(&root, &listing(&root)?, name)? {
            plan.project["clients"][0]["asset_prefix"] = json!(folder);
            if let Some(fonts) = plan.project.get_mut("fonts").and_then(Value::as_object_mut) {
                for value in fonts.values_mut() {
                    if let Some(path) = value.as_str() {
                        *value = json!(format!("{folder}/{path}"));
                    }
                }
            }
            for evidence in &mut plan.evidence {
                evidence.path = format!("{folder}/{}", evidence.path);
            }
            return Ok(Some(plan));
        }
    }
    Ok(None)
}

fn detect_root(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let root = dir.canonicalize()?;
    let mut mounts = Vec::new();
    let mut bootstrap = None;
    for source in with_ext(names, "arc") {
        let full = dir.join(source).canonicalize()?;
        ensure!(
            full.starts_with(&root),
            "BGI archive escapes game directory"
        );
        let reader: Arc<dyn AssetReader> = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(full)?)));
        if crysirror_formats_arc::BgiArc
            .probe(reader.as_ref())?
            .confidence
            == 0
        {
            continue;
        }
        let context = DecodeContext::new(crysirror_core::state::ClientId(0));
        let archive = crysirror_formats_arc::BgiArc.open(reader, &context)?;
        if source.eq_ignore_ascii_case("system.arc") {
            if let Some(entry) = archive
                .entries()
                .iter()
                .find(|entry| entry.path.eq_ignore_ascii_case("ipl._bp"))
            {
                let bytes = crysirror_resource::read_all(
                    archive.open(entry.id, &context)?.as_ref(),
                    16 * 1024 * 1024,
                )?;
                let bytes = if bytes.starts_with(b"DSC FORMAT 1.00") {
                    crysirror_resource::dsc::decode_with_limit(&bytes, 16 * 1024 * 1024)?
                } else {
                    bytes
                };
                ensure!(
                    valid_bp(&bytes),
                    "BGI system.arc/ipl._bp has an invalid BP image"
                );
                bootstrap = Some(format!("{source}:{}", entry.path));
            }
        }
        mounts.push(ArchiveMount {
            source: source.clone(),
            target: format!("__bgi/{source}"),
            fallback: false,
            options: json!({"format":"bgi/arc"}),
        });
    }
    if bootstrap.is_none() {
        for candidate in ["ipl._bp", "main._bp"] {
            if let Some(file) = find(names, candidate) {
                let path = dir.join(file).canonicalize()?;
                ensure!(path.starts_with(&root), "BGI BP escapes game directory");
                let metadata = path.metadata()?;
                ensure!(
                    metadata.len() <= 16 * 1024 * 1024,
                    "BGI bootstrap byte budget"
                );
                ensure!(valid_bp(&path.read()?), "invalid BGI BP bootstrap");
                bootstrap = Some(file.clone());
                break;
            }
        }
    }
    let Some(bootstrap) = bootstrap else {
        return Ok(None);
    };
    ensure!(mounts.len() <= 256, "BGI mounted archive limit");
    let mut project = project(
        name,
        [1280, 720],
        "bgi",
        json!({"script":bootstrap,"root":"","font":"default"}),
    );
    if let Some(font) = find(names, "default.ttf").or_else(|| find(names, "default.otf")) {
        project["fonts"] = json!({"default":font});
    }
    Ok(Some(Plan {
        engine: "BGI",
        project,
        profiles: Default::default(),
        evidence: mounts
            .into_iter()
            .map(|mount| FileEvidence {
                path: mount.source,
                identity: None,
            })
            .collect(),
        // The original archive manager owns these packs. Do not index/mount a
        // second copy in the general resource service during player startup.
        mounts: Vec::new(),
    }))
}
fn valid_bp(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let header = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    header >= 8 && size > 0 && header.checked_add(size) == Some(bytes.len())
}
