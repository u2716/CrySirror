// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! RAIN13's original single class-wide particle bank and RNG stream. Native
//! motion/lifecycle stays here; shared IntegerLines owns every rendered pixel.
use anyhow::{Result, ensure};
use crysirror_image::{integer_lines::IntegerLine, temporal_pattern::PatternRng};
#[derive(Clone, Copy, Debug, Default)]
struct Drop {
    x: i32,
    y: i32,
    direction: i32,
    length: i32,
    speed: i32,
}
#[derive(Clone, Debug)]
pub(crate) struct RainClass {
    parameters: [i32; 8],
    drops: Vec<Drop>,
}
pub(crate) fn parameters(values: &[i32]) -> [i32; 8] {
    let mut p = std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 30,
            2 => 100,
            3 => 60,
            4 => 80,
            5 => 255,
            _ => 0,
        })
    });
    if values.len() < 7 {
        p[6] = p[5];
    }
    if values.len() < 8 {
        p[7] = p[6];
    }
    p[2] = p[2].min(150);
    p[3] = p[3].max(1);
    p[4] = p[4].max(1);
    p
}
impl RainClass {
    /// SetParam on ANY effect slot overwrites one CEffectRain member bank.
    /// InitParameter calls this setter and consumes the same complete RNG
    /// prefix; it is not raw hydration of an independent per-slot particle list.
    pub(crate) fn set(p: [i32; 8], canvas: [u32; 2], rng: &mut PatternRng) -> Result<Self> {
        Self::validate(p, canvas)?;
        let mut class = Self {
            parameters: p,
            drops: vec![Drop::default(); p[2].max(0) as usize],
        };
        for i in 0..class.drops.len() {
            class.replace(i, canvas, rng)?;
            let range = canvas[1]
                .checked_add(p[4] as u32)
                .ok_or_else(|| anyhow::anyhow!("NNN rain initial height overflow"))?;
            let y = (rng.next_draw()? % range) as i32;
            class.drops[i].y = y.wrapping_sub(p[4]).wrapping_mul(256);
        }
        Ok(class)
    }
    fn validate(p: [i32; 8], canvas: [u32; 2]) -> Result<()> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN RAIN canvas bounds"
        );
        ensure!(
            p[2] <= 150 && p[3] >= 1 && p[4] >= 1,
            "NNN RAIN native count/divisor words"
        );
        ensure!(
            p[3] <= 1_000_000 && p[4] <= 1_000_000,
            "NNN RAIN fixed-point motion budget"
        );
        Ok(())
    }
    fn replace(&mut self, index: usize, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        let p = self.parameters;
        let half_speed = (p[3] / 2).max(1);
        let half_length = (p[4] / 2).max(1);
        // Keep all FIVE source calls, including range1 draws whose numeric
        // result is always zero. Initialization adds a sixth Y draw per drop.
        let direction = p[1]
            .wrapping_add((rng.next_draw()? % 16) as i32)
            .wrapping_sub(8);
        let displacement = direction.wrapping_mul(canvas[1] as i32) / 256;
        // abs(INT_MIN)/subsequent double-width overflow had no native defined
        // domain. Limit that exceptional range while retaining the draw order.
        let displacement = displacement
            .checked_abs()
            .unwrap_or(i32::MAX)
            .min((i32::MAX - canvas[0] as i32) / 2);
        let width = canvas[0] as i32 + displacement * 2;
        let x = (rng.next_draw()? % width as u32) as i32 - displacement;
        let y = -((rng.next_draw()? % p[3] as u32) as i32);
        let speed = (rng.next_draw()? % half_speed as u32) as i32 + p[3];
        let length = half_length + (rng.next_draw()? % p[4] as u32) as i32;
        self.drops[index] = Drop {
            x: x.wrapping_mul(256),
            y: y.wrapping_mul(256),
            direction: direction.wrapping_mul(256),
            length,
            speed,
        };
        Ok(())
    }
    /// One call per active RAIN slot during the native all-slot Calcu pass.
    /// Prints then all see the final shared bank, not their intermediate clone.
    pub(crate) fn calcu(&mut self, canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
        Self::validate(self.parameters, canvas)?;
        for i in 0..self.drops.len() {
            let drop = &mut self.drops[i];
            drop.x = drop
                .x
                .wrapping_add(drop.direction.wrapping_mul(drop.speed) / 256);
            drop.y = drop.y.wrapping_add(drop.speed.wrapping_mul(256));
            if drop.y >= (canvas[1] as i32).wrapping_mul(256) {
                self.replace(i, canvas, rng)?;
            }
        }
        Ok(())
    }
    pub(crate) fn lines(&self, canvas: [u32; 2]) -> Result<Vec<IntegerLine>> {
        Self::validate(self.parameters, canvas)?;
        let p = self.parameters;
        let packed = p[5].wrapping_shl(16) | p[6].wrapping_shl(8) | p[7];
        let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255];
        let mut lines = Vec::with_capacity(self.drops.len());
        for drop in &self.drops {
            let y = drop.y / 256;
            if y >= canvas[1] as i32 || y.wrapping_add(drop.length) < 0 {
                continue;
            }
            let x = drop.x.wrapping_add(128) >> 8;
            lines.push(IntegerLine {
                from: [x, y],
                to: [
                    x.wrapping_add(drop.direction.wrapping_mul(drop.length) >> 16),
                    y.wrapping_add(drop.length),
                ],
                color,
            });
        }
        crysirror_image::integer_lines::validate(&lines)?;
        Ok(lines)
    }
}
// CountIsMax TRUE changes no flag/command. CheckNeedPicPrint is inheritedTRUE.
// Source Init/BeginPrint/End are empty. Its unnamed per-particle words are
// class state absent from the raw EFFECT save body; restore reruns SetParam.
