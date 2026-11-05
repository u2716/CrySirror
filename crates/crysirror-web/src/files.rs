// SPDX-License-Identifier: MPL-2.0
//! Immutable, explicitly imported browser files; no ambient filesystem access.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AssetReader, ClientStorage, MemoryAssetReader, StorageMetadata},
    Error, Result,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const MAX_IMPORT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_IMPORT_FILE_BYTES: usize = 64 * 1024 * 1024;
/// Files are admitted once before the Runtime starts. Asset reads never start
/// a hidden network request, block on a browser promise, or expose disk paths.
#[derive(Default)]
pub struct ImportedFiles {
    files: BTreeMap<String, Arc<[u8]>>,
    bytes: usize,
}
impl ImportedFiles {
    pub fn insert(&mut self, relative: &str, bytes: Vec<u8>) -> Result<()> {
        let name = VfsPath::new(Mount::Asset, relative)?.relative().to_owned();
        if self.files.contains_key(&name) {
            return Err(Error::Invalid("duplicate imported file"));
        }
        if bytes.len() > MAX_IMPORT_FILE_BYTES
            || self.files.len() >= 65536
            || self
                .bytes
                .checked_add(bytes.len())
                .is_none_or(|n| n > MAX_IMPORT_BYTES)
        {
            return Err(Error::Limit("browser imported files"));
        }
        let child_prefix = format!("{name}/");
        let shadows_child = self
            .files
            .range(child_prefix.clone()..)
            .next()
            .is_some_and(|(old, _)| old.starts_with(&child_prefix));
        let shadows_parent = name
            .match_indices('/')
            .any(|(at, _)| self.files.contains_key(&name[..at]));
        if shadows_child || shadows_parent {
            return Err(Error::Invalid("imported file shadows a directory"));
        }
        self.bytes += bytes.len();
        self.files.insert(name, bytes.into());
        Ok(())
    }
    pub fn scoped(self: &Arc<Self>, prefix: &str) -> Result<Arc<dyn ClientStorage>> {
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            VfsPath::new(Mount::Asset, prefix)?.relative().into()
        };
        if !prefix.is_empty()
            && !self
                .files
                .keys()
                .any(|name| name.starts_with(&format!("{prefix}/")))
        {
            return Err(Error::NotFound);
        }
        Ok(Arc::new(ScopedFiles {
            files: self.clone(),
            prefix,
        }))
    }
}
struct ScopedFiles {
    files: Arc<ImportedFiles>,
    prefix: String,
}
impl ScopedFiles {
    fn name(&self, mount: Mount, relative: &str) -> Result<String> {
        if mount != Mount::Asset {
            return Err(Error::ReadOnly);
        }
        let relative = if relative.is_empty() {
            ""
        } else {
            if VfsPath::new(mount, relative)?.relative() != relative {
                return Err(Error::Invalid("noncanonical browser path"));
            }
            relative
        };
        Ok(match (self.prefix.is_empty(), relative.is_empty()) {
            (true, _) => relative.into(),
            (_, true) => self.prefix.clone(),
            _ => format!("{}/{}", self.prefix, relative),
        })
    }
}
impl ClientStorage for ScopedFiles {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        let reader = self.open_asset(path)?;
        let mut bytes = vec![0; reader.len()? as usize];
        reader.read_at(0, &mut bytes)?;
        Ok(bytes)
    }
    fn open_asset(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        let name = self.name(path.mount(), path.relative())?;
        Ok(Arc::new(MemoryAssetReader(
            self.files
                .files
                .get(&name)
                .cloned()
                .ok_or(Error::NotFound)?,
        )))
    }
    fn open_read(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        self.open_asset(path)
    }
    fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> Result<()> {
        Err(Error::ReadOnly)
    }
    fn remove(&self, _: &VfsPath) -> Result<()> {
        Err(Error::ReadOnly)
    }
    fn metadata(&self, mount: Mount, relative: &str) -> Result<Option<StorageMetadata>> {
        let name = self.name(mount, relative)?;
        let file = self.files.files.get(&name);
        let directory = name.is_empty()
            || self
                .files
                .files
                .keys()
                .any(|n| n.starts_with(&format!("{name}/")));
        Ok(if file.is_some() || directory {
            Some(StorageMetadata {
                is_directory: directory,
                size: file.map(|b| b.len() as u64),
                created_ms: None,
                modified_ms: None,
                accessed_ms: None,
            })
        } else {
            None
        })
    }
    fn read_only(&self, mount: Mount, relative: &str) -> Result<bool> {
        self.name(mount, relative)?;
        Ok(true)
    }
    fn list(&self, mount: Mount, dir: &str) -> Result<Vec<String>> {
        let name = self.name(mount, dir)?;
        let prefix = if name.is_empty() {
            String::new()
        } else {
            format!("{name}/")
        };
        let names: BTreeSet<_> = self
            .files
            .files
            .keys()
            .filter_map(|n| n.strip_prefix(&prefix)?.split('/').next().map(String::from))
            .collect();
        Ok(names.into_iter().collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_failed_budget_admission_does_not_install_the_file() {
        let mut files = ImportedFiles::default();
        files.bytes = MAX_IMPORT_BYTES;
        assert!(files.insert("over.bin", vec![1]).is_err());
        assert!(!files.files.contains_key("over.bin"));
    }
    #[test]
    fn portable_runtime_uses_scoped_roots_and_existing_snapshot_semantics() {
        use crysirror_core::{
            service::{FileSystem, InputFrame},
            state::ClientId,
        };
        use crysirror_runtime::{AdapterRegistry, Project, Runtime};
        let mut files = ImportedFiles::default();
        files
            .insert("assets/one/item.bin", b"one".to_vec())
            .unwrap();
        files
            .insert("assets/two/item.bin", b"two".to_vec())
            .unwrap();
        let files = Arc::new(files);
        let project: Project = serde_json::from_value(serde_json::json!({
            "schema":2,"name":"portable","content_revision":"1","canvas":[64,64],"assets":"assets",
            "clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one","asset_prefix":"one","program":[{"op":"wait_input"}]},
                       {"name":"two","adapter":"crysirror-script-v1","game":"two","asset_prefix":"two","program":[{"op":"wait_input"}]}]
        })).unwrap();
        let clients = BTreeMap::from([
            (ClientId(1), files.scoped("assets/one").unwrap()),
            (ClientId(2), files.scoped("assets/two").unwrap()),
        ]);
        let mut runtime = Runtime::with_storage(
            project.clone(),
            files.scoped("assets").unwrap(),
            clients,
            AdapterRegistry::default(),
        )
        .unwrap();
        let path = VfsPath::new(Mount::Asset, "item.bin").unwrap();
        assert_eq!(runtime.host.fs.read(ClientId(1), &path).unwrap(), b"one");
        assert_eq!(
            runtime
                .assets
                .read_for(ClientId(2), "item.bin")
                .unwrap()
                .as_ref(),
            b"two"
        );
        assert_eq!(
            runtime
                .assets
                .read_async(ClientId(1), "item.bin")
                .unwrap()
                .wait()
                .unwrap()
                .as_ref(),
            b"one"
        );
        assert!(runtime.host.fs.read(ClientId(3), &path).is_err());
        assert!(runtime
            .assets
            .read_for(ClientId(1), "two/item.bin")
            .is_err());
        runtime.tick(10, InputFrame::default()).unwrap();
        let bytes = runtime.snapshot().unwrap();
        runtime.tick(20, InputFrame::default()).unwrap();
        runtime.restore(&bytes).unwrap();
        assert_eq!(runtime.now_ms(), 10);
        assert!(Runtime::with_storage(
            project,
            files.scoped("assets").unwrap(),
            BTreeMap::new(),
            AdapterRegistry::default()
        )
        .is_err());
    }
    #[test]
    fn imports_are_bounded_and_capabilities_cannot_escape_their_prefix() {
        let mut files = ImportedFiles::default();
        files.insert("one/a.bin", b"abcd".to_vec()).unwrap();
        files.insert("two/private.bin", b"secret".to_vec()).unwrap();
        assert!(files.insert("one/./a.bin", vec![]).is_err());
        assert!(files.insert("../outside", vec![]).is_err());
        assert!(files.insert("one", vec![]).is_err());
        let files = Arc::new(files);
        let one = files.scoped("one").unwrap();
        let path = VfsPath::new(Mount::Asset, "a.bin").unwrap();
        let reader = one.open_asset(&path).unwrap();
        let mut out = [0; 3];
        assert_eq!(reader.read_at(2, &mut out).unwrap(), 2);
        assert_eq!(&out[..2], b"cd");
        assert!(one
            .read(&VfsPath::new(Mount::Asset, "two/private.bin").unwrap())
            .is_err());
        assert!(one
            .read(&VfsPath::new(Mount::Save, "a.bin").unwrap())
            .is_err());
        assert_eq!(one.list(Mount::Asset, "").unwrap(), vec!["a.bin"]);
        assert!(one.write_atomic(&path, b"overwrite").is_err());
    }
}
