use std::sync::Arc;
use std::time::Duration;

use crate::native_vm::runtime::RuntimeStatus;

pub mod blend;

#[derive(Clone, Debug)]
pub struct FrameScene {
    pub clear_color: [f64; 4],
    pub diagnostic_label: String,
    pub logical_width: u32,
    pub logical_height: u32,
    pub textures: Vec<SceneTexture>,
    pub commands: Vec<DrawCommand>,
    pub raster_stage: Option<RasterStage>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterStage {
    pub priority: i32,
    pub uniforms: crate::native_vm::fx::RasterUniforms,
}

impl FrameScene {
    pub const PAL_DEFAULT_WIDTH: u32 = 1280;
    pub const PAL_DEFAULT_HEIGHT: u32 = 720;

    pub fn boot() -> Self {
        Self {
            clear_color: [0.02, 0.02, 0.025, 1.0],
            diagnostic_label: "boot".to_owned(),
            logical_width: Self::PAL_DEFAULT_WIDTH,
            logical_height: Self::PAL_DEFAULT_HEIGHT,
            textures: Vec::new(),
            commands: Vec::new(),
            raster_stage: None,
        }
    }

    pub fn from_runtime_status(status: &RuntimeStatus, elapsed: Duration) -> Self {
        let pulse = ((elapsed.as_secs_f64() * 2.0).sin() * 0.5) + 0.5;
        let clear_color = match status {
            RuntimeStatus::NotBooted => [0.02, 0.02, 0.025, 1.0],
            RuntimeStatus::Running { .. } => [0.015, 0.020 + pulse * 0.010, 0.045, 1.0],
            RuntimeStatus::WaitFrame { .. } => [0.020, 0.030, 0.055, 1.0],
            RuntimeStatus::WaitClick { .. } => [0.035 + pulse * 0.015, 0.020, 0.050, 1.0],
            RuntimeStatus::Halted { .. } => [0.015, 0.045, 0.020, 1.0],
            RuntimeStatus::UnsupportedCommand { .. } | RuntimeStatus::UnsupportedExtCall { .. } => {
                [0.065, 0.040 + pulse * 0.015, 0.015, 1.0]
            }
            RuntimeStatus::Faulted { .. } => [0.070, 0.010, 0.010, 1.0],
        };
        Self {
            clear_color,
            diagnostic_label: status.to_string(),
            logical_width: Self::PAL_DEFAULT_WIDTH,
            logical_height: Self::PAL_DEFAULT_HEIGHT,
            textures: Vec::new(),
            commands: Vec::new(),
            raster_stage: None,
        }
    }

    pub fn with_logical_size(mut self, width: u32, height: u32) -> Self {
        self.logical_width = width.max(1);
        self.logical_height = height.max(1);
        self
    }

    pub fn with_texture(mut self, texture: SceneTexture) -> Self {
        self.textures.push(texture);
        self
    }

    pub fn with_command(mut self, command: DrawCommand) -> Self {
        self.commands.push(command);
        self
    }
}

impl Default for FrameScene {
    fn default() -> Self {
        Self::boot()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SceneTextureId(pub u64);

#[derive(Clone, Debug)]
pub struct SceneTexture {
    pub id: SceneTextureId,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub format: SceneTextureFormat,
    // Share ownership of the decoder allocation without copying its pixels.
    pub pixels: Arc<Vec<u8>>,
}

impl SceneTexture {
    pub fn rgba8(
        id: SceneTextureId,
        generation: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            id,
            generation,
            width,
            height,
            format: SceneTextureFormat::Rgba8,
            pixels: Arc::new(pixels),
        }
    }

    pub fn checked_rgba8(
        id: SceneTextureId,
        generation: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> anyhow::Result<Self> {
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(anyhow::anyhow!(
                "RGBA texture {:?} has {} bytes, expected {} for {}x{}",
                id,
                pixels.len(),
                expected,
                width,
                height
            ));
        }
        Ok(Self::rgba8(id, generation, width, height, pixels))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneTextureFormat {
    Rgba8,
}

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Sprite(SpriteDraw),
    SolidQuad(SolidQuad),
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteDraw {
    pub texture_id: SceneTextureId,
    /// Smooth UI surfaces when the window has more physical than PAL pixels.
    pub smooth_upscale: bool,
    pub priority: i32,
    pub dst: RectF,
    pub src: RectF,
    pub source_rect: [i32; 4],
    pub texture_size: [u32; 2],
    pub cell_size: [u32; 2],
    pub position: [f32; 3],
    pub offset: [i32; 2],
    pub color: [f32; 4],
    pub scale: f32,
    pub rotation: [f32; 3],
    pub center_offset: [f32; 2],
    pub render_mode: u32,
    pub blend_state: blend::NativeDrawState,
    pub raster_fx: Option<crate::native_vm::fx::RasterUniforms>,
}

impl SpriteDraw {
    pub fn effective_blend_state(&self) -> blend::NativeDrawState {
        self.blend_state.for_mode(self.render_mode)
    }

    pub fn full_frame(texture: &SceneTexture, width: u32, height: u32) -> Self {
        Self {
            texture_id: texture.id,
            smooth_upscale: false,
            priority: 0,
            dst: RectF::new(0.0, 0.0, width as f32, height as f32),
            src: RectF::new(0.0, 0.0, 1.0, 1.0),
            source_rect: [0, 0, texture.width as i32, texture.height as i32],
            texture_size: [texture.width, texture.height],
            cell_size: [texture.width, texture.height],
            position: [0.0; 3],
            offset: [0; 2],
            color: [1.0; 4],
            scale: 1.0,
            rotation: [0.0; 3],
            center_offset: [0.0; 2],
            render_mode: 0,
            blend_state: blend::NativeDrawState::default(),
            raster_fx: None,
        }
    }
    /// PAL rotation uses 512 units per turn, about the visible source center.
    /// Raster's vertex program runs in local clip coordinates before placement.
    pub fn quad(&self, apply_fx: bool) -> [[f32; 2]; 4] {
        let angle = self.rotation[2] * (std::f32::consts::TAU / 512.0);
        let (sin, cos) = angle.sin_cos();
        let center = [self.dst.x + self.dst.w * 0.5, self.dst.y + self.dst.h * 0.5];
        [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]].map(|mut point| {
            if apply_fx {
                if let Some(fx) = self.raster_fx {
                    point = fx.vertex(point);
                }
            }
            let x = point[0] * self.dst.w * 0.5;
            let y = -point[1] * self.dst.h * 0.5;
            [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos]
        })
    }
}

/// Flatten the render-tree prefix captured by a global PAL FX insertion. UI
/// commands in front of that depth remain outside the effect. The capture is
/// currently CPU-composited; the resulting FX quad uses the selected backend.
pub fn resolve_raster_stage(scene: &FrameScene) -> std::borrow::Cow<'_, FrameScene> {
    use std::hash::{Hash, Hasher};
    let Some(stage) = scene.raster_stage else {
        return std::borrow::Cow::Borrowed(scene);
    };
    let split = scene
        .commands
        .iter()
        .position(|command| match command {
            DrawCommand::Sprite(sprite) => sprite.priority > stage.priority,
            DrawCommand::SolidQuad(_) => true,
        })
        .unwrap_or(scene.commands.len());
    let mut prefix = scene.clone();
    prefix.raster_stage = None;
    prefix.commands.truncate(split);
    let pixels = rasterize_scene_rgba(&prefix);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixels.hash(&mut hasher);
    let texture = SceneTexture::rgba8(
        SceneTextureId(u64::MAX - 2),
        hasher.finish(),
        scene.logical_width.max(1),
        scene.logical_height.max(1),
        pixels,
    );
    let mut draw = SpriteDraw::full_frame(&texture, scene.logical_width, scene.logical_height);
    draw.priority = stage.priority;
    draw.raster_fx = Some(stage.uniforms);
    let mut resolved = scene.clone();
    resolved.raster_stage = None;
    resolved.textures.push(texture);
    resolved.commands = std::iter::once(DrawCommand::Sprite(draw))
        .chain(scene.commands[split..].iter().cloned())
        .collect();
    std::borrow::Cow::Owned(resolved)
}

#[derive(Clone, Copy, Debug)]
pub struct SolidQuad {
    pub dst: RectF,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn is_drawable(self) -> bool {
        self.w.abs() > f32::EPSILON && self.h.abs() > f32::EPSILON
    }
}

pub fn rasterize_scene_rgba(scene: &FrameScene) -> Vec<u8> {
    rasterize_scene_rgba_inner(scene, false)
}

/// PAL temporary render targets retain their alpha; presentation stays opaque.
pub fn rasterize_target_rgba(scene: &FrameScene) -> Vec<u8> {
    rasterize_scene_rgba_inner(scene, true)
}

fn rasterize_scene_rgba_inner(scene: &FrameScene, preserve_alpha: bool) -> Vec<u8> {
    let resolved = resolve_raster_stage(scene);
    let scene = resolved.as_ref();
    let width = scene.logical_width.max(1);
    let height = scene.logical_height.max(1);
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let bg = [
        (scene.clear_color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (scene.clear_color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (scene.clear_color[2].clamp(0.0, 1.0) * 255.0) as u8,
        if preserve_alpha {
            (scene.clear_color[3].clamp(0.0, 1.0) * 255.0) as u8
        } else {
            255
        },
    ];
    for px in pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }
    for command in &scene.commands {
        match command {
            DrawCommand::Sprite(sprite) => {
                if let Some(texture) = scene
                    .textures
                    .iter()
                    .find(|texture| texture.id == sprite.texture_id)
                {
                    if sprite.raster_fx.is_some() || sprite.rotation[2].rem_euclid(512.0) != 0.0 {
                        blit_transformed_sprite(
                            &mut pixels,
                            width,
                            height,
                            texture,
                            sprite,
                            preserve_alpha,
                        );
                        continue;
                    }
                    blit_sprite(
                        &mut pixels,
                        width,
                        height,
                        texture,
                        sprite.dst,
                        sprite.source_rect,
                        sprite.color,
                        sprite.render_mode,
                        sprite.effective_blend_state(),
                        preserve_alpha,
                    );
                }
            }
            DrawCommand::SolidQuad(quad) => {
                fill_rect(
                    &mut pixels,
                    width,
                    height,
                    quad.dst,
                    quad.color,
                    preserve_alpha,
                );
            }
        }
    }
    pixels
}

/// CPU reference for the GPU quad path, also used by the FFI console hosts.
/// Barycentric interpolation preserves source-cell UVs through rotation and FX.
fn blit_transformed_sprite(
    target: &mut [u8],
    width: u32,
    height: u32,
    texture: &SceneTexture,
    sprite: &SpriteDraw,
    preserve_alpha: bool,
) {
    let corners = sprite.quad(true);
    let min_x = corners
        .iter()
        .map(|p| p[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_x = corners
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, width as f32) as u32;
    let min_y = corners
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_y = corners
        .iter()
        .map(|p| p[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, height as f32) as u32;
    let uvs = [
        [sprite.src.x, sprite.src.y],
        [sprite.src.x + sprite.src.w, sprite.src.y],
        [sprite.src.x + sprite.src.w, sprite.src.y + sprite.src.h],
        [sprite.src.x, sprite.src.y + sprite.src.h],
    ];
    for y in min_y..max_y {
        for x in min_x..max_x {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let mut uv = None;
            for indices in [[0, 1, 2], [0, 2, 3]] {
                let [a, b, c] = indices.map(|i| corners[i]);
                let cross = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| {
                    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
                };
                let area = cross(a, b, c);
                if area.abs() < f32::EPSILON {
                    continue;
                }
                let wa = cross(point, b, c) / area;
                let wb = cross(a, point, c) / area;
                let wc = 1.0 - wa - wb;
                if wa >= -0.00001 && wb >= -0.00001 && wc >= -0.00001 {
                    let [a, b, c] = indices.map(|i| uvs[i]);
                    uv = Some([
                        a[0] * wa + b[0] * wb + c[0] * wc,
                        a[1] * wa + b[1] * wb + c[1] * wc,
                    ]);
                    break;
                }
            }
            let Some(mut uv) = uv else {
                continue;
            };
            if let Some(fx) = sprite.raster_fx {
                uv = fx.sample_uv(uv);
            }
            let sx = (uv[0] * texture.width as f32)
                .floor()
                .clamp(0.0, texture.width.saturating_sub(1) as f32) as usize;
            let sy = (uv[1] * texture.height as f32)
                .floor()
                .clamp(0.0, texture.height.saturating_sub(1) as f32) as usize;
            let index = (sy * texture.width as usize + sx) * 4;
            let color = sprite.effective_blend_state().modulation(sprite.color);
            let src = std::array::from_fn(|i| {
                texture.pixels[index + i] as f32 * color[i].clamp(0.0, 1.0)
            });
            blend_pixel_mode(
                target,
                width,
                x,
                y,
                src,
                sprite.render_mode,
                sprite.effective_blend_state(),
                preserve_alpha,
            );
        }
    }
}

fn blit_sprite(
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    texture: &SceneTexture,
    dst: RectF,
    source_rect: [i32; 4],
    color: [f32; 4],
    render_mode: u32,
    blend_state: blend::NativeDrawState,
    preserve_alpha: bool,
) {
    let color = blend_state.modulation(color);
    let x0 = dst.x.floor().max(0.0) as i32;
    let y0 = dst.y.floor().max(0.0) as i32;
    let x1 = (dst.x + dst.w).ceil().min(target_width as f32) as i32;
    let y1 = (dst.y + dst.h).ceil().min(target_height as f32) as i32;
    if x1 <= x0 || y1 <= y0 || dst.w.abs() < f32::EPSILON || dst.h.abs() < f32::EPSILON {
        return;
    }
    let sx0 = source_rect[0].max(0) as f32;
    let sy0 = source_rect[1].max(0) as f32;
    let sw = source_rect[2].saturating_sub(source_rect[0]).max(1) as f32;
    let sh = source_rect[3].saturating_sub(source_rect[1]).max(1) as f32;
    for y in y0..y1 {
        let v = ((y as f32 + 0.5 - dst.y) / dst.h).clamp(0.0, 1.0);
        let sy = (sy0 + v * sh)
            .floor()
            .clamp(0.0, texture.height.saturating_sub(1) as f32) as u32;
        for x in x0..x1 {
            let u = ((x as f32 + 0.5 - dst.x) / dst.w).clamp(0.0, 1.0);
            let sx = (sx0 + u * sw)
                .floor()
                .clamp(0.0, texture.width.saturating_sub(1) as f32) as u32;
            let src_idx = (sy as usize * texture.width as usize + sx as usize) * 4;
            let src = [
                texture.pixels[src_idx] as f32 * color[0].clamp(0.0, 1.0),
                texture.pixels[src_idx + 1] as f32 * color[1].clamp(0.0, 1.0),
                texture.pixels[src_idx + 2] as f32 * color[2].clamp(0.0, 1.0),
                texture.pixels[src_idx + 3] as f32 * color[3].clamp(0.0, 1.0),
            ];
            blend_pixel_mode(
                target,
                target_width,
                x as u32,
                y as u32,
                src,
                render_mode,
                blend_state,
                preserve_alpha,
            );
        }
    }
}

fn fill_rect(
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    rect: RectF,
    color: [f32; 4],
    preserve_alpha: bool,
) {
    let x0 = rect.x.floor().max(0.0) as i32;
    let y0 = rect.y.floor().max(0.0) as i32;
    let x1 = (rect.x + rect.w).ceil().min(target_width as f32) as i32;
    let y1 = (rect.y + rect.h).ceil().min(target_height as f32) as i32;
    let src = [
        color[0].clamp(0.0, 1.0) * 255.0,
        color[1].clamp(0.0, 1.0) * 255.0,
        color[2].clamp(0.0, 1.0) * 255.0,
        color[3].clamp(0.0, 1.0) * 255.0,
    ];
    for y in y0..y1 {
        for x in x0..x1 {
            blend_pixel(
                target,
                target_width,
                x as u32,
                y as u32,
                src,
                preserve_alpha,
            );
        }
    }
}

fn blend_pixel(
    target: &mut [u8],
    target_width: u32,
    x: u32,
    y: u32,
    src: [f32; 4],
    preserve_alpha: bool,
) {
    blend_pixel_mode(
        target,
        target_width,
        x,
        y,
        src,
        1,
        blend::NativeDrawState::default(),
        preserve_alpha,
    );
}

fn blend_pixel_mode(
    target: &mut [u8],
    target_width: u32,
    x: u32,
    y: u32,
    src: [f32; 4],
    mode: u32,
    blend_state: blend::NativeDrawState,
    preserve_alpha: bool,
) {
    let idx = (y as usize * target_width as usize + x as usize) * 4;
    let rgb = blend_state.blend_rgb(mode, [target[idx], target[idx + 1], target[idx + 2]], src);
    target[idx..idx + 3].copy_from_slice(&rgb);
    target[idx + 3] = if preserve_alpha {
        blend_state.blend_alpha(mode, target[idx + 3], src[3])
    } else {
        255
    };
}
