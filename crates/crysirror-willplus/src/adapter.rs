// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! Player factory for the AdvHD WS2 executor (`vm`).
use crate::vm::Machine;
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{HostServices, InputFrame},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_runtime::{model::ClientSpec, runtime::VmFactory, AdapterRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Mounted archive prefixes in lookup order.
    pub archives: Vec<String>,
    /// First WS2 scene (start.ws2 in the original boot).
    pub start: String,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.archives.is_empty() && self.archives.len() <= 64, "AdvHD archive list");
        ensure!(!self.start.is_empty() && self.start.len() <= 64 && !self.start.contains(['/', '\\']), "AdvHD start scene");
        Ok(())
    }
}

pub struct AdvHdVm {
    machine: Machine,
}
impl VmAdapter for AdvHdVm {
    fn engine_id(&self) -> &'static str {
        "willplus"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, budget: u32) -> crysirror_core::Result<VmYield> {
        self.machine.step(host, input, budget.max(1)).map_err(|error| {
            let error = format!("{error:#}");
            self.machine.last_error = Some(error.clone());
            Error::Io(error)
        })
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("AdvHD snapshot"))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("AdvHD restore"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let state = &self.machine.state;
        serde_json::to_vec(&serde_json::json!({"scene":state.scene,"pc":state.pc,"instructions":state.executed,"layers":state.layers.len(),"wait":state.wait,"error":self.machine.last_error,"audio_pending":state.audio_pending.len(),"channels":state.channels.len(),
            "message":{"open":state.msg.open,"page":state.msg.page,"speaker":state.speaker},
            "subs":if std::env::var_os("WILLPLUS_UI_DUMP").is_some() { state.ui.sets.iter().map(|set| serde_json::json!({"set":set.name,"items":set.items.iter().map(|i| serde_json::json!({"key":i.key,"visible":i.visible,"pos":i.params.pos,"kind":format!("{:?}", i.kind).chars().take(60).collect::<String>(),
                "subs":i.subs.iter().filter(|s| s.1.visible).map(|(r, s)| format!("{r}@{:.0},{:.0}a{:.2}", s.params.pos[0], s.params.pos[1], s.params.color[3])).collect::<Vec<_>>()})).collect::<Vec<_>>()})).collect::<Vec<_>>() } else { Vec::new() },
            "sets":state.ui.sets.iter().map(|set| serde_json::json!({"name":set.name,"opacity":set.opacity,
                "items":set.items.iter().filter(|i| i.visible).map(|i| format!("{}@{:.0},{:.0}a{:.2}{:?}{:?}", i.key, i.params.pos[0], i.params.pos[1], i.params.color[3],
                    i.effects.iter().filter(|e| e.1.start.is_some()).map(|e| (e.0, e.1.loops)).collect::<Vec<_>>(),
                    i.subs.iter().flat_map(|(r, s)| s.effects.iter().filter(|e| e.1.start.is_some()).map(move |e| (*r, e.0.clone(), e.1.loops))).collect::<Vec<_>>())).collect::<Vec<_>>()})).collect::<Vec<_>>()})).unwrap_or_default()
    }
}

struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "AdvHD executes native WS2, not data VM actions");
        let config: Config = serde_json::from_value(spec.config.clone())?;
        config.validate()?;
        //Debug-only entry redirect (validation sweeps), like CIRCUS_DC3_START.
        let start = std::env::var("WILLPLUS_START").unwrap_or(config.start.clone());
        Ok(Box::new(AdvHdVm { machine: Machine::new(client, config.archives, &start) }))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("willplus", Arc::new(Factory))
}
