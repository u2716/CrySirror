// SPDX-License-Identifier: MPL-2.0
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
pub(super) struct Field {
    flakes: Vec<Flake>,
    target: usize,
    speed: i32,
    horizontal: bool,
    pub reverse: bool,
    pub trails: bool,
}
struct Flake {
    fixed: [i32; 2],
    velocity: [i32; 2],
    sprite: usize,
    active: bool,
    opacity: u8,
}
impl Field {
    pub fn new(
        rng: &mut PatternRng,
        canvas: [i32; 2],
        kind: &str,
        count: i32,
        signed: i32,
    ) -> Result<Self> {
        let target = count.max(0) as usize;
        ensure!(target <= 4096, "Snow2 particle budget");
        let speed = signed
            .checked_abs()
            .filter(|n| *n > 0 && *n <= 0x7f_ffff - 128)
            .ok_or_else(|| anyhow::anyhow!("invalid Snow2 velocity"))?;
        let mut field = Self {
            flakes: Vec::with_capacity(target),
            target,
            speed,
            horizontal: kind != "Snow2",
            reverse: kind == "Snow2HR",
            trails: signed >= 0,
        };
        for _ in 0..target {
            let flake = field.spawn(rng, canvas, false)?;
            field.flakes.push(flake);
        }
        Ok(field)
    }
    pub fn retarget(&mut self, count: i32) -> Result<()> {
        ensure!(count <= 4096, "Snow2 particle budget");
        self.target = count.max(0) as usize;
        Ok(())
    }
    fn spawn(&self, rng: &mut PatternRng, canvas: [i32; 2], edge: bool) -> Result<Flake> {
        let mut draw = || rng.next_draw().map(|v| v as i32);
        let axis = usize::from(!self.horizontal);
        let mut fixed = [draw()?, draw()?];
        for i in 0..2 {
            let range = canvas[i] + [100, 60][i];
            let v = if edge && i == axis {
                -(fixed[i] % 30)
            } else {
                fixed[i] % range
            };
            fixed[i] = v << 16;
        }
        let base = self.speed << 8;
        let drift = draw()? % base;
        let drift = if draw()? > 16383 { drift } else { -drift };
        let speed = base + draw()? % 65536;
        let velocity = if self.horizontal {
            [speed, drift]
        } else {
            [drift, speed]
        };
        Ok(Flake {
            fixed,
            velocity,
            sprite: usize::from(speed >= base + 16384) + usize::from(speed >= base + 21845),
            active: true,
            opacity: 255,
        })
    }
    pub fn advance(
        &mut self,
        rng: &mut PatternRng,
        canvas: [i32; 2],
        dt: i32,
        elapsed: u32,
        alpha: i32,
    ) -> Result<Vec<(usize, [i32; 2], u8)>> {
        let mut stamps = Vec::with_capacity(self.flakes.len());
        let mut i = 0;
        while i < self.flakes.len() {
            if !self.flakes[i].active {
                self.flakes[i] = self.spawn(rng, canvas, true)?;
            } else {
                let f = &mut self.flakes[i];
                for axis in 0..2 {
                    f.fixed[axis] = f.fixed[axis].wrapping_add(f.velocity[axis].wrapping_mul(dt));
                }
                let pos = [f.fixed[0] / 65536 - 50, f.fixed[1] / 65536 - 30];
                let axis = usize::from(!self.horizontal);
                f.active = pos[axis] < canvas[axis];
                if f.active {
                    stamps.push((
                        f.sprite,
                        pos,
                        (i32::from(f.opacity) * alpha / 255).min(255) as u8,
                    ));
                    f.opacity = f.opacity.saturating_add((elapsed >> 8).clamp(1, 255) as u8);
                } else if self.flakes.len() > self.target {
                    self.flakes.pop();
                }
            }
            i += 1;
        }
        if self.flakes.len() < self.target {
            let mut f = self.spawn(rng, canvas, false)?;
            f.opacity = 0;
            self.flakes.push(f);
        }
        Ok(stamps)
    }
}
