// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native screens pushed above the title or the running script.
//!
//! - `CDIBSaveLoadScreen` (0x40CC50..): SAVELOAD.MSD screen 4. Controls:
//!   index 0 cancel, 1..=5 pages, 6..=15 the page's ten slots (`Move`
//!   0x40F7C0); positions keys 50+i, cells 100+i, ids 10+i (hit colours).
//!   Page buttons are off on the current page (`OnCtrlPage` 0x40F510);
//!   in load mode a slot is on only when its file exists
//!   (`UpdateSlotCaption` 0x40EC70). Captions: `No.n` (key 240), the
//!   subtitle (241), `YYYY/MM/DD hh:mm` of the file time (243), the
//!   thumbnail (242: x, y, w, h); per-state ARGB colours: text 220..224,
//!   thumbnail tint 225..229, shadow 230..234. A slot asks `CDIBConfScreen`
//!   (type 2 save, 3 load, `OnCtrlSlot` 0x40EB40); OK saves and refreshes,
//!   or loads (`Reaction` 0x40F570). Cancel returns (`OnCtrlCancel`).
//! - `CDIBConfScreen` (0x401000..): CONF0n over the caller (type 0 exit,
//!   1 to title, 2 save, 3 load, 4 to title from a replay), parts CONF_P,
//!   OK (index 0) / Cancel (1). Its MSD button records use screen 0 while
//!   `SetupStatic` 0x4010A0 reads screen 1, so the `.data` defaults apply:
//!   positions (240,280)/(420,280), cells 1..5 / 6..10 (0x491758, 0x491768).
use super::panel::{Control, Panel};
use super::slots::{SlotFile, read_slot};
use super::*;
use crysirror_core::image_processing::{ImageJob, ImageProcessState};

const SL_ID: u32 = 6000;
const CONF_ID: u32 = 7000;
const SLOTS: usize = 10;
const PAGES: usize = 5;

pub(super) enum NativeScreen {
    SaveLoad(Box<SaveLoad>),
    Conf(Conf),
    Option(Box<super::option::OptionScreen>),
    Omake(Box<super::omake::Omake>),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ConfFor {
    Exit,
    ToTitle,
    Save(u32),
    Load(u32),
}
pub(super) struct Conf {
    kind: i32,
    what: ConfFor,
    panel: Panel,
    shown: bool,
}
#[derive(Clone)]
struct Caption {
    pos: [i32; 2],
    shadow: [i32; 2],
    size: i32,
}
pub(super) struct SaveLoad {
    save: bool,
    page: usize,
    panel: Panel,
    background: String,
    files: Vec<Option<SlotFile>>,
    read: Option<Task<Vec<SlotFile>>>,
    thumbs: Vec<Option<(ImageJob, usize)>>,
    published: Vec<Option<String>>,
    number: Caption,
    subtitle: Caption,
    date: Caption,
    thumb: [i32; 4],
    text: [[u8; 4]; 5],
    tint: [[u8; 4]; 5],
    shadow: [[u8; 4]; 5],
    dirty: bool,
}

fn argb(v: &[i32]) -> [u8; 4] {
    let c = |i: usize| v.get(i).copied().unwrap_or(255).clamp(0, 255) as u8;
    [c(1), c(2), c(3), c(0)]
}

impl NSystemVm {
    fn setup_ints(&self, script: &Script, screen: i32, key: i32) -> Result<Vec<i32>> {
        script
            .setup(screen, key)
            .with_context(|| format!("N-System setup {screen}/{key}"))?
            .iter()
            .filter(|o| !o.is_string())
            .map(|o| self.value(o))
            .collect()
    }
    fn setup_text(&self, script: &Script, screen: i32, key: i32) -> Result<String> {
        let o = script.setup(screen, key).and_then(|o| o.first()).with_context(|| format!("N-System setup {screen}/{key}"))?;
        Ok(utf8(&self.string(o)?).trim_end_matches(".MGD").to_owned())
    }
    pub(super) fn open_conf(&mut self, kind: i32, what: ConfFor) -> Result<()> {
        let mut panel = Panel::new("CONF_P", CONF_ID + 1, 7001);
        for (pos, first) in [([240, 280], 1), ([420, 280], 6)] {
            panel.controls.push(Control { pos, cells: [first, first + 1, first + 2, first + 3, first + 4], on: true, visible: true, alt: None, toggled: false, slider: None });
        }
        panel.focus = Some(1);
        self.screens.push(NativeScreen::Conf(Conf { kind, what, panel, shown: false }));
        Ok(())
    }
    pub(super) fn open_save_load(&mut self, save: bool) -> Result<()> {
        let script = self.scripts.get("saveload").cloned().context("N-System SAVELOAD.MSD")?;
        let sheet = self.setup_text(&script, 4, 2)?;
        let panel = Panel::from_setup(self, &script, 4, 6 + SLOTS, 50, 100, &sheet, SL_ID + 1, 6001)?;
        let cap = |key: i32| -> Result<Caption> {
            let v = self.setup_ints(&script, 4, key)?;
            ensure!(v.len() >= 6, "SAVELOAD caption {key}");
            Ok(Caption { pos: [v[0], v[1]], shadow: [v[2], v[3]], size: v[4] })
        };
        let colors = |base: i32| -> Result<[[u8; 4]; 5]> {
            let mut out = [[255; 4]; 5];
            for (i, o) in out.iter_mut().enumerate() {
                *o = argb(&self.setup_ints(&script, 4, base + i as i32)?);
            }
            Ok(out)
        };
        let thumb = self.setup_ints(&script, 4, 242)?;
        ensure!(thumb.len() == 4, "SAVELOAD thumbnail rect");
        let sl = SaveLoad {
            save,
            page: 0,
            panel,
            background: self.setup_text(&script, 4, if save { 0 } else { 1 })?,
            files: vec![None; SLOTS],
            read: None,
            thumbs: (0..SLOTS).map(|_| None).collect(),
            published: vec![None; SLOTS],
            number: cap(240)?,
            subtitle: cap(241)?,
            date: cap(243)?,
            thumb: [thumb[0], thumb[1], thumb[2], thumb[3]],
            text: colors(220)?,
            tint: colors(225)?,
            shadow: colors(230)?,
            dirty: true,
        };
        self.screens.push(NativeScreen::SaveLoad(Box::new(sl)));
        Ok(())
    }
    fn begin_page_read(&mut self, h: &mut dyn HostServices, page: usize) -> Result<Task<Vec<SlotFile>>> {
        let storage = h.storage(self.client)?;
        let title = self.title_cp932.clone();
        let pool = self.pool.as_ref().context("N-System task pool")?;
        Ok(pool.spawn(move |cancel| {
            (0..SLOTS)
                .map(|i| {
                    cancel.check()?;
                    read_slot(&storage, (page * SLOTS + i) as u32, &title)
                })
                .collect()
        })?)
    }
    fn local_date(&self, ms: i64) -> Option<String> {
        let zone = self.timezone.as_ref()?;
        let t = zone.crt_local_time(self.client, ms.div_euclid(1000))?;
        Some(format!("{:04}/{:02}/{:02} {:02}:{:02}", t.year, t.month, t.day, t.hour, t.minute))
    }
    fn save_load_layers(&mut self, h: &mut dyn HostServices, sl: &mut SaveLoad) -> Result<Vec<Json>> {
        let mut out = vec![{
            let asset = self.asset("mgd", &sl.background, "mgd")?;
            let size = h.image_size(self.client, &asset)?;
            json!({"op":"put","layer":{"id":SL_ID,"order":[6000,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":asset}}})
        }];
        out.extend(sl.panel.layers(self, h)?);
        for j in 0..SLOTS {
            let c = &sl.panel.controls[6 + j];
            let state = match (c.on, sl.panel.focus == Some(6 + j)) {
                (true, false) => 0,
                (true, true) => 1,
                (false, false) => 3,
                (false, true) => 4,
            };
            let base = SL_ID + 100 + 8 * j as u32;
            let text = |id: u32, at: Caption, s: String, out: &mut Vec<Json>, font: &str| {
                for (k, (d, color)) in [(at.shadow, sl.shadow[state]), ([0, 0], sl.text[state])].into_iter().enumerate() {
                    if k == 0 && color[3] == 0 {
                        out.push(json!({"op":"remove","id":id}));
                        continue;
                    }
                    out.push(json!({"op":"put","layer":{"id":id + k as u32,"order":[6002,k as i32],
                        "rect":[c.pos[0] + at.pos[0] + d[0], c.pos[1] + at.pos[1] + d[1], 360, at.size + 8],
                        "paint":{"kind":"text","text":s,"font":font,"size":at.size,"color":color,"layout":UNWRAPPED_TEXT_LAYOUT}}}));
                }
            };
            let n = sl.page * SLOTS + j + 1;
            let font = self.config.font.clone();
            text(base, sl.number.clone(), format!("No.{n}"), &mut out, &font);
            let file = sl.files[j].as_ref();
            match file.and_then(|f| f.slot.as_ref()) {
                Some(slot) => {
                    text(base + 2, sl.subtitle.clone(), utf8(&slot.subtitle), &mut out, &font);
                    let date = file.and_then(|f| f.modified_ms).and_then(|ms| self.local_date(ms)).unwrap_or_default();
                    text(base + 4, sl.date.clone(), date, &mut out, &font);
                    if let Some(path) = &sl.published[j] {
                        let [x, y, w, hh] = sl.thumb;
                        out.push(json!({"op":"put","layer":{"id":base + 6,"order":[6002,2],"rect":[c.pos[0] + x, c.pos[1] + y, w, hh],
                            "paint":{"kind":"image","asset":path,"tint":sl.tint[state]}}}));
                    }
                }
                None => {
                    for k in 2..7 {
                        out.push(json!({"op":"remove","id":base + k}));
                    }
                }
            }
        }
        Ok(out)
    }
    fn remove_screen_layers(&mut self, h: &mut dyn HostServices, from: u32, to: u32) -> Result<()> {
        let cmds: Vec<Json> = self.live.range(from..to).map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, cmds, vec![])
    }
    /// Top-screen frame; the script and title are frozen underneath.
    pub(super) fn screen_step(&mut self, h: &mut dyn HostServices, now: u64, input: InputFrame) -> Result<VmYield> {
        let Some(top) = self.screens.pop() else { return Ok(VmYield::WaitUntil(now + 16)) };
        let cancel = input.buttons_pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0;
        match top {
            NativeScreen::Omake(o) => self.omake_step(h, o, input)?,
            NativeScreen::Option(mut o) => {
                if self.option_step(h, &mut o, input)? {
                    self.screens.push(NativeScreen::Option(o));
                }
            }
            NativeScreen::Conf(mut conf) => {
                if !conf.shown {
                    let script = self.scripts.get("conf").cloned().context("N-System CONF.MSD")?;
                    let image = self.setup_text(&script, 1, conf.kind)?;
                    let asset = self.asset("mgd", &image, "mgd")?;
                    let size = h.image_size(self.client, &asset)?;
                    let mut cmds = vec![json!({"op":"put","layer":{"id":CONF_ID,"order":[7000,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":asset}}})];
                    cmds.extend(conf.panel.layers(self, h)?);
                    self.submit(h, cmds, vec![])?;
                    conf.shown = true;
                }
                let before = conf.panel.focus;
                let picked = conf.panel.step(self, input).or(cancel.then_some(1));
                if let Some(i) = picked {
                    self.remove_screen_layers(h, CONF_ID, CONF_ID + 100)?;
                    return self.conf_done(h, now, conf.what, i == 0);
                }
                if conf.panel.focus != before {
                    let cmds = conf.panel.layers(self, h)?;
                    self.submit(h, cmds, vec![])?;
                }
                self.screens.push(NativeScreen::Conf(conf));
            }
            NativeScreen::SaveLoad(mut sl) => {
                if sl.read.is_none() && sl.dirty {
                    sl.read = Some(self.begin_page_read(h, sl.page)?);
                    sl.dirty = false;
                }
                if let Some(task) = &mut sl.read {
                    if let Some(files) = task.try_take()? {
                        sl.read = None;
                        for (j, f) in files.into_iter().enumerate() {
                            if let Some(t) = f.slot.as_ref().and_then(|s| s.thumbnail.clone()) {
                                sl.thumbs[j] = Some((self.begin_thumbnail(h, &t)?, j));
                            }
                            sl.files[j] = Some(f);
                        }
                        for p in 0..PAGES {
                            sl.panel.controls[1 + p].on = p != sl.page;
                        }
                        for j in 0..SLOTS {
                            sl.panel.controls[6 + j].on = sl.save || sl.files[j].as_ref().is_some_and(|f| f.slot.is_some());
                        }
                        let cmds = self.save_load_layers(h, &mut sl)?;
                        self.submit(h, cmds, vec![])?;
                    }
                }
                for j in 0..SLOTS {
                    if let Some((job, _)) = &mut sl.thumbs[j] {
                        match job.poll()? {
                            ImageProcessState::Pending => {}
                            ImageProcessState::Failed(e) => bail!("N-System slot thumbnail: {e}"),
                            ImageProcessState::Ready(image) => {
                                let id = self.next_image;
                                self.next_image += 1;
                                let up = h.publish_image_resource(self.client, id, image)?;
                                if let Some(old) = sl.published[j].replace(up.path.relative().to_owned()) {
                                    h.release_image_resource(self.client, &old)?;
                                }
                                sl.thumbs[j] = None;
                                let cmds = self.save_load_layers(h, &mut sl)?;
                                self.submit(h, cmds, vec![])?;
                            }
                        }
                    }
                }
                let before = sl.panel.focus;
                let picked = if sl.read.is_some() { None } else { sl.panel.step(self, input) };
                match picked.or(cancel.then_some(0)) {
                    Some(0) => {
                        self.close_save_load(h, &sl)?;
                        return Ok(VmYield::WaitUntil(now + 16));
                    }
                    Some(i @ 1..=5) => {
                        sl.page = i - 1;
                        sl.dirty = true;
                    }
                    Some(i) => {
                        let slot = (sl.page * SLOTS + i - 6) as u32;
                        self.screens.push(NativeScreen::SaveLoad(sl));
                        let save = matches!(self.screens.last(), Some(NativeScreen::SaveLoad(s)) if s.save);
                        self.open_conf(if save { 2 } else { 3 }, if save { ConfFor::Save(slot) } else { ConfFor::Load(slot) })?;
                        return Ok(VmYield::WaitUntil(now + 16));
                    }
                    None => {
                        if sl.panel.focus != before && sl.read.is_none() {
                            let cmds = self.save_load_layers(h, &mut sl)?;
                            self.submit(h, cmds, vec![])?;
                        }
                    }
                }
                self.screens.push(NativeScreen::SaveLoad(sl));
            }
        }
        Ok(VmYield::WaitUntil(now + 16))
    }
    fn close_save_load(&mut self, h: &mut dyn HostServices, sl: &SaveLoad) -> Result<()> {
        for p in sl.published.iter().flatten() {
            h.release_image_resource(self.client, p)?;
        }
        self.remove_screen_layers(h, SL_ID, SL_ID + 1000)
    }
    /// `CDIBConfScreen` result to its caller (`CDIBMesWnd::Reaction`
    /// 0x404950, `CDIBSaveLoadScreen::Reaction` 0x40F570, title End).
    fn conf_done(&mut self, h: &mut dyn HostServices, now: u64, what: ConfFor, ok: bool) -> Result<VmYield> {
        if !ok {
            return Ok(VmYield::WaitUntil(now + 16));
        }
        match what {
            ConfFor::Exit => {
                self.halted = true;
                return Ok(VmYield::Halted);
            }
            ConfFor::ToTitle => {
                self.close_all_screens(h)?;
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM_CHANNEL, fade_ms: 0.0 })?;
                self.clear_scene(h)?;
                self.script = None;
                self.title = Some(title::Title::menu());
            }
            ConfFor::Save(slot) => {
                self.begin_save(h, slot)?;
                if let Some(NativeScreen::SaveLoad(sl)) = self.screens.last_mut() {
                    sl.dirty = true;
                }
            }
            ConfFor::Load(slot) => self.begin_load(h, slot)?,
        }
        Ok(VmYield::WaitUntil(now + 16))
    }
    pub(super) fn close_all_screens(&mut self, h: &mut dyn HostServices) -> Result<()> {
        while let Some(s) = self.screens.pop() {
            if let NativeScreen::SaveLoad(sl) = s {
                self.close_save_load(h, &sl)?;
            }
        }
        self.remove_screen_layers(h, SL_ID, CONF_ID + 100)
    }
    pub(super) fn begin_load(&mut self, h: &mut dyn HostServices, slot: u32) -> Result<()> {
        let storage = h.storage(self.client)?;
        let title = self.title_cp932.clone();
        let pool = self.pool.as_ref().context("N-System task pool")?;
        self.load_job = Some(pool.spawn(move |_| read_slot(&storage, slot, &title))?);
        Ok(())
    }
    /// Commit a finished slot read (the live game stays until it is valid).
    pub(super) fn poll_load(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let Some(task) = &mut self.load_job else { return Ok(false) };
        let Some(file) = task.try_take()? else { return Ok(true) };
        self.load_job = None;
        let slot = file.slot.context("N-System load of an empty slot")?;
        self.close_all_screens(h)?;
        self.title = None;
        self.apply_slot(h, &slot)?;
        Ok(false)
    }
}
