// Native VM control/introspection from xsystem4 vm.c a85467d78e (GPL2+) plus
// reviewed original Sys42VM.dll peek-counter ABI (see SOURCE.json).
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
use super::*;
impl Machine {
    pub(crate) fn peek_locked(&self) -> bool {
        self.peek_lock > 0
    }
    pub(crate) fn complete_system_exit(
        &mut self,
        call: &ExternalCall,
        words: usize,
        status: i32,
    ) -> Result<()> {
        self.validate_system(call)?;
        self.complete_system(call, words, None)?;
        self.exit_status = Some(status);
        self.halted = true;
        Ok(())
    }
    pub(super) fn complete_pure_control(&mut self, call: &ExternalCall) -> Result<bool> {
        let ExternalTarget::System(code) = call.target else {
            return Ok(false);
        };
        match code {
            0 => {
                // vm_exit consumes its result, then the session owner runs
                // library teardown. The v5 interpreter never exits the host.
                let status = self.peek(0)?;
                self.complete_system_exit(call, 1, status)?;
            }
            0x11 => {
                // Pinned non-debug host has no connected VM debugger. This is
                // the original IsDebugMode branch, not an unknown-call ACK.
                ensure!(
                    self.stack.len() < MAX_STACK,
                    "System4 debug-query stack budget"
                );
                self.complete_system(call, 0, Some(0))?;
            }
            3 | 4 => {
                self.validate_system(call)?;
                ensure!(
                    self.stack.len() < MAX_STACK,
                    "System4 peek-lock return stack budget"
                );
                let depth = if code == 3 {
                    self.peek_lock
                        .checked_add(1)
                        .filter(|n| *n <= 65536)
                        .context("System4 native peek-lock depth budget")?
                } else {
                    self.peek_lock.saturating_sub(1).max(0)
                };
                self.complete_system(call, 0, Some(depth))?;
                self.peek_lock = depth;
            }
            0x13 => {
                self.validate_system(call)?;
                let index = self.peek(0)?;
                let name = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.frames.len().checked_sub(i + 1))
                    .map(|i| self.program.functions[self.frames[i].function].name.clone())
                    .unwrap_or_default();
                let result = self.heap.alloc(Object::String(Arc::new(name)))?;
                self.complete_system(call, 1, Some(result))?;
            }
            0x1b => {
                self.validate_system(call)?;
                let slot = self.peek(0)?;
                let name = self.heap.string(slot)?;
                let found = self
                    .program
                    .functions
                    .iter()
                    .position(|function| {
                        function.name.as_slice() == name.split(|b| *b == 0).next().unwrap()
                    })
                    .is_some_and(|index| index > 0);
                self.consume(1)?;
                self.release_string_now(slot)?;
                self.push(i32::from(found))?;
                // Retain the system receipt until all original ownership work
                // succeeded; there are no destructors in string-slot cleanup.
                self.pending = None;
                self.pc = call.next;
                self.instructions += 1;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
