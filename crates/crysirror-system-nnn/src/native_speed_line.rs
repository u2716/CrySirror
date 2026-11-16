// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SPEEDLINE85 shared100-line bank, native RNG/Calcu and ordered strip geometry.
//! Full828-line source/header read. The software three-row bodies differ from
//! D3D, including the bottom solid/ramp overlap. Shared image jobs own mixing.
use anyhow::{Result, ensure};
use crysirror_image::{
    stored_gradient_spans::{StoredGradientSpan, StoredGradientSpans},
    temporal_pattern::PatternRng,
};
#[derive(Clone, Copy, Default)]
struct Line {
    x: i32,
    y: i32,
    speed: i32,
    length: i32,
    thick: bool,
}
#[derive(Clone)]
pub(crate) struct SpeedLineClass {
    lines: [Line; 100],
}
impl Default for SpeedLineClass {
    fn default() -> Self {
        Self {
            lines: [Line::default(); 100],
        }
    }
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 12] {
    let defaults = [
        20,
        10,
        100,
        3,
        30,
        255,
        3,
        3,
        0,
        0,
        canvas[0] as i32,
        canvas[1] as i32,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[2] = p[2].max(81);
    for axis in 0..2 {
        p[8 + axis] = p[8 + axis].max(0).min(canvas[axis] as i32 - 1);
        if p[8 + axis].wrapping_add(p[10 + axis]) > canvas[axis] as i32 {
            p[10 + axis] = canvas[axis] as i32 - p[8 + axis];
        }
    }
    if p[11] < 3 {
        p[9] -= 2;
        p[11] = 3;
        if p[9] < 0 {
            p[9] += 2;
        }
    }
    p
}
fn count(p: &[i32; 12]) -> usize {
    p[1].max(0).min(100) as usize
}
pub(crate) fn validate(p: &[i32; 12]) -> Result<()> {
    ensure!(
        (81..=1_000_000).contains(&p[2]) && p[8..12].iter().all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN speed-line geometry domain"
    );
    Ok(())
}
impl SpeedLineClass {
    fn replace(
        &mut self,
        p: &[i32; 12],
        i: usize,
        in_screen: bool,
        rng: &mut PatternRng,
    ) -> Result<()> {
        validate(p)?;
        let len = p[2];
        let speed = p[4];
        let width = p[10].wrapping_add(len.wrapping_mul(2)).max(1);
        let height = p[11].wrapping_sub(2).max(1);
        let y = p[9].wrapping_add((rng.next_draw()? % height as u32) as i32);
        let x = if in_screen {
            p[8].wrapping_sub(len)
                .wrapping_add((rng.next_draw()? % width as u32) as i32)
        } else if speed < 0 {
            p[8].wrapping_add(p[10])
                .wrapping_add((rng.next_draw()? % (len + 1) as u32) as i32)
        } else {
            p[8].wrapping_sub(len.wrapping_mul(2))
                .wrapping_add((rng.next_draw()? % (len + 1) as u32) as i32)
        };
        let spread = (speed / 4).unsigned_abs().max(1);
        let speed = speed
            .wrapping_add((rng.next_draw()? % (spread * 2)) as i32)
            .wrapping_sub(spread as i32);
        let length = len.wrapping_add((rng.next_draw()? % (len / 2).max(1) as u32) as i32);
        let thick = rng.next_draw()? % 2 != 0;
        self.lines[i] = Line {
            x,
            y,
            speed,
            length,
            thick,
        };
        Ok(())
    }
    pub(crate) fn initialize(&mut self, p: &[i32; 12], rng: &mut PatternRng) -> Result<()> {
        // The original heap buffer has exactly100 records but does not clamp
        // para1. Preserve its raw word and bound only the former OOB loop.
        for i in 0..count(p) {
            self.replace(p, i, true, rng)?;
        }
        Ok(())
    }
    pub(crate) fn calcu(&mut self, p: &[i32; 12], rng: &mut PatternRng) -> Result<()> {
        validate(p)?;
        let left = p[8].wrapping_sub(p[2].wrapping_mul(125) / 100);
        let right = p[8].wrapping_add(p[10]);
        for i in 0..count(p) {
            let line = &mut self.lines[i];
            line.x = line.x.wrapping_add(line.speed);
            if p[4] > 0 && line.x > right || p[4] <= 0 && line.x < left {
                self.replace(p, i, false, rng)?;
            }
        }
        Ok(())
    }
    pub(crate) fn plan(&self, p: &[i32; 12]) -> Result<StoredGradientSpans> {
        validate(p)?;
        let color = [p[5] as u8, p[6] as u8, p[7] as u8];
        let left = i64::from(p[8]);
        let right = left + i64::from(p[10]);
        let mut spans = Vec::with_capacity(count(p) * 6);
        let mut emit = |start: i64, row: i64, length: i64, copy: bool| {
            let x = start.max(left);
            let end = (start + length).min(right);
            if end <= x {
                return;
            }
            let delta = x - start;
            let a = (255 - 4 * delta).clamp(0, 255) as u8;
            let b = (4 * delta).clamp(0, 255) as u8;
            spans.push(StoredGradientSpan {
                destination: [x as i32, row as i32, (end - x) as i32],
                color,
                weights: [a, b],
                steps: if copy { [0, 0] } else { [-4, 4] },
                copy,
            });
        };
        for line in &self.lines[..count(p)] {
            let x = i64::from(line.x);
            let y = i64::from(line.y);
            let len = i64::from(line.length);
            if x >= right || x + len < left {
                continue;
            }
            if line.thick {
                emit(x, y, len - 80, true);
                emit(x + len - 80, y, 64, false);
            }
            emit(x, y + 1, len - 64, true);
            emit(x + len - 64, y + 1, 64, false);
            if line.thick {
                // Bottom solid is len-64, but its ramp begins at len-80. The
                // final16 solid pixels are blended again, as the MMX body does.
                emit(x, y + 2, len - 64, true);
                emit(x + len - 80, y + 2, 64, false);
            }
        }
        let plan = StoredGradientSpans { spans };
        plan.validate()?;
        Ok(plan)
    }
}
// Fresh SetParam resets count/max and invokes InitParameter. Raw restore calls
// only InitParameter: it reseeds this global bank but preserves EFFECT counters.
// Calcu runs per active slot BEFORE all Prints see the final shared bank.
// CountIsMaxFALSE, CheckNeedPicPrintTRUE, CheckAllPrintTRUE. para3/wide is stored
// but software Print uses exactly three rows and the random thick flag.
