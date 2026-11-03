//! Continuations from the native VM to CrySirror's semantic adapter.
use crate::{
    GraphApi, HostRequest, HostResponse, NativeCallFrame, SoundApi, SysApi, Value, Vm, VmError,
    VmResult, VmRunOptions, VmStopReason, script,
};
use anyhow::{Result, bail, ensure};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCall {
    pub group: u8,
    pub id: u16,
    pub args: Vec<i32>,
    pub outputs: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Continue,
    Halted,
    Call(NativeCall),
    /// The instruction is still pending. Only a worker-published resource can
    /// release this fence; no placeholder module is installed on a miss.
    LoadProgram {
        archive: String,
        file: String,
    },
    Yielded,
    Input,
    Selection {
        id: u64,
        request: crate::native_selection::NativeSelectionRequest,
    },
    UserDialog {
        request: crate::UserDialogRequest,
    },
    Prompt {
        text: String,
        default_yes: Option<bool>,
    },
    NativeFileRead {
        path: String,
    },
    NativeFileWrite {
        path: String,
        bytes: std::sync::Arc<[u8]>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageSignal {
    Reveal,
    Finish,
    Caret { now_tick: u32 },
}
#[derive(Clone, Debug)]
pub struct SelectionPoll {
    pub id: u64,
    pub callbacks: Vec<[i32; 2]>,
    pub cancelled: bool,
}

enum Effect {
    Selection {
        id: u64,
        request: crate::native_selection::NativeSelectionRequest,
    },
    UserDialog {
        id: u64,
        request: crate::UserDialogRequest,
    },
    Prompt {
        id: u64,
        text: String,
        default_yes: Option<bool>,
    },
    FileRead {
        id: u64,
        context: u64,
        path: String,
    },
    FileWrite {
        id: u64,
        context: u64,
        path: String,
        bytes: std::sync::Arc<[u8]>,
    },
    Native {
        id: u64,
        call: NativeCall,
        strings: BTreeMap<i32, String>,
        memory: BTreeMap<u32, Vec<u8>>,
    },
    Program {
        id: u64,
        archive: String,
        file: String,
    },
}

#[derive(Default)]
struct Services {
    next_id: u64,
    effects: VecDeque<Effect>,
    programs: BTreeMap<(String, String), script::BpProgram>,
    held_keys: Vec<i32>,
    pointer: [i32; 2],
    staged_memory: BTreeMap<u32, Vec<u8>>,
    game_id: String,
    profile: crate::NativeAbiProfile,
    input: crate::input_policy::NativeInputPolicy,
    windows: std::collections::BTreeSet<i32>,
    selections: std::collections::BTreeSet<u64>,
    selection_results: BTreeMap<u64, crate::native_selection::NativeSelectionCompletion>,
    selection_polls: BTreeMap<u64, SelectionPoll>,
    message_animating: bool,
    message_auto_delay: Option<i32>,
    message_signals: Vec<MessageSignal>,
    queued_events: VecDeque<[i32; 3]>,
    error: Option<String>,
    storage_context: u64,
    reads: BTreeMap<(u64, String), Option<Vec<u8>>>,
    writes: BTreeMap<(u64, String), (std::sync::Arc<[u8]>, bool)>,
}

impl Services {
    fn flush_input_events(&mut self) {
        while let Some(event) = self.input.system_events.pop_front() {
            if self.queued_events.len() >= 4096 {
                self.error = Some("BGI queued event limit".into());
                break;
            }
            self.queued_events.push_back(event);
        }
    }
}
fn key(archive: &str, file: &str) -> (String, String) {
    (
        archive.replace('\\', "/").to_lowercase(),
        file.replace('\\', "/").to_lowercase(),
    )
}

fn unsupported(call: &NativeCallFrame) -> VmError {
    VmError::Runtime(format!(
        "BGI native call {:02x}:{:02x} has no CrySirror semantic mapping",
        call.group(),
        call.id()
    ))
}

impl SysApi for Services {
    fn post_queued_event(&mut self, code: i32, parameter: i32) {
        if self.queued_events.len() >= 4096 {
            self.error = Some("BGI queued event limit".into());
        } else {
            self.queued_events.push_back([code, parameter, 0]);
        }
    }
    fn poll_queued_event(&mut self) -> Option<[i32; 3]> {
        self.queued_events.pop_front()
    }

    fn begin_native_storage(&mut self, context: u64) {
        self.storage_context = context;
    }
    fn prepare_file_read(&mut self, path: &str) -> VmResult<bool> {
        let key = (self.storage_context, path.to_owned());
        if self.reads.contains_key(&key) {
            return Ok(true);
        }
        if !self
            .effects
            .iter()
            .any(|e| matches!(e,Effect::FileRead{context,path:p,..}if *context==key.0&&p==path))
        {
            if self.effects.len() >= 512 {
                return Err(VmError::Runtime("BGI host request limit".into()));
            }
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or_else(|| VmError::Runtime("BGI request id exhausted".into()))?;
            self.effects.push_back(Effect::FileRead {
                id: self.next_id,
                context: key.0,
                path: key.1,
            });
        }
        Ok(false)
    }
    fn prepare_file_write(&mut self, path: &str, bytes: &[u8]) -> VmResult<bool> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(VmError::Runtime("BGI native save byte limit".into()));
        }
        let key = (self.storage_context, path.to_owned());
        if let Some((written, _)) = self.writes.get(&key) {
            return Ok(written.as_ref() == bytes);
        }
        if !self.effects.iter().any(|e|matches!(e,Effect::FileWrite{context,path:p,bytes:b,..}if *context==key.0&&p==path&&b.as_ref()==bytes)){
            if self.effects.len()>=512{return Err(VmError::Runtime("BGI host request limit".into()));}
            self.next_id=self.next_id.checked_add(1).ok_or_else(||VmError::Runtime("BGI request id exhausted".into()))?;
            self.effects.push_back(Effect::FileWrite{id:self.next_id,context:key.0,path:key.1,bytes:std::sync::Arc::from(bytes)});
        }
        Ok(false)
    }
    fn read_user_file_bytes(&mut self, path: &str) -> Option<Vec<u8>> {
        self.reads
            .remove(&(self.storage_context, path.to_owned()))
            .flatten()
    }
    fn read_disk_file(&mut self, path: &str) -> Option<Vec<u8>> {
        self.read_user_file_bytes(path)
    }
    fn write_file_bytes(&mut self, path: &str, bytes: &[u8]) -> bool {
        self.writes
            .remove(&(self.storage_context, path.to_owned()))
            .is_some_and(|(prepared, success)| prepared.as_ref() == bytes && success)
    }
    fn game_id(&self) -> &str {
        &self.game_id
    }

    fn capture_native_inputs(
        &mut self,
        opcode: crate::NativeOpcode,
        args: &[Value],
        read: &dyn Fn(u32, usize) -> VmResult<Vec<u8>>,
    ) -> VmResult<()> {
        self.staged_memory.clear();
        let integer = |index: usize| args.get(index).map(Value::as_i32).unwrap_or_default();
        match (opcode.group, opcode.id) {
            (0x90, 0x14) => {
                let (width, height, format) = (integer(1), integer(2), integer(3));
                let bpp = match format {
                    0 => 2,
                    1 | 7 => 3,
                    2 | 4 | 5 => 4,
                    3 => 1,
                    6 => 6,
                    _ => return Err(VmError::Runtime("BGI pixel format".into())),
                };
                let length = usize::try_from(width)
                    .ok()
                    .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
                    .and_then(|n| n.checked_mul(bpp))
                    .filter(|&n| n <= 64 * 1024 * 1024)
                    .ok_or_else(|| VmError::Runtime("BGI pixel input limit".into()))?;
                let pointer = integer(4) as u32;
                self.staged_memory.insert(pointer, read(pointer, length)?);
            }
            (0x80, 0x32) => {
                let length = integer(2) as u32 as usize;
                if length > 64 * 1024 * 1024 {
                    return Err(VmError::Runtime("BGI file-write byte limit".into()));
                }
                let pointer = integer(1) as u32;
                self.staged_memory.insert(pointer, read(pointer, length)?);
            }
            (0xa0, 0x28) => {
                let pointer = integer(1) as u32;
                let header = read(pointer, 64)?;
                let word = |at| u32::from_le_bytes(header[at..at + 4].try_into().unwrap()) as usize;
                let length = word(0)
                    .checked_add(word(8))
                    .filter(|&n| (64..=64 * 1024 * 1024).contains(&n))
                    .ok_or_else(|| VmError::Runtime("BGI memory sound length".into()))?;
                self.staged_memory.insert(pointer, read(pointer, length)?);
            }
            (0x90, 0x78) => {
                let length = usize::try_from(integer(1))
                    .ok()
                    .filter(|&n| n > 0)
                    .and_then(|w| {
                        usize::try_from(integer(2))
                            .ok()
                            .filter(|&n| n > 0)
                            .and_then(|h| w.checked_mul(h))
                    })
                    .and_then(|n| n.checked_mul(2))
                    .filter(|&n| n <= 64 * 1024 * 1024)
                    .ok_or_else(|| VmError::Runtime("BGI tile table dimensions/limit".into()))?;
                let pointer = integer(3) as u32;
                self.staged_memory.insert(pointer, read(pointer, length)?);
            }
            (0x90, 0x44) => {
                let count = integer(0);
                if !(2..=32).contains(&count) {
                    return Err(VmError::Runtime("BGI background frame count".into()));
                }
                let pointer = integer(1) as u32;
                self.staged_memory
                    .insert(pointer, read(pointer, count as usize * 4)?);
            }
            (0x90, 0x98) => {
                let count = integer(0);
                if count > 1 {
                    let count = usize::try_from(count).unwrap();
                    if count > 65536 {
                        return Err(VmError::Runtime("BGI caret frame limit".into()));
                    }
                    let pointer = integer(1) as u32;
                    self.staged_memory
                        .insert(pointer, read(pointer, count * 4)?);
                }
            }
            (0x91, 0x38) if integer(2) as u32 == 0x7fffffff => {
                let pointer = integer(0) as u32;
                self.staged_memory.insert(pointer, read(pointer, 4)?);
            }
            (0x90, 0xcc) => {
                let pointer = integer(1) as u32;
                if pointer != 0 {
                    self.staged_memory.insert(pointer, read(pointer, 24)?);
                }
            }
            _ => (),
        }
        Ok(())
    }
    fn primary_resource_root(&mut self) -> Option<String> {
        Some("C:\\CrySirror\\".into())
    }
    fn allow_direct_filesystem_fallback(&self) -> bool {
        false
    }
    fn host_requests_enabled(&self) -> bool {
        true
    }

    fn prepare_program(&mut self, archive: &str, file: &str) -> VmResult<bool> {
        if self.programs.contains_key(&key(archive, file)) {
            return Ok(true);
        }
        if !self.effects.iter().any(|effect| {
            matches!(effect,
            Effect::Program { archive: a, file: f, .. } if key(a, f) == key(archive, file))
        }) {
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or_else(|| VmError::Runtime("BGI request id exhausted".into()))?;
            self.effects.push_back(Effect::Program {
                id: self.next_id,
                archive: archive.into(),
                file: file.into(),
            });
        }
        Ok(false)
    }

    fn load_program(&mut self, archive: &str, file: &str) -> Option<script::BpProgram> {
        self.programs.get(&key(archive, file)).cloned()
    }

    fn begin_host_request(&mut self, request: &HostRequest) -> VmResult<Option<u64>> {
        if !matches!(request, HostRequest::Native { .. }) {
            if self.effects.len() >= 512 {
                return Err(VmError::Runtime("BGI host request limit".into()));
            }
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or_else(|| VmError::Runtime("BGI request id exhausted".into()))?;
            let effect = match request {
                HostRequest::UserDialog(request) => Effect::UserDialog {
                    id: self.next_id,
                    request: request.clone(),
                },
                HostRequest::Confirmation { text, default_yes } => Effect::Prompt {
                    id: self.next_id,
                    text: text.clone(),
                    default_yes: Some(*default_yes),
                },
                HostRequest::Message { text } => Effect::Prompt {
                    id: self.next_id,
                    text: text.clone(),
                    default_yes: None,
                },
                _ => unreachable!(),
            };
            self.effects.push_back(effect);
            return Ok(Some(self.next_id));
        }
        let HostRequest::Native { opcode, args } = request else {
            unreachable!()
        };
        if opcode.group == 0x90 && matches!(opcode.id, 0xa0 | 0xa2 | 0xa3 | 0xb0 | 0xb1) {
            return Ok(None);
        }
        // Native VM/system/heap/module/scheduler semantics execute in the VM.
        // Visual and sound effects always leave it through a continuation.
        if !matches!(opcode.group, 0x90..=0x92 | 0xa0 | 0xb0 | 0xc0)
            && !(opcode.group == 0x80 && matches!(opcode.id, 0x30..=0x35))
        {
            return Ok(None);
        }
        let mut strings = BTreeMap::new();
        let mut values = Vec::with_capacity(args.len());
        for (index, value) in args.iter().enumerate() {
            values.push(match value {
                Value::None => 0,
                Value::Int(value) => *value,
                Value::Ptr(value) => *value as i32,
                Value::Str(text) => {
                    // Adapter-only string tokens. Never installed in guest
                    // memory or exposed as host pointers/executable addresses.
                    let token = 0xf000_0000u32.wrapping_add(index as u32) as i32;
                    strings.insert(token, text.clone());
                    token
                }
                _ => {
                    return Err(VmError::Runtime(format!(
                        "BGI native {:02x}:{:02x} requires a typed value mapping",
                        opcode.group, opcode.id
                    )));
                }
            });
        }
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| VmError::Runtime("BGI host request id exhausted".into()))?;
        let outputs = script::native_abi::lookup_profile(self.profile, opcode.group, opcode.id)
            .map(|abi| abi.stack_outputs)
            .unwrap_or_else(|| {
                script::calls::known_call_stack_output_count(opcode.group, opcode.id)
            });
        self.effects.push_back(Effect::Native {
            id: self.next_id,
            call: NativeCall {
                group: opcode.group,
                id: opcode.id,
                args: values,
                outputs,
            },
            strings,
            memory: std::mem::take(&mut self.staged_memory),
        });
        Ok(Some(self.next_id))
    }

    fn call_sys(&mut self, call: &mut NativeCallFrame) -> VmResult<Value> {
        Err(unsupported(call))
    }
    fn query_key_held(&mut self, descriptor: i32) -> bool {
        self.input.query_held(descriptor)
    }
    fn keyboard_state(&mut self) -> [u8; 256] {
        self.input.keyboard_state()
    }
    fn pointer_position(&mut self, index: i32) -> Option<(i32, i32)> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.input.press_points.get(i))
            .map(|p| (p[0], p[1]))
    }
    fn read_input_state(&mut self, d: i32) -> i32 {
        self.input.read_state(d)
    }
    fn peek_input_state(&mut self, d: i32) -> i32 {
        self.input.peek_state(d)
    }
    fn sample_configured_input(&mut self) {
        self.input.sample_configured();
    }
    fn reset_input_configuration(&mut self, v: i32) {
        self.input.configuration_enabled = v;
        self.input.clear();
    }
    fn register_input_scope(&mut self, s: i32) {
        self.input.register_scope(s, false);
    }
    fn query_and_unregister_input_scope(&mut self, s: i32) {
        self.input.unregister_scope(s, false);
    }
    fn register_message_input_scope(&mut self, s: i32) {
        self.input.register_scope(s, true);
    }
    fn unregister_message_input_scope(&mut self, s: i32) {
        self.input.unregister_scope(s, true);
    }
    fn query_input_event_bits(&mut self, s: i32) -> i32 {
        self.input.query_bits(s, false)
    }
    fn query_message_input_event_bits(&mut self, s: i32) -> i32 {
        self.input.query_bits(s, true)
    }
    fn register_input_class_descriptors(&mut self, c: i32, d: &[i32]) {
        self.input.register_class(c, d);
    }
    fn query_configured_input_gate(&mut self) -> i32 {
        self.input.configured_gate()
    }
    fn query_input_descriptor_state(&mut self, c: i32) -> i32 {
        self.input.class_state(c)
    }
    fn query_scoped_input_event(&mut self, d: i32, s: i32) -> i32 {
        self.input.scoped_event(d, s)
    }
    fn set_input_master_gate(&mut self, v: i32) {
        self.input.master_gate = v;
    }
    fn set_input_latched_state(&mut self, v: i32) {
        self.input.latched_state = v;
    }
    fn set_background_key_polling(&mut self, v: i32) {
        self.input.background_polling = v;
    }
    fn swap_input_repeat_flag(&mut self, d: i32, v: i32) -> i32 {
        usize::try_from(d)
            .ok()
            .and_then(|i| self.input.repeat.get_mut(i))
            .map(|old| std::mem::replace(old, v))
            .unwrap_or(0)
    }
    fn input_message_serial(&mut self) -> i32 {
        self.input.serial
    }
    fn window_message_sequence(&mut self) -> i32 {
        self.input.window_serial
    }
    fn poll_window_message(&mut self, m: i32, s: i32) -> Option<(i32, i32)> {
        self.input.poll_window(m, s)
    }
    fn set_mouse_button_mapping(&mut self, mode: i32) -> bool {
        if !(0..=1).contains(&mode) {
            return false;
        }
        self.input.mouse_mapping_mode = mode;
        true
    }
    fn user_data_root(&mut self, _kind: i32) -> Option<String> {
        self.primary_resource_root()
    }
    fn directory_exists(&mut self, path: &str) -> bool {
        path.replace('\\', "/").trim_end_matches('/') == "C:/CrySirror"
    }
    fn window_active(&mut self) -> bool {
        self.input.focused
    }
    fn window_minimized(&mut self) -> bool {
        !self.input.restored
    }
    fn inject_mouse_click(&mut self, button: i32) -> bool {
        if !matches!(button, 1 | 2 | 4 | 5 | 6) {
            return false;
        }
        self.input.pulse(button);
        true
    }
}
impl GraphApi for Services {
    fn graph_window_exists(&self, window: i32) -> bool {
        self.windows.contains(&window)
    }
    fn start_native_selection(
        &mut self,
        request: crate::native_selection::NativeSelectionRequest,
    ) -> VmResult<u64> {
        if self.selections.len() >= 128 || self.effects.len() >= 512 {
            return Err(VmError::Runtime("BGI native selection limit".into()));
        }
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| VmError::Runtime("BGI request id exhausted".into()))?;
        let id = self.next_id;
        self.selections.insert(id);
        self.effects.push_back(Effect::Selection { id, request });
        Ok(id)
    }
    fn poll_native_selection(
        &mut self,
        id: u64,
        callbacks: &[[i32; 2]],
        cancelled: bool,
    ) -> Option<crate::native_selection::NativeSelectionCompletion> {
        if let Some(result) = self.selection_results.remove(&id) {
            self.selections.remove(&id);
            self.selection_polls.remove(&id);
            return Some(result);
        }
        if self.selections.contains(&id) {
            let poll = self
                .selection_polls
                .entry(id)
                .or_insert_with(|| SelectionPoll {
                    id,
                    callbacks: Vec::new(),
                    cancelled: false,
                });
            if poll.callbacks.len().saturating_add(callbacks.len()) > 4096 {
                self.error = Some("BGI selection callback limit".into());
            } else {
                poll.callbacks.extend_from_slice(callbacks);
            }
            poll.cancelled |= cancelled;
        }
        None
    }

    fn native_message_is_animating(&self) -> bool {
        self.message_animating
    }
    fn native_message_auto_advance_delay_ms(&self, _configured: Option<i32>) -> Option<i32> {
        self.message_auto_delay
    }
    fn reveal_native_message(&mut self) -> bool {
        let was = self.message_animating;
        self.message_animating = false;
        self.message_signals.push(MessageSignal::Reveal);
        was
    }
    fn finish_native_message(&mut self) {
        self.message_animating = false;
        self.message_signals.push(MessageSignal::Finish);
    }
    fn tick_native_message_caret(&mut self, now_tick: u32) {
        self.message_signals.push(MessageSignal::Caret { now_tick });
    }

    fn call_graph(&mut self, call: &mut NativeCallFrame) -> VmResult<Value> {
        Err(unsupported(call))
    }
}
impl SoundApi for Services {
    fn call_sound(&mut self, call: &mut NativeCallFrame) -> VmResult<Value> {
        Err(unsupported(call))
    }
    fn call_user(&mut self, call: &mut NativeCallFrame) -> VmResult<Option<Value>> {
        Err(unsupported(call))
    }
}

/// The original native VM with engine-owned host effects. No second renderer,
/// image implementation, font rasterizer, sound device or mixer is embedded.
pub struct Machine {
    vm: Vm,
    services: Services,
    now_ms: u64,
    offset: usize,
    work_steps: usize,
}
impl Machine {
    pub fn from_bp(bytes: &[u8]) -> Result<Self> {
        Self::from_bp_with_profile(bytes, crate::NativeAbiProfile::Tayutama)
    }
    pub fn from_bp_with_profile(bytes: &[u8], profile: crate::NativeAbiProfile) -> Result<Self> {
        ensure!(
            bytes.len() >= 8 && bytes.len() <= 16 * 1024 * 1024,
            "BGI BP source budget/header"
        );
        let header = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        ensure!(
            header >= 8
                && header <= bytes.len()
                && header.checked_add(size) == Some(bytes.len())
                && size > 0,
            "inconsistent native BP image"
        );
        let program = script::parse_bp_program(None, bytes);
        ensure!(
            !program.instructions.is_empty(),
            "BGI BP has no instructions"
        );
        let mut vm = Vm::new();
        vm.set_native_abi_profile(profile);
        vm.start(&program);
        Ok(Self {
            vm,
            services: Services {
                profile,
                game_id: detect_game_id_from_bootstrap(&program)
                    .unwrap_or_else(|| "Tayutama2TV".into()),
                ..Default::default()
            },
            now_ms: 0,
            offset: 0,
            work_steps: 0,
        })
    }
    pub fn pc(&self) -> usize {
        self.offset
    }
    pub fn halted(&self) -> bool {
        self.vm.halted
    }
    pub fn module_count(&self) -> usize {
        self.vm
            .program_active
            .iter()
            .filter(|&&active| active)
            .count()
    }
    pub fn thread_count(&self) -> usize {
        self.vm.resource_usage().vm_count
    }
    pub fn stack_index(&self) -> usize {
        self.vm.stack.len() % crate::OPERAND_STACK_CAPACITY
    }
    pub fn top(&self) -> i32 {
        self.vm
            .stack
            .last()
            .map(Value::as_i32)
            .unwrap_or_else(|| self.vm.operand_slots[crate::OPERAND_STACK_CAPACITY - 1].as_i32())
    }
    #[cfg(test)]
    pub(crate) fn push(&mut self, value: i32) {
        self.vm.push_value(Value::Int(value));
    }
    #[cfg(test)]
    pub(crate) fn pop(&mut self) -> i32 {
        self.vm.pop_value().unwrap().as_i32()
    }
    pub fn pulse_input(&mut self, descriptor: i32) {
        self.services.input.pulse(descriptor);
        self.services.flush_input_events();
    }
    pub fn update_message_state(&mut self, animating: bool, auto_delay: Option<i32>) {
        self.services.message_animating = animating;
        self.services.message_auto_delay = auto_delay;
    }
    pub fn take_message_signals(&mut self) -> Vec<MessageSignal> {
        std::mem::take(&mut self.services.message_signals)
    }
    pub fn post_message_event(&mut self, event: [i32; 3]) -> Result<()> {
        ensure!(
            self.services.queued_events.len() < 4096,
            "BGI queued event limit"
        );
        self.services.queued_events.push_back(event);
        Ok(())
    }
    pub fn update_host(&mut self, now_ms: u64, held_keys: Vec<i32>, pointer: [i32; 2]) {
        self.vm.advance_time_ms(now_ms.saturating_sub(self.now_ms));
        self.now_ms = now_ms;
        self.services.input.update(&held_keys, pointer);
        self.services.flush_input_events();
        self.services.held_keys = held_keys;
        self.services.pointer = pointer;
    }
    pub fn string(&self, pointer: i32) -> Result<String> {
        if let Some(Effect::Native { strings, .. }) = self.services.effects.front() {
            if let Some(text) = strings.get(&pointer) {
                return Ok(text.clone());
            }
        }
        self.vm.read_c_string(pointer as u32).map_err(Into::into)
    }
    pub fn commit_call_with_start(
        &mut self,
        outputs: &[i32],
        start: crate::native_call::NativeProcedureStart,
    ) -> Result<()> {
        let Some(Effect::Native { id, call, .. }) = self.services.effects.front() else {
            bail!("no pending BGI native call")
        };
        ensure!(
            outputs.len() == call.outputs,
            "BGI native procedure output count"
        );
        self.vm.complete_host_request_with_start(
            *id,
            HostResponse {
                outputs: outputs.iter().copied().map(Value::Int).collect(),
                writes: Vec::new(),
            },
            crate::NativeOpcode {
                group: call.group,
                id: call.id,
            },
            start,
        )?;
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn commit_call(&mut self, outputs: &[i32]) -> Result<()> {
        self.commit_call_writes(outputs, vec![])
    }
    pub fn memory(&self, pointer: u32, count: usize) -> Result<Vec<u8>> {
        let Some(Effect::Native { memory, .. }) = self.services.effects.front() else {
            bail!("no pending BGI memory request")
        };
        for (&start, bytes) in memory {
            if let Some(offset) = pointer.checked_sub(start).map(|v| v as usize) {
                if let Some(bytes) = offset
                    .checked_add(count)
                    .and_then(|end| bytes.get(offset..end))
                {
                    return Ok(bytes.to_vec());
                }
            }
        }
        bail!("BGI native input memory was not captured: {pointer:#x}+{count}")
    }
    pub fn commit_call_writes(
        &mut self,
        outputs: &[i32],
        writes: Vec<crate::HostMemoryWrite>,
    ) -> Result<()> {
        let Some(Effect::Native { id, call, .. }) = self.services.effects.front() else {
            bail!("no pending BGI native call");
        };
        ensure!(
            outputs.len() == call.outputs,
            "BGI native result count differs from ABI"
        );
        self.vm.complete_host_request(
            *id,
            HostResponse {
                outputs: outputs.iter().copied().map(Value::Int).collect(),
                writes,
            },
        )?;
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn update_window_handles(&mut self, windows: Vec<i32>) {
        self.services.windows = windows.into_iter().collect();
    }
    pub fn commit_selection_start(&mut self, id: u64) -> Result<()> {
        ensure!(
            matches!(self.services.effects.front(),Some(Effect::Selection{id:expected,..})if *expected==id),
            "BGI selection start identity"
        );
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn complete_selection(
        &mut self,
        id: u64,
        result: crate::native_selection::NativeSelectionCompletion,
    ) -> Result<()> {
        ensure!(
            self.services.selections.contains(&id)
                && !self.services.selection_results.contains_key(&id),
            "BGI stale/duplicate selection result"
        );
        ensure!(result.count <= 3, "BGI selection result count");
        self.services.selection_results.insert(id, result);
        Ok(())
    }
    pub fn take_selection_polls(&mut self) -> Vec<SelectionPoll> {
        std::mem::take(&mut self.services.selection_polls)
            .into_values()
            .collect()
    }
    pub fn query_selection_input(&mut self, keyboard_scope: i32, pointer_scope: i32) -> i32 {
        self.services
            .input
            .query_bits_split(keyboard_scope, pointer_scope)
    }
    pub fn register_selection_input(
        &mut self,
        _id: u64,
        scope: i32,
        keyboard: bool,
        pointer: bool,
    ) {
        self.services
            .input
            .register_selection_scopes(scope, keyboard, pointer);
    }
    pub fn unregister_selection_input(
        &mut self,
        _id: u64,
        scope: i32,
        keyboard: bool,
        pointer: bool,
    ) {
        self.services
            .input
            .unregister_selection_scopes(scope, keyboard, pointer);
    }
    pub fn query_selection_descriptor(&mut self, descriptor: i32, scope: i32) -> i32 {
        self.services.query_scoped_input_event(descriptor, scope)
    }
    pub fn commit_user_dialog(
        &mut self,
        response: Option<crate::UserDialogResponse>,
    ) -> Result<()> {
        let Some(Effect::UserDialog { id, .. }) = self.services.effects.front() else {
            bail!("no BGI user dialog pending")
        };
        self.vm.complete_user_dialog(*id, response)?;
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn commit_prompt(&mut self, accepted: bool) -> Result<()> {
        let Some(Effect::Prompt {
            id, default_yes, ..
        }) = self.services.effects.front()
        else {
            bail!("no BGI prompt pending")
        };
        self.vm.complete_host_request(
            *id,
            HostResponse {
                outputs: if default_yes.is_some() {
                    vec![Value::Int(i32::from(accepted))]
                } else {
                    Vec::new()
                },
                writes: Vec::new(),
            },
        )?;
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn commit_file_read(&mut self, bytes: Option<Vec<u8>>) -> Result<()> {
        let Some(Effect::FileRead { context, path, .. }) = self.services.effects.front() else {
            bail!("no BGI native file read pending")
        };
        ensure!(
            bytes.as_ref().is_none_or(|b| b.len() <= 64 * 1024 * 1024),
            "BGI native file read limit"
        );
        self.services.reads.insert((*context, path.clone()), bytes);
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn commit_file_write(&mut self, success: bool) -> Result<()> {
        let Some(Effect::FileWrite {
            context,
            path,
            bytes,
            ..
        }) = self.services.effects.front()
        else {
            bail!("no BGI native file write pending")
        };
        self.services
            .writes
            .insert((*context, path.clone()), (bytes.clone(), success));
        self.services.effects.pop_front();
        Ok(())
    }
    pub fn commit_program(&mut self, bytes: &[u8]) -> Result<()> {
        let Some(Effect::Program { archive, file, .. }) = self.services.effects.front() else {
            bail!("no pending BGI program load");
        };
        ensure!(bytes.len() <= 16 * 1024 * 1024, "BGI module byte budget");
        let program = script::parse_bp_program(Some(format!("{archive}:{file}")), bytes);
        ensure!(
            !program.instructions.is_empty(),
            "BGI loaded module has no instructions"
        );
        let total: usize = self
            .services
            .programs
            .values()
            .map(|program| program.code.len())
            .sum();
        ensure!(
            total.saturating_add(program.code.len()) <= 64 * 1024 * 1024,
            "BGI module cache byte budget"
        );
        self.services.programs.insert(key(archive, file), program);
        self.services.effects.pop_front();
        Ok(())
    }
    fn effect(&self) -> Option<Step> {
        self.services.effects.front().map(|effect| match effect {
            Effect::Selection { id, request } => Step::Selection {
                id: *id,
                request: request.clone(),
            },
            Effect::UserDialog { request, .. } => Step::UserDialog {
                request: request.clone(),
            },
            Effect::Prompt {
                text, default_yes, ..
            } => Step::Prompt {
                text: text.clone(),
                default_yes: *default_yes,
            },
            Effect::FileRead { path, .. } => Step::NativeFileRead { path: path.clone() },
            Effect::FileWrite { path, bytes, .. } => Step::NativeFileWrite {
                path: path.clone(),
                bytes: bytes.clone(),
            },
            Effect::Native { call, .. } => Step::Call(call.clone()),
            Effect::Program { archive, file, .. } => Step::LoadProgram {
                archive: archive.clone(),
                file: file.clone(),
            },
        })
    }
    pub fn step(&mut self) -> Result<Step> {
        self.step_with_budget(1)
    }
    pub fn consumed_steps(&self) -> usize {
        self.work_steps
    }
    pub fn step_with_budget(&mut self, budget: usize) -> Result<Step> {
        self.work_steps = 0;
        if self.vm.halted {
            self.services.effects.clear();
            return Ok(Step::Halted);
        }
        if let Some(effect) = self.effect() {
            return Ok(effect);
        }
        if self.vm.halted {
            return Ok(Step::Halted);
        }
        let (reason, _) = self.pump_vm(budget)?;
        if self.vm.halted {
            self.services.effects.clear();
            return Ok(Step::Halted);
        }
        if let Some(effect) = self.effect() {
            return Ok(effect);
        }
        match reason {
            VmStopReason::Completed | VmStopReason::InterpreterTerminated => Ok(Step::Halted),
            VmStopReason::WaitingForInput if self.thread_count() == 1 => Ok(Step::Input),
            VmStopReason::WaitingForTime
            | VmStopReason::WaitingForInputOrTime
            | VmStopReason::WaitingForInput
            | VmStopReason::WaitingForProcedure
            | VmStopReason::Yielded
            | VmStopReason::SchedulerPassEnded => Ok(Step::Yielded),
            VmStopReason::WaitingForHost => bail!("BGI host wait has no semantic request"),
            _ => Ok(Step::Continue),
        }
    }
    pub fn request_id(&self) -> Option<u64> {
        self.services.effects.front().map(|effect| match effect {
            Effect::Native { id, .. }
            | Effect::Program { id, .. }
            | Effect::FileRead { id, .. }
            | Effect::FileWrite { id, .. }
            | Effect::UserDialog { id, .. }
            | Effect::Prompt { id, .. }
            | Effect::Selection { id, .. } => *id,
        })
    }
    pub fn postpone_request(&mut self) {
        if let Some(effect) = self.services.effects.pop_front() {
            self.services.effects.push_back(effect);
        }
    }
    pub fn select_request(&mut self, seen: &std::collections::BTreeSet<u64>) -> bool {
        for _ in 0..self.services.effects.len() {
            if self.request_id().is_some_and(|id| !seen.contains(&id)) {
                return true;
            }
            self.postpone_request();
        }
        false
    }
    pub fn advance_scheduler(&mut self) -> Result<bool> {
        self.advance_scheduler_with_budget(1)
    }
    pub fn advance_scheduler_with_budget(&mut self, budget: usize) -> Result<bool> {
        self.work_steps = 0;
        if self.vm.halted {
            return Ok(false);
        }
        Ok(self.pump_vm(budget)?.1)
    }
    fn pump_vm(&mut self, budget: usize) -> Result<(VmStopReason, bool)> {
        let report = self.vm.run_loaded(
            &mut self.services,
            &VmRunOptions {
                max_steps: budget.max(1),
                trace: false,
                fail_on_stub: true,
                collect_diagnostics: false,
            },
        );
        if let Some(error) = self.services.error.take() {
            bail!("{error}");
        }
        self.offset = report
            .offset
            .map(|offset| offset as usize)
            .unwrap_or(report.pc);
        let reports = self.vm.take_async_reports();
        self.work_steps = reports.iter().fold(report.steps, |total, report| {
            total.saturating_add(report.steps)
        });
        let progressed = report.steps > 0 || reports.iter().any(|report| report.steps > 0);
        ensure!(
            !report.stop_reason.is_fatal(),
            "BGI {} pc={:#x}: {:?}: {}",
            report.program,
            self.offset,
            report.stop_reason,
            self.vm
                .recent_trace
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        );
        Ok((report.stop_reason, progressed))
    }
}

fn detect_game_id_from_bootstrap(program: &script::BpProgram) -> Option<String> {
    for (index, instruction) in program.instructions.iter().enumerate() {
        if instruction.known_call != Some("GetGameId") {
            continue;
        }

        let following = program.instructions.iter().skip(index + 1).take(6);
        let mut candidate = None;
        for instruction in following {
            if instruction.opcode.name() == "streq" {
                if candidate.is_some() {
                    return candidate;
                }
                break;
            }
            if let [script::BpOperand::String(value)] = instruction.operands.as_ref()
                && !value.is_empty()
            {
                candidate = Some(value.clone());
            }
        }
    }
    None
}
