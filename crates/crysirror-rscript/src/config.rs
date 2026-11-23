// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The configuration screen of the Shapeshifter build (`CConfig` 0x417D30,
//! opened by sub_41B170 from the panel's menu button through sub_430EA0, by
//! button mode 1 id 4, and as the title's configuration state 0x41CFE0).
//!
//! `confscrn.lwg` layers: `bg`; buttons `save`, `load`, `exit`, `title`,
//! `close` (`_f` focus); `tab_0`..`tab_2`; radio groups (CStateBtn
//! 0x4253F0, buttons added only when their layer exists, sub_4256C0):
//! `scm_wnd/scm_ful`, `msp_now/msp_fst/msp_nom/msp_slw` and `X_off/X_on`
//! for sef, bgm, gef, msk, voc, cmp, usr, vocst, awh and bgr, then
//! `exNN_k` / `gxNN_k`; sliders (CAnSelSlide 0x4629B0) with the `X_lev`
//! track and the `X_vol` knob for bgm, voc, sef (0..255) and auto
//! (0..5000); voice-bank switches `pvNN_k` (CAnSelBtns 0x463AD0, the
//! button of the current value hidden); test buttons `bgm_test`,
//! `sef_test`, `voc_test`, `pvNN_test`; the font button `font` with its
//! name at `font_txt`, and the `fontwnd` window position.
use crate::memory::{Memory, config as C, scene as S};
use crate::res::join;
use crate::scene::Ctx;
use crate::screens::LwgButton;
use crate::sprite::{Point, Sprite, pt};
use crate::vm::{MUSIC_CHANNEL, SOUND_CHANNEL, SYSTEM_CHANNEL, VOICE_CHANNEL, Vm};
use anyhow::Result;
use crysirror_core::service::{AudioRequest, HostServices, buttons};
use serde_json::json;

/// What a radio group stores (the setter callbacks 0x41AE20..0x41A710).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Setting {
    Screen,
    Speed,
    Sound,
    Music,
    Effects,
    SkipUnread,
    Voice,
    Companion,
    User,
    VoiceContinues,
    AutoHide,
    Background,
    /// Scene word `i` (0x9942), shown while scene word `i + 10` is set.
    Extra(usize),
    /// Configuration word `i` (0x3C), shown while word `i + 10` is set.
    System(usize),
}
impl Setting {
    fn read(self, mem: &Memory) -> i32 {
        match self {
            Setting::User => i32::from(mem.vars[7000]),
            Setting::Extra(i) => i32::from(mem.w(S::WORDS + 2 * i)),
            s => i32::from(mem.cfg(s.config())),
        }
    }
    fn write(self, mem: &mut Memory, v: i32) {
        match self {
            Setting::User => mem.vars[7000] = v as i16,
            Setting::Extra(i) => mem.set_w(S::WORDS + 2 * i, v),
            s => mem.set_cfg(s.config(), v),
        }
    }
    fn config(self) -> usize {
        match self {
            Setting::Screen => C::SCREEN_MODE,
            Setting::Speed => C::MESSAGE_SPEED,
            Setting::Sound => C::SOUND_ENABLED,
            Setting::Music => C::MUSIC_ENABLED,
            Setting::Effects => C::EFFECTS_ENABLED,
            Setting::SkipUnread => C::SKIP_UNREAD,
            Setting::Voice => C::VOICE_ENABLED,
            Setting::Companion => C::COMPANION,
            Setting::VoiceContinues => C::VOICE_CONTINUES,
            Setting::AutoHide => C::AUTO_HIDE,
            Setting::Background => C::BACKGROUND_AUDIO,
            Setting::System(i) => C::SYSTEM_WORDS + 2 * i,
            Setting::User | Setting::Extra(_) => unreachable!(),
        }
    }
}

pub struct Radio {
    pub setting: Setting,
    pub buttons: Vec<LwgButton>,
    pub selected: usize,
    pub visible: bool,
}
impl Radio {
    /// sub_425A30: the selected button is disabled and shown checked.
    pub fn select(&mut self, index: usize) {
        if index >= self.buttons.len() {
            return;
        }
        self.selected = index;
        for (i, b) in self.buttons.iter_mut().enumerate() {
            b.face.visible = self.visible;
            if i == index {
                b.press();
                b.face.interactive = false;
            } else {
                b.hover(false);
                b.face.interactive = self.visible;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Volume {
    /// The panel's window slider (box back transparency).
    Window,
    Music,
    Voice,
    Sound,
    Auto,
}

/// CAnSelSlide: the knob sits at `track + value · size / max`.
pub struct Slider {
    pub volume: Volume,
    pub track: [i32; 4],
    pub knob: LwgButton,
    pub max: i32,
    pub value: i32,
    /// A track taller than wide moves the knob vertically.
    pub vertical: bool,
}
impl Slider {
    pub fn new(volume: Volume, track: [i32; 4], knob: LwgButton, max: i32) -> Self {
        Self { volume, track, knob, max, value: 0, vertical: track[3] > track[2] }
    }
    pub fn place(&mut self) {
        let [x, y, w, h] = self.track;
        let m = self.max.max(1);
        self.knob.face.pos = if self.vertical { pt(x, y + self.value * h / m) } else { pt(x + self.value * w / m, y) };
    }
    pub fn value_at(&self, px: i32, py: i32) -> i32 {
        let [x, y, w, h] = self.track;
        let v = if self.vertical { (py - y) * self.max / h.max(1) } else { (px - x) * self.max / w.max(1) };
        v.clamp(0, self.max)
    }
    pub fn hit(&self, x: i32, y: i32) -> bool {
        let [tx, ty, tw, th] = self.track;
        self.knob.face.contains(x, y) || (x >= tx && y >= ty && x < tx + tw && y < ty + th.max(1))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Save,
    Load,
    Exit,
    Title,
    Close,
    Tab(usize),
    Radio(usize, usize),
    Slider(usize),
    Bank(usize, usize),
    TestMusic,
    TestSound,
    TestVoice,
    TestBank(usize),
    Font,
    FontRow(usize),
    FontPrev,
    FontNext,
    FontExit,
}

/// CFontSelectWnd (0x40E5C0): `fontwnd.lwg` with ten `list` rows 22
/// pixels apart, `prev`, `next` and `exit`; a row selects font
/// `row + 10 · page` (sub_40F870), exit selects none (sub_40ED20).
pub struct FontWindow {
    pub origin: Point,
    pub bg: Sprite,
    pub rows: Vec<LwgButton>,
    pub caps: Vec<LwgButton>,
    pub name_at: Point,
    pub sample_at: Point,
    pub row_size: [u32; 2],
    pub prev: Option<LwgButton>,
    pub next: Option<LwgButton>,
    pub exit: Option<LwgButton>,
    pub page: usize,
    pub visible: bool,
}

pub struct ConfigScreen {
    /// Opened from the title state (save/load hidden, title/exit/close
    /// replaced by the title's own close).
    pub from_title: bool,
    pub bg: Sprite,
    pub save: Option<LwgButton>,
    pub load: Option<LwgButton>,
    pub exit: Option<LwgButton>,
    pub title: Option<LwgButton>,
    pub close: Option<LwgButton>,
    pub tabs: Vec<Option<LwgButton>>,
    pub tab: usize,
    pub radios: Vec<Radio>,
    pub sliders: Vec<Slider>,
    pub banks: Vec<Vec<LwgButton>>,
    pub bank_tests: Vec<Option<LwgButton>>,
    pub test_music: Option<LwgButton>,
    pub test_sound: Option<LwgButton>,
    pub test_voice: Option<LwgButton>,
    pub font: Option<LwgButton>,
    pub font_text_at: Option<(Point, [u32; 2])>,
    pub font_window: Option<FontWindow>,
    /// Installed (authorized) font aliases offered by the window.
    pub fonts: Vec<String>,
    pub hovered: Option<Hit>,
    pub dragging: Option<usize>,
    /// A test track or sound is playing (stopped on close, sub_41A1A0).
    pub testing_music: bool,
    pub testing_sound: bool,
    /// The tab pages `status` and `desig` (CDivParam, CDivDesig).
    pub pages: crate::status::StatusPages,
}

const RADIOS: [(&str, Setting); 12] = [
    ("scm", Setting::Screen),
    ("msp", Setting::Speed),
    ("sef", Setting::Sound),
    ("bgm", Setting::Music),
    ("gef", Setting::Effects),
    ("msk", Setting::SkipUnread),
    ("voc", Setting::Voice),
    ("cmp", Setting::Companion),
    ("usr", Setting::User),
    ("vocst", Setting::VoiceContinues),
    ("awh", Setting::AutoHide),
    ("bgr", Setting::Background),
];

impl ConfigScreen {
    pub fn open(cx: &mut Ctx, mem: &Memory, from_title: bool, save_load: bool, fonts: Vec<String>) -> Option<Self> {
        let sys = cx.apini.dir(crate::apini::Dir::System).to_owned();
        let lwg = join(&sys, "confscrn");
        let layout = cx.res.lwg_of(&lwg)?.clone();
        let layer = |name: &str| layout.layers.iter().find(|l| l.name == name);
        let mut bg = Sprite::default();
        if let Some((c, at)) = cx.lwg_layer(&lwg, "bg") {
            bg = Sprite { content: Some(c), pos: at, visible: true, ..Default::default() };
        }
        let button = |cx: &mut Ctx, name: &str, id: u16| LwgButton::load(cx, &lwg, name, id);
        let mut radios = Vec::new();
        for (name, setting) in RADIOS {
            let names: Vec<String> = match name {
                "scm" => vec!["scm_wnd".into(), "scm_ful".into()],
                "msp" => ["msp_now", "msp_fst", "msp_nom", "msp_slw"].iter().map(|s| s.to_string()).collect(),
                n => vec![format!("{n}_off"), format!("{n}_on")],
            };
            let mut buttons = Vec::new();
            for n in &names {
                // sub_425850 stops at the first missing on/off layer.
                match button(cx, n, buttons.len() as u16) {
                    Some(b) => buttons.push(b),
                    None if name != "scm" && name != "msp" => break,
                    None => {}
                }
            }
            radios.push(Radio { setting, buttons, selected: 0, visible: true });
        }
        for (prefix, make) in [("ex", Setting::Extra as fn(usize) -> Setting), ("gx", Setting::System as fn(usize) -> Setting)] {
            for i in 0..10 {
                let buttons = (0..10).filter_map(|k| button(cx, &format!("{prefix}{i:02}_{k}"), 0)).enumerate().map(|(n, mut b)| {
                    b.id = n as u16;
                    b
                });
                radios.push(Radio { setting: make(i), buttons: buttons.collect(), selected: 0, visible: true });
            }
        }
        let mut sliders = Vec::new();
        for (name, volume, max) in [("bgm", Volume::Music, 255), ("voc", Volume::Voice, 255), ("sef", Volume::Sound, 255), ("auto", Volume::Auto, 5000)] {
            let Some(track) = layer(&format!("{name}_lev")) else { continue };
            let size = track.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([1, 1]);
            let Some(knob) = button(cx, &format!("{name}_vol"), 0) else { continue };
            sliders.push(Slider::new(volume, [track.x, track.y, size[0] as i32, size[1] as i32], knob, max));
        }
        let banks = (0..10).map(|i| (0..10).map_while(|k| button(cx, &format!("pv{i:02}_{k}"), k)).collect()).collect();
        let bank_tests = (0..10).map(|i| button(cx, &format!("pv{i:02}_test"), i)).collect();
        let tabs = (0..3).map(|i| button(cx, &format!("tab_{i}"), i)).collect();
        let font_text_at = layer("font_txt").map(|l| {
            let size = l.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([300, 24]);
            (pt(l.x, l.y), size)
        });
        let font_window = layer("fontwnd").map(|l| pt(l.x, l.y)).and_then(|origin| FontWindow::open(cx, &sys, origin));
        let mut s = Self {
            from_title,
            bg,
            save: button(cx, "save", 0),
            load: button(cx, "load", 0),
            exit: button(cx, "exit", 0),
            title: button(cx, "title", 0),
            close: button(cx, "close", 0),
            tabs,
            tab: 0,
            radios,
            sliders,
            banks,
            bank_tests,
            test_music: button(cx, "bgm_test", 0),
            test_sound: button(cx, "sef_test", 0),
            test_voice: button(cx, "voc_test", 0),
            font: button(cx, "font", 0),
            font_text_at,
            font_window,
            fonts,
            hovered: None,
            dragging: None,
            testing_music: false,
            testing_sound: false,
            pages: crate::status::StatusPages::open(cx),
        };
        s.arrange(mem, save_load);
        Some(s)
    }
    /// sub_41B170: buttons by origin, every control from the state.
    pub fn arrange(&mut self, mem: &Memory, save_load: bool) {
        let from_title = self.from_title;
        for b in [&mut self.save, &mut self.load].into_iter().flatten() {
            b.face.visible = !from_title && save_load;
            b.face.interactive = b.face.visible;
        }
        for b in [&mut self.exit, &mut self.title, &mut self.close].into_iter().flatten() {
            b.face.visible = !from_title;
            b.face.interactive = !from_title;
        }
        for r in &mut self.radios {
            r.visible = match r.setting {
                Setting::Extra(i) => mem.w(S::WORDS + 20 + 2 * i) != 0,
                Setting::System(i) => mem.cfg(C::SYSTEM_WORDS + 20 + 2 * i) != 0,
                _ => true,
            };
            let v = r.setting.read(mem).max(0) as usize;
            r.selected = v;
            let visible = r.visible;
            r.select(v);
            if !visible {
                for b in &mut r.buttons {
                    b.face.visible = false;
                    b.face.interactive = false;
                }
            }
        }
        for (i, group) in self.banks.iter_mut().enumerate() {
            let v = mem.cfg(C::VOICE_BANKS + 2 * i).max(0) as usize;
            set_bank(group, v);
        }
        // The tests are disabled while the scene's music or loop plays.
        let music = mem.uw(S::MUSIC) != 0 && !from_title;
        let sound = mem.uw(S::SOUND_CHANNELS + 2) != 0 && !from_title;
        if let Some(b) = &mut self.test_music {
            b.face.interactive = !music;
        }
        if let Some(b) = &mut self.test_sound {
            b.face.interactive = !sound;
        }
        for s in &mut self.sliders {
            s.value = match s.volume {
                Volume::Music => i32::from(mem.cfg(C::MUSIC_VOLUME)),
                Volume::Voice => i32::from(mem.cfg(C::VOICE_VOLUME)),
                Volume::Sound => i32::from(mem.cfg(C::SOUND_VOLUME)),
                Volume::Auto | Volume::Window => i32::from(mem.cfg(C::AUTO_WAIT)),
            }
            .clamp(0, s.max);
            s.place();
        }
        if let Some(w) = &mut self.font_window {
            w.visible = false;
        }
        self.set_tab(0);
    }
    /// sub_41A2B0: tab 0 shows the main page, 1 the status and 2 the
    /// designation page.
    pub fn set_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.pages.show(tab);
    }
    pub fn main_page(&self) -> bool {
        self.tab == 0
    }
    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        if let Some(w) = self.font_window.as_ref().filter(|w| w.visible) {
            let (lx, ly) = (x - w.origin.x, y - w.origin.y);
            let on = |b: &Option<LwgButton>| b.as_ref().is_some_and(|b| b.face.interactive && b.face.contains(lx, ly));
            if on(&w.prev) {
                return Some(Hit::FontPrev);
            }
            if on(&w.next) {
                return Some(Hit::FontNext);
            }
            if on(&w.exit) {
                return Some(Hit::FontExit);
            }
            return w.rows.iter().position(|b| b.face.visible && b.face.contains(lx, ly)).map(Hit::FontRow);
        }
        let on = |b: &Option<LwgButton>| b.as_ref().is_some_and(|b| b.face.interactive && b.face.contains(x, y));
        for (b, hit) in [(&self.save, Hit::Save), (&self.load, Hit::Load), (&self.exit, Hit::Exit), (&self.title, Hit::Title), (&self.close, Hit::Close)] {
            if on(b) {
                return Some(hit);
            }
        }
        if let Some(i) = self.tabs.iter().position(on) {
            return Some(Hit::Tab(i));
        }
        if !self.main_page() {
            return None;
        }
        for (i, r) in self.radios.iter().enumerate() {
            if let Some(k) = r.buttons.iter().position(|b| b.face.interactive && b.face.contains(x, y)) {
                return Some(Hit::Radio(i, k));
            }
        }
        if let Some(i) = self.sliders.iter().position(|s| s.hit(x, y)) {
            return Some(Hit::Slider(i));
        }
        for (i, g) in self.banks.iter().enumerate() {
            if let Some(k) = g.iter().position(|b| b.face.interactive && b.face.contains(x, y)) {
                return Some(Hit::Bank(i, k));
            }
        }
        if let Some(i) = self.bank_tests.iter().position(on) {
            return Some(Hit::TestBank(i));
        }
        for (b, hit) in [(&self.test_music, Hit::TestMusic), (&self.test_sound, Hit::TestSound), (&self.test_voice, Hit::TestVoice), (&self.font, Hit::Font)] {
            if on(b) {
                return Some(hit);
            }
        }
        None
    }
    fn button_mut(&mut self, hit: Hit) -> Option<&mut LwgButton> {
        match hit {
            Hit::Save => self.save.as_mut(),
            Hit::Load => self.load.as_mut(),
            Hit::Exit => self.exit.as_mut(),
            Hit::Title => self.title.as_mut(),
            Hit::Close => self.close.as_mut(),
            Hit::Tab(i) => self.tabs[i].as_mut(),
            Hit::Radio(i, k) => self.radios[i].buttons.get_mut(k),
            Hit::Slider(i) => Some(&mut self.sliders[i].knob),
            Hit::Bank(i, k) => self.banks[i].get_mut(k),
            Hit::TestMusic => self.test_music.as_mut(),
            Hit::TestSound => self.test_sound.as_mut(),
            Hit::TestVoice => self.test_voice.as_mut(),
            Hit::TestBank(i) => self.bank_tests[i].as_mut(),
            Hit::Font => self.font.as_mut(),
            Hit::FontRow(i) => self.font_window.as_mut().and_then(|w| w.rows.get_mut(i)),
            Hit::FontPrev => self.font_window.as_mut().and_then(|w| w.prev.as_mut()),
            Hit::FontNext => self.font_window.as_mut().and_then(|w| w.next.as_mut()),
            Hit::FontExit => self.font_window.as_mut().and_then(|w| w.exit.as_mut()),
        }
    }
    pub fn set_hover(&mut self, hit: Option<Hit>) {
        if let Some(old) = self.hovered.take() {
            if let Some(b) = self.button_mut(old) {
                if b.face.interactive {
                    b.hover(false);
                }
            }
            if let Hit::FontRow(i) = old {
                if let Some(c) = self.font_window.as_mut().and_then(|w| w.caps.get_mut(i)) {
                    c.hover(false);
                }
            }
        }
        if let Some(h) = hit {
            if let Some(b) = self.button_mut(h) {
                b.hover(true);
            }
            if let Hit::FontRow(i) = h {
                if let Some(c) = self.font_window.as_mut().and_then(|w| w.caps.get_mut(i)) {
                    c.hover(true);
                }
            }
        }
        self.hovered = hit;
    }
}

/// CAnSelBtns mode 2 (sub_463850): the button of the value is hidden.
fn set_bank(group: &mut [LwgButton], value: usize) {
    for (k, b) in group.iter_mut().enumerate() {
        b.hover(false);
        b.face.visible = k != value;
        b.face.interactive = k != value;
    }
}

impl FontWindow {
    fn open(cx: &mut Ctx, sys: &str, origin: Point) -> Option<Self> {
        let lwg = join(sys, "fontwnd");
        let layout = cx.res.lwg_of(&lwg)?.clone();
        let at = |name: &str| layout.layers.iter().find(|l| l.name == name).map(|l| pt(l.x, l.y));
        let mut bg = Sprite::default();
        if let Some((c, p)) = cx.lwg_layer(&lwg, "bg") {
            bg = Sprite { content: Some(c), pos: p, visible: true, ..Default::default() };
        }
        let mut rows = Vec::new();
        let mut caps = Vec::new();
        let mut row_size = [0, 0];
        for i in 0..10 {
            let dy = 22 * i as i32;
            if let Some(mut b) = LwgButton::load(cx, &lwg, "list", i) {
                row_size = [b.face.width() as u32, b.face.height() as u32];
                b.face.pos.y += dy;
                rows.push(b);
            }
            if let Some(mut b) = LwgButton::load(cx, &lwg, "list_cap", i) {
                b.face.pos.y += dy;
                caps.push(b);
            }
        }
        Some(Self {
            origin,
            bg,
            rows,
            caps,
            name_at: at("list_txt").unwrap_or_default(),
            sample_at: at("list_ctx").unwrap_or_default(),
            row_size,
            prev: LwgButton::load(cx, &lwg, "prev", 0),
            next: LwgButton::load(cx, &lwg, "next", 0),
            exit: LwgButton::load(cx, &lwg, "exit", 0),
            page: 0,
            visible: false,
        })
    }
    /// sub_40F070: the rows of the current page.
    pub fn arrange(&mut self, fonts: usize) {
        for (i, b) in self.rows.iter_mut().chain(self.caps.iter_mut()).enumerate() {
            let row = i % 10;
            let shown = self.page * 10 + row < fonts;
            b.face.visible = shown;
            b.face.interactive = shown;
        }
        let last = fonts.saturating_sub(1) / 10;
        if let Some(b) = &mut self.next {
            b.face.interactive = self.page != last;
        }
        if let Some(b) = &mut self.prev {
            b.face.interactive = self.page != 0;
        }
    }
}

/// Font face name stored in the configuration (CP932, at most 51 bytes).
pub fn font_name(mem: &Memory) -> String {
    let raw = &mem.config[C::FONT_NAME..C::FONT_NAME + 52];
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..end]).0.into_owned()
}

impl Vm {
    /// sub_430EA0 / sub_41B170: hides the message window, darkens the scene
    /// unless the APINI keeps it and shows the screen.
    pub(crate) fn open_config(&mut self, h: &mut dyn HostServices, from_title: bool) -> Result<()> {
        let fonts = h.font_library(self.client).and_then(|l| l.aliases()).unwrap_or_default();
        let save_load = self.mem.d(S::PANEL_SAVE_LOAD) != 0;
        let screen = {
            let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
            ConfigScreen::open(&mut cx, &self.mem, from_title, save_load, fonts)
        };
        match screen {
            Some(s) => self.config_screen = Some(s),
            None => self.res.diag("missing configuration screen confscrn.lwg"),
        }
        self.dirty = true;
        Ok(())
    }
    /// sub_42E980 / sub_42EA40 (and sub_41A1A0): stops the tests and closes.
    pub(crate) fn close_config(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(c) = self.config_screen.take() {
            if c.testing_music {
                h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms: 0.0 })?;
                self.music = None;
                let music = self.mem.uw(S::MUSIC);
                if music != 0 {
                    self.mem.set_w(S::MUSIC, 0);
                    self.resume_music(h, music)?;
                }
            }
            if c.testing_sound {
                h.submit_audio(self.client, AudioRequest::Stop { channel: SOUND_CHANNEL + 2, fade_ms: 0.0 })?;
            }
        }
        self.dirty = true;
        Ok(())
    }
    fn resume_music(&mut self, h: &mut dyn HostServices, track: u16) -> Result<()> {
        self.play_music_track(h, track)
    }
    /// Input while the configuration screen is open.
    pub(crate) fn config_input(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let input = self.input;
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let (x, y) = (input.pointer_x, input.pointer_y);
        let Some(c) = self.config_screen.as_mut() else { return Ok(()) };
        if let Some(i) = c.dragging {
            if input.buttons_down & buttons::MOUSE_LEFT == 0 {
                c.dragging = None;
            } else {
                let v = c.sliders[i].value_at(x, y);
                if v != c.sliders[i].value {
                    self.set_slider(h, i, v)?;
                }
                return Ok(());
            }
        }
        let c = self.config_screen.as_mut().unwrap();
        let hit = c.hit(x, y);
        if hit != c.hovered {
            c.set_hover(hit);
            self.dirty = true;
        }
        if pressed(buttons::MOUSE_RIGHT) || pressed(buttons::ESCAPE) {
            let c = self.config_screen.as_mut().unwrap();
            if let Some(w) = c.font_window.as_mut().filter(|w| w.visible) {
                w.visible = false;
                self.dirty = true;
                return Ok(());
            }
            self.system_sound(h, 1)?;
            return self.close_config(h);
        }
        if !pressed(buttons::MOUSE_LEFT) {
            return Ok(());
        }
        let sound_on = self.mem.cfg(C::SOUND_ENABLED) != 0;
        match hit {
            None => {}
            Some(Hit::Close) => {
                self.system_sound(h, 1)?;
                self.close_config(h)?;
            }
            // sub_431140 / sub_430B60: the save or load screen over the
            // configuration, which returns to it when closed.
            Some(Hit::Save) => {
                self.system_sound(h, 1)?;
                self.open_save_screen(h, true)?;
            }
            Some(Hit::Load) => {
                self.system_sound(h, 1)?;
                self.open_save_screen(h, false)?;
            }
            // sub_42EFE0: SC_CLOSE.
            Some(Hit::Exit) => {
                self.system_sound(h, 1)?;
                self.quit(h)?;
            }
            // sub_4315B0.
            Some(Hit::Title) => {
                self.system_sound(h, 1)?;
                self.ask(h, crate::save::Ask::Title)?;
            }
            Some(Hit::Tab(i)) => {
                if sound_on {
                    self.system_sound(h, 1)?;
                }
                let mem = self.mem.clone();
                let c = self.config_screen.as_mut().unwrap();
                c.set_tab(i);
                c.pages.refresh(&mem);
                self.dirty = true;
            }
            Some(Hit::Radio(i, k)) => self.set_radio(h, i, k)?,
            Some(Hit::Slider(i)) => {
                let c = self.config_screen.as_mut().unwrap();
                c.dragging = Some(i);
                let v = c.sliders[i].value_at(x, y);
                self.set_slider(h, i, v)?;
            }
            // sub_41AC70: the bank's on/off voice plays when voices are on.
            Some(Hit::Bank(i, k)) => {
                let c = self.config_screen.as_mut().unwrap();
                set_bank(&mut c.banks[i], k);
                self.mem.set_cfg(C::VOICE_BANKS + 2 * i, k as i32);
                if self.mem.cfg(C::VOICE_ENABLED) != 0 {
                    let file = if k != 0 { 9999 } else { 9998 };
                    self.play_voice_file(h, i, file)?;
                }
                self.dirty = true;
            }
            // sub_419DE0 / sub_41B000 / sub_41BD40 / sub_41AD80.
            Some(Hit::TestMusic) => {
                if self.mem.cfg(C::MUSIC_ENABLED) != 0 {
                    self.config_screen.as_mut().unwrap().testing_music = true;
                    let track = self.apini.test_music;
                    let keep = self.mem.uw(S::MUSIC);
                    self.play_music_track(h, track)?;
                    self.mem.set_w(S::MUSIC, i32::from(keep));
                }
            }
            Some(Hit::TestSound) => {
                if sound_on {
                    self.config_screen.as_mut().unwrap().testing_sound = true;
                    let sound = self.apini.test_sound;
                    self.play_sound_file(h, 2, sound)?;
                }
            }
            Some(Hit::TestVoice) => {
                let voice = self.apini.test_voice;
                self.play_voice(h, voice, 0, false, 0)?;
            }
            Some(Hit::TestBank(i)) => {
                let voice = self.apini.test_bank_voices[i];
                self.play_voice(h, voice, 0, false, 0)?;
            }
            // sub_41A470: the font window opens over the screen.
            Some(Hit::Font) => {
                let c = self.config_screen.as_mut().unwrap();
                let n = c.fonts.len();
                if let Some(w) = &mut c.font_window {
                    w.visible = true;
                    w.page = 0;
                    w.arrange(n);
                }
                self.dirty = true;
            }
            Some(Hit::FontPrev) | Some(Hit::FontNext) => {
                let c = self.config_screen.as_mut().unwrap();
                let n = c.fonts.len();
                if let Some(w) = &mut c.font_window {
                    let last = n.saturating_sub(1) / 10;
                    if hit == Some(Hit::FontPrev) && w.page != 0 {
                        w.page -= 1;
                    } else if hit == Some(Hit::FontNext) && w.page != last {
                        w.page += 1;
                    }
                    w.arrange(n);
                }
                self.dirty = true;
            }
            Some(Hit::FontExit) => self.select_font(None),
            Some(Hit::FontRow(r)) => {
                let index = self.config_screen.as_ref().and_then(|c| c.font_window.as_ref()).map_or(0, |w| w.page * 10 + r);
                self.select_font(Some(index));
            }
        }
        Ok(())
    }
    /// sub_41A4E0: the chosen face becomes the configuration font.
    fn select_font(&mut self, index: Option<usize>) {
        let Some(c) = self.config_screen.as_mut() else { return };
        if let Some(w) = &mut c.font_window {
            w.visible = false;
        }
        if let Some(name) = index.and_then(|i| c.fonts.get(i)).cloned() {
            let bytes = encoding_rs::SHIFT_JIS.encode(&name).0.into_owned();
            let n = bytes.len().min(51);
            self.mem.config[C::FONT_NAME..C::FONT_NAME + 52].fill(0);
            self.mem.config[C::FONT_NAME..C::FONT_NAME + n].copy_from_slice(&bytes[..n]);
            self.text_font = Some(name);
        }
        self.dirty = true;
    }
    /// The radio setters (0x41AE20..0x41A710).
    fn set_radio(&mut self, h: &mut dyn HostServices, i: usize, k: usize) -> Result<()> {
        let Some(c) = self.config_screen.as_mut() else { return Ok(()) };
        let r = &mut c.radios[i];
        r.select(k);
        let setting = r.setting;
        let sound_before = self.mem.cfg(C::SOUND_ENABLED) != 0;
        if sound_before && setting != Setting::Sound {
            self.system_sound(h, 1)?;
        }
        setting.write(&mut self.mem, k as i32);
        let on = k != 0;
        match setting {
            Setting::Screen => {
                h.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"ui":[{"op":"set_fullscreen","enabled":on}]}))?)?;
            }
            // sub_430C10: every box takes the speed.
            Setting::Speed => self.apply_message_speed(h),
            // sub_41BB30 + sub_431360: the scene's loops restart or stop.
            Setting::Sound => {
                if on {
                    self.system_sound(h, 1)?;
                    for ch in 0..3 {
                        let base = S::SOUND_CHANNELS + ch * S::SOUND_STRIDE;
                        let sound = i32::from(self.mem.uw(base));
                        if sound != 0 && self.mem.uw(base + 2) != 0 {
                            self.load_sound(h, ch as i32, sound)?;
                            let pan = i32::from(self.mem.w(base + 4));
                            self.play_sound(h, ch as i32, 999, 0, pan)?;
                        }
                    }
                } else {
                    for ch in 0..3 {
                        h.submit_audio(self.client, AudioRequest::Stop { channel: SOUND_CHANNEL + ch, fade_ms: 0.0 })?;
                    }
                }
            }
            // sub_419D10 + sub_42E930.
            Setting::Music => {
                let track = self.mem.uw(S::MUSIC);
                if on {
                    if track != 0 {
                        self.play_music_track(h, track)?;
                    }
                } else if self.music.take().is_some() {
                    h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms: 0.0 })?;
                    self.music_stream ^= 1;
                }
            }
            // sub_41A020 + sub_428C40: the panel follows the switch.
            Setting::Companion => {
                let m = |o: usize| crate::scene::MessageWindow::dword(&self.mem, o);
                let show = on && m(crate::memory::message::PANEL_ENABLED) != 0 && self.message.boxes[0].visible && !self.message.auto_hidden;
                self.message.panel_visible = show;
            }
            // sub_419E70: the frame window keeps audio while inactive.
            Setting::Background => self.apply_focus_audio(h)?,
            _ => {}
        }
        self.dirty = true;
        Ok(())
    }
    /// The slider callbacks (sub_419F20, sub_41BC70, sub_41AF00, sub_419BF0).
    fn set_slider(&mut self, h: &mut dyn HostServices, i: usize, v: i32) -> Result<()> {
        let Some(c) = self.config_screen.as_mut() else { return Ok(()) };
        let s = &mut c.sliders[i];
        s.value = v;
        s.place();
        let volume = s.volume;
        let (at, channels): (usize, &[u32]) = match volume {
            Volume::Music => (C::MUSIC_VOLUME, &[MUSIC_CHANNEL, MUSIC_CHANNEL + 1]),
            Volume::Voice => (C::VOICE_VOLUME, &[VOICE_CHANNEL]),
            Volume::Sound => (C::SOUND_VOLUME, &[SOUND_CHANNEL, SOUND_CHANNEL + 1, SOUND_CHANNEL + 2, SYSTEM_CHANNEL]),
            Volume::Auto | Volume::Window => (C::AUTO_WAIT, &[]),
        };
        self.mem.set_cfg(at, v);
        let gain = self.gain(at);
        for &channel in channels {
            h.submit_audio(self.client, AudioRequest::SetGain { channel, gain, fade_ms: 0.0 })?;
        }
        self.dirty = true;
        Ok(())
    }
    /// sub_42A230 for every box: the configuration speed (+70).
    pub(crate) fn apply_message_speed(&mut self, h: &mut dyn HostServices) {
        let speed = i32::from(self.mem.cfg(C::MESSAGE_SPEED));
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        for b in &mut self.message.boxes {
            self.mem.set_w(b.record + crate::memory::text_box::SPEED, speed);
            b.apply(&self.mem, &mut cx);
        }
    }
}
