// SPDX-License-Identifier: MPL-2.0
//! Renderer shader modules resolved to the native PSV GXP catalog.
//!
//! `cooked/index.json` is written by `crysirror-shader-tools cook-vita` from a
//! renderer capture: it is keyed by the blake3 of the exact WGSL the shared
//! renderer creates and joins each entry with its compiled GXP programs
//! (`crysirror_shaders::native::PSV_PROGRAMS`) and reflection. No shader is
//! compiled on the device. A module that has no native program still yields a
//! pipeline, but the driver refuses each of its draws visibly.
use crysirror_wgpu_deko3d::driver::Driver;
use crysirror_wgpu_deko3d::library::{Program, Reflection, ShaderLibrary, Stage};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const INDEX: &str = include_str!("../cooked/index.json");

#[derive(Deserialize)]
struct Index {
    modules: HashMap<String, Vec<Entry>>,
}
#[derive(Deserialize)]
struct Entry {
    entry: String,
    stage: String,
    #[serde(default)]
    unavailable: bool,
    #[serde(default)]
    uniforms: Vec<(u32, u32, u32)>,
    #[serde(default)]
    storage: Vec<(u32, u32, u32)>,
    #[serde(default)]
    textures: Vec<(u32, u32, u32, u32, u32)>,
    /// Storage outputs of a compute entry with a fragment lowering.
    #[serde(default)]
    images: Vec<(u32, u32, u32)>,
    #[serde(default)]
    gxm: Value,
}

pub struct Library {
    index: Index,
    loaded: Mutex<HashMap<(String, String, Stage), Arc<Program>>>,
}

impl Library {
    pub fn embedded() -> Result<Self, String> {
        let index = serde_json::from_str(INDEX).map_err(|e| format!("PSV shader index: {e}"))?;
        Ok(Self { index, loaded: Mutex::new(HashMap::new()) })
    }
    /// The program description the driver receives for one entry.
    pub fn code(&self, hash: &str, entry: Option<&str>, stage: Stage) -> Result<(Vec<u8>, Reflection), String> {
        let stage_name = match stage {
            Stage::Vertex => "vertex",
            Stage::Fragment => "fragment",
            Stage::Compute => "compute",
        };
        let Some(entries) = self.index.modules.get(hash) else {
            return Ok((format!("unavailable {hash} {stage_name} {}", entry.unwrap_or("-")).into_bytes(), Reflection::default()));
        };
        let found: Vec<&Entry> = entries.iter().filter(|e| e.stage == stage_name && entry.is_none_or(|n| n == e.entry)).collect();
        let e = match found.as_slice() {
            [one] => *one,
            [] => return Ok((format!("unavailable {hash} {stage_name} {}", entry.unwrap_or("-")).into_bytes(), Reflection::default())),
            _ => return Err(format!("PSV shader module {hash}: {stage_name} entry is ambiguous")),
        };
        if e.unavailable {
            return Ok((format!("unavailable {hash} {stage_name} {}", e.entry).into_bytes(), Reflection::default()));
        }
        let code = serde_json::to_vec(&serde_json::json!({"name": format!("{hash}-{}", e.entry), "stage": stage_name, "gxm": e.gxm}))
            .map_err(|err| err.to_string())?;
        let reflection = Reflection { uniforms: e.uniforms.clone(), storage: e.storage.clone(), textures: e.textures.clone(), images: e.images.clone() };
        Ok((code, reflection))
    }
}

impl ShaderLibrary for Library {
    fn program(&self, driver: &dyn Driver, _source: &str, hash: &str, entry: Option<&str>, stage: Stage) -> Result<Arc<Program>, String> {
        let key = (hash.to_string(), entry.unwrap_or("-").to_string(), stage);
        if let Some(p) = self.loaded.lock().unwrap().get(&key) {
            return Ok(p.clone());
        }
        let (code, reflection) = self.code(hash, entry, stage)?;
        let shader = driver.shader(&code);
        if shader <= 0 {
            return Err(format!("PSV shader {hash} {entry:?}: GXM driver rejected the program"));
        }
        let program = Arc::new(Program { shader, reflection });
        self.loaded.lock().unwrap().insert(key, program.clone());
        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_cooked_variant_is_in_the_native_catalog() {
        let library = Library::embedded().unwrap();
        let mut programs = 0;
        for entries in library.index.modules.values() {
            for e in entries.iter().filter(|e| !e.unavailable) {
                for v in e.gxm["variants"].as_array().unwrap() {
                    let name = v["program"].as_str().unwrap();
                    assert!(crysirror_shaders::native::psv_program(name).is_some(), "{name}");
                    programs += 1;
                }
            }
        }
        assert!(programs > 0);
    }
}
