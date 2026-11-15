// SPDX-License-Identifier: MPL-2.0
//! Developer tool: dump QLIE FilePack entries whose logical path contains a
//! filter into an output directory. Reads the user's local installation only.
//!
//! cargo run --release -p crysirror-desktop --example qlie_dump -- GAME OUT [FILTER]
use anyhow::{Context, Result};
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{ArchiveFormat, DecodeContext};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{Arc, Mutex},
};
struct Reader(Mutex<File>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        let f = self.0.lock().map_err(|_| crysirror_core::Error::Invalid("lock"))?;
        f.metadata().map(|m| m.len()).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut f = self.0.lock().map_err(|_| crysirror_core::Error::Invalid("lock"))?;
        f.seek(SeekFrom::Start(at))
            .and_then(|_| f.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let game = PathBuf::from(args.get(1).context("GAME")?).canonicalize()?;
    let out = PathBuf::from(args.get(2).context("OUT")?);
    let filter = args.get(3).cloned().unwrap_or_default().to_ascii_lowercase();
    let format = crysirror_formats_arc::legacy_archive("qlie/pack").context("qlie/pack")?;
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = 1024 * 1024 * 1024;
    let mut parameters = serde_json::json!({});
    let exe = std::fs::read_dir(&game)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe") && !n.contains("設定"))
        .max_by_key(|n| game.join(n).metadata().map(|m| m.len()).unwrap_or(0));
    if let Some(exe) = exe {
        parameters["executable"] = exe.into();
    }
    if game.join("DLL/key.fkey").exists() {
        parameters["fkey"] = "DLL/key.fkey".into();
    }
    cx.parameters.insert("qlie/pack".into(), parameters);
    let root = game.clone();
    cx.resolver = Some(Arc::new(move |name: &str| -> Result<Arc<dyn AssetReader>> {
        Ok(Arc::new(Reader(Mutex::new(File::open(root.join(name))?))))
    }));
    let mut packs: Vec<_> = std::fs::read_dir(game.join("GameData"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "pack"))
        .collect();
    packs.sort();
    for pack in packs {
        let source = Arc::new(Reader(Mutex::new(File::open(&pack)?)));
        if format.probe(source.as_ref())?.confidence == 0 {
            continue;
        }
        cx.source_name = Some(format!("GameData/{}", pack.file_name().unwrap().to_string_lossy()));
        let archive = format.open(source, &cx)?;
        for e in archive.entries() {
            let path = e.path.replace('\\', "/");
            if !path.to_ascii_lowercase().contains(&filter) {
                continue;
            }
            let bytes = crysirror_resource::read_all(archive.open(e.id, &cx)?.as_ref(), 1 << 30)?;
            let target = out.join(&path);
            std::fs::create_dir_all(target.parent().unwrap())?;
            std::fs::write(&target, bytes)?;
        }
    }
    Ok(())
}
