// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The RScript script thread (dispatcher 0x4358A0 in the Shapeshifter build)
//! and its native handlers. The native
//! thread blocks on an event; here every blocking handler records a `Wait`
//! plus continuations, and the owner tick resumes them.
use crate::apini::{Apini, Dir};
use crate::gsc::{self, Program};
use crate::layouts::layout_1_11;
use crate::memory::{self, Memory, config as C, layer as L, message as M, scene as S, text_box as T};
use crate::res::{Resources, join};
use crate::scene::{Choice, Ctx, Layer, MessageWindow};
use crate::sprite::{Content, Sprite};
use anyhow::{Context, Result, bail};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AudioAsset, AudioParameters, AudioRequest, HostServices, InputFrame, MediaRequest},
    state::ClientId,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Text colour presets of opcode 0x5F, as 0xRRGGBB pixels.
const TEXT_COLORS: [u32; 9] = [0, 0xFFFFFF, 7991794, 12006195, 16768512, 16298479, 8380325, 12683254, 16425562];
pub const MUSIC_CHANNEL: u32 = 1;
pub const SOUND_CHANNEL: u32 = 10;
pub const VOICE_CHANNEL: u32 = 20;
pub const SYSTEM_CHANNEL: u32 = 30;
pub(crate) const MOVIE_ID: u32 = 1;
/// A scene context over the VM's disjoint resource fields.
macro_rules! cx {
    ($s:ident, $h:ident) => {
        Ctx { h: &mut *$h, client: $s.client, res: &mut $s.res, apini: &$s.apini }
    };
}
/// Stream fade timer period: 20 ms per fade step.
pub(crate) const FADE_STEP_MS: f32 = 20.0;

fn u16v(v: i32) -> i32 {
    v & 0xFFFF
}

/// MSVC `rand`.
#[derive(Clone, Copy)]
pub struct MsvcRand(pub u32);
impl MsvcRand {
    pub fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        ((self.0 >> 16) & 0x7FFF) as i32
    }
}

#[derive(Default, Clone)]
pub struct Flags {
    pub choice: bool,
    pub batch: bool,
    pub complete_animations: bool,
    pub wait_input: bool,
    pub wait_animation: bool,
    pub wait_click: bool,
    pub sleeping: bool,
    pub interrupt: bool,
    pub wait_sound: bool,
    pub wait_voice: bool,
    pub skip_held: bool,
    pub skip: bool,
    pub fast_skip: bool,
    pub fast_skip_request: bool,
    pub auto: bool,
    pub button_wait: u8,
    pub button_cancel: bool,
    pub button_timer: bool,
    pub window_hidden: bool,
}

#[derive(Clone)]
pub struct Thread {
    pub program: Option<Arc<Program>>,
    pub script: i32,
    pub pc: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Wait {
    /// The native default case and unimplemented handlers: one frame.
    Frame,
    /// Until no object animates.
    Refresh,
    /// A state other than the game is shown (title, extras).
    State,
    /// A zoom or shrink transition runs.
    Zoom,
    /// A click (or skip/auto).
    Click,
    Auto { until: u64 },
    /// A real-time sleep a click interrupts.
    Sleep { until: u64 },
    Audio { channel: u32 },
    Choice,
    Buttons,
    /// A display effect that ends at `until`.
    Until { until: u64 },
    /// Shake/flash steps: (duration ms, offset, fill colour).
    Steps { steps: Vec<(u64, [i32; 2], Option<u32>)>, at: u64 },
    Movie,
    /// The text-entry dialog of 0x9C is open.
    Edit,
    /// The save screen of 0x11 is open.
    Screen,
    /// A mask image being prepared for a wipe.
    Mask { name: String, duration: u64 },
}

/// Work left after a wait, run in LIFO order.
#[derive(Debug, Clone)]
pub enum Cont {
    /// The click loop of click and button waits: re-wait after a system call.
    ClickDone,
    ClearBox(usize),
    MessageRevealed { b: usize, wait: bool, mode: i32, new_page: bool, voice: u32, repeat: i32 },
    MessageClicked { b: usize, mode: i32, new_page: bool, voice: u32, repeat: i32 },
    ChoiceShown { targets: [u32; 5], effect_mode: i32 },
    DynShown { effect_mode: i32 },
    DynAnswered { effect_mode: i32 },
    DynClosed,
    MapDone { timer_var: i32, hide_timer: i32, cancel: bool },
    RetryMap { timer_var: i32, hide_timer: i32, cancel: bool },
    ChoiceAnswered { targets: [u32; 5], effect_mode: i32 },
    ChoiceClosed { targets: [u32; 5] },
    ButtonsDone { timer_var: i32, hide_timer: i32, cancel: bool },
    RetryClick,
    PointerDone { x_var: i32, y_var: i32, timer_var: i32, hide_timer: i32, cancel: bool },
    RetryPointer { x_var: i32, y_var: i32, timer_var: i32, hide_timer: i32, cancel: bool },
    RetryButtons { timer_var: i32, hide_timer: i32, cancel: bool },
    Opening,
    /// Second half of a dip through black/white.
    DipBack { duration: u64 },
    WaitInputEnd,
    WaitSoundEnd,
    WaitVoiceEnd,
}

/// The outer thread saved by a system call.
pub struct Nest {
    scene: Vec<u8>,
    thread: Thread,
    cont: Vec<Cont>,
    wait: Option<Wait>,
}

pub struct Vm {
    pub client: ClientId,
    pub apini: Apini,
    pub res: Resources,
    pub mem: Memory,
    pub programs: BTreeMap<i32, Arc<Program>>,
    pub thread: Thread,
    pub nest: Vec<Nest>,
    pub flags: Flags,
    pub wait: Option<Wait>,
    pub cont: Vec<Cont>,
    pub layers: Vec<Layer>,
    pub message: MessageWindow,
    pub choice: Option<Choice>,
    pub effect: Sprite,
    pub overlay: Sprite,
    pub rng: MsvcRand,
    pub now: u64,
    pub next_tick: u64,
    pub pointer: [i32; 2],
    pub input: InputFrame,
    /// Pending presentation (native display.update).
    pub dirty: bool,
    /// A transition captures the presented frame before the next present.
    pub transition: Option<serde_json::Value>,
    pub shake: [i32; 2],
    pub flash: Option<u32>,
    pub dip: Option<u32>,
    pub masks: BTreeMap<String, String>,
    pub mask_job: Option<(String, crysirror_core::image_processing::ImageJob)>,
    pub next_image: u32,
    /// Read text (key = script, bit = string index), kept in the system save.
    pub read_text: crate::save::BitSets,
    pub music: Option<u16>,
    pub executed: u64,
    pub halted: bool,
    pub movie_queue: Vec<u16>,
    pub button_timer: (i32, u64),
    /// The voice stops after the click.
    pub voice_stop: bool,
    pub music_stream: u32,
    pub loaded_sound: [Option<u16>; 3],
    pub audio_pending: Vec<u64>,
    /// Media surface path of the playing movie.
    pub movie: Option<String>,
    pub presented: crate::present::Presented,
    /// word_4ED28C: set when the game started from the legacy top menu.
    pub top_menu_start: bool,
    pub map: crate::screens::MapScreen,
    pub edit: Option<crate::screens::EditDialog>,
    pub save_screen: Option<crate::save::SaveScreen>,
    /// Project font alias of engine dialogs.
    pub ui_font: String,
    pub gauges: Vec<crate::screens::Gauge>,
    pub dyn_select: Option<crate::screens::DynSelect>,
    /// Seen-image bit set (key 0) shared with 0x72, kept in the system save.
    pub seen_images: crate::save::BitSets,
    /// An open native question or a slot header being stamped.
    pub pending: Option<crate::save::Pending>,
    /// The configuration screen (CConfig).
    pub config_screen: Option<crate::config::ConfigScreen>,
    /// The backlog entry shown in box 0 while browsing.
    pub backlog_view: Option<usize>,
    /// The title, its sub-states and the extras (no game state).
    pub state_screen: Option<crate::extras::StateScreen>,
    /// Face chosen on the configuration screen (an authorized alias).
    pub text_font: Option<String>,
    /// Audio muted while the window is inactive (background audio off).
    pub inactive_muted: bool,
    /// Ticks without input while waiting (auto hide, dword_4ED124).
    pub idle_ticks: u32,
    pub idle_hidden: bool,
    /// The panel slider is being dragged.
    pub panel_drag: bool,
    /// A framework restore or original-save import re-enters the game.
    pub pending_restore: bool,
    /// The running zoom/shrink transition.
    pub zoom: Option<crate::zoom::Zoom>,
    /// A scene-view replay runs (word_4ED28C).
    pub replaying: bool,
    /// The scene in use when the scene view opened (dword_50AEE8).
    pub scene_view_saved: Option<Vec<u8>>,
    pub hovered: Option<crate::owner::Hit>,
    pub ctrl_skip: bool,
}

impl Vm {
    pub fn new(client: ClientId, apini: Apini, res: Resources) -> Self {
        let seed = 0x5EED;
        Self {
            client,
            apini,
            res,
            mem: Memory::default(),
            programs: BTreeMap::new(),
            thread: Thread { program: None, script: -1, pc: 0 },
            nest: Vec::new(),
            flags: Flags::default(),
            wait: None,
            cont: Vec::new(),
            layers: (0..S::LAYER_COUNT).map(Layer::new).collect(),
            message: MessageWindow::new(),
            choice: None,
            effect: Sprite::default(),
            overlay: Sprite::default(),
            rng: MsvcRand(seed),
            now: 0,
            next_tick: 0,
            pointer: [0, 0],
            input: InputFrame::default(),
            dirty: true,
            transition: None,
            shake: [0, 0],
            flash: None,
            dip: None,
            masks: BTreeMap::new(),
            mask_job: None,
            next_image: 0,
            read_text: Default::default(),
            music: None,
            executed: 0,
            halted: false,
            movie_queue: Vec::new(),
            button_timer: (0, 0),
            voice_stop: false,
            music_stream: 0,
            loaded_sound: [None; 3],
            audio_pending: Vec::new(),
            movie: None,
            presented: Default::default(),
            top_menu_start: false,
            seen_images: Default::default(),
            pending: None,
            config_screen: None,
            backlog_view: None,
            state_screen: None,
            text_font: None,
            inactive_muted: false,
            idle_ticks: 0,
            idle_hidden: false,
            panel_drag: false,
            pending_restore: false,
            zoom: None,
            replaying: false,
            scene_view_saved: None,
            map: Default::default(),
            edit: None,
            save_screen: None,
            ui_font: String::new(),
            gauges: (0..S::GAUGE_COUNT).map(|_| Default::default()).collect(),
            dyn_select: None,
            hovered: None,
            ctrl_skip: false,
        }
    }
    pub fn screen(&self) -> [i32; 2] {
        [self.apini.width as i32, self.apini.height as i32]
    }
    fn skipping(&self) -> bool {
        self.flags.fast_skip
    }
    fn effects_enabled(&self) -> bool {
        self.mem.cfg(C::EFFECTS_ENABLED) != 0
    }
    /// The effect value, dropped while skipping or with effects disabled.
    fn effect_of(&self, v: i32) -> i32 {
        if self.flags.skip || !self.effects_enabled() { 0 } else { v }
    }
    pub fn depth(&self) -> usize {
        usize::from(self.mem.uw(S::CALL_DEPTH))
    }
    fn script_at(&self, depth: usize) -> i32 {
        i32::from(self.mem.uw(S::SCRIPT_STACK + depth * 2))
    }
    fn set_script_at(&mut self, depth: usize, script: i32) {
        self.mem.set_w(S::SCRIPT_STACK + depth * 2, script);
    }
    pub(crate) fn return_at(&self, depth: usize) -> u32 {
        self.mem.d(S::RETURN_STACK + depth * 4) as u32
    }
    fn set_return_at(&mut self, depth: usize, pc: u32) {
        self.mem.set_d(S::RETURN_STACK + depth * 4, pc as i32);
    }
    pub fn current_script(&self) -> i32 {
        self.script_at(self.depth())
    }
    /// `value` operands: low signed word, high word counts indirections.
    pub fn value(&self, raw: i32) -> i32 {
        let mut v = i32::from(raw as i16);
        for _ in 0..(raw >> 16).max(0) {
            v = i32::from(self.mem.var(v));
        }
        v
    }
    fn set_var(&mut self, i: i32, v: i32) {
        if !self.mem.set_var(i, v as i16) {
            self.res.diag(format!("variable {i} is out of range"));
        }
    }
    fn string(&self, index: i32) -> Result<Vec<u8>> {
        let p = self.thread.program.as_ref().context("no GSC program loaded")?;
        let s = usize::try_from(index).ok().and_then(|i| p.strings.get(i)).with_context(|| format!("GSC string {index} out of range in script {}", self.thread.script))?;
        Ok(self.expand(s))
    }
    pub fn expand(&self, s: &[u8]) -> Vec<u8> {
        crate::text::expand(
            s,
            |levels, mut v| {
                for _ in 0..levels {
                    v = i32::from(self.mem.var(v));
                }
                v
            },
            |r| self.mem.str_at(S::STRING_REGISTERS + r * S::STRING_STRIDE, S::STRING_STRIDE).to_vec(),
        )
    }

    // ----------------------------------------------------------- program flow

    pub(crate) fn load(&mut self, script: i32) -> Result<()> {
        let depth = self.depth();
        self.set_script_at(depth, script);
        if self.thread.script != script || self.thread.program.is_none() {
            let p = self.programs.get(&script).cloned().with_context(|| format!("RScript script {script:04}.gsc is not available (native compile error box)"))?;
            self.thread.program = Some(p);
            self.thread.script = script;
        }
        Ok(())
    }
    pub(crate) fn jump(&mut self, pc: u32) {
        if self.thread.program.as_ref().is_some_and(|p| pc as usize >= p.code.len()) {
            self.res.diag(format!("GSC jump to {pc:#x} is outside script {}", self.thread.script));
        }
        self.thread.pc = pc;
    }
    fn label_name(&self, label: i32) -> Result<Option<Vec<u8>>> {
        Ok(if label != 0 { Some(self.string(label)?) } else { None })
    }
    fn goto(&mut self, script: i32, label: i32) -> Result<()> {
        let name = self.label_name(label)?;
        self.load(script)?;
        let off = self.thread.program.as_ref().unwrap().label_offset(name.as_deref());
        self.jump(off);
        Ok(())
    }
    fn call(&mut self, script: i32, label: i32, params: &[i32]) -> Result<()> {
        if self.depth() >= usize::from(memory::MAX_DEPTH) {
            bail!("RScript script nest overflow (native message box ends the thread)");
        }
        let values: Vec<i32> = params.iter().map(|&r| self.value(r)).collect();
        let d = self.depth();
        self.set_return_at(d, self.thread.pc);
        let name = self.label_name(label)?;
        self.mem.set_w(S::CALL_DEPTH, d as i32 + 1);
        self.load(script)?;
        let off = self.thread.program.as_ref().unwrap().label_offset(name.as_deref());
        self.jump(off);
        for (i, v) in values.iter().enumerate().take(10) {
            self.set_var(10 + i as i32, *v);
        }
        Ok(())
    }
    fn local_call(&mut self, target: u32, params: &[i32]) -> Result<()> {
        if self.depth() >= usize::from(memory::MAX_DEPTH) {
            bail!("RScript script nest overflow (native message box ends the thread)");
        }
        let d = self.depth();
        self.set_return_at(d, self.thread.pc);
        let s = self.script_at(d);
        self.set_script_at(d + 1, s);
        self.mem.set_w(S::CALL_DEPTH, d as i32 + 1);
        self.jump(target);
        let values: Vec<i32> = params.iter().map(|&r| self.value(r)).collect();
        for (i, v) in values.iter().enumerate().take(10) {
            self.set_var(10 + i as i32, *v);
        }
        Ok(())
    }
    fn ret(&mut self, result: i32) -> Result<()> {
        let d = self.depth();
        if d == 0 {
            bail!("RScript script nest underflow (native message box ends the thread)");
        }
        self.mem.set_w(S::CALL_DEPTH, d as i32 - 1);
        let s = self.script_at(d - 1);
        self.load(s)?;
        let pc = self.return_at(d - 1);
        self.jump(pc);
        self.set_var(0, result);
        Ok(())
    }
    fn operand(&self, op: u16, v: i32, left: bool) -> i32 {
        let high = op >> 8;
        let mode = if left { (high >> 2) & 3 } else { high & 3 };
        let levels = if left { (op >> 4) & 0xF } else { op & 0xF };
        match mode {
            0 => v,
            1 => usize::try_from(v).ok().and_then(|i| self.mem.regs.get(i)).map_or(0, |r| i32::from(*r)),
            2 => {
                let mut i = v;
                for _ in 0..levels {
                    i = i32::from(self.mem.var(i));
                }
                i32::from(self.mem.var(i))
            }
            _ => 0,
        }
    }
    /// Expression family (results stored as signed words).
    fn expression(&mut self, op: u16, o: &[i32]) {
        let target = o[0] as usize;
        let store = |vm: &mut Self, v: i32| {
            if let Some(r) = vm.mem.regs.get_mut(target) {
                *r = v as i16;
            }
        };
        if op & 0xF000 == 0xF000 {
            let v = self.operand(op, o[1], false);
            store(self, v);
            return;
        }
        let (a, b) = (o[1], o[2]);
        let right = self.operand(op, b, false);
        let left = || self.operand(op, a, true);
        let v = match op & 0xF000 {
            0x1000 => {
                match (op >> 10) & 3 {
                    1 => {
                        if let Some(r) = self.mem.regs.get_mut(a as usize) {
                            *r = right as i16;
                        }
                    }
                    2 => {
                        let mut i = a;
                        for _ in 0..((op >> 4) & 0xF) {
                            i = i32::from(self.mem.var(i));
                        }
                        self.set_var(i, right);
                    }
                    _ => {}
                }
                right
            }
            0x2000 => i32::from(left() != 0 || right != 0),
            0x3000 => i32::from(left() != 0 && right != 0),
            0x4000 => i32::from(left() == right),
            0x5000 => i32::from(left() >= right),
            0x6000 => i32::from(left() > right),
            0x7000 => i32::from(left() <= right),
            0x8000 => i32::from(left() < right),
            0x9000 => i32::from(left() != right),
            0xA000 => left().wrapping_add(right),
            0xB000 => left().wrapping_sub(right),
            0xC000 => left().wrapping_mul(right),
            _ => {
                let l = left();
                if right == 0 {
                    self.res.diag(format!("division by zero in script {}", self.thread.script));
                    0
                } else if op & 0xF000 == 0xD000 {
                    l.wrapping_div(right)
                } else {
                    l.wrapping_rem(right)
                }
            }
        };
        store(self, v);
    }

    // ------------------------------------------------------------- execution

    /// Runs instructions until a handler waits or the budget is spent.
    pub fn run(&mut self, h: &mut dyn HostServices, budget: u32) -> Result<()> {
        let mut left = budget.max(1);
        while self.wait.is_none() && !self.halted {
            if left == 0 {
                return Ok(());
            }
            left -= 1;
            self.step_instruction(h)?;
        }
        Ok(())
    }

    fn step_instruction(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if !self.flags.batch {
            self.flags.skip = self.flags.skip_held;
        }
        if self.flags.fast_skip_request {
            self.flags.fast_skip = true;
            self.flags.fast_skip_request = false;
        }
        let program = self.thread.program.clone().context("no GSC program loaded")?;
        let d = self.depth();
        self.set_return_at(d, self.thread.pc);
        let pc = self.thread.pc;
        if pc as usize + 2 > program.code.len() {
            // Reads past the end return zero natively: the default case.
            self.wait = Some(Wait::Frame);
            return Ok(());
        }
        let op = u16::from_le_bytes([program.code[pc as usize], program.code[pc as usize + 1]]);
        if !gsc::is_expression(op) && layout_1_11(op).is_none() {
            self.thread.pc += 2;
            self.wait = Some(Wait::Frame);
            return Ok(());
        }
        let ins = gsc::decode(&program.code, pc, layout_1_11)?;
        self.thread.pc = ins.next;
        self.executed += 1;
        if gsc::is_expression(op) {
            self.expression(op, &ins.operands);
            return Ok(());
        }
        self.execute(h, op, &ins.operands, &program)
            .with_context(|| format!("RScript opcode {op:#04x} at {:04}.gsc:{pc:#x}", self.thread.script))
    }

    /// sub_43D610: the paged selection built by 0xD2..0xD4.
    fn dyn_select_open(&mut self, h: &mut dyn HostServices, effect_mode: i32, selmap: i32, arrangement: i32) -> Result<()> {
        if self.flags.batch {
            self.flags.batch = false;
            self.dirty = true;
        }
        if self.flags.fast_skip {
            self.flags.fast_skip = false;
            self.rebuild(h);
        }
        self.stop_auto();
        let visible = crate::screens::DynSelect::filter(&self.mem);
        if visible.is_empty() {
            self.set_var(0, 0);
            return Ok(());
        }
        let custom = join(self.apini.dir(Dir::System), &format!("selmap{selmap:02}"));
        if self.res.lwg_of(&custom).is_some() {
            bail!("RScript selection layout {custom}.lwg is not mapped");
        }
        let mut dyn_select = crate::screens::DynSelect { visible, cursor: 0, page: Vec::new(), arrangement };
        dyn_select.next_page();
        self.dyn_select = Some(dyn_select);
        let instant = !self.effects_enabled() || self.flags.skip || effect_mode != 0;
        self.dyn_build(h, instant)?;
        self.cont.push(Cont::DynShown { effect_mode });
        self.present_choice(h, effect_mode)
    }
    /// Builds the choice plates of the current page.
    pub(crate) fn dyn_build(&mut self, h: &mut dyn HostServices, instant: bool) -> Result<()> {
        let header = (self.mem.uw(S::SELECTION + 4), self.mem.d(S::SELECTION + 8) as u32);
        let question = self.text_of(header.0, header.1)?;
        // sub_43D610 keeps the question as the slot page text.
        self.message.page_text = question.clone();
        let page = self.dyn_select.as_ref().map(|d| d.page.clone()).unwrap_or_default();
        let mut answers = Vec::new();
        for item in &page {
            answers.push(match item {
                Some(i) => {
                    let (script, text, _, _) = crate::screens::entry(&self.mem, *i);
                    self.text_of(script, text)?
                }
                None => {
                    let (script, text) = (self.mem.uw(S::SELECTION + 0x4BC), self.mem.d(S::SELECTION + 0x4C0) as u32);
                    if script != 0 || text != 0 { self.text_of(script, text)? } else { crate::screens::NEXT_LABEL.to_vec() }
                }
            });
        }
        let arrangement = self.dyn_select.as_ref().map_or(0, |d| d.arrangement);
        let mut choice = {
            let mut cx = cx!(self, h);
            Choice::open(&mut cx, &question, &answers, arrangement)
        };
        choice.appear(instant);
        self.choice = Some(choice);
        self.dirty = true;
        Ok(())
    }
    /// Expanded string `text` of script `script` (selection callbacks).
    fn text_of(&self, script: u16, text: u32) -> Result<Vec<u8>> {
        let p = self.programs.get(&i32::from(script)).with_context(|| format!("RScript script {script:04}.gsc is not available"))?;
        let s = p.strings.get(text as usize).with_context(|| format!("GSC string {text} out of range in script {script}"))?;
        Ok(self.expand(s))
    }
    /// sub_43E320: waits for a map button.
    pub(crate) fn map_wait(&mut self, timer_var: i32, hide_timer: i32, cancel: bool) {
        self.flags.button_cancel = cancel;
        if self.flags.batch {
            self.flags.batch = false;
            self.dirty = true;
        }
        self.flags.fast_skip = false;
        self.stop_auto();
        self.flags.interrupt = false;
        self.flags.button_wait = 2;
        if timer_var != 0 {
            if hide_timer == 0 {
                self.start_timer_gauge(i32::from(self.mem.var(timer_var)));
            }
            self.flags.button_timer = true;
            self.button_timer = (i32::from(self.mem.var(timer_var)), self.now);
        } else {
            self.message.set_input(true);
        }
        self.map.set_input(true);
        self.flags.wait_input = true;
        self.dirty = true;
        self.cont.push(Cont::MapDone { timer_var, hide_timer, cancel });
        self.wait = Some(Wait::Buttons);
    }
    /// Scene offset of string register `i` (0..9).
    fn reg_at(i: i32) -> Result<usize> {
        let i = usize::try_from(i & 0xFFFF).unwrap_or(usize::MAX);
        if i >= 10 {
            bail!("RScript string register {i} is out of range");
        }
        Ok(S::STRING_REGISTERS + i * S::STRING_STRIDE)
    }
    /// Gauge records (scene +0x5030, 52 bytes each): visibility +0, position
    /// +4/+8, digits position +12/+16, bar position +20/+24, Num/Bar/Fix
    /// images +28/+30/+32, maximum +36, minimum +40, value +44, words
    /// +48/+50.
    fn gauge_op(&mut self, op: u16, raw: &[i32]) -> Result<()> {
        let v = self.vals(raw);
        let g = u16v(v[0]) as usize;
        if g >= S::GAUGE_COUNT {
            bail!("RScript gauge {g} is out of range");
        }
        let r = S::GAUGES + S::GAUGE_STRIDE * g;
        match op {
            0x82 => {
                self.mem.set_w(r + 28, v[1]);
                self.mem.set_w(r + 30, v[2]);
                self.mem.set_w(r + 32, v[3]);
            }
            0x83 => {
                self.mem.set_d(r + 40, v[1]);
                self.mem.set_d(r + 36, v[2]);
                self.mem.set_w(r + 48, v[3]);
                self.mem.set_w(r + 50, v[4]);
            }
            0x86 => {
                self.mem.set_d(r + 4, v[1]);
                self.mem.set_d(r + 8, v[2]);
            }
            0x87 => {
                for (k, x) in v[1..5].iter().enumerate() {
                    self.mem.set_d(r + 12 + 4 * k, *x);
                }
            }
            _ => {
                // 0x88: value, clamped to [min, max] (sub_4652D0); an effect
                // animates the bar unless skipping or effects are off.
                let (max, min) = (self.mem.d(r + 36), self.mem.d(r + 40));
                let value = if v[1] > max { max } else if v[1] < min { self.mem.d(r + 44) } else { v[1] };
                self.mem.set_d(r + 44, value);
                self.dirty = true;
                let animate = v[2] != 0 && self.effects_enabled() && !self.flags.skip;
                self.gauges[g].animating = animate;
                if !animate {
                    self.gauges[g].shown = value;
                }
                if animate && !self.flags.batch {
                    self.refresh();
                }
            }
        }
        if !self.skipping() {
            self.dirty = true;
        }
        Ok(())
    }
    /// Loads a gauge's `Num%02d`, `Bar%02d` and `Fix%02d` images and its
    /// shown value (sub_41C850, sub_465300).
    pub(crate) fn load_gauge(&mut self, h: &mut dyn HostServices, g: usize) {
        let r = S::GAUGES + S::GAUGE_STRIDE * g;
        let sys = self.apini.dir(Dir::System).to_owned();
        let mut img = |name: &str, n: u16| if n == 0 { None } else { self.res.image(h, self.client, &join(&sys, &format!("{name}{n:02}"))) };
        let digits = img("Num", self.mem.uw(r + 28));
        let bar = img("Bar", self.mem.uw(r + 30));
        let fix = img("Fix", self.mem.uw(r + 32));
        let gauge = &mut self.gauges[g];
        gauge.digits = digits;
        gauge.bar = bar;
        gauge.fix = crate::sprite::Sprite { visible: fix.is_some(), content: fix, ..Default::default() };
        gauge.target = self.mem.d(r + 44);
        if !gauge.animating {
            gauge.shown = gauge.target;
        }
        self.dirty = true;
    }
    /// sub_43E0C0 with a shown timer: gauge 0 counts down from the timer
    /// (sub_41CDE0(min 0, max timer, 4, 0), sub_41CE60(timer)).
    fn start_timer_gauge(&mut self, timer: i32) {
        let r = S::GAUGES;
        self.mem.set_d(r + 40, 0);
        self.mem.set_d(r + 36, timer);
        self.mem.set_w(r + 48, 4);
        self.mem.set_w(r + 50, 0);
        self.update_timer_gauge(timer);
    }
    /// sub_41CE60(remaining) without animation.
    pub(crate) fn update_timer_gauge(&mut self, remaining: i32) {
        self.mem.set_d(S::GAUGES + 44, remaining);
        let g = &mut self.gauges[0];
        if g.shown != remaining || g.target != remaining {
            g.target = remaining;
            g.shown = remaining;
            g.animating = false;
            self.dirty = true;
        }
    }
    /// sub_43E0C0: waits for a click anywhere, then stores the pointer.
    pub(crate) fn pointer_wait(&mut self, x_var: i32, y_var: i32, timer_var: i32, hide_timer: i32, cancel: bool) {
        self.flags.button_cancel = cancel;
        if self.flags.batch {
            self.flags.batch = false;
            self.dirty = true;
        }
        self.flags.fast_skip = false;
        self.stop_auto();
        self.flags.interrupt = false;
        self.flags.button_wait = 3;
        if timer_var != 0 {
            if hide_timer == 0 {
                self.start_timer_gauge(i32::from(self.mem.var(timer_var)));
            }
            self.flags.button_timer = true;
            self.button_timer = (i32::from(self.mem.var(timer_var)), self.now);
        } else {
            self.message.set_input(true);
        }
        self.flags.wait_input = true;
        self.cont.push(Cont::PointerDone { x_var, y_var, timer_var, hide_timer, cancel });
        self.wait = Some(Wait::Buttons);
    }
    fn vals(&self, raw: &[i32]) -> Vec<i32> {
        raw.iter().map(|&r| self.value(r)).collect()
    }

    fn execute(&mut self, h: &mut dyn HostServices, op: u16, raw: &[i32], program: &Arc<Program>) -> Result<()> {
        let screen = self.screen();
        match op {
            0x03 => {
                if self.mem.regs[0] == 0 {
                    self.jump(raw[0] as u32);
                }
            }
            0x04 => {
                if self.mem.regs[0] != 0 {
                    self.jump(raw[0] as u32);
                }
            }
            0x05 => self.jump(raw[0] as u32),
            0x08 => self.end_thread(h)?,
            0x09 => {
                let r = raw[0] as usize % memory::REGISTERS;
                self.mem.regs[r] = self.rng.next() as i16;
            }
            0x0A => {
                self.snapshot();
                self.wait_click();
            }
            0x0B => {
                self.snapshot();
                if !self.skipping() {
                    self.cont.push(Cont::ClearBox(0));
                    self.wait_click();
                }
            }
            0x0C => {
                let s = self.value(raw[0]);
                self.goto(s, raw[1])?;
            }
            0x0D => {
                if !(self.flags.skip || self.skipping()) {
                    let t = u16v(self.value(raw[0]));
                    self.flags.wait_input = true;
                    self.flags.sleeping = true;
                    self.cont.push(Cont::WaitInputEnd);
                    self.wait = Some(Wait::Sleep { until: self.now + 100 * t as u64 });
                }
            }
            0x0E => self.choose(h, raw)?,
            0x0F => {
                let s = self.value(raw[0]);
                self.call(s, raw[1], &raw[2..12])?;
            }
            0x10 => {
                let v = self.value(raw[0]);
                self.ret(v)?;
            }
            0x12 => {
                let at = u16v(self.value(raw[0])) as usize;
                if let Some(a) = usize::try_from(raw[1]).ok().and_then(|i| program.arrays.get(i)) {
                    let n = a.len().min(self.mem.vars.len().saturating_sub(at));
                    self.mem.vars[at..at + n].copy_from_slice(&a[..n]);
                }
            }
            0x13 => {
                let s = u16v(self.value(raw[0]));
                self.system_call(h, s)?;
            }
            0x14 => {
                let v = self.vals(raw);
                let image = u16v(v[0]) as u16;
                let sk = self.skipping();
                self.mark_seen(image);
                let mut cx = cx!(self, h);
                let layer = &mut self.layers[0];
                layer.set_kind(&mut self.mem, screen, 0, sk);
                if image != 0 {
                    layer.load(&mut self.mem, &mut cx, image, 0, 0, 0, v[1], sk);
                } else {
                    layer.fill(&mut self.mem, &mut cx, 0, sk);
                }
                if !self.flags.batch {
                    self.transition(h, 0, 15, 0)?;
                }
            }
            0x15 => {
                let white = self.value(raw[0]) == 1;
                let sk = self.skipping();
                let mut cx = cx!(self, h);
                self.layers[0].set_kind(&mut self.mem, screen, 0, sk);
                self.layers[0].fill(&mut self.mem, &mut cx, if white { 0xFFFFFF } else { 0 }, sk);
                if !self.flags.batch {
                    self.transition(h, 1, 15, 0)?;
                }
            }
            0x16 => {
                let v = self.vals(raw);
                self.move_layer(0, v[0], v[1], v[2], v[3], false);
                self.after_layers();
            }
            0x17 => {
                let v = self.vals(raw);
                if !(self.flags.skip || self.skipping() || !self.effects_enabled()) {
                    let times = u16v(v[0]).max(1);
                    let steps = match v[1] {
                        1 => shake(times, 15, 5, true),
                        2 => shake_loop(times, 60, 5),
                        _ => shake(times, 15, 5, false),
                    };
                    self.start_steps(steps);
                }
            }
            0x18 => {
                let v = self.vals(raw);
                if !(self.flags.skip || self.skipping() || !self.effects_enabled()) {
                    let times = u16v(v[0]).max(1);
                    let mut steps = Vec::new();
                    let colors: Vec<u32> = if v[1] == 1 {
                        vec![0xFFFFFF, 0x0000FF, 0xFFFFFF, 0xFF0000, 0xFFFFFF, 0xFFFF00]
                    } else {
                        // COLORREF 0xFF is red, 0x80FFFF a pale yellow.
                        vec![match v[1] {
                            2 => 0xFF0000,
                            3 => 0xFFFF80,
                            _ => 0xFFFFFF,
                        }]
                    };
                    for _ in 0..times {
                        for &c in &colors {
                            steps.push((31, [0, 0], Some(c)));
                            steps.push((31, [0, 0], None));
                        }
                    }
                    self.start_steps(steps);
                }
            }
            0x1A => self.flags.batch = true,
            0x1B => self.flush_batch(),
            0x1C => {
                let v = self.vals(raw);
                self.flags.batch = false;
                self.transition(h, u16v(v[0]), v[1], u16v(v[2]))?;
            }
            0x1D => {
                let v = self.vals(raw);
                self.flags.batch = false;
                self.transition(h, if v[0] != 0 { 100 } else { 200 } + v[1], 0, 0)?;
            }
            0x1E => {
                // The dispatcher hands 0x1E operands over reordered:
                // (layer, image, x, y, effect, flags).
                let v = self.vals(raw);
                let id = u16v(v[0]) as usize;
                if id != 0 && id < S::LAYER_COUNT {
                    let (sx, sy) = (i32::from(self.mem.w(S::SCALE_X)), i32::from(self.mem.w(S::SCALE_Y)));
                    let effect = self.effect_of(v[4]);
                    let sk = self.skipping();
                    let mut cx = cx!(self, h);
                    self.layers[id].load(&mut self.mem, &mut cx, u16v(v[1]) as u16, effect, v[2] * sx, v[3] * sy, v[5], sk);
                    self.after_layers();
                } else if id >= S::LAYER_COUNT {
                    self.res.diag(format!("layer {id} is out of range"));
                }
            }
            0x21 | 0x22 => {
                let v = self.vals(raw);
                let (sx, sy) = (i32::from(self.mem.w(S::SCALE_X)), i32::from(self.mem.w(S::SCALE_Y)));
                for i in self.targets(u16v(v[0])) {
                    self.move_layer(i, v[3], v[1] * sx, v[2] * sy, v[4], op == 0x22);
                }
                self.after_layers();
            }
            0x24 => {
                let v = self.vals(raw);
                let effect = self.effect_of(u16v(v[1]));
                let sk = self.skipping();
                for i in self.targets(u16v(v[0])) {
                    self.layers[i].hide(&mut self.mem, screen, effect, sk);
                }
                self.after_layers();
            }
            0x25 => {
                let v = self.vals(raw);
                let sk = self.skipping();
                for i in self.targets(u16v(v[0])) {
                    self.layers[i].set_visible(&mut self.mem, v[1], sk);
                }
                self.dirty |= !sk;
            }
            0x26 => {
                let v = self.vals(raw);
                for i in self.targets(u16v(v[0])) {
                    let r = S::LAYERS + i * S::LAYER_STRIDE;
                    self.mem.set_d(r + L::CENTER_X, v[1]);
                    self.mem.set_d(r + L::CENTER_Y, v[2]);
                    self.mem.set_d(r + L::KEEP_ON_SCREEN, v[3]);
                }
            }
            0x27 => {
                let v = self.vals(raw);
                let sk = self.skipping();
                for i in self.targets(u16v(v[0])) {
                    self.layers[i].set_blend(&mut self.mem, v[1], u16v(v[2]), sk);
                }
            }
            0x28 => {
                let v = self.vals(raw);
                for i in self.targets(u16v(v[0])) {
                    self.mem.set_w(S::LAYER_ORDERS + 2 * i, v[1]);
                }
                self.dirty |= !self.skipping();
            }
            0x29 => {
                let v = self.vals(raw);
                let id = u16v(v[0]) as usize;
                if id != 0 && id < S::LAYER_COUNT {
                    self.mem.set_w(S::LAYER_GROUPS + 2 * id, v[1]);
                }
            }
            0x2A => {
                let v = self.vals(raw);
                let sk = self.skipping();
                let mut cx = cx!(self, h);
                self.message.show_box(&mut self.mem, &mut cx, (u16v(v[0]) & 3) as usize, v[1] != 0, sk);
                self.after_layers();
            }
            0x2B => {
                let v = self.vals(raw);
                self.mem.set_w(S::EFFECT_IMAGE, v[0]);
                self.mem.set_w(S::EFFECT_BLEND, v[1]);
                if !self.skipping() {
                    self.apply_effect_screen(h);
                }
            }
            0x2C => {
                let v = self.value(raw[0]);
                self.mem.set_w(S::EFFECT_ORDER, v);
                self.dirty = true;
            }
            0x2D => {
                let v = self.vals(raw);
                self.mem.set_w(S::OVERLAY_PERCENT, v[0]);
                self.mem.set_w(S::OVERLAY_BLEND, v[1]);
                if !self.skipping() {
                    self.apply_overlay();
                    if !self.flags.batch {
                        self.transition(h, 1, 15, 0)?;
                    }
                }
            }
            0x2E => {
                let v = self.value(raw[0]);
                self.mem.set_w(S::OVERLAY_ORDER, v);
                self.dirty = true;
            }
            0x2F => {
                let v = self.vals(raw);
                self.mem.set_w(S::SCALE_X, v[0]);
                self.mem.set_w(S::SCALE_Y, v[1]);
            }
            0x31 => {
                let v = self.vals(raw);
                let sk = self.skipping();
                for i in self.targets(u16v(v[0])) {
                    self.layers[i].set_kind(&mut self.mem, screen, v[1], sk);
                }
            }
            0x32 => {
                if self.nest.is_empty() {
                    self.mem.promote_message_snapshot();
                }
            }
            0x33 => {
                self.flags.skip = false;
                self.flags.fast_skip = false;
                self.mem.restore_previous_snapshot();
                MessageWindow::clear_backlog(&mut self.mem);
                let s = self.current_script();
                self.load(s)?;
                let pc = self.return_at(self.depth());
                self.jump(pc);
                self.rebuild(h);
            }
            0x34 => {
                self.flags.skip_held = false;
                if self.flags.fast_skip {
                    self.flags.fast_skip = false;
                    self.rebuild(h);
                }
            }
            0x35 => {
                let v = u16v(self.value(raw[0]));
                self.mem.set_d(S::AUTO_REBUILD, v);
            }
            0x37 => {
                if self.nest.is_empty() {
                    self.mem.capture_message_snapshot();
                }
            }
            0x38 => {
                let v = self.vals(raw);
                let m = |o: usize| S::MESSAGE + o;
                self.mem.set_d(m(M::MENU_ENABLED), u16v(v[0]));
                self.mem.set_d(m(M::PANEL_BUTTONS), u16v(v[1]));
                self.mem.set_d(m(M::TAB_ENABLED), u16v(v[2]));
                self.mem.set_d(m(M::BACKLOG_ENABLED), u16v(v[3]));
                self.mem.set_d(S::PANEL_SAVE_LOAD, u16v(v[1]));
                self.mem.set_d(S::MESSAGE_SNAPSHOTS, u16v(v[4]));
            }
            0x39 => MessageWindow::clear_backlog(&mut self.mem),
            0x3C => {
                let v = self.vals(raw);
                self.play_music(h, u16v(v[0]) as u16, v[1] != 0, u16v(v[2]))?;
            }
            0x3D => {
                let v = self.vals(raw);
                self.stop_music(h, v[0] != 0, u16v(v[1]))?;
            }
            0x3E => {
                let v = self.vals(raw);
                self.load_sound(h, u16v(v[0]), u16v(v[1]))?;
            }
            0x3F => {
                let v = self.vals(raw);
                self.play_sound(h, u16v(v[0]), u16v(v[1]), v[2], v[3])?;
            }
            0x40 => {
                let v = self.vals(raw);
                self.stop_sound(h, u16v(v[0]), v[1] != 0)?;
            }
            0x41 => {
                let movie = u16v(self.value(raw[0])) as u16;
                self.play_movie(h, movie)?;
            }
            0x42 => {
                // 1.11 reads the voice number as a raw dword.
                let voice = raw[0] as u32;
                let (repeat, fade, pan) = (self.value(raw[1]), self.value(raw[2]), self.value(raw[3]));
                if voice != 0 && !self.skipping() && !self.flags.skip {
                    self.play_voice(h, voice, repeat, fade != 0, pan)?;
                }
            }
            0x43 => {
                let fade = self.value(raw[0]) != 0;
                if !self.skipping() {
                    h.submit_audio(self.client, AudioRequest::Stop { channel: VOICE_CHANNEL, fade_ms: if fade { 150.0 * FADE_STEP_MS } else { 0.0 } })?;
                }
            }
            0x44 => {
                self.mem.set_w(S::SOUND_CHANNELS + 2, 0);
                if !(self.skipping() || self.flags.skip) {
                    self.flags.wait_sound = true;
                    self.cont.push(Cont::WaitSoundEnd);
                    self.wait = Some(Wait::Audio { channel: SOUND_CHANNEL });
                }
            }
            0x45 => {
                if !(self.skipping() || self.flags.skip) {
                    self.flags.wait_voice = true;
                    self.cont.push(Cont::WaitVoiceEnd);
                    self.wait = Some(Wait::Audio { channel: VOICE_CHANNEL });
                }
            }
            0x46 | 0x47 | 0x4D => {
                let v = self.vals(raw);
                let id = u16v(v[0]) as usize;
                let mode = match op {
                    0x46 => 0,
                    0x47 => 1,
                    _ => 2,
                };
                let sk = self.skipping();
                if id != 0 && id < S::LAYER_COUNT {
                    self.layers[id].set_button(&mut self.mem, mode, v[2], v[1], v[3], sk);
                }
            }
            0x48 => {
                let id = u16v(self.value(raw[0])) as usize;
                let sk = self.skipping();
                if id != 0 && id < S::LAYER_COUNT {
                    self.layers[id].stop_button(&mut self.mem, true, sk);
                }
            }
            0x49 => {
                let v = self.vals(raw);
                self.button_wait(u16v(v[0]), v[1], v[2] != 0);
            }
            0x4A => {
                let v = u16v(self.value(raw[0]));
                self.mem.set_d(S::LAYER_ANIMATION_RESET, v);
            }
            0x4B => {
                let v = self.vals(raw);
                let id = u16v(v[0]) as usize;
                let sk = self.skipping();
                if id != 0 && id < S::LAYER_COUNT {
                    let mut cx = cx!(self, h);
                    self.layers[id].set_overlay(&mut self.mem, &mut cx, u16v(v[4]) as usize, u16v(v[1]) as u16, v[2], v[3], sk);
                }
            }
            0x51 => {
                let (b, voice, repeat, pan, name, text, wait) =
                    (self.value(raw[0]), raw[1] as u32, self.value(raw[2]), self.value(raw[3]), raw[4] as u32, raw[5] as u32, self.value(raw[6]));
                self.show_message(h, u16v(b) as usize, text, name, wait, voice, repeat, pan, true)?;
            }
            0x52 => {
                let (b, text, wait) = (self.value(raw[0]), raw[4] as u32, self.value(raw[5]));
                self.show_message(h, u16v(b) as usize, text, 0, wait, 0, 0, 0, false)?;
            }
            0x53 => {
                let b = u16v(self.value(raw[0])) as usize;
                let sk = self.skipping();
                self.message.clear(&mut self.mem, b & 3, sk);
                self.after_layers();
            }
            0x5A..=0x6C => self.box_setter(h, op, raw),
            0x79 => {
                let id = u16v(self.value(raw[0]));
                let name = self.string(raw[1])?;
                let mut dir = b".\\".to_vec();
                dir.extend(name);
                for i in self.targets(id) {
                    self.mem.set_str(S::LAYERS + i * S::LAYER_STRIDE + L::DIRECTORY, 23, &dir);
                }
            }
            0x84 => {
                let v = self.vals(raw);
                if (u16v(v[0]) as usize) < S::GAUGE_COUNT {
                    self.mem.set_w(S::GAUGES + S::GAUGE_STRIDE * u16v(v[0]) as usize, v[1]);
                }
            }
            0xC8 => self.local_call(raw[0] as u32, &raw[1..11])?,
            0xCA => {
                let v = self.vals(raw);
                let (from, to) = (u16v(v[0]) as usize, (u16v(v[1]) as usize).min(memory::VARIABLES - 1));
                if from <= to {
                    self.mem.vars[from..=to].fill(v[2] as i16);
                }
            }
            0xE6 => {
                let image = u16v(self.value(raw[0])) as u16;
                self.mark_seen(image);
                if image != 0 {
                    let sk = self.skipping();
                    let mut cx = cx!(self, h);
                    self.layers[0].set_kind(&mut self.mem, screen, 2, sk);
                    self.layers[0].load(&mut self.mem, &mut cx, image, 0, 0, 0, 0, sk);
                    if !self.flags.batch {
                        self.transition(h, 0, 15, 0)?;
                    }
                }
            }
            0xFF => {} // 1.11 reads 0xFF's operands and ignores them (nullsub_1).
            0x19 => {
                // sub_43D500: a plug-in scene by DLL name (crate::plugins).
                let name = self.string(raw[0])?;
                let name = encoding_rs::SHIFT_JIS.decode(&name).0.into_owned();
                let value = self.value(raw[1]) as i16;
                self.mem.set_var(0, value);
                match self.resolve_plugin(h, &name, "opcode 0x19")? {
                    crate::plugins::Resolved::Facade(f) => match f.scene {
                        Some(scene) => scene(self, h, value)?,
                        None => bail!("RScript plug-in {name:?} has no scene interface"),
                    },
                    // sub_42D680: the DLL does not load; the game goes on.
                    crate::plugins::Resolved::Absent => self.res.diag(format!("plug-in scene {name:?} is not shipped; the game state continues")),
                }
            }
            0x20 => {
                // sub_43CC70: text layer (source 2) from a string.
                let v = self.vals(&raw[..5]);
                let id = u16v(v[0]) as usize;
                let text = self.string(raw[5])?;
                if id != 0 && id < S::LAYER_COUNT {
                    let (sx, sy) = (i32::from(self.mem.w(S::SCALE_X)), i32::from(self.mem.w(S::SCALE_Y)));
                    let effect = self.effect_of(u16v(v[3]));
                    let sk = self.skipping();
                    let mut cx = cx!(self, h);
                    self.layers[id].load_text(&mut self.mem, &mut cx, &text, effect, v[1] * sx, v[2] * sy, v[4] != 0, sk);
                    if !self.flags.batch {
                        self.refresh();
                    }
                } else if id >= S::LAYER_COUNT {
                    bail!("RScript layer {id} is out of range (native message box)");
                }
            }
            0x23 => {
                // sub_43CAC0 → sub_414C60: the animation pattern of layers
                // that hold an image or source.
                let v = self.vals(raw);
                let sk = self.skipping();
                for i in self.targets(u16v(v[0])) {
                    self.layers[i].set_pattern(&mut self.mem, &mut self.res, u16v(v[1]), sk);
                }
                self.after_layers();
            }
            0x3A => {
                // Ends the thread while word_4ED28C is set (sub_440690 sets
                // it for a top-menu start; the 0x40F/0x415 paths clear it).
                if self.top_menu_start {
                    self.end_thread(h)?;
                }
            }
            0x3B => {
                // sub_43CB80 → sub_4109A0: image at the current origin.
                let v = self.vals(raw);
                let id = u16v(v[0]) as usize;
                if id != 0 && id < S::LAYER_COUNT {
                    let effect = self.effect_of(u16v(v[2]));
                    let sk = self.skipping();
                    let mut cx = cx!(self, h);
                    self.layers[id].load_at_origin(&mut self.mem, &mut cx, u16v(v[1]) as u16, effect, v[3], sk);
                    self.after_layers();
                } else if id >= S::LAYER_COUNT {
                    bail!("RScript layer {id} is out of range (native message box)");
                }
            }
            0x50 => {
                // sub_43F560 → sub_428B50: clear a box's source and text.
                let b = (u16v(self.value(raw[0])) & 3) as usize;
                let sk = self.skipping();
                self.message.clear(&mut self.mem, b, sk);
                self.dirty |= !sk;
            }
            0x54 => {
                // sub_43F6C0: set a box's text from a string at once
                // (sub_42A4D0) or clear its source (sub_428B10).
                let b = (u16v(self.value(raw[0])) & 3) as usize;
                let sk = self.skipping();
                if raw[1] != 0 {
                    let script = self.current_script() as u16;
                    self.mem.set_w(S::MESSAGE + M::CURRENT_BOX, b as i32);
                    let base = S::MESSAGE + M::SOURCES + b * M::SOURCE_STRIDE;
                    self.mem.set_w(base, i32::from(script));
                    self.mem.set_d(base + 4, raw[1]);
                    if !sk {
                        let text = self.string(raw[1])?;
                        let bx = &mut self.message.boxes[b];
                        bx.clear_text();
                        bx.append_text(&text);
                        bx.finish();
                        self.dirty = true;
                    }
                } else {
                    self.mem.set_d(S::MESSAGE + M::SOURCES + b * M::SOURCE_STRIDE + 4, 0);
                    if !sk {
                        self.message.boxes[b].clear_text();
                        self.dirty = true;
                    }
                }
            }
            0x6E => {
                // sub_43DA80: a layer's record position in script units.
                let v = self.vals(raw);
                let r = S::LAYERS + (u16v(v[0]) as usize).min(S::LAYER_COUNT - 1) * S::LAYER_STRIDE;
                let (sx, sy) = (i32::from(self.mem.w(S::SCALE_X)), i32::from(self.mem.w(S::SCALE_Y)));
                if sx == 0 || sy == 0 {
                    bail!("RScript 0x6E with a zero coordinate scale (native division fault)");
                }
                let (x, y) = (self.mem.d(r + L::X) / sx, self.mem.d(r + L::Y) / sy);
                self.set_var(u16v(v[1]), x);
                self.set_var(u16v(v[2]), y);
            }
            0x6F => {
                let v = self.vals(raw);
                let (a, b, c) = (i32::from(v[0] as i16), i32::from(v[1] as i16), i32::from(v[2] as i16));
                if c == 0 {
                    bail!("RScript 0x6F divides by zero (native division fault)");
                }
                self.set_var(0, b.wrapping_mul(a) / c);
            }
            0x70 => {
                let a = f64::from(self.value(raw[0]) as i16);
                self.set_var(0, (a.sqrt() * 1000.0) as i32);
            }
            0x71 => {
                let v = self.vals(raw);
                let (a, n) = (v[0] as i16, v[1] as i16);
                let mut r = a;
                for _ in 1..n.max(1) {
                    r = r.wrapping_mul(a);
                }
                self.set_var(0, i32::from(r));
            }
            0x72 => {
                // Seen-image flags (bitset key 0): set, or query into var 0.
                let v = self.vals(raw);
                let image = u16v(v[0]) as u32;
                if v[1] != 0 {
                    self.seen_images.set(0, image);
                } else {
                    let seen = self.seen_images.get(0, image);
                    self.set_var(0, i32::from(seen));
                }
            }
            0x73 => {
                let v = self.vals(raw);
                self.mem.set_w(S::WORDS_B + 2 * (u16v(v[0]) as usize).min(9), v[1]);
            }
            0x74 => {
                let v = self.vals(raw);
                self.mem.set_w(S::WORDS + 2 * (u16v(v[0]) as usize).min(9), v[1]);
            }
            0x75 => {
                let v = self.vals(raw);
                let w = self.mem.w(S::WORDS + 2 * (u16v(v[0]) as usize).min(9));
                self.set_var(u16v(v[1]), i32::from(w));
            }
            0x78 => {
                // sub_43CDB0 → sub_414C10: text-layer font size (+92).
                let v = self.vals(raw);
                for i in self.targets(u16v(v[0])) {
                    self.mem.set_w(S::LAYERS + i * S::LAYER_STRIDE + L::PRIORITY, v[1]);
                }
            }
            0x82 | 0x83 | 0x86 | 0x87 | 0x88 => {
                self.gauge_op(op, raw)?;
                let g = (u16v(self.value(raw[0])) as usize).min(S::GAUGE_COUNT - 1);
                if !self.skipping() {
                    self.load_gauge(h, g);
                }
            }
            0x8C => {
                let v = self.vals(raw);
                self.mem.set_cfg(C::SYSTEM_WORDS_B + 2 * (u16v(v[0]) as usize).min(9), v[1]);
            }
            0x8D => {
                let v = self.vals(raw);
                self.mem.set_cfg(C::SYSTEM_WORDS + 2 * (u16v(v[0]) as usize).min(9), v[1]);
            }
            0x8E => {
                let v = self.vals(raw);
                let w = self.mem.cfg(C::SYSTEM_WORDS + 2 * (u16v(v[0]) as usize).min(9));
                self.set_var(u16v(v[1]), i32::from(w));
            }
            0x96 | 0x97 => {
                let r = Self::reg_at(self.value(raw[0]))?;
                let mut s = if op == 0x97 { self.mem.str_at(r, S::STRING_STRIDE).to_vec() } else { Vec::new() };
                s.extend(self.string(raw[1])?);
                self.mem.set_str(r, S::STRING_STRIDE, &s);
            }
            0x98 => {
                // sub_44E0A0: a variable as full-width Shift-JIS digits.
                let v = self.vals(raw);
                let r = Self::reg_at(v[0])?;
                let n = i32::from(self.mem.var(u16v(v[1])));
                let mut out = Vec::new();
                for c in n.to_string().bytes() {
                    out.extend(if c == b'-' { [0x81, 0x7C] } else { [0x82, c + 31] });
                }
                self.mem.set_str(r, S::STRING_STRIDE, &out);
            }
            0x99 => {
                // sub_44E540: atoi of a string register (spaces/tabs, sign).
                let v = self.vals(raw);
                let r = Self::reg_at(v[1])?;
                let n = atoi(self.mem.str_at(r, S::STRING_STRIDE));
                self.set_var(u16v(v[0]), n);
            }
            0x9A | 0x9B => {
                let v = self.vals(raw);
                let (a, b) = (Self::reg_at(v[0])?, Self::reg_at(v[1])?);
                let mut s = if op == 0x9B { self.mem.str_at(a, S::STRING_STRIDE).to_vec() } else { Vec::new() };
                s.extend_from_slice(self.mem.str_at(b, S::STRING_STRIDE));
                self.mem.set_str(a, S::STRING_STRIDE, &s);
            }
            0x9D => {
                // sub_43F1A0: the flag byte of a string register.
                let v = self.vals(raw);
                let i = (u16v(v[0]) as usize).min(9);
                self.mem.scene[S::STRING_FLAGS + 4 * i] = v[1] as u8;
            }
            0x9E => {
                let v = self.vals(raw);
                let s = self.mem.str_at(Self::reg_at(v[0])?, S::STRING_STRIDE).to_vec();
                self.mem.set_cfg_str(C::SYSTEM_STRINGS + 201 * (u16v(v[1]) as usize).min(9), 201, &s);
            }
            0x9F => {
                let v = self.vals(raw);
                let s = self.mem.cfg_str(C::SYSTEM_STRINGS + 201 * (u16v(v[1]) as usize).min(9), 201).to_vec();
                self.mem.set_str(Self::reg_at(v[0])?, S::STRING_STRIDE, &s);
            }
            0xC9 => {
                // sub_43C630 → sub_413DF0: the mirror, zoom, rotation and
                // overlay words (+26..+32) of kind 0 image loads (sub_410C50).
                let v = self.vals(raw);
                for i in self.targets(u16v(v[0])) {
                    let r = S::LAYERS + i * S::LAYER_STRIDE;
                    for (k, x) in v[1..5].iter().enumerate() {
                        self.mem.set_w(r + 26 + 2 * k, *x);
                    }
                }
            }
            0xD2 => {
                // sub_43D8C0: begin a selection list.
                let keep = self.value(raw[1]) != 0;
                if !keep {
                    self.mem.scene[S::SELECTION..S::SELECTION + S::SELECTION_SIZE].fill(0);
                }
                let script = self.current_script();
                self.mem.set_w(S::SELECTION + 4, script);
                self.mem.set_d(S::SELECTION + 8, raw[0]);
            }
            0xD3 => {
                // sub_43D550: append (script, target, value, condition).
                let (value, cond, negate) = (self.value(raw[1]), self.value(raw[2]), self.value(raw[3]));
                let n = self.mem.uw(S::SELECTION) as usize;
                if n <= 99 {
                    let e = S::SELECTION + 12 + 12 * n;
                    let script = self.current_script();
                    self.mem.set_w(e, script);
                    self.mem.set_d(e + 4, raw[0]);
                    self.mem.set_w(e + 8, value);
                    self.mem.set_w(e + 10, if negate != 0 { cond | 0x8000 } else { cond });
                    self.mem.set_w(S::SELECTION, n as i32 + 1);
                }
            }
            0xD4 => {
                // sub_43D5E0: the "next page" label (script, string).
                let script = self.current_script();
                self.mem.set_w(S::SELECTION + 0x4BC, script);
                self.mem.set_d(S::SELECTION + 0x4C0, raw[0]);
            }
            0x30 => {
                // sub_43B990 → sub_429000: the message face image (+468).
                let v = self.vals(raw);
                let image = u16v(v[1]) as u16;
                let effect = if self.flags.skip || !self.effects_enabled() { 0 } else { u16v(v[2]) };
                self.mem.set_w(S::MESSAGE + 468, i32::from(image));
                if !self.skipping() {
                    let mut cx = cx!(self, h);
                    self.message.set_face(&mut cx, image, effect);
                    self.after_layers();
                }
            }
            0x9C => {
                // sub_43F1C0: window message 0x432 opens CEditMenu, then
                // the thread waits until the dialog closes.
                let n = u16v(self.value(raw[0])) as u16;
                if self.flags.batch {
                    self.flags.batch = false;
                }
                let dialog = {
                    let mut cx = cx!(self, h);
                    crate::screens::EditDialog::open(&mut cx, &self.mem, n)
                };
                let Some(mut dialog) = dialog else {
                    bail!("RScript text entry screen texedt{n:02}.lwg is missing");
                };
                let rect = dialog.fields.first().map(|f| [dialog.origin.x + f.rect[0], dialog.origin.y + f.rect[1], f.rect[2].max(1), f.rect[3].max(1)]);
                let spec = crysirror_core::service::TextInputSessionSpec { capture: true, ime_allowed: true, cursor_area: rect, ..Default::default() };
                dialog.session = Some(h.open_text_input_session(self.client, spec)?);
                self.edit = Some(dialog);
                self.dirty = true;
                self.wait = Some(Wait::Edit);
            }
            0x11 => {
                // sub_43E860: fast skip and auto stop, then sub_430FB0(1, 1)
                // opens the save screen and the thread waits for it.
                if self.flags.fast_skip {
                    self.flags.fast_skip = false;
                    self.rebuild(h);
                }
                self.stop_auto();
                if self.flags.batch {
                    self.flags.batch = false;
                }
                if !self.open_save_screen(h, true)? {
                    bail!("RScript save screen savescrn.lwg is missing");
                }
                self.wait = Some(Wait::Screen);
            }
            0xD5 => {
                let v = self.vals(raw);
                self.dyn_select_open(h, v[0], u16v(v[1]), v[2])?;
            }
            0xDC => {
                let v = self.vals(raw);
                self.map_wait(u16v(v[0]), v[1], v[2] != 0);
            }
            0xDD | 0xDE => {
                let (n, sub) = if op == 0xDD {
                    let v = self.vals(raw);
                    (u16v(v[0]) as u16, u16v(v[1]) as u16)
                } else {
                    (0, 0)
                };
                let sk = self.skipping();
                if !sk {
                    let mut cx = cx!(self, h);
                    self.map.open(&mut cx, n, sub);
                }
                self.map.number = n;
                self.map.sub = sub;
                self.mem.set_w(S::MAP, i32::from(n));
                self.mem.set_w(S::MAP + 2, i32::from(sub));
                if !sk && !self.flags.batch {
                    self.transition(h, 1, 15, 0)?;
                }
            }
            0xDF => {
                // sub_43E5A0: hide flag = (enable == 0), id 0 for all buttons.
                let v = self.vals(raw);
                let (id, hide) = (u16v(v[0]) as usize, v[1] == 0);
                if id == 0 {
                    self.map.hidden[1..].fill(hide);
                } else if id <= 20 {
                    self.map.hidden[id] = hide;
                }
                // sub_417CD0: one byte per id after the numbers.
                let ids = if id == 0 { 1..21 } else { id..(id + 1).min(21) };
                for i in ids {
                    self.mem.scene[S::MAP + 4 + i] = u8::from(hide);
                }
                let sk = self.skipping();
                if !sk {
                    self.map.apply();
                    if !self.flags.batch {
                        self.transition(h, 1, 15, 0)?;
                    }
                }
            }
            0xE1 => {
                let v = self.vals(raw);
                self.pointer_wait(u16v(v[0]), u16v(v[1]), u16v(v[2]), v[3], v[4] != 0);
            }
            0xE7 => {
                // sub_43CEB0 → sub_415110: the layer sprite's blocking
                // animation call while input waits (dword_4ED220).
                let id = u16v(self.value(raw[0])) as usize;
                if id >= S::LAYER_COUNT {
                    bail!("RScript layer {id} is out of range (native message box)");
                }
                if !(self.flags.skip || self.flags.fast_skip) && id != 0 && self.layers[id].main.animating {
                    self.flags.wait_animation = true;
                    self.wait = Some(Wait::Refresh);
                }
            }
            _ => {
                // A native handler not mapped yet: fail visibly at its PC.
                bail!("RScript opcode {op:#04x} has no CrySirror implementation (operands {raw:?})");
            }
        }
        Ok(())
    }

    fn box_setter(&mut self, h: &mut dyn HostServices, op: u16, raw: &[i32]) {
        let v = self.vals(raw);
        let set_w = |vm: &mut Self, b: usize, f: usize, x: i32| {
            let r = vm.message.boxes[b].record;
            vm.mem.set_w(r + f, x);
        };
        let set_d = |vm: &mut Self, b: usize, f: usize, x: i32| {
            let r = vm.message.boxes[b].record;
            vm.mem.set_d(r + f, x);
        };
        // A zero selector is box 0, anything else boxes 1..3 ("all" setters);
        // single-box setters address box `n & 3`.
        let all = |sel: i32| -> Vec<usize> { if sel != 0 { vec![1, 2, 3] } else { vec![0] } };
        let single = |sel: i32| vec![(u16v(sel) & 3) as usize];
        let boxes = match op {
            0x5A | 0x5C | 0x67 => single(v[0]),
            0x5D | 0x5E | 0x65 | 0x66 | 0x69 => vec![],
            _ => all(v[0]),
        };
        for &b in &boxes {
            match op {
                0x5A => {
                    set_d(self, b, T::X, v[1]);
                    set_d(self, b, T::Y, v[2]);
                }
                0x5B => {
                    set_d(self, b, T::TEXT_X, v[1]);
                    set_d(self, b, T::TEXT_Y, v[2]);
                    set_d(self, b, T::TEXT_WIDTH, u16v(v[3]));
                    set_d(self, b, T::TEXT_HEIGHT, u16v(v[4]));
                }
                0x5C => set_w(self, b, T::FRAME, u16v(v[1])),
                0x5F => set_d(self, b, T::COLOR, TEXT_COLORS.get(v[1] as usize).copied().unwrap_or(0) as i32),
                0x60 => set_w(self, b, T::SIZE, v[1]),
                0x61 => set_w(self, b, T::FACE, v[1]),
                0x62 => set_w(self, b, T::ALIGN, v[1]),
                0x63 => {
                    set_w(self, b, T::INDENT, u16v(v[1]));
                    set_w(self, b, T::FIRST_INDENT, v[2]);
                }
                0x64 => {
                    set_d(self, b, T::WAIT_X, v[1]);
                    set_d(self, b, T::WAIT_Y, v[2]);
                }
                0x67 => set_w(self, b, T::WAIT_ICON, u16v(v[1])),
                0x68 => set_d(self, b, T::GLOW, ((v[1] & 0xFF) << 16) | ((v[2] & 0xFF) << 8) | (v[3] & 0xFF)),
                0x6A => {
                    set_d(self, b, T::NAME_X, v[1]);
                    set_d(self, b, T::NAME_Y, v[2]);
                    set_d(self, b, T::NAME_WIDTH, u16v(v[3]));
                    set_d(self, b, T::NAME_HEIGHT, u16v(v[4]));
                }
                0x6B => {
                    set_w(self, b, T::LINE_SPACING, v[1]);
                    set_w(self, b, T::CHAR_SPACING, v[2]);
                }
                0x6C => {
                    set_w(self, b, T::RUBY_FACE, v[1]);
                    set_w(self, b, T::RUBY_SIZE, v[2]);
                    set_w(self, b, T::RUBY_RAISE, u16v(v[3]));
                }
                _ => {}
            }
        }
        let m = |o: usize| S::MESSAGE + o;
        match op {
            0x5D => {
                self.mem.set_d(m(M::PANEL_X), v[0]);
                self.mem.set_d(m(M::PANEL_Y), v[1]);
            }
            0x5E => self.mem.set_d(m(M::PANEL_ENABLED), u16v(v[0])),
            0x65 => {
                self.mem.set_d(m(M::FACE_X), v[0]);
                self.mem.set_d(m(M::FACE_Y), v[1]);
            }
            0x66 => self.mem.set_w(S::MESSAGE_MODE, v[0]),
            0x69 => self.mem.set_w(m(M::FACE_ORDER), if v[1] != 0 { 0 } else { 10 }),
            0x6E..=0x75 => {}
            _ => {}
        }
        if !self.skipping() {
            let mut cx = cx!(self, h);
            for &b in &boxes {
                self.message.boxes[b].apply(&self.mem, &mut cx);
            }
            self.message.update_panel(&self.mem);
        }
    }


    /// Layer targets: 0 every layer from 1, 1..100 one layer, larger a group.
    fn targets(&mut self, id: i32) -> Vec<usize> {
        if id == 0 {
            return (1..S::LAYER_COUNT).collect();
        }
        if id <= 100 {
            if (id as usize) < S::LAYER_COUNT {
                return vec![id as usize];
            }
            self.res.diag(format!("layer {id} is out of range"));
            return vec![];
        }
        let group = id % 100;
        (1..S::LAYER_COUNT).filter(|&i| i32::from(self.mem.w(S::LAYER_GROUPS + 2 * i)) == group).collect()
    }
    /// Moves; effects ≥ 7 drop when skipping.
    fn move_layer(&mut self, i: usize, kind: i32, x: i32, y: i32, speed: i32, relative: bool) {
        let mut kind = u16v(kind);
        if self.flags.skip || !self.effects_enabled() {
            if kind % 100 >= 7 {
                return;
            }
            kind = 0;
        }
        let r = S::LAYERS + i * S::LAYER_STRIDE;
        let (x, y) = if relative { (x + self.mem.d(r + L::X), y + self.mem.d(r + L::Y)) } else { (x, y) };
        let (screen, sk) = (self.screen(), self.skipping());
        self.layers[i].move_to(&mut self.mem, screen, kind, x, y, u16v(speed), sk);
    }
    fn mark_seen(&mut self, image: u16) {
        if image < 1000 {
            self.mem.vars[9000 + usize::from(image)] = 1;
        }
    }
    pub(crate) fn snapshot(&mut self) {
        if self.mem.d(S::MESSAGE_SNAPSHOTS) != 0 && self.nest.is_empty() {
            self.mem.capture_message_snapshot();
        }
    }

    // ---------------------------------------------------------------- waits

    /// Present layer changes, waiting for their effects.
    pub(crate) fn refresh(&mut self) {
        if self.flags.fast_skip {
            return;
        }
        self.dirty = true;
        if self.flags.skip || !self.effects_enabled() {
            return;
        }
        self.flags.wait_animation = true;
        self.wait = Some(Wait::Refresh);
    }
    fn after_layers(&mut self) {
        if !self.flags.batch {
            self.refresh();
        }
    }
    fn flush_batch(&mut self) {
        if self.flags.batch {
            self.flags.batch = false;
            self.refresh();
        }
    }
    /// A click, auto-mode wait or skip.
    pub(crate) fn wait_click(&mut self) {
        if self.flags.skip || self.flags.fast_skip {
            return;
        }
        self.flags.wait_input = true;
        if self.flags.auto {
            // The configured wait, then the voice.
            let wait = u64::from(self.mem.cfg(C::AUTO_WAIT) as u16);
            self.cont.push(Cont::WaitInputEnd);
            self.wait = Some(Wait::Auto { until: self.now + wait });
            return;
        }
        self.flags.interrupt = false;
        self.message.set_waiting(&self.mem, true);
        self.message.set_input(true);
        self.set_button_input(true, true);
        self.flags.wait_click = true;
        self.cont.push(Cont::ClickDone);
        self.wait = Some(Wait::Click);
    }
    pub(crate) fn set_button_input(&mut self, enabled: bool, only_interrupts: bool) {
        for l in &mut self.layers {
            l.set_button_input(&self.mem, enabled, only_interrupts);
        }
    }
    /// Waits for a layer button.
    pub(crate) fn button_wait(&mut self, timer_var: i32, hide_timer: i32, cancel: bool) {
        self.flags.button_cancel = cancel;
        if self.flags.batch {
            self.flags.batch = false;
            self.dirty = true;
        }
        self.flags.fast_skip = false;
        self.stop_auto();
        self.snapshot();
        self.flags.interrupt = false;
        self.flags.button_wait = 1;
        self.set_button_input(true, false);
        if timer_var != 0 {
            if hide_timer == 0 {
                self.start_timer_gauge(i32::from(self.mem.var(timer_var)));
            }
            self.flags.button_timer = true;
            self.button_timer = (i32::from(self.mem.var(timer_var)), self.now);
        } else {
            self.message.set_input(true);
        }
        self.flags.wait_input = true;
        self.cont.push(Cont::ButtonsDone { timer_var, hide_timer, cancel });
        self.wait = Some(Wait::Buttons);
    }
    pub(crate) fn stop_auto(&mut self) {
        if self.flags.auto {
            self.flags.auto = false;
            self.message.auto_hidden = false;
            self.message.update_panel(&self.mem);
            self.dirty = true;
        }
    }

    /// Ends the running thread (0x08): a nested call returns, the main
    /// thread restarts the opening (the native thread posts 0x40F).
    fn end_thread(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if let Some(n) = self.nest.pop() {
            let backlog = MessageWindow::backlog_range();
            let saved_backlog = self.mem.scene[backlog.clone()].to_vec();
            self.mem.scene.copy_from_slice(&n.scene);
            self.mem.scene[backlog].copy_from_slice(&saved_backlog);
            self.thread = n.thread;
            self.cont = n.cont;
            self.flags.skip = false;
            self.flags.fast_skip = false;
            self.stop_auto();
            self.rebuild(h);
            self.wait = n.wait;
            return Ok(());
        }
        // Message 0x40F: back to the state before the game (the title,
        // or the scene view after a replay); the movies do not play again.
        self.stop_all_audio(h)?;
        if std::mem::take(&mut self.replaying) {
            self.top_menu_start = false;
            return self.enter_state(h, crate::extras::State::SceneView);
        }
        self.return_to_title(h)
    }
    /// Runs `script` as a nested thread; the scene is restored
    /// afterwards. `resume` is the wait the outer thread returns to.
    pub(crate) fn system_call_with(&mut self, h: &mut dyn HostServices, script: i32, resume: Option<Wait>) -> Result<()> {
        if self.flags.batch {
            self.flags.batch = false;
        }
        self.flags.skip = false;
        self.flags.fast_skip = false;
        self.stop_auto();
        let saved = Nest { scene: self.mem.scene.clone(), thread: self.thread.clone(), cont: std::mem::take(&mut self.cont), wait: resume };
        self.nest.push(saved);
        let sk = self.skipping();
        for l in self.layers.iter_mut().skip(1) {
            l.stop_button(&mut self.mem, true, false);
        }
        let mut cx = cx!(self, h);
        self.message.clear(&mut self.mem, 0, sk);
        self.message.show_box(&mut self.mem, &mut cx, 0, false, false);
        self.mem.set_w(S::CALL_DEPTH, 0);
        self.thread.script = -1;
        self.thread.program = None;
        self.load(script)?;
        self.jump(0);
        Ok(())
    }
    fn system_call(&mut self, h: &mut dyn HostServices, script: i32) -> Result<()> {
        self.system_call_with(h, script, None)
    }

    // ------------------------------------------------------------- messages

    #[allow(clippy::too_many_arguments)]
    fn show_message(&mut self, h: &mut dyn HostServices, b: usize, text: u32, name: u32, wait: i32, voice: u32, repeat: i32, pan: i32, new_page: bool) -> Result<()> {
        if b > 3 {
            bail!("message box {b} is out of range");
        }
        self.flags.batch = false;
        let script = self.current_script();
        let unread = !self.read_text.get(script as u16, text);
        let skip_unread = self.mem.cfg(C::SKIP_UNREAD) != 0;
        if self.flags.fast_skip && !skip_unread && unread {
            self.flags.fast_skip = false;
            self.rebuild(h);
        }
        if self.flags.skip && !skip_unread && unread {
            self.flags.skip_held = false;
            self.flags.skip = false;
        }
        self.read_text.set(script as u16, text);
        self.snapshot();
        let mode = i32::from(self.mem.w(S::MESSAGE_MODE));
        let sk = self.skipping();
        if new_page {
            if mode == 1 {
                self.message.clear(&mut self.mem, b, sk);
            }
            self.mem.set_d(S::MESSAGE + M::VOICE, voice as i32);
            self.mem.set_w(S::MESSAGE + M::VOICE_PAN, pan);
            self.message.set_voice_available(voice != 0);
        }
        // Record the source and show the box.
        self.mem.set_w(S::MESSAGE + M::CURRENT_BOX, b as i32);
        MessageWindow::set_source(&mut self.mem, b, (script as u16, text, name));
        {
            let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
            self.message.show_box(&mut self.mem, &mut cx, b, true, sk);
        }
        if b == 0 {
            MessageWindow::push_backlog(&mut self.mem, new_page);
        }
        if sk {
            if mode == 0 {
                self.message.clear(&mut self.mem, b, true);
            }
            return Ok(());
        }
        let body = self.string(text as i32)?;
        if new_page {
            self.message.page_text = body.clone();
            let n = if name != 0 { Some(self.string(name as i32)?) } else { None };
            self.message.boxes[b].set_name(&mut self.mem, n.as_deref());
        } else {
            self.message.page_text.extend_from_slice(&body);
        }
        self.message.boxes[b].append_text(&body);
        if new_page && voice != 0 && !self.flags.skip {
            self.play_voice(h, voice, repeat, false, pan)?;
        }
        self.dirty = true;
        if self.flags.skip {
            self.message.finish();
            self.after_reveal(b, wait != 0, mode, new_page, voice, repeat);
        } else {
            self.message.reveal();
            self.flags.wait_animation = true;
            self.cont.push(Cont::MessageRevealed { b, wait: wait != 0, mode, new_page, voice, repeat });
            self.wait = Some(Wait::Refresh);
        }
        Ok(())
    }
    pub(crate) fn after_reveal(&mut self, b: usize, wait: bool, mode: i32, new_page: bool, voice: u32, repeat: i32) {
        if wait {
            self.cont.push(Cont::MessageClicked { b, mode, new_page, voice, repeat });
            self.wait_click();
            if self.wait.is_none() {
                // Skipping: the click resolves at once.
                self.cont.pop();
                self.after_click(b, mode, new_page, voice, repeat, true);
            }
        } else {
            self.message.finish();
            self.dirty = true;
        }
    }
    pub(crate) fn after_click(&mut self, b: usize, mode: i32, new_page: bool, voice: u32, repeat: i32, waited: bool) {
        if mode == 0 {
            let sk = self.skipping();
            self.message.clear(&mut self.mem, b, sk);
        }
        self.message.finish();
        self.dirty = true;
        if new_page && voice != 0 && repeat == 0 && waited && self.mem.cfg(C::VOICE_CONTINUES) == 0 {
            self.voice_stop = true;
        }
    }

    // --------------------------------------------------------------- choices

    fn choose(&mut self, h: &mut dyn HostServices, raw: &[i32]) -> Result<()> {
        let header = raw[0];
        let question = self.string(raw[1])?;
        let mut targets = [0u32; 5];
        for (i, t) in raw[2..7].iter().enumerate() {
            targets[i] = *t as u32;
        }
        let count = ((header & 0xFF) as usize).min(5);
        let answers: Vec<Vec<u8>> = raw[7..7 + count].iter().map(|&i| self.string(i)).collect::<Result<_>>()?;
        let effect_mode = self.value(raw[12]);
        let arrangement = self.value(raw[14]);
        self.flags.batch = false;
        if self.flags.fast_skip {
            self.flags.fast_skip = false;
            self.rebuild(h);
        }
        self.stop_auto();
        self.snapshot();
        self.message.page_text = question.clone();
        let mut choice = {
            let mut cx = cx!(self, h);
            Choice::open(&mut cx, &question, &answers, arrangement)
        };
        let instant = !self.effects_enabled() || self.flags.skip || effect_mode != 0;
        choice.appear(instant);
        self.choice = Some(choice);
        self.cont.push(Cont::ChoiceShown { targets, effect_mode });
        self.present_choice(h, effect_mode)?;
        Ok(())
    }
    pub(crate) fn present_choice(&mut self, h: &mut dyn HostServices, effect_mode: i32) -> Result<()> {
        match effect_mode {
            2 => self.transition(h, 1, 15, 20)?,
            3 => self.transition(h, 2, 15, 20)?,
            _ => self.refresh(),
        }
        Ok(())
    }

    // ----------------------------------------------------------- transitions

    /// Screen transitions after layer changes.
    pub fn transition(&mut self, h: &mut dyn HostServices, kind: i32, steps: i32, ms: i32) -> Result<()> {
        if self.flags.fast_skip {
            return Ok(());
        }
        self.dirty = true;
        let count = if steps == 0 { 10 } else { steps } as u64;
        let ms = ms.max(0) as u64;
        if self.flags.skip || !self.effects_enabled() || kind == 0 {
            return Ok(());
        }
        let step_ms = ms + 1;
        if kind < 100 {
            match kind {
                1 => self.begin_transition(serde_json::json!({"kind":"crossfade"}), (count + 1) * step_ms),
                2 => self.begin_transition(serde_json::json!({"kind":"ordered_dither","cell":[1,1],"steps":16}), count.saturating_sub(1).max(1) * step_ms),
                3 | 4 => {
                    // Fade to black or white, then in.
                    let white = kind == 4;
                    let d = (count + 1) * step_ms;
                    self.dip = Some(if white { 0xFFFFFF } else { 0 });
                    self.begin_transition(serde_json::json!({"kind":"crossfade"}), d);
                    self.cont.push(Cont::DipBack { duration: d });
                }
                _ => {
                    let name = join(self.apini.dir(Dir::System), &format!("ef{kind:02}"));
                    let duration = (2 * count).saturating_sub(1).max(1) * step_ms;
                    match self.res.file(&format!("{name}.msk")).cloned() {
                        Some(asset) => self.begin_mask(h, asset, duration)?,
                        None => self.res.diag(format!("missing transition mask {name}")),
                    }
                }
            }
            return Ok(());
        }
        // sub_43B390: zoom (1xx, sub_453A60) and shrink (2xx, sub_4537E0)
        // between the numpad anchors of the tens and units digits.
        if kind / 100 > 2 {
            return Ok(());
        }
        let screen = self.screen();
        let anchor = |d: i32| crate::zoom::anchor(d, screen);
        let job = h.begin_frame_capture(self.client)?;
        let k = if kind / 100 == 2 { crate::zoom::Kind::Shrink } else { crate::zoom::Kind::Zoom };
        self.zoom = Some(crate::zoom::Zoom::new(k, anchor(kind % 100 / 10), anchor(kind % 10), job));
        self.wait = Some(Wait::Zoom);
        Ok(())
    }
    pub(crate) fn begin_transition(&mut self, effect: serde_json::Value, duration: u64) {
        let duration = duration.clamp(1, 600_000);
        self.transition = Some(serde_json::json!({"effect":effect,"duration_ms":duration}));
        self.wait = Some(Wait::Until { until: self.now + duration });
    }
    /// `.msk` masks are bitmaps whose blue byte is the native transparency:
    /// dark pixels change first. The shared mask reveals bright
    /// pixels first, so the mask is inverted once by the shared image worker.
    fn begin_mask(&mut self, h: &mut dyn HostServices, asset: String, duration: u64) -> Result<()> {
        if let Some(prepared) = self.masks.get(&asset).cloned() {
            self.begin_transition(serde_json::json!({"kind":"mask","asset":prepared,"vague":31}), duration);
            return Ok(());
        }
        let size = self.res.size(h, self.client, &asset).context("transition mask size")?;
        let effects = serde_json::to_vec(&[crysirror_image::effects::ImageEffect::Invert])?;
        let job = h.image_metadata(self.client)?.begin_process(
            crysirror_core::image_processing::ImageInput::Asset { path: asset.clone(), size, fingerprint: None },
            effects.into(),
        )?;
        self.mask_job = Some((asset.clone(), job));
        self.wait = Some(Wait::Mask { name: asset, duration });
        Ok(())
    }

    // ---------------------------------------------------------------- audio

    pub(crate) fn gain(&self, volume_at: usize) -> f32 {
        let v = i32::from(self.mem.cfg(volume_at));
        let hundredths = 3000 * (v - 255) / 255;
        10f32.powf(hundredths.min(0) as f32 / 2000.0)
    }
    fn audio_asset(&mut self, native: &str) -> Option<String> {
        let a = self.res.file(native).cloned();
        if a.is_none() {
            self.res.diag(format!("missing sound {native}"));
        }
        a
    }
    fn play(&mut self, h: &mut dyn HostServices, channel: u32, asset: String, loops: i32, fade_ms: f32, gain: f32, pan: f32) -> Result<()> {
        let parameters = AudioParameters {
            looping: loops != 0,
            loop_passes: (loops > 0).then(|| loops as u32 + 1),
            fade_ms,
            gain,
            pan,
            ..Default::default()
        };
        let source = AudioAsset { path: VfsPath::new(Mount::Asset, &asset)?, frame_limit: None };
        let ticket = h.begin_audio(self.client, AudioRequest::PlayAsset { channel, source, intro: None, parameters })?;
        self.audio_pending.push(ticket);
        Ok(())
    }
    /// `Track%02d`, fading the previous track out.
    pub(crate) fn play_music(&mut self, h: &mut dyn HostServices, track: u16, fade: bool, steps: i32) -> Result<()> {
        self.mem.set_w(S::MUSIC, i32::from(track));
        if self.flags.fast_skip || self.mem.cfg(C::MUSIC_ENABLED) == 0 || track == 0 || self.music == Some(track) {
            return Ok(());
        }
        let fade_ms = if fade && !self.flags.skip { (if steps == 0 { 150 } else { steps }) as f32 * FADE_STEP_MS } else { 0.0 };
        if self.music.is_some() {
            h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms })?;
            self.music_stream ^= 1;
        }
        self.music = Some(track);
        let native = join(self.apini.dir(Dir::Music), &format!("Track{track:02}.wav"));
        if let Some(asset) = self.audio_asset(&native) {
            let gain = self.gain(C::MUSIC_VOLUME);
            self.play(h, MUSIC_CHANNEL + self.music_stream, asset, -1, fade_ms, gain, 0.5)?;
        }
        Ok(())
    }
    pub(crate) fn stop_music(&mut self, h: &mut dyn HostServices, fade: bool, steps: i32) -> Result<()> {
        self.mem.set_w(S::MUSIC, 0);
        if self.flags.fast_skip || self.music.is_none() {
            return Ok(());
        }
        let fade_ms = if fade && !self.flags.skip { (if steps == 0 { 150 } else { steps }) as f32 * FADE_STEP_MS } else { 0.0 };
        h.submit_audio(self.client, AudioRequest::Stop { channel: MUSIC_CHANNEL + self.music_stream, fade_ms })?;
        self.music_stream ^= 1;
        self.music = None;
        Ok(())
    }
    fn sound_base(channel: i32) -> usize {
        S::SOUND_CHANNELS + (if channel > 2 { 0 } else { channel }) as usize * S::SOUND_STRIDE
    }
    pub(crate) fn load_sound(&mut self, h: &mut dyn HostServices, channel: i32, sound: i32) -> Result<()> {
        let index = if channel > 2 { 0 } else { channel };
        self.mem.set_w(Self::sound_base(index), sound);
        if sound == 0 || self.flags.fast_skip {
            return Ok(());
        }
        self.stop_sound(h, index, false)?;
        self.loaded_sound[index as usize] = Some(sound as u16);
        Ok(())
    }
    pub(crate) fn play_sound(&mut self, h: &mut dyn HostServices, channel: i32, repeat: i32, fade: i32, pan: i32) -> Result<()> {
        let index = if channel > 2 { 0 } else { channel };
        let base = Self::sound_base(index);
        if repeat == 999 {
            self.mem.set_w(base + 2, 1);
        }
        self.mem.set_w(base + 4, pan);
        if self.flags.fast_skip || self.mem.cfg(C::SOUND_ENABLED) == 0 {
            return Ok(());
        }
        let Some(sound) = self.loaded_sound[index as usize] else { return Ok(()) };
        let native = join(self.apini.dir(Dir::Sounds), &format!("{sound:04}.wav"));
        if let Some(asset) = self.audio_asset(&native) {
            let loops = if repeat == 999 { -1 } else { (repeat - 1).max(0) };
            let fade_ms = if fade != 0 && !self.flags.skip { 150.0 * FADE_STEP_MS } else { 0.0 };
            let gain = self.gain(C::SOUND_VOLUME);
            self.play(h, SOUND_CHANNEL + index as u32, asset, loops, fade_ms, gain, pan_of(pan))?;
        }
        Ok(())
    }
    pub(crate) fn stop_sound(&mut self, h: &mut dyn HostServices, channel: i32, fade: bool) -> Result<()> {
        let index = if channel > 2 { 0 } else { channel };
        self.mem.set_w(Self::sound_base(index) + 2, 0);
        if self.flags.fast_skip {
            return Ok(());
        }
        let fade_ms = if fade && !self.flags.skip { 150.0 * FADE_STEP_MS } else { 0.0 };
        h.submit_audio(self.client, AudioRequest::Stop { channel: SOUND_CHANNEL + index as u32, fade_ms })?;
        Ok(())
    }
    pub(crate) fn play_voice(&mut self, h: &mut dyn HostServices, voice: u32, repeat: i32, fade: bool, pan: i32) -> Result<()> {
        if self.mem.cfg(C::VOICE_ENABLED) == 0 {
            return Ok(());
        }
        let bank = (voice / 10_000) as usize;
        if bank < 10 && self.mem.cfg(C::VOICE_BANKS + 2 * bank) == 0 {
            return Ok(());
        }
        let dir = self.apini.dir(Dir::Voices).to_owned();
        let native = if bank != 0 { format!("{dir}\\{bank}\\{:04}.wav", voice % 10_000) } else { format!("{dir}\\{voice:04}.wav") };
        if let Some(asset) = self.audio_asset(&native) {
            let loops = if repeat == 999 { -1 } else { (repeat - 1).max(0) };
            let gain = self.gain(C::VOICE_VOLUME);
            self.play(h, VOICE_CHANNEL, asset, loops, if fade { 150.0 * FADE_STEP_MS } else { 0.0 }, gain, pan_of(pan))?;
        }
        Ok(())
    }
    /// `Track%02d` without fades (configuration callbacks, sub_44F1D0).
    pub(crate) fn play_music_track(&mut self, h: &mut dyn HostServices, track: u16) -> Result<()> {
        self.music = None;
        self.play_music(h, track, false, 0)
    }
    /// `<sounds>\NNNN.wav` on a sound channel (sub_4522D0/sub_4522F0).
    pub(crate) fn play_sound_file(&mut self, h: &mut dyn HostServices, channel: u32, sound: u16) -> Result<()> {
        let native = join(self.apini.dir(Dir::Sounds), &format!("{sound:04}.wav"));
        if let Some(asset) = self.audio_asset(&native) {
            let gain = self.gain(C::SOUND_VOLUME);
            self.play(h, SOUND_CHANNEL + channel, asset, 0, 0.0, gain, 0.5)?;
        }
        Ok(())
    }
    /// `<voices>[\bank]\NNNN.wav` regardless of the bank switch.
    pub(crate) fn play_voice_file(&mut self, h: &mut dyn HostServices, bank: usize, file: u32) -> Result<()> {
        let dir = self.apini.dir(Dir::Voices).to_owned();
        let native = if bank != 0 { format!("{dir}\\{bank}\\{file:04}.wav") } else { format!("{dir}\\{file:04}.wav") };
        if let Some(asset) = self.audio_asset(&native) {
            let gain = self.gain(C::VOICE_VOLUME);
            self.play(h, VOICE_CHANNEL, asset, 0, 0.0, gain, 0.5)?;
        }
        Ok(())
    }
    /// The frame window mutes audio while inactive unless the background
    /// switch is on (sub_419E70 sets the window's flag).
    pub(crate) fn apply_focus_audio(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let inactive = h.window_focused(self.client)? == Some(false);
        let mute = inactive && self.mem.cfg(C::BACKGROUND_AUDIO) == 0;
        if mute == self.inactive_muted {
            return Ok(());
        }
        self.inactive_muted = mute;
        for (at, channels) in [
            (C::MUSIC_VOLUME, &[MUSIC_CHANNEL, MUSIC_CHANNEL + 1][..]),
            (C::SOUND_VOLUME, &[SOUND_CHANNEL, SOUND_CHANNEL + 1, SOUND_CHANNEL + 2, SYSTEM_CHANNEL][..]),
            (C::VOICE_VOLUME, &[VOICE_CHANNEL][..]),
        ] {
            let gain = if mute { 0.0 } else { self.gain(at) };
            for &channel in channels {
                h.submit_audio(self.client, AudioRequest::SetGain { channel, gain, fade_ms: 0.0 })?;
            }
        }
        Ok(())
    }
    /// System decision sound.
    pub fn system_sound(&mut self, h: &mut dyn HostServices, sound: u16) -> Result<()> {
        if self.mem.cfg(C::SOUND_ENABLED) == 0 {
            return Ok(());
        }
        let native = join(self.apini.dir(Dir::Sounds), &format!("{sound:04}.wav"));
        if let Some(asset) = self.audio_asset(&native) {
            let gain = self.gain(C::SOUND_VOLUME);
            self.play(h, SYSTEM_CHANNEL, asset, 0, 0.0, gain, 0.5)?;
        }
        Ok(())
    }
    fn play_movie(&mut self, h: &mut dyn HostServices, movie: u16) -> Result<()> {
        let native = join(self.apini.dir(Dir::Movies), &format!("{movie:04}.mpg"));
        let Some(asset) = self.res.file(&native).cloned() else {
            self.res.diag(format!("missing movie {native}"));
            return Ok(());
        };
        let path = h.media_request(self.client, MediaRequest::Play { id: MOVIE_ID, asset, looping: false, audio: true, gain: 1.0 })?;
        self.movie = Some(path.relative().to_owned());
        self.dirty = true;
        self.wait = Some(Wait::Movie);
        Ok(())
    }

    // ------------------------------------------------------- effect screens

    pub fn apply_effect_screen(&mut self, h: &mut dyn HostServices) {
        let image = self.mem.uw(S::EFFECT_IMAGE);
        if image == 0 {
            self.effect.visible = false;
        } else {
            let native = join(self.apini.dir(Dir::System), &format!("es{image:03}"));
            let content = self.res.image(h, self.client, &native);
            self.effect.set_content(content);
            self.effect.show(true);
            self.effect.mode = if self.mem.w(S::EFFECT_BLEND) == 1 { 0x67 } else { 0x68 };
        }
        self.dirty = true;
    }
    pub fn apply_overlay(&mut self) {
        let percent = self.mem.uw(S::OVERLAY_PERCENT);
        if percent == 0 {
            self.overlay.visible = false;
        } else {
            self.overlay.content = Some(Content::Fill { color: 0xFFFFFF, size: [self.apini.width, self.apini.height] });
            self.overlay.level = (255 * i32::from(percent) / 100) as u8;
            self.overlay.visible = true;
            self.overlay.mode = if self.mem.w(S::OVERLAY_BLEND) == 1 { 0x6D } else { 0x6C };
        }
        self.dirty = true;
    }

    /// Restart loops and rebuild every object from the scene.
    pub fn rebuild(&mut self, h: &mut dyn HostServices) {
        if let Err(e) = self.rebuild_audio(h) {
            self.res.diag(format!("audio rebuild: {e:#}"));
        }
        self.rebuild_objects(h);
        self.dirty = true;
    }
    fn rebuild_audio(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let music = self.mem.uw(S::MUSIC);
        if music != 0 {
            self.play_music(h, music, true, 0)?;
        }
        for channel in 0..3 {
            let base = S::SOUND_CHANNELS + channel * S::SOUND_STRIDE;
            let sound = i32::from(self.mem.uw(base));
            if sound == 0 {
                continue;
            }
            self.load_sound(h, channel as i32, sound)?;
            if self.mem.uw(base + 2) != 0 {
                let pan = i32::from(self.mem.w(base + 4));
                self.play_sound(h, channel as i32, 999, 1, pan)?;
            }
        }
        Ok(())
    }
    pub fn rebuild_objects(&mut self, h: &mut dyn HostServices) {
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        for l in &mut self.layers {
            l.restore(&self.mem, &mut cx);
        }
        let programs = &self.programs;
        let mem_ro = self.mem.clone();
        let mut string = |script: u16, index: u32| -> Option<Vec<u8>> {
            let p = programs.get(&i32::from(script))?;
            let s = p.strings.get(index as usize)?;
            Some(crate::text::expand(
                s,
                |levels, mut v| {
                    for _ in 0..levels {
                        v = i32::from(mem_ro.var(v));
                    }
                    v
                },
                |r| mem_ro.str_at(S::STRING_REGISTERS + r * S::STRING_STRIDE, S::STRING_STRIDE).to_vec(),
            ))
        };
        self.message.restore(&mut self.mem, &mut cx, &mut string);
        drop(cx);
        self.apply_effect_screen(h);
        self.apply_overlay();
        // sub_41CB70 for each gauge, sub_417B10 for the map.
        for g in 0..S::GAUGE_COUNT {
            let r = S::GAUGES + S::GAUGE_STRIDE * g;
            if self.mem.uw(r + 28) != 0 || self.mem.uw(r + 30) != 0 || self.mem.uw(r + 32) != 0 {
                self.load_gauge(h, g);
            }
        }
        let (n, sub) = (self.mem.uw(S::MAP), self.mem.uw(S::MAP + 2));
        for i in 1..21 {
            self.map.hidden[i] = self.mem.scene[S::MAP + 4 + i] != 0;
        }
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        self.map.open(&mut cx, n, sub);
        self.map.apply();
    }
    /// The message part of the rebuild (sub_429940).
    pub(crate) fn rebuild_message(&mut self, h: &mut dyn HostServices) {
        let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
        let programs = &self.programs;
        let mem_ro = self.mem.clone();
        let mut string = |script: u16, index: u32| -> Option<Vec<u8>> {
            let p = programs.get(&i32::from(script))?;
            let s = p.strings.get(index as usize)?;
            Some(crate::text::expand(
                s,
                |levels, mut v| {
                    for _ in 0..levels {
                        v = i32::from(mem_ro.var(v));
                    }
                    v
                },
                |r| mem_ro.str_at(S::STRING_REGISTERS + r * S::STRING_STRIDE, S::STRING_STRIDE).to_vec(),
            ))
        };
        self.message.restore(&mut self.mem, &mut cx, &mut string);
    }

    // ---------------------------------------------------------- the opening

    /// The top-menu scene: movies 2 then 1, then the start script when the
    /// APINI skips the legacy top menu (sub_432BC0 in the Shapeshifter build).
    pub fn start_opening(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.nest.clear();
        self.cont.clear();
        self.wait = None;
        self.flags = Flags::default();
        self.choice = None;
        self.movie_queue = vec![1, 2];
        self.next_movie(h)
    }
    pub fn next_movie(&mut self, h: &mut dyn HostServices) -> Result<()> {
        while let Some(m) = self.movie_queue.pop() {
            self.play_movie(h, m)?;
            if self.wait.is_some() {
                self.cont.push(Cont::Opening);
                return Ok(());
            }
        }
        self.show_title(h)
    }
    /// New game (sub_432BC0): default scene state, then `script` at depth 0;
    /// the message and previous-choice snapshots take the new scene.
    pub fn new_game(&mut self, h: &mut dyn HostServices, script: u16) -> Result<()> {
        self.wait = None;
        let seed = self.rng.next() as u16;
        self.mem.init_scene(&self.apini, true, seed);
        self.mem.set_w(S::CALL_DEPTH, 0);
        self.message = MessageWindow::new();
        {
            let mut cx = Ctx { h: &mut *h, client: self.client, res: &mut self.res, apini: &self.apini };
            self.message.load_panel(&mut cx);
        }
        self.sync_panel_slider();
        let alpha = self.mem.cfg(C::WINDOW_ALPHA) as u8;
        for b in &mut self.message.boxes {
            b.back_alpha = alpha;
        }
        self.rebuild_objects(h);
        self.thread = Thread { program: None, script: -1, pc: 0 };
        self.load(i32::from(script))?;
        self.jump(0);
        self.mem.capture_message_snapshot();
        self.mem.promote_message_snapshot();
        self.apply_message_speed(h);
        self.dirty = true;
        Ok(())
    }
}

/// sub_44E540: leading spaces/tabs, an optional minus, decimal digits.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while matches!(s.get(i), Some(b' ' | b'\t')) {
        i += 1;
    }
    let neg = s.get(i) == Some(&b'-');
    if neg {
        i += 1;
    }
    let mut n: i32 = 0;
    while let Some(d) = s.get(i).filter(|d| d.is_ascii_digit()) {
        n = n.wrapping_mul(10).wrapping_add(i32::from(d - b'0'));
        i += 1;
    }
    if neg { n.wrapping_neg() } else { n }
}

/// DirectSound pan in hundredths of a decibel as a 0..1 balance.
fn pan_of(hundredths: i32) -> f32 {
    if hundredths == 0 {
        return 0.5;
    }
    let other = 10f32.powf(-(hundredths.unsigned_abs().min(10_000) as f32) / 2000.0);
    let lean = (1.0 - other) / 2.0;
    if hundredths > 0 { 0.5 + lean } else { 0.5 - lean }
}

/// A triangle wave in thirds of `amplitude`.
fn shake(count: i32, amplitude: i32, ms: u64, vertical: bool) -> Vec<(u64, [i32; 2], Option<u32>)> {
    let s = amplitude / 3;
    let wave = [s, 2 * s, 3 * s, 2 * s, s, 0, -s, -2 * s, -3 * s, -2 * s, -s, 0];
    (0..count).flat_map(|_| wave.iter().map(|&d| (ms + 1, if vertical { [0, d] } else { [d, 0] }, None))).collect()
}
/// A loop through sixths of `amplitude`.
fn shake_loop(count: i32, amplitude: i32, ms: u64) -> Vec<(u64, [i32; 2], Option<u32>)> {
    let s = amplitude / 6;
    let path = [[0, 2], [1, 3], [2, 4], [1, 5], [0, 6], [-1, 5], [-2, 4], [-3, 3], [-4, 2], [-5, 0], [-6, -2], [-4, -3], [-2, -2], [-1, -1], [0, 0]];
    (0..count).flat_map(|_| path.iter().map(|p| (ms + 1, [p[0] * s, p[1] * s], None))).collect()
}

impl Vm {
    fn start_steps(&mut self, steps: Vec<(u64, [i32; 2], Option<u32>)>) {
        self.dirty = true;
        self.wait = Some(Wait::Steps { steps, at: self.now });
    }
}
