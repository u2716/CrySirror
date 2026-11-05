//! Seeded atlas-mask fields with explicit integer delays and float32 motion.
//! Sampling and stepping are independent of rendering, clocks, VMs and devices.
use crate::falling_atlas::SequenceRng;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TileMaskPattern {
    Base,
    Scatter,
    Diagonal,
    Down,
    ReverseDiagonal,
    WideScatter,
    Staircase,
    Diamond,
    Up,
    RandomDelay,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TileMaskSpec {
    pub grid: [u16; 2],
    pub spacing: [u16; 2],
    pub canvas: [u32; 2],
    pub pattern: TileMaskPattern,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Tile {
    life: i32,
    age: i32,
    frame: i32,
    origin: [f32; 2],
    position: [f32; 2],
    velocity: [f32; 2],
    drift: f32,
    time: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TileMaskField {
    spec: TileMaskSpec,
    tiles: Vec<Tile>,
    acceleration: f32,
    time_step: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileMaskSample {
    pub position: [i32; 2],
    pub frame: u16,
}
impl TileMaskField {
    pub fn new(spec: TileMaskSpec, rng: &mut SequenceRng) -> Result<Self> {
        let count = usize::from(spec.grid[0]) * usize::from(spec.grid[1]);
        ensure!(
            count > 0
                && count <= 4096
                && spec.spacing.iter().all(|v| *v > 0)
                && spec.canvas.iter().all(|v| *v > 0 && *v <= 16384),
            "tile mask field domain"
        );
        let mut tiles = Vec::with_capacity(count);
        for row in 0..spec.grid[1] {
            for col in 0..spec.grid[0] {
                let mut vx = 32 - (rng.next_u32()? & 31) as i32;
                let vy = (rng.next_u32()? & 31) as i32 + 32;
                if rng.next_u32()? & 1 == 0 {
                    vx = (rng.next_u32()? & 31) as i32 - 32;
                }
                let life = (rng.next_u32()? % 32) as i32 + 16;
                let origin = [
                    f32::from(col) * f32::from(spec.spacing[0]),
                    f32::from(row) * f32::from(spec.spacing[1]),
                ];
                tiles.push(Tile {
                    life,
                    age: 0,
                    frame: 16,
                    origin,
                    position: origin,
                    velocity: [vx as f32, vy as f32],
                    drift: 0.,
                    time: 0.,
                });
            }
        }
        let mut acceleration = 1.;
        let mut time_step = 0.08;
        for (index, tile) in tiles.iter_mut().enumerate() {
            let row = (index / usize::from(spec.grid[0])) as i32;
            let col = (index % usize::from(spec.grid[0])) as i32;
            use TileMaskPattern::*;
            match spec.pattern {
                Base => {}
                Scatter => {
                    acceleration = 0.;
                    tile.velocity = [
                        (48 - (rng.next_u32()? & 31) as i32) as f32,
                        (48 - (rng.next_u32()? & 31) as i32) as f32,
                    ];
                }
                Diagonal => {
                    tile.age =
                        (rng.next_u32()? % 2) as i32 - row / 5 + col - i32::from(spec.grid[0]);
                    tile.velocity = [
                        (32 - (rng.next_u32()? & 31) as i32) as f32,
                        (32 - (rng.next_u32()? & 0x30) as i32) as f32,
                    ];
                }
                Down => {
                    acceleration = 0.;
                    tile.age = (rng.next_u32()? % 2) as i32 + row - i32::from(spec.grid[1]);
                    tile.velocity = [
                        (4 - (rng.next_u32()? & 8) as i32) as f32,
                        ((rng.next_u32()? & 31) + 32) as f32,
                    ];
                }
                ReverseDiagonal => {
                    acceleration = -1.;
                    tile.age = (rng.next_u32()? % 2) as i32 - row / 5 - col;
                    tile.velocity = [
                        ((rng.next_u32()? & 31) as i32 - 32) as f32,
                        (32 - (rng.next_u32()? & 0x30) as i32) as f32,
                    ];
                }
                WideScatter => {
                    acceleration = 0.;
                    tile.velocity = [
                        (32 - (rng.next_u32()? & 63) as i32) as f32,
                        (32 - (rng.next_u32()? & 0x30) as i32) as f32,
                    ];
                }
                Staircase => {
                    acceleration = 0.;
                    tile.age = -2 * row - col;
                    tile.velocity = [
                        (2 - (rng.next_u32()? & 8) as i32) as f32,
                        ((rng.next_u32()? & 8) as i32 - 2) as f32,
                    ];
                }
                Diamond => {
                    acceleration = 0.;
                    tile.age = -((i32::from(spec.grid[1]) / 2 - row).abs()
                        + (i32::from(spec.grid[0]) / 2 - col).abs());
                    tile.velocity = [
                        (4 - (rng.next_u32()? & 8) as i32) as f32,
                        (4 - (rng.next_u32()? & 8) as i32) as f32,
                    ];
                }
                Up => {
                    acceleration = 0.;
                    time_step = 0.32;
                    tile.velocity = [
                        ((rng.next_u32()? % 16) as i32 - 8) as f32,
                        (-32 - (rng.next_u32()? % 32) as i32) as f32,
                    ];
                }
                RandomDelay => tile.age = -((rng.next_u32()? % 20) as i32),
            }
        }
        Ok(Self {
            spec,
            tiles,
            acceleration,
            time_step,
        })
    }
    pub fn active(&self) -> bool {
        self.tiles.iter().any(|tile| tile.life > 0)
    }
    pub fn advance(&mut self, steps: u8) -> Result<()> {
        ensure!(steps > 0, "tile mask zero rate");
        for _ in 0..steps {
            for tile in &mut self.tiles {
                if tile.life <= 0 {
                    tile.life = 0;
                    continue;
                }
                if tile.age < 0 {
                    tile.age += 1;
                    continue;
                }
                let intermediate = (f64::from(tile.velocity[0]) * f64::from(tile.time)
                    + f64::from(tile.drift)) as f32;
                tile.position[0] =
                    (f64::from(intermediate) * 4. + f64::from(tile.origin[0])) as f32;
                tile.drift += self.acceleration;
                tile.position[1] = (f64::from(tile.origin[1])
                    - (f64::from(tile.time) * f64::from(tile.velocity[1])
                        + f64::from(tile.time) * 3.920000076293945 * f64::from(tile.time)))
                    as f32;
                if tile.position[0] >= self.spec.canvas[0] as f32
                    || tile.position[1] >= self.spec.canvas[1] as f32
                {
                    tile.life = 0;
                }
                tile.life -= 1;
                tile.age += 1;
                tile.time = (f64::from(tile.time) + self.time_step) as f32;
                if tile.age >= 24 {
                    tile.life = 0;
                }
                if tile.life <= 16 {
                    tile.frame = tile.life.max(0);
                    if tile.life <= 0 {
                        tile.life = 0;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn samples(&self) -> Vec<TileMaskSample> {
        self.tiles
            .iter()
            .filter(|tile| tile.life > 0)
            .map(|tile| TileMaskSample {
                position: tile.position.map(|v| v as i32),
                frame: tile.frame as u16,
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn spec(pattern: TileMaskPattern) -> TileMaskSpec {
        TileMaskSpec {
            grid: [2, 2],
            spacing: [32, 32],
            canvas: [1280, 720],
            pattern,
        }
    }
    #[test]
    fn crt_seed_one_base_record_and_second_native_tick_match_scalar_oracle() {
        let mut field =
            TileMaskField::new(spec(TileMaskPattern::Base), &mut SequenceRng::msvc(1)).unwrap();
        assert_eq!(field.tiles[0].life, 17);
        assert_eq!(field.tiles[0].velocity, [-28., 35.]);
        field.advance(1).unwrap();
        assert_eq!(field.tiles[0].frame, 16);
        assert_eq!(field.tiles[0].position, [0., 0.]);
        field.advance(1).unwrap();
        assert_eq!(
            field.samples()[0],
            TileMaskSample {
                position: [-4, -2],
                frame: 15
            }
        );
        assert_eq!(field.tiles[0].drift, 2.);
    }
    #[test]
    fn first_tick_retains_grid_and_later_ticks_consume_no_random() {
        let mut rng = SequenceRng::msvc(1);
        let mut field = TileMaskField::new(spec(TileMaskPattern::Scatter), &mut rng).unwrap();
        let state = rng.clone();
        field.advance(1).unwrap();
        assert_eq!(field.samples()[0].position, [0, 0]);
        field.advance(1).unwrap();
        assert_ne!(field.samples()[0].position, [0, 0]);
        assert_eq!(rng, state);
        for _ in 0..40 {
            field.advance(1).unwrap();
        }
        assert!(!field.active());
    }
    #[test]
    fn delayed_masks_remain_visible_and_state_continues_after_roundtrip() {
        let mut field =
            TileMaskField::new(spec(TileMaskPattern::Staircase), &mut SequenceRng::msvc(7))
                .unwrap();
        field.advance(1).unwrap();
        assert_eq!(field.samples()[3].position, [32, 32]);
        let mut restored: TileMaskField =
            serde_json::from_slice(&serde_json::to_vec(&field).unwrap()).unwrap();
        field.advance(2).unwrap();
        restored.advance(2).unwrap();
        assert_eq!(field.samples(), restored.samples());
    }
}
