// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Extra menu, CG view, scene view and music menu states.
//!
//! Extra menu (sub_42E2B0): `extrmenu.lwg` `bg`, `cg`, `sc`, `ms`, `exit`.
//!
//! CG view (sub_4319B0): `cgview.lwg` `bg`, thumbnails `thmb_0..11` with the
//! `thmbframe` frame, `page_N`, `next` (while page ≤ 4), `prev` (while page
//! ≠ 0), `exit`. Entry `page·12 + i` of the executable's table lists up to
//! six `.\grpe\NNNN` images; its thumbnail `grps\thmb\NNNN` shows once
//! variable 9000 + first image is set (sub_4317D0). A thumbnail opens the
//! first image; a click shows the next and closes after the last, a right
//! click closes (sub_431630, sub_432130).
//!
//! Scene view (sub_43FD10): `scview.lwg` with twelve thumbnails from the
//! scene table (shown once its flag variable is set, `grps\thmb\0000`
//! otherwise), the hovered entry's title in the 24-pixel `text` box
//! (sub_440750); a thumbnail replays: a new scene, variable 0 = the entry's
//! start value, script 9999 at depth 0 in replay mode (sub_440690); the
//! scene in use when the view opened comes back on exit (sub_440710).
//!
//! Music menu (sub_434730): `muscmenu.lwg` `bg`, `play`, `stop`, `pause`,
//! `exit`, `rev`, `fow`, the `level` slider with its `bar` knob and the
//! track buttons `1`..`20` with `N_p` shown while that track plays; a track
//! button selects (and restarts a playing selection), play starts
//! `Track(N)`, stop stops, pause toggles, rev/fow step (fow past the last
//! stops and selects the first), the slider sets the music volume
//! (sub_434510, sub_434450, sub_434660, sub_4343C0, sub_4344F0,
//! sub_434380, sub_434620).
use crate::config::{Slider, Volume};
use crate::extras::State;
use crate::memory::{Memory, config as C};
use crate::res::join;
use crate::scene::Ctx;
use crate::screens::LwgButton;
use crate::sprite::{Point, Sprite, pt};
use crate::text::Style;
use crate::vm::{MUSIC_CHANNEL, Vm};
use anyhow::Result;
use crysirror_core::service::{AudioRequest, HostServices, buttons};

pub enum Draw {
    Sprite(Sprite, Point),
    Text(Vec<u8>, Style, Point),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Button(usize),
    Thumb(usize),
    Page(usize),
    Track(usize),
    Slider,
}

pub struct Gallery {
    pub state: State,
    bg: Sprite,
    /// Named buttons (cg/sc/ms/exit, next/prev/exit, play/stop/...).
    buttons: Vec<(&'static str, LwgButton)>,
    pages: Vec<LwgButton>,
    thumbs: Vec<(Sprite, Option<Sprite>)>,
    page: usize,
    /// CG view: the entry and image shown full screen.
    viewing: Option<(usize, usize)>,
    viewer: Sprite,
    /// Scene view: the hovered title and the `text` box.
    title: Option<Vec<u8>>,
    text_at: Option<(Point, [u32; 2])>,
    /// Music menu.
    tracks: Vec<(LwgButton, Option<Sprite>)>,
    selected: usize,
    playing: bool,
    paused: bool,
    slider: Option<Slider>,
    hovered: Option<Hit>,
    cg: Vec<u32>,
    scenes: Vec<Vec<u16>>,
}

impl Gallery {
    pub fn open(cx: &mut Ctx, mem: &Memory, state: State) -> Option<Self> {
        let sys = cx.apini.dir(crate::apini::Dir::System).to_owned();
        let name = match state {
            State::ExtraMenu => "extrmenu",
            State::CgView => "cgview",
            State::SceneView => "scview",
            State::MusicMenu => "muscmenu",
            _ => return None,
        };
        let lwg = join(&sys, name);
        let layout = cx.res.lwg_of(&lwg)?.clone();
        let mut bg = Sprite::default();
        if let Some((c, at)) = cx.lwg_layer(&lwg, "bg") {
            bg = Sprite { content: Some(c), pos: at, visible: true, ..Default::default() };
        }
        let tables = cx.apini.gallery.clone().unwrap_or_default();
        let mut g = Gallery {
            state,
            bg,
            buttons: Vec::new(),
            pages: Vec::new(),
            thumbs: Vec::new(),
            page: 0,
            viewing: None,
            viewer: Sprite::default(),
            title: None,
            text_at: None,
            tracks: Vec::new(),
            selected: 0,
            playing: false,
            paused: false,
            slider: None,
            hovered: None,
            cg: tables.cg,
            scenes: tables.scenes,
        };
        let names: &[&'static str] = match state {
            State::ExtraMenu => &["cg", "sc", "ms", "exit"],
            State::MusicMenu => &["play", "stop", "pause", "exit", "rev", "fow"],
            _ => &["next", "prev", "exit"],
        };
        for &n in names {
            if let Some(b) = LwgButton::load(cx, &lwg, n, 0) {
                g.buttons.push((n, b));
            }
        }
        match state {
            State::CgView | State::SceneView => {
                let frame = join(&sys, "thmbframe");
                for i in 0..12 {
                    let Some(l) = layout.layers.iter().find(|l| l.name == format!("thmb_{i}")) else { continue };
                    let at = pt(l.x, l.y);
                    let border = cx.lwg_layer(&frame, "frame").map(|(c, p)| Sprite { content: Some(c), pos: pt(at.x + p.x, at.y + p.y), visible: true, ..Default::default() });
                    g.thumbs.push((Sprite { pos: at, visible: true, interactive: true, ..Default::default() }, border));
                }
                g.pages = (0..10).map_while(|k| LwgButton::load(cx, &lwg, &format!("page_{k}"), k)).collect();
                g.text_at = layout.layers.iter().find(|l| l.name == "text").map(|l| (pt(l.x, l.y), l.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([600, 40])));
                g.refresh_thumbs(cx, mem);
            }
            State::MusicMenu => {
                for n in 1..=20 {
                    let Some(b) = LwgButton::load(cx, &lwg, &n.to_string(), n - 1) else { break };
                    let playing = cx.lwg_layer(&lwg, &format!("{n}_p")).map(|(c, at)| Sprite { content: Some(c), pos: at, visible: false, ..Default::default() });
                    g.tracks.push((b, playing));
                }
                if let Some(track) = layout.layers.iter().find(|l| l.name == "level") {
                    let size = track.asset.as_ref().and_then(|a| cx.res.size(cx.h, cx.client, a)).unwrap_or([1, 1]);
                    if let Some(knob) = LwgButton::load(cx, &lwg, "bar", 0) {
                        let mut s = Slider::new(Volume::Music, [track.x, track.y, size[0] as i32, size[1] as i32], knob, 255);
                        s.value = i32::from(mem.cfg(C::MUSIC_VOLUME)).clamp(0, 255);
                        s.place();
                        g.slider = Some(s);
                    }
                }
                g.select_marks();
            }
            _ => {}
        }
        Some(g)
    }
    /// sub_4317D0 / sub_4407E0.
    fn refresh_thumbs(&mut self, cx: &mut Ctx, mem: &Memory) {
        for i in 0..self.thumbs.len() {
            let (image, shown) = if self.state == State::CgView {
                let first = self.cg.get((self.page * 12 + i) * 6).copied().unwrap_or(0);
                let seen = first != 0 && mem.vars.get(9000 + first as usize).is_some_and(|v| *v != 0);
                (if seen { Some(format!("grps\\thmb\\{first:04}")) } else { None }, first != 0 && seen)
            } else {
                let e = self.scenes.get(i).cloned().unwrap_or_default();
                let (flag, thumb) = (e.first().copied().unwrap_or(0), e.get(1).copied().unwrap_or(0));
                if flag == 0 {
                    (None, false)
                } else if mem.vars.get(usize::from(flag)).is_some_and(|v| *v != 0) {
                    (Some(format!("grps\\thmb\\{thumb:04}")), true)
                } else {
                    (Some("grps\\thmb\\0000".to_owned()), false)
                }
            };
            let content = image.and_then(|p| cx.image(&p));
            let (t, border) = &mut self.thumbs[i];
            t.visible = content.is_some();
            t.content = content;
            t.interactive = shown;
            if let Some(b) = border {
                b.visible = shown;
            }
        }
        let page = self.page;
        for (n, b) in &mut self.buttons {
            match *n {
                "next" => b.face.visible = self.state == State::CgView && page <= 4,
                "prev" => b.face.visible = self.state == State::CgView && page != 0,
                _ => {}
            }
            b.face.interactive = b.face.visible;
        }
        for p in &mut self.pages {
            p.face.visible = self.state == State::CgView;
            let current = usize::from(p.id) == page;
            p.face.interactive = p.face.visible && !current;
            if current {
                p.press();
            } else {
                p.hover(false);
            }
        }
    }
    /// The music menu's playing marks (sub_434450 / sub_434660).
    fn select_marks(&mut self) {
        for (i, (b, p)) in self.tracks.iter_mut().enumerate() {
            let on = self.playing && i == self.selected;
            b.face.visible = !on;
            if i == self.selected { b.press() } else { b.hover(false) }
            if let Some(p) = p {
                p.visible = on;
            }
        }
    }
    fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        if self.viewing.is_some() {
            return None;
        }
        if let Some(i) = self.buttons.iter().position(|(_, b)| b.face.interactive && b.face.contains(x, y)) {
            return Some(Hit::Button(i));
        }
        if let Some(i) = self.pages.iter().position(|b| b.face.interactive && b.face.contains(x, y)) {
            return Some(Hit::Page(i));
        }
        if let Some(i) = self.thumbs.iter().position(|(t, _)| t.interactive && t.contains(x, y)) {
            return Some(Hit::Thumb(i));
        }
        if let Some(i) = self.tracks.iter().position(|(b, _)| b.face.contains(x, y)) {
            return Some(Hit::Track(i));
        }
        if self.slider.as_ref().is_some_and(|s| s.hit(x, y)) {
            return Some(Hit::Slider);
        }
        None
    }
    pub fn draws(&self) -> Vec<Draw> {
        let zero = Point::default();
        let mut out = vec![Draw::Sprite(self.bg.clone(), zero)];
        if self.viewing.is_some() {
            out.push(Draw::Sprite(self.viewer.clone(), zero));
            return out;
        }
        for (t, b) in &self.thumbs {
            out.push(Draw::Sprite(t.clone(), zero));
            if let Some(b) = b {
                out.push(Draw::Sprite(b.clone(), zero));
            }
        }
        for (_, b) in &self.buttons {
            out.push(Draw::Sprite(b.face.clone(), zero));
        }
        for p in &self.pages {
            out.push(Draw::Sprite(p.face.clone(), zero));
        }
        for (b, p) in &self.tracks {
            out.push(Draw::Sprite(b.face.clone(), zero));
            if let Some(p) = p {
                out.push(Draw::Sprite(p.clone(), zero));
            }
        }
        if let Some(s) = &self.slider {
            out.push(Draw::Sprite(s.knob.face.clone(), zero));
        }
        if let (Some(t), Some((at, [w, h]))) = (&self.title, self.text_at) {
            let style = Style { size: 24, width: w as i32, height: h as i32, color: 0xFFFFFF, ..Default::default() };
            out.push(Draw::Text(t.clone(), style, at));
        }
        out
    }
}

impl Vm {
    pub(crate) fn gallery_input(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let input = self.input;
        let pressed = |b: u64| input.buttons_pressed & b != 0;
        let (x, y) = (input.pointer_x, input.pointer_y);
        let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) else { return Ok(()) };
        // CG viewing: a click steps, a right click closes (sub_432130).
        if let Some((entry, n)) = g.viewing {
            if pressed(buttons::MOUSE_LEFT) || pressed(buttons::MOUSE_RIGHT) {
                let next = g.cg.get(entry * 6 + n + 1).copied().filter(|v| *v != 0 && n + 1 < 6);
                match next.filter(|_| pressed(buttons::MOUSE_LEFT)) {
                    Some(img) => {
                        g.viewing = Some((entry, n + 1));
                        let c = self.res.image(h, self.client, &format!(".\\grpe\\{img:04}"));
                        let g = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()).unwrap();
                        g.viewer = Sprite { content: c, visible: true, ..Default::default() };
                    }
                    None => g.viewing = None,
                }
                self.begin_transition(serde_json::json!({"kind":"ordered_dither","cell":[1,1],"steps":16}), 16 * 20);
                self.dirty = true;
            }
            return Ok(());
        }
        if let Some(true) = g.slider.as_ref().map(|_| input.buttons_down & buttons::MOUSE_LEFT != 0 && g.hovered == Some(Hit::Slider)) {
            let s = g.slider.as_mut().unwrap();
            let v = s.value_at(x, y);
            if v != s.value {
                s.value = v;
                s.place();
                let gain = 10f32.powf((3000 * (v - 255) / 255).min(0) as f32 / 2000.0);
                for ch in [MUSIC_CHANNEL, MUSIC_CHANNEL + 1] {
                    h.submit_audio(self.client, AudioRequest::SetGain { channel: ch, gain, fade_ms: 0.0 })?;
                }
                self.dirty = true;
            }
            return Ok(());
        }
        let hit = g.hit(x, y);
        if hit != g.hovered {
            let set = |g: &mut Gallery, h: Hit, on: bool| match h {
                Hit::Button(i) => g.buttons[i].1.hover(on),
                Hit::Page(i) => g.pages[i].hover(on),
                _ => {}
            };
            if let Some(old) = g.hovered.take() {
                set(g, old, false);
            }
            if let Some(new) = hit {
                set(g, new, true);
            }
            // sub_440750 / sub_440980: the hovered scene's title.
            g.title = match hit {
                Some(Hit::Thumb(i)) if g.state == State::SceneView => g.scenes.get(i).map(|e| {
                    let bytes: Vec<u8> = e.iter().skip(3).flat_map(|w| w.to_le_bytes()).collect();
                    bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())].to_vec()
                }),
                _ => None,
            };
            g.hovered = hit;
            self.dirty = true;
        }
        if pressed(buttons::MOUSE_RIGHT) || pressed(buttons::ESCAPE) {
            return self.gallery_action(h, "exit");
        }
        if !pressed(buttons::MOUSE_LEFT) {
            return Ok(());
        }
        let Some(hit) = hit else { return Ok(()) };
        let g = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()).unwrap();
        match hit {
            Hit::Button(i) => {
                let name = g.buttons[i].0;
                self.gallery_action(h, name)?;
            }
            Hit::Page(i) => {
                g.page = usize::from(g.pages[i].id);
                self.system_sound(h, 1)?;
                self.refresh_gallery(h);
            }
            Hit::Thumb(i) => {
                let state = g.state;
                let entry = g.page * 12 + i;
                let first = g.cg.get(entry * 6).copied().unwrap_or(0);
                let start = g.scenes.get(i).and_then(|e| e.get(2)).copied().unwrap_or(0);
                if state == State::CgView {
                    g.viewing = Some((entry, 0));
                }
                self.system_sound(h, 1)?;
                if state == State::CgView {
                    let c = self.res.image(h, self.client, &format!(".\\grpe\\{first:04}"));
                    let g = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()).unwrap();
                    g.viewer = Sprite { content: c, visible: true, ..Default::default() };
                    self.begin_transition(serde_json::json!({"kind":"ordered_dither","cell":[1,1],"steps":16}), 16 * 20);
                } else {
                    self.replay_scene(h, start as i16)?;
                }
            }
            Hit::Track(i) => self.select_track(h, i)?,
            Hit::Slider => {}
        }
        self.dirty = true;
        Ok(())
    }
    fn refresh_gallery(&mut self, h: &mut dyn HostServices) {
        let mem = self.mem.clone();
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        if let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) {
            g.refresh_thumbs(&mut cx, &mem);
        }
        self.dirty = true;
    }
    fn gallery_action(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) else { return Ok(()) };
        let state = g.state;
        match name {
            "cg" | "sc" | "ms" => {
                self.system_sound(h, 1)?;
                let to = match name {
                    "cg" => State::CgView,
                    "sc" => State::SceneView,
                    _ => State::MusicMenu,
                };
                self.enter_state(h, to)?;
            }
            "next" | "prev" => {
                if name == "next" { g.page += 1 } else { g.page = g.page.saturating_sub(1) }
                self.system_sound(h, 1)?;
                self.refresh_gallery(h);
            }
            "play" => self.play_track(h)?,
            "stop" => self.stop_tracks(h)?,
            "pause" => {
                if g.paused {
                    g.paused = false;
                    if g.playing {
                        h.submit_audio(self.client, AudioRequest::Pause { channel: MUSIC_CHANNEL + self.music_stream, paused: false, fade_ms: 0.0 })?;
                    }
                } else if g.playing {
                    g.paused = true;
                    h.submit_audio(self.client, AudioRequest::Pause { channel: MUSIC_CHANNEL + self.music_stream, paused: true, fade_ms: 0.0 })?;
                }
            }
            "rev" => {
                if g.selected > 0 {
                    let s = g.selected - 1;
                    self.select_track(h, s)?;
                }
            }
            "fow" => {
                if g.selected + 1 >= g.tracks.len() {
                    self.stop_tracks(h)?;
                    self.select_track(h, 0)?;
                } else {
                    let s = g.selected + 1;
                    self.select_track(h, s)?;
                }
            }
            // sub_431610 / sub_440710 / sub_434360 / sub_42E290.
            _ => {
                self.system_sound(h, 1)?;
                if state == State::MusicMenu {
                    self.stop_tracks(h)?;
                }
                if state == State::SceneView {
                    if let Some(saved) = self.scene_view_saved.take() {
                        self.mem.scene.copy_from_slice(&saved);
                    }
                }
                if state == State::ExtraMenu {
                    self.leave_state(h)?;
                } else {
                    self.enter_state(h, State::ExtraMenu)?;
                }
            }
        }
        self.dirty = true;
        Ok(())
    }
    /// sub_434510.
    fn select_track(&mut self, h: &mut dyn HostServices, i: usize) -> Result<()> {
        let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) else { return Ok(()) };
        if g.playing {
            g.selected = i;
            g.playing = false;
            g.paused = false;
            return self.play_track(h);
        }
        if g.selected == i {
            return self.play_track(h);
        }
        g.selected = i;
        g.select_marks();
        Ok(())
    }
    /// sub_434450: `Track(N)` of the selection.
    fn play_track(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) else { return Ok(()) };
        if g.playing {
            return Ok(());
        }
        g.playing = true;
        g.select_marks();
        let track = g.selected as u16 + 1;
        self.play_music_track(h, track)
    }
    /// sub_434660.
    fn stop_tracks(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(g) = self.state_screen.as_mut().and_then(|s| s.gallery.as_mut()) {
            g.playing = false;
            g.paused = false;
            g.select_marks();
        }
        if self.music.take().is_some() {
            h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms: 0.0 })?;
            self.music_stream ^= 1;
        }
        Ok(())
    }
    /// sub_440690: a new scene, variable 0 = the start value, script 9999
    /// in replay mode; the end of the thread returns to the scene view.
    fn replay_scene(&mut self, h: &mut dyn HostServices, start: i16) -> Result<()> {
        self.state_screen = None;
        self.wait = None;
        self.new_game(h, 9999)?;
        self.mem.set_var(0, start);
        self.top_menu_start = true;
        self.replaying = true;
        Ok(())
    }
}
