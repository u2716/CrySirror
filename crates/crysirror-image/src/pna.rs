//! WillPlus PNAP layered-image container: a canvas extent and an ordered list
//! of group-open (1), group-close (2) and image (0) records. Each image record
//! carries an id, its canvas offset, extent, opacity and the length of an
//! embedded PNG; the PNG payloads follow the table in record order.
//! Record layout (40 bytes, little endian): kind, id, x, y, width, height,
//! one reserved dword, a double opacity (bytes 28..36), payload length.
//! Parsing returns byte ranges only; PNG decoding stays with the shared
//! decoder registry.
use anyhow::{ensure, Context, Result};
use std::ops::Range;

pub const MAX_PNA_RECORDS: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct PnaPart {
    /// Record index in file order.
    pub index: usize,
    pub id: i32,
    pub offset: [i32; 2],
    pub size: [u32; 2],
    pub opacity: f64,
    /// Group nesting depth of the record.
    pub depth: u32,
    /// Index of the enclosing group-open record, if any.
    pub group: Option<usize>,
    pub png: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pna {
    pub canvas: [u32; 2],
    /// The header dword after the signature (not interpreted).
    pub header: u32,
    pub parts: Vec<PnaPart>,
}

impl Pna {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() >= 20 && &bytes[..4] == b"PNAP", "PNAP signature");
        let dword = |at: usize| -> Result<u32> {
            Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("truncated PNAP header")?.try_into()?))
        };
        let header = dword(4)?;
        let canvas = [dword(8)?, dword(12)?];
        let count = usize::try_from(dword(16)?)?;
        ensure!(count <= MAX_PNA_RECORDS, "PNAP record budget");
        let table_end = 20 + count.checked_mul(40).context("PNAP table extent")?;
        ensure!(table_end <= bytes.len(), "truncated PNAP record table");
        let mut data = table_end;
        let mut parts = Vec::new();
        let mut groups: Vec<usize> = Vec::new();
        for index in 0..count {
            let at = 20 + index * 40;
            let int = |field: usize| -> Result<i32> { Ok(i32::from_le_bytes(bytes[at + field * 4..at + field * 4 + 4].try_into()?)) };
            let kind = int(0)?;
            match kind {
                1 => {
                    groups.push(index);
                    continue;
                }
                2 => {
                    ensure!(groups.pop().is_some(), "PNAP group close without open");
                    continue;
                }
                0 => {}
                other => anyhow::bail!("PNAP record kind {other}"),
            }
            let length = usize::try_from(int(9)?).context("PNAP payload length")?;
            let end = data.checked_add(length).context("PNAP payload extent")?;
            ensure!(end <= bytes.len(), "truncated PNAP payload");
            let width = u32::try_from(int(4)?).context("PNAP part width")?;
            let height = u32::try_from(int(5)?).context("PNAP part height")?;
            let opacity = f64::from_le_bytes(bytes[at + 28..at + 36].try_into()?);
            ensure!(opacity.is_finite(), "PNAP part opacity");
            if length > 0 {
                parts.push(PnaPart {
                    index,
                    id: int(1)?,
                    offset: [int(2)?, int(3)?],
                    size: [width, height],
                    opacity,
                    depth: u32::try_from(groups.len())?,
                    group: groups.last().copied(),
                    png: data..end,
                });
            }
            data = end;
        }
        ensure!(groups.is_empty(), "PNAP group left open");
        Ok(Self { canvas, header, parts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(kind: i32, id: i32, x: i32, y: i32, w: i32, h: i32, len: i32) -> Vec<u8> {
        let mut out = Vec::new();
        for value in [kind, id, x, y, w, h, 0] {
            out.extend(value.to_le_bytes());
        }
        out.extend(if kind == 0 { 1.0f64 } else { 0.0 }.to_le_bytes());
        out.extend(len.to_le_bytes());
        out
    }
    #[test]
    fn groups_and_payload_ranges() {
        let mut bytes = b"PNAP".to_vec();
        for value in [7u32, 100, 50, 4] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(record(1, -1, 0, 0, 0, 0, 0));
        bytes.extend(record(0, 3, 5, 6, 2, 2, 3));
        bytes.extend(record(2, -1, 0, 0, 0, 0, 0));
        bytes.extend(record(0, 1, 0, 0, 1, 1, 2));
        bytes.extend([9, 9, 9, 8, 8]);
        let pna = Pna::parse(&bytes).unwrap();
        assert_eq!(pna.canvas, [100, 50]);
        assert_eq!(pna.parts.len(), 2);
        assert_eq!((pna.parts[0].id, pna.parts[0].offset, pna.parts[0].depth, pna.parts[0].group), (3, [5, 6], 1, Some(0)));
        assert_eq!(&bytes[pna.parts[1].png.clone()], &[8, 8]);
        bytes.truncate(bytes.len() - 1);
        assert!(Pna::parse(&bytes).is_err());
    }
}
