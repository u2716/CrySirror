// SPDX-License-Identifier: MPL-2.0
use anyhow::{Result, ensure};
use crysirror_image::{ImageSurface, bitmap::BitmapRect, temporal_pattern::PatternRng};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) enum Grid {
    #[serde(rename = "fixed-800x600")]
    Fixed,
    #[serde(rename = "canvas")]
    Canvas,
    #[serde(rename = "canvas-fast-finish")]
    CanvasFastFinish,
}

impl Grid {
    pub fn finishes_immediately(self, speed: i32) -> bool {
        matches!(self, Self::CanvasFastFinish) && speed > 255
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retail_tile_transition_draws_each_native_pattern_and_clips_the_last_cell() {
        let delays = [
            [0, 0, 0, 0, 0, 0],
            [0, 10, 20, 30, 40, 50],
            [50, 40, 30, 20, 10, 0],
            [0, 30, 60, 10, 40, 70],
            [70, 40, 10, 60, 30, 0],
            [0, 50, 100, 50, 100, 150],
            [150, 100, 50, 100, 50, 0],
            [0, 30, 0, 30, 42, 30],
            [42, 30, 42, 30, 0, 30],
            [41, 47, 34, 40, 29, 4],
            [40, 20, 0, 70, 50, 30],
        ];
        for (index, delays) in delays.into_iter().enumerate() {
            let mode = index as i32 + 3;
            let mut tiles = TileTransition::new(
                Grid::Canvas,
                [83, 51],
                mode,
                10,
                0,
                &mut PatternRng::msvc(1),
            )
            .unwrap();
            let last = delays.iter().max().unwrap() + 60;
            for phase in 0..=last {
                assert!(!matches!(
                    tiles.update(phase as u64 * 10, 1).unwrap(),
                    Update::Finished
                ));
                for y in 0..51 {
                    for x in 0..83 {
                        let delay = delays[(y / 40 * 3 + x / 40) as usize];
                        let side = 4 * ((phase - delay) / 6).clamp(0, 10);
                        let exposed = x % 40 < side && y % 40 < side;
                        assert_eq!(
                            tiles.mask.rgba[((83 * y + x) * 4) as usize] == 0,
                            exposed,
                            "mode {mode}, phase {phase}, pixel ({x},{y})"
                        );
                    }
                }
            }
            assert!(matches!(
                tiles.update((last + 1) as u64 * 10, 1).unwrap(),
                Update::Finished
            ));
        }
    }

    #[test]
    fn retail_tile_transition_honors_draw_gate_and_previous_signed_quotient() {
        let mut tiles =
            TileTransition::new(Grid::Canvas, [120, 80], 4, 20, 0, &mut PatternRng::msvc(1))
                .unwrap();
        tiles.update(0, 5).unwrap();
        assert!(matches!(tiles.update(40, 5).unwrap(), Update::Unchanged));
        assert!(matches!(tiles.update(550, 5).unwrap(), Update::Changed));
        assert_eq!(tiles.mask.rgba[0], 0);
        assert_eq!(
            tiles.mask.rgba[40 * 4],
            255,
            "a skipped delayed tile is copied only at completion"
        );
        assert!(matches!(tiles.update(560, 5).unwrap(), Update::Unchanged));
        assert!(matches!(tiles.update(600, 5).unwrap(), Update::Finished));
    }
}

pub(crate) enum Update {
    Unchanged,
    Changed,
    Finished,
}

pub(crate) struct TileTransition {
    pub mask: ImageSurface,
    columns: i32,
    counters: Vec<i32>,
    started: u64,
    last_draw: Option<u32>,
    phase: u32,
    speed: u32,
}

impl TileTransition {
    pub fn new(
        grid: Grid,
        canvas: [i32; 2],
        mode: i32,
        speed: i32,
        now: u64,
        rng: &mut PatternRng,
    ) -> Result<Self> {
        ensure!(
            (3..=13).contains(&mode),
            "minori tile transition type {mode}"
        );
        ensure!(
            !matches!(grid, Grid::Fixed) || canvas == [800, 600],
            "minori fixed tile canvas"
        );
        let [width, height] = canvas.map(u32::try_from);
        let (width, height) = (width?, height?);
        let mask = ImageSurface::filled(width, height, [255; 4])?;
        let columns = width.div_ceil(40) as i32;
        let rows = height.div_ceil(40) as i32;
        let count = columns * rows;
        let mut shuffled = Vec::new();
        if mode == 13 {
            shuffled.extend(0..columns);
            for i in 0..columns as usize {
                let other = rng.next_draw()? as usize % columns as usize;
                shuffled.swap(i, other);
            }
        }
        let mut counters = Vec::with_capacity(count as usize);
        for y in 0..rows {
            for x in 0..columns {
                let delay = match mode {
                    3 => 0,
                    4 => 10 * (y * columns + x),
                    5 => 10 * (count - 1 - y * columns - x),
                    6 => 10 * (x * columns + y),
                    7 => 10 * ((columns - 1 - x) * columns + rows - 1 - y),
                    8 => 10 * (columns + rows) * (x + y),
                    9 => 10 * (columns + rows) * (columns + rows - 2 - x - y),
                    10 | 11 => {
                        let mut dx = (x - columns / 2).abs();
                        let mut dy = (y - rows / 2).abs();
                        if mode == 10 {
                            dx -= columns / 2;
                            dy -= rows / 2;
                        }
                        (f64::from(dx * dx + dy * dy).sqrt() * f64::from(10 * columns)) as i32
                    }
                    12 => (rng.next_draw()? % (10 * count) as u32) as i32,
                    13 => 10 * (shuffled[x as usize] * rows + y * columns),
                    _ => unreachable!(),
                };
                counters.push(-delay);
            }
        }
        Ok(Self {
            mask,
            columns,
            counters,
            started: now,
            last_draw: None,
            phase: 0,
            speed: speed as u32,
        })
    }

    pub fn update(&mut self, now: u64, interval: u32) -> Result<Update> {
        self.draw(now, interval, None)
    }

    pub fn draw(&mut self, now: u64, interval: u32,
        mut frames: Option<(&ImageSurface, &mut ImageSurface)>) -> Result<Update> {
        let ticks = (now.saturating_sub(self.started) / 10) as u32;
        if self
            .last_draw
            .is_some_and(|last| ticks.wrapping_sub(last) < interval)
        {
            return Ok(Update::Unchanged);
        }
        self.last_draw = Some(ticks);
        let phase = ticks.wrapping_mul(self.speed) / 10;
        let delta = phase.wrapping_sub(self.phase) as i32;
        if delta == 0 {
            return Ok(Update::Unchanged);
        }
        self.phase = phase;
        let count = self.counters.len() as i32;
        let mut active = false;
        let mut changed = false;
        for (i, counter) in self.counters.iter_mut().enumerate() {
            if *counter >= 10 * count {
                continue;
            }
            active = true;
            let previous = *counter / count;
            *counter = counter.wrapping_add(delta);
            let next = (*counter / count).min(10);
            // The original tests the previous signed quotient before copying.
            // A large time step can cross an entire delayed tile without a blit.
            if previous >= 0 && next > previous {
                let side = (4 * next) as u32;
                let x = (i as i32 % self.columns) * 40;
                let y = (i as i32 / self.columns) * 40;
                let rect = BitmapRect::new(x, y, side, side);
                self.mask.fill_rect(rect, [0, 0, 0, 255])?;
                if let Some((source, history)) = frames.as_mut() {
                    history.blit(source, crysirror_image::bitmap::BlitOptions {
                        source_rect: rect, destination: [x, y], opacity: 255,
                        mode: crysirror_image::bitmap::PixelMath::Copy, color_key: None,
                    })?;
                }
                changed = true;
            }
        }
        Ok(if !active {
            Update::Finished
        } else if changed {
            Update::Changed
        } else {
            Update::Unchanged
        })
    }
}
