//! Verified Ragnarok EXEC/CFI profile. Archives remain in place; keys are read
//! only from the selected caller-owned executable, never written to a sidecar.
use super::*;
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{DecodeContext, MemoryKeyProvider};
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};
struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("Malie source lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut f = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("Malie source lock"))?;
        f.seek(SeekFrom::Start(at))
            .and_then(|_| f.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
fn reader(root: &SourcePath, name: &str) -> Result<Arc<dyn AssetReader>> {
    let path = root.join(name).canonicalize()?;
    ensure!(path.starts_with(root), "Malie source escapes selected game");
    Ok(Arc::new(Reader(Mutex::new(
        crate::source_path::SourceFile::open(path)?,
    ))))
}
pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let (Some(exe), Some(ini)) = (find(names, "malie.exe"), find(names, "malie.ini")) else {
        return Ok(None);
    };
    if find(names, "data1.dat").is_none() {
        return Ok(None);
    }
    let root = dir.canonicalize()?;
    let bytes = crysirror_resource::read_all(reader(&root, exe)?.as_ref(), 16 * 1024 * 1024)?;
    let material = crysirror_malie::material::archive_key(&bytes)?;
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entries = 200_000;
    let keys = Arc::new(MemoryKeyProvider::default());
    keys.insert(ClientId(1), "malie-game", material)?;
    cx.keys = keys;
    let params = json!({"encryption":"cfi","key_id":"malie-game","data_alignment":4096});
    cx.parameters.insert("malie/libp".into(), params.clone());
    let bytes = crysirror_resource::read_all(reader(&root, ini)?.as_ref(), 1024 * 1024)?;
    let text = encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned();
    let mut section = String::new();
    let mut settings = std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_ascii_lowercase();
        } else if section == "system" {
            if let Some((k, v)) = line.split_once('=') {
                settings.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
            }
        }
    }
    let size = [
        settings
            .get("screenwidth")
            .context("Malie ScreenWidth")?
            .parse::<u32>()?,
        settings
            .get("screenheight")
            .context("Malie ScreenHeight")?
            .parse::<u32>()?,
    ];
    ensure!(
        size.iter().all(|v| (1..=8192).contains(v)),
        "Malie canvas budget"
    );
    let format = crysirror_formats_arc::legacy_archive("malie/libp")
        .context("Malie LIBP shared format unavailable")?;
    let mut archives: Vec<_> = names
        .iter()
        .filter(|s| s.starts_with("data") && s.ends_with(".dat"))
        .collect();
    archives.sort_by_key(|s| {
        s.trim_start_matches("data")
            .trim_end_matches(".dat")
            .parse::<u32>()
            .unwrap_or(u32::MAX)
    });
    let mut mounts = Vec::new();
    let mut verified = false;
    let mut font = None;
    for file in archives {
        let archive = format.open(reader(&root, file)?, &cx)?;
        for entry in archive.entries() {
            if entry.path.eq_ignore_ascii_case("data/system/exec.dat") {
                let bytes = crysirror_resource::read_all(
                    archive.open(entry.id, &cx)?.as_ref(),
                    64 * 1024 * 1024,
                )?;
                let program = crysirror_malie::program::Program::parse(&bytes)?;
                ensure!(
                    program.labels.contains_key("main"),
                    "Malie main label missing"
                );
                verified = true;
            }
            if entry.path.eq_ignore_ascii_case("data/font/telopminpro.otf") {
                font = Some(entry.path.clone());
            }
        }
        mounts.push(ArchiveMount {
            fallback: false,
            source: file.clone(),
            target: "".into(),
            options: json!({"format":"malie/libp","profile":"malie"}),
        });
    }
    ensure!(verified, "Malie EXEC startup resource missing");
    let mut project = project(
        name,
        size,
        "malie",
        json!({"executable":exe,"canvas":size,"font":"malie-default","title":settings.get("title")}),
    );
    if let Some(font) = font {
        project["clients"][0]["config"]["font_source"] = json!(font);
    }
    Ok(Some(Plan {
        engine: "Malie",
        project,
        profiles: std::collections::BTreeMap::from([(
            "malie".into(),
            json!({"parameters":{"malie/libp":params},"limits":{"max_entries":200_000}}),
        )]),
        mounts,
        evidence: Vec::new(),
    }))
}
