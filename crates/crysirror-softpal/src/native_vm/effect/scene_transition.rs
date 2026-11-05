//! PalEffectEx callbacks shared by the reference PAL and Koikake's DLL.
//!
//! See reverse/scene-transitions.md for addresses and native arithmetic. PAL
//! updates the captured image on the CPU even in its Direct3D path. The result
//! is an ordinary scene texture on wgpu, deko3d, and GXM alike.

use crate::native_asset::ResourceManager;

use super::PalEffectState;
use crate::native_vm::image::decode_image;
use crate::native_vm::scene::{
    rasterize_scene_rgba, DrawCommand, FrameScene, RectF, SceneTexture, SceneTextureId, SpriteDraw,
};

mod sine_table;

pub(crate) struct SceneTransition {
    pub start_ms: u32,
    pub effect_id: u32,
    image: SceneTexture,
    mask: Option<Vec<u8>>,
    captured_new_frame: bool,
}

impl SceneTransition {
    pub fn capture(
        effect: PalEffectState,
        previous: &FrameScene,
        resources: Option<&mut ResourceManager>,
        generation: u64,
    ) -> Option<Self> {
        let mask = if effect.effect_id >= 10 {
            let resources = resources?;
            let name = format!("Mask{:02}.bmp", effect.effect_id - 10);
            let asset = resources
                .open(&format!("Mask\\{name}"))
                .or_else(|_| resources.open(&name))
                .ok()?;
            let image = match decode_image(&asset.bytes) {
                Ok(image) => image,
                Err(error) => {
                    log::warn!("transition mask {name}: {error}");
                    return None;
                }
            };
            // PAL consumes packed BGRA blue bytes in memory order. It does not
            // rescale a mask to the screen. Bound mismatched resources safely.
            Some(image.rgba.chunks_exact(4).map(|pixel| pixel[2]).collect())
        } else {
            None
        };
        if !matches!(effect.effect_id, 1..=3 | 5 | 6 | 10..=49) {
            return None;
        }
        let mut pixels = rasterize_scene_rgba(previous);
        if effect.effect_id == 5 {
            pixels.fill(255);
        }
        Some(Self {
            start_ms: effect.start_ms,
            effect_id: effect.effect_id,
            image: SceneTexture::rgba8(
                SceneTextureId(u64::MAX),
                generation,
                previous.logical_width,
                previous.logical_height,
                pixels,
            ),
            mask,
            captured_new_frame: false,
        })
    }

    pub fn append(&mut self, scene: &mut FrameScene, effect: PalEffectState, now_ms: u32) {
        let elapsed = if effect.state == 2 {
            effect.duration_ms
        } else {
            now_ms.wrapping_sub(effect.start_ms).min(effect.duration_ms)
        };
        let t = elapsed as f32 / effect.duration_ms.max(1) as f32;
        let mut texture = self.image.clone();
        let mut draw = SpriteDraw::full_frame(&texture, texture.width, texture.height);
        draw.priority = i32::MAX;
        match effect.effect_id {
            1 | 2 => {
                if effect.effect_id == 2 {
                    blur_in_place(
                        std::sync::Arc::make_mut(&mut self.image.pixels).as_mut_slice(),
                        self.image.width as usize,
                        self.image.height as usize,
                    );
                    self.image.generation = self.image.generation.wrapping_add(1);
                    texture = self.image.clone();
                }
                draw.color[3] = alpha_byte(1.0 - t) as f32 / 255.0;
            }
            3 => {
                let width = ((1.0 - t) * texture.width as f32) as u32;
                let height = (u64::from(width) * u64::from(texture.height)
                    / u64::from(texture.width)) as u32;
                let x = (texture.width - width) / 2;
                let y = (texture.height - height) / 2;
                // Both native D3D builds pass x for the destination y too.
                // Keep that observable placement, with the actual y in UVs.
                draw.dst = RectF::new(x as f32, x as f32, width as f32, height as f32);
                draw.src = RectF::new(
                    x as f32 / texture.width as f32,
                    y as f32 / texture.height as f32,
                    width as f32 / texture.width as f32,
                    height as f32 / texture.height as f32,
                );
                draw.source_rect = [x as i32, y as i32, (x + width) as i32, (y + height) as i32];
            }
            5 => draw.color[3] = alpha_byte(t) as f32 / 255.0,
            6 => {
                if !self.captured_new_frame && elapsed > effect.duration_ms / 2 {
                    self.image.pixels = rasterize_scene_rgba(scene).into();
                    self.image.width = scene.logical_width;
                    self.image.height = scene.logical_height;
                    self.captured_new_frame = true;
                }
                let index = (t * 256.0) as usize;
                let sine = f32::from_bits(sine_table::SINE_BITS[index.min(256)]);
                let inverse = alpha_byte(1.0 - sine);
                let block = ((256 - u32::from(inverse)) >> 1) + 1;
                let pixels = mosaic(
                    &self.image.pixels,
                    self.image.width as usize,
                    self.image.height as usize,
                    block,
                );
                self.image.generation = self.image.generation.wrapping_add(1);
                texture = SceneTexture::rgba8(
                    self.image.id,
                    self.image.generation,
                    self.image.width,
                    self.image.height,
                    pixels,
                );
                draw = SpriteDraw::full_frame(&texture, texture.width, texture.height);
                draw.priority = i32::MAX;
            }
            10..=49 => {
                let pixels = std::sync::Arc::make_mut(&mut self.image.pixels);
                let threshold = alpha_byte(1.0 - t);
                if let Some(mask) = self.mask.as_ref() {
                    for (pixel, &level) in pixels.chunks_exact_mut(4).zip(mask) {
                        pixel[3] = mask_alpha(level, threshold);
                    }
                }
                self.image.generation = self.image.generation.wrapping_add(1);
                texture = self.image.clone();
            }
            _ => return,
        }
        if draw.color[3] > 0.0 && draw.dst.is_drawable() {
            scene.textures.push(texture);
            scene.commands.push(DrawCommand::Sprite(draw));
        }
    }
}

fn alpha_byte(value: f32) -> u8 {
    (value * 255.0) as u8
}

fn mask_alpha(level: u8, threshold: u8) -> u8 {
    let upper = threshold.saturating_add(31);
    if level < threshold {
        255
    } else if level >= upper {
        0
    } else {
        (upper - level) * 8
    }
}

/// Native MMX five-point filter is in-place: left/top are already updated.
fn blur_in_place(pixels: &mut [u8], width: usize, height: usize) {
    if width < 3 || height < 3 {
        return;
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let offset = (y * width + x) * 4;
            for c in 0..3 {
                pixels[offset + c] = ((u16::from(pixels[offset + c]) * 4
                    + u16::from(pixels[offset - 4 + c])
                    + u16::from(pixels[offset + 4 + c])
                    + u16::from(pixels[offset - width * 4 + c])
                    + u16::from(pixels[offset + width * 4 + c]))
                    >> 3) as u8;
            }
        }
    }
    // Native walks the boundary clockwise, including its corner ordering.
    for x in 0..width - 1 {
        copy_pixel(pixels, x, width + x);
    }
    for y in 0..height - 1 {
        copy_pixel(pixels, y * width + width - 1, y * width + width - 2);
    }
    for x in (1..width).rev() {
        copy_pixel(pixels, (height - 1) * width + x, (height - 2) * width + x);
    }
    for y in (1..height).rev() {
        copy_pixel(pixels, y * width, y * width + 1);
    }
}

fn copy_pixel(pixels: &mut [u8], dst: usize, src: usize) {
    pixels.copy_within(src * 4..src * 4 + 4, dst * 4);
}

fn mosaic(pixels: &[u8], width: usize, height: usize, block: u32) -> Vec<u8> {
    let sx = width as f32 / (width as f32 * block as f32);
    let sy = height as f32 / (height as f32 * block as f32);
    let small_width = ((width as f32 * sx) as usize).max(1);
    let small_height = ((height as f32 * sy) as usize).max(1);
    let small = resize_fixed(pixels, width, height, small_width, small_height, sx, sy);
    resize_fixed(
        &small,
        small_width,
        small_height,
        width,
        height,
        width as f32 / small_width as f32,
        height as f32 / small_height as f32,
    )
}

/// Native nearest-neighbor resampling accumulates truncated 7-bit steps.
fn resize_fixed(
    src: &[u8],
    sw: usize,
    sh: usize,
    dw: usize,
    dh: usize,
    sx: f32,
    sy: f32,
) -> Vec<u8> {
    let step_x = (128.0 / sx).abs() as usize;
    let step_y = (128.0 / sy).abs() as usize;
    let mut dst = vec![0; dw * dh * 4];
    for y in 0..dh {
        for x in 0..dw {
            let src_offset =
                (((y * step_y) >> 7).min(sh - 1) * sw + ((x * step_x) >> 7).min(sw - 1)) * 4;
            dst[(y * dw + x) * 4..][..4].copy_from_slice(&src[src_offset..][..4]);
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_threshold_has_native_31_level_band_and_endpoint_clamp() {
        assert_eq!(
            [
                mask_alpha(99, 100),
                mask_alpha(100, 100),
                mask_alpha(130, 100),
                mask_alpha(131, 100)
            ],
            [255, 248, 8, 0]
        );
        assert_eq!(
            [
                mask_alpha(249, 250),
                mask_alpha(250, 250),
                mask_alpha(254, 250),
                mask_alpha(255, 250)
            ],
            [255, 40, 8, 0]
        );
        assert_eq!(
            [
                mask_alpha(254, 255),
                mask_alpha(255, 255),
                mask_alpha(0, 0),
                mask_alpha(31, 0)
            ],
            [255, 0, 248, 0]
        );
    }

    #[test]
    fn blur_accumulates_in_place_and_preserves_center_alpha() {
        let mut pixels = vec![0; 5 * 5 * 4];
        pixels[(2 * 5 + 2) * 4..][..4].copy_from_slice(&[255, 128, 64, 77]);
        blur_in_place(&mut pixels, 5, 5);
        assert_eq!(&pixels[(2 * 5 + 2) * 4..][..4], &[135, 68, 34, 77]);
        assert_eq!(pixels[(2 * 5 + 3) * 4], 17);
        assert_eq!(pixels[(3 * 5 + 2) * 4], 17);
    }

    #[test]
    fn resize_uses_seven_bit_steps_instead_of_float_uvs() {
        let src: Vec<u8> = (0..7u8).flat_map(|x| [x, x, x, 255]).collect();
        let dst = resize_fixed(&src, 7, 1, 10, 1, 10.0 / 7.0, 1.0);
        assert_eq!(
            dst.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>(),
            [0, 0, 1, 2, 2, 3, 4, 4, 5, 6]
        );
    }

    fn solid(color: [u8; 4]) -> FrameScene {
        let mut scene = FrameScene::boot().with_logical_size(256, 256);
        scene.clear_color = color.map(|c| c as f64 / 255.0);
        scene
    }

    fn effect(id: u32) -> PalEffectState {
        PalEffectState {
            effect_id: id,
            duration_ms: 100,
            start_ms: 10,
            arg: 0,
            wait: false,
            state: 1,
        }
    }

    #[test]
    fn crossfade_uses_captured_pixels_and_byte_quantized_alpha() {
        let old = solid([255, 0, 0, 255]);
        let mut next = solid([0, 0, 255, 255]);
        let mut transition = SceneTransition::capture(effect(1), &old, None, 1).unwrap();
        transition.append(&mut next, effect(1), 60);
        let pixels = rasterize_scene_rgba(&next);
        assert_eq!(&pixels[..4], &[127, 0, 128, 255]);
    }

    #[test]
    fn mosaic_switches_source_strictly_after_midpoint_and_keeps_it() {
        let old = solid([255, 0, 0, 255]);
        let mut transition = SceneTransition::capture(effect(6), &old, None, 1).unwrap();
        for (now, background, expected) in [
            (60, [0, 0, 255, 255], [255, 0, 0, 255]),
            (61, [0, 0, 255, 255], [0, 0, 255, 255]),
            (80, [0, 255, 0, 255], [0, 0, 255, 255]),
        ] {
            let mut next = solid(background);
            transition.append(&mut next, effect(6), now);
            assert_eq!(&rasterize_scene_rgba(&next)[..4], &expected);
        }
    }

    #[test]
    fn white_fade_ignores_argument_sign_and_draws_terminal_frame() {
        let old = solid([255, 0, 0, 255]);
        let mut next = solid([0, 0, 255, 255]);
        let mut state = effect(5);
        state.arg = -1;
        let mut transition = SceneTransition::capture(state, &old, None, 1).unwrap();
        transition.append(&mut next, state, 110);
        assert_eq!(&rasterize_scene_rgba(&next)[..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn real_mask_archive_decodes_and_changes_old_frame_alpha() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testcase");
        if !root.join("mask.pac").is_file() {
            return;
        }
        let mut resources = ResourceManager::bootstrap(root, crate::native_asset::Nls::ShiftJis).unwrap();
        let old = solid([255, 0, 0, 255]).with_logical_size(1280, 720);
        let mut transition =
            SceneTransition::capture(effect(10), &old, Some(&mut resources), 1).unwrap();
        assert_eq!(transition.mask.as_ref().unwrap().len(), 1280 * 720);
        let mut next = solid([0, 0, 255, 255]).with_logical_size(1280, 720);
        transition.append(&mut next, effect(10), 60);
        let alpha: Vec<_> = next.textures[0]
            .pixels
            .chunks_exact(4)
            .map(|p| p[3])
            .collect();
        assert!(alpha.contains(&0) && alpha.contains(&255));
        assert!(alpha.iter().any(|a| *a > 0 && *a < 255));
    }
}
