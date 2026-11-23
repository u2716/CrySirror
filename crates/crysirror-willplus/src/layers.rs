// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! `cclass.lua_Layers`: AdvHD's named layer sets as the Lua system layer
//! addresses them. Composition goes through the shared Layer service; this
//! module keeps the AdvHD object model (sets, keyed layers, PNA record
//! sublayers, named relative effects, set transitions, hit tests).
//!
//! Conventions inferred from Script.arc call sites (AdvHD.exe is packed):
//! sublayer numbers are PNA record indices; get* calls take four
//! placeholder values and return four values; `LUA_PARAM_SIZE` yields
//! (w, h); `getCursorPos` returns hit record index + 1 (0 = none); effect
//! values are deltas applied over `duration` milliseconds after `delay`.
use crate::system::{int, num, text};
use anyhow::{bail, Context, Result};
use crysirror_lua::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PARAM_POS: i64 = 1;
pub const PARAM_COLOR: i64 = 2;
pub const PARAM_SIZE: i64 = 3;
pub const PARAM_SCALE: i64 = 4;
pub const PARAM_RECT: i64 = 5;
pub const PARAM_ROTATE: i64 = 6;
pub const PARAM_CENTERZ: i64 = 7;
pub const EFFECT_OPACITY: i64 = 1;
pub const EFFECT_POS: i64 = 2;
pub const EFFECT_SCALE: i64 = 3;
pub const EFFECT_ROTATE: i64 = 4;
pub const TRANSITION_ERASE: i64 = 2;
pub const LAYER_STD: i64 = 1;
pub const LAYER_PNA: i64 = 2;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Params {
    pub pos: [f64; 4],
    pub color: [f64; 4],
    pub scale: [f64; 4],
    pub rotate: [f64; 4],
    pub rect: Option<[f64; 4]>,
    pub centerz: [f64; 4],
}
impl Default for Params {
    fn default() -> Self {
        Self { pos: [0.; 4], color: [1.; 4], scale: [1., 1., 1., 0.], rotate: [0.; 4], rect: None, centerz: [0.; 4] }
    }
}
impl Params {
    fn slot(&mut self, kind: i64) -> Result<&mut [f64; 4]> {
        Ok(match kind {
            PARAM_POS => &mut self.pos,
            PARAM_COLOR => &mut self.color,
            PARAM_SCALE => &mut self.scale,
            PARAM_ROTATE => &mut self.rotate,
            PARAM_CENTERZ => &mut self.centerz,
            PARAM_RECT => self.rect.get_or_insert([0.; 4]),
            other => bail!("AdvHD layer param {other} is not settable"),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub kind: i64,
    pub delta: [f64; 4],
    pub ease: i64,
    pub duration: f64,
    pub delay: f64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Effect {
    pub tracks: Vec<Track>,
    /// Start time once running; loops < 0 repeat forever.
    pub start: Option<u64>,
    pub loops: i64,
    /// Delta already folded into params by earlier passes of each track.
    pub applied: Vec<[f64; 4]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sub {
    pub visible: bool,
    /// POS was set explicitly: `params.pos` is then the layer-local origin
    /// (what getSubLayerParam returns); otherwise it is a delta from the
    /// PNA record's own offset.
    #[serde(default)]
    pub placed: bool,
    pub params: Params,
    pub effects: BTreeMap<String, Effect>,
    pub hit: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    Std { path: String },
    Pna { path: String },
    /// Host-drawn text: `setText` styles it, or the `CLegacyFontInfo`
    /// entry for the same (set, key) when one is registered.
    Text { size: [f64; 2], margins: [f64; 4], line: f64, text: String, font_size: f64, color: [u8; 4], #[serde(default)] edge: Option<u32>,
        /// `effectText` reveal start (ms); None = fully shown.
        #[serde(default)] reveal: Option<u64> },
    /// One record of another PNA (thumbnails, numbers).
    PnaRef { path: String, record: usize },
    /// Save thumbnail (`GetThumbnailPath` value; composed by the host).
    Thumbnail { path: String },
    /// Live game screen scaled to `size` (config page previews).
    Preview { size: [f64; 2] },
    /// Message log text (`log`); `top` is the first shown line.
    Log { size: [f64; 2], margins: [f64; 4], line: f64, top: usize, hover: i64 },
    /// Scene of a log entry's checkpoint scaled to `size`.
    LogPreview { entry: usize, size: [f64; 2] },
}

/// Sublayer frame flip: (record, phase 0..1) frames over `period` ms.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Animation {
    pub frames: Vec<(usize, f64)>,
    pub start: Option<u64>,
    pub period: f64,
    pub loops: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub key: String,
    pub kind: Kind,
    pub visible: bool,
    pub params: Params,
    pub effects: BTreeMap<String, Effect>,
    pub subs: BTreeMap<usize, Sub>,
    #[serde(default)]
    pub animations: BTreeMap<String, Animation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub from: f64,
    pub to: f64,
    pub start: u64,
    pub duration: f64,
    pub delay: f64,
    pub erase: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Set {
    pub name: String,
    pub items: Vec<Item>,
    pub opacity: f64,
    pub transition: Option<Transition>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Layers {
    /// Milliseconds per character for `effectText` (getMessageSpeed).
    #[serde(default)]
    pub text_interval: u64,
    pub sets: Vec<Set>,
    pub snaps: BTreeMap<String, bool>,
    pub force_bg: bool,
    pub dirty: bool,
    #[serde(default)]
    pub overlay: Option<String>,
}

/// Resolved PNA geometry the service needs (record → offset/size).
pub trait Geometry {
    fn records(&mut self, path: &str) -> Result<Vec<Option<([i32; 2], [u32; 2])>>>;
    fn image_size(&mut self, path: &str) -> Result<[u32; 2]>;
}

fn ease(kind: i64, t: f64) -> f64 {
    match kind {
        1 => t * t,
        2 => 1. - (1. - t) * (1. - t),
        3 => t * t * (3. - 2. * t),
        _ => t,
    }
}

fn four(values: [f64; 4]) -> Vec<Value> {
    values.iter().map(|v| if v.fract() == 0. && v.abs() < 1e15 { Value::Int(*v as i64) } else { Value::Num(*v) }).collect()
}

impl Sub {
    /// Layer-local origin of the sublayer whose PNA record sits at `offset`.
    pub fn origin(&self, offset: [i32; 2]) -> [f64; 2] {
        if self.placed {
            [self.params.pos[0], self.params.pos[1]]
        } else {
            [self.params.pos[0] + f64::from(offset[0]), self.params.pos[1] + f64::from(offset[1])]
        }
    }
}

impl Effect {
    /// Advance; returns false when finished (deltas fully applied).
    fn tick(&mut self, params: &mut Params, now: u64) -> Result<bool> {
        let Some(start) = self.start else { return Ok(true) };
        if self.applied.len() != self.tracks.len() {
            self.applied = vec![[0.; 4]; self.tracks.len()];
        }
        let elapsed = now.saturating_sub(start) as f64;
        let mut running = false;
        let span = self.tracks.iter().map(|t| t.delay + t.duration).fold(0., f64::max);
        for (n, track) in self.tracks.iter().enumerate() {
            let t = if track.duration <= 0. {
                f64::from(u8::from(elapsed >= track.delay))
            } else {
                ((elapsed - track.delay) / track.duration).clamp(0., 1.)
            };
            if t < 1. {
                running = true;
            }
            let k = ease(track.ease, t);
            let slot = match track.kind {
                EFFECT_OPACITY => &mut params.color,
                EFFECT_POS => &mut params.pos,
                EFFECT_SCALE => &mut params.scale,
                EFFECT_ROTATE => &mut params.rotate,
                other => bail!("AdvHD effect type {other} is not connected"),
            };
            for i in 0..4 {
                let target = track.delta[i] * k;
                slot[i] += target - self.applied[n][i];
                self.applied[n][i] = target;
            }
        }
        if !running && self.loops != 0 && span > 0. {
            if self.loops > 0 {
                self.loops -= 1;
            }
            self.start = Some(start + span as u64);
            self.applied = vec![[0.; 4]; self.tracks.len()];
            return Ok(true);
        }
        Ok(running)
    }
}

impl Layers {
    fn set_index(&self, name: &str) -> Option<usize> {
        self.sets.iter().position(|s| s.name == name)
    }
    pub fn set_mut(&mut self, name: &str) -> Result<&mut Set> {
        self.sets.iter_mut().find(|s| s.name == name).with_context(|| format!("AdvHD layer set {name:?} does not exist"))
    }
    pub fn item_mut(&mut self, set: &str, key: &str) -> Result<&mut Item> {
        self.set_mut(set)?.items.iter_mut().find(|i| i.key == key).with_context(|| format!("AdvHD layer {set:?}/{key:?} does not exist"))
    }
    fn sub_mut(&mut self, set: &str, key: &str, record: i64) -> Result<&mut Sub> {
        let record = usize::try_from(record).context("AdvHD sublayer index")?;
        Ok(self.item_mut(set, key)?.subs.entry(record).or_default())
    }

    /// Advance effects and transitions.
    pub fn tick(&mut self, now: u64) -> Result<bool> {
        let mut animating = false;
        for set in &mut self.sets {
            if let Some(tr) = &set.transition {
                let t = ((now.saturating_sub(tr.start) as f64 - tr.delay) / tr.duration.max(1.)).clamp(0., 1.);
                set.opacity = tr.from + (tr.to - tr.from) * t;
                if tr.erase {
                    set.opacity = 1. - set.opacity;
                }
                if t >= 1. {
                    set.transition = None;
                } else {
                    animating = true;
                }
                self.dirty = true;
            }
            for item in &mut set.items {
                let mut done = Vec::new();
                for (name, effect) in &mut item.effects {
                    if effect.start.is_some() {
                        self.dirty = true;
                        if effect.tick(&mut item.params, now)? {
                            animating = true;
                        } else {
                            done.push(name.clone());
                        }
                    }
                }
                for name in done {
                    item.effects.remove(&name);
                }
                let mut finished = Vec::new();
                for (name, animation) in &mut item.animations {
                    let Some(start) = animation.start else { continue };
                    if animation.frames.is_empty() || animation.period <= 0. {
                        continue;
                    }
                    let elapsed = now.saturating_sub(start) as f64 / animation.period;
                    let phase = if animation.loops < 0 { elapsed.fract() } else if elapsed >= (animation.loops.max(1)) as f64 { finished.push(name.clone()); 0.999_999 } else { elapsed.fract() };
                    let current = animation.frames.iter().filter(|(_, at)| *at <= phase).max_by(|a, b| a.1.total_cmp(&b.1)).or(animation.frames.first()).map(|(r, _)| *r);
                    for (record, _) in &animation.frames {
                        item.subs.entry(*record).or_default().visible = Some(*record) == current;
                    }
                    animating = true;
                    self.dirty = true;
                }
                for name in finished {
                    if let Some(a) = item.animations.get_mut(&name) {
                        a.start = None;
                    }
                }
                for sub in item.subs.values_mut() {
                    let mut done = Vec::new();
                    for (name, effect) in &mut sub.effects {
                        if effect.start.is_some() {
                            self.dirty = true;
                            if effect.tick(&mut sub.params, now)? {
                                animating = true;
                            } else {
                                done.push(name.clone());
                            }
                        }
                    }
                    for name in done {
                        sub.effects.remove(&name);
                    }
                }
            }
        }
        Ok(animating)
    }

    /// Topmost hit-test sublayer under (x, y) in `set`: record index.
    pub fn hit(&self, set: &str, x: f64, y: f64, geometry: &mut dyn Geometry) -> Result<Option<(String, usize)>> {
        let Some(index) = self.set_index(set) else { return Ok(None) };
        let set = &self.sets[index];
        for item in set.items.iter().rev() {
            if !item.visible {
                continue;
            }
            let Kind::Pna { path } = &item.kind else { continue };
            let records = geometry.records(path)?;
            //Lower record indices draw above higher ones.
            for (record, sub) in &item.subs {
                if sub.hit.is_none() {
                    continue;
                }
                let Some(Some((offset, size))) = records.get(*record) else { continue };
                let origin = sub.origin(*offset);
                let left = item.params.pos[0] + origin[0];
                let top = item.params.pos[1] + origin[1];
                if x >= left && y >= top && x < left + f64::from(size[0]) && y < top + f64::from(size[1]) {
                    return Ok(Some((item.key.clone(), *record)));
                }
            }
        }
        Ok(None)
    }

    pub fn call(&mut self, method: &str, args: &[Value], now: u64, cursor: [f64; 2], geometry: &mut dyn Geometry) -> Result<Vec<Value>> {
        //args[0] is the object itself.
        let a = &args[1.min(args.len())..];
        let s = |n: usize| text(a, n);
        self.dirty = true;
        //Calls addressing a layer that was never inserted (e.g. MsgFace
        //before faceRead) leave state unchanged; getters echo placeholders.
        const KEYED: &[&str] = &["setParam", "getParam", "setVisible", "initSubLayer", "setSubLayerVisible", "setSubLayerParam", "getSubLayerParam",
            "setSubLayerHitTest", "setEffect", "setSubLayerEffect", "startEffect", "startSubLayerEffect", "setText", "setAnimation", "startAnimation"];
        if KEYED.contains(&method) {
            let (set, key) = (s(0)?, s(1)?);
            if !self.sets.iter().any(|x| x.name == set && x.items.iter().any(|i| i.key == key)) {
                return Ok(match method {
                    "getParam" => a.get(3..7.min(a.len())).unwrap_or_default().to_vec(),
                    "getSubLayerParam" => a.get(4..8.min(a.len())).unwrap_or_default().to_vec(),
                    _ => vec![],
                });
            }
        }
        Ok(match method {
            "create" | "delete" => vec![],
            "add" => {
                let name = s(0)?;
                if a.len() > 1 {
                    bail!("AdvHD lua_Layers:add with text-style arguments ({} values) is not connected", a.len());
                }
                if self.set_index(&name).is_some() {
                    vec![Value::Bool(false)]
                } else {
                    self.sets.push(Set { name, items: Vec::new(), opacity: 1., transition: None });
                    vec![Value::Bool(true)]
                }
            }
            "remove" => {
                let name = s(0)?;
                if a.len() > 1 {
                    let key = s(1)?;
                    if let Ok(set) = self.set_mut(&name) {
                        set.items.retain(|i| i.key != key);
                    }
                } else {
                    self.sets.retain(|set| set.name != name);
                }
                vec![]
            }
            "insertLayer" => {
                //(set, key, kind, archive from getGraphicsArcFileName, file, visible);
                //names are unique across archives, so the file alone resolves.
                let (set, key, kind, path) = (s(0)?, s(1)?, int(a, 2)?, s(4)?);
                let path = path.replace('\\', "/");
                let kind = match kind {
                    LAYER_PNA => Kind::Pna { path },
                    LAYER_STD => Kind::Std { path },
                    other => bail!("AdvHD insertLayer kind {other}"),
                };
                let visible = int(a, 5)? != 0;
                //A file absent from every archive (Sys_No.pna in this title)
                //is a failed insert: false, and no layer.
                let probe = match &kind {
                    Kind::Pna { path } | Kind::Std { path } => geometry.image_size(path),
                    _ => Ok([0, 0]),
                };
                if probe.is_err() {
                    if let Ok(target) = self.set_mut(&set) {
                        target.items.retain(|i| i.key != key);
                    }
                    return Ok(vec![Value::Bool(false)]);
                }
                let target = self.set_mut(&set)?;
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            "insertPNARefLayer" => {
                let (set, key, path) = (s(0)?, s(1)?, s(2)?);
                let record = usize::try_from(int(a, 3)?).context("AdvHD PNA reference record")?;
                let visible = int(a, 4)? != 0;
                let target = self.set_mut(&set)?;
                //The source names a PNA layer already in the set (config
                //page samples), or a file.
                let path = match target.items.iter().find(|i| i.key == path).map(|i| &i.kind) {
                    Some(Kind::Pna { path }) | Some(Kind::PnaRef { path, .. }) => path.clone(),
                    _ => path.replace('\\', "/"),
                };
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind: Kind::PnaRef { path, record }, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            "insertThumbnailLayer" => {
                //(set, key, path from GetThumbnailPath, visible).
                let (set, key, path) = (s(0)?, s(1)?, s(2)?);
                let visible = int(a, 3)? != 0;
                let target = self.set_mut(&set)?;
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind: Kind::Thumbnail { path }, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            "insertLogPreviewLayer" => {
                //(set, key, entry from getIndexLogLayerText, width, height, visible).
                let (set, key) = (s(0)?, s(1)?);
                let entry = usize::try_from(int(a, 2)?).unwrap_or(usize::MAX);
                let size = [num(a, 3)?, num(a, 4)?];
                let visible = int(a, 5)? != 0;
                let target = self.set_mut(&set)?;
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind: Kind::LogPreview { entry, size }, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            "insertPreviewLayer" => {
                //(set, key, width, height, visible).
                let (set, key) = (s(0)?, s(1)?);
                let size = [num(a, 2)?, num(a, 3)?];
                let visible = int(a, 4)? != 0;
                let target = self.set_mut(&set)?;
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind: Kind::Preview { size }, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            "insertTextLayer" => {
                let (set, key) = (s(0)?, s(1)?);
                let kind = Kind::Text {
                    size: [num(a, 2)?, num(a, 3)?],
                    margins: [num(a, 4)?, num(a, 5)?, num(a, 6)?, num(a, 7)?],
                    line: num(a, 8)?,
                    text: String::new(),
                    font_size: 24.,
                    color: [255; 4],
                    edge: None,
                    reveal: None,
                };
                let visible = int(a, 9)? != 0;
                let target = self.set_mut(&set)?;
                target.items.retain(|i| i.key != key);
                target.items.push(Item { key, kind, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
                vec![Value::Bool(true)]
            }
            //effectText reveals the text at the message speed, then the host
            //calls onMessageOutEnd(set) (config TEXT page sample).
            "setText" | "effectText" => {
                //(set, key, text, font, size, frame, ?, colour, edge, ...):
                //the CLegacyFontInfo:add argument shape; colours 0xRRGGBBAA.
                let (set, key, body) = (s(0)?, s(1)?, s(2)?);
                let size = num(a, 4)?;
                let frame = num(a, 5)?;
                let color = int(a, 7)? as u32;
                let edge = int(a, 8)? as u32;
                let item = self.item_mut(&set, &key)?;
                let Kind::Text { text, font_size, color: c, edge: e, reveal, .. } = &mut item.kind else { bail!("AdvHD setText on a non-text layer {set}/{key}") };
                *reveal = (method == "effectText").then_some(now);
                *text = body;
                *font_size = size;
                *c = crate::vm::rgba(color);
                *e = (frame > 0.).then_some(edge);
                vec![]
            }
            "messageLength" => {
                let (set, key) = (s(0)?, s(1)?);
                let length = match self.item_mut(&set, &key).map(|i| &i.kind) {
                    Ok(Kind::Text { text, .. }) => crate::vm::Machine::message(text).unwrap_or_else(|_| text.clone()).chars().filter(|c| *c != '\n').count(),
                    _ => 0,
                };
                vec![Value::Int(length as i64)]
            }
            "clearText" => {
                let (set, key) = (s(0)?, s(1)?);
                if let Ok(item) = self.item_mut(&set, &key) {
                    if let Kind::Text { text, .. } = &mut item.kind {
                        text.clear();
                    }
                }
                vec![]
            }
            "removeLayer" => {
                let (set, key) = (s(0)?, s(1)?);
                if let Ok(target) = self.set_mut(&set) {
                    target.items.retain(|i| i.key != key);
                }
                vec![]
            }
            "initSubLayer" => {
                let (set, key) = (s(0)?, s(1)?);
                let visible = int(a, 3)? != 0;
                let sub = self.sub_mut(&set, &key, int(a, 2)?)?;
                *sub = Sub { visible, ..Default::default() };
                vec![]
            }
            "setSubLayerVisible" => {
                let (set, key) = (s(0)?, s(1)?);
                let visible = int(a, 3)? != 0;
                self.sub_mut(&set, &key, int(a, 2)?)?.visible = visible;
                vec![]
            }
            "setSubLayerParam" => {
                let (set, key) = (s(0)?, s(1)?);
                let kind = int(a, 3)?;
                let values = [num(a, 4)?, num(a, 5)?, num(a, 6)?, num(a, 7)?];
                let sub = self.sub_mut(&set, &key, int(a, 2)?)?;
                *sub.params.slot(kind)? = values;
                if kind == PARAM_POS {
                    sub.placed = true;
                }
                vec![]
            }
            "getSubLayerParam" => {
                let (set, key) = (s(0)?, s(1)?);
                let record = int(a, 2)?;
                let kind = int(a, 3)?;
                let item = self.item_mut(&set, &key)?;
                let path = match &item.kind {
                    Kind::Pna { path } => path.clone(),
                    _ => bail!("AdvHD getSubLayerParam on a non-PNA layer {set}/{key}"),
                };
                let sub = item.subs.get(&usize::try_from(record)?).cloned().unwrap_or_default();
                let geometry_record = geometry.records(&path)?.get(record as usize).copied().flatten();
                let (offset, size) = geometry_record.unwrap_or(([0, 0], [0, 0]));
                four(match kind {
                    PARAM_POS => {
                        let origin = sub.origin(offset);
                        [origin[0], origin[1], sub.params.pos[2], sub.params.pos[3]]
                    }
                    PARAM_SIZE => [f64::from(size[0]), f64::from(size[1]), num(a, 6)?, num(a, 7)?],
                    PARAM_COLOR => sub.params.color,
                    PARAM_SCALE => sub.params.scale,
                    PARAM_ROTATE => sub.params.rotate,
                    PARAM_RECT => sub.params.rect.unwrap_or([0., 0., f64::from(size[0]), f64::from(size[1])]),
                    other => bail!("AdvHD getSubLayerParam type {other}"),
                })
            }
            "setParam" => {
                let (set, key) = (s(0)?, s(1)?);
                let kind = int(a, 2)?;
                let values = [num(a, 3)?, num(a, 4)?, num(a, 5)?, num(a, 6)?];
                *self.item_mut(&set, &key)?.params.slot(kind)? = values;
                vec![]
            }
            "getParam" => {
                let (set, key) = (s(0)?, s(1)?);
                let kind = int(a, 2)?;
                let item = self.item_mut(&set, &key)?.clone();
                let size = match &item.kind {
                    Kind::Std { path } => geometry.image_size(path)?.map(f64::from),
                    Kind::Pna { path } => geometry.image_size(path)?.map(f64::from),
                    Kind::Text { size, .. } => *size,
                    Kind::PnaRef { path, record } => geometry.records(path)?.get(*record).copied().flatten().map_or([0., 0.], |(_, s)| s.map(f64::from)),
                    Kind::Thumbnail { .. } => crate::vm::THUMB_SIZE,
                    Kind::Preview { size } | Kind::LogPreview { size, .. } | Kind::Log { size, .. } => *size,
                };
                four(match kind {
                    PARAM_POS => item.params.pos,
                    PARAM_SIZE => [size[0], size[1], num(a, 5)?, num(a, 6)?],
                    PARAM_COLOR => item.params.color,
                    PARAM_SCALE => item.params.scale,
                    PARAM_ROTATE => item.params.rotate,
                    PARAM_RECT => item.params.rect.unwrap_or([0., 0., size[0], size[1]]),
                    PARAM_CENTERZ => item.params.centerz,
                    other => bail!("AdvHD getParam type {other}"),
                })
            }
            "setVisible" => {
                let (set, key) = (s(0)?, s(1)?);
                let visible = int(a, 2)? != 0;
                self.item_mut(&set, &key)?.visible = visible;
                vec![]
            }
            "isVisible" => {
                let (set, key) = (s(0)?, s(1)?);
                vec![Value::Bool(self.item_mut(&set, &key).is_ok_and(|i| i.visible))]
            }
            "setSubLayerHitTest" => {
                let (set, key) = (s(0)?, s(1)?);
                let enable = int(a, 3)? != 0;
                let threshold = num(a, 4)?;
                self.sub_mut(&set, &key, int(a, 2)?)?.hit = enable.then_some(threshold);
                vec![]
            }
            "clearHitTestAll" => {
                let (set, key) = (s(0)?, s(1)?);
                if let Ok(item) = self.item_mut(&set, &key) {
                    for sub in item.subs.values_mut() {
                        sub.hit = None;
                    }
                }
                vec![]
            }
            "getCursorPos" => {
                let set = s(0)?;
                //(set, x, y, 0): the host's cursor when x/y are placeholders.
                let (x, y) = (num(a, 1)?, num(a, 2)?);
                let (x, y) = if a.len() >= 3 { (x, y) } else { (cursor[0], cursor[1]) };
                vec![Value::Int(self.hit(&set, x, y, geometry)?.map_or(0, |(_, record)| record as i64 + 1))]
            }
            "setEffect" | "setSubLayerEffect" => {
                let sub = method == "setSubLayerEffect";
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                let at = if sub { 4 } else { 3 };
                let track = Track {
                    kind: int(a, at)?,
                    delta: [num(a, at + 1)?, num(a, at + 2)?, num(a, at + 3)?, num(a, at + 4)?],
                    ease: int(a, at + 5)?,
                    duration: num(a, at + 6)?,
                    delay: num(a, at + 7)?,
                };
                let effects = if sub { &mut self.sub_mut(&set, &key, int(a, 3)?)?.effects } else { &mut self.item_mut(&set, &key)?.effects };
                let effect = effects.entry(name).or_default();
                if effect.start.is_some() {
                    *effect = Effect::default();
                }
                effect.tracks.push(track);
                vec![]
            }
            "startEffect" | "startSubLayerEffect" => {
                let sub = method == "startSubLayerEffect";
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                let loops = int(a, if sub { 4 } else { 3 })?;
                let effects = if sub { &mut self.sub_mut(&set, &key, int(a, 3)?)?.effects } else { &mut self.item_mut(&set, &key)?.effects };
                if let Some(effect) = effects.get_mut(&name) {
                    effect.start = Some(now);
                    effect.loops = loops;
                    effect.applied.clear();
                }
                vec![]
            }
            "cancelEffect" | "cancelSubLayerEffect" => {
                let sub = method == "cancelSubLayerEffect";
                let (set, key) = (s(0)?, s(1)?);
                let name = s(if sub { 3 } else { 2 })?;
                let Ok(item) = self.item_mut(&set, &key) else { return Ok(vec![]) };
                let effects = if sub {
                    match item.subs.get_mut(&usize::try_from(int(a, 2)?)?) {
                        Some(sub) => &mut sub.effects,
                        None => return Ok(vec![]),
                    }
                } else {
                    &mut item.effects
                };
                effects.remove(&name);
                vec![]
            }
            "isEffect" => {
                let (set, name) = (s(0)?, s(1)?);
                let running = self.sets.iter().filter(|s| s.name == set).flat_map(|s| &s.items).any(|item| {
                    item.effects.get(&name).is_some_and(|e| e.start.is_some()) || item.subs.values().any(|sub| sub.effects.get(&name).is_some_and(|e| e.start.is_some()))
                });
                vec![Value::Bool(running)]
            }
            "startTransition" => {
                let set = s(0)?;
                let transition = Transition {
                    from: num(a, 1)?,
                    to: num(a, 2)?,
                    start: now,
                    duration: num(a, 3)?,
                    delay: num(a, 4)?,
                    erase: int(a, 5)? == TRANSITION_ERASE,
                };
                let target = self.set_mut(&set)?;
                target.opacity = if transition.erase { 1. - transition.from } else { transition.from };
                target.transition = Some(transition);
                vec![]
            }
            "isTransition" => {
                let set = s(0)?;
                vec![Value::Bool(self.sets.iter().any(|s| s.name == set && s.transition.is_some()))]
            }
            "forwardLayerOrder" | "swapLayerOrder" => {
                let (set, first, second) = (s(0)?, s(1)?, s(2)?);
                let target = self.set_mut(&set)?;
                let (Some(x), Some(y)) = (target.items.iter().position(|i| i.key == first), target.items.iter().position(|i| i.key == second)) else {
                    return Ok(vec![]);
                };
                if method == "swapLayerOrder" {
                    target.items.swap(x, y);
                } else if x < y {
                    //Bring `first` directly above `second`.
                    let item = target.items.remove(x);
                    target.items.insert(y, item);
                }
                vec![]
            }
            "setAnimation" => {
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                let record = usize::try_from(int(a, 3)?).context("AdvHD animation record")?;
                let phase = num(a, 4)?;
                let animation = self.item_mut(&set, &key)?.animations.entry(name).or_default();
                animation.frames.retain(|(r, _)| *r != record);
                animation.frames.push((record, phase));
                vec![]
            }
            "startAnimation" => {
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                let (period, loops) = (num(a, 3)?, int(a, 4)?);
                if let Some(animation) = self.item_mut(&set, &key)?.animations.get_mut(&name) {
                    animation.start = Some(now);
                    animation.period = period;
                    animation.loops = loops;
                }
                vec![]
            }
            "stopAnimation" => {
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                if let Ok(item) = self.item_mut(&set, &key) {
                    if let Some(animation) = item.animations.get_mut(&name) {
                        animation.start = None;
                    }
                }
                vec![]
            }
            "getAnimationState" => {
                let (set, key, name) = (s(0)?, s(1)?, s(2)?);
                let state = self.item_mut(&set, &key).ok().and_then(|i| i.animations.get(&name)).map(|a| a.start.is_some());
                vec![Value::Bool(state.unwrap_or(false))]
            }
            "setOverlaySet" => {
                //The named set composes above every other set.
                self.overlay = Some(s(0)?);
                vec![]
            }
            //Text layers read their CLegacyFontInfo colours at composition.
            "updateTextColor" => vec![],
            //Snapshot blur radius for menu backgrounds (snapshots compose live).
            "setBlurCount" => vec![],
            "getScreenAspect" => four([0., 0., 1280., 720.]),
            "isMosaic" => vec![Value::Bool(false)],
            //setSnap2 re-takes an existing snapshot (log redraw).
            "setSnap" | "setSnap2" => {
                self.snaps.insert(s(0)?, a.get(1).is_some_and(Value::truthy));
                vec![]
            }
            "copySnap" | "rtSnap" => vec![],
            "removeSnap" => {
                self.snaps.remove(&s(0)?);
                vec![]
            }
            "setForceBG" => {
                self.force_bg = a.first().is_some_and(Value::truthy);
                vec![]
            }
            other => bail!("AdvHD lua_Layers:{other} is not connected"),
        })
    }
}
