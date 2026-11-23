// SPDX-License-Identifier: MPL-2.0
//! wgpu custom-backend interfaces over a [`Driver`].
//!
//! Commands are recorded as plain values and replayed into one driver command
//! list per `Queue::submit`, after the queue's own buffer/texture writes (wgpu
//! ordering). Resources are freed only after the submission that last saw
//! them completes. Unsupported API (ray tracing, timestamps, render bundles,
//! external textures, surfaces) panics with the feature name instead of
//! silently doing nothing; the adapter never advertises those features.
#![allow(clippy::too_many_arguments)]

use crate::driver::{self, fmt, ty, Driver, Id, View};
use crate::library::{self, Program, ShaderLibrary, Stage};
use std::collections::BTreeMap;
use std::future::ready;
use std::ops::Range;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use wgpu::custom::*;

const IMAGE_SLOTS: u32 = 16384;
const SAMPLER_SLOTS: u32 = 2048;
const STAGING_CHUNK: u64 = 8 * 1024 * 1024;

fn unsupported(what: &str) -> ! {
    panic!("deko3d backend: {what} is not supported on this console")
}

// ---------------------------------------------------------------- shared

#[derive(Default)]
struct Slots {
    next: u32,
    free: Vec<u32>,
}
impl Slots {
    fn take(&mut self, limit: u32) -> u32 {
        if let Some(s) = self.free.pop() {
            return s;
        }
        assert!(self.next < limit, "deko3d backend: descriptor table exhausted");
        self.next += 1;
        self.next - 1
    }
}

enum Retire {
    Mem(Id),
    Image(Id),
    ImageSlot(u32),
    SamplerSlot(u32),
}

struct Chunk {
    mem: Id,
    cpu: usize,
    gpu: u64,
    size: u64,
    used: u64,
    retired: Option<u64>,
}

struct MapRequest {
    after: u64,
    buffer: Arc<BufInner>,
    range: Range<u64>,
    read: bool,
    callback: BufferMapCallback,
}

pub(crate) struct Shared {
    pub driver: Arc<dyn Driver>,
    pub library: Arc<dyn ShaderLibrary>,
    last_seq: AtomicU64,
    retire: Mutex<Vec<(u64, Retire)>>,
    image_slots: Mutex<Slots>,
    sampler_slots: Mutex<Slots>,
    pending: Mutex<Vec<Cmd>>,
    staging: Mutex<Vec<Chunk>>,
    maps: Mutex<Vec<MapRequest>>,
    done_callbacks: Mutex<Vec<(u64, BoxSubmittedWorkDoneCallback)>>,
    submit_lock: Mutex<()>,
}
impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deko3d::Shared({})", self.driver.name())
    }
}

impl Shared {
    pub fn new(driver: Arc<dyn Driver>, library: Arc<dyn ShaderLibrary>) -> Arc<Self> {
        Arc::new(Self {
            driver,
            library,
            last_seq: AtomicU64::new(0),
            retire: Mutex::default(),
            image_slots: Mutex::default(),
            sampler_slots: Mutex::default(),
            pending: Mutex::default(),
            staging: Mutex::default(),
            maps: Mutex::default(),
            done_callbacks: Mutex::default(),
            submit_lock: Mutex::default(),
        })
    }
    fn later(&self, r: Retire) {
        let seq = self.last_seq.load(Ordering::Acquire);
        self.retire.lock().unwrap().push((seq, r));
    }
    /// Staging memory valid until the next submission completes.
    fn stage(&self, bytes: &[u8]) -> u64 {
        let size = (bytes.len() as u64).max(1);
        let mut chunks = self.staging.lock().unwrap();
        let driver = &self.driver;
        for c in chunks.iter_mut() {
            if let Some(seq) = c.retired {
                if driver.done(seq) {
                    c.retired = None;
                    c.used = 0;
                }
            }
        }
        let pick = chunks.iter().position(|c| c.retired.is_none() && (c.used + 255) / 256 * 256 + size <= c.size);
        let index = match pick {
            Some(i) => i,
            None => {
                let csize = size.max(STAGING_CHUNK).next_multiple_of(4096);
                let mem = driver.mem_create(csize, driver::MEM_UPLOAD);
                assert!(mem > 0, "deko3d backend: staging allocation of {csize} bytes failed");
                chunks.push(Chunk { mem, cpu: driver.mem_cpu(mem) as usize, gpu: driver.mem_gpu(mem), size: csize, used: 0, retired: None });
                chunks.len() - 1
            }
        };
        let c = &mut chunks[index];
        let offset = c.used.next_multiple_of(256);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), (c.cpu + offset as usize) as *mut u8, bytes.len()) };
        c.used = offset + size;
        c.gpu + offset
    }
    fn reclaim(&self) {
        let driver = &self.driver;
        let ready: Vec<Retire> = {
            let mut list = self.retire.lock().unwrap();
            let (done, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut *list).into_iter().partition(|(seq, _)| *seq == 0 || driver.done(*seq));
            *list = keep;
            done.into_iter().map(|(_, r)| r).collect()
        };
        for r in ready {
            match r {
                Retire::Mem(id) => driver.mem_destroy(id),
                Retire::Image(id) => driver.image_destroy(id),
                Retire::ImageSlot(s) => self.image_slots.lock().unwrap().free.push(s),
                Retire::SamplerSlot(s) => self.sampler_slots.lock().unwrap().free.push(s),
            }
        }
        let due: Vec<MapRequest> = {
            let mut maps = self.maps.lock().unwrap();
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut *maps).into_iter().partition(|m| m.after == 0 || driver.done(m.after));
            *maps = keep;
            due
        };
        for m in due {
            if m.read {
                driver.mem_invalidate(m.buffer.mem, m.range.start, m.range.end - m.range.start);
            }
            (m.callback)(Ok(()));
        }
        let calls: Vec<BoxSubmittedWorkDoneCallback> = {
            let mut cbs = self.done_callbacks.lock().unwrap();
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut *cbs).into_iter().partition(|(seq, _)| *seq == 0 || driver.done(*seq));
            *cbs = keep;
            due.into_iter().map(|(_, c)| c).collect()
        };
        for c in calls {
            c();
        }
    }
    fn idle(&self) -> bool {
        self.driver.done(self.last_seq.load(Ordering::Acquire))
    }
}

// ---------------------------------------------------------------- formats

pub(crate) fn cs_format(f: wgpu::TextureFormat) -> u32 {
    use wgpu::TextureFormat as T;
    match f {
        T::R8Unorm => fmt::R8_UNORM,
        T::R8Snorm => fmt::R8_SNORM,
        T::R8Uint => fmt::R8_UINT,
        T::R8Sint => fmt::R8_SINT,
        T::R16Float => fmt::R16_FLOAT,
        T::R16Unorm => fmt::R16_UNORM,
        T::R16Uint => fmt::R16_UINT,
        T::R16Sint => fmt::R16_SINT,
        T::R32Float => fmt::R32_FLOAT,
        T::R32Uint => fmt::R32_UINT,
        T::R32Sint => fmt::R32_SINT,
        T::Rg8Unorm => fmt::RG8_UNORM,
        T::Rg8Snorm => fmt::RG8_SNORM,
        T::Rg8Uint => fmt::RG8_UINT,
        T::Rg8Sint => fmt::RG8_SINT,
        T::Rg16Float => fmt::RG16_FLOAT,
        T::Rg16Unorm => fmt::RG16_UNORM,
        T::Rg16Uint => fmt::RG16_UINT,
        T::Rg16Sint => fmt::RG16_SINT,
        T::Rg32Float => fmt::RG32_FLOAT,
        T::Rg32Uint => fmt::RG32_UINT,
        T::Rg32Sint => fmt::RG32_SINT,
        T::Rgba8Unorm => fmt::RGBA8_UNORM,
        T::Rgba8UnormSrgb => fmt::RGBA8_UNORM_SRGB,
        T::Rgba8Snorm => fmt::RGBA8_SNORM,
        T::Rgba8Uint => fmt::RGBA8_UINT,
        T::Rgba8Sint => fmt::RGBA8_SINT,
        T::Bgra8Unorm => fmt::BGRA8_UNORM,
        T::Bgra8UnormSrgb => fmt::BGRA8_UNORM_SRGB,
        T::Rgba16Float => fmt::RGBA16_FLOAT,
        T::Rgba16Unorm => fmt::RGBA16_UNORM,
        T::Rgba16Uint => fmt::RGBA16_UINT,
        T::Rgba16Sint => fmt::RGBA16_SINT,
        T::Rgba32Float => fmt::RGBA32_FLOAT,
        T::Rgba32Uint => fmt::RGBA32_UINT,
        T::Rgba32Sint => fmt::RGBA32_SINT,
        T::Rgb10a2Unorm => fmt::RGB10A2_UNORM,
        T::Rg11b10Ufloat => fmt::RG11B10_FLOAT,
        T::Depth16Unorm => fmt::DEPTH16,
        T::Depth24Plus => fmt::DEPTH24,
        T::Depth24PlusStencil8 => fmt::DEPTH24_STENCIL8,
        T::Depth32Float => fmt::DEPTH32F,
        T::Depth32FloatStencil8 => fmt::DEPTH32F_STENCIL8,
        T::Stencil8 => fmt::STENCIL8,
        T::Bc1RgbaUnorm => fmt::BC1,
        T::Bc1RgbaUnormSrgb => fmt::BC1_SRGB,
        T::Bc2RgbaUnorm => fmt::BC2,
        T::Bc2RgbaUnormSrgb => fmt::BC2_SRGB,
        T::Bc3RgbaUnorm => fmt::BC3,
        T::Bc3RgbaUnormSrgb => fmt::BC3_SRGB,
        T::Bc4RUnorm => fmt::BC4_UNORM,
        T::Bc4RSnorm => fmt::BC4_SNORM,
        T::Bc5RgUnorm => fmt::BC5_UNORM,
        T::Bc5RgSnorm => fmt::BC5_SNORM,
        T::Bc6hRgbUfloat => fmt::BC6H_UFLOAT,
        T::Bc6hRgbFloat => fmt::BC6H_SFLOAT,
        T::Bc7RgbaUnorm => fmt::BC7,
        T::Bc7RgbaUnormSrgb => fmt::BC7_SRGB,
        other => unsupported(&format!("texture format {other:?}")),
    }
}
fn supported_format(f: wgpu::TextureFormat) -> bool {
    std::panic::catch_unwind(|| cs_format(f)).is_ok()
}

// ---------------------------------------------------------------- instance / adapter

#[derive(Debug, Clone)]
pub struct Instance(pub(crate) Arc<Shared>);
#[derive(Debug, Clone)]
pub struct Adapter(pub(crate) Arc<Shared>);
#[derive(Debug, Clone)]
pub struct Device(pub(crate) Arc<Shared>);
#[derive(Debug, Clone)]
pub struct Queue(pub(crate) Arc<Shared>);

pub(crate) fn features() -> wgpu::Features {
    wgpu::Features::TEXTURE_COMPRESSION_BC
        | wgpu::Features::FLOAT32_FILTERABLE
        | wgpu::Features::DEPTH_CLIP_CONTROL
        | wgpu::Features::INDIRECT_FIRST_INSTANCE
        | wgpu::Features::DEPTH32FLOAT_STENCIL8
}
pub(crate) fn limits() -> wgpu::Limits {
    wgpu::Limits {
        max_texture_dimension_1d: 16384,
        max_texture_dimension_2d: 16384,
        max_texture_dimension_3d: 2048,
        max_texture_array_layers: 2048,
        max_bind_groups: 8,
        max_storage_buffers_per_shader_stage: 16,
        max_uniform_buffers_per_shader_stage: 14,
        max_sampled_textures_per_shader_stage: 32,
        max_samplers_per_shader_stage: 32,
        max_storage_textures_per_shader_stage: 8,
        max_uniform_buffer_binding_size: 64 * 1024,
        max_storage_buffer_binding_size: 256 * 1024 * 1024,
        max_buffer_size: 1 << 30,
        min_uniform_buffer_offset_alignment: 256,
        min_storage_buffer_offset_alignment: 256,
        ..wgpu::Limits::default()
    }
}
fn info() -> wgpu::AdapterInfo {
    wgpu::AdapterInfo {
        name: "NVIDIA Tegra X1 (deko3d)".into(),
        vendor: 0x10de,
        device: 0,
        device_type: wgpu::DeviceType::IntegratedGpu,
        device_pci_bus_id: String::new(),
        driver: "deko3d".into(),
        driver_info: "CrySirror custom wgpu backend".into(),
        backend: wgpu::Backend::Noop,
        subgroup_min_size: 32,
        subgroup_max_size: 32,
        transient_saves_memory: None,
        limit_bucket: None,
    }
}

impl InstanceInterface for Instance {
    fn new(_: wgpu::InstanceDescriptor) -> Self {
        unsupported("Instance::new (use crysirror_wgpu_deko3d::create)")
    }
    unsafe fn create_surface(&self, _: wgpu::SurfaceTargetUnsafe) -> Result<DispatchSurface, wgpu::CreateSurfaceError> {
        unsupported("surfaces (the console host owns the display)")
    }
    fn request_adapter(&self, _: &wgpu::RequestAdapterOptions<'_, '_>) -> Pin<Box<dyn RequestAdapterFuture>> {
        let a = DispatchAdapter::custom(Adapter(self.0.clone()));
        Box::pin(ready(Ok(a)))
    }
    fn poll_all_devices(&self, _: bool) -> bool {
        self.0.reclaim();
        self.0.idle()
    }
    fn wgsl_language_features(&self) -> wgpu::WgslLanguageFeatures {
        wgpu::WgslLanguageFeatures::empty()
    }
    fn enumerate_adapters(&self, _: wgpu::Backends) -> Pin<Box<dyn EnumerateAdapterFuture>> {
        let a = DispatchAdapter::custom(Adapter(self.0.clone()));
        Box::pin(ready(vec![a]))
    }
}

impl AdapterInterface for Adapter {
    fn request_device(&self, _: &wgpu::DeviceDescriptor<'_>) -> Pin<Box<dyn RequestDeviceFuture>> {
        let d = DispatchDevice::custom(Device(self.0.clone()));
        let q = DispatchQueue::custom(Queue(self.0.clone()));
        Box::pin(ready(Ok((d, q))))
    }
    fn is_surface_supported(&self, _: &DispatchSurface) -> bool {
        false
    }
    fn features(&self) -> wgpu::Features {
        features()
    }
    fn limits(&self) -> wgpu::Limits {
        limits()
    }
    fn downlevel_capabilities(&self) -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities::default()
    }
    fn get_info(&self) -> wgpu::AdapterInfo {
        info()
    }
    fn get_texture_format_features(&self, format: wgpu::TextureFormat) -> wgpu::TextureFormatFeatures {
        use wgpu::TextureFormatFeatureFlags as F;
        if !supported_format(format) {
            return wgpu::TextureFormatFeatures { allowed_usages: wgpu::TextureUsages::empty(), flags: F::empty() };
        }
        let mut flags = F::FILTERABLE;
        if !format.is_compressed() && !format.is_depth_stencil_format() {
            flags |= F::MULTISAMPLE_X4 | F::MULTISAMPLE_RESOLVE | F::BLENDABLE;
        }
        if format.is_depth_stencil_format() {
            flags |= F::MULTISAMPLE_X4;
        }
        let mut allowed_usages = wgpu::TextureUsages::all();
        if !self.0.driver.renderable(cs_format(format)) {
            allowed_usages.remove(wgpu::TextureUsages::RENDER_ATTACHMENT);
            flags.remove(F::MULTISAMPLE_X4 | F::MULTISAMPLE_RESOLVE | F::BLENDABLE);
        }
        if !self.0.driver.blendable(cs_format(format)) {
            flags.remove(F::BLENDABLE);
        }
        if !self.0.driver.storage_format(cs_format(format)) {
            allowed_usages.remove(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::STORAGE_ATOMIC);
        }
        wgpu::TextureFormatFeatures { allowed_usages, flags }
    }
    fn get_presentation_timestamp(&self) -> wgpu::PresentationTimestamp {
        wgpu::PresentationTimestamp::INVALID_TIMESTAMP
    }
    fn cooperative_matrix_properties(&self) -> Vec<wgpu::wgt::CooperativeMatrixProperties> {
        Vec::new()
    }
}

// ---------------------------------------------------------------- resources

#[derive(Debug)]
pub(crate) struct BufInner {
    shared: Arc<Shared>,
    mem: Id,
    cpu: usize,
    gpu: u64,
    size: u64,
}
impl Drop for BufInner {
    fn drop(&mut self) {
        self.shared.later(Retire::Mem(self.mem));
    }
}
#[derive(Debug, Clone)]
pub struct Buffer(Arc<BufInner>);

#[derive(Debug)]
pub(crate) struct TexInner {
    shared: Arc<Shared>,
    image: Id,
    owned: bool,
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
    mips: u32,
    samples: u32,
    dimension: wgpu::TextureDimension,
}
impl Drop for TexInner {
    fn drop(&mut self) {
        if self.owned {
            self.shared.later(Retire::Image(self.image));
        }
    }
}
#[derive(Debug, Clone)]
pub struct Texture(pub(crate) Arc<TexInner>);

#[derive(Debug)]
pub(crate) struct ViewInner {
    tex: Arc<TexInner>,
    view: View,
    sampled: Option<u32>,
    storage: Option<u32>,
    size: [u32; 2],
}
impl Drop for ViewInner {
    fn drop(&mut self) {
        if let Some(s) = self.sampled {
            self.tex.shared.later(Retire::ImageSlot(s));
        }
        if let Some(s) = self.storage {
            self.tex.shared.later(Retire::ImageSlot(s));
        }
    }
}
#[derive(Debug, Clone)]
pub struct TextureView(Arc<ViewInner>);

#[derive(Debug)]
pub(crate) struct SamplerInner {
    shared: Arc<Shared>,
    slot: u32,
}
impl Drop for SamplerInner {
    fn drop(&mut self) {
        self.shared.later(Retire::SamplerSlot(self.slot));
    }
}
#[derive(Debug, Clone)]
pub struct Sampler(Arc<SamplerInner>);

#[derive(Debug, Clone, Default)]
pub struct BindGroupLayout(Arc<Vec<(u32, bool)>>);

#[derive(Debug, Clone)]
enum Res {
    Buffer(Arc<BufInner>, u64, Option<u64>),
    View(Arc<ViewInner>),
    Sampler(Arc<SamplerInner>),
}
#[derive(Debug)]
pub(crate) struct BgInner {
    entries: BTreeMap<u32, Res>,
    dynamic: Vec<u32>,
}
#[derive(Debug, Clone)]
pub struct BindGroup(Arc<BgInner>);

#[derive(Debug, Clone)]
pub struct PipelineLayout(Arc<Vec<BindGroupLayout>>);

#[derive(Debug)]
pub struct ShaderModule {
    source: Arc<str>,
    hash: String,
}

#[derive(Debug)]
pub(crate) struct RpInner {
    vs: Arc<Program>,
    fs: Option<Arc<Program>>,
    state: driver::Pipeline,
    prim: u32,
    layout: Vec<BindGroupLayout>,
}
#[derive(Debug, Clone)]
pub struct RenderPipeline(Arc<RpInner>);

#[derive(Debug)]
pub(crate) struct CpInner {
    cs: Arc<Program>,
    layout: Vec<BindGroupLayout>,
}
#[derive(Debug, Clone)]
pub struct ComputePipeline(Arc<CpInner>);

#[derive(Debug)]
pub struct Unsupported;

// ---------------------------------------------------------------- commands

#[derive(Debug, Clone, Copy)]
struct Copy3 {
    origin: [u32; 3],
    extent: [u32; 3],
    mip: u32,
    aspect_stencil: bool,
}

#[derive(Debug)]
enum Cmd {
    CopyBuffer { src: Arc<BufInner>, so: u64, dst: Arc<BufInner>, d: u64, size: u64 },
    StagedToBuffer { src: u64, dst: Arc<BufInner>, d: u64, size: u64 },
    BufferToTexture { src: u64, keep: Option<Arc<BufInner>>, bytes_per_row: u32, rows: u32, dst: Arc<TexInner>, region: Copy3 },
    TextureToBuffer { src: Arc<TexInner>, region: Copy3, dst: Arc<BufInner>, offset: u64, bytes_per_row: u32, rows: u32 },
    TextureToTexture { src: Arc<TexInner>, sr: Copy3, dst: Arc<TexInner>, dr: Copy3 },
    ClearBuffer { dst: Arc<BufInner>, offset: u64, size: u64 },
    Render(Box<PassDesc>, Vec<RCmd>),
    Compute(Vec<CCmd>),
}

#[derive(Debug)]
struct PassDesc {
    colors: Vec<(Arc<ViewInner>, Option<Arc<ViewInner>>, Option<[f32; 4]>)>,
    depth: Option<(Arc<ViewInner>, Option<f32>, Option<u8>)>,
}

#[derive(Debug)]
enum RCmd {
    Pipeline(Arc<RpInner>),
    Group(u32, Option<Arc<BgInner>>, Vec<u32>),
    Index(Arc<BufInner>, bool, u64),
    Vertex(u32, Option<Arc<BufInner>>, u64, Option<u64>),
    Viewport([f32; 6]),
    Scissor([u32; 4]),
    StencilRef(u32),
    BlendConstant([f32; 4]),
    Draw(Range<u32>, Range<u32>),
    DrawIndexed(Range<u32>, i32, Range<u32>),
    DrawIndirect(Arc<BufInner>, u64, bool, u32),
}

#[derive(Debug)]
enum CCmd {
    Pipeline(Arc<CpInner>),
    Group(u32, Option<Arc<BgInner>>, Vec<u32>),
    Dispatch(u32, u32, u32),
    DispatchIndirect(Arc<BufInner>, u64),
}

type CmdList = Arc<Mutex<Vec<Cmd>>>;

#[derive(Debug)]
pub struct CommandEncoder(CmdList);
#[derive(Debug)]
pub struct CommandBuffer(Mutex<Vec<Cmd>>);
#[derive(Debug)]
pub struct RenderPass {
    list: CmdList,
    desc: Option<Box<PassDesc>>,
    cmds: Vec<RCmd>,
}
impl Drop for RenderPass {
    fn drop(&mut self) {
        if let Some(desc) = self.desc.take() {
            self.list.lock().unwrap().push(Cmd::Render(desc, std::mem::take(&mut self.cmds)));
        }
    }
}
#[derive(Debug)]
pub struct ComputePass {
    list: CmdList,
    cmds: Vec<CCmd>,
}
impl Drop for ComputePass {
    fn drop(&mut self) {
        let cmds = std::mem::take(&mut self.cmds);
        self.list.lock().unwrap().push(Cmd::Compute(cmds));
    }
}

fn buf(b: &DispatchBuffer) -> Arc<BufInner> {
    b.as_custom::<Buffer>().expect("deko3d buffer").0.clone()
}
fn api_buf(b: &wgpu::Buffer) -> Arc<BufInner> {
    b.as_custom::<Buffer>().expect("deko3d buffer").0.clone()
}
fn api_tex(t: &wgpu::Texture) -> Arc<TexInner> {
    t.as_custom::<Texture>().expect("deko3d texture").0.clone()
}
fn api_view(v: &wgpu::TextureView) -> Arc<ViewInner> {
    v.as_custom::<TextureView>().expect("deko3d texture view").0.clone()
}
fn group(g: &DispatchBindGroup) -> Arc<BgInner> {
    g.as_custom::<BindGroup>().expect("deko3d bind group").0.clone()
}
fn bgl(l: &wgpu::BindGroupLayout) -> BindGroupLayout {
    l.as_custom::<BindGroupLayout>().expect("deko3d bind group layout").clone()
}

// ---------------------------------------------------------------- device

fn view_type(dim: wgpu::TextureViewDimension, samples: u32) -> u32 {
    use wgpu::TextureViewDimension as D;
    match dim {
        D::D1 => ty::D1,
        D::D2 if samples > 1 => ty::D2_MS,
        D::D2 => ty::D2,
        D::D2Array if samples > 1 => ty::D2_MS_ARRAY,
        D::D2Array => ty::D2_ARRAY,
        D::Cube => ty::CUBE,
        D::CubeArray => ty::CUBE_ARRAY,
        D::D3 => ty::D3,
    }
}
fn image_type(t: &TexInner) -> u32 {
    match t.dimension {
        wgpu::TextureDimension::D1 => ty::D1,
        wgpu::TextureDimension::D3 => ty::D3,
        wgpu::TextureDimension::D2 if t.size.depth_or_array_layers > 1 || t.samples > 1 && t.size.depth_or_array_layers > 1 => {
            if t.samples > 1 { ty::D2_MS_ARRAY } else { ty::D2_ARRAY }
        }
        wgpu::TextureDimension::D2 => if t.samples > 1 { ty::D2_MS } else { ty::D2 },
    }
}
fn region_view(t: &TexInner, r: &Copy3, layer: u32) -> View {
    let d3 = t.dimension == wgpu::TextureDimension::D3;
    View {
        image: t.image,
        ty: image_type(t),
        format: 0,
        mip: r.mip,
        mips: 1,
        layer: if d3 { 0 } else { layer },
        layers: if d3 { 0 } else { 1 },
        stencil: u32::from(r.aspect_stencil),
    }
}

fn blend_factor(f: wgpu::BlendFactor) -> u8 {
    use wgpu::BlendFactor as B;
    (match f {
        B::Zero => 0,
        B::One => 1,
        B::Src => 2,
        B::OneMinusSrc => 3,
        B::SrcAlpha => 4,
        B::OneMinusSrcAlpha => 5,
        B::Dst => 6,
        B::OneMinusDst => 7,
        B::DstAlpha => 8,
        B::OneMinusDstAlpha => 9,
        B::SrcAlphaSaturated => 10,
        B::Constant => 11,
        B::OneMinusConstant => 12,
        other => unsupported(&format!("blend factor {other:?}")),
    }) as u8
}
fn blend_op(o: wgpu::BlendOperation) -> u8 {
    use wgpu::BlendOperation as O;
    match o {
        O::Add => 0,
        O::Subtract => 1,
        O::ReverseSubtract => 2,
        O::Min => 3,
        O::Max => 4,
    }
}
fn compare(c: wgpu::CompareFunction) -> u8 {
    use wgpu::CompareFunction as C;
    match c {
        C::Never => 0,
        C::Less => 1,
        C::Equal => 2,
        C::LessEqual => 3,
        C::Greater => 4,
        C::NotEqual => 5,
        C::GreaterEqual => 6,
        C::Always => 7,
    }
}
fn stencil_op(o: wgpu::StencilOperation) -> u8 {
    use wgpu::StencilOperation as S;
    match o {
        S::Keep => 0,
        S::Zero => 1,
        S::Replace => 2,
        S::Invert => 3,
        S::IncrementClamp => 4,
        S::DecrementClamp => 5,
        S::IncrementWrap => 6,
        S::DecrementWrap => 7,
    }
}
fn stencil_face(s: &wgpu::StencilFaceState) -> [u8; 4] {
    [compare(s.compare), stencil_op(s.fail_op), stencil_op(s.depth_fail_op), stencil_op(s.pass_op)]
}
fn vertex_format(f: wgpu::VertexFormat) -> (u32, u32, u32) {
    use driver::vt;
    use wgpu::VertexFormat as V;
    match f {
        V::Float32 => (vt::FLOAT, 32, 1),
        V::Float32x2 => (vt::FLOAT, 32, 2),
        V::Float32x3 => (vt::FLOAT, 32, 3),
        V::Float32x4 => (vt::FLOAT, 32, 4),
        V::Uint32 => (vt::UINT, 32, 1),
        V::Uint32x2 => (vt::UINT, 32, 2),
        V::Uint32x3 => (vt::UINT, 32, 3),
        V::Uint32x4 => (vt::UINT, 32, 4),
        V::Sint32 => (vt::SINT, 32, 1),
        V::Sint32x2 => (vt::SINT, 32, 2),
        V::Sint32x3 => (vt::SINT, 32, 3),
        V::Sint32x4 => (vt::SINT, 32, 4),
        V::Float16x2 => (vt::HALF, 16, 2),
        V::Float16x4 => (vt::HALF, 16, 4),
        V::Unorm16x2 => (vt::UNORM, 16, 2),
        V::Unorm16x4 => (vt::UNORM, 16, 4),
        V::Snorm16x2 => (vt::SNORM, 16, 2),
        V::Snorm16x4 => (vt::SNORM, 16, 4),
        V::Uint16x2 => (vt::UINT, 16, 2),
        V::Uint16x4 => (vt::UINT, 16, 4),
        V::Sint16x2 => (vt::SINT, 16, 2),
        V::Sint16x4 => (vt::SINT, 16, 4),
        V::Unorm8x2 => (vt::UNORM, 8, 2),
        V::Unorm8x4 => (vt::UNORM, 8, 4),
        V::Snorm8x2 => (vt::SNORM, 8, 2),
        V::Snorm8x4 => (vt::SNORM, 8, 4),
        V::Uint8x2 => (vt::UINT, 8, 2),
        V::Uint8x4 => (vt::UINT, 8, 4),
        V::Sint8x2 => (vt::SINT, 8, 2),
        V::Sint8x4 => (vt::SINT, 8, 4),
        other => unsupported(&format!("vertex format {other:?}")),
    }
}

impl Device {
    fn program(&self, module: &wgpu::ShaderModule, entry: Option<&str>, stage: Stage) -> Arc<Program> {
        let m = module.as_custom::<ShaderModule>().expect("deko3d shader module");
        self.0
            .library
            .program(&*self.0.driver, &m.source, &m.hash, entry, stage)
            .unwrap_or_else(|e| panic!("deko3d backend: {e}"))
    }
}

impl DeviceInterface for Device {
    fn features(&self) -> wgpu::Features {
        features()
    }
    fn limits(&self) -> wgpu::Limits {
        limits()
    }
    fn adapter_info(&self) -> wgpu::AdapterInfo {
        info()
    }
    fn create_shader_module(&self, desc: wgpu::ShaderModuleDescriptor<'_>, _: wgpu::ShaderRuntimeChecks) -> DispatchShaderModule {
        let source: Arc<str> = match desc.source {
            wgpu::ShaderSource::Wgsl(s) => s.into(),
            _ => unsupported("non-WGSL shader modules"),
        };
        let hash = library::hash(&source);
        DispatchShaderModule::custom(ShaderModule { source, hash })
    }
    unsafe fn create_shader_module_passthrough(&self, _: &wgpu::ShaderModuleDescriptorPassthrough<'_>) -> DispatchShaderModule {
        unsupported("passthrough shaders")
    }
    fn create_bind_group_layout(&self, desc: &wgpu::BindGroupLayoutDescriptor<'_>) -> DispatchBindGroupLayout {
        let entries = desc
            .entries
            .iter()
            .map(|e| {
                let dynamic = matches!(e.ty, wgpu::BindingType::Buffer { has_dynamic_offset: true, .. });
                (e.binding, dynamic)
            })
            .collect();
        DispatchBindGroupLayout::custom(BindGroupLayout(Arc::new(entries)))
    }
    fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor<'_>) -> DispatchBindGroup {
        let layout = bgl(desc.layout);
        let mut entries = BTreeMap::new();
        for e in desc.entries {
            let r = match &e.resource {
                wgpu::BindingResource::Buffer(b) => Res::Buffer(api_buf(b.buffer), b.offset, b.size.map(|s| s.get())),
                wgpu::BindingResource::TextureView(v) => Res::View(api_view(v)),
                wgpu::BindingResource::Sampler(s) => Res::Sampler(s.as_custom::<Sampler>().expect("deko3d sampler").0.clone()),
                _ => unsupported("binding arrays, acceleration structures and external textures"),
            };
            entries.insert(e.binding, r);
        }
        let mut dynamic: Vec<u32> = layout.0.iter().filter(|(_, d)| *d).map(|(b, _)| *b).collect();
        dynamic.sort_unstable();
        DispatchBindGroup::custom(BindGroup(Arc::new(BgInner { entries, dynamic })))
    }
    fn create_pipeline_layout(&self, desc: &wgpu::PipelineLayoutDescriptor<'_>) -> DispatchPipelineLayout {
        let layouts = desc.bind_group_layouts.iter().map(|l| l.map(bgl).unwrap_or_default()).collect();
        DispatchPipelineLayout::custom(PipelineLayout(Arc::new(layouts)))
    }
    fn create_render_pipeline(&self, desc: &wgpu::RenderPipelineDescriptor<'_>) -> DispatchRenderPipeline {
        let vs = self.program(desc.vertex.module, desc.vertex.entry_point, Stage::Vertex);
        let fs = desc.fragment.as_ref().map(|f| self.program(f.module, f.entry_point, Stage::Fragment));
        let mut s = driver::Pipeline { vs: vs.shader, fs: fs.as_ref().map_or(0, |f| f.shader), ..Default::default() };
        let p = &desc.primitive;
        s.cull = match p.cull_mode {
            None => 0,
            Some(wgpu::Face::Front) => 1,
            Some(wgpu::Face::Back) => 2,
        };
        s.front_face = u32::from(p.front_face == wgpu::FrontFace::Cw);
        s.unclipped_depth = u32::from(p.unclipped_depth);
        let prim = match p.topology {
            wgpu::PrimitiveTopology::PointList => 0,
            wgpu::PrimitiveTopology::LineList => 1,
            wgpu::PrimitiveTopology::LineStrip => 2,
            wgpu::PrimitiveTopology::TriangleList => 3,
            wgpu::PrimitiveTopology::TriangleStrip => 4,
        };
        if let Some(f) = &desc.fragment {
            s.color_count = f.targets.len() as u32;
            for (i, t) in f.targets.iter().enumerate().take(8) {
                let Some(t) = t else { continue };
                let b = &mut s.blends[i];
                b.write_mask = t.write_mask.bits() as u8;
                if let Some(bl) = t.blend {
                    *b = driver::Blend {
                        enable: 1,
                        color_op: blend_op(bl.color.operation),
                        color_src: blend_factor(bl.color.src_factor),
                        color_dst: blend_factor(bl.color.dst_factor),
                        alpha_op: blend_op(bl.alpha.operation),
                        alpha_src: blend_factor(bl.alpha.src_factor),
                        alpha_dst: blend_factor(bl.alpha.dst_factor),
                        write_mask: t.write_mask.bits() as u8,
                    };
                }
            }
        }
        if let Some(d) = &desc.depth_stencil {
            let compare_fn = d.depth_compare.unwrap_or(wgpu::CompareFunction::Always);
            s.depth_test = u8::from(compare_fn != wgpu::CompareFunction::Always || d.depth_write_enabled == Some(true));
            s.depth_write = u8::from(d.depth_write_enabled == Some(true));
            s.depth_compare = compare(compare_fn);
            s.stencil_enable = u8::from(d.stencil.is_enabled());
            s.stencil_front = stencil_face(&d.stencil.front);
            s.stencil_back = stencil_face(&d.stencil.back);
            s.stencil_read_mask = d.stencil.read_mask as u8;
            s.stencil_write_mask = d.stencil.write_mask as u8;
            s.depth_bias = d.bias.constant;
            s.depth_bias_slope = d.bias.slope_scale;
            s.depth_bias_clamp = d.bias.clamp;
        }
        s.samples = desc.multisample.count;
        s.alpha_to_coverage = u8::from(desc.multisample.alpha_to_coverage_enabled);
        let mut n = 0usize;
        for (bi, b) in desc.vertex.buffers.iter().enumerate().take(16) {
            let Some(b) = b else { continue };
            s.strides[bi] = b.array_stride as u32;
            s.instanced[bi] = u32::from(b.step_mode == wgpu::VertexStepMode::Instance);
            for a in b.attributes {
                assert!(n < 16, "deko3d backend: more than 16 vertex attributes");
                let (ty, bits, components) = vertex_format(a.format);
                s.attribs[n] = driver::Attrib { location: a.shader_location, buffer: bi as u32, offset: a.offset as u32, ty, bits, components };
                n += 1;
            }
        }
        s.buffer_count = desc.vertex.buffers.len().min(16) as u32;
        s.attrib_count = n as u32;
        let layout = desc
            .layout
            .map(|l| l.as_custom::<PipelineLayout>().expect("deko3d pipeline layout").0.as_ref().clone())
            .unwrap_or_default();
        DispatchRenderPipeline::custom(RenderPipeline(Arc::new(RpInner { vs, fs, state: s, prim, layout })))
    }
    fn create_mesh_pipeline(&self, _: &wgpu::MeshPipelineDescriptor<'_>) -> DispatchRenderPipeline {
        unsupported("mesh shaders")
    }
    fn create_compute_pipeline(&self, desc: &wgpu::ComputePipelineDescriptor<'_>) -> DispatchComputePipeline {
        let cs = self.program(desc.module, desc.entry_point, Stage::Compute);
        let layout = desc
            .layout
            .map(|l| l.as_custom::<PipelineLayout>().expect("deko3d pipeline layout").0.as_ref().clone())
            .unwrap_or_default();
        DispatchComputePipeline::custom(ComputePipeline(Arc::new(CpInner { cs, layout })))
    }
    unsafe fn create_pipeline_cache(&self, _: &wgpu::PipelineCacheDescriptor<'_>) -> DispatchPipelineCache {
        unsupported("pipeline caches")
    }
    fn create_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> DispatchBuffer {
        let read = desc.usage.contains(wgpu::BufferUsages::MAP_READ);
        let size = desc.size.max(4).next_multiple_of(256);
        let d = &self.0.driver;
        let mem = d.mem_create(size, if read { driver::MEM_READBACK } else { driver::MEM_UPLOAD });
        assert!(mem > 0, "deko3d backend: buffer allocation of {size} bytes failed");
        DispatchBuffer::custom(Buffer(Arc::new(BufInner {
            shared: self.0.clone(),
            mem,
            cpu: d.mem_cpu(mem) as usize,
            gpu: d.mem_gpu(mem),
            size: desc.size,
        })))
    }
    fn create_texture(&self, desc: &wgpu::TextureDescriptor<'_>) -> DispatchTexture {
        let mut inner = TexInner {
            shared: self.0.clone(),
            image: 0,
            owned: true,
            format: desc.format,
            size: desc.size,
            mips: desc.mip_level_count,
            samples: desc.sample_count,
            dimension: desc.dimension,
        };
        let mut flags = 0;
        if desc.usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
            flags |= driver::IMAGE_RENDER;
        }
        if desc.usage.contains(wgpu::TextureUsages::STORAGE_BINDING) {
            flags |= driver::IMAGE_STORAGE;
        }
        if desc.usage.intersects(wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST) && !desc.format.is_compressed() {
            flags |= driver::IMAGE_COPY;
        }
        let t = image_type(&inner);
        inner.image = self.0.driver.image_create(
            cs_format(desc.format),
            t,
            desc.size.width,
            desc.size.height,
            desc.size.depth_or_array_layers,
            desc.mip_level_count,
            desc.sample_count,
            flags,
        );
        assert!(inner.image > 0, "deko3d backend: texture {:?} {:?} creation failed", desc.format, desc.size);
        DispatchTexture::custom(Texture(Arc::new(inner)))
    }
    fn create_external_texture(&self, _: &wgpu::ExternalTextureDescriptor<'_>, _: &[&wgpu::TextureView]) -> DispatchExternalTexture {
        unsupported("external textures")
    }
    fn create_blas(&self, _: &wgpu::CreateBlasDescriptor<'_>, _: wgpu::BlasGeometrySizeDescriptors) -> (Option<u64>, DispatchBlas) {
        unsupported("ray tracing acceleration structures")
    }
    fn create_tlas(&self, _: &wgpu::CreateTlasDescriptor<'_>) -> DispatchTlas {
        unsupported("ray tracing acceleration structures")
    }
    fn create_sampler(&self, desc: &wgpu::SamplerDescriptor<'_>) -> DispatchSampler {
        let wrap = |a: wgpu::AddressMode| match a {
            wgpu::AddressMode::ClampToEdge => 0,
            wgpu::AddressMode::Repeat => 1,
            wgpu::AddressMode::MirrorRepeat => 2,
            wgpu::AddressMode::ClampToBorder => 3,
        };
        let filter = |f: wgpu::FilterMode| u32::from(f == wgpu::FilterMode::Linear);
        let mip = u32::from(desc.mipmap_filter == wgpu::MipmapFilterMode::Linear);
        let border = match desc.border_color {
            Some(wgpu::SamplerBorderColor::OpaqueBlack) => [0., 0., 0., 1.],
            Some(wgpu::SamplerBorderColor::OpaqueWhite) => [1.; 4],
            _ => [0.; 4],
        };
        let s = driver::Sampler {
            min_filter: filter(desc.min_filter),
            mag_filter: filter(desc.mag_filter),
            mip_filter: mip,
            wrap: [wrap(desc.address_mode_u), wrap(desc.address_mode_v), wrap(desc.address_mode_w)],
            lod_min: desc.lod_min_clamp,
            lod_max: desc.lod_max_clamp,
            compare_enable: u32::from(desc.compare.is_some()),
            compare: desc.compare.map_or(0, |c| u32::from(compare(c))),
            anisotropy: f32::from(desc.anisotropy_clamp),
            border,
        };
        let slot = self.0.sampler_slots.lock().unwrap().take(SAMPLER_SLOTS);
        assert!(self.0.driver.sampler_descriptor(slot, &s), "deko3d backend: sampler descriptor");
        DispatchSampler::custom(Sampler(Arc::new(SamplerInner { shared: self.0.clone(), slot })))
    }
    fn create_query_set(&self, _: &wgpu::QuerySetDescriptor<'_>) -> DispatchQuerySet {
        unsupported("query sets (timestamps are not advertised)")
    }
    fn create_command_encoder(&self, _: &wgpu::CommandEncoderDescriptor<'_>) -> DispatchCommandEncoder {
        DispatchCommandEncoder::custom(CommandEncoder(Arc::default()))
    }
    fn create_render_bundle_encoder(&self, _: &wgpu::RenderBundleEncoderDescriptor<'_>) -> DispatchRenderBundleEncoder {
        unsupported("render bundles")
    }
    fn set_device_lost_callback(&self, _: BoxDeviceLostCallback) {}
    fn on_uncaptured_error(&self, _: Arc<dyn wgpu::UncapturedErrorHandler>) {}
    fn push_error_scope(&self, _: wgpu::ErrorFilter) -> u32 {
        0
    }
    fn pop_error_scope(&self, _: u32) -> Pin<Box<dyn PopErrorScopeFuture>> {
        Box::pin(ready(None))
    }
    unsafe fn start_graphics_debugger_capture(&self) {}
    unsafe fn stop_graphics_debugger_capture(&self) {}
    fn poll(&self, poll: wgpu::wgt::PollType<u64>) -> Result<wgpu::PollStatus, wgpu::PollError> {
        if let wgpu::wgt::PollType::Wait { submission_index, .. } = poll {
            let seq = submission_index.unwrap_or_else(|| self.0.last_seq.load(Ordering::Acquire));
            self.0.driver.wait(seq);
            self.0.reclaim();
            return Ok(if self.0.idle() { wgpu::PollStatus::QueueEmpty } else { wgpu::PollStatus::WaitSucceeded });
        }
        self.0.reclaim();
        Ok(if self.0.idle() { wgpu::PollStatus::QueueEmpty } else { wgpu::PollStatus::Poll })
    }
    fn get_internal_counters(&self) -> wgpu::InternalCounters {
        Default::default()
    }
    fn generate_allocator_report(&self) -> Option<wgpu::AllocatorReport> {
        None
    }
    fn destroy(&self) {}
}

// ---------------------------------------------------------------- queue

fn block_layout(f: wgpu::TextureFormat, stencil: bool) -> (u32, u32, u32) {
    let (bw, bh) = f.block_dimensions();
    let aspect = if stencil { Some(wgpu::TextureAspect::StencilOnly) } else if f.is_depth_stencil_format() { Some(wgpu::TextureAspect::DepthOnly) } else { None };
    let size = f.block_copy_size(aspect).unwrap_or(4);
    (bw, bh, size)
}
fn copy3(info: &wgpu::TexelCopyTextureInfo<'_>, extent: wgpu::Extent3d) -> Copy3 {
    Copy3 {
        origin: [info.origin.x, info.origin.y, info.origin.z],
        extent: [extent.width, extent.height, extent.depth_or_array_layers],
        mip: info.mip_level,
        aspect_stencil: info.aspect == wgpu::TextureAspect::StencilOnly,
    }
}

impl QueueInterface for Queue {
    fn write_buffer(&self, buffer: &DispatchBuffer, offset: u64, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let src = self.0.stage(data);
        self.0.pending.lock().unwrap().push(Cmd::StagedToBuffer { src, dst: buf(buffer), d: offset, size: data.len() as u64 });
    }
    fn create_staging_buffer(&self, size: wgpu::BufferSize) -> Option<DispatchQueueWriteBuffer> {
        Some(DispatchQueueWriteBuffer::custom(StagingWrite(vec![0; size.get() as usize])))
    }
    fn validate_write_buffer(&self, _: &DispatchBuffer, _: u64, _: wgpu::BufferSize) -> Option<()> {
        Some(())
    }
    fn write_staging_buffer(&self, buffer: &DispatchBuffer, offset: u64, staging: &DispatchQueueWriteBuffer) {
        let data = &staging.as_custom::<StagingWrite>().expect("deko3d staging").0;
        self.write_buffer(buffer, offset, data);
    }
    fn write_texture(&self, texture: wgpu::TexelCopyTextureInfo<'_>, data: &[u8], layout: wgpu::TexelCopyBufferLayout, size: wgpu::Extent3d) {
        let dst = api_tex(texture.texture);
        let region = copy3(&texture, size);
        let (bw, bh, bs) = block_layout(dst.format, region.aspect_stencil);
        let bpr = layout.bytes_per_row.unwrap_or(size.width.div_ceil(bw) * bs);
        let rows = layout.rows_per_image.unwrap_or(size.height.div_ceil(bh));
        let bytes = &data[layout.offset as usize..];
        let needed = (bpr as u64 * rows as u64 * size.depth_or_array_layers.max(1) as u64).min(bytes.len() as u64) as usize;
        let src = self.0.stage(&bytes[..needed]);
        self.0.pending.lock().unwrap().push(Cmd::BufferToTexture { src, keep: None, bytes_per_row: bpr, rows, dst, region });
    }
    fn submit(&self, buffers: &mut dyn Iterator<Item = DispatchCommandBuffer>) -> u64 {
        let _guard = self.0.submit_lock.lock().unwrap();
        let mut lists = vec![std::mem::take(&mut *self.0.pending.lock().unwrap())];
        for b in buffers {
            let cb = b.as_custom::<CommandBuffer>().expect("deko3d command buffer");
            lists.push(std::mem::take(&mut *cb.0.lock().unwrap()));
        }
        let d = &*self.0.driver;
        d.begin();
        let mut ex = Executor::new(d);
        for list in &lists {
            for c in list {
                ex.run(c);
            }
        }
        let seq = d.submit();
        self.0.last_seq.store(seq, Ordering::Release);
        for c in self.0.staging.lock().unwrap().iter_mut() {
            if c.used > 0 && c.retired.is_none() {
                c.retired = Some(seq);
            }
        }
        drop(ex);
        drop(lists);
        self.0.reclaim();
        seq
    }
    fn get_timestamp_period(&self) -> f32 {
        1.0
    }
    fn on_submitted_work_done(&self, callback: BoxSubmittedWorkDoneCallback) {
        let seq = self.0.last_seq.load(Ordering::Acquire);
        self.0.done_callbacks.lock().unwrap().push((seq, callback));
    }
    fn compact_blas(&self, _: &DispatchBlas) -> (Option<u64>, DispatchBlas) {
        unsupported("ray tracing acceleration structures")
    }
    fn present(&self, _: &DispatchSurfaceOutputDetail) {
        unsupported("surfaces (use Display::present)")
    }
}

#[derive(Debug)]
pub struct StagingWrite(Vec<u8>);
impl QueueWriteBufferInterface for StagingWrite {
    fn len(&self) -> usize {
        self.0.len()
    }
    unsafe fn write_slice(&mut self) -> wgpu::WriteOnly<'_, [u8]> {
        wgpu::WriteOnly::from_mut(self.0.as_mut_slice())
    }
}

// ---------------------------------------------------------------- execution

struct Executor<'a> {
    d: &'a dyn Driver,
}
impl<'a> Executor<'a> {
    fn new(d: &'a dyn Driver) -> Self {
        Self { d }
    }
    fn run(&mut self, c: &Cmd) {
        let d = self.d;
        match c {
            Cmd::CopyBuffer { src, so, dst, d: dof, size } => {
                d.copy_buffer(src.gpu + so, dst.gpu + dof, *size);
                d.barrier();
            }
            Cmd::StagedToBuffer { src, dst, d: dof, size } => {
                d.copy_buffer(*src, dst.gpu + dof, *size);
                d.barrier();
            }
            Cmd::ClearBuffer { dst, offset, size } => {
                // No fill engine: copy from zeroed staging.
                let zeros = vec![0u8; *size as usize];
                let src = dst.shared.stage(&zeros);
                d.copy_buffer(src, dst.gpu + offset, *size);
                d.barrier();
            }
            Cmd::BufferToTexture { src, keep, bytes_per_row, rows, dst, region } => {
                let src = keep.as_ref().map_or(*src, |b| b.gpu + src);
                Self::buffer_image(d, src, *bytes_per_row, *rows, dst, region, true);
                d.barrier();
            }
            Cmd::TextureToBuffer { src, region, dst, offset, bytes_per_row, rows } => {
                d.barrier();
                Self::buffer_image(d, dst.gpu + offset, *bytes_per_row, *rows, src, region, false);
                d.barrier();
            }
            Cmd::TextureToTexture { src, sr, dst, dr } => {
                d.barrier();
                let d3 = src.dimension == wgpu::TextureDimension::D3;
                let layers = if d3 { 1 } else { sr.extent[2].max(1) };
                for l in 0..layers {
                    let a = region_view(src, sr, sr.origin[2] + l);
                    let b = region_view(dst, dr, dr.origin[2] + l);
                    let (sz, dz, depth) = if d3 { (sr.origin[2], dr.origin[2], sr.extent[2]) } else { (0, 0, 1) };
                    d.copy_image(&a, [sr.origin[0], sr.origin[1], sz], &b, [dr.origin[0], dr.origin[1], dz], [sr.extent[0], sr.extent[1], depth]);
                }
                d.barrier();
            }
            Cmd::Render(desc, cmds) => self.render(desc, cmds),
            Cmd::Compute(cmds) => self.compute(cmds),
        }
    }
    fn buffer_image(d: &dyn Driver, gpu: u64, bpr: u32, rows: u32, t: &TexInner, r: &Copy3, upload: bool) {
        // DkCopyBuf.rowLength is the row pitch in bytes (verified on
        // Ryujinx: a pixel count shrank uploads to a quarter of their rows);
        // imageHeight is in texel rows.
        let (_, bh, _) = block_layout(t.format, r.aspect_stencil);
        let row_pixels = bpr;
        let image_rows = rows * bh;
        let d3 = t.dimension == wgpu::TextureDimension::D3;
        let layers = if d3 { 1 } else { r.extent[2].max(1) };
        let layer_bytes = bpr as u64 * rows as u64;
        for l in 0..layers {
            let v = region_view(t, r, r.origin[2] + l);
            let (z, depth) = if d3 { (r.origin[2], r.extent[2]) } else { (0, 1) };
            let addr = gpu + layer_bytes * l as u64;
            if upload {
                d.copy_buffer_to_image(addr, row_pixels, image_rows, &v, [r.origin[0], r.origin[1], z], [r.extent[0], r.extent[1], depth]);
            } else {
                d.copy_image_to_buffer(&v, [r.origin[0], r.origin[1], z], [r.extent[0], r.extent[1], depth], addr, row_pixels, image_rows);
            }
        }
    }
    fn bind(d: &dyn Driver, stage: u32, p: &Program, groups: &[Option<(Arc<BgInner>, Vec<u32>)>; 8]) {
        let find = |g: u32, b: u32| -> Option<(&Res, u64)> {
            let (bg, offsets) = groups.get(g as usize)?.as_ref()?;
            let r = bg.entries.get(&b)?;
            let dynamic = bg.dynamic.iter().position(|x| *x == b).and_then(|i| offsets.get(i)).copied().unwrap_or(0);
            Some((r, u64::from(dynamic)))
        };
        let buffer = |g: u32, b: u32| -> Option<(u64, u32)> {
            match find(g, b)? {
                (Res::Buffer(buf, offset, size), dynamic) => {
                    let start = offset + dynamic;
                    let len = size.unwrap_or(buf.size.saturating_sub(start));
                    Some((buf.gpu + start, len as u32))
                }
                _ => None,
            }
        };
        let r = &p.reflection;
        for &(g, b, slot) in &r.uniforms {
            if let Some((gpu, size)) = buffer(g, b) {
                d.uniform(stage, slot, gpu, size.min(64 * 1024));
            }
        }
        for &(g, b, slot) in &r.storage {
            if let Some((gpu, size)) = buffer(g, b) {
                d.storage(stage, slot, gpu, size);
            }
        }
        for &(slot, tg, tb, sg, sb) in &r.textures {
            let Some((Res::View(v), _)) = find(tg, tb) else { continue };
            let sampler = if sg == u32::MAX {
                0
            } else {
                match find(sg, sb) {
                    Some((Res::Sampler(s), _)) => s.slot,
                    _ => 0,
                }
            };
            if let Some(image) = v.sampled {
                d.texture(stage, slot, image, sampler);
            }
        }
        for &(g, b, slot) in &r.images {
            if let Some((Res::View(v), _)) = find(g, b) {
                if let Some(image) = v.storage {
                    d.image(stage, slot, image);
                }
            }
        }
    }
    fn render(&mut self, desc: &PassDesc, cmds: &[RCmd]) {
        let d = self.d;
        d.barrier();
        let colors: Vec<View> = desc.colors.iter().map(|(v, _, _)| v.view).collect();
        let depth = desc.depth.as_ref().map(|(v, _, _)| v.view);
        d.targets(&colors, depth.as_ref());
        let size = desc.colors.first().map(|c| c.0.size).or_else(|| desc.depth.as_ref().map(|z| z.0.size)).unwrap_or([1, 1]);
        d.scissor(0, 0, size[0], size[1]);
        for (i, (_, _, clear)) in desc.colors.iter().enumerate() {
            if let Some(c) = clear {
                d.clear_color(i as u32, *c);
            }
        }
        if let Some((_, dc, sc)) = &desc.depth {
            if dc.is_some() || sc.is_some() {
                d.clear_depth_stencil(dc.is_some(), dc.unwrap_or(1.0), sc.is_some(), sc.unwrap_or(0));
            }
        }
        d.viewport(0., 0., size[0] as f32, size[1] as f32, 0., 1.);
        let mut pipeline: Option<Arc<RpInner>> = None;
        let mut groups: [Option<(Arc<BgInner>, Vec<u32>)>; 8] = Default::default();
        let mut vertex: [Option<(Arc<BufInner>, u64, Option<u64>)>; 16] = Default::default();
        let mut index: Option<(Arc<BufInner>, bool, u64)> = None;
        let mut stencil_ref = 0u32;
        let flush = |pipeline: &Option<Arc<RpInner>>, groups: &[Option<(Arc<BgInner>, Vec<u32>)>; 8], vertex: &[Option<(Arc<BufInner>, u64, Option<u64>)>; 16], index: &Option<(Arc<BufInner>, bool, u64)>| -> Option<u32> {
            let p = pipeline.as_ref()?;
            Self::bind(d, driver::STAGE_VERTEX, &p.vs, groups);
            if let Some(fs) = &p.fs {
                Self::bind(d, driver::STAGE_FRAGMENT, fs, groups);
            }
            for (slot, v) in vertex.iter().enumerate() {
                if let Some((b, off, size)) = v {
                    let len = size.unwrap_or(b.size.saturating_sub(*off));
                    d.vertex_buffer(slot as u32, b.gpu + off, len as u32);
                }
            }
            if let Some((b, u32i, off)) = index {
                d.index_buffer(b.gpu + off, *u32i);
            }
            Some(p.prim)
        };
        for c in cmds {
            match c {
                RCmd::Pipeline(p) => {
                    d.bind_graphics(&p.state);
                    let s = &p.state;
                    d.stencil_reference(stencil_ref as u8, s.stencil_read_mask, s.stencil_write_mask);
                    pipeline = Some(p.clone());
                }
                RCmd::Group(i, g, offsets) => {
                    if let Some(slot) = groups.get_mut(*i as usize) {
                        *slot = g.clone().map(|g| (g, offsets.clone()));
                    }
                }
                RCmd::Index(b, u32i, off) => index = Some((b.clone(), *u32i, *off)),
                RCmd::Vertex(slot, b, off, size) => {
                    if let Some(v) = vertex.get_mut(*slot as usize) {
                        *v = b.clone().map(|b| (b, *off, *size));
                    }
                }
                RCmd::Viewport(v) => d.viewport(v[0], v[1], v[2], v[3], v[4], v[5]),
                RCmd::Scissor(s) => d.scissor(s[0], s[1], s[2], s[3]),
                RCmd::StencilRef(r) => {
                    stencil_ref = *r;
                    if let Some(p) = &pipeline {
                        d.stencil_reference(*r as u8, p.state.stencil_read_mask, p.state.stencil_write_mask);
                    }
                }
                RCmd::BlendConstant(c) => d.blend_constant(*c),
                RCmd::Draw(v, i) => {
                    if let Some(prim) = flush(&pipeline, &groups, &vertex, &index) {
                        d.draw(prim, v.end - v.start, i.end - i.start, v.start, i.start);
                    }
                }
                RCmd::DrawIndexed(ix, base, i) => {
                    if let Some(prim) = flush(&pipeline, &groups, &vertex, &index) {
                        d.draw_indexed(prim, ix.end - ix.start, i.end - i.start, ix.start, *base, i.start);
                    }
                }
                RCmd::DrawIndirect(b, off, indexed, count) => {
                    if let Some(prim) = flush(&pipeline, &groups, &vertex, &index) {
                        let stride = if *indexed { 20 } else { 16 };
                        for k in 0..*count {
                            d.draw_indirect(prim, b.gpu + off + stride * k as u64, *indexed);
                        }
                    }
                }
            }
        }
        for (v, resolve, _) in &desc.colors {
            if let Some(r) = resolve {
                d.resolve(&v.view, &r.view);
            }
        }
        d.barrier();
    }
    fn compute(&mut self, cmds: &[CCmd]) {
        let d = self.d;
        let mut pipeline: Option<Arc<CpInner>> = None;
        let mut groups: [Option<(Arc<BgInner>, Vec<u32>)>; 8] = Default::default();
        for c in cmds {
            match c {
                CCmd::Pipeline(p) => {
                    d.bind_compute(p.cs.shader);
                    pipeline = Some(p.clone());
                }
                CCmd::Group(i, g, offsets) => {
                    if let Some(slot) = groups.get_mut(*i as usize) {
                        *slot = g.clone().map(|g| (g, offsets.clone()));
                    }
                }
                CCmd::Dispatch(x, y, z) => {
                    if let Some(p) = &pipeline {
                        d.barrier();
                        Self::bind(d, driver::STAGE_COMPUTE, &p.cs, &groups);
                        d.dispatch(*x, *y, *z);
                    }
                }
                CCmd::DispatchIndirect(b, off) => {
                    if let Some(p) = &pipeline {
                        d.barrier();
                        Self::bind(d, driver::STAGE_COMPUTE, &p.cs, &groups);
                        d.dispatch_indirect(b.gpu + off);
                    }
                }
            }
        }
        d.barrier();
    }
}

// ---------------------------------------------------------------- interfaces

impl ShaderModuleInterface for ShaderModule {
    fn get_compilation_info(&self) -> Pin<Box<dyn ShaderCompilationInfoFuture>> {
        Box::pin(ready(wgpu::CompilationInfo { messages: Vec::new() }))
    }
}
impl BindGroupLayoutInterface for BindGroupLayout {}
impl BindGroupInterface for BindGroup {}
impl TextureViewInterface for TextureView {}
impl SamplerInterface for Sampler {}
impl PipelineLayoutInterface for PipelineLayout {}
impl CommandBufferInterface for CommandBuffer {}
impl RenderPipelineInterface for RenderPipeline {
    fn get_bind_group_layout(&self, index: u32) -> DispatchBindGroupLayout {
        DispatchBindGroupLayout::custom(self.0.layout.get(index as usize).cloned().unwrap_or_default())
    }
}
impl ComputePipelineInterface for ComputePipeline {
    fn get_bind_group_layout(&self, index: u32) -> DispatchBindGroupLayout {
        DispatchBindGroupLayout::custom(self.0.layout.get(index as usize).cloned().unwrap_or_default())
    }
}

#[derive(Debug)]
pub struct MappedRange {
    ptr: usize,
    len: usize,
}
impl BufferMappedRangeInterface for MappedRange {
    fn len(&self) -> usize {
        self.len
    }
    unsafe fn read_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
    unsafe fn write_slice(&mut self) -> wgpu::WriteOnly<'_, [u8]> {
        wgpu::WriteOnly::from_mut(unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.len) })
    }
}

impl BufferInterface for Buffer {
    fn map_async(&self, mode: wgpu::MapMode, range: Range<u64>, callback: BufferMapCallback) {
        let s = &self.0.shared;
        s.maps.lock().unwrap().push(MapRequest {
            after: s.last_seq.load(Ordering::Acquire),
            buffer: self.0.clone(),
            range,
            read: mode == wgpu::MapMode::Read,
            callback,
        });
    }
    fn get_mapped_range(&self, sub_range: Range<u64>) -> Result<DispatchBufferMappedRange, wgpu::MapRangeError> {
        Ok(DispatchBufferMappedRange::custom(MappedRange {
            ptr: self.0.cpu + sub_range.start as usize,
            len: (sub_range.end - sub_range.start) as usize,
        }))
    }
    fn unmap(&self) {}
    fn destroy(&self) {}
}

impl TextureInterface for Texture {
    fn create_view(&self, desc: &wgpu::TextureViewDescriptor<'_>) -> DispatchTextureView {
        let t = &self.0;
        let shared = &t.shared;
        let dim = desc.dimension.unwrap_or(match t.dimension {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            wgpu::TextureDimension::D2 if t.size.depth_or_array_layers > 1 => wgpu::TextureViewDimension::D2Array,
            wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
        });
        let format = desc.format.unwrap_or(t.format);
        let mips = desc.mip_level_count.unwrap_or(t.mips - desc.base_mip_level);
        let layers = desc.array_layer_count.unwrap_or(if t.dimension == wgpu::TextureDimension::D3 { 1 } else { t.size.depth_or_array_layers - desc.base_array_layer });
        let view = View {
            image: t.image,
            ty: view_type(dim, t.samples),
            format: if format == t.format { 0 } else { cs_format(format) },
            mip: desc.base_mip_level,
            mips,
            layer: desc.base_array_layer,
            layers,
            stencil: u32::from(desc.aspect == wgpu::TextureAspect::StencilOnly),
        };
        let usage = desc.usage.unwrap_or(wgpu::TextureUsages::all());
        let d = &shared.driver;
        let sampled = (usage.contains(wgpu::TextureUsages::TEXTURE_BINDING)).then(|| {
            let slot = shared.image_slots.lock().unwrap().take(IMAGE_SLOTS);
            assert!(d.image_descriptor(slot, &view, false), "deko3d backend: image descriptor");
            slot
        });
        let storage = usage.contains(wgpu::TextureUsages::STORAGE_BINDING).then(|| {
            let slot = shared.image_slots.lock().unwrap().take(IMAGE_SLOTS);
            assert!(d.image_descriptor(slot, &view, true), "deko3d backend: storage image descriptor");
            slot
        });
        let mip_size = |v: u32| (v >> desc.base_mip_level).max(1);
        DispatchTextureView::custom(TextureView(Arc::new(ViewInner {
            tex: t.clone(),
            view,
            sampled,
            storage,
            size: [mip_size(t.size.width), mip_size(t.size.height)],
        })))
    }
    fn destroy(&self) {}
}

impl CommandEncoderInterface for CommandEncoder {
    fn copy_buffer_to_buffer(&self, source: &DispatchBuffer, so: u64, destination: &DispatchBuffer, d: u64, size: Option<u64>) {
        let src = buf(source);
        let size = size.unwrap_or(src.size - so);
        self.0.lock().unwrap().push(Cmd::CopyBuffer { src, so, dst: buf(destination), d, size });
    }
    fn copy_buffer_to_texture(&self, source: wgpu::TexelCopyBufferInfo<'_>, destination: wgpu::TexelCopyTextureInfo<'_>, size: wgpu::Extent3d) {
        let dst = api_tex(destination.texture);
        let region = copy3(&destination, size);
        let (bw, bh, bs) = block_layout(dst.format, region.aspect_stencil);
        let bpr = source.layout.bytes_per_row.unwrap_or(size.width.div_ceil(bw) * bs);
        let rows = source.layout.rows_per_image.unwrap_or(size.height.div_ceil(bh));
        let src = api_buf(source.buffer);
        self.0.lock().unwrap().push(Cmd::BufferToTexture { src: source.layout.offset, keep: Some(src), bytes_per_row: bpr, rows, dst, region });
    }
    fn copy_texture_to_buffer(&self, source: wgpu::TexelCopyTextureInfo<'_>, destination: wgpu::TexelCopyBufferInfo<'_>, size: wgpu::Extent3d) {
        let src = api_tex(source.texture);
        let region = copy3(&source, size);
        let (bw, bh, bs) = block_layout(src.format, region.aspect_stencil);
        let bpr = destination.layout.bytes_per_row.unwrap_or(size.width.div_ceil(bw) * bs);
        let rows = destination.layout.rows_per_image.unwrap_or(size.height.div_ceil(bh));
        self.0.lock().unwrap().push(Cmd::TextureToBuffer {
            src,
            region,
            dst: api_buf(destination.buffer),
            offset: destination.layout.offset,
            bytes_per_row: bpr,
            rows,
        });
    }
    fn copy_texture_to_texture(&self, source: wgpu::TexelCopyTextureInfo<'_>, destination: wgpu::TexelCopyTextureInfo<'_>, size: wgpu::Extent3d) {
        let cmd = Cmd::TextureToTexture { src: api_tex(source.texture), sr: copy3(&source, size), dst: api_tex(destination.texture), dr: copy3(&destination, size) };
        self.0.lock().unwrap().push(cmd);
    }
    fn begin_compute_pass(&self, _: &wgpu::ComputePassDescriptor<'_>) -> DispatchComputePass {
        DispatchComputePass::custom(ComputePass { list: self.0.clone(), cmds: Vec::new() })
    }
    fn begin_render_pass(&self, desc: &wgpu::RenderPassDescriptor<'_>) -> DispatchRenderPass {
        let colors = desc
            .color_attachments
            .iter()
            .flatten()
            .map(|a| {
                let clear = match a.ops.load {
                    wgpu::LoadOp::Clear(c) => Some([c.r as f32, c.g as f32, c.b as f32, c.a as f32]),
                    _ => None,
                };
                (api_view(a.view), a.resolve_target.map(api_view), clear)
            })
            .collect();
        let depth = desc.depth_stencil_attachment.as_ref().map(|a| {
            let dc = a.depth_ops.as_ref().and_then(|o| match o.load {
                wgpu::LoadOp::Clear(v) => Some(v),
                _ => None,
            });
            let sc = a.stencil_ops.as_ref().and_then(|o| match o.load {
                wgpu::LoadOp::Clear(v) => Some(v as u8),
                _ => None,
            });
            (api_view(a.view), dc, sc)
        });
        DispatchRenderPass::custom(RenderPass { list: self.0.clone(), desc: Some(Box::new(PassDesc { colors, depth })), cmds: Vec::new() })
    }
    fn finish(&mut self) -> DispatchCommandBuffer {
        let cmds = std::mem::take(&mut *self.0.lock().unwrap());
        DispatchCommandBuffer::custom(CommandBuffer(Mutex::new(cmds)))
    }
    fn clear_texture(&self, _: &DispatchTexture, _: &wgpu::ImageSubresourceRange) {
        unsupported("clear_texture")
    }
    fn clear_buffer(&self, buffer: &DispatchBuffer, offset: u64, size: Option<u64>) {
        let dst = buf(buffer);
        let size = size.unwrap_or(dst.size - offset);
        self.0.lock().unwrap().push(Cmd::ClearBuffer { dst, offset, size });
    }
    fn insert_debug_marker(&self, _: &str) {}
    fn push_debug_group(&self, _: &str) {}
    fn pop_debug_group(&self) {}
    fn write_timestamp(&self, _: &DispatchQuerySet, _: u32) {
        unsupported("timestamps")
    }
    fn resolve_query_set(&self, _: &DispatchQuerySet, _: u32, _: u32, _: &DispatchBuffer, _: u64) {
        unsupported("query sets")
    }
    fn mark_acceleration_structures_built<'a>(&self, _: &mut dyn Iterator<Item = &'a wgpu::Blas>, _: &mut dyn Iterator<Item = &'a wgpu::Tlas>) {
        unsupported("ray tracing acceleration structures")
    }
    fn build_acceleration_structures<'a>(&self, _: &mut dyn Iterator<Item = &'a wgpu::BlasBuildEntry<'a>>, _: &mut dyn Iterator<Item = &'a wgpu::Tlas>) {
        unsupported("ray tracing acceleration structures")
    }
    fn transition_resources<'a>(
        &mut self,
        _: &mut dyn Iterator<Item = wgpu::wgt::BufferTransition<&'a DispatchBuffer>>,
        _: &mut dyn Iterator<Item = wgpu::wgt::TextureTransition<&'a DispatchTexture>>,
    ) {
    }
}

impl ComputePassInterface for ComputePass {
    fn set_pipeline(&mut self, pipeline: &DispatchComputePipeline) {
        let p = pipeline.as_custom::<ComputePipeline>().expect("deko3d compute pipeline").0.clone();
        self.cmds.push(CCmd::Pipeline(p));
    }
    fn set_bind_group(&mut self, index: u32, g: Option<&DispatchBindGroup>, offsets: &[u32]) {
        self.cmds.push(CCmd::Group(index, g.map(group), offsets.to_vec()));
    }
    fn set_immediates(&mut self, _: u32, _: &[u8]) {
        unsupported("immediates")
    }
    fn insert_debug_marker(&mut self, _: &str) {}
    fn push_debug_group(&mut self, _: &str) {}
    fn pop_debug_group(&mut self) {}
    fn write_timestamp(&mut self, _: &DispatchQuerySet, _: u32) {
        unsupported("timestamps")
    }
    fn begin_pipeline_statistics_query(&mut self, _: &DispatchQuerySet, _: u32) {
        unsupported("pipeline statistics")
    }
    fn end_pipeline_statistics_query(&mut self) {}
    fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        self.cmds.push(CCmd::Dispatch(x, y, z));
    }
    fn dispatch_workgroups_indirect(&mut self, b: &DispatchBuffer, off: u64) {
        self.cmds.push(CCmd::DispatchIndirect(buf(b), off));
    }
    fn transition_resources<'a>(
        &mut self,
        _: &mut dyn Iterator<Item = wgpu::wgt::BufferTransition<&'a DispatchBuffer>>,
        _: &mut dyn Iterator<Item = wgpu::wgt::TextureTransition<&'a DispatchTextureView>>,
    ) {
    }
}

impl RenderPassInterface for RenderPass {
    fn set_pipeline(&mut self, pipeline: &DispatchRenderPipeline) {
        let p = pipeline.as_custom::<RenderPipeline>().expect("deko3d render pipeline").0.clone();
        self.cmds.push(RCmd::Pipeline(p));
    }
    fn set_bind_group(&mut self, index: u32, g: Option<&DispatchBindGroup>, offsets: &[u32]) {
        self.cmds.push(RCmd::Group(index, g.map(group), offsets.to_vec()));
    }
    fn set_index_buffer(&mut self, b: &DispatchBuffer, f: wgpu::IndexFormat, off: u64, _: Option<wgpu::BufferSize>) {
        self.cmds.push(RCmd::Index(buf(b), f == wgpu::IndexFormat::Uint32, off));
    }
    fn set_vertex_buffer(&mut self, slot: u32, b: Option<&DispatchBuffer>, off: u64, size: Option<wgpu::BufferSize>) {
        self.cmds.push(RCmd::Vertex(slot, b.map(buf), off, size.map(|s| s.get())));
    }
    fn set_immediates(&mut self, _: u32, _: &[u8]) {
        unsupported("immediates")
    }
    fn set_blend_constant(&mut self, c: wgpu::Color) {
        self.cmds.push(RCmd::BlendConstant([c.r as f32, c.g as f32, c.b as f32, c.a as f32]));
    }
    fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.cmds.push(RCmd::Scissor([x, y, w, h]));
    }
    fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, a: f32, b: f32) {
        self.cmds.push(RCmd::Viewport([x, y, w, h, a, b]));
    }
    fn set_stencil_reference(&mut self, r: u32) {
        self.cmds.push(RCmd::StencilRef(r));
    }
    fn draw(&mut self, v: Range<u32>, i: Range<u32>) {
        self.cmds.push(RCmd::Draw(v, i));
    }
    fn draw_indexed(&mut self, ix: Range<u32>, base: i32, i: Range<u32>) {
        self.cmds.push(RCmd::DrawIndexed(ix, base, i));
    }
    fn draw_mesh_tasks(&mut self, _: u32, _: u32, _: u32) {
        unsupported("mesh shaders")
    }
    fn draw_indirect(&mut self, b: &DispatchBuffer, off: u64) {
        self.cmds.push(RCmd::DrawIndirect(buf(b), off, false, 1));
    }
    fn draw_indexed_indirect(&mut self, b: &DispatchBuffer, off: u64) {
        self.cmds.push(RCmd::DrawIndirect(buf(b), off, true, 1));
    }
    fn draw_mesh_tasks_indirect(&mut self, _: &DispatchBuffer, _: u64) {
        unsupported("mesh shaders")
    }
    fn multi_draw_indirect(&mut self, b: &DispatchBuffer, off: u64, count: u32) {
        self.cmds.push(RCmd::DrawIndirect(buf(b), off, false, count));
    }
    fn multi_draw_indexed_indirect(&mut self, b: &DispatchBuffer, off: u64, count: u32) {
        self.cmds.push(RCmd::DrawIndirect(buf(b), off, true, count));
    }
    fn multi_draw_indirect_count(&mut self, _: &DispatchBuffer, _: u64, _: &DispatchBuffer, _: u64, _: u32) {
        unsupported("indirect draw counts")
    }
    fn multi_draw_mesh_tasks_indirect(&mut self, _: &DispatchBuffer, _: u64, _: u32) {
        unsupported("mesh shaders")
    }
    fn multi_draw_indexed_indirect_count(&mut self, _: &DispatchBuffer, _: u64, _: &DispatchBuffer, _: u64, _: u32) {
        unsupported("indirect draw counts")
    }
    fn multi_draw_mesh_tasks_indirect_count(&mut self, _: &DispatchBuffer, _: u64, _: &DispatchBuffer, _: u64, _: u32) {
        unsupported("mesh shaders")
    }
    fn insert_debug_marker(&mut self, _: &str) {}
    fn push_debug_group(&mut self, _: &str) {}
    fn pop_debug_group(&mut self) {}
    fn write_timestamp(&mut self, _: &DispatchQuerySet, _: u32) {
        unsupported("timestamps")
    }
    fn begin_occlusion_query(&mut self, _: u32) {
        unsupported("occlusion queries")
    }
    fn end_occlusion_query(&mut self) {}
    fn begin_pipeline_statistics_query(&mut self, _: &DispatchQuerySet, _: u32) {
        unsupported("pipeline statistics")
    }
    fn end_pipeline_statistics_query(&mut self) {}
    fn execute_bundles(&mut self, _: &mut dyn Iterator<Item = &DispatchRenderBundle>) {
        unsupported("render bundles")
    }
}

/// Wrap a driver image (a display framebuffer) as a wgpu texture.
pub(crate) fn display_texture(shared: &Arc<Shared>, image: Id, size: [u32; 2]) -> Texture {
    Texture(Arc::new(TexInner {
        shared: shared.clone(),
        image,
        owned: false,
        format: wgpu::TextureFormat::Rgba8Unorm,
        size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
        mips: 1,
        samples: 1,
        dimension: wgpu::TextureDimension::D2,
    }))
}
