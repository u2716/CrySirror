//! RFVP's original Vita/GXM renderer, retained behind an optional native output.
//! The CrySirror host initializes the driver, drives one session on its owner
//! thread, drops renderer textures, then shuts the driver down. This module
//! does not create a window, input loop, audio device or a second VM.
mod render;
mod texture_policy;
mod thumbnail;
pub use render::PsvRenderer;

unsafe extern "C" {
    pub fn rfvp_psv_gpu_init() -> i32;
    pub fn rfvp_psv_gpu_fini();
}
