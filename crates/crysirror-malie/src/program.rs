// SPDX-License-Identifier: MPL-2.0
//! Bounded EXEC sections. Layout reference: crskycode/Malie_Script_Tool
//! 276be31fbf166139e66c625476e6c5035adceffb Script.Read; native interpreter
//! Ragnarok 67A0F0 and message section lookup 677C20. Implemented independently.
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: u32,
    pub size: u32,
    pub offset: u32,
    pub convention: u32,
}
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub index: u32,
    pub kind: u32,
    pub pc: u32,
}
pub struct Program {
    pub symbols: Vec<Symbol>,
    pub functions: Vec<Function>,
    pub labels: BTreeMap<String, u32>,
    pub global_size: u32,
    pub strings: Vec<u8>,
    pub code: Vec<u8>,
    pub message_ranges: Vec<[u32; 2]>,
    pub messages: Vec<u8>,
}
struct Read<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Read<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).context("Malie section arithmetic")?;
        let data = self
            .data
            .get(self.at..end)
            .context("Malie truncated EXEC section")?;
        self.at = end;
        Ok(data)
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn count(&mut self, max: u32) -> Result<usize> {
        let n = self.word()?;
        ensure!(n <= max, "Malie section count budget");
        Ok(n as usize)
    }
    fn name(&mut self) -> Result<String> {
        let tag = self.word()?;
        let n = (tag & 0x7fff_ffff) as usize;
        ensure!((1..=32768).contains(&n), "Malie name extent");
        let data = self.take(n)?;
        if tag >> 31 != 0 {
            ensure!(
                n % 2 == 0 && data.ends_with(&[0, 0]),
                "Malie Unicode name framing"
            );
            Ok(String::from_utf16(
                &data[..n - 2]
                    .chunks_exact(2)
                    .map(|v| u16::from_le_bytes([v[0], v[1]]))
                    .collect::<Vec<_>>(),
            )?)
        } else {
            ensure!(data[n - 1] == 0, "Malie narrow name terminator");
            let (text, _, bad) = encoding_rs::SHIFT_JIS.decode(&data[..n - 1]);
            ensure!(!bad, "Malie name encoding");
            Ok(text.into_owned())
        }
    }
    fn segment(&mut self, max: u32) -> Result<Vec<u8>> {
        let n = self.count(max)?;
        Ok(self.take(n)?.to_vec())
    }
}
impl Program {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 64 * 1024 * 1024, "Malie EXEC byte budget");
        let mut r = Read { data: bytes, at: 0 };
        let mut symbols = Vec::new();
        for _ in 0..r.count(65536)? {
            let name = r.name()?;
            let mut terminated = false;
            for _ in 0..1024 {
                if r.word()? == 0 {
                    terminated = true;
                    break;
                }
                r.word()?;
            }
            ensure!(terminated, "Malie dependency list budget");
            symbols.push(Symbol {
                name,
                kind: r.word()?,
                size: r.word()?,
                offset: r.word()?,
                convention: r.word()?,
            });
        }
        let global_size = r.word()?;
        ensure!(
            global_size <= 16 * 1024 * 1024,
            "Malie global memory budget"
        );
        let mut functions = Vec::new();
        for _ in 0..r.count(65536)? {
            functions.push(Function {
                name: r.name()?,
                index: r.word()?,
                kind: r.word()?,
                pc: r.word()?,
            });
        }
        let mut labels = BTreeMap::new();
        for _ in 0..r.count(1_000_000)? {
            let name = r.name()?;
            let pc = r.word()?;
            ensure!(labels.insert(name, pc).is_none(), "duplicate Malie label");
        }
        let strings = r.segment(32 * 1024 * 1024)?;
        let code = r.segment(32 * 1024 * 1024)?;
        let mut message_ranges = Vec::new();
        for _ in 0..r.count(1_000_000)? {
            message_ranges.push([r.word()?, r.word()?]);
        }
        let messages = r.segment(32 * 1024 * 1024)?;
        ensure!(
            r.at == bytes.len(),
            "Malie EXEC has an unknown trailing layout"
        );
        for &[at, len] in &message_ranges {
            ensure!(
                at % 2 == 0
                    && len % 2 == 0
                    && u64::from(at) + u64::from(len) <= messages.len() as u64,
                "Malie message range"
            );
        }
        ensure!(
            labels.values().all(|pc| (*pc as usize) < code.len()),
            "Malie label outside code"
        );
        for f in &functions {
            ensure!(
                f.kind == 0 || ((f.pc as usize) < code.len()),
                "Malie function outside code"
            );
        }
        Ok(Self {
            symbols,
            functions,
            labels,
            global_size,
            strings,
            code,
            message_ranges,
            messages,
        })
    }
    pub fn message(&self, index: u32) -> Result<String> {
        let [at, len] = *self
            .message_ranges
            .get(index as usize)
            .context("Malie message index")?;
        Ok(String::from_utf16(
            &self.messages[at as usize..(at + len) as usize]
                .chunks_exact(2)
                .map(|v| u16::from_le_bytes([v[0], v[1]]))
                .collect::<Vec<_>>(),
        )?)
    }
}
