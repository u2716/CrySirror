// SPDX-License-Identifier: MPL-2.0
//! Stateless falling-streak field (rain): every streak's position is a pure
//! function of (seed, index, elapsed time), so a caller can redraw any frame
//! without keeping per-particle state. Streaks wrap through the area.

/// Field parameters in area pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreakField {
    pub seed: u32,
    pub count: u32,
    pub area: [f32; 2],
    /// Fall speed in pixels per second (before per-streak variation).
    pub speed: f32,
    /// Horizontal drift per vertical pixel (slant).
    pub slant: f32,
    /// Streak length in pixels.
    pub length: f32,
}

/// One streak: start point, end point and opacity 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Streak {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub opacity: f32,
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}
fn unit(seed: u32, index: u32, lane: u32) -> f32 {
    (hash(seed ^ hash(index.wrapping_mul(4).wrapping_add(lane))) >> 8) as f32 / (1 << 24) as f32
}

impl StreakField {
    pub fn at(&self, elapsed_ms: u64) -> Vec<Streak> {
        let t = elapsed_ms as f32 / 1000.;
        let [w, h] = self.area;
        let span = h + self.length;
        (0..self.count.min(4096))
            .map(|i| {
                let speed = self.speed * (0.75 + 0.5 * unit(self.seed, i, 0));
                let y = (unit(self.seed, i, 1) * span + t * speed).rem_euclid(span) - self.length;
                let x0 = unit(self.seed, i, 2) * (w + h * self.slant.abs()) - h * self.slant.max(0.);
                let x = x0 + y * self.slant;
                let length = self.length * (0.6 + 0.8 * unit(self.seed, i, 3));
                Streak { from: [x, y], to: [x + length * self.slant, y + length], opacity: 0.25 + 0.5 * unit(self.seed, i, 3) }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaks_are_deterministic_and_fall() {
        let field = StreakField { seed: 7, count: 32, area: [1280., 720.], speed: 900., slant: 0.1, length: 40. };
        assert_eq!(field.at(500), field.at(500));
        let (a, b) = (field.at(0), field.at(10));
        assert!(a.iter().zip(&b).filter(|(p, q)| q.from[1] > p.from[1]).count() > 24);
        assert!(a.iter().all(|s| s.from[1] >= -40. && s.from[1] <= 720.));
    }
}
