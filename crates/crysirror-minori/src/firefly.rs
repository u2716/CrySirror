// SPDX-License-Identifier: MPL-2.0
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;

pub(super) struct Fireflies {
    particles: Vec<Particle>,
    target: usize,
    lifetime: i32,
    pub stem: String,
}
struct Particle {
    controls: [[i32; 2]; 7],
    size: usize,
    life: i32,
    elapsed: i32,
}
impl Particle {
    fn spawn(rng: &mut PatternRng, life: i32, canvas: [i32; 2]) -> Result<Self> {
        let mut draw = || rng.next_draw().map(|x| x as i32);
        let mut controls = [[0; 2]; 7];
        for p in &mut controls {
            for (axis, dim) in canvas.into_iter().enumerate() {
                p[axis] = draw()? % (2 * dim) - dim / 2;
            }
        }
        let size = draw()? as usize % 3;
        let life = life.saturating_sub(life / 2).saturating_add(draw()? % life);
        Ok(Self {
            controls,
            size,
            life,
            elapsed: 0,
        })
    }
    fn sample(&self) -> ([i32; 2], i32) {
        const COEFFICIENTS: [u32; 7] = [1, 6, 15, 20, 15, 6, 1];
        let t = f64::from(self.elapsed) / f64::from(self.life);
        let mut xy = [0.0; 2];
        for (i, p) in self.controls.iter().enumerate() {
            let weight =
                f64::from(COEFFICIENTS[i]) * t.powi(i as i32) * (1.0 - t).powi(6 - i as i32);
            for axis in 0..2 {
                xy[axis] += f64::from(p[axis]) * weight;
            }
        }
        let alpha = if t < 0.255 {
            (t * 1000.0) as i32
        } else if t > 0.755 {
            ((1.0 - t) * 1000.0) as i32
        } else {
            255
        };
        (xy.map(|v| v as i32), alpha)
    }
}
impl Fireflies {
    pub fn new(
        rng: &mut PatternRng,
        canvas: [i32; 2],
        stem: &str,
        count: i32,
        life: i32,
    ) -> Result<Self> {
        let target = count.max(0) as usize;
        let lifetime = life
            .checked_abs()
            .filter(|v| *v > 0)
            .ok_or_else(|| anyhow::anyhow!("invalid minori Firefly lifetime"))?;
        ensure!(target <= 4096, "minori Firefly particle budget");
        let particles = (0..target)
            .map(|_| Particle::spawn(rng, lifetime, canvas))
            .collect::<Result<_>>()?;
        Ok(Self {
            particles,
            target,
            lifetime,
            stem: stem.to_owned(),
        })
    }
    pub fn retarget(&mut self, stem: &str, count: i32, life: i32) -> Result<bool> {
        if self.stem != stem
            || Some(self.lifetime) != life.checked_abs()
            || self.target == count.max(0) as usize
        {
            return Ok(false);
        }
        ensure!(count <= 4096, "minori Firefly particle budget");
        self.target = count.max(0) as usize;
        Ok(true)
    }
    pub fn advance(
        &mut self,
        rng: &mut PatternRng,
        canvas: [i32; 2],
        dt: i32,
        alpha: i32,
    ) -> Result<Vec<(usize, [i32; 2], u8)>> {
        let mut stamps = Vec::with_capacity(self.particles.len());
        for p in &mut self.particles {
            p.elapsed = p.elapsed.saturating_add(dt);
            if p.elapsed >= p.life {
                // The native particle restarts with its own previous lifetime.
                *p = Particle::spawn(rng, p.life, canvas)?;
            }
            let (xy, a) = p.sample();
            stamps.push((p.size, xy, (a * alpha / 255).clamp(0, 255) as u8));
        }
        if self.particles.len() < self.target {
            self.particles
                .push(Particle::spawn(rng, self.lifetime, canvas)?);
        }
        Ok(stamps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curve_reaches_midpoint_and_native_fade_edges() {
        let mut p = Particle {
            controls: [[20, -5]; 7],
            size: 1,
            life: 1000,
            elapsed: 500,
        };
        assert_eq!(p.sample(), ([20, -5], 255));
        p.elapsed = 100;
        assert_eq!(p.sample().1, 100);
        p.elapsed = 875;
        assert_eq!(p.sample().1, 125);
    }
}
