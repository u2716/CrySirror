// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HOTARU71's fixed-point600-object bank and procedural-brush placement.
//! Full1031-line class/header read, including distinct D3D/software drawing.
//! The8x8 signed RGB brush and batch saturation belong to shared image jobs.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone, Copy, Default)]
struct Firefly {
    position: [i32; 2],
    speed: [i32; 2],
    phase: i32,
    period: i32,
}
#[derive(Clone)]
pub(crate) struct HotaruClass {
    objects: [Firefly; 600],
    config: [i32; 20],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 20] {
    let defaults = [
        20,
        100,
        canvas[0] as i32,
        canvas[1] as i32 - 180,
        100,
        100,
        -2,
        -3,
        255,
        255,
        255,
        0,
        0,
        0,
        100,
        15,
        -1,
        20,
        0,
        0,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[4] = p[4].max(1);
    p[5] = p[5].max(1);
    p[15] = p[15].max(1);
    if i64::from(p[13]) + i64::from(p[1]) > 600 {
        p[13] = 600i32.wrapping_sub(p[1]);
    }
    p
}
pub(crate) fn validate(p: &[i32; 20]) -> Result<()> {
    ensure!(
        (1..=1_000_000).contains(&p[4])
            && (1..=1_000_000).contains(&p[5])
            && (1..=1_000_000).contains(&p[15]),
        "NNN firefly spawn/random domain"
    );
    Ok(())
}
fn range(start: i32, count: i32) -> std::ops::Range<usize> {
    let end = i64::from(start) + i64::from(count);
    let a = i64::from(start).clamp(0, 600) as usize;
    let b = end.clamp(0, 600) as usize;
    a..b.max(a)
}
pub(crate) struct Paint {
    pub centers: Vec<[i32; 2]>,
    pub color: [i32; 3],
    pub percent: i32,
}
impl HotaruClass {
    pub(crate) fn initialize(rng: &mut PatternRng) -> Result<Self> {
        Self::initialize_with_speed([-3, -2], rng)
    }
    /// Actual MUGEN2 constructor0x42c9e0 stores -100,10 before invoking its
    /// 600-object SetNew0x42d940. Later Source32 stores -3,-2 instead.
    pub(crate) fn initialize_legacy(rng: &mut PatternRng) -> Result<Self> {
        Self::initialize_with_speed([-100, 10], rng)
    }
    fn initialize_with_speed(speed: [i32; 2], rng: &mut PatternRng) -> Result<Self> {
        let mut p = [0; 20];
        for (i, v) in [
            (2, 600),
            (3, 300),
            (4, 100),
            (5, 100),
            (6, speed[0]),
            (7, speed[1]),
            (15, 15),
        ] {
            p[i] = v;
        }
        let mut class = Self {
            objects: [Firefly::default(); 600],
            config: p,
        };
        for i in 0..600 {
            class.replace(i, rng)?;
        }
        Ok(class)
    }
    pub(crate) fn set(&mut self, p: &[i32; 20], rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        if p[12] == 0 {
            self.config = *p;
            for i in range(p[13], p[1]) {
                self.replace(i, rng)?;
            }
        }
        Ok(())
    }
    fn replace(&mut self, i: usize, rng: &mut PatternRng) -> Result<()> {
        let p = &self.config;
        validate(p)?;
        let position = [
            p[2].wrapping_add((rng.next_draw()? % p[4] as u32) as i32)
                .wrapping_mul(256),
            p[3].wrapping_add((rng.next_draw()? % p[5] as u32) as i32)
                .wrapping_mul(256),
        ];
        let mut speed = [p[6].wrapping_mul(256), p[7].wrapping_mul(256)];
        for v in &mut speed {
            let d = (*v / 2).wrapping_abs().max(1);
            *v = v
                .wrapping_add((rng.next_draw()? % d as u32) as i32)
                .wrapping_sub(d / 2);
        }
        let period = (rng.next_draw()? % (p[15] as u32 * 4)) as i32 + p[15];
        let phase = (rng.next_draw()? % period as u32) as i32;
        self.objects[i] = Firefly {
            position,
            speed,
            period,
            phase,
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
        let p = self.config;
        if self.objects[i].period == 0 {
            if p[11] != 0 {
                self.replace(i, rng)?;
            }
            return Ok(());
        }
        let f = &mut self.objects[i];
        for a in 0..2 {
            f.position[a] = f.position[a].wrapping_add(f.speed[a]);
        }
        f.phase = f.phase.wrapping_add(1);
        if f.phase >= f.period {
            f.phase -= f.period;
        }
        let mut th = f64::from(f.phase);
        th *= 3.14159 * 2.0;
        th /= f64::from(f.period);
        let wave = th.cos() * f64::from(p[17].wrapping_mul(256));
        let mut delta = [0; 2];
        if p[16] == -1 {
            delta[if f.speed[0].wrapping_abs() > f.speed[1].wrapping_abs() {
                1
            } else {
                0
            }] = (wave * 0.2 + 0.5) as i32;
        } else {
            for a in 0..2 {
                delta[a] = (wave * sway[a] + 0.5) as i32;
            }
        }
        for a in 0..2 {
            f.position[a] = f.position[a].wrapping_add(delta[a]);
        }
        for a in 0..2 {
            let point = f.position[a] / 256;
            if (f.speed[a] < 0 && point < clip[a]) || (f.speed[a] >= 0 && point > clip[a + 2]) {
                f.period = 0;
                return Ok(());
            }
        }
        Ok(())
    }
    pub(crate) fn print_plan(
        &mut self,
        p: &[i32; 20],
        count: i32,
        frames: i32,
        canvas: [u32; 2],
        rng: &mut PatternRng,
    ) -> Result<Option<Paint>> {
        validate(p)?;
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN firefly canvas"
        );
        self.config = *p;
        let clip = [4, 4, canvas[0] as i32 - 4, canvas[1] as i32 - 4];
        let sway = if p[16] == -1 {
            [0.2, 0.0]
        } else {
            let mut th = f64::from(p[16]);
            th *= 3.14159 * 2.0;
            th /= 360.0;
            [0.2 * th.cos(), 0.2 * th.sin()]
        };
        // The native Print uses para12 (NOT INITIALIZE flag) as its array
        // start, although SetParam initialization uses para13. Keep both.
        let indices = range(p[12], p[1]);
        if indices.is_empty() {
            return Ok(None);
        }
        for i in indices.clone() {
            self.step(i, clip, sway, rng)?;
        }
        let percent = 100i32
            .wrapping_add(p[14].wrapping_sub(100).wrapping_mul(count) / frames.max(1))
            .clamp(0, 100);
        let mut centers = Vec::with_capacity(indices.len());
        for i in indices {
            let f = self.objects[i];
            if f.period == 0 {
                continue;
            }
            // Software MMX arithmetic-shifts Q8 for drawing; its motion-death
            // comparisons above use signed /256. Negative fractions differ.
            let at = f.position.map(|v| v >> 8);
            if at[0] >= clip[0] && at[1] >= clip[1] && at[0] < clip[2] && at[1] < clip[3] {
                centers.push(at);
            }
        }
        Ok(Some(Paint {
            centers,
            color: [p[8], p[9], p[10]],
            percent,
        }))
    }
}
// Shared brush contract: for each8x8 cell, xx=(x-4)*2+1, yy=(y-4)*2+1;
// compute r=sqrt(double(xx²+yy²)), then mm=int(r*r*2), retaining the sqrt
// round-trip rather than algebraically replacing it with2*(xx²+yy²).
// For each SIGNED RGB channel choose add OR subtract, value=clamp0..255
// ((abs(channel)-mm)*percent/100) with explicit bounded32-bit overflow policy.
// Paint the complete8x8 at center-4: saturated BYTE add then BYTE subtract.
// Destination alpha is retained; software does not use the D3D half-range
// encoded brush or source screen-copy texture. No partial-edge sprite writes.
// Constructor has6 draws/object; shared class600 bank advances EACH Print,
// not once-per-frame. Bounded indices replace the old out-of-bank Print reads.
// Omitted18/19 were unread uninitialized words ->0. Raw InitParameter replays
// all20, including conditional initialization. Calcu empty, CountIsMaxFALSE,
// NeedPicPrint inheritedTRUE, no picture/asset guard in the software Print.

impl Paint {
    pub(crate) fn brush_spec(
        &self,
    ) -> Result<crysirror_image::radial_rgb_brushes::RadialRgbBrushes> {
        let spec = crysirror_image::radial_rgb_brushes::RadialRgbBrushes {
            brush_size: [8, 8],
            coordinate_origin: [-7, -7],
            coordinate_step: [2, 2],
            radial_strength: 2,
            signed_rgb: self.color,
            percent: self.percent as u32,
            positions: self.centers.iter().map(|p| [p[0] - 4, p[1] - 4]).collect(),
        };
        spec.validate()?;
        Ok(spec)
    }
}
