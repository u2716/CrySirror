//! DC3WY4233A0 case10 and40C480 script-bank reset. Preferences/read marks
//! stay persisted; the live protagonist copies are independent native data.
use super::*;
impl Machine {
    pub(super) fn execute_reset(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x10 {
            return Ok(None);
        }
        let mut data = self.state.system_data.clone();
        let mut names = data.names()?;
        let defaults = crate::dc3_settings::SystemData::default().names()?;
        for index in 0..2 {
            if names[index].is_empty() {
                names[index] = defaults[index].clone();
            }
        }
        data.set_names(&names)?;
        let mut history = self.state.history.clone();
        history.reset()?;
        if a[0] == 0 {
            ensure!(
                self.state.script_bytes.len() == 1100 && history.mirror.len() == 1100,
                "DC3WY reset bank extent"
            );
            self.state.script_bytes.fill(0);
            history.mirror.fill(0);
        }
        self.state.system_data = data;
        self.state.protagonist_names = names;
        self.state.history = history;
        self.state.runtime_values.insert(233, 0);
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}
