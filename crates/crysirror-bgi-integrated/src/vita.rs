//! Original GXM driver. One CrySirror console output owner manages its lifetime.
//! Initialize before creating render resources; drop them all before shutdown.
pub use crate::native_render::{NativeDraw, NativeGpu, NativeVertex, Renderer};

unsafe extern "C" {
    pub fn ethornell_vita_gpu_init() -> i32;
    pub fn ethornell_vita_gpu_exit();
    fn ethornell_gpu_create(w: u32, h: u32, rgba: *const u8) -> u64;
    fn ethornell_gpu_update(texture: u64, w: u32, h: u32, rgba: *const u8) -> i32;
    fn ethornell_gpu_destroy(texture: u64);
    fn ethornell_gpu_begin(clear: *const f32) -> i32;
    fn ethornell_gpu_draw(draw: *const NativeDraw) -> i32;
    fn ethornell_gpu_end() -> i32;
}

/// # Safety
/// The sole output owner must have successfully initialized the GXM driver,
/// keep it on the same thread, and outlive every renderer using these callbacks.
pub unsafe fn callbacks() -> NativeGpu {
    NativeGpu {
        create: ethornell_gpu_create,
        update: ethornell_gpu_update,
        destroy: ethornell_gpu_destroy,
        begin: ethornell_gpu_begin,
        draw: ethornell_gpu_draw,
        end: ethornell_gpu_end,
    }
}
