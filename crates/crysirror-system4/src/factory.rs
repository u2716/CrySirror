// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Explicit first-class AIN factory. Host detection supplies original paths;
//! it does not translate a game into data-VM actions or run an old engine.
use anyhow::{ensure, Result};
use crysirror_core::{state::ClientId, vm::VmAdapter};
use crysirror_runtime::{model::ClientSpec, runtime::VmFactory, AdapterRegistry};
use std::sync::Arc;
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "System4 executes original AIN, not data VM actions"
        );
        Ok(Box::new(
            crate::worker_adapter::NativeAdapter::from_factory(
                client,
                serde_json::from_value(spec.config.clone())?,
            )?,
        ))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("system4", Arc::new(Factory))
}
