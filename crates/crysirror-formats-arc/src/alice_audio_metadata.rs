// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-3.0-or-later. Format reference: xsystem4 a85467d78eb5c7fb6e581d7fe5a6fdaec42b9ef7
// src/audio_meta.c (GPL-2.0-or-later). No VM, decoder/mixer or ambient IO.
//! Bounded AliceSoft BGI music-loop and XI2/WAI wave-routing metadata. These
//! are independent resource catalogues, not executable engine/plugin code.
use anyhow::{ensure, Context, Result};
use crysirror_core::service::AssetReader;
use crysirror_resource::{read_all, DecodeContext};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicMetadataVersion {
    FiveFields,
    SixFields,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicLoopEntry {
    pub number: i32,
    pub loop_count: i32,
    pub loop_start: i32,
    pub loop_end: i32,
    /// Original field, including the old-format -1 default.
    pub mixer_group: i32,
    pub volume_percent: i32,
}
#[derive(Clone, Debug)]
pub struct MusicLoopCatalog {
    pub version: MusicMetadataVersion,
    pub entries: Vec<MusicLoopEntry>,
}
impl MusicLoopCatalog {
    /// Original lookup preserves source order and the first duplicate number.
    pub fn entry(&self, number: i32) -> Option<&MusicLoopEntry> {
        self.entries.iter().find(|entry| entry.number == number)
    }
    pub fn load(source: &dyn AssetReader, cx: &DecodeContext) -> Result<Self> {
        cx.check_cancelled()?;
        let mut bytes = read_all(source, cx.limits.max_index_size.min(8 * 1024 * 1024))?;
        for (index, byte) in bytes.iter_mut().enumerate() {
            if index % 4096 == 0 {
                cx.check_cancelled()?;
            }
            *byte = byte.rotate_left(4);
        }
        // Native fgets-like reader stops after255 bytes even without newline.
        let mut rows = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            cx.check_cancelled()?;
            let end = (at + 255).min(bytes.len());
            let stop = bytes[at..end]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(end, |i| at + i + 1);
            ensure!(rows.len() < 65_536, "BGI row-work budget");
            rows.push(&bytes[at..stop]);
            at = stop;
        }
        let mut entries = Vec::new();
        let mut new_format = false;
        for row in &rows {
            cx.check_cancelled()?;
            let Some(values) = fields(row, 5)? else {
                continue;
            };
            if values[4] != -1 {
                new_format = true;
                break;
            }
            entries.push(MusicLoopEntry {
                number: values[0],
                loop_count: values[1],
                loop_start: values[2],
                loop_end: values[3],
                mixer_group: -1,
                volume_percent: 100,
            });
            ensure!(
                entries.len() <= cx.limits.max_entries,
                "BGI record count budget"
            );
            if entries.len() == 100 {
                break;
            }
        }
        if new_format {
            entries.clear();
            for row in rows {
                cx.check_cancelled()?;
                let Some(values) = fields(row, 6)? else {
                    continue;
                };
                entries.push(MusicLoopEntry {
                    number: values[0],
                    loop_count: values[1],
                    loop_start: values[2],
                    loop_end: values[3],
                    mixer_group: values[4],
                    volume_percent: values[5],
                });
                ensure!(
                    entries.len() <= cx.limits.max_entries,
                    "BGI record count budget"
                );
                if entries.len() == 100 {
                    break;
                }
            }
        }
        Ok(Self {
            version: if new_format {
                MusicMetadataVersion::SixFields
            } else {
                MusicMetadataVersion::FiveFields
            },
            entries,
        })
    }
}
fn fields(bytes: &[u8], count: usize) -> Result<Option<Vec<i32>>> {
    let mut at = 0;
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let negative = bytes.get(at) == Some(&b'-');
        if negative || bytes.get(at) == Some(&b'+') {
            at += 1;
        }
        let start = at;
        let mut value = 0i64;
        while let Some(byte @ b'0'..=b'9') = bytes.get(at) {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(i64::from(byte - b'0')))
                .context("BGI decimal field overflow")?;
            ensure!(
                value <= i64::from(i32::MAX) + i64::from(negative),
                "BGI decimal field outside native i32"
            );
            at += 1;
        }
        if at == start {
            return Ok(None);
        }
        values.push(if negative {
            -value as i32
        } else {
            value as i32
        });
        if index + 1 < count {
            if bytes.get(at) != Some(&b',') {
                return Ok(None);
            }
            at += 1;
        }
    }
    Ok(Some(values))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveRoutingEntry {
    /// One-based native file ID; the header's reserved count is excluded.
    pub number: u32,
    pub mixer_group: i32,
    /// Undocumented fields are retained, never treated as silent capabilities.
    pub words: Vec<i32>,
}
#[derive(Clone, Debug)]
pub struct WaveRoutingCatalog {
    pub version: u32,
    pub entries: Vec<WaveRoutingEntry>,
}
impl WaveRoutingCatalog {
    pub fn entry(&self, number: u32) -> Option<&WaveRoutingEntry> {
        number
            .checked_sub(1)
            .and_then(|i| self.entries.get(i as usize))
    }
    pub fn load(source: &dyn AssetReader, cx: &DecodeContext) -> Result<Self> {
        cx.check_cancelled()?;
        let bytes = read_all(source, cx.limits.max_index_size.min(32 * 1024 * 1024))?;
        ensure!(
            bytes.len() >= 48 && bytes[..4] == *b"XI2\0",
            "invalid WAI/XI2 metadata header"
        );
        let word = |at: usize| i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let count = usize::try_from(word(8).checked_sub(1).context("WAI count underflow")?)?;
        ensure!(
            count <= cx.limits.max_entries && count <= 1_000_000,
            "WAI record count budget"
        );
        let version = u32::try_from(word(12))?;
        let (header, stride) = match version {
            3 => (36, 12),
            4 => (40, 16),
            _ => anyhow::bail!("unsupported WAI version {version}"),
        };
        let end = count
            .checked_mul(stride)
            .and_then(|n| n.checked_add(header))
            .context("WAI table extent overflow")?;
        ensure!(end <= bytes.len(), "truncated WAI record table");
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            if index % 256 == 0 {
                cx.check_cancelled()?;
            }
            let base = header + index * stride;
            let words = (0..stride / 4)
                .map(|field| word(base + field * 4))
                .collect::<Vec<_>>();
            entries.push(WaveRoutingEntry {
                number: index as u32 + 1,
                mixer_group: words[2],
                words,
            });
        }
        cx.check_cancelled()?;
        Ok(Self { version, entries })
    }
}
