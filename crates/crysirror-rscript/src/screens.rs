// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native script-driven windows of the Shapeshifter build: the map button
//! screen (`CMap`, opcodes 0xDC..0xDF) and the paged selection
//! (`CDynSelect`, 0xD2..0xD5).
use crate::memory::{Memory, scene as S};
use crate::res::join;
use crate::scene::Ctx;
use crate::sprite::{Content, Point, Sprite};

/// An LWG button: `name`, `name_f` (focus), `name_c` (pressed) layers
/// (CAnSelBtn loader 0x4597C0).
#[derive(Clone, Debug, Default)]
pub struct LwgButton {
    pub id: u16,
    pub face: Sprite,
    pub normal: Option<Content>,
    pub focus: Option<Content>,
    pub pressed: Option<Content>,
}
impl LwgButton {
    pub fn load(cx: &mut Ctx, lwg: &str, name: &str, id: u16) -> Option<Self> {
        let (normal, at) = cx.lwg_layer(lwg, name)?;
        let focus = cx.lwg_layer(lwg, &format!("{name}_f")).map(|f| f.0);
        let pressed = cx.lwg_layer(lwg, &format!("{name}_c")).map(|f| f.0);
        let face = Sprite { content: Some(normal.clone()), pos: at, visible: true, ..Default::default() };
        Some(Self { id, face, normal: Some(normal), focus, pressed })
    }
    pub fn hover(&mut self, inside: bool) {
        self.face.hovered = inside;
        let c = if inside { self.focus.clone().or(self.normal.clone()) } else { self.normal.clone() };
        self.face.content = c;
    }
    pub fn press(&mut self) {
        if let Some(p) = self.pressed.clone().or(self.focus.clone()) {
            self.face.content = Some(p);
        }
    }
}

/// `CMap` (0x4172D0): `map%04d.lwg` with buttons `01`..`20`; a per-button
/// flag hides a button (sub_417C20); the sub-map flag raises the screen to
/// priority 0x90 (sub_417790).
#[derive(Default)]
pub struct MapScreen {
    pub number: u16,
    pub sub: u16,
    pub buttons: Vec<LwgButton>,
    /// Hidden flags by button id 1..20.
    pub hidden: [bool; 21],
    pub priority: i16,
    pub input: bool,
}
impl MapScreen {
    pub fn open(&mut self, cx: &mut Ctx, number: u16, sub: u16) {
        self.buttons.clear();
        self.number = number;
        self.sub = sub;
        self.input = false;
        if number == 0 {
            return;
        }
        let lwg = join(cx.apini.dir(crate::apini::Dir::System), &format!("map{number:04}"));
        if cx.res.lwg_of(&lwg).is_none() {
            cx.res.diag(format!("missing map screen {lwg}.lwg"));
            return;
        }
        for id in 1..=20u16 {
            if let Some(b) = LwgButton::load(cx, &lwg, &format!("{id:02}"), id) {
                self.buttons.push(b);
            }
        }
        self.priority = if sub != 0 { 0x90 } else { 0 };
        self.apply();
    }
    /// sub_417C20: buttons show unless flagged.
    pub fn apply(&mut self) {
        for b in &mut self.buttons {
            b.face.visible = !self.hidden[usize::from(b.id)] && b.face.content.is_some();
            b.face.interactive = self.input && b.face.visible;
        }
    }
    pub fn set_input(&mut self, enabled: bool) {
        self.input = enabled;
        if !enabled {
            for b in &mut self.buttons {
                b.hover(false);
            }
        }
        self.apply();
    }
    pub fn visible(&self) -> bool {
        self.number != 0
    }
}

/// One visible page of a `CDynSelect` list: entry indexes, `None` for the
/// "next page" button (sub_409750).
#[derive(Default)]
pub struct DynSelect {
    /// Entry indexes that pass their condition (sub_409910).
    pub visible: Vec<usize>,
    pub cursor: usize,
    pub page: Vec<Option<usize>>,
    pub arrangement: i32,
}

/// Entry `i` of the selection list: (script, text, value, condition).
pub fn entry(mem: &Memory, i: usize) -> (u16, u32, i16, u16) {
    let e = S::SELECTION + 12 + 12 * i;
    (mem.uw(e), mem.d(e + 4) as u32, mem.w(e + 8), mem.uw(e + 10))
}

impl DynSelect {
    /// Entries whose condition variable allows them: bit 15 set shows the
    /// entry while the variable is non-zero, clear while it is zero.
    pub fn filter(mem: &Memory) -> Vec<usize> {
        (0..100)
            .filter(|&i| {
                let (script, text, _, cond) = entry(mem, i);
                if script == 0 && text == 0 {
                    return false;
                }
                if cond == 0 {
                    return true;
                }
                let v = mem.var(i32::from(cond & 0x7FFF)) != 0;
                if cond & 0x8000 != 0 { v } else { !v }
            })
            .collect()
    }
    /// sub_409750: all entries when five or fewer, else four from the
    /// cursor (wrapping) and the "next page" button.
    pub fn next_page(&mut self) {
        self.page.clear();
        if self.visible.len() < 6 {
            self.page = self.visible.iter().map(|&i| Some(i)).collect();
            return;
        }
        if self.cursor >= self.visible.len() {
            self.cursor = 0;
        }
        while self.page.len() < 4 && self.cursor < self.visible.len() {
            self.page.push(Some(self.visible[self.cursor]));
            self.cursor += 1;
        }
        self.page.push(None);
    }
}

/// The paged-selection "next" label without a 0xD4 entry (.rdata 0x4C902C).
pub const NEXT_LABEL: &[u8] = &[0x8E, 0x9F, 0x82, 0xD6];

/// A gauge's widgets: the `CSelNum` digit strip (ten equal cells), the
/// bar and the fixed image; `shown` steps toward `target` while animating.
#[derive(Default)]
pub struct Gauge {
    pub digits: Option<Content>,
    pub bar: Option<Content>,
    pub fix: Sprite,
    pub target: i32,
    pub shown: i32,
    pub animating: bool,
}
impl Gauge {
    /// One tick of the value animation (one unit per tick).
    pub fn tick(&mut self) -> bool {
        if !self.animating {
            return false;
        }
        if self.shown == self.target {
            self.animating = false;
            return false;
        }
        self.shown += (self.target - self.shown).signum();
        true
    }
}

/// One `textN` field of the edit dialog.
#[derive(Clone, Debug)]
pub struct EditField {
    pub register: usize,
    pub rect: [i32; 4],
    pub text: Vec<u8>,
    /// Byte limit (the register's flag byte); 0 is unlimited (200).
    pub limit: usize,
}

/// `CEditMenu` (0x409BC0, opened by window message 0x432 from opcode
/// 0x9C): `texedt%02d.lwg` centred on the screen, edit fields over the
/// `text0`..`text9` layers bound to the string registers, `ok`/`cansel`
/// buttons (sub_40A5B0, sub_40A310, sub_409F80).
pub struct EditDialog {
    pub origin: Point,
    pub layers: Vec<Sprite>,
    pub ok: Option<LwgButton>,
    pub cancel: Option<LwgButton>,
    pub fields: Vec<EditField>,
    pub focus: usize,
    pub session: Option<crysirror_core::service::TextInputSessionHandle>,
    pub preedit: String,
}

/// Truncates to `limit` bytes without splitting a double-byte character.
pub fn truncate_sjis(text: &mut Vec<u8>, limit: usize) {
    if text.len() <= limit {
        return;
    }
    let mut i = 0;
    while i < limit {
        let step = if crate::text::is_lead(text[i]) && i + 1 < text.len() { 2 } else { 1 };
        if i + step > limit {
            break;
        }
        i += step;
    }
    text.truncate(i);
}
/// Removes the last character (one or two bytes).
pub fn pop_sjis(text: &mut Vec<u8>) {
    let mut i = 0;
    let mut last = 0;
    while i < text.len() {
        last = i;
        i += if crate::text::is_lead(text[i]) && i + 1 < text.len() { 2 } else { 1 };
    }
    text.truncate(last);
}

impl EditDialog {
    pub fn open(cx: &mut Ctx, mem: &Memory, number: u16) -> Option<Self> {
        let lwg = join(cx.apini.dir(crate::apini::Dir::System), &format!("texedt{number:02}"));
        let layout = cx.res.lwg_of(&lwg)?.clone();
        let origin = Point { x: cx.apini.width as i32 / 2, y: cx.apini.height as i32 / 2 };
        let ok = LwgButton::load(cx, &lwg, "ok", 1);
        let cancel = LwgButton::load(cx, &lwg, "cansel", 2);
        let mut fields = Vec::new();
        let mut layers = Vec::new();
        let button_state = |n: &str| ["ok", "cansel"].iter().any(|b| n == *b || n.strip_prefix(b).is_some_and(|s| ["_f", "_c", "_l"].contains(&s)));
        for l in &layout.layers {
            if let Some(i) = l.name.strip_prefix("text").and_then(|d| d.parse::<usize>().ok()).filter(|i| *i < 10) {
                let size = l.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([200, 24]);
                let at = S::STRING_REGISTERS + i * S::STRING_STRIDE;
                fields.push(EditField {
                    register: i,
                    rect: [l.x, l.y, size[0] as i32, size[1] as i32],
                    text: mem.str_at(at, S::STRING_STRIDE).to_vec(),
                    limit: usize::from(mem.scene[S::STRING_FLAGS + 4 * i]),
                });
            } else if !button_state(&l.name) {
                if let Some((c, at)) = cx.lwg_layer(&lwg, &l.name) {
                    layers.push(Sprite { content: Some(c), pos: at, visible: true, ..Default::default() });
                }
            }
        }
        Some(Self { origin, layers, ok, cancel, fields, focus: 0, session: None, preedit: String::new() })
    }
    /// sub_40A310: OK stores every field; a limited field left empty
    /// keeps the dialog open.
    pub fn commit(&self, mem: &mut Memory) -> bool {
        for f in &self.fields {
            let mut t = f.text.clone();
            truncate_sjis(&mut t, if f.limit == 0 { 200 } else { f.limit });
            mem.set_str(S::STRING_REGISTERS + f.register * S::STRING_STRIDE, S::STRING_STRIDE, &t);
            if f.limit != 0 && t.is_empty() {
                return false;
            }
        }
        true
    }
    pub fn type_text(&mut self, committed: &str) {
        let Some(f) = self.fields.get_mut(self.focus) else { return };
        f.text.extend_from_slice(&encoding_rs::SHIFT_JIS.encode(committed).0);
        let limit = if f.limit == 0 { 200 } else { f.limit };
        truncate_sjis(&mut f.text, limit);
    }
    pub fn backspace(&mut self) {
        if let Some(f) = self.fields.get_mut(self.focus) {
            pop_sjis(&mut f.text);
        }
    }
}
