// SPDX-License-Identifier: MPL-2.0
//! The GPU driver below the wgpu custom backend. `Deko3d` calls the C glue
//! in platform/switch/runtime/source/csdk.c (all structs and constants mirror
//! csdk.h); `Null` keeps host memory for buffers and discards GPU work, so the
//! backend and the shared renderer can be exercised on a desktop.
#![allow(dead_code)]

pub type Id = i32;

pub mod fmt {
    pub const NONE: u32 = 0;
    pub const R8_UNORM: u32 = 1;
    pub const R8_SNORM: u32 = 2;
    pub const R8_UINT: u32 = 3;
    pub const R8_SINT: u32 = 4;
    pub const R16_FLOAT: u32 = 5;
    pub const R16_UNORM: u32 = 6;
    pub const R16_UINT: u32 = 7;
    pub const R16_SINT: u32 = 8;
    pub const R32_FLOAT: u32 = 9;
    pub const R32_UINT: u32 = 10;
    pub const R32_SINT: u32 = 11;
    pub const RG8_UNORM: u32 = 12;
    pub const RG8_SNORM: u32 = 13;
    pub const RG8_UINT: u32 = 14;
    pub const RG8_SINT: u32 = 15;
    pub const RG16_FLOAT: u32 = 16;
    pub const RG16_UNORM: u32 = 17;
    pub const RG16_UINT: u32 = 18;
    pub const RG16_SINT: u32 = 19;
    pub const RG32_FLOAT: u32 = 20;
    pub const RG32_UINT: u32 = 21;
    pub const RG32_SINT: u32 = 22;
    pub const RGBA8_UNORM: u32 = 23;
    pub const RGBA8_UNORM_SRGB: u32 = 24;
    pub const RGBA8_SNORM: u32 = 25;
    pub const RGBA8_UINT: u32 = 26;
    pub const RGBA8_SINT: u32 = 27;
    pub const BGRA8_UNORM: u32 = 28;
    pub const BGRA8_UNORM_SRGB: u32 = 29;
    pub const RGBA16_FLOAT: u32 = 30;
    pub const RGBA16_UNORM: u32 = 31;
    pub const RGBA16_UINT: u32 = 32;
    pub const RGBA16_SINT: u32 = 33;
    pub const RGBA32_FLOAT: u32 = 34;
    pub const RGBA32_UINT: u32 = 35;
    pub const RGBA32_SINT: u32 = 36;
    pub const RGB10A2_UNORM: u32 = 37;
    pub const RG11B10_FLOAT: u32 = 38;
    pub const DEPTH16: u32 = 39;
    pub const DEPTH24: u32 = 40;
    pub const DEPTH24_STENCIL8: u32 = 41;
    pub const DEPTH32F: u32 = 42;
    pub const DEPTH32F_STENCIL8: u32 = 43;
    pub const STENCIL8: u32 = 44;
    pub const BC1: u32 = 45;
    pub const BC1_SRGB: u32 = 46;
    pub const BC2: u32 = 47;
    pub const BC2_SRGB: u32 = 48;
    pub const BC3: u32 = 49;
    pub const BC3_SRGB: u32 = 50;
    pub const BC4_UNORM: u32 = 51;
    pub const BC4_SNORM: u32 = 52;
    pub const BC5_UNORM: u32 = 53;
    pub const BC5_SNORM: u32 = 54;
    pub const BC6H_UFLOAT: u32 = 55;
    pub const BC6H_SFLOAT: u32 = 56;
    pub const BC7: u32 = 57;
    pub const BC7_SRGB: u32 = 58;
}
pub mod ty {
    pub const D2: u32 = 0;
    pub const D2_ARRAY: u32 = 1;
    pub const CUBE: u32 = 2;
    pub const CUBE_ARRAY: u32 = 3;
    pub const D3: u32 = 4;
    pub const D2_MS: u32 = 5;
    pub const D2_MS_ARRAY: u32 = 6;
    pub const D1: u32 = 7;
}
pub const IMAGE_RENDER: u32 = 1;
pub const IMAGE_STORAGE: u32 = 2;
/// Source or destination of image-to-image copies (deko3d 2D engine).
pub const IMAGE_COPY: u32 = 8;
pub const MEM_UPLOAD: u32 = 0;
pub const MEM_READBACK: u32 = 1;
pub const STAGE_VERTEX: u32 = 0;
pub const STAGE_FRAGMENT: u32 = 1;
pub const STAGE_COMPUTE: u32 = 2;
pub mod vt {
    pub const FLOAT: u32 = 0;
    pub const UINT: u32 = 1;
    pub const SINT: u32 = 2;
    pub const UNORM: u32 = 3;
    pub const SNORM: u32 = 4;
    pub const HALF: u32 = 5;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    pub image: Id,
    pub ty: u32,
    pub format: u32,
    pub mip: u32,
    pub mips: u32,
    pub layer: u32,
    pub layers: u32,
    pub stencil: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Blend {
    pub enable: u8,
    pub color_op: u8,
    pub color_src: u8,
    pub color_dst: u8,
    pub alpha_op: u8,
    pub alpha_src: u8,
    pub alpha_dst: u8,
    pub write_mask: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Attrib {
    pub location: u32,
    pub buffer: u32,
    pub offset: u32,
    pub ty: u32,
    pub bits: u32,
    pub components: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Pipeline {
    pub vs: Id,
    pub fs: Id,
    pub cull: u32,
    pub front_face: u32,
    pub unclipped_depth: u32,
    pub color_count: u32,
    pub blends: [Blend; 8],
    pub depth_test: u8,
    pub depth_write: u8,
    pub depth_compare: u8,
    pub stencil_enable: u8,
    pub stencil_front: [u8; 4],
    pub stencil_back: [u8; 4],
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub alpha_to_coverage: u8,
    pub pad: u8,
    pub depth_bias: i32,
    pub depth_bias_slope: f32,
    pub depth_bias_clamp: f32,
    pub samples: u32,
    pub attrib_count: u32,
    pub attribs: [Attrib; 16],
    pub buffer_count: u32,
    pub strides: [u32; 16],
    pub instanced: [u32; 16],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sampler {
    pub min_filter: u32,
    pub mag_filter: u32,
    pub mip_filter: u32,
    pub wrap: [u32; 3],
    pub lod_min: f32,
    pub lod_max: f32,
    pub compare_enable: u32,
    pub compare: u32,
    pub anisotropy: f32,
    pub border: [f32; 4],
}

/// Everything the backend needs from a GPU. Calls between `begin` and
/// `submit` record into one command list.
pub trait Driver: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn mem_create(&self, size: u64, kind: u32) -> Id;
    fn mem_destroy(&self, id: Id);
    fn mem_cpu(&self, id: Id) -> *mut u8;
    fn mem_gpu(&self, id: Id) -> u64;
    fn mem_invalidate(&self, id: Id, offset: u64, size: u64);
    #[allow(clippy::too_many_arguments)]
    fn image_create(&self, format: u32, ty: u32, w: u32, h: u32, d: u32, mips: u32, samples: u32, flags: u32) -> Id;
    /// Whether `image_create` accepts `format` with `IMAGE_RENDER`. The
    /// backend reports it through the adapter's format features so the
    /// renderer picks a supported target up front instead of failing.
    fn renderable(&self, _format: u32) -> bool {
        true
    }
    /// Whether blending into a `format` render target is available.
    fn blendable(&self, format: u32) -> bool {
        self.renderable(format)
    }
    /// Whether compute entries may write `format` storage images. A driver
    /// without general compute reports only the formats its lowered compute
    /// entries write.
    fn storage_format(&self, _format: u32) -> bool {
        true
    }
    fn image_destroy(&self, id: Id);
    fn image_descriptor(&self, slot: u32, view: &View, load_store: bool) -> bool;
    fn sampler_descriptor(&self, slot: u32, sampler: &Sampler) -> bool;
    fn shader(&self, code: &[u8]) -> Id;
    fn begin(&self);
    fn submit(&self) -> u64;
    fn done(&self, seq: u64) -> bool;
    fn wait(&self, seq: u64);
    fn barrier(&self);
    fn targets(&self, colors: &[View], depth: Option<&View>);
    fn clear_color(&self, index: u32, rgba: [f32; 4]);
    fn clear_depth_stencil(&self, depth: bool, value: f32, stencil: bool, stencil_value: u8);
    fn resolve(&self, src: &View, dst: &View);
    fn viewport(&self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32);
    fn scissor(&self, x: u32, y: u32, w: u32, h: u32);
    fn blend_constant(&self, rgba: [f32; 4]);
    fn stencil_reference(&self, reference: u8, read_mask: u8, write_mask: u8);
    fn bind_graphics(&self, pipeline: &Pipeline);
    fn bind_compute(&self, cs: Id);
    fn vertex_buffer(&self, slot: u32, gpu: u64, size: u32);
    fn index_buffer(&self, gpu: u64, u32_indices: bool);
    fn uniform(&self, stage: u32, slot: u32, gpu: u64, size: u32);
    fn storage(&self, stage: u32, slot: u32, gpu: u64, size: u32);
    fn texture(&self, stage: u32, slot: u32, image_slot: u32, sampler_slot: u32);
    fn image(&self, stage: u32, slot: u32, image_slot: u32);
    fn draw(&self, prim: u32, count: u32, instances: u32, first: u32, first_instance: u32);
    fn draw_indexed(&self, prim: u32, count: u32, instances: u32, first: u32, base_vertex: i32, first_instance: u32);
    fn draw_indirect(&self, prim: u32, gpu: u64, indexed: bool);
    fn dispatch(&self, x: u32, y: u32, z: u32);
    fn dispatch_indirect(&self, gpu: u64);
    fn copy_buffer(&self, src: u64, dst: u64, size: u64);
    #[allow(clippy::too_many_arguments)]
    fn copy_buffer_to_image(&self, src: u64, row_pixels: u32, image_rows: u32, dst: &View, origin: [u32; 3], extent: [u32; 3]);
    #[allow(clippy::too_many_arguments)]
    fn copy_image_to_buffer(&self, src: &View, origin: [u32; 3], extent: [u32; 3], dst: u64, row_pixels: u32, image_rows: u32);
    fn copy_image(&self, src: &View, so: [u32; 3], dst: &View, d: [u32; 3], extent: [u32; 3]);
    fn display_size(&self) -> [u32; 2];
    fn display_image(&self, index: u32) -> Id;
    fn acquire(&self) -> Option<u32>;
    fn present(&self, index: u32);
}

/// Host-memory driver: buffers are real bytes, GPU work is discarded and every
/// submission completes at once. Used to capture shader sources on desktop.
#[derive(Default)]
pub struct Null {
    mems: std::sync::Mutex<std::collections::HashMap<Id, Box<[u8]>>>,
    next: std::sync::atomic::AtomicI32,
    seq: std::sync::atomic::AtomicU64,
}
impl Null {
    fn id(&self) -> Id {
        self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
}
impl Driver for Null {
    fn name(&self) -> &str {
        "null"
    }
    fn mem_create(&self, size: u64, _kind: u32) -> Id {
        let id = self.id();
        let mut mems = self.mems.lock().unwrap();
        mems.insert(id, vec![0u8; size.max(1) as usize].into_boxed_slice());
        if std::env::var_os("NULL_MEM_TRACE").is_some() && id % 500 == 0 {
            eprintln!("null mems: {} live, {} bytes", mems.len(), mems.values().map(|m| m.len()).sum::<usize>());
        }
        id
    }
    fn mem_destroy(&self, id: Id) {
        self.mems.lock().unwrap().remove(&id);
    }
    fn mem_cpu(&self, id: Id) -> *mut u8 {
        self.mems.lock().unwrap().get_mut(&id).map_or(std::ptr::null_mut(), |m| m.as_mut_ptr())
    }
    fn mem_gpu(&self, id: Id) -> u64 {
        (id as u64) << 32
    }
    fn mem_invalidate(&self, _: Id, _: u64, _: u64) {}
    fn image_create(&self, _: u32, _: u32, _: u32, _: u32, _: u32, _: u32, _: u32, _: u32) -> Id {
        self.id()
    }
    fn image_destroy(&self, _: Id) {}
    fn image_descriptor(&self, _: u32, _: &View, _: bool) -> bool {
        true
    }
    fn sampler_descriptor(&self, _: u32, _: &Sampler) -> bool {
        true
    }
    fn shader(&self, _: &[u8]) -> Id {
        self.id()
    }
    fn begin(&self) {}
    fn submit(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
    fn done(&self, _: u64) -> bool {
        true
    }
    fn wait(&self, _: u64) {}
    fn barrier(&self) {}
    fn targets(&self, _: &[View], _: Option<&View>) {}
    fn clear_color(&self, _: u32, _: [f32; 4]) {}
    fn clear_depth_stencil(&self, _: bool, _: f32, _: bool, _: u8) {}
    fn resolve(&self, _: &View, _: &View) {}
    fn viewport(&self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
    fn scissor(&self, _: u32, _: u32, _: u32, _: u32) {}
    fn blend_constant(&self, _: [f32; 4]) {}
    fn stencil_reference(&self, _: u8, _: u8, _: u8) {}
    fn bind_graphics(&self, _: &Pipeline) {}
    fn bind_compute(&self, _: Id) {}
    fn vertex_buffer(&self, _: u32, _: u64, _: u32) {}
    fn index_buffer(&self, _: u64, _: bool) {}
    fn uniform(&self, _: u32, _: u32, _: u64, _: u32) {}
    fn storage(&self, _: u32, _: u32, _: u64, _: u32) {}
    fn texture(&self, _: u32, _: u32, _: u32, _: u32) {}
    fn image(&self, _: u32, _: u32, _: u32) {}
    fn draw(&self, _: u32, _: u32, _: u32, _: u32, _: u32) {}
    fn draw_indexed(&self, _: u32, _: u32, _: u32, _: u32, _: i32, _: u32) {}
    fn draw_indirect(&self, _: u32, _: u64, _: bool) {}
    fn dispatch(&self, _: u32, _: u32, _: u32) {}
    fn dispatch_indirect(&self, _: u64) {}
    fn copy_buffer(&self, src: u64, dst: u64, size: u64) {
        // Host copy keeps readbacks meaningful for buffer-only work.
        let mut mems = self.mems.lock().unwrap();
        let (si, so) = ((src >> 32) as Id, (src & 0xffff_ffff) as usize);
        let (di, d_o) = ((dst >> 32) as Id, (dst & 0xffff_ffff) as usize);
        let Some(bytes) = mems.get(&si).map(|m| m[so..so + size as usize].to_vec()) else { return };
        if let Some(d) = mems.get_mut(&di) {
            d[d_o..d_o + size as usize].copy_from_slice(&bytes);
        }
    }
    fn copy_buffer_to_image(&self, _: u64, _: u32, _: u32, _: &View, _: [u32; 3], _: [u32; 3]) {}
    fn copy_image_to_buffer(&self, _: &View, _: [u32; 3], _: [u32; 3], _: u64, _: u32, _: u32) {}
    fn copy_image(&self, _: &View, _: [u32; 3], _: &View, _: [u32; 3], _: [u32; 3]) {}
    fn display_size(&self) -> [u32; 2] {
        [1280, 720]
    }
    fn display_image(&self, _: u32) -> Id {
        self.id()
    }
    fn acquire(&self) -> Option<u32> {
        Some(0)
    }
    fn present(&self, _: u32) {}
}

#[cfg(target_os = "horizon")]
pub use horizon::Deko3d;

#[cfg(target_os = "horizon")]
mod horizon {
    use super::*;
    unsafe extern "C" {
        fn csdk_init() -> bool;
        fn csdk_display_width() -> u32;
        fn csdk_display_height() -> u32;
        fn csdk_display_image(index: u32) -> i32;
        fn csdk_acquire() -> i32;
        fn csdk_present(index: i32);
        fn csdk_mem_create(size: u64, kind: u32) -> i32;
        fn csdk_mem_destroy(id: i32);
        fn csdk_mem_cpu(id: i32) -> *mut u8;
        fn csdk_mem_gpu(id: i32) -> u64;
        fn csdk_mem_invalidate(id: i32, offset: u64, size: u64);
        fn csdk_image_create(f: u32, t: u32, w: u32, h: u32, d: u32, m: u32, s: u32, flags: u32) -> i32;
        fn csdk_image_destroy(id: i32);
        fn csdk_image_descriptor(slot: u32, view: *const View, load_store: bool) -> bool;
        fn csdk_sampler_descriptor(slot: u32, sampler: *const Sampler) -> bool;
        fn csdk_shader(code: *const u8, size: u32) -> i32;
        fn csdk_begin();
        fn csdk_submit() -> u64;
        fn csdk_done(seq: u64) -> bool;
        fn csdk_wait(seq: u64);
        fn csdk_barrier();
        fn csdk_targets(count: u32, colors: *const View, depth: *const View);
        fn csdk_clear_color(index: u32, rgba: *const f32);
        fn csdk_clear_depth_stencil(depth: bool, value: f32, stencil: bool, sv: u8);
        fn csdk_resolve(src: *const View, dst: *const View);
        fn csdk_viewport(x: f32, y: f32, w: f32, h: f32, a: f32, b: f32);
        fn csdk_scissor(x: u32, y: u32, w: u32, h: u32);
        fn csdk_blend_constant(rgba: *const f32);
        fn csdk_stencil_reference(r: u8, rm: u8, wm: u8);
        fn csdk_bind_graphics(p: *const Pipeline);
        fn csdk_bind_compute(cs: i32);
        fn csdk_vertex_buffer(slot: u32, gpu: u64, size: u32);
        fn csdk_index_buffer(gpu: u64, format: u32);
        fn csdk_uniform(stage: u32, slot: u32, gpu: u64, size: u32);
        fn csdk_storage(stage: u32, slot: u32, gpu: u64, size: u32);
        fn csdk_texture(stage: u32, slot: u32, image_slot: u32, sampler_slot: u32);
        fn csdk_image(stage: u32, slot: u32, image_slot: u32);
        fn csdk_draw(prim: u32, count: u32, inst: u32, first: u32, first_inst: u32);
        fn csdk_draw_indexed(prim: u32, count: u32, inst: u32, first: u32, base: i32, first_inst: u32);
        fn csdk_draw_indirect(prim: u32, gpu: u64, indexed: bool);
        fn csdk_dispatch(x: u32, y: u32, z: u32);
        fn csdk_dispatch_indirect(gpu: u64);
        fn csdk_copy_buffer(src: u64, dst: u64, size: u64);
        fn csdk_copy_buffer_to_image(src: u64, rp: u32, ir: u32, dst: *const View, x: u32, y: u32, z: u32, w: u32, h: u32, d: u32);
        fn csdk_copy_image_to_buffer(src: *const View, x: u32, y: u32, z: u32, w: u32, h: u32, d: u32, dst: u64, rp: u32, ir: u32);
        fn csdk_copy_image(src: *const View, sx: u32, sy: u32, sz: u32, dst: *const View, dx: u32, dy: u32, dz: u32, w: u32, h: u32, d: u32);
    }

    /// The deko3d driver. Create once after the shell started gpu.c.
    pub struct Deko3d(());
    impl Deko3d {
        pub fn new() -> Option<Self> {
            unsafe { csdk_init() }.then_some(Self(()))
        }
    }
    impl Driver for Deko3d {
        fn name(&self) -> &str {
            "deko3d"
        }
        fn mem_create(&self, size: u64, kind: u32) -> Id {
            unsafe { csdk_mem_create(size, kind) }
        }
        fn mem_destroy(&self, id: Id) {
            unsafe { csdk_mem_destroy(id) }
        }
        fn mem_cpu(&self, id: Id) -> *mut u8 {
            unsafe { csdk_mem_cpu(id) }
        }
        fn mem_gpu(&self, id: Id) -> u64 {
            unsafe { csdk_mem_gpu(id) }
        }
        fn mem_invalidate(&self, id: Id, offset: u64, size: u64) {
            unsafe { csdk_mem_invalidate(id, offset, size) }
        }
        fn image_create(&self, f: u32, t: u32, w: u32, h: u32, d: u32, m: u32, s: u32, flags: u32) -> Id {
            unsafe { csdk_image_create(f, t, w, h, d, m, s, flags) }
        }
        fn image_destroy(&self, id: Id) {
            unsafe { csdk_image_destroy(id) }
        }
        fn image_descriptor(&self, slot: u32, view: &View, load_store: bool) -> bool {
            unsafe { csdk_image_descriptor(slot, view, load_store) }
        }
        fn sampler_descriptor(&self, slot: u32, sampler: &Sampler) -> bool {
            unsafe { csdk_sampler_descriptor(slot, sampler) }
        }
        fn shader(&self, code: &[u8]) -> Id {
            unsafe { csdk_shader(code.as_ptr(), code.len() as u32) }
        }
        fn begin(&self) {
            unsafe { csdk_begin() }
        }
        fn submit(&self) -> u64 {
            unsafe { csdk_submit() }
        }
        fn done(&self, seq: u64) -> bool {
            unsafe { csdk_done(seq) }
        }
        fn wait(&self, seq: u64) {
            unsafe { csdk_wait(seq) }
        }
        fn barrier(&self) {
            unsafe { csdk_barrier() }
        }
        fn targets(&self, colors: &[View], depth: Option<&View>) {
            unsafe { csdk_targets(colors.len() as u32, colors.as_ptr(), depth.map_or(std::ptr::null(), |d| d)) }
        }
        fn clear_color(&self, index: u32, rgba: [f32; 4]) {
            unsafe { csdk_clear_color(index, rgba.as_ptr()) }
        }
        fn clear_depth_stencil(&self, depth: bool, value: f32, stencil: bool, sv: u8) {
            unsafe { csdk_clear_depth_stencil(depth, value, stencil, sv) }
        }
        fn resolve(&self, src: &View, dst: &View) {
            unsafe { csdk_resolve(src, dst) }
        }
        fn viewport(&self, x: f32, y: f32, w: f32, h: f32, a: f32, b: f32) {
            unsafe { csdk_viewport(x, y, w, h, a, b) }
        }
        fn scissor(&self, x: u32, y: u32, w: u32, h: u32) {
            unsafe { csdk_scissor(x, y, w, h) }
        }
        fn blend_constant(&self, rgba: [f32; 4]) {
            unsafe { csdk_blend_constant(rgba.as_ptr()) }
        }
        fn stencil_reference(&self, r: u8, rm: u8, wm: u8) {
            unsafe { csdk_stencil_reference(r, rm, wm) }
        }
        fn bind_graphics(&self, p: &Pipeline) {
            unsafe { csdk_bind_graphics(p) }
        }
        fn bind_compute(&self, cs: Id) {
            unsafe { csdk_bind_compute(cs) }
        }
        fn vertex_buffer(&self, slot: u32, gpu: u64, size: u32) {
            unsafe { csdk_vertex_buffer(slot, gpu, size) }
        }
        fn index_buffer(&self, gpu: u64, u32_indices: bool) {
            unsafe { csdk_index_buffer(gpu, u32::from(u32_indices)) }
        }
        fn uniform(&self, stage: u32, slot: u32, gpu: u64, size: u32) {
            unsafe { csdk_uniform(stage, slot, gpu, size) }
        }
        fn storage(&self, stage: u32, slot: u32, gpu: u64, size: u32) {
            unsafe { csdk_storage(stage, slot, gpu, size) }
        }
        fn texture(&self, stage: u32, slot: u32, i: u32, s: u32) {
            unsafe { csdk_texture(stage, slot, i, s) }
        }
        fn image(&self, stage: u32, slot: u32, i: u32) {
            unsafe { csdk_image(stage, slot, i) }
        }
        fn draw(&self, p: u32, c: u32, i: u32, f: u32, fi: u32) {
            unsafe { csdk_draw(p, c, i, f, fi) }
        }
        fn draw_indexed(&self, p: u32, c: u32, i: u32, f: u32, b: i32, fi: u32) {
            unsafe { csdk_draw_indexed(p, c, i, f, b, fi) }
        }
        fn draw_indirect(&self, p: u32, gpu: u64, indexed: bool) {
            unsafe { csdk_draw_indirect(p, gpu, indexed) }
        }
        fn dispatch(&self, x: u32, y: u32, z: u32) {
            unsafe { csdk_dispatch(x, y, z) }
        }
        fn dispatch_indirect(&self, gpu: u64) {
            unsafe { csdk_dispatch_indirect(gpu) }
        }
        fn copy_buffer(&self, s: u64, d: u64, n: u64) {
            unsafe { csdk_copy_buffer(s, d, n) }
        }
        fn copy_buffer_to_image(&self, src: u64, rp: u32, ir: u32, dst: &View, o: [u32; 3], e: [u32; 3]) {
            unsafe { csdk_copy_buffer_to_image(src, rp, ir, dst, o[0], o[1], o[2], e[0], e[1], e[2]) }
        }
        fn copy_image_to_buffer(&self, src: &View, o: [u32; 3], e: [u32; 3], dst: u64, rp: u32, ir: u32) {
            unsafe { csdk_copy_image_to_buffer(src, o[0], o[1], o[2], e[0], e[1], e[2], dst, rp, ir) }
        }
        fn copy_image(&self, src: &View, so: [u32; 3], dst: &View, d: [u32; 3], e: [u32; 3]) {
            unsafe { csdk_copy_image(src, so[0], so[1], so[2], dst, d[0], d[1], d[2], e[0], e[1], e[2]) }
        }
        fn display_size(&self) -> [u32; 2] {
            unsafe { [csdk_display_width(), csdk_display_height()] }
        }
        fn display_image(&self, index: u32) -> Id {
            unsafe { csdk_display_image(index) }
        }
        fn acquire(&self) -> Option<u32> {
            let i = unsafe { csdk_acquire() };
            (i >= 0).then_some(i as u32)
        }
        fn present(&self, index: u32) {
            unsafe { csdk_present(index as i32) }
        }
    }
}
