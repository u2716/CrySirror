//! Pre-instruction fences for native saves. Guest state/encoding stay native;
//! all actual reads and atomic commits are owned by the host capability.
use crate::{NativeOpcode, SysApi, Vm, VmError, VmResult};
#[derive(Debug)]
pub(crate) struct PreparedNativeWrite {
    opcode: NativeOpcode,
    path: String,
    bytes: Vec<u8>,
}
impl Vm {
    pub(crate) fn take_prepared_storage_write(
        &mut self,
        opcode: NativeOpcode,
        path: &str,
    ) -> VmResult<Option<Vec<u8>>> {
        let context = self.thread.thread_id() as u32 as u64;
        let Some(write) = self.prepared_native_write.remove(&context) else {
            return Ok(None);
        };
        if write.opcode != opcode || write.path != path {
            self.prepared_native_write.insert(context, write);
            return Err(VmError::Runtime(
                "native save continuation identity changed".into(),
            ));
        }
        Ok(Some(write.bytes))
    }
    pub(crate) fn prepare_native_storage<A: SysApi>(
        &mut self,
        api: &mut A,
        opcode: NativeOpcode,
    ) -> VmResult<bool> {
        if !api.host_requests_enabled() {
            return Ok(true);
        }
        if opcode.group != 0x80 || !matches!(opcode.id, 0x78..=0x7b | 0x80 | 0x81) {
            return Ok(true);
        }
        let context = self.thread.thread_id() as u32 as u64;
        api.begin_native_storage(context);
        let int_at = |vm: &Self, tail| {
            vm.operand_from_top(tail)
                .map(|at| vm.operand_value(at).as_i32())
                .ok_or(VmError::StackUnderflow)
        };
        let path = match opcode.id {
            0x78 => self.config_slot_path(api, int_at(self, 1)?),
            0x79..=0x7b => self.config_slot_path(api, int_at(self, 0)?),
            0x80 | 0x81 => self.global_data_path(api),
            _ => unreachable!(),
        };
        if matches!(opcode.id, 0x79..=0x7b | 0x80) {
            return api.prepare_file_read(&path);
        }
        if !self.prepared_native_write.contains_key(&context) {
            let bytes = if opcode.id == 0x78 {
                let at = self.operand_from_top(0).ok_or(VmError::StackUnderflow)?;
                let label = self.value_as_string_lossy(self.operand_value(at))?;
                self.config_slot_payload(&label)?
            } else {
                self.global_user_data_payload(api)
            };
            self.prepared_native_write.insert(
                context,
                PreparedNativeWrite {
                    opcode,
                    path: path.clone(),
                    bytes,
                },
            );
        }
        let write = self.prepared_native_write.get(&context).unwrap();
        if write.opcode != opcode || write.path != path {
            return Err(VmError::Runtime(
                "native save instruction changed while its host write was pending".into(),
            ));
        }
        api.prepare_file_write(&path, &write.bytes)
    }
}
