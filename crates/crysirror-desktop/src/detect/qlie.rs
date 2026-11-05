//! Original FilePack bootstrap. Reads indexes and engine settings only; no
//! installer, external extractor, generated scenario or Windows DLL is run.
use super::*;
use crysirror_core::{service::AssetReader, state::ClientId};
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
            .map_err(|_| crysirror_core::Error::Invalid("QLIE reader lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut f = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("QLIE reader lock"))?;
        f.seek(SeekFrom::Start(at))
            .and_then(|_| f.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let prefix = find(names, "GameData").map(String::as_str).unwrap_or("");
    let root = dir.join(prefix).canonicalize()?;
    ensure!(
        root.starts_with(dir.canonicalize()?),
        "QLIE resource root escapes selected game"
    );
    if !root.is_dir() {
        return Ok(None);
    }
    let local = listing(&root)?;
    let format =
        crysirror_formats_arc::legacy_archive("qlie/pack").context("QLIE shared format missing")?;
    let mut files = with_ext(&local, "pack").cloned().collect::<Vec<_>>();
    ensure!(files.len() <= 128, "QLIE archive count budget");
    // Native dataN then update/patch archives, with numeric comparison.
    files.sort_by_key(|s| {
        let s = s.to_ascii_lowercase();
        let number = s
            .trim_start_matches("data")
            .trim_end_matches(".pack")
            .parse::<u32>()
            .ok();
        (number.is_none(), number.unwrap_or(0), s)
    });
    let mut mounts = Vec::new();
    let mut assets = std::collections::BTreeMap::new();
    let mut settings = None;
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = 1024 * 1024 * 1024;
    for file in files {
        let full = root.join(&file).canonicalize()?;
        ensure!(
            full.starts_with(&root),
            "QLIE archive escapes capability root"
        );
        let source = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(
            full,
        )?)));
        if format.probe(source.as_ref())?.confidence == 0 {
            continue;
        }
        let archive = format.open(source, &cx)?;
        for e in archive.entries() {
            let path = e.path.replace('\\', "/");
            let logical = crysirror_qlie::program::canonical(&path)?;
            ensure!(assets.len() < 65536, "QLIE resource catalog limit");
            assets.insert(logical.clone(), path);
            if logical == "system/etc/game.txt" {
                settings = Some(crysirror_qlie::program::decode_text(
                    &crysirror_resource::read_all(archive.open(e.id, &cx)?.as_ref(), 65536)?,
                )?);
            }
        }
        mounts.push(ArchiveMount {
            fallback: false,
            source: if prefix.is_empty() {
                file
            } else {
                format!("{prefix}/{file}")
            },
            target: "".into(),
            options: json!({"format":"qlie/pack","profile":"game"}),
        });
    }
    if mounts.is_empty() {
        return Ok(None);
    }
    ensure!(
        assets.contains_key("startup.s"),
        "QLIE original startup.s missing"
    );
    let settings = settings.context("QLIE original system/etc/game.txt missing")?;
    let values = settings
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let width = values
        .get("screenwidth")
        .context("QLIE ScreenWidth")?
        .parse::<u32>()?;
    let height = values
        .get("screenheight")
        .context("QLIE ScreenHeight")?
        .parse::<u32>()?;
    ensure!(
        (1..=8192).contains(&width) && (1..=8192).contains(&height),
        "QLIE screen geometry"
    );
    Ok(Some(Plan {
        engine: "QLIE",
        project: project(
            name,
            [width, height],
            "qlie",
            json!({"script":"startup.s","assets":assets,"canvas":[width,height]}),
        ),
        mounts,
        profiles: std::collections::BTreeMap::from([(
            "game".into(),
            json!({"limits":{"max_entry_size":1024u64*1024*1024}}),
        )]),
        evidence: Vec::new(),
    }))
}
