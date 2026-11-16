// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! `CDIBOptionScreen` (HANANYU.EXE 0x408C60..), OPTION.MSD screen 2.
//!
//! 38 controls: positions keys 50+i, cells 90+i; page 1 (background key 0,
//! sheet key 1) holds 0..=19, page 2 (key 5, sheet key 6) 20..=37.
//! `Move` 0x40C850 dispatches by index: 0/1 window/full, 2/3 skip read/all,
//! 4 text speed, 5 auto speed, 6/7 BGM on/off, 8 BGM volume, 9/10 SE on/off,
//! 11 SE test (key 3), 12 SE volume, 13/14 voice on/off, 15 voice test
//! (key 4), 16 voice volume, 17/35 OK, 18/36 cancel, 19/37 page; 20/21
//! NonePause on/off, 22..=25 effect speed (FrameSpeed 3, 2, 1, 0; 22 also
//! NoneEffect), 26/27 BGMSmallVolume on/off, 28/29 previous/next speaker,
//! 30..=32 its R/G/B, 33/34 its voice on/off. Sliders (`CDIBScrollCtrl`,
//! 4/5/8/12/16/30/31/32) run 0..=255 with track margins from keys 170..177.
//! A selected radio is "off" (cells 3/4). OK copies the 32 settings bytes
//! and the ConfigEx block (`OnCtrlOK` 0x409A50); cancel restores the live
//! volumes (`OnCtrlCancel` 0x409AF0). Page 2 previews key 213 in the
//! speaker's colour at key 212 (rect) / 214 (size) / 215 (shadow) and the
//! NAME2 plate of the speaker at key 216; key 211 is the speaker count.
use super::panel::{Panel, Slider};
use super::*;

const OPT_ID: u32 = 6500;
const SLIDERS: [usize; 8] = [4, 5, 8, 12, 16, 30, 31, 32];

pub(super) struct OptionScreen {
    page: usize,
    panel: Panel,
    panel2: Panel,
    settings: [u8; 32],
    ex: crate::system::ConfigEx,
    original: ([u8; 32], crate::system::ConfigEx),
    speaker: usize,
    speakers: usize,
    backgrounds: [String; 2],
    names: String,
    se_test: String,
    voice_test: String,
    sample: String,
    sample_rect: [i32; 4],
    sample_size: i32,
    sample_shadow: [i32; 2],
    plate: [i32; 2],
    shown: bool,
}

fn get32(s: &[u8; 32], o: usize) -> i32 {
    i32::from_le_bytes(s[o..o + 4].try_into().unwrap())
}
fn set32(s: &mut [u8; 32], o: usize, v: i32) {
    s[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
/// Fit of the native DirectSound volume table `dword_5C8950` (0 → −100 dB,
/// 1 → −62.24 dB, 255 → −6.73 dB): mB ≈ −6224 + 2310·log10 v.
pub(super) fn volume_gain(v: i32) -> f32 {
    if v <= 0 {
        return 0.0;
    }
    let mb = -6224.0 + 2310.0 * f64::from(v.min(255)).log10();
    10f64.powf(mb / 2000.0) as f32
}

impl NSystemVm {
    pub(super) fn open_option(&mut self) -> Result<()> {
        let script = self.scripts.get("option").cloned().context("N-System OPTION.MSD")?;
        let text = |key: i32| -> Result<String> {
            let o = script.setup(2, key).and_then(|o| o.first()).with_context(|| format!("OPTION setup {key}"))?;
            Ok(utf8(&self.string(o)?).trim_end_matches(".MGD").to_owned())
        };
        let ints = |key: i32| -> Result<Vec<i32>> {
            script.setup(2, key).with_context(|| format!("OPTION setup {key}"))?.iter().map(|o| self.value(o)).collect()
        };
        let mut all = Panel::from_setup(self, &script, 2, 38, 50, 90, &text(1)?, OPT_ID + 1, 6501)?;
        for (k, i) in SLIDERS.iter().enumerate() {
            let p = ints(170 + k as i32)?;
            ensure!(p.len() == 3, "OPTION slider {i} setup");
            all.controls[*i].slider = Some(Slider { value: 0, vertical: p[0] != 0, margins: [p[1], p[2]] });
        }
        let mut panel2 = Panel::new(&text(6)?, OPT_ID + 1, 6501);
        panel2.controls = all.controls.clone();
        for (i, c) in all.controls.iter_mut().enumerate() {
            c.visible = i < 20;
        }
        for (i, c) in panel2.controls.iter_mut().enumerate() {
            c.visible = i >= 20;
        }
        let rect = ints(212)?;
        let shadow = ints(215)?;
        let plate = ints(216)?;
        ensure!(rect.len() == 4 && shadow.len() == 2 && plate.len() == 2, "OPTION preview setup");
        let mut o = OptionScreen {
            page: 0,
            panel: all,
            panel2,
            settings: self.system.settings,
            ex: self.config_ex.clone(),
            original: (self.system.settings, self.config_ex.clone()),
            speaker: 0,
            speakers: usize::try_from(ints(211)?.first().copied().unwrap_or(1)).unwrap_or(1).clamp(1, 256),
            backgrounds: [text(0)?, text(5)?],
            names: text(8)?,
            se_test: text(3)?.trim_end_matches(".OGG").to_owned(),
            voice_test: text(4)?.trim_end_matches(".OGG").to_owned(),
            sample: text(213)?,
            sample_rect: [rect[0], rect[1], rect[2], rect[3]],
            sample_size: ints(214)?.first().copied().unwrap_or(18),
            sample_shadow: [shadow[0], shadow[1]],
            plate: [plate[0], plate[1]],
            shown: false,
        };
        Self::option_states(&mut o);
        self.screens.push(screens::NativeScreen::Option(Box::new(o)));
        Ok(())
    }
    /// `sub_408EB0`: control states and slider values from the working copy.
    fn option_states(o: &mut OptionScreen) {
        let s = o.settings;
        let radio = |p: &mut Panel, on_first: bool, a: usize, b: usize| {
            p.controls[a].on = !on_first;
            p.controls[b].on = on_first;
        };
        for p in [&mut o.panel, &mut o.panel2] {
            radio(p, s[0] == 0, 0, 1);
            radio(p, get32(&s, 4) == 1, 2, 3);
            radio(p, s[16] != 0, 6, 7);
            radio(p, s[17] != 0, 9, 10);
            radio(p, s[18] != 0, 13, 14);
            p.controls[8].on = s[16] != 0;
            p.controls[12].on = s[17] != 0;
            p.controls[16].on = s[18] != 0;
            radio(p, o.ex.none_pause, 20, 21);
            let speed = if o.ex.none_effect { 3 } else { o.ex.frame_speed.clamp(0, 3) };
            for (k, i) in [22usize, 23, 24, 25].iter().enumerate() {
                p.controls[*i].on = speed != 3 - k as i32;
            }
            radio(p, o.ex.bgm_small_volume, 26, 27);
            let (voice, [_, r, g, b]) = o.ex.chars[o.speaker];
            radio(p, voice, 33, 34);
            for (i, v) in [(4, get32(&s, 8)), (5, get32(&s, 12)), (8, get32(&s, 20)), (12, get32(&s, 24)), (16, get32(&s, 28)), (30, i32::from(r)), (31, i32::from(g)), (32, i32::from(b))] {
                if let Some(sl) = &mut p.controls[i].slider {
                    sl.value = v.clamp(0, 255);
                }
            }
        }
    }
    /// The live mix (`CBGMBuffer::SetMute`/`SetVolume`, SE, voice).
    pub(super) fn apply_volumes(&mut self, h: &mut dyn HostServices, s: &[u8; 32]) -> Result<()> {
        for (group, on, vol) in [("bgm", s[16], get32(s, 20)), ("se", s[17], get32(s, 24)), ("voice", s[18], get32(s, 28))] {
            let gain = if on != 0 { volume_gain(vol) } else { 0.0 };
            h.submit_audio(self.client, AudioRequest::SetMixGroupGain { name: format!("nsystem-{group}"), gain, fade_ms: 0.0 })?;
        }
        Ok(())
    }
    fn option_layers(&mut self, h: &mut dyn HostServices, o: &OptionScreen) -> Result<Vec<Json>> {
        let bg = self.asset("mgd", &o.backgrounds[o.page], "mgd")?;
        let size = h.image_size(self.client, &bg)?;
        let mut out = vec![json!({"op":"put","layer":{"id":OPT_ID,"order":[6500,0],"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":bg}}})];
        let panel = if o.page == 0 { &o.panel } else { &o.panel2 };
        out.extend(panel.layers(self, h)?);
        if o.page == 1 {
            let (_, [a, r, g, b]) = o.ex.chars[o.speaker];
            let [x, y, w, hh] = o.sample_rect;
            let [dx, dy] = o.sample_shadow;
            out.push(json!({"op":"put","layer":{"id":OPT_ID + 300,"order":[6502,0],"rect":[x + dx, y + dy, w, hh],
                "paint":{"kind":"text","text":o.sample,"font":self.config.font,"size":o.sample_size,"color":[0,0,0,255],"layout":UNWRAPPED_TEXT_LAYOUT}}}));
            out.push(json!({"op":"put","layer":{"id":OPT_ID + 301,"order":[6502,1],"rect":[x, y, w, hh],
                "paint":{"kind":"text","text":o.sample,"font":self.config.font,"size":o.sample_size,"color":[r,g,b,a],"layout":UNWRAPPED_TEXT_LAYOUT}}}));
            out.push(self.cell(h, OPT_ID + 302, 6502, &o.names, o.speaker, o.plate)?);
        } else {
            for id in [OPT_ID + 300, OPT_ID + 301, OPT_ID + 302] {
                out.push(json!({"op":"remove","id":id}));
            }
        }
        Ok(out)
    }
    /// One frame of the option screen; false once it has closed.
    pub(super) fn option_step(&mut self, h: &mut dyn HostServices, o: &mut OptionScreen, input: InputFrame) -> Result<bool> {
        if !o.shown {
            o.shown = true;
            let cmds = self.option_layers(h, o)?;
            self.submit(h, cmds, vec![])?;
        }
        let cancel = input.buttons_pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0;
        let panel = if o.page == 0 { &mut o.panel } else { &mut o.panel2 };
        let before = panel.focus;
        let picked = panel.step(self, input);
        let value = |p: &Panel, i: usize| p.controls[i].slider.map_or(0, |s| s.value);
        let p = if o.page == 0 { o.panel.clone() } else { o.panel2.clone() };
        let mut changed = before != p.focus;
        match picked.or(cancel.then_some(18)) {
            None => {}
            Some(i) => {
                changed = true;
                let s = &mut o.settings;
                match i {
                    0 | 1 => {
                        s[0] = u8::from(i == 1);
                        // OnCtrlWndMode/OnCtrlFullMode: message 0x600 to the main window.
                        self.submit(h, vec![], vec![json!({"op":"set_fullscreen","enabled":i == 1})])?;
                    }
                    2 | 3 => set32(s, 4, i32::from(i == 2)),
                    4 => set32(s, 8, value(&p, 4)),
                    5 => set32(s, 12, value(&p, 5)),
                    6 | 7 => s[16] = u8::from(i == 6),
                    8 => set32(s, 20, value(&p, 8)),
                    9 | 10 => s[17] = u8::from(i == 9),
                    12 => set32(s, 24, value(&p, 12)),
                    13 | 14 => s[18] = u8::from(i == 13),
                    16 => set32(s, 28, value(&p, 16)),
                    11 | 15 => {
                        let (bank, name, ch) = if i == 11 { ("se", o.se_test.clone(), SE_CHANNEL + 63) } else { ("voice", o.voice_test.clone(), VOICE_CHANNEL) };
                        let path = self.asset(bank, &name, "ogg")?;
                        self.play(h, ch, path, false)?;
                    }
                    17 | 35 => {
                        self.system.settings = o.settings;
                        self.config_ex = o.ex.clone();
                        self.write_ini(h, o.speakers)?;
                        self.apply_volumes(h, &o.settings.clone())?;
                        self.option_close(h)?;
                        return Ok(false);
                    }
                    18 | 36 => {
                        let (s0, ex0) = o.original.clone();
                        self.apply_volumes(h, &s0)?;
                        self.config_ex = ex0;
                        self.option_close(h)?;
                        return Ok(false);
                    }
                    19 | 37 => o.page ^= 1,
                    20 | 21 => o.ex.none_pause = i == 20,
                    22..=25 => {
                        o.ex.frame_speed = 25 - i as i32;
                        o.ex.none_effect = i == 22;
                    }
                    26 | 27 => o.ex.bgm_small_volume = i == 26,
                    28 => o.speaker = (o.speaker + o.speakers - 1) % o.speakers,
                    29 => o.speaker = (o.speaker + 1) % o.speakers,
                    30..=32 => o.ex.chars[o.speaker].1[i - 29] = value(&p, i) as u8,
                    33 | 34 => o.ex.chars[o.speaker].0 = i == 33,
                    _ => {}
                }
                // Mute/volume changes are heard at once (OnCtrlBGMON …).
                if matches!(i, 6..=16) {
                    let s = o.settings;
                    self.apply_volumes(h, &s)?;
                }
                Self::option_states(o);
            }
        }
        if changed {
            let cmds = self.option_layers(h, o)?;
            self.submit(h, cmds, vec![])?;
        }
        Ok(true)
    }
    fn option_close(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let cmds: Vec<Json> = self.live.range(OPT_ID..OPT_ID + 500).map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, cmds, vec![])
    }
    /// Novel.ini on the save capability (native writes it beside the EXE).
    fn write_ini(&mut self, h: &mut dyn HostServices, speakers: usize) -> Result<()> {
        let bytes = encoding_rs::SHIFT_JIS.encode(&self.config_ex.to_ini(speakers)).0.into_owned();
        let storage = h.storage(self.client)?;
        let path = VfsPath::new(Mount::Save, "Novel.ini")?;
        let pool = self.pool.as_ref().context("N-System task pool")?;
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            storage.write_atomic(&path, &bytes)?;
            Ok(())
        })?;
        self.ini_write = Some(task);
        Ok(())
    }
}
