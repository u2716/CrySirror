//! Complete BGI runtime maintained inside the CrySirror workspace.
//! The host owns the window, presentation device and PCM output device.
extern crate wgpu as bgi_wgpu;
#[cfg(feature = "crysirror-host")]
mod adapter;
#[cfg(all(target_os = "vita", feature = "vita-gxm"))]
pub mod console;
mod native_app;
mod native_archive;
mod native_audio;
mod native_core;
mod native_image;
mod native_render;
mod native_script;
mod native_vm;
#[cfg(all(target_os = "vita", feature = "vita-gxm"))]
pub mod vita;
#[cfg(feature = "crysirror-host")]
pub use adapter::{Config, register};
pub use native_app::host_runtime::{HostConfig, HostEvent, HostFrame, HostRuntime};

/// Bootstrap container names accepted by the retained publisher-language view.
/// Selection and fallback order remain owned by the original archive manager.
pub fn is_bootstrap_archive(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "system.arc"
        || name
            .strip_prefix("system.")
            .and_then(|name| name.strip_suffix(".arc"))
            .is_some_and(|tag| native_archive::ArchiveLanguage::from_tag(tag).is_some())
}

/// Inspect the same declared image bounds as the runtime, including tail data.
pub fn is_bootstrap_image(bytes: &[u8]) -> bool {
    native_script::is_declared_bp_image(bytes)
}
