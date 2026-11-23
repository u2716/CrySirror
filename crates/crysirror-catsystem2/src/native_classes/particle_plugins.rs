// SPDX-License-Identifier: MPL-2.0
//! The emotion particle plugins of `ptclpi` (em_note, em_double, em_question, em_question_a,
//! em_sweat, em_angry) as engine code. The originals are Windows DLLs (MakeInstance →
//! vtable init/update) writing 148-byte records that 5F61A0 copies into the task's quads;
//! each plugin's behaviour is restated here from its static code. No DLL is loaded.
use crate::motion::RandomStreams;
use anyhow::Result;
use crysirror_core::state::ClientId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Note,
    Double,
    Question,
    /// em_question_a: identical except that it holds still in its last phase.
    QuestionStill,
    Sweat,
    Angry,
    Rain,
    Dash,
    FilmNoise,
    Blur,
    Steam,
    Common,
}

/// 148-byte plugin record: quad corners, pivot +0x60, position +0x6C, rotation +0x78,
/// scale +0x84.
#[derive(Clone, Debug)]
pub(crate) struct Record {
    /// Corners replaced by the plugin, centred on the origin.
    pub corners: Option<[[f32; 2]; 4]>,
    /// Texture coordinates replaced by the plugin (+0x10 of each vertex).
    pub uvs: Option<[[f32; 2]; 4]>,
    /// Vertex z values replaced by the plugin (+8 of each vertex).
    pub depths: Option<[f32; 4]>,
    /// Blue, green, red of every vertex (+0xC of each).
    pub color: Option<[u8; 3]>,
    /// Per-vertex alpha, overriding `alpha`.
    pub alphas: Option<[u8; 4]>,
    /// +0x90: a hidden record draws nothing.
    pub visible: bool,
    pub pivot: [f32; 3],
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: [f32; 3],
    pub alpha: u8,
}

impl Default for Record {
    /// 5F6500: what the task's own quad table holds, opaque.
    fn default() -> Self {
        Self { corners: None, uvs: None, depths: None, color: None, alphas: None, visible: true, pivot: [0.; 3], position: [0.; 3], rotation: 0., scale: [1.; 3], alpha: 255 }
    }
}

/// The Hermite path of 1780/18F0/1C40.
#[derive(Clone, Debug, Default)]
pub(crate) struct Path {
    points: Vec<[f32; 3]>,
    /// Node i's distance from node i - 1.
    lengths: Vec<f32>,
    tangents: Vec<[f32; 3]>,
    total: f32,
    duration: f32,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn length(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn scaled(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
/// Direction times magnitude; a zero vector divides as the original does only when non-zero.
fn along(a: [f32; 3], magnitude: f32) -> [f32; 3] {
    let l = length(a);
    if l > 0. { scaled(a, magnitude / l) } else { [0.; 3] }
}

impl Path {
    /// 1780.
    pub(crate) fn add(&mut self, point: [f32; 3]) {
        let distance = self.points.last().map_or(0., |last| length(sub(point, *last)));
        self.points.push(point);
        self.lengths.push(distance);
        self.total += distance;
    }
    /// 18F0: tangents of the given magnitude, chords at the ends and between neighbours inside.
    pub(crate) fn build(&mut self, duration: f32, magnitude: f32) {
        let n = self.points.len();
        if duration == 0. || n < 2 {
            return;
        }
        self.tangents = vec![[0.; 3]; n];
        for i in 1..n - 1 {
            self.tangents[i] = along(sub(self.points[i + 1], self.points[i - 1]), magnitude);
        }
        self.tangents[0] = along(sub(self.points[1], self.points[0]), magnitude);
        self.tangents[n - 1] = along(sub(self.points[n - 1], self.points[n - 2]), magnitude);
        self.duration = duration;
    }
    /// 1C40.
    pub(crate) fn at(&self, time: f32) -> [f32; 3] {
        let n = self.points.len();
        if n == 0 {
            return [0.; 3];
        }
        if n < 2 || time <= 0. || self.duration == 0. {
            return self.points[0];
        }
        let distance = time.min(self.duration) / self.duration * self.total;
        let mut covered = 0.;
        let mut segment = 1;
        while segment < n && covered + self.lengths[segment] <= distance {
            covered += self.lengths[segment];
            segment += 1;
        }
        if segment == n {
            return self.points[n - 1];
        }
        let u = (distance - covered) / self.lengths[segment];
        let (p0, p1, t0, t1) = (self.points[segment - 1], self.points[segment], self.tangents[segment - 1], self.tangents[segment]);
        let (u2, u3) = (u * u, u * u * u);
        let mut out = [0.; 3];
        for axis in 0..3 {
            out[axis] = (2. * u3 - 3. * u2 + 1.) * p0[axis] + (-2. * u3 + 3. * u2) * p1[axis] + (u3 - 2. * u2 + u) * t0[axis] + (u3 - u2) * t1[axis];
        }
        out
    }
}

/// Host vtable +0x74 (65B900).
fn lerp(from: f32, to: f32, span: f32, at: f32) -> f32 {
    if at <= 0. {
        return from;
    }
    if at >= span {
        return to;
    }
    at / span * (to - from) + from
}

/// Host vtable +0x78 (65B9A0): a triangle wave of the given period.
fn triangle(x: f32, period: f32) -> f32 {
    let (x, period) = (x.abs(), period.abs());
    let a = if x > 0. { x % period } else { 0. };
    let q = period * 0.25;
    let r = if a > 0. { a % q } else { 0. } / q;
    if q > a {
        r
    } else if q * 2. > a {
        1. - r
    } else if q * 3. > a {
        -r
    } else {
        r - 1.
    }
}

#[derive(Clone, Debug, Default)]
struct Sub {
    clock: i32,
    mark: i32,
    phase: i32,
    /// Root: fade in, hold and fade out frames (+0x2AC..); question: resting rotation.
    times: [i32; 3],
    rest: f32,
    /// A trail copy's frozen position.
    frozen: [f32; 3],
    path: Path,
}
impl Sub {
    fn elapsed(&self) -> i32 {
        self.clock - self.mark
    }
    fn enter(&mut self, phase: i32) {
        self.phase = phase;
        self.mark = self.clock;
    }
}

/// What a plugin's update receives from the host.
pub(crate) struct Tick<'a> {
    pub canvas: [u32; 2],
    pub texture: [f32; 2],
    pub base: [f32; 2],
    pub parameters: Vec<f32>,
    pub dt: i32,
    pub random: &'a mut RandomStreams,
    pub client: ClientId,
}
/// A plugin restated in its own module.
pub(crate) trait Custom: Send {
    fn update(&mut self, records: &mut [Record], tick: &mut Tick) -> Result<()>;
}

/// Parameter words at info +0x14 as floats, read by each init.
pub(crate) struct Plugin {
    kind: Kind,
    start: [f32; 3],
    flag: i32,
    size: f32,
    times: [i32; 9],
    half: [f32; 2],
    /// +0x70 / +0x6C: 1 for an image over 256 tall, else 0.5.
    large: f32,
    center: [f32; 2],
    half_height: f32,
    active: usize,
    roots: Vec<Sub>,
    /// Trail copies of the single root (em_angry, em_sweat), and their records' index.
    trail: Vec<Sub>,
    records: Vec<Record>,
    rain: Option<super::particle_rain::Rain>,
    dash: Option<super::particle_dash::Dash>,
    custom: Option<Box<dyn Custom>>,
    /// `cmnptcl`'s configuration file, read before the first update.
    plist: Option<Vec<u8>>,
    initialized: bool,
}

impl Plugin {
    pub(crate) fn kind_of(name: &[u8]) -> Option<Kind> {
        match name {
            b"em_note" => Some(Kind::Note),
            b"em_double" => Some(Kind::Double),
            b"em_question" => Some(Kind::Question),
            b"em_question_a" => Some(Kind::QuestionStill),
            b"em_sweat" => Some(Kind::Sweat),
            b"em_angry" => Some(Kind::Angry),
            b"rain" => Some(Kind::Rain),
            b"dash" => Some(Kind::Dash),
            b"filmnoise" => Some(Kind::FilmNoise),
            b"blur" => Some(Kind::Blur),
            b"steam" => Some(Kind::Steam),
            b"cmnptcl" => Some(Kind::Common),
            _ => None,
        }
    }
    pub(crate) fn new(kind: Kind) -> Self {
        Self { rain: None, dash: None, custom: None, plist: None, kind, start: [0.; 3], flag: 0, size: 0., times: [0; 9], half: [0.; 2], large: 0.5, center: [0.; 2], half_height: 0., active: 1, roots: Vec::new(), trail: Vec::new(), records: Vec::new(), initialized: false }
    }
    pub(crate) fn set_plist(&mut self, plist: Option<Vec<u8>>) {
        self.plist = plist;
    }
    pub(crate) fn records(&self) -> &[Record] {
        &self.records
    }
    /// Vtable +4, sent by 5F5AE3 when the task (re)starts.
    pub(crate) fn restart(&mut self) {
        self.initialized = false;
    }

    fn init(&mut self, count: usize, canvas: [u32; 2], texture: [f32; 2], parameters: &[u8], random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let word = |at: usize| f32::from_le_bytes(parameters[at * 4..at * 4 + 4].try_into().unwrap());
        self.records = vec![Record::default(); count];
        if self.kind == Kind::Rain {
            self.rain = Some(super::particle_rain::Rain::new(&mut self.records, core::array::from_fn(word), canvas, texture, random, client)?);
            self.initialized = true;
            return Ok(());
        }
        if matches!(self.kind, Kind::FilmNoise | Kind::Blur | Kind::Steam | Kind::Common) {
            let words: Vec<f32> = (0..parameters.len() / 4).map(word).collect();
            let mut tick = Tick { canvas, texture, base: [0.; 2], parameters: words.clone(), dt: 0, random, client };
            self.custom = Some(match self.kind {
                Kind::FilmNoise => Box::new(super::particle_effects::FilmNoise::new(&mut self.records, &mut tick)?) as Box<dyn Custom>,
                Kind::Blur => Box::new(super::particle_effects::Blur::new(&mut self.records, &words, &mut tick)?),
                Kind::Common => Box::new(super::particle_cmn::CmnParticles::new(self.plist.as_deref(), &mut self.records, &words, &mut tick)?),
                _ => Box::new(super::particle_effects::Steam::new(&mut self.records, &words, &mut tick)?),
            });
            self.initialized = true;
            return Ok(());
        }
        if self.kind == Kind::Dash {
            self.dash = Some(super::particle_dash::Dash::new(&mut self.records, core::array::from_fn(word), texture, random, client)?);
            self.initialized = true;
            return Ok(());
        }
        self.half = [texture[0] * 0.5, texture[1] * 0.5];
        self.large = if texture[1] > 256. { 1. } else { 0.5 };
        self.center = [canvas[0] as f32 * 0.5, canvas[1] as f32 * 0.5];
        self.half_height = texture[1] * 0.5;
        self.active = 1;
        let size = |value: f32| (value / 100.).max(0.001);
        let int = |at: usize| word(at) as i32;
        // Each init reads its own run of words after the shared info header.
        let (roots, trail) = match self.kind {
            // Answered above; its drops are not roots.
            Kind::Rain | Kind::Dash | Kind::FilmNoise | Kind::Blur | Kind::Steam | Kind::Common => (0, 0),
            Kind::Note => {
                self.start = [word(1), word(2), 0.];
                self.flag = int(3);
                self.size = size(word(4));
                self.times = [int(5), int(6), int(7), 0, 0, 0, 0, 0, 0];
                let per = int(0).max(1) as usize;
                (count / per, 0)
            }
            Kind::Double | Kind::Question | Kind::QuestionStill => {
                self.start = [word(0), word(1), 0.];
                self.flag = int(2);
                self.size = size(word(3));
                self.times = [int(4), int(5), int(6), int(7), int(8), 0, 0, 0, 0];
                (count, 0)
            }
            Kind::Sweat => {
                self.start = [word(0), word(1), 0.];
                self.size = size(word(2));
                self.times = [int(3), int(4), 0, 0, 0, 0, 0, 0, 0];
                (usize::from(count > 0), count.saturating_sub(1))
            }
            Kind::Angry => {
                self.start = [word(0), word(1), 0.];
                self.size = size(word(2));
                for (at, slot) in self.times.iter_mut().enumerate() {
                    *slot = int(3 + at);
                }
                (usize::from(count > 0), count.saturating_sub(1))
            }
        };
        self.roots = vec![Sub::default(); roots];
        self.trail = vec![Sub::default(); trail];
        for index in 0..roots {
            self.spawn(index, random, client)?;
        }
        self.initialized = true;
        Ok(())
    }

    fn centred(&self) -> [[f32; 2]; 4] {
        let [w, h] = self.half;
        [[-w, -h], [w, -h], [-w, h], [w, h]]
    }

    fn spawn(&mut self, index: usize, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let start = self.start;
        let s = self.size;
        let mut record = Record { position: start, alpha: 0, ..Default::default() };
        let mut sub = Sub::default();
        match self.kind {
            Kind::Rain | Kind::Dash | Kind::FilmNoise | Kind::Blur | Kind::Steam | Kind::Common => {}
            // 24E0.
            Kind::Note => {
                record.corners = Some(self.centred());
                let scale = random.float_value(client)? * s * 0.4 + s * 0.6;
                record.scale = [scale, scale, 1.];
                for slot in 0..3 {
                    let t = self.times[slot] as f32;
                    sub.times[slot] = (random.float_value(client)? * t * 0.5 + t * 0.5) as i32;
                }
                record.rotation = random.float_value(client)? - 0.5;
                let sign = if self.flag == 0 { -1. } else { 1. };
                let reach = s * 80. * self.large;
                let heading = along([sign, -1., 0.], 1.);
                let first = reach + random.float_value(client)? * reach;
                let mut point = [start[0] + heading[0] * first, start[1] + heading[1] * first, start[2] + heading[2] * first];
                let direction = along(sub_direction(point, start), 1.);
                let (sin, cos) = 50f32.to_radians().sin_cos();
                let left = [direction[0] * cos + direction[1] * sin, -direction[0] * sin + direction[1] * cos];
                let right = [direction[0] * cos - direction[1] * sin, direction[0] * sin + direction[1] * cos];
                for _ in 0..4 {
                    let dx = (right[0] - left[0]).abs();
                    let dy = (right[1] - left[1]).abs();
                    let nx = random.float_value(client)? * dx - dx * 0.5 + direction[0];
                    let ny = random.float_value(client)? * dy - dy * 0.5 + direction[1];
                    let next = along([nx, ny, 0.], 1.);
                    let step = reach + random.float_value(client)? * reach;
                    point = [point[0] + next[0] * step, point[1] + next[1] * step, point[2]];
                    sub.path.add(point);
                }
                let duration = (sub.times[0] + sub.times[1] + sub.times[2]) as f32;
                sub.path.build(duration, s * 120. * self.large);
            }
            // 1900.
            Kind::Double => {
                record.corners = Some(self.centred());
                record.scale = [s * 3., s * 3., 1.];
                record.rotation = if self.flag == 0 { -0.4 } else { 0.4 };
            }
            // 19C0.
            Kind::Question | Kind::QuestionStill => {
                record.corners = Some(self.centred());
                record.scale = [s, s, 1.];
                sub.rest = if self.flag == 0 { -0.5 } else { 0.5 };
                record.rotation = sub.rest;
            }
            // 2560.
            Kind::Sweat => {
                record.pivot = [self.half[0], self.half[1], 0.];
                record.scale = [s, s, 1.];
                sub.path.add(start);
                sub.path.add([start[0], start[1] + s * 150. * self.large, 0.]);
                sub.path.build((self.times[0] + self.times[1]) as f32, 10.);
            }
            // 1CC0.
            Kind::Angry => {
                record.pivot = [self.half[0], self.half[1], 0.];
                record.scale = [s * 0.1, s * 0.1, 1.];
            }
        }
        if matches!(self.kind, Kind::Sweat | Kind::Angry) {
            // The trail copies start hidden on the root's pose.
            for (at, copy) in self.trail.iter_mut().enumerate() {
                *copy = Sub::default();
                self.records[1 + at] = Record { alpha: 0, ..record.clone() };
            }
        }
        self.records[index] = record;
        self.roots[index] = sub;
        Ok(())
    }

    /// Vtable +8: the elapsed milliseconds are the plugin's clock.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(&mut self, count: usize, canvas: [u32; 2], texture: [f32; 2], parameters: &[u8], base: [f32; 2], dt: i32, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        if !self.initialized {
            return self.init(count, canvas, texture, parameters, random, client);
        }
        if let Some(rain) = &mut self.rain {
            return rain.update(&mut self.records, canvas, texture, dt, random, client);
        }
        if let Some(custom) = &mut self.custom {
            let words = parameters.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            let mut tick = Tick { canvas, texture, base, parameters: words, dt, random, client };
            return custom.update(&mut self.records, &mut tick);
        }
        if let Some(dash) = &mut self.dash {
            return dash.update(&mut self.records, texture, dt, random, client);
        }
        let offset = [base[0] - self.center[0], base[1] - self.center[1]];
        if self.kind == Kind::Note {
            // 21C0: one more emitter joins on a 1-in-51 draw.
            if random.integer_word(client)? % 51 == 0 {
                self.active += 1;
            }
            self.active = self.active.min(self.roots.len());
        } else {
            self.active = self.roots.len();
        }
        for index in 0..self.active {
            match self.kind {
                Kind::Note => self.update_note(index, random, client)?,
                Kind::Double => self.update_double(index, random, client)?,
                Kind::Question | Kind::QuestionStill => self.update_question(index, random, client)?,
                Kind::Sweat => self.update_sweat(index),
                Kind::Angry => self.update_angry(index),
                Kind::Rain | Kind::Dash | Kind::FilmNoise | Kind::Blur | Kind::Steam | Kind::Common => {}
            }
            let record = &mut self.records[index];
            record.position[0] += offset[0];
            record.position[1] += offset[1];
            self.roots[index].clock = self.roots[index].clock.wrapping_add(dt);
        }
        if matches!(self.kind, Kind::Sweat | Kind::Angry) && !self.roots.is_empty() {
            self.update_trail(offset, dt);
        }
        Ok(())
    }

    fn update_note(&mut self, index: usize, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let phase = self.roots[index].phase;
        if !(0..=2).contains(&phase) {
            return self.spawn(index, random, client);
        }
        let sub = &mut self.roots[index];
        let point = sub.path.at(sub.clock as f32);
        let e = sub.elapsed();
        let record = &mut self.records[index];
        record.position[0] = point[0];
        record.position[1] = point[1];
        match phase {
            0 => {
                record.alpha = lerp(0., 255., sub.times[0] as f32, e as f32) as u8;
                if e > sub.times[0] {
                    sub.enter(1);
                }
            }
            1 => {
                if e > sub.times[1] {
                    sub.enter(2);
                }
            }
            _ => {
                record.alpha = lerp(255., 0., sub.times[2] as f32, e as f32) as u8;
                if e > sub.times[2] {
                    sub.enter(10);
                }
            }
        }
        Ok(())
    }

    fn update_double(&mut self, index: usize, random: &mut RandomStreams, client: ClientId) -> Result<()> {
        let [t0, t1, t2, ..] = self.times;
        let s = self.size;
        let start = self.start;
        let phase = self.roots[index].phase;
        if !(0..=2).contains(&phase) {
            return self.spawn(index, random, client);
        }
        let sub = &mut self.roots[index];
        let e = sub.elapsed();
        let record = &mut self.records[index];
        match phase {
            0 => {
                record.position = start;
                let scale = lerp(s * 3., s, t0 as f32, e as f32);
                record.scale = [scale, scale, 1.];
                record.alpha = lerp(0., 255., t0 as f32, e as f32) as u8;
                if e > t0 {
                    sub.enter(1);
                }
            }
            1 => {
                let jitter = lerp(s * 35. * self.large, 0., t1 as f32, e as f32);
                let (x, y) = match e.rem_euclid(4) {
                    0 => (-jitter, -jitter),
                    1 => (jitter, jitter),
                    2 => (jitter, -jitter),
                    _ => (-jitter, jitter),
                };
                record.position = [start[0] + x, start[1] + y, 0.];
            }
            _ => {
                record.position = start;
                if e > t2 {
                    sub.enter(1);
                }
            }
        }
        Ok(())
    }

    fn update_question(&mut self, index: usize, _random: &mut RandomStreams, _client: ClientId) -> Result<()> {
        let [t0, t1, t2, t3, t4, ..] = self.times;
        let s = self.size;
        let still = self.kind == Kind::QuestionStill;
        let start = self.start;
        let sub = &mut self.roots[index];
        let e = sub.elapsed();
        let at = e as f32;
        let record = &mut self.records[index];
        match sub.phase {
            0 => {
                record.scale[1] = lerp(s, s * 1.5, t0 as f32, at);
                record.scale[0] = lerp(s, s * 0.4, t0 as f32, at);
                record.alpha = lerp(0., 128., t0 as f32, at) as u8;
                if e > t0 {
                    sub.enter(1);
                }
            }
            1 => {
                record.scale[1] = lerp(s * 1.5, s * 0.5, t1 as f32, at);
                record.scale[0] = lerp(s * 0.4, s * 2.4, t1 as f32, at);
                record.alpha = lerp(128., 255., t1 as f32, at) as u8;
                if e > t1 {
                    sub.enter(2);
                }
            }
            2 => {
                record.scale[1] = lerp(s * 0.5, s, t2 as f32, at);
                record.scale[0] = lerp(s * 2.4, s, t2 as f32, at);
                if e > t2 {
                    sub.enter(3);
                }
            }
            3 => {
                if !still && e > t3 {
                    sub.enter(4);
                }
            }
            4 => {
                let angle = std::f32::consts::TAU / t4 as f32 * at;
                record.rotation += angle.cos() / 50.;
                if e > t4 {
                    record.rotation = sub.rest;
                    sub.enter(3);
                }
            }
            _ => {}
        }
        record.position = start;
        Ok(())
    }

    fn update_sweat(&mut self, index: usize) {
        let [t0, t1, ..] = self.times;
        let sub = &mut self.roots[index];
        let point = sub.path.at(sub.clock as f32);
        let e = sub.elapsed();
        let record = &mut self.records[index];
        record.position[0] = point[0];
        record.position[1] = point[1];
        match sub.phase {
            0 => {
                record.alpha = lerp(0., 255., t0 as f32, e as f32) as u8;
                if e > t0 {
                    sub.enter(1);
                }
            }
            1 => {
                if e > t1 {
                    sub.enter(10);
                }
            }
            _ => {}
        }
    }

    fn update_angry(&mut self, index: usize) {
        let t = self.times;
        let s = self.size;
        let start = self.start;
        let large = self.large;
        let sub = &mut self.roots[index];
        let e = sub.elapsed();
        let at = e as f32;
        let record = &mut self.records[index];
        record.position = start;
        let shake = start[0] + triangle(at, 4.) * s * 4. * large;
        match sub.phase {
            0 => {
                let scale = lerp(s * 0.1, s * 1.4, t[0] as f32, at);
                record.scale = [scale, scale, 1.];
                record.rotation = lerp(0., 1., t[1] as f32, at);
                record.alpha = lerp(0., 255., t[0] as f32, at) as u8;
                if e > t[0] {
                    sub.enter(1);
                }
            }
            1 => {
                let scale = lerp(s * 1.4, s, t[1] as f32, at);
                record.scale = [scale, scale, 1.];
                record.rotation = lerp(1., 0.3, t[1] as f32, at);
                if e > t[1] {
                    sub.enter(2);
                }
            }
            2 => {
                record.position[0] = shake;
                if e > t[2] {
                    sub.enter(3);
                }
            }
            3 => {
                record.position[0] = shake;
                let scale = lerp(s, s * 1.1, t[3] as f32, at);
                record.scale = [scale, scale, 1.];
                if e > t[3] {
                    sub.enter(4);
                }
            }
            4 => {
                record.position[0] = shake;
                let scale = lerp(s * 1.2, s, t[4] as f32, at);
                record.scale = [scale, scale, 1.];
                if e > t[4] {
                    sub.enter(2);
                }
            }
            _ => {}
        }
    }

    /// Each tick one free copy freezes the root's pose (position before the screen offset),
    /// one unit behind it, then fades out in place; em_angry's also grow by 1%.
    fn update_trail(&mut self, offset: [f32; 2], dt: i32) {
        let angry = self.kind == Kind::Angry;
        let root_phase = self.roots[0].phase;
        let root = self.records[0].clone();
        let frozen = [root.position[0] - offset[0], root.position[1] - offset[1], root.position[2]];
        let (fade_start, fade_span, end) = if angry { (30., 29., 29) } else { (30., 22., 25) };
        if angry || root_phase <= 1 {
            if let Some(at) = self.trail.iter().position(|copy| copy.phase == 0) {
                let record = &mut self.records[1 + at];
                record.position = [frozen[0], frozen[1], frozen[2] - 1.];
                if angry {
                    record.rotation = root.rotation;
                    record.scale = root.scale;
                }
                self.trail[at].frozen = frozen;
                self.trail[at].phase = 1;
            }
        }
        for (at, copy) in self.trail.iter_mut().enumerate() {
            if copy.phase == 0 {
                continue;
            }
            let record = &mut self.records[1 + at];
            record.alpha = lerp(fade_start, 0., fade_span, copy.clock as f32) as u8;
            if angry {
                let grown = record.scale[0] * 1.01;
                record.scale[0] = grown;
                record.scale[1] = grown;
            }
            record.position[0] = copy.frozen[0] + offset[0];
            record.position[1] = copy.frozen[1] + offset[1];
            copy.clock = copy.clock.wrapping_add(dt);
            if copy.clock > end {
                record.alpha = 0;
                copy.clock = 0;
                copy.phase = 0;
            }
        }
    }
}

/// 1730 for the spawn's first chord.
fn sub_direction(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    sub(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_lerp_and_path_follow_the_host_helpers() {
        assert_eq!(triangle(0., 4.), 0.);
        assert_eq!(triangle(1., 4.), 1.);
        assert_eq!(triangle(2., 4.), 0.);
        assert_eq!(triangle(3., 4.), -1.);
        assert_eq!(lerp(0., 255., 10., 5.), 127.5);
        assert_eq!(lerp(0., 255., 10., 11.), 255.);
        let mut path = Path::default();
        path.add([0., 0., 0.]);
        path.add([10., 0., 0.]);
        path.build(10., 1.);
        assert_eq!(path.at(0.), [0., 0., 0.]);
        assert!((path.at(10.)[0] - 10.).abs() < 1e-3);
        assert!(path.at(5.)[0] > 0. && path.at(5.)[0] < 10.);
    }
}
