//! Original SoftPAL GXM scene output, using vita2d's linked GXP programs.
//! The host owns initialization and the context, and releases scene resources
//! before destroying it. Borrowed scene views may not overlap the next VM step.
use core::ffi::{c_char, c_void};
pub use crate::native_vm::ffi;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
    pub left: bool,
    pub right: bool,
    pub skip: bool,
    pub hide: bool,
}

extern "C" {
    /// Call with an initialized vita2d context and a live SoftPAL scene-mode
    /// host, on its owner thread. A false return is an output failure.
    pub fn sena_vita_scene_render(host: *mut c_void, cursor: *const Cursor) -> bool;
    pub fn sena_vita_scene_free();
    pub fn sena_vita_scene_log_memory(stage: *const c_char, frame: u64);
}
