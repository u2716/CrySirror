// SPDX-License-Identifier: MPL-2.0
use anyhow::Result;
use crysirror_image::{integer_lines::IntegerLine, temporal_pattern::PatternRng};

struct Drop {
    position: [i32; 2],
    velocity: [i32; 2],
    coverage: u8,
}
impl Drop {
    fn spawn(
        rng: &mut PatternRng,
        canvas: [i32; 2],
        horizontal: bool,
        edge: bool,
        alpha: i32,
    ) -> Result<Self> {
        let mut draw = || rng.next_draw().map(|v| v as i32);
        let position = if horizontal {
            let y = draw()? % canvas[1];
            [
                if edge {
                    0
                } else {
                    draw()? % (canvas[0] + 100) - 100
                },
                y,
            ]
        } else {
            [
                draw()? % (canvas[0] + 100) - 100,
                if edge { 0 } else { draw()? % canvas[1] },
            ]
        };
        let major = 30 + draw()? % 200;
        let minor = draw()? % (major / 10);
        let velocity = if horizontal {
            [major, minor]
        } else {
            [minor, major]
        };
        Ok(Self {
            position,
            velocity,
            coverage: (alpha * (32 + draw()? % 128) / 256) as u8,
        })
    }
}

pub(super) struct Rain {
    drops: Vec<Drop>,
    pub horizontal: bool,
}

impl Rain {
    pub fn new(rng: &mut PatternRng, canvas: [i32; 2], horizontal: bool) -> Result<Self> {
        let mut drops = (0..200)
            .map(|_| Drop::spawn(rng, canvas, false, false, 0))
            .collect::<Result<Vec<_>>>()?;
        if horizontal {
            for drop in &mut drops {
                *drop = Drop::spawn(rng, canvas, true, false, 0)?;
            }
        }
        Ok(Self { drops, horizontal })
    }
    pub fn advance(
        &mut self,
        rng: &mut PatternRng,
        canvas: [i32; 2],
        delta: u32,
        alpha: i32,
    ) -> Result<Vec<IntegerLine>> {
        let mut lines = Vec::with_capacity(self.drops.len());
        for drop in &mut self.drops {
            let from = drop.position;
            let to = std::array::from_fn(|axis| from[axis] + drop.velocity[axis]);
            if (0..2).all(|i| from[i] >= 0 && to[i] <= canvas[i]) {
                lines.push(IntegerLine {
                    from,
                    to,
                    color: [235, 235, 255, drop.coverage],
                });
            }
            let reset = self.horizontal && delta > 50;
            let steps = if self.horizontal && delta >= 25 { 4 } else { 2 };
            let next = std::array::from_fn(|axis| from[axis] + steps * drop.velocity[axis]);
            if reset || next[0] >= canvas[0] || next[1] >= canvas[1] {
                *drop = Drop::spawn(rng, canvas, self.horizontal, !reset, alpha)?;
            } else {
                drop.position = next;
            }
        }
        Ok(lines)
    }
}
