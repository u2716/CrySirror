//! Early D.C. System.MES resource directory (DC.EXE406790), separate from
//! #ADV_TXT scenario records and the later binary NAMES.DAT generations.
//! This retains resource policy only; file resolution/decoding is engine-owned.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct Music {
    pub name: String,
    pub native_parameter: i32,
}
#[derive(Clone, Debug, Default)]
pub struct SystemData {
    pub music: BTreeMap<i32, Music>,
    pub sound: BTreeMap<i32, String>,
}
impl SystemData {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 1024 * 1024, "DC1 System.MES byte limit");
        ensure!(
            bytes.get(..11) == Some(b"#SYS_SET\0\n\r"),
            "DC1 System.MES header"
        );
        let mut result = Self::default();
        let mut cursor = 11;
        for _ in 0..crate::MAX_INSTRUCTIONS {
            let opcode = *bytes
                .get(cursor)
                .context("DC1 System.MES missing terminal record")?;
            cursor += 1;
            let body = bytes.get(cursor..).context("DC1 System.MES cursor")?;
            let length = body
                .windows(2)
                .take(1024)
                .position(|part| part == [10, 13])
                .context("DC1 System.MES record delimiter")?;
            ensure!(length < 1023, "DC1 System.MES record limit");
            let record = &body[..length];
            cursor = cursor
                .checked_add(length + 2)
                .context("DC1 System.MES cursor overflow")?;
            let integer = |at: usize| -> Result<i32> {
                Ok(i32::from_le_bytes(
                    record
                        .get(at..at + 4)
                        .context("DC1 System.MES numeric operand")?
                        .try_into()?,
                ))
            };
            let string = |at: usize| -> Result<String> {
                let raw = record.get(at..).context("DC1 System.MES resource name")?;
                let raw = &raw[..raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len())];
                ensure!(raw.len() < 256, "DC1 System.MES256-byte resource field");
                let (name, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(raw);
                ensure!(!error, "DC1 System.MES CP932 resource name");
                Ok(name.into_owned())
            };
            match opcode {
                1 => {
                    //Native music slots occupy50*260 bytes before the256-byte SFX table.
                    let index = integer(0)?;
                    ensure!((0..50).contains(&index), "DC1 native music directory index");
                    let parameter = integer(4)?;
                    //406790 stores -parameter, including native two's-complement wrapping.
                    result.music.insert(
                        index,
                        Music {
                            name: string(8)?,
                            native_parameter: parameter.wrapping_neg(),
                        },
                    );
                }
                2 => {
                    let index = integer(0)?;
                    ensure!(
                        (0..4096).contains(&index),
                        "DC1 native sound resource index limit"
                    );
                    //This portable directory is a bounded map. No unreviewed native table
                    //extent is used as a permission to index an original memory array.
                    result.sound.insert(index, string(4)?);
                }
                3 => return Ok(result),
                _ => {} //406790's literal nested-loop default consumes this record.
            }
        }
        anyhow::bail!("DC1 System.MES instruction budget")
    }
}
