// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SAN15's seven-band class state and five-band software Print plan.
//! Full388-line source/header and MMX read; D3D's triangle path is separate.
//! Additive triangular ramps require a shared image kernel, not VM pixels.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Default)]
struct Band {
    enabled: bool,
    count: i32,
    x_q8: i32,
    delta_q8: i32,
    light_q8: i32,
}
#[derive(Clone, Default)]
pub(crate) struct SunRaysClass {
    bands: [Band; 7],
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 3] {
    std::array::from_fn(|i| values.get(i).copied().unwrap_or([15, 0, 0][i]))
}
#[derive(Clone, Copy)]
pub(crate) struct Ramp {
    pub x_q8: i32,
    pub delta_x_q8: i32,
    pub light_q8: i32,
    pub delta_y_q8: i32,
    pub half_width: u32,
    pub delta_light_q8: i32,
    pub rows: u32,
}
impl SunRaysClass {
    pub(crate) fn image_input(
        &mut self,
        count: i32,
        frames: i32,
        canvas: [u32; 2],
        base: crysirror_core::image_processing::ImageInput,
        rng: &mut PatternRng,
    ) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
        use crysirror_image::{
            effects::ImageEffect,
            stored_word_spans::{StoredWordMath, StoredWordSpan, StoredWordSpans},
        };
        let ramps = self.print_plan(count, frames, canvas[1], rng)?;
        let mut spans = Vec::with_capacity(ramps.len() * canvas[1] as usize);
        for ramp in ramps {
            for y in 0..ramp.rows {
                let x = ramp
                    .x_q8
                    .wrapping_add(ramp.delta_x_q8.wrapping_mul(y as i32))
                    >> 8;
                let light = ramp
                    .light_q8
                    .wrapping_add(ramp.delta_y_q8.wrapping_mul(y as i32));
                if x < -48 || x >= canvas[0] as i32 + 48 || light < -256 * 24 * 8 {
                    continue;
                }
                for (start_x, start_light, step) in [
                    (x, light, ramp.delta_light_q8),
                    (
                        x.wrapping_add(24),
                        light.wrapping_add(24i32.wrapping_mul(ramp.delta_light_q8)),
                        ramp.delta_light_q8.wrapping_neg(),
                    ),
                ] {
                    spans.push(StoredWordSpan {
                        destination: [start_x, y as i32, 24],
                        math: StoredWordMath::SignedRampAdd {
                            start: start_light,
                            step,
                            shift: 8,
                        },
                    });
                }
            }
        }
        // Height1 had a zero MMX do/while counter and read beyond the screen;
        // the explicit zero-row plan bounds that undefined case to no writes.
        if spans.is_empty() {
            return Ok(None);
        }
        let spec = StoredWordSpans { spans };
        spec.validate()?;
        Ok(Some(
            crysirror_core::image_processing::ImageInput::Process {
                input: Box::new(base),
                effects: serde_json::to_vec(&[ImageEffect::StoredWordSpans { spec }])?.into(),
            },
        ))
    }
    pub(crate) fn initialize(
        &mut self,
        h: i32,
        canvas: [u32; 2],
        rng: &mut PatternRng,
    ) -> Result<()> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN sun-rays canvas"
        );
        let direction = 1i32.wrapping_sub(2i32.wrapping_mul(h));
        for i in 0..7 {
            let x = (canvas[0] as i32)
                .wrapping_mul(h)
                .wrapping_add(direction.wrapping_mul((i as i32 - 7 / 2 + 1).wrapping_mul(96)))
                .wrapping_mul(256);
            let delta = (i as i32 * 128 + 96).wrapping_mul(direction);
            let light = (7 - i as i32).wrapping_mul((canvas[1] as i32 / 2).wrapping_mul(256)) / 7
                - 336 * 256;
            self.bands[i] = Band {
                enabled: true,
                count: (rng.next_draw()? % 40) as i32 + 20,
                x_q8: x,
                delta_q8: delta,
                light_q8: light,
            };
        }
        Ok(())
    }
    pub(crate) fn print_plan(
        &mut self,
        count: i32,
        frames: i32,
        height: u32,
        rng: &mut PatternRng,
    ) -> Result<Vec<Ramp>> {
        ensure!((1..=16384).contains(&height), "NNN sun-rays row budget");
        let (count, frames) = if frames < 1 { (1, 1) } else { (count, frames) };
        let phase = count.wrapping_mul(256 * 256) / frames;
        let mut plan = Vec::with_capacity(5);
        // Source intentionally stops at SAN_KOSUU-2. The last two records are
        // initialized in class memory, but never ticked or printed.
        for band in &mut self.bands[..5] {
            band.count = band.count.wrapping_sub(1);
            if band.count <= 0 {
                band.enabled = !band.enabled;
                band.count = if band.enabled {
                    (rng.next_draw()? % 160) as i32 + 60
                } else {
                    (rng.next_draw()? % 2) as i32 + 1
                };
            }
            if band.enabled {
                plan.push(Ramp {
                    x_q8: band.x_q8,
                    delta_x_q8: band.delta_q8,
                    light_q8: phase.wrapping_add(band.light_q8),
                    delta_y_q8: -256,
                    half_width: 24,
                    delta_light_q8: 256 * 8,
                    rows: height / 2,
                });
            }
        }
        Ok(plan)
    }
}
// Shared software ramp execution must arithmetic-shift xQ8 (not /256), skip
// a row when x<-48 or x>=width+48 or light<-24*8*256, and retain a48-pixel
// span: first24 add8*256 per pixel, next24 subtract it AFTER each sample.
// Nonnegative light uses LOW WORD(light>>8), unsigned-word-saturated add to
// each RGB byte, then unsigned-byte pack. Negative light skips only that
// pixel. The second group samples the peak BEFORE its first decrement. Alpha remains
// engine coverage, rather than the original unused BGRX high byte.
// SetParam/raw InitParameter initialize7 RNG counters and reset EFFECT count;
// countMax=para0, CountIsMaxTRUE, Calcu empty, NeedPicPrint inheritedTRUE.
