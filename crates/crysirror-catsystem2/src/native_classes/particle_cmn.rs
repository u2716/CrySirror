// SPDX-License-Identifier: MPL-2.0
//! The `cmnptcl` particle plugin: an emitter configured by a `ptcl<set><id>.plist`, in the
//! keys of the common particle-editor format. Restated from 2AB0, 3850 and 4310.
use super::particle_plugins::{Custom, Record, Tick};
use anyhow::Result;
use std::collections::BTreeMap;

/// 3E7E: the extra factor on a particle's per-tick displacement.
const STEP_SCALE: f32 = 1.0;

/// `<key>name</key><real|integer|string>value</...>` pairs of a flat dictionary.
pub(crate) fn parse_plist(bytes: &[u8]) -> BTreeMap<String, f32> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = BTreeMap::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find("<key>") {
        rest = &rest[start + 5..];
        let Some(end) = rest.find("</key>") else { break };
        let key = rest[..end].trim().to_owned();
        rest = &rest[end + 6..];
        let Some(open) = rest.find('<') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];
        if !tag.ends_with('/') {
            let body = &after[close + 1..];
            if let Some(stop) = body.find("</") {
                if let Ok(value) = body[..stop].trim().parse::<f32>() {
                    out.insert(key, value);
                }
            }
        }
    }
    out
}

/// The dictionary after 2AB0's unit conversions: per-second values to per-tick, colours to
/// 0..255, sizes over the 30-pixel unit, and y flipped into screen space.
#[derive(Clone, Debug, Default, PartialEq)]
struct Config {
    angle: (f32, f32),
    duration: f32,
    radial_mode: bool,
    finish: [(f32, f32); 4],
    start: [(f32, f32); 4],
    finish_size: (f32, f32),
    start_size: (f32, f32),
    gravity: [f32; 2],
    radius_from: (f32, f32),
    radius_to: f32,
    life: (f32, f32),
    radial: (f32, f32),
    spin: (f32, f32),
    rotation_start: (f32, f32),
    rotation_end: (f32, f32),
    source: [f32; 2],
    source_spread: [f32; 2],
    speed: (f32, f32),
    tangential: (f32, f32),
}

impl Config {
    fn from(keys: &BTreeMap<String, f32>, canvas_height: f32) -> Self {
        let k = |name: &str| keys.get(name).copied().unwrap_or(0.);
        let colour = |prefix: &str, suffix: &str| k(&format!("{prefix}Color{suffix}")) * 255.;
        let group = |prefix: &str| -> [(f32, f32); 4] {
            core::array::from_fn(|at| {
                let name = ["Alpha", "Blue", "Green", "Red"][at];
                (colour(prefix, name), colour(prefix, &format!("Variance{name}")))
            })
        };
        let duration = k("duration");
        Self {
            angle: (-k("angle"), k("angleVariance")),
            duration: if duration > 0. { duration * 60. } else { duration },
            radial_mode: k("emitterType") as i32 == 1,
            finish: group("finish"),
            start: group("start"),
            finish_size: (k("finishParticleSize") / 30., k("finishParticleSizeVariance") / 30.),
            start_size: (k("startParticleSize") / 30., k("startParticleSizeVariance") / 30.),
            gravity: [k("gravityx") / 3600., -k("gravityy") / 3600.],
            radius_from: (k("maxRadius"), k("maxRadiusVariance")),
            radius_to: k("minRadius"),
            life: (k("particleLifespan") * 60., k("particleLifespanVariance") * 60.),
            radial: (k("radialAcceleration") / 360000., k("radialAccelVariance") / 360000.),
            spin: (-k("rotatePerSecond") / 60., k("rotatePerSecondVariance") / 60.),
            rotation_start: (k("rotationStart"), k("rotationStartVariance")),
            rotation_end: (k("rotationEnd"), k("rotationEndVariance")),
            source: [k("sourcePositionx"), canvas_height - k("sourcePositiony")],
            source_spread: [k("sourcePositionVariancex"), k("sourcePositionVariancey")],
            speed: (k("speed") / 60., k("speedVariance") / 60.),
            tangential: (k("tangentialAcceleration") / 3600., k("tangentialAccelVariance") / 3600.),
        }
    }
}

#[derive(Clone, Default)]
struct Particle {
    life: f32,
    position: [f32; 2],
    direction: [f32; 2],
    colour: [f32; 4],
    colour_step: [f32; 4],
    size: f32,
    size_step: f32,
    rotation: f32,
    rotation_step: f32,
    radial: f32,
    tangential: f32,
    angle: f32,
    angle_step: f32,
    radius: f32,
    radius_step: f32,
    done: bool,
}

pub(crate) struct CmnParticles {
    config: Option<Config>,
    burst: bool,
    rate: f32,
    emit_clock: f32,
    elapsed: f32,
    active: usize,
    stopped: bool,
    finished: usize,
    half_canvas: [f32; 2],
    particles: Vec<Particle>,
}

impl CmnParticles {
    /// No plist answers no particles: 2AB0 leaves +336 clear and 3850 returns at once.
    pub(crate) fn new(plist: Option<&[u8]>, records: &mut [Record], words: &[f32], tick: &mut Tick) -> Result<Self> {
        let config = plist.map(|bytes| Config::from(&parse_plist(bytes), tick.canvas[1] as f32));
        let life = config.as_ref().map_or(0., |config| config.life.0);
        let mut this = Self {
            config,
            burst: words.get(2).is_some_and(|word| *word as i32 != 0),
            rate: if life > 0. { records.len() as f32 / life } else { 0. },
            emit_clock: 0.,
            elapsed: 0.,
            active: 0,
            stopped: false,
            finished: 0,
            half_canvas: [tick.canvas[0] as f32 * 0.5, tick.canvas[1] as f32 * 0.5],
            particles: vec![Particle::default(); records.len()],
        };
        if this.config.is_some() {
            for index in 0..records.len() {
                this.spawn(index, records, tick)?;
            }
        }
        Ok(this)
    }

    /// 4310.
    fn spawn(&mut self, index: usize, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        let config = self.config.clone().expect("spawn without a config");
        let mut draw = |base: (f32, f32)| -> Result<f32> { Ok((tick.random.float_value(tick.client)? * 2. - 1.) * base.1 + base.0) };
        let mut particle = Particle { life: draw(config.life)?.max(0.), ..Particle::default() };
        let life = particle.life.max(f32::MIN_POSITIVE);
        particle.position = [draw((config.source[0], config.source_spread[0]))?, draw((config.source[1], config.source_spread[1]))?];
        for channel in 0..4 {
            particle.colour[channel] = draw(config.start[channel])?.clamp(0., 255.);
        }
        for channel in 0..4 {
            let finish = draw(config.finish[channel])?.clamp(0., 255.);
            particle.colour_step[channel] = (finish - particle.colour[channel]) / life;
        }
        particle.size = draw(config.start_size)?.max(0.);
        particle.size_step = (draw(config.finish_size)?.max(0.) - particle.size) / life;
        particle.rotation = draw(config.rotation_start)?.to_radians();
        particle.rotation_step = (draw(config.rotation_end)?.to_radians() - particle.rotation) / life;
        let angle = draw(config.angle)?.to_radians();
        if config.radial_mode {
            particle.radius = draw(config.radius_from)?;
            particle.radius_step = (draw((config.radius_to, 0.))? - particle.radius) / life;
            particle.angle = angle;
            particle.angle_step = draw(config.spin)?.to_radians();
        } else {
            let speed = draw(config.speed)?;
            particle.direction = [angle.cos() * speed, angle.sin() * speed];
            particle.radial = draw(config.radial)?;
            particle.tangential = draw(config.tangential)?;
        }
        let record = &mut records[index];
        record.pivot = [tick.texture[0] * 0.5, tick.texture[1] * 0.5, 0.];
        record.visible = false;
        self.particles[index] = particle;
        Ok(())
    }
}

impl Custom for CmnParticles {
    fn update(&mut self, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        let Some(config) = self.config.clone() else { return Ok(()) };
        let dt = tick.dt as f32;
        let total = records.len();
        // 3850: emission by rate, or everything at once with the burst word.
        if !self.stopped {
            if self.active < total && self.rate > 0. {
                let period = 1. / self.rate;
                self.emit_clock += dt;
                while self.emit_clock > period && self.active < total {
                    self.active = if self.burst { total } else { self.active + 1 };
                    self.emit_clock -= period;
                }
            }
            self.elapsed += dt;
            if config.duration != -1. && self.elapsed > config.duration {
                self.stopped = true;
            }
        }
        let offset = [tick.base[0] - self.half_canvas[0], tick.base[1] - self.half_canvas[1]];
        let mut index = 0;
        while index < self.active {
            self.particles[index].life -= dt;
            if self.particles[index].life <= 0. {
                if self.stopped {
                    if !self.particles[index].done {
                        self.particles[index].done = true;
                        records[index].visible = false;
                        self.finished += 1;
                    }
                    if self.finished == self.active {
                        self.config = None;
                        return Ok(());
                    }
                    index += 1;
                } else {
                    // The last live particle takes this slot and a fresh one goes last.
                    self.spawn(index, records, tick)?;
                    let last = self.active - 1;
                    if index != last {
                        records.swap(index, last);
                        self.particles.swap(index, last);
                    }
                    self.active -= 1;
                }
                continue;
            }
            let particle = &mut self.particles[index];
            let record = &mut records[index];
            record.visible = true;
            if config.radial_mode {
                particle.angle += particle.angle_step * dt;
                particle.radius += particle.radius_step * dt;
                particle.position = [
                    config.source[0] - particle.angle.cos() * particle.radius,
                    config.source[1] - particle.angle.sin() * particle.radius,
                ];
            } else {
                let away = [particle.position[0] - config.source[0], particle.position[1] - config.source[1]];
                let length = (away[0] * away[0] + away[1] * away[1]).sqrt();
                let unit = if length > 0. { [away[0] / length, away[1] / length] } else { [0.; 2] };
                let force = [
                    unit[0] * particle.radial - unit[1] * particle.tangential + config.gravity[0],
                    unit[1] * particle.radial + unit[0] * particle.tangential + config.gravity[1],
                ];
                particle.direction[0] += force[0] * dt;
                particle.direction[1] += force[1] * dt;
                particle.position[0] += particle.direction[0] * dt * STEP_SCALE;
                particle.position[1] += particle.direction[1] * dt * STEP_SCALE;
            }
            for channel in 0..4 {
                particle.colour[channel] = (particle.colour[channel] + particle.colour_step[channel] * dt).clamp(0., 255.);
            }
            particle.size = (particle.size + particle.size_step * dt).max(0.);
            particle.rotation += particle.rotation_step * dt;
            record.position = [particle.position[0] + offset[0], particle.position[1] + offset[1], 0.];
            record.rotation = particle.rotation;
            record.scale = [particle.size, particle.size, 1.];
            let [alpha, blue, green, red] = particle.colour;
            record.color = Some([blue as u8, green as u8, red as u8]);
            record.alpha = alpha as u8;
            index += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_pairs_convert_units() {
        let keys = parse_plist(b"<plist><dict><key>speed</key><real>60</real><key>sourcePositiony</key><integer>100</integer><key>name</key><string>x</string></dict></plist>");
        assert_eq!(keys["speed"], 60.);
        assert!(!keys.contains_key("name"));
        let config = Config::from(&keys, 720.);
        assert_eq!(config.speed.0, 1.);
        assert_eq!(config.source[1], 620.);
    }
}
