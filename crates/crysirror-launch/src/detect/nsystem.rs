//! N-System: extensionless FJSYS banks (MSD scripts, MGD images, Ogg audio)
//! beside the game EXE. Banks stay mounted in place, one namespace each. The
//! script key is the build's window title; it is taken from a known profile
//! and verified by decrypting `start.MSD`, never searched for.
use super::*;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
    state::ClientId,
};
use crysirror_resource::DecodeContext;
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("N-System reader lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self.0.lock().map_err(|_| crysirror_core::Error::Invalid("N-System reader lock"))?;
        file.seek(SeekFrom::Start(at))
            .and_then(|_| file.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

/// Known builds: executable name → window title (`lpWindowName`).
const PROFILES: &[(&str, &str, &str)] = &[(
    "hananyu.exe",
    "hananyu-2015",
    "その花びらにくちづけを　にゅーじぇね！",
)];
const BANKS: &[&str] = &["msd", "mgd", "bgm", "voice", "se", "data"];

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    if find(names, "msd").is_none() || find(names, "mgd").is_none() {
        return Ok(None);
    }
    let root = dir.canonicalize()?;
    let format = crysirror_formats_arc::legacy_archive("nsystem/fjsys")
        .context("N-System shared FJSYS format unavailable")?;
    let Some((exe, profile, title)) = PROFILES.iter().find_map(|(exe, profile, title)| {
        find(names, exe).map(|file| (file.clone(), *profile, *title))
    }) else {
        return Ok(None);
    };
    let context = DecodeContext::new(ClientId(1));
    let mut assets = std::collections::BTreeMap::new();
    let mut mounts = Vec::new();
    let mut verified = false;
    for bank in BANKS {
        let Some(file) = find(names, bank) else { continue };
        let full = root.join(file).canonicalize()?;
        ensure!(full.starts_with(&root), "N-System bank escapes selected game");
        if !full.is_file() {
            continue;
        }
        let source = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(full)?)));
        if format.probe(source.as_ref())?.confidence == 0 {
            return Ok(None);
        }
        let archive = format.open(source, &context)?;
        for entry in archive.entries() {
            let path = entry.path.replace('\\', "/");
            let logical = VfsPath::new(Mount::Asset, &path)?.relative().to_ascii_lowercase();
            ensure!(!logical.contains('/'), "N-System nested FJSYS entry");
            ensure!(assets.len() < 200_000, "N-System asset catalog budget");
            ensure!(
                assets.insert(format!("{bank}/{logical}"), format!("{bank}/{path}")).is_none(),
                "N-System duplicate case-insensitive resource in {bank}: {logical}"
            );
            if *bank == "msd" && logical == "start.msd" {
                let reader = archive.open(entry.id, &context)?;
                let mut bytes = crysirror_resource::read_all(reader.as_ref(), 8 * 1024 * 1024)?;
                let key = crysirror_nsystem::script::title_key(title);
                crysirror_nsystem::script::decrypt(&mut bytes, &key)
                    .with_context(|| format!("N-System profile {profile} does not match {exe}"))?;
                crysirror_nsystem::script::Script::parse("start", &bytes)?;
                verified = true;
            }
        }
        mounts.push(ArchiveMount {
            fallback: false,
            source: file.clone(),
            target: (*bank).into(),
            options: json!({"format":"nsystem/fjsys"}),
        });
    }
    if !verified {
        return Ok(None);
    }
    Ok(Some(Plan {
        engine: "N-System",
        project: project(
            name,
            [800, 600],
            "nsystem",
            json!({"entry":"start.MSD","title":title,"canvas":[800,600],"assets":assets,"font":"default"}),
        ),
        profiles: std::collections::BTreeMap::from([("nsystem".into(), json!({"limits":{"max_entry_size":256u64*1024*1024}}))]),
        mounts,
        evidence: Vec::new(),
    }))
}
