// SPDX-License-Identifier: MPL-2.0
//! CVNS execution records from sub_4258e0 and the detached counterpart of
//! the main-image swap in sub_430fe0 followed by sub_426310.
use super::*;
use crate::vm::{csp_banks, save_slot::csp1};
use anyhow::{Context, Result, ensure};

impl Machine {
    /// Execution records of sub_4258e0. SP is a byte offset; call depth
    /// indexes the 64-word caller-base table with element zero as its root.
    pub(in crate::vm) fn csp_execution(&self, out: &mut Vec<u8>) -> anyhow::Result<()> {
        use crate::vm::save_slot::csp1::append;
        let main = &self.images[&0];
        for (tag, slot) in [(0x101, 0), (0x102, 1)] {
            let name = self
                .images
                .get(&slot)
                .map_or(&[][..], |i| i.name.as_slice());
            let mut text = name.to_vec();
            text.push(0);
            append(out, tag, &[], &text)?;
        }
        for (tag, slot) in [(0x103, 0), (0x104, 1)] {
            let length = self
                .images
                .get(&slot)
                .map_or(0, |i| i.script.expanded_len());
            append(out, tag, &[], &u32::try_from(length)?.to_le_bytes())?;
        }
        for (tag, value) in [
            (0x105, u32::from(self.slot)),
            (0x106, u32::try_from(self.pc)?),
            (0x108, self.repeat_count),
            (0x280, u32::try_from(self.stack.len_bytes())?),
            (0x281, u32::try_from(self.frames.len())?),
        ] {
            append(out, tag, &[], &value.to_le_bytes())?;
        }
        ensure!(
            self.frames.len() <= MAX_CALL_DEPTH,
            "CSP1 frame depth exceeds its table"
        );
        let arguments: Vec<_> = self
            .cvns_arguments
            .iter()
            .copied()
            .flat_map(u32::to_le_bytes)
            .collect();
        append(out, 0x107, &[], &arguments)?;
        let mut frames = [0u8; 256];
        for (i, base) in self.frames.iter().enumerate() {
            frames[4 * (i + 1)..4 * (i + 2)].copy_from_slice(&u32::try_from(*base)?.to_le_bytes());
        }
        append(out, 0x282, &[], &frames)?;
        append(out, 0x283, &[], &self.stack.cvns_image())?;
        let mut registers = [0u8; 28];
        for (id, word) in registers.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *word = self
                .state
                .system_integer(id as u32)
                .unwrap_or(0)
                .to_le_bytes();
        }
        append(out, 0x288, &[], &registers)?;
        append(out, 0x900, &[], &main.data)
    }

    /// The original slot contains the expanded main image. The common image
    /// belongs to the live session and is retained across the main-image swap.
    pub(in crate::vm) fn from_csp_execution(
        &self,
        script: Script,
        records: &[u8],
        strings: u32,
    ) -> Result<Self> {
        ensure!(
            self.profile() == VmProfile::Cvns20040515X86,
            "CSP1 requires the CVNS ABI"
        );
        let mut fields = BTreeMap::new();
        // Store the slices only for the lifetime of the validated checkpoint.
        csp1::visit(records, |tag, _, data| {
            if matches!(tag,0x101..=0x108 | 0x280..=0x283 | 0x288 | 0x2a0..=0x2a2 | 0x900) {
                ensure!(
                    fields.insert(tag, data).is_none(),
                    "duplicate CSP1 execution record {tag:#06x}"
                );
            }
            Ok(())
        })?;
        let field = |tag: u16| {
            fields
                .get(&tag)
                .copied()
                .with_context(|| format!("missing CSP1 execution record {tag:#06x}"))
        };
        let word = |tag| -> Result<u32> { Ok(u32::from_le_bytes(field(tag)?.try_into()?)) };
        let name = field(0x101)?
            .strip_suffix(&[0])
            .context("unterminated CSP1 main name")?;
        let PreparedScript(mut main) = PreparedScript::new(self.profile(), script, name.to_vec())?;
        let data = field(0x900)?;
        ensure!(
            data.len() == main.data.len(),
            "CSP1 main data size disagrees with PSS0"
        );
        main.data.copy_from_slice(data);
        let mut staged = self.clone();
        staged.images.insert(0, main);
        staged.slot = u8::try_from(word(0x105)?)?;
        staged.pc = word(0x106)? as usize;
        ensure!(
            staged.slot <= 1 && staged.is_boundary(staged.slot, staged.pc),
            "CSP1 resume address outside the loaded scripts"
        );
        // sub_426310 imports the logical names but keeps the live common code.
        if let Some(common) = staged.images.get_mut(&1) {
            let name = field(0x102)?
                .strip_suffix(&[0])
                .context("unterminated CSP1 common name")?;
            common.name = name.to_vec();
        }
        let sp = word(0x280)? as usize;
        staged.stack = ParameterStack::from_csp_image(field(0x283)?, sp)?;
        let depth = word(0x281)? as usize;
        ensure!(
            depth <= MAX_CALL_DEPTH,
            "CSP1 call depth exceeds the frame table"
        );
        let raw = field(0x282)?.as_chunks::<4>().0;
        ensure!(raw[0] == [0; 4], "invalid CSP1 root frame");
        staged.frames = raw[1..1 + depth]
            .iter()
            .map(|v| u32::from_le_bytes(*v) as usize)
            .collect();
        ensure!(
            staged.frames.is_sorted() && staged.frames.last().is_none_or(|&base| base <= sp),
            "invalid CSP1 call frame"
        );
        for (arg, bytes) in staged
            .cvns_arguments
            .iter_mut()
            .zip(field(0x107)?.as_chunks::<4>().0)
        {
            *arg = u32::from_le_bytes(*bytes);
        }
        staged.pending = None;
        staged.repeat_count = word(0x108)?;
        staged.cross_calls.clear();
        staged.native_return_stack = sp != 0;
        for (id, v) in field(0x288)?.as_chunks::<4>().0.iter().enumerate() {
            staged
                .state
                .set_system_integer(id as u32, u32::from_le_bytes(*v))
                .map_err(anyhow::Error::msg)?;
        }
        for tag in [0x2a0, 0x2a1, 0x2a2] {
            csp_banks::restore(&mut staged.state, strings, tag, field(tag)?)?;
        }
        Ok(staged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script() -> Result<Script> {
        let mut bytes = b"PSS0".to_vec();
        for word in [48u32, 0, 0, 0, 4, 4, 0, 0, 0, 8, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend([0, 0x20, 0, 0x20, 2, 3, 5, 7]);
        Script::decode(bytes)
    }

    fn machine() -> Result<Machine> {
        Ok(Machine::new(PreparedScript::new(
            VmProfile::Cvns20040515X86,
            script()?,
            b"main.pss".to_vec(),
        )?))
    }

    #[test]
    fn detached_execution_resumes_at_saved_pc_with_active_and_retired_stack_bytes() -> Result<()> {
        let mut source = machine()?;
        assert_eq!(source.step()?, Step::Yield);
        source
            .stack
            .push_u32(0xaabbccdd)
            .map_err(|e| anyhow::anyhow!(e.0))?;
        source
            .stack
            .push_u32(0x11223344)
            .map_err(|e| anyhow::anyhow!(e.0))?;
        source.stack.discard(4).map_err(|e| anyhow::anyhow!(e.0))?;
        source
            .state
            .set_system_integer(3, 43)
            .map_err(anyhow::Error::msg)?;
        let mut records = Vec::new();
        source.csp_execution(&mut records)?;
        csp_banks::capture(source.state(), [8, 1, 0], &mut records)?;
        let live = machine()?;
        let mut restored = live.from_csp_execution(script()?, &records, 0)?;
        assert_eq!(restored.pc(), 2);
        assert_eq!(
            restored
                .stack
                .peek_u32(4)
                .map_err(|e| anyhow::anyhow!(e.0))?,
            0xaabbccdd
        );
        assert_eq!(
            &restored.stack.cvns_image()[4..8],
            &0x11223344u32.to_le_bytes()
        );
        assert_eq!(restored.state.system_integer(3), Some(43));
        assert_eq!(restored.step()?, Step::Yield);
        assert_eq!(live.pc(), 0);
        let mut invalid = Vec::new();
        csp1::visit(&records, |tag, indices, data| {
            if tag == 0x106 {
                csp1::append(&mut invalid, tag, indices, &1u32.to_le_bytes())
            } else {
                csp1::append(&mut invalid, tag, indices, data)
            }
        })?;
        assert!(live.from_csp_execution(script()?, &invalid, 0).is_err());
        assert_eq!(live.pc(), 0);
        Ok(())
    }
}
