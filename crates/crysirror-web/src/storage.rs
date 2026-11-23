// SPDX-License-Identifier: MPL-2.0
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AssetReader, ClientStorage, MemoryAssetReader, StorageMetadata, StorageWrite,
        StorageWriteState,
    },
    Error, Result,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/web/storage.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = loadFiles)]
    async fn load_files(namespace: &str) -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = commitFile)]
    async fn commit_file(
        namespace: &str,
        name: &str,
        bytes: &[u8],
    ) -> std::result::Result<(), JsValue>;
}
fn error(e: impl std::fmt::Debug) -> Error {
    Error::Io(format!("browser storage: {e:?}"))
}
fn key(path: &VfsPath) -> Result<String> {
    match path.mount() {
        Mount::Asset => Err(Error::ReadOnly),
        Mount::Save => Ok(format!("save/{}", path.relative())),
        Mount::Cache => Ok(format!("cache/{}", path.relative())),
    }
}
type Writable = Arc<Mutex<BTreeMap<String, Arc<[u8]>>>>;
pub(crate) struct BrowserStorage {
    assets: Arc<dyn ClientStorage>,
    namespace: String,
    writable: Writable,
    busy: Arc<AtomicBool>,
}
impl BrowserStorage {
    pub async fn refreshed(&self) -> Result<Arc<Self>> {
        Self::open(self.assets.clone(), self.namespace.clone()).await
    }
    pub async fn open(assets: Arc<dyn ClientStorage>, namespace: String) -> Result<Arc<Self>> {
        let rows = load_files(&namespace).await.map_err(error)?;
        let mut writable = BTreeMap::new();
        let mut total = 0usize;
        for row in js_sys::Array::from(&rows) {
            let row = js_sys::Array::from(&row);
            let name = row
                .get(0)
                .as_string()
                .ok_or(Error::Invalid("browser save key"))?;
            let (mount, relative) = if let Some(p) = name.strip_prefix("save/") {
                (Mount::Save, p)
            } else if let Some(p) = name.strip_prefix("cache/") {
                (Mount::Cache, p)
            } else {
                return Err(Error::Invalid("browser save mount"));
            };
            if VfsPath::new(mount, relative)?.relative() != relative {
                return Err(Error::Invalid("browser save path"));
            }
            let bytes = js_sys::Uint8Array::new(&row.get(1)).to_vec();
            total = total
                .checked_add(bytes.len())
                .ok_or(Error::Limit("browser saves"))?;
            if bytes.len() > 4 * 1024 * 1024 || total > 64 * 1024 * 1024 || writable.len() >= 4096 {
                return Err(Error::Limit("browser saves"));
            }
            writable.insert(name, Arc::from(bytes));
        }
        Ok(Arc::new(Self {
            assets,
            namespace,
            writable: Arc::new(Mutex::new(writable)),
            busy: Arc::new(AtomicBool::new(false)),
        }))
    }
}
struct Receipt(Arc<Mutex<StorageWriteState>>);
impl StorageWrite for Receipt {
    fn poll(&mut self) -> Result<StorageWriteState> {
        Ok(self.0.lock().map_err(error)?.clone())
    }
}
impl ClientStorage for BrowserStorage {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        if path.mount() == Mount::Asset {
            return self.assets.read(path);
        }
        self.writable
            .lock()
            .map_err(error)?
            .get(&key(path)?)
            .map(|b| b.to_vec())
            .ok_or(Error::NotFound)
    }
    fn open_asset(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        self.assets.open_asset(path)
    }
    fn open_read(&self, path: &VfsPath) -> Result<Arc<dyn AssetReader>> {
        if path.mount() == Mount::Asset {
            return self.open_asset(path);
        }
        Ok(Arc::new(MemoryAssetReader(
            self.writable
                .lock()
                .map_err(error)?
                .get(&key(path)?)
                .cloned()
                .ok_or(Error::NotFound)?,
        )))
    }
    fn metadata(&self, mount: Mount, relative: &str) -> Result<Option<StorageMetadata>> {
        if mount == Mount::Asset {
            return self.assets.metadata(mount, relative);
        }
        let key = if relative.is_empty() {
            match mount {
                Mount::Save => "save".into(),
                _ => "cache".into(),
            }
        } else {
            key(&VfsPath::new(mount, relative)?)?
        };
        let map = self.writable.lock().map_err(error)?;
        let size = map.get(&key).map(|b| b.len() as u64);
        let directory =
            relative.is_empty() || map.keys().any(|n| n.starts_with(&format!("{key}/")));
        Ok(if size.is_some() || directory {
            Some(StorageMetadata {
                is_directory: directory,
                size,
                created_ms: None,
                modified_ms: None,
                accessed_ms: None,
            })
        } else {
            None
        })
    }
    fn begin_write_atomic(
        &self,
        path: &VfsPath,
        bytes: Arc<[u8]>,
    ) -> Result<Box<dyn StorageWrite>> {
        let name = key(path)?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(Error::Limit("browser atomic write"));
        }
        if self.busy.swap(true, Ordering::AcqRel) {
            return Err(Error::Limit("browser storage write pending"));
        }
        let admitted = (|| -> Result<()> {
            let map = self.writable.lock().map_err(error)?;
            if map.keys().any(|old| {
                old.starts_with(&format!("{name}/")) || name.starts_with(&format!("{old}/"))
            }) {
                return Err(Error::Invalid("browser save shadows a directory"));
            }
            if !map.contains_key(&name) && map.len() >= 4096 {
                return Err(Error::Limit("browser save files"));
            }
            if map
                .iter()
                .filter(|(k, _)| **k != name)
                .map(|(_, b)| b.len())
                .sum::<usize>()
                + bytes.len()
                > 64 * 1024 * 1024
            {
                return Err(Error::Limit("browser save bytes"));
            }
            Ok(())
        })();
        if let Err(e) = admitted {
            self.busy.store(false, Ordering::Release);
            return Err(e);
        }
        let state = Arc::new(Mutex::new(StorageWriteState::Pending));
        let done = state.clone();
        let map = self.writable.clone();
        let busy = self.busy.clone();
        let namespace = self.namespace.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = commit_file(&namespace, &name, &bytes).await;
            let status = match result {
                Ok(()) => {
                    map.lock().unwrap().insert(name, bytes);
                    StorageWriteState::Ready
                }
                Err(e) => StorageWriteState::Failed(format!("{e:?}")),
            };
            *done.lock().unwrap() = status;
            busy.store(false, Ordering::Release);
        });
        Ok(Box::new(Receipt(state)))
    }
    fn write_atomic(&self, path: &VfsPath, _: &[u8]) -> Result<()> {
        key(path)?;
        Err(Error::Unsupported(
            "synchronous browser save; use begin_write_atomic",
        ))
    }
    fn remove(&self, path: &VfsPath) -> Result<()> {
        key(path)?;
        Err(Error::Unsupported("synchronous browser remove"))
    }
    fn list(&self, mount: Mount, dir: &str) -> Result<Vec<String>> {
        if mount == Mount::Asset {
            return self.assets.list(mount, dir);
        }
        let prefix = if dir.is_empty() {
            format!(
                "{}/",
                if mount == Mount::Save {
                    "save"
                } else {
                    "cache"
                }
            )
        } else {
            format!("{}/", key(&VfsPath::new(mount, dir)?)?)
        };
        let map = self.writable.lock().map_err(error)?;
        let names: BTreeSet<_> = map
            .keys()
            .filter_map(|n| n.strip_prefix(&prefix)?.split('/').next().map(String::from))
            .collect();
        Ok(names.into_iter().collect())
    }
}
