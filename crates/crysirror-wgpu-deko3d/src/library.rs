// SPDX-License-Identifier: MPL-2.0
//! Precompiled shader programs, keyed by the exact WGSL the renderer creates.
//!
//! deko3d consumes DKSH produced offline by UAM; there is no shader compiler
//! on the console. `Cooked` loads `<dir>/index.json` written by
//! `crysirror-shader-tools cook-switch`. `Capture` records every module and
//! entry the renderer requests (run on desktop with the null driver) so the
//! cooked set matches the renderer byte for byte. A source that was never
//! cooked fails visibly, naming its hash.
use crate::driver::{Driver, Id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Vertex,
    Fragment,
    Compute,
}

/// Where each WGSL `(group, binding)` lands in the deko3d per-stage slots.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reflection {
    /// (group, binding, uniform buffer slot)
    #[serde(default)]
    pub uniforms: Vec<(u32, u32, u32)>,
    /// (group, binding, storage buffer slot)
    #[serde(default)]
    pub storage: Vec<(u32, u32, u32)>,
    /// (texture slot, texture group, texture binding, sampler group, sampler binding);
    /// sampler group u32::MAX: none (texelFetch only).
    #[serde(default)]
    pub textures: Vec<(u32, u32, u32, u32, u32)>,
    /// (group, binding, image slot) for storage textures.
    #[serde(default)]
    pub images: Vec<(u32, u32, u32)>,
}

#[derive(Debug)]
pub struct Program {
    pub shader: Id,
    pub reflection: Reflection,
}

pub trait ShaderLibrary: Send + Sync + 'static {
    fn program(
        &self,
        driver: &dyn Driver,
        source: &str,
        hash: &str,
        entry: Option<&str>,
        stage: Stage,
    ) -> Result<Arc<Program>, String>;
}

pub fn hash(source: &str) -> String {
    blake3::hash(source.as_bytes()).to_hex()[..32].to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CookedEntry {
    pub entry: String,
    pub stage: Stage,
    pub file: String,
    #[serde(flatten)]
    pub reflection: Reflection,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CookedIndex {
    pub modules: HashMap<String, Vec<CookedEntry>>,
}

pub struct Cooked {
    dir: PathBuf,
    index: CookedIndex,
    loaded: Mutex<HashMap<(String, String, Stage), Arc<Program>>>,
}
impl Cooked {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, String> {
        let dir = dir.into();
        let text = std::fs::read_to_string(dir.join("index.json"))
            .map_err(|e| format!("{}/index.json: {e}", dir.display()))?;
        let index = serde_json::from_str(&text).map_err(|e| format!("shader index: {e}"))?;
        Ok(Self { dir, index, loaded: Mutex::new(HashMap::new()) })
    }
}
impl ShaderLibrary for Cooked {
    fn program(&self, driver: &dyn Driver, _source: &str, hash: &str, entry: Option<&str>, stage: Stage) -> Result<Arc<Program>, String> {
        let entries = self
            .index
            .modules
            .get(hash)
            .ok_or_else(|| format!("shader module {hash} was not cooked for this console build"))?;
        let found: Vec<&CookedEntry> = entries
            .iter()
            .filter(|e| e.stage == stage && entry.is_none_or(|n| n == e.entry))
            .collect();
        let e = match found.as_slice() {
            [one] => *one,
            [] => return Err(format!("shader module {hash}: no cooked {stage:?} entry {entry:?}")),
            _ => return Err(format!("shader module {hash}: {stage:?} entry is ambiguous")),
        };
        let key = (hash.to_string(), e.entry.clone(), stage);
        if let Some(p) = self.loaded.lock().unwrap().get(&key) {
            return Ok(p.clone());
        }
        let code = std::fs::read(self.dir.join(&e.file)).map_err(|err| format!("{}: {err}", e.file))?;
        let shader = driver.shader(&code);
        if shader <= 0 {
            return Err(format!("{}: deko3d rejected the DKSH", e.file));
        }
        let program = Arc::new(Program { shader, reflection: e.reflection.clone() });
        self.loaded.lock().unwrap().insert(key, program.clone());
        Ok(program)
    }
}

/// Records requested modules: `<dir>/<hash>.wgsl` and `<dir>/<hash>.entries`
/// (one `stage entry` per line, `-` when the pipeline named no entry).
pub struct Capture {
    dir: PathBuf,
    seen: Mutex<std::collections::HashSet<(String, String, Stage)>>,
}
impl Capture {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let _ = std::fs::create_dir_all(&dir);
        Self { dir, seen: Mutex::new(Default::default()) }
    }
}
impl ShaderLibrary for Capture {
    fn program(&self, driver: &dyn Driver, source: &str, hash: &str, entry: Option<&str>, stage: Stage) -> Result<Arc<Program>, String> {
        let name = entry.unwrap_or("-").to_string();
        if self.seen.lock().unwrap().insert((hash.to_string(), name.clone(), stage)) {
            let wgsl = self.dir.join(format!("{hash}.wgsl"));
            if !wgsl.exists() {
                std::fs::write(&wgsl, source).map_err(|e| e.to_string())?;
            }
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.dir.join(format!("{hash}.entries")))
                .map_err(|e| e.to_string())?;
            let stage = match stage {
                Stage::Vertex => "vertex",
                Stage::Fragment => "fragment",
                Stage::Compute => "compute",
            };
            writeln!(f, "{stage} {name}").map_err(|e| e.to_string())?;
        }
        Ok(Arc::new(Program { shader: driver.shader(&[]), reflection: Reflection::default() }))
    }
}
