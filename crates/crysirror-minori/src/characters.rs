// SPDX-License-Identifier: MPL-2.0
use crate::message_action::{MessageAction, Operation};
use super::face::Face;
use crysirror_image::temporal_pattern::PatternRng;
use super::{Sprite, asset, remove, sprite};
use crate::vm::{CharRequest, Vm};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    service::{HostServices, MediaPhase, MediaRequest},
    state::ClientId,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const VIDEO: u32 = 2;

#[derive(Default)]
pub(super) struct Characters {
    pub eye_preset: Option<i32>,
    pub eye_sequences: super::face::EyeSequences,
    by_id: BTreeMap<u32, Character>,
    next_layer: u32,
    frozen: Vec<u32>,
    frozen_faces: Vec<(u32, Face)>,
    movie: Option<(u32, bool)>,
}
struct Character {
    face: Face,
    layer: u32,
    name: String,
    parts: Vec<Sprite>,
    pos: [i32; 2],
    scroll_y: i32,
    size: Option<[i32; 2]>,
    order: i32,
    blend: i32,
    visible: bool,
    current: bool,
    next: bool,
    motion: Option<Movement>,
    sequence: Option<Sequence>,
    video: Option<String>,
}
struct Movement {
    start: u64,
    duration: u64,
    from: [i32; 3],
    to: [i32; 3],
    ease: [i32; 2],
}
struct Sequence {
    start: u64,
    frames: u32,
    rate: u32,
    first: u32,
    shown: u32,
}

impl Character {
    fn draw(&self, ops: &mut Vec<Value>) {
        for (index, art) in self.parts.iter().enumerate() {
            let id = self.layer + index as u32;
            if !self.current || !self.visible {
                ops.push(remove(id));
                continue;
            }
            let mut art = art.clone();
            if index == 0 {
                if let Some(size) = self.size {
                    art.rect[2..].copy_from_slice(&size);
                }
                if let Some(video) = &self.video {
                    art.asset = video.clone();
                }
            }
            art.rect[0] = self.pos[0].saturating_sub(art.rect[2] / 2);
            art.rect[1] = self.pos[1].saturating_add(self.scroll_y);
            let mut op = art.layer(id, ((self.blend.clamp(0, 256) * 255 + 128) / 256) as u8);
            super::foreground::blend(&mut op, &art.asset);
            op["layer"]["order"] = json!([40, self.order.saturating_add(index as i32)]);
            ops.push(op);
            if index == 0 {
                let original = [self.parts[0].rect[2], self.parts[0].rect[3]];
                self.face.draw(self.layer, art.rect, original, [40, self.order],
                    ((self.blend.clamp(0, 256) * 255 + 128) / 256) as u8, ops);
            }
        }
        if !self.current || !self.visible { self.face.remove(self.layer, ops); }
    }
    fn remove(&self, ops: &mut Vec<Value>) {
        self.face.remove(self.layer, ops);
        for i in 0..self.parts.len() {
            ops.push(remove(self.layer + i as u32));
        }
    }
    fn move_to(
        &mut self,
        now: u64,
        coords: [i32; 2],
        ease: [i32; 2],
        duration: i32,
        rate: i32,
        offset: bool,
    ) {
        let target = if offset {
            [
                self.pos[0].saturating_add(coords[0]),
                self.pos[1].saturating_add(coords[1]),
            ]
        } else {
            coords
        };
        let end_blend = if rate < 0 {
            self.blend
        } else {
            rate.clamp(0, 256)
        };
        if duration <= 0 {
            self.motion = None;
            self.pos = target;
            self.blend = end_blend;
            return;
        }
        self.motion = Some(Movement {
            start: now,
            duration: duration as u64 * 10,
            from: [self.pos[0], self.pos[1], self.blend],
            to: [target[0], target[1], end_blend],
            ease,
        });
    }
    fn move_frame(&mut self, now: u64) -> bool {
        let Some(m) = &self.motion else { return false };
        let elapsed = now.saturating_sub(m.start);
        let t = (elapsed as f64 / m.duration as f64).min(1.);
        let at = |i: usize, mode: i32| {
            let u = match mode {
                1 => t * t,
                2 => t * (2. - t),
                _ => t,
            };
            (i64::from(m.from[i]) + ((f64::from(m.to[i]) - f64::from(m.from[i])) * u) as i64) as i32
        };
        self.pos = [at(0, m.ease[0]), at(1, m.ease[1])];
        self.blend = at(2, 0);
        if elapsed >= m.duration {
            self.motion = None;
        }
        true
    }
}
fn within(id: u32, a: i32, b: i32) -> bool {
    if a <= 0 && b <= 0 {
        return true;
    }
    let (a, b) = (a.unsigned_abs(), b.unsigned_abs());
    (a.min(b)..=a.max(b)).contains(&id)
}
fn read_part(h: &mut dyn HostServices, c: ClientId, name: &str) -> Result<Sprite> {
    sprite(h, c, &["st", "bg", "sys"], name)
}

impl Characters {
    pub(super) fn position(&self,id:i32)->Option<[i32;2]> {self.by_id.get(&id.unsigned_abs()).map(|ch|ch.pos)}

    pub fn generation(&self, id: i32) -> Option<u32> {
        self.by_id.get(&id.unsigned_abs()).map(|ch| ch.layer)
    }

    pub fn message_action(&mut self, h: &mut dyn HostServices, c: ClientId, action: &mut MessageAction,
                          force: bool, ops: &mut Vec<Value>) -> Result<bool> {
        let now = h.now_ms();
        if !force && !action.started && now / 10 < action.due { return Ok(false); }
        let key = action.id.unsigned_abs();
        let Some(ch) = self.by_id.get_mut(&key) else { return Ok(force); };
        match &action.operation {
            Operation::Load { image, duration, blend } => {
                let pos = ch.pos;
                let temp = action.id.wrapping_add(1);
                let temp_key = temp.unsigned_abs();
                if !action.started {
                    self.load(h, c, temp, image.clone(), [None, None], true, ops)?;
                    let new = self.by_id.get_mut(&temp_key).context("missing inline replacement")?;
                    new.pos = pos;
                    new.visible = true;
                    new.current = true;
                    new.blend = 0;
                    new.move_to(now, pos, [0, 0], *duration, *blend, false);
                    new.draw(ops);
                    action.started = true;
                }
                let Some(new) = self.by_id.get_mut(&temp_key) else { return Ok(true); };
                if !force && new.motion.is_some() { return Ok(false); }
                new.move_frame(u64::MAX);
                let mut new = self.by_id.remove(&temp_key).unwrap();
                let old = self.by_id.remove(&key).unwrap();
                old.remove(ops);
                old.face.release(h, c, old.layer)?;
                new.order = old.order;
                self.by_id.insert(key, new);
            }
            _ if Some(ch.layer) != action.generation => return Ok(true),
            Operation::Visible(value) => ch.visible = *value,
            Operation::Position(pos) => ch.pos = *pos,
            Operation::Blend { duration, blend } => {
                if !action.started {
                    ch.move_to(now, ch.pos, [0, 0], *duration, *blend, false);
                    action.started = true;
                }
                if force { ch.move_frame(u64::MAX); }
                if ch.motion.is_some() { ch.draw(ops); return Ok(false); }
            }
        }
        if let Some(ch) = self.by_id.get(&key) { ch.draw(ops); }
        Ok(true)
    }

    fn scroll_actor(&self, prefix: &str) -> Option<&Character> {
        let matches = || self.by_id.values().filter(|ch| ch.name.starts_with(prefix));
        matches()
            .find(|ch| ch.name.contains("_ani_fg."))
            .or_else(|| matches().next())
    }
    pub fn mouth_centers(&self,speaker:&str,out:&mut Vec<[i32;2]>) {
        for ch in self.by_id.values().filter(|ch|ch.visible && ch.current) {
            let [_,_,w,h]=ch.parts[0].rect;
            let size=ch.size.unwrap_or([w,h]);
            ch.face.mouth_centers(speaker,[ch.pos[0].saturating_sub(size[0]/2),ch.pos[1].saturating_add(ch.scroll_y),size[0],size[1]],[w,h],out);
        }
    }
    pub fn scroll_height(&self, prefix: &str) -> Option<i32> {
        let ch = self.scroll_actor(prefix)?;
        ch.size
            .map(|s| s[1])
            .or_else(|| ch.parts.first().map(|p| p.rect[3]))
    }
    pub fn scroll_sample(&self, prefix: &str, curve: &[i32], now: u64) -> Option<i32> {
        if curve.is_empty() {
            return None;
        }
        let seq = self.scroll_actor(prefix)?.sequence.as_ref()?;
        let time = u128::from(now.saturating_sub(seq.start)) * u128::from(seq.rate);
        let i = ((time / 1000 + u128::from(seq.first)) % u128::from(seq.frames)) as usize;
        let next = (i + 1) % seq.frames as usize;
        let a = curve[i % curve.len()];
        let b = curve[next % curve.len()];
        Some((f64::from(a) + (f64::from(b) - f64::from(a)) * (time % 1000) as f64 / 1000.0) as i32)
    }
    pub fn scroll_offset(&mut self, prefix: &str, y: i32, ops: &mut Vec<Value>) {
        for ch in self
            .by_id
            .values_mut()
            .filter(|ch| ch.name.starts_with(prefix) && ch.scroll_y != y)
        {
            ch.scroll_y = y;
            ch.draw(ops);
        }
    }
    fn load(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        id: i32,
        name: String,
        extra: [Option<String>; 2],
        blink: bool,
        ops: &mut Vec<Value>,
    ) -> Result<()> {
        let art = read_part(h, c, &name)?;
        let face = Face::open(h, c, &art, blink, self.eye_preset.unwrap_or(2),self.eye_sequences.clone())?;
        let mut parts = vec![art];
        for n in extra
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty() && s != "*")
        {
            let path = asset("st", &n)?;
            if h.storage(c)?.exists(&path) {
                parts.push(sprite(h, c, &["st"], &n)?);
            }
        }
        let key = id.unsigned_abs();
        self.stop_movie_for(h, c, key)?;
        if let Some(ch) = self.by_id.get_mut(&key) {
            ch.remove(ops);
            ch.face.release(h, c, ch.layer)?;
            ch.face = face;
            ch.parts = parts;
            ch.name = name;
            ch.video = None;
            ch.current |= id >= 0;
            ch.next |= id < 0;
            ch.draw(ops);
        } else {
            self.next_layer = self
                .next_layer
                .checked_add(4)
                .context("minori character layer exhaustion")?;
            let layer = 20000u32
                .checked_add(self.next_layer)
                .context("minori character layer exhaustion")?;
            ensure!(layer <= i32::MAX as u32 - 3, "minori character layer range");
            let ch = Character {
                face,
                layer,
                name,
                parts,
                pos: [0, 0],
                scroll_y: 0,
                size: None,
                order: id.unsigned_abs().min(500000000) as i32 * 4 + 100,
                blend: 256,
                visible: true,
                current: id >= 0,
                next: id < 0,
                motion: None,
                sequence: None,
                video: None,
            };
            ch.draw(ops);
            self.by_id.insert(key, ch);
        }
        Ok(())
    }
    pub fn stop_movie(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if let Some((id, _)) = self.movie {
            self.stop_movie_for(h, c, id)?;
        }
        Ok(())
    }
    fn stop_movie_for(&mut self, h: &mut dyn HostServices, c: ClientId, key: u32) -> Result<()> {
        if self.movie.is_some_and(|(id, _)| id == key) {
            h.media_request(c, MediaRequest::Stop { id: VIDEO })?;
            self.movie = None;
            if let Some(ch) = self.by_id.get_mut(&key) {
                ch.video = None;
            }
        }
        Ok(())
    }
    pub fn release_faces(&self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        for ch in self.by_id.values() { ch.face.release(h, c, ch.layer)?; }
        for (layer, face) in &self.frozen_faces { face.release(h, c, *layer)?; }
        Ok(())
    }
    pub fn active(&self) -> bool {
        self.movie.is_some()
            || self
                .by_id
                .values()
                .any(|ch| ch.motion.is_some() || ch.sequence.is_some() || ch.face.active())
    }
    pub fn moving(&self) -> bool {
        self.by_id.values().any(|ch| ch.motion.is_some())
    }
    pub fn stage(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        ops: &mut Vec<Value>,
    ) -> Result<()> {
        for id in self.frozen.drain(..) {
            ops.push(remove(id));
        }
        if let Some((key, _)) = self.movie {
            if self.by_id.get(&key).is_none_or(|ch| !ch.next) {
                self.stop_movie_for(h, c, key)?;
            }
        }
        for ch in self.by_id.values().filter(|ch| !ch.next) {
            ch.face.release(h, c, ch.layer)?;
            ch.face.remove(ch.layer, ops);
        }
        for (layer, face) in self.frozen_faces.drain(..) {
            face.remove(layer, ops);
            face.release(h, c, layer)?;
        }
        self.by_id.retain(|_, ch| {
            ch.current = ch.next;
            ch.next = false;
            ch.draw(ops);
            ch.current
        });
        Ok(())
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        advance: bool,
        speaker: &str,
        level: super::VoiceLevel,
        rng: &mut PatternRng,
        ops: &mut Vec<Value>,
    ) -> Result<()> {
        if let Some((key, skippable)) = self.movie {
            let phase = h.media_info(c, VIDEO)?.phase;
            ensure!(
                phase != MediaPhase::Failed,
                "minori character movie decode failed"
            );
            if advance && skippable || matches!(phase, MediaPhase::Ended | MediaPhase::Stopped) {
                self.stop_movie_for(h, c, key)?;
                if let Some(ch) = self.by_id.get(&key) {
                    ch.draw(ops);
                }
            }
        }
        let now = h.now_ms();
        for ch in self.by_id.values_mut() {
            let face_changed = ch.face.poll(h, c, ch.layer, speaker, level, rng)?;
            let mut changed = ch.move_frame(now) || face_changed;
            if let Some(seq) = &mut ch.sequence {
                let step = (now.saturating_sub(seq.start) as u128 * u128::from(seq.rate) / 1000
                    + u128::from(seq.first))
                    % u128::from(seq.frames);
                let frame = step as u32;
                if frame != seq.shown {
                    let (base, ext) = ch
                        .name
                        .rsplit_once('.')
                        .context("minori sequence requires image extension")?;
                    let split = base
                        .len()
                        .checked_sub(4)
                        .context("minori sequence requires four-digit suffix")?;
                    let prefix = base
                        .get(..split)
                        .context("minori sequence filename boundary")?;
                    ensure!(
                        base[split..].bytes().all(|b| b.is_ascii_digit()),
                        "minori sequence requires four-digit suffix"
                    );
                    ch.parts[0] = read_part(h, c, &format!("{prefix}{:04}.{ext}", frame + 1))?;
                    seq.shown = frame;
                    changed = true;
                }
            }
            if changed {
                ch.draw(ops);
            }
        }
        Ok(())
    }
    pub fn apply(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        vm: &mut Vm,
        r: CharRequest,
        ops: &mut Vec<Value>,
    ) -> Result<()> {
        let now = h.now_ms();
        match r {
            CharRequest::Available(id) => {
                if let Some(ch) = self.by_id.get_mut(&id.unsigned_abs()) {
                    ch.current = true;
                    ch.draw(ops);
                }
            }
            CharRequest::Load {
                id,
                image,
                secondary,
                tertiary,
                blink,
            } => self.load(h, c, id, image, [secondary, tertiary], blink, ops)?,
            CharRequest::LoadAlias { id, image, blink, .. } => {
                self.load(h, c, id, image, [None, None], blink, ops)?
            }
            CharRequest::Movie {
                id,
                name,
                skippable,
            } => {
                let key = id.unsigned_abs();
                if !self.by_id.contains_key(&key) {
                    return Ok(());
                }
                if let Some((old, _)) = self.movie {
                    self.stop_movie_for(h, c, old)?;
                    if let Some(ch) = self.by_id.get(&old) {
                        ch.draw(ops);
                    }
                }
                let path = asset("mov", &name)?;
                let image = h.media_request(
                    c,
                    MediaRequest::Play {
                        id: VIDEO,
                        asset: path.relative().into(),
                        looping: false,
                        audio: true,
                        gain: 1.,
                    },
                )?;
                let ch = self.by_id.get_mut(&key).unwrap();
                ch.video = Some(image.relative().into());
                ch.draw(ops);
                self.movie = Some((key, skippable));
            }
            CharRequest::Keep { first, last } => {
                for (&id, ch) in &mut self.by_id {
                    if ch.current && within(id, first, last) {
                        ch.next = true;
                    }
                }
            }
            CharRequest::Clear { first, last } => {
                if let Some((key, _)) = self.movie {
                    if within(key, first, last) {
                        self.stop_movie_for(h, c, key)?;
                    }
                }
                for (_, ch) in self.by_id.iter().filter(|(id, _)| within(**id, first, last)) { ch.face.release(h, c, ch.layer)?; }
                self.by_id.retain(|&id, ch| {
                    if within(id, first, last) {
                        ch.remove(ops);
                        false
                    } else {
                        true
                    }
                });
            }
            CharRequest::Freeze => {
                if let Some((key, _)) = self.movie {
                    self.stop_movie_for(h, c, key)?;
                }
                for (_, ch) in std::mem::take(&mut self.by_id) {
                    if ch.current && ch.visible {
                        ch.draw(ops);
                        self.frozen
                            .extend((0..ch.parts.len()).map(|i| ch.layer + i as u32));
                    } else {
                        ch.remove(ops);
                    }
                    self.frozen_faces.push((ch.layer, ch.face));
                }
            }
            CharRequest::AllMove {
                x,
                y,
                duration,
                mode,
            } => {
                for ch in self.by_id.values_mut() {
                    ch.move_to(now, [x, y], [mode; 2], duration, -1, true);
                    ch.draw(ops);
                }
            }
            CharRequest::Move {
                id,
                x,
                y,
                x_mode,
                y_mode,
                duration,
                blocking,
                blend_rate,
                offset,
            } => {
                for (&key, ch) in &mut self.by_id {
                    if id >= 0 && key != id as u32 {
                        continue;
                    }
                    ch.move_to(now, [x, y], [x_mode, y_mode], duration, blend_rate, offset);
                    ch.draw(ops);
                }
                if blocking && self.moving() {
                    vm.begin_char_move();
                }
            }
            req => {
                let id = match &req {
                    CharRequest::Position { id, .. }
                    | CharRequest::Size { id, .. }
                    | CharRequest::Order { id, .. }
                    | CharRequest::BlendRate { id, .. }
                    | CharRequest::Visible { id, .. }
                    | CharRequest::Sequence { id, .. } => *id,
                    _ => bail!("minori character operation {req:?}"),
                };
                let Some(ch) = self.by_id.get_mut(&id.unsigned_abs()) else {
                    return Ok(());
                };
                match req {
                    CharRequest::Position { x, y, .. } => ch.pos = [x, y],
                    CharRequest::Size { width, height, .. } => {
                        ch.size = (width > 0 && height > 0).then_some([width, height])
                    }
                    CharRequest::Order {
                        order, secondary, ..
                    } => {
                        ch.order = if secondary {
                            order
                        } else {
                            order.saturating_mul(4).saturating_add(if order >= 2000 {
                                1000000
                            } else {
                                100
                            })
                        }
                    }
                    CharRequest::BlendRate { rate, .. } => ch.blend = rate,
                    CharRequest::Visible { visible, .. } => ch.visible = visible,
                    CharRequest::Sequence {
                        timed,
                        frames,
                        rate,
                        start,
                        ..
                    } => {
                        ch.sequence = timed.then_some(Sequence {
                            start: now,
                            frames: frames.max(1) as u32,
                            rate: rate.max(1) as u32,
                            first: start.max(0) as u32,
                            shown: 0,
                        })
                    }
                    _ => unreachable!(),
                }
                ch.draw(ops);
            }
        }
        Ok(())
    }
}
