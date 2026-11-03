// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Reviewed native optional-interface policy. Modules are read as scoped data,
//! never loaded/executed. An unknown version grants no absent-interface profile.
use crate::bootstrap::{Document, Value};
use anyhow::Result;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::Cancellation;
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReviewedMusic {
    Rance7Original,
}
const SACT: &str = "3b03b168e11f136efc22168f8b0e019f107f4350b1c210a2d45ef82913e27659";
const MUSIC: &str = "601d5ffc106ec8837c4a8e80153ac659da564a3178e3f20ea8eeecefcafdcd4f";
pub(crate) fn load(
    document: &Document,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<Option<ReviewedMusic>> {
    let Some(entry) = document.entries.iter().rev().find(|e| e.name == "DLL") else {
        return Ok(None);
    };
    let Value::String(folder) = &entry.value else {
        anyhow::bail!("System4 native DLL folder must be string")
    };
    let folder = folder.replace('\\', "/");
    for (file, length, hash) in [
        ("SACT2.dll", 282624, SACT),
        ("MusicSystem.dll", 102400, MUSIC),
    ] {
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
            return Ok(None);
        };
        let reader = storage.open_asset(&path)?;
        if reader.len()? != length {
            return Ok(None);
        }
        let bytes = crysirror_resource::read_all(reader.as_ref(), length)?;
        cancel.check()?;
        if format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Ok(None);
        }
    }
    Ok(Some(ReviewedMusic::Rance7Original))
}
impl ReviewedMusic {
    // Native resource CMusic::GetInterface compares the exact IMusicPause GUID
    // and prints 'not implemented', then returns null. All three original
    // facade calls return0 for that absence; this is not a generic fake Pause.
    pub(crate) fn unavailable(self, name: &[u8]) -> bool {
        match self {
            Self::Rance7Original => {
                matches!(
                    name,
                    b"Music_Pause"
                        | b"Music_Restart"
                        | b"Music_IsPause"
                        | b"Music_GetLoopCount"
                        | b"Music_GetSamplePos"
                        | b"Music_GetSampleLength"
                        | b"Music_Seek"
                )
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{service::MemoryAssetReader, Error};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Storage {
        opened: Arc<AtomicUsize>,
        path: &'static str,
        size: usize,
    }
    impl ClientStorage for Storage {
        fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            panic!("bounded reader required")
        }
        fn open_asset(
            &self,
            _: &VfsPath,
        ) -> crysirror_core::Result<Arc<dyn crysirror_core::service::AssetReader>> {
            self.opened.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(MemoryAssetReader(vec![0; self.size].into())))
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn list(&self, _: Mount, dir: &str) -> crysirror_core::Result<Vec<String>> {
            Ok(if dir.is_empty() {
                vec!["dll".into()]
            } else if dir == "dll" {
                vec![self.path.into()]
            } else {
                vec![]
            })
        }
    }
    #[test]
    fn absent_unknown_changed_or_cancelled_modules_do_not_grant_reviewed_interface_policy() {
        let d = Document::parse(br#"CodeName="main.ain" DLL="DLL""#).unwrap();
        let opened = Arc::new(AtomicUsize::new(0));
        let s = Storage {
            opened: opened.clone(),
            path: "other.dll",
            size: 0,
        };
        assert_eq!(load(&d, &s, &Cancellation::default()).unwrap(), None);
        assert_eq!(opened.load(Ordering::SeqCst), 0);
        let s = Storage {
            opened: opened.clone(),
            path: "sact2.dll",
            size: 1,
        };
        assert_eq!(load(&d, &s, &Cancellation::default()).unwrap(), None);
        let s = Storage {
            opened: opened.clone(),
            path: "sact2.dll",
            size: 282624,
        };
        assert_eq!(load(&d, &s, &Cancellation::default()).unwrap(), None); // Exact size is not proof.
        let cancel = Cancellation::default();
        cancel.cancel();
        let count = opened.load(Ordering::SeqCst);
        assert!(load(&d, &s, &cancel).is_err());
        assert_eq!(opened.load(Ordering::SeqCst), count);
        let escaped = Document::parse(br#"CodeName="main.ain" DLL="../DLL""#).unwrap();
        assert!(load(&escaped, &s, &Cancellation::default()).is_err());
    }
    #[cfg(feature = "runtime")]
    #[test]
    #[ignore = "owner-installed original modules; runtime read only, no execution"]
    fn owner_original_music_profile_is_bound_only_after_both_scoped_module_hashes_match() {
        use crysirror_core::{service::HostServices, state::ClientId};
        let root = std::path::PathBuf::from(
            std::env::var_os("CRYSIRROR_SYSTEM4_ROOT").expect("original System4 root"),
        );
        let temp = tempfile::tempdir().unwrap();
        let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native module evidence","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"test"}]})).unwrap();
        let mut r = crysirror_runtime::Runtime::with_asset_root(
            project,
            &root,
            temp.path(),
            Default::default(),
        )
        .unwrap();
        let storage = r.host.storage(ClientId(1)).unwrap();
        let path = VfsPath::new(Mount::Asset, "System40.ini").unwrap();
        let ini = storage.open_asset(&path).unwrap();
        let bytes = crysirror_resource::read_all(ini.as_ref(), 1024 * 1024).unwrap();
        assert_eq!(
            load(
                &Document::parse(&bytes).unwrap(),
                storage.as_ref(),
                &Cancellation::default()
            )
            .unwrap(),
            Some(ReviewedMusic::Rance7Original)
        );
    }
}
