//! Installed N2System NSS/NPA layout. Original archives remain mounted; only
//! bounded indexes, system.ini and the startup NSS are read during detection.
use super::*;
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{DecodeContext, MemoryKeyProvider};
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
            .map_err(|_| crysirror_core::Error::Invalid("N2System archive lock"))?
            .metadata()
            .map(|m| m.len())
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| crysirror_core::Error::Invalid("N2System archive lock"))?;
        file.seek(SeekFrom::Start(at))
            .and_then(|_| file.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}
fn reader(root: &SourcePath, name: &str) -> Result<Arc<dyn AssetReader>> {
    let path = root.join(name).canonicalize()?;
    ensure!(
        path.starts_with(root.canonicalize()?),
        "N2System resource escapes selected game"
    );
    Ok(Arc::new(Reader(Mutex::new(
        crate::source_path::SourceFile::open(path)?,
    ))))
}
fn ini(text: &str) -> BTreeMap<(String, String), String> {
    let mut result = BTreeMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].into();
        } else if let Some((key, value)) = line.split_once('=') {
            result.insert(
                (section.clone(), key.trim().to_ascii_lowercase()),
                value.trim().into(),
            );
        }
    }
    result
}
fn canvas(source: &crysirror_n2system::source::Source) -> Result<[u32; 2]> {
    use crysirror_n2system::{
        source::{Expr, Statement},
        value::Value,
    };
    fn statements(body: &[Statement], votes: &mut BTreeMap<[u32; 2], usize>) {
        for statement in body {
            match statement {
                Statement::Eval(Expr::Call(name, args)) if name == "CreateColor" => {
                    if let (
                        Some(Expr::Literal(Value::Number(w))),
                        Some(Expr::Literal(Value::Number(h))),
                    ) = (args.get(4), args.get(5))
                    {
                        if *w >= 640. && *w <= 8192. && *h >= 360. && *h <= 8192. {
                            *votes.entry([*w as u32, *h as u32]).or_default() += 1;
                        }
                    }
                }
                Statement::Block(body) | Statement::Case(_, body) => statements(body, votes),
                Statement::If(_, yes, no) => {
                    statements(std::slice::from_ref(yes.as_ref()), votes);
                    if let Some(no) = no {
                        statements(std::slice::from_ref(no.as_ref()), votes);
                    }
                }
                Statement::While(_, body) => statements(std::slice::from_ref(body.as_ref()), votes),
                Statement::Select(cases) => {
                    for (_, body) in cases {
                        statements(body, votes)
                    }
                }
                _ => {}
            }
        }
    }
    // This is startup-script evidence for the authored canvas, not a claim
    // that every N2System revision shares a hard-coded screen resolution.
    let mut votes = BTreeMap::new();
    for routine in &source.routines {
        statements(&routine.body, &mut votes);
    }
    votes.into_iter().max_by_key(|(_,count)|*count).map(|(size,_)|size).context("N2System canvas unavailable in startup script; explicit project configuration required")
}
pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(nss) = find(names, "nss.npa") else {
        return Ok(None);
    };
    let Some(system) = find(names, "system.npa") else {
        return Ok(None);
    };
    let root = dir.canonicalize()?;
    let format = crysirror_formats_arc::legacy_archive("nitroplus/npa")
        .context("shared NPA format missing")?;
    let material = crysirror_n2system::archive::ArchiveMaterial {
        permutation: ".crysirror/private-npa-permutation.bin".into(),
        data_key: ".crysirror/private-npa-data-key.bin".into(),
        npf_permutation:None,
    };
    let mut context = DecodeContext::new(ClientId(1));
    let keys = Arc::new(MemoryKeyProvider::default());
    for (alias, path, expected) in [
        ("n2-permutation", &material.permutation, 256),
        ("n2-data-key", &material.data_key, 4),
    ] {
        let source = reader(&root, path)
            .with_context(|| format!("N2System caller-owned archive material missing: {path}"))?;
        ensure!(
            source.len()? == expected,
            "N2System archive material length"
        );
        keys.insert(
            ClientId(1),
            alias,
            crysirror_resource::read_all(source.as_ref(), expected)?,
        )?;
    }
    context.keys = keys;
    context.parameters.insert("nitroplus/npa".into(),json!({"file_name_key_source":"header_product","key_id":"n2-permutation","data_key_id":"n2-data-key"}));
    let system_archive = format.open(reader(&root, system)?, &context)?;
    let entry = system_archive
        .entries()
        .iter()
        .find(|e| e.path.eq_ignore_ascii_case("system.ini"))
        .context("N2System system.ini missing from system archive")?;
    let bytes = crysirror_resource::read_all(
        system_archive.open(entry.id, &context)?.as_ref(),
        1024 * 1024,
    )?;
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
    ensure!(!errors, "N2System system.ini encoding");
    let settings = ini(&text);
    let start = settings
        .get(&("スクリプト".into(), "start".into()))
        .context("N2System startup script setting missing")?
        .replace('\\', "/");
    let stem = start.strip_prefix("nss/").unwrap_or(&start);
    let scripts = format.open(reader(&root, nss)?, &context)?;
    let entry = scripts
        .entries()
        .iter()
        .find(|e| e.path.eq_ignore_ascii_case(stem))
        .context("N2System startup NSS missing")?;
    let bytes =
        crysirror_resource::read_all(scripts.open(entry.id, &context)?.as_ref(), 8 * 1024 * 1024)?;
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
    ensure!(!errors, "N2System startup NSS encoding");
    let source = crysirror_n2system::source::Source::parse(&text)?;
    let size = canvas(&source)?;
    let mut project = project(
        name,
        size,
        "n2system",
        json!({"entry":stem,"script_root":"nss","encoding":"shift-jis","canvas":size,"font":"default","platform":100,"archive_material":material}),
    );
    let mut fonts = BTreeMap::new();
    for entry in system_archive.entries() {
        if Path::new(&entry.path).extension().is_some_and(|e| {
            e.to_str().is_some_and(|e| {
                e.eq_ignore_ascii_case("otf")
                    || e.eq_ignore_ascii_case("ttf")
                    || e.eq_ignore_ascii_case("ttc")
            })
        }) {
            let path = format!("system/{}", entry.path);
            fonts.entry("default".to_owned()).or_insert(path.clone());
            fonts.insert(format!("n2-font-{}",fonts.len()), path);
        }
    }
    ensure!(
        !fonts.is_empty(),
        "N2System game-supplied fonts unavailable"
    );
    // These faces live inside system.npa. Project fonts load before archive
    // publication; the dormant adapter registers its scoped faces afterwards.
    project["clients"][0]["config"]["font_sources"] = serde_json::to_value(fonts)?;
    let mut mounts = Vec::new();
    for (bank, target) in [
        ("nss", "nss"),
        ("system", "system"),
        ("cg", "cg"),
        ("cg2", "cg"),
        ("sound", "sound"),
        ("voice", "voice"),
    ] {
        if let Some(file) = find(names, &format!("{bank}.npa")) {
            mounts.push(ArchiveMount {
                fallback: false,
                source: file.clone(),
                target: target.into(),
                options: json!({"format":"nitroplus/npa","profile":"n2system"}),
            });
        }
    }
    Ok(Some(Plan {
        engine: "N2System",
        project,
        profiles: BTreeMap::from([("n2system".into(), crysirror_n2system::archive::profile())]),
        mounts,
        evidence: Vec::new(),
    }))
}
