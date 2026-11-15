//! Process case 6 dialogue controls: the message-window button templates.
//!
//! Both installations' start.mes build two control templates through 0x18:
//! selector 1 (Rust slot 9) holds the collapsed tab (`:n`) and selector 6
//! (slot 10) the expanded bar (torisetu, auto, rapid, qs, ql, backlog,
//! option, clear and `:n` again). While a rendered line waits:
//!
//! * DC3WY 42A160 case 6 (value 99/234 == 0) calls 4130B0 on template 9
//!   when Buf+34 (value 99/227) is clear, else on template 10, and hands a
//!   nonempty action to 421AD0: `n` toggles Buf+34, `c` sets this[2839]
//!   (window hidden until the next click), `r` 4215E0(0) toggles skip
//!   (Buf[62076], value 99/195) and clears auto (Buf+4, value 99/13), `a`
//!   4215E0(2) toggles auto and clears skip, `o` 4215E0(6) enters "config",
//!   `ds`/`dl` 4215E0(4/5) enter "save"/"load" unless var(9,7).
//! * D.C.III 429B80 (mouse branch) hit-tests the active template C37500; `n`
//!   swaps it with C3D698 (the other 0x18 1/6 template), `c` 424340 hides
//!   the window, `r` toggles CB9A94 unless a choice is open, `a` toggles the
//!   auto flag 7E0C68, `s`/`l`/`h`/`o`/`b` 423630 the NAMES.DAT bootstrap
//!   scenes SAVE/LOAD/ADVMENU/OPTION/LOG, `t` 423630("manual").
//!
//! Not connected (fail visibly): quick save/load (`qs`/`ql`: DC3WY 4215E0
//! 7/8 confirmation templates, D.C.III 4221E0/4220D0 slot 101), the DC3WY
//! native backlog `b` (41DE40(2) + 405010) and voice replay `t`, D.C.III's
//! auto-speed bar actions (`a<`/`a>`/`a+`/`a-`/`a=`/`m`/`u`), and the value
//! 99/234 template 3..6 path.
//!
//! Auto mode (41E740): with Buf+4 set the wait ends once the time since the
//! line was dispatched exceeds 5 * (23 - value 99/29) * (bytes / 2) ms
//! (433210 over the CP932 body), and, with Buf+5 (value 99/28), once voice
//! channels 7..10 (432060(2)) are silent.
use super::*;
use crate::machine::text_ops::TextRequest;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ControlsState {
    ///The template currently published as the dialogue-control menu.
    pub shown: Option<u8>,
    ///D.C.III: C37500 and C3D698 swapped by `n`.
    pub swapped: bool,
    ///DC3WY this[2839] / D.C.III 424340: the window is hidden until a click.
    pub hidden: bool,
    ///A release the control menu received outside its buttons.
    #[serde(skip)]
    pub advance: bool,
}

impl Machine {
    fn control_template(&self) -> Result<u8> {
        let expanded = if self.state.original { self.state.controls.swapped } else { self.value(99, 227)? != 0 };
        Ok(if expanded { 10 } else { 9 })
    }

    ///Remove the published control menu (the line's wait has ended).
    pub(super) fn clear_dialogue_controls(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if self.state.controls.shown.is_none() {
            return Ok(true);
        }
        if host.text(&TextRequest::RefreshDialogueControls { group: None })?.is_none() {
            return Ok(false);
        }
        self.state.controls.shown = None;
        Ok(true)
    }

    fn set_window_hidden(&mut self, host: &mut dyn NativeHost, hidden: bool) -> Result<bool> {
        let visible = !hidden;
        if !host.emit(&Event::PresentationPolicy {
            window_visible: visible,
            confirmation_visible: true,
            dialogue_visible: visible,
            status_visible: false,
            indicator: None,
        })? {
            return Ok(false);
        }
        self.state.controls.hidden = hidden;
        Ok(true)
    }

    ///One process frame of the controls; Some(outcome) when this frame is
    ///consumed (a hidden window, a control action or pending work).
    pub(super) fn poll_dialogue_controls(&mut self, host: &mut dyn NativeHost, pressed: bool, now: u64) -> Result<Option<Outcome>> {
        if self.state.controls.hidden {
            //The hidden menu still owns the pointer: any click reveals.
            let hidden_action = if self.state.controls.shown.is_some() { host.menu_action()?.is_some() } else { false };
            if hidden_action {
                self.state.controls.shown = None;
            }
            let pressed = pressed || hidden_action || host.menu_free_release();
            if !pressed {
                return Ok(Some(Outcome::WaitForInput));
            }
            if !self.set_window_hidden(host, false)? {
                return Ok(Some(Outcome::Pending));
            }
            //The revealing click is consumed.
            return Ok(Some(Outcome::WaitForInput));
        }
        if !self.state.original {
            ensure!(self.value(99, 234)? == 0, "DC3WY 42A160 value 99/234 template 3..6 controls are not connected");
        }
        let template = self.control_template()?;
        if self.state.controls.shown != Some(template) {
            let group = self.state.templates.get(&template).map(|builder| builder.group.clone());
            if group.is_none() {
                return Ok(None);
            }
            if host.text(&TextRequest::RefreshDialogueControls { group })?.is_none() {
                return Ok(Some(Outcome::Pending));
            }
            self.state.controls.shown = Some(template);
        }
        let action = host.menu_action()?;
        if !host.callbacks_ready() {
            return Ok(Some(Outcome::Pending));
        }
        let Some(action) = action.filter(|action| !action.is_empty() && action != "-") else {
            //The menu's full-canvas root takes the pointer; a release outside
            //every control is the message window's own advance click.
            self.state.controls.advance |= host.menu_free_release();
            return Ok(None);
        };
        //The activation disabled the menu; publish it again next frame.
        self.state.controls.shown = None;
        self.dialogue_action(host, &action, now).map(Some)
    }

    fn toggle_skip(&mut self) -> Result<()> {
        let skip = self.state.runtime_values.get(&195).copied().unwrap_or(0) == 0;
        self.state.runtime_values.insert(195, i32::from(skip));
        let mut data = self.state.system_data.clone();
        ensure!(data.set_prefix_value(13, 0)?, "DC3 auto-mode native field");
        self.state.system_data = data;
        Ok(())
    }

    fn toggle_auto(&mut self, now: u64) -> Result<()> {
        let auto = self.value(99, 13)? == 0;
        let mut data = self.state.system_data.clone();
        ensure!(data.set_prefix_value(13, i32::from(auto))?, "DC3 auto-mode native field");
        self.state.system_data = data;
        self.state.runtime_values.insert(195, 0);
        //4215E0(2): a1[470] = 0 restarts the line timer.
        self.state.text.shown_at = Some(now);
        Ok(())
    }

    fn dialogue_action(&mut self, host: &mut dyn NativeHost, action: &str, now: u64) -> Result<Outcome> {
        let name = action.strip_prefix(':').unwrap_or(action);
        let entry = |machine: &mut Self, scene: &str| -> Result<Outcome> {
            machine.begin_system_entry(scene)?;
            Ok(Outcome::WaitUntil(0))
        };
        if self.state.original {
            match name {
                "n" | "n<" | "n>" => {
                    self.state.controls.swapped = !self.state.controls.swapped;
                    Ok(Outcome::WaitForInput)
                }
                _ if name.starts_with('c') => Ok(if self.set_window_hidden(host, true)? { Outcome::WaitForInput } else { Outcome::Pending }),
                _ if name.starts_with('r') => {
                    self.toggle_skip()?;
                    Ok(Outcome::WaitForInput)
                }
                _ if name.starts_with('a') && !matches!(name.as_bytes().get(1), Some(b'<' | b'>' | b'+' | b'-' | b'=')) => {
                    self.toggle_auto(now)?;
                    Ok(Outcome::WaitForInput)
                }
                _ if name.starts_with('s') => entry(self, "SAVE"),
                _ if name.starts_with('l') => entry(self, "LOAD"),
                _ if name.starts_with('h') => entry(self, "ADVMENU"),
                _ if name.starts_with('o') => entry(self, "OPTION"),
                _ if name.starts_with('b') => entry(self, "LOG"),
                _ if name.starts_with('t') => entry(self, "manual"),
                _ => bail!("D.C.III 429B80 dialogue control action {action:?} is not connected"),
            }
        } else {
            match name.as_bytes() {
                [b'n', ..] => {
                    let open = self.value(99, 227)? == 0;
                    let mut data = self.state.system_data.clone();
                    ensure!(data.set_prefix_value(227, i32::from(open))?, "DC3WY Buf+34 native field");
                    self.state.system_data = data;
                    Ok(Outcome::WaitForInput)
                }
                [b'c', ..] => Ok(if self.set_window_hidden(host, true)? { Outcome::WaitForInput } else { Outcome::Pending }),
                [b'r', ..] => {
                    self.toggle_skip()?;
                    Ok(Outcome::WaitForInput)
                }
                [b'o', ..] => entry(self, "config"),
                [b'd', b's', ..] if self.value(9, 7)? == 0 => entry(self, "save"),
                [b'd', b'l', ..] if self.value(9, 7)? == 0 => entry(self, "load"),
                [b'd', b's' | b'l', ..] => Ok(Outcome::WaitForInput),
                [b'a', b'a', ..] => bail!("DC3WY 421AD0 action {action:?} is not connected"),
                [b'a', ..] => {
                    self.toggle_auto(now)?;
                    Ok(Outcome::WaitForInput)
                }
                _ => bail!("DC3WY 421AD0 dialogue control action {action:?} is not connected"),
            }
        }
    }

    ///41E740's auto-mode branch for the waiting line.
    pub(super) fn auto_advance_due(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        if self.value(99, 13)? == 0 {
            return Ok(false);
        }
        let Some(start) = self.state.text.shown_at else {
            return Ok(false);
        };
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(&self.state.text.body);
        let characters = i64::try_from(encoded.len() / 2)?;
        let wait = 5 * (23 - i64::from(self.value(99, 29)?)) * characters;
        if (now.saturating_sub(start) as i64) <= wait {
            return Ok(false);
        }
        if self.value(99, 28)? != 0 {
            for channel in 8..=11 {
                if host.audio_playing(channel)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
