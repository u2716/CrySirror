// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KIRA42 native global star clocks and four-call initialization order.
//! Complete original cpp/header and PrintKiraSub's MMX inspected. Shared
//! colour-matrix and saturated raster jobs replace its private blue table.
use crate::image_bridge::NativePictureViews;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    integer_color::ColorMatrixFixed,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
#[derive(Clone, Copy, Default)]
struct Star {
    count: i32,
    frames: i32,
    point: [i32; 2],
}
#[derive(Clone, Default)]
pub(crate) struct KiraClass {
    configured: bool,
    picture: Option<NativePictureViews>,
    stars: [Star; 20],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 6] {
    // para0 is never defaulted or used, although it enters original saves.
    // Define that fresh uninitialized word as0, preserving supplied/raw values.
    let defaults = [0, 20, 32, 32, canvas[0] as i32 - 64, canvas[1] as i32 - 64];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[1] = p[1].clamp(1, 20);
    for axis in 0..2 {
        let limit = canvas[axis] as i32;
        p[2 + axis] = p[2 + axis].max(32).min(limit - 32);
        p[4 + axis] = p[4 + axis].max(1);
        if p[2 + axis].wrapping_add(p[4 + axis]) > limit - 33 {
            p[4 + axis] = limit - 33 - p[2 + axis];
        }
        p[4 + axis] = p[4 + axis].max(1);
    }
    p
}
impl KiraClass {
    pub(crate) fn picture(&mut self, picture: Option<NativePictureViews>) {
        self.configured = true;
        self.picture = picture;
    }
    fn replace(
        &mut self,
        p: &[i32; 6],
        canvas: [u32; 2],
        i: usize,
        rng: &mut PatternRng,
    ) -> Result<()> {
        let frames = (rng.next_draw()? % 8) as i32 + 5;
        let dx = (rng.next_draw()? % p[4] as u32) as i32;
        let dy = (rng.next_draw()? % p[5] as u32) as i32;
        let point = std::array::from_fn(|axis| {
            p[2 + axis]
                .wrapping_add([dx, dy][axis])
                .wrapping_sub(32)
                .max(32)
                .min(canvas[axis] as i32 - 32)
        });
        self.stars[i] = Star {
            count: 0,
            frames,
            point,
        };
        Ok(())
    }
    pub(crate) fn set(
        &mut self,
        p: &[i32; 6],
        canvas: [u32; 2],
        rng: &mut PatternRng,
    ) -> Result<()> {
        ensure!(
            self.configured,
            "NNN KIRA constructor asset view is not configured"
        );
        ensure!(
            (1..=20).contains(&p[1]) && p[4] > 0 && p[5] > 0,
            "NNN KIRA bank/modulus"
        );
        for i in 0..p[1] as usize {
            self.replace(p, canvas, i, rng)?;
            self.stars[i].count = (rng.next_draw()? % self.stars[i].frames as u32) as i32;
        }
        Ok(())
    }
    pub(crate) fn image_input(
        &mut self,
        p: &[i32; 6],
        canvas: [u32; 2],
        mut base: ImageInput,
        rng: &mut PatternRng,
    ) -> Result<Option<ImageInput>> {
        ensure!(
            self.configured,
            "NNN KIRA constructor asset view is not configured"
        );
        let Some(picture) = self.picture.clone() else {
            return Ok(None);
        };
        let size = picture.raw.size();
        if size[0] < 64 || size[1] < 64 {
            return Ok(None);
        }
        ensure!(
            (1..=20).contains(&p[1]) && p[4] > 0 && p[5] > 0,
            "NNN KIRA Print work bounds"
        );
        // One global bank is advanced by EVERY KIRA Print, including multiple
        // slots in one frame. Unlike SNOW there is no per-frame guard.
        for i in 0..p[1] as usize {
            let c = self.stars[i].count + 1;
            if c >= self.stars[i].frames {
                self.replace(p, canvas, i, rng)?;
            } else {
                self.stars[i].count = c;
            }
        }
        for star in &self.stars[..p[1] as usize] {
            let frames = star.frames.max(1);
            let first = ((frames * 7 + 5) / 10).max(1);
            let last = (frames - first).max(1);
            let percent = (if star.count <= first {
                star.count * 100 / first
            } else {
                (frames - star.count) * 100 / last
            })
            .clamp(0, 100);
            let threshold = 255 - percent * 255 / 100;
            let crop = ImageInput::Composite {
                base: Box::new(ImageInput::Solid {
                    size: [64, 64],
                    color: [0, 0, 0, 255],
                }),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(picture.raw.clone())),
                    source: [0, 0, 64, 64],
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                }],
            };
            let source = ImageInput::Process {
                input: Box::new(crop),
                effects: serde_json::to_vec(&[ImageEffect::IntegerColorMatrix {
                    spec: ColorMatrixFixed {
                        matrix: [[0, 0, 1]; 3],
                        offset: [-threshold; 3],
                        divisor: 1,
                    },
                }])?
                .into(),
            };
            // Original clamps against size-32 although it then writes64pixels.
            // Shared destination clipping defines those out-of-screen writes.
            let point =
                std::array::from_fn(|axis| star.point[axis].max(32).min(canvas[axis] as i32 - 32));
            let spec = RowBlit {
                source: [0, 0, 64, 64],
                destination: point,
                row_offsets: vec![0; 64],
                row_math: vec![],
                math: ByteRasterMath::SaturatedRgb { subtract: false },
            };
            spec.validate()?;
            base = ImageInput::Process {
                input: Box::new(ImageInput::Raster {
                    destination: Box::new(base),
                    source: Box::new(source),
                }),
                effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                    spec: ByteRaster::RowBlit { spec },
                }])?
                .into(),
            };
        }
        Ok(Some(base))
    }
}
// SetParam/InitParameter restart count0/max99999; CountIsMaxTRUE changes no
// state. All generic Calcu/Init/BeginPrint/End hooks are empty, ordinary picture
// underlay is inheritedTRUE. Asset absence/undersize returns BEFORE Print RNG.
