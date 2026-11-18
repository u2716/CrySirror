// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Original slot files and the save screen of the Shapeshifter build
//! (`CSave` 0x41D230, opened in save mode by opcode 0x11 through
//! sub_430FB0; writer sub_433240, reader sub_432FB0, header sub_433450).
//!
//! `<prefix>NN.dat` (NN = page·10 + slot + 1): a 0x68-byte header (year,
//! month, day, hour, minute, variables 1..3, two zero words, the backdrop
//! image number of the message snapshot (layer 0), then the page text from
//! +22, at most 80 bytes cut on a character boundary), the message-snapshot scene (0x996C) and play-through
//! variables (0x36B0), then the previous-choice scene and variables.
//!
//! `<prefix>.dat` (sub_433180, read at start-up by sub_432D60): the
//! configuration block, variables 7000..9999, then two bit-set maps
//! (sub_42B4B0/sub_42B2A0): read text (key = script, bit = string index) and
//! seen images (key 0, bit = image number).
use crate::apini::{Apini, Dir};
use crate::memory::{CONFIG_SIZE, GAME_VARIABLES, Memory, SCENE_SIZE, config as C};
use crate::res::join;
use crate::scene::Ctx;
use crate::screens::LwgButton;
use crate::sprite::{Content, Point, Sprite, pt};
use crate::text::is_lead;
use crate::vm::Vm;
use anyhow::{Result, bail, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    image_processing::ImageProcessState,
    service::{CivilTimeQuery, CivilTimeState, EngineQuery, HostServices, buttons},
};
use serde_json::json;

pub const HEADER: usize = 0x68;
pub const SLOT_SIZE: usize = HEADER + 2 * (SCENE_SIZE + GAME_VARIABLES * 2);
/// Message box strings of the verified build (.rdata 0x4CB854, 0x4CB85C).
pub const CONFIRM_CAPTION: &str = "確認";
pub const OVERWRITE_TEXT: &str = "セーブデータを上書きします。\nよろしいですか？";
/// .rdata 0x4CB88C, 0x4CB9BC, 0x4CB9D8, 0x4CB828.
pub const LOAD_TEXT: &str = "セーブデータをロードします。\nよろしいですか？";
pub const QUICK_SAVE_TEXT: &str = "クイックセーブしますか？";
pub const QUICK_LOAD_TEXT: &str = "クイックロードしますか？";
pub const TITLE_TEXT: &str = "タイトル画面に戻ります。\nよろしいですか？";

/// A `std::map<u16, byte vector>` bit set (sub_42B5A0 sets, sub_42B230
/// tests): bit n is `1 << (n & 7)` of byte `n >> 3`, the vector growing to
/// the byte it needs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitSets(pub std::collections::BTreeMap<u16, Vec<u8>>);
impl BitSets {
    pub fn get(&self, key: u16, bit: u32) -> bool {
        self.0.get(&key).and_then(|v| v.get((bit >> 3) as usize)).is_some_and(|b| b & (1 << (bit & 7)) != 0)
    }
    pub fn set(&mut self, key: u16, bit: u32) {
        let v = self.0.entry(key).or_default();
        let at = (bit >> 3) as usize;
        if v.len() <= at {
            v.resize(at + 1, 0);
        }
        v[at] |= 1 << (bit & 7);
    }
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend((self.0.len() as u32).to_le_bytes());
        for (k, v) in &self.0 {
            out.extend(k.to_le_bytes());
            out.extend((v.len() as u32).to_le_bytes());
            out.extend(v);
        }
    }
    /// Reads one map; a truncated tail ends the map like the native reads.
    pub fn decode(b: &[u8], at: &mut usize) -> Self {
        let mut out = Self::default();
        let rd = |at: usize, n: usize| b.get(at..at + n);
        let Some(count) = rd(*at, 4).map(|c| u32::from_le_bytes(c.try_into().unwrap())) else { return out };
        *at += 4;
        for _ in 0..count {
            let (Some(k), Some(n)) = (rd(*at, 2), rd(*at + 2, 4)) else { break };
            let k = u16::from_le_bytes(k.try_into().unwrap());
            let n = u32::from_le_bytes(n.try_into().unwrap()) as usize;
            *at += 6;
            if n == 0 {
                continue;
            }
            let Some(v) = rd(*at, n) else { break };
            *at += n;
            out.0.entry(k).or_insert_with(|| v.to_vec());
        }
        out
    }
}

pub fn slot_name(prefix: &str, slot: u16) -> String {
    format!("{prefix}{slot:02}.dat")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub date: [u16; 5],
    /// Variables 1..3: the two icon numbers and the date colour index.
    pub vars: [u16; 3],
    pub image: u16,
    pub text: Vec<u8>,
}
impl Header {
    pub fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < HEADER {
            return None;
        }
        let w = |i: usize| u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        let t = &b[22..HEADER];
        let text = t[..t.iter().position(|&c| c == 0).unwrap_or(t.len())].to_vec();
        Some(Self { date: [w(0), w(1), w(2), w(3), w(4)], vars: [w(5), w(6), w(7)], image: w(10), text })
    }
}

/// sub_433240: the slot bytes of the current snapshots.
pub fn encode_slot(mem: &Memory, date: [u16; 5], page_text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SLOT_SIZE);
    for w in date {
        out.extend(w.to_le_bytes());
    }
    // Variables 1..3 of the message snapshot (dword_4ED29C).
    for v in &mem.message_vars[1..4] {
        out.extend(v.to_le_bytes());
    }
    out.extend([0u8; 4]);
    out.extend(snapshot_image(mem).to_le_bytes());
    // Page text: up to 80 bytes, never splitting a double-byte character.
    let mut i = 0;
    while i < page_text.len() && page_text[i] != 0 {
        let step = if is_lead(page_text[i]) && i + 1 < page_text.len() { 2 } else { 1 };
        if i + step > 80 {
            break;
        }
        i += step;
    }
    out.extend(&page_text[..i]);
    out.resize(HEADER, 0);
    out.extend(&mem.message_scene);
    for v in &mem.message_vars {
        out.extend(v.to_le_bytes());
    }
    out.extend(&mem.previous_scene);
    for v in &mem.previous_vars {
        out.extend(v.to_le_bytes());
    }
    out
}

/// sub_432FB0: loads a slot into the live scene and the snapshots.
pub fn decode_slot(mem: &mut Memory, b: &[u8]) -> Result<()> {
    ensure!(b.len() >= SLOT_SIZE, "RScript slot is {} bytes, expected {SLOT_SIZE}", b.len());
    let mut at = HEADER;
    let mut take = |n: usize| {
        let s = &b[at..at + n];
        at += n;
        s
    };
    let scene = take(SCENE_SIZE).to_vec();
    let vars: Vec<i16> = take(GAME_VARIABLES * 2).chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    let previous = take(SCENE_SIZE).to_vec();
    let pvars: Vec<i16> = take(GAME_VARIABLES * 2).chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    mem.scene.copy_from_slice(&scene);
    mem.vars[..GAME_VARIABLES].copy_from_slice(&vars);
    mem.previous_scene.copy_from_slice(&previous);
    mem.previous_vars.copy_from_slice(&pvars);
    Ok(())
}

/// sub_433180: `<prefix>.dat`.
pub fn encode_system(mem: &Memory, read: &BitSets, seen: &BitSets) -> Vec<u8> {
    let mut out = mem.config[..CONFIG_SIZE].to_vec();
    for v in &mem.vars[GAME_VARIABLES..] {
        out.extend(v.to_le_bytes());
    }
    read.encode(&mut out);
    seen.encode(&mut out);
    out
}
/// sub_432D60 with an existing file: the bytes it holds replace the
/// configuration and variables 7000..9999, then the two maps are read.
pub fn decode_system(mem: &mut Memory, b: &[u8]) -> (BitSets, BitSets) {
    let n = b.len().min(CONFIG_SIZE);
    mem.config[..n].copy_from_slice(&b[..n]);
    let vars = b.get(CONFIG_SIZE..).unwrap_or_default();
    for (i, c) in vars.chunks_exact(2).take(crate::memory::VARIABLES - GAME_VARIABLES).enumerate() {
        mem.vars[GAME_VARIABLES + i] = i16::from_le_bytes([c[0], c[1]]);
    }
    let mut at = CONFIG_SIZE + (crate::memory::VARIABLES - GAME_VARIABLES) * 2;
    let read = BitSets::decode(b, &mut at);
    let seen = BitSets::decode(b, &mut at);
    (read, seen)
}

/// Backdrop image of the message snapshot (word_4F7516, layer 0 image).
pub fn snapshot_image(mem: &Memory) -> u16 {
    let at = crate::memory::scene::LAYERS + crate::memory::layer::IMAGE;
    u16::from_le_bytes([mem.message_scene[at], mem.message_scene[at + 1]])
}

/// Save-mount path of a native save file (`<save dir>\<name>`).
pub fn save_path(apini: &Apini, name: &str) -> Result<VfsPath> {
    let dir = crate::res::key(&apini.save_dir).unwrap_or_default();
    let rel = if dir.is_empty() { name.to_owned() } else { format!("{dir}/{name}") };
    Ok(VfsPath::new(Mount::Save, &rel)?)
}

/// One `CFileBtn` (sub_40C040): an invisible hit area the size of
/// `saveconf.lwg` at the `savescrn` layer `0`..`9`, inverted while hovered
/// (blend 0x67 on enter, 0x17 on leave: sub_40C980, sub_40CA30).
#[derive(Default)]
pub struct SlotView {
    pub at: Point,
    pub header: Option<Header>,
    pub icon1: Option<Content>,
    pub icon2: Option<Content>,
    pub thumb: Option<Content>,
    pub hovered: bool,
}

/// `saveconf.lwg` placements: `icon` (default 59,0), `date` (110,40),
/// `new`, `thmb` and the page `text` field.
#[derive(Default)]
pub struct SlotLayout {
    pub size: [u32; 2],
    pub icon_at: Point,
    pub date_at: Point,
    pub new_marker: Option<(Content, Point)>,
    pub thumb_at: Option<Point>,
    pub text: Option<(Point, [u32; 2])>,
    /// `thmb` layer size: the captured frame is shrunk to it (dword_50A764).
    pub thumb_size: Option<[u32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Slot(usize),
    Exit,
    Next,
    Prev,
    Tab(usize),
}

/// A native message box (sub_450650, MB_OKCANCEL | MB_ICONQUESTION) and
/// what its OK continues with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ask {
    Overwrite(u16),
    Load(u16),
    QuickSave,
    QuickLoad,
    Title,
}

pub enum Pending {
    /// A question is open.
    Ask(Ask),
    /// The local time is being sampled for a slot header (slot, from the screen).
    Stamp(u16, bool, Box<dyn CivilTimeQuery>),
}

pub const DIALOG: u32 = 0x5A11;

/// `CSave` in one mode (save: `bg_save`, load: `bg_load`).
pub struct SaveScreen {
    pub save_mode: bool,
    pub bg: Sprite,
    pub slots: Vec<SlotView>,
    pub layout: SlotLayout,
    pub exit: Option<LwgButton>,
    pub next: Option<LwgButton>,
    pub prev: Option<LwgButton>,
    /// Page buttons `01`..`10`; the current page's is pressed and inert.
    pub tabs: Vec<LwgButton>,
    pub number_at: Option<Point>,
    /// `nonbl` layers `1`..`10` (page digits).
    pub digits: Vec<Option<(Content, Point)>>,
    pub hovered: Option<Target>,
    /// Frame capture for the thumbnail, taken before the screen is drawn.
    pub capture: Option<crysirror_core::image_processing::ImageJob>,
    pub thumbnail: Option<(u32, u32, Vec<u8>)>,
}

impl SaveScreen {
    /// sub_41D230 + sub_41E250.
    pub fn open(cx: &mut Ctx, save_mode: bool) -> Option<Self> {
        let sys = cx.apini.dir(crate::apini::Dir::System).to_owned();
        let scrn = join(&sys, "savescrn");
        let conf = join(&sys, "saveconf");
        let screen = cx.res.lwg_of(&scrn)?.clone();
        let mut bg = Sprite::default();
        if let Some((c, at)) = cx.lwg_layer(&scrn, if save_mode { "bg_save" } else { "bg_load" }) {
            bg = Sprite { content: Some(c), pos: at, visible: true, ..Default::default() };
        }
        let at_of = |l: &crate::res::Lwg, name: &str| l.layers.iter().find(|x| x.name == name).map(|x| pt(x.x, x.y));
        let slots = (0..10).map(|i| SlotView { at: at_of(&screen, &i.to_string()).unwrap_or_default(), ..Default::default() }).collect();
        let mut layout = SlotLayout { icon_at: pt(59, 0), date_at: pt(110, 40), ..Default::default() };
        if let Some(c) = cx.res.lwg_of(&conf).cloned() {
            layout.size = c.canvas;
            layout.icon_at = at_of(&c, "icon").unwrap_or(layout.icon_at);
            layout.date_at = at_of(&c, "date").unwrap_or(layout.date_at);
            layout.thumb_at = at_of(&c, "thmb");
            if let Some(l) = c.layers.iter().find(|x| x.name == "thmb") {
                layout.thumb_size = l.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a));
            }
            if let Some(at) = at_of(&c, "new") {
                layout.new_marker = cx.image(&join(&sys, "dat_new")).map(|m| (m, at));
            }
            if let Some(l) = c.layers.iter().find(|x| x.name == "text") {
                let size = l.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([300, 30]);
                layout.text = Some((pt(l.x, l.y), size));
            }
        } else {
            cx.res.diag(format!("missing save slot layout {conf}.lwg"));
        }
        let tabs = (1..=10u16).filter_map(|n| LwgButton::load(cx, &scrn, &format!("{n:02}"), n - 1)).collect();
        let nonbl = join(&sys, "nonbl");
        let digits = (1..=10).map(|n| cx.lwg_layer(&nonbl, &n.to_string())).collect();
        Some(Self {
            save_mode,
            bg,
            slots,
            layout,
            exit: LwgButton::load(cx, &scrn, "exit", 0),
            next: LwgButton::load(cx, &scrn, "next", 0),
            prev: LwgButton::load(cx, &scrn, "prev", 0),
            tabs,
            number_at: at_of(&screen, "number"),
            digits,
            hovered: None,
            capture: None,
            thumbnail: None,
        })
    }
    /// sub_41E050: page arrows and page buttons for `page`.
    pub fn arrange(&mut self, page: u16, pages: u16) {
        if let Some(b) = &mut self.next {
            b.face.visible = i32::from(page) <= i32::from(pages) - 2;
        }
        if let Some(b) = &mut self.prev {
            b.face.visible = page != 0;
        }
        for b in &mut self.tabs {
            b.face.visible = true;
            if b.id == page {
                b.press();
                b.face.interactive = false;
            } else {
                b.hover(false);
                b.face.interactive = true;
            }
        }
    }
    pub fn slot_rect(&self, i: usize) -> [i32; 4] {
        let at = self.slots[i].at;
        [at.x, at.y, self.layout.size[0] as i32, self.layout.size[1] as i32]
    }
    pub fn hit(&self, x: i32, y: i32) -> Option<Target> {
        let on = |b: &Option<LwgButton>| b.as_ref().is_some_and(|b| b.face.contains(x, y));
        if on(&self.exit) {
            return Some(Target::Exit);
        }
        if on(&self.next) {
            return Some(Target::Next);
        }
        if on(&self.prev) {
            return Some(Target::Prev);
        }
        if let Some(i) = self.tabs.iter().position(|b| b.face.interactive && b.face.contains(x, y)) {
            return Some(Target::Tab(i));
        }
        (0..self.slots.len()).find(|&i| {
            let [sx, sy, w, h] = self.slot_rect(i);
            x >= sx && y >= sy && x < sx + w && y < sy + h
        })
        .map(Target::Slot)
    }
    pub fn set_hover(&mut self, target: Option<Target>) {
        let set = |t: Target, on: bool, s: &mut Self| match t {
            Target::Slot(i) => s.slots[i].hovered = on,
            Target::Exit => {
                if let Some(b) = &mut s.exit {
                    b.hover(on)
                }
            }
            Target::Next => {
                if let Some(b) = &mut s.next {
                    b.hover(on)
                }
            }
            Target::Prev => {
                if let Some(b) = &mut s.prev {
                    b.hover(on)
                }
            }
            Target::Tab(i) => s.tabs[i].hover(on),
        };
        if let Some(old) = self.hovered.take() {
            set(old, false, self);
        }
        if let Some(t) = target {
            set(t, true, self);
        }
        self.hovered = target;
    }
}

/// `YYYY/MM/DD hh:mm` of a slot header.
pub fn date_text(d: [u16; 5]) -> Vec<u8> {
    format!("{:04}/{:02}/{:02} {:02}:{:02}", d[0], d[1], d[2], d[3], d[4]).into_bytes()
}

impl Vm {
    fn page(&self) -> u16 {
        self.mem.cfg(C::SAVE_PAGE) as u16
    }
    /// sub_430FB0: hides the message window, captures the frame for the
    /// thumbnail (sub_462870), darkens the scene unless the APINI keeps it
    /// (sub_445A50(80)) and opens the screen; the script thread waits until
    /// it closes (sub_4415F0).
    pub(crate) fn open_save_screen(&mut self, h: &mut dyn HostServices, save_mode: bool) -> Result<bool> {
        let screen = {
            let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
            SaveScreen::open(&mut cx, save_mode)
        };
        let Some(mut screen) = screen else { return Ok(false) };
        if screen.layout.thumb_size.is_some() {
            match h.begin_frame_capture(self.client) {
                Ok(job) => screen.capture = Some(job),
                Err(e) => self.res.diag(format!("save thumbnail frame capture: {e}")),
            }
        }
        self.save_screen = Some(screen);
        self.refresh_slots(h)?;
        self.dirty = true;
        Ok(true)
    }
    /// sub_40CB60 for each slot of the current page.
    pub(crate) fn refresh_slots(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let page = self.page();
        let pages = self.apini.save_pages.max(1);
        let storage = h.storage(self.client)?;
        let Some(mut screen) = self.save_screen.take() else { return Ok(()) };
        screen.arrange(page, pages);
        let sys = self.apini.dir(Dir::System).to_owned();
        for i in 0..screen.slots.len() {
            let number = page * 10 + i as u16 + 1;
            let name = slot_name(&self.apini.save_prefix, number);
            let header = save_path(&self.apini, &name).ok().and_then(|p| storage.read(&p).ok()).and_then(|b| Header::parse(&b));
            let slot = &mut screen.slots[i];
            slot.thumb = None;
            slot.icon1 = None;
            slot.icon2 = None;
            if let Some(hd) = &header {
                let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
                slot.icon1 = cx.image(&join(&sys, &format!("dt1_{:04}", hd.vars[0])));
                slot.icon2 = cx.image(&join(&sys, &format!("dt2_{:04}", hd.vars[1])));
                if screen.layout.thumb_at.is_some() {
                    slot.thumb = if self.apini.thumb_by_image && hd.image != 0 && hd.image < self.apini.thumb_image_limit {
                        cx.image(&join(&format!("{sys}\\thmb"), &format!("thmb{:04}", hd.image)))
                    } else {
                        let file = format!("{}{number:02}.wcg", self.apini.save_prefix);
                        match save_path(&self.apini, &file).ok().and_then(|p| storage.read(&p).ok()) {
                            Some(bytes) => {
                                self.next_image += 1;
                                let up = h.upload_encoded_image(self.client, self.next_image, &file, &bytes)?;
                                Some(Content::Image { asset: up.path.relative().to_owned(), size: up.size })
                            }
                            None => None,
                        }
                    };
                }
            }
            slot.header = header;
        }
        self.save_screen = Some(screen);
        self.dirty = true;
        Ok(())
    }
    /// Opens a native question (sub_450650).
    pub(crate) fn ask(&mut self, h: &mut dyn HostServices, ask: Ask) -> Result<()> {
        let message = match ask {
            Ask::Overwrite(_) => OVERWRITE_TEXT,
            Ask::Load(_) => LOAD_TEXT,
            Ask::QuickSave => QUICK_SAVE_TEXT,
            Ask::QuickLoad => QUICK_LOAD_TEXT,
            Ask::Title => TITLE_TEXT,
        };
        let dialog = json!({"id":DIALOG,"title":CONFIRM_CAPTION,"message":message,
            "buttons":[{"id":1,"label":"OK"},{"id":2,"label":"キャンセル"}],"default":1,"cancel":2,"font":self.ui_font});
        h.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"ui":[{"op":"open_dialog","dialog":dialog}]}))?)?;
        self.pending = Some(Pending::Ask(ask));
        self.dirty = true;
        Ok(())
    }
    /// Polls an open question or a header time sample; true while one is
    /// pending (all other input waits).
    pub(crate) fn poll_pending(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        match self.pending.take() {
            None => Ok(false),
            Some(Pending::Ask(ask)) => {
                let bytes = h.query_engine(self.client, EngineQuery::DialogEvents)?;
                let events: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap_or_default();
                let answer = events.iter().find(|e| e["kind"] == "dialog_result" && e["widget"]["id"].as_u64() == Some(u64::from(DIALOG)));
                let Some(answer) = answer else {
                    self.pending = Some(Pending::Ask(ask));
                    return Ok(true);
                };
                let ok = answer["value"].as_f64() == Some(1.0);
                h.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"ui":[{"op":"close_dialog","id":DIALOG}]}))?)?;
                self.dirty = true;
                if !ok {
                    return Ok(true);
                }
                match ask {
                    Ask::Overwrite(slot) => self.pending = Some(Pending::Stamp(slot, true, h.begin_civil_time(self.client)?)),
                    // sub_42EE10: slot 0, no thumbnail, the last slot kept.
                    Ask::QuickSave => self.pending = Some(Pending::Stamp(0, false, h.begin_civil_time(self.client)?)),
                    Ask::Load(slot) => self.load_slot(h, slot, true)?,
                    Ask::QuickLoad => self.load_slot(h, 0, false)?,
                    Ask::Title => self.return_to_title(h)?,
                }
                Ok(true)
            }
            Some(Pending::Stamp(slot, screen, mut query)) => {
                match query.poll()? {
                    CivilTimeState::Pending => self.pending = Some(Pending::Stamp(slot, screen, query)),
                    CivilTimeState::Failed(e) => bail!("RScript save {slot}: local time is unavailable ({e})"),
                    CivilTimeState::Ready(t) => {
                        let date = [t.year as u16, u16::from(t.month), u16::from(t.day), u16::from(t.hour), u16::from(t.minute)];
                        self.write_slot(h, slot, date, screen)?;
                    }
                }
                Ok(true)
            }
        }
    }
    /// Input while the save screen is open.
    pub(crate) fn save_screen_input(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let input = self.input;
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let page = self.page();
        let Some(screen) = self.save_screen.as_mut() else { return Ok(()) };
        if let Some(job) = screen.capture.as_mut() {
            let failed = match job.poll() {
                Ok(ImageProcessState::Pending) => return Ok(()),
                Ok(ImageProcessState::Ready(image)) => {
                    let [w, hgt] = image.size();
                    let [tw, th] = screen.layout.thumb_size.unwrap_or([w, hgt]);
                    // sub_462870: whole percentages of the screen size.
                    let (px, py) = ((100 * tw / w.max(1)) as u16, (100 * th / hgt.max(1)) as u16);
                    screen.thumbnail = Some(crysirror_image::wcg_encode::percent_shrink(w, hgt, &image.rgba(), px, py)?);
                    None
                }
                Ok(ImageProcessState::Failed(e)) => Some(e.to_string()),
                Err(e) => Some(e.to_string()),
            };
            screen.capture = None;
            self.dirty = true;
            if let Some(e) = failed {
                self.res.diag(format!("save thumbnail frame capture: {e}"));
            }
            return Ok(());
        }
        let (x, y) = (input.pointer_x, input.pointer_y);
        let hit = screen.hit(x, y);
        if hit != screen.hovered {
            screen.set_hover(hit);
            self.dirty = true;
        }
        if pressed(buttons::MOUSE_RIGHT) || pressed(buttons::ESCAPE) {
            return self.close_save_screen(h);
        }
        if !pressed(buttons::MOUSE_LEFT) {
            return Ok(());
        }
        let save_mode = screen.save_mode;
        let tab_page = match hit {
            Some(Target::Tab(i)) => screen.tabs[i].id,
            _ => 0,
        };
        let exists: Vec<bool> = screen.slots.iter().map(|s| s.header.is_some()).collect();
        let exists = |i: usize| exists[i];
        match hit {
            Some(Target::Exit) => self.close_save_screen(h)?,
            // sub_41DF50 / sub_41E1A0 / sub_41E020.
            Some(Target::Next) if page != 9 => self.turn_page(h, page + 1)?,
            Some(Target::Prev) if page != 0 => self.turn_page(h, page - 1)?,
            Some(Target::Tab(_)) => self.turn_page(h, tab_page)?,
            Some(Target::Slot(i)) if save_mode => {
                // sub_4304C0: an existing slot asks before it is replaced.
                self.system_sound(h, 1)?;
                let slot = page * 10 + i as u16 + 1;
                if exists(i) {
                    self.ask(h, Ask::Overwrite(slot))?;
                } else {
                    self.pending = Some(Pending::Stamp(slot, true, h.begin_civil_time(self.client)?));
                }
            }
            // sub_42FBF0: only an existing slot loads, after the question.
            Some(Target::Slot(i)) if exists(i) => {
                let slot = page * 10 + i as u16 + 1;
                self.ask(h, Ask::Load(slot))?;
            }
            _ => {}
        }
        Ok(())
    }
    fn turn_page(&mut self, h: &mut dyn HostServices, page: u16) -> Result<()> {
        self.mem.set_cfg(C::SAVE_PAGE, i32::from(page));
        self.system_sound(h, 1)?;
        self.refresh_slots(h)
    }
    /// sub_433240 (+ sub_4304C0 for screen saves: last slot, thumbnail).
    fn write_slot(&mut self, h: &mut dyn HostServices, slot: u16, date: [u16; 5], screen: bool) -> Result<()> {
        if screen {
            self.mem.set_cfg(C::SAVE_SLOT, i32::from(slot));
        }
        let bytes = encode_slot(&self.mem, date, &self.message.page_text);
        let storage = h.storage(self.client)?;
        storage.write_atomic(&save_path(&self.apini, &slot_name(&self.apini.save_prefix, slot))?, &bytes)?;
        let image = snapshot_image(&self.mem);
        let by_image = self.apini.thumb_by_image && image != 0 && image < self.apini.thumb_image_limit;
        if screen && !by_image {
            if let Some((w, hgt, rgba)) = self.save_screen.as_ref().and_then(|s| s.thumbnail.as_ref()) {
                let wcg = crysirror_image::wcg_encode::encode_wcg(*w, *hgt, rgba)?;
                let file = format!("{}{slot:02}.wcg", self.apini.save_prefix);
                storage.write_atomic(&save_path(&self.apini, &file)?, &wcg)?;
            }
        }
        self.write_system(h)?;
        self.refresh_slots(h)
    }
    /// sub_433180.
    pub(crate) fn write_system(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let system = save_path(&self.apini, &format!("{}.dat", self.apini.save_prefix))?;
        h.storage(self.client)?.write_atomic(&system, &encode_system(&self.mem, &self.read_text, &self.seen_images))?;
        Ok(())
    }
    /// sub_432D60 at start-up: an absent file keeps the defaults.
    pub(crate) fn read_system(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let system = save_path(&self.apini, &format!("{}.dat", self.apini.save_prefix))?;
        let storage = h.storage(self.client)?;
        if let Ok(bytes) = storage.read(&system) {
            self.mem.config.fill(0);
            self.mem.vars[GAME_VARIABLES..].fill(0);
            let (read, seen) = decode_system(&mut self.mem, &bytes);
            self.read_text = read;
            self.seen_images = seen;
        }
        Ok(())
    }
    /// sub_432FB0 then the game state is entered again (0x410): the screen
    /// load plays the decision sound (sub_4336B0(1)).
    pub(crate) fn load_slot(&mut self, h: &mut dyn HostServices, slot: u16, sound: bool) -> Result<()> {
        let path = save_path(&self.apini, &slot_name(&self.apini.save_prefix, slot))?;
        let Ok(bytes) = h.storage(self.client)?.read(&path) else { return Ok(()) };
        // The slot is validated before the live state changes.
        let mut mem = self.mem.clone();
        decode_slot(&mut mem, &bytes)?;
        self.mem = mem;
        if sound {
            self.system_sound(h, 1)?;
        }
        self.reenter_game(h)
    }
    /// The quick-load button (sub_42EDB0): slot 0 must exist.
    pub(crate) fn quick_load(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let path = save_path(&self.apini, &slot_name(&self.apini.save_prefix, 0))?;
        if h.storage(self.client)?.read(&path).is_ok() {
            self.ask(h, Ask::QuickLoad)?;
        }
        Ok(())
    }
    pub(crate) fn close_save_screen(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.save_screen.take().is_some() && matches!(self.pending, Some(Pending::Ask(_))) {
            h.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"ui":[{"op":"close_dialog","id":DIALOG}]}))?)?;
            self.pending = None;
        }
        self.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slot_header_and_snapshots_round_trip() {
        let mut mem = Memory::default();
        mem.message_vars[1] = 1011;
        mem.message_vars[2] = 1012;
        mem.message_vars[3] = 2;
        let at = crate::memory::scene::LAYERS + crate::memory::layer::IMAGE;
        mem.message_scene[at..at + 2].copy_from_slice(&42u16.to_le_bytes());
        mem.message_scene[100] = 7;
        // 79 ASCII bytes then a double-byte character: the pair is cut.
        let mut text = vec![b'a'; 79];
        text.extend([0x82, 0xA0]);
        let bytes = encode_slot(&mem, [2026, 11, 17, 9, 5], &text);
        assert_eq!(bytes.len(), SLOT_SIZE);
        let h = Header::parse(&bytes).unwrap();
        assert_eq!((h.date, h.vars, h.image, h.text.len()), ([2026, 11, 17, 9, 5], [1011, 1012, 2], 42, 79));
        let mut back = Memory::default();
        decode_slot(&mut back, &bytes).unwrap();
        assert_eq!(back.scene[100], 7);
        assert_eq!(&back.vars[1..4], &[1011, 1012, 2]);
        assert_eq!(slot_name("SS_T_save", 3), "SS_T_save03.dat");
    }
    #[test]
    fn system_save_keeps_config_variables_and_bit_sets() {
        let mut mem = Memory::default();
        mem.config[0x0A] = 200;
        mem.vars[7000] = 3;
        mem.vars[9999] = -1;
        let mut read = BitSets::default();
        read.set(12, 0);
        read.set(12, 17);
        let mut seen = BitSets::default();
        seen.set(0, 999);
        let bytes = encode_system(&mem, &read, &seen);
        assert_eq!(bytes.len(), CONFIG_SIZE + 6000 + (4 + 6 + 3) + (4 + 6 + 125));
        let mut back = Memory::default();
        let (r, s) = decode_system(&mut back, &bytes);
        assert_eq!((back.config[0x0A], back.vars[7000], back.vars[9999]), (200, 3, -1));
        assert!(r.get(12, 17) && r.get(12, 0) && !r.get(12, 1) && !r.get(13, 0));
        assert!(s.get(0, 999) && !s.get(0, 998));
        assert_eq!((r, s), (read, seen));
    }
}
