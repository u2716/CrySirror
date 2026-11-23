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
    let mut legacy_settings = None;
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = 1024 * 1024 * 1024;
    // Key companions remain scoped resources, not bytes serialized into the
    // launch plan. The same profile is retained by every runtime mount.
    let mut parameters = json!({});
    let executable = main_exe(dir, names);
    if let Some(executable) = &executable {
        parameters["executable"] = executable.clone().into();
    }
    let fkey = find(names, "key.fkey").cloned().or_else(|| {
        let dll = find(names, "DLL")?;
        let files = listing(&dir.join(dll)).ok()?;
        find(&files, "key.fkey").map(|file| format!("{dll}/{file}"))
    });
    if let Some(fkey) = fkey {
        parameters["fkey"] = fkey.into();
    }
    cx.parameters.insert("qlie/pack".into(), parameters.clone());
    let capability = dir.canonicalize()?;
    cx.resolver = Some(Arc::new(move |name: &str| -> Result<Arc<dyn AssetReader>> {
        let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, name)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let full = capability.join(path.relative()).canonicalize()?;
        ensure!(full.starts_with(&capability), "QLIE key companion escapes game root");
        Ok(Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(full)?))))
    }));
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
        cx.source_name = Some(if prefix.is_empty() { file.clone() } else { format!("{prefix}/{file}") });
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
            } else if logical == "system/etc/game.dat" {
                // Older FilePack3.0 titles use the same CP932 INI payload
                // under game.dat. The archive cipher already decoded it.
                legacy_settings = Some(crysirror_qlie::program::decode_text(
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
    // FilePack enumerates packed assets only. Native movie loaders also open
    // loose files below the installed Movie directory; keep actual names for
    // storage (notably macOS NFD) and composed/case-folded lookup keys.
    let mut movie_dirs = Vec::new();
    if let Some(movie) = find(names, "Movie") {
        movie_dirs.push(movie.clone());
    }
    if !prefix.is_empty() {
        if let Some(movie) = find(&local, "Movie") {
            movie_dirs.push(format!("{prefix}/{movie}"));
        }
    }
    for movie in movie_dirs {
        catalog_movies(dir, &movie, &mut assets)?;
    }
    ensure!(
        assets.contains_key("startup.s"),
        "QLIE original startup.s missing"
    );
    let settings = settings.or(legacy_settings)
        .context("QLIE original system/etc/game.txt or game.dat missing")?;
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
            json!({"script":"startup.s","assets":assets,"canvas":[width,height],"executable":main_exe(dir,names)}),
        ),
        mounts,
        profiles: std::collections::BTreeMap::from([(
            "game".into(),
            json!({"limits":{"max_entry_size":1024u64*1024*1024},"parameters":{"qlie/pack":parameters}}),
        )]),
        evidence: Vec::new(),
    }))
}

fn catalog_movies(
    dir: &SourcePath,
    movie: &str,
    assets: &mut std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let capability = dir.canonicalize()?;
    let mut pending = vec![(movie.to_owned(), 0)];
    let mut visited = std::collections::BTreeSet::new();
    let mut files = std::collections::BTreeMap::new();
    while let Some((relative, depth)) = pending.pop() {
        ensure!(depth <= 8 && visited.len() < 1024, "QLIE movie directory budget");
        let directory = dir.join(&relative).canonicalize()?;
        ensure!(directory.starts_with(&capability), "QLIE movie directory escapes game root");
        ensure!(visited.insert(directory.to_string()), "QLIE cyclic movie directory");
        for entry in directory.read_dir()? {
            let entry = entry?;
            let name = entry.file_name().into_string().map_err(|_| anyhow::anyhow!("QLIE non-Unicode movie filename"))?;
            let actual = format!("{relative}/{name}");
            let full = dir.join(&actual).canonicalize()?;
            ensure!(full.starts_with(&capability), "QLIE movie file escapes game root");
            let metadata = full.metadata()?;
            if metadata.is_dir() {
                pending.push((actual, depth + 1));
            } else if metadata.is_file() {
                ensure!(files.len() < 4096, "QLIE movie file count budget");
                let logical = crysirror_qlie::program::canonical(&actual)?;
                ensure!(!files.contains_key(&logical), "QLIE ambiguous normalized movie filename: {logical}");
                files.insert(logical, actual);
            }
        }
    }
    for (logical, actual) in files {
        ensure!(assets.len() < 65536, "QLIE resource catalog limit");
        assets.entry(logical).or_insert(actual);
    }
    Ok(())
}
