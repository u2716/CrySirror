//! D.C.III (DC3.exe, 0x8a71) opcode 0x27: 424FE0 case 39 -> 40AB30.
//!
//! This is the older weather system shared with DC2 (417010), not DC3WY's
//! 41DAC0 particle pools. Kinds:
//!  * 0: 4073C0 + 4088C0 + 409200 + 40A050 + 409810 (everything off).
//!  * 1, 3, 4, 9, 11 (40A0C0): up to 50 petals from original slot 17
//!    (SACRA01/02, YUKI, SAKU_YUKI02); count 47AA50[p & 15] (50 from 5),
//!    wind p / 16 through 4072C0. 40A530 moves them on every second call,
//!    407300 draws atlas cell (32 * 47A8D0[shape][frame], 28 * shape, 32x24)
//!    with 412270's FF00FF-keyed plain blit (414B50 alpha for an alpha
//!    atlas) into layer 1 (flag set) or layer 7. Kind 9 rises (y = 576 - y)
//!    and retires petals instead of respawning them.
//!  * 2 (4084B0): rain, min(p, 3) modes with 10/20/50 drops from slot 18
//!    (RAINCHIP); 408600 moves them every call, 4087A0 draws 1-px columns
//!    (source x 2 * life, height 24/56/104) into layer 1.
//!  * 7 (4091F0): fog mode C65298 = p, drawn by 423290 from original slot
//!    16. Mode 3 scrolls layer 2 (rows 0..576) and layer 8 (rows 600..1176)
//!    left by 2048 with 402CC0 records {0x1104, 4, 2048, 0, 512/256}; other
//!    modes draw the slot statically on layer 8. 0 (41EBD0) stops drawing.
//!  * 6 (4090E0 / 40A7A0): background pan from the 0x69 mode-2 start to
//!    its target over 10 * p calls; 409190 snaps to the target at the end.
//!  * 5, 8, 10: not used by the shipped scripts; they fail visibly.
//! 40AD20 runs from the 30 ms idle timer (418870 -> 4214F0) and 42BA70;
//! this module steps once per 30 ms of engine time. 7E0C7C (var(99,10))
//! off: kinds other than 6/7 keep their selector but create nothing.
//!
//! Known differences: fog mode 3 uses the shared cloud objects 52/53 (above
//! the portraits, while native layer 2 is below them); periods are converted
//! from the 30 ms timer to 60 Hz sequence frames.
use super::*;
use crate::dc2::{WeatherFrame, WeatherGroup};
use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, SpriteResourceBinding, SpriteResourceRequest, linear};
use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation};

/// 418870 re-arms the idle timer at 30 ms.
const TICK_MS: u64 = 30;
/// Original slots 17/18 keep their numbers in the DC3WY slot space.
const PETAL_SLOT: u8 = 17;
const RAIN_SLOT: u8 = 18;
/// Original slot 16 EFF\FOGX* (3072x1200) is translated to slot 21.
const FOG_SCROLL_SLOT: u8 = 21;
/// 47A8D0: atlas columns per shape row, -1 terminated.
const FRAMES: [[i32; 16]; 6] = [
    [0, 1, 2, 3, 4, 5, 6, -1, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, -1, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, -1, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, -1, 0, 0, 0, 0, 0, 0, 0],
];
/// 47AA50 petal counts for p & 15 = 0..4.
const COUNTS: [i32; 5] = [0, 5, 10, 15, 30];
/// Painter orders: native layer 1 sits on the background (layer 0) below
/// portraits (3..6); layer 7 is above them and below fog layer 8.
const BEHIND: [i32; 2] = [0, 100];
const FRONT: [i32; 2] = [1, 50 * 64 + 51];

/// 481EFC -> trunc(4096 * cos(2 * pi * i / 4096)), read through 407260.
fn sway(phase: i32, amplitude: i32) -> i32 {
    let index = (((phase % 360) << 12) / 360).rem_euclid(4096);
    let cosine = (4096.0 * (std::f64::consts::TAU * f64::from(index) / 4096.0).cos()) as i32;
    amplitude * cosine / 4096
}

/// One 100-byte record of 7E7128 (dword index in the comments).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Petal {
    active: bool,
    /// 0: atlas row.
    shape: i32,
    /// 3/4: drawn position; 5/6: sway origin.
    position: [i32; 2],
    origin: [i32; 2],
    /// 14: fall per step; 15: frame interval; 16: frame counter.
    fall: i32,
    interval: i32,
    counter: i32,
    /// 18: layer 1 instead of layer 7.
    behind: bool,
    /// 19: atlas column index; 21: sway amplitude; 22/24: phase step/phase.
    frame: i32,
    amplitude: i32,
    phase_step: i32,
    phase: i32,
}
/// One 16-byte record of 7E6E08: x, y, length variant, life.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Drop {
    x: i32,
    y: i32,
    variant: i32,
    life: i32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OriginalWeather {
    /// C6529C / C652A0.
    pub petal_kind: i32,
    pub petal_parameter: i32,
    petals: Vec<Petal>,
    /// 7E9300 / 7E9304.
    wind_mode: i32,
    wind: i32,
    /// 7E9340.
    toggle: bool,
    /// C652A4 (= 7E688C).
    pub rain: i32,
    drops: Vec<Drop>,
    /// C65298.
    pub fog: i32,
    /// C6528C / C65294.
    pub other: i32,
    other_parameter: i32,
    next_tick: Option<u64>,
    /// A presentation frame is owed to the host.
    dirty: bool,
    /// The host currently shows a frame.
    shown: bool,
}

impl OriginalWeather {
    fn rand(rng: &mut crysirror_vfx::xorshift::Xorshift128) -> i32 {
        rng.next_i31()
    }
    /// 40A0C0 with 7E0C7C set.
    fn start_petals(&mut self, kind: i32, parameter: i32, rng: &mut crysirror_vfx::xorshift::Xorshift128) {
        let mut r = || Self::rand(rng);
        self.petals = vec![Petal::default(); 50];
        let low = (parameter & 15) as usize;
        let count = COUNTS.get(low).copied().unwrap_or(50);
        for petal in self.petals.iter_mut().take(count as usize) {
            petal.active = true;
            let x = r() % 1024;
            let y = -32 - r() % 300;
            petal.origin = [x, y];
            let shape = r() % 6;
            petal.shape = shape;
            match shape {
                0..=2 => {
                    petal.behind = false;
                    petal.amplitude = r() % 24 + 24;
                }
                3 => {
                    petal.behind = r() % 2 != 0;
                    petal.amplitude = r() % 16 + 16;
                }
                _ => {
                    petal.behind = true;
                    petal.amplitude = r() % 8 + 8;
                }
            }
            petal.phase_step = r() % 8 + 4;
            petal.interval = r() % 8 + 4;
            petal.fall = match kind {
                3 => {
                    let fall = r() % 2 + 2;
                    petal.amplitude /= 4;
                    fall
                }
                4 => r() % 4 + 5,
                11 => r() % 10 + 10,
                9 => r() % 15 + 15,
                _ => r() % 4 + 2,
            };
            petal.position = [x + sway(0, petal.amplitude), y];
        }
        //4072C0(mode, speed); p / 16 >= 5 leaves the previous wind.
        let speed = match kind {
            4 => 4,
            11 => 8,
            _ => 2,
        };
        let wind = match parameter / 16 {
            0 => Some((0, 0)),
            1 => Some((1, speed)),
            2 => Some((2, speed)),
            3 => Some((1, 3 * speed)),
            4 => Some((2, 3 * speed)),
            _ => None,
        };
        if let Some((mode, speed)) = wind {
            self.wind_mode = mode;
            self.wind = match mode {
                1 => speed,
                2 => -speed,
                _ => 0,
            };
        }
    }
    /// 40A530. Returns false once no petal is active (4073C0).
    fn step_petals(&mut self, rng: &mut crysirror_vfx::xorshift::Xorshift128) -> bool {
        self.toggle = !self.toggle;
        if !self.toggle {
            return true;
        }
        let mut r = || Self::rand(rng);
        let mut any = false;
        for p in self.petals.iter_mut().filter(|p| p.active) {
            any = true;
            p.origin[0] += self.wind;
            let x = sway(p.phase, p.amplitude) + p.origin[0];
            let y = p.position[1] + p.fall;
            p.phase += p.phase_step;
            p.position = [x, y];
            let outside = y >= 576
                || match self.wind_mode {
                    1 => x >= 1024,
                    2 => x + 32 < 0,
                    _ => false,
                };
            if outside {
                if self.petal_kind == 9 {
                    p.active = false;
                    continue;
                }
                let origin = match self.wind_mode {
                    1 => {
                        if r() % 2 == 0 {
                            let x = r() % 1024;
                            [x, -32 - r() % 64]
                        } else {
                            let x = -32 - r() % 64;
                            [x, r() % 400]
                        }
                    }
                    2 => {
                        if r() % 2 == 0 {
                            let x = r() % 1024;
                            [x, r() % 64 - 64]
                        } else {
                            let x = r() % 64 + 1024;
                            [x, r() % 400]
                        }
                    }
                    _ => {
                        let x = r() % 1024;
                        [x, -(r() % 300)]
                    }
                };
                p.origin = origin;
                p.phase = 0;
                p.position = [origin[0] + sway(0, p.amplitude), origin[1]];
            }
            p.counter += 1;
            if p.counter >= p.interval {
                p.frame += 1;
                p.counter = 0;
                if FRAMES[p.shape as usize][p.frame as usize] == -1 {
                    p.frame = 0;
                }
            }
        }
        any
    }
    /// 4084B0 with 7E0C7C set.
    fn start_rain(&mut self, mode: i32, rng: &mut crysirror_vfx::xorshift::Xorshift128) {
        let mut r = || Self::rand(rng);
        let count = match mode {
            1 => 10,
            2 => 20,
            3 => 50,
            _ => 0,
        };
        self.drops = (0..count)
            .map(|_| {
                let x = r() % 1024;
                let y = r() % 576;
                let (variant, life) = match mode {
                    1 => (0, r() % 4 + 1),
                    _ => {
                        let variant = r() % if mode == 2 { 2 } else { 3 };
                        (variant, r() % 4 + 2)
                    }
                };
                Drop { x, y, variant, life }
            })
            .collect();
    }
    /// 408600.
    fn step_rain(&mut self, rng: &mut crysirror_vfx::xorshift::Xorshift128) {
        let mut r = || Self::rand(rng);
        let mode = self.rain;
        for d in &mut self.drops {
            d.y += match (mode, d.variant) {
                (1, _) | (_, 0) => 32,
                (_, 1) => 64,
                (3, 2) => 112,
                _ => 0,
            };
            d.life -= 1;
            if d.life < 0 {
                d.x = r() % 1024;
                if mode == 1 {
                    let y = r();
                    d.variant = 0;
                    d.y = y % 576 - 24;
                    d.life = r() % 4 + 1;
                } else {
                    d.y = r() % 576 - 24;
                    d.variant = r() % if mode == 2 { 2 } else { 3 };
                    d.life = r() % 4 + 2;
                }
            }
        }
    }
    fn active(&self) -> bool {
        self.petal_kind != 0 || self.rain != 0
    }
    /// 407300 then 4087A0 into layers 1 and 7.
    fn frame(&self) -> WeatherFrame {
        let mut behind = Vec::new();
        let mut front = Vec::new();
        if self.petal_kind != 0 {
            for p in self.petals.iter().filter(|p| p.active) {
                let column = FRAMES[p.shape as usize][p.frame as usize].max(0);
                let y = if self.petal_kind == 9 { 576 - p.position[1] } else { p.position[1] };
                let sprite = ([32 * column, 28 * p.shape, 32, 24], [p.position[0], y]);
                if p.behind { behind.push(sprite) } else { front.push(sprite) }
            }
        }
        let mut groups = Vec::new();
        if !behind.is_empty() {
            groups.push(WeatherGroup { texture_slot: PETAL_SLOT, weight: 16, order: BEHIND, sprites: behind });
        }
        if self.rain != 0 && !self.drops.is_empty() {
            let sprites = self
                .drops
                .iter()
                .filter_map(|d| {
                    let height = match (self.rain, d.variant) {
                        (_, 0) => 24,
                        (2 | 3, 1) => 56,
                        (3, 2) => 104,
                        _ => return None,
                    };
                    Some(([2 * d.life, 0, 1, height], [d.x, d.y]))
                })
                .collect();
            groups.push(WeatherGroup { texture_slot: RAIN_SLOT, weight: 16, order: [BEHIND[0], BEHIND[1] + 1], sprites });
        }
        if !front.is_empty() {
            groups.push(WeatherGroup { texture_slot: PETAL_SLOT, weight: 16, order: FRONT, sprites: front });
        }
        WeatherFrame { canvas: [1024, 576], groups }
    }
}

impl Machine {
    fn effects_enabled(&self) -> Result<bool> {
        Ok(self.value(99, 10)? != 0)
    }
    /// D.C.III 0x69 mode 2 (424FE0 case 105): store the pan start and
    /// target for a later 0x27 6; nothing moves yet.
    pub(super) fn execute_original_pan_target(&mut self, program: &Program, instruction: &Instruction, a: [i16; 8]) -> Result<Option<Outcome>> {
        if !self.state.original || instruction.opcode != 0x7f || program.original_opcode(instruction) != Some(0x69) || a[2] != 2 {
            return Ok(None);
        }
        self.state.original_pan = Some([a[0], a[1]]);
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
    /// 424FE0 case 39 for the translated D.C.III program.
    pub(super) fn execute_original_weather(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        if !self.state.original || instruction.opcode != 0x27 {
            return Ok(None);
        }
        let (kind, parameter) = (i32::from(a[0]), i32::from(a[1]));
        //With the operand 0 and CB9A78 (a 0x2F/0x30 portrait declaration),
        //421780(0) first builds the declared portrait layers.
        if parameter == 0 {
            let portraits: Vec<(u8, String)> = self
                .state
                .resources
                .pending
                .iter()
                .filter(|(slot, _)| (2..=9).contains(*slot))
                .map(|(slot, name)| (*slot, name.clone()))
                .collect();
            for (slot, name) in portraits {
                if !self.publish_texture(host, slot, &name)? {
                    return Ok(Some(Outcome::Pending));
                }
            }
        }
        ensure!(kind != 255, "D.C.III 0x27 255 (7E92F0 layer animation pause) is not connected");
        let mut weather = self.state.original_weather.clone().unwrap_or_default();
        let enabled = self.effects_enabled()?;
        match kind {
            0 => {
                if !self.original_fog(host, 0)? {
                    return Ok(Some(Outcome::Pending));
                }
                if weather.other == 6 && !self.finish_original_pan(host)? {
                    return Ok(Some(Outcome::Pending));
                }
                weather = OriginalWeather { shown: weather.shown, dirty: true, ..Default::default() };
            }
            1 | 3 | 4 | 9 | 11 => {
                if parameter == 0 {
                    weather.petal_kind = 0;
                    weather.petals.clear();
                } else {
                    weather.petal_kind = kind;
                    weather.petal_parameter = parameter;
                    if enabled {
                        if !self.publish_pending_slot(host, PETAL_SLOT)? {
                            return Ok(Some(Outcome::Pending));
                        }
                        ensure!(self.state.image_names.contains_key(&PETAL_SLOT), "D.C.III 0x27 petals: original slot 17 is empty");
                        weather.start_petals(kind, parameter, &mut self.state.original_rng);
                    } else {
                        weather.petals.clear();
                    }
                }
                weather.dirty = true;
            }
            2 => {
                if parameter == 0 {
                    weather.rain = 0;
                    weather.drops.clear();
                } else {
                    weather.rain = parameter.min(3);
                    if enabled {
                        if !self.publish_pending_slot(host, RAIN_SLOT)? {
                            return Ok(Some(Outcome::Pending));
                        }
                        ensure!(self.state.image_names.contains_key(&RAIN_SLOT), "D.C.III 0x27 rain: original slot 18 is empty");
                        weather.start_rain(weather.rain, &mut self.state.original_rng);
                    } else {
                        weather.drops.clear();
                    }
                }
                weather.dirty = true;
            }
            7 => {
                if !self.original_fog(host, parameter)? {
                    return Ok(Some(Outcome::Pending));
                }
                weather.fog = parameter;
            }
            6 => {
                //4090E0: 10 * max(p, 1) calls of 40A7A0 from the 0x69 start.
                let target = self.state.original_pan.context("D.C.III 0x27 6 without a 0x69 mode-2 pan target")?;
                let calls = 10 * parameter.max(1);
                let frames = u32::try_from(i64::from(calls) * TICK_MS as i64 * 60 / 1000)?.max(1);
                let request = SpriteRequest {
                    commands: vec![SpriteCommand {
                        seed: self.object_seed(0),
                        operations: vec![
                            SequenceOperation::Finish { cancel: false, lanes: None },
                            SequenceOperation::Enqueue {
                                lane: 0,
                                bank: None,
                                track: linear(vec![0, 1], target.map(f32::from).to_vec(), frames, vec![0, 1], vec![0, 1], DeltaSource::Retained, false, true),
                            },
                        ],
                    }],
                };
                if !host.sprite(&request)? {
                    return Ok(Some(Outcome::Pending));
                }
                let background = self.state.sprites.entry(0).or_default();
                background.actor = true;
                background.position = target;
                weather.other = 6;
                weather.other_parameter = parameter;
            }
            _ => bail!("D.C.III 0x27 {kind} ({parameter}) is not connected (40AB30 default branch)"),
        }
        if weather.active() && weather.next_tick.is_none() {
            weather.next_tick = Some(self.state.original_clock + TICK_MS);
        }
        if !weather.active() {
            weather.next_tick = None;
        }
        self.state.original_weather = Some(weather);
        if !self.present_original_weather(host)? {
            return Ok(Some(Outcome::Pending));
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
    /// 409190: the pan ends at its target and C63CD8/DC clear.
    fn finish_original_pan(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if !host.sprite(&SpriteRequest {
            commands: vec![SpriteCommand { seed: self.object_seed(0), operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }] }],
        })? {
            return Ok(false);
        }
        self.state.original_pan = None;
        Ok(true)
    }
    fn publish_pending_slot(&mut self, host: &mut dyn NativeHost, slot: u8) -> Result<bool> {
        if let Some(name) = self.state.resources.pending.get(&slot).cloned() {
            return self.publish_texture(host, slot, &name);
        }
        Ok(true)
    }
    /// 4091F0 / 41EBD0 over the translated fog slots: SYSTEM\FOG* (20,
    /// bound to object 54 on load) or EFF\FOGX* (21, scrolled on 52/53).
    fn original_fog(&mut self, host: &mut dyn NativeHost, mode: i32) -> Result<bool> {
        if !self.publish_pending_slot(host, FOG_SCROLL_SLOT)? {
            return Ok(false);
        }
        if mode == 0 {
            if self.state.clouds {
                if !self.publish_texture(host, FOG_SCROLL_SLOT, "-")? {
                    return Ok(false);
                }
                self.state.clouds = false;
            }
            return self.stop_fog(host);
        }
        self.state.fog = true;
        if self.state.image_names.contains_key(&FOG_SCROLL_SLOT) && !self.state.clouds {
            if mode == 3 {
                //{0x1104, 4, 2048, 0, 512/256}: one step per four 4034A0
                //calls, so 2048 and 1024 calls of 30 ms.
                let frames = |calls: u64| u32::try_from(calls * TICK_MS * 60 / 1000).unwrap_or(u32::MAX);
                if !self.start_cloud_scroll(host, [frames(2048), frames(1024)])? {
                    return Ok(false);
                }
            } else {
                //Static draw on layer 8 at (0, 0): the slot's top-left extent.
                if host
                    .sprite_resources(&SpriteResourceRequest {
                        bindings: vec![SpriteResourceBinding { slot: FOG_SCROLL_SLOT, object: 53, crop: Some([0, 0, 1024, 576]) }],
                    })?
                    .is_none()
                {
                    return Ok(false);
                }
            }
            self.state.clouds = true;
        }
        Ok(true)
    }
    /// 40AD20 on the 30 ms timer; never blocks the script.
    pub(super) fn tick_original_weather(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<()> {
        self.state.original_clock = now;
        let Some(mut weather) = self.state.original_weather.take() else { return Ok(()) };
        while let Some(due) = weather.next_tick.filter(|due| now >= *due) {
            if weather.petal_kind != 0 && !weather.step_petals(&mut self.state.original_rng) {
                weather.petal_kind = 0;
                weather.petals.clear();
            }
            if weather.rain != 0 {
                weather.step_rain(&mut self.state.original_rng);
            }
            weather.dirty = true;
            weather.next_tick = weather.active().then_some(due + TICK_MS);
            //A long stall (load, menu) catches up at most one second.
            if now.saturating_sub(due) > 1000 {
                weather.next_tick = weather.next_tick.map(|_| now + TICK_MS);
            }
        }
        self.state.original_weather = Some(weather);
        self.present_original_weather(host)?;
        Ok(())
    }
    fn present_original_weather(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let Some(weather) = self.state.original_weather.as_mut() else { return Ok(true) };
        if !weather.dirty {
            return Ok(true);
        }
        let frame = weather.frame();
        let ok = if frame.groups.is_empty() {
            !weather.shown || host.dc2_weather(None)?
        } else {
            host.dc2_weather(Some(&frame))?
        };
        if ok {
            weather.shown = !frame.groups.is_empty();
            weather.dirty = false;
        }
        Ok(ok)
    }
    /// Next 30 ms weather wakeup.
    pub(super) fn original_weather_due(&self) -> Option<u64> {
        self.state.original_weather.as_ref().and_then(|w| w.next_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn petal_density_and_wind() {
        let mut rng = crysirror_vfx::xorshift::Xorshift128::default();
        let mut w = OriginalWeather::default();
        w.start_petals(1, 0x12, &mut rng);
        assert_eq!(w.petals.iter().filter(|p| p.active).count(), 10);
        assert_eq!((w.wind_mode, w.wind), (1, 2));
        w.petal_kind = 1;
        assert!(w.step_petals(&mut rng));
        let frame = w.frame();
        assert_eq!(frame.groups.iter().map(|g| g.sprites.len()).sum::<usize>(), 10);
    }
    #[test]
    fn rising_petals_retire() {
        let mut rng = crysirror_vfx::xorshift::Xorshift128::default();
        let mut w = OriginalWeather { petal_kind: 9, ..Default::default() };
        w.start_petals(9, 1, &mut rng);
        let mut steps = 0;
        while w.step_petals(&mut rng) {
            steps += 1;
            assert!(steps < 10_000);
        }
    }
    #[test]
    fn rain_modes() {
        let mut rng = crysirror_vfx::xorshift::Xorshift128::default();
        let mut w = OriginalWeather { rain: 3, ..Default::default() };
        w.start_rain(3, &mut rng);
        assert_eq!(w.drops.len(), 50);
        w.step_rain(&mut rng);
        assert!(w.drops.iter().all(|d| (0..3).contains(&d.variant)));
    }
}
