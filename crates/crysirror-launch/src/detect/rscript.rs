//! codeX RScript (Liar-soft): an executable carrying an `APINI` block beside
//! `LB` (`.xfl`) archives. Every archive stays mounted in place at its stem;
//! archives nested in archives (voice banks) and the system LWG composites
//! are mounted from those mounts. The executable is only read as data to
//! select its verified APINI layout.
use super::*;
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::DecodeContext;
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

struct Reader(Mutex<crate::source_path::SourceFile>);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("RScript reader lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self.0.lock().map_err(|_| crysirror_core::Error::Invalid("RScript reader lock"))?;
        file.seek(SeekFrom::Start(at)).and_then(|_| file.read(out)).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

const MAX_EXE: u64 = 32 * 1024 * 1024;
const MAX_ASSETS: usize = 200_000;

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    if with_ext(names, "xfl").next().is_none() {
        return Ok(None);
    }
    let root = dir.canonicalize()?;
    let mut found = None;
    for exe in with_ext(names, "exe") {
        let path = root.join(exe).canonicalize()?;
        ensure!(path.starts_with(&root), "RScript executable escapes selected game");
        let mut bytes = Vec::new();
        crate::source_path::SourceFile::open(path)?.take(MAX_EXE + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_EXE || !bytes.windows(7).any(|w| w == b"RSCRIPT") || !bytes.windows(6).any(|w| w == b"APINI\0") {
            continue;
        }
        let rsinit = match find(names, "RsInit.cfg") {
            Some(cfg) => {
                let mut b = Vec::new();
                crate::source_path::SourceFile::open(root.join(cfg).canonicalize()?)?.take(4096).read_to_end(&mut b)?;
                Some(b)
            }
            None => None,
        };
        let apini = crysirror_rscript::apini::from_executable(&bytes, rsinit.as_deref()).with_context(|| format!("codeX RScript executable {exe}"))?;
        found = Some(apini);
        break;
    }
    let Some(apini) = found else { return Ok(None) };
    let xfl = crysirror_formats_arc::legacy_archive("liar-soft/xfl").context("shared liar-soft/xfl format unavailable")?;
    let lwg_format = crysirror_formats_arc::legacy_archive("liar-soft/lwg").context("shared liar-soft/lwg format unavailable")?;
    let context = DecodeContext::new(ClientId(1));
    let mut assets = BTreeMap::new();
    let mut lwg = BTreeMap::new();
    let mut mounts = Vec::new();
    let insert = |assets: &mut BTreeMap<String, String>, key: String, path: String| -> Result<()> {
        ensure!(assets.len() < MAX_ASSETS, "RScript asset catalog budget");
        // A loose file wins over an archive member (native resolver order).
        assets.entry(key).or_insert(path);
        Ok(())
    };
    // Loose files directly in the game folder and one directory below.
    for n in names {
        let p = root.join(n);
        if p.is_dir() {
            for child in listing(&dir.join(n))? {
                if root.join(n).join(&child).is_file() {
                    insert(&mut assets, format!("{n}/{child}").to_ascii_lowercase(), format!("{n}/{child}"))?;
                }
            }
        } else {
            insert(&mut assets, n.to_ascii_lowercase(), n.clone())?;
        }
    }
    for file in with_ext(names, "xfl") {
        let full = root.join(file).canonicalize()?;
        ensure!(full.starts_with(&root), "RScript archive escapes selected game");
        let source = Arc::new(Reader(Mutex::new(crate::source_path::SourceFile::open(full)?)));
        if xfl.probe(source.as_ref())?.confidence == 0 {
            continue;
        }
        let stem = file[..file.len() - 4].to_ascii_lowercase();
        let archive = xfl.open(source, &context)?;
        mounts.push(ArchiveMount { fallback: false, source: file.clone(), target: stem.clone(), options: json!({"format":"liar-soft/xfl"}) });
        for entry in archive.entries() {
            let member = entry.path.replace('\\', "/");
            ensure!(!member.contains('/') && !member.contains(".."), "RScript nested XFL path {member}");
            let mounted = format!("{stem}/{member}");
            let lower = member.to_ascii_lowercase();
            insert(&mut assets, format!("{stem}/{lower}"), mounted.clone())?;
            if let Some(inner) = lower.strip_suffix(".xfl") {
                // `voice\1\0002.wav` names `1.xfl` inside `voice.xfl`.
                let reader = archive.open(entry.id, &context)?;
                let nested = xfl.open(reader, &context)?;
                let target = format!("{stem}/{inner}");
                mounts.push(ArchiveMount { fallback: false, source: mounted.clone(), target: target.clone(), options: json!({"format":"liar-soft/xfl"}) });
                for e in nested.entries() {
                    let m = e.path.replace('\\', "/");
                    ensure!(!m.contains('/') && !m.contains(".."), "RScript nested XFL path {m}");
                    insert(&mut assets, format!("{target}/{}", m.to_ascii_lowercase()), format!("{target}/{m}"))?;
                }
            } else if let Some(inner) = lower.strip_suffix(".lwg") {
                let reader = archive.open(entry.id, &context)?;
                let image = lwg_format.open(reader, &context)?;
                let target = format!("lwg/{stem}/{inner}");
                let mut canvas = [0u32; 2];
                let mut layers = Vec::new();
                for e in image.entries() {
                    let meta = &e.metadata;
                    if let Some(c) = meta["canvas"].as_array() {
                        canvas = [c[0].as_u64().unwrap_or(0) as u32, c[1].as_u64().unwrap_or(0) as u32];
                    }
                    layers.push(crysirror_rscript::res::LwgLayer {
                        name: e.path.clone(),
                        x: meta["x"].as_i64().unwrap_or(0) as i32,
                        y: meta["y"].as_i64().unwrap_or(0) as i32,
                        asset: (e.size > 0).then(|| format!("{target}/{}", e.path)),
                    });
                }
                mounts.push(ArchiveMount { fallback: false, source: mounted, target, options: json!({"format":"liar-soft/lwg"}) });
                lwg.insert(format!("{stem}/{inner}"), crysirror_rscript::res::Lwg { canvas, layers });
            }
        }
    }
    let scripts = crysirror_rscript::res::key(apini.dir(crysirror_rscript::apini::Dir::Scripts)).context("RScript script directory")?;
    if !assets.keys().any(|k| k.starts_with(&format!("{scripts}/")) && k.ends_with(".gsc")) {
        return Ok(None);
    }
    let canvas = [apini.width, apini.height];
    let config = crysirror_rscript::Config { apini, assets, lwg, font: "default".into() };
    Ok(Some(Plan {
        engine: "codeX RScript",
        project: project(name, canvas, "rscript", serde_json::to_value(&config)?),
        profiles: Default::default(),
        mounts,
        evidence: Vec::new(),
    }))
}
