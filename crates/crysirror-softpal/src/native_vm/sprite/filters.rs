use super::*;

#[path = "filter_tables.rs"]
mod tables;
use tables::{BLUR_WEIGHTS, TONE_PALETTE};

impl SpriteSystem {
    /// Native PalSpriteSetOption backs up the first unfiltered surface, applies
    /// subsequent filters cumulatively, and restores that backup for option 0.
    pub(super) fn apply_surface_option(&mut self, handle: SpriteHandle, option: u32) -> bool {
        let Some(id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(surface) = self.surfaces.get(&id) else {
            return true;
        };
        if option == 0 {
            if let Some(original) = self.option_backups.remove(&handle) {
                let current = self.surfaces.get_mut(&id).unwrap();
                if current.width == original.width && current.height == original.height {
                    current.pixels = original.pixels;
                    current.mark_dirty();
                }
            }
            return true;
        }
        self.option_backups
            .entry(handle)
            .or_insert_with(|| surface.clone());
        let surface = self.surfaces.get_mut(&id).unwrap();
        let (width, height) = (surface.width as usize, surface.height as usize);
        match option {
            1 | 2 => {
                for pixel in surface.pixels_mut().chunks_exact_mut(4) {
                    let gray =
                        (3 * u16::from(pixel[0]) + 6 * u16::from(pixel[1]) + u16::from(pixel[2]))
                            / 10;
                    let rgb = if option == 1 {
                        TONE_PALETTE[gray as usize]
                    } else {
                        [gray as u8; 3]
                    };
                    pixel[..3].copy_from_slice(&rgb);
                }
            }
            3 => smooth_cross(surface.pixels_mut(), width, height),
            4 => {
                for pixel in surface.pixels_mut().chunks_exact_mut(4) {
                    if pixel[3] != 0 {
                        for component in &mut pixel[..3] {
                            *component = 255 - *component;
                        }
                        // PAL's scalar operation ORs the complement of RGB;
                        // its high byte becomes FF for every nontransparent pixel.
                        pixel[3] = 255;
                    }
                }
            }
            0x105 | 0x205 | 0x305 | 0x405 | 0x505 | 0x605 | 0x705 | 0x805 => {
                let level = ((option >> 8) - 1) as usize;
                let (half_width, half_height) = (width / 2, height / 2);
                if half_width != 0 && half_height != 0 {
                    let mut reduced = vec![0; half_width * half_height * 4];
                    for y in 0..half_height {
                        for x in 0..half_width {
                            let src = ((y * 2) * width + x * 2) * 4;
                            let dst = (y * half_width + x) * 4;
                            reduced[dst..dst + 4].copy_from_slice(&surface.pixels[src..src + 4]);
                        }
                    }
                    let blurred = blur(&reduced, half_width, half_height, level);
                    upscale_two(
                        &blurred,
                        half_width,
                        half_height,
                        surface.pixels_mut(),
                        width,
                    );
                }
            }
            0x905 => {
                let result = blur(&surface.pixels, width, height, 1);
                surface.pixels_mut().copy_from_slice(&result);
            }
            _ => {} // Native retains the option and backup for unknown values.
        }
        true
    }

    pub(crate) fn option_backup(&self, handle: SpriteHandle) -> Option<&[u8]> {
        let current = self
            .get(handle)
            .and_then(|sprite| self.surface(sprite.surface))?;
        let original = self.option_backups.get(&handle)?;
        (current.width == original.width && current.height == original.height)
            .then_some(original.pixels.as_ref())
    }

    pub(crate) fn restore_option(&mut self, handle: SpriteHandle, option: u32, pixels: &[u8]) {
        let Some(sprite) = self.get_mut(handle) else {
            return;
        };
        sprite.option_type = option;
        sprite.option_aux = (option != 0).then_some(SpriteOptionAux {
            raw_type: option,
            raw_value: 0,
        });
        let id = sprite.surface;
        self.option_backups.remove(&handle);
        if let Some(surface) = self.surfaces.get(&id) {
            if !pixels.is_empty() && pixels.len() == surface.pixels.len() {
                let mut original = surface.clone();
                original.pixels = Arc::new(pixels.to_vec());
                self.option_backups.insert(handle, original);
            }
        }
    }
}

fn smooth_cross(pixels: &mut [u8], width: usize, height: usize) {
    if width < 3 || height < 3 {
        return;
    }
    // The native MMX routine reads and writes the same buffer, row by row.
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = (y * width + x) * 4;
            for c in 0..3 {
                pixels[i + c] = ((4 * u16::from(pixels[i + c])
                    + u16::from(pixels[i + 4 + c])
                    + u16::from(pixels[i - 4 + c])
                    + u16::from(pixels[i + width * 4 + c])
                    + u16::from(pixels[i - width * 4 + c]))
                    >> 3) as u8;
            }
        }
    }
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

fn blur(source: &[u8], width: usize, height: usize, level: usize) -> Vec<u8> {
    let weights = &BLUR_WEIGHTS[level];
    let pass = |input: &[u8], horizontal: bool| {
        let mut output = vec![0; input.len()];
        for y in 0..height {
            for x in 0..width {
                let position = if horizontal { x } else { y };
                let extent = if horizontal { width } else { height };
                let interior = position >= 8 && position + 8 < extent;
                let mut sum = [0u32; 4];
                let mut denominator = 0u32;
                for (tap, &weight) in weights.iter().enumerate() {
                    let at = position as isize + tap as isize - 8;
                    if at < 0 || at >= extent as isize {
                        continue;
                    }
                    let offset = if horizontal {
                        y * width + at as usize
                    } else {
                        at as usize * width + x
                    } * 4;
                    let weight = u32::from(weight);
                    denominator += weight;
                    if interior || input[offset + 3] != 0 {
                        for c in 0..4 {
                            sum[c] += weight * u32::from(input[offset + c]);
                        }
                    }
                }
                let dst = (y * width + x) * 4;
                if !interior && (sum[3] == 0 || denominator == 0) {
                    output[dst..dst + 4].copy_from_slice(&input[dst..dst + 4]);
                } else {
                    for c in 0..4 {
                        output[dst + c] = if interior {
                            (sum[c] >> (level + 1)) as u8
                        } else {
                            (sum[c] / denominator) as u8
                        };
                    }
                }
            }
        }
        output
    };
    pass(&pass(source, true), false)
}

fn upscale_two(source: &[u8], width: usize, height: usize, dest: &mut [u8], stride: usize) {
    // PAL uses 7-bit fixed-point bilinear interpolation for the 2x pass.
    // Odd trailing rows/columns are outside its written rectangle.
    for y in 0..height * 2 {
        for x in 0..width * 2 {
            let (sx, sy) = (x / 2, y / 2);
            let (next_x, next_y) = ((sx + 1).min(width - 1), (sy + 1).min(height - 1));
            let (fx, fy) = ((x % 2) as u32 * 64, (y % 2) as u32 * 64);
            for c in 0..4 {
                let a = u32::from(source[(sy * width + sx) * 4 + c]);
                let b = u32::from(source[(sy * width + next_x) * 4 + c]);
                let d = u32::from(source[(next_y * width + sx) * 4 + c]);
                let e = u32::from(source[(next_y * width + next_x) * 4 + c]);
                dest[(y * stride + x) * 4 + c] = (((128 - fy) * ((128 - fx) * a + fx * b)
                    + fy * ((128 - fx) * d + fx * e))
                    >> 14) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (SpriteSystem, SpriteHandle) {
        let mut sprites = SpriteSystem::new();
        let handle = sprites
            .create_rgba_sprite(
                4,
                4,
                [100, 150, 200, 128].repeat(16),
                PalVec3::default(),
                0,
                "filter",
            )
            .unwrap();
        (sprites, handle)
    }

    #[test]
    fn filters_modify_pixels_cumulatively_and_reset_original() {
        let (mut sprites, handle) = fixture();
        let id = sprites.get(handle).unwrap().surface;
        sprites.set_option(handle, 2);
        assert_eq!(
            &sprites.surface(id).unwrap().pixels[..4],
            &[140, 140, 140, 128]
        );
        sprites.set_option(handle, 4);
        assert_eq!(
            &sprites.surface(id).unwrap().pixels[..4],
            &[115, 115, 115, 255]
        );
        sprites.set_option(handle, 0);
        assert_eq!(
            &sprites.surface(id).unwrap().pixels[..4],
            &[100, 150, 200, 128]
        );
        assert!(sprites.option_backups.is_empty());
    }

    #[test]
    fn blur_uses_native_asymmetric_kernel_and_transparent_border_rule() {
        let source = [20, 40, 80, 255, 100, 120, 160, 255, 200, 220, 240, 255];
        let out = blur(&source, 3, 1, 0);
        assert_eq!(out, [20, 40, 80, 255, 60, 80, 120, 255, 150, 170, 200, 255]);
        assert_eq!(blur(&[10, 20, 30, 0], 1, 1, 7), [10, 20, 30, 0]);
    }

    #[test]
    fn paint_replaces_alpha_clips_rectangle_and_invalidates_texture() {
        let (mut sprites, handle) = fixture();
        let id = sprites.get(handle).unwrap().surface;
        let generation = sprites.surface(id).unwrap().generation;
        assert!(sprites.paint_rect(
            handle,
            Some(PalRect::new(-2, 1, 2, 3)),
            PalColor(0x00112233)
        ));
        let surface = sprites.surface(id).unwrap();
        assert!(surface.generation > generation);
        for y in 0..4 {
            for x in 0..4 {
                let expected = if x < 2 && (1..3).contains(&y) {
                    [17, 34, 51, 0]
                } else {
                    [100, 150, 200, 128]
                };
                assert_eq!(
                    &surface.pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4],
                    &expected
                );
            }
        }
    }
}
