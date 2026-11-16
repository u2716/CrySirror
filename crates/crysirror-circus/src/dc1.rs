//! Original D.C. #ADV_TXT scenario variables (DC.EXE 4124A0/404600). This
//! generation uses record-boundary offsets and thirteen 99-dword flag banks,
//! not the later offset1/2 MES byte banks. The dispatcher lives in dc1_story.
use crate::advtxt::Program;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub scene: String,
    pub pc: u32,
    pub instructions: u64,
    pub flags: Vec<i32>,
    pub affection: [i32; 16],
    pub previous_affection: [i32; 16],
    pub phase: i32,
    pub text_pacing: i32,
}
impl State {
    pub fn new(scene: String, program: &Program) -> Result<Self> {
        Ok(Self {
            scene,
            pc: u32::try_from(program.code_file_offset)?,
            instructions: 0,
            flags: vec![0; 13 * 99],
            affection: [0; 16],
            previous_affection: [0; 16],
            phase: 0,
            text_pacing: 1,
        })
    }
}
///404600's text fields reverse each pair of encoded bytes before passing
///the result to the original CP932 font path. Preserve its first-NUL and odd
///tail behavior using checked bytes; do not reproduce the original256-byte
///destination overflow when the record reader admits1023 operand bytes.
pub fn decode_text_field(operands: &[u8]) -> Result<String> {
    let length = operands
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(operands.len());
    ensure!(
        length < 256,
        "DC1 native text field exceeds256-byte destination"
    );
    let mut output = Vec::with_capacity(length + 1);
    for at in (0..length).step_by(2) {
        output.push(if at + 1 < length { operands[at + 1] } else { 0 });
        output.push(operands[at]);
    }
    let end = output
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(output.len());
    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&output[..end]);
    ensure!(!error, "DC1 swapped CP932 text field");
    Ok(text.into_owned())
}
