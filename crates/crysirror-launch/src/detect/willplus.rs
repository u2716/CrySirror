//! WillPlus/AdvHD boot over shared archives. Reads ARC indexes only; the
//! packed AdvHD.exe, launcher and installers are never run.
use super::*;
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_tasks::Cancellation;
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};
struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("AdvHD reader lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut f = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("AdvHD reader lock"))?;
        f.seek(SeekFrom::Start(at))
            .and_then(|_| f.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(_) = find(names, "AdvHD.exe") else { return Ok(None) };
    let root = dir.canonicalize()?;
    let mut files = with_ext(names, "arc").cloned().collect::<Vec<_>>();
    ensure!(files.len() <= 64, "AdvHD archive probe budget");
    files.sort_by_key(|s| s.to_ascii_lowercase());
    let cancel = Cancellation::default();
    let mut mounts = Vec::new();
    let mut archives = Vec::new();
    let mut scripts = 0;
    for file in files {
        let full = dir.join(&file).canonicalize()?;
        ensure!(full.starts_with(&root), "AdvHD archive escapes game root");
        let source: Arc<dyn AssetReader> = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(full)?)));
        let Some(probe) = crysirror_willplus::bootstrap::probe_archive(ClientId(1), source, &cancel)? else {
            continue;
        };
        scripts += probe.ws2_scripts;
        let stem = file.rsplit_once('.').map_or(file.as_str(), |(s, _)| s).to_owned();
        let target = format!("__willplus/{stem}");
        archives.push(target.clone());
        mounts.push(ArchiveMount {
            fallback: false,
            source: file,
            target,
            options: json!({"format":probe.kind.format(),"profile":"willplus"}),
        });
    }
    if scripts == 0 {
        return Ok(None);
    }
    let canvas = crysirror_willplus::vm::CANVAS.map(|v| v as u32);
    Ok(Some(Plan {
        engine: "WillPlus",
        project: project(name, canvas, "willplus", json!({"archives":archives,"start":"START"})),
        profiles: std::collections::BTreeMap::from([(
            "willplus".into(),
            json!({"limits":{"max_entry_size":1024u64*1024*1024}}),
        )]),
        mounts,
        evidence: vec![],
    }))
}
