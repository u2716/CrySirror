// SPDX-License-Identifier: MPL-2.0
//! CreateEffect "Rain", "Drop", "FloatBubble" and "LookupRain" (setup 438A10,
//! 437100, 4385C0, 4390D0; per-frame callbacks 438E90, 437D60, 4389B0, 439360).
//! Each effect owns a fresh transparent w*h surface. Particles walk precomputed
//! straight paths; the native clock adds `speed` path points per 16 ms. The
//! original seeds `rand()` from wall-clock time, so a deterministic generator
//! seeded from the entity id replaces it: motion is statistically equivalent,
//! never frame-identical. Pixel work stays in shared `streak_field` and
//! `tile_compose` operations.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Rain,
    Drop,
    Bubble,
    /// 4390D0 creates its own white strip with46EC10(1,16,-1,0x80),
    /// wraps it with491B80, and selects one fixed random Z angle. The width
    /// rounds to4; packed -1 OR alpha<<24 yields alpha255, not128.
    Lookup,
}
impl Kind {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "Rain" => Self::Rain,
            "Drop" => Self::Drop,
            "FloatBubble" => Self::Bubble,
            "LookupRain" => Self::Lookup,
            _ => return None,
        })
    }
}
struct Particle {
    from: [i32; 2],
    to: [i32; 2],
    points: i32,
    /// 16.16 path index.
    index: u64,
    speed: i32,
    /// Sprite scale in per mille.
    scale: i32,
    rotation: [i32; 3],
    angle: [i32; 3],
}
impl Particle {
    fn at(&self, i: i32) -> [i32; 2] {
        let n = i64::from(self.points.max(1));
        let i = i64::from(i.clamp(0, self.points));
        let lerp = |a: i32, b: i32| (i64::from(a) + (i64::from(b) - i64::from(a)) * i / n) as i32;
        [
            lerp(self.from[0], self.to[0]),
            lerp(self.from[1], self.to[1]),
        ]
    }
}
pub(super) struct Particles {
    kind: Kind,
    size: [u32; 2],
    particles: Vec<Particle>,
    sprite: Option<(String, [u32; 2])>,
    last: u64,
    job: Option<ImageJob>,
    lookup_angle: i32,
}
pub(super) struct Rng(u64);
impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    fn next(&mut self) -> i32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 33) & 0x7fff_ffff) as i32
    }
    /// `(rand() + 1) % n` as used throughout the native setups.
    fn below(&mut self, n: i32) -> i32 {
        (self.next() + 1) % n.max(1)
    }
}
/// 4325A0: alpha ramp of one rain drop, `((e+20)<<24)|0xADADAD`.
fn rain_ramp() -> Vec<u8> {
    let mut e = Vec::with_capacity(196);
    for k in 0..98 {
        e.push((f64::from(k) * 1.142857193946838) as i32);
    }
    for k in 98..196 {
        let previous: i32 = e[k - 1];
        e.push(previous - 1);
    }
    e.into_iter().map(|v| (v + 20) as u8).collect()
}
impl N2SystemVm {
    fn effect_number(&self, name: &str) -> Option<f64> {
        self.machine
            .variables
            .get(name)
            .and_then(|v| v.number().ok())
            .map(f64::from)
    }
    pub(super) fn native_create_particles(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        kind: Kind,
    ) -> Result<bool> {
        let (w, h) = (args.integer(4)?, args.integer(5)?);
        ensure!(
            (1..=2048).contains(&w) && (1..=2048).contains(&h),
            "N2System {kind:?} extent"
        );
        let size = [w as u32, h as u32];
        let sprite_path = match kind {
            Kind::Drop => self
                .machine
                .variables
                .get("SYSTEM_effect_drop_image")
                .and_then(|v| v.string().ok())
                .filter(|s| !s.is_empty()),
            Kind::Bubble => Some("data/particle.png".to_owned()),
            _ => None,
        };
        let mut sprite = None;
        if let Some(path) = sprite_path {
            if state.particle_sprite.is_none() {
                let metadata = self.images.as_ref().unwrap().clone();
                let probe = path.clone();
                match self.pool.as_ref().unwrap().spawn(move |cancel| {
                    cancel.check()?;
                    Ok(metadata.size(&probe)?)
                }) {
                    Ok(task) => state.particle_sprite = Some(task),
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                }
            }
            let task = state.particle_sprite.as_mut().unwrap();
            let measured = match task.try_take() {
                Ok(Some(size)) => size,
                Ok(None) => return Ok(false),
                Err(_) => {
                    // 4385C0/437100 abort the effect when the sprite cannot
                    // be loaded; the script continues without it.
                    state.particle_sprite = None;
                    host.append_log(
                        self.client,
                        &format!(
                            "N2System: {kind:?} sprite {path:?} unavailable; effect not created"
                        ),
                    )?;
                    self.machine.complete(&mut state.thread, Value::Null)?;
                    return Ok(true);
                }
            };
            state.particle_sprite = None;
            ensure!(
                measured.iter().all(|n| (1..=4096).contains(n)),
                "N2System particle sprite extent"
            );
            sprite = Some((path, measured));
        }
        let id = self.visual(host, current, args, json!({"kind":"group"}), size, None)?;
        self.bitmap_formats.insert(
            id,
            bitmap_read::BitmapFormat {
                origin: None,
                alpha: Some(true),
            },
        );
        let mut rng = Rng::new((u64::from(id) << 32) ^ host.now_ms() ^ 0xa076_1d64_78bd_642f);
        let particles = self.spawn_particles(kind, size, sprite.as_ref().map(|s| s.1), &mut rng);
        let lookup_angle = if kind == Kind::Lookup {
            rng.next() % 360
        } else {
            0
        };
        self.particles.insert(
            id,
            Particles {
                kind,
                size,
                particles,
                sprite,
                last: 0,
                job: None,
                lookup_angle,
            },
        );
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    fn spawn_particles(
        &self,
        kind: Kind,
        size: [u32; 2],
        sprite: Option<[u32; 2]>,
        rng: &mut Rng,
    ) -> Vec<Particle> {
        let (w, h) = (size[0] as i32, size[1] as i32);
        let mut out = Vec::new();
        match kind {
            Kind::Rain => {
                let count = self
                    .effect_number("SYSTEM_effect_rain_dencity")
                    .map_or(128, |v| (v as i32).min(128))
                    .max(0);
                let mut speeds: [i32; 12] = std::array::from_fn(|k| 152 - 8 * k as i32);
                if let Some(v) = self.effect_number("SYSTEM_effect_rain_speed") {
                    let mut value = v as i32;
                    for slot in &mut speeds {
                        *slot = value.max(1);
                        value -= 8;
                    }
                }
                for i in 0..count {
                    let seed = if i == 0 { 64 } else { rng.next() };
                    let x0 = rng.below(w);
                    let y0 = -(seed % 128 + rng.below(h >> 1));
                    let delta = rng.below(64);
                    let x1 = if seed % 2 != 0 {
                        x0 + delta
                    } else {
                        x0 - delta
                    };
                    let x1 = x1.clamp(0, w);
                    out.push(Particle {
                        from: [x0, y0],
                        to: [x1, h],
                        points: (h - y0).clamp(1, 3999),
                        index: 0,
                        speed: speeds[rng.below(12) as usize],
                        scale: 1000,
                        rotation: [0; 3],
                        angle: [0; 3],
                    });
                }
            }
            Kind::Drop => {
                let [sw, sh] = sprite.unwrap_or([64, 64]).map(|n| n as i32);
                let count = self
                    .effect_number("SYSTEM_effect_drop_dencity")
                    .map_or(256, |v| (v as i32).min(256))
                    .max(0);
                let base = [4, 3, 4, 3, 2, 3, 1, 2, 2, 1, 1, 2];
                let weight = [1., 0.75, 1., 0.75, 0.5, 0.75, 0.3, 0.5, 0.5, 0.3, 0.3, 0.5];
                let faster = self.effect_number("SYSTEM_effect_drop_speed");
                let table: Vec<i32> = (0..12)
                    .map(|k| base[k] + faster.map_or(0, |v| (v * weight[k]) as i32))
                    .collect();
                let swing = self
                    .effect_number("SYSTEM_effect_drop_swing")
                    .map_or(0, |v| v as i32);
                let axis = |name: &str| self.effect_number(name).map_or(0, |v| v as i32);
                let turns = [
                    axis("SYSTEM_effect_drop_rotate_x"),
                    axis("SYSTEM_effect_drop_rotate_y"),
                    axis("SYSTEM_effect_drop_rotate_z"),
                ];
                let scales = [1000, 400, 800, 200];
                for i in 0..count {
                    let kind = rng.below(4) as usize;
                    let x0 = rng.below(w);
                    let y0 = -rng.below(h >> 1);
                    let sway = if swing > 0 {
                        rng.below(swing * 2 + 1) - swing
                    } else {
                        0
                    };
                    let delta = rng.below(128) * if rng.next() % 2 == 0 { 1 } else { -1 };
                    let y1 = h + 2 * sh;
                    out.push(Particle {
                        from: [x0, y0],
                        to: [x0 + delta + sway, y1],
                        points: (y1 - y0).clamp(1, 3999),
                        index: 0,
                        speed: table[3 * kind + rng.below(3) as usize].max(1),
                        scale: scales[kind],
                        rotation: turns.map(|n| if n > 0 { i % n + 1 } else { 0 }),
                        angle: [0; 3],
                    });
                    let _ = sw;
                }
            }
            Kind::Bubble => {
                let [_, sh] = sprite.unwrap_or([64, 64]).map(|n| n as i32);
                let scales = [1000, 600, 800, 200];
                let speeds = [10, 9, 8, 7, 6, 5, 4, 3, 4, 3, 2, 3];
                for _ in 0..16 {
                    let kind = rng.below(4) as usize;
                    let x0 = rng.below(w);
                    let top = -rng.below((h >> 1).max(1));
                    let drift = rng.below(128) * if rng.next() % 2 == 0 { 1 } else { -1 };
                    let y0 = h + sh + 128;
                    out.push(Particle {
                        from: [x0 + drift, y0],
                        to: [x0, top],
                        points: (y0 - top).clamp(1, 3999),
                        index: 0,
                        speed: speeds[3 * kind + rng.below(3) as usize],
                        scale: scales[kind],
                        rotation: [0; 3],
                        angle: [0; 3],
                    });
                }
            }
            Kind::Lookup => {}
        }
        out
    }
    pub(super) fn update_particles(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let ids = self.particles.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let fx = self.particles.get_mut(&id).unwrap();
            if let Some(job) = &mut fx.job {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(e) => bail!("N2System {:?}: {e}", fx.kind),
                    ImageProcessState::Ready(image) => {
                        fx.job = None;
                        self.publish_capture(host, id, image)?;
                    }
                }
            }
            let fx = self.particles.get_mut(&id).unwrap();
            // Rain/Drop update after 42 ms (438E90/437D60); the others every frame.
            let interval = match fx.kind {
                Kind::Rain | Kind::Drop => 42,
                Kind::Lookup => 32,
                _ => 16,
            };
            if now.saturating_sub(fx.last) < interval {
                continue;
            }
            let elapsed = if fx.last == 0 { 16 } else { now - fx.last };
            fx.last = now;
            for p in &mut fx.particles {
                p.index += (i64::from(p.speed) * elapsed as i64 * 65536 / 16) as u64;
                if (p.index >> 16) as i32 >= p.points {
                    p.index = 0;
                }
                for axis in 0..3 {
                    p.angle[axis] = (p.angle[axis] + p.rotation[axis] * 3) % 360;
                }
            }
            let [w, h] = fx.size;
            let (input, effects) = match fx.kind {
                Kind::Rain => {
                    let streaks = fx
                        .particles
                        .iter()
                        .map(|p| p.at((p.index >> 16) as i32))
                        .collect::<Vec<_>>();
                    (
                        ImageInput::Solid {
                            size: fx.size,
                            color: [0; 4],
                        },
                        serde_json::to_vec(&json!([{"kind":"streak_field","spec":{
                            "streaks":streaks,"ramp":rain_ramp(),"color":[173,173,173]}}]))?,
                    )
                }
                Kind::Drop | Kind::Bubble => {
                    let (path, [sw, sh]) = fx.sprite.clone().context("N2System particle sprite")?;
                    let tiles = fx
                        .particles
                        .iter()
                        .map(|p| {
                            let [x, y] = p.at((p.index >> 16) as i32);
                            let k = p.scale * 256 / 1000;
                            let flip = |degrees: i32| {
                                ((f64::from(degrees).to_radians().cos()) * f64::from(k)) as i32
                            };
                            json!({"src":[0,0,sw,sh],
                                "dst":[x - sw as i32 / 2, y - sh as i32 / 2],
                                "scale":[flip(p.angle[1]), flip(p.angle[0])],"opacity":255})
                        })
                        .collect::<Vec<_>>();
                    (
                        ImageInput::Asset {
                            path,
                            size: [sw, sh],
                            fingerprint: None,
                        },
                        serde_json::to_vec(&json!([{"kind":"tile_compose","spec":{
                            "canvas":[w,h],"tiles":tiles}}]))?,
                    )
                }
                // 439360 clears/repaints after32ms, but never advances the
                // angle or its unused speed slot. Do not invent a spinner or
                // an external texture dependency from the effect's name.
                Kind::Lookup => (
                    ImageInput::Solid {
                        size: [4, 16],
                        color: [255; 4],
                    },
                    serde_json::to_vec(&json!([{"kind":"tile_compose","spec":{
                        "canvas":fx.size,"tiles":[{"src":[0,0,4,16],
                            "dst":[(w/2) as i32-2,(h/2) as i32-8],
                            "scale":[256,256],"opacity":255,"rotation_degrees":fx.lookup_angle}]
                    }}]))?,
                ),
            };
            match self
                .images
                .as_ref()
                .unwrap()
                .begin_process(input, Arc::from(effects))
            {
                Ok(job) => self.particles.get_mut(&id).unwrap().job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
