// SPDX-License-Identifier: MPL-2.0
//! The wgpu custom backend's `Driver` over GXM (PS Vita).
//!
//! Every pass of the shared renderer is a GXM scene on vita2d's context and
//! shader patcher (the same ownership the integrated Siglus/BGI Vita renderers
//! use). Composition, blending, filters and transitions are GPU draws of the
//! native GXP catalog; nothing is composed on the CPU. GXM has no transfer
//! queue in this path, so buffer/texture uploads and readbacks are CPU copies
//! into GPU-mapped memory after the GPU is idle.
//!
//! Scenes are synchronous: `submit` waits for the GPU. Per-draw program
//! variants are selected exactly as `crysirror_shaders::native` specifies:
//! transitions by the bound wipe kind, quad filters per primitive run by the
//! bound LayerFilter record (`d.z` distortion, `c.w` flags). The one compute
//! entry the 2D renderer uses (the separable box filter) runs as its native
//! fragment lowering. Unsupported features (other compute, constant blend
//! factors, blending into float targets, mipmaps/array layers) are refused
//! visibly and logged once.
use crysirror_wgpu_deko3d::driver::{fmt, vt, Driver, Id, Pipeline, Sampler, View, STAGE_COMPUTE, STAGE_FRAGMENT, STAGE_VERTEX};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::sync::Mutex;
use vitasdk_sys::*;

#[repr(C)]
struct Vita2dTexture {
    gxm_tex: SceGxmTexture,
    _private: [u8; 0],
}

#[link(name = "vita2d", kind = "static")]
unsafe extern "C" {
    fn vita2d_init_advanced(temp_pool_size: u32) -> i32;
    fn vita2d_fini() -> i32;
    fn vita2d_get_context() -> *mut SceGxmContext;
    fn vita2d_get_shader_patcher() -> *mut SceGxmShaderPatcher;
    fn vita2d_start_drawing_advanced(target: *mut Vita2dTexture, flags: u32);
    fn vita2d_end_drawing();
    fn vita2d_wait_rendering_done();
    fn vita2d_swap_buffers();
    fn vita2d_set_vblank_wait(enable: i32);
    fn vita2d_pool_reset();
    fn vita2d_set_clear_color(color: u32);
    fn vita2d_clear_screen();
    fn vita2d_draw_texture(texture: *const Vita2dTexture, x: f32, y: f32);
    fn vita2d_texture_set_alloc_memblock_type(kind: u32);
    fn vita2d_create_empty_texture_format(w: u32, h: u32, format: SceGxmTextureFormat) -> *mut Vita2dTexture;
    fn vita2d_free_texture(texture: *mut Vita2dTexture);
    fn vita2d_texture_get_stride(texture: *const Vita2dTexture) -> u32;
    fn vita2d_texture_get_datap(texture: *const Vita2dTexture) -> *mut c_void;
}

const DISPLAY: [u32; 2] = [960, 544];
/// Frames a retired texture/buffer is kept: Vita3K reads resources on its own
/// thread and can still reach them after the GPU wait returned.
const RETIRE_FRAMES: u64 = 4;
const SEQUENTIAL_INDICES: usize = 65536;
/// vita2d.h layout: SceGxmTexture (16 bytes), data_UID, palette_UID, gxm_rtgt,
/// then the render target's color surface.
const VITA2D_COLOR_SURFACE: usize = 28;
const VITA2D_RENDER_TARGET: usize = 24;
const VITA2D_DEPTH_SURFACE: usize = VITA2D_COLOR_SURFACE + std::mem::size_of::<SceGxmColorSurface>();
const VITA2D_DEPTH_UID: usize = VITA2D_DEPTH_SURFACE + std::mem::size_of::<SceGxmDepthStencilSurface>();
const _: () = assert!(VITA2D_DEPTH_UID == 96, "vita2d_texture layout");

static LOG: std::sync::OnceLock<fn(&str)> = std::sync::OnceLock::new();

/// Send driver diagnostics to the host log instead of stderr.
pub fn set_log(sink: fn(&str)) {
    let _ = LOG.set(sink);
}

fn log(message: &str) {
    match LOG.get() {
        Some(sink) => sink(message),
        None => eprintln!("{message}"),
    }
}

static DUMP_IMAGES: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// Diagnostics: at the next present, write every resident colour image as
/// raw rows (`<id>-<w>x<h>-f<format>-s<stride>.raw`) into `dir`.
static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Log every scene, draw outcome and CPU readback (diagnostics/self-test).
pub fn trace(on: bool) {
    TRACE.store(on, std::sync::atomic::Ordering::Relaxed);
}
fn tracing() -> bool {
    TRACE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn dump_images(dir: std::path::PathBuf) {
    if let Ok(mut d) = DUMP_IMAGES.lock() {
        *d = Some(dir);
    }
}

/// Free user/CDRAM/phycont memory, for allocation-failure reports.
fn memory_report() -> String {
    let mut info: SceKernelFreeMemorySizeInfo = unsafe { std::mem::zeroed() };
    info.size = std::mem::size_of::<SceKernelFreeMemorySizeInfo>() as i32;
    let rc = unsafe { sceKernelGetFreeMemorySize(&mut info) };
    format!("free user={} cdram={} phycont={} (rc {rc:#x})", info.size_user, info.size_cdram, info.size_phycont)
}

fn log_once(message: &str) {
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    if let Ok(mut seen) = SEEN.lock() {
        if seen.iter().any(|m| m == message) || seen.len() > 128 {
            return;
        }
        seen.push(message.to_owned());
    }
    log(&format!("gxm: {message}"));
}

struct Block {
    uid: SceUID,
    base: *mut u8,
    size: usize,
}
impl Block {
    fn new(size: usize) -> Option<Block> {
        let size = size.max(1).div_ceil(4096) * 4096;
        unsafe {
            let uid = sceKernelAllocMemBlock(c"crysirror-gxm".as_ptr(), SCE_KERNEL_MEMBLOCK_TYPE_USER_RW_UNCACHE, size as SceSize, std::ptr::null_mut());
            if uid < 0 {
                return None;
            }
            let mut base = std::ptr::null_mut();
            if sceKernelGetMemBlockBase(uid, &mut base) < 0
                || sceGxmMapMemory(base, size as SceSize, SCE_GXM_MEMORY_ATTRIB_READ | SCE_GXM_MEMORY_ATTRIB_WRITE) < 0
            {
                sceKernelFreeMemBlock(uid);
                return None;
            }
            Some(Block { uid, base: base.cast(), size })
        }
    }
}
impl Drop for Block {
    fn drop(&mut self) {
        unsafe {
            sceGxmUnmapMemory(self.base.cast());
            sceKernelFreeMemBlock(self.uid);
        }
    }
}

/// Buffers up to `SMALL_MAX` bytes are carved from shared mapped arenas: a
/// kernel memblock is at least 4 KiB and the process may hold only a limited
/// number of them, while the renderer creates many small uniform/vertex
/// buffers. Freed chunks return to their power-of-two class after the retire
/// delay; arenas live as long as the driver.
const SMALL_MIN: usize = 256;
const SMALL_MAX: usize = 64 * 1024;
const ARENA: usize = 1024 * 1024;

enum Mem {
    Own(Block),
    Small { base: *mut u8, class: usize },
}
impl Mem {
    fn base(&self) -> *mut u8 {
        match self {
            Mem::Own(b) => b.base,
            Mem::Small { base, .. } => *base,
        }
    }
}

#[derive(Default)]
struct SmallPool {
    arenas: Vec<Block>,
    /// Bump offset inside the newest arena.
    used: usize,
    free: [Vec<*mut u8>; 9],
}
impl SmallPool {
    fn class(size: usize) -> usize {
        size.max(SMALL_MIN).next_power_of_two().trailing_zeros() as usize - SMALL_MIN.trailing_zeros() as usize
    }
    fn alloc(&mut self, size: usize) -> Option<Mem> {
        let class = Self::class(size);
        if let Some(base) = self.free[class].pop() {
            return Some(Mem::Small { base, class });
        }
        let bytes = SMALL_MIN << class;
        if self.arenas.is_empty() || self.used + bytes > ARENA {
            self.arenas.push(Block::new(ARENA)?);
            self.used = 0;
        }
        let base = unsafe { self.arenas.last().unwrap().base.add(self.used) };
        self.used += bytes;
        Some(Mem::Small { base, class })
    }
    fn release(&mut self, mem: Mem) {
        if let Mem::Small { base, class } = mem {
            self.free[class].push(base);
        }
    }
}

struct Image {
    tex: *mut Vita2dTexture,
    /// RGBA32F images exist only as the horizontal sums of the box filter's
    /// fragment lowering: two U8x4 surfaces holding the RG (`tex`) and BA
    /// (`pair`) 16-bit sums as high/low bytes. Null for every other format.
    pair: *mut Vita2dTexture,
    size: [u32; 2],
    format: u32,
    bpp: usize,
    stride: usize,
    data: *mut u8,
    render: bool,
    /// Depth/stencil images live inside their color target's vita2d surface.
    depth: bool,
    /// A colour target's depth/stencil surface, allocated by the first pass
    /// that binds depth (most 2D canvases never do).
    has_depth: bool,
}

fn texture_format(format: u32) -> Option<(SceGxmTextureFormat, usize)> {
    Some(match format {
        fmt::RGBA8_UNORM | fmt::RGBA8_UNORM_SRGB => (SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR, 4),
        fmt::BGRA8_UNORM | fmt::BGRA8_UNORM_SRGB => (SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ARGB, 4),
        fmt::R8_UNORM => (SCE_GXM_TEXTURE_FORMAT_U8_1RRR, 1),
        fmt::RG8_UNORM => (SCE_GXM_TEXTURE_FORMAT_U8U8_00GR, 2),
        fmt::RGBA16_FLOAT => (SCE_GXM_TEXTURE_FORMAT_F16F16F16F16_ABGR, 8),
        fmt::R16_FLOAT => (SCE_GXM_TEXTURE_FORMAT_F16_1RRR, 2),
        fmt::RG16_FLOAT => (SCE_GXM_TEXTURE_FORMAT_F16F16_00GR, 4),
        fmt::R32_FLOAT => (SCE_GXM_TEXTURE_FORMAT_F32_1RRR, 4),
        fmt::RG32_FLOAT => (SCE_GXM_TEXTURE_FORMAT_F32F32_GR, 8),
        // Per half; see `Image::pair`.
        fmt::RGBA32_FLOAT => (SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR, 4),
        _ => return None,
    })
}
/// Colour surface format, fragment output register format and size of a
/// render target format.
fn color_target(format: u32) -> Option<(SceGxmColorFormat, SceGxmOutputRegisterFormat, SceGxmOutputRegisterSize)> {
    Some(match format {
        fmt::RGBA8_UNORM | fmt::RGBA8_UNORM_SRGB | fmt::RGBA32_FLOAT => (SCE_GXM_COLOR_FORMAT_A8B8G8R8, SCE_GXM_OUTPUT_REGISTER_FORMAT_UCHAR4, SCE_GXM_OUTPUT_REGISTER_SIZE_32BIT),
        fmt::RGBA16_FLOAT => (SCE_GXM_COLOR_FORMAT_F16F16F16F16_ABGR, SCE_GXM_OUTPUT_REGISTER_FORMAT_HALF4, SCE_GXM_OUTPUT_REGISTER_SIZE_64BIT),
        fmt::RG32_FLOAT => (SCE_GXM_COLOR_FORMAT_F32F32_GR, SCE_GXM_OUTPUT_REGISTER_FORMAT_FLOAT2, SCE_GXM_OUTPUT_REGISTER_SIZE_64BIT),
        fmt::R32_FLOAT => (SCE_GXM_COLOR_FORMAT_F32_R, SCE_GXM_OUTPUT_REGISTER_FORMAT_FLOAT, SCE_GXM_OUTPUT_REGISTER_SIZE_32BIT),
        _ => return None,
    })
}
fn f16_bits(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    if v.is_nan() {
        return sign | 0x7e00;
    }
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    if exp >= 31 {
        return sign | 0x7c00;
    }
    if exp <= 0 {
        // Subnormal: units of 2^-24.
        return sign | (v.abs() * 16_777_216.0).round() as u16;
    }
    // A mantissa carry correctly increments the exponent.
    sign | (((exp as u32) << 10) + (((bits & 0x7f_ffff) + 0x1000) >> 13)).min(0x7c00) as u16
}
fn is_depth(format: u32) -> bool {
    matches!(format, fmt::DEPTH16 | fmt::DEPTH24 | fmt::DEPTH24_STENCIL8 | fmt::DEPTH32F | fmt::DEPTH32F_STENCIL8 | fmt::STENCIL8)
}
fn is_srgb(format: u32) -> bool {
    matches!(format, fmt::RGBA8_UNORM_SRGB | fmt::BGRA8_UNORM_SRGB)
}

#[derive(Clone)]
struct Member {
    name: String,
    offset: usize,
    bytes: usize,
}
#[derive(Clone)]
struct BlockMeta {
    name: String,
    storage: bool,
    members: Vec<Member>,
    /// One record of a lowered storage array.
    record: usize,
}
struct Variant {
    program: &'static [u8],
    name: String,
    wipe: Option<i64>,
    distortion: Option<u32>,
    sampling: Option<u32>,
    /// (backend texture slot, GXM unit, dimensions uniform)
    textures: Vec<(u32, u32, String)>,
    blocks: Vec<usize>,
    registered: Option<(SceGxmShaderPatcherId, *const SceGxmProgram)>,
}
struct Shader {
    name: String,
    unavailable: bool,
    /// A compute entry with a native fragment lowering (`box_filter`).
    lowering: Option<String>,
    /// Hand-written native programs name their vertex inputs instead of
    /// using ATTRn/TEXCOORDn semantics: (location, parameter name).
    attributes: Vec<(u32, String)>,
    blocks: Vec<BlockMeta>,
    variants: Vec<Variant>,
}

fn parse_shader(code: &[u8]) -> Result<Shader, String> {
    if let Some(rest) = code.strip_prefix(b"unavailable ") {
        return Ok(Shader { name: String::from_utf8_lossy(rest).into_owned(), unavailable: true, lowering: None, attributes: Vec::new(), blocks: Vec::new(), variants: Vec::new() });
    }
    let v: Value = serde_json::from_slice(code).map_err(|e| e.to_string())?;
    let name = v["name"].as_str().unwrap_or("?").to_string();
    let gxm = &v["gxm"];
    let mut blocks = Vec::new();
    for b in gxm["blocks"].as_array().cloned().unwrap_or_default() {
        let members: Vec<Member> = b["members"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|m| Member {
                name: m["name"].as_str().unwrap_or("").to_string(),
                offset: m["offset"].as_u64().unwrap_or(0) as usize,
                bytes: 4 * m["components"].as_u64().unwrap_or(4) as usize,
            })
            .collect();
        let record = members.iter().map(|m| m.offset + 16).max().unwrap_or(16).div_ceil(16) * 16;
        blocks.push(BlockMeta { name: b["name"].as_str().unwrap_or("").to_string(), storage: b["storage"] == true, members, record });
    }
    let mut variants = Vec::new();
    for x in gxm["variants"].as_array().cloned().unwrap_or_default() {
        let program_name = x["program"].as_str().unwrap_or("");
        let program = crysirror_shaders::native::psv_program(program_name).ok_or_else(|| format!("{name}: GXP {program_name} is not in the catalog"))?;
        variants.push(Variant {
            program: program.gxp,
            name: program_name.to_string(),
            wipe: x["wipe"].as_i64(),
            distortion: x["distortion"].as_u64().map(|v| v as u32),
            sampling: x["sampling"].as_u64().map(|v| v as u32),
            textures: x["textures"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|t| (t["slot"].as_u64().unwrap_or(0) as u32, t["unit"].as_u64().unwrap_or(0) as u32, t["dimensions"].as_str().unwrap_or("").to_string()))
                .collect(),
            blocks: x["blocks"].as_array().cloned().unwrap_or_default().iter().filter_map(|b| b.as_u64().map(|b| b as usize)).collect(),
            registered: None,
        });
    }
    if variants.is_empty() {
        return Err(format!("{name}: no GXP variant"));
    }
    let lowering = gxm["lowering"].as_str().map(str::to_string);
    let attributes = gxm["attributes"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| Some((p[0].as_u64()? as u32, p[1].as_str()?.to_string()))).collect())
        .unwrap_or_default();
    Ok(Shader { name, unavailable: false, lowering, attributes, blocks, variants })
}

#[derive(Default, Clone, Copy)]
struct Bound {
    gpu: u64,
    size: u32,
}

struct Inner {
    ctx: *mut SceGxmContext,
    patcher: *mut SceGxmShaderPatcher,
    next: Id,
    mems: HashMap<Id, Mem>,
    small: SmallPool,
    images: HashMap<Id, Image>,
    image_slots: HashMap<u32, View>,
    sampler_slots: HashMap<u32, Sampler>,
    shaders: HashMap<Id, Shader>,
    vertex_programs: HashMap<Vec<u32>, *mut SceGxmVertexProgram>,
    fragment_programs: HashMap<(usize, usize, [u8; 4], bool, SceGxmOutputRegisterFormat), *mut SceGxmFragmentProgram>,
    /// Position-only fullscreen vertex program for lowered compute passes.
    fullscreen: Option<(SceGxmShaderPatcherId, *const SceGxmProgram, *mut SceGxmVertexProgram)>,
    parameters: HashMap<(usize, String), *const SceGxmProgramParameter>,
    pipeline: Pipeline,
    vertex: [Bound; 16],
    index: Option<(u64, bool)>,
    uniforms: [HashMap<u32, Bound>; 3],
    storage: [HashMap<u32, Bound>; 3],
    textures: [HashMap<u32, (u32, u32)>; 3],
    /// Storage image slot -> image descriptor slot (compute stage).
    storage_images: HashMap<u32, u32>,
    compute: Id,
    viewport: [f32; 6],
    scissor: [u32; 4],
    target: Option<Id>,
    target_size: [u32; 2],
    target_format: u32,
    in_scene: bool,
    pending: bool,
    seq: u64,
    frame: u64,
    retired_images: Vec<(u64, *mut Vita2dTexture)>,
    retired_mems: Vec<(u64, Mem)>,
    /// Per-present counters: draw calls, outside a scene, program link
    /// failures, GXM draws issued, scenes begun, sceGxmDraw errors.
    stats: [u64; 6],
    /// (target, draws, clears) of each scene this present, for diagnostics.
    scenes: Vec<(Id, u32, u32)>,
    sequential: Block,
    display: Id,
}
unsafe impl Send for Inner {}

pub struct Gxm(Mutex<Inner>);

impl Gxm {
    pub fn new() -> Result<Self, String> {
        unsafe {
            if vita2d_init_advanced(256 * 1024) < 0 {
                return Err("vita2d_init_advanced failed".into());
            }
            vita2d_set_vblank_wait(1);
        }
        let ctx = unsafe { vita2d_get_context() };
        let patcher = unsafe { vita2d_get_shader_patcher() };
        if ctx.is_null() || patcher.is_null() {
            return Err("vita2d has no GXM context/shader patcher".into());
        }
        let sequential = Block::new(SEQUENTIAL_INDICES * 2).ok_or("sequential index allocation")?;
        unsafe {
            let indices = std::slice::from_raw_parts_mut(sequential.base.cast::<u16>(), SEQUENTIAL_INDICES);
            for (i, v) in indices.iter_mut().enumerate() {
                *v = i as u16;
            }
        }
        let mut inner = Inner {
            ctx,
            patcher,
            next: 1,
            mems: HashMap::new(),
            small: SmallPool::default(),
            images: HashMap::new(),
            image_slots: HashMap::new(),
            sampler_slots: HashMap::new(),
            shaders: HashMap::new(),
            vertex_programs: HashMap::new(),
            fragment_programs: HashMap::new(),
            fullscreen: None,
            parameters: HashMap::new(),
            pipeline: Pipeline::default(),
            vertex: [Bound::default(); 16],
            index: None,
            uniforms: Default::default(),
            storage: Default::default(),
            textures: Default::default(),
            storage_images: HashMap::new(),
            compute: 0,
            viewport: [0.0, 0.0, DISPLAY[0] as f32, DISPLAY[1] as f32, 0.0, 1.0],
            scissor: [0, 0, DISPLAY[0], DISPLAY[1]],
            target: None,
            target_size: DISPLAY,
            target_format: fmt::RGBA8_UNORM,
            in_scene: false,
            pending: false,
            seq: 0,
            frame: 0,
            retired_images: Vec::new(),
            retired_mems: Vec::new(),
            stats: [0; 6],
            scenes: Vec::new(),
            sequential,
            display: 0,
        };
        let display = inner.image_create(fmt::RGBA8_UNORM, DISPLAY[0], DISPLAY[1], 1, crysirror_wgpu_deko3d::driver::IMAGE_RENDER);
        if display <= 0 {
            return Err("display render target allocation failed".into());
        }
        inner.display = display;
        Ok(Self(Mutex::new(inner)))
    }
}

impl Drop for Gxm {
    fn drop(&mut self) {
        let mut inner = self.0.lock().unwrap();
        inner.sync();
        let patcher = inner.patcher;
        unsafe {
            for (_, vp) in inner.vertex_programs.drain() {
                sceGxmShaderPatcherReleaseVertexProgram(patcher, vp);
            }
            for (_, fp) in inner.fragment_programs.drain() {
                sceGxmShaderPatcherReleaseFragmentProgram(patcher, fp);
            }
            if let Some((id, _, vp)) = inner.fullscreen.take() {
                sceGxmShaderPatcherReleaseVertexProgram(patcher, vp);
                sceGxmShaderPatcherUnregisterProgram(patcher, id);
            }
            for shader in inner.shaders.values_mut() {
                for v in &mut shader.variants {
                    if let Some((id, _)) = v.registered.take() {
                        sceGxmShaderPatcherUnregisterProgram(patcher, id);
                    }
                }
            }
            for (_, image) in inner.images.drain() {
                for tex in [image.tex, image.pair] {
                    if !tex.is_null() {
                        vita2d_free_texture(tex);
                    }
                }
            }
            for (_, tex) in inner.retired_images.drain(..) {
                vita2d_free_texture(tex);
            }
        }
        inner.mems.clear();
        inner.retired_mems.clear();
        unsafe { vita2d_fini() };
    }
}

/// Colour surface and GXM render target for a colour-only vita2d texture,
/// stored in the texture's own fields so `vita2d_free_texture` destroys them.
unsafe fn init_render_target(tex: *mut Vita2dTexture, w: u32, h: u32, stride_pixels: u32, data: *mut u8, format: u32) -> bool {
    let Some((color, _, register_size)) = color_target(format) else { return false };
    unsafe {
        let base = tex.cast::<u8>();
        let surface = base.add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
        let rc = sceGxmColorSurfaceInit(
            surface,
            color,
            SCE_GXM_COLOR_SURFACE_LINEAR,
            SCE_GXM_COLOR_SURFACE_SCALE_NONE,
            register_size,
            w,
            h,
            stride_pixels,
            data.cast(),
        );
        if rc < 0 {
            log_once(&format!("sceGxmColorSurfaceInit format {format} {w}x{h} stride {stride_pixels}: {rc:#x}"));
            return false;
        }
        let params = SceGxmRenderTargetParams {
            flags: 0,
            width: w as u16,
            height: h as u16,
            scenesPerFrame: 1,
            multisampleMode: SCE_GXM_MULTISAMPLE_NONE as u16,
            multisampleLocations: 0,
            driverMemBlock: -1,
        };
        let mut target = std::ptr::null_mut();
        let rc = sceGxmCreateRenderTarget(&params, &mut target);
        if rc < 0 {
            log_once(&format!("sceGxmCreateRenderTarget {w}x{h}: {rc:#x}"));
            return false;
        }
        base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().write(target);
        true
    }
}

/// S8D24 tiled depth/stencil surface for a render target, as vita2d's own
/// render-target helper creates it; its memblock is freed with the texture.
unsafe fn init_depth_surface(tex: *mut Vita2dTexture, w: u32, h: u32) -> bool {
    unsafe {
        let stride = w.next_multiple_of(SCE_GXM_TILE_SIZEX);
        let rows = h.next_multiple_of(SCE_GXM_TILE_SIZEY);
        let size = (4 * stride * rows).next_multiple_of(256 * 1024) as SceSize;
        let mut uid = sceKernelAllocMemBlock(c"crysirror-gxm-depth".as_ptr(), SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW, size, std::ptr::null_mut());
        if uid < 0 {
            let size = (4 * stride * rows).next_multiple_of(4096) as SceSize;
            uid = sceKernelAllocMemBlock(c"crysirror-gxm-depth".as_ptr(), SCE_KERNEL_MEMBLOCK_TYPE_USER_RW_UNCACHE, size, std::ptr::null_mut());
        }
        if uid < 0 {
            return false;
        }
        let mut data = std::ptr::null_mut();
        if sceKernelGetMemBlockBase(uid, &mut data) < 0
            || sceGxmMapMemory(data, (4 * stride * rows).next_multiple_of(4096) as SceSize, SCE_GXM_MEMORY_ATTRIB_READ | SCE_GXM_MEMORY_ATTRIB_WRITE) < 0
        {
            sceKernelFreeMemBlock(uid);
            return false;
        }
        let base = tex.cast::<u8>();
        let surface = base.add(VITA2D_DEPTH_SURFACE).cast::<SceGxmDepthStencilSurface>();
        if sceGxmDepthStencilSurfaceInit(surface, SCE_GXM_DEPTH_STENCIL_FORMAT_S8D24, SCE_GXM_DEPTH_STENCIL_SURFACE_TILED, stride, data, std::ptr::null_mut()) < 0 {
            sceGxmUnmapMemory(data);
            sceKernelFreeMemBlock(uid);
            return false;
        }
        base.add(VITA2D_DEPTH_UID).cast::<SceUID>().write(uid);
        true
    }
}

impl Inner {
    fn id(&mut self) -> Id {
        self.next += 1;
        self.next
    }
    fn end_scene(&mut self) {
        if self.in_scene && tracing() {
            let [calls, outside, link, drawn, scenes, errors] = self.stats;
            let last = self.scenes.last().copied().unwrap_or_default();
            log(&format!("gxm trace: end scene target #{} d{} c{} (totals: {calls} calls, {outside} outside, {link} unlinked, {drawn} drawn, {scenes} scenes, {errors} errors)", last.0, last.1, last.2));
        }
        if self.in_scene {
            unsafe { sceGxmEndScene(self.ctx, std::ptr::null(), std::ptr::null()) };
            self.in_scene = false;
            if tracing() {
                if let Some(id) = self.scenes.last().map(|s| s.0) {
                    if let Some(i) = self.images.get(&id) {
                        let (tex, data, fmt_, stride) = (i.tex, i.data, i.format, i.stride);
                        unsafe {
                            vita2d_wait_rendering_done();
                            let base = tex.cast::<u8>();
                            let target = base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().read();
                            let surface = base.add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
                            for _ in 0..2 {
                                if sceGxmBeginScene(self.ctx, 0, target, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), surface, std::ptr::null()) >= 0 {
                                    sceGxmEndScene(self.ctx, std::ptr::null(), std::ptr::null());
                                }
                            }
                            vita2d_wait_rendering_done();
                            let first = std::slice::from_raw_parts(data, 16.min(stride));
                            log(&format!("gxm trace:   #{id} format {fmt_} stride {stride} data {data:?} first {first:?}"));
                        }
                    }
                }
            }
        }
    }
    /// End the open scene and wait until the GPU no longer reads anything.
    fn sync(&mut self) {
        self.end_scene();
        if self.pending {
            unsafe { vita2d_wait_rendering_done() };
            self.pending = false;
        }
    }
    fn image_create(&mut self, format: u32, w: u32, h: u32, mips: u32, flags: u32) -> Id {
        if is_depth(format) {
            let id = self.id();
            self.images.insert(id, Image { tex: std::ptr::null_mut(), pair: std::ptr::null_mut(), size: [w, h], format, bpp: 0, stride: 0, data: std::ptr::null_mut(), render: true, depth: true, has_depth: false });
            return id;
        }
        let Some((gxm_format, bpp)) = texture_format(format) else {
            log_once(&format!("texture format {format} is not available on GXM"));
            return 0;
        };
        if w == 0 || h == 0 || w > 4096 || h > 4096 {
            log_once(&format!("texture size {w}x{h} is outside GXM limits"));
            return 0;
        }
        if mips > 1 {
            log_once("mipmapped textures: only the base level is resident on GXM");
        }
        // Storage images are written by lowered compute passes, which render.
        let render = flags & (crysirror_wgpu_deko3d::driver::IMAGE_RENDER | crysirror_wgpu_deko3d::driver::IMAGE_STORAGE) != 0 || format == fmt::RGBA32_FLOAT;
        if render && color_target(format).is_none() {
            log_once(&format!("render target format {format} is not available on GXM"));
            return 0;
        }
        let Some((tex, stride, data)) = self.surface(gxm_format, format, w, h, bpp, render) else { return 0 };
        let mut pair = std::ptr::null_mut();
        if format == fmt::RGBA32_FLOAT {
            match self.surface(gxm_format, format, w, h, bpp, render) {
                Some((t, _, _)) => pair = t,
                None => {
                    unsafe { vita2d_free_texture(tex) };
                    return 0;
                }
            }
        }
        let id = self.id();
        self.images.insert(id, Image { tex, pair, size: [w, h], format, bpp, stride, data, render, depth: false, has_depth: false });
        id
    }
    /// One zeroed vita2d texture; a render target also gets its colour
    /// surface and GXM render target here. vita2d's render-target helper
    /// would also allocate a full-size depth/stencil surface for every
    /// target, doubling 2D canvases.
    fn surface(&mut self, gxm_format: SceGxmTextureFormat, format: u32, w: u32, h: u32, bpp: usize, render: bool) -> Option<(*mut Vita2dTexture, usize, *mut u8)> {
        let bytes = w as usize * h as usize * bpp;
        // vita2d sizes textures by its own bytes-per-pixel table, which has
        // no 64-bit formats (F16x4, F32x2 count as 4 bytes). Allocate those
        // as a U8x4 texture of twice the (8-aligned) width and re-describe
        // the same memory with the real format below.
        let wide = bpp == 8;
        let alloc_w = if wide { w.next_multiple_of(8) * 2 } else { w };
        let alloc_format = if wide { SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR } else { gxm_format };
        let create = |kind: u32| unsafe {
            vita2d_texture_set_alloc_memblock_type(kind);
            vita2d_create_empty_texture_format(alloc_w, h, alloc_format)
        };
        let attempt = || {
            let mut tex = std::ptr::null_mut();
            if render || bytes >= 256 * 1024 {
                tex = create(SCE_KERNEL_MEMBLOCK_TYPE_USER_CDRAM_RW);
            }
            if tex.is_null() {
                tex = create(SCE_KERNEL_MEMBLOCK_TYPE_USER_RW_UNCACHE);
            }
            tex
        };
        let mut tex = attempt();
        if tex.is_null() && self.reclaim() {
            tex = attempt();
        }
        if tex.is_null() {
            log(&format!("gxm: texture allocation {w}x{h} format {format} failed; {}", self.budget()));
            return None;
        }
        // vita2d_create_empty_texture_format leaves its render-target fields
        // uninitialized but vita2d_free_texture reads them.
        unsafe {
            let base = tex.cast::<u8>();
            base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().write(std::ptr::null_mut());
            base.add(VITA2D_DEPTH_UID).cast::<SceUID>().write(0);
        }
        let data = unsafe { vita2d_texture_get_datap(tex) }.cast::<u8>();
        let stride = if wide {
            let rc = unsafe { sceGxmTextureInitLinear(&mut (*tex).gxm_tex, data.cast(), gxm_format, w, h, 0) };
            if rc < 0 {
                unsafe { vita2d_free_texture(tex) };
                log_once(&format!("sceGxmTextureInitLinear format {format} {w}x{h}: {rc:#x}"));
                return None;
            }
            w.next_multiple_of(8) as usize * bpp
        } else {
            unsafe { vita2d_texture_get_stride(tex) as usize }
        };
        if render && !unsafe { init_render_target(tex, w, h, (stride / bpp) as u32, data, format) } {
            unsafe { vita2d_free_texture(tex) };
            log(&format!("gxm: render target {w}x{h} format {format} creation failed; {}", self.budget()));
            return None;
        }
        unsafe { std::ptr::write_bytes(data, 0, stride * h as usize) };
        Some((tex, stride, data))
    }
    /// Begin a scene on a render-target texture (and its depth surface).
    fn begin_scene(&mut self, tex: *mut Vita2dTexture, depth: bool) -> bool {
        unsafe {
            let base = tex.cast::<u8>();
            let surface = base.add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
            vita2d_pool_reset();
            let target = base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().read();
            let depth_surface = if depth { base.add(VITA2D_DEPTH_SURFACE).cast::<SceGxmDepthStencilSurface>() as *const _ } else { std::ptr::null() };
            let rc = sceGxmBeginScene(
                self.ctx,
                SCE_GXM_SCENE_FRAGMENT_SET_DEPENDENCY | SCE_GXM_SCENE_VERTEX_WAIT_FOR_DEPENDENCY,
                target,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                surface,
                depth_surface,
            );
            if rc < 0 {
                log_once(&format!("sceGxmBeginScene failed {rc:#x}"));
                return false;
            }
        }
        self.in_scene = true;
        self.pending = true;
        self.stats[4] += 1;
        true
    }
    fn parameter(&mut self, program: *const SceGxmProgram, name: &str) -> *const SceGxmProgramParameter {
        let key = (program as usize, name.to_string());
        if let Some(p) = self.parameters.get(&key) {
            return *p;
        }
        let c = CString::new(name).unwrap_or_default();
        let p = unsafe { sceGxmProgramFindParameterByName(program, c.as_ptr()) };
        self.parameters.insert(key, p);
        p
    }
    fn register(&mut self, shader: Id, variant: usize) -> Option<(SceGxmShaderPatcherId, *const SceGxmProgram)> {
        let patcher = self.patcher;
        let v = &mut self.shaders.get_mut(&shader)?.variants[variant];
        if v.registered.is_none() {
            let mut id = std::ptr::null_mut();
            let code = unsafe { sceGxmShaderPatcherRegisterProgram(patcher, v.program.as_ptr().cast(), &mut id) };
            if code < 0 {
                log_once(&format!("register {}: {code:#x}", v.name));
                return None;
            }
            let program = unsafe { sceGxmShaderPatcherGetProgramFromId(id) };
            v.registered = Some((id, program));
        }
        v.registered
    }
    fn vertex_program(&mut self, shader: Id, u32_indices: bool) -> Option<(*mut SceGxmVertexProgram, *const SceGxmProgram)> {
        let (id, program) = self.register(shader, 0)?;
        let p = self.pipeline;
        let mut key = vec![shader as u32, u32::from(u32_indices), p.buffer_count, p.attrib_count];
        key.extend(p.strides[..p.buffer_count as usize].iter().copied());
        key.extend(p.instanced[..p.buffer_count as usize].iter().copied());
        for a in &p.attribs[..p.attrib_count as usize] {
            key.extend([a.location, a.buffer, a.offset, a.ty, a.bits, a.components]);
        }
        if let Some(vp) = self.vertex_programs.get(&key) {
            return Some((*vp, program));
        }
        let mut attributes = Vec::new();
        for a in &p.attribs[..p.attrib_count as usize] {
            // Entry parameters are lowered to ATTRn; inputs declared as a
            // struct (puppet `vs_part(i: In)`) keep the TEXCOORDn their
            // `@location(n)` became, and are still vertex attributes.
            let named = self.shaders[&shader].attributes.iter().find(|(l, _)| *l == a.location).map(|(_, n)| n.clone());
            let mut param = match named {
                Some(name) => self.parameter(program, &name),
                None => unsafe { sceGxmProgramFindParameterBySemantic(program, SCE_GXM_PARAMETER_SEMANTIC_ATTR, a.location) },
            };
            if param.is_null() {
                let texcoord = unsafe { sceGxmProgramFindParameterBySemantic(program, SCE_GXM_PARAMETER_SEMANTIC_TEXCOORD, a.location) };
                if !texcoord.is_null() && unsafe { sceGxmProgramParameterGetCategory(texcoord) } == SCE_GXM_PARAMETER_CATEGORY_ATTRIBUTE {
                    param = texcoord;
                }
            }
            if param.is_null() {
                log_once(&format!("{}: no vertex input for location {}", self.shaders[&shader].name, a.location));
                continue;
            }
            let format = match (a.ty, a.bits) {
                (vt::FLOAT, 32) => SCE_GXM_ATTRIBUTE_FORMAT_F32,
                (vt::HALF, 16) => SCE_GXM_ATTRIBUTE_FORMAT_F16,
                (vt::UNORM, 8) => SCE_GXM_ATTRIBUTE_FORMAT_U8N,
                (vt::SNORM, 8) => SCE_GXM_ATTRIBUTE_FORMAT_S8N,
                (vt::UINT, 8) => SCE_GXM_ATTRIBUTE_FORMAT_U8,
                (vt::SINT, 8) => SCE_GXM_ATTRIBUTE_FORMAT_S8,
                (vt::UNORM, 16) => SCE_GXM_ATTRIBUTE_FORMAT_U16N,
                (vt::SNORM, 16) => SCE_GXM_ATTRIBUTE_FORMAT_S16N,
                (vt::UINT, 16) => SCE_GXM_ATTRIBUTE_FORMAT_U16,
                (vt::SINT, 16) => SCE_GXM_ATTRIBUTE_FORMAT_S16,
                // GXM has no 32-bit integer attributes. The lowered programs
                // read these scalar indices as floats; the low half carries
                // every index below 65536 exactly (little endian).
                (vt::UINT, 32) if a.components == 1 => SCE_GXM_ATTRIBUTE_FORMAT_U16,
                (vt::SINT, 32) if a.components == 1 => SCE_GXM_ATTRIBUTE_FORMAT_S16,
                _ => {
                    log_once(&format!("vertex attribute type {} bits {} x{} is not available on GXM", a.ty, a.bits, a.components));
                    return None;
                }
            };
            let mut attr: SceGxmVertexAttribute = unsafe { std::mem::zeroed() };
            attr.streamIndex = a.buffer as u16;
            attr.offset = a.offset as u16;
            attr.format = format as u8;
            attr.componentCount = a.components as u8;
            attr.regIndex = unsafe { sceGxmProgramParameterGetResourceIndex(param) } as u16;
            attributes.push(attr);
        }
        // WGSL `@builtin(vertex_index)` is lowered to `float cg_vertex_index :
        // ATTR0`; it is fed from the sequential u16 index buffer through one
        // extra stream, offset like the vertex streams at draw time.
        let index_param = self.parameter(program, "cg_vertex_index");
        let index_stream = (!index_param.is_null()).then_some(p.buffer_count as usize);
        if let Some(stream) = index_stream {
            let mut attr: SceGxmVertexAttribute = unsafe { std::mem::zeroed() };
            attr.streamIndex = stream as u16;
            attr.offset = 0;
            attr.format = SCE_GXM_ATTRIBUTE_FORMAT_U16 as u8;
            attr.componentCount = 1;
            attr.regIndex = unsafe { sceGxmProgramParameterGetResourceIndex(index_param) } as u16;
            attributes.push(attr);
        }
        let stream_count = (p.buffer_count as usize + usize::from(index_stream.is_some())).max(1);
        let streams: Vec<SceGxmVertexStream> = (0..stream_count)
            .map(|b| SceGxmVertexStream {
                stride: if Some(b) == index_stream { 2 } else { p.strides[b] as u16 },
                indexSource: if p.instanced[b] != 0 && Some(b) != index_stream {
                    SCE_GXM_INDEX_SOURCE_INSTANCE_16BIT
                } else if u32_indices {
                    SCE_GXM_INDEX_SOURCE_INDEX_32BIT
                } else {
                    SCE_GXM_INDEX_SOURCE_INDEX_16BIT
                } as u16,
            })
            .collect();
        let mut vp = std::ptr::null_mut();
        let code = unsafe {
            sceGxmShaderPatcherCreateVertexProgram(self.patcher, id, attributes.as_ptr(), attributes.len() as u32, streams.as_ptr(), streams.len() as u32, &mut vp)
        };
        if code < 0 || vp.is_null() {
            log_once(&format!("vertex program {}: {code:#x}", self.shaders[&shader].name));
            return None;
        }
        self.vertex_programs.insert(key, vp);
        Some((vp, program))
    }
    fn blend(&self) -> Result<Option<[u8; 4]>, String> {
        let b = self.pipeline.blends[0];
        let mask = |m: u8| -> u8 {
            (if m & 1 != 0 { SCE_GXM_COLOR_MASK_R as u8 } else { 0 })
                | (if m & 2 != 0 { SCE_GXM_COLOR_MASK_G as u8 } else { 0 })
                | (if m & 4 != 0 { SCE_GXM_COLOR_MASK_B as u8 } else { 0 })
                | (if m & 8 != 0 { SCE_GXM_COLOR_MASK_A as u8 } else { 0 })
        };
        if b.enable == 0 {
            if b.write_mask & 15 == 15 {
                return Ok(None);
            }
            return Ok(Some([mask(b.write_mask), 0, 0, 0]));
        }
        let func = |o: u8| -> Result<u8, String> {
            Ok(match o {
                0 => SCE_GXM_BLEND_FUNC_ADD,
                1 => SCE_GXM_BLEND_FUNC_SUBTRACT,
                2 => SCE_GXM_BLEND_FUNC_REVERSE_SUBTRACT,
                3 => SCE_GXM_BLEND_FUNC_MIN,
                4 => SCE_GXM_BLEND_FUNC_MAX,
                _ => return Err(format!("blend operation {o}")),
            } as u8)
        };
        // Backend factor numbering 0..=10 equals SceGxmBlendFactor; GXM has
        // no constant colour factor.
        let factor = |f: u8| -> Result<u8, String> { if f <= 10 { Ok(f) } else { Err(format!("blend factor {f} (constant colour) is not available on GXM")) } };
        Ok(Some([
            mask(b.write_mask),
            func(b.color_op)? | (func(b.alpha_op)? << 4),
            factor(b.color_src)? | (factor(b.color_dst)? << 4),
            factor(b.alpha_src)? | (factor(b.alpha_dst)? << 4),
        ]))
    }
    fn fragment_program(&mut self, shader: Id, variant: usize, vertex: *const SceGxmProgram) -> Option<(*mut SceGxmFragmentProgram, *const SceGxmProgram)> {
        let (id, program) = self.register(shader, variant)?;
        let blend = match self.blend() {
            Ok(b) => b,
            Err(e) => {
                log_once(&e);
                return None;
            }
        };
        let Some((_, output, _)) = color_target(self.target_format) else {
            log_once(&format!("render target format {} has no fragment output", self.target_format));
            return None;
        };
        if blend.is_some() && output != SCE_GXM_OUTPUT_REGISTER_FORMAT_UCHAR4 {
            log_once(&format!("blending into render target format {} is not available on GXM", self.target_format));
            return None;
        }
        let name = self.shaders[&shader].variants[variant].name.clone();
        self.patch_fragment(id, program, output, blend, vertex, &name).map(|fp| (fp, program))
    }
    fn patch_fragment(
        &mut self,
        id: SceGxmShaderPatcherId,
        program: *const SceGxmProgram,
        output: SceGxmOutputRegisterFormat,
        blend: Option<[u8; 4]>,
        vertex: *const SceGxmProgram,
        name: &str,
    ) -> Option<*mut SceGxmFragmentProgram> {
        let key = (program as usize, vertex as usize, blend.unwrap_or([255; 4]), blend.is_some(), output);
        if let Some(fp) = self.fragment_programs.get(&key) {
            return Some(*fp);
        }
        let info: Option<SceGxmBlendInfo> = blend.map(|bytes| unsafe { std::mem::transmute::<[u8; 4], SceGxmBlendInfo>(bytes) });
        let mut fp = std::ptr::null_mut();
        let code = unsafe {
            sceGxmShaderPatcherCreateFragmentProgram(
                self.patcher,
                id,
                output,
                SCE_GXM_MULTISAMPLE_NONE,
                info.as_ref().map_or(std::ptr::null(), |i| i),
                vertex,
                &mut fp,
            )
        };
        if code < 0 || fp.is_null() {
            log_once(&format!("fragment program {name}: {code:#x}"));
            return None;
        }
        self.fragment_programs.insert(key, fp);
        Some(fp)
    }
    /// The GXM texture for a bound (image slot, sampler slot).
    fn gxm_texture(&self, image_slot: u32, sampler_slot: u32) -> Option<(SceGxmTexture, [u32; 2])> {
        let view = self.image_slots.get(&image_slot)?;
        let image = self.images.get(&view.image)?;
        if image.tex.is_null() {
            return None;
        }
        let mut t = unsafe { (*image.tex).gxm_tex };
        let format = if view.format != 0 { view.format } else { image.format };
        let s = self.sampler_slots.get(&sampler_slot).copied().unwrap_or_default();
        // 32-bit float textures are data (texel loads); they are not filterable.
        let data = matches!(format, fmt::R32_FLOAT | fmt::RG32_FLOAT);
        let filter = |f: u32| if f != 0 && !data { SCE_GXM_TEXTURE_FILTER_LINEAR } else { SCE_GXM_TEXTURE_FILTER_POINT };
        let wrap = |w: u32| match w {
            1 => SCE_GXM_TEXTURE_ADDR_REPEAT,
            2 => SCE_GXM_TEXTURE_ADDR_MIRROR,
            _ => SCE_GXM_TEXTURE_ADDR_CLAMP,
        };
        unsafe {
            sceGxmTextureSetMinFilter(&mut t, filter(s.min_filter));
            sceGxmTextureSetMagFilter(&mut t, filter(s.mag_filter));
            sceGxmTextureSetUAddrMode(&mut t, wrap(s.wrap[0]));
            sceGxmTextureSetVAddrMode(&mut t, wrap(s.wrap[1]));
            sceGxmTextureSetGammaMode(&mut t, if is_srgb(format) { SCE_GXM_TEXTURE_GAMMA_BGR } else { SCE_GXM_TEXTURE_GAMMA_NONE });
        }
        Some((t, image.size))
    }
    /// Copy one uniform block (or one storage record) into a reserved default
    /// uniform buffer through the program's named parameters.
    fn write_block(&mut self, buffer: *mut u8, program: *const SceGxmProgram, block: &BlockMeta, source: *const u8, available: usize) {
        for m in &block.members {
            // Hand-written programs declare bare uniforms (empty block name).
            let name = if block.name.is_empty() { m.name.clone() } else { format!("{}.{}", block.name, m.name) };
            let param = self.parameter(program, &name);
            if param.is_null() || m.offset + m.bytes > available {
                continue;
            }
            let index = unsafe { sceGxmProgramParameterGetResourceIndex(param) } as usize;
            unsafe { std::ptr::copy_nonoverlapping(source.add(m.offset), buffer.add(index * 4), m.bytes) };
        }
    }
    fn select_wipe(&self, shader: &Shader) -> usize {
        // Kind from the bound transition uniform, as psv_wipe_program.
        let mut kind = None;
        for (slot, block) in shader.blocks.iter().enumerate() {
            if block.name == "wipe" {
                if let (Some(b), Some(m)) = (self.uniforms[1].get(&(slot as u32)), block.members.iter().find(|m| m.name == "kind_progress")) {
                    if b.gpu != 0 && b.size as usize >= m.offset + 4 {
                        let x = unsafe { (b.gpu as usize as *const u8).add(m.offset).cast::<f32>().read_unaligned() };
                        kind = Some(x.round() as i64);
                    }
                }
            }
        }
        let kind = kind.map(|k| if (5..200).contains(&k) && k != 50 { 900 } else { k });
        shader
            .variants
            .iter()
            .position(|v| v.wipe == kind && kind.is_some())
            .or_else(|| shader.variants.iter().position(|v| v.wipe == Some(-1)))
            .unwrap_or(0)
    }
    fn sampling(flags: u32) -> u32 {
        if flags & 128 != 0 {
            if flags & 64 != 0 { 3 } else { 4 }
        } else if flags & 32 != 0 {
            if flags & 64 != 0 { 1 } else { 2 }
        } else {
            0
        }
    }
    fn read_index(&self, at: u32) -> Option<u32> {
        let (gpu, u32i) = self.index?;
        Some(unsafe {
            if u32i {
                (gpu as usize as *const u32).add(at as usize).read_unaligned()
            } else {
                u32::from((gpu as usize as *const u16).add(at as usize).read_unaligned())
            }
        })
    }
    /// Filter record index of one vertex (the pipeline's location-3 attribute).
    fn filter_index(&self, vertex: i64) -> Option<u32> {
        let p = &self.pipeline;
        let a = p.attribs[..p.attrib_count as usize].iter().find(|a| a.location == 3)?;
        let b = self.vertex[a.buffer as usize];
        let at = vertex * i64::from(p.strides[a.buffer as usize]) + i64::from(a.offset);
        if b.gpu == 0 || at < 0 || at as u64 + 4 > u64::from(b.size) {
            return None;
        }
        let ptr = unsafe { (b.gpu as usize as *const u8).add(at as usize) };
        Some(unsafe {
            match a.ty {
                vt::FLOAT => ptr.cast::<f32>().read_unaligned() as u32,
                _ => ptr.cast::<u32>().read_unaligned(),
            }
        })
    }
    /// Draw `count` vertices/indices starting at `first`, split into runs that
    /// share one native program variant.
    fn issue(&mut self, prim: u32, count: u32, instances: u32, first: u32, base_vertex: i32, first_instance: u32, indexed: bool) {
        self.stats[0] += 1;
        if !self.in_scene {
            self.stats[1] += 1;
            return;
        }
        if count == 0 || instances == 0 {
            return;
        }
        let p = self.pipeline;
        let (Some(vs), Some(fs)) = (self.shaders.get(&p.vs), self.shaders.get(&p.fs)) else {
            log_once("draw without a vertex+fragment program (depth-only passes are not available on GXM)");
            return;
        };
        if vs.unavailable || fs.unavailable {
            log_once(&format!("draw refused: no native GXM program for {} / {}", vs.name, fs.name));
            return;
        }
        let filtered = fs.variants[0].distortion.is_some();
        let wipe = fs.variants[0].wipe.is_some();
        if !filtered {
            let variant = if wipe { self.select_wipe(fs) } else { 0 };
            self.draw_range(prim, first, count, instances, base_vertex, first_instance, indexed, variant, None);
            return;
        }
        if prim != 3 {
            log_once("filtered quads are drawn as triangle lists only");
            return;
        }
        // One LayerFilter record per primitive run (crysirror_shaders::native).
        let Some(storage_slot) = fs.blocks.iter().position(|b| b.storage) else { return };
        let record = fs.blocks[storage_slot].record;
        let Some(storage) = self.storage[1].get(&(storage_slot as u32)).copied() else {
            log_once("filtered quad without its filter records");
            return;
        };
        let triangles = count / 3;
        let mut run_start = 0;
        let mut run_filter = None;
        let mut runs = Vec::new();
        for t in 0..triangles {
            let at = first + t * 3;
            let v = if indexed { self.read_index(at).map(|i| i64::from(i) + i64::from(base_vertex)) } else { Some(i64::from(at)) };
            let f = v.and_then(|v| self.filter_index(v));
            if t == 0 {
                run_filter = f;
            } else if f != run_filter {
                runs.push((run_start, t - run_start, run_filter));
                run_start = t;
                run_filter = f;
            }
        }
        runs.push((run_start, triangles - run_start, run_filter));
        for (start, len, filter) in runs {
            let Some(filter) = filter else {
                log_once("filtered quad without a readable filter index");
                continue;
            };
            let offset = filter as usize * record;
            if storage.gpu == 0 || offset + record > storage.size as usize {
                log_once("filter index outside the bound filter records");
                continue;
            }
            let rec = unsafe { (storage.gpu as usize as *const u8).add(offset) };
            let distortion = unsafe { rec.add(56).cast::<f32>().read_unaligned() };
            let flags = unsafe { rec.add(44).cast::<f32>().read_unaligned() } as u32;
            if !distortion.is_finite() || !(0.0..=10.0).contains(&distortion) || distortion.fract() != 0.0 {
                log_once(&format!("filter distortion {distortion} has no native program"));
                continue;
            }
            let sampling = Self::sampling(flags);
            let fs = &self.shaders[&p.fs];
            let Some(variant) = fs.variants.iter().position(|v| v.distortion == Some(distortion as u32) && v.sampling == Some(sampling)) else {
                log_once(&format!("filter variant d{distortion} s{sampling} missing"));
                continue;
            };
            self.draw_range(prim, first + start * 3, len * 3, instances, base_vertex, first_instance, indexed, variant, Some((storage_slot, rec)));
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_range(&mut self, prim: u32, first: u32, count: u32, instances: u32, base_vertex: i32, first_instance: u32, indexed: bool, variant: usize, record: Option<(usize, *const u8)>) {
        let p = self.pipeline;
        let u32_indices = indexed && self.index.is_some_and(|(_, u)| u);
        let Some((vp, vprogram)) = self.vertex_program(p.vs, u32_indices) else {
            self.stats[2] += 1;
            return;
        };
        let Some((fp, fprogram)) = self.fragment_program(p.fs, variant, vprogram) else {
            self.stats[2] += 1;
            return;
        };
        let ctx = self.ctx;
        let primitive = match prim {
            0 => SCE_GXM_PRIMITIVE_POINTS,
            1 => SCE_GXM_PRIMITIVE_LINES,
            3 => SCE_GXM_PRIMITIVE_TRIANGLES,
            4 => SCE_GXM_PRIMITIVE_TRIANGLE_STRIP,
            _ => {
                log_once(&format!("primitive {prim} is not available on GXM"));
                return;
            }
        };
        unsafe {
            sceGxmSetVertexProgram(ctx, vp);
            sceGxmSetFragmentProgram(ctx, fp);
            let [x, y, w, h, zmin, zmax] = self.viewport;
            sceGxmSetViewport(ctx, x + w * 0.5, w * 0.5, y + h * 0.5, -h * 0.5, zmin, zmax - zmin);
            sceGxmSetViewportEnable(ctx, SCE_GXM_VIEWPORT_ENABLED);
            let [sx, sy, sw, sh] = self.scissor;
            let [tw, th] = self.target_size;
            let (x1, y1) = ((sx + sw).min(tw), (sy + sh).min(th));
            if sw == 0 || sh == 0 || sx >= x1 || sy >= y1 {
                return;
            }
            sceGxmSetRegionClip(ctx, SCE_GXM_REGION_CLIP_OUTSIDE, sx, sy, x1 - 1, y1 - 1);
            // The viewport's y flip reverses window-space winding.
            let front_ccw_in_window = p.front_face != 0;
            sceGxmSetCullMode(
                ctx,
                match p.cull {
                    1 => if front_ccw_in_window { SCE_GXM_CULL_CCW } else { SCE_GXM_CULL_CW },
                    2 => if front_ccw_in_window { SCE_GXM_CULL_CW } else { SCE_GXM_CULL_CCW },
                    _ => SCE_GXM_CULL_NONE,
                },
            );
            let depth_func = |c: u8| match c {
                0 => SCE_GXM_DEPTH_FUNC_NEVER,
                1 => SCE_GXM_DEPTH_FUNC_LESS,
                2 => SCE_GXM_DEPTH_FUNC_EQUAL,
                3 => SCE_GXM_DEPTH_FUNC_LESS_EQUAL,
                4 => SCE_GXM_DEPTH_FUNC_GREATER,
                5 => SCE_GXM_DEPTH_FUNC_NOT_EQUAL,
                6 => SCE_GXM_DEPTH_FUNC_GREATER_EQUAL,
                _ => SCE_GXM_DEPTH_FUNC_ALWAYS,
            };
            let stencil_func = |c: u8| match c {
                0 => SCE_GXM_STENCIL_FUNC_NEVER,
                1 => SCE_GXM_STENCIL_FUNC_LESS,
                2 => SCE_GXM_STENCIL_FUNC_EQUAL,
                3 => SCE_GXM_STENCIL_FUNC_LESS_EQUAL,
                4 => SCE_GXM_STENCIL_FUNC_GREATER,
                5 => SCE_GXM_STENCIL_FUNC_NOT_EQUAL,
                6 => SCE_GXM_STENCIL_FUNC_GREATER_EQUAL,
                _ => SCE_GXM_STENCIL_FUNC_ALWAYS,
            };
            let stencil_op = |o: u8| match o {
                1 => SCE_GXM_STENCIL_OP_ZERO,
                2 => SCE_GXM_STENCIL_OP_REPLACE,
                3 => SCE_GXM_STENCIL_OP_INVERT,
                4 => SCE_GXM_STENCIL_OP_INCR,
                5 => SCE_GXM_STENCIL_OP_DECR,
                6 => SCE_GXM_STENCIL_OP_INCR_WRAP,
                7 => SCE_GXM_STENCIL_OP_DECR_WRAP,
                _ => SCE_GXM_STENCIL_OP_KEEP,
            };
            sceGxmSetFrontDepthFunc(ctx, if p.depth_test != 0 { depth_func(p.depth_compare) } else { SCE_GXM_DEPTH_FUNC_ALWAYS });
            sceGxmSetFrontDepthWriteEnable(ctx, if p.depth_write != 0 { SCE_GXM_DEPTH_WRITE_ENABLED } else { SCE_GXM_DEPTH_WRITE_DISABLED });
            sceGxmSetTwoSidedEnable(ctx, SCE_GXM_TWO_SIDED_DISABLED);
            if p.stencil_enable != 0 {
                let f = p.stencil_front;
                sceGxmSetFrontStencilFunc(ctx, stencil_func(f[0]), stencil_op(f[1]), stencil_op(f[2]), stencil_op(f[3]), p.stencil_read_mask, p.stencil_write_mask);
            } else {
                sceGxmSetFrontStencilFunc(ctx, SCE_GXM_STENCIL_FUNC_ALWAYS, SCE_GXM_STENCIL_OP_KEEP, SCE_GXM_STENCIL_OP_KEEP, SCE_GXM_STENCIL_OP_KEEP, 0xff, 0);
            }
        }
        // Textures, dimensions and uniform blocks of both stages.
        for (stage, program, shader) in [(STAGE_VERTEX, vprogram, p.vs), (STAGE_FRAGMENT, fprogram, p.fs)] {
            let size = unsafe { sceGxmProgramGetDefaultUniformBufferSize(program) };
            let mut buffer: *mut c_void = std::ptr::null_mut();
            if size > 0 {
                unsafe {
                    if stage == STAGE_VERTEX {
                        sceGxmReserveVertexDefaultUniformBuffer(ctx, &mut buffer);
                    } else {
                        sceGxmReserveFragmentDefaultUniformBuffer(ctx, &mut buffer);
                    }
                }
            }
            let v = if stage == STAGE_VERTEX { 0 } else { variant };
            let (textures, blocks, metas) = {
                let s = &self.shaders[&shader];
                let var = &s.variants[v];
                (var.textures.clone(), var.blocks.clone(), s.blocks.clone())
            };
            for (slot, unit, dims) in textures {
                let Some(&(image_slot, sampler_slot)) = self.textures[stage as usize].get(&slot) else {
                    log_once(&format!("{}: stage {stage} texture slot {slot} has no binding", self.shaders[&shader].name));
                    continue;
                };
                let Some((t, size)) = self.gxm_texture(image_slot, sampler_slot) else {
                    log_once(&format!("{}: texture slot {slot} (image slot {image_slot}, sampler {sampler_slot}) is not a GXM texture", self.shaders[&shader].name));
                    continue;
                };
                unsafe {
                    if stage == STAGE_VERTEX {
                        sceGxmSetVertexTexture(ctx, unit, &t);
                    } else {
                        sceGxmSetFragmentTexture(ctx, unit, &t);
                    }
                }
                if !buffer.is_null() && !dims.is_empty() {
                    let param = self.parameter(program, &dims);
                    if !param.is_null() {
                        let index = unsafe { sceGxmProgramParameterGetResourceIndex(param) } as usize;
                        let (w, h) = (size[0] as f32, size[1] as f32);
                        let value = [w, h, 1.0 / w, 1.0 / h];
                        unsafe { std::ptr::copy_nonoverlapping(value.as_ptr().cast::<u8>(), buffer.cast::<u8>().add(index * 4), 16) };
                    }
                }
            }
            if buffer.is_null() {
                continue;
            }
            for slot in blocks {
                let Some(meta) = metas.get(slot) else { continue };
                if meta.storage {
                    if let Some((record_slot, rec)) = record.filter(|(s, _)| *s == slot && stage == STAGE_FRAGMENT) {
                        let _ = record_slot;
                        self.write_block(buffer.cast(), program, meta, rec, meta.record);
                    }
                    continue;
                }
                let Some(b) = self.uniforms[stage as usize].get(&(slot as u32)).copied() else {
                    log_once(&format!("{}: stage {stage} uniform block {slot} ({}) has no binding", self.shaders[&shader].name, meta.name));
                    continue;
                };
                if b.gpu != 0 {
                    self.write_block(buffer.cast(), program, meta, b.gpu as usize as *const u8, b.size as usize);
                }
            }
        }
        let index_param = self.parameter(vprogram, "cg_vertex_index");
        // Vertex streams: GXM has no base vertex or first-vertex offset, so
        // the stream pointers move instead.
        let stream_offset = |b: usize, vertex: i64, instance: i64| -> *const c_void {
            let stride = i64::from(p.strides[b]);
            let element = if p.instanced[b] != 0 { instance } else { vertex };
            (self.vertex[b].gpu as i64 + element * stride) as usize as *const c_void
        };
        let index_size = if u32_indices { 4 } else { 2 };
        let index_stream = (!index_param.is_null()).then_some(p.buffer_count);
        let sequential = self.sequential.base as usize;
        let set_index_stream = |vertex: i64| -> bool {
            let Some(stream) = index_stream else { return true };
            if !(0..SEQUENTIAL_INDICES as i64).contains(&vertex) {
                log_once(&format!("vertex index base {vertex} is outside the sequential index buffer"));
                return false;
            }
            unsafe { sceGxmSetVertexStream(ctx, stream, (sequential + vertex as usize * 2) as *const c_void) };
            true
        };
        for instance in 0..instances {
            let inst = i64::from(first_instance + instance);
            if indexed {
                let Some((gpu, _)) = self.index else { return };
                for b in 0..p.buffer_count as usize {
                    unsafe { sceGxmSetVertexStream(ctx, b as u32, stream_offset(b, i64::from(base_vertex), inst)) };
                }
                if !set_index_stream(i64::from(base_vertex)) {
                    return;
                }
                let indices = (gpu as usize + first as usize * index_size) as *const c_void;
                let format = if u32_indices { SCE_GXM_INDEX_FORMAT_U32 } else { SCE_GXM_INDEX_FORMAT_U16 };
                let rc = unsafe { sceGxmDraw(ctx, primitive, format, indices, count) };
                self.stats[if rc < 0 { 5 } else { 3 }] += 1;
                if let Some(scene) = self.scenes.last_mut() {
                    scene.1 += 1;
                }
            } else {
                let mut done = 0;
                while done < count {
                    let n = (count - done).min(SEQUENTIAL_INDICES as u32 - SEQUENTIAL_INDICES as u32 % 3);
                    for b in 0..p.buffer_count as usize {
                        unsafe { sceGxmSetVertexStream(ctx, b as u32, stream_offset(b, i64::from(first + done), inst)) };
                    }
                    if index_stream.is_some() && u64::from(first + done) + u64::from(n) > SEQUENTIAL_INDICES as u64 {
                        log_once("non-indexed draw past the sequential index buffer");
                        return;
                    }
                    if !set_index_stream(i64::from(first + done)) {
                        return;
                    }
                    let rc = unsafe { sceGxmDraw(ctx, primitive, SCE_GXM_INDEX_FORMAT_U16, self.sequential.base.cast(), n) };
                    self.stats[if rc < 0 { 5 } else { 3 }] += 1;
                    if let Some(scene) = self.scenes.last_mut() {
                        scene.1 += 1;
                    }
                    done += n;
                }
            }
        }
        self.pending = true;
    }
    fn image_rows(&self, view: &View) -> Option<&Image> {
        let image = self.images.get(&view.image)?;
        if image.tex.is_null() || image.data.is_null() {
            return None;
        }
        if !image.pair.is_null() {
            log_once("CPU copies of RGBA32F images are not available on GXM");
            return None;
        }
        if view.mip > 0 || view.layer > 0 {
            log_once("copies to mip levels/array layers above zero are not available on GXM");
            return None;
        }
        Some(image)
    }
    /// Clear a float render target: GXM has no clear and vita2d's clear
    /// program writes UCHAR4. The pass's scene is ended, the surface filled
    /// after the GPU is idle, and the scene begun again on the same target.
    fn clear_float(&mut self, rgba: [f32; 4]) {
        let Some(id) = self.target else { return };
        let Some((tex, size, stride, data)) = self.images.get(&id).map(|i| (i.tex, i.size, i.stride, i.data)) else { return };
        let has_depth = self.images[&id].has_depth;
        let mut texel = Vec::with_capacity(8);
        match self.target_format {
            fmt::RGBA16_FLOAT => rgba.iter().for_each(|v| texel.extend(f16_bits(*v).to_le_bytes())),
            fmt::RG32_FLOAT => rgba[..2].iter().for_each(|v| texel.extend(v.to_le_bytes())),
            fmt::R32_FLOAT => texel.extend(rgba[0].to_le_bytes()),
            other => {
                log_once(&format!("clear of render target format {other} is not available on GXM"));
                return;
            }
        }
        self.sync();
        let row = size[0] as usize * texel.len();
        for y in 0..size[1] as usize {
            let line = unsafe { std::slice::from_raw_parts_mut(data.add(y * stride), row) };
            for px in line.chunks_exact_mut(texel.len()) {
                px.copy_from_slice(&texel);
            }
        }
        if !self.begin_scene(tex, has_depth) {
            self.target = None;
        }
    }
    /// Position-only fullscreen triangle (the present vertex program) fed
    /// `cg_vertex_index` from the sequential index buffer.
    fn fullscreen(&mut self) -> Option<(*mut SceGxmVertexProgram, *const SceGxmProgram)> {
        if let Some((_, program, vp)) = self.fullscreen {
            return Some((vp, program));
        }
        let gxp = crysirror_shaders::native::psv_program("shared_present_vs_main_v")?;
        let mut id = std::ptr::null_mut();
        if unsafe { sceGxmShaderPatcherRegisterProgram(self.patcher, gxp.gxp.as_ptr().cast(), &mut id) } < 0 {
            log_once("register fullscreen vertex program failed");
            return None;
        }
        let program = unsafe { sceGxmShaderPatcherGetProgramFromId(id) };
        let index = self.parameter(program, "cg_vertex_index");
        if index.is_null() {
            log_once("fullscreen vertex program has no cg_vertex_index");
            return None;
        }
        let mut attr: SceGxmVertexAttribute = unsafe { std::mem::zeroed() };
        attr.format = SCE_GXM_ATTRIBUTE_FORMAT_U16 as u8;
        attr.componentCount = 1;
        attr.regIndex = unsafe { sceGxmProgramParameterGetResourceIndex(index) } as u16;
        let stream = SceGxmVertexStream { stride: 2, indexSource: SCE_GXM_INDEX_SOURCE_INDEX_16BIT as u16 };
        let mut vp = std::ptr::null_mut();
        let code = unsafe { sceGxmShaderPatcherCreateVertexProgram(self.patcher, id, &attr, 1, &stream, 1, &mut vp) };
        if code < 0 || vp.is_null() {
            log_once(&format!("fullscreen vertex program: {code:#x}"));
            return None;
        }
        self.fullscreen = Some((id, program, vp));
        Some((vp, program))
    }
    /// The bound compute entry through its native fragment lowering. Only
    /// the separable box filter has one (`crysirror-shader-tools psv`).
    /// The horizontal dispatch renders its exact 16-bit sums (RGBA32F on
    /// desktop) as two U8x4 passes of high/low bytes (`extra.y` 0 = RG,
    /// 1 = BA); the vertical dispatch reads both halves and writes the exact
    /// integer average into the output format.
    fn dispatch_lowered(&mut self) {
        let Some(shader) = self.shaders.get(&self.compute) else {
            log_once("compute dispatch without a compute program");
            return;
        };
        if shader.unavailable || shader.lowering.as_deref() != Some("box_filter") {
            log_once(&format!("compute dispatch refused: {} has no GXM fragment lowering", shader.name));
            return;
        }
        let Some(meta) = shader.blocks.first().cloned() else { return };
        let Some(params) = self.uniforms[STAGE_COMPUTE as usize].get(&0).copied().filter(|b| b.gpu != 0 && b.size >= 48) else {
            log_once("box filter dispatch without its parameters");
            return;
        };
        let mut values = [0u32; 12];
        unsafe { std::ptr::copy_nonoverlapping(params.gpu as usize as *const u32, values.as_mut_ptr(), 12) };
        let view = |s: &Self, slot: Option<u32>| slot.and_then(|slot| s.image_slots.get(&slot).copied());
        let (Some(output), Some(input)) = (
            view(self, self.storage_images.get(&0).copied()),
            view(self, self.textures[STAGE_COMPUTE as usize].get(&0).map(|t| t.0)),
        ) else {
            log_once("box filter dispatch without its input or output image");
            return;
        };
        let (Some(out), Some(src)) = (self.images.get(&output.image), self.images.get(&input.image)) else { return };
        let (out_tex, out_pair, out_format, out_size) = (out.tex, out.pair, out.format, out.size);
        let src_format = if input.format != 0 { input.format } else { src.format };
        let (src_tex, src_pair, src_size) = (src.tex, src.pair, src.size);
        if out_tex.is_null() || src_tex.is_null() || !out.render {
            log_once("box filter images are not GXM render targets/textures");
            return;
        }
        // (target, target format, extra.y, unit 0, unit 1)
        let passes: Vec<(*mut Vita2dTexture, u32, u32, *mut Vita2dTexture, *mut Vita2dTexture)> = if values[0] == 0 {
            if out_pair.is_null() {
                log_once("box filter horizontal sums need an RGBA32F output");
                return;
            }
            vec![(out_tex, fmt::RGBA8_UNORM, 0, src_tex, src_tex), (out_pair, fmt::RGBA8_UNORM, 1, src_tex, src_tex)]
        } else {
            if src_pair.is_null() {
                log_once("box filter vertical pass needs RGBA32F horizontal sums");
                return;
            }
            vec![(out_tex, out_format, 0, src_tex, src_pair)]
        };
        if tracing() {
            let rect: Vec<f32> = values[4..8].iter().map(|v| f32::from_bits(*v)).collect();
            log(&format!("gxm trace: box dispatch config {:?} rect {rect:?} extra {:?} input #{} ({src_format}) output #{} ({out_format}) size {out_size:?}", &values[..4], &values[8..], input.image, output.image));
        }
        self.run_box(values, passes, output.image, out_size, src_format, src_pair, src_tex, src_size);
    }
    #[allow(clippy::too_many_arguments)]
    fn run_box(&mut self, values: [u32; 12], passes: Vec<(*mut Vita2dTexture, u32, u32, *mut Vita2dTexture, *mut Vita2dTexture)>, output_image: Id, out_size: [u32; 2], src_format: u32, src_pair: *mut Vita2dTexture, src_tex: *mut Vita2dTexture, src_size: [u32; 2]) {
        let Some(meta) = self.shaders.get(&self.compute).and_then(|s| s.blocks.first().cloned()) else { return };
        let Some((id, program)) = self.register(self.compute, 0) else { return };
        let Some((vp, vprogram)) = self.fullscreen() else { return };
        let name = self.shaders[&self.compute].variants[0].name.clone();
        let textures = self.shaders[&self.compute].variants[0].textures.clone();
        self.end_scene();
        for (target, format, extra_y, unit0, unit1) in passes {
            let Some((_, output_format, _)) = color_target(format) else { return };
            let Some(fp) = self.patch_fragment(id, program, output_format, None, vprogram, &name) else { return };
            if !self.begin_scene(target, false) {
                return;
            }
            self.scenes.push((output_image, 1, 0));
            let ctx = self.ctx;
            let [w, h] = out_size;
            let mut local = values;
            local[9] = extra_y;

            unsafe {
                sceGxmSetVertexProgram(ctx, vp);
                sceGxmSetFragmentProgram(ctx, fp);
                sceGxmSetViewport(ctx, w as f32 * 0.5, w as f32 * 0.5, h as f32 * 0.5, -(h as f32) * 0.5, 0.0, 1.0);
                sceGxmSetViewportEnable(ctx, SCE_GXM_VIEWPORT_ENABLED);
                sceGxmSetRegionClip(ctx, SCE_GXM_REGION_CLIP_OUTSIDE, 0, 0, w - 1, h - 1);
                sceGxmSetCullMode(ctx, SCE_GXM_CULL_NONE);
                sceGxmSetFrontDepthFunc(ctx, SCE_GXM_DEPTH_FUNC_ALWAYS);
                sceGxmSetFrontDepthWriteEnable(ctx, SCE_GXM_DEPTH_WRITE_DISABLED);
                sceGxmSetFrontStencilFunc(ctx, SCE_GXM_STENCIL_FUNC_ALWAYS, SCE_GXM_STENCIL_OP_KEEP, SCE_GXM_STENCIL_OP_KEEP, SCE_GXM_STENCIL_OP_KEEP, 0xff, 0);
            }
            let mut buffer: *mut c_void = std::ptr::null_mut();
            unsafe { sceGxmReserveFragmentDefaultUniformBuffer(ctx, &mut buffer) };
            for (slot, unit, dims) in &textures {
                let tex = if *slot == 0 { unit0 } else { unit1 };
                let srgb = *slot == 0 && unit0 == src_tex && src_pair.is_null() && is_srgb(src_format);
                let mut t = unsafe { (*tex).gxm_tex };
                unsafe {
                    sceGxmTextureSetMinFilter(&mut t, SCE_GXM_TEXTURE_FILTER_POINT);
                    sceGxmTextureSetMagFilter(&mut t, SCE_GXM_TEXTURE_FILTER_POINT);
                    sceGxmTextureSetUAddrMode(&mut t, SCE_GXM_TEXTURE_ADDR_CLAMP);
                    sceGxmTextureSetVAddrMode(&mut t, SCE_GXM_TEXTURE_ADDR_CLAMP);
                    sceGxmTextureSetGammaMode(&mut t, if srgb { SCE_GXM_TEXTURE_GAMMA_BGR } else { SCE_GXM_TEXTURE_GAMMA_NONE });
                    sceGxmSetFragmentTexture(ctx, *unit, &t);
                }
                if !buffer.is_null() && !dims.is_empty() {
                    let param = self.parameter(program, dims);
                    if !param.is_null() {
                        let index = unsafe { sceGxmProgramParameterGetResourceIndex(param) } as usize;
                        let (tw, th) = (src_size[0] as f32, src_size[1] as f32);
                        let value = [tw, th, 1.0 / tw, 1.0 / th];
                        unsafe { std::ptr::copy_nonoverlapping(value.as_ptr().cast::<u8>(), buffer.cast::<u8>().add(index * 4), 16) };
                    }
                }
            }
            if !buffer.is_null() {
                self.write_block(buffer.cast(), program, &meta, local.as_ptr().cast(), 48);
            }
            let rc = unsafe {
                sceGxmSetVertexStream(ctx, 0, self.sequential.base.cast());
                sceGxmDraw(ctx, SCE_GXM_PRIMITIVE_TRIANGLES, SCE_GXM_INDEX_FORMAT_U16, self.sequential.base.cast(), 3)
            };
            self.stats[if rc < 0 { 5 } else { 3 }] += 1;
            // The next pass samples this target: wait for it, as `submit`
            // does between ordinary passes.
            self.sync();
            if tracing() && w == 8 && h == 4 {
                self.sync();
                let display = self.images.get(&self.display).map(|d| d.tex);
                unsafe {
                    for t in display.into_iter().chain([target, target]) {
                        let base = t.cast::<u8>();
                        let rt = base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().read();
                        let surface = base.add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
                        if sceGxmBeginScene(self.ctx, 0, rt, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), surface, std::ptr::null()) >= 0 {
                            sceGxmEndScene(self.ctx, std::ptr::null(), std::ptr::null());
                        }
                    }
                    vita2d_wait_rendering_done();
                    let data = vita2d_texture_get_datap(target).cast::<u8>();
                    let rows: Vec<Vec<u8>> = (0..2).map(|y| std::slice::from_raw_parts(data.add(y * 32), 32).to_vec()).collect();
                    log(&format!("gxm trace: box pass extra.y {extra_y} target rows {:?} {:?}", rows[0], rows[1]));
                }
            }
        }
        self.target = None;
    }
    /// Make a render target's pixels visible to CPU reads. On hardware
    /// `sync` suffices. Vita3K writes a colour surface back to guest memory
    /// only after later scenes (measured 2026-11-15: a cleared target read
    /// zero after sceGxmFinish; a target written by the scene just before
    /// read zero after one more empty scene on it, and its pixels after an
    /// empty scene on another surface and two on itself). Empty scenes load
    /// and store the tiles unchanged, so they are harmless on hardware.
    fn flush_for_cpu(&mut self, id: Id) {
        self.sync();
        let Some(image) = self.images.get(&id) else { return };
        if !image.render || image.tex.is_null() {
            return;
        }
        let tex = image.tex;
        let other = self.images.get(&self.display).map(|d| d.tex).filter(|d| *d != tex);
        unsafe {
            for t in other.into_iter().chain([tex, tex]) {
                let base = t.cast::<u8>();
                let target = base.add(VITA2D_RENDER_TARGET).cast::<*mut SceGxmRenderTarget>().read();
                let surface = base.add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
                if sceGxmBeginScene(self.ctx, 0, target, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), surface, std::ptr::null()) >= 0 {
                    sceGxmEndScene(self.ctx, std::ptr::null(), std::ptr::null());
                }
            }
            vita2d_wait_rendering_done();
        }
    }
    /// Resident GPU memory by owner, for allocation-failure reports.
    fn budget(&self) -> String {
        let live: usize = self.images.values().map(|i| i.stride * i.size[1] as usize).sum();
        let mut sizes: Vec<String> = self.images.values().filter(|i| !i.depth).map(|i| format!("{}x{}", i.size[0], i.size[1])).collect();
        sizes.sort();
        let own: usize = self.mems.values().map(|m| if let Mem::Own(b) = m { b.size } else { 0 }).sum();
        let retired_own: usize = self.retired_mems.iter().map(|(_, m)| if let Mem::Own(b) = m { b.size } else { 0 }).sum();
        format!(
            "{} textures {live} bytes [{}]; {} retired textures; buffers {own} bytes + {} retired; {} small arenas; {}",
            self.images.len(),
            sizes.join(" "),
            self.retired_images.len(),
            retired_own,
            self.small.arenas.len(),
            memory_report()
        )
    }
    /// Wait for the GPU and free every retired texture and buffer now. Used
    /// when an allocation fails: a burst of destroys inside one engine step
    /// (no present between them) would otherwise hold memory for
    /// `RETIRE_FRAMES` presents. Returns whether anything was freed.
    fn reclaim(&mut self) -> bool {
        if self.retired_images.is_empty() && self.retired_mems.is_empty() {
            return false;
        }
        self.sync();
        unsafe { vita2d_wait_rendering_done() };
        for (_, tex) in self.retired_images.drain(..) {
            unsafe { vita2d_free_texture(tex) };
        }
        for (_, mem) in std::mem::take(&mut self.retired_mems) {
            self.small.release(mem);
        }
        true
    }
    fn retire(&mut self) {
        let frame = self.frame;
        self.retired_images.retain(|(at, tex)| {
            if frame >= at + RETIRE_FRAMES {
                unsafe { vita2d_free_texture(*tex) };
                false
            } else {
                true
            }
        });
        let (expired, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.retired_mems).into_iter().partition(|(at, _)| frame >= at + RETIRE_FRAMES);
        self.retired_mems = kept;
        for (_, mem) in expired {
            self.small.release(mem);
        }
    }
}

impl Driver for Gxm {
    fn name(&self) -> &str {
        "PS Vita GXM"
    }
    fn mem_create(&self, size: u64, _kind: u32) -> Id {
        let mut s = self.0.lock().unwrap();
        let size = size as usize;
        let alloc = |s: &mut Inner| if size <= SMALL_MAX { s.small.alloc(size) } else { Block::new(size).map(Mem::Own) };
        let mut mem = alloc(&mut s);
        if mem.is_none() && s.reclaim() {
            mem = alloc(&mut s);
        }
        let Some(mem) = mem else {
            log(&format!("gxm: GPU memory allocation of {size} bytes failed; {}", s.budget()));
            return 0;
        };
        let id = s.id();
        s.mems.insert(id, mem);
        id
    }
    fn mem_destroy(&self, id: Id) {
        let mut s = self.0.lock().unwrap();
        if let Some(block) = s.mems.remove(&id) {
            let frame = s.frame;
            s.retired_mems.push((frame, block));
        }
    }
    fn mem_cpu(&self, id: Id) -> *mut u8 {
        self.0.lock().unwrap().mems.get(&id).map_or(std::ptr::null_mut(), Mem::base)
    }
    fn mem_gpu(&self, id: Id) -> u64 {
        // GXM addresses are the CPU mapping.
        self.0.lock().unwrap().mems.get(&id).map_or(0, |m| m.base() as usize as u64)
    }
    fn mem_invalidate(&self, _: Id, _: u64, _: u64) {}
    fn image_create(&self, format: u32, ty: u32, w: u32, h: u32, d: u32, mips: u32, samples: u32, flags: u32) -> Id {
        if ty != crysirror_wgpu_deko3d::driver::ty::D2 || d > 1 || samples > 1 {
            log_once(&format!("texture type {ty} depth {d} samples {samples} is not available on GXM"));
            return 0;
        }
        self.0.lock().unwrap().image_create(format, w, h, mips, flags)
    }
    fn renderable(&self, format: u32) -> bool {
        // Depth and stencil attach to their colour target's surface. RGBA32F
        // exists only as the split storage target of lowered compute.
        (color_target(format).is_some() && format != fmt::RGBA32_FLOAT) || is_depth(format)
    }
    fn blendable(&self, format: u32) -> bool {
        // The shader patcher blends only the UCHAR4 output register.
        matches!(format, fmt::RGBA8_UNORM | fmt::RGBA8_UNORM_SRGB)
    }
    fn storage_format(&self, format: u32) -> bool {
        // Outputs of the box filter's fragment lowering, the only compute
        // entry with a native program: its byte-packed sums (RGBA32F) and
        // U8x4 averages. F16 colour surfaces are not used for it (Vita3K
        // 0.2.1 returned zeros when sampling them).
        matches!(format, fmt::RGBA32_FLOAT | fmt::RGBA8_UNORM)
    }
    fn image_destroy(&self, id: Id) {
        let mut s = self.0.lock().unwrap();
        if let Some(image) = s.images.remove(&id) {
            let frame = s.frame;
            for tex in [image.tex, image.pair] {
                if !tex.is_null() {
                    s.retired_images.push((frame, tex));
                }
            }
        }
    }
    fn image_descriptor(&self, slot: u32, view: &View, _load_store: bool) -> bool {
        self.0.lock().unwrap().image_slots.insert(slot, *view);
        true
    }
    fn sampler_descriptor(&self, slot: u32, sampler: &Sampler) -> bool {
        self.0.lock().unwrap().sampler_slots.insert(slot, *sampler);
        true
    }
    fn shader(&self, code: &[u8]) -> Id {
        let mut s = self.0.lock().unwrap();
        match parse_shader(code) {
            Ok(shader) => {
                let id = s.id();
                s.shaders.insert(id, shader);
                id
            }
            Err(e) => {
                log_once(&e);
                0
            }
        }
    }
    fn begin(&self) {}
    fn submit(&self) -> u64 {
        let mut s = self.0.lock().unwrap();
        s.sync();
        s.seq += 1;
        s.seq
    }
    fn done(&self, _: u64) -> bool {
        true
    }
    fn wait(&self, _: u64) {}
    fn barrier(&self) {}
    fn targets(&self, colors: &[View], depth: Option<&View>) {
        let mut s = self.0.lock().unwrap();
        s.end_scene();
        let Some(view) = colors.first() else {
            log_once("depth-only passes are not available on GXM");
            s.target = None;
            return;
        };
        let Some(image) = s.images.get(&view.image) else { return };
        if !image.render || image.tex.is_null() {
            log_once("render pass target is not a GXM render target");
            s.target = None;
            return;
        }
        let tex = image.tex;
        let size = image.size;
        let mut has_depth = image.has_depth;
        let format = if view.format != 0 { view.format } else { image.format };
        if depth.is_some() && !has_depth {
            has_depth = unsafe { init_depth_surface(tex, size[0], size[1]) };
            if has_depth {
                if let Some(image) = s.images.get_mut(&view.image) {
                    image.has_depth = true;
                }
            } else {
                log(&format!("gxm: depth/stencil surface {}x{} allocation failed; {}", size[0], size[1], s.budget()));
            }
        }
        if !s.images[&view.image].pair.is_null() {
            log_once("RGBA32F render passes are not available on GXM (two RG32F halves)");
            s.target = None;
            return;
        }
        unsafe {
            let surface = tex.cast::<u8>().add(VITA2D_COLOR_SURFACE).cast::<SceGxmColorSurface>();
            sceGxmColorSurfaceSetGammaMode(surface, if is_srgb(format) { SCE_GXM_COLOR_SURFACE_GAMMA_BGR } else { SCE_GXM_COLOR_SURFACE_GAMMA_NONE });
        }
        if !s.begin_scene(tex, has_depth) {
            s.target = None;
            return;
        }
        s.scenes.push((view.image, 0, 0));
        s.target = Some(view.image);
        s.target_format = format;
        s.target_size = size;
        s.viewport = [0.0, 0.0, size[0] as f32, size[1] as f32, 0.0, 1.0];
        s.scissor = [0, 0, size[0], size[1]];
    }
    fn clear_color(&self, _index: u32, rgba: [f32; 4]) {
        let mut s = self.0.lock().unwrap();
        if !s.in_scene {
            return;
        }
        if let Some(scene) = s.scenes.last_mut() {
            scene.2 += 1;
        }
        if !matches!(s.target_format, fmt::RGBA8_UNORM | fmt::RGBA8_UNORM_SRGB) {
            s.clear_float(rgba);
            return;
        }
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
        unsafe {
            vita2d_set_clear_color(b(rgba[0]) | (b(rgba[1]) << 8) | (b(rgba[2]) << 16) | (b(rgba[3]) << 24));
            vita2d_clear_screen();
        }
    }
    fn clear_depth_stencil(&self, _: bool, _: f32, _: bool, _: u8) {
        // vita2d scenes start with depth 1 and stencil 0.
    }
    fn resolve(&self, _: &View, _: &View) {
        log_once("multisample resolve is not available on GXM");
    }
    fn viewport(&self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        self.0.lock().unwrap().viewport = [x, y, w, h, min_depth, max_depth];
    }
    fn scissor(&self, x: u32, y: u32, w: u32, h: u32) {
        self.0.lock().unwrap().scissor = [x, y, w, h];
    }
    fn blend_constant(&self, _: [f32; 4]) {}
    fn stencil_reference(&self, reference: u8, _: u8, _: u8) {
        let s = self.0.lock().unwrap();
        if s.in_scene {
            unsafe { sceGxmSetFrontStencilRef(s.ctx, u32::from(reference)) };
        }
    }
    fn bind_graphics(&self, pipeline: &Pipeline) {
        self.0.lock().unwrap().pipeline = *pipeline;
    }
    fn bind_compute(&self, cs: Id) {
        self.0.lock().unwrap().compute = cs;
    }
    fn vertex_buffer(&self, slot: u32, gpu: u64, size: u32) {
        if let Some(v) = self.0.lock().unwrap().vertex.get_mut(slot as usize) {
            *v = Bound { gpu, size };
        }
    }
    fn index_buffer(&self, gpu: u64, u32_indices: bool) {
        self.0.lock().unwrap().index = Some((gpu, u32_indices));
    }
    fn uniform(&self, stage: u32, slot: u32, gpu: u64, size: u32) {
        if let Some(map) = self.0.lock().unwrap().uniforms.get_mut(stage as usize) {
            map.insert(slot, Bound { gpu, size });
        }
    }
    fn storage(&self, stage: u32, slot: u32, gpu: u64, size: u32) {
        if let Some(map) = self.0.lock().unwrap().storage.get_mut(stage as usize) {
            map.insert(slot, Bound { gpu, size });
        }
    }
    fn texture(&self, stage: u32, slot: u32, image_slot: u32, sampler_slot: u32) {
        if let Some(map) = self.0.lock().unwrap().textures.get_mut(stage as usize) {
            map.insert(slot, (image_slot, sampler_slot));
        }
    }
    fn image(&self, stage: u32, slot: u32, image_slot: u32) {
        if stage == STAGE_COMPUTE {
            self.0.lock().unwrap().storage_images.insert(slot, image_slot);
        } else {
            log_once("storage images outside compute are not available on GXM");
        }
    }
    fn draw(&self, prim: u32, count: u32, instances: u32, first: u32, first_instance: u32) {
        self.0.lock().unwrap().issue(prim, count, instances, first, 0, first_instance, false);
    }
    fn draw_indexed(&self, prim: u32, count: u32, instances: u32, first: u32, base_vertex: i32, first_instance: u32) {
        self.0.lock().unwrap().issue(prim, count, instances, first, base_vertex, first_instance, true);
    }
    fn draw_indirect(&self, prim: u32, gpu: u64, indexed: bool) {
        // Arguments live in CPU-visible memory written before this pass.
        let words = unsafe { std::slice::from_raw_parts(gpu as usize as *const u32, 5) };
        let mut s = self.0.lock().unwrap();
        if indexed {
            s.issue(prim, words[0], words[1], words[2], words[3] as i32, words[4], true);
        } else {
            s.issue(prim, words[0], words[1], words[2], 0, words[3], false);
        }
    }
    fn dispatch(&self, _: u32, _: u32, _: u32) {
        // Every lowered entry covers its whole output; the group counts only
        // tile the original compute version.
        self.0.lock().unwrap().dispatch_lowered();
    }
    fn dispatch_indirect(&self, _: u64) {
        log_once("compute dispatch is not available on GXM");
    }
    fn copy_buffer(&self, src: u64, dst: u64, size: u64) {
        let mut s = self.0.lock().unwrap();
        s.sync();
        unsafe { std::ptr::copy(src as usize as *const u8, dst as usize as *mut u8, size as usize) };
    }
    fn copy_buffer_to_image(&self, src: u64, row_bytes: u32, _image_rows: u32, dst: &View, origin: [u32; 3], extent: [u32; 3]) {
        let mut s = self.0.lock().unwrap();
        s.sync();
        let Some(image) = s.image_rows(dst) else { return };
        let (w, h) = (extent[0].min(image.size[0].saturating_sub(origin[0])) as usize, extent[1].min(image.size[1].saturating_sub(origin[1])) as usize);
        for y in 0..h {
            unsafe {
                let from = (src as usize + y * row_bytes as usize) as *const u8;
                let to = image.data.add((origin[1] as usize + y) * image.stride + origin[0] as usize * image.bpp);
                std::ptr::copy_nonoverlapping(from, to, w * image.bpp);
            }
        }
    }
    fn copy_image_to_buffer(&self, src: &View, origin: [u32; 3], extent: [u32; 3], dst: u64, row_bytes: u32, _image_rows: u32) {
        let mut s = self.0.lock().unwrap();
        s.flush_for_cpu(src.image);
        if tracing() {
            let small: Vec<Id> = s.images.iter().filter(|(_, i)| i.size == [8, 4] && !i.data.is_null()).map(|(id, _)| *id).collect();
            for id in small {
                s.flush_for_cpu(id);
                let i = &s.images[&id];
                let rows: Vec<Vec<u8>> = (0..2).map(|y| unsafe { std::slice::from_raw_parts(i.data.add(y * i.stride), 32) }.to_vec()).collect();
                log(&format!("gxm trace: image #{id} f{} row0 {:?} row1 {:?}", i.format, rows[0], rows[1]));
            }
            let first = s.images.get(&src.image).filter(|i| !i.data.is_null()).map(|i| unsafe { std::slice::from_raw_parts(i.data, 8) }.to_vec());
            log(&format!("gxm trace: readback #{} origin {origin:?} extent {extent:?} row {row_bytes} first texels {first:?}", src.image));
        }
        let Some(image) = s.image_rows(src) else { return };
        let (w, h) = (extent[0].min(image.size[0].saturating_sub(origin[0])) as usize, extent[1].min(image.size[1].saturating_sub(origin[1])) as usize);
        for y in 0..h {
            unsafe {
                let from = image.data.add((origin[1] as usize + y) * image.stride + origin[0] as usize * image.bpp);
                let to = (dst as usize + y * row_bytes as usize) as *mut u8;
                std::ptr::copy_nonoverlapping(from, to, w * image.bpp);
            }
        }
    }
    fn copy_image(&self, src: &View, so: [u32; 3], dst: &View, d: [u32; 3], extent: [u32; 3]) {
        let mut s = self.0.lock().unwrap();
        s.flush_for_cpu(src.image);
        let (Some(a), Some(b)) = (s.image_rows(src), s.image_rows(dst)) else { return };
        if a.bpp != b.bpp {
            log_once("texture copy between formats of different size");
            return;
        }
        let w = (extent[0].min(a.size[0].saturating_sub(so[0])).min(b.size[0].saturating_sub(d[0]))) as usize;
        let h = (extent[1].min(a.size[1].saturating_sub(so[1])).min(b.size[1].saturating_sub(d[1]))) as usize;
        for y in 0..h {
            unsafe {
                let from = a.data.add((so[1] as usize + y) * a.stride + so[0] as usize * a.bpp);
                let to = b.data.add((d[1] as usize + y) * b.stride + d[0] as usize * b.bpp);
                std::ptr::copy(from, to, w * a.bpp);
            }
        }
    }
    fn display_size(&self) -> [u32; 2] {
        DISPLAY
    }
    fn display_image(&self, _: u32) -> Id {
        self.0.lock().unwrap().display
    }
    fn acquire(&self) -> Option<u32> {
        Some(0)
    }
    fn present(&self, _: u32) {
        let mut s = self.0.lock().unwrap();
        s.sync();
        let display = s.display;
        let Some(tex) = s.images.get(&display).map(|i| i.tex) else { return };
        unsafe {
            vita2d_pool_reset();
            vita2d_start_drawing_advanced(std::ptr::null_mut(), 0);
            vita2d_draw_texture(tex, 0.0, 0.0);
            vita2d_end_drawing();
            vita2d_wait_rendering_done();
            vita2d_swap_buffers();
        }
        if let Some(dir) = DUMP_IMAGES.lock().ok().and_then(|mut d| d.take()) {
            let _ = std::fs::create_dir_all(&dir);
            let ids: Vec<Id> = s.images.keys().copied().collect();
            for id in ids {
                s.flush_for_cpu(id);
            }
            for (id, image) in &s.images {
                if image.depth || image.data.is_null() {
                    continue;
                }
                let bytes = unsafe { std::slice::from_raw_parts(image.data, image.stride * image.size[1] as usize) };
                let name = format!("{id}-{}x{}-f{}-s{}{}.raw", image.size[0], image.size[1], image.format, image.stride, if image.render { "-rt" } else { "" });
                // Write from a heap copy: Vita3K rejects file writes sourced
                // from some GPU-mapped memory.
                if let Err(e) = std::fs::write(dir.join(&name), bytes.to_vec()) {
                    log(&format!("gxm: dump {name}: {e}"));
                }
            }
            log(&format!("gxm: dumped {} images", s.images.len()));
        }
        if s.frame < 4 || s.frame % 300 == 0 {
            let [calls, outside, link, drawn, scenes, errors] = s.stats;
            log(&format!("gxm: present {}: {scenes} scenes, {calls} draw calls ({outside} outside a scene, {link} without a linked program), {drawn} GXM draws, {errors} draw errors", s.frame));
            let list: Vec<String> = s.scenes.iter().map(|(t, d, c)| format!("#{t}{} d{d} c{c}", if *t == s.display { "(display)" } else { "" })).collect();
            log(&format!("gxm:   scenes {}", list.join(", ")));
        }
        s.scenes.clear();
        s.stats = [0; 6];
        s.frame += 1;
        s.retire();
    }
}
