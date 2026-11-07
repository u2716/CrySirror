// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native module and return-address policy. 409CE0 executes a loaded module's
//! entry with remaining arguments; 40AD30 prefers the current module, then
//! traverses newest-loaded modules. 40CEB0 caches each resolved call operand.
use super::*;
pub(super) const MAX_MODULES: usize = 512;
pub(super) struct LinkedModule {
    pub name: String,
    pub program: Arc<Program>,
}
impl Machine {
    /// Original424CB0 resolves `$init@GLOBAL` and executes it before the MJO
    /// header entry. Starting directly at header.entry skips all library and
    /// resource-key initialization. Only the native adapter requests this;
    /// standalone Machine consumers may intentionally execute a single entry.
    pub(crate) fn enter_global_initializer(&mut self) -> Result<()> {
        ensure!(
            self.frames.len() == 1
                && self.stack.is_empty()
                && self.pending.is_none()
                && self.executed == 0,
            "Majiro initializer must precede execution"
        );
        let hash = crate::program::name_hash(b"$init@GLOBAL");
        let target = self
            .program
            .functions
            .get(&hash)
            .copied()
            .context("Majiro startup module has no $init@GLOBAL function")?;
        self.program.instruction(target)?;
        let serial = self.frame_serial()?;
        self.frames.push(Frame {
            serial,
            interrupted: None,
            global_initializer: true,
            return_pc: self.pc,
            return_module: self.active_module,
            stack_base: 0,
            arguments: vec![],
            locals: vec![],
            discard_return: true,
            bsel: Bsel::default(),
        });
        self.pc = target;
        Ok(())
    }
    pub(crate) fn root_name(&mut self, name: String) {
        self.modules[0].name = name;
    }
    pub fn module_name(&self) -> &str {
        &self.modules[self.active_module].name
    }
    pub(crate) fn linked_module(&self, name: &str) -> Option<Arc<Program>> {
        self.modules
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.program.clone())
    }
    pub(super) fn resolve_call(&mut self, hash: u32, pc: u32) -> Result<(usize, u32)> {
        if let Some(target) = self.call_cache.get(&(self.active_module, pc)) {
            return Ok(*target);
        }
        let resolved = self.resolve_function(hash)?;
        ensure!(
            self.call_cache.len() < 65536,
            "Majiro resolved-call cache budget"
        );
        self.call_cache.insert((self.active_module, pc), resolved);
        Ok(resolved)
    }
    pub(super) fn resolve_function(&self, hash: u32) -> Result<(usize, u32)> {
        let local = self
            .program
            .functions
            .get(&hash)
            .copied()
            .map(|target| (self.active_module, target));
        local
            .or_else(|| {
                self.modules
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, module)| {
                        module
                            .program
                            .functions
                            .get(&hash)
                            .copied()
                            .map(|target| (index, target))
                    })
            })
            .with_context(|| format!("Majiro external library function {hash:#010x} is not loaded"))
    }
    /// Commit only a prepared, validated original MJO. No dummy syscall ACK:
    /// the new frame executes the module entry and returns to the pending
    /// syscall's next instruction, preserving the caller module and locals.
    pub(crate) fn enter_library(&mut self, name: String, program: Arc<Program>) -> Result<()> {
        let (event, next, consumed) = self
            .pending
            .as_ref()
            .context("Majiro library receipt absent")?;
        let Event::Native {
            hash,
            arguments: native_arguments,
            returns,
            ..
        } = event
        else {
            bail!("Majiro library receipt mismatch");
        };
        //409A90/409CE0 enter the module entry. 409B40/409C10 load the
        //module, then resolve an integer/encoded-name function in that module
        //before falling back to the newest loaded export provider.
        let named = matches!(*hash, 0x9627ddc3 | 0x0edefb0b | 0xedefb0b0);
        ensure!(
            matches!(*hash, 0x10db0a43 | 0xcd00d280 | 0x9627ddc3 | 0x0edefb0b | 0xedefb0b0),
            "Majiro library native selector"
        );
        let skip = if named { 2 } else { 1 };
        ensure!(
            *consumed >= skip && self.frames.len() < MAX_FRAMES,
            "Majiro library argument/depth bounds"
        );
        let args = self.tail(*consumed)?;
        args.last().unwrap().bytes()?;
        let requested = if named {
            Some(if *hash == 0x9627ddc3 {
                native_arguments[1].integer()? as u32
            } else {
                let bytes = native_arguments[1].bytes()?;
                crate::program::name_hash(
                    &bytes[..bytes
                        .iter()
                        .position(|byte| *byte == 0)
                        .unwrap_or(bytes.len())],
                )
            })
        } else {
            None
        };
        let arguments = args[..args.len() - skip].to_vec();
        let base = self.stack.len() - consumed;
        program.instruction(program.entry)?;
        let existing = self.modules.iter().position(|m| m.name == name);
        let loaded_index = existing.unwrap_or(self.modules.len());
        let target = if let Some(hash) = requested {
            program
                .functions
                .get(&hash)
                .copied()
                .map(|pc| (loaded_index, pc))
                .or_else(|| {
                    self.modules
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(index, module)| {
                            module
                                .program
                                .functions
                                .get(&hash)
                                .copied()
                                .map(|pc| (index, pc))
                        })
                })
                .with_context(|| format!("Majiro loaded function {hash:#010x} is absent"))?
        } else {
            (loaded_index, program.entry)
        };
        if let Some(index) = existing {
            ensure!(
                Arc::ptr_eq(&self.modules[index].program, &program),
                "Majiro module generation changed during linking"
            );
        } else {
            ensure!(
                self.modules.len() < MAX_MODULES,
                "Majiro linked module count budget"
            );
            let bytes = self
                .modules
                .iter()
                .map(|m| u64::from(m.program.code_size))
                .sum::<u64>();
            let instructions = self
                .modules
                .iter()
                .map(|m| m.program.instructions.len())
                .sum::<usize>();
            ensure!(
                bytes + u64::from(program.code_size) <= 64 * 1024 * 1024
                    && instructions + program.instructions.len()
                        <= crate::program::MAX_INSTRUCTIONS,
                "Majiro linked module aggregate budget"
            );
        }
        let return_pc = *next;
        let discard_return = !*returns;
        let serial = self.frame_serial()?;
        let frame = Frame {
            serial,
            interrupted: None,
            global_initializer: false,
            return_pc,
            return_module: self.active_module,
            stack_base: base,
            arguments,
            locals: vec![],
            discard_return,
            bsel: Bsel::default(),
        };
        let index = if let Some(index) = existing {
            index
        } else {
            let index = self.modules.len();
            self.modules.push(LinkedModule {
                name,
                program: program.clone(),
            });
            index
        };
        self.frames.push(frame);
        self.stack.truncate(base);
        self.pending = None;
        debug_assert_eq!(index, loaded_index);
        self.active_module = target.0;
        self.pc = target.1;
        self.program = self.modules[target.0].program.clone();
        self.executed += 1;
        Ok(())
    }
}
