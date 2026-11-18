// SPDX-License-Identifier: MPL-2.0
//! Caller-owned archive material. Profiles carry scoped filenames/aliases;
//! game permutations and data words never enter project or snapshot JSON.
use anyhow::{Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_resource::{KeyMaterial, KeyProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMaterial {
    pub permutation: String,
    pub data_key: String,
    /// Optional NPSF inverse table. Without it, only a unique algebraic root
    /// of the admitted effective NPA permutation may be used for this profile.
    #[serde(default)]
    pub npf_permutation: Option<String>,
}
impl ArchiveMaterial {
    pub fn validate(&self) -> Result<()> {
        VfsPath::new(Mount::Asset, &self.permutation)?;
        VfsPath::new(Mount::Asset, &self.data_key)?;
        if let Some(path) = &self.npf_permutation {
            VfsPath::new(Mount::Asset, path)?;
        }
        Ok(())
    }
    pub fn provider(
        &self,
        client: ClientId,
        storage: Arc<dyn ClientStorage>,
    ) -> Result<Arc<dyn KeyProvider>> {
        self.validate()?;
        Ok(Arc::new(ScopedKeys {
            client,
            storage,
            material: self.clone(),
        }))
    }
}
struct ScopedKeys {
    client: ClientId,
    storage: Arc<dyn ClientStorage>,
    material: ArchiveMaterial,
}
impl KeyProvider for ScopedKeys {
    fn key(&self, client: ClientId, id: &str) -> Result<KeyMaterial> {
        ensure!(
            client == self.client,
            "N2System archive key belongs to another client"
        );
        let (source, length) = match id {
            "n2-permutation" => (&self.material.permutation, 256),
            "n2-data-key" => (&self.material.data_key, 4),
            _ => anyhow::bail!("N2System unknown archive material alias"),
        };
        let reader = self
            .storage
            .open_read(&VfsPath::new(Mount::Asset, source)?)?;
        ensure!(reader.len()? == length, "N2System archive material length");
        let bytes = crysirror_resource::read_all(reader.as_ref(), length)?;
        if length == 256 {
            let mut seen = [false; 256];
            for b in &bytes {
                ensure!(!seen[*b as usize], "N2System invalid archive permutation");
                seen[*b as usize] = true;
            }
        }
        KeyMaterial::new(bytes)
    }
}
pub fn profile() -> serde_json::Value {
    serde_json::json!({"limits":{"max_entry_size":128u64*1024*1024,"max_resident_bytes":128u64*1024*1024},
        "parameters":{"nitroplus/npa":{"file_name_key_source":"header_product","key_id":"n2-permutation","data_key_id":"n2-data-key"}}})
}
