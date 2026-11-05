//! SoftPAL original runtime, hosted by CrySirror without a second window or device.
//! Native execution and its host PCM/frame port are owned by a dedicated worker.
mod adapter;
pub mod native_asset;
pub mod native_script;
pub mod native_vm;
pub use adapter::{register, Config, SoftpalVm};
mod detection;
pub use detection::detect_game;
