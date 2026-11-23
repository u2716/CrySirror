// SPDX-License-Identifier: MPL-2.0
//! The `rain` particle plugin restated from its static code; see `particle_plugins`.
use super::particle_plugins::Record;
use crate::motion::RandomStreams;
use anyhow::Result;
use crysirror_core::state::ClientId;

/// 0x10011D10: the four cells of the image, one chosen per drop.
const CELLS: [[f32; 2]; 4] = [[0., 0.], [0.5, 0.], [0., 0.5], [0.5, 0.5]];

#[derive(Clone, Default)]
struct Drop {
    /// +0: milliseconds since it spawned.
    clock: i32,
    falling: bool,
    velocity: [f32; 2],
    /// +0x24: milliseconds its path across 1020 pixels takes.
    life: f32,
    /// +0x28: its opacity.
    opacity: f32,
}

pub(crate) struct Rain {
    /// +0x3C, +0x40: shortest life and its spread, in milliseconds.
    life: (f32, f32),
    /// +0x44, +0x48: wind angle and its spread, in radians.
    wind: (f32, f32),
    /// +0x4C, +0x50: one call in this many releases `release` more drops.
    period: u32,
    release: usize,
    /// +0x5C.
    active: usize,
    drops: Vec<Drop>,
}

impl Rain {
    /// Vtable +4, 1070.
    pub(crate) fn new(records: &mut [Record], words: [f32; 7], canvas: [u32; 2], texture: [f32; 2], random: &mut RandomStreams, client: ClientId) -> Result<Self> {
        let [shortest, spread, wind, wind_spread, rate, release, warm] = words;
        let mut rain = Self {
            life: ((shortest as i32).max(1) as f32, (spread as i32).max(0) as f32),
            wind: ((wind / 100.).clamp(-1., 1.), (wind_spread / 100.).clamp(-0.2, 0.2)),
            period: (0x3d - rate as i32).clamp(1, 0x3c) as u32,
            release: release as i32 as usize,
            active: 0,
            drops: vec![Drop::default(); records.len()],
        };
        // A warm start has half the drops already part way down.
        let warmed = if warm as i32 != 0 { records.len() / 2 } else { 0 };
        rain.active = warmed;
        for index in 0..records.len() {
            rain.spawn(index, records, canvas, texture, random, client)?;
            if index < warmed {
                let (drop, record) = (&mut rain.drops[index], &mut records[index]);
                let steps = (random.float_value(client)? * drop.life) as i32;
                drop.clock = steps;
                record.position[0] += drop.velocity[0] * steps as f32;
                record.position[1] += drop.velocity[1] * steps as f32;
            }
        }
        Ok(rain)
    }

    /// 1440.
    fn spawn(&mut self, index: usize, records: &mut [Record], canvas: [u32; 2], texture: [f32; 2], random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let [u, v] = CELLS[(random.integer_word(client)? & 3) as usize];
        let wide = (canvas[0] as f32 * 2.8) as i32 as f32;
        let x = random.float_value(client)? * wide - (wide - canvas[0] as f32) * 0.5;
        let z = random.float_value(client)? * 10.;
        let angle = random.float_value(client)? * self.wind.1 - self.wind.1 * 0.5 + self.wind.0;
        let life = random.float_value(client)? * self.life.1 + self.life.0;
        let opacity = (random.integer_word(client)? % 0x46 + 0x64) as f32;
        let record = &mut records[index];
        record.uvs = Some([[u, v], [u + 0.5, v], [u, v + 0.5], [u + 0.5, v + 0.5]]);
        record.pivot = [texture[0] * 0.5, texture[1] * 0.5, 0.];
        record.position = [x, -100., z];
        record.rotation = angle;
        record.alpha = 0;
        // 1B90 snaps the angle to its table's step before taking the matrix's Y row.
        let (sin, cos) = angle.sin_cos();
        let speed = 1020. / life;
        self.drops[index] = Drop { clock: 0, falling: true, velocity: [-sin * speed, cos * speed], life, opacity };
        Ok(())
    }

    /// Vtable +8, 1270.
    pub(crate) fn update(&mut self, records: &mut [Record], canvas: [u32; 2], texture: [f32; 2], dt: i32, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        if random.integer_word(client)? % self.period == 0 {
            self.active += self.release;
        }
        self.active = self.active.min(records.len());
        for index in 0..self.active {
            let drop = &self.drops[index];
            if drop.falling {
                let velocity = drop.velocity;
                // Host +0x74 over [0, 2] ms: opaque almost at once.
                let opacity = drop.opacity * (drop.clock as f32 / 2.).clamp(0., 1.);
                let record = &mut records[index];
                record.position[0] += velocity[0];
                record.position[1] += velocity[1];
                record.alpha = opacity as i32 as u8;
                if canvas[1] as f32 + 200. < record.position[1] {
                    self.drops[index].falling = false;
                }
            } else {
                self.spawn(index, records, canvas, texture, random, client)?;
            }
            self.drops[index].clock = self.drops[index].clock.wrapping_add(dt);
        }
        Ok(())
    }
}
