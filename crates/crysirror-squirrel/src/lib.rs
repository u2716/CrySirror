// SPDX-License-Identifier: MIT
//! Bounded, single-owner Squirrel source sessions with an explicit host callback.
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::Interpreter;

/// The browser toolchain has no C++ runtime for this backend yet. Preserve
/// build compatibility and reject the unavailable native interpreter explicitly.
#[cfg(target_arch = "wasm32")]
pub struct Interpreter;
#[cfg(target_arch = "wasm32")]
impl Interpreter {
    pub fn new(
        _: impl FnMut(Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> + 'static,
    ) -> anyhow::Result<Self> {
        anyhow::bail!("Squirrel browser backend is not connected")
    }
    pub fn eval(&mut self, _: &str, _: u64) -> anyhow::Result<()> {
        anyhow::bail!("Squirrel browser backend is not connected")
    }
    pub fn call(&mut self, _: &str, _: u64) -> anyhow::Result<()> {
        anyhow::bail!("Squirrel browser backend is not connected")
    }
    pub fn callback(&mut self, _: u32, _: u64) -> anyhow::Result<()> {
        anyhow::bail!("Squirrel browser backend is not connected")
    }
}
