// Copyright (c) 2026 CrySirror contributors. MIT.
//! Classic40B0A0/40B0D0/40B1F0 native fast/read/auto policy. Modifiers follow
//!41C720; native411630/40B350 skip is a frame target condition, not a click.
use super::*;

struct MaskedInput {
    held: [bool; 148],
    pressed: [bool; 148],
}
pub(super) struct FastMode {
    force: bool,
    read_skip: bool,
    auto: bool,
    special: bool,
    allow_special: bool,
    allow_menu: bool,
    preference: i32,
    read_flag: Option<bool>,
    ctrl_down: bool,
    shift_down: bool,
    saved_auto: bool,
    caps_lock: bool,
    input_blocked: bool,
    masked: Option<MaskedInput>,
    cursor: [i32; 2],
}
impl Default for FastMode {
    fn default() -> Self {
        Self {
            force: false,
            read_skip: false,
            auto: false,
            special: false,
            allow_special: false,
            allow_menu: false,
            preference: 0,
            read_flag: Some(false),
            ctrl_down: false,
            shift_down: false,
            saved_auto: false,
            caps_lock: false,
            input_blocked: false,
            masked: None,
            cursor: [0, 0],
        }
    }
}
impl FastMode {
    ///40B0D0's switch intentionally leaves force unchanged in mode2.
    fn mode(&mut self, mut mode: u32, neko_menu: i32) {
        self.special = false;
        if mode == 5 {
            mode = if neko_menu != 0 { 1 } else { 4 };
        }
        match mode {
            0 | 7 => {
                self.force = false;
                self.read_skip = false;
                self.auto = false;
            }
            1 if self.preference != 0 => {
                self.force = false;
                self.read_skip = true;
                self.auto = false;
            }
            1 | 4 => {
                self.force = true;
                self.read_skip = false;
                self.auto = false;
            }
            2 => {
                self.read_skip = true;
                self.auto = false;
            }
            3 => {
                self.force = false;
                self.read_skip = false;
                self.auto = true;
            }
            6 => {
                self.force = true;
                self.read_skip = false;
                self.auto = false;
                self.special = true;
            }
            _ => {} //40B0D0's original default only clears the special flag.
        }
    }
    fn mask_bits(&self) -> i32 {
        i32::from(self.force) | i32::from(self.read_skip) << 1 | i32::from(self.auto) << 2
    }
}
impl MajiroVm {
    pub(super) fn fast_mode_diagnostics(&self) -> serde_json::Value {
        json!({"force":self.fast_mode.force,"read_skip":self.fast_mode.read_skip,
            "auto":self.fast_mode.auto,"special":self.fast_mode.special,
            "read_flag":self.fast_mode.read_flag,"input_blocked":self.fast_mode.input_blocked,
            "raw_ctrl":self.fast_mode.ctrl_down,"raw_shift":self.fast_mode.shift_down,
            "caps_lock":self.fast_mode.caps_lock,"preference":self.fast_mode.preference,
            "allow_special":self.fast_mode.allow_special,"allow_menu":self.fast_mode.allow_menu,
            "read_marks":"standard_mjo_session"})
    }
    pub(super) fn refresh_fast_read_flag(&mut self, value: Option<bool>) {
        self.fast_mode.read_flag = value;
    }

    /// Called once after a new raw/source-aware input snapshot. Track physical
    /// modifier boundaries before masking the published native key view.
    pub(super) fn update_fast_mode_input(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.settings.is_none() && self.config.environment.is_some() {
            return Ok(());
        }
        let (ctrl_keydef, neko_menu) = self
            .settings
            .as_ref()
            .map(|s| (s.ctrl_keydef, s.neko_menu))
            .unwrap_or((0, 0));
        let ctrl = self.input.held[17];
        let shift = self.input.held[16];
        // Snapshot-only hosts cannot reorder simultaneous key events. Use the
        // native VK order for equal-frame edges; ordinary modifier edges retain
        //41C720's shared saved-auto word and its exact key-up restoration.
        for (key, down, old) in [
            (16, shift, self.fast_mode.shift_down),
            (17, ctrl, self.fast_mode.ctrl_down),
        ] {
            if down && !old {
                if self.native_layout == crate::native_abi::NativeAbi::Extended {
                    //425460 accumulates the saved auto flag across modifier
                    //key downs; classic41C720 replaces its saved word.
                    self.fast_mode.saved_auto |= self.fast_mode.auto;
                } else {
                    self.fast_mode.saved_auto = self.fast_mode.auto;
                }
                if !self.fast_mode.special {
                    self.fast_mode.mode(
                        if key == 16 {
                            2
                        } else if ctrl_keydef != 0 {
                            1
                        } else {
                            4
                        },
                        neko_menu,
                    );
                }
            } else if !down && old {
                if !self.fast_mode.special {
                    let mode = if self.fast_mode.saved_auto { 3 } else { 0 };
                    self.fast_mode.mode(mode, neko_menu);
                }
                self.fast_mode.saved_auto = false;
            }
        }
        self.fast_mode.ctrl_down = ctrl;
        self.fast_mode.shift_down = shift;
        //41C720/41CF10 cancel ordinary modes on a real left/right press,
        //including their keyboard aliases. A forced skip never sets these
        //press bits, and the native special mode is protected from cancellation.
        if !self.fast_mode.special
            && (self.input.pressed[syscalls::KEY_LEFT_CLICK]
                || self.input.pressed[syscalls::KEY_RIGHT_CLICK])
        {
            self.fast_mode.mode(0, neko_menu);
        }
        let locks = host.keyboard_locks(self.client).map_err(core_error)?;
        self.fast_mode.caps_lock = !self.fast_mode.input_blocked && locks.logical.caps_lock;
        //41D090 replaces the published snapshot each owner update. A disabled
        //snapshot must not resurrect pre-disable press edges when re-enabled.
        self.fast_mode.masked = None;
        if self.fast_mode.input_blocked {
            self.input.held.fill(false);
            self.input.pressed.fill(false);
            self.input.cursor = self.fast_mode.cursor;
        } else {
            self.fast_mode.cursor = self.input.cursor;
        }
        Ok(())
    }

    ///40B1F0 and the owner pass share this predicate. Unknown imported read
    ///profiles fail only when a read-dependent branch actually needs them;
    ///force mode/Ctrl default0 does not fabricate or require read history.
    pub(super) fn fast_skip_active(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.fast_disable_depth != 0 {
            return Ok(false);
        }
        if self.fast_mode.force || self.fast_mode.special {
            return Ok(true);
        }
        if self.fast_mode.caps_lock || self.fast_mode.read_skip {
            let read = self
                .fast_mode
                .read_flag
                .context("Majiro native read-mark profile is not connected")?;
            if !read {
                self.fast_mode.read_skip = false;
                return Ok(false);
            }
            if self.fast_mode.caps_lock && self.input.pressed[syscalls::KEY_RIGHT_CLICK] {
                self.input.take(syscalls::KEY_RIGHT_CLICK);
                self.input.take(syscalls::KEY_LEFT_CLICK);
                self.input.take(syscalls::KEY_MIDDLE);
                let mut state = host
                    .keyboard_locks(self.client)
                    .map_err(core_error)?
                    .logical;
                state.caps_lock = false;
                let committed = host
                    .set_keyboard_locks(self.client, state)
                    .map_err(core_error)?;
                ensure!(
                    committed.client == self.client && !committed.logical.caps_lock,
                    "Majiro fast cancel keyboard completion"
                );
                self.fast_mode.caps_lock = false;
                self.fast_mode.read_skip = false;
                self.api_calls += 1;
                return Ok(false);
            }
            return Ok(true);
        }
        Ok(self.fast_mode.auto && self.auto_mode_config != 0)
    }

    pub(super) fn fast_mode_native(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        if !matches!(
            hash,
            0xa62aa5eb
                | 0x818a4b92
                | 0x9ef1ddc3
                | 0x86f0f608
                | 0xfac4f361
                | 0x4b306c5c
                | 0xbc42ca9a
                | 0x3c84073c
                | 0xc21f8b49
                | 0x38721a26
                | 0xf3d276f1
                | 0x8fe5d6e7
                | 0x45acd4a7
                | 0xc102bcc3
                | 0xc68c011a
                | 0x86e933d1
                | 0x3eabf498
                | 0xf6416e87
                | 0x60085fa4
                | 0x44bc7555
        ) {
            return Ok(None);
        }
        self.refresh_fast_read_flag(m.native_read_flag());
        ensure!(
            a.len() == usize::from(matches!(hash, 0xbc42ca9a | 0x3eabf498)),
            "Majiro fast/read/auto control argument count"
        );
        let neko_menu = self.settings.as_ref().map_or(0, |s| s.neko_menu);
        let value = match hash {
            0xa62aa5eb => i32::from(self.fast_skip_active(host)?),
            0xf6416e87 => i32::from(
                self.fast_mode
                    .read_flag
                    .context("Majiro native read-mark profile is not connected")?,
            ),
            0x818a4b92 | 0x9ef1ddc3 | 0x86f0f608 | 0xfac4f361 | 0x4b306c5c => {
                let mode = match hash {
                    0x818a4b92 => 0,
                    0x9ef1ddc3 => 1,
                    0x86f0f608 => 2,
                    0xfac4f361 => 3,
                    _ => 4,
                };
                self.fast_mode.mode(mode, neko_menu);
                0
            }
            0xbc42ca9a => {
                let bits = a[0].integer()?;
                self.fast_mode.force = bits & 1 != 0;
                self.fast_mode.read_skip = bits & 2 != 0;
                self.fast_mode.auto = bits & 4 != 0;
                0
            }
            0x3c84073c => {
                if self.fast_mode.ctrl_down {
                    0
                } else {
                    self.fast_mode.mask_bits()
                }
            }
            0xc21f8b49 => i32::from(self.fast_mode.auto),
            0x38721a26 => i32::from(self.fast_mode.special),
            0xf3d276f1 => {
                let old = self.fast_mode.special;
                if old {
                    self.fast_mode.mode(7, neko_menu);
                }
                i32::from(old)
            }
            0x8fe5d6e7 => {
                self.fast_mode.allow_special = true;
                0
            }
            0x45acd4a7 => {
                self.fast_mode.allow_special = false;
                0
            }
            0xc68c011a => {
                self.fast_mode.allow_menu = true;
                0
            }
            0x86e933d1 => {
                self.fast_mode.allow_menu = false;
                0
            }
            0xc102bcc3 => {
                if self.fast_mode.allow_special {
                    self.synthesize_native_middle(host.now_ms() as u32);
                    self.fast_mode.mode(6, neko_menu);
                }
                0
            }
            0x3eabf498 => {
                let value = a[0].integer()?;
                //BSS49B024 defaults0, and this native40B2C0 writes the exact
                //REG_SZ key. It is not an ENV-derived FastModeFlag boolean.
                let settings = self
                    .settings
                    .as_ref()
                    .context("Majiro fast preference requires native settings")?;
                let candidate = settings.candidate_integer("FastModeFlg_AR", value, false)?;
                self.settings = Some(candidate);
                self.settings_dirty = true;
                self.fast_mode.preference = value;
                0
            }
            0x60085fa4 => {
                let old = self.fast_mode.input_blocked;
                if !old {
                    self.fast_mode.masked = Some(MaskedInput {
                        held: self.input.held,
                        pressed: self.input.pressed,
                    });
                    self.fast_mode.input_blocked = true;
                    self.input.held.fill(false);
                    self.input.pressed.fill(false);
                }
                i32::from(old) - 1
            }
            0x44bc7555 => {
                self.fast_mode.input_blocked = false;
                if let Some(snapshot) = self.fast_mode.masked.take() {
                    self.input.held = snapshot.held;
                    self.input.pressed = snapshot.pressed;
                }
                0
            }
            _ => unreachable!("preflighted native fast selector"),
        };
        Ok(Some(if returns {
            vec![Value::Integer(value)]
        } else {
            vec![]
        }))
    }
}
