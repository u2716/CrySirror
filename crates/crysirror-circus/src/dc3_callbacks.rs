//!4233A0 case41 and literal4A/5A own the16 native64-byte handler fields.
//! They retain bytecode labels; they are never native function pointers.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Handlers {
    pub labels: Vec<Vec<u8>>,
    pub text_timer_origin: u64,
    pub user_timer_origin: u64,
    pub user_timer_duration: u32,
}
impl Default for Handlers {
    fn default() -> Self {
        Self {
            labels: vec![Vec::new(); 16],
            text_timer_origin: 0,
            user_timer_origin: 0,
            user_timer_duration: 0,
        }
    }
}
impl Machine {
    pub(super) fn execute_handlers(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        now: u64,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        let (slot, clear) = match instruction.opcode {
            0x41 => match a[0] {
                0 => (2, Some(0)),
                1 => (3, Some(1)),
                2 => (0, None),
                3 => (1, None),
                4 => (4, None),
                5 => (5, None),
                6 => (8, None),
                10 => (6, None),
                11 => (7, None),
                12 => (9, None),
                13 => (10, None),
                14 => (11, None),
                15 => (13, None),
                16 => (14, None),
                //The source switch default performs no write at all.
                _ => return self.advance(instruction.next, instruction.opcode).map(Some),
            },
            0x4a => (12, None),
            0x5a => (15, None),
            _ => return Ok(None),
        };
        let raw = program
            .literal_bytes(instruction.offset)
            .context("DC3WY native handler literal")?;
        ensure!(raw.len() < 64, "DC3WY native64-byte handler capacity");
        ensure!(
            self.state.handlers.labels.len() == 16,
            "DC3WY handler table extent"
        );
        if instruction.opcode == 0x41 && a[0] == 16 {
            match host.system(crate::machine::system_ops::SystemRequest::CallbackTimer(
                60000,
            ))? {
                None => return Ok(Some(Outcome::Pending)),
                Some(crate::machine::system_ops::SystemReply::Done) => {}
                Some(_) => bail!("DC3WY callback timer returned a foreign receipt"),
            }
        }
        if let Some(clear) = clear {
            self.state.handlers.labels[clear].clear();
        }
        self.state.handlers.labels[slot] = raw.to_vec();
        if instruction.opcode == 0x41 && a[0] == 6 {
            //Uses the device time source in49D54C+8, not wall-calendar time.
            self.state.handlers.text_timer_origin = now;
        }
        if instruction.opcode == 0x41 && a[0] == 16 {
            self.state.handlers.user_timer_origin = now;
            self.state.handlers.user_timer_duration = 60000;
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}
