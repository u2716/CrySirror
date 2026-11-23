// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HANABIRA70's per-layer300-object bank, native global configuration and
//! scalar atlas/animation plan. Full587-line class/header reviewed; RGB/mask
//! composition belongs to existing shared row/pair image operations.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Default)]
struct Petal {
    pic: i32,
    reverse: bool,
    position: [i32; 2],
    speed: [i32; 2],
    phase: i32,
    period: i32,
}
#[derive(Clone, Copy)]
struct Config {
    center: [i32; 2],
    radius: i32,
    speed: [i32; 2],
    looping: bool,
    parts: i32,
    frames: i32,
    reverse: bool,
    sway_period: i32,
    anime_type: i32,
}
#[derive(Clone)]
pub(crate) struct HanabiraClass {
    petals: Vec<Petal>,
    config: Config,
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 24] {
    let defaults = [
        20, 100, 100, 100, 0, 1, 600, 300, 100, -100, 10, 0, 0, 0, 100, 100, 100, 50, 1, 1, 0, 0,
        1, 0,
    ];
    // Source omits defaults13/23 but still copies them.13 affects the count
    // clamp despite not selecting a starting object anywhere: define0 on a
    // fresh setter; explicit/restored words retain their actual clamp effect.
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[1] = p[1].clamp(1, 300);
    p[13] = p[13].clamp(0, 299);
    if p[1] + p[13] > 300 {
        p[1] = 300 - p[13];
    }
    p[5] = p[5].max(1);
    p[18] = p[18].max(1);
    p[22] = p[22].max(1);
    p
}
pub(crate) fn validate(p: &[i32; 24]) -> Result<()> {
    ensure!(
        (0..=300).contains(&p[1])
            && p[5] > 0
            && p[18] > 0
            && p[22] > 0
            && i64::from(p[5]) * i64::from(p[18]) * i64::from(p[22]) <= i64::from(i32::MAX),
        "NNN petal count/atlas divisor domain"
    );
    crate::native_curve::validate(p[0], p[21], p[20])
}
fn config(p: &[i32; 24]) -> Config {
    Config {
        center: [p[6], p[7]],
        radius: p[8],
        speed: [p[9], p[10]],
        looping: p[11] != 0,
        parts: p[5],
        frames: p[18],
        reverse: p[19] != 0,
        sway_period: p[17],
        anime_type: p[22],
    }
}
pub(crate) struct Sprite {
    pub destination: [i32; 4],
    pub first: [i32; 2],
    pub second: [i32; 2],
    pub percentages: [i32; 2],
    pub interpolate: bool,
}
impl HanabiraClass {
    pub(crate) fn initialize(layers: usize, rng: &mut PatternRng) -> Result<Self> {
        ensure!(
            matches!(layers, 16 | 32),
            "NNN petal layer-allocation profile"
        );
        let mut class = Self {
            petals: vec![Petal::default(); layers * 300],
            config: Config {
                center: [600, 300],
                radius: 100,
                speed: [-100, 10],
                looping: false,
                parts: 1,
                frames: 1,
                reverse: false,
                sway_period: 0,
                anime_type: 1,
            },
        };
        // The constructor never initialized m_yureTH before SetNewHanabira.
        // Define that scalar0 (period2), retaining all seven RNG draws/object
        // instead of depending on allocator garbage or allowing a zero divisor.
        for i in 0..class.petals.len() {
            class.replace(i, rng)?;
        }
        Ok(class)
    }
    pub(crate) fn set(&mut self, p: &[i32; 24], slot: i32, rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        let slot = slot.max(0) as usize;
        ensure!(slot < self.petals.len() / 300, "NNN petal SetParam layer");
        if p[12] == 0 {
            self.config = config(p);
            for i in 0..p[1] as usize {
                self.replace(slot * 300 + i, rng)?;
            }
        }
        Ok(())
    }
    fn replace(&mut self, i: usize, rng: &mut PatternRng) -> Result<()> {
        let c = self.config;
        let divisor = i64::from(c.parts) * i64::from(c.frames) * i64::from(c.anime_type);
        ensure!(
            (1..=i64::from(i32::MAX)).contains(&divisor),
            "NNN petal picture modulus"
        );
        let pic = (rng.next_draw()? % divisor as u32) as i32;
        // Radius0/negative formerly performed rand()%0 or selected an invalid
        // signed span. Keep the draw and use a bounded positive placement span.
        let spread = i64::from(c.radius)
            .unsigned_abs()
            .saturating_mul(2)
            .clamp(1, u32::MAX as u64) as u32;
        let mut position = [0; 2];
        for a in 0..2 {
            position[a] = c.center[a]
                .wrapping_add((rng.next_draw()? % spread) as i32)
                .wrapping_sub(c.radius / 2)
                .wrapping_mul(100);
        }
        let mut speed = c.speed.map(|v| v.wrapping_mul(100));
        for s in &mut speed {
            let d = (*s / 2).unsigned_abs().max(1);
            *s = s
                .wrapping_add((rng.next_draw()? % d) as i32)
                .wrapping_sub((d / 2) as i32);
        }
        let rd = c.sway_period.max(1) as u32;
        let period = ((rng.next_draw()? % rd) as i32)
            .wrapping_add(c.sway_period / 2)
            .wrapping_add(2)
            .max(1);
        let phase = (rng.next_draw()? % period as u32) as i32;
        let reverse = if c.reverse {
            rng.next_draw()? % 100 < 50
        } else {
            false
        };
        self.petals[i] = Petal {
            pic,
            reverse,
            position,
            speed,
            phase,
            period,
        };
        Ok(())
    }
    fn step(
        &mut self,
        i: usize,
        clip: [i32; 4],
        sway: [f64; 2],
        rng: &mut PatternRng,
    ) -> Result<()> {
        if self.petals[i].pic == -1 {
            if self.config.looping {
                self.replace(i, rng)?;
            } else {
                return Ok(());
            }
        }
        let c = self.config;
        let f = &mut self.petals[i];
        if c.frames > 1 {
            let modulus = c
                .frames
                .checked_mul(c.anime_type)
                .ok_or_else(|| anyhow::anyhow!("NNN petal animation modulus"))?;
            let base = (f.pic / modulus) * modulus;
            let mut phase = f.pic % modulus;
            phase = phase
                .wrapping_add(modulus)
                .wrapping_add(if f.reverse { -1 } else { 1 });
            f.pic = base.wrapping_add(phase % modulus);
        }
        f.phase = (f.phase + 1) % f.period;
        let mut angle = f64::from(f.phase);
        angle *= 3.14159 * 2.0;
        angle /= f64::from(f.period);
        let sin = angle.sin();
        for a in 0..2 {
            let delta = (sin * sway[a] * 0.2 + 0.5) as i32;
            f.position[a] = f.position[a].wrapping_add(f.speed[a]).wrapping_add(delta);
        }
        let [x, y] = f.position.map(|v| v / 100);
        if (if f.speed[0] < 0 {
            x < clip[0]
        } else {
            x > clip[2]
        }) || (if f.speed[1] < 0 {
            y < clip[1]
        } else {
            y > clip[3]
        }) {
            f.pic = -1;
        }
        Ok(())
    }
    pub(crate) fn print_plan(
        &mut self,
        p: &[i32; 24],
        slot: i32,
        count: i32,
        frames: i32,
        canvas: [u32; 2],
        picture_size: [u32; 2],
        rng: &mut PatternRng,
    ) -> Result<Vec<Sprite>> {
        validate(p)?;
        let divisor = frames.max(1);
        if count >= divisor || p[4] == -1 {
            return Ok(vec![]);
        }
        // A valid bank slot always has a CPicture, even before its RGB
        // allocation exists; InitWork dimensions are1x1. The caller supplies
        // metadata independently of whether an engine image lease exists.
        let size = picture_size;
        let [w, h] = [p[2], p[3]];
        if size.contains(&0) || w < 1 || h < 1 {
            return Ok(vec![]);
        }
        let columns = size[0] as i32 / w;
        let rows = size[1] as i32 / h;
        if columns < 1 || rows < 1 {
            return Ok(vec![]);
        }
        let slot = slot.max(0) as usize;
        ensure!(slot < self.petals.len() / 300, "NNN petal Print layer");
        self.config = config(p);
        let clip = [
            -w / 2,
            -h / 2,
            (canvas[0] as i32).saturating_add(w / 2),
            (canvas[1] as i32).saturating_add(h / 2),
        ];
        // The source computes sqrt(speedX²+speedY²) but never uses it. Do not
        // normalize sway by that discarded result.
        let scale = f64::from(p[16].wrapping_mul(100));
        let sway = [
            f64::from(p[10].wrapping_neg()) * scale / 100.0,
            f64::from(p[9]) * scale / 100.0,
        ];
        for i in 0..p[1] as usize {
            self.step(slot * 300 + i, clip, sway, rng)?;
        }
        let percent = crate::native_curve::value_with_random(
            count, divisor, p[15], p[14], p[21], p[20], rng,
        )?
        .clamp(0, 100);
        if percent == 0 {
            return Ok(vec![]);
        }
        let mut plan = Vec::with_capacity(p[1] as usize);
        for f in &self.petals[slot * 300..slot * 300 + p[1] as usize] {
            if f.pic == -1 {
                continue;
            }
            let [x, y] = f.position.map(|v| v / 100);
            if x < clip[0] || x >= clip[2] || y < clip[1] || y >= clip[3] {
                continue;
            }
            let first = f.pic / p[22];
            let mut second = first + 1;
            if second % p[18] == 0 {
                second = (first / p[18]) * p[18];
            }
            if f.reverse {
                second = if first % p[18] == 0 {
                    first + p[18] - 1
                } else {
                    first - 1
                };
            }
            let second_percent = ((f.pic % p[22]) * 100) / p[22];
            let weights = if p[22] <= 1 {
                [percent, 0]
            } else {
                [
                    (100 - second_percent) * percent / 100,
                    second_percent * percent / 100,
                ]
            };
            let cell = |id: i32| {
                [
                    (id % columns).wrapping_mul(w),
                    (id / columns).wrapping_mul(h),
                ]
            };
            plan.push(Sprite {
                destination: [x - w / 2, y - h / 2, w, h],
                first: cell(if p[22] <= 1 { f.pic } else { first }),
                second: cell(second),
                percentages: weights,
                interpolate: p[22] > 1,
            });
        }
        Ok(plan)
    }
}
// Class constructor allocates300*LAYER_KOSUU_MAX and initializes EVERY entry.
// Fresh SetParam/raw InitParameter reset the selected prefix unless para12;
// para13 limits its length but never offsets the actual prefix. Print advances
// each selected object before evaluating opacity, on EACH visible slot Print.
// Calcu empty, CountIsMaxFALSE, NeedPicPrintFALSE. Pair mode uses CPicture's
// ChangeTranslateBlt clip-union/ClipEtc sequence, not CHANGE40's effect guards.
