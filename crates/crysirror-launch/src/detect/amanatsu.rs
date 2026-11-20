//! 甘夏ろけーしょん (ILLGAMES `AmanatsuLocation`, Unity 6 IL2CPP): the
//! player data directory and the `lib/` asset bundles. Never launches the
//! executable or loads `GameAssembly.dll`.
use super::*;

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(data_dir) = find(names, "AmanatsuLocation_Data").filter(|d| dir.join(d).is_dir()).cloned() else {
        return Ok(None);
    };
    if find(names, "lib").is_none_or(|l| !dir.join(l).is_dir()) || find(names, "GameAssembly.dll").is_none() {
        return Ok(None);
    }
    ensure!(dir.join(&data_dir).join("globalgamemanagers").is_file(), "AmanatsuLocation globalgamemanagers is missing");
    Ok(Some(Plan {
        engine: "AmanatsuLocation",
        project: project(name, [1920, 1080], "amanatsu", json!({"data_dir": data_dir})),
        profiles: std::collections::BTreeMap::new(),
        mounts: Vec::new(),
        evidence: Vec::new(),
    }))
}
