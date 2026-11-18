// SPDX-License-Identifier: MPL-2.0
//! Registered natives (62AAA0, all 31) and the MalieSystem frame callback
//! 62CF80 (System_GetScreen(1000)). Imports a script declares but the EXE
//! does not register fail to link natively ("LINK ERROR"); calling them
//! returns 0 (67AEA0 dispatches only kinds 1 and 2).
use super::*;

const NAMES: usize = 64;
/// Root frames the EXE creates (52D470): MalieSystem 1000, capture 12346,
/// ADV 300, MESSAGEBOX 451, IME 5, FADE 302, 310 and CURSOR_SCREEN 452.
/// 4FAA30 (`System_GetScreen`) yields null for any other id.
const ROOT_FRAMES: [u32; 8] = [1000, 12346, 300, 451, 5, 302, 310, 452];
/// Screens registered for `System_ChangeScreen` (52D470 via 4FAF10).
const SCREENS: [u32; 3] = [1000, 12346, 300];
impl MalieVm {
    pub(super) fn import(&mut self, h: &mut dyn HostServices, call: &Call) -> Result<u32> {
        let machine = self.machine.as_ref().unwrap();
        let name = machine.program.functions[call.function as usize].name.clone();
        let a = call.args.clone();
        let arg = |i: usize| -> Result<u32> { a.get(i).copied().context("Malie native import arity") };
        let now = h.now_ms();
        Ok(match name.as_str() {
            "System_ChangeScreen" => {
                self.change_screen(h, arg(0)?)?;
                0
            }
            // 4FB240: capture (0/1) and release (2/3) frame 12346 for
            // screen-change overlaps; the transition itself is `_ms_overlap`.
            "System_TakeScreen" => {
                if arg(0)? == 0 && !self.skip_active {
                    self.imgblk = Some(("ol".into(), 500));
                } else if arg(0)? >= 2 {
                    if let Some((mask, time)) = self.imgblk.take() {
                        self.screen_transition(&mask, time, now);
                    }
                }
                0
            }
            // 4FAA30: the root child with that id, or null.
            "System_GetScreen" => {
                let id = arg(0)?;
                if ROOT_FRAMES.contains(&id) { id } else { 0 }
            }
            // 6124D0: 441 opens the title (613260 "title") and resumes the
            // script. Every other id runs 612580's modal loop on root child
            // `id`; only the frames above exist, and 612580 reads the frame
            // flags without a null check, so absent ids crash natively.
            "MalieSystem_OpenDialog" => {
                match arg(0)? {
                    441 => {}
                    // 612580: ChangeScreen(id) inside a screen transition with
                    // input disabled while it runs.
                    id if ROOT_FRAMES.contains(&id) => {
                        self.change_screen(h, id)?;
                        if !self.skip_active {
                            self.screen_transition("ol", 500, now);
                        }
                    }
                    id => bail!("Malie OpenDialog({id}): no root frame {id}; 612580 dereferences the null frame (native crash)"),
                }
                0
            }
            "MalieSystem_PlaySystemSE" => {
                // 627B20 indices into off_7B5344.
                const SE: [&str; 11] = ["ok", "cancel", "focus", "mouseover", "setfocus", "error", "check", "page", "start", "logo", "title"];
                if let Some(n) = SE.get(arg(0)? as usize) {
                    self.system_se(h, n)?;
                }
                0
            }
            "tag" => {
                let text = self.format(call)?;
                self.tag(h, &text)?;
                0
            }
            // 628130: elapsed BGM play time (ms).
            "getbgmtime" => {
                if self.bgm.is_some() { now.saturating_sub(self.tags.bgm_started) as u32 } else { 0 }
            }
            // 610BB0: Sakamoto day-of-week (y, m, d) -> 0..6.
            "DayOfWeek" => {
                const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
                let (y, m, d) = (arg(0)? as i32, arg(1)? as i32, arg(2)? as i32);
                ensure!((1..=12).contains(&m), "Malie DayOfWeek month");
                let y = y - i32::from(m < 3);
                ((y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d).rem_euclid(7)) as u32
            }
            // 62B140: address of a global slot.
            "toaddr" => crate::vm::GLOBAL_BASE.checked_add(arg(0)?).context("Malie toaddr")?,
            // 4EF520 records the value for quick-jump replay and returns it.
            "qj_flag" => arg(0)?,
            // Frame items are not separate objects here; 614160 yields null
            // for absent items and every FrameLayer call on null returns 0.
            "FrameLayer_GetItem" => 0,
            "FrameLayer_SetVisible" | "FrameLayer_SetFocus" | "FrameLayer_SetEnable" => 0,
            "FrameLayer_SendMessage" => {
                ensure!(a.len() == 4, "Malie FrameLayer_SendMessage arity");
                match a[0] {
                    1000 => self.system(h, a[1], a[2], a[3])?,
                    0 => 0,
                    300 => self.adv_frame(h, a[1], a[2], a[3])?,
                    // FrameLayer handles are only produced by System_GetScreen.
                    f => bail!("Malie native frame={f} message={} not connected", a[1]),
                }
            }
            "Random" => {
                let n = arg(0)?;
                self.rng = self.rng.wrapping_mul(214_013).wrapping_add(2_531_011);
                if n == 0 { 0 } else { (self.rng >> 16 & 0x7fff) % n }
            }
            "System_TimerWait" => {
                self.wait = Wait::Until(now + u64::from(arg(0)?));
                0
            }
            "strcpy" | "strcat" => {
                let m = self.machine.as_mut().unwrap();
                let src = m.string(arg(1)?)?;
                let dst = arg(0)?;
                let text = if name == "strcat" { m.string(dst)? + &src } else { src };
                m.write_string(dst, &text)?;
                dst
            }
            "sprintf" => {
                let dst = arg(0)?;
                let text = self.printf(arg(1)?, &a[2..])?;
                self.machine.as_mut().unwrap().write_string(dst, &text)?;
                text.encode_utf16().count() as u32
            }
            "sscanf" => self.scanf(arg(0)?, arg(1)?, &a[2..])?,
            "memcpy" => {
                let m = self.machine.as_mut().unwrap();
                let n = arg(2)? as usize;
                let bytes = m.read_bytes(arg(1)?, n)?;
                m.write_bytes(arg(0)?, &bytes)?;
                arg(0)?
            }
            "CSV_Open" => {
                let path = self.machine.as_ref().unwrap().string(arg(0)?)?.replace('\\', "/");
                let path = path.trim_start_matches("./").to_ascii_lowercase();
                match self.read_text(&path)? {
                    Some(text) => {
                        let rows: Vec<Vec<String>> = text.lines().map(|l| l.split(',').map(|v| v.trim().to_owned()).collect()).collect();
                        ensure!(self.csv.len() < 64, "Malie CSV handle budget");
                        self.csv.push(Some(rows));
                        self.csv.len() as u32
                    }
                    None => 0,
                }
            }
            "CSV_Close" => {
                if let Some(slot) = (arg(0)? as usize).checked_sub(1).and_then(|i| self.csv.get_mut(i)) {
                    *slot = None;
                }
                0
            }
            "CSV_GetCount" => self.csv_rows(arg(0)?)?.map_or(0, |r| r.len() as u32),
            "CSV_GetNumber" | "CSV_RefString" => {
                let (row, col) = (arg(1)? as usize, arg(2)? as usize);
                let cell = self.csv_rows(arg(0)?)?.and_then(|r| r.get(row)).and_then(|r| r.get(col)).cloned().unwrap_or_default();
                if name == "CSV_GetNumber" {
                    cell.parse::<i32>().unwrap_or(0) as u32
                } else {
                    self.machine.as_mut().unwrap().host_string(&cell)?
                }
            }
            "CSV_FindString" => {
                let key = self.machine.as_ref().unwrap().string(arg(2)?)?;
                let col = arg(1)? as usize;
                self.csv_rows(arg(0)?)?.and_then(|r| r.iter().position(|row| row.get(col).is_some_and(|c| *c == key))).map_or(u32::MAX, |i| i as u32)
            }
            "out_debug" => 0,
            // 6137D0 after a dialog: resume when it closes.
            "System_Await" => {
                if self.dialog.is_some() {
                    self.dialog_return = Wait::None;
                    self.wait = Wait::Dialog;
                }
                0
            }
            "System_GetResult" => self.result,
            _ => {
                // Not in the EXE registry: linked as LINK ERROR, returns 0.
                if self.unlinked.insert(name.clone()) {
                    log::warn!("Malie LINK ERROR : {name}");
                }
                0
            }
        })
    }
    fn csv_rows(&self, handle: u32) -> Result<Option<&Vec<Vec<String>>>> {
        Ok((handle as usize).checked_sub(1).and_then(|i| self.csv.get(i)).and_then(Option::as_ref))
    }
    /// msvcrt-compatible `swprintf` subset used by scripts: %d %i %u %x %X
    /// %c %s %% with flags `-0`, width and precision.
    pub(super) fn printf(&self, format: u32, args: &[u32]) -> Result<String> {
        let m = self.machine.as_ref().unwrap();
        let f: Vec<char> = m.string(format)?.chars().collect();
        let mut out = String::new();
        let mut args = args.iter();
        let mut i = 0;
        while i < f.len() {
            if f[i] != '%' {
                out.push(f[i]);
                i += 1;
                continue;
            }
            i += 1;
            let (mut left, mut zero) = (false, false);
            while i < f.len() && matches!(f[i], '-' | '0' | '+' | ' ' | '#') {
                left |= f[i] == '-';
                zero |= f[i] == '0';
                i += 1;
            }
            let mut width = 0usize;
            while i < f.len() && f[i].is_ascii_digit() {
                width = width * 10 + f[i].to_digit(10).unwrap() as usize;
                i += 1;
            }
            if i < f.len() && f[i] == '.' {
                i += 1;
                while i < f.len() && f[i].is_ascii_digit() {
                    i += 1;
                }
            }
            while i < f.len() && matches!(f[i], 'l' | 'h') {
                i += 1;
            }
            let conv = *f.get(i).context("Malie printf conversion")?;
            i += 1;
            let mut next = || args.next().copied().context("Malie printf arity");
            let body = match conv {
                '%' => "%".to_owned(),
                'd' | 'i' => (next()? as i32).to_string(),
                'u' => next()?.to_string(),
                'x' => format!("{:x}", next()?),
                'X' => format!("{:X}", next()?),
                'c' => char::from_u32(next()?).unwrap_or('?').to_string(),
                's' | 'S' => m.string(next()?)?,
                other => bail!("Malie printf %{other}"),
            };
            let pad = width.saturating_sub(body.chars().count());
            if left {
                out.push_str(&body);
                out.extend(std::iter::repeat_n(' ', pad));
            } else if zero && conv != 's' {
                let (sign, digits) = body.strip_prefix('-').map_or(("", body.as_str()), |d| ("-", d));
                out.push_str(sign);
                out.extend(std::iter::repeat_n('0', pad));
                out.push_str(digits);
            } else {
                out.extend(std::iter::repeat_n(' ', pad));
                out.push_str(&body);
            }
            ensure!(out.len() <= 64 * 1024, "Malie printf budget");
        }
        Ok(out)
    }
    /// `swscanf` subset: %d stores an int, %s a word; returns fields read.
    fn scanf(&mut self, input: u32, format: u32, ptrs: &[u32]) -> Result<u32> {
        let m = self.machine.as_mut().unwrap();
        let text: Vec<char> = m.string(input)?.chars().collect();
        let fmt: Vec<char> = m.string(format)?.chars().collect();
        let (mut i, mut j, mut n) = (0, 0, 0usize);
        while j < fmt.len() {
            if fmt[j] == '%' && j + 1 < fmt.len() {
                while i < text.len() && text[i].is_whitespace() {
                    i += 1;
                }
                let start = i;
                match fmt[j + 1] {
                    'd' => {
                        if i < text.len() && (text[i] == '-' || text[i] == '+') {
                            i += 1;
                        }
                        while i < text.len() && text[i].is_ascii_digit() {
                            i += 1;
                        }
                        let Ok(v) = text[start..i].iter().collect::<String>().parse::<i32>() else { break };
                        let p = *ptrs.get(n).context("Malie sscanf arity")?;
                        m.write_word(p, v as u32)?;
                    }
                    's' => {
                        while i < text.len() && !text[i].is_whitespace() {
                            i += 1;
                        }
                        if i == start {
                            break;
                        }
                        let p = *ptrs.get(n).context("Malie sscanf arity")?;
                        m.write_string(p, &text[start..i].iter().collect::<String>())?;
                    }
                    other => bail!("Malie sscanf %{other}"),
                }
                n += 1;
                j += 2;
            } else if fmt[j].is_whitespace() {
                while i < text.len() && text[i].is_whitespace() {
                    i += 1;
                }
                j += 1;
            } else {
                if text.get(i) != Some(&fmt[j]) {
                    break;
                }
                i += 1;
                j += 1;
            }
        }
        Ok(n as u32)
    }
    /// System_ChangeScreen (4FA860): only registered screens switch; any
    /// other id misses the registry loop and returns without effect.
    pub(super) fn change_screen(&mut self, h: &mut dyn HostServices, id: u32) -> Result<()> {
        if !SCREENS.contains(&id) || id == self.active_screen {
            return Ok(());
        }
        // 4FAF30 hides every registered screen and shows the selected one.
        self.active_screen = id;
        let adv = self.adv.as_mut().unwrap();
        adv.hidden = id == 1000 || self.jump.is_some();
        // 12346 displays the TakeScreen capture: the ADV picture holds.
        adv.frozen = id == 12346;
        if id == 300 {
            self.hide_screen(h)?;
        }
        Ok(())
    }
    /// 630470/630E30: start a jump with the ADV screen hidden.
    pub(super) fn start_jump(&mut self, kind: Jump) {
        self.jump = Some(kind);
        self.auto_active = false;
        self.adv.as_mut().unwrap().hidden = true;
    }
    /// 6317C0/6317A0: end the jump and show the ADV screen again.
    pub(super) fn stop_jump(&mut self) {
        if self.jump.take().is_some() {
            let shown = self.active_screen == 1000;
            self.adv.as_mut().unwrap().hidden = shown;
        }
    }
    /// GAME_SCREEN (300) message callback 52D7F0. Numbers outside its switch
    /// return 0. 1007, 1014-1016, 1024 and 5001-5013 are forwarded to the
    /// ADV_FRAME item 4; their last parameters are retained for the frame.
    fn adv_frame(&mut self, h: &mut dyn HostServices, msg: u32, w: u32, l: u32) -> Result<u32> {
        let idle = self.dialog.is_none() && self.screen.is_none();
        Ok(match msg {
            // 52D670: Layer_LoadBG with the text at `l`.
            1000 => {
                let src = if l == 0 { String::new() } else { self.machine.as_ref().unwrap().string(l)? };
                self.emit(h, format!("<layer parent='back' id='bg' src='{src}' opacity='1' time='0'>"))?;
                1
            }
            // 52D600: leave skip, else show a hidden window, else leave auto.
            2 | 1010 => {
                if self.skip_mode {
                    self.system_se(h, "cancel")?;
                    self.skip_mode = false;
                } else if self.adv.as_ref().unwrap().window_hidden {
                    self.adv.as_mut().unwrap().window_hidden = false;
                } else if self.auto_active {
                    self.system_se(h, "cancel")?;
                    self.auto_active = false;
                }
                self.sync_frame_checks();
                0
            }
            // 2001 message enter (suspend), 2002 leave (resume).
            4 => {
                match w {
                    2001 => self.wait = Wait::Halted,
                    2002 if matches!(self.wait, Wait::Halted) => self.wait = Wait::None,
                    _ => {}
                }
                0
            }
            // 632B50 command 1002: the backlog (632C10).
            1002 => {
                self.shortcut(h, "backlog")?;
                0
            }
            1005 => {
                self.wait = Wait::Halted;
                0
            }
            1007 | 1014 | 1015 | 1016 | 1024 | 5001..=5013 => {
                self.adv_frame_state.insert(msg, (w, l));
                0
            }
            // 52D710: leave skip, else show a hidden window, else release the
            // pending click wait (86C34C).
            1009 => {
                if self.skip_mode {
                    self.system_se(h, "cancel")?;
                    self.skip_mode = false;
                    self.sync_frame_checks();
                } else if self.adv.as_ref().unwrap().window_hidden {
                    self.adv.as_mut().unwrap().window_hidden = false;
                } else {
                    self.pending_click = true;
                }
                0
            }
            1019 if w == 0 && idle => {
                self.open_dialog(h, "qload")?;
                0
            }
            1020 if w == 0 && idle => {
                self.quick_save(h)?;
                0
            }
            // 1022 toggle skip mode, 1051 toggle auto mode.
            1022 if w == 0 && idle => {
                self.shortcut(h, "skip")?;
                0
            }
            1051 if w == 0 && idle => {
                if self.auto_active {
                    self.system_se(h, "cancel")?;
                } else {
                    self.system_voice(h, "auto")?;
                }
                self.shortcut(h, "auto")?;
                0
            }
            1028 | 1029 if w == 0 && idle => {
                self.auto_active = false;
                self.open_dialog(h, if msg == 1028 { "load" } else { "save" })?;
                0
            }
            // 1030/1033 post commands 6 and 9, which 632C10 logs as unknown
            // and drops.
            1050 if w == 0 => {
                self.shortcut(h, "voicerepeat")?;
                0
            }
            // 1052: toggle flag 886838 (system SE 6 first).
            1052 => {
                self.system_se(h, "cancel")?;
                self.frame_flag_886838 = !self.frame_flag_886838;
                0
            }
            // 6121F0: the "end1" prompt.
            1061 => {
                self.open_dialog(h, "end")?;
                0
            }
            1070..=1073 if w == 0 && idle => {
                self.auto_active = false;
                self.native_dialog(h, ["textconfig", "soundconfig", "keyconfig", "voiceconfig"][(msg - 1070) as usize])?;
                0
            }
            1040 if w == 0 && idle => {
                self.auto_active = false;
                self.native_dialog(h, "skill")?;
                0
            }
            // 6121A0: release the page wait (86C348).
            1001 => {
                if matches!(self.wait, Wait::Click) {
                    self.pending_click = true;
                }
                0
            }
            // 530C20 PSP_HIDE: hide the window until a click (system SE 6).
            1004 => {
                self.system_se(h, "cancel")?;
                self.adv.as_mut().unwrap().window_hidden = true;
                0
            }
            // 583C70: the WIKIDIALOG frame (wiki/wiki.svg, absent here).
            1041 => {
                self.open_dialog(h, "wikidialog")?;
                0
            }
            // 52E410: the `menu` dialog; its result is sent back here.
            1026 | 8500 if idle && (self.skip_mode || self.auto_active || matches!(self.wait, Wait::Click | Wait::Select | Wait::Suspended)) => {
                self.native_dialog(h, "menu")?;
                0
            }
            // 6313B0: return to the previous choice (snapshot stack 883B10).
            1100 if w == 0 && idle && !self.choice_history.is_empty() => {
                self.system_se(h, "ok")?;
                self.open_message_box("mb_yesno", "前の選択肢へ戻ります。\nよろしいでしょうか？", MessageBoxPurpose::ChoiceBack)?;
                0
            }
            // 631350: skip to the next choice; 631480: to the next chapter.
            1101 | 1102 if w == 0 && idle && self.jump.is_none() => {
                self.system_se(h, "ok")?;
                if msg == 1102 {
                    self.system_voice(h, "jump")?;
                    self.open_message_box("mb_yesno", "次のシーンへ進みます。\nよろしいでしょうか？", MessageBoxPurpose::NextScene)?;
                } else {
                    self.open_message_box("mb_yesno", "次の選択肢へ進みます。\nよろしいでしょうか？", MessageBoxPurpose::NextChoice)?;
                }
                0
            }
            // Wheel: outside backlog mode opens the backlog on wheel up.
            8000 | 8001 => {
                if msg == 8000 && idle {
                    self.shortcut(h, "backlog")?;
                }
                0
            }
            _ => 0,
        })
    }
    /// MalieSystem frame callback 62CF80.
    pub(super) fn system(&mut self, h: &mut dyn HostServices, msg: u32, w: u32, l: u32) -> Result<u32> {
        if self.trace {
            eprintln!("MALIE_TRACE {} system {msg} {w:#x} {l:#x}", h.now_ms());
        }
        let now = h.now_ms();
        let string = |vm: &Self, v: u32| -> Result<String> { if v == 0 { Ok(String::new()) } else { vm.machine.as_ref().unwrap().string(v) } };
        Ok(match msg {
            // 10: reset to the title (611800/676EA0 "title", 52EE60).
            10 => {
                self.return_to_title(h)?;
                0
            }
            // 6373F0: ten string registers.
            101 => {
                if w <= 9 {
                    self.string_vars[w as usize] = string(self, l)?;
                }
                0
            }
            102 => {
                self.tags.text_mode = w.min(3);
                0
            }
            // 612B30: current CG name (also marks it seen).
            103 => {
                let cg = string(self, l)?.to_ascii_lowercase();
                if !cg.is_empty() && self.seen_pictures.insert(cg.clone()) {
                    self.system_dirty = true;
                }
                self.current_cg = cg;
                0
            }
            // 648B80(mode, ms): text speed override.
            104 => {
                self.tags.text_speed = (w != 0).then_some(u64::from(l));
                0
            }
            // 648B40: font size (0 = normal).
            105 => {
                self.tags.font_size = (w != 0).then_some(w as i32);
                0
            }
            106 => {
                self.wait = Wait::Until(now + u64::from(w));
                0
            }
            108 => {
                self.cg_params = (w, l);
                0
            }
            109 => {
                self.register_90d138 = w;
                0
            }
            // 62CEF0: picture into frame 12346 (screen overlay picture).
            110 => {
                let src = string(self, l)?;
                self.emit(h, format!("<layer parent='front' id='__frame12346' src='{src}'>"))?;
                0
            }
            115 => {
                self.adv.as_mut().unwrap().state.select = Some(adv::Select::default());
                0
            }
            116 => {
                if w >> 16 != 0 {
                    let mut text = string(self, l)?;
                    if text.starts_with('<') {
                        // 63AB10: a tag item carries its caption in `text`.
                        let tags = adv::parse_tags(&text)?;
                        text = tags.first().and_then(|(_, a)| a.get("text").cloned()).unwrap_or_default();
                    }
                    let sel = self.adv.as_mut().unwrap().state.select.get_or_insert_with(Default::default);
                    ensure!(sel.items.len() < 32, "Malie choice budget");
                    sel.items.push((w & 0xffff, text));
                }
                0
            }
            117 => {
                let sel = self.adv.as_mut().unwrap().state.select.get_or_insert_with(Default::default);
                ensure!(!sel.items.is_empty(), "Malie empty choice list");
                sel.shown = true;
                sel.focus = None;
                self.wait = Wait::Select;
                // 4EF400: a shown choice ends a jump and is the snapshot point.
                self.stop_jump();
                self.choice_snapshot_due = true;
                0
            }
            118 => self.adv_frame(h, 1007, w, l)?,
            // 613800: compile the script source file `l` (6758A0 → 68A340)
            // and switch to it. When the file cannot be opened the compiler
            // yields null and 613800 returns 0 with the current module kept.
            120 => {
                let name = string(self, l)?;
                u32::from(self.switch_module(&name)?)
            }
            // 63B190(…, 10): deferred reset to the title.
            121 => {
                self.after_wait = None;
                self.return_to_title(h)?;
                0
            }
            123 | 142 => {
                self.pending_voice = Some(string(self, l)?);
                0
            }
            124 => {
                self.mark_time = now;
                0
            }
            // 4FB2E0: wait for event `w` (resumed by `event`/click).
            125 => {
                self.wait = Wait::Suspended;
                0
            }
            126 => {
                self.wait = Wait::Channel(VOICE);
                0
            }
            // Timer item 302 with a text payload, then suspend.
            127 => {
                self.wait = Wait::Until(now + 1000);
                0
            }
            128 | 177 => {
                self.bgm_loop_flag = w != 0;
                0
            }
            // 129: struct tm fields (0 year-1900, 1 month-1, 2 day, 3 hour, 4 min, 5 sec).
            129 => match self.local_time(now) {
                Some((y, mo, d, hh, mm, ss)) => match w {
                    0 => (y - 1900) as u32,
                    1 => mo - 1,
                    2 => d,
                    3 => hh,
                    4 => mm,
                    5 => ss,
                    _ => 0,
                },
                None => 0,
            },
            140 => {
                if self.bgm != Some(w) {
                    self.tags.bgm_volume = 1.;
                    self.start_bgm(h, w, self.bgm_loop_flag, 0, u64::from(l), (0, None))?;
                }
                0
            }
            141 => {
                self.bgm = None;
                self.tags.bgm = None;
                let fade = if self.skip_active { 0 } else { l.min(60_000) };
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM, fade_ms: fade as f32 })?;
                0
            }
            // 62F5F0: looping numbered SE.
            143 => {
                if w > 0 {
                    self.emit(h, format!("<se src='{w}' loop='true'>"))?;
                }
                0
            }
            // Character registry: 150 front, 151 remove, 152 dress, 153 face, 163 place.
            150 | 151 | 152 | 153 | 163 => {
                let who = string(self, w)?.to_ascii_lowercase();
                let value = string(self, l)?.to_ascii_lowercase();
                let adv = self.adv.as_mut().unwrap();
                match msg {
                    151 => {
                        adv.state.chars.remove(&who);
                    }
                    _ => {
                        let rec = adv.state.chars.entry(who).or_insert_with(|| adv::CharRecord { opacity: 1., ..Default::default() });
                        match msg {
                            152 => rec.dress = value,
                            153 => rec.face = value.trim_end_matches("_f").to_owned(),
                            163 => rec.place = value.parse().ok(),
                            _ => {}
                        }
                    }
                }
                0
            }
            156 => self.adv_frame(h, 1011, w, 1)?,
            157 => {
                let name = string(self, w)?;
                self.native_dialog(h, &name)?;
                0
            }
            // Messages 104/108–112/120 to ADV item 201 (CUTIN2). The class
            // registered by 50C000 has no message callback (+76), so 614730
            // returns 0; 154/158 first register the character (644E00).
            154 | 158 => {
                let who = string(self, w)?.to_ascii_lowercase();
                if !who.is_empty() {
                    self.adv.as_mut().unwrap().state.chars.entry(who).or_insert_with(|| adv::CharRecord { opacity: 1., ..Default::default() });
                }
                0
            }
            159..=162 | 401 => 0,
            164 => self.adv_frame(h, 1023, w, l)?,
            // 4FAF00: address of name slot `w`.
            165 | 166 => {
                let text = self.name_slots.get(w as usize).cloned().unwrap_or_default();
                self.machine.as_mut().unwrap().host_string(&text)?
            }
            // 4FB160: set name slot (w >= 10 formats l as a number).
            167 => {
                ensure!((w as usize) < NAMES, "Malie name slot {w}");
                let text = if w >= 10 { (l as i32).to_string() } else { string(self, l)? };
                self.name_slots[w as usize] = text;
                0
            }
            168 => self.adv_frame(h, 5001, w, l)?,
            169 => {
                self.ini_section = string(self, l)?;
                1
            }
            172 => {
                let raw = if l != 0 { string(self, l)? } else { self.name_slots.get(w as usize).cloned().unwrap_or_default() };
                let name = self.nameconv.get(&raw).cloned().unwrap_or(raw);
                self.adv.as_mut().unwrap().state.name = name;
                0
            }
            173 => {
                self.font_position = w;
                0
            }
            174 => {
                self.tags.text_color = Some([(w >> 16) as u8, (w >> 8) as u8, w as u8]);
                self.tags.user_color = false;
                self.adv_frame(h, 5011, 0, w)?
            }
            175 => {
                self.window_mode = w;
                0
            }
            176 => {
                self.window_mode = w;
                self.adv_frame(h, 1037, w, l)?
            }
            178 => {
                self.register_86c560 = w;
                0
            }
            // 62FC60/62FC80: imgblk enter/leave.
            179 => {
                if w != 0 {
                    self.imgblk = Some(("ol".into(), 500));
                } else if let Some((mask, time)) = self.imgblk.take() {
                    if !self.skip_active {
                        self.screen_transition(&mask, time, now);
                    }
                }
                0
            }
            180 => {
                if w < 32 {
                    self.int_vars[w as usize] = l;
                }
                0
            }
            182 => {
                self.se_loop_flag = w != 0;
                0
            }
            190 | 191 => {
                let f = string(self, l)?;
                let f = if f.eq_ignore_ascii_case("none") { String::new() } else { f };
                self.emit(h, format!("<{} src='{}'>", if msg == 190 { "cgfilter" } else { "charfilter" }, if f.is_empty() { "none" } else { &f }))?;
                0
            }
            // 62F870: wait ms, or the background SVG animation.
            201 => {
                if w == u32::MAX {
                    self.emit(h, "<layerwait parent='back' id='bg' type='svg'>".into())?;
                } else {
                    self.wait = Wait::Until(now + u64::from(w));
                }
                0
            }
            // 210 (_ms_overlap) has no selector in 62CF80.
            210 => 0,
            220 | 221 => {
                self.wait = Wait::Click;
                0
            }
            222 => {
                let adv = self.adv.as_mut().unwrap();
                let name = adv.state.name.clone();
                let text = adv.clear();
                if !text.trim().is_empty() {
                    if self.backlog.len() >= 4096 {
                        self.backlog.remove(0);
                    }
                    self.backlog.push((name, text, self.last_voice.clone()));
                }
                0
            }
            223 => {
                let voice = self.adv.as_mut().unwrap().out(now);
                if let Some(v) = self.pending_voice.take().or(voice) {
                    if !self.skip_active {
                        self.play_voice(h, &v, 1.)?;
                    }
                }
                0
            }
            224 => {
                let text = self.machine.as_ref().unwrap().program.message(l)?;
                let text = self.substitute(&text);
                let segments = message::segments(&text).with_context(|| format!("Malie message {l}"))?;
                self.adv.as_mut().unwrap().message(segments);
                0
            }
            230 => {
                self.tag_builder = Some((string(self, w)?.to_ascii_lowercase(), Attrs::new()));
                0
            }
            231 => {
                let key = string(self, w)?.to_ascii_lowercase();
                let value = string(self, l)?;
                if let Some((_, a)) = &mut self.tag_builder {
                    ensure!(a.len() < 64, "Malie tag builder budget");
                    a.insert(key, value);
                }
                0
            }
            232 => {
                if let Some((name, a)) = self.tag_builder.take() {
                    self.tag_one(h, &name, &a)?;
                }
                0
            }
            // 240: raw tag text.
            240 => {
                let t = string(self, l)?;
                self.tag(h, &t)?;
                0
            }
            250 => {
                self.label = string(self, l)?;
                0
            }
            // 62CE70/611DC0: message-frame source.
            300 => {
                let src = string(self, l)?;
                self.adv.as_mut().unwrap().state.frame.src = src.to_ascii_lowercase();
                0
            }
            // 638620: 1 = no auto mode, 2 = no skip mode.
            400 => u32::from(match w {
                1 => !self.auto_active,
                2 => !self.skip_mode,
                _ => false,
            }),
            402 => {
                self.save_disable = if w != 0 { self.save_disable.saturating_sub(1) } else { self.save_disable + 1 };
                0
            }
            403 => {
                self.tags.save_enabled = w != 0;
                0
            }
            // No 500 selector exists; _obj_ALL_CLEAR receives null items.
            500 => 0,
            1000 => {
                // 6112B0: stop audio, clear every root, black back layer, jump `title`.
                self.return_to_title(h)?;
                self.emit(h, "<layer parent='back' id='bg' src='black'>".into())?;
                return Ok(0);
            }
            // 62DA30: cancel the open dialog.
            9000 => {
                if self.dialog.is_some() {
                    self.close_dialog(h)?;
                }
                0
            }
            _ => 0,
        })
    }
    /// `\x07` message records may reference name slots as `%s`-free text;
    /// MALIE_NAME_NUM slots are applied when the name is shown.
    fn substitute(&self, text: &str) -> String {
        text.to_owned()
    }
}
