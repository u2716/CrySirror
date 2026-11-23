// SPDX-License-Identifier: MPL-2.0
//! CVNS per-sprite transitions (`0x218d`): setup `0x4176e0` and the per-frame
//! cell update `0x418910`. While a transition runs, the sprite is drawn as
//! its cells (80-byte records at +40) instead of itself.
//!
//! Types below 0x100 reveal the sprite; the same family with 0x100 set hides
//! it, after which `0x41b5a0` deletes the sprite.

use anyhow::{bail, Result};

/// A drawn piece: source rectangle (left, top, right, bottom), screen
/// position and opacity.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Cell {
    pub crop: [i32; 4],
    pub dest: [i32; 2],
    pub alpha: i32,
    /// +40: the per-row start delay of types 24/25.
    delay: i32,
}

/// The sprite fields `0x418910` reads each frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Base {
    pub crop: [i32; 4],
    /// Position minus origin.
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub alpha: i32,
}

#[derive(Clone, Debug)]
pub(super) struct Effect {
    /// this[0].
    pub kind: u32,
    /// this[1]: the sprite is deleted at the end.
    pub hides: bool,
    columns: i32,
    rows: i32,
    duration: i32,
    steps: i32,
    size: i32,
    stagger: i32,
    speed: i32,
    elapsed: i32,
    /// 306: the cells already removed.
    removed: Vec<bool>,
    pub cells: Vec<Cell>,
    /// 26: the bitmap faded from.
    pub from: Option<i16>,
}

fn ceil_div(value: i32, divisor: i32) -> i32 {
    value / divisor + i32::from(value % divisor != 0)
}

/// `0x439a90`: truncated `cos` and `sin` of whole degrees.
fn polar(degrees: i32, radius: i32) -> [i32; 2] {
    let angle = if degrees != 0 { f64::from(degrees) * 0.005555555555555556 * std::f64::consts::PI } else { 0.0 };
    [(angle.cos() * f64::from(radius)) as i64 as i32, (angle.sin() * f64::from(radius)) as i64 as i32]
}

impl Effect {
    pub(super) fn record(&self) -> [u32;10] {
        let args=if self.kind==1 || self.kind==257 { [0;3] } else { [self.size,self.stagger,self.speed] };
        [self.kind,self.hides as u32,self.columns as u32,self.rows as u32,self.duration as u32,
         self.steps as u32,args[0] as u32,args[1] as u32,args[2] as u32,self.elapsed as u32]
    }

    /// `0x4176e0` with `a4` = kind, `a5` = duration and `a6`..`a8` as given.
    pub fn new(kind: u32, duration: u32, a6: i32, a7: i32, a8: i32, base: Base, random: &mut impl FnMut() -> i32) -> Result<Self> {
        let [w, h] = base.size;
        let low = kind & 0xff;
        let hides = match kind {
            1 | 2..=19 | 24 | 25 | 26 | 764 => false,
            _ => true,
        };
        let need = |value: i32| -> Result<i32> {
            if value == 0 {
                bail!("CVNS transition {kind:#x} has a zero divisor");
            }
            Ok(value)
        };
        let (columns, rows, steps) = match low {
            1 if kind == 1 || kind == 257 => (1, 1, 256),
            2 | 4 | 6 | 7 => {
                let rows = ceil_div(h, need(a6)?);
                (1, rows, a6 / need(a8)? + (rows - 1) / need(a7)?)
            }
            3 | 5 | 8 | 9 => {
                let columns = ceil_div(w, need(a6)?);
                (columns, 1, a6 / need(a8)? + (columns - 1) / need(a7)?)
            }
            10 | 11 => {
                let columns = ceil_div(w, need(a6)?);
                (columns, 1, 256 / need(a8)? + columns - 1)
            }
            12 | 13 => {
                let rows = ceil_div(h, need(a6)?);
                (1, rows, 256 / need(a8)? + rows - 1)
            }
            14..=17 => {
                let columns = ceil_div(w, need(a6)?);
                let rows = ceil_div(h, a6);
                (columns, rows, rows + 256 / need(a8)? + columns - 2)
            }
            18 | 19 | 24 | 25 => (1, h, a7 * (w / 2)),
            26 if kind == 26 => (1, 1, 256 / need(a6)? + 1),
            50 if kind == 306 => (a6, a6, a6 * a6),
            252 if kind == 764 => (1, h, 0),
            _ => bail!("CVNS sprite transition type {kind:#x} is not defined"),
        };
        let mut effect = Self {
            kind,
            hides,
            columns,
            rows,
            duration: duration as i32,
            steps,
            size: a6,
            stagger: a7,
            speed: a8,
            elapsed: 0,
            removed: vec![false; if kind == 306 { (a6 * a6) as usize } else { 0 }],
            cells: vec![Cell { alpha: base.alpha, ..Cell::default() }; (columns * rows).max(0) as usize],
            from: (kind == 26).then_some(a8 as i16),
        };
        // Initial cell opacity and the 24/25 row delays.
        for (index, cell) in effect.cells.iter_mut().enumerate() {
            cell.alpha = match low {
                1 if kind == 1 => 0,
                1 => 256,
                10..=17 if kind < 0x100 => 0,
                10..=17 => 256,
                18 if kind == 18 => base.alpha,
                18 | 19 => 0,
                26 => 0,
                _ => base.alpha,
            };
            if matches!(low, 24 | 25) {
                cell.delay = a7 / need(a6)? * (random() % a6);
            }
            if kind == 764 {
                let row = index as i32;
                cell.crop = [base.crop[0], base.crop[1] + row, base.crop[2], base.crop[1] + row + 1];
                cell.dest = [base.at[0], base.at[1] + row];
            }
        }
        Ok(effect)
    }

fn clipped(size: i32, start: i32, index: i32, end: i32) -> i32 {
        // The last strip ends at the sprite edge.
        if start + size * (index + 1) > end {
            size + end - size * (index + 1) - start
        } else {
            size
        }
    }

    /// `0x418910`; true when the transition has ended.
    pub fn advance(&mut self, delta: u32, base: Base, random: &mut impl FnMut() -> i32) -> Result<bool> {
        self.elapsed = self.elapsed.wrapping_add(delta as i32);
        let t = self.elapsed;
        let [left, top, right, bottom] = base.crop;
        let [x, y] = base.at;
        let size = self.size;
        let mut done = false;
        let low = self.kind & 0xff;
        let kind = self.kind;
        match kind {
            1 | 257 | 26 => {
                let alpha = if t < self.duration {
                    if kind == 257 { 256 - (t << 8) / self.duration } else { (t << 8) / self.duration }
                } else {
                    done = true;
                    if kind == 257 { 0 } else { 256 }
                };
                let cell = &mut self.cells[0];
                cell.alpha = alpha.clamp(0, 256);
                cell.crop = base.crop;
                cell.dest = [x, y];
            }
            764 => {
                if t < self.duration {
                    return Ok(false);
                }
                self.steps = (self.steps + self.stagger).rem_euclid(720);
                for row in 0..self.rows {
                    let [dx, _] = polar((self.steps + row * (360 / size)) % 360, self.speed);
                    let cell = &mut self.cells[row as usize];
                    cell.crop = [left, top + row, right, top + row + 1];
                    cell.dest = [dx + x, y + row];
                    cell.alpha = base.alpha;
                }
                self.elapsed = 0;
            }
            306 => {
                if t >= self.duration {
                    return Ok(true);
                }
                let total = size * size;
                let mut removed = self.removed.iter().filter(|cell| **cell).count() as i32;
                let target = t * total / self.duration;
                while removed < target {
                    let pick = (random() % total) as usize;
                    if !self.removed[pick] {
                        self.removed[pick] = true;
                        removed += 1;
                    }
                }
                let [cw, ch] = [base.size[0] / size, base.size[1] / size];
                for index in 0..total {
                    let [cx, cy] = [cw * (index % size), ch * (index / size)];
                    let cell = &mut self.cells[index as usize];
                    cell.dest = [cx, cy];
                    cell.crop = if self.removed[index as usize] { [cx, cy, cx, cy] } else { [cx, cy, cx + cw, cy + ch] };
                }
            }
            _ => {
                for row in 0..self.rows {
                    for column in 0..self.columns {
                        let index = (column + row * self.columns) as usize;
                        if t >= self.duration {
                            done = true;
                            continue;
                        }
                        let step = t * self.steps / self.duration;
                        let cell = &mut self.cells[index];
                        match low {
                            2 | 4 | 6 | 7 | 3 | 5 | 8 | 9 => {
                                let along_rows = matches!(low, 2 | 4 | 6 | 7);
                                let (index, count) = if along_rows { (row, self.rows) } else { (column, self.columns) };
                                let forward = matches!(low, 2 | 3 | 6 | 8);
                                let order = if forward { index } else { count - index - 1 };
                                let p = (self.speed * (step - order / self.stagger)).clamp(0, size);
                                let near = index * size;
                                let far = (index + 1) * size;
                                // Which edge of each strip moves, by family and direction.
                                let (from, to, at) = match (low, self.hides) {
                                    (2, false) | (7, false) | (3, false) | (9, false) => (near, near + p, near),
                                    (4, false) | (6, false) | (5, false) | (8, false) => (far - p, far, far - p),
                                    (2, true) | (7, true) | (3, true) | (9, true) => (near + p, far, near + p),
                                    _ => (near, far - p, near),
                                };
                                if along_rows {
                                    cell.crop = [left, top + from, right, top + to];
                                    cell.dest = [x, y + at];
                                } else {
                                    cell.crop = [left + from, top, left + to, bottom];
                                    cell.dest = [x + at, y];
                                }
                            }
                            10..=17 => {
                                let s = match low {
                                    10 => step - column,
                                    11 => step - self.columns + column + 1,
                                    12 => step - row,
                                    13 => step - self.rows + row + 1,
                                    14 => step - column - row,
                                    15 => column + step - self.rows - self.columns + row + 2,
                                    16 => step - self.rows - column + row + 1,
                                    _ => step - self.columns - row + column + 1,
                                };
                                let mut alpha = (s.max(0) * self.speed).min(256);
                                if kind >= 0x100 {
                                    alpha = 256 - alpha;
                                }
                                cell.alpha = alpha;
                                let (cx, cw) = if matches!(low, 12 | 13) {
                                    (left, right - left)
                                } else {
                                    (left + column * size, Self::clipped(size, left, column, right))
                                };
                                let (cy, chh) = if matches!(low, 10 | 11) {
                                    (top, bottom - top)
                                } else {
                                    (top + row * size, Self::clipped(size, top, row, bottom))
                                };
                                cell.crop = [cx, cy, cx + cw, cy + chh];
                                cell.dest = [x + cx - left, y + cy - top];
                            }
                            18 | 19 => {
                                let radius = if kind >= 0x100 { t * size / self.duration } else { size - t * size / self.duration };
                                let [dx, _] = polar(self.stagger * (row + 360 * t / self.duration) % 360, radius);
                                match kind {
                                    275 => cell.alpha = 256 - (t << 8) / self.duration,
                                    19 => cell.alpha = (t << 8) / self.duration,
                                    _ => {}
                                }
                                cell.crop = [left, top + row, right, top + row + 1];
                                cell.dest = [x + dx * self.speed, y + row];
                            }
                            24 | 25 => {
                                let span = self.duration - self.stagger;
                                let since = t - cell.delay;
                                // Only 24 shrinks its offset; 25, 280 and 281 grow it.
                                let grows = kind != 24;
                                let value = if !grows {
                                    if t < cell.delay {
                                        self.speed
                                    } else if since < span {
                                        self.speed - self.speed * since / span
                                    } else {
                                        0
                                    }
                                } else if t < cell.delay {
                                    0
                                } else if since < span {
                                    since * self.speed / span
                                } else {
                                    self.speed
                                };
                                let line = top + row;
                                let (crop, dest) = match kind {
                                    24 => ([value, line, self.speed, line + 1], [value, row]),
                                    280 => ([0, line, self.speed - value, line + 1], [0, row]),
                                    25 => ([0, line, value, line + 1], [0, row]),
                                    _ => ([value, line, self.speed, line + 1], [value, row]),
                                };
                                cell.crop = crop;
                                cell.dest = dest;
                            }
                            _ => bail!("CVNS sprite transition type {kind:#x} update"),
                        }
                    }
                }
                // Rows of types 18/19 still finish with the full row in place.
                if done && matches!(low, 18 | 19) {
                    for cell in &mut self.cells {
                        cell.dest[0] = x;
                        if matches!(kind, 19 | 275) {
                            cell.alpha = 256;
                        }
                    }
                }
            }
        }
        Ok(done)
    }
}
