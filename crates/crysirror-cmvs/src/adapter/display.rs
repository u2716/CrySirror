// SPDX-License-Identifier: MPL-2.0
//! Native display configuration and user settings (`cmvs.cfg`) over shared window
//! services. Field offsets name the 2021 x64 display configuration object
//! (engine +1976 → +0) and the engine settings block at +1496.

use crate::vm::{CmvsVm, VmProfile};
use anyhow::{ensure, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{HostServices, PresentationScale},
    state::ClientId,
};
use crysirror_resource::private_profile::PrivateProfile;
use serde_json::json;

/// Frame periods selected by `GRAPHIC_CONFIG/FRAME_RATE` (0x140021820).
const FRAME_RATES: [u32; 4] = [60, 30, 20, 120];

pub(super) struct Display {
    profile: VmProfile,
    /// +112: windowed presentation. Scripts query the inverse with `0x2163`.
    windowed: bool,
    /// +136: `FRAME_RATE` index.
    frame_rate: u32,
    /// +140: presentation filter selected by `0x216d`.
    filter: u32,
    /// +148: display adapter selection, 5 meaning the adapter under the window.
    adapter: u32,
    /// PC20160201 CD3DDevice +0x90, set by the native other-display option.
    other_display: bool,
    /// +152: the window may take any size, with the canvas fitted inside it.
    free_size: bool,
    /// +156: fullscreen layout, 0 centred, 1 and 2 fitted and 3 stretched.
    screen_mode: u32,
    /// Layout object +188, `0x22ec`: fullscreen output fills the display.
    fill: bool,
    /// Layout object +164/+168: the current client size.
    window_size: [u32; 2],
    settings: Settings,
    /// Contents of `cmvs.cfg`, whose keys `0x22d2..` write back.
    config: PrivateProfile,
    config_dirty: bool,
    /// Next frame start of the native limiter, in host milliseconds.
    next_frame: Option<u64>,
    cvns_frame_period: u32,
    cursor_hidden: bool,
    cursor_shown_by_script: bool,
}

/// Engine settings block at +1496, mirrored by the native menu's check marks.
#[derive(Clone, Copy)]
struct Settings {
    /// +1496/+1500: `SYSTEM_INFO/DLG_LOAD` and `DLG_SAVE` confirmations.
    dialogs: [bool; 2],
    /// +1504: `SYSTEM_INFO/WAIT_TIMING`, the performance-counter clock.
    wait_timing: bool,
    /// +1636: `MOUSE_INFO/AUTO_CLEAR`, hiding the cursor after 5 s per step.
    auto_clear: u32,
    /// +1640: `VIDEO_INFO/FRAME_CTRL`.
    frame_ctrl: u32,
    /// Sound device +2796 (`0x22ed`): keep mixing while the window is inactive.
    inactive_sound: bool,
    /// PC20211007 engine +1552, forwarded to the outline font renderers.
    alpha_revision: bool,
}

/// Native menu items whose checks follow these settings.
const MENU_DIALOGS: u32 = 32764;
const MENU_WAIT_TIMING: u32 = 32769;
const MENU_FRAME_CTRL: u32 = 32775;
const MENU_AUTO_CLEAR: u32 = 32778;
const MENU_WINDOW: u32 = 32759;
const MENU_FREE_SIZE: u32 = 32763;
const MENU_ALPHA_REVISION: u32 = 32931;

impl Display {
    pub fn new(profile: VmProfile, canvas: [u32; 2], config: &[u8]) -> Self {
        let (text, _) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(config);
        let config = PrivateProfile::parse(&text);
        let int = |section, key, default| config.integer(section, key, default);
        let frame_rate = match int("GRAPHIC_CONFIG", "FRAME_RATE", 0) {
            rate @ 0..=3 => rate as u32,
            _ => 0,
        };
        let filter = match int("GRAPHIC_CONFIG", "ASPECT_RATIO_FILTER", 0) {
            value @ (0 | 1) => 1 - value,
            value @ 2..=3 => value,
            _ => 0,
        };
        let screen_mode = match int("DISPLAY_CONFIG", "ASPECT_RATIO_SCREEN", 1) {
            mode @ 0..=3 => mode as u32,
            _ => 1,
        };
        let settings = Settings {
            dialogs: [
                int("SYSTEM_INFO", "DLG_LOAD", 0) != 0,
                int("SYSTEM_INFO", "DLG_SAVE", 0) != 0,
            ],
            wait_timing: int("SYSTEM_INFO", "WAIT_TIMING", 0) & 1 != 0,
            auto_clear: int("MOUSE_INFO", "AUTO_CLEAR", 1).clamp(0, 4) as u32,
            frame_ctrl: int("VIDEO_INFO", "FRAME_CTRL", 0).clamp(0, 2) as u32,
            inactive_sound: int("SOUND_INFO", "INACTIVE_SOUND_PLAY", 1) != 0,
            alpha_revision: int("MESSAGE_INFO", "TEXT_REVISION_ALPHA", 0) != 0,
        };
        Self {
            profile,
            windowed: int("WINDOW_STATUS", "WINDOW_MODE", 0) == 0,
            frame_rate,
            filter: if filter < 3 { filter as u32 } else { 1 },
            adapter: int("GRAPHIC_CONFIG", "USE_ADPTER_NO", 0).clamp(0, 5) as u32,
            other_display: int("WINDOW_RESOLVING", "OTHERS_DISPLAY_USE", 0) != 0,
            free_size: true,
            screen_mode,
            fill: false,
            window_size: canvas,
            settings,
            config,
            config_dirty: false,
            next_frame: None,
            cvns_frame_period: 17,
            cursor_hidden: false,
            cursor_shown_by_script: true,
        }
    }

    /// Milliseconds per native frame (`0x2005`).
    pub fn frame_period(&self) -> u32 {
        if self.profile == VmProfile::Cvns20040515X86 {
            self.cvns_frame_period
        } else {
            1000 / FRAME_RATES[self.frame_rate as usize]
        }
    }

    pub fn cvns_set_frame_period(&mut self, vm: &mut CmvsVm) -> Result<()> {
        let [period] = vm.owner_arguments(0x2002)?;
        ensure!(period != 0, "CVNS frame timer cannot advance with a zero period");
        vm.complete_owner_call(1, None)?;
        self.cvns_frame_period = period;
        Ok(())
    }

    /// Native frame limiter: a frame may start once its period has passed.
    /// Returns the time to wait for when the next frame is not due yet.
    pub fn frame_due(&mut self, now: u64) -> Option<u64> {
        let period = u64::from(self.frame_period());
        match self.next_frame {
            Some(due) if now < due => Some(due),
            Some(due) if self.profile == VmProfile::Cvns20040515X86 => {
                // sub_43a730 catches up missed periods without executing a
                // batch for each skipped deadline.
                self.next_frame = Some(now.saturating_add(period - (now - due) % period));
                None
            }
            Some(due) => {
                // Late frames start at once; the schedule never runs more than a period behind.
                self.next_frame = Some((due + period).max(now));
                None
            }
            None => {
                self.next_frame = Some(now + period);
                None
            }
        }
    }

    pub fn inactive_sound(&self) -> bool {
        self.settings.inactive_sound
    }

    /// Changed `cmvs.cfg` contents to store under the save root, once.
    pub fn take_config(&mut self) -> Option<Vec<u8>> {
        if !std::mem::take(&mut self.config_dirty) {
            return None;
        }
        let text = self.config.to_text();
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(&text);
        Some(bytes.into_owned())
    }

    pub fn config_path() -> Result<VfsPath> {
        Ok(VfsPath::new(Mount::Save, "cmvs.cfg")?)
    }

    pub fn alpha_revision(&self) -> Option<bool> {
        (self.profile == VmProfile::Pc20211007X64).then_some(self.settings.alpha_revision)
    }

    fn alpha_command(&mut self, vm: &mut CmvsVm) -> Result<()> {
        let [mode, value] = vm.owner_arguments(0x22f9)?;
        if mode == 0 {
            return Ok(vm.complete_owner_call(2, Some(u32::from(self.settings.alpha_revision)))?);
        }
        let enabled = value != 0;
        if self.settings.alpha_revision != enabled {
            self.settings.alpha_revision = enabled;
            self.write("MESSAGE_INFO", "TEXT_REVISION_ALPHA", u32::from(enabled));
        }
        Ok(vm.complete_owner_call(2, None)?)
    }

    fn write(&mut self, section: &str, key: &str, value: u32) {
        self.config.set(section, key, &value.to_string());
        self.config_dirty = true;
    }

    /// Menu check marks for the current settings, in native item order.
    pub fn menu_checks(&self) -> Vec<(u32, bool)> {
        let mut checks = vec![
            (MENU_DIALOGS, self.settings.dialogs[0]),
            (MENU_DIALOGS + 1, self.settings.dialogs[1]),
            (MENU_WAIT_TIMING, !self.settings.wait_timing),
            (MENU_WAIT_TIMING + 1, self.settings.wait_timing),
            (MENU_WINDOW, self.windowed),
            (MENU_WINDOW + 1, !self.windowed),
            (MENU_FREE_SIZE, self.free_size),
        ];
        checks.extend((0..3).map(|item| (MENU_FRAME_CTRL + item, self.settings.frame_ctrl == item)));
        checks.extend((0..5).map(|item| (MENU_AUTO_CLEAR + item, self.settings.auto_clear == item)));
        if let Some(enabled) = self.alpha_revision() {
            checks.push((MENU_ALPHA_REVISION, enabled));
        }
        checks
    }

    fn presentation(&self) -> PresentationScale {
        if self.windowed {
            return PresentationScale::Fit;
        }
        if self.fill {
            return PresentationScale::Stretch;
        }
        match self.screen_mode {
            0 => PresentationScale::Center,
            3 => PresentationScale::Stretch,
            _ => PresentationScale::Fit,
        }
    }

    fn submit_ui(host: &mut dyn HostServices, client: ClientId, ops: Vec<serde_json::Value>) -> Result<()> {
        let batch = json!({ "ui": ops });
        host.submit_engine(client, 1, &serde_json::to_vec(&batch)?)?;
        Ok(())
    }

    fn apply_mode(&self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let scale = serde_json::to_value(self.presentation())?;
        Self::submit_ui(
            host,
            client,
            vec![
                json!({"op":"set_fullscreen","enabled":!self.windowed}),
                json!({"op":"set_presentation_scale","mode":scale}),
            ],
        )
    }

    /// Observes the host window before scripts query it.
    pub fn observe(&mut self, host: &dyn HostServices, client: ClientId) -> Result<()> {
        if let Some(fullscreen) = host.window_fullscreen(client)? {
            self.windowed = !fullscreen;
        }
        if let Some(geometry) = host.window_geometry(client)? {
            self.window_size = geometry.inner_size;
        }
        Ok(())
    }

    /// `MOUSE_INFO/AUTO_CLEAR` (0x14002d260): hide the cursor after 5 s per step
    /// without pointer movement, while the script has not hidden it itself.
    pub fn advance_cursor(&mut self, host: &mut dyn HostServices, client: ClientId, idle_ms: Option<u64>) -> Result<()> {
        let hidden = self.settings.auto_clear != 0
            && idle_ms.is_some_and(|idle| idle >= 5000 * u64::from(self.settings.auto_clear));
        if hidden != self.cursor_hidden && self.cursor_shown_by_script {
            self.cursor_hidden = hidden;
            Self::submit_ui(host, client, vec![json!({"op":"set_cursor_visible","visible":!hidden})])?;
        }
        Ok(())
    }

    /// Handles a display or settings selector. Returns false for other selectors.
    pub fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let revision = self.profile == VmProfile::Pc20211007X64;
        match opcode {
            0x22f9 if revision => self.alpha_command(vm)?,
            0x22ff if self.profile == VmProfile::Pc20160201X64 => {
                vm.complete_owner_call(0, Some(u32::from(self.other_display)))?;
            }
            0x2005 => vm.complete_owner_call(0, Some(self.frame_period()))?,
            0x20cd => {
                let [visible] = vm.owner_arguments(opcode)?;
                self.cursor_shown_by_script = visible != 0;
                self.cursor_hidden = false;
                Self::submit_ui(host, client, vec![json!({"op":"set_cursor_visible","visible":visible != 0})])?;
                vm.complete_owner_call(1, None)?;
            }
            0x2163 => {
                self.observe(host, client)?;
                vm.complete_owner_call(0, Some(u32::from(!self.windowed)))?;
            }
            0x2164 => {
                let [fullscreen] = vm.owner_arguments(opcode)?;
                self.windowed = fullscreen == 0;
                self.write("WINDOW_STATUS", "WINDOW_MODE", u32::from(!self.windowed));
                self.apply_mode(host, client)?;
                vm.finish_owner_call(1, None, true)?;
            }
            0x2168 => {
                let [x, y] = vm.owner_arguments(opcode)?;
                if self.windowed {
                    Self::submit_ui(
                        host,
                        client,
                        vec![json!({"op":"set_window_position","position":[x as i32, y as i32]})],
                    )?;
                }
                vm.finish_owner_call(2, None, true)?;
            }
            0x2169 => {
                // IsIconic gives 2; the host reports no maximized state, which would give 1.
                let minimized = host.window_minimized(client)?.unwrap_or(false);
                vm.complete_owner_call(0, Some(if minimized { 2 } else { 0 }))?;
            }
            0x216c => vm.complete_owner_call(0, Some(self.filter))?,
            0x216d => {
                let [filter] = vm.owner_arguments(opcode)?;
                let filter = match filter as i32 {
                    value @ 0..=2 => value as u32,
                    3 => 1,
                    _ => 0,
                };
                self.filter = filter;
                let stored = if filter <= 1 { 1 - filter } else { filter };
                self.write("GRAPHIC_CONFIG", "ASPECT_RATIO_FILTER", stored);
                vm.complete_owner_call(1, None)?;
            }
            0x216e => {
                // 0x14002f4a0: restore the canvas size when the window has been resized.
                self.observe(host, client)?;
                let resize = self.free_size && self.window_size != canvas;
                if resize {
                    self.window_size = canvas;
                    if self.windowed {
                        Self::submit_ui(host, client, vec![json!({"op":"set_window_size","size":canvas})])?;
                    }
                }
                vm.finish_owner_call(0, None, resize && self.windowed)?;
            }
            0x216f | 0x22ca => vm.complete_owner_call(0, Some(u32::from(self.free_size)))?,
            0x22c8 => vm.complete_owner_call(0, Some(1))?,
            0x22c9 => {
                let [enabled] = vm.owner_arguments(opcode)?;
                let enabled = enabled != 0;
                let changed = self.free_size != enabled;
                self.free_size = enabled;
                Self::submit_ui(
                    host,
                    client,
                    vec![json!({"op":"set_window_policy","decorated":true,"resizable":enabled})],
                )?;
                // A fullscreen device is recreated with the new layout.
                let recreate = changed && !self.windowed;
                if recreate {
                    self.apply_mode(host, client)?;
                }
                vm.finish_owner_call(1, None, recreate)?;
            }
            0x22d2 => {
                let [write, value] = vm.owner_arguments(opcode)?;
                if write != 0 {
                    self.settings.wait_timing = (value as i32).max(0) % 2 != 0;
                    self.write("SYSTEM_INFO", "WAIT_TIMING", u32::from(self.settings.wait_timing));
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(u32::from(self.settings.wait_timing)))?;
                }
            }
            0x22d3 | 0x22d4 => {
                let [write, value] = vm.owner_arguments(opcode)?;
                let index = usize::from(opcode == 0x22d4);
                if write != 0 {
                    self.settings.dialogs[index] = value != 0;
                    let key = ["DLG_LOAD", "DLG_SAVE"][index];
                    self.write("SYSTEM_INFO", key, u32::from(value != 0));
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(u32::from(self.settings.dialogs[index])))?;
                }
            }
            0x22e9 => {
                let [write, value] = vm.owner_arguments(opcode)?;
                if write != 0 {
                    self.settings.auto_clear = if value <= 4 { value } else { 0 };
                    self.write("MOUSE_INFO", "AUTO_CLEAR", self.settings.auto_clear);
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(self.settings.auto_clear))?;
                }
            }
            0x22ea => {
                let [write, value] = vm.owner_arguments(opcode)?;
                if write != 0 {
                    self.settings.frame_ctrl = ((value as i32).clamp(0, 2) as u32) % 3;
                    self.write("VIDEO_INFO", "FRAME_CTRL", self.settings.frame_ctrl);
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(self.settings.frame_ctrl))?;
                }
            }
            0x22ec => {
                let [write, value] = vm.owner_arguments(opcode)?;
                if write != 0 {
                    self.fill = value != 0;
                    if !self.windowed {
                        self.apply_mode(host, client)?;
                    }
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(u32::from(self.fill)))?;
                }
            }
            0x22ed if revision => {
                let [write, value] = vm.owner_arguments(opcode)?;
                if write != 0 {
                    self.settings.inactive_sound = value != 0;
                    self.write("SOUND_INFO", "INACTIVE_SOUND_PLAY", u32::from(value != 0));
                    vm.complete_owner_call(2, None)?;
                } else {
                    vm.complete_owner_call(2, Some(u32::from(self.settings.inactive_sound)))?;
                }
            }
            0x22fc => vm.complete_owner_call(0, Some(self.screen_mode))?,
            0x22fd => {
                let [mode] = vm.owner_arguments(opcode)?;
                let mode = if mode <= 3 { mode } else { 1 };
                if mode != self.screen_mode {
                    self.screen_mode = mode;
                    self.write("DISPLAY_CONFIG", "ASPECT_RATIO_SCREEN", mode);
                    if self.free_size && !self.windowed {
                        self.apply_mode(host, client)?;
                    }
                }
                vm.complete_owner_call(1, None)?;
            }
            0x23c5 if revision => vm.complete_owner_call(0, Some(self.adapter))?,
            0x23c6 if revision => {
                // 0x140031af0 with the single display the host exposes: 1 names it,
                // 2..=4 fall back to 0 and 5 follows the window.
                let [adapter] = vm.owner_arguments(opcode)?;
                let adapter = if (2..=4).contains(&adapter) { 0 } else { adapter };
                self.adapter = adapter;
                self.write("GRAPHIC_CONFIG", "USE_ADPTER_NO", adapter);
                vm.complete_owner_call(1, None)?;
            }
            0x23c7 if revision => vm.complete_owner_call(0, Some(1))?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::VmYield;
    use crysirror_core::service::MemoryAssetReader;

    #[test]
    fn alpha_revision_updates_its_own_key_and_reclaims_both_arguments() -> Result<()> {
        let profile = VmProfile::Pc20211007X64;
        let config = b"[MESSAGE_INFO]\r\nFONT=Arial\r\nTEXT_REVISION_ALPHA=0\r\n";
        let mut display = Display::new(profile, [640, 480], config);
        assert_eq!(display.alpha_revision(), Some(false));
        assert_eq!(Display::new(VmProfile::Pc20160201X64, [640, 480], config).alpha_revision(), None);

        let code = [0x0440u16, 4, 3, 0, 0x0440, 4, 2, 0, 0x22f9, 0x2000];
        let code_len = (code.len() * 2) as u32;
        let mut script = b"PS2A".to_vec();
        for word in [48u32, 0x32083, 0, 0, code_len, 0, 0, 0, 0, code_len, 0] {
            script.extend_from_slice(&word.to_le_bytes());
        }
        script.extend(code.into_iter().flat_map(u16::to_le_bytes).map(|byte| {
            byte.rotate_left(1).wrapping_add(0x7c)
        }));
        let mut vm = CmvsVm::prepare(profile, &MemoryAssetReader(script.into()))?;
        assert!(matches!(vm.run(3)?, VmYield::SystemCall { opcode: 0x22f9, .. }));
        display.alpha_command(&mut vm)?;
        assert_eq!(vm.parameter(4), None);
        assert_eq!(display.alpha_revision(), Some(true));
        assert!(display.menu_checks().contains(&(MENU_ALPHA_REVISION, true)));
        let bytes = display.take_config().expect("changed text config");
        let options = PrivateProfile::parse(std::str::from_utf8(&bytes)?);
        assert_eq!(options.integer("MESSAGE_INFO", "TEXT_REVISION_ALPHA", 0), 1);
        assert_eq!(options.string("MESSAGE_INFO", "FONT"), Some("Arial"));
        Ok(())
    }

    #[test]
    fn cvns_applies_the_new_period_after_the_already_scheduled_deadline() -> Result<()> {
        let profile = VmProfile::Cvns20040515X86;
        let mut display = Display::new(profile, [640, 480], b"");
        let code = [0x0440u16, 4, 33, 0, 0x2001, 0x2002, 0x2003, 0x2000];
        let mut script = b"PSS0".to_vec();
        for word in [48u32, 0, 0, 0, (code.len() * 2) as u32, 0, 0, 0, 0, 0, 0] {
            script.extend_from_slice(&word.to_le_bytes());
        }
        script.extend(code.into_iter().flat_map(u16::to_le_bytes));
        let mut vm = CmvsVm::prepare(profile, &MemoryAssetReader(script.into()))?;
        assert_eq!(display.frame_period(), 17);
        assert_eq!(display.frame_due(100), None);
        assert!(matches!(vm.run(3)?, VmYield::SystemCall { pc: 10, opcode: 0x2002 }));
        display.cvns_set_frame_period(&mut vm)?;
        assert_eq!(display.frame_period(), 33);
        assert_eq!(display.frame_due(116), Some(117));
        assert_eq!(display.frame_due(200), None);
        assert_eq!(display.frame_due(215), Some(216));
        assert!(matches!(vm.run(1)?, VmYield::SystemCall { pc: 12, opcode: 0x2003 }));
        vm.complete_owner_call(0, Some(display.frame_period()))?;
        assert!(matches!(vm.run(1)?, VmYield::Frame));
        Ok(())
    }
}
