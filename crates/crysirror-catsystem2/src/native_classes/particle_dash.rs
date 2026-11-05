// SPDX-License-Identifier: MPL-2.0
//! The `dash` particle plugin restated from its static code; see `particle_plugins`.
//! Streaks start beside the screen centre and fly outward along a fixed heading. Each root
//! lays fading strips behind it, stretched to cover the distance since its last position.
use super::particle_plugins::Record;
use crate::motion::RandomStreams;
use anyhow::Result;
use crysirror_core::state::ClientId;

/// 0x100170B0: the image's four columns.
const COLUMNS: [f32; 4] = [0., 0.25, 0.5, 0.75];
/// 1A00 places streaks around this fixed point.
const CENTRE: [f32; 2] = [640., 360.];

#[derive(Clone, Default)]
struct Root {
    respawn: bool,
    clock: i32,
    start: [f32; 3],
    position: [f32; 3],
    last: [f32; 3],
    velocity: [f32; 3],
    reach: f32,
}

#[derive(Clone, Copy, Default)]
struct Strip {
    active: bool,
    clock: u32,
}

pub(crate) struct Dash {
    /// Records per root: the root, then its strips.
    group: usize,
    speed: f32,
    width: f32,
    heading: f32,
    spread: f32,
    radius: f32,
    alpha: u8,
    /// `dword_10017110`, static in the original: active roots grow by one per update.
    active: usize,
    roots: Vec<Root>,
    strips: Vec<Vec<Strip>>,
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

impl Dash {
    /// Vtable +4, 1390 and 1120. Words: records per root, speed, width %, heading, spread,
    /// radius, alpha.
    pub(crate) fn new(records: &mut [Record], words: [f32; 7], texture: [f32; 2], random: &mut RandomStreams, client: ClientId) -> Result<Self> {
        let group = (words[0] as i32).max(1) as usize;
        let count = records.len() / group;
        let mut dash = Self {
            group,
            speed: words[1],
            width: words[2] / 100.,
            heading: words[3],
            spread: words[4],
            radius: words[5],
            alpha: words[6] as i32 as u8,
            active: 0,
            roots: vec![Root::default(); count],
            strips: vec![vec![Strip::default(); group - 1]; count],
        };
        for index in 0..count {
            dash.spawn(index, records, texture, random, client)?;
        }
        Ok(dash)
    }

    /// 1A00.
    fn spawn(&mut self, index: usize, records: &mut [Record], texture: [f32; 2], random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let at = index * self.group;
        let u = COLUMNS[(random.integer_word(client)? & 3) as usize];
        let width = random.float_value(client)? * self.width * 0.8 + self.width * 0.2;
        let mut speed = self.speed;
        let mut offset = 0.;
        for step in 1..6 {
            offset += (random.float_value(client)? * 0.9 + 0.1) * (step as f32 * 0.1 * self.spread);
            speed *= random.float_value(client)? * 0.1 + 0.85;
            if random.integer_word(client)? % 5 == 0 {
                break;
            }
        }
        let half = self.spread * 0.5;
        let across = if index % 2 != 0 { offset - half } else { half - offset };
        let angle = self.heading.to_radians();
        // 1B90-style: the angle snaps to thousandths before the sine and cosine.
        let (sin, cos) = (((angle * 1000.) as i32) as f32 / 1000.).sin_cos();
        let place = |along: f32| [cos * across - sin * along + CENTRE[0], sin * across + cos * along + CENTRE[1], 0.];
        let start = place(-self.radius);
        let end = place(self.radius + texture[1]);
        let reach = distance(end, start);
        let mut velocity = [end[0] - start[0], end[1] - start[1], 0.];
        if reach > 0. {
            velocity = [velocity[0] / reach * speed, velocity[1] / reach * speed, 0.];
        }
        let record = &mut records[at];
        record.pivot = [texture[0] * 0.5, texture[1] * 0.5, 0.];
        record.uvs = Some([[u, 0.], [u + 0.25, 0.], [u, 1.], [u + 0.25, 1.]]);
        record.scale = [width, 1., 1.];
        record.rotation = angle;
        record.alpha = 0;
        let position = record.position;
        self.roots[index] = Root { respawn: false, clock: 0, start, position: start, last: position, velocity, reach };
        for (strip, state) in self.strips[index].iter_mut().enumerate() {
            *state = Strip::default();
            let strip = &mut records[at + 1 + strip];
            strip.position = position;
            strip.uvs = records_uvs(u);
            strip.alpha = 0;
        }
        Ok(())
    }

    /// Vtable +8, 1460.
    pub(crate) fn update(&mut self, records: &mut [Record], texture: [f32; 2], dt: i32, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        self.active = (self.active + 1).min(self.roots.len());
        let elapsed = dt.max(0) as u32;
        for index in 0..self.active {
            let at = index * self.group;
            if self.roots[index].respawn {
                self.spawn(index, records, texture, random, client)?;
            } else {
                let root = &mut self.roots[index];
                root.position[0] += root.velocity[0];
                root.position[1] += root.velocity[1];
                records[at].position = root.position;
                records[at].alpha = self.alpha;
                if distance(root.position, root.start) > root.reach {
                    root.respawn = true;
                }
            }
            // One free strip takes the step just flown, while the root is still in flight.
            if !self.roots[index].respawn {
                if let Some(free) = self.strips[index].iter().position(|strip| !strip.active) {
                    let current = records[at].position;
                    let last = self.roots[index].last;
                    let step = distance(current, last);
                    let (pivot, scale) = (records[at].pivot, records[at].scale);
                    let strip = &mut records[at + 1 + free];
                    strip.pivot = pivot;
                    strip.scale = scale;
                    if step == 0. {
                        strip.position = current;
                    } else {
                        let (dx, dy, dz) = (current[0] - last[0], current[1] - last[1], current[2] - last[2]);
                        let mut angle = (dy / step).clamp(-1., 1.).acos();
                        if dx > 0. {
                            angle = -angle;
                        }
                        let _ = dz;
                        strip.rotation = angle;
                        strip.position = [(current[0] + last[0]) * 0.5, (current[1] + last[1]) * 0.5, (current[2] + last[2]) * 0.5];
                        strip.scale[1] *= step * 0.7;
                    }
                    self.strips[index][free].active = true;
                }
            }
            let life = self.strips[index].len() as u32;
            for (offset, strip) in self.strips[index].iter_mut().enumerate() {
                if !strip.active {
                    continue;
                }
                let record = &mut records[at + 1 + offset];
                record.alpha = (255. * (1. - strip.clock as f32 / life as f32)).clamp(0., 255.) as u8;
                strip.clock += elapsed;
                if strip.clock >= life {
                    *strip = Strip::default();
                    record.alpha = 0;
                }
            }
            let root = &mut self.roots[index];
            root.last = records[at].position;
            root.clock = root.clock.wrapping_add(dt);
        }
        Ok(())
    }
}

fn records_uvs(u: f32) -> Option<[[f32; 2]; 4]> {
    Some([[u, 0.], [u + 0.25, 0.], [u, 1.], [u + 0.25, 1.]])
}
