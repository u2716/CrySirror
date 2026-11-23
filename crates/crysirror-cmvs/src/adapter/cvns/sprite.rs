// SPDX-License-Identifier: MPL-2.0
//! CVNS sprite records (`0x41adf0`, 0x53c bytes) and their keyframe animator.

use anyhow::{bail, ensure, Result};
use crysirror_core::service::UploadedImage;
use serde_json::json;

mod checkpoint;

pub(super) const SPRITES: u16 = 1024;
pub(super) const PARTS: u8 = 20;
const FRAMES: usize = 64;
const SLOTS: usize = 6;
const SPRITE_LAYER: u32 = 0x4356_0000;

/// A sprite record. `0x40fa20` buckets global sprites, then each ADV window's
/// parts, then message glyphs; each insertion goes to the bucket front.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SpriteId {
    Global(u16),
    /// Part `1` of a window is its click-wait icon.
    Part(u8, u8),
}

impl SpriteId {
    /// Insertion position in the `0x40fa20` bucket pass.
    pub fn sequence(self) -> i32 {
        match self {
            Self::Global(index) => i32::from(index),
            Self::Part(window, part) => 1024 + 20 * i32::from(window) + i32::from(part),
        }
    }

    fn layer_id(self) -> u32 {
        SPRITE_LAYER + self.sequence() as u32
    }
}

/// Draw order for one bucket entry: bucket 127 first, then later insertions first.
pub(super) fn order(layer: u32, sequence: i32) -> [i32; 2] {
    [127 - layer as i32, 10_000 - sequence]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Type 1: a global bitmap.
    Bitmap,
    /// Type 9: the published picture of a scene movie slot.
    Movie,
    /// Type 6 (`0x41aed0`): a rectangle filled with the colour at +16.
    Fill,
    /// Type 30 (`0x2138`): a bitmap of the owning ADV window (+24).
    Part(u8),
}

/// An 18-byte frame at +184: bitmap, crop, origin, duration and next frame.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Frame {
    pub bitmap: i16,
    pub crop: [i16; 4],
    pub origin: [i16; 2],
    pub duration: i16,
    /// `>= 0` continues there; -1 deletes, -2 stops, -3 stops and disables,
    /// -11..-13 restart at slots 1..3.
    pub next: i16,
}

#[derive(Clone, Debug)]
pub(super) struct Sprite {
    pub kind: Kind,
    pub(super) last_bounds:Option<[i32;4]>,
    /// +42; negative draws nothing.
    pub source: i16,
    /// +44..+50 as left, top, right, bottom.
    pub crop: [i16; 4],
    /// +88, +92.
    pub position: [i32; 2],
    /// +96, +100. Animated frames store their signed width and height here
    /// even when the 16-bit crop endpoints wrap.
    pub size: [i32; 2],
    /// +104, +108.
    pub origin: [i32; 2],
    /// +112, +116 in thousandths.
    pub scale: [i32; 2],
    /// +120 in degrees.
    pub rotation: i32,
    /// +28, 0..256.
    pub alpha: u16,
    /// +20; nonzero skips the blit in `0x410700`.
    pub hidden: i16,
    /// +124; `0x41b0f0` ignores values of 128 and above.
    pub layer: u32,
    /// +80; disabled sprites are not bucketed.
    pub enabled: bool,
    /// +84; `0x40fb60` deletes the record after the frame.
    pub dead: bool,
    pub fill: u32,
    /// +24 tint colour and +22 image weight (256 keeps the image).
    pub tint: (u32, i16),
    /// Sprite field +56 equals -200 for a rule blit.
    pub rule_blit: bool,
    frames: Box<[Frame; FRAMES]>,
    /// +172: first frame of each animation slot, -1 when unset.
    starts: [i16; SLOTS],
    /// +160, +164, +166, +168.
    running: bool,
    slot: i16,
    current: i16,
    remaining: i32,
    /// +1336: the running `0x218d` transition.
    pub effect: Option<super::fx::Effect>,
}

impl Sprite {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            last_bounds:None,
            source: 0,
            crop: [0; 4],
            position: [0; 2],
            size: [0; 2],
            origin: [0; 2],
            scale: [1000; 2],
            rotation: 0,
            alpha: 256,
            hidden: 0,
            layer: 1,
            enabled: true,
            dead: false,
            fill: 0,
            tint: (if let Kind::Part(owner) = kind {u32::from(owner)} else {0}, 256),
            rule_blit: false,
            frames: Box::new([Frame::default(); FRAMES]),
            starts: [-1; SLOTS],
            running: false,
            slot: 0,
            current: 0,
            remaining: 0,
            effect: None,
        }
    }

    /// `0x41a640` and `0x41af90`: source bitmap and crop rectangle.
    pub fn set_source(&mut self, bitmap: i16, [x, y, w, h]: [i16; 4]) {
        self.source = bitmap;
        self.crop = [x, y, x.wrapping_add(w), y.wrapping_add(h)];
        self.measure_crop();
    }

    pub fn measure_crop(&mut self) {
        let [left, top, right, bottom] = self.crop.map(i32::from);
        self.size = [right - left, bottom - top];
    }

    /// `0x41a690`: frame `index`, optionally the start of animation `slot`.
    pub fn set_frame(&mut self, index: u32, frame: Frame, slot: Option<u32>) -> Result<()> {
        ensure!((index as usize) < FRAMES, "CVNS animation frame {index} outside 0..63");
        if let Some(slot) = slot {
            ensure!((slot as usize) < SLOTS, "CVNS animation slot {slot} outside 0..5");
            self.starts[slot as usize] = index as i16;
        }
        self.frames[index as usize] = frame;
        Ok(())
    }

    fn apply(&mut self, index: i16) -> Result<()> {
        let frame = *self
            .frames
            .get(index as usize)
            .ok_or_else(|| anyhow::anyhow!("CVNS animation frame {index} outside 0..63"))?;
        self.current = index;
        self.set_source(frame.bitmap, frame.crop);
        self.size = [i32::from(frame.crop[2]), i32::from(frame.crop[3])];
        self.origin = frame.origin.map(i32::from);
        Ok(())
    }

    /// `0x41b390`.
    pub fn start(&mut self, slot: u32) -> Result<()> {
        let start = *self
            .starts
            .get(slot as usize)
            .ok_or_else(|| anyhow::anyhow!("CVNS animation slot {slot} outside 0..5"))?;
        self.slot = slot as i16;
        self.running = true;
        self.apply(start)?;
        self.remaining = i32::from(self.frames[start as usize].duration);
        Ok(())
    }

    /// `0x41b4d0` returns to the first frame of slot 0.
    pub fn stop(&mut self) -> Result<()> {
        if self.running {
            self.running = false;
            if self.starts[0] > -1 {
                self.apply(self.starts[0])?;
                self.measure_crop();
            }
        }
        Ok(())
    }

    /// `0x41a720`.
    pub fn animation(&self) -> i32 {
        if self.running {
            i32::from(self.slot)
        } else {
            -1
        }
    }

    /// The timed part of `0x41b5a0`; a due frame does not carry the overshoot.
    pub fn advance(&mut self, delta: u32) -> Result<()> {
        if self.dead || !self.running {
            return Ok(());
        }
        let delta = delta as i32;
        if delta < self.remaining {
            self.remaining -= delta;
            return Ok(());
        }
        let next = self.frames[self.current as usize].next;
        match next {
            0.. => {
                self.apply(next)?;
                self.remaining = i32::from(self.frames[next as usize].duration);
            }
            -1 => {
                self.stop()?;
                self.dead = true;
            }
            -2 => self.stop()?,
            -3 => {
                self.stop()?;
                self.enabled = false;
            }
            -13..=-11 => {
                self.stop()?;
                self.start((-10 - next) as u32)?;
            }
            _ => bail!("CVNS animation next-frame code {next}"),
        }
        Ok(())
    }

    /// The sprite fields `0x418910` reads.
    pub fn base(&self) -> super::fx::Base {
        let crop = self.crop.map(i32::from);
        super::fx::Base {
            crop,
            at: [self.position[0] - self.origin[0], self.position[1] - self.origin[1]],
            size: self.size,
            alpha: i32::from(self.alpha),
        }
    }

    /// `0x41b5a0` runs the transition first; a finished hiding transition deletes the sprite.
    pub fn advance_effect(&mut self, delta: u32, random: &mut impl FnMut() -> i32) -> Result<()> {
        if self.dead || self.source < 0 {
            return Ok(());
        }
        let base = self.base();
        if let Some(effect) = &mut self.effect {
            if effect.advance(delta, base, random)? {
                let hides = effect.hides;
                self.effect = None;
                if hides {
                    self.dead = true;
                }
            }
        }
        Ok(())
    }

    /// `0x41b8b0` ends the transition at once.
    pub fn finish_effect(&mut self) {
        if let Some(effect) = self.effect.take() {
            if effect.hides {
                self.dead = true;
            }
            if effect.kind == 1 {
                self.alpha = 256;
            }
        }
    }

    /// The cell layers drawn in place of a sprite with a running transition.
    pub fn effect_layers(
        &self,
        id: SpriteId,
        bitmap: impl Fn(Kind, i16) -> Option<UploadedImage>,
        offset: [i32; 2],
    ) -> Result<Option<Vec<serde_json::Value>>> {
        let Some(effect) = &self.effect else {
            return Ok(None);
        };
        if self.dead || !self.enabled || self.hidden != 0 || self.source < 0 {
            return Ok(Some(Vec::new()));
        }
        let image = bitmap(self.kind, self.source)
            .ok_or_else(|| anyhow::anyhow!("CVNS sprite {id:?} draws unloaded bitmap {}", self.source))?;
        let from = effect.from.map(|from| {
            bitmap(self.kind, from).ok_or_else(|| anyhow::anyhow!("CVNS transition bitmap {from} is not loaded"))
        });
        let from = from.transpose()?;
        let order = order(self.layer, id.sequence());
        let base_id = 0x4400_0000 + id.sequence() as u32 * 4096;
        let mut layers = Vec::new();
        for (index, cell) in effect.cells.iter().enumerate() {
            let [iw, ih] = image.size.map(|axis| axis as i32);
            // The blitter clips cell rectangles to the bitmap.
            let [left, top] = [cell.crop[0].clamp(0, iw), cell.crop[1].clamp(0, ih)];
            let [right, bottom] = [cell.crop[2].clamp(left, iw), cell.crop[3].clamp(top, ih)];
            if right <= left || bottom <= top || cell.alpha <= 0 {
                continue;
            }
            let dest = [cell.dest[0] + left - cell.crop[0], cell.dest[1] + top - cell.crop[1]];
            let rect = [dest[0] as f32 + offset[0] as f32, dest[1] as f32 + offset[1] as f32, (right - left) as f32, (bottom - top) as f32];
            let uv = |size: [u32; 2]| {
                let [w, h] = size.map(|axis| axis as f32);
                [left as f32 / w, top as f32 / h, right as f32 / w, bottom as f32 / h]
            };
            // 0x416f20 mixes the old bitmap toward the sprite by the cell opacity.
            if let Some(from) = &from {
                layers.push(json!({"op":"put","layer":{"id":base_id + 2 * index as u32 + 1,"rect":rect,
                    "order":order,"tie_order":2 * index as u64,"uv":uv(from.size),
                    "paint":{"kind":"image","asset":from.path.relative()}}}));
            }
            let opacity = (cell.alpha.min(256) as u32 * 255 / 256) as u8;
            layers.push(json!({"op":"put","layer":{"id":base_id + 2 * index as u32,"rect":rect,
                "order":order,"tie_order":2 * index as u64 + 1,"opacity":opacity,"uv":uv(image.size),
                "paint":{"kind":"image","asset":image.path.relative()}}}));
        }
        Ok(Some(layers))
    }

    /// The layer `0x410700` draws for this record, if any.
    pub fn layer(
        &self,
        id: SpriteId,
        bitmap: impl Fn(Kind, i16) -> Option<UploadedImage>,
        offset: [i32; 2],
    ) -> Result<Option<serde_json::Value>> {
        if self.dead || !self.enabled || self.hidden != 0 || self.alpha == 0 {
            return Ok(None);
        }
        let [left, top, right, bottom] = self.crop.map(i32::from);
        let [w, h] = [right - left, bottom - top];
        let [sx, sy] = self.scale;
        if w <= 0 || h <= 0 || sx == 0 || sy == 0 {
            return Ok(None);
        }
        let [x, y] = self.position;
        let [ox, oy] = self.origin;
        let rect = [(x - ox) as f32 + offset[0] as f32, (y - oy) as f32 + offset[1] as f32, w as f32, h as f32];
        let transform = json!({"scale_x":sx as f32 / 1000.0,"scale_y":sy as f32 / 1000.0,
            "rotate":(self.rotation as f32).to_radians(),"pivot_x":ox as f32,"pivot_y":oy as f32});
        let order = order(self.layer, id.sequence());
        let opacity = (u32::from(self.alpha) * 255 / 256) as u8;
        let mut layer = json!({"op":"put","layer":{"id":id.layer_id(),"rect":rect,
            "transform":transform,"order":order,"opacity":opacity}});
        if self.kind == Kind::Fill {
            // 0x4144c0 adds the packed products before taking their high bytes.
            let (color, weight) = self.tint;
            let weight = i32::from(weight) as u32;
            let blend = |mask: u32| (self.fill & mask).wrapping_mul(weight)
                .wrapping_add((color & mask).wrapping_mul(256u32.wrapping_sub(weight)));
            let [r, g, b] = super::rgb(((blend(0xff00) & 0xff0000) | (blend(0xff00ff) & 0xff00ff00)) >> 8);
            layer["layer"]["paint"] = json!({"kind":"solid","color":[r, g, b, 255]});
            return Ok(Some(layer));
        }
        if self.source < 0 {
            return Ok(None);
        }
        let Some(image) = bitmap(self.kind, self.source) else {
            bail!("CVNS sprite {id:?} draws unloaded bitmap {}", self.source);
        };
        // A script may scroll the right/bottom of a source past its edge. Only
        // its intersection is drawable. Keep the original world-space pivot
        // when clipping removes pixels on the left or top of a transformed quad.
        let [iw, ih] = image.size.map(|n| n as i32);
        let [l, t, r, b] = [left.max(0), top.max(0), right.min(iw), bottom.min(ih)];
        if r <= l || b <= t { return Ok(None); }
        let [dx, dy] = [(l - left) as f32, (t - top) as f32];
        layer["layer"]["rect"] = json!([rect[0] + dx, rect[1] + dy, r - l, b - t]);
        layer["layer"]["transform"]["pivot_x"] = json!(ox as f32 - dx);
        layer["layer"]["transform"]["pivot_y"] = json!(oy as f32 - dy);
        layer["layer"]["uv"] = json!([l as f32 / iw as f32, t as f32 / ih as f32,
            r as f32 / iw as f32, b as f32 / ih as f32]);
        layer["layer"]["paint"] = json!({"kind":"image","asset":image.path.relative()});
        Ok(Some(layer))
    }

    pub fn layer_id(id: SpriteId) -> u32 {
        id.layer_id()
    }
}
