// Native stack and heap-lifetime policy researched from xsystem4 vm.c at
// a85467d78eb5c7fb6e581d7fe5a6fdaec42b9ef7, (C) 2019 Nunuhara Cabbage.
// GPL-2.0-or-later; see SOURCE.json. All byte-character/numeric algorithms use
// the shared crysirror-text-legacy service, not a private VM implementation.
use super::*;
use crysirror_text_legacy::{
    ByteNumberSymbols, ByteStringContext, ByteStringLimits, ByteStringProfile, NumericCompaction,
};
impl Machine {
    pub(super) fn execute_strings(
        &mut self,
        instruction: &crate::instruction::Instruction,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<bool> {
        let text = ByteStringContext::new(
            ByteStringProfile::SHIFT_JIS_MASK,
            ByteStringLimits::default(),
            check,
        )?;
        match instruction.name {
            "S_LENGTH" | "S_LENGTH2" => {
                let slot = if instruction.name == "S_LENGTH" {
                    let (page, index) = self.reference(0)?;
                    self.heap.get(page, index)?
                } else {
                    self.peek(0)?
                };
                let string = self.heap.string(slot)?;
                let count = i32::try_from(text.character_count(&string)?)?;
                self.consume(if instruction.name == "S_LENGTH" { 2 } else { 1 })?;
                self.push(count)?;
                if instruction.name == "S_LENGTH2" {
                    self.release_string_now(slot)?
                }
            }
            "S_FIND" => {
                let (needle, source) = (self.peek(0)?, self.peek(1)?);
                let count = text.find(&self.heap.string(source)?, &self.heap.string(needle)?)?;
                self.consume(2)?;
                self.release_string_now(needle)?;
                self.release_string_now(source)?;
                self.push(count)?;
            }
            "S_GETPART" => {
                let (count, index, source) = (self.peek(0)?, self.peek(1)?, self.peek(2)?);
                let bytes = text.substring(&self.heap.string(source)?, index, count)?;
                // Native make_string creates bytes before unref, but the new
                // HEAP SLOT is allocated afterwards and can reuse source's ID.
                self.consume(3)?;
                self.release_string_now(source)?;
                let result = self.heap.alloc(Object::String(Arc::new(bytes)))?;
                self.push(result)?;
            }
            "S_PUSHBACK2" => {
                let (character, slot) = (self.peek(0)?, self.peek(1)?);
                let bytes = text.push_character(&self.heap.string(slot)?, character as u32)?;
                self.heap.set_string(slot, Arc::new(bytes))?;
                self.consume(2)?;
            }
            "S_POPBACK2" => {
                let slot = self.peek(0)?;
                let bytes = text.pop_character(&self.heap.string(slot)?)?;
                self.heap.set_string(slot, Arc::new(bytes))?;
                self.consume(1)?;
            }
            "S_ERASE2" => {
                let (_reserved, index, slot) = (self.peek(0)?, self.peek(1)?, self.peek(2)?);
                let bytes = text.erase_character(&self.heap.string(slot)?, index)?;
                self.heap.set_string(slot, Arc::new(bytes))?;
                self.consume(3)?;
            }
            "C_REF" => {
                let (index, slot) = (self.peek(0)?, self.peek(1)?);
                let value = text.get_character(&self.heap.string(slot)?, index)?;
                self.consume(2)?;
                self.push(value)?;
            }
            "C_ASSIGN" => {
                let (character, index, slot) = (self.peek(0)?, self.peek(1)?, self.peek(2)?);
                let bytes =
                    text.set_character(&self.heap.string(slot)?, index, character as u32)?;
                self.heap.set_string(slot, Arc::new(bytes))?;
                self.consume(3)?;
                self.push(character)?;
            }
            "STOI" => {
                let slot = self.peek(0)?;
                let value = text.decimal_integer(
                    &self.heap.string(slot)?,
                    ByteNumberSymbols::ShiftJis,
                    NumericCompaction::PreserveStoredTail,
                )?;
                self.consume(1)?;
                self.push(value)?;
                self.release_string_now(slot)?;
            }
            "FTOS" => {
                let precision = self.peek(0)?;
                let value = f32::from_bits(self.peek(1)? as u32);
                let bytes = text.fixed_float(value, precision)?;
                let result = self.heap.alloc(Object::String(Arc::new(bytes)))?;
                self.consume(2)?;
                self.push(result)?;
            }
            _ => return Ok(false),
        }
        self.pc = instruction.next;
        Ok(true)
    }
}
