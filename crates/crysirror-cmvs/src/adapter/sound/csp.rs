// SPDX-License-Identifier: MPL-2.0
//! The block at CVNS +2768. The original reserves 52 entries but restores
//! audio for the first 48 only. Device channels 60..63 bypass this block.
use super::{EffectSource, Effects, Music};
use crate::vm::save_slot::csp1::append;
use anyhow::{Result, ensure};

const COUNT: usize = 52;
const NAMES: usize = COUNT * 64;

#[derive(Clone)]
pub(super) struct SoundTable(Box<[u8; 4368]>);

impl Default for SoundTable {
    fn default() -> Self {
        Self(Box::new([0; 4368]))
    }
}

impl SoundTable {
    pub(super) fn word(&mut self, column: usize, slot: usize, value: u32) {
        if slot < 48 {
            let at = NAMES + 4 * (column * COUNT + slot);
            self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    pub(super) fn clear_name(&mut self, slot: usize) {
        if slot < 48 {
            self.0[slot * 64] = 0;
        }
    }

    pub(super) fn load(&mut self, slot: usize, source: &EffectSource) {
        if slot >= 48 {
            return;
        }
        let name = encoding_rs::SHIFT_JIS.encode(&source.name).0;
        // The loader copies the terminated name and retains bytes beyond it.
        let dst = &mut self.0[slot * 64..(slot + 1) * 64];
        let n = name.len().min(dst.len());
        dst[..n].copy_from_slice(&name[..n]);
        if n < dst.len() {
            dst[n] = 0;
        }
        for (column, v) in [
            u32::from(source.looping),
            source.volume as u32,
            100,
            0,
            u32::from(source.started),
        ]
        .into_iter()
        .enumerate()
        {
            self.word(column, slot, v);
        }
    }
}

impl Effects {
    pub(in crate::adapter) fn csp_record(&self, out: &mut Vec<u8>) -> Result<()> {
        let table = self
            .cvns_table
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CSP1 needs a CVNS effect table"))?;
        ensure!(
            table.0[..NAMES]
                .chunks_exact(64)
                .all(|name| name.contains(&0)),
            "CVNS effect name exceeds CSP1 sound slot"
        );
        append(out, 0x301, &[], table.0.as_slice())
    }

    pub(in crate::adapter) fn stop_all(
        &mut self,
        host: &mut dyn crysirror_core::service::HostServices,
        client: crysirror_core::state::ClientId,
    ) -> Result<()> {
        for channel in 0..self.count().min(48) {
            // 0x42a240 clears the name only; preserve the start flag.
            let at = NAMES + 4 * (4 * COUNT + channel);
            let started = self
                .cvns_table
                .as_ref()
                .map(|t| u32::from_le_bytes(t.0[at..at + 4].try_into().unwrap()));
            self.stop(host, client, channel)?;
            if let (Some(table), Some(started)) = (&mut self.cvns_table, started) {
                table.word(4, channel, started);
            }
        }
        Ok(())
    }
}

impl Music {
    pub(in crate::adapter) fn csp_record(&self, out: &mut Vec<u8>) -> Result<()> {
        let name = self
            .retained
            .as_ref()
            .map_or("", |request| request.first.as_str());
        let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(name);
        ensure!(!invalid, "CSP1 music name is not CP932");
        let mut bytes = encoded.into_owned();
        bytes.push(0);
        append(out, 0x300, &[], &bytes)
    }
}

impl Effects {
    pub(in crate::adapter) fn restore_csp_table(&mut self, bytes: &[u8]) -> Result<()> {
        let raw: &[u8; 4368] = bytes.try_into()?;
        for slot in 0..48 {
            let name = &raw[slot * 64..(slot + 1) * 64];
            let end = name
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| anyhow::anyhow!("unterminated CSP1 effect name"))?;
            if end == 0 {
                continue;
            }
            let (name, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&name[..end]);
            ensure!(!invalid, "invalid CSP1 sound name");
            let word = |col: usize| {
                let at = NAMES + 4 * (col * COUNT + slot);
                u32::from_le_bytes(raw[at..at + 4].try_into().unwrap())
            };
            let source = EffectSource {
                name: name.into_owned(),
                looping: word(0) != 0,
                volume: word(1) as i32,
                started: word(4) != 0,
            };
            self.volume_words[slot] = source.volume;
            self.sources[slot] = Some(source);
            self.mix_words[slot] = super::EffectMix {
                multiplier: word(2),
                pan: word(3) as i32,
            };
        }
        self.cvns_table = Some(SoundTable(Box::new(*raw)));
        Ok(())
    }
}

impl Music {
    pub(in crate::adapter) fn restore_csp_name(&mut self, bytes: &[u8]) -> Result<()> {
        let end = bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("unterminated CSP1 music name"))?;
        if end != 0 {
            let (name, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes[..end]);
            ensure!(!invalid, "invalid CSP1 music name");
            let request = super::MusicRequest {
                restored: true,
                first: name.into_owned(),
                second: None,
                duration: 0,
                looping: true,
                words: 0,
            };
            if self.settings.enabled {
                self.commit_reopen(request)
            } else {
                self.retained = Some(request);
            }
        }
        Ok(())
    }
}
