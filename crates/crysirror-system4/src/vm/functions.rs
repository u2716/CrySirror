// Native AIN function/message/label policy. GPL-2.0-or-later.
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// xsystem4 a85467d vm.c, plus owner Sys42VM's original OJMP/SJUMP ABI:
// see SOURCE.json. There is no text layout, event loop or hosted interpreter.
use super::*;

#[derive(Clone, Debug)]
pub(super) enum ScenarioDestination {
    Address(u32),
    Function(usize),
}

impl Machine {
    fn native_function_name(&self, name: &[u8]) -> Option<usize> {
        let name = name.split(|b| *b == 0).next().unwrap();
        self.program
            .functions
            .iter()
            .position(|f| f.name.as_slice() == name)
    }

    pub(super) fn execute_functions(
        &mut self,
        instruction: &crate::instruction::Instruction,
    ) -> Result<bool> {
        match instruction.name {
            "MSG" => {
                let function = self.program.message_function;
                if function >= 0 {
                    let function = usize::try_from(function)?;
                    let definition = self
                        .program
                        .functions
                        .get(function)
                        .context("System4 message function index")?;
                    // The actual Sys42VM message entry validates three args;
                    // its reverse temporary vector transfers id/count/string.
                    ensure!(
                        definition.arguments == 3
                            && definition.variables.len() >= 3
                            && definition.variables[0].kind == 10
                            && definition.variables[1].kind == 10
                            && definition.variables[2].kind == 12,
                        "System4 native message function ABI"
                    );
                    ensure!(
                        self.frames.len() < MAX_CALLS && self.stack.len() + 3 <= MAX_STACK,
                        "System4 native message call budget"
                    );
                    let index = usize::try_from(instruction.arguments[0])?;
                    let text = self
                        .program
                        .messages
                        .get(index)
                        .context("System4 message index")?
                        .clone();
                    let slot = self.heap.alloc(Object::String(Arc::new(text)))?;
                    self.stack
                        .extend([index as i32, self.program.messages.len() as i32, slot]);
                    self.call(function, -1, Return::Address(instruction.next))?;
                    return Ok(true);
                }
                // An explicitly negative MSGF means there is no message
                // function; do not draw text or invent a host text receipt.
            }
            "FT_ASSIGNS" => {
                self.peek(0)?; // Compiler-only function-type index.
                let string = self.peek(1)?;
                let name = self.heap.string(string)?;
                let (page, index) = self.reference(2)?;
                let function = self
                    .native_function_name(&name)
                    .filter(|f| *f > 0)
                    .map_or(0, |f| f as i32);
                self.heap.set(page, index, function)?;
                self.consume(4)?;
                // FT_ASSIGNS returns the same owned string slot. Unlike
                // ExistFunc or CALLONJUMP it must not unref that temporary.
                self.push(string)?;
            }
            "CALLONJUMP" => {
                if self.program.scenario_label_section || !self.program.labels.is_empty() {
                    let slot = self.peek(0)?;
                    let text = self.heap.string(slot)?;
                    let name = text.split(|b| *b == 0).next().unwrap();
                    let address = self
                        .program
                        .labels
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, address)| *address)
                        .context("System4 invalid scenario label")?;
                    self.program.instruction(address)?;
                    self.consume(1)?;
                    self.release_string_now(slot)?;
                    self.push(i32::try_from(address)?)?;
                } else {
                    match self.program.object_jump {
                        Some(function) if function >= 0 => {
                            let function = usize::try_from(function)?;
                            let definition = self.program.functions.get(function)
                                .context("System4 OJMP function index")?;
                            ensure!(definition.arguments == 1
                                && definition.variables.first().is_some_and(|v| v.kind == 12),
                                "System4 original OJMP resolver ABI");
                            self.heap.string(self.peek(0)?)?;
                            self.call(function, -1, Return::Address(instruction.next))?;
                            return Ok(true);
                        }
                        Some(_) => {
                            // Original Sys42VM returns before popping the
                            // string when its OJMP index is negative. SJUMP
                            // subsequently consumes that original string.
                        }
                        None => bail!("System4 scenario ABI without SLBL/OJMP metadata is unsupported; no guessed preallocated page"),
                    }
                }
            }
            "SJUMP" => {
                let operand = self.peek(0)?;
                let destination =
                    if self.program.scenario_label_section || !self.program.labels.is_empty() {
                        let address = u32::try_from(operand)?;
                        self.program.instruction(address)?;
                        ScenarioDestination::Address(address)
                    } else {
                        ensure!(
                            self.program.object_jump.is_some(),
                            "System4 scenario ABI without SLBL/OJMP metadata is unsupported"
                        );
                        let name = self.heap.string(operand)?;
                        let function = self
                            .native_function_name(&name)
                            .context("System4 invalid native scenario function")?;
                        let f = &self.program.functions[function];
                        ensure!(
                            f.is_label,
                            "System4 native scenario target is not a label function"
                        );
                        ensure!(
                            f.arguments as usize <= self.stack.len() - 1,
                            "System4 native scenario argument count"
                        );
                        self.program.instruction(f.address)?;
                        ScenarioDestination::Function(function)
                    };
                self.consume(1)?;
                if matches!(destination, ScenarioDestination::Function(_)) {
                    self.release_string_now(operand)?;
                }
                // This is an original non-returning jump. Discard only the
                // abandoned VM continuation policy, then release old frames
                // top-down. Destructors may wait on real engine services.
                self.actions.clear();
                self.actions.push(Action::ScenarioUnwind {
                    destination,
                    call_pc: instruction.pc,
                });
                self.running_actions = true;
                return Ok(true);
            }
            _ => return Ok(false),
        }
        self.pc = instruction.next;
        Ok(true)
    }

    pub(super) fn scenario_unwind(
        &mut self,
        destination: ScenarioDestination,
        call_pc: u32,
    ) -> Result<()> {
        if let Some(frame) = self.frames.last() {
            let slot = frame.local_page;
            self.actions.push(Action::ScenarioUnwind {
                destination,
                call_pc,
            });
            self.actions.push(Action::ScenarioDropFrame);
            self.actions.push(Action::Release(slot));
            return Ok(());
        }
        match destination {
            ScenarioDestination::Address(address) => {
                self.pc = address;
                self.running_actions = false;
            }
            ScenarioDestination::Function(function) => {
                self.pc = call_pc;
                self.actions.push(Action::Finish);
                self.call(function, -1, Return::Actions)?;
                self.running_actions = false;
            }
        }
        Ok(())
    }
}
