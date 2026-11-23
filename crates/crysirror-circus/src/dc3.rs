//! D.C.III With You ABI 0xa09f, statically recovered from DC3WY.exe.
//! 4233A0 is a separate 159-entry dispatcher, not DC4PH with shifted opcodes.
//! State below retains native declaration/PC policy; NativeHost owns shared
//! images, retained UI, queues and completion. Original save layout is not
//! projected through DC4PH NativeLiveState.
use crate::machine::{Button, Event, Group, ImagePart, NativeHost, NativeSlider, Outcome};
use crate::{Arguments, Instruction, Profile, Program};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// 437C40 initializes the source rectangle to this native canvas; 437A20
/// selects the same extent for the logical projection matrix.
pub const CANVAS: [u32; 2] = [1024, 576];
// Original49D728 static bytes.414A00/414630 push byte1,byte0,byte2
// into433230's RRGGBB packer. Keep raw VM bytes separate from logical RGB.
const NATIVE_PALETTE: [[u8; 3]; 14] = [
    [0, 0, 0],
    [0, 0, 255],
    [0, 255, 0],
    [0, 255, 255],
    [255, 0, 0],
    [255, 0, 255],
    [255, 255, 0],
    [255, 255, 255],
    [78, 133, 40],
    [204, 204, 255],
    [68, 119, 0],
    [127, 255, 127],
    [204, 204, 204],
    [170, 170, 255],
];
fn logical_color(raw: [u8; 3]) -> [u8; 3] {
    [raw[1], raw[0], raw[2]]
}
#[path = "dc3_audio.rs"]
mod audio;
#[path = "dc3_callbacks.rs"]
mod callbacks;
#[path = "dc3_history.rs"]
mod history;
#[path = "dc3_advmenu.rs"]
mod advmenu;
#[path = "dc3_body.rs"]
mod body;
#[path = "dc3_controls.rs"]
mod controls;
#[path = "dc3_credits.rs"]
pub(crate) mod credits;
#[path = "dc3_effects.rs"]
mod effects;
#[path = "dc3_original_effects.rs"]
pub(crate) mod original_effects;
#[path = "dc3_original_weather.rs"]
pub(crate) mod original_weather;
#[path = "dc3_load.rs"]
mod load;
#[path = "dc3_menu.rs"]
mod menu;
#[path = "dc3_motion.rs"]
mod motion;
#[path = "dc3_movie.rs"]
mod movie;
#[path = "dc3_portrait.rs"]
mod portrait;
#[path = "dc3_reset.rs"]
mod reset;
#[path = "dc3_resources.rs"]
mod resources;
#[path = "dc3_scene.rs"]
mod scene;
#[path = "dc3_slot.rs"]
pub(crate) mod slot;
#[path = "dc3_system.rs"]
mod system;
#[path = "dc3_text.rs"]
pub(crate) mod text;
#[path = "dc3_ui.rs"]
mod ui;
#[path = "dc3_value_reads.rs"]
mod value_reads;
#[path = "dc3_values.rs"]
mod values;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sprite {
    pub position: [i16; 2],
    pub retained_position: [i16; 2],
    pub resource: Option<String>,
    pub queued: bool,
    ///The shared sequence actor owns this object's channels (dc3_motion).
    #[serde(default)]
    pub actor: bool,
    ///437130 draw offset (dwords 3/4), added to the position channels.
    #[serde(default)]
    pub offset: [i16; 2],
    ///4371F0 scale percent (dwords 26/27); None is 437C40's 100.
    #[serde(default)]
    pub scale: Option<f32>,
    ///437280 pivot (dwords 29/30).
    #[serde(default)]
    pub pivot: [f32; 2],
    ///4377B0 draw priority (dword 115); None is 41EB70's 50 (0 for object 0).
    #[serde(default)]
    pub priority: Option<i16>,
    ///437850 screen-space zoom (dwords 6..10): percent and center.
    #[serde(default)]
    pub zoom: Option<(f32, [f32; 2])>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Builder {
    pub group: Group,
    pub pending_button: Option<Button>,
    pub pending_sounds: [u8; 2],
    pub pending_flags: BTreeSet<u8>,
    ///410B60/410B80's declaration offset is independent of224/225 motion.
    pub offset: [i16; 2],
}
impl Builder {
    fn seal(&mut self) {
        if let Some(button) = self.pending_button.take() {
            self.group.buttons.push(button);
        }
        self.pending_sounds = [0; 2];
        self.pending_flags.clear();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub scene: String,
    pub pc: u32,
    pub instructions: u64,
    pub last_opcode: Option<u8>,
    pub last_offset: u32,
    pub version: Vec<u8>,
    pub builder: Builder,
    pub templates: BTreeMap<u8, Builder>,
    pub image_names: BTreeMap<u8, String>,
    pub palette: BTreeMap<u8, [u8; 3]>,
    pub sprites: BTreeMap<u16, Sprite>,
    pub system_data: crate::dc3_settings::SystemData,
    pub script_bytes: Vec<u8>,
    pub runtime_values: BTreeMap<i16, i32>,
    pub source_count: u8,
    pub palette_published: u8,
    pub read: BTreeSet<(String, u16)>,
    pub savepoint: Option<(u16, u32, u16)>,
    pub(crate) history: history::History,
    pub(crate) handlers: callbacks::Handlers,
    pub(crate) observations: Option<value_reads::Observations>,
    pub(crate) resources: resources::Resources,
    pub(crate) movie: Option<movie::PendingMovie>,
    pub protagonist_names: [String; 2],
    pub(crate) audio: audio::AudioState,
    pub(crate) preference_reload: Option<system::PreferenceReload>,
    pub window_transition_flag: i32,
    ///Native this+11360/11364 input/skip latches, separate fromBuf62076.
    pub skip_latches: [i32; 2],
    ///Native this+13464, set by16/15 before41D800 clears live skip state.
    pub skip_mode: i32,
    pub halted: bool,
    /// Local v373 in 4233A0. A dispatch fence clears this flag.
    pub auxiliary_flag: bool,
    pub wait_until: Option<u64>,
    pub wait_skippable: bool,
    ///0x13 254 (1584) continuation owner; see dc3_motion.
    #[serde(default)]
    pub layer_fence: Option<u32>,
    ///0x0A process-10 menu owner (dc3_menu).
    #[serde(default)]
    pub menu_wait: Option<u32>,
    ///0x0E 15/16 started owner (dc3_effects).
    #[serde(default)]
    pub effect_started: Option<u32>,
    ///Buf+94568: the body 0x1A last captured (dc3_body).
    #[serde(default)]
    pub snapshot: Option<Vec<u8>>,
    ///v397/v393: 0x62 requested a tail snapshot resuming at this offset.
    #[serde(default)]
    pub text_snapshot: Option<u32>,
    #[serde(default)]
    pub text: text::TextState,
    ///Buf23398/23399: 0x27 weather kind and density.
    #[serde(default)]
    pub weather: Option<[i16; 2]>,
    #[serde(default)]
    pub weather_started: Option<u32>,
    ///4233A0 local v374 (see dc3_resources b_rewrite); cleared at yields.
    #[serde(default)]
    pub b_suppressed: bool,
    ///Buf23397: 0x27 7 fog (slot 20 on object 54) is active.
    #[serde(default)]
    pub fog: bool,
    ///0x16 0 load continuation (dc3_load).
    #[serde(default)]
    pub load: Option<load::LoadProgress>,
    ///40B6A0 in flight: the clock-stamped slot body.
    #[serde(default)]
    pub slot_write: Option<(u16, Vec<u8>)>,
    ///skip_for: ((pc, instruction count), 41D820) for the current instruction.
    #[serde(default)]
    pub skip_decision: Option<((u32, u64), bool)>,
    ///Buf+171432: the body 41FC40 kept for the 0x1B return.
    #[serde(default)]
    pub return_body: Option<Vec<u8>>,
    ///420640 entry continuation step.
    #[serde(default)]
    pub menu_entry: Option<u8>,
    ///The system scene that entry switches to (ADVMENU for the right
    ///button; dialogue-control actions name others).
    #[serde(default)]
    pub menu_entry_scene: Option<String>,
    ///Process case 6 dialogue controls (templates 9/10).
    #[serde(default)]
    pub controls: controls::ControlsState,
    #[serde(default)]
    pub rollback_step: Option<u8>,
    ///408F00 case 18 preference backup (this+17728).
    #[serde(default)]
    pub preference_backup: Option<Vec<u8>>,
    ///D.C.III (0x8a71) running over translated scripts (dc3_original).
    #[serde(default)]
    pub original: bool,
    ///Buf23574: a 0x93 camera is active (0x81's 0x10 scale is ignored);
    ///cleared by a background declaration without the :n hold.
    #[serde(default)]
    pub camera: bool,
    ///Buf23402: the 0x27 16 cloud scroll is running.
    #[serde(default)]
    pub clouds: bool,
    ///Slots whose Buf+17872 name a 0x49 transfer cleared while their
    ///textures stay loaded (the saved name is empty while it still matches).
    #[serde(default)]
    pub nameless: BTreeMap<u8, String>,
    ///D.C.III 0x6C face draw origin (object 65).
    #[serde(default)]
    pub face_origin: Option<[i16; 2]>,
    ///Buf23400: the 0x27 2 rain density.
    #[serde(default)]
    pub rain: Option<i16>,
    ///Buf23569 tint mode (0x94), reset by scene transfer (420370).
    #[serde(default)]
    pub tint: i32,
    #[serde(default)]
    pub choice: Option<menu::ChoiceState>,
    ///Buf23571: the menu wait belongs to a choice.
    #[serde(default)]
    pub choice_wait: bool,
    ///Weather pool spawn window (0x9A bounds, 0x9B chance).
    #[serde(default)]
    pub spawn_window: ([i32; 4], i32),
    ///408F00 process mode 2: the ending credits (dc3_credits).
    #[serde(default)]
    pub credits: Option<credits::Credits>,
    ///D.C.III process state 14 (dc3_original_effects).
    #[serde(default)]
    pub original_effect: Option<original_effects::OriginalEffect>,
    ///D.C.III 407190 stream (47AD7C..47AD88), seeded once per process.
    #[serde(default)]
    pub original_rng: crysirror_vfx::xorshift::Xorshift128,
    ///D.C.III 0x27 weather (dc3_original_weather).
    #[serde(default)]
    pub original_weather: Option<original_weather::OriginalWeather>,
    ///D.C.III 0x69 mode-2 pan target (C63CD8/C63CDC).
    #[serde(default)]
    pub original_pan: Option<[i16; 2]>,
    ///Engine time of the latest step (weather timer base).
    #[serde(default)]
    pub original_clock: u64,
}
impl State {
    pub fn new(scene: String) -> Self {
        Self {
            scene,
            pc: 3,
            instructions: 0,
            last_opcode: None,
            last_offset: 3,
            version: Vec::new(),
            builder: Builder::default(),
            templates: BTreeMap::new(),
            image_names: BTreeMap::new(),
            palette: (0..64)
                .map(|index| {
                    (
                        index,
                        NATIVE_PALETTE
                            .get(index as usize)
                            .copied()
                            .unwrap_or([0; 3]),
                    )
                })
                .collect(),
            sprites: BTreeMap::new(),
            system_data: Default::default(),
            script_bytes: vec![0; 1100],
            runtime_values: BTreeMap::new(),
            source_count: 0,
            palette_published: 0,
            read: BTreeSet::new(),
            savepoint: None,
            history: Default::default(),
            handlers: Default::default(),
            observations: None,
            resources: Default::default(),
            movie: None,
            protagonist_names: [String::new(), String::new()],
            audio: Default::default(),
            preference_reload: None,
            window_transition_flag: 1, //4311A0 Device+24
            skip_latches: [0; 2],
            skip_mode: 0,
            halted: false,
            auxiliary_flag: false,
            wait_until: None,
            wait_skippable: false,
            layer_fence: None,
            menu_wait: None,
            effect_started: None,
            snapshot: None,
            text_snapshot: None,
            text: Default::default(),
            weather: None,
            weather_started: None,
            b_suppressed: false,
            fog: false,
            load: None,
            slot_write: None,
            skip_decision: None,
            return_body: None,
            menu_entry: None,
            menu_entry_scene: None,
            controls: Default::default(),
            rollback_step: None,
            preference_backup: None,
            original: false,
            camera: false,
            clouds: false,
            rain: None,
            face_origin: None,
            nameless: BTreeMap::new(),
            tint: 0,
            choice: None,
            choice_wait: false,
            spawn_window: ([0; 4], 0),
            credits: None,
            original_effect: None,
            original_rng: Default::default(),
            original_weather: None,
            original_pan: None,
            original_clock: 0,
        }
    }
}

/// Actual default-target entries of the native compressed switch at423525,
/// recovered with its value table as well as its jump table. This list must
/// not be inferred from missing decompiler `case` labels. Values >=159 also
/// take that native default target; malformed operands still fail parsing.
pub fn native_noop(opcode: u8) -> bool {
    opcode >= 0x9f
        || matches!(
            opcode,
            0x01 | 0x02
                | 0x0c
                | 0x0d
                | 0x11
                | 0x17
                | 0x1d
                | 0x21
                | 0x24
                | 0x26
                | 0x28
                | 0x29
                | 0x2a
                | 0x2b
                | 0x2c
                | 0x2d
                | 0x2e
                | 0x2f
                | 0x30
                | 0x38
                | 0x39
                | 0x40
                | 0x42
                | 0x44
                | 0x47
                | 0x4c
                | 0x50
                | 0x55
                | 0x56
                | 0x59
                | 0x5b
                | 0x5f
                | 0x60
                | 0x63
                | 0x64
                | 0x6d
                | 0x72
                | 0x80
                | 0x87
                | 0x88
                | 0x89
                | 0x8d
                | 0x8e
                | 0x99
        )
}

pub struct Machine {
    pub state: State,
}
impl Machine {
    pub fn new(scene: String) -> Self {
        Self {
            state: State::new(scene),
        }
    }

    pub fn step(
        &mut self,
        program: &Program,
        host: &mut dyn NativeHost,
        now: u64,
        pressed: bool,
        skip_held: bool,
        right_pressed: bool,
    ) -> Result<Outcome> {
        //D.C.III 40AD20 keeps running beneath script waits (30 ms timer).
        if self.state.original {
            self.tick_original_weather(host, now)?;
        }
        let outcome = self.step_script(program, host, now, pressed, skip_held, right_pressed)?;
        Ok(match (outcome, self.original_weather_due()) {
            (Outcome::WaitUntil(deadline), Some(timer)) => Outcome::WaitUntil(deadline.min(timer.max(now + 1))),
            (Outcome::WaitForInput, Some(timer)) => Outcome::WaitUntil(timer.max(now + 1)),
            (outcome, _) => outcome,
        })
    }
    fn step_script(
        &mut self,
        program: &Program,
        host: &mut dyn NativeHost,
        now: u64,
        pressed: bool,
        skip_held: bool,
        right_pressed: bool,
    ) -> Result<Outcome> {
        //422050: key 11 (Ctrl) held with skip mode this[3366] sets the
        //this[2840]/[2841] latches; without the key they clear.
        let latch = i32::from(skip_held && self.state.skip_mode != 0);
        self.state.skip_latches = [latch, latch];
        ensure!(
            program.profile == Profile::Dc3WithYou,
            "DC3WY executor requires native ABI 0xa09f"
        );
        if self.state.halted {
            return Ok(Outcome::Halted);
        }
        while self.state.palette_published < 64 {
            let index = self.state.palette_published;
            let raw = self
                .state
                .palette
                .get(&index)
                .copied()
                .context("DC3WY native palette state extent")?;
            if !host.emit(&Event::Palette {
                index,
                rgb: logical_color(raw),
            })? {
                return Ok(Outcome::Pending);
            }
            self.state.palette_published += 1;
        }
        if let Some(deadline) = self.state.wait_until {
            //42A160 case 0x13: 41D820 ends the wait unless var(9,99).
            let skipping = self.native_skipping() && self.value(9, 99)? == 0;
            if now < deadline && !(pressed && self.state.wait_skippable) && !skipping {
                return Ok(Outcome::WaitUntil(deadline));
            }
            self.state.wait_until = None;
        }
        if let Some(outcome) = self.poll_credits(host, now, pressed)? {
            return Ok(outcome);
        }
        if self.state.menu_entry.is_some() || (right_pressed && self.state.text.wait.is_some() && self.state.text.rendered) {
            if let Some(outcome) = self.right_click_menu(host)? {
                return Ok(outcome);
            }
        }
        if let Some(outcome) = self.poll_text_wait(host, pressed, now)? {
            return Ok(outcome);
        }
        if self.state.menu_wait.is_some() {
            if let Some(outcome) = self.poll_menu(program, host)? {
                return Ok(outcome);
            }
        }
        let instruction = program.instruction(self.state.pc)?;
        let outcome = self.execute(program, instruction, host, now)
            .with_context(|| {
                format!(
                    "Circus DC3WY {} pc={:#x} opcode={:#04x}",
                    self.state.scene, instruction.offset, instruction.opcode
                )
            });
        if crate::trace_enabled() {
            eprintln!("DC3 {} pc={:#x} op={:#04x} args={:?} -> {:?}", self.state.scene, instruction.offset, instruction.opcode, instruction.arguments, outcome.as_ref().ok());
        }
        outcome
    }

    fn execute(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome> {
        let mut a = [0i16; 8];
        let text = match &instruction.arguments {
            Arguments::Bytes(values) => {
                a[0] = values[0].into();
                a[1] = values[1].into();
                ""
            }
            Arguments::Words(values) => {
                a[..4].copy_from_slice(values);
                ""
            }
            Arguments::SlotString { slot, text } => {
                a[0] = (*slot).into();
                text
            }
            Arguments::String(text) => text,
        };
        if let Some(values) = instruction.continuation {
            a[4..].copy_from_slice(&values);
        }
        if instruction.opcode == crate::dc3_original::FACE_ORIGIN {
            if a[3] == 0 {
                self.state.face_origin = Some([a[0], a[1]]);
            }
            return self.advance(instruction.next, instruction.opcode);
        }
        if instruction.opcode == crate::dc3_original::UNTRANSLATED {
            bail!(
                "D.C.III opcode {:#04x} at {}:{:#x} has no reviewed DC3WY mapping",
                program.original_opcode(instruction).unwrap_or(instruction.opcode),
                self.state.scene,
                instruction.offset
            );
        }
        if let Some(outcome) = self.execute_original_effects(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_original_weather(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_original_pan_target(program, instruction, a)? {
            return Ok(outcome);
        }
        if !self.prepare_value_reads(program, instruction, a, host)? {
            return Ok(Outcome::Pending);
        }
        if let Some(outcome) = self.execute_handlers(program, instruction, a, now, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_reset(instruction, a)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_rollback(program, instruction, a, text, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_scene(program, instruction, a, text, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_resources(program, instruction, a, text, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_text(instruction, a, text, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_return(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_snapshot(program, instruction, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_spawn_window(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_weather(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_effects(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_menu(program, instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_portrait(instruction, a, text, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_motion(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_movie(instruction, text, host, now)? {
            return Ok(outcome);
        }
        if let Some(next) = self.execute_ui(program, instruction, a)? {
            return self.advance(next, instruction.opcode);
        }
        if let Some(complete) = self.execute_audio(instruction, a, text, host)? {
            return if complete {
                self.advance(instruction.next, instruction.opcode)
            } else {
                Ok(Outcome::Pending)
            };
        }
        if instruction.opcode == 0x16 && a[0] == 0 {
            if let Some(outcome) = self.execute_load(instruction, host)? {
                return Ok(outcome);
            }
        }
        if let Some(complete) = self.execute_system(instruction, a, host)? {
            return if complete {
                self.advance(instruction.next, instruction.opcode)
            } else {
                Ok(Outcome::Pending)
            };
        }
        let mut next = instruction.next;
        let mut outcome = Outcome::Advanced;
        match instruction.opcode {
            // Native case0 does not advance the decoder. It yields to its
            // process loop; it is not an exit or an invented scene end.
            0 => {
                self.state.auxiliary_flag = false;
                self.state.last_opcode = Some(0);
                self.state.last_offset = instruction.offset;
                return Ok(Outcome::WaitForInput);
            }
            0x0f => {
                self.advance(next, instruction.opcode)?;
                self.state.halted = true;
                return Ok(Outcome::Halted);
            }
            0x03 => {
                let label = (a[0] as u16) | ((a[1] as u16) << 8);
                let raw = *program
                    .labels
                    .get(label as usize)
                    .context("DC3WY read/save label index")?;
                let metadata = *program
                    .label_metadata
                    .get(label as usize)
                    .context("DC3WY native label metadata")?;
                let Some(scene) = host.scene_index(&self.state.scene)? else {
                    return Ok(Outcome::Pending);
                };
                let mut history = self.state.history.clone();
                let mut data = self.state.system_data.clone();
                if program
                    .instruction(next)
                    .map_or(true, |following| following.opcode != 0x45)
                {
                    history.note_label(
                        &mut data,
                        scene,
                        label,
                        raw,
                        self.state.runtime_values.get(&233).copied().unwrap_or(0) != 0,
                    )?;
                }
                let unread = scene < 0 || !data.label_is_read(scene as usize, label)?;
                ensure!(
                    self.state.runtime_values.get(&195).copied().unwrap_or(0) == 0
                        || !unread
                        || data.prefix_value(30)?.unwrap_or(0) == 0,
                    "DC3WY unread-skip notice requires its shared sprite receipt"
                );
                self.state.history = history;
                self.state.system_data = data;
                self.state.savepoint = Some((label, raw, metadata));
            }
            0x04 => {
                let label = (a[0] as u16) | ((a[1] as u16) << 8);
                let Some(scene) = host.scene_index(&self.state.scene)? else {
                    return Ok(Outcome::Pending);
                };
                let mut data = self.state.system_data.clone();
                if scene >= 0 && label != 0 {
                    data.mark_read(scene as usize, label, label)?;
                }
                if !host.emit(&Event::ReadLabel {
                    scene: self.state.scene.clone(),
                    label,
                })? {
                    return Ok(Outcome::Pending);
                }
                self.state.system_data = data;
                if scene >= 0 && label != 0 {
                    self.state.read.insert((self.state.scene.clone(), label));
                }
            }
            0x09 if a[0] != 0 => {
                //4233A0's nonzero branch does NOT open/publish a text
                //window or enterprocess6. It applies only these two native
                //skip/auto guards; the zero branch still needs its UI fence.
                let mut data = self.state.system_data.clone();
                let skipping = self.state.runtime_values.get(&193).copied().unwrap_or(0) == 0
                    && (self.state.skip_latches.iter().any(|value| *value != 0)
                        || self.state.runtime_values.get(&195).copied().unwrap_or(0) != 0);
                if data.prefix_value(210)?.unwrap_or(0) != 0 && skipping {
                    self.state.skip_latches[0] = 0;
                    self.state.runtime_values.insert(195, 0);
                }
                if data.prefix_value(209)?.unwrap_or(0) != 0
                    && data.prefix_value(13)?.unwrap_or(0) != 0
                {
                    ensure!(
                        data.set_prefix_value(13, 0)?,
                        "DC3WY auto-mode native field"
                    );
                }
                self.state.system_data = data;
            }
            //Actual effect-selector fall-throughs of4233A0, including the
            //bootstrap's14/32. Other selectors retain their native fence.
            0x0e if matches!(a[0], 7 | 10 | 11 | 13 | 14 | 23 | 24 | 25 | 30 | 31 | 32) => {}
            0x1e if a[0] != 0 => {} // actual native delimiter-only case
            0x1e => {
                next = skip_marker(program, next, a[1])?;
            }
            0x1c if a[0] == 1 => {
                let Some(crate::machine::system_ops::SystemReply::Hour(hour)) =
                    host.system(crate::machine::system_ops::SystemRequest::LocalHour)?
                else {
                    return Ok(Outcome::Pending);
                };
                if !self.assign(host, 3, 0, i32::from(hour))? {
                    return Ok(Outcome::Pending);
                }
            }
            //408F00 case 19: 40B960(var(99,254)) reads DC3WY%02d.dat in
            //full (76864 bytes); var(9,87) = 1 when that read fails.
            0x1c if a[0] == 19 => {
                let slot = u16::try_from(self.value(99, 254)?).context("DC3WY quick slot index")?;
                let reply = host.resource(&crate::machine::resource_ops::ResourceRequest::ProfileSlotFull {
                    profile: Profile::Dc3WithYou,
                    slot,
                    bytes: 76864,
                })?;
                let full = match reply {
                    None => return Ok(Outcome::Pending),
                    Some(crate::machine::resource_ops::ResourceReply::Readable(full)) => full,
                    Some(_) => bail!("DC3WY slot read returned a foreign receipt"),
                };
                if !self.assign(host, 9, 87, i32::from(!full))? {
                    return Ok(Outcome::Pending);
                }
            }
            //0x23: var(99,2) = rand() % operand (the shared CRT sequence).
            0x23 => {
                let modulus = u16::try_from(a[0]).ok().filter(|m| *m > 0).context("DC3WY rand modulus")?;
                let Some(value) = host.random(modulus)? else { return Ok(Outcome::Pending) };
                if !self.assign(host, 99, 2, value)? {
                    return Ok(Outcome::Pending);
                }
            }
            //408F00 case 9: the credit object starts and the dispatcher
            //yields into process mode 2 until 42E4D0 completes.
            0x1c if a[0] == 9 => {
                self.begin_credits(now)?;
                outcome = Outcome::WaitUntil(now.saturating_add(16));
            }
            0x1c if a[0] == 207 => {
                let scene = self.state.scene.to_ascii_lowercase();
                let value = (1..=9)
                    .find(|index| scene.contains(&format!("wy_{index}")))
                    .map_or(0, |index| index - 1);
                if !self.assign(host, 5, 10, value)? {
                    return Ok(Outcome::Pending);
                }
            }
            //408F00's actual switch default returns the unchanged process
            //state. All admitted producers above keep that process idle;
            //modal/native-system selectors below still stop explicitly.
            0x1c if !matches!(
                a[0],
                1 | 6
                    | 7
                    | 9
                    | 18
                    | 19
                    | 50
                    | 51
                    | 98
                    | 99
                    | 200
                    | 201
                    | 202
                    | 203
                    | 204
                    | 206
                    | 207
            ) => {}
            0x7c => {
                let left = self.value(a[0], a[1])?;
                let (mode, right) = if (a[3] as u8 as i8) < 0 {
                    (a[3] & 0x7f, self.value(a[2] >> 8, i16::from(a[2] as u8))?)
                } else {
                    (a[3], i32::from(a[2]))
                };
                let accepted = match mode {
                    0 => left == right,
                    1 => left != right,
                    2 => left > right,
                    3 => left < right,
                    4 => left >= right,
                    5 => left <= right,
                    _ => left != 0,
                };
                let marker = program
                    .instruction(next)
                    .context("DC3WY comparison marker")?;
                let Arguments::Bytes([tag, _]) = marker.arguments else {
                    bail!("DC3WY comparison marker width")
                };
                next = marker.next;
                if !accepted {
                    next = skip_marker(program, next, i16::from(tag as i8))?;
                }
            }
            0x78 => {
                let value = match a[3] {
                    0 => i32::from(a[2]),
                    1 => self.value(a[0], a[1])?.wrapping_add(i32::from(a[2])),
                    2 => self.value(a[0], a[1])?.wrapping_sub(i32::from(a[2])),
                    _ => return self.advance(instruction.next, instruction.opcode),
                };
                if !self.assign(host, a[0], a[1], value)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x79 => {
                let value = self.value(a[2], a[3])?;
                if !self.assign(host, a[0], a[1], value)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x7a => {
                let mut total = 0i32;
                for _ in 0..crate::MAX_INSTRUCTIONS {
                    if program
                        .native_byte(next)
                        .context("DC3WY variable sum terminator")?
                        == 2
                    {
                        break;
                    }
                    let word = |at: u32| -> Result<i16> {
                        Ok(i16::from_le_bytes([
                            program
                                .native_byte(at)
                                .context("DC3WY sum operand low byte")?,
                            program
                                .native_byte(at + 1)
                                .context("DC3WY sum operand high byte")?,
                        ]))
                    };
                    total = total.wrapping_add(self.value(word(next + 1)?, word(next + 3)?)?);
                    next = next.checked_add(9).context("DC3WY sum cursor overflow")?;
                }
                ensure!(
                    program.instruction(next)?.opcode == 2,
                    "DC3WY variable sum budget exceeded"
                );
                if !self.assign(host, a[0], a[1], total)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x7b => {
                let total = self
                    .value(a[0], a[1])?
                    .wrapping_add(self.value(a[2], a[3])?);
                if !self.assign(host, a[0], a[1], total)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x5e => {
                let raw = program
                    .literal_bytes(instruction.offset)
                    .context("DC3WY voice-source prefix bytes")?;
                if raw.is_empty() {
                    self.state.system_data.reset_sources();
                    self.state.source_count = 0;
                } else {
                    self.state
                        .system_data
                        .register_source(raw, &mut self.state.source_count)?;
                }
            }
            0x5c => {
                let bytes = program
                    .literal_bytes(instruction.offset)
                    .context("DC3WY version bytes")?;
                if self
                    .state
                    .version
                    .len()
                    .checked_add(bytes.len())
                    .is_some_and(|length| length < 128)
                {
                    self.state.version.extend_from_slice(bytes);
                }
            }
            0x3b => {
                ensure!((0..=4).contains(&a[0]), "DC3WY SYSGRP overflow");
                let slot = a[0] as u8 + 28;
                // 4233A0: '-' means reload the retained system filename,
                // unlike the later normal resource unload convention.
                let name = if text == "-" {
                    self.state
                        .image_names
                        .get(&slot)
                        .context("DC3WY system-image reload before a declaration")?
                        .as_str()
                } else {
                    text
                };
                if !host.emit(&Event::LoadImage {
                    slot,
                    name: name.to_owned(),
                })? {
                    return Ok(Outcome::Pending);
                }
                self.state
                    .image_names
                    .insert(slot, name.to_ascii_uppercase());
            }
            0x86 if a[0] >= 64 => {} //4233A0's actual upper-bound early return
            0x86 => {
                ensure!(a[0] >= 0, "DC3WY negative palette index");
                let rgb = [a[1] as u8, a[2] as u8, a[3] as u8];
                if !host.emit(&Event::Palette {
                    index: a[0] as u8,
                    rgb: logical_color(rgb),
                })? {
                    return Ok(Outcome::Pending);
                }
                self.state.palette.insert(a[0] as u8, rgb);
            }
            0x83 => {
                //426DCE/426DD6 selects renderer*(this+22488), object65.
                //The wrapper's hidden ECX receiver must not be confused with
                //the adjacent81's10/20/30/40 renderer selection.
                let sprite = self.state.sprites.entry(65).or_default();
                ensure!(!sprite.queued, "DC3WY queued message renderer position requires its queue bridge");
                if sprite.actor {
                    if !self.place_object(host, 65, [i32::from(a[0]), i32::from(a[1])])? {
                        return Ok(Outcome::Pending);
                    }
                } else {
                    sprite.position = [a[0], a[1]];
                }
            }
            0x22 => {
                // Source does nothing for0/8..12 and unknown selectors.
                // The pending button is transferred without 414390 here.
                if let Some(slot) = template_slot(a[0], false) {
                    self.state
                        .templates
                        .insert(slot, std::mem::take(&mut self.state.builder));
                }
            }
            0x18 if a[0] > 0 => {
                // 414390 runs even for a selector without a destination.
                let mut builder = self.state.builder.clone();
                builder.seal();
                if let Some(slot) = template_slot(a[0], true) {
                    if !host.emit(&Event::DefineGroup {
                        slot,
                        group: builder.group.clone(),
                    })? {
                        return Ok(Outcome::Pending);
                    }
                    self.state.templates.insert(slot, builder);
                    self.state.builder = Builder::default();
                } else {
                    self.state.builder = builder;
                }
            }
            0x18 => {
                // 41A580 clears the five live owners, not the stored
                // template pointers. No DC4PH preference offsets are used.
                if !host.emit(&Event::ClearUi)? {
                    return Ok(Outcome::Pending);
                }
                self.state.builder = Builder::default();
            }
            0x69 => {
                let label = program
                    .instruction(next)
                    .context("DC3WY button action literal")?;
                let Arguments::String(action) = &label.arguments else {
                    bail!("DC3WY button action is not a string token")
                };
                ensure!(
                    (0x42..=0x5f).contains(&label.opcode),
                    "DC3WY button requires a plain action literal"
                );
                let sounds = if self.state.builder.pending_button.is_some() {
                    [0; 2]
                } else {
                    self.state.builder.pending_sounds
                };
                let flags = if self.state.builder.pending_button.is_some() {
                    BTreeSet::new()
                } else {
                    self.state.builder.pending_flags.clone()
                };
                self.state.builder.seal();
                self.state.builder.pending_button = Some(Button {
                    bounds: [a[0], a[1], a[2], a[3]],
                    action: action.clone(),
                    sounds,
                    behavior_flags: flags.clone(),
                    ..Default::default()
                });
                self.state.builder.pending_sounds = sounds;
                self.state.builder.pending_flags = flags;
                self.state.auxiliary_flag = false;
                next = label.next;
            }
            0x15 => {
                // 410D00 retains each field when its operand is255.
                if a[0] != 255 {
                    self.state.builder.pending_sounds[0] = a[0] as u8;
                }
                if a[1] != 255 {
                    self.state.builder.pending_sounds[1] = a[1] as u8;
                }
                if let Some(button) = &mut self.state.builder.pending_button {
                    button.sounds = self.state.builder.pending_sounds;
                }
            }
            0x6a => {
                if a[7] == 5 {
                    let button = self
                        .state
                        .builder
                        .pending_button
                        .as_mut()
                        .context("DC3WY slider without current button")?;
                    ensure!(a[2] > 1, "DC3WY slider has undefined native reciprocal");
                    button.slider = Some(NativeSlider {
                        minimum: a[0],
                        maximum: a[1],
                        stops: a[2],
                        variable: i32::from(a[4]) + 1000 * i32::from(a[3]),
                        initial_value: 0,
                    });
                } else {
                    let state = match a[7] {
                        6 => 100,
                        8 => 101,
                        10 => 102,
                        11 => 3,
                        value => value,
                    };
                    let part = self.image_part(
                        a,
                        [0, 0],
                        state,
                        !matches!(a[7], 6 | 8 | 10 | 11) && self.state.auxiliary_flag,
                        host,
                    )?;
                    if let Some(part) = part {
                        let button = self
                            .state
                            .builder
                            .pending_button
                            .as_mut()
                            .context("DC3WY associated image without current button")?;
                        ensure!(
                            button.images.len() < 128,
                            "DC3WY button image declaration limit"
                        );
                        button.images.push(part);
                    }
                }
            }
            0x70 => {
                let base = self.state.builder.offset.map(i32::from);
                let mut operands = a;
                operands[6] &= 0x3f;
                if let Some(part) =
                    self.image_part(operands, base, 0, (a[6] as u8 as i8) < 0, host)?
                {
                    ensure!(
                        self.state.builder.group.images.len() < 4096,
                        "DC3WY standalone image declaration limit"
                    );
                    self.state.builder.group.images.push(part);
                }
            }
            0x19 => {
                if a[0] == 0 {
                    self.state.auxiliary_flag = !self.state.auxiliary_flag;
                }
            }
            0x4d => {
                // 42F660 returns-1 without moving the decoder for an absent
                // named label. Match the source label token, not a suffix.
                if let Some(target) = program
                    .instructions
                    .iter()
                    .take_while(|item| item.opcode != 0)
                    .find(|item| {
                        item.opcode == 0x44
                            && !text.is_empty()
                            && program.literal_bytes(item.offset)
                                == program.literal_bytes(instruction.offset)
                    })
                {
                    next = target.offset;
                }
            }
            0x13 if a[0] < 254 => {
                let milliseconds = u64::try_from(a[0]).context("DC3WY negative byte delay")? * 200;
                self.state.wait_skippable = a[1] != 0;
                let deadline = if milliseconds == 0 && self.state.wait_skippable {
                    u64::MAX
                } else {
                    now.saturating_add(milliseconds)
                };
                self.state.wait_until = Some(deadline);
                self.state.auxiliary_flag = false;
                outcome = Outcome::WaitUntil(deadline);
            }
            opcode if native_noop(opcode) => {}
            opcode => bail!(
                "unsupported original DC3WY instruction {opcode:#04x}; execution stopped without advancing"
            ),
        }
        self.advance(next, instruction.opcode)?;
        Ok(outcome)
    }
    fn advance(&mut self, next: u32, opcode: u8) -> Result<Outcome> {
        let instructions = self
            .state
            .instructions
            .checked_add(1)
            .context("DC3WY instruction counter overflow")?;
        self.state.last_offset = self.state.pc;
        self.state.pc = next;
        self.state.instructions = instructions;
        self.state.last_opcode = Some(opcode);
        self.state.observations = None;
        Ok(Outcome::Advanced)
    }

    fn image_part(
        &self,
        a: [i16; 8],
        base: [i32; 2],
        state: i16,
        auxiliary_flag: bool,
        host: &mut dyn NativeHost,
    ) -> Result<Option<ImagePart>> {
        ensure!((0..36).contains(&a[6]), "DC3WY native image slot");
        let slot = a[6] as u8;
        // 414440/4147A0 omit a null source texture. They do not invent an
        // image handle or wait for a resource the script never declared.
        if host.image_dimensions(slot)?.is_none() {
            return Ok(None);
        }
        Ok(Some(ImagePart {
            position: [base[0] + i32::from(a[0]), base[1] + i32::from(a[1])],
            source: [a[2], a[3], a[4], a[5]],
            slot,
            state,
            auxiliary_flag,
            resource_binding: host.image_identity(slot)?,
            binding_declared: true,
        }))
    }
}

///42F730 uses MOVSX for the opcode and marker bytes. In particular80..ff
///take its three-byte branch rather than its WORD branch. Preserve that
///source scan (which can inspect operand bytes), with bounded reads and an
///instruction-boundary check before transferring control to the decoder.
fn skip_marker(program: &Program, mut pc: u32, tag: i16) -> Result<u32> {
    for _ in 0..crate::MAX_INSTRUCTIONS {
        let opcode = program
            .native_byte(pc)
            .context("DC3WY marker scan outside script")? as i8;
        ensure!(
            opcode != 0,
            "DC3WY native conditional marker {tag} was not found"
        );
        if opcode == 0x1e
            && i16::from(
                program
                    .native_byte(pc + 2)
                    .context("DC3WY truncated conditional marker")? as i8,
            ) == tag
        {
            let target = pc.checked_add(3).context("DC3WY marker offset overflow")?;
            program.instruction(target)?;
            return Ok(target);
        }
        let width = if opcode < 0x39 {
            3
        } else if opcode >= 0x64 {
            9
        } else {
            let start = if opcode < 0x42 { 2 } else { 1 };
            let mut end = start;
            loop {
                ensure!(
                    end - start <= crate::MAX_STRING_BYTES as u32,
                    "DC3WY marker string limit"
                );
                if program
                    .native_byte(pc + end)
                    .context("DC3WY truncated marker string")?
                    == 0
                {
                    break;
                }
                end += 1;
            }
            end + 1
        };
        pc = pc
            .checked_add(width)
            .context("DC3WY marker scan overflow")?;
    }
    bail!("DC3WY conditional marker scan budget exceeded")
}

fn template_slot(selector: i16, finalizing: bool) -> Option<u8> {
    if finalizing {
        match selector {
            1 => Some(9),
            2 => Some(12),
            3 => Some(11),
            6 => Some(10),
            7 => Some(13),
            13 => Some(17),
            14 => Some(18),
            15 => Some(19),
            16 => Some(20),
            _ => None,
        }
    } else {
        match selector {
            1..=7 => Some((selector - 1) as u8),
            13 => Some(17),
            14 => Some(18),
            15 => Some(7),
            16 => Some(8),
            _ => None,
        }
    }
}
