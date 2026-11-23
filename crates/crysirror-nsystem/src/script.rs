// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! N-System `MSCENARIO FILE` scripts (HANANYU.EXE `CSnrBuffer`, 0x442540..).
//!
//! - `CSnrBuffer::Read` 0x442B60: a file not starting with the magic is
//!   decrypted in 32-byte blocks, block `n` XORed with the lower-case hex
//!   MD5 digest of `<window title><n>` (CP932, `%s%d`); the result must then
//!   start with the magic. The title is the build's `lpWindowName`.
//! - Layout: magic (16), version u32 @0x10, label count u32 @0x14, 256
//!   load-point table counts u32 @0x18, header end 0x458; then the label
//!   table, the 256 load-point tables (u32 code offsets each), then code.
//! - Commands (`GetCmdID`/`NextCommand`): u16 id, u16 operand byte length,
//!   operands. Operand tags (`NextParam` 0x442980, `GetValueParam` 0x4427C0,
//!   `GetStringParam` 0x4428D0): 1 int32, 2 numeric variable index, 3 inline
//!   CP932 string with `&&` → `&` and `&` + u16 string-variable index, 4 RPN
//!   (op,value) pairs ended by op −1 (0 push value, 1 push variable, 2 +,
//!   3 −, 4 ×, 5 ÷, 6 %), 5 string-variable index.
use anyhow::{Context, Result, bail, ensure};
use std::sync::Arc;

pub const MAGIC: &[u8; 16] = b"MSCENARIO FILE  ";
const HEADER: usize = 0x458;
pub const MAX_SCRIPT_BYTES: usize = 8 * 1024 * 1024;

/// The CP932 bytes of a UTF-8 window title, as `wsprintfA("%s%d")` uses them.
pub fn title_key(title: &str) -> Vec<u8> {
    encoding_rs::SHIFT_JIS.encode(title).0.into_owned()
}
/// Decrypt in place when the plain magic is absent (0x442B60).
pub fn decrypt(bytes: &mut [u8], title_cp932: &[u8]) -> Result<()> {
    if bytes.starts_with(MAGIC) {
        return Ok(());
    }
    for (block, chunk) in bytes.chunks_mut(32).enumerate() {
        let mut key = title_cp932.to_vec();
        key.extend(block.to_string().as_bytes());
        use md5::Digest;
        let hex: String = md5::Md5::digest(&key).iter().map(|b| format!("{b:02x}")).collect();
        for (b, k) in chunk.iter_mut().zip(hex.as_bytes()) {
            *b ^= k;
        }
    }
    ensure!(bytes.starts_with(MAGIC), "N-System script is not decrypted by the profile title");
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Int(i32),
    Var(u32),
    Text(Vec<TextPart>),
    Expr(Vec<(i32, i32)>),
    StrVar(u32),
}
#[derive(Clone, Debug, PartialEq)]
pub enum TextPart {
    Bytes(Vec<u8>),
    StrVar(u16),
}
impl Operand {
    pub fn is_string(&self) -> bool {
        matches!(self, Self::Text(_) | Self::StrVar(_))
    }
}
#[derive(Clone, Debug)]
pub struct Command {
    /// Byte offset from the code start.
    pub pc: u32,
    pub id: u16,
    pub operands: Vec<Operand>,
}
#[derive(Debug)]
pub struct Script {
    pub name: String,
    pub labels: Vec<u32>,
    pub load_points: Vec<Vec<u32>>,
    pub commands: Vec<Command>,
    /// Code offset → command index.
    index: std::collections::BTreeMap<u32, usize>,
    code_len: u32,
}
fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(at..at + 4).context("N-System script truncated")?.try_into()?))
}
fn operand(b: &[u8], mut i: usize, end: usize) -> Result<(Operand, usize)> {
    let tag = *b.get(i).context("N-System operand tag")?;
    i += 1;
    let word = |i: usize| -> Result<i32> {
        ensure!(i + 4 <= end, "N-System operand overruns its command");
        Ok(u32_at(b, i)? as i32)
    };
    Ok(match tag {
        1 => (Operand::Int(word(i)?), i + 4),
        2 => (Operand::Var(word(i)? as u32), i + 4),
        5 => (Operand::StrVar(word(i)? as u32), i + 4),
        3 => {
            let rel = b[i..end].iter().position(|c| *c == 0).context("N-System unterminated string")?;
            let raw = &b[i..i + rel];
            let mut parts = Vec::new();
            let mut text = Vec::new();
            let mut k = 0;
            while k < raw.len() {
                if raw[k] == b'&' {
                    if raw.get(k + 1) == Some(&b'&') {
                        text.push(b'&');
                        k += 2;
                        continue;
                    }
                    let index = u16::from_le_bytes(raw.get(k + 1..k + 3).context("N-System string variable")?.try_into()?);
                    if !text.is_empty() {
                        parts.push(TextPart::Bytes(std::mem::take(&mut text)));
                    }
                    parts.push(TextPart::StrVar(index & 0x7FFF));
                    k += 3;
                    continue;
                }
                text.push(raw[k]);
                k += 1;
            }
            if !text.is_empty() || parts.is_empty() {
                parts.push(TextPart::Bytes(text));
            }
            (Operand::Text(parts), i + rel + 1)
        }
        4 => {
            let mut pairs = Vec::new();
            loop {
                let op = word(i)?;
                if op == -1 {
                    break (Operand::Expr(pairs), i + 8);
                }
                ensure!(pairs.len() < 256, "N-System expression length");
                pairs.push((op, word(i + 4)?));
                i += 8;
            }
        }
        other => bail!("N-System operand tag {other} at byte {i:#x}"),
    })
}
impl Script {
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Arc<Self>> {
        ensure!(bytes.len() <= MAX_SCRIPT_BYTES, "N-System script size budget");
        ensure!(bytes.starts_with(MAGIC) && bytes.len() >= HEADER, "N-System script header");
        let label_count = u32_at(bytes, 0x14)? as usize;
        ensure!(label_count <= 65536, "N-System label count");
        let mut at = HEADER;
        let mut labels = Vec::with_capacity(label_count);
        for _ in 0..label_count {
            labels.push(u32_at(bytes, at)?);
            at += 4;
        }
        let mut load_points = Vec::with_capacity(256);
        let mut counts = Vec::with_capacity(256);
        for t in 0..256 {
            counts.push(u32_at(bytes, 0x18 + 4 * t)? as usize);
        }
        for count in counts {
            ensure!(count <= 65536, "N-System load-point count");
            let mut table = Vec::with_capacity(count);
            for _ in 0..count {
                table.push(u32_at(bytes, at)?);
                at += 4;
            }
            load_points.push(table);
        }
        let code = &bytes[at..];
        let mut commands = Vec::new();
        let mut index = std::collections::BTreeMap::new();
        let mut i = 0usize;
        while i < code.len() {
            ensure!(i + 4 <= code.len(), "N-System command header truncated");
            let id = u16::from_le_bytes([code[i], code[i + 1]]);
            let len = u16::from_le_bytes([code[i + 2], code[i + 3]]) as usize;
            let end = i + 4 + len;
            ensure!(end <= code.len(), "N-System command overruns script");
            let mut operands = Vec::new();
            let mut j = i + 4;
            while j < end {
                let (o, next) = operand(code, j, end).with_context(|| format!("{name} command at {i:#x}"))?;
                operands.push(o);
                j = next;
            }
            ensure!(j == end, "N-System operands overrun command at {i:#x}");
            index.insert(i as u32, commands.len());
            commands.push(Command { pc: i as u32, id, operands });
            i = end;
        }
        for l in labels.iter().chain(load_points.iter().flatten()) {
            ensure!(index.contains_key(l) || *l as usize == code.len(), "N-System jump target {l:#x} is not a command");
        }
        Ok(Arc::new(Self { name: name.into(), labels, load_points, commands, index, code_len: code.len() as u32 }))
    }
    /// Command index at a code offset (the end of code maps past the last).
    pub fn at(&self, offset: u32) -> Result<usize> {
        match self.index.get(&offset) {
            Some(i) => Ok(*i),
            None if offset == self.code_len => Ok(self.commands.len()),
            None => bail!("N-System code offset {offset:#x} is not a command"),
        }
    }
    pub fn label(&self, n: i32) -> Result<usize> {
        let offset = *self.labels.get(usize::try_from(n).ok().context("N-System negative label")?).context("N-System label index")?;
        self.at(offset)
    }
    /// `GetSetupParam` 0x442F60: the operands after `(screen, key)` of the
    /// first id-0 record matching both, as used by native screens.
    pub fn setup(&self, screen: i32, key: i32) -> Option<&[Operand]> {
        self.commands.iter().find_map(|c| {
            (c.id == 0
                && c.operands.len() >= 2
                && c.operands[0] == Operand::Int(screen)
                && c.operands[1] == Operand::Int(key))
                .then(|| &c.operands[2..])
        })
    }
}
