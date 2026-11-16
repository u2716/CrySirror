// SPDX-License-Identifier: MPL-2.0
//! Integer tile-scatter ("screen break") simulation. A captured screen is cut
//! into a grid of square tiles; each tile waits for its delay, then moves with
//! 10-bit fixed-point velocity, a shared horizontal acceleration and a fixed
//! quadratic vertical term, and fades over its last `fade_steps` lives.
//! Arithmetic is C integer arithmetic (truncating division) so a native
//! profile can reproduce its original frames exactly. Rendering is separate:
//! callers turn [`TileScatter::tiles`] into ordinary image composite layers.
use anyhow::{ensure, Result};

/// One tile's native state. Positions/velocities are 10-bit fixed point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tile {
    /// Remaining life; zero or negative is finished.
    pub life: i32,
    /// Negative values count up before motion starts; afterwards the number
    /// of motion steps taken (also the per-tile overlay frame index).
    pub delay: i32,
    pub velocity: [i32; 2],
    /// Accumulated horizontal acceleration term.
    pub drift: i32,
    /// Time accumulator, advanced by `time_step` after each motion step.
    pub time: i32,
    pub origin: [i32; 2],
    pub position: [i32; 2],
    /// Fade weight in `0..=fade_steps`.
    pub alpha: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TileScatter {
    pub columns: u32,
    pub rows: u32,
    pub tile: i32,
    pub bounds: [i32; 2],
    pub acceleration: i32,
    pub life_step: i32,
    pub time_step: i32,
    /// Quadratic vertical coefficient in 1/1024 units (e.g. 4008).
    pub gravity: i32,
    pub fade_steps: i32,
    pub tiles: Vec<Tile>,
}
impl TileScatter {
    /// Grid in row-major order with origins at `tile` multiples (fixed point).
    pub fn new(columns: u32, rows: u32, tile: i32, bounds: [i32; 2]) -> Result<Self> {
        ensure!(
            (1..=256).contains(&columns) && (1..=256).contains(&rows) && (1..=1024).contains(&tile),
            "tile scatter grid"
        );
        let tiles = (0..rows as i32)
            .flat_map(|row| {
                (0..columns as i32).map(move |column| {
                    let origin = [column * tile << 10, row * tile << 10];
                    Tile { origin, position: origin, ..Default::default() }
                })
            })
            .collect();
        Ok(Self {
            columns,
            rows,
            tile,
            bounds,
            acceleration: 0,
            life_step: 1,
            time_step: 81,
            gravity: 4008,
            fade_steps: 16,
            tiles,
        })
    }
    /// One simulation step. Returns true when every tile has finished.
    pub fn step(&mut self) -> bool {
        let limit = [self.bounds[0] << 10, self.bounds[1] << 10];
        for t in &mut self.tiles {
            if t.life <= 0 {
                t.life = 0;
                continue;
            }
            if t.delay < 0 {
                t.delay += 1;
                continue;
            }
            let time = t.time;
            t.position[0] = t.origin[0] + 4 * (t.drift + time * t.velocity[0] / 1024);
            t.drift += self.acceleration;
            t.position[1] = t.origin[1] + time * t.velocity[1] / -1024
                - time * (self.gravity * time / 1024) / 1024;
            if t.position[0] >= limit[0] || t.position[1] >= limit[1] {
                t.life = 0;
            }
            t.life -= self.life_step;
            t.time += self.time_step;
            t.delay += 1;
            if t.life <= self.fade_steps {
                t.alpha = t.life;
                if t.life <= 0 {
                    t.alpha = 0;
                    t.life = 0;
                }
            }
        }
        self.tiles.iter().all(|t| t.life <= 0)
    }
    /// Pixel position of a live tile.
    pub fn pixel(&self, tile: &Tile) -> [i32; 2] {
        [tile.position[0] / 1024, tile.position[1] / 1024]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_motion_matches_truncating_reference() {
        let mut s = TileScatter::new(2, 1, 32, [800, 600]).unwrap();
        s.tiles[0] = Tile { life: 3, velocity: [5 << 10, 40 << 10], alpha: 16, ..s.tiles[0] };
        s.tiles[1].life = 0;
        assert!(!s.step());
        // time 0 at the first step: origin unchanged.
        assert_eq!(s.pixel(&s.tiles[0]), [0, 0]);
        assert!(!s.step());
        // time 81: x = 4*(81*5120/1024) = 1620 -> 1px; y = -(81*40960/1024) - 81*(4008*81/1024)/1024.
        let y = 81 * (40 << 10) / -1024 - 81 * (4008 * 81 / 1024) / 1024;
        assert_eq!(s.tiles[0].position, [4 * (81 * (5 << 10) / 1024), y]);
        assert!(s.step());
        assert_eq!(s.tiles[0].alpha, 0);
    }
}
