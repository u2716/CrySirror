// SPDX-License-Identifier: MPL-2.0
//! The shared CrySirror renderer on PS Vita: the wgpu custom backend
//! (`crysirror-wgpu-deko3d`'s recorder, unchanged) over a GXM `Driver`, with
//! programs resolved from the native PSV GXP catalog. See `gxm` for the
//! driver boundary and `library` for the shader index.
pub mod library;
#[cfg(target_os = "vita")]
pub mod gxm;

/// The console GPU for adopted engines. Call once on the thread that owns the
/// display; drop it before process exit.
#[cfg(target_os = "vita")]
pub fn create() -> Result<crysirror_wgpu_deko3d::Gpu, String> {
    let driver = gxm::Gxm::new()?;
    let library = library::Library::embedded()?;
    Ok(crysirror_wgpu_deko3d::Gpu::new(std::sync::Arc::new(driver), std::sync::Arc::new(library)))
}
