//! DC2 opcode0x26 -> 417010 weather. Native policy: 4176C0 petal setup and
//! rand() order, 4179B0's every-other 40ms-tick motion, 50E15C atlas frame
//! rows and 4175F0's cosine sway. Pixels are shared engine Layers.
//!
//! Gap (recorded): 4258C0 draws the 417580(1) pass between the background
//! and 41A950's portraits, and 417580(0) after them. This VM's composed
//! native0 already contains background and portraits, so both passes are
//! presented above the composed screen; with portraits loaded the first pass
//! therefore covers characters it would natively pass behind.
use super::*;
use crate::machine::Event;

/// Engine image slot of the petal atlas (402FE0 surface 50E158).
pub(crate) const TEXTURE_SLOT: u8 = 47;
/// 50E15C: atlas columns per shape row, -1 terminated.
const FRAMES: [[i32; 16]; 6] = [
    [0, 1, 2, 3, 4, 5, 6, -1, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, -1, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, -1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, -1, 0, 0, 0, 0, 0, 0, 0],
];
/// 50E2DC petal counts by density0..4; 5 and above use50.
const COUNTS: [i32; 5] = [0, 5, 10, 15, 30];

/// 43D2D0 -> trunc(4096*cos(2*pi*i/4096)).
fn cosine(index: usize) -> i32 {
    (4096.0 * (std::f64::consts::TAU * index as f64 / 4096.0).cos()) as i32
}
fn sway(phase: i32, amplitude: i32) -> i32 {
    let index = (((phase % 360) << 12) / 360) as usize & 4095;
    amplitude * cosine(index) / 4096
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Petal {
    origin: [i32; 2],
    position: [i32; 2],
    shape: i32,
    front: bool,
    amplitude: i32,
    fall: i32,
    phase_step: i32,
    interval: i32,
    counter: i32,
    frame: i32,
    phase: i32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Weather {
    /// B0CE20/B0CE28.
    pub kind: i32,
    pub density: i32,
    petals: Vec<Petal>,
    wind_mode: i32,
    wind: i32,
    toggle: bool,
    texture: bool,
    next_tick: u64,
    /// CEBA24: script-paused timer.
    pub paused: bool,
    published: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherFrame {
    /// Native canvas the group layer covers.
    pub canvas: [i32; 2],
    pub groups: Vec<WeatherGroup>,
}
/// One FF00FF-keyed atlas pass at a scene painter order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherGroup {
    pub texture_slot: u8,
    /// Opacity numerator over16 (40C360 weight; 16 is the plain blit).
    pub weight: u8,
    /// Scene-root painter order of this pass.
    pub order: [i32; 2],
    /// (atlas source rect, destination).
    pub sprites: Vec<([i32; 4], [i32; 2])>,
}

fn texture_name(kind: i32) -> &'static str {
    match kind {
        11 => "system\\SAKU_YUKI01",
        12 => "system\\SAKU_YUKI02",
        3 => "system\\YUKI01",
        1 => "system\\SACRA01",
        _ => "system\\SACRA02",
    }
}

/// 4176C0(density, variant) with 417980's wind for that variant.
fn initialize(density: i32, variant: i32, rand: &mut dyn FnMut() -> Result<i32>) -> Result<(Vec<Petal>, i32, i32)> {
    let count = if density >= 5 { 50 } else { COUNTS.get(density.max(0) as usize).copied().unwrap_or(0) };
    let mut petals = Vec::with_capacity(count.max(0) as usize);
    for _ in 0..count {
        let x = rand()? % 800;
        let y = -32 - rand()? % 300;
        let choice = rand()? % 6;
        let (shape, front, amplitude) = if variant == 4 || choice >= 5 {
            (5, true, (rand()? & 7) + 8)
        } else {
            match choice {
                0..=2 => (choice, false, rand()? % 24 + 24),
                3 => {
                    let front = rand()? % 2 != 0;
                    (3, front, rand()? % 16 + 16)
                }
                _ => (4, true, (rand()? & 7) + 8),
            }
        };
        let fall = if variant == 4 { rand()? % 4 + 2 } else { rand()? % 8 + 4 };
        let phase_step = rand()? % 8 + 4;
        let interval = rand()? % 8 + 4;
        petals.push(Petal {
            origin: [x, y],
            position: [x, y],
            shape,
            front,
            amplitude,
            fall,
            phase_step,
            interval,
            ..Default::default()
        });
    }
    let (mode, wind) = match variant {
        0 => (0, 0),
        1 | 4 => (1, 2),
        2 => (2, -2),
        _ => (0, 0),
    };
    Ok((petals, mode, wind))
}

impl Weather {
    /// 4179B0. Runs on every second40ms timer tick.
    fn step(&mut self, rand: &mut dyn FnMut() -> Result<i32>) -> Result<()> {
        self.toggle = !self.toggle;
        if !self.toggle {
            return Ok(());
        }
        for p in &mut self.petals {
            p.origin[0] += self.wind;
            let x = p.origin[0] + sway(p.phase, p.amplitude);
            let y = p.position[1] + p.fall;
            p.phase += p.phase_step;
            p.position = [x, y];
            let outside = match self.wind_mode {
                1 => y >= 600 || x >= 800,
                2 => y >= 600 || x + 32 < 0,
                _ => y >= 600,
            };
            if outside {
                let (nx, ny) = match self.wind_mode {
                    1 => {
                        if rand()? & 1 == 0 {
                            let nx = rand()? % 800;
                            (nx, -32 - rand()? % 64)
                        } else {
                            let nx = -32 - rand()? % 64;
                            (nx, rand()? % 400)
                        }
                    }
                    2 => {
                        if rand()? & 1 != 0 {
                            let nx = rand()? % 64 + 800;
                            (nx, rand()? % 400)
                        } else {
                            let nx = rand()? % 800;
                            (nx, rand()? % 64 - 64)
                        }
                    }
                    _ => {
                        let nx = rand()? % 800;
                        (nx, -(rand()? % 300))
                    }
                };
                p.origin = [nx, ny];
                p.position = [nx, ny];
                p.phase = 0;
            }
            p.counter += 1;
            if p.counter >= p.interval {
                p.counter = 0;
                p.frame += 1;
                if FRAMES[p.shape as usize][p.frame as usize] == -1 {
                    p.frame = 0;
                }
            }
        }
        Ok(())
    }
    fn frame(&self) -> WeatherFrame {
        let mut petals = Vec::new();
        //417BB0 back pass (flag1 via 4258C0 bit2) before the front pass.
        for front in [true, false] {
            for p in self.petals.iter().filter(|p| p.front == front) {
                let column = FRAMES[p.shape as usize][p.frame as usize].max(0);
                petals.push(([32 * column, 28 * p.shape, 32, 24], p.position));
            }
        }
        WeatherFrame {
            canvas: [800, 600],
            groups: vec![WeatherGroup {
                texture_slot: TEXTURE_SLOT,
                weight: if self.kind == 3 { 12 } else { 8 },
                order: [2, 0],
                sprites: petals,
            }],
        }
    }
}

impl Machine {
    /// 417010(kind, density). Returns false while the atlas/rand are pending.
    pub(super) fn start_weather(&mut self, kind: i32, density: i32, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        let current = self.state.weather.as_ref().map_or(0, |w| w.kind);
        if current != kind && current != 0 {
            //416F70 releases the pool and its atlas.
            if !host.dc2_weather(None)? {
                return Ok(false);
            }
            self.state.weather = None;
        }
        let enabled = self.state.settings.prefix(20)?.unwrap_or(0) != 0;
        if kind == 0 || !enabled {
            self.state.weather = (kind != 0).then(|| Weather { kind, density, ..Default::default() });
            return Ok(true);
        }
        let variant = match kind {
            1 | 4 | 11 | 12 => 0,
            3 => 4,
            _ => bail!("DC2 weather profile {kind} (417CE0/4198C0/4199B0) is not connected"),
        };
        let mut weather = self.state.weather.clone().unwrap_or_default();
        weather.kind = kind;
        weather.density = density;
        if !weather.texture {
            if !host.emit(&Event::LoadImage { slot: TEXTURE_SLOT, name: texture_name(kind).into() })? {
                self.state.weather = Some(weather);
                return Ok(false);
            }
            weather.texture = true;
        }
        let mut result = None;
        if !host.random_batch(&mut |rand| {
            result = Some(initialize(density, variant, rand)?);
            Ok(())
        })? {
            self.state.weather = Some(weather);
            return Ok(false);
        }
        let (petals, mode, wind) = result.unwrap();
        weather.petals = petals;
        weather.wind_mode = mode;
        weather.wind = wind;
        weather.next_tick = now + 40;
        weather.published = false;
        self.state.weather = Some(weather);
        Ok(true)
    }
    /// 423450 timer5 path for weather. Never blocks the script.
    pub(super) fn tick_weather(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<()> {
        let Some(mut weather) = self.state.weather.clone() else { return Ok(()) };
        if weather.petals.is_empty() || weather.paused {
            return Ok(());
        }
        let mut changed = !weather.published;
        while now >= weather.next_tick {
            let mut failed = None;
            let ready = host.random_batch(&mut |rand| {
                if let Err(e) = weather.step(rand) {
                    failed = Some(e);
                }
                Ok(())
            })?;
            if let Some(e) = failed {
                return Err(e);
            }
            if !ready {
                break;
            }
            weather.next_tick += 40;
            changed = true;
        }
        if changed && host.dc2_weather(Some(&weather.frame()))? {
            weather.published = true;
        }
        self.state.weather = Some(weather);
        Ok(())
    }
    /// True while the weather timer needs another40ms wakeup.
    pub(super) fn weather_due(&self) -> Option<u64> {
        self.state.weather.as_ref().filter(|w| !w.petals.is_empty() && !w.paused).map(|w| w.next_tick)
    }
}
