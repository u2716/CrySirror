// Copyright (c) 2026 CrySirror contributors. MIT.
//! Reviewed original syscall policy. Shared Canvas owns page lifetime; the
//! engine clock owns waits. No original device, renderer or timer is hosted.
use super::*;

pub(super) struct NativeWait {
    pub pc: u32,
    pub deadline: Option<u64>,
}
impl MajiroVm {
    pub(super) fn native_wait(
        &mut self,
        pc: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(arguments.len() == 1, "Majiro wait ABI");
        let duration = arguments[0].integer()?;
        //410060/CF35F0E3: -2 ends this native update and resumes on the next;
        //other negative values park this native frame indefinitely. Callback
        //frames may run above it, and thread teardown cancels the receipt.
        ensure!(
            duration <= 43_200_000,
            "Majiro long native wait profile is not connected"
        );
        if let Some(wait) = &self.wait {
            ensure!(
                wait.pc == pc,
                "Majiro wait receipt belongs to another instruction"
            );
            if wait
                .deadline
                .is_none_or(|deadline| host.now_ms() < deadline)
            {
                return Ok(None);
            }
            self.wait = None;
            return Ok(Some(if returns {
                vec![Value::Integer(0)]
            } else {
                vec![]
            }));
        }
        let deadline = if duration < 0 && duration != -2 {
            None
        } else {
            Some(
                host.now_ms()
                    .checked_add(duration.max(0) as u64)
                    .context("Majiro native wait clock overflow")?,
            )
        };
        self.wait = Some(NativeWait { pc, deadline });
        //Even zero/-2 yields once. The pending Native event retains all
        //arguments/PC until an actual subsequent owner update observes expiry.
        Ok(None)
    }
}
