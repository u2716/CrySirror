// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FLOWER12's persistent200-object bank, native trigonometric deltas, atlas
//! geometry and ordered RNG. Full297-line source/header and CPicture::Blt,
//! Blt256/MaskBlt helpers read. Raster/colour/index keying use engine services.
use crate::image_bridge::NativePictureViews;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::temporal_pattern::PatternRng;
const SIZES: [i32; 3] = [32, 16, 8];
const TOPS: [i32; 3] = [0, 32, 48];
#[derive(Clone, Copy, Default)]
struct Flower {
    kind: usize,
    pattern: i32,
    point: [i32; 2],
    position: [i32; 2],
    speed: [i32; 2],
    count: i32,
    frames: i32,
}
#[derive(Clone)]
pub(crate) struct FlowerClass {
    expanded_random: bool,
    picture: Option<NativePictureViews>,
    flowers: [Flower; 200],
    delta_x: [[i32; 40]; 10],
    delta_y: [[i32; 40]; 10],
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 4] {
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or([20, 0, 2, 100][i]));
    p[3] = p[3].min(200);
    p
}
impl FlowerClass {
    pub(crate) fn initialize(
        canvas: [u32; 2],
        picture: Option<NativePictureViews>,
        rng: &mut PatternRng,
    ) -> Result<Self> {
        Self::initialize_profile(canvas, picture, true, rng)
    }
    /// MUGEN2 0x4bcaf0 uses one rand/axis (five draws/object); the later
    /// source combines two draws/axis (seven). Full ctor0x4c3520 and native
    /// Flower Init0x4bcc70 were statically reviewed, not executed.
    pub(crate) fn initialize_legacy(
        canvas: [u32; 2],
        picture: Option<NativePictureViews>,
        rng: &mut PatternRng,
    ) -> Result<Self> {
        Self::initialize_profile(canvas, picture, false, rng)
    }
    fn initialize_profile(
        canvas: [u32; 2],
        picture: Option<NativePictureViews>,
        expanded_random: bool,
        rng: &mut PatternRng,
    ) -> Result<Self> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN flower canvas"
        );
        let mut class = Self {
            expanded_random,
            picture,
            flowers: [Flower::default(); 200],
            delta_x: [[0; 40]; 10],
            delta_y: [[0; 40]; 10],
        };
        for j in 0..10 {
            let frames = j + 30;
            for i in 0..frames {
                // Keep the source's staged multiplies/divisions and .5/trunc
                // for negative values; this is not round-to-nearest.
                let mut a = (i + frames / 4) as f64;
                let mut b = (i + frames / 4 + 1) as f64;
                a *= 3.14159 * 2.0;
                b *= 3.14159 * 2.0;
                a /= frames as f64;
                b /= frames as f64;
                class.delta_x[j][i] = ((b.cos() - a.cos()) * 2000.0 + 0.5) as i32;
                let mut c = (i % (frames / 2)) as f64;
                c *= 3.14159 * 2.0;
                c /= frames as f64;
                class.delta_y[j][i] = ((c.sin() - 0.8) * 300.0) as i32;
            }
        }
        for i in 0..200 {
            class.flowers[i].kind = if i < 50 {
                2
            } else if i < 85 {
                1
            } else {
                0
            };
            class.replace(i, canvas, rng)?;
        }
        // CAllEffect::Init invokes this even when both constructor bitmap
        // probes failed: 1400 Source32 draws or 1000 MUGEN2 draws.
        Ok(class)
    }
    fn replace(&mut self, i: usize, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        let kind = self.flowers[i].kind;
        let mut position = [0; 2];
        for axis in 0..2 {
            let low = rng.next_draw()?;
            let random = if self.expanded_random {
                low + 32768 * (rng.next_draw()? % 16)
            } else {
                low
            };
            position[axis] = ((random % (canvas[axis] * 40)) as i32).wrapping_mul(64) / SIZES[kind];
        }
        let speed = [
            (rng.next_draw()? % 600) as i32 - 300,
            (rng.next_draw()? % 150) as i32 + 200,
        ];
        let frames = (rng.next_draw()? % 10) as i32 + 30;
        self.flowers[i] = Flower {
            kind,
            position,
            speed,
            frames,
            ..Default::default()
        };
        self.xy(i);
        Ok(())
    }
    fn xy(&mut self, i: usize) {
        let f = &mut self.flowers[i];
        f.point = f.position.map(|v| v.wrapping_mul(SIZES[f.kind]) / 2560);
        let phase = f.count.wrapping_mul(8) / f.frames;
        f.pattern = if phase >= 5 { 8 - phase } else { phase };
    }
    pub(crate) fn advance_print(&mut self, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        if self.picture.is_none() {
            return Ok(());
        }
        for i in 0..200 {
            let f = &mut self.flowers[i];
            f.count += 1;
            if f.count >= f.frames {
                f.count = 0;
            }
            f.position[0] = f.position[0]
                .wrapping_add(f.speed[0])
                .wrapping_add(self.delta_x[(f.frames - 30) as usize][f.count as usize]);
            f.position[1] = f.position[1]
                .wrapping_add(f.speed[1])
                .wrapping_add(self.delta_y[(f.frames - 30) as usize][f.count as usize]);
            self.xy(i);
            if self.flowers[i].point[1] >= canvas[1] as i32 {
                self.replace(i, canvas, rng)?;
                self.flowers[i].position[1] = -1000;
                self.xy(i);
            }
        }
        Ok(())
    }
    pub(crate) fn image_input(&self, p: &[i32; 4], base: ImageInput) -> Result<Option<ImageInput>> {
        let Some(picture) = &self.picture else {
            return Ok(None);
        };
        let resource = if picture.has_mask {
            picture.raw.clone()
        } else {
            picture.keyed.clone()
        };
        let mut layers = Vec::with_capacity(p[3].clamp(0, 200) as usize);
        for f in &self.flowers[..p[3].clamp(0, 200) as usize] {
            if (f.kind as i32) < p[1] || (f.kind as i32) > p[2] {
                continue;
            }
            let size = SIZES[f.kind];
            let Some((source, destination)) =
                crysirror_image::coverage_bounds::intersect_source_window(
                    [size * f.pattern, TOPS[f.kind], size, size],
                    f.point,
                    picture.source_clip,
                )?
            else {
                continue;
            };
            layers.push(ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(resource.clone())),
                source,
                destination,
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::MaskSeparate256,
            });
        }
        Ok(Some(ImageInput::Composite {
            base: Box::new(base),
            layers,
        }))
    }
}
// Asset constructor probes sys/ta_eff_saku32-bit then sys/eff_saku indexed.
// BeginPrint resets the class-wide once flag; only its FIRST visible Print
// advances all200 objects, regardless of that slot's requested render count.
// SetParam does not reinitialize this bank. Calcu empty; inherited raw restore
// hook empty; countMax99999, CountIsMaxTRUE, inherited NeedPicPrintTRUE.
