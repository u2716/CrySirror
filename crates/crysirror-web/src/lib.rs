// SPDX-License-Identifier: MPL-2.0
//! Browser host for the shared CrySirror Runtime and wgpu renderer.
#![forbid(unsafe_code)]
#[cfg(target_arch = "wasm32")]
mod browser;
pub mod files;
#[cfg(target_arch = "wasm32")]
mod storage;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserPlayer;

#[cfg(all(target_arch = "wasm32", feature = "threads"))]
mod threads;
#[cfg(all(target_arch = "wasm32", feature = "threads"))]
pub use threads::init_cpu_workers;
