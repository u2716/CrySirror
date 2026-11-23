// Copyright (c) 2026 CrySirror contributors. MIT.
//! YUKIIRO447B10/448050/448890/449130/4499D0/44A570 (classic
//!438170/4387C0/439250/439D00/43A7B0/43B260) WaitFirstPass semantics.
//! The function's C return1 asks the VM to retry this same native operation;
//! it is not the script result. Only completion pushes script Integer0.
use super::*;
use crysirror_core::service::AudioRequest;

impl MajiroVm {
    pub(super) fn native_audio_wait(
        &mut self,
        machine: &mut Machine,
        hash: u32,
        host: &mut dyn HostServices,
    ) -> Result<Option<syscalls::Out>> {
        let channel = match hash {
            0xd8279a9b => syscalls::CH_MUSIC,
            0xd1f672c7 => syscalls::CH_VOICE,
            0x5785a054 => syscalls::CH_SOUND,
            0x804ff66d => syscalls::CH_SOUND + 1,
            0x57ad7635 => syscalls::CH_SOUND + 2,
            0x1d93fd7e => syscalls::CH_SOUND + 3,
            _ => return Ok(None),
        };
        //These handlers do not read argc/operands. The native dispatcher and
        //Machine::acknowledge consume the declared arguments on completion.
        let key = machine.context_id();
        let now = self.native_now() as i32;
        if self.timed_waits.get(&key).is_some_and(|deadline| {
            (now as u32) < (*deadline as u32)
                && now.wrapping_sub(*deadline).wrapping_abs() <= 43_200_000
        }) {
            return Ok(Some(syscalls::Out::Wait));
        }
        //GetPlayState precedes the fast predicate in the original. A loop
        //reports-1 after its first endpoint and therefore no longer blocks.
        let state = self.native_audio_state(host, channel)?;
        self.refresh_fast_read_flag(machine.native_read_flag());
        if self.fast_skip_active(host)? {
            //Stop(0) has no native side effects. Only a retained native
            //instance may control this client-scoped shared channel.
            if self.audio_names.contains_key(&channel) {
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel,
                        fade_ms: 0.0,
                    },
                )
                .map_err(core_error)?;
                self.api_calls += 1;
            }
            self.audio_names.remove(&channel);
        } else if state == 1 {
            //The pending opcode, argument receipt and callback ownership stay
            //unchanged. The VM may run other threads during this10ms gate.
            self.timed_waits.insert(key, now.wrapping_add(10));
            return Ok(Some(syscalls::Out::Wait));
        }
        self.timed_waits.remove(&key);
        Ok(Some(syscalls::Out::Value(Value::Integer(0))))
    }
}
