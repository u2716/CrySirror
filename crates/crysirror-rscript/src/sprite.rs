// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native sprite state: position,
//! blend mode and level, and the per-tick fade and point-motion
//! components with their truncating integer math. Sprites hold
//! asset references only; the shared scene renderer draws them.
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// CMath sine/cosine tables, 256 steps scaled by 65535.
static SIN: LazyLock<[i64; 256]> =
    LazyLock::new(|| std::array::from_fn(|i| ((i as f64 * 0.00390625 * 6.283185308).sin() * 65535.0) as i64));
static COS: LazyLock<[i64; 256]> =
    LazyLock::new(|| std::array::from_fn(|i| ((i as f64 * 0.00390625 * 6.283185308).cos() * 65535.0) as i64));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
pub const fn pt(x: i32, y: i32) -> Point {
    Point { x, y }
}

/// Image content of a sprite.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    /// A decoded image asset of known size.
    Image { asset: String, size: [u32; 2] },
    /// A full-screen colour fill, 0xRRGGBB.
    Fill { color: u32, size: [u32; 2] },
    /// A text layer (layer source 2): Shift-JIS text in fixed cells.
    Text { text: Vec<u8>, cell: i32, shadow: bool, size: [u32; 2] },
}
impl Content {
    pub fn size(&self) -> [u32; 2] {
        match self {
            Content::Image { size, .. } | Content::Fill { size, .. } | Content::Text { size, .. } => *size,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fade {
    pub active: bool,
    /// 0 fades in (level falls to 0), 1 fades out (rises to 255, then hides).
    pub direction: u8,
    pub looping: bool,
    pub step: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub active: bool,
    kind: u8,
    target: Point,
    delta: Point,
    offset: Point,
    angle_step: i64,
    remaining: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    pub content: Option<Content>,
    pub pos: Point,
    pub visible: bool,
    /// Native blend mode (vtable +176) and level (+180).
    pub mode: u8,
    pub level: u8,
    /// Colour of the additive glow mode 0x6B.
    pub color: u32,
    pub fade: Fade,
    pub motion: Motion,
    /// Per-tick animation enabled.
    pub animating: bool,
    pub interactive: bool,
    pub hovered: bool,
    /// Frame strips (CSelBtnEx, FSC layers): the shown part of the content
    /// as [x, y, w, h]; `pos` places that part.
    pub crop: Option<[i32; 4]>,
}

impl Sprite {
    pub fn width(&self) -> i32 {
        self.crop.map_or_else(|| self.content.as_ref().map_or(0, |c| c.size()[0] as i32), |c| c[2])
    }
    pub fn height(&self) -> i32 {
        self.crop.map_or_else(|| self.content.as_ref().map_or(0, |c| c.size()[1] as i32), |c| c[3])
    }
    /// Frame `index` of `count` frames stacked vertically.
    pub fn set_frame(&mut self, index: u32, count: u32) {
        let Some(c) = &self.content else { return };
        let [w, h] = c.size();
        let fh = (h / count.max(1)) as i32;
        self.crop = Some([0, fh * index.min(count.max(1) - 1) as i32, w as i32, fh]);
    }
    pub fn set_content(&mut self, content: Option<Content>) {
        if content.is_none() {
            self.visible = false;
        }
        self.content = content;
    }
    pub fn show(&mut self, visible: bool) {
        self.visible = visible && self.content.is_some();
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.visible && x >= self.pos.x && y >= self.pos.y && x < self.pos.x + self.width() && y < self.pos.y + self.height()
    }

    /// Start a fade in `mode` (in: level 255→0, out: 0→255).
    pub fn start_fade(&mut self, mode: u8, direction: u8, step: i32) {
        self.fade = Fade { active: true, direction, looping: false, step };
        self.mode = mode;
        self.level = if direction != 0 { 0 } else { 255 };
    }
    fn fade_tick(&mut self) -> bool {
        if !self.fade.active {
            return false;
        }
        let mut running = true;
        let level = if self.fade.direction == 0 {
            let mut l = i32::from(self.level) - self.fade.step;
            if l <= 0 {
                l = 0;
                if self.fade.looping {
                    self.fade.direction = 1;
                } else {
                    self.mode = 0;
                    self.fade.active = false;
                    running = false;
                }
            }
            l
        } else {
            let mut l = i32::from(self.level) + self.fade.step;
            if l > 255 {
                l = 255;
                if self.fade.looping {
                    self.fade.direction = 0;
                } else {
                    self.mode = 0;
                    self.visible = false;
                    self.fade.active = false;
                    running = false;
                }
            }
            l
        };
        self.level = level as u8;
        running
    }
    fn fade_finish(&mut self) {
        if !self.fade.active {
            return;
        }
        self.fade.active = false;
        if self.fade.direction != 0 {
            self.level = 255;
            self.visible = false;
        } else {
            self.level = 0;
        }
        self.mode = 0;
    }

    fn begin(&mut self, kind: u8, from: Point, to: Point) -> i64 {
        let m = &mut self.motion;
        m.kind = kind;
        m.target = to;
        m.delta = pt(to.x - from.x, to.y - from.y);
        (f64::from(m.delta.x).powi(2) + f64::from(m.delta.y).powi(2)).sqrt() as i64
    }
    fn scale_delta(&mut self) {
        let m = &mut self.motion;
        m.delta = pt(
            (i64::from(m.delta.x) * 0xFFFF / m.remaining) as i32,
            (i64::from(m.delta.y) * 0xFFFF / m.remaining) as i32,
        );
    }
    /// Straight line at `speed` pixels per tick.
    pub fn linear(&mut self, from: Point, to: Point, speed: i32) {
        if speed == 0 {
            return;
        }
        self.motion.remaining = (self.begin(0, from, to) / i64::from(speed)) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.scale_delta();
        self.motion.active = true;
    }
    /// Line with a sine bulge of `bulge/20` of the distance.
    pub fn arc(&mut self, from: Point, to: Point, speed: i32, waves: i64, bulge: i32) {
        if speed == 0 {
            return;
        }
        let distance = self.begin(1, from, to);
        let d = self.motion.delta;
        self.motion.offset = pt(d.y * bulge / 20, -d.x * bulge / 20);
        self.motion.remaining = (distance / i64::from(speed)) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.scale_delta();
        self.motion.angle_step = (0xFFFF * waves / self.motion.remaining) & 0xFFFF;
        self.motion.active = true;
    }
    /// Half-circle arc to either side.
    pub fn swing(&mut self, from: Point, to: Point, speed: i32, clockwise: bool) {
        if speed == 0 {
            return;
        }
        let distance = self.begin(1, from, to);
        let s = if clockwise { 1 } else { -1 };
        let d = self.motion.delta;
        self.motion.offset = pt(s * d.y / 2, -s * d.x / 2);
        self.motion.remaining = (distance / i64::from(speed)) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.scale_delta();
        self.motion.angle_step = (0x8000 / self.motion.remaining) & 0xFFFF;
        self.motion.active = true;
    }
    /// Sine (decelerating) or cosine (accelerating) curve.
    pub fn curved(&mut self, from: Point, to: Point, speed: i32, accelerate: bool) {
        if speed == 0 {
            return;
        }
        let distance = self.begin(if accelerate { 4 } else { 3 }, from, to);
        let d = self.motion.delta;
        self.motion.offset = pt(-d.x, -d.y);
        self.motion.remaining = (distance / i64::from(speed)) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.motion.angle_step = (0x4000 / self.motion.remaining) & 0xFFFF;
        self.motion.active = true;
    }
    /// Oscillation around the current point.
    pub fn bounce(&mut self, at: Point, count: i32, amplitude: i32, speed: i32, direction: i32) {
        if speed == 0 {
            return;
        }
        self.begin(5, at, at);
        self.motion.offset = match direction {
            1 => pt(-amplitude, 0),
            2 => pt(0, amplitude),
            3 => pt(amplitude, 0),
            _ => pt(0, -amplitude),
        };
        self.motion.remaining = i64::from(amplitude * count / speed) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.motion.angle_step = ((i64::from(count) << 15) / self.motion.remaining) & 0xFFFF;
        self.motion.active = true;
    }
    /// A single vertical hop.
    pub fn hop(&mut self, at: Point, height: i32, speed: i32) {
        if speed == 0 {
            return;
        }
        self.begin(6, at, at);
        self.motion.offset = pt(0, -height);
        self.motion.remaining = i64::from(height / speed) & 0xFFFF;
        if self.motion.remaining == 0 {
            return;
        }
        self.motion.angle_step = (34816 / self.motion.remaining + 2) & 0xFFFF;
        self.motion.active = true;
    }
    fn motion_tick(&mut self) -> bool {
        let m = &mut self.motion;
        if !m.active {
            return false;
        }
        if m.remaining == 0 {
            self.pos = m.target;
            m.active = false;
            return false;
        }
        let angle = (m.remaining * m.angle_step / 255) as usize;
        let scaled = |o: Point, f: i64| pt((i64::from(o.x) * f / 0xFFFF) as i32, (i64::from(o.y) * f / 0xFFFF) as i32);
        let t = m.target;
        self.pos = match m.kind {
            1 => {
                let line = scaled(m.delta, m.remaining);
                let bulge = scaled(m.offset, SIN[angle % 255]);
                pt(t.x - line.x + bulge.x, t.y - line.y + bulge.y)
            }
            3 | 5 => {
                let s = scaled(m.offset, SIN[angle % 255]);
                pt(t.x + s.x, t.y + s.y)
            }
            4 => {
                let s = scaled(m.offset, COS[angle % 255]);
                pt(t.x + m.offset.x - s.x, t.y + m.offset.y - s.y)
            }
            6 => {
                let s = scaled(m.offset, SIN[(390 - angle as i64).rem_euclid(255) as usize]);
                pt(t.x + s.x, t.y + s.y)
            }
            _ => {
                let line = scaled(m.delta, m.remaining);
                pt(t.x - line.x, t.y - line.y)
            }
        };
        m.remaining -= 1;
        true
    }
    pub fn motion_finish(&mut self) {
        if self.motion.active {
            self.motion.active = false;
            self.pos = self.motion.target;
        }
    }

    /// One tick; returns whether the sprite still animates.
    pub fn tick(&mut self) -> bool {
        if !self.visible || !self.animating {
            return false;
        }
        let mut running = self.fade_tick();
        running |= self.motion_tick();
        if !running {
            self.animating = false;
        }
        running
    }
    /// vtable +96: jump to the end state.
    pub fn stop_animation(&mut self) {
        self.fade_finish();
        self.motion_finish();
        self.animating = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fade_in_and_out_follow_native_steps() {
        let mut s = Sprite { content: Some(Content::Fill { color: 0, size: [4, 4] }), visible: true, ..Default::default() };
        s.start_fade(2, 0, 32);
        s.animating = true;
        let mut ticks = 0;
        while s.tick() {
            ticks += 1;
        }
        assert_eq!((ticks, s.level, s.mode, s.visible), (7, 0, 0, true));
        s.start_fade(2, 1, 128);
        s.animating = true;
        while s.tick() {}
        assert!(!s.visible);
    }
    #[test]
    fn linear_motion_reaches_the_target() {
        let mut s = Sprite { content: Some(Content::Fill { color: 0, size: [4, 4] }), visible: true, ..Default::default() };
        s.linear(pt(0, 0), pt(100, 0), 10);
        s.animating = true;
        let mut path = Vec::new();
        while s.tick() {
            path.push(s.pos.x);
        }
        assert_eq!(path.first(), Some(&0));
        assert_eq!(s.pos, pt(100, 0));
    }
}
