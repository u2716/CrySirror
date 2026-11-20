//! Select the original SoftPAL runtime from its own archive/script structures.
use super::*;

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(local) = dir.local_path() else {
        ensure!(!names.iter().any(|name| matches!(name.to_ascii_lowercase().as_str(), "archive.dat" | "script.src" | "data.pac" | "csv.pac")), "SoftPAL direct integration currently requires a local or OS-mounted game directory");
        return Ok(None);
    };
    if !crysirror_softpal::detect_game(local)? {
        return Ok(None);
    }
    let mut project = project(name, [800, 600], "softpal", json!({}));
    if let Some(font) = find(names, "default.ttf").or_else(|| find(names, "default.otf")) {
        project["fonts"] = json!({"default": font});
    }
    let evidence = ["archive.dat", "Script.src"]
        .into_iter()
        .filter_map(|wanted| find(names, wanted))
        .map(|path| FileEvidence {
            path: path.clone(),
            identity: None,
        })
        .collect();
    Ok(Some(Plan {
        engine: "SoftPAL",
        project,
        profiles: Default::default(),
        // The original ResourceManager reads PACs directly; no second index or
        // externally extracted copy is needed by this runtime.
        mounts: Vec::new(),
        evidence,
    }))
}
