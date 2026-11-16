// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! `CDIBOmakeScreen` (HANANYU.EXE 0x4069E0..), OMAKE.MSD screen 5.
//!
//! Two modes (`ChangeMode` 0x407440): 0 CG (background key 10, parts key
//! 11), 1 scene replay (keys 20/21). 66 controls per mode, dispatched by
//! index (`Move` 0x408AF0): 0 cancel, 1..=4 BGM prev/play/stop/next, 5 mode,
//! 6..=15 pages, 16..=35 characters, 36..=65 the grid. Positions keys
//! 300+i / 400+i, cells 500+i / 600+i. Grid size keys 1100/1101, character
//! counts 1200/1201, music title position 1300/1301 (MUSIC sheet key 0,
//! track names key 1). Grid buttons take the (character, page) sheet of keys
//! 13/23 and are on when the entry's first CG/scene flag is set (key 1500
//! opens all). A CG entry runs CGVIEW (key 1400: script, count/character/
//! entry variables) with the entry's unlocked variants in image slots 0..;
//! a scene entry runs its SCENELIST script. Both set the replay flag that
//! CHKSCEXEC reads, and their END returns here (`OnCtrlSum` 0x408240).
use super::panel::Panel;
use super::*;

const OM_ID: u32 = 8000;

pub(super) struct Omake {
    mode: usize,
    character: usize,
    page: usize,
    music: usize,
    playing: bool,
    panel: Panel,
    grid: Panel,
    shown: bool,
}
pub(super) struct ReplayReturn {
    pub title: Option<title::Title>,
    pub omake: Box<Omake>,
}

impl NSystemVm {
    fn omake_script(&self) -> Result<Arc<Script>> {
        self.scripts.get("omake").cloned().context("N-System OMAKE.MSD")
    }
    fn om_ints(&self, key: i32) -> Result<Vec<i32>> {
        let s = self.omake_script()?;
        s.setup(5, key).with_context(|| format!("OMAKE setup {key}"))?.iter().filter(|o| !o.is_string()).map(|o| self.value(o)).collect()
    }
    fn om_text(&self, key: i32) -> Result<String> {
        let s = self.omake_script()?;
        let o = s.setup(5, key).and_then(|o| o.iter().find(|o| o.is_string())).with_context(|| format!("OMAKE setup {key}"))?;
        Ok(utf8(&self.string(o)?).trim_end_matches(".MGD").to_owned())
    }
    fn om_one(&self, key: i32) -> Result<i32> {
        self.om_ints(key)?.first().copied().with_context(|| format!("OMAKE setup {key}"))
    }
    fn music_list(&self) -> Result<Vec<String>> {
        let s = self.omake_script()?;
        let mut out = Vec::new();
        for c in s.commands.iter().filter(|c| c.id == 0 && c.operands.len() == 4) {
            if c.operands[0] == Operand::Int(5) && c.operands[1] == Operand::Int(1) {
                let i = usize::try_from(self.value(&c.operands[2])?)?;
                ensure!(i < 256, "OMAKE music index");
                if out.len() <= i {
                    out.resize(i + 1, String::new());
                }
                out[i] = utf8(&self.string(&c.operands[3])?);
            }
        }
        Ok(out)
    }
    /// Keys 13/23: (character, page) → thumbnail sheet.
    fn grid_sheet(&self, mode: usize, character: usize, page: usize) -> Result<Option<String>> {
        let s = self.omake_script()?;
        let key = if mode == 0 { 13 } else { 23 };
        for c in s.commands.iter().filter(|c| c.id == 0 && c.operands.len() == 5) {
            if c.operands[0] == Operand::Int(5) && c.operands[1] == Operand::Int(key) {
                let (ch, pg) = (self.value(&c.operands[2])?, self.value(&c.operands[3])?);
                if ch - 1 == character as i32 && pg - 1 == page as i32 {
                    return Ok(Some(utf8(&self.string(&c.operands[4])?).trim_end_matches(".MGD").to_owned()));
                }
            }
        }
        Ok(None)
    }
    fn entries(&self, mode: usize, character: usize) -> usize {
        if mode == 0 { self.lists.cg.iter().filter(|(p, _, _)| *p == character).count() } else { self.lists.scenes.iter().filter(|(l, _, _)| *l == character).count() }
    }
    fn entry_open(&self, mode: usize, character: usize, entry: usize) -> Result<bool> {
        if self.om_one(1500).unwrap_or(0) != 0 {
            return Ok(true);
        }
        Ok(if mode == 0 { self.system.cg.get(2000 * character + 20 * entry).is_some_and(|f| *f != 0) } else { self.system.scenes.get(20 * character + entry).is_some_and(|f| *f != 0) })
    }
    pub(super) fn open_omake(&mut self) -> Result<()> {
        let mut o = Box::new(Omake { mode: 0, character: 0, page: 0, music: 0, playing: false, panel: Panel::new("", OM_ID + 1, 8001), grid: Panel::new("", OM_ID + 101, 8002), shown: false });
        self.omake_mode(&mut o, 0)?;
        self.screens.push(screens::NativeScreen::Omake(o));
        Ok(())
    }
    /// `ChangeMode` / `ChangeChar` / `ChangePage`: rebuild control state.
    fn omake_mode(&mut self, o: &mut Omake, mode: usize) -> Result<()> {
        let script = self.omake_script()?;
        o.mode = mode;
        let chars = usize::try_from(self.om_one(1200 + mode as i32)?)?.min(20);
        if o.character >= chars {
            o.character = 0;
        }
        let base = if mode == 0 { 300 } else { 400 };
        let cells = if mode == 0 { 500 } else { 600 };
        let sheet = self.om_text(if mode == 0 { 11 } else { 21 })?;
        let mut all = Panel::from_setup(self, &script, 5, 66, base, cells, &sheet, OM_ID + 1, 8001)?;
        let grid_n = usize::try_from(self.om_one(1100 + mode as i32)?)?.min(30);
        let count = self.entries(mode, o.character);
        let pages = count.div_ceil(grid_n.max(1)).min(10);
        if o.page >= pages.max(1) {
            o.page = 0;
        }
        for (i, c) in all.controls.iter_mut().enumerate() {
            c.visible = match i {
                0..=5 => c.pos != [0, 0],
                6..=15 => i - 6 < pages,
                16..=35 => i - 16 < chars,
                _ => false,
            };
            c.on = match i {
                2 => !o.playing,
                3 => o.playing,
                6..=15 => i - 6 != o.page,
                16..=35 => i - 16 != o.character,
                _ => true,
            };
        }
        let mut grid = Panel::new("", OM_ID + 101, 8002);
        if let Some(sheet) = self.grid_sheet(mode, o.character, o.page)? {
            grid.sheet = sheet;
            for k in 0..grid_n {
                let mut c = all.controls[36 + k].clone();
                let entry = o.page * grid_n + k;
                c.visible = entry < count;
                c.on = entry < count && self.entry_open(mode, o.character, entry)?;
                grid.controls.push(c);
            }
        }
        all.controls.truncate(36);
        all.focus = o.panel.focus.filter(|f| *f < 36);
        o.panel = all;
        o.grid = grid;
        Ok(())
    }
    fn omake_layers(&mut self, h: &mut dyn HostServices, o: &Omake) -> Result<Vec<Json>> {
        let cmds: Vec<u32> = self.live.range(OM_ID..OM_ID + 400).copied().collect();
        let mut out: Vec<Json> = cmds.iter().map(|id| json!({"op":"remove","id":id})).collect();
        let bg = self.asset("mgd", &self.om_text(if o.mode == 0 { 10 } else { 20 })?, "mgd")?;
        let size = h.image_size(self.client, &bg)?;
        out.push(json!({"op":"put","layer":{"id":OM_ID,"order":[8000,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":bg}}}));
        out.extend(o.panel.layers(self, h)?);
        if !o.grid.sheet.is_empty() {
            out.extend(o.grid.layers(self, h)?);
        }
        // OnCtrlBGMNext: the track title cell of the MUSIC sheet.
        let pos = self.om_ints(1300 + o.mode as i32)?;
        if pos.len() == 2 {
            out.push(self.cell(h, OM_ID + 300, 8003, &self.om_text(0)?, o.music, [pos[0], pos[1]])?);
        }
        Ok(out)
    }
    pub(super) fn omake_step(&mut self, h: &mut dyn HostServices, mut o: Box<Omake>, input: InputFrame) -> Result<()> {
        if !o.shown {
            o.shown = true;
            let cmds = self.omake_layers(h, &o)?;
            self.submit(h, cmds, vec![])?;
        }
        let cancel = input.buttons_pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0;
        let (f0, f1) = (o.panel.focus, o.grid.focus);
        let picked = o.panel.step(self, input).or_else(|| o.grid.step(self, InputFrame { buttons_pressed: input.buttons_pressed & buttons::MOUSE_LEFT, ..input }).map(|k| 36 + k));
        let mut changed = o.panel.focus != f0 || o.grid.focus != f1;
        let music = self.music_list()?;
        match picked.or(cancel.then_some(0)) {
            None => {}
            // OnCtrlCancel 0x407F00: back to the title (music keeps playing).
            Some(0) => {
                let ids: Vec<Json> = self.live.range(OM_ID..OM_ID + 400).map(|id| json!({"op":"remove","id":id})).collect();
                self.submit(h, ids, vec![])?;
                return Ok(());
            }
            Some(i @ (1 | 4)) => {
                let n = music.len().max(1);
                o.music = if i == 1 { (o.music + n - 1) % n } else { (o.music + 1) % n };
                if o.playing {
                    let path = self.asset("bgm", &music[o.music], "ogg")?;
                    self.play(h, BGM_CHANNEL, path, true)?;
                }
                changed = true;
            }
            Some(2) => {
                o.playing = true;
                let path = self.asset("bgm", music.get(o.music).context("OMAKE music list")?, "ogg")?;
                self.play(h, BGM_CHANNEL, path, true)?;
                changed = true;
            }
            Some(3) => {
                o.playing = false;
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
                changed = true;
            }
            Some(5) => {
                let m = o.mode ^ 1;
                o.page = 0;
                self.omake_mode(&mut o, m)?;
                changed = true;
            }
            Some(i @ 6..=15) => {
                o.page = i - 6;
                let m = o.mode;
                self.omake_mode(&mut o, m)?;
                changed = true;
            }
            Some(i @ 16..=35) => {
                o.character = i - 16;
                o.page = 0;
                let m = o.mode;
                self.omake_mode(&mut o, m)?;
                changed = true;
            }
            Some(k) => {
                let grid_n = usize::try_from(self.om_one(1100 + o.mode as i32)?)?;
                let entry = o.page * grid_n + (k - 36);
                return self.omake_replay(h, o, entry);
            }
        }
        if changed {
            let m = o.mode;
            self.omake_mode(&mut o, m)?;
            let cmds = self.omake_layers(h, &o)?;
            self.submit(h, cmds, vec![])?;
        }
        self.screens.push(screens::NativeScreen::Omake(o));
        Ok(())
    }
    /// `OnCtrlSum` 0x408240.
    fn omake_replay(&mut self, h: &mut dyn HostServices, o: Box<Omake>, entry: usize) -> Result<()> {
        let script = if o.mode == 0 {
            let spec = self.omake_script()?;
            let rec = spec.setup(5, 1400).context("OMAKE setup 1400")?.to_vec();
            ensure!(rec.len() == 4, "OMAKE CG view record");
            let vars: Vec<i32> = rec[1..].iter().map(|v| self.value(v)).collect::<Result<_>>()?;
            let (_, _, names) = self.lists.cg.iter().filter(|(p, _, _)| *p == o.character).nth(entry).cloned().context("OMAKE CG entry")?;
            self.slots.clear();
            let mut n: i32 = 0;
            for (v, name) in names.iter().enumerate() {
                if self.system.cg.get(2000 * o.character + 20 * entry + v).is_some_and(|f| *f != 0) || self.om_one(1500).unwrap_or(0) != 0 {
                    self.asset("mgd", name, "mgd")?;
                    self.slots.insert(n, name.clone());
                    n += 1;
                }
            }
            self.set_var(vars[0], n)?;
            self.set_var(vars[1], o.character as i32)?;
            self.set_var(vars[2], entry as i32)?;
            utf8(&self.string(&rec[0])?)
        } else {
            self.lists.scenes.iter().filter(|(l, _, _)| *l == o.character).nth(entry).map(|(_, _, n)| n.clone()).context("OMAKE scene entry")?
        };
        let ids: Vec<Json> = self.live.range(OM_ID..OM_ID + 400).map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, ids, vec![])?;
        if !o.playing {
            h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
        }
        let title = self.title.take();
        let title_ids: Vec<Json> = self.live.range(600..700).map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, title_ids, vec![])?;
        self.replay = Some(ReplayReturn { title, omake: o });
        self.enter(&script)?;
        Ok(())
    }
    /// END during a replay: back to the omake screen over the title.
    pub(super) fn end_replay(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let Some(mut r) = self.replay.take() else { return Ok(false) };
        self.clear_scene(h)?;
        self.script = None;
        if !r.omake.playing {
            h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
        }
        self.title = r.title.map(|_| title::Title::menu());
        r.omake.shown = false;
        let m = r.omake.mode;
        self.omake_mode(&mut r.omake, m)?;
        self.screens.push(screens::NativeScreen::Omake(r.omake));
        Ok(true)
    }
}
