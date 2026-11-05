//! Native B0 command banks and @dN re-entry (4284B0/431F20). The records contain
//! resolved scalar arguments and strings, never callbacks or executable code.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct NativeCommand {
    opcode: u8,
    arguments: [i16; 8],
    strings: [String; 8],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MacroReplay {
    id: u8,
    index: usize,
    commands: Vec<NativeCommand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MacroState {
    recording: bool,
    group: u8,
    cursor: u8,
    banks: BTreeMap<u8, Vec<NativeCommand>>,
    replay: Option<MacroReplay>,
    begin_dispatch: bool,
    #[serde(skip)]
    executing: bool,
}
impl Default for MacroState {
    fn default() -> Self {
        Self {
            recording: false,
            group: 0,
            cursor: 0,
            banks: BTreeMap::new(),
            replay: None,
            begin_dispatch: true,
            executing: false,
        }
    }
}
impl MacroState {
    pub(super) fn suppresses_last_opcode_update(&self)->bool{self.executing}
}
impl MacroState {
    pub(super) fn from_native_body(
        body: &crate::native_save::NativeSaveBody,
        previous: &Self,
    ) -> Result<Self> {
        let mut result = Self::default();
        result.banks = previous
            .banks
            .iter()
            .filter(|(group, _)| **group >= 5)
            .map(|(group, bank)| (*group, bank.clone()))
            .collect();
        result.begin_dispatch = false;
        for group in 0..5u8 {
            let mut bank = Vec::with_capacity(10);
            for index in 0..10usize {
                let base = 269852 + 2116 * (usize::from(group) * 10 + index);
                let opcode = u8::try_from(body.native_word(base)?)
                    .context("Circus saved macro opcode range")?;
                let mut arguments = [0i16; 8];
                let mut strings = std::array::from_fn(|_| String::new());
                for field in 0..8 {
                    arguments[field] =
                        i16::try_from(body.native_word(base + 2084 + 4 * field)? as i32)
                            .context("Circus saved macro signed argument range")?;
                    strings[field] = body.native_string(base + 4 + 260 * field, 260)?;
                }
                // Zero opcodes terminate execution, but residual operands are
                // retained for a later in-place native bank overwrite.
                bank.push(NativeCommand {
                    opcode,
                    arguments,
                    strings,
                });
            }
            result.banks.insert(group, bank);
        }
        Ok(result)
    }
    pub(super) fn finish_dispatch(&mut self) {
        self.recording = false;
        self.group = 0;
        self.cursor = 0;
        self.begin_dispatch = true;
    }
    pub(super) fn executing_positive(&self) -> bool {
        self.executing
    }
    pub(super) fn active(&self) -> bool {
        self.replay.is_some()
    }
}
impl Machine {
    /// This hook commits decoded record/selector commands itself. They do not
    /// execute the captured opcode's normal resource/state mutation paths.
    pub(super) fn record_macro(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
    ) -> Result<Option<Outcome>> {
        if self.state.macros.executing {
            return if i.opcode == 0xb0 {
                Ok(Some(Outcome::Advanced))
            } else {
                Ok(None)
            };
        }
        if self.state.macros.begin_dispatch {
            self.state.macros.banks.retain(|group, _| *group >= 5);
            self.state.native_live.clear_macro_records();
            self.state.macros.begin_dispatch = false;
        }
        let handled = if i.opcode == 0xb0 {
            if a[0] == 255 {
                ensure!(a[1] >= 0, "Circus native macro group index is negative");
                let group = a[1].min(9) as u8;
                if i16::from(self.state.macros.group) != a[1] {
                    self.state.macros.group = group;
                    self.state.macros.cursor = 0;
                }
            } else {
                self.state.macros.recording = !self.state.macros.recording;
            }
            true
        } else if self.state.macros.recording {
            if matches!(i.opcode, 0x6a | 0x6b) {
                self.state.macros.group = 0;
                self.state.macros.cursor = 0;
                false
            } else {
                let macros = &mut self.state.macros;
                ensure!(
                    macros.cursor < 10,
                    "Circus native macro bank exceeds ten commands (native overflow dialog)"
                );
                let bank = macros.banks.entry(macros.group).or_default();
                let index = usize::from(macros.cursor);
                let mut strings = bank
                    .get(index)
                    .map(|command| command.strings.clone())
                    .unwrap_or_else(|| std::array::from_fn(|_| String::new()));
                let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(text);
                ensure!(!error, "Circus macro command text encoding");
                // The original retains previous bytes when a source string is
                // too long, rather than copying a truncated string.
                if bytes.len() < 260 {
                    strings[0] = text.into();
                }
                for string in &mut strings[1..] {
                    *string = String::new();
                }
                self.state.native_live.macro_record(
                    macros.group,
                    macros.cursor,
                    i.opcode,
                    a,
                    &strings,
                )?;
                let command = NativeCommand {
                    opcode: i.opcode,
                    arguments: a,
                    strings,
                };
                if index < bank.len() {
                    bank[index] = command;
                } else {
                    bank.push(command);
                }
                macros.cursor += 1;
                true
            }
        } else {
            false
        };
        if handled {
            self.state
                .native_live
                .advance_pc(&self.state.scene, i.next)?;
            self.state.pc = i.next;
            self.state.instructions += 1;
            Ok(Some(Outcome::Advanced))
        } else {
            Ok(None)
        }
    }

    pub(super) fn replay_macro(
        &mut self,
        id: i32,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<bool> {
        ensure!((0..=10).contains(&id), "Circus native inline macro index");
        let id = id as u8;
        if let Some(replay) = &self.state.macros.replay {
            ensure!(replay.id == id, "Circus native macro continuation identity");
        } else {
            let commands = if id == 0 {
                Vec::new()
            } else {
                self.state
                    .macros
                    .banks
                    .get(&(id - 1))
                    .cloned()
                    .unwrap_or_default()
            };
            ensure!(commands.len() <= 10, "Circus native macro command count");
            self.state.macros.replay = Some(MacroReplay {
                id,
                index: 0,
                commands,
            });
            if id == 0 {
                self.state.macros.recording = false;
                self.state.macros.group = 0;
                self.state.macros.cursor = 0;
                self.state.macros.begin_dispatch = false;
            }
        }
        // Native @d0 calls the normal dispatcher at the current main PC.
        // Positive IDs read up to ten frozen native records without changing
        // the caller's main PC or its instruction counter.
        for _ in 0..4096 {
            let replay = self.state.macros.replay.as_ref().unwrap();
            let outer_pc = self.state.pc;
            let outer_count = self.state.instructions;
            let outer_scene = self.state.scene.clone();
            if self.program.is_none() {
                let Some(program) = host.scene(&self.state.scene)? else {
                    return Ok(false);
                };
                self.program = Some(program);
            }
            let program = self.program.as_ref().unwrap().clone();
            let instruction = if id == 0 {
                program.instruction(outer_pc)?.clone()
            } else {
                let Some(command) = replay.commands.get(replay.index) else {
                    self.state.macros.replay = None;
                    return Ok(true);
                };
                if command.opcode == 0 {
                    self.state.macros.replay = None;
                    return Ok(true);
                }
                command.instruction(outer_pc)
            };
            self.state.macros.executing = id != 0;
            let outer_dispatch=(self.state.dispatch_a2,self.state.dispatch_a3);
            self.state.dispatch_a2=0;
            self.state.dispatch_a3=false;
            self.state.native_flags.insert(272090,-i32::from(id));
            let result = self.execute(&program, &instruction, host, now);
            self.state.dispatch_a2=outer_dispatch.0;
            self.state.dispatch_a3=outer_dispatch.1;
            self.state.macros.executing = false;
            if id != 0 {
                self.state.instructions = outer_count;
                if self.state.scene == outer_scene {
                    self.state.pc = outer_pc;
                    self.state
                        .native_live
                        .advance_pc(&self.state.scene, outer_pc)?;
                }
            }
            let outcome = result?;
            if outcome == Outcome::Pending {
                return Ok(false);
            }
            self.state.macros.replay.as_mut().unwrap().index += 1;
            if outcome != Outcome::Advanced {
                self.state.macros.replay = None;
                return Ok(true);
            }
        }
        bail!("Circus reentrant native macro dispatch instruction budget exceeded")
    }
}
impl NativeCommand {
    fn instruction(&self, pc: u32) -> Instruction {
        let a = self.arguments;
        let arguments = match self.opcode {
            0x00..=0x3a => Arguments::Bytes([a[0] as u8, a[1] as u8]),
            0x3b..=0x47 => Arguments::SlotString {
                slot: a[0] as u8,
                text: self.strings[0].clone(),
            },
            0x48..=0x6d => Arguments::String(self.strings[0].clone()),
            _ => Arguments::Words(a[..4].try_into().unwrap()),
        };
        Instruction {
            offset: pc,
            next: pc,
            opcode: self.opcode,
            arguments,
            continuation: Some(a[4..].try_into().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        pending: bool,
        events: usize,
    }
    impl NativeHost for Host {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            Ok(None)
        }
        fn emit(&mut self, _: &Event) -> Result<bool> {
            if self.pending {
                self.pending = false;
                Ok(false)
            } else {
                self.events += 1;
                Ok(true)
            }
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            Ok(false)
        }
    }
    fn command(opcode: u8, args: [i16; 8]) -> NativeCommand {
        NativeCommand {
            opcode,
            arguments: args,
            strings: std::array::from_fn(|_| String::new()),
        }
    }
    #[test]
    fn original_save_restores_only_five_banks_and_retains_zero_record_payloads() {
        let mut machine = Machine::new("test");
        let mut names = std::array::from_fn(|_| String::new());
        names[0] = "retained".into();
        machine
            .state
            .native_live
            .macro_record(0, 0, 0, [0; 8], &names)
            .unwrap();
        machine
            .state
            .macros
            .banks
            .insert(5, vec![command(0x82, [3, 2, 55, 0, 0, 0, 0, 0])]);
        let body = machine.state.native_live.checkpoint(3).unwrap();
        let restored = MacroState::from_native_body(&body, &machine.state.macros).unwrap();
        assert_eq!(restored.banks[&0].len(), 10);
        assert_eq!(restored.banks[&0][0].opcode, 0);
        assert_eq!(restored.banks[&0][0].strings[0], "retained");
        assert_eq!(restored.banks[&5][0].arguments[2], 55);
    }
    fn scene() -> Arc<Program> {
        Arc::new(Program {
            profile: Profile::Dc4PlusHarmony,
            code_file_offset: 0,
            labels: vec![],
            label_metadata: vec![],
            raw_strings: Default::default(),
            instructions: vec![],
        })
    }
    #[test]
    fn recording_freezes_values_without_running_assignment() {
        let mut machine = Machine::new("test");
        let toggle = command(0xb0, [0; 8]).instruction(3);
        assert!(machine.record_macro(&toggle, [0; 8], "").unwrap().is_some());
        let assign = command(0x82, [3, 2, 44, 0, 0, 0, 0, 0]).instruction(6);
        machine
            .record_macro(&assign, [3, 2, 44, 0, 0, 0, 0, 0], "")
            .unwrap();
        assert_eq!(machine.state.value(3, 2).unwrap(), 0);
        assert_eq!(machine.state.macros.banks[&0][0].arguments[2], 44);
        machine.state.macros.recording = false;
        machine.program = Some(scene());
        let mut host = Host::default();
        assert!(machine.replay_macro(1, &mut host, 0).unwrap());
        assert_eq!(machine.state.value(3, 2).unwrap(), 44);
    }
    #[test]
    fn resource_retry_keeps_macro_cursor_and_outer_pc_counter() {
        let mut machine = Machine::new("test");
        machine.program = Some(scene());
        machine.state.pc = 912;
        machine.state.instructions = 77;
        machine.state.macros.banks.insert(
            0,
            vec![
                command(0x90, [7, 1, 2, 3, 0, 0, 0, 0]),
                command(0x82, [3, 2, 44, 0, 0, 0, 0, 0]),
            ],
        );
        let mut host = Host {
            pending: true,
            ..Default::default()
        };
        assert!(!machine.replay_macro(1, &mut host, 0).unwrap());
        assert_eq!(machine.state.pc, 912);
        assert_eq!(machine.state.instructions, 77);
        assert_eq!(machine.state.macros.replay.as_ref().unwrap().index, 0);
        assert!(machine.replay_macro(1, &mut host, 0).unwrap());
        assert_eq!(machine.state.pc, 912);
        assert_eq!(machine.state.instructions, 77);
        assert_eq!(host.events, 1);
        assert_eq!(machine.state.value(3, 2).unwrap(), 44);
    }
    #[test]
    fn bank_expiry_occurs_at_next_dispatch_and_upper_banks_survive() {
        let mut machine = Machine::new("test");
        machine.program = Some(scene());
        machine
            .state
            .macros
            .banks
            .insert(0, vec![command(0x82, [3, 2, 44, 0, 0, 0, 0, 0])]);
        machine
            .state
            .macros
            .banks
            .insert(5, vec![command(0x82, [3, 2, 55, 0, 0, 0, 0, 0])]);
        machine.state.macros.finish_dispatch();
        assert_eq!(machine.state.macros.banks.len(), 2);
        machine.replay_macro(1, &mut Host::default(), 0).unwrap();
        assert_eq!(machine.state.value(3, 2).unwrap(), 44);
        let ordinary = command(0x82, [3, 2, 66, 0, 0, 0, 0, 0]).instruction(3);
        assert!(
            machine
                .record_macro(&ordinary, [3, 2, 66, 0, 0, 0, 0, 0], "")
                .unwrap()
                .is_none()
        );
        assert!(!machine.state.macros.banks.contains_key(&0));
        assert!(machine.state.macros.banks.contains_key(&5));
    }
    #[test]
    fn macro_bank_overflow_is_visible_before_consuming_eleventh_command() {
        let mut machine = Machine::new("test");
        let toggle = command(0xb0, [0; 8]).instruction(3);
        machine.record_macro(&toggle, [0; 8], "").unwrap();
        let i = command(0x82, [3, 2, 1, 0, 0, 0, 0, 0]).instruction(6);
        for _ in 0..10 {
            machine
                .record_macro(&i, [3, 2, 1, 0, 0, 0, 0, 0], "")
                .unwrap();
        }
        let count = machine.state.instructions;
        assert!(
            machine
                .record_macro(&i, [3, 2, 1, 0, 0, 0, 0, 0], "")
                .is_err()
        );
        assert_eq!(machine.state.instructions, count);
        assert_eq!(machine.state.macros.banks[&0].len(), 10);
    }
}
