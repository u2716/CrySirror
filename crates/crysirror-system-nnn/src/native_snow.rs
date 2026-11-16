// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SNOW14 and SNOW2 57 class-wide100-sprite banks. Both complete original
//! classes/headers were inspected. Only their initial coordinate RNG differs;
//! the original snow bitmap remains a shared engine image, never private pixels.
use crate::image_bridge::NativePictureViews;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use crysirror_image::temporal_pattern::PatternRng;

const SIZES: [i32; 6] = [64, 48, 32, 24, 16, 8];
const TOPS: [i32; 6] = [0, 64, 112, 144, 168, 184];
#[derive(Clone, Copy, Debug, Default)]
struct Flake {
    kind: usize,
    pattern: i32,
    point: [i32; 2],
    position: [i32; 2],
    speed: [i32; 2],
    count: i32,
    frames: i32,
}
#[derive(Clone)]
pub(crate) struct SnowClass {
    expanded_random: bool,
    picture: Option<NativePictureViews>,
    flakes: [Flake; 100],
    count: usize,
    delta: [[i32; 30]; 10],
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 3] {
    std::array::from_fn(|i| values.get(i).copied().unwrap_or([20, 0, 5][i]))
}
impl SnowClass {
    pub(crate) fn new(
        canvas: [u32; 2],
        expanded_random: bool,
        picture: Option<NativePictureViews>,
        rng: &mut PatternRng,
    ) -> Result<Self> {
        let found = picture.is_some();
        let mut state = Self {
            expanded_random,
            picture,
            flakes: [Flake::default(); 100],
            count: if found { 100 } else { 1 },
            delta: [[0; 30]; 10],
        };
        // Failed bitmap load returns from the original constructor BEFORE
        // Init(max100) and InitSnow. CAllEffect's final all-class Init() still
        // sets count100, without initializing particles or drawing RNG values.
        if found {
            state.reset(canvas, rng)?;
        }
        state.count = 100;
        Ok(state)
    }
    pub(crate) fn reset(&mut self, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN snow canvas"
        );
        for j in 0..10 {
            let frames = j + 20;
            for i in 0..frames {
                // Keep both independent divisions and the asymmetric+.5/trunc.
                let a = ((i as f64) * (3.14159 * 2.0) / (frames as f64)).cos();
                let b = (((i + 1) as f64) * (3.14159 * 2.0) / (frames as f64)).cos();
                self.delta[j][i] = ((b - a) * 1000.0 + 0.5) as i32;
            }
        }
        for i in 0..self.count {
            self.replace(i, canvas, rng)?;
        }
        Ok(())
    }
    fn replace(&mut self, i: usize, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        let r = rng.next_draw()? % 25600;
        let kind = if r < 1 {
            0
        } else if r < 10 {
            1
        } else if r < 2000 {
            2
        } else if r < 6000 {
            3
        } else if r < 14000 {
            4
        } else {
            5
        };
        let pattern = (rng.next_draw()? % 4) as i32;
        let mut coordinates = [0; 2];
        for axis in 0..2 {
            let low = rng.next_draw()?;
            let random = if self.expanded_random {
                low + 32768 * (rng.next_draw()? % 16)
            } else {
                low
            };
            coordinates[axis] =
                ((random % (canvas[axis] * 40)) as i32).wrapping_mul(64) / SIZES[kind];
        }
        let speed = [
            (rng.next_draw()? % 300) as i32 - 150,
            (rng.next_draw()? % 200) as i32 + 500,
        ];
        let frames = (rng.next_draw()? % 10) as i32 + 20;
        self.flakes[i] = Flake {
            kind,
            pattern,
            point: [0, 0],
            position: coordinates,
            speed,
            count: 0,
            frames,
        };
        self.xy(i);
        Ok(())
    }
    fn xy(&mut self, i: usize) {
        let f = &mut self.flakes[i];
        f.point = f.position.map(|p| p.wrapping_mul(SIZES[f.kind]) / 2560);
    }
    /// Native BeginPrint clears a class-wide flag. The ordered frame owner
    /// calls this once on the FIRST visible Print of this class, never Calcu.
    pub(crate) fn advance_print(&mut self, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        if self.picture.is_none() {
            return Ok(());
        }
        for i in 0..self.count {
            let f = &mut self.flakes[i];
            f.count += 1;
            if f.count >= f.frames {
                f.count = 0;
            }
            f.position[0] = f.position[0]
                .wrapping_add(f.speed[0])
                .wrapping_add(self.delta[(f.frames - 20) as usize][f.count as usize]);
            f.position[1] = f.position[1].wrapping_add(f.speed[1]);
            self.xy(i);
            if self.flakes[i].point[1] >= canvas[1] as i32 {
                self.replace(i, canvas, rng)?;
                self.flakes[i].position[1] = -1000;
                self.xy(i);
            }
        }
        Ok(())
    }
    pub(crate) fn image_input(&self, p: &[i32; 3], base: ImageInput) -> Result<Option<ImageInput>> {
        let Some(picture) = &self.picture else {
            return Ok(None);
        };
        let first = if p[1] == -1 { 0 } else { p[1] };
        let last = if p[2] == -1 { 5 } else { p[2] };
        let resource = if picture.has_mask {
            picture.raw.clone()
        } else {
            picture.keyed.clone()
        };
        let mut layers = Vec::with_capacity(self.count);
        for f in &self.flakes[..self.count] {
            if (f.kind as i32) < first || (f.kind as i32) > last {
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
// CountIsMaxFALSE, Calcu empty, inherited CheckNeedPicPrintTRUE. Every setter
// (also raw InitParameter) resets the class bank and consumes its full RNG
// sequence. The bank itself is not in EFFECT's original3-word save parameters.
