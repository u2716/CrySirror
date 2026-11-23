// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! GRASSPLANE93 original shared soliton bank and depth/wind phase. Generic
//! RowBlit performs the opaque displaced-row copies; pixels stay in the engine.
use crate::native_picture_effect::PictureEffectFrame;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};
#[derive(Clone, Copy, Debug, Default)]
struct Soliton {
    wait: i32,
    speed: i32,
    y: i32,
    power: i32,
    width: i32,
}
#[derive(Clone, Debug)]
pub(crate) struct GrassClass {
    near: f64,
    far: f64,
    dots: f64,
    length: f64,
    solitons: [Soliton; 20],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 12] {
    let defaults = [
        20,
        canvas[1] as i32 / 2,
        canvas[1] as i32 / 2,
        500,
        100,
        0,
        100,
        5,
        100,
        50,
        20,
        -1,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[1] = p[1].clamp(0, canvas[1] as i32 - 1);
    p[2] = p[2].clamp(1, canvas[1] as i32).min(canvas[1] as i32 - p[1]);
    if p[3] <= p[4] {
        p[3] = p[4].saturating_add(1);
    }
    // Source writes beyond its20-member arrays when requested more solitons.
    p[7] = p[7].min(20);
    p
}
pub(crate) fn validate(p: &[i32; 12]) -> Result<()> {
    ensure!(
        p[2] > 0 && p[2] <= 16384 && p[3].checked_sub(p[4]).is_some_and(|n| n > 0) && p[7] <= 20,
        "NNN GRASSPLANE depth/work bounds"
    );
    Ok(())
}
impl GrassClass {
    pub(crate) fn set(p: &[i32; 12], rng: &mut PatternRng) -> Result<Self> {
        validate(p)?;
        let near = f64::from(p[4]);
        let far = f64::from(p[3]);
        let dots = f64::from(p[2]);
        let mut state = Self {
            near,
            far,
            dots,
            length: far / (far - near) * dots,
            solitons: [Soliton::default(); 20],
        };
        // InitParameter resets ALL20 members, irrespective of requested count.
        // Even overwritten wait draws and modulo1 calls remain in the stream.
        for n in 0..20 {
            state.replace(p, n, true, rng)?;
        }
        Ok(state)
    }
    fn replace(
        &mut self,
        p: &[i32; 12],
        n: usize,
        in_screen: bool,
        rng: &mut PatternRng,
    ) -> Result<()> {
        let mut wait = (rng.next_draw()? % 50) as i32;
        let random_power = 50 + (rng.next_draw()? % 100) as i32;
        let power = (p[6].wrapping_mul(p[8]) / 100).wrapping_mul(random_power) / 100;
        let random_speed = 70 + (rng.next_draw()? % 60) as i32;
        let speed = (p[9].wrapping_mul(random_speed) / 100).max(1);
        let mut y = if p[5] == 0 { p[4] } else { p[3] };
        let random_width = 80 + (rng.next_draw()? % 40) as i32;
        let width = (p[10].wrapping_mul(random_width) / 100).max(1);
        if in_screen {
            wait = 0;
            let displacement = (rng.next_draw()? % (p[3].wrapping_sub(p[4])) as u32) as i32;
            y = if p[5] == 0 {
                p[4].wrapping_add(displacement)
            } else {
                p[3].wrapping_sub(displacement)
            };
        }
        self.solitons[n] = Soliton {
            wait,
            speed,
            y,
            power,
            width,
        };
        Ok(())
    }
    pub(crate) fn calcu(&mut self, p: &[i32; 12], rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        for n in 0..p[7].max(0) as usize {
            let s = &mut self.solitons[n];
            if s.wait == 0 {
                if p[5] == 0 {
                    s.y = s.y.wrapping_add(s.speed);
                    if s.y > p[3] {
                        self.replace(p, n, false, rng)?;
                    }
                } else {
                    s.y = s.y.wrapping_sub(s.speed);
                    if s.y < p[4] {
                        self.replace(p, n, false, rng)?;
                    }
                }
            } else {
                s.wait = s.wait.wrapping_sub(1);
            }
        }
        Ok(())
    }
    fn dot_to_distance(&self, dot: f64) -> f64 {
        self.length * self.near / (self.length - self.dots + dot).max(0.1)
    }
    fn distance_to_dot(&self, distance: f64) -> f64 {
        let distance = distance.max(0.1);
        self.dots - (distance - self.near) * self.length / distance
    }
    pub(crate) fn image_input(
        &self,
        frame: &PictureEffectFrame,
        p: &[i32; 12],
        base: ImageInput,
    ) -> Result<Option<ImageInput>> {
        validate(p)?;
        let slot = (if p[11] == -1 { frame.slot } else { p[11] }).clamp(0, frame.bank_limit - 1);
        let Some(picture) = frame.bank.picture(slot).map(|b| &b.picture) else {
            return Ok(None);
        };
        let [width, height] = picture.raw.size();
        let screen_height = frame.canvas[1] as usize;
        let mut phase = f64::from(frame.state.count);
        if p[5] == 1 {
            phase = -phase;
        }
        phase /= f64::from(frame.canvas[1]);
        phase *= 3.14159 * 2.0 * 5.0;
        let mut offsets = Vec::with_capacity(screen_height);
        for row in 0..screen_height {
            let distance = self.dot_to_distance(row as f64);
            let theta = distance * 0.08 + phase;
            // The first pass truncates to INT before storing into doublework.
            offsets.push(f64::from(
                (f64::from(p[6]) * (self.near / distance) * theta.cos() + 0.5) as i32,
            ));
        }
        for s in self
            .solitons
            .iter()
            .take(p[7].max(0) as usize)
            .filter(|s| s.wait == 0)
        {
            let near = f64::from(s.y.wrapping_sub(s.width)).max(1.0);
            let mut far = f64::from(s.y.wrapping_add(s.width));
            if far <= near {
                far = near + 0.1;
            }
            let near_row = self.distance_to_dot(near) as i32;
            let far_row = self.distance_to_dot(far) as i32;
            // Outside rows have no source side effects. Intersecting this
            // iteration avoids huge native temporary-array/OOB traversals.
            for row in far_row.max(0)..=near_row.min(p[2] - 1).min(frame.canvas[1] as i32 - 1) {
                let distance = self.dot_to_distance(f64::from(row));
                let theta = (distance - near) / (far - near) * 3.14159 * 2.0;
                offsets[row as usize] +=
                    (1.0 - theta.cos()) * f64::from(s.power) * (self.near / distance);
            }
        }
        let mut row_offsets: Vec<i32> = offsets.into_iter().map(|v| (v + 0.5) as i32).collect();
        // DeltaBlt clips destination rows but did not bound its source-indexed
        // displacement table for a tall/negative-origin picture. Define any
        // inaccessible tail displacement as0 rather than read outside storage.
        row_offsets.resize(height as usize, 0);
        let spec = RowBlit {
            source: [0, 0, width as i32, height as i32],
            destination: [
                frame.rectangles.destination[0],
                frame.rectangles.destination[1],
            ],
            row_offsets,
            math: ByteRasterMath::CopyRgb,
            row_math: vec![],
        };
        spec.validate()?;
        // para1(startY) and CharaClip are intentionally unused by this source
        // Print/DeltaBlt(FALSE) path. Preserve them as native saved words only.
        Ok(Some(ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Resource(picture.raw.clone())),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::RowBlit { spec },
            }])?
            .into(),
        }))
    }
}
// One bank and four depth constants are shared by all93 slots. Each SetParam
// or raw-save InitParameter replaces them; ordered all-slot Calcu then changes
// the shared bank before any Print. No per-slot clone may hide those writes.
// CountIsMax FALSE, CheckAllPrint TRUE, CheckNeedPicPrint FALSE; other hooks empty.
