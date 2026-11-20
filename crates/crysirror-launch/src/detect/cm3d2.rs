//! Custom Maid 3D 2: `GameData/*.arc` (KISS warc) plus the Unity player data
//! directory. Never launches the executable or loads the native plug-in.
use super::*;
use std::io::Read;

/// `GameUty.Init` archive order; later entries are optional DLC/patches.
const ARCHIVES: &[(&str, bool)] = &[
    ("csv", true), ("material", true), ("material_chirashi001", false),
    ("material_shop001", false), ("material_shop002", false), ("material_shop003", false),
    ("material_shop004", false), ("material_shop005", false), ("material_shop006", false),
    ("material_shop007", false), ("material_shop008", false), ("material_shop009", false),
    ("menu", true), ("menu_chirashi001", false), ("menu_shop001", false), ("menu_shop002", false),
    ("menu_shop003", false), ("menu_shop004", false), ("menu_shop005", false), ("menu_shop006", false),
    ("menu_shop007", false), ("menu_shop008", false), ("menu_shop009", false),
    ("model", true), ("model_chirashi001", false), ("model_shop001", false), ("model_shop002", false),
    ("model_shop003", false), ("model_shop004", false), ("model_shop005", false), ("model_shop006", false),
    ("model_shop007", false), ("model_shop008", false), ("model_shop009", false),
    ("motion", true), ("script", true), ("sound", true), ("system", true), ("texture", true),
    ("texture_chirashi001", false), ("texture_shop001", false), ("texture_shop002", false),
    ("texture_shop003", false), ("texture_shop004", false), ("texture_shop005", false),
    ("texture_shop006", false), ("texture_shop007", false), ("texture_shop008", false),
    ("texture_shop009", false), ("tjs", true), ("voice", true), ("voice_a", true), ("voice_b", true),
    ("voice_c", true),
];

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(data_dir) = find(names, "CM3D2x64_Data").filter(|d| dir.join(d).is_dir()).cloned() else {
        return Ok(None);
    };
    let Some(game_data) = find(names, "GameData").filter(|d| dir.join(d).is_dir()).cloned() else {
        return Ok(None);
    };
    let root = dir.canonicalize()?;
    let arcs = listing(&dir.join(&game_data))?;
    let mut mounts = Vec::new();
    let mut targets = Vec::new();
    for (stem, required) in ARCHIVES {
        let Some(file) = find(&arcs, &format!("{stem}.arc")) else {
            ensure!(!required, "CM3D2 required archive GameData/{stem}.arc is missing");
            continue;
        };
        let full = dir.join(&game_data).join(file).canonicalize()?;
        ensure!(full.starts_with(&root), "CM3D2 archive escapes the game root");
        let mut magic = [0u8; 8];
        crate::source_path::SourceFile::open(full)?.read_exact(&mut magic)?;
        if magic != *b"warc\xff\xaa\x45\xf1" {
            return Ok(None);
        }
        let target = format!("__cm3d2/{stem}");
        mounts.push(ArchiveMount {
            fallback: false,
            source: format!("{game_data}/{file}"),
            target: target.clone(),
            options: json!({"format": "kiss/warc", "profile": "game"}),
        });
        targets.push(target);
    }
    ensure!(dir.join(&data_dir).join("mainData").is_file(), "CM3D2 Unity mainData is missing");
    Ok(Some(Plan {
        engine: "CM3D2",
        project: project(name, [1920, 1080], "cm3d2", json!({"data_dir": data_dir, "archives": targets})),
        profiles: std::collections::BTreeMap::from([(
            "game".into(),
            json!({"limits": {"max_entry_size": 1024u64 * 1024 * 1024}}),
        )]),
        mounts,
        evidence: Vec::new(),
    }))
}
