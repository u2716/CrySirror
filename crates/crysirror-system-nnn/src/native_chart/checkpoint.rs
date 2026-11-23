// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original chartN/slotN.dat: GAMESTATUS plus separately length-prefixed
//! zero-word, zero-byte and all-one-word run streams. Generic expansion lives
//! in Resource; these sections are not ordinary dataN.sav blocks.
use super::*;
use crate::native_save::SaveBlock;
use crysirror_resource::sentinel_runs;
#[derive(Clone, Debug)]
pub(crate) struct ChartCheckpoint {
    pub status: SaveBlock,
    pub variables: Vec<i32>,
    pub state: ChartState,
}
fn section<'a>(bytes: &'a [u8], at: &mut usize) -> Result<&'a [u8]> {
    let head = bytes
        .get(*at..*at + 4)
        .context("NNN chart checkpoint truncated section length")?;
    let size = u32::from_le_bytes(head.try_into().unwrap()) as usize;
    *at += 4;
    let end = at
        .checked_add(size)
        .context("NNN chart checkpoint section extent overflow")?;
    let data = bytes
        .get(*at..end)
        .context("NNN chart checkpoint truncated section")?;
    *at = end;
    Ok(data)
}
impl ChartCheckpoint {
    pub fn prepare_session(
        &self,
        configuration: &crate::native_bootstrap::NativeConfiguration,
        current: &crate::expression::Variables,
        loader: &crate::adapter::ScriptLoader,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<crate::native_control::ScriptSession> {
        let mut variables = current.clone();
        ensure!(
            self.variables.len() == 1000 && variables.stored_word_count() == 1000,
            "NNN chart checkpoint variable ABI"
        );
        for index in (0..200).chain(300..1000) {
            variables.write_stored(index as u16, self.variables[index]);
        }
        variables.finish_native_instruction();
        crate::native_save::NativeSaveFile::prepare_status_contexts(
            &self.status,
            configuration,
            variables,
            |name, cancel| loader(name, cancel),
            cancel,
        )
    }
    pub fn parse(
        bytes: &[u8],
        layout: ChartLayout,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        ensure!(
            bytes.len() >= 7088 && bytes.len() <= 64 << 20,
            "NNN chart checkpoint file extent"
        );
        let size = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let code = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        ensure!(
            size == 7088 && code == 2 && bytes[8..16] == [0; 8],
            "NNN chart checkpoint GAMESTATUS header"
        );
        ensure!(
            bytes[16..32].split(|b| *b == 0).next() == Some(b"GAMESTATUS"),
            "NNN chart checkpoint GAMESTATUS label"
        );
        let status = SaveBlock {
            code,
            label: bytes[16..32].try_into().unwrap(),
            payload: bytes[32..7088].to_vec(),
        };
        let mut at = 7088;
        let variables =
            sentinel_runs::decode_words(section(bytes, &mut at)?, 0, 1000, 4096, check)?;
        let roads = sentinel_runs::decode_bytes(
            section(bytes, &mut at)?,
            0,
            layout.road_bytes,
            64 << 20,
            check,
        )?;
        let permissions = sentinel_runs::decode_words(
            section(bytes, &mut at)?,
            u32::MAX,
            layout.selection_words,
            64 << 20,
            check,
        )?;
        let mut state = ChartState::fresh(layout);
        let mut ext = permissions;
        ext.extend(roads);
        state.load(&ext)?;
        Ok(Self {
            status,
            variables: variables
                .chunks_exact(4)
                .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
                .collect(),
            state,
        })
    }
    pub fn encode(&self, check: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
        ensure!(
            self.status.code == 2
                && self.status.payload.len() == 7056
                && self.variables.len() == 1000,
            "NNN chart checkpoint native status/variable layout"
        );
        let mut out = Vec::new();
        out.extend(7088u32.to_le_bytes());
        out.extend(2u32.to_le_bytes());
        out.extend([0; 8]);
        out.extend(self.status.label);
        out.extend(&self.status.payload);
        let variables: Vec<_> = self
            .variables
            .iter()
            .copied()
            .flat_map(i32::to_le_bytes)
            .collect();
        let ext = self.state.store();
        let split = self.state.layout.selection_words * 4;
        for packed in [
            sentinel_runs::encode_words(&variables, 0, check)?,
            sentinel_runs::encode_bytes(&ext[split..], 0, check)?,
            sentinel_runs::encode_words(&ext[..split], u32::MAX, check)?,
        ] {
            out.extend(u32::try_from(packed.len())?.to_le_bytes());
            out.extend(packed);
        }
        ensure!(out.len() <= 64 << 20, "NNN chart checkpoint encoded budget");
        Ok(out)
    }
    pub fn path(&self, slot: i32, block_size: usize) -> Result<String> {
        let index = usize::try_from(slot)?;
        ensure!(
            block_size > 0 && index < self.state.layout.slots,
            "NNN chart checkpoint slot bounds"
        );
        Ok(format!("chart{}/slot{slot}.dat", index / block_size + 1))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chart_checkpoint_uses_original_three_run_streams_and_checks_lengths() {
        let layout = ChartLayout::read(&Names { tokens: vec![] }).unwrap();
        let checkpoint = ChartCheckpoint {
            status: SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056]).unwrap(),
            variables: vec![0; 1000],
            state: ChartState::fresh(layout),
        };
        let bytes = checkpoint.encode(&|| Ok(())).unwrap();
        assert_eq!(bytes.len(), 7124);
        assert_eq!(&bytes[7088..7100], &[8, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0]);
        assert_eq!(
            &bytes[7100..7112],
            &[8, 0, 0, 0, 0, 255, 0, 255, 0, 255, 0, 243]
        );
        assert_eq!(
            &bytes[7112..7124],
            &[8, 0, 0, 0, 255, 255, 255, 255, 32, 0, 0, 0]
        );
        let restored = ChartCheckpoint::parse(&bytes, layout, &|| Ok(())).unwrap();
        assert_eq!(restored.variables, vec![0; 1000]);
        assert_eq!(restored.state.store(), checkpoint.state.store());
        assert_eq!(restored.path(1, 1000).unwrap(), "chart1/slot1.dat");
        for cut in [0, 7087, 7091, 7101, 7123] {
            assert!(ChartCheckpoint::parse(&bytes[..cut], layout, &|| Ok(())).is_err());
        }
        let mut invalid = bytes;
        invalid[7096..7100].copy_from_slice(&1001u32.to_le_bytes());
        assert!(ChartCheckpoint::parse(&invalid, layout, &|| Ok(())).is_err());
    }
}
