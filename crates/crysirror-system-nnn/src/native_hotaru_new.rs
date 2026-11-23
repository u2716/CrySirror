// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HOTARUNEW138 float32 motion, lifetimes, shared600-object bank and atlas
//! plans. Full669-line class/header, CPicture::AddSubBlt and its233-line MMX
//! helper reviewed. Bulk additive raster work must use shared image workers.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Default)]
struct Firefly {
    frame: i32,
    fade_in: i32,
    keep: i32,
    fade_out: i32,
    position: [f32; 2],
    speed: [f32; 2],
    phase: i32,
    period: i32,
}
#[derive(Clone)]
pub(crate) struct HotaruNewClass {
    objects: [Firefly; 600],
    config: [i32; 32],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 32] {
    let defaults = [
        20,
        100,
        16,
        16,
        0,
        1,
        50,
        100,
        canvas[0] as i32 / 2,
        canvas[1] as i32 / 2,
        100,
        100,
        -2,
        -3,
        0,
        5,
        10,
        0,
        0,
        0,
        1,
        15,
        -1,
        20,
    ];
    let mut p = std::array::from_fn(|i| {
        values
            .get(i)
            .copied()
            .unwrap_or_else(|| defaults.get(i).copied().unwrap_or(0))
    });
    p[21] = p[21].max(1);
    p[10] = p[10].max(1);
    p[11] = p[11].max(1);
    p
}
pub(crate) fn restored(values: &[i32; 32], canvas: [u32; 2]) -> [i32; 32] {
    // Native InitParameter copies ONLY20 words then calls SetParam(20).
    // Re-population and all three sway words are reset to setter defaults;
    // do not mistakenly replay all32 saved words.
    parameters(&values[..20], canvas)
}
pub(crate) fn validate(p: &[i32; 32]) -> Result<()> {
    ensure!(
        (1..=1_000_000).contains(&p[10])
            && (1..=1_000_000).contains(&p[11])
            && (1..=1_000_000).contains(&p[21])
            && p[14].unsigned_abs() <= 1_000_000
            && p[17].unsigned_abs() <= 1_000_000,
        "NNN new-firefly spawn/random domain"
    );
    Ok(())
}
fn count(p: &[i32; 32]) -> usize {
    p[1].clamp(0, 600) as usize
}
pub(crate) struct Sprite {
    pub destination: [i32; 4],
    pub source: [i32; 2],
    pub percent: i32,
}
impl HotaruNewClass {
    pub(crate) fn image_input(
        &mut self,
        p: &[i32; 32],
        canvas: [u32; 2],
        picture_size: [u32; 2],
        picture: Option<(&crate::image_bridge::NativePictureViews, bool)>,
        base: crysirror_core::image_processing::ImageInput,
        rng: &mut PatternRng,
    ) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
        use crysirror_core::image_processing::ImageInput;
        use crysirror_image::{
            byte_raster::{ByteRaster, ByteRasterMath},
            effects::ImageEffect,
            row_blit::{RowBlit, RowBlitBatch},
        };
        let sprites = self.print_plan(p, canvas, picture_size, rng)?;
        // Every object's motion/respawn happens before native AddSubBlt's
        // loadOk check. It reads a raw mask pointer; no allocation is defined
        // as no-write rather than recreating the original null-pointer read.
        let Some((picture, true)) = picture else {
            return Ok(None);
        };
        let Some(mask) = &picture.mask_storage else {
            return Ok(None);
        };
        let mut operations = Vec::with_capacity(sprites.len());
        for sprite in sprites {
            if sprite.percent == 0 {
                continue;
            }
            let Some((source, destination)) =
                crysirror_image::coverage_bounds::intersect_source_window(
                    [
                        sprite.source[0],
                        sprite.source[1],
                        sprite.destination[2],
                        sprite.destination[3],
                    ],
                    [sprite.destination[0], sprite.destination[1]],
                    picture.source_clip,
                )?
            else {
                continue;
            };
            let scale = sprite.percent.wrapping_mul(256) / 100;
            operations.push(RowBlit {
                source,
                destination,
                row_offsets: vec![0; source[3] as usize],
                row_math: vec![],
                math: ByteRasterMath::SignedScaledMaskedRgb256 {
                    scale: scale.wrapping_abs() as i16,
                    subtract: scale < 0,
                },
            });
        }
        if operations.is_empty() {
            return Ok(None);
        }
        let spec = RowBlitBatch { operations };
        spec.validate()?;
        Ok(Some(ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Resource(mask.view.clone())),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::RowBlitBatch { spec },
            }])?
            .into(),
        }))
    }
    pub(crate) fn initialize(rng: &mut PatternRng) -> Result<Self> {
        let mut p = [0; 32];
        for (i, v) in [
            (0, 20),
            (1, 600),
            (2, 16),
            (3, 16),
            (5, 1),
            (7, 100),
            (8, 600),
            (9, 300),
            (10, 100),
            (11, 100),
            (12, -3),
            (13, -2),
            (15, 1),
            (16, 10),
            (20, 1),
            (21, 15),
            (23, 1),
        ] {
            p[i] = v;
        }
        let mut class = Self {
            objects: [Firefly::default(); 600],
            config: p,
        };
        for i in 0..600 {
            class.replace(i, 0, rng)?;
        }
        Ok(class)
    }
    pub(crate) fn set(&mut self, p: &[i32; 32], rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        self.config = *p;
        // The native setter guards SetNew's writes at600 but leaves count
        // unclamped for Print's later array access. Preserve the raw count,
        // bounding only those formerly out-of-allocation Print/Calcu loops.
        for i in 0..count(p) {
            self.replace(i, p[18], rng)?;
        }
        Ok(())
    }
    fn random_multiplier(percent: i32, rng: &mut PatternRng, positive: bool) -> Result<f32> {
        let mut value =
            (100.0_f32 + (rng.next_draw()? % (percent as u32 * 2)) as f32) - (percent / 2) as f32;
        if positive && value < 1.0 {
            value = 1.0;
        }
        Ok(value)
    }
    fn replace(&mut self, i: usize, random_start: i32, rng: &mut PatternRng) -> Result<()> {
        let p = &self.config;
        validate(p)?;
        let position = [
            p[8].wrapping_add((rng.next_draw()? % p[10] as u32) as i32) as f32,
            p[9].wrapping_add((rng.next_draw()? % p[11] as u32) as i32) as f32,
        ];
        let mut multipliers = [1.0_f32; 2];
        if p[14] > 0 {
            for v in &mut multipliers {
                *v = Self::random_multiplier(p[14], rng, true)? / 100.0_f32;
            }
        }
        let speed = [p[12] as f32 * multipliers[0], p[13] as f32 * multipliers[1]];
        let period = (rng.next_draw()? % (p[21] as u32 * 4)) as i32 + p[21];
        let phase = (rng.next_draw()? % period as u32) as i32;
        let mut fade_in = p[15];
        let mut fade_out = p[16];
        if p[17] > 0 {
            fade_in =
                ((fade_in as f32 * Self::random_multiplier(p[17], rng, false)?) / 100.0_f32) as i32;
            fade_out = ((fade_out as f32 * Self::random_multiplier(p[17], rng, false)?) / 100.0_f32)
                as i32;
            fade_in = fade_in.max(0);
            fade_out = fade_out.max(0);
        }
        let mut keep = p[0].wrapping_sub(fade_in).wrapping_sub(fade_out).max(1);
        if p[17] > 0 {
            keep = ((keep as f32 * Self::random_multiplier(p[17], rng, false)?) / 100.0_f32) as i32;
            keep = keep.max(1);
        }
        let duration = fade_in.wrapping_add(keep).wrapping_add(fade_out);
        let frame = if random_start > 0 && duration > 0 {
            (rng.next_draw()? % duration as u32) as i32
        } else {
            0
        };
        self.objects[i] = Firefly {
            position,
            speed,
            period,
            phase,
            fade_in,
            keep,
            fade_out,
            frame,
        };
        Ok(())
    }
    fn step(
        &mut self,
        i: usize,
        clip: [i64; 4],
        sway: [f64; 2],
        rng: &mut PatternRng,
    ) -> Result<()> {
        let p = self.config;
        if self.objects[i].period == 0 {
            if p[20] != 0 {
                self.replace(i, 0, rng)?;
            }
            return Ok(());
        }
        let f = &mut self.objects[i];
        f.frame = f.frame.wrapping_add(1);
        if f.frame > f.fade_in.wrapping_add(f.keep).wrapping_add(f.fade_out) {
            if p[19] == 0 {
                f.period = 0;
                return Ok(());
            }
            f.frame = 0;
        }
        for a in 0..2 {
            f.position[a] += f.speed[a];
        }
        f.phase = f.phase.wrapping_add(1);
        if f.phase >= f.period {
            f.phase -= f.period;
        }
        let mut th = f64::from(f.phase);
        th *= 3.14159 * 2.0;
        th /= f64::from(f.period);
        let wave = th.cos() * f64::from(p[23]);
        let mut delta = [0.0_f32; 2];
        if p[22] == -1 {
            // Source literal0.2f is rounded tofloat32 BEFORE promotion for
            // the double product. The explicit-angle branch uses double0.2.
            delta[if f.speed[0].abs() > f.speed[1].abs() {
                1
            } else {
                0
            }] = (wave * f64::from(0.2_f32) + 0.5) as f32;
        } else {
            for a in 0..2 {
                delta[a] = (wave * sway[a] + 0.5) as f32;
            }
        }
        for a in 0..2 {
            f.position[a] += delta[a];
        }
        for a in 0..2 {
            let value = f64::from(f.position[a]);
            if (f.speed[a] < 0.0 && value < clip[a] as f64)
                || (f.speed[a] >= 0.0 && value > clip[a + 2] as f64)
            {
                f.period = 0;
                return Ok(());
            }
        }
        Ok(())
    }
    pub(crate) fn print_plan(
        &mut self,
        p: &[i32; 32],
        canvas: [u32; 2],
        picture_size: [u32; 2],
        rng: &mut PatternRng,
    ) -> Result<Vec<Sprite>> {
        validate(p)?;
        // Deliberate native ordering: clip derives from PREVIOUS class cell
        // dimensions before GetInitParam overwrites them for this slot.
        let clip = [
            -i64::from(self.config[2]) / 2,
            -i64::from(self.config[3]) / 2,
            i64::from(canvas[0]) + i64::from(self.config[2]) / 2,
            i64::from(canvas[1]) + i64::from(self.config[3]) / 2,
        ];
        self.config = *p;
        let sway = if p[22] == -1 {
            [0.2, 0.0]
        } else {
            let mut th = f64::from(p[22]);
            th *= 3.14159 * 2.0;
            th /= 360.0;
            [0.2 * th.cos(), 0.2 * th.sin()]
        };
        for i in 0..count(p) {
            self.step(i, clip, sway, rng)?;
        }
        // Lifetime/repopulation/RNG occurs before ALL image and cell guards.
        let [w, h] = [p[2], p[3]];
        if w <= 0 || h <= 0 || picture_size.contains(&0) {
            return Ok(vec![]);
        }
        let columns = picture_size[0] as i32 / w;
        let rows = picture_size[1] as i32 / h;
        if columns <= 0 || rows <= 0 {
            return Ok(vec![]);
        }
        let mut sprites = Vec::with_capacity(count(p));
        for (i, f) in self.objects[..count(p)].iter().enumerate() {
            if f.period == 0 {
                continue;
            }
            let mut anime = f.frame;
            if p[5] > 1 {
                anime /= p[5];
            }
            anime %= columns;
            let [x, y] = f.position.map(|v| v as i32);
            let mut percent = if f.frame < f.fade_in {
                p[6].wrapping_add(p[7].wrapping_sub(p[6]).wrapping_mul(f.frame) / f.fade_in.max(1))
            } else if f.frame < f.fade_in.wrapping_add(f.keep) {
                p[7]
            } else if f.frame < f.fade_in.wrapping_add(f.keep).wrapping_add(f.fade_out) {
                p[7].wrapping_sub(
                    p[7].wrapping_sub(p[6])
                        .wrapping_mul(f.frame.wrapping_sub(f.fade_in.wrapping_add(f.keep)))
                        / f.fade_out.max(1),
                )
            } else {
                p[6]
            };
            percent = percent.max(p[6]).min(p[7]);
            if p[4] == 1 {
                percent = percent.wrapping_neg();
            }
            sprites.push(Sprite {
                destination: [x.wrapping_sub(w / 2), y.wrapping_sub(h / 2), w, h],
                source: [anime.wrapping_mul(w), (i as i32 % rows).wrapping_mul(h)],
                percent,
            });
        }
        Ok(sprites)
    }
}
// Constructor has600 objects and four RNG draws/object by default, independent
// of resources. Every slot shares that same bank, each Print advances again;
// no once-per-frame flag. EFFECT.count/countMax do not gate this Print.
// Raw native periods/frame sums that formerly overflowed signed integers use
// explicit wrapping; float32 positions/speeds retain separate source stages.
// AddSubBlt checks loadOk/indexed/CharaClip, then its shared software profile
// scales colour by abs(percent*256/100), saturates stored RGB add/sub, and
// floors mask/destination products separately INCLUDING mask255 (no copy
// shortcut). A missing native mask pointer was undefined; define no write.
// Calcu empty; CountIsMaxFALSE; NeedPicPrintFALSE. Unread omitted24..31 ->0.
