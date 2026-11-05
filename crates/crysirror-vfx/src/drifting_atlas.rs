//! Externally clocked streak and buoyant atlas pools sharing an owner RNG.
use crate::{Particle, falling_atlas::SequenceRng};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftMotion {
    /// Downward streaks with a fixed atlas row and clipped lower edge.
    Streak,
    /// Signed vertical drift, sinusoidal horizontal motion and animated atlas rows.
    Buoyant,
    /// Downward sinusoidal motion with a slow translucent pulse.
    FallingGlow,
    /// Upward sinusoidal motion with a full-range alpha pulse.
    RisingGlow,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriftSpawnWindow {
    /// Inclusive spawn rectangle [left, top, right, bottom], in canvas pixels.
    pub bounds: [i32; 4],
    /// Draw in 0..100 passes when it is at most this value; zero disables bias.
    pub chance: i32,
    /// Maximum rejection-sampling attempts per biased coordinate.
    pub attempts: u16,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriftingAtlasSpec {
    /// Logical canvas width and height, in pixels.
    pub canvas: [u32; 2],
    /// Atlas texture width and height, in pixels.
    pub texture_extent: [u32; 2],
    /// Number of explicit simulation steps per second.
    pub tick_hz: u32,
    /// Pool depth in 0..5, selecting the buoyant atlas row pair.
    pub depth: u32,
    /// Motion and atlas-layout preset consumed by this node.
    pub motion: DriftMotion,
    #[serde(default)]
    /// Optional biased respawn area; updates preserve live particle attributes.
    pub spawn_window: Option<DriftSpawnWindow>,
}
impl DriftingAtlasSpec {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.canvas
                .iter()
                .chain(&self.texture_extent)
                .all(|v| *v > 0 && *v <= 16384)
                && self.tick_hz > 0
                && self.tick_hz <= 1000
                && self.depth < 5,
            "drifting atlas geometry/clock"
        );
        match self.motion {
            DriftMotion::Streak => ensure!(
                self.texture_extent[0] >= 18 && self.texture_extent[1] >= 72,
                "streak atlas extent"
            ),
            DriftMotion::Buoyant | DriftMotion::FallingGlow | DriftMotion::RisingGlow => ensure!(
                self.texture_extent[0] >= 252 && self.texture_extent[1] >= (self.depth + 2) * 28,
                "buoyant atlas extent"
            ),
        }
        if let Some(window) = &self.spawn_window {
            ensure!(
                window.attempts <= 1000
                    && window.bounds.iter().all(|v| v.unsigned_abs() <= 1_000_000),
                "drifting spawn window"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriftingAtlasState {
    /// Horizontal orbit center, in canvas pixels.
    pub center_x: f32,
    /// Horizontal oscillation phase, in radians.
    pub phase: f32,
    /// Signed vertical displacement per simulation tick, in pixels.
    pub speed: f32,
    /// Horizontal oscillation radius, in pixels.
    pub amplitude: u32,
    /// Constant horizontal offset in tenths of a pixel.
    pub horizontal: i32,
    /// Selected atlas row index.
    pub row: u32,
    /// Current animation frame within that row.
    pub frame: u32,
    /// Previous atlas frame timestamp, in wrapping milliseconds.
    pub clock: u32,
    /// Current straight alpha in 0..1.
    pub alpha: f32,
    /// Whether the alpha pulse is moving toward its upper bound.
    pub rising: bool,
    /// Multiplicative pixel size, retained across streak respawns.
    pub scale: f32,
}
#[derive(Clone, Debug)]
pub struct CompiledDriftingAtlas {
    spec: DriftingAtlasSpec,
}
impl CompiledDriftingAtlas {
    pub fn compile(spec: DriftingAtlasSpec) -> Result<Self> {
        spec.validate()?;
        Ok(Self { spec })
    }
    pub fn update_window(&mut self, window: Option<DriftSpawnWindow>) -> Result<()> {
        let mut next = self.spec.clone();
        next.spawn_window = window;
        next.validate()?;
        self.spec = next;
        Ok(())
    }
    fn draw(rng: &mut SequenceRng, m: u32) -> Result<u32> {
        Ok(rng.next_u32()? % m)
    }
    pub fn initialize(
        &self,
        p: &mut Particle,
        rng: &mut SequenceRng,
        clock: u32,
        inherited_scale: f32,
    ) -> Result<DriftingAtlasState> {
        let s = &self.spec;
        let mut x = Self::draw(rng, s.canvas[0])? as f32;
        let sample_y = Self::draw(rng, s.canvas[1])? as f32;
        let mut y = if matches!(s.motion, DriftMotion::Streak | DriftMotion::FallingGlow) {
            -sample_y
        } else {
            sample_y
        };
        if let Some(w) = &s.spawn_window {
            if w.chance != 0
                && w.bounds[0] < w.bounds[2]
                && (s.motion != DriftMotion::Buoyant || w.bounds[1] < w.bounds[3])
                && Self::draw(rng, 100)? as i32 <= w.chance
            {
                for _ in 0..w.attempts {
                    x = Self::draw(rng, s.canvas[0])? as f32;
                    if x >= w.bounds[0] as f32 && x <= w.bounds[2] as f32 {
                        break;
                    }
                }
                if s.motion == DriftMotion::Buoyant {
                    for _ in 0..w.attempts {
                        y = Self::draw(rng, s.canvas[1])? as f32;
                        if y >= w.bounds[1] as f32 && y <= w.bounds[3] as f32 {
                            break;
                        }
                    }
                }
            }
        }
        let mut state = DriftingAtlasState {
            center_x: x,
            phase: 0.,
            speed: 0.,
            amplitude: 0,
            horizontal: 0,
            row: 0,
            frame: 0,
            clock,
            alpha: 0.,
            rising: true,
            scale: if s.motion == DriftMotion::Streak {
                inherited_scale
            } else {
                1.
            },
        };
        match s.motion {
            DriftMotion::Streak => {
                state.speed = (Self::draw(rng, 4)? + 24) as f32;
                state.row = Self::draw(rng, 3)?;
                state.alpha = (0.5 - f64::from(Self::draw(rng, 4)?) * 0.1000000014901161) as f32;
            }
            DriftMotion::Buoyant => {
                state.speed = (f64::from(Self::draw(rng, 4)?) * 0.2000000029802322
                    + 0.2000000029802322) as f32;
                if Self::draw(rng, 2)? == 0 {
                    state.speed = -state.speed;
                }
                state.amplitude = Self::draw(rng, 32)? + 28;
                state.horizontal = Self::draw(rng, 3)? as i32;
                if Self::draw(rng, 2)? == 0 {
                    state.horizontal = -state.horizontal;
                }
                state.row = s.depth + Self::draw(rng, 2)?;
                state.phase = Self::draw(rng, 360)? as f32;
            }
            DriftMotion::FallingGlow | DriftMotion::RisingGlow => {
                let falling = s.motion == DriftMotion::FallingGlow;
                state.speed = if falling {
                    (f64::from(Self::draw(rng, 3)?) * 2.5 + 0.5) as f32
                } else {
                    (f64::from(Self::draw(rng, 4)? / 2) + 0.2000000029802322) as f32
                };
                state.amplitude = if falling {
                    Self::draw(rng, 16)? + 4
                } else {
                    Self::draw(rng, 32)? + 28
                };
                state.horizontal = if falling { 4 } else { 2 };
                state.row = s.depth + Self::draw(rng, 2)?;
                state.phase = Self::draw(rng, 360)? as f32;
                state.frame = Self::draw(rng, [7, 7, 6, 8, 5, 8][state.row as usize])?;
                if falling {
                    state.alpha = (f64::from(Self::draw(rng, 10)?) / 10.) as f32;
                    state.rising = rng.next_u32()? != 0;
                }
            }
        }
        p.position = [x, y, 0.];
        p.previous_position = p.position;
        p.age = 0.;
        p.lifetime = f32::MAX;
        p.color[3] = state.alpha;
        let cell = self.cell(&state);
        p.size = [cell[0] as f32 * state.scale, cell[1] as f32 * state.scale];
        p.velocity = [
            0.,
            state.speed
                * s.tick_hz as f32
                * if matches!(s.motion, DriftMotion::Buoyant | DriftMotion::RisingGlow) {
                    -1.
                } else {
                    1.
                },
            0.,
        ];
        Ok(state)
    }
    fn cell(&self, state: &DriftingAtlasState) -> [u32; 2] {
        match self.spec.motion {
            DriftMotion::Streak => [2, 24 * (state.row + 1)],
            _ => [28, 28],
        }
    }
    fn uv_height(&self, state: &DriftingAtlasState, height: u32) -> [f32; 4] {
        let (x, y) = match self.spec.motion {
            DriftMotion::Streak => (8 * state.row, 0),
            _ => (32 * state.frame, 28 * state.row),
        };
        let cell = self.cell(state);
        [
            x as f32 / self.spec.texture_extent[0] as f32,
            y as f32 / self.spec.texture_extent[1] as f32,
            (x + cell[0]) as f32 / self.spec.texture_extent[0] as f32,
            (y + height) as f32 / self.spec.texture_extent[1] as f32,
        ]
    }
    pub fn initial_uv(&self, state: &DriftingAtlasState) -> [f32; 4] {
        // The initial crop starts at column zero; the independently seeded
        // frame counter is consumed only by the first integration step.
        let mut initial = state.clone();
        initial.frame = 0;
        self.uv_height(&initial, self.cell(state)[1])
    }
    pub fn step(
        &self,
        p: &mut Particle,
        state: &mut DriftingAtlasState,
        rng: &mut SequenceRng,
        wall_ms: u64,
        emitting: bool,
    ) -> Result<crate::falling_atlas::FallingStep> {
        let s = &self.spec;
        p.previous_position = p.position;
        let cell = self.cell(state);
        let recycle;
        let mut height = cell[1];
        match s.motion {
            DriftMotion::Streak => {
                p.position[0] = state.center_x;
                p.position[1] += state.speed;
                if p.position[1] + height as f32 > s.canvas[1] as f32 {
                    height = (s.canvas[1] as f32 - p.position[1]).max(0.) as u32;
                }
                recycle = p.position[1] > s.canvas[1] as f32 - cell[1] as f32;
            }
            DriftMotion::Buoyant | DriftMotion::FallingGlow | DriftMotion::RisingGlow => {
                let falling = s.motion == DriftMotion::FallingGlow;
                let sin = f64::from(state.phase).sin() as f32;
                p.position[0] = (f64::from(sin) * f64::from(state.amplitude)
                    + f64::from(state.center_x)
                    + if s.motion == DriftMotion::Buoyant {
                        f64::from(state.horizontal) / 10.
                    } else {
                        0.
                    }) as f32;
                state.phase = (f64::from(state.phase)
                    + if falling {
                        0.05000000074505806
                    } else {
                        0.009999999776482582
                    }) as f32;
                if s.motion == DriftMotion::RisingGlow && state.phase >= 360. {
                    state.phase -= 360.;
                }
                if (wall_ms as u32).wrapping_sub(state.clock) as i32 > 500 {
                    state.clock = wall_ms as u32;
                    state.frame += 1;
                    let counts = [7, 7, 6, 8, 5, 8];
                    if state.frame >= counts[state.row as usize] {
                        state.frame = 0;
                    }
                }
                let alpha_step = if falling {
                    0.002000000094994903
                } else {
                    0.004999999888241291
                };
                let alpha_max = if s.motion == DriftMotion::RisingGlow {
                    1.
                } else {
                    0.800000011920929
                };
                if state.rising {
                    state.alpha = (f64::from(state.alpha) + alpha_step) as f32;
                    if f64::from(state.alpha) >= alpha_max {
                        state.alpha = alpha_max as f32;
                        state.rising = false;
                    }
                } else {
                    state.alpha = (f64::from(state.alpha) - alpha_step) as f32;
                    if state.alpha <= 0. {
                        state.alpha = 0.;
                        state.rising = true;
                    }
                }
                p.color[3] = state.alpha;
                p.position[1] += if falling { state.speed } else { -state.speed };
                if falling && p.position[1] + height as f32 > s.canvas[1] as f32 {
                    height = (s.canvas[1] as f32 - p.position[1]).max(0.) as u32;
                }
                recycle = if falling {
                    p.position[1] > s.canvas[1] as f32
                } else {
                    p.position[0] < -(cell[0] as f32)
                        || p.position[0] > s.canvas[0] as f32
                        || p.position[1] < -(cell[1] as f32)
                        || p.position[1] > s.canvas[1] as f32 + cell[1] as f32
                };
            }
        }
        for axis in 0..3 {
            p.velocity[axis] = (p.position[axis] - p.previous_position[axis]) * s.tick_hz as f32;
        }
        p.age += 1. / s.tick_hz as f32;
        p.size = [cell[0] as f32 * state.scale, height as f32 * state.scale];
        if recycle {
            if !emitting {
                return Ok(crate::falling_atlas::FallingStep {
                    alive: false,
                    recycled: false,
                    uv: self.uv_height(state, 0),
                });
            }
            *state = self.initialize(p, rng, state.clock, state.scale)?;
            return Ok(crate::falling_atlas::FallingStep {
                alive: true,
                recycled: true,
                uv: self.initial_uv(state),
            });
        }
        Ok(crate::falling_atlas::FallingStep {
            alive: true,
            recycled: false,
            uv: self.uv_height(state, height),
        })
    }
}
