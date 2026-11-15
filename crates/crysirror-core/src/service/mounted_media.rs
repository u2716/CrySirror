// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Host-published, client-scoped media aliases. A label identifies a mounted
//! asset root; it never establishes an optical drive or an entitlement check.
use crate::{
    Error, Result,
    path::{Mount, VfsPath},
    state::ClientId,
};
use alloc::{string::String, vec::Vec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountedMedia {
    pub label: String,
    /// Legacy drive identity supplied by the host, A through Z.
    pub drive: char,
    /// Canonical directory relative to this client's asset capability.
    pub root: String,
}
impl MountedMedia {
    pub fn validate(&self) -> Result<()> {
        if self.label.is_empty()
            || self.label.len() > 255
            || self
                .label
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
            || !self.drive.is_ascii_uppercase()
        {
            return Err(Error::Invalid("mounted media label or drive"));
        }
        if !self.root.is_empty() && VfsPath::new(Mount::Asset, &self.root)?.relative() != self.root
        {
            return Err(Error::Invalid("mounted media root"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct MountedMediaCatalog {
    client: ClientId,
    volumes: Vec<MountedMedia>,
}
impl MountedMediaCatalog {
    pub fn new(client: ClientId, volumes: Vec<MountedMedia>) -> Result<Self> {
        if volumes.len() > 26 {
            return Err(Error::Limit("mounted media count"));
        }
        for (i, volume) in volumes.iter().enumerate() {
            volume.validate()?;
            if volumes[..i].iter().any(|prior| {
                prior.drive == volume.drive
                    || prior.label.to_lowercase() == volume.label.to_lowercase()
            }) {
                return Err(Error::Invalid("duplicate mounted media"));
            }
        }
        let mut volumes = volumes;
        volumes.sort_by_key(|v| v.drive);
        Ok(Self { client, volumes })
    }
    pub fn client(&self) -> ClientId {
        self.client
    }
    pub fn volumes(&self) -> &[MountedMedia] {
        &self.volumes
    }
    pub fn find_label(&self, client: ClientId, label: &str) -> Result<Option<&MountedMedia>> {
        if client != self.client {
            return Err(Error::Invalid("foreign mounted media client"));
        }
        if label.len() > 255 {
            return Err(Error::Limit("mounted media query"));
        }
        Ok(self
            .volumes
            .iter()
            .find(|v| v.label.to_lowercase() == label.to_lowercase()))
    }
}
