// SPDX-License-Identifier: MPL-2.0
//! wgpu custom backend over deko3d for the CrySirror Switch build.
//!
//! The shared renderer (`crysirror-wgpu`) runs unchanged: it receives an
//! ordinary `wgpu::Adapter/Device/Queue` whose implementation records
//! commands and replays them through [`driver::Driver`]. Shaders are the
//! renderer's own WGSL, compiled offline to DKSH (see [`library`]).
mod backend;
pub mod driver;
pub mod library;

use std::sync::Arc;

pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    display: Display,
}

/// The console display: framebuffers acquired and presented around one
/// rendered frame.
pub struct Display {
    shared: Arc<backend::Shared>,
}
pub struct Frame {
    pub texture: wgpu::Texture,
    pub index: u32,
}

impl Gpu {
    pub fn new(driver: Arc<dyn driver::Driver>, library: Arc<dyn library::ShaderLibrary>) -> Self {
        let shared = backend::Shared::new(driver, library);
        let adapter = wgpu::Adapter::from_custom(backend::Adapter(shared.clone()));
        let device = wgpu::Device::from_custom(backend::Device(shared.clone()));
        let queue = wgpu::Queue::from_custom(backend::Queue(shared.clone()));
        Self { adapter, device, queue, display: Display { shared } }
    }
    pub fn display(&self) -> &Display {
        &self.display
    }
}

impl Display {
    pub fn size(&self) -> [u32; 2] {
        self.shared.driver.display_size()
    }
    /// The next framebuffer as an RGBA8 texture (render through an sRGB view
    /// if the renderer outputs sRGB). None while the compositor holds all.
    pub fn acquire(&self) -> Option<Frame> {
        let index = self.shared.driver.acquire()?;
        let image = self.shared.driver.display_image(index);
        let size = self.size();
        let desc = wgpu::TextureDescriptor {
            label: Some("display"),
            size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        };
        let texture = wgpu::Texture::from_custom(backend::display_texture(&self.shared, image, size), &desc);
        Some(Frame { texture, index })
    }
    /// Present after the frame's rendering was submitted.
    pub fn present(&self, frame: Frame) {
        self.shared.driver.present(frame.index);
    }
}
