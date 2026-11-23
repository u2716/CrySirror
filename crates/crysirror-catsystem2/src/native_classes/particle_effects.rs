// SPDX-License-Identifier: MPL-2.0
//! The `filmnoise`, `blur` and `steam` particle plugins restated from their static code;
//! see `particle_plugins`.
use super::particle_plugins::{Custom, Path, Record, Tick};
use anyhow::Result;

fn eased(from: f32, to: f32, span: f32, at: f32) -> f32 {
    if span <= 0. {
        return to;
    }
    from + (to - from) * (at / span).clamp(0., 1.)
}

/// 0x10015500 and 0x100154F0: the emission path's duration and tangent size.
const SOURCE_DURATION: f32 = 100.;
const SOURCE_MAGNITUDE: f32 = 1.;

fn cell(u: f32, v: f32, width: f32, height: f32) -> Option<[[f32; 2]; 4]> {
    Some([[u, v], [u + width, v], [u, v + height], [u + width, v + height]])
}

/// 1B90-style snap applied before every sine and cosine in these plugins.
fn snapped(angle: f32) -> (f32, f32) {
    (((angle * 1000.) as i32) as f32 / 1000.).sin_cos()
}

/// `filmnoise`, 1470/1520.
#[derive(Clone, Copy, Default)]
struct Grain {
    clock: i32,
    mark: i32,
    state: u32,
    inverted: bool,
    wait: f32,
}

pub(crate) struct FilmNoise {
    active: usize,
    grains: Vec<Grain>,
}

impl FilmNoise {
    pub(crate) fn new(records: &mut [Record], tick: &mut Tick) -> Result<Self> {
        let mut noise = Self { active: 3, grains: vec![Grain::default(); records.len()] };
        for index in 0..records.len() {
            noise.spawn(index, records, tick)?;
        }
        Ok(noise)
    }
    fn spawn(&mut self, index: usize, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        let record = &mut records[index];
        record.pivot = [tick.texture[0] * 0.5, tick.texture[1] * 0.5, 0.];
        let mut grain = Grain::default();
        match index {
            // 1840: the two full-screen frames that alternate.
            0 | 1 => {
                record.uvs = if index == 0 { cell(0., 0., 0.625, 0.3125) } else { cell(0., 0.5, 0.625, 0.3125) };
                record.position = [640., 360., 20.];
                record.scale = [1.25, 0.8, 0.];
                record.alphas = Some([5; 4]);
                record.visible = false;
                grain.inverted = index == 1;
                grain.state = if index == 0 { 1 } else { 0 };
            }
            // 1C20: the scratch.
            2 => {
                record.uvs = cell(0.75, 0.5, 0.25, 0.5);
                record.position = [tick.random.float_value(tick.client)? * 1280., 200., 10.];
                record.scale = [tick.random.float_value(tick.client)? * 0.1 + 0.1, 1., 1.];
                let mut alphas = [0u8; 4];
                for alpha in &mut alphas {
                    *alpha = tick.random.integer_word(tick.client)? as u8;
                }
                record.alphas = Some(alphas);
                grain.wait = (tick.random.integer_word(tick.client)? % 5 + 1) as f32;
                record.visible = true;
            }
            // 1A60: a speck.
            _ => {
                record.uvs = cell(0.75, 0., 0.25, 0.25);
                let x = tick.random.float_value(tick.client)? * 1280.;
                let y = tick.random.float_value(tick.client)? * 720.;
                record.position = [x, y, index as f32 - 2.];
                record.rotation = tick.random.float_value(tick.client)? * std::f32::consts::PI;
                let size = tick.random.float_value(tick.client)? * 0.01 + 0.005;
                record.scale = [size, size, 1.];
                let mut alphas = [0u8; 4];
                for alpha in &mut alphas {
                    *alpha = (tick.random.integer_word(tick.client)? % 0xC8) as u8;
                }
                record.alphas = Some(alphas);
                grain.wait = (tick.random.integer_word(tick.client)? & 7) as f32;
                record.visible = false;
            }
        }
        self.grains[index] = grain;
        Ok(())
    }
}

impl Custom for FilmNoise {
    fn update(&mut self, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        if tick.random.integer_word(tick.client)? % 0x1e == 0 {
            self.active += 1;
        }
        self.active = self.active.min(records.len());
        for index in 0..self.active {
            let grain = self.grains[index];
            let elapsed = grain.clock - grain.mark;
            let mut next = grain;
            let mut respawn = false;
            match (index, grain.state) {
                // 15E0: each frame shows for two ticks, then the other.
                (0 | 1, 0) => {
                    records[index].visible = !grain.inverted;
                    if elapsed > 1 {
                        next.state = 1;
                        next.mark = grain.clock;
                    }
                }
                (0 | 1, 1) => {
                    records[index].visible = grain.inverted;
                    if elapsed > 1 {
                        next.state = 0;
                        next.mark = grain.clock;
                    }
                }
                (0 | 1, _) => respawn = true,
                // 1780 and 16D0: hold, then respawn.
                (_, 0) => {
                    if index > 2 {
                        records[index].visible = true;
                    }
                    if elapsed as f32 > grain.wait {
                        next.state = 1;
                        next.mark = grain.clock;
                    }
                }
                _ => respawn = true,
            }
            self.grains[index] = next;
            if respawn {
                self.spawn(index, records, tick)?;
            }
            self.grains[index].clock = self.grains[index].clock.wrapping_add(tick.dt);
        }
        Ok(())
    }
}

/// `blur`, 1C60/1E30: a stack of strips, each drawn through one matrix.
pub(crate) struct Blur {
    translate: [f32; 2],
    jitter: f32,
    scale: [f32; 2],
    angles: [f32; 3],
    color: [u8; 3],
    spread: [u8; 3],
    jittering: bool,
    half_canvas: [f32; 2],
    locals: Vec<[[f32; 2]; 4]>,
}

type Matrix = [[f32; 4]; 4];
fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [[0.; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            out[row][column] = (0..4).map(|k| a[row][k] * b[k][column]).sum();
        }
    }
    out
}

impl Blur {
    pub(crate) fn new(records: &mut [Record], words: &[f32], tick: &mut Tick) -> Result<Self> {
        let count = records.len().max(1) as f32;
        let width = tick.texture[0];
        let step = width / count;
        let (half, v_step) = (width * 0.5, 1. / count);
        let mut locals = Vec::with_capacity(records.len());
        for (index, record) in records.iter_mut().enumerate() {
            let at = index as f32;
            locals.push([[-half, at * step], [half, at * step], [-half, at * step + step], [half, at * step + step]]);
            record.uvs = Some([[0., at * v_step], [1., at * v_step], [0., at * v_step + v_step], [1., at * v_step + v_step]]);
        }
        let word = |at: usize| words[at];
        Ok(Self {
            translate: [word(1), word(2)],
            jitter: word(3),
            scale: [word(4) / 100., word(5) / 100.],
            angles: [word(6).to_radians(), word(7).to_radians(), word(8).to_radians()],
            color: [word(9) as i32 as u8, word(10) as i32 as u8, word(11) as i32 as u8],
            spread: [word(12) as i32 as u8, word(13) as i32 as u8, word(14) as i32 as u8],
            jittering: false,
            half_canvas: [tick.canvas[0] as f32 * 0.5, tick.canvas[1] as f32 * 0.5],
            locals,
        })
    }
}

impl Custom for Blur {
    fn update(&mut self, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        self.jittering = tick.parameters.first().is_some_and(|word| *word as i32 != 0);
        let offset = [tick.base[0] - self.half_canvas[0], tick.base[1] - self.half_canvas[1]];
        let translate = [self.translate[0] + offset[0], self.translate[1] + offset[1]];
        let (sx, cx) = snapped(self.angles[0]);
        let (sy, cy) = snapped(self.angles[1]);
        let (sz, cz) = snapped(self.angles[2]);
        let mut matrix: Matrix = [[self.scale[0], 0., 0., 0.], [0., self.scale[1], 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
        // 1290 multiplies each new matrix on the left.
        matrix = multiply(&[[1., 0., 0., 0.], [0., cx, sx, 0.], [0., -sx, cx, 0.], [0., 0., 0., 1.]], &matrix);
        matrix = multiply(&[[cy, 0., -sy, 0.], [0., 1., 0., 0.], [sy, 0., cy, 0.], [0., 0., 0., 1.]], &matrix);
        matrix = multiply(&[[cz, sz, 0., 0.], [-sz, cz, 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]], &matrix);
        matrix[3][0] += translate[0];
        matrix[3][1] += translate[1];
        for (index, record) in records.iter_mut().enumerate() {
            let mut corners = [[0.; 2]; 4];
            let mut depths = [0.; 4];
            for vertex in 0..4 {
                let [mut x, y] = self.locals[index][vertex];
                if self.jittering {
                    let wide = tick.random.integer_word(tick.client)? & 0x7f == 0;
                    let amount = if wide { self.jitter * 50. } else { self.jitter };
                    x += (tick.random.float_value(tick.client)? * 2. - 1.) * amount;
                }
                corners[vertex] = [matrix[1][0] * y + matrix[0][0] * x + matrix[3][0], matrix[1][1] * y + matrix[0][1] * x + matrix[3][1]];
                depths[vertex] = matrix[1][2] * y + matrix[0][2] * x + matrix[3][2];
            }
            // Each channel is redrawn per frame, clamped between its base and white.
            let mut color = [0u8; 3];
            for channel in 0..3 {
                let base = f32::from(self.color[channel]);
                let drawn = tick.random.float_value(tick.client)? * f32::from(self.spread[channel]) + base;
                color[channel] = drawn.clamp(base, 255.) as u8;
            }
            record.corners = Some(corners);
            record.depths = Some(depths);
            record.color = Some(color);
            record.alpha = 255;
        }
        Ok(())
    }
}

/// `steam`, 21E0/26E0: puffs rising along short random Hermite paths.
#[derive(Clone, Default)]
struct Puff {
    clock: i32,
    mark: i32,
    state: u32,
    opacity: f32,
    path: Path,
}

pub(crate) struct Steam {
    length: (f32, f32),
    angle: (f32, f32),
    size: (f32, f32),
    opacity: (f32, f32),
    fade_in: i32,
    hold: i32,
    fade_out: i32,
    period: u32,
    release: usize,
    active: usize,
    source: Path,
    direction: [f32; 3],
    puffs: Vec<Puff>,
}

/// 0x1001A0B0.
const STEAM_CELLS: [[f32; 2]; 4] = [[0., 0.], [0.5, 0.], [0., 0.5], [0.5, 0.5]];

impl Steam {
    pub(crate) fn new(records: &mut [Record], words: &[f32], tick: &mut Tick) -> Result<Self> {
        let word = |at: usize| words[at];
        let angle = |value: f32| (value / 100.).clamp(-std::f32::consts::PI, std::f32::consts::PI);
        let (first, second) = ([word(11), word(12), 0.], [word(13), word(14), 0.]);
        let mut source = Path::default();
        source.add(first);
        source.add(second);
        source.build(SOURCE_DURATION, SOURCE_MAGNITUDE);
        // The chord, turned a quarter (1.5707964 at 0x100154F4), then normalized.
        let (dx, dy) = (second[0] - first[0], second[1] - first[1]);
        let (sin, cos) = snapped(std::f32::consts::FRAC_PI_2);
        let turned = [cos * dx - sin * dy, sin * dx + cos * dy, 0.];
        let length = (turned[0] * turned[0] + turned[1] * turned[1]).sqrt();
        let direction = if length > 0. { [turned[0] / length, turned[1] / length, 0.] } else { turned };
        let warm = word(17) as i32 != 0;
        let mut steam = Self {
            length: (word(0).max(10.), word(1).max(0.)),
            angle: (angle(word(2)), angle(word(3))),
            size: ((word(4) / 100.).max(0.01), (word(5) / 100.).max(0.)),
            opacity: (word(6).max(0.), word(7).max(0.)),
            fade_in: (word(8) as i32).max(0),
            hold: (word(9) as i32).max(0),
            fade_out: (word(10) as i32).max(0),
            period: (61 - word(15) as i32).clamp(1, 60) as u32,
            release: (word(16) as i32).max(1) as usize,
            active: if warm { records.len() / 2 } else { 1 },
            source,
            direction,
            puffs: vec![Puff::default(); records.len()],
        };
        for index in 0..records.len() {
            steam.spawn(index, records, tick)?;
            // 3320: the warm half starts part way through its life.
            if warm && index < steam.active {
                let total = (steam.fade_in + steam.hold + steam.fade_out) as f32;
                let clock = (tick.random.float_value(tick.client)? * total) as i32;
                let puff = &mut steam.puffs[index];
                puff.clock = clock;
                if clock > steam.fade_in {
                    puff.state += 1;
                    puff.mark += steam.fade_in;
                    records[index].alpha = 0x80;
                }
                if clock > steam.hold {
                    puff.state += 1;
                    puff.mark += steam.hold;
                }
            }
        }
        Ok(steam)
    }

    /// 2C20.
    fn spawn(&mut self, index: usize, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        let [u, v] = STEAM_CELLS[(tick.random.integer_word(tick.client)? & 3) as usize];
        let record = &mut records[index];
        record.pivot = [tick.texture[0] * 0.5, tick.texture[1] * 0.5, 0.];
        record.uvs = cell(u, v, 0.5, 0.5);
        let size = tick.random.float_value(tick.client)? * self.size.1 - self.size.1 * 0.5 + self.size.0;
        record.scale = [size, size, 1.];
        record.alpha = 0;
        let start = self.source.at(tick.random.float_value(tick.client)? * SOURCE_DURATION);
        record.position = start;
        let mut path = Path::default();
        path.add(start);
        let mut at = start;
        let step = |at: &mut [f32; 3], tick: &mut Tick, this: &Self| -> Result<()> {
            let angle = tick.random.float_value(tick.client)? * this.angle.1 - this.angle.1 * 0.5 + this.angle.0;
            let length = tick.random.float_value(tick.client)? * this.length.1 + this.length.1 * 0.5 + this.length.0;
            let (sin, cos) = snapped(angle);
            let [dx, dy, _] = this.direction;
            at[0] += (cos * dx - sin * dy) * length;
            at[1] += (sin * dx + cos * dy) * length;
            Ok(())
        };
        step(&mut at, tick, self)?;
        path.add(at);
        for _ in 0..3 {
            if tick.random.integer_word(tick.client)? % 5 != 0 {
                step(&mut at, tick, self)?;
                path.add(at);
            }
        }
        let total = (self.fade_in + self.hold + self.fade_out) as f32;
        path.build(total, total * 0.4);
        let opacity = tick.random.float_value(tick.client)? * self.opacity.1 - self.opacity.1 * 0.5 + self.opacity.0;
        self.puffs[index] = Puff { clock: 0, mark: 0, state: 0, opacity, path };
        Ok(())
    }
}

impl Custom for Steam {
    fn update(&mut self, records: &mut [Record], tick: &mut Tick) -> Result<()> {
        if tick.random.integer_word(tick.client)? % self.period == 0 {
            self.active += self.release;
        }
        self.active = self.active.min(records.len());
        for index in 0..self.active {
            let puff = &mut self.puffs[index];
            let clock = puff.clock;
            let elapsed = clock - puff.mark;
            let mut respawn = false;
            let alpha = match puff.state {
                0 => {
                    if clock >= self.fade_in {
                        puff.state = 1;
                        puff.mark = clock;
                    }
                    eased(0., puff.opacity, self.fade_in as f32, clock as f32)
                }
                1 => {
                    if elapsed >= self.hold {
                        puff.state = 2;
                        puff.mark = clock;
                    }
                    puff.opacity
                }
                2 => {
                    if elapsed > self.fade_out {
                        puff.state = 3;
                        puff.mark = clock;
                    }
                    eased(puff.opacity, 0., self.fade_out as f32, elapsed as f32)
                }
                _ => {
                    respawn = true;
                    0.
                }
            };
            if respawn {
                self.spawn(index, records, tick)?;
            } else {
                let position = self.puffs[index].path.at(clock as f32);
                records[index].alpha = alpha as i32 as u8;
                records[index].position = position;
            }
            let puff = &mut self.puffs[index];
            puff.clock = puff.clock.wrapping_add(tick.dt);
        }
        Ok(())
    }
}
