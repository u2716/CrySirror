//! Complete BGI runtime maintained inside the CrySirror workspace.
//! The host owns the window, presentation device and PCM output device.
extern crate wgpu as bgi_wgpu;
mod adapter;
mod native_app;
mod native_archive;
mod native_audio;
mod native_core;
mod native_image;
mod native_render;
mod native_script;
mod native_vm;
pub use adapter::{Config, register};
pub use native_app::host_runtime::{HostConfig, HostEvent, HostFrame, HostRuntime};
