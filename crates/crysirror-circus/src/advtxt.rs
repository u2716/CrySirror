//! Original D.C. #ADV_TXT record format, distinct from later offset1/2 MES.
//! DC.EXE 4042A0 reads count WORDs after the 12-byte header and starts at
//! 14 + count*2. 4041B0 reads one opcode and a bounded LF/CR-terminated body;
//! 404030 indexes #ADR records by the absolute position after the delimiter.
//! These are native records, not an alias for the DC4PH bytecode dispatcher.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub offset: u32,
    pub next: u32,
    pub opcode: u8,
    /// Original bytes, including embedded NULs and little-endian integers.
    pub operands: Vec<u8>,
}
impl Record {
    pub fn integer(&self, at: usize) -> Result<i32> {
        let end = at
            .checked_add(4)
            .context("Circus AdvTxt operand overflow")?;
        Ok(i32::from_le_bytes(
            self.operands
                .get(at..end)
                .context("truncated Circus AdvTxt integer")?
                .try_into()?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub code_file_offset: usize,
    /// Native read-history WORDs, including negative/sentinel bit patterns.
    pub read_marks: Vec<i16>,
    /// Native #ADR destinations point after their declaration, not at it.
    pub labels: BTreeMap<i32, u32>,
    pub records: Vec<Record>,
}
impl Program {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= crate::MAX_SCRIPT_BYTES,
            "Circus AdvTxt script byte limit"
        );
        ensure!(
            bytes.get(..8) == Some(b"#ADV_TXT"),
            "unsupported Circus AdvTxt signature"
        );
        let count = i32::from_le_bytes(
            bytes
                .get(8..12)
                .context("truncated Circus AdvTxt header")?
                .try_into()?,
        );
        // DC.EXE 4042A0 owns a 1000-WORD read-history buffer. Refuse its
        // unchecked overflow rather than copying bytes over another field.
        ensure!(
            (0..=1000).contains(&count),
            "Circus AdvTxt read-history count limit"
        );
        let code_file_offset = 14usize
            .checked_add(
                (count as usize)
                    .checked_mul(2)
                    .context("Circus AdvTxt index overflow")?,
            )
            .context("Circus AdvTxt index overflow")?;
        let history = bytes
            .get(12..code_file_offset - 2)
            .context("truncated Circus AdvTxt read history")?;
        ensure!(
            bytes.get(code_file_offset - 2..code_file_offset) == Some(&[10, 13]),
            "Circus AdvTxt header delimiter"
        );
        let read_marks = history
            .chunks_exact(2)
            .map(|v| i16::from_le_bytes([v[0], v[1]]))
            .collect();
        let mut records = Vec::new();
        let mut labels = BTreeMap::new();
        let mut cursor = code_file_offset;
        while cursor < bytes.len() {
            ensure!(
                records.len() < crate::MAX_INSTRUCTIONS,
                "Circus AdvTxt record count limit"
            );
            let opcode = bytes[cursor];
            let body = &bytes[cursor + 1..];
            let length = body
                .windows(2)
                .take(1024)
                .position(|v| v == [10, 13])
                .context("unterminated/oversized Circus AdvTxt record")?;
            ensure!(length < 1023, "Circus AdvTxt record operand limit");
            let next = cursor + 1 + length + 2;
            let operands = body[..length].to_vec();
            if operands.get(..4) == Some(b"#ADR") {
                let value = i32::from_le_bytes(
                    operands
                        .get(4..8)
                        .context("truncated Circus AdvTxt label")?
                        .try_into()?,
                );
                // Native 404030 has 200 label slots. Negative indices are
                // rejected here instead of reproducing its unchecked write.
                ensure!(
                    (0..200).contains(&value),
                    "Circus AdvTxt label outside native table"
                );
                labels.insert(value, next as u32);
            }
            records.push(Record {
                offset: cursor as u32,
                next: next as u32,
                opcode,
                operands,
            });
            cursor = next;
        }
        Ok(Self {
            code_file_offset,
            read_marks,
            labels,
            records,
        })
    }

    pub fn instruction(&self, offset: u32) -> Result<&Record> {
        let index = self
            .records
            .binary_search_by_key(&offset, |record| record.offset)
            .map_err(|_| {
                anyhow::anyhow!("Circus AdvTxt PC {offset:#x} is not a record boundary")
            })?;
        Ok(&self.records[index])
    }
}
