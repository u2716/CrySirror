// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Client-capability asset routing. Rules are logical state, not filesystem
//! links or executable filters. Every resolved name stays inside the base root.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AssetReader, ClientStorage, StorageLease, StorageMetadata, StorageWrite},
    Error, Result,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

struct Rule {
    target: String,
    find: String,
    replace: String,
}
pub struct RedirectStorage {
    source: Arc<dyn ClientStorage>,
    rules: RwLock<BTreeMap<String, Rule>>,
}
impl RedirectStorage {
    pub fn new(source: Arc<dyn ClientStorage>) -> Self {
        Self {
            source,
            rules: RwLock::new(BTreeMap::new()),
        }
    }
    fn validate(name: &str, empty: bool) -> Result<()> {
        if empty && name.is_empty() {
            return Ok(());
        }
        let path = VfsPath::new(Mount::Asset, name)?;
        if path.relative() != name || name.len() > 1024 {
            return Err(Error::Invalid("asset redirect canonical path"));
        }
        Ok(())
    }
    /// Resolve once for host fast paths that still use this same capability root.
    pub fn resolve(&self, path: &VfsPath) -> Result<VfsPath> {
        if path.mount() != Mount::Asset {
            return Ok(path.clone());
        }
        let rules = self
            .rules
            .read()
            .map_err(|_| Error::Io("asset redirect lock".into()))?;
        let relative = path.relative();
        // Longest matching prefix; one rewrite only, never recursive aliases.
        let rule = rules
            .iter()
            .filter_map(|(source, rule)| {
                if relative
                    .get(..source.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(source))
                    && (relative.len() == source.len()
                        || relative.as_bytes().get(source.len()) == Some(&b'/'))
                {
                    Some((source.len(), rule))
                } else {
                    None
                }
            })
            .max_by_key(|(len, _)| *len);
        let Some((len, rule)) = rule else {
            return Ok(path.clone());
        };
        let member = relative[len..].trim_start_matches('/');
        let member = if rule.find.is_empty() {
            member.to_owned()
        } else {
            member.replace(&rule.find, &rule.replace)
        };
        let output = match (rule.target.is_empty(), member.is_empty()) {
            (true, _) => member,
            (false, true) => rule.target.clone(),
            _ => format!("{}/{member}", rule.target),
        };
        Self::validate(&output, false)?;
        VfsPath::new(Mount::Asset, &output)
    }
    fn relative(&self, mount: Mount, name: &str) -> Result<String> {
        if name.is_empty() {
            return Ok(String::new());
        }
        Ok(self.resolve(&VfsPath::new(mount, name)?)?.relative().into())
    }
}
impl ClientStorage for RedirectStorage {
    fn insert_asset_redirect(
        &self,
        source: &str,
        target: &str,
        find: &str,
        replace: &str,
    ) -> Result<()> {
        Self::validate(source, false)?;
        Self::validate(target, true)?;
        if find.len() > 1024
            || replace.len() > 1024
            || find.contains('\0')
            || replace.contains('\0')
        {
            return Err(Error::Limit("asset redirect substitution"));
        }
        let mut rules = self
            .rules
            .write()
            .map_err(|_| Error::Io("asset redirect lock".into()))?;
        let key = source.to_ascii_lowercase();
        if !rules.contains_key(&key) && rules.len() >= 256 {
            return Err(Error::Limit("asset redirect rules"));
        }
        rules.entry(key).or_insert_with(|| Rule {
            target: target.into(),
            find: find.into(),
            replace: replace.into(),
        });
        Ok(())
    }
    fn erase_asset_redirect(&self, source: &str) -> Result<()> {
        Self::validate(source, false)?;
        self.rules
            .write()
            .map_err(|_| Error::Io("asset redirect lock".into()))?
            .remove(&source.to_ascii_lowercase());
        Ok(())
    }
    fn clear_asset_redirects(&self) -> Result<()> {
        self.rules
            .write()
            .map_err(|_| Error::Io("asset redirect lock".into()))?
            .clear();
        Ok(())
    }
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        self.source.read(&self.resolve(path)?)
    }
    fn open_asset(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        self.source.open_asset(&self.resolve(path)?)
    }
    fn open_read(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        self.source.open_read(&self.resolve(path)?)
    }
    fn metadata(&self, mount: Mount, name: &str) -> Result<Option<StorageMetadata>> {
        self.source.metadata(mount, &self.relative(mount, name)?)
    }
    fn list(&self, mount: Mount, dir: &str) -> Result<Vec<String>> {
        self.source.list(mount, &self.relative(mount, dir)?)
    }
    fn exists(&self, path: &VfsPath) -> bool {
        self.resolve(path)
            .is_ok_and(|path| self.source.exists(&path))
    }
    fn served_by_container(&self, path: &VfsPath) -> bool {
        self.resolve(path)
            .is_ok_and(|path| self.source.served_by_container(&path))
    }
    fn serving_container(&self, path: &VfsPath) -> Option<(String, String)> {
        self.source.serving_container(&self.resolve(path).ok()?)
    }
    fn container_mount(&self, source: &str) -> Result<Option<String>> {
        self.source
            .container_mount(&self.relative(Mount::Asset, source)?)
    }
    fn read_only(&self, mount: Mount, name: &str) -> Result<bool> {
        self.source.read_only(mount, &self.relative(mount, name)?)
    }
    fn write_atomic(&self, path: &VfsPath, bytes: &[u8]) -> Result<()> {
        self.source.write_atomic(path, bytes)
    }
    fn begin_write_atomic(
        &self,
        path: &VfsPath,
        bytes: Arc<[u8]>,
    ) -> Result<Box<dyn StorageWrite>> {
        self.source.begin_write_atomic(path, bytes)
    }
    fn write_atomic_existing_parent(&self, path: &VfsPath, bytes: &[u8]) -> Result<()> {
        self.source.write_atomic_existing_parent(path, bytes)
    }
    fn remove(&self, path: &VfsPath) -> Result<()> {
        self.source.remove(path)
    }
    fn rename_no_replace(&self, source: &VfsPath, target: &VfsPath) -> Result<()> {
        self.source.rename_no_replace(source, target)
    }
    fn create_directory(&self, path: &VfsPath) -> Result<()> {
        self.source.create_directory(path)
    }
    fn remove_directory(&self, path: &VfsPath) -> Result<()> {
        self.source.remove_directory(path)
    }
    fn copy_file(&self, source: &VfsPath, target: &VfsPath, replace: bool) -> Result<()> {
        self.source.copy_file(source, target, replace)
    }
    fn acquire_lease(&self, name: &str) -> Result<Box<dyn StorageLease>> {
        self.source.acquire_lease(name)
    }
}
