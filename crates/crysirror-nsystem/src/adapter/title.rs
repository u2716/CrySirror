// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! N-System title screen (`CDIBTitleScreen`, HANANYU.EXE 0x4123C0..).
//!
//! - `SetupStatic` 0x412540 reads TITLE.MSD screen-0 records: 0 the three
//!   name images (TITLE01..03, each with an optional rule), 1 the parts
//!   sheet, 2 the hit mask, 3 the title BGM, 4 the opening movie; 10..14
//!   button ids, 20..24 positions, 30..34 state cells; 50 the name-logo zoom
//!   (x, y, grow w, grow h, ms); 66 the idle time before the movie repeats.
//! - `Move` 0x4144D0: state 0 logos (`sub_413CA0`: black → LOGO01 → LOGO02
//!   → black, 1000 ms fades, 3000 ms holds), 1 the movie (`MoveOP`), 2 the
//!   name (`MoveName`: TITLE01 → TITLE02 → TITLE03, then the logo zoom), 3
//!   the menu (`MoveMenu`; ids 0 start, 1 load, 2 option, 3 omake, 4 end).
//! - `END` re-enters with the menu already shown (`Initialize` a2 ≠ 0).
use super::*;

const BG_ID: u32 = 600;
const NAME_ID: u32 = 601;
const BUTTON_ID: u32 = 610;
const MOVIE_ID: u32 = 620;
const MEDIA_ID: u32 = 1;
/// `sub_413CA0` fade and hold times.
const LOGO_FADE_MS: u64 = 1000;
const LOGO_HOLD_MS: u64 = 3000;

#[derive(Clone, Debug)]
pub(super) struct Title {
    step: u32,
    t0: u64,
    focus: usize,
    idle: u64,
    setup: Option<Setup>,
    /// Last movie state, for diagnostics.
    pub(super) media: Option<String>,
}
#[derive(Clone, Debug)]
struct Setup {
    names: [String; 3],
    sheet: String,
    bgm: String,
    movie: String,
    buttons: Vec<(i32, [i32; 2], [i32; 5])>,
    zoom: [i32; 5],
    idle_ms: u64,
}
impl Title {
    pub(super) fn boot() -> Self {
        Self { step: 0, t0: 0, focus: 0, idle: 0, setup: None, media: None }
    }
    pub(super) fn step(&self) -> u32 {
        self.step
    }
    pub(super) fn menu() -> Self {
        Self { step: 30, ..Self::boot() }
    }
}

impl NSystemVm {
    fn title_setup(&self) -> Result<Setup> {
        let script = self.scripts.get("title").cloned().context("N-System TITLE.MSD")?;
        let ints = |key: i32, n: usize| -> Result<Vec<i32>> {
            let v: Vec<i32> = script
                .setup(0, key)
                .with_context(|| format!("TITLE setup {key}"))?
                .iter()
                .map(|o| self.value(o))
                .collect::<Result<_>>()?;
            ensure!(v.len() == n, "TITLE setup {key} takes {n} values");
            Ok(v)
        };
        let text = |key: i32, i: usize| -> Result<String> {
            let o = script.setup(0, key).and_then(|o| o.get(i)).with_context(|| format!("TITLE setup {key}"))?;
            Ok(utf8(&self.string(o)?))
        };
        let mut buttons = Vec::new();
        for i in 0..5 {
            let id = ints(10 + i, 1)?[0];
            let pos = ints(20 + i, 2)?;
            let cells = ints(30 + i, 5)?;
            buttons.push((id, [pos[0], pos[1]], [cells[0], cells[1], cells[2], cells[3], cells[4]]));
        }
        for (k, rule) in [(0, 1), (0, 3), (0, 5)] {
            ensure!(text(k, rule)?.is_empty(), "TITLE name rule images (CDIBOverlap Alpha) are not mapped");
        }
        let z = ints(50, 5)?;
        Ok(Setup {
            names: [text(0, 0)?, text(0, 2)?, text(0, 4)?],
            sheet: text(1, 0)?.trim_end_matches(".MGD").to_owned(),
            bgm: text(3, 0)?,
            movie: text(4, 0)?,
            buttons,
            zoom: [z[0], z[1], z[2], z[3], z[4]],
            idle_ms: u64::try_from(ints(66, 1)?[0]).unwrap_or(0),
        })
    }
    fn full_image(&self, h: &mut dyn HostServices, id: u32, name: &str) -> Result<Json> {
        let asset = self.asset("mgd", name, "mgd")?;
        let size = h.image_size(self.client, &asset)?;
        Ok(json!({"op":"put","layer":{"id":id,"order":[3000,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":asset}}}))
    }
    fn fade(&mut self, h: &mut dyn HostServices, mut cmds: Vec<Json>, ms: u64) -> Result<()> {
        cmds.insert(0, json!({"op":"transition","transition":{"effect":{"kind":"crossfade"},"duration_ms":ms}}));
        self.submit(h, cmds, vec![])
    }
    /// Remove every layer this client has put (screen change).
    pub(super) fn clear_scene(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let cmds: Vec<Json> = self.live.iter().map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, cmds, vec![])?;
        self.staged = vec![Layer::default(); LAYERS];
        self.shown = vec![Layer::default(); LAYERS];
        self.window.open = false;
        self.window.glyphs.clear();
        self.choices.clear();
        self.select = None;
        self.actions.clear();
        self.window.hidden = false;
        self.window.skip_mode = false;
        self.window.auto_mode = false;
        Ok(())
    }
    fn menu_layers(&self, h: &mut dyn HostServices, setup: &Setup, focus: Option<usize>) -> Result<Vec<Json>> {
        let mut out = vec![self.full_image(h, BG_ID, &setup.names[2])?];
        out.push(self.cell(h, NAME_ID, 3001, &setup.sheet, 0, [setup.zoom[0], setup.zoom[1]])?);
        for (i, (_, pos, cells)) in setup.buttons.iter().enumerate() {
            // Cell 0 normal, 1 focused (inferred from the TITLE_P sheet).
            let cell = if focus == Some(i) { cells[1] } else { cells[0] };
            out.push(self.cell(h, BUTTON_ID + i as u32, 3002, &setup.sheet, usize::try_from(cell)?, *pos)?);
        }
        Ok(out)
    }
    pub(super) fn title_step(&mut self, h: &mut dyn HostServices, now: u64, dt: u64, input: InputFrame) -> Result<VmYield> {
        let mut title = self.title.clone().context("title state")?;
        if title.setup.is_none() {
            title.setup = Some(self.title_setup()?);
        }
        let setup = title.setup.clone().unwrap();
        let click = input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0;
        let elapsed = now.saturating_sub(title.t0);
        let next = |t: &mut Title, step: u32| {
            t.step = step;
            t.t0 = now;
        };
        let mut result = VmYield::WaitUntil(now + 16);
        match title.step {
            // Logos: black, then LOGO01, LOGO02, black.
            0 => {
                let black = json!({"op":"put","layer":{"id":BG_ID,"order":[3000,0],"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"solid","color":[0,0,0,255]}}});
                self.submit(h, vec![black], vec![])?;
                let logo = self.full_image(h, BG_ID, "LOGO01")?;
                self.fade(h, vec![logo], LOGO_FADE_MS)?;
                next(&mut title, 1);
            }
            1 | 3 if elapsed >= LOGO_FADE_MS + LOGO_HOLD_MS || (elapsed >= LOGO_FADE_MS && click) => {
                let cmd = if title.step == 1 {
                    self.full_image(h, BG_ID, "LOGO02")?
                } else {
                    json!({"op":"put","layer":{"id":BG_ID,"order":[3000,0],"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"solid","color":[0,0,0,255]}}})
                };
                self.fade(h, vec![cmd], LOGO_FADE_MS)?;
                let step = title.step + 2;
                next(&mut title, step);
            }
            5 if elapsed >= LOGO_FADE_MS => next(&mut title, 10),
            // MoveOP: the opening movie; a click ends it.
            10 => {
                let asset = self.asset("data", &setup.movie.to_ascii_lowercase(), "mpg")?;
                let path = h.media_request(self.client, MediaRequest::Play { id: MEDIA_ID, asset, looping: false, audio: true, gain: 1.0 })?;
                let [w, hh] = self.config.canvas;
                self.submit(h, vec![json!({"op":"put","layer":{"id":MOVIE_ID,"order":[3500,0],"rect":[0,0,w,hh],"paint":{"kind":"image","asset":path.relative()}}})], vec![])?;
                next(&mut title, 11);
            }
            11 => {
                let info = h.media_info(self.client, MEDIA_ID)?;
                title.media = Some(format!("{:?} {}ms frames={} size={:?}", info.phase, info.position_ms, info.published_frames, info.size));
                if matches!(info.phase, MediaPhase::Ended | MediaPhase::Failed) || click {
                    h.media_request(self.client, MediaRequest::Remove { id: MEDIA_ID })?;
                    self.submit(h, vec![json!({"op":"remove","id":MOVIE_ID})], vec![])?;
                    let path = self.asset("bgm", &setup.bgm, "ogg")?;
                    self.play(h, BGM_CHANNEL, path, true)?;
                    let first = self.full_image(h, BG_ID, &setup.names[0])?;
                    self.fade(h, vec![first], 1000)?;
                    next(&mut title, 20);
                }
            }
            // MoveName: TITLE01 → TITLE02 → TITLE03, 1000 ms each.
            20 | 21 if elapsed >= 1000 => {
                let img = self.full_image(h, BG_ID, &setup.names[title.step as usize - 19])?;
                self.fade(h, vec![img], 1000)?;
                let step = title.step + 1;
                next(&mut title, step);
            }
            22 if elapsed >= 1000 => next(&mut title, 23),
            // Name logo zoom (UpdateNameVB 0x4136D0): from the enlarged,
            // transparent rect to its place over key 50's duration.
            23 => {
                let [x, y, gw, gh, ms] = setup.zoom;
                let p = (elapsed as f32 / ms.max(1) as f32).min(1.0);
                let mut put = self.cell(h, NAME_ID, 3001, &setup.sheet, 0, [0, 0])?;
                let w = put["layer"]["rect"][2].as_i64().unwrap_or(0) as f32;
                let hh = put["layer"]["rect"][3].as_i64().unwrap_or(0) as f32;
                let (gw, gh) = (gw as f32, gh as f32);
                put["layer"]["rect"] = json!([
                    x as f32 + p * gw / 2.0 - gw / 2.0,
                    y as f32 + p * gh / 2.0 - gh / 2.0,
                    gw + w - gw * p,
                    gh + hh - gh * p
                ]);
                put["layer"]["opacity"] = json!((255.0 * p) as u8);
                self.submit(h, vec![put], vec![])?;
                if p >= 1.0 {
                    next(&mut title, 30);
                }
            }
            // Menu entry (also after END).
            30 => {
                if !self.live.contains(&BG_ID) {
                    let path = self.asset("bgm", &setup.bgm, "ogg")?;
                    self.play(h, BGM_CHANNEL, path, true)?;
                }
                let cmds = self.menu_layers(h, &setup, Some(title.focus))?;
                self.submit(h, cmds, vec![])?;
                title.idle = 0;
                next(&mut title, 31);
            }
            31 => {
                title.idle += dt;
                let mut focus = title.focus;
                let mut picked = None;
                let (px, py) = (input.pointer_x, input.pointer_y);
                for (i, (_, pos, cells)) in setup.buttons.iter().enumerate() {
                    let key = format!("mgd/{}.mgd", setup.sheet.to_ascii_lowercase());
                    let Some(r) = self.regions.get(&key).and_then(|r| r.get(usize::try_from(cells[0]).ok()?)) else { continue };
                    if px >= pos[0] && py >= pos[1] && px < pos[0] + i32::from(r[2]) && py < pos[1] + i32::from(r[3]) {
                        focus = i;
                        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                            picked = Some(i);
                        }
                    }
                }
                let n = setup.buttons.len();
                if input.buttons_pressed & buttons::DOWN != 0 {
                    focus = (focus + 1) % n;
                }
                if input.buttons_pressed & buttons::UP != 0 {
                    focus = (focus + n - 1) % n;
                }
                if input.buttons_pressed & (buttons::ENTER | buttons::SPACE) != 0 {
                    picked = Some(focus);
                }
                if input.buttons_pressed != 0 {
                    title.idle = 0;
                }
                if focus != title.focus {
                    title.focus = focus;
                    let cmds = self.menu_layers(h, &setup, Some(focus))?;
                    self.submit(h, cmds, vec![])?;
                }
                // MoveMenu 0x4143E0 dispatches on the control index (vfunc 116):
                // 0 start, 1 load, 2 option, 3 omake, 4 end; key 10+i is the
                // hit-mask id, not the action.
                match picked {
                    // OnStartBtn 0x412E30: change screen to START (key 60 fades).
                    Some(0) => {
                        h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 1000.0 })?;
                        let black = json!({"op":"put","layer":{"id":MOVIE_ID,"order":[3500,0],"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"solid","color":[0,0,0,255]}}});
                        self.fade(h, vec![black], 1000)?;
                        next(&mut title, 40);
                    }
                    // OnLoadBtn 0x412FE0: the save/load screen in load mode.
                    Some(1) => self.open_save_load(false)?,
                    // OnOptionBtn 0x413210.
                    Some(2) => self.open_option()?,
                    // OnOmakeBtn 0x4130F0.
                    Some(3) => self.open_omake()?,
                    // OnEndBtn 0x413320: CONF type 0.
                    Some(4) => self.open_conf(0, super::screens::ConfFor::Exit)?,
                    _ => {}
                }
                // MoveMenu 0x4143E0: idle past key 66 replays the movie.
                if setup.idle_ms > 0 && title.idle > setup.idle_ms {
                    h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
                    next(&mut title, 10);
                }
            }
            40 if elapsed >= 1000 => {
                self.clear_scene(h)?;
                self.title = None;
                let entry = self.config.entry.clone();
                self.enter(&entry)?;
                return Ok(VmYield::WaitUntil(now + 16));
            }
            _ => result = VmYield::WaitUntil(now + 16),
        }
        self.title = Some(title);
        Ok(result)
    }
}
