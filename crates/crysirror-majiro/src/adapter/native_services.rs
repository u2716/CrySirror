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
        _host: &mut dyn HostServices,
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
            if wait.deadline.is_none_or(|deadline| {
                let (now, deadline) = (self.native_now(), deadline as u32);
                now < deadline && (now.wrapping_sub(deadline) as i32).wrapping_abs() <= 43_200_000
            }) {
                return Ok(None);
            }
            self.wait = None;
            self.clock.frozen = false;
            return Ok(Some(if returns {
                vec![Value::Integer(0)]
            } else {
                vec![]
            }));
        }
        let deadline = if duration < 0 && duration != -2 {
            None
        } else {
            Some(u64::from(
                self.native_now().wrapping_add(duration.max(0) as u32),
            ))
        };
        //416430 caches ENV NOUPDATEONWAIT(default0) through42D570.
        //-2 freezes the published tick unconditionally; zero does so only
        //when requested. Completion clears the flag, and the owner has its
        //500ms escape if the pending call is abandoned by native control.
        if duration == -2
            || (duration == 0
                && self
                    .settings
                    .as_ref()
                    .is_some_and(|settings| settings.no_update_on_wait != 0))
        {
            self.clock.frozen = true;
        }
        self.wait = Some(NativeWait { pc, deadline });
        //Even zero/-2 yields once. The pending Native event retains all
        //arguments/PC until an actual subsequent owner update observes expiry.
        Ok(None)
    }
}
