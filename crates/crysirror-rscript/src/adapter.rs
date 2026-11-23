// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! `VmAdapter` over the RScript scene: boot (scripts read on a worker),
//! owner steps and diagnostics.
use crate::apini::Apini;
use crate::gsc::Program;
use crate::res::{Lwg, Resources};
use crate::vm::Vm;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{HostServices, InputFrame},
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Launch configuration produced by installed-game detection.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub apini: Apini,
    /// Case-folded native path with extension → mounted asset path.
    pub assets: BTreeMap<String, String>,
    /// Case-folded native path of every LWG (without extension).
    pub lwg: BTreeMap<String, Lwg>,
    /// Project font alias for message text.
    pub font: String,
}

const MAX_SCRIPT: u64 = 16 * 1024 * 1024;

pub struct RScriptVm {
    vm: Vm,
    font: String,
    pool: Option<TaskPool>,
    boot: Option<Task<BTreeMap<i32, Arc<Program>>>>,
    booted: bool,
    error: Option<String>,
}

impl RScriptVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        ensure!(!config.assets.is_empty(), "RScript launch without resources");
        let res = Resources::new(config.assets, config.lwg);
        let mut vm = Vm::new(client, config.apini, res);
        vm.ui_font = config.font.clone();
        Ok(Self { vm, font: config.font, pool: None, boot: None, booted: false, error: None })
    }
    fn begin_boot(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let storage = h.storage(self.vm.client)?;
        let dir = crate::res::key(self.vm.apini.dir(crate::apini::Dir::Scripts)).context("RScript script directory")?;
        let scripts: Vec<(i32, String)> = self
            .vm
            .res
            .assets
            .iter()
            .filter_map(|(k, v)| {
                let name = k.strip_prefix(&format!("{dir}/"))?.strip_suffix(".gsc")?;
                Some((name.parse().ok()?, v.clone()))
            })
            .collect();
        ensure!(!scripts.is_empty(), "RScript game has no {dir}/*.gsc scripts");
        let pool = self.pool.get_or_insert(TaskPool::new("rscript-scripts", 1, 4)?);
        self.boot = Some(pool.spawn(move |cancel| {
            let mut out = BTreeMap::new();
            for (n, path) in scripts {
                cancel.check()?;
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
                let bytes = crysirror_resource::read_all(reader.as_ref(), MAX_SCRIPT)?;
                let p = Program::parse(&bytes).with_context(|| format!("RScript {path}"))?;
                out.insert(n, Arc::new(p));
            }
            Ok(out)
        })?);
        Ok(())
    }
    fn step_inner(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield> {
        if self.vm.halted {
            return Ok(VmYield::Halted);
        }
        if !self.booted {
            if self.boot.is_none() {
                self.begin_boot(h)?;
            }
            match self.boot.as_mut().unwrap().try_take()? {
                None => return Ok(VmYield::BudgetExhausted),
                Some(programs) => {
                    self.vm.programs = programs;
                    self.boot = None;
                    self.booted = true;
                    // sub_432D60: defaults, replaced by `<prefix>.dat` when present.
                    let apini = self.vm.apini.clone();
                    self.vm.mem.init_config(&apini);
                    self.vm.read_system(h)?;
                    let face = crate::config::font_name(&self.vm.mem);
                    let fonts = h.font_library(self.vm.client).and_then(|l| l.aliases()).unwrap_or_default();
                    self.vm.text_font = fonts.into_iter().find(|f| *f == face);
                    self.vm.start_opening(h)?;
                }
            }
        }
        let now = h.now_ms();
        if std::mem::take(&mut self.vm.pending_restore) {
            self.vm.reenter_game(h)?;
        }
        self.vm.update(h, input, now, budget)?;
        self.vm.present(h, &self.font)?;
        Ok(match self.vm.wait {
            None => VmYield::BudgetExhausted,
            Some(_) => VmYield::WaitUntil(self.vm.next_tick.max(now + 1)),
        })
    }
}

/// Framework snapshot header.
const SNAPSHOT_MAGIC: [u8; 4] = *b"RSS1";

impl VmAdapter for RScriptVm {
    fn engine_id(&self) -> &'static str {
        "rscript"
    }
    fn instruction_budget(&self) -> u32 {
        4096
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        !self.booted
    }
    fn step(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> crysirror_core::Result<VmYield> {
        self.step_inner(h, input, budget).map_err(|e| {
            let message = format!("{e:#}");
            self.error = Some(message.clone());
            Error::Io(message)
        })
    }
    /// The native safepoint: the message snapshot and previous-choice
    /// snapshot as an original slot (sub_433240 layout, zero date), then the
    /// system data (sub_433180 layout). Restore re-enters the game state
    /// like a slot load.
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        let vm = &self.vm;
        let slot = crate::save::encode_slot(&vm.mem, [0; 5], &vm.message.page_text);
        let system = crate::save::encode_system(&vm.mem, &vm.read_text, &vm.seen_images);
        let mut out = Vec::with_capacity(12 + slot.len() + system.len());
        out.extend(SNAPSHOT_MAGIC);
        out.extend((slot.len() as u32).to_le_bytes());
        out.extend((system.len() as u32).to_le_bytes());
        out.extend(slot);
        out.extend(system);
        Ok(out)
    }
    fn restore(&mut self, bytes: &[u8]) -> crysirror_core::Result<()> {
        let bad = || Error::Io("invalid RScript snapshot".into());
        if bytes.len() < 12 || bytes[..4] != SNAPSHOT_MAGIC {
            return Err(bad());
        }
        let n = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let m = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        if bytes.len() != 12 + n + m {
            return Err(bad());
        }
        let mut mem = self.vm.mem.clone();
        crate::save::decode_slot(&mut mem, &bytes[12..12 + n]).map_err(|e| Error::Io(format!("{e:#}")))?;
        let (read, seen) = crate::save::decode_system(&mut mem, &bytes[12 + n..]);
        self.vm.mem = mem;
        self.vm.read_text = read;
        self.vm.seen_images = seen;
        self.vm.pending_restore = true;
        Ok(())
    }
    /// An original slot of the current snapshots (sub_433240, zero date).
    fn export_original_save(&mut self) -> crysirror_core::Result<Vec<u8>> {
        Ok(crate::save::encode_slot(&self.vm.mem, [0; 5], &self.vm.message.page_text))
    }
    /// An original slot (sub_432FB0), validated before the live state
    /// changes; the game state is entered again on the next step.
    fn import_original_save(&mut self, bytes: &[u8]) -> crysirror_core::Result<()> {
        let mut mem = self.vm.mem.clone();
        crate::save::decode_slot(&mut mem, bytes).map_err(|e| Error::Io(format!("{e:#}")))?;
        self.vm.mem = mem;
        self.vm.pending_restore = true;
        Ok(())
    }
    fn diagnostics(&self) -> Vec<u8> {
        let vm = &self.vm;
        serde_json::to_vec(&json!({"adapter":"rscript","mode":"adopt","layout":vm.apini.layout,
            "script":vm.thread.script,"pc":format!("{:#x}",vm.thread.pc),"depth":vm.depth(),"nested":vm.nest.len(),
            "executed":vm.executed,"wait":format!("{:?}",vm.wait),"conts":vm.cont.len(),
            "text":vm.message.boxes[0].text.plain(),"name":vm.message.boxes[0].name.plain(),
            "choice":vm.choice.as_ref().map(|c|c.answers.iter().map(|a|a.text.plain()).collect::<Vec<_>>()),
            "music":vm.music,"diagnostics":vm.res.diagnostics,"error":self.error}))
        .unwrap_or_default()
    }
}

#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register("rscript", Arc::new(Factory))
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &crysirror_runtime::model::ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "RScript executes native GSC programs, not data actions");
        Ok(Box::new(RScriptVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}
