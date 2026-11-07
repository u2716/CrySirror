// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Original Graph arithmetic profile, read as data on the startup worker.
//! No module is loaded or executed; unknown modules grant no native profile.
use crate::bootstrap::{Document, Value};
use anyhow::Result;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::Cancellation;
use sha2::{Digest, Sha256};
pub(crate) fn load(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<bool> {
    load_modules(
        document,
        storage,
        cancel,
        &[
            (
                "DrawGraph.dll",
                122880,
                "d971926868adf5936b598a83dd9185c3e74d9d1ce57a7c7c732ccd5c59d09eae",
            ),
            (
                "Graph.dll",
                114688,
                "7bae2983dd5b82dfc904f1f45359081da115838c5cc2cf1742edf67fab789720",
            ),
        ],
    )
}
pub(crate) fn load_text(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<bool> {
    load_modules(
        document,
        storage,
        cancel,
        &[(
            "DrawText2.dll",
            65536,
            "cb429ab7a32dc1c140f2e502cbb5de7ad72ae71c98c1f77a539ab274337c35c8",
        )],
    )
}
/// Native PlayMovie child-window/transport profile, never executable code.
pub(crate) fn load_movie(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<bool> {
    load_modules(
        document,
        storage,
        cancel,
        &[(
            "PlayMovie.dll",
            49152,
            "4ac952009a13518d759967ea85ccab39339b5398cd0c44f60ff155d6d181c20f",
        )],
    )
}
fn load_modules(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
    modules: &[(&str, u64, &str)],
) -> Result<bool> {
    let Some(entry) = document.entries.iter().rev().find(|e| e.name == "DLL") else {
        return Ok(false);
    };
    let Value::String(folder) = &entry.value else {
        anyhow::bail!("System4 native DLL folder must be string");
    };
    let folder = folder.replace('\\', "/");
    for &(file, length, hash) in modules {
        cancel.check()?;
        let name = if folder.is_empty() {
            file.to_owned()
        } else {
            format!("{folder}/{file}")
        };
        let path = VfsPath::new(Mount::Asset, &name)?;
        let Some(path) =
            crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, &path)?
        else {
            return Ok(false);
        };
        let reader = storage.open_asset(&path)?;
        if reader.len()? != length {
            return Ok(false);
        }
        let bytes = crysirror_resource::read_all(reader.as_ref(), length)?;
        cancel.check()?;
        if format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Ok(false);
        }
    }
    Ok(true)
}
#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crysirror_core::{service::HostServices, state::ClientId};
    use crysirror_runtime::Runtime;
    fn runtime(root: &std::path::Path, state: &std::path::Path) -> Runtime {
        let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"Graph identity","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]})).unwrap();
        Runtime::with_asset_root(project, root, state, Default::default()).unwrap()
    }
    #[test]
    fn missing_changed_cancelled_and_escape_modules_never_grant_native_arithmetic() {
        let t = tempfile::tempdir().unwrap();
        let mut rt = runtime(t.path(), &t.path().join("state"));
        let storage = rt.host.storage(ClientId(1)).unwrap();
        let doc = Document::parse(b"DLL = \"DLL\"\r\n").unwrap();
        assert!(!load(&doc, storage.as_ref(), &Cancellation::default()).unwrap());
        std::fs::create_dir(t.path().join("DLL")).unwrap();
        std::fs::write(t.path().join("DLL/DrawGraph.dll"), vec![0; 122880]).unwrap();
        assert!(!load(&doc, storage.as_ref(), &Cancellation::default()).unwrap());
        let cancel = Cancellation::default();
        cancel.cancel();
        assert!(load(&doc, storage.as_ref(), &cancel).is_err());
        let doc = Document::parse(b"DLL = \"../escape\"\r\n").unwrap();
        assert!(load(&doc, storage.as_ref(), &Cancellation::default()).is_err());
    }
    #[test]
    #[ignore = "owner-installed Graph/DrawGraph modules; scoped read only, no execution"]
    fn owner_graph_profile_requires_both_real_module_hashes() {
        let root = std::path::PathBuf::from(std::env::var("CRYSIRROR_SYSTEM4_ROOT").unwrap());
        let state = tempfile::tempdir().unwrap();
        let mut rt = runtime(&root, state.path());
        let storage = rt.host.storage(ClientId(1)).unwrap();
        let document = Document::parse(
            &storage
                .read(&VfsPath::new(Mount::Asset, "System40.ini").unwrap())
                .unwrap(),
        )
        .unwrap();
        assert!(load(&document, storage.as_ref(), &Cancellation::default()).unwrap());
    }
}
