// SPDX-License-Identifier: MPL-2.0
//! Lite software rays (ABI v1, docs/CONSOLE_SHADERS.md): the reduced
//! fragment profile of `RenderingPath::ComputeRaytrace` for GPUs without
//! compute storage (PS Vita GXM). A crysirror-raytrace stackless BVH in two
//! RG32F textures; one primary ray, one directional shadow ray, ambient and
//! a sky gradient per pixel. The lighting constants equal the compute
//! profile (`raytrace.wgsl`): sun direction (0.45, 0.85, 0.3), ambient 0.18,
//! direct 0.82, background sky. Differences from the compute profile:
//! at most 1024 triangles, two-sided normals and an sRGB 8-bit target.
use crate::{geometry, Gpu};
use anyhow::{Result, ensure};
use bytemuck::{Pod, Zeroable};
use crysirror_runtime::model::Scene3d;
use wgpu::util::DeviceExt;

/// Output of the lite profile: linear colour stored sRGB-encoded.
pub(crate) const TARGET: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DATA_WIDTH: u32 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    nodes: [f32; 4],
    triangles: [f32; 4],
    eye: [f32; 4],
    corner: [f32; 4],
    dx: [f32; 4],
    dy: [f32; 4],
    light: [f32; 4],
    sun: [f32; 4],
    ambient: [f32; 4],
    sky_low: [f32; 4],
    sky_high: [f32; 4],
}

/// Whether `gpu` should trace through this profile instead of compute.
pub(crate) fn preferred(gpu: &Gpu) -> bool {
    let features = |f| gpu.adapter.get_texture_format_features(f).allowed_usages;
    !features(wgpu::TextureFormat::Rgba16Float).contains(wgpu::TextureUsages::STORAGE_BINDING)
}
/// Whether `gpu` can run this profile at all.
pub(crate) fn supported(gpu: &Gpu) -> bool {
    let f = |format| gpu.adapter.get_texture_format_features(format).allowed_usages;
    f(wgpu::TextureFormat::Rg32Float).contains(wgpu::TextureUsages::TEXTURE_BINDING)
        && f(TARGET).contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
}

pub(crate) struct RayLite {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    fullscreen: wgpu::Buffer,
}

impl RayLite {
    pub fn new(gpu: &Gpu) -> Self {
        let d = &gpu.device;
        let data = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lite rays"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                data(1),
                data(2),
            ],
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("lite rays"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lite rays"),
            source: wgpu::ShaderSource::Wgsl(crysirror_shaders::wgsl::RAY_LITE.into()),
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lite rays"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: TARGET, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        // Oversized fullscreen triangle: clip position, then top-left UV.
        let vertices: [f32; 12] = [-1., -1., 0., 1., 3., -1., 2., 1., -1., 3., 0., -1.];
        let fullscreen = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lite rays fullscreen"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self { pipeline, layout, fullscreen }
    }

    pub fn encode(&self, gpu: &Gpu, scene: &Scene3d, canvas: [u32; 2], background: [u8; 4], encoder: &mut wgpu::CommandEncoder) -> Result<wgpu::Texture> {
        ensure!(
            canvas.iter().all(|n| *n > 0 && *n <= gpu.device.limits().max_texture_dimension_2d),
            "lite ray target exceeds device dimensions"
        );
        let vertices = geometry::triangles(scene)?;
        let triangles: Vec<crysirror_raytrace::Triangle> = vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|v| crysirror_raytrace::Triangle {
                vertices: [v[0].position, v[1].position, v[2].position],
                color: [v[0].color[0], v[0].color[1], v[0].color[2]],
            })
            .collect();
        let packed = crysirror_raytrace::build(&triangles, DATA_WIDTH).map_err(|e| anyhow::anyhow!("lite ray scene: {e:?}"))?;
        let inverse = glam::Mat4::from_cols_array_2d(&geometry::camera(scene, canvas)).inverse();
        ensure!(inverse.is_finite(), "lite ray camera matrix is not invertible");
        let far = |x: f32, y: f32| {
            let p = inverse * glam::Vec4::new(x, y, 1., 1.);
            p.truncate() / p.w
        };
        let eye = glam::Vec3::from_array(scene.camera.eye);
        let corner = far(-1., 1.) - eye;
        let dx = far(1., 1.) - eye - corner;
        let dy = far(-1., -1.) - eye - corner;
        ensure!(corner.is_finite() && dx.is_finite() && dy.is_finite(), "lite ray camera exceeds supported f32 precision");
        let sky = geometry::tint(background);
        let reach = (corner.length() + dx.length() + dy.length()).max(1.0);
        let params = Params {
            nodes: [DATA_WIDTH as f32, packed.nodes().height() as f32, packed.node_count() as f32, 0.],
            triangles: [DATA_WIDTH as f32, packed.triangles().height() as f32, packed.triangle_count() as f32, 0.],
            eye: [eye.x, eye.y, eye.z, 1e-4 * reach.max(1.0).min(1e4)],
            corner: [corner.x, corner.y, corner.z, 0.],
            dx: [dx.x, dx.y, dx.z, 0.],
            dy: [dy.x, dy.y, dy.z, 0.],
            light: [0.45, 0.85, 0.3, reach * 2.0],
            sun: [0.82, 0.82, 0.82, 0.],
            ambient: [0.18, 0.18, 0.18, 0.],
            sky_low: [sky[0], sky[1], sky[2], 0.],
            sky_high: [sky[0], sky[1], sky[2], 0.],
        };
        let d = &gpu.device;
        let data = |label, texture: &crysirror_raytrace::DataTexture| -> wgpu::TextureView {
            let t = d.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: texture.width(), height: texture.height(), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rg32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            gpu.queue.write_texture(
                t.as_image_copy(),
                bytemuck::cast_slice(texture.texels()),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(texture.width() * 8), rows_per_image: Some(texture.height()) },
                wgpu::Extent3d { width: texture.width(), height: texture.height(), depth_or_array_layers: 1 },
            );
            t.create_view(&Default::default())
        };
        let nodes = data("lite ray nodes", packed.nodes());
        let tris = data("lite ray triangles", packed.triangles());
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("lite ray camera"), contents: bytemuck::bytes_of(&params), usage: wgpu::BufferUsages::UNIFORM });
        let bind = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lite rays"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&nodes) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tris) },
            ],
        });
        let texture = d.create_texture(&wgpu::TextureDescriptor {
            label: Some("lite ray colour"),
            size: wgpu::Extent3d { width: canvas[0], height: canvas[1], depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lite primary and shadow rays"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_vertex_buffer(0, self.fullscreen.slice(..));
            pass.draw(0..3, 0..1);
        }
        Ok(texture)
    }
}
