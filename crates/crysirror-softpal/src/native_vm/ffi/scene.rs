//! Borrowed C scene views for native GPU hosts. Pixel storage stays owned by
//! the host until its next step or destruction; consumers must upload it first.
use super::*;
use crate::native_vm::scene::{resolve_raster_stage, DrawCommand};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SenaSceneTexture {
    pub id: u64,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: *const u8,
    pub byte_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SenaSceneDraw {
    pub texture_id: u64,
    pub quad: [[f32; 2]; 4],
    pub uv: [[f32; 2]; 4],
    pub color: [f32; 4],
    pub raster: [f32; 4],
    pub smooth_upscale: u32,
    pub render_mode: u32,
    pub blend_disabled: u32,
    pub texture_rgb: u32,
}

pub(super) struct NativeScene {
    scene: FrameScene,
    draws: Vec<SenaSceneDraw>,
    clear: [f32; 4],
}

impl NativeScene {
    pub(super) fn new(scene: FrameScene) -> Self {
        let scene = resolve_raster_stage(&scene).into_owned();
        let draws = scene
            .commands
            .iter()
            .map(|command| match command {
                DrawCommand::Sprite(sprite) => SenaSceneDraw {
                    texture_id: sprite.texture_id.0,
                    quad: sprite.quad(false),
                    uv: [
                        [sprite.src.x, sprite.src.y],
                        [sprite.src.x + sprite.src.w, sprite.src.y],
                        [sprite.src.x + sprite.src.w, sprite.src.y + sprite.src.h],
                        [sprite.src.x, sprite.src.y + sprite.src.h],
                    ],
                    color: sprite.color,
                    raster: sprite
                        .raster_fx
                        .map(|fx| [fx.offset, fx.undulate_x, fx.undulate_y, 1.0])
                        .unwrap_or([0.0; 4]),
                    smooth_upscale: sprite.smooth_upscale as u32,
                    render_mode: sprite.render_mode,
                    blend_disabled: sprite.effective_blend_state().blend_disabled as u32,
                    texture_rgb: sprite.effective_blend_state().texture_rgb as u32,
                },
                DrawCommand::SolidQuad(quad) => SenaSceneDraw {
                    texture_id: 0,
                    quad: [
                        [quad.dst.x, quad.dst.y],
                        [quad.dst.x + quad.dst.w, quad.dst.y],
                        [quad.dst.x + quad.dst.w, quad.dst.y + quad.dst.h],
                        [quad.dst.x, quad.dst.y + quad.dst.h],
                    ],
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    color: quad.color,
                    raster: [0.0; 4],
                    smooth_upscale: 0,
                    render_mode: 1,
                    blend_disabled: 0,
                    texture_rgb: 0,
                },
            })
            .collect();
        Self {
            clear: scene.clear_color.map(|v| v as f32),
            scene,
            draws,
        }
    }
}

#[no_mangle]
pub extern "C" fn sena_host_set_scene_rendering(handle: *mut c_void, enabled: i32) {
    if let Some(host) = host_from_handle(handle) {
        host.scene_rendering = enabled != 0;
        host.native_scene = None;
        host.rgba.clear();
    }
}

#[no_mangle]
pub extern "C" fn sena_host_scene_texture_count(handle: *mut c_void) -> usize {
    host_from_handle(handle)
        .and_then(|host| host.native_scene.as_ref())
        .map_or(0, |frame| frame.scene.textures.len())
}

#[no_mangle]
pub extern "C" fn sena_host_scene_draw_count(handle: *mut c_void) -> usize {
    host_from_handle(handle)
        .and_then(|host| host.native_scene.as_ref())
        .map_or(0, |frame| frame.draws.len())
}

#[no_mangle]
pub extern "C" fn sena_host_scene_texture(
    handle: *mut c_void,
    index: usize,
    out: *mut SenaSceneTexture,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    let Some(texture) = host_from_handle(handle)
        .and_then(|host| host.native_scene.as_ref())
        .and_then(|frame| frame.scene.textures.get(index))
    else {
        return 0;
    };
    unsafe {
        out.write(SenaSceneTexture {
            id: texture.id.0,
            generation: texture.generation,
            width: texture.width,
            height: texture.height,
            rgba: texture.pixels.as_ptr(),
            byte_len: texture.pixels.len(),
        });
    }
    1
}

#[no_mangle]
pub extern "C" fn sena_host_scene_draw(
    handle: *mut c_void,
    index: usize,
    out: *mut SenaSceneDraw,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    let Some(draw) = host_from_handle(handle)
        .and_then(|host| host.native_scene.as_ref())
        .and_then(|frame| frame.draws.get(index))
    else {
        return 0;
    };
    unsafe {
        out.write(*draw);
    }
    1
}

#[no_mangle]
pub extern "C" fn sena_host_scene_clear_color(handle: *mut c_void) -> *const f32 {
    host_from_handle(handle)
        .and_then(|host| host.native_scene.as_ref())
        .map_or(ptr::null(), |frame| frame.clear.as_ptr())
}

/// Render the current scene on demand without advancing the VM. Native hosts
/// use this for effects unsupported by their precompiled GPU programs.
#[no_mangle]
pub extern "C" fn sena_host_scene_rasterize(handle: *mut c_void) -> *const u8 {
    let Some(host) = host_from_handle(handle) else {
        return ptr::null();
    };
    if host.rgba.is_empty() {
        if let Some(frame) = &host.native_scene {
            host.rgba = rasterize_scene_rgba(&frame.scene);
        }
    }
    if host.rgba.is_empty() {
        ptr::null()
    } else {
        host.rgba.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_vm::scene::{SceneTexture, SceneTextureId, SpriteDraw};
    #[test]
    fn scene_views_keep_pixels_and_unapplied_fx_for_the_native_renderer() {
        let texture = SceneTexture::rgba8(SceneTextureId(7), 9, 2, 2, vec![255; 16]);
        let mut draw = SpriteDraw::full_frame(&texture, 2, 2);
        draw.render_mode = 10;
        draw.blend_state = crate::native_vm::scene::blend::NativeDrawState {
            blend_disabled: true,
            texture_rgb: true,
        };
        draw.raster_fx = Some(crate::native_vm::fx::RasterUniforms {
            offset: 0.3,
            undulate_x: 0.2,
            undulate_y: 0.1,
        });
        let scene = FrameScene::boot()
            .with_texture(texture)
            .with_command(DrawCommand::Sprite(draw));
        let exported = NativeScene::new(scene);
        assert_eq!(
            exported.draws[0].quad,
            [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]]
        );
        assert_eq!(exported.draws[0].raster, [0.3, 0.2, 0.1, 1.0]);
        assert_eq!(exported.draws[0].render_mode, 10);
        assert_eq!(exported.draws[0].blend_disabled, 1);
        assert_eq!(exported.draws[0].texture_rgb, 1);
        assert_eq!(&*exported.scene.textures[0].pixels, &[255; 16]);
    }
}
