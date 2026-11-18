// SPDX-License-Identifier: MPL-2.0
//! Explicitly retained immutable assets, scoped to one capability owner.
use crysirror_core::{
    Error, Result,
    path::{Mount, VfsPath},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct RetainedAssets(RwLock<State>);
#[derive(Default)]
struct State {
    entries: BTreeMap<String, Arc<[u8]>>,
    bytes: u64,
}
impl RetainedAssets {
    fn key(path: &VfsPath) -> Result<String> {
        if path.mount() != Mount::Asset {
            return Err(Error::ReadOnly);
        }
        Ok(path.relative().to_owned())
    }
    pub fn get(&self, path: &VfsPath) -> Result<Option<Arc<[u8]>>> {
        if path.mount() != Mount::Asset {
            return Ok(None);
        }
        Ok(self
            .0
            .read()
            .map_err(|_| Error::Io("retained assets lock".into()))?
            .entries
            .get(&Self::key(path)?)
            .cloned())
    }
    pub fn insert(&self, path: &VfsPath, bytes: Arc<[u8]>, budget: u64) -> Result<()> {
        let key = Self::key(path)?;
        if budget > 64 * 1024 * 1024 {
            return Err(Error::Limit("retained assets budget"));
        }
        let mut state = self
            .0
            .write()
            .map_err(|_| Error::Io("retained assets lock".into()))?;
        let old = state.entries.get(&key).map_or(0, |v| v.len() as u64);
        let total = state.bytes - old + bytes.len() as u64;
        if total > budget || (!state.entries.contains_key(&key) && state.entries.len() >= 1024) {
            return Err(Error::Limit("retained assets capacity"));
        }
        state.entries.insert(key, bytes);
        state.bytes = total;
        Ok(())
    }
    pub fn remove(&self, path: Option<&VfsPath>) -> Result<()> {
        let key = path.map(Self::key).transpose()?;
        let mut state = self
            .0
            .write()
            .map_err(|_| Error::Io("retained assets lock".into()))?;
        if let Some(key) = key {
            if let Some(bytes) = state.entries.remove(&key) {
                state.bytes -= bytes.len() as u64;
            }
        } else {
            state.entries.clear();
            state.bytes = 0;
        }
        Ok(())
    }
}
