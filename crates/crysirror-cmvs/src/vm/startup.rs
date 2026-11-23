// SPDX-License-Identifier: MPL-2.0
//! Native startup settings and resource registration over the CMVS machine.

use super::{
    abi::SystemResult,
    control::Control,
    evaluate::error,
    gesture::PointerGesture,
    history::History,
    input::Actions,
    machine::{Machine, PreparedScript, Step},
    native_state::NativeState,
    resources::path_string,
    screen::Screen,
    sequence::{Kind as SequenceKind, Sequence},
    DecodeError, PreparedSaveMenu, PreparedSystemSave, ResourceBindings, SaveMenu, SaveMenuRequest,
    SystemSaveRequest, VmProfile,
};
use crate::Script;
use anyhow::Result;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature="runtime")]
pub(crate) mod csp;
mod cvns;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartupSettings {
    pub game_id: Option<u32>,
    pub engine_field_12236: Option<u32>,
    pub archive_mode: Option<u32>,
    pub engine_field_12084: bool,
    pub gestures_enabled: bool,
    pub thumbnail_size: [u32; 2],
    /// `0x23ac`: glyph advances follow the font's cell width.
    pub proportional_glyphs: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmYield {
    BudgetExhausted,
    Frame,
    /// The owner still owes this system call its platform or resource effect.
    SystemCall {
        pc: usize,
        opcode: u16,
    },
}

pub struct CmvsVm {
    machine: Machine,
    control: Control,
    settings: StartupSettings,
    bindings: ResourceBindings,
    cursors: Vec<Vec<u8>>,
    session: u64,
    io_generation: u64,
    save_menu: Option<SaveMenu>,
    second_save_group: bool,
    persistent_lists: [Vec<Vec<u8>>; 2],
    native: NativeState,
    title: Option<Vec<u8>>,
    save_title: Vec<u8>,
    scene_title: Vec<u8>,
    popup_enabled: bool,
    skipping: bool,
    auto_advance: bool,
    message_bypass: bool,
    gesture_options: PointerGesture,
    history: History,
    message_windows: [Option<bool>; 8],
    actions: Actions,
    screen: Screen,
    sequence: Sequence,
    save_information: [Option<u32>; 3],
    /// Object command and slot standing in for a pending message window child command.
    owner_alias: Option<(u16, u32)>,
    cvns: super::cvns::CvnsState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRequest {
    profile: VmProfile,
    session: u64,
    generation: u64,
    caller: u8,
    pc: usize,
    common_slot: Option<u32>,
    reset: bool,
    name: Vec<u8>,
}

impl ScriptRequest {
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    pub fn common_slot(&self) -> Option<u32> {
        self.common_slot
    }

    /// Prepare and validate a replacement image off the VM owner thread.
    pub fn prepare(self, input: &dyn AssetReader) -> Result<PreparedTransfer> {
        let image = PreparedScript::new(self.profile, Script::read(input)?, self.name.clone())?;
        Ok(PreparedTransfer {
            request: self,
            image,
        })
    }
}

pub struct PreparedTransfer {
    request: ScriptRequest,
    image: PreparedScript,
}

impl PreparedTransfer {
    pub fn is_main(&self) -> bool {
        self.request.common_slot.is_none()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn is_reset(&self) -> bool {
        self.request.reset
    }
}

/// Logical VM image at a native checkpoint. It does not contain device state.
pub(crate) struct MainVmCheckpoint {
    #[cfg(feature = "runtime")]
    pub(crate) cvns_records: Vec<u8>,
    machine: Machine,
    _control: Control,
    _second_save_group: bool,
    _save_title: Vec<u8>,
    _scene_title: Vec<u8>,
    _popup_enabled: bool,
    _screen: Screen,
    _gesture: PointerGesture,
    _history: History,
    _settings: StartupSettings,
    _bindings: ResourceBindings,
    _message_windows: [Option<bool>; 8],
    _sequence: Sequence,
    _save_information: [Option<u32>; 3],
    _native: NativeState,
}

impl MainVmCheckpoint {
    pub(crate) fn location(&self) -> (&[u8], usize) {
        (self.machine.current_name(), self.machine.pc())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn snapshot(&self) -> serde_json::Value {
        use super::snapshot::hex;
        serde_json::json!({
            "machine": self.machine.snapshot(),
            "control": self._control.snapshot(),
            "second_save_group": self._second_save_group,
            "save_title": hex(&self._save_title),
            "scene_title": hex(&self._scene_title),
            "popup": self._popup_enabled,
            "screen": self._screen.snapshot(),
            "gesture": self._gesture.snapshot(),
            "history": self._history.snapshot(),
            "windows": self._message_windows,
            "sequence": self._sequence.snapshot(),
            "information": self._save_information,
            "native": self._native.snapshot(),
        })
    }

    pub(crate) fn save_title(&self) -> &[u8] {
        &self._save_title
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn main_expanded(&self) -> Option<Vec<u8>> {
        self.machine.main_expanded()
    }
}

#[cfg(feature = "runtime")]
pub(crate) enum EffectSound {
    Load {
        channel: usize,
        name: String,
        volume: i32,
        looping: bool,
        immediate: bool,
        fade_ms: u32,
    },
    Start {
        channel: usize,
        looping: bool,
    },
    Stop {
        channel: usize,
    },
}

#[cfg(feature = "runtime")]
pub(crate) enum SceneObject {
    Clear,
    Reset {
        slot: usize,
        child: Option<usize>,
        create: bool,
    },
    Load {
        slot: usize,
        name: String,
    },
    Draw {
        slot: usize,
        child: Option<usize>,
        property: DrawProperty,
    },
}

#[cfg(feature = "runtime")]
pub(crate) enum DrawProperty {
    Reset,
    Crop([i16; 4]),
    Position([i32; 2]),
    Origin([i16; 2]),
    Priority(i32),
    Opacity(i32),
    Scale(f32),
    Shown(bool),
    LogicalSize([u32; 2]),
    ClearQuakeFollow,
    Frame { crop: [i16; 4], origin: [i16; 2] },
}

#[cfg(feature = "runtime")]
pub(crate) enum FadeCommand {
    Begin {
        reveal: bool,
        color: [u8; 4],
        duration: u32,
        priority: i32,
    },
    Remove,
    Finish,
    Remaining,
    Wait {
        allow_skip: bool,
    },
}

impl CmvsVm {
    /// Prepare a script for the explicitly selected CMVS ABI on a storage worker.
    pub fn prepare(profile: VmProfile, startup_script: &dyn AssetReader) -> Result<Self> {
        let script = Script::read(startup_script)?;
        let name: &[u8] = match profile {
            VmProfile::Cvns20040515X86 => b"start.pss",
            VmProfile::Pc20070119X86 => b"start.ps2",
            _ => b"start.ps3",
        };
        let startup = PreparedScript::new(profile, script, name.to_vec())?;
        let session = NEXT_SESSION
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| anyhow::anyhow!("CMVS session IDs exhausted"))?;
        // CVNS starts with the popup gate +20440 cleared until 0x2068 sets it.
        let popup_enabled = profile != VmProfile::Cvns20040515X86;
        Ok(Self {
            machine: Machine::new(startup),
            control: Control::default(),
            settings: StartupSettings::default(),
            bindings: ResourceBindings::default(),
            cursors: Vec::new(),
            session,
            io_generation: 0,
            save_menu: (profile == VmProfile::Cvns20040515X86).then(|| SaveMenu {
                slots: Vec::new(),
                latest: None,
            }),
            second_save_group: false,
            persistent_lists: [Vec::new(), Vec::new()],
            native: NativeState::default(),
            title: None,
            save_title: Vec::new(),
            scene_title: Vec::new(),
            popup_enabled,
            skipping: false,
            auto_advance: false,
            message_bypass: false,
            gesture_options: PointerGesture::default(),
            history: if profile == VmProfile::Cvns20040515X86 { History::cvns() } else { History::default() },
            message_windows: [None; 8],
            actions: Actions::default(),
            screen: Screen::default(),
            owner_alias: None,
            sequence: Sequence::default(),
            save_information: [None; 3],
            cvns: super::cvns::CvnsState {
                persistent_start: [4096, 1024, 32],
                version: 0x1_0000,
                ..Default::default()
            },
        })
    }

    pub fn pc(&self) -> usize {
        self.machine.pc()
    }

    pub fn script_name(&self) -> &[u8] {
        self.machine.current_name()
    }

    pub fn settings(&self) -> &StartupSettings {
        &self.settings
    }

    pub fn resources(&self) -> &ResourceBindings {
        &self.bindings
    }

    pub fn save_menu(&self) -> Option<&SaveMenu> {
        self.save_menu.as_ref()
    }

    pub fn persistent_lists(&self) -> &[Vec<Vec<u8>>; 2] {
        &self.persistent_lists
    }

    /// Image loads `0x2030`, `0x2032`, `0x2035`, `0x2037` and `0x203b` record their name
    /// after the load attempt, whether or not it succeeded.
    #[cfg(feature = "runtime")]
    pub(crate) fn record_seen_image(&mut self, name: &str) {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(name);
        self.native.record(
            &mut self.persistent_lists,
            super::native_state::SEEN_IMAGE,
            &bytes,
        );
    }

    pub fn condition(&self) -> Option<bool> {
        self.machine.state().condition
    }

    /// Cursor file registered by `0x2098`, numbered from 1 as `0x2099` selects it.
    #[cfg(feature = "runtime")]
    pub(crate) fn cursor_name(&self, index: u32) -> Option<&[u8]> {
        self.cursors.get(usize::try_from(index).ok()?.checked_sub(1)?).map(Vec::as_slice)
    }

    pub(crate) fn bit(&self, index: u32) -> Option<bool> {
        self.machine.state().bit(index)
    }

    pub fn integer(&self, index: usize) -> Option<u32> {
        self.machine
            .state()
            .integer(u32::try_from(index).ok()?)
            .map(|n| n as u32)
    }

    /// The script in the main slot (engine +14616).
    #[cfg(feature = "runtime")]
    pub(crate) fn main_script_name(&self) -> &[u8] {
        self.machine.main_name()
    }

    pub fn text_reference(&self, reference: u32) -> Option<&[u8]> {
        self.machine.resolve_string(reference).ok()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn cameras(&self) -> [[u32; 40]; 2] {
        self.screen.cameras()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn set_camera_position(&mut self, position: [f32; 3]) {
        self.screen.set_camera_position(position);
    }

    /// `0x2287` steps: eases camera 0's eye to `target` over `duration` ms.
    #[cfg(feature = "runtime")]
    pub(crate) fn tween_camera(&mut self, mode: u32, target: [f32; 3], duration: u32) {
        self.screen.tween_camera(mode, target, duration);
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn sequence_mut(&mut self) -> &mut Sequence {
        &mut self.sequence
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn camera_tween_active(&self) -> bool {
        self.screen.tween_active()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn finish_camera_tween(&mut self) {
        self.screen.finish_tween();
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn advance_camera_tween(&mut self, delta: u32) {
        self.screen.advance_tween(delta);
    }

    pub fn profile(&self) -> VmProfile {
        self.machine.profile()
    }

    pub fn parameter(&self, distance: usize) -> Option<u32> {
        self.machine.stack().peek_u32(distance).ok()
    }

    pub fn pending_title(&self) -> Result<Option<String>, DecodeError> {
        let Some((pc, command @ (0x160 | 0x161))) = self.machine.pending_system() else {
            return Ok(None);
        };
        let opcode = 0x2000 + command;
        let value = self.required_parameter(opcode, 4)?;
        let bytes = self.machine.resolve_string(value)?;
        if bytes.len() > 255 {
            return Err(error(pc, opcode, "CMVS native title byte limit"));
        }
        let title = if command == 0x161 {
            let prefix = self
                .title
                .as_deref()
                .ok_or_else(|| error(pc, opcode, "CMVS game title unavailable"))?;
            if prefix.len() + 2 + bytes.len() >= 128 {
                return Err(error(pc, opcode, "CMVS combined title limit"));
            }
            [prefix, &[0x81, 0x40], bytes].concat()
        } else {
            bytes.to_vec()
        };
        let (title, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&title);
        if errors {
            return Err(error(pc, opcode, "invalid CMVS title encoding"));
        }
        Ok(Some(title.into_owned()))
    }

    pub fn complete_title(&mut self) -> Result<(), DecodeError> {
        let Some((pc, command @ (0x160 | 0x161))) = self.machine.pending_system() else {
            return Err(error(self.pc(), 0x2160, "CMVS title request missing"));
        };
        let value = self.required_parameter(0x2000 + command, 4)?;
        let bytes = self.machine.resolve_string(value)?.to_vec();
        self.pending_title()?
            .ok_or_else(|| error(pc, 0x2160, "CMVS title request missing"))?;
        self.machine.complete_system(SystemResult(0x4004))?;
        if command == 0x160 {
            self.title = Some(bytes);
        } else {
            self.scene_title = bytes;
        }
        Ok(())
    }

    pub fn pending_icon(&self) -> Result<Option<VfsPath>, DecodeError> {
        let Some((pc, 0x8f)) = self.machine.pending_system() else {
            return Ok(None);
        };
        let raw = self.required_parameter(0x208f, 4)?;
        let path = path_string(self.machine.resolve_string(raw)?)
            .map_err(|_| error(pc, 0x208f, "invalid CMVS icon filename encoding"))?;
        VfsPath::new(Mount::Asset, &path.replace('\\', "/"))
            .map(Some)
            .map_err(|_| error(pc, 0x208f, "invalid scoped CMVS icon filename"))
    }

    pub fn complete_icon(&mut self) -> Result<(), DecodeError> {
        if self.pending_icon()?.is_none() {
            return Err(error(self.pc(), 0x208f, "CMVS icon request missing"));
        }
        self.machine.complete_system(SystemResult(0x4004))?;
        Ok(())
    }

    pub fn pending_script(&self) -> Result<Option<ScriptRequest>, DecodeError> {
        let Some((pc, command @ (0x03 | 0x80 | 0x81 | 0x805 | 0x810 | 0x812))) =
            self.machine.pending_system()
        else {
            return Ok(None);
        };
        let opcode = 0x2000 + command;
        // CVNS 0x2010 (0x4317c0) replaces the main image and 0x2012 (0x42c3c0)
        // calls into image slot 1; both read the name at SP-4.
        let (common_slot, distance) = if command == 0x812 {
            (Some(0), 4)
        } else if command == 0x81 {
            let slot = self.required_parameter(opcode, 4)?;
            if slot >= 3 {
                return Err(error(pc, opcode, "common script slot outside 0..3"));
            }
            (Some(slot), 8)
        } else {
            (None, 4)
        };
        let name = if command == 0x03 {
            if self.machine.profile() == VmProfile::Pc20070119X86 { &b"main.ps2"[..] } else { &b"main.ps3"[..] }
        } else if command == 0x805 {
            b"main.pss"
        } else {
            let reference = self.required_parameter(opcode, distance)?;
            self.machine.resolve_string(reference)?
        };
        if name.is_empty() || name.len() >= if command >= 0x800 { 32 } else { 64 } {
            return Err(error(pc, opcode, "script basename exceeds native bounds"));
        }
        Ok(Some(ScriptRequest {
            profile: self.machine.profile(),
            session: self.session,
            generation: self.io_generation,
            caller: self.machine.current_slot(),
            pc,
            common_slot,
            reset: matches!(command, 0x03 | 0x805),
            name: name.to_vec(),
        }))
    }

    /// Install a prepared common script only while its native request is live.
    pub fn complete_common_script(
        &mut self,
        prepared: PreparedTransfer,
    ) -> Result<(), DecodeError> {
        let pending = self.pending_script()?;
        if pending.as_ref() != Some(&prepared.request) {
            return Err(error(self.pc(), 0x2081, "stale CMVS script preparation"));
        }
        let slot = prepared.request.common_slot.ok_or_else(|| {
            error(
                self.pc(),
                0x2080,
                "main switch needs native snapshot services",
            )
        })?;
        let generation = self.next_io_generation(0x2081)?;
        self.machine.enter_common(slot, prepared.image)?;
        self.io_generation = generation;
        Ok(())
    }

    /// Commit a main transfer once its owner has captured the shared scene.
    pub(crate) fn complete_main_script(
        &mut self,
        prepared: PreparedTransfer,
        cvns_scene: Vec<u8>,
    ) -> Result<MainVmCheckpoint, DecodeError> {
        if !prepared.is_main()
            || prepared.request.reset
            || self.pending_script()?.as_ref() != Some(&prepared.request)
        {
            return Err(error(
                self.pc(),
                0x2080,
                "stale CMVS main-script preparation",
            ));
        }
        let generation = self.next_io_generation(0x2080)?;
        let name = prepared.request.name.clone();
        let mut candidate = self.machine.clone();
        candidate.enter_main(prepared.image)?;
        let mut control = self.control.clone();
        control.replace_main();
        let checkpoint = self.vm_checkpoint(candidate.clone(), control.clone(), 0x2080, cvns_scene)?;
        candidate
            .state_mut()
            .fill_bits(0, 1, false)
            .map_err(|why| error(self.pc(), 0x2080, why))?;
        self.machine = candidate;
        self.control = control;
        self.io_generation = generation;
        // 0x140085080 records the name of every main script it loads.
        self.native
            .record(&mut self.persistent_lists, super::native_state::SEEN_SCRIPT, &name);
        Ok(checkpoint)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn validate_reset(&self, prepared: &PreparedTransfer) -> Result<(), DecodeError> {
        if !prepared.request.reset || self.pending_script()?.as_ref() != Some(&prepared.request) {
            return Err(error(self.pc(), 0x2003, "stale CMVS reset preparation"));
        }
        self.next_io_generation(0x2003)?;
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_reset(&mut self, prepared: PreparedTransfer) -> Result<(), DecodeError> {
        self.validate_reset(&prepared)?;
        let name = prepared.request.name.clone();
        self.machine.enter_main(prepared.image)?;
        self.native
            .record(&mut self.persistent_lists, super::native_state::SEEN_SCRIPT, &name);
        if self.profile() == VmProfile::Cvns20040515X86 {
            // 0x431070 keeps the banks, configuration, callback ownership and
            // history capacity; it clears the linked history entries.
            self.control.replace_main();
            self.history.clear_entries();
            self.message_windows = [None; 8];
            self.skipping = false;
            self.auto_advance = false;
            self.io_generation += 1;
            return Ok(());
        }
        // 0x140084960 sets the slot-save flag again on reset.
        self.native.set_saved_flag(true);
        self.control.replace_main();
        self.control.reset_frame_delta();
        self.screen.reset();
        self.history = History::default();
        self.message_windows = [None; 8];
        self.sequence.reset();
        self.skipping = false;
        self.auto_advance = false;
        self.popup_enabled = true;
        self.io_generation += 1;
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_scene_checkpoint(&mut self, cvns_scene: Vec<u8>) -> Result<MainVmCheckpoint, DecodeError> {
        let pc = self.pc();
        if self.machine.pending_system() != Some((pc, 0x2b2)) {
            return Err(error(pc, 0x22b2, "CMVS scene checkpoint request missing"));
        }
        let mut candidate = self.machine.clone();
        candidate.complete_system(SystemResult(0x4000))?;
        let checkpoint = self.vm_checkpoint(candidate.clone(), self.control.clone(), 0x22b2, cvns_scene)?;
        candidate
            .state_mut()
            .fill_bits(0, 1, false)
            .map_err(|why| error(pc, 0x22b2, why))?;
        self.machine = candidate;
        Ok(checkpoint)
    }

    fn vm_checkpoint(
        &self,
        mut machine: Machine,
        control: Control,
        opcode: u16,
        _cvns_scene: Vec<u8>,
    ) -> Result<MainVmCheckpoint, DecodeError> {
        if machine.profile() != VmProfile::Cvns20040515X86 {
            machine
                .state_mut()
                .fill_bits(0, 1, true)
                .map_err(|why| error(self.pc(), opcode, why))?;
        }
        #[cfg(feature = "runtime")]
        let cvns_records = if machine.profile() == VmProfile::Cvns20040515X86 {
            let capture = || -> anyhow::Result<Vec<u8>> {
                let mut records = _cvns_scene;
                for (tag,title) in [(0x100,&self.save_title),(0xff,&self.scene_title)] {
                    let mut name=title.clone();
                    name.push(0);
                    super::save_slot::csp1::append(&mut records,tag,&[],&name)?;
                }
                super::save_slot::csp1::append(&mut records,0x204,&[],&u32::from(self.popup_enabled).to_le_bytes())?;
                super::save_slot::csp1::append(&mut records,0x10a,&[],&u32::from(self.second_save_group).to_le_bytes())?;
                control.csp_records(&mut records)?;
                machine.csp_execution(&mut records)?;
                super::csp_banks::capture(machine.state(), self.cvns.persistent_start, &mut records)?;
                self.history.csp_record(&mut records)?;
                super::save_slot::csp1::ordered(&records)
            };
            capture().map_err(|_| error(self.pc(), opcode, "CVNS native execution checkpoint exceeds its layout"))?
        } else { Vec::new() };
        Ok(MainVmCheckpoint {
            #[cfg(feature = "runtime")]
            cvns_records,
            machine,
            _control: control,
            _second_save_group: self.second_save_group,
            _save_title: self.save_title.clone(),
            _scene_title: self.scene_title.clone(),
            _popup_enabled: self.popup_enabled,
            _screen: self.screen.clone(),
            _gesture: self.gesture_options.clone(),
            _history: self.history.checkpoint(),
            _settings: self.settings.clone(),
            _bindings: self.bindings.clone(),
            _message_windows: self.message_windows,
            _sequence: self.sequence.clone(),
            _save_information: self.save_information,
            _native: self.native.clone(),
        })
    }

    pub fn pending_save_menu(&self) -> Result<Option<SaveMenuRequest>, DecodeError> {
        let Some((pc, 0x2b3)) = self.machine.pending_system() else {
            return Ok(None);
        };
        let raw_count = self.required_parameter(0x22b3, 4)? as i32;
        let count = if raw_count < 0 {
            10
        } else {
            raw_count.min(256) as u16
        };
        let root = self
            .bindings
            .save_suffix()
            .cloned()
            .ok_or_else(|| error(pc, 0x22b3, "CMVS save root is not registered"))?;
        Ok(Some(SaveMenuRequest {
            session: self.session,
            generation: self.io_generation,
            pc,
            root,
            count,
        }))
    }

    /// Commit only after the owner publishes the prepared scan to its menu.
    pub fn complete_save_menu(&mut self, prepared: PreparedSaveMenu) -> Result<(), DecodeError> {
        if self.pending_save_menu()?.as_ref() != Some(&prepared.request) {
            return Err(error(self.pc(), 0x22b3, "stale CMVS save-menu preparation"));
        }
        let generation = self.next_io_generation(0x22b3)?;
        self.machine.complete_system(SystemResult(0x4004))?;
        self.save_menu = Some(prepared.menu);
        self.io_generation = generation;
        Ok(())
    }

    /// Rescan of the native load/save menu after `0x228c..=0x228e` change slot files
    /// below ten (0x1400aa140 rebuilds the menu from the slot headers).
    #[cfg(feature = "runtime")]
    pub(crate) fn save_menu_rescan(&self) -> Option<SaveMenuRequest> {
        let menu = self.save_menu.as_ref()?;
        Some(SaveMenuRequest {
            session: self.session,
            generation: self.io_generation,
            pc: usize::MAX,
            root: self.bindings.save_suffix().cloned()?,
            count: menu.slots.len() as u16,
        })
    }

    /// Replaces the menu scan unless a later save-menu load superseded the rescan.
    #[cfg(feature = "runtime")]
    pub(crate) fn refresh_save_menu(&mut self, prepared: PreparedSaveMenu) -> bool {
        let current = prepared.request.session == self.session && prepared.request.generation == self.io_generation;
        if current {
            self.save_menu = Some(prepared.menu);
        }
        current
    }

    pub fn pending_save_group(&self) -> Result<Option<bool>, DecodeError> {
        if matches!(self.machine.pending_system(), Some((_, 0x2b4))) {
            Ok(Some(self.required_parameter(0x22b4, 4)? != 0))
        } else {
            Ok(None)
        }
    }

    pub fn menu_options(&self) -> (bool, bool) {
        (self.popup_enabled, self.second_save_group)
    }

    pub fn skipping(&self) -> bool {
        self.skipping || self.actions.held_skip()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn prepare_message_read(
        &mut self,
        id: u32,
        only_read: bool,
    ) -> Result<bool, DecodeError> {
        let read = if only_read && id as i32 >= 0 {
            self.machine
                .state()
                .bit(id)
                .ok_or_else(|| error(self.pc(), 0x2152, "CMVS message read bit outside bank"))?
        } else {
            true
        };
        if !read {
            self.skipping = false;
        }
        Ok(read || self.message_bypass)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn message_modes(&self) -> [bool; 4] {
        [
            self.skipping,
            self.auto_advance,
            self.message_bypass,
            self.gesture_options.holding(),
        ]
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn repeats_system(&self) -> bool {
        self.machine.repeats_system()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn configure_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.actions.configure(bytes)
    }
    #[cfg(feature = "runtime")]
    pub(crate) fn configure_canvas(&mut self, size: [u32; 2]) {
        self.screen.set_size(size);
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn set_message_window(&mut self, index: usize, status: Option<bool>) {
        self.message_windows[index] = status;
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn image_name(&self, opcode: u16, reference: u32) -> Result<String, DecodeError> {
        let name = self.machine.resolve_string(reference)?;
        if name.len() >= 64 {
            return Err(error(self.pc(), opcode, "CMVS image name byte limit"));
        }
        path_string(name).map_err(|_| error(self.pc(), opcode, "CMVS image name encoding"))
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn admit_input(&mut self, input: crysirror_core::service::InputFrame) {
        self.actions.admit(input);
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn suspend_frame(&mut self) {
        self.actions.suspend();
        self.control.reset_frame_delta();
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn take_fade_confirm(&mut self) -> bool {
        self.actions.confirm_released()
    }

    pub fn pending_popup(&self) -> Result<Option<bool>, DecodeError> {
        if matches!(self.machine.pending_system(), Some((_, 0x2cd))) {
            Ok(Some(self.required_parameter(0x22cd, 4)? != 0))
        } else {
            Ok(None)
        }
    }

    pub fn complete_popup(&mut self, enabled: bool) -> Result<(), DecodeError> {
        if self.pending_popup()? != Some(enabled) {
            return Err(error(self.pc(), 0x22cd, "CMVS popup request differs"));
        }
        self.machine.complete_system(SystemResult(0x4004))?;
        self.popup_enabled = enabled;
        Ok(())
    }

    pub fn pending_clear_color(&self) -> Result<Option<[u8; 4]>, DecodeError> {
        if !matches!(self.machine.pending_system(), Some((_, 0x2cb))) {
            return Ok(None);
        }
        let [_, r, g, b] = self.required_parameter(0x22cb, 4)?.to_be_bytes();
        Ok(Some([r, g, b, 255]))
    }

    pub fn complete_clear_color(&mut self) -> Result<(), DecodeError> {
        if self.pending_clear_color()?.is_none() {
            return Err(error(self.pc(), 0x22cb, "CMVS clear color request missing"));
        }
        self.machine.complete_system(SystemResult(0x4004))?;
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn pending_effect_sound(&self) -> Result<EffectSound, DecodeError> {
        let Some((pc, command @ (0xb0 | 0xb1 | 0xb3 | 0xba | 0x8c5))) = self.machine.pending_system()
        else {
            return Err(error(
                self.pc(),
                0x20b0,
                "CMVS effect sound request missing",
            ));
        };
        let opcode = 0x2000 + command;
        let channel = self.required_parameter(opcode, 4)? as usize;
        let cvns = self.machine.profile() == VmProfile::Cvns20040515X86;
        let sample = command == 0x8c5;
        let limit = if sample { 4 } else if cvns { 48 } else { 6 };
        if channel >= limit {
            return Err(error(pc, opcode, "effect channel outside the native table"));
        }
        Ok(match command {
            0xb0 | 0xba | 0x8c5 => {
                let name = self
                    .machine
                    .resolve_string(self.required_parameter(opcode, 8)?)?;
                // CVNS keeps 64-byte names at +2768 + 64 * channel.
                if !sample && name.len() >= if cvns { 64 } else { 32 } {
                    return Err(error(pc, opcode, "CMVS effect sound name byte limit"));
                }
                let name = path_string(name)
                    .map_err(|_| error(pc, opcode, "CMVS effect sound name encoding"))?;
                EffectSound::Load {
                    channel: if sample { channel + 48 } else { channel },
                    name,
                    volume: self.required_parameter(opcode, 12)? as i32,
                    looping: self
                        .required_parameter(opcode, if command == 0xb0 { 16 } else { 20 })?
                        != 0,
                    immediate: self
                        .required_parameter(opcode, if command == 0xb0 { 20 } else { 24 })?
                        != 0,
                    fade_ms: if command != 0xb0 {
                        self.required_parameter(opcode, 16)?
                    } else {
                        0
                    },
                }
            }
            0xb1 => EffectSound::Start {
                channel,
                looping: self.required_parameter(opcode, 8)? != 0,
            },
            _ => EffectSound::Stop { channel },
        })
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_effect_sound(
        &mut self,
        loaded: Option<bool>,
    ) -> Result<(), DecodeError> {
        let cleanup = match self.pending_effect_sound()? {
            EffectSound::Load { .. } => {
                if matches!(self.machine.pending_system(), Some((_, 0xba | 0x8c5))) {
                    24
                } else {
                    20
                }
            }
            EffectSound::Start { .. } => 8,
            EffectSound::Stop { .. } => 4,
        };
        let pc = self.pc();
        self.machine
            .complete_system(SystemResult(0x4000 | cleanup))?;
        if let Some(loaded) = loaded {
            self.machine
                .state_mut()
                .set_system_integer(0, u32::from(loaded))
                .map_err(|why| error(pc, 0x20b0, why))?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn pending_scene_object(&self) -> Result<(SceneObject, u16), DecodeError> {
        let Some((pc, command @ (0x20..=0x23 | 0x2f | 0x30 | 0x40 | 0x44..=0x48 | 0x50))) =
            self.machine.pending_system()
        else {
            return Err(error(
                self.pc(),
                0x2020,
                "CMVS scene object request missing",
            ));
        };
        if command == 0x2f {
            return Ok((SceneObject::Clear, 0));
        }
        let opcode = 0x2000 + command;
        let raw = |distance| self.required_parameter(opcode, distance);
        let slot = raw(4)? as usize;
        if slot >= 256 {
            return Err(error(pc, opcode, "CMVS scene object outside 0..255"));
        }
        let child = if matches!(command, 0x22 | 0x23 | 0x44..=0x48 | 0x50) {
            let value = raw(8)? as i32;
            if value < 0 && matches!(command, 0x44..=0x48) {
                None
            } else if (0..768).contains(&value) {
                Some(value as usize)
            } else {
                return Err(error(pc, opcode, "CMVS child index outside 0..767"));
            }
        } else {
            None
        };
        Ok(match command {
            0x20..=0x23 => (
                SceneObject::Reset {
                    slot,
                    child,
                    create: matches!(command, 0x20 | 0x22),
                },
                if child.is_some() { 8 } else { 4 },
            ),
            0x30 => {
                let value = self.machine.resolve_string(raw(8)?)?;
                if value.len() >= 64 {
                    return Err(error(pc, opcode, "CMVS scene image name byte limit"));
                }
                let name = path_string(value)
                    .map_err(|_| error(pc, opcode, "CMVS scene image name encoding"))?;
                (SceneObject::Load { slot, name }, 8)
            }
            0x40 | 0x50 => (
                SceneObject::Draw {
                    slot,
                    child,
                    property: DrawProperty::Reset,
                },
                if child.is_some() { 8 } else { 4 },
            ),
            _ => {
                let (property, bytes) = match command {
                    0x44 => (
                        DrawProperty::Crop([
                            raw(12)? as i16,
                            raw(16)? as i16,
                            raw(20)? as i16,
                            raw(24)? as i16,
                        ]),
                        24,
                    ),
                    0x45 => (
                        DrawProperty::Position([raw(12)? as i32, raw(16)? as i32]),
                        16,
                    ),
                    0x46 => (DrawProperty::Origin([raw(12)? as i16, raw(16)? as i16]), 16),
                    0x47 => (DrawProperty::Priority(raw(12)? as i32), 12),
                    _ => (DrawProperty::Opacity(raw(12)? as i32), 12),
                };
                (
                    SceneObject::Draw {
                        slot,
                        child,
                        property,
                    },
                    bytes,
                )
            }
        })
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_scene_object(
        &mut self,
        loaded: Option<bool>,
    ) -> Result<(), DecodeError> {
        let (_, cleanup) = self.pending_scene_object()?;
        let pc = self.pc();
        self.machine
            .complete_system(SystemResult(0x4000 | cleanup))?;
        if let Some(loaded) = loaded {
            self.machine
                .state_mut()
                .set_system_integer(0, u32::from(loaded))
                .map_err(|why| error(pc, 0x2030, why))?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn store_owner_string(&mut self, raw: u32, bytes: &[u8]) -> Result<(), DecodeError> {
        self.machine.store_string(raw, bytes)
    }

    pub(crate) fn owner_arguments<const N: usize>(
        &self,
        opcode: u16,
    ) -> Result<[u32; N], DecodeError> {
        let alias = self.owner_alias.filter(|(command, _)| *command == opcode);
        if alias.is_none()
            && !matches!(self.machine.pending_system(), Some((_, op)) if op + 0x2000 == opcode)
        {
            return Err(error(self.pc(), opcode, "CMVS owner command differs"));
        }
        let mut args = [0; N];
        for (index, word) in args.iter_mut().enumerate() {
            *word = self.required_parameter(opcode, (index + 1) * 4)?;
        }
        if let (Some((_, slot)), Some(first)) = (alias, args.first_mut()) {
            *first = slot;
        }
        Ok(args)
    }

    /// Native file of a save slot under the registered save root.
    #[cfg(feature = "runtime")]
    pub(crate) fn save_slot_path(&self, slot: u32) -> Result<VfsPath, DecodeError> {
        let pc = self.pc();
        let (base, directory) = self
            .bindings
            .save_file_base()
            .ok_or_else(|| error(pc, 0x22b6, "CMVS save root is not registered"))?;
        let name = format!("save{:03}.dat", slot % 1000);
        let path = if directory {
            format!("{}/{name}", base.relative())
        } else {
            format!("{}{name}", base.relative())
        };
        VfsPath::new(Mount::Save, &path).map_err(|_| error(pc, 0x22b6, "invalid CMVS slot path"))
    }

    /// Replaces the running VM with a saved checkpoint at its next instruction.
    #[cfg(feature = "runtime")]
    pub(crate) fn restore_checkpoint(
        &mut self,
        value: &serde_json::Value,
        scripts: super::SavedScripts,
    ) -> Option<()> {
        use super::snapshot::bytes;
        let machine = Machine::from_snapshot(&value["machine"], scripts.0)?;
        let control = Control::from_snapshot(&value["control"])?;
        let screen = Screen::from_snapshot(&value["screen"])?;
        let gesture = PointerGesture::from_snapshot(&value["gesture"])?;
        let history = History::from_snapshot(&value["history"])?;
        let save_title = bytes(value, "save_title")?;
        let scene_title = bytes(value, "scene_title")?;
        let sequence = Sequence::from_snapshot(&value["sequence"])?;
        let mut windows = [None; 8];
        for (slot, saved) in windows.iter_mut().zip(value["windows"].as_array()?) {
            *slot = saved.as_bool();
        }
        let mut information = [None; 3];
        for (slot, saved) in information.iter_mut().zip(value["information"].as_array()?) {
            *slot = saved.as_u64().map(|word| word as u32);
        }
        self.machine = machine;
        self.control = control;
        self.control.reset_frame_delta();
        self.screen = screen;
        self.gesture_options = gesture;
        self.history = history;
        self.second_save_group = value["second_save_group"].as_bool()?;
        self.save_title = save_title;
        self.scene_title = scene_title;
        self.popup_enabled = value["popup"].as_bool()?;
        self.message_windows = windows;
        self.sequence = sequence;
        self.save_information = information;
        if let Some(mut native) = NativeState::from_snapshot(&value["native"]) {
            // 0x14007e210 sets the slot-save flag after reading a slot.
            native.set_saved_flag(true);
            self.native = native;
        }
        self.skipping = false;
        self.auto_advance = false;
        self.owner_alias = None;
        self.io_generation += 1;
        Some(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn set_owner_alias(&mut self, alias: Option<(u16, u32)>) {
        self.owner_alias = alias;
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn owner_argument(&self, opcode: u16, distance: usize) -> Result<u32, DecodeError> {
        self.owner_arguments::<0>(opcode)?;
        self.required_parameter(opcode, distance)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn sound_name(&self, raw: u32) -> Result<Option<String>, DecodeError> {
        let name = self.machine.resolve_string(raw)?;
        let opcode = self
            .machine
            .pending_system()
            .map_or(0, |(_, op)| op + 0x2000);
        if self.profile() == VmProfile::Cvns20040515X86 && opcode == 0x28b8 {
            if name.is_empty() || name.len() >= 64 {
                return Ok(None);
            }
        } else if name.is_empty() || (matches!(opcode, 0x2240 | 0x2248) && name.len() >= 32) {
            return Ok(None);
        } else if name.len() >= 32 {
            return Err(error(self.pc(), opcode, "CMVS sound name byte limit"));
        }
        path_string(name)
            .map(Some)
            .map_err(|_| error(self.pc(), opcode, "CMVS sound name encoding"))
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn movie_path(&self) -> Result<VfsPath, DecodeError> {
        let opcode = self.machine.pending_system().map_or(0x20c0, |(_, op)| op + 0x2000);
        let [reference] = self.owner_arguments(opcode)?;
        self.movie_path_for(reference)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn movie_path_for(&self, reference: u32) -> Result<VfsPath, DecodeError> {
        let name = self.machine.resolve_string(reference)?;
        let pc = self.pc();
        let name =
            path_string(name).map_err(|_| error(pc, 0x20c0, "CMVS movie filename encoding"))?;
        let root = self
            .bindings
            .movie_root()
            .ok_or_else(|| error(pc, 0x20c0, "CMVS movie root missing"))?;
        let basename = VfsPath::new(Mount::Asset, &name)
            .map_err(|_| error(pc, 0x20c0, "invalid CMVS movie filename"))?;
        if basename.relative() != name || name.contains('/') {
            return Err(error(pc, 0x20c0, "CMVS movie requires a basename"));
        }
        VfsPath::new(Mount::Asset, &format!("{}/{name}", root.relative()))
            .map_err(|_| error(pc, 0x20c0, "invalid CMVS movie path"))
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_owner_call(
        &mut self,
        words: u16,
        result: Option<u32>,
    ) -> Result<(), DecodeError> {
        self.finish_owner_call(words, result, false)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_slot_metadata(
        &mut self,
        metadata: Option<&super::SlotMetadata>,
        publish: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let cvns = self.profile() == VmProfile::Cvns20040515X86;
        let (opcode, words) = if cvns { (0x29ac, 4) } else { (0x22bd, 5) };
        let Some(metadata) = metadata else {
            return Ok(self.complete_owner_call(words, Some(0))?);
        };
        let timestamp = self.owner_argument(opcode, usize::from(words - 1) * 4)?;
        let comment = self.owner_argument(opcode, usize::from(words) * 4)?;
        let mut candidate = self.machine.clone();
        candidate.write_string_reference(opcode, timestamp, &metadata.timestamp)?;
        candidate.write_string_reference(opcode, comment, &metadata.comment)?;
        candidate.complete_system(SystemResult(0x4000 | words * 4))?;
        let [first, second, third] = metadata.values;
        for (index, value) in [
            (0, 1),
            (1, metadata.date_result),
            (4, first),
            (5, second),
            (6, third),
        ] {
            candidate
                .state_mut()
                .set_system_integer(index, value)
                .map_err(|why| error(self.pc(), opcode, why))?;
        }
        publish()?;
        self.machine = candidate;
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_font_face_query(&mut self, face: &[u8]) -> Result<(), DecodeError> {
        let [_, destination] = self.owner_arguments(0x22de)?;
        self.machine
            .write_string_reference(0x22de, destination, face)?;
        self.complete_owner_call(2, Some(1))
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_voice(&mut self, words: u16, loaded: bool) -> Result<(), DecodeError> {
        self.complete_owner_call(words, Some(u32::from(loaded)))?;
        // The unit of the original MOG_GetPcmLength result is still unverified.
        if self.profile() != VmProfile::Cvns20040515X86 {
            self.machine.state_mut().forget_system_integer(4);
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn finish_owner_call(
        &mut self,
        words: u16,
        result: Option<u32>,
        frame: bool,
    ) -> Result<(), DecodeError> {
        let pc = self.pc();
        let flags = if frame { 0xc000 } else { 0x4000 };
        self.machine
            .complete_system(SystemResult(flags | (4 * words)))?;
        if let Some(value) = result {
            self.machine
                .state_mut()
                .set_system_integer(0, value)
                .map_err(|why| error(pc, 0, why))?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn wait_owner_call(
        &mut self,
        opcode: u16,
        result: Option<u32>,
    ) -> Result<(), DecodeError> {
        let pc = self.pc();
        self.machine.wait_system(opcode - 0x2000)?;
        if let Some(value) = result {
            self.machine
                .state_mut()
                .set_system_integer(0, value)
                .map_err(|why| error(pc, opcode, why))?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_owner_results(
        &mut self,
        words: u16,
        results: &[(u32, u32)],
    ) -> Result<(), DecodeError> {
        self.complete_owner_typed_results(words, results, &[])
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_owner_typed_results(
        &mut self,
        words: u16,
        results: &[(u32, u32)],
        floats: &[(u32, u32)],
    ) -> Result<(), DecodeError> {
        let pc = self.pc();
        self.machine
            .complete_system(SystemResult(0x4000 | (4 * words)))?;
        for &(index, value) in results {
            self.machine
                .state_mut()
                .set_system_integer(index, value)
                .map_err(|why| error(pc, 0, why))?;
        }
        for &(index, value) in floats {
            self.machine
                .state_mut()
                .set_system_float_bits(index, value)
                .map_err(|why| error(pc, 0, why))?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn action(&self, index: usize) -> [bool; 3] {
        self.actions.state(index)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn consume_action(&mut self, index: usize) {
        self.actions.consume(index);
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn pointer(&self) -> [i32; 2] {
        self.actions.pointer()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn move_pointer(&mut self, pointer: [i32; 2]) {
        self.actions.move_pointer(pointer);
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn pending_fade(&self) -> Result<FadeCommand, DecodeError> {
        let Some((pc, command @ 0x220..=0x224)) = self.machine.pending_system() else {
            return Err(error(self.pc(), 0x2220, "CMVS fade request missing"));
        };
        let opcode = 0x2000 + command;
        let argument = |distance| self.required_parameter(opcode, distance);
        Ok(match command {
            0x220 => {
                let direction = argument(4)?;
                if direction > 1 {
                    return Err(error(pc, opcode, "CMVS fade direction outside 0..1"));
                }
                let [_, r, g, b] = argument(8)?.to_be_bytes();
                FadeCommand::Begin {
                    reveal: direction == 1,
                    color: [r, g, b, 255],
                    duration: argument(12)?,
                    priority: argument(16)? as i32,
                }
            }
            0x221 => FadeCommand::Remove,
            0x222 => FadeCommand::Finish,
            0x223 => FadeCommand::Remaining,
            _ => FadeCommand::Wait {
                allow_skip: argument(4)? != 0,
            },
        })
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_fade(
        &mut self,
        result: Option<u32>,
        wait: bool,
        yield_frame: bool,
    ) -> Result<(), DecodeError> {
        let cleanup = match self.pending_fade()? {
            FadeCommand::Begin { .. } => 16,
            FadeCommand::Wait { .. } => 4,
            _ => 0,
        };
        let pc = self.pc();
        if wait {
            self.machine.wait_system(0x224)?;
        } else {
            let flags = if yield_frame { 0xc000 } else { 0x4000 };
            self.machine
                .complete_system(SystemResult(flags | cleanup))?;
        }
        if let Some(result) = result {
            self.machine
                .state_mut()
                .set_system_integer(0, result)
                .map_err(|why| error(pc, 0x2220, why))?;
        }
        Ok(())
    }

    /// Finish the group switch once the owner has applied its enabled state.
    pub fn complete_save_group(&mut self, enabled: bool) -> Result<(), DecodeError> {
        if self.pending_save_group()? != Some(enabled) {
            return Err(error(self.pc(), 0x22b4, "CMVS save group request differs"));
        }
        self.machine.complete_system(SystemResult(0x4004))?;
        self.second_save_group = enabled;
        Ok(())
    }

    /// Path and contents of the `0x2129` system data write.
    #[cfg(feature = "runtime")]
    pub(crate) fn system_save_image(&self) -> Result<(VfsPath, VfsPath, Vec<u8>), DecodeError> {
        let pc = self.pc();
        let root = self
            .bindings
            .save_suffix()
            .ok_or_else(|| error(pc, 0x2129, "CMVS save root is not registered"))?;
        let file = |name: &str| {
            VfsPath::new(Mount::Save, &format!("{}/{name}", root.relative()))
                .map_err(|_| error(pc, 0x2129, "invalid system save path"))
        };
        // 0x14002e2a0 writes each list only while its recording flag is set.
        let lists = std::array::from_fn(|kind| {
            if self.native.recording(kind) {
                self.persistent_lists[kind].clone()
            } else {
                Vec::new()
            }
        });
        let image = super::system_save::encode(
            self.machine.state(),
            &lists,
            self.settings.game_id,
            self.native.system_flag(),
        )
        .map_err(|_| error(pc, 0x2129, "CMVS system save encoding failed"))?;
        Ok((file("system.dat")?, file("system.bak")?, image))
    }

    pub fn pending_system_save(&self) -> Result<Option<SystemSaveRequest>, DecodeError> {
        let Some((pc, 0x128)) = self.machine.pending_system() else {
            return Ok(None);
        };
        let root = self
            .bindings
            .save_suffix()
            .ok_or_else(|| error(pc, 0x2128, "CMVS save root is not registered"))?;
        let path = VfsPath::new(Mount::Save, &format!("{}/system.dat", root.relative()))
            .map_err(|_| error(pc, 0x2128, "invalid system save path"))?;
        Ok(Some(SystemSaveRequest {
            session: self.session,
            generation: self.io_generation,
            pc,
            path,
            game_id: self.settings.game_id,
        }))
    }

    pub fn complete_system_save(
        &mut self,
        prepared: PreparedSystemSave,
    ) -> Result<(), DecodeError> {
        if self.pending_system_save()?.as_ref() != Some(&prepared.request) {
            return Err(error(
                self.pc(),
                0x2128,
                "stale CMVS system-save preparation",
            ));
        }
        let generation = self.next_io_generation(0x2128)?;
        let candidate = prepared
            .contents
            .map(|contents| contents.apply(self.machine.state()))
            .transpose()
            .map_err(|why| error(self.pc(), 0x2128, why))?;
        self.machine.complete_system(SystemResult(0x4000))?;
        if let Some((state, lists)) = candidate {
            *self.machine.state_mut() = state;
            // 0x14002df00 inserts loaded names through the gated, deduplicating recorder.
            for (kind, names) in lists.iter().enumerate() {
                for name in names {
                    self.native.record(&mut self.persistent_lists, kind, name);
                }
            }
        }
        self.io_generation = generation;
        Ok(())
    }

    fn next_io_generation(&self, opcode: u16) -> Result<u64, DecodeError> {
        self.io_generation
            .checked_add(1)
            .ok_or_else(|| error(self.pc(), opcode, "CMVS I/O generation exhausted"))
    }

    /// Called once for each original VM frame, never for storage polls or slices.
    pub fn begin_frame(&mut self, delta_ms: u32) -> Result<(), DecodeError> {
        self.actions.begin_frame();
        self.gesture_options.frame(&mut self.actions, delta_ms);
        let visible_messages = self.message_windows.map(|window| window.unwrap_or(false));
        self.control
            .begin_frame(&mut self.machine, delta_ms, visible_messages)
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn complete_quit(&mut self) -> Result<(), DecodeError> {
        if !matches!(self.machine.pending_system(), Some((_, 1))) {
            return Err(error(self.pc(), 0x2001, "CMVS quit request missing"));
        }
        self.machine.complete_system(SystemResult(0x4000))?;
        Ok(())
    }

    pub fn run(&mut self, instruction_budget: u32) -> Result<VmYield, DecodeError> {
        self.run_slice(&mut { instruction_budget })
    }

    pub(crate) fn run_slice(&mut self, budget: &mut u32) -> Result<VmYield, DecodeError> {
        while *budget > 0 {
            *budget -= 1;
            match self.machine.step()? {
                Step::Continue => {}
                Step::Yield => return Ok(VmYield::Frame),
                Step::System { pc, command } => {
                    let opcode = command + 0x2000;
                    if self.profile() == VmProfile::Pc20070119X86 {
                        if command >= 800 {
                            return Err(error(pc, opcode, "outside CMVS 2007 syscall table"));
                        }
                        if super::noop::returns_without_effect_20070119(opcode) {
                            self.machine.complete_system(SystemResult(0x4000))?;
                            continue;
                        }
                    }
                    if let Some(selector) = super::cvns::selector(opcode) {
                        let result = if super::cvns::returns_without_effect(selector) {
                            SystemResult(0x4000)
                        } else {
                            match self.cvns_command(selector)? {
                                Some(result) => result,
                                None => return Ok(VmYield::SystemCall { pc, opcode }),
                            }
                        };
                        if self.machine.complete_system(result)? == Step::Yield {
                            return Ok(VmYield::Frame);
                        }
                        continue;
                    }
                    if opcode == 0x208a {
                        self.control.invoke(&mut self.machine)?;
                        continue;
                    }
                    let Some(result) = self.startup_command(opcode)? else {
                        return Ok(VmYield::SystemCall { pc, opcode });
                    };
                    if self.machine.complete_system(result)? == Step::Yield {
                        return Ok(VmYield::Frame);
                    }
                }
            }
        }
        Ok(VmYield::BudgetExhausted)
    }

    fn startup_command(&mut self, opcode: u16) -> Result<Option<SystemResult>, DecodeError> {
        if let Some(result) =
            self.native
                .command(&mut self.machine, &mut self.persistent_lists, opcode)?
        {
            return Ok(Some(result));
        }
        let cleanup = match opcode {
            0x201d..=0x201f => {
                let value = self.required_parameter(opcode, 4)?;
                let setting = match opcode {
                    0x201d => &mut self.settings.game_id,
                    0x201e => &mut self.settings.engine_field_12236,
                    _ => &mut self.settings.archive_mode,
                };
                *setting = Some(value);
                4
            }
            0x2090 => {
                self.settings.engine_field_12084 = true;
                0
            }
            0x2091..=0x2094 => {
                self.fill_variables(opcode)?;
                12
            }
            0x23ac if self.machine.profile() == VmProfile::Pc20211007X64 => {
                self.settings.proportional_glyphs = self.required_parameter(opcode, 4)? != 0;
                4
            }
            // The 2021 dispatch table returns from these cases after reclaiming the arguments.
            0x2019 | 0x209c if self.machine.profile() == VmProfile::Pc20211007X64 => 8,
            0x23ae if self.machine.profile() == VmProfile::Pc20211007X64 => 4,
            0x2268..=0x226e if self.machine.profile() == VmProfile::Pc20211007X64 => {
                let slot = self.required_parameter(opcode, 4)?;
                let reference = self.required_parameter(opcode, 8)?;
                let words = [
                    self.required_parameter(opcode, 12)?,
                    self.required_parameter(opcode, 16)?,
                    self.required_parameter(opcode, 20)?,
                    self.required_parameter(opcode, 24)?,
                ];
                let pc = self.pc();
                let (bank, limit) = match opcode {
                    0x2268 => (0x2010, 5),
                    0x2269 => (0x2011, 10),
                    0x226a => (0x2012, 5),
                    0x226b => (0x2013, 5),
                    0x226c => (0x2014, 5),
                    0x226d => (0x2019, 5),
                    _ => (0x201a, 5),
                };
                if slot >= limit {
                    return Err(error(
                        pc,
                        opcode,
                        "resource source slot outside supported range",
                    ));
                }
                let path = self.machine.resolve_string(reference)?;
                self.bindings
                    .set_keyed_source(bank, slot, path, words)
                    .map_err(|_| error(pc, opcode, "invalid CMVS resource registration path"))?;
                24
            }
            0x2010..=0x2014 | 0x2017 | 0x2018 | 0x2019 | 0x201a
                if opcode != 0x2019 || self.machine.profile() == VmProfile::Pc20160201X64 =>
            {
                let slot = self.required_parameter(opcode, 4)?;
                let reference = self.required_parameter(opcode, 8)?;
                let pc = self.pc();
                let limit = if opcode == 0x2011 { 10 } else { 5 };
                if slot >= limit {
                    return Err(error(
                        pc,
                        opcode,
                        "resource source slot outside supported range",
                    ));
                }
                let path = self.machine.resolve_string(reference)?;
                self.bindings
                    .set_source(opcode, slot, path)
                    .map_err(|_| error(pc, opcode, "invalid CMVS resource registration path"))?;
                8
            }
            0x2015 | 0x2016 => {
                let reference = self.required_parameter(opcode, 4)?;
                let pc = self.pc();
                let path = self.machine.resolve_string(reference)?;
                self.bindings
                    .set_root(opcode, path)
                    .map_err(|_| error(pc, opcode, "invalid CMVS resource root path"))?;
                4
            }
            0x2098 => {
                let reference = self.required_parameter(opcode, 4)?;
                let pc = self.pc();
                let name = self.machine.resolve_string(reference)?;
                if name.len() >= 128 || self.cursors.len() >= 1024 {
                    return Err(error(pc, opcode, "cursor registration limit"));
                }
                self.cursors.push(name.to_vec());
                self.machine
                    .state_mut()
                    .set_system_integer(0, self.cursors.len() as u32)
                    .map_err(|why| error(pc, opcode, why))?;
                4
            }
            0x212e..=0x2137 => self.history.command(&mut self.machine, opcode)?,
            0x2350..=0x235b | 0x2360 | 0x2362 | 0x2364..=0x236b => {
                let enabled = if opcode == 0x2350 {
                    Some(self.required_parameter(opcode, 4)? != 0)
                } else {
                    None
                };
                let Some(cleanup) =
                    self.gesture_options
                        .command(&mut self.machine, opcode, &self.actions)?
                else {
                    return Ok(None);
                };
                if let Some(enabled) = enabled {
                    self.settings.gestures_enabled = enabled;
                }
                cleanup
            }
            0x2162 => {
                let pc = self.pc();
                let raw = self.required_parameter(opcode, 4)?;
                let title = self.machine.resolve_string(raw)?;
                if title.len() >= 128 {
                    return Err(error(pc, opcode, "CMVS save title byte limit"));
                }
                self.save_title = title.to_vec();
                4
            }
            0x20f0..=0x20fd | 0x20ff => super::strings::command(&mut self.machine, opcode)?,
            0x214b => {
                let pc = self.pc();
                let index = self.required_parameter(opcode, 4)? as usize;
                let window = self
                    .message_windows
                    .get_mut(index)
                    .ok_or_else(|| error(pc, opcode, "CMVS message window outside 0..7"))?;
                self.machine
                    .state_mut()
                    .set_system_integer(0, u32::from(window.is_some()))
                    .map_err(|why| error(pc, opcode, why))?;
                4
            }
            0x22b0 => {
                self.settings.thumbnail_size = [
                    self.required_parameter(opcode, 4)?,
                    self.required_parameter(opcode, 8)?,
                ];
                8
            }
            0x22b1 => {
                self.history
                    .set_save_limit(self.required_parameter(opcode, 4)?);
                4
            }
            0x2280 => {
                self.sequence.reset();
                0
            }
            0x2281 | 0x2285 => {
                let value = if opcode == 0x2285 { self.sequence.count() } else { self.sequence.current() };
                let pc = self.pc();
                self.machine
                    .state_mut()
                    .set_system_integer(0, value)
                    .map_err(|why| error(pc, opcode, why))?;
                0
            }
            0x2283 => {
                self.sequence.stop();
                0
            }
            0x2286..=0x228b => {
                let word = |index: usize| self.required_parameter(opcode, 4 * index);
                let name = |this: &Self, reference: u32| {
                    this.machine.resolve_string(reference).map(<[u8]>::to_vec)
                };
                let words = match opcode {
                    0x2286 => {
                        let duration = word(1)?;
                        self.sequence.push(SequenceKind::Wait, 0, 0, &[], duration, [0; 3], 0);
                        1
                    }
                    0x2287 => {
                        let [mode, x, y, z, duration] = [word(1)?, word(2)?, word(3)?, word(4)?, word(5)?];
                        self.sequence.push(SequenceKind::Camera, mode, 0, &[], duration, [x, y, z], 0);
                        5
                    }
                    0x2288 | 0x228b => {
                        let [slot, reference, duration] = [word(1)?, word(2)?, word(3)?];
                        let name = name(self, reference)?;
                        let kind = if opcode == 0x2288 { SequenceKind::Object } else { SequenceKind::Window };
                        self.sequence.push(kind, 0, slot, &name, duration, [0; 3], 0);
                        3
                    }
                    _ => {
                        let [slot, x, y, z, mode, duration] = [word(1)?, word(2)?, word(3)?, word(4)?, word(5)?, word(6)?];
                        let sub = if opcode == 0x2289 { 1 } else { 2 };
                        self.sequence.push(SequenceKind::Object, sub, slot, &[], duration, [x, y, z], mode);
                        6
                    }
                };
                words * 4
            }
            0x22b9 => {
                self.save_information[0] = Some(self.required_parameter(opcode, 4)?);
                4
            }
            0x2340 => {
                self.message_bypass = self.required_parameter(opcode, 4)? != 0;
                4
            }
            0x22ba => {
                let second = self.required_parameter(opcode, 4)?;
                let third = self.required_parameter(opcode, 8)?;
                self.save_information[1..].copy_from_slice(&[Some(second), Some(third)]);
                8
            }
            0x22e5 | 0x22eb => {
                let pc = self.pc();
                let setting = self.required_parameter(opcode, 4)? != 0;
                if self.machine.stack().len_bytes() < 8 {
                    return Err(error(pc, opcode, "CMVS setting parameters outside stack"));
                }
                if setting {
                    let value = self.required_parameter(opcode, 8)? != 0;
                    if opcode == 0x22e5 {
                        self.auto_advance = value;
                    } else {
                        self.skipping = value;
                    }
                } else {
                    let enabled = if opcode == 0x22e5 {
                        self.auto_advance
                    } else {
                        self.skipping
                    };
                    self.machine
                        .state_mut()
                        .set_system_integer(0, u32::from(enabled))
                        .map_err(|why| error(pc, opcode, why))?;
                }
                8
            }
            0x22ce | 0x22ee => {
                let pc = self.pc();
                let enabled = if opcode == 0x22ce {
                    self.popup_enabled
                } else {
                    self.second_save_group
                };
                self.machine
                    .state_mut()
                    .set_system_integer(0, u32::from(enabled))
                    .map_err(|why| error(pc, opcode, why))?;
                0
            }
            _ => {
                let revision = self.machine.profile() == VmProfile::Pc20211007X64;
                if (revision && (0x23d0..=0x23df).contains(&opcode))
                    || (self.profile() != VmProfile::Pc20070119X86
                        && super::noop::returns_without_effect(opcode)
                        && !(revision && super::noop::implemented_in_20211007(opcode)))
                    || self
                        .actions
                        .command(&mut self.machine, opcode, self.skipping)?
                {
                    0
                } else if let Some(cleanup) = self.control.command(&mut self.machine, opcode)? {
                    cleanup
                } else if let Some(cleanup) = self.screen.command(&mut self.machine, opcode)? {
                    cleanup
                } else {
                    return Ok(None);
                }
            }
        };
        Ok(Some(SystemResult(0x4000 | cleanup)))
    }

    fn required_parameter(&self, opcode: u16, distance: usize) -> Result<u32, DecodeError> {
        self.parameter(distance).ok_or_else(|| {
            error(
                self.pc(),
                opcode,
                "parameter stack underflow or unknown word",
            )
        })
    }

    fn fill_variables(&mut self, opcode: u16) -> Result<(), DecodeError> {
        let start = self.required_parameter(opcode, 4)?;
        let count = self.required_parameter(opcode, 8)?;
        let raw = self.required_parameter(opcode, 12)?;
        let pc = self.pc();
        let outcome = match opcode {
            0x2091 => self.machine.state_mut().fill_bits(start, count, raw != 0),
            0x2092 => self
                .machine
                .state_mut()
                .fill_integers(start, count, raw as i32),
            0x2093 => self.machine.state_mut().fill_floats(start, count, raw),
            0x2094 => {
                let bytes = self.machine.resolve_string(raw)?.to_vec();
                self.machine.state_mut().fill_strings(start, count, &bytes)
            }
            _ => unreachable!("called for variable initialization opcodes"),
        };
        outcome.map_err(|why| error(pc, opcode, why))
    }
}

#[cfg(all(feature = "runtime", test))]
mod slot_metadata_tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;

    #[test]
    fn metadata_and_stack_advance_only_after_scene_publication() {
        let mut code = Vec::new();
        for word in [0x8000_0001u32, 0x8000_0000, u32::MAX, 1, 5] {
            code.extend_from_slice(&[0x40, 0x04, 4, 0]);
            code.extend_from_slice(&word.to_le_bytes());
        }
        code.extend_from_slice(&[0xbd, 0x22, 0, 0x20]);
        let mut bytes = b"PS2A".to_vec();
        for word in [
            48,
            1,
            0,
            0,
            code.len() as u32,
            0,
            0,
            0,
            0,
            code.len() as u32,
            0,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend(
            code.into_iter()
                .map(|byte| byte.rotate_left(1).wrapping_add(0x7c)),
        );
        let reader = MemoryAssetReader(bytes.into());
        let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &reader).unwrap();
        assert_eq!(
            vm.run_slice(&mut 64).unwrap(),
            VmYield::SystemCall {
                pc: 40,
                opcode: 0x22bd
            }
        );
        vm.machine
            .write_string_reference(0x22bd, 0x8000_0000, b"old")
            .unwrap();
        for (index, value) in [(0, 99), (2, 88), (3, 77)] {
            vm.machine
                .state_mut()
                .set_system_integer(index, value)
                .unwrap();
        }
        let metadata = super::super::SlotMetadata {
            timestamp: b"new".to_vec(),
            comment: vec![0x82, 0xa0],
            date_result: 8,
            values: [9, 10, 11],
            preview: None,
        };
        assert!(vm
            .complete_slot_metadata(Some(&metadata), || anyhow::bail!("scene refused batch"))
            .is_err());
        assert_eq!(vm.pc(), 40);
        assert_eq!(vm.machine.stack().len_bytes(), 20);
        assert_eq!(vm.machine.resolve_string(0x8000_0000).unwrap(), b"old");
        assert_eq!(vm.machine.state().system_integer(0), Some(99));

        vm.complete_slot_metadata(Some(&metadata), || Ok(()))
            .unwrap();
        assert_eq!(vm.pc(), 42);
        assert_eq!(vm.machine.stack().len_bytes(), 0);
        assert_eq!(vm.machine.resolve_string(0x8000_0000).unwrap(), b"new");
        assert_eq!(
            vm.machine.resolve_string(0x8000_0001).unwrap(),
            [0x82, 0xa0]
        );
        let results: Vec<_> = (0..7)
            .map(|index| vm.machine.state().system_integer(index).unwrap())
            .collect();
        assert_eq!(results, [1, 8, 88, 77, 9, 10, 11]);
    }
}
