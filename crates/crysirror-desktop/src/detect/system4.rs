// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original System40.ini / AliceStart.ini -> first-class AIN configuration and
//! shared archive mounts. Native archive-number policy is owned by the adapter;
//! no executable, external bootstrap manifest or unpacked game copy is used.
use super::*;
use std::io::Read;
pub(super) fn detect(dir: &Path, names: &[String], name: &str) -> Result<Option<Plan>> {
    for prefix in std::iter::once("").chain(find(names, "GAMEDATA").map(String::as_str)) {
        let root = dir.join(prefix).canonicalize()?;
        ensure!(
            root.starts_with(dir.canonicalize()?),
            "System4 nested capability root escapes selected game"
        );
        if !root.is_dir() {
            continue;
        }
        let local = listing(&root)?;
        let Some(ini) = find(&local, "System40.ini").or_else(|| find(&local, "AliceStart.ini"))
        else {
            continue;
        };
        {
            return native_plan(dir, &root, &local, prefix, ini, name).map(Some);
        }
    }
    Ok(None)
}
fn native_plan(
    dir: &Path,
    root: &Path,
    names: &[String],
    prefix: &str,
    ini: &str,
    _name: &str,
) -> Result<Plan> {
    use crysirror_system4::{
        assets::Volume,
        bootstrap::{Bootstrap, Document},
    };
    let read = |path: &str, limit: u64| -> Result<Vec<u8>> {
        let full = root.join(path).canonicalize()?;
        ensure!(
            full.starts_with(root),
            "System4 detection input escapes capability root"
        );
        let mut bytes = Vec::new();
        std::fs::File::open(full)?
            .take(limit + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= limit, "System4 detection byte budget");
        Ok(bytes)
    };
    let document = Document::parse(&read(ini, 1024 * 1024)?)?;
    let bootstrap = Bootstrap::from_document(&document)?;
    let program = root.join(&bootstrap.program).canonicalize()?;
    ensure!(
        program.starts_with(root) && program.is_file(),
        "System4 original AIN unavailable/escapes game root"
    );
    let mut mounts = Vec::new();
    let mut volumes = Vec::new();
    for source in names {
        if let Some((class, volume)) =
            crysirror_system4::assets::classify(&bootstrap.program, source)
        {
            let full = root.join(source).canonicalize()?;
            ensure!(
                full.starts_with(root) && full.is_file(),
                "System4 archive escapes capability root"
            );
            let target = format!("__system4/{class:?}/{volume}");
            volumes.push(Volume {
                class,
                volume,
                source: source.clone(),
                mount: target.clone(),
            });
            mounts.push(ArchiveMount {
                fallback: false,
                source: source.clone(),
                target,
                options: json!({"format":"alice-soft/ald","profile":"game"}),
            });
        }
    }
    let dll = if let Some(folder) = find(names, "DLL").filter(|n| root.join(n).is_dir()) {
        let files = listing(&root.join(folder))?;
        find(&files, "OutputLog.dll").map(|n| format!("{folder}/{n}"))
    } else {
        None
    };
    let config = crysirror_system4::worker_adapter::Config {
        program: bootstrap.program.clone(),
        ini: Some(ini.into()),
        volumes,
        canvas: Some(bootstrap.canvas),
        font_aliases: ["default".into(), "default".into()],
        reviewed_output_log_dll: dll,
    };
    config.validate()?;
    let mut p = project(
        &bootstrap.game_name,
        bootstrap.canvas,
        "system4",
        serde_json::to_value(config)?,
    );
    if !prefix.is_empty() {
        p["clients"][0]["asset_prefix"] = json!(prefix);
    }
    let scoped = |path: &str| {
        if prefix.is_empty() {
            path.to_string()
        } else {
            format!("{prefix}/{path}")
        }
    };
    let _ = dir;
    Ok(Plan {
        engine: "System4",
        project: p,
        profiles: std::collections::BTreeMap::from([(
            "game".into(),
            json!({"limits":{"max_entry_size":1024u64*1024*1024}}),
        )]),
        mounts,
        evidence: vec![
            FileEvidence {
                path: scoped(ini),
                identity: None,
            },
            FileEvidence {
                path: scoped(&bootstrap.program),
                identity: None,
            },
        ],
    })
}
