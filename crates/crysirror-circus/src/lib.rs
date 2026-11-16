//! Native Circus MES bytecode/resource policy, independent of rendering/devices.
//! Original instruction semantics are separate from shared engine services.
//! Nothing in this module fabricates successful execution of an unknown opcode.
#![forbid(unsafe_code)]
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
mod adapter;
pub mod advtxt;
pub mod dc1;
pub mod dc1_system;
pub mod dc1_frontend;
pub mod dc1_story;
pub mod dc1_config;
pub mod dc2;
pub mod dc2_bitmap;
pub use dc2::audio as dc2_audio;
pub mod dc2_names;
pub mod dc2_settings;
pub mod dc3;
pub mod dc3_settings;
pub mod dc3_original;
pub mod machine;
pub mod names;
pub mod opcodes;
pub mod settings;
#[cfg(feature = "runtime")]
pub use adapter::register;
pub use adapter::{CircusVm, Config, InstallationPolicy};

pub const MAX_SCRIPT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_INSTRUCTIONS: usize = 1_000_000;
pub const MAX_STRING_BYTES: usize = 8191;

/// Explicit bytecode ABI profiles; file extensions and game names are not ABIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    Dc2PlusCommunication,
    Dc3WithYou,
    ///D.C.III (DC3.exe, MES 0x8a71).
    Dc3,
    Dc4,
    Dc4PlusHarmony,
}
impl Profile {
    fn version(self) -> u16 {
        match self {
            Self::Dc2PlusCommunication => 0x5769,
            Self::Dc3WithYou => 0xa09f,
            Self::Dc3 => 0x8a71,
            Self::Dc4 => 0xaab6,
            Self::Dc4PlusHarmony => 0xabb6,
        }
    }
    fn encoding(self) -> Encoding {
        // MesTextTool script_info at the existing SOURCE.json revision. These
        // are byte layouts, not aliases for another generation's dispatch.
        match self {
            Self::Dc2PlusCommunication => Encoding {
                index_stride: 4,
                prefix: 2,
                bytes_end: 0x28,
                slot_end: 0x2e,
                plain_end: 0x49,
                encrypted_end: 0x4d,
                continuation: None,
            },
            // DC3WY sub_42F910 appends four WORDs when the next token is6d.
            Self::Dc3WithYou => Encoding {
                index_stride: 6,
                prefix: 3,
                bytes_end: 0x38,
                slot_end: 0x41,
                plain_end: 0x5f,
                encrypted_end: 0x63,
                continuation: Some(0x6d),
            },
            //D.C.III (DC3.exe 424FE0 decoder): <0x2b bytes, <0x33 slot
            //string, <0x4e plain, <0x52 +32 string, then four WORDs; a
            //following 0x57 appends four more.
            Self::Dc3 => Encoding {
                index_stride: 6,
                prefix: 3,
                bytes_end: 0x2a,
                slot_end: 0x32,
                plain_end: 0x4d,
                encrypted_end: 0x51,
                continuation: Some(0x57),
            },
            Self::Dc4 | Self::Dc4PlusHarmony => Encoding {
                index_stride: 6,
                prefix: 3,
                bytes_end: 0x3a,
                slot_end: 0x47,
                plain_end: 0x68,
                encrypted_end: 0x6d,
                continuation: Some(0x77),
            },
        }
    }
}
struct Encoding {
    index_stride: usize,
    prefix: usize,
    bytes_end: u8,
    slot_end: u8,
    plain_end: u8,
    encrypted_end: u8,
    continuation: Option<u8>,
}

/// Original encoded arguments. Their native interpretation depends on opcode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arguments {
    Bytes([u8; 2]),
    SlotString { slot: u8, text: String },
    String(String),
    Words([i16; 4]),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Native code offset, including the profile's version prefix.
    pub offset: u32,
    pub next: u32,
    pub opcode: u8,
    pub arguments: Arguments,
    /// Profile-specific four-word extension consumed by the preceding command.
    pub continuation: Option<[i16; 4]>,
}
#[derive(Debug, Clone)]
pub struct Program {
    pub profile: Profile,
    pub code_file_offset: usize,
    /// Original label words, including native high-byte metadata. Resolving
    /// their execution policy is deliberately separate from format validation.
    pub labels: Vec<u32>,
    /// Offset2 auxiliary u16 per label (437FD0/DC4PH, 42F440/DC3WY).
    /// Offset1 DC2 has no auxiliary label table.
    pub label_metadata: Vec<u16>,
    pub instructions: Vec<Instruction>,
    ///Parser-owned, decrypted literal bytes keyed by native instruction offset.
    ///Keep CP932 aliases distinct when native code reinterprets bytes as WCHARs.
    raw_strings: std::collections::BTreeMap<u32, Vec<u8>>,
    ///D.C.III programs rewritten into DC3WY numbering keep each original
    ///opcode by native offset (dc3_original).
    pub original: Option<std::collections::BTreeMap<u32, u8>>,
}
fn word(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .context("truncated Circus word")?
            .try_into()?,
    ))
}
fn long(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .context("truncated Circus header/index")?
            .try_into()?,
    ))
}
fn words(bytes: &[u8], at: usize) -> Result<[i16; 4]> {
    Ok([
        word(bytes, at)? as i16,
        word(bytes, at + 2)? as i16,
        word(bytes, at + 4)? as i16,
        word(bytes, at + 6)? as i16,
    ])
}
fn string_bytes(
    bytes: &[u8],
    at: usize,
    encrypted: bool,
    escape_space: bool,
) -> Result<(String, usize, Vec<u8>)> {
    let rest = bytes.get(at..).context("truncated Circus string")?;
    let end = rest
        .iter()
        .take(MAX_STRING_BYTES + 1)
        .position(|b| *b == 0)
        .context("unterminated/oversized Circus string")?;
    let raw = rest[..end]
        .iter()
        .map(|b| {
            if encrypted {
                let value = b.wrapping_add(0x20);
                if escape_space && value == b'$' {
                    b' '
                } else {
                    value
                }
            } else {
                *b
            }
        })
        .collect::<Vec<_>>();
    let (decoded, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw);
    ensure!(!errors, "invalid Circus Shift-JIS string at {at:#x}");
    Ok((decoded.into_owned(), at + end + 1, raw))
}
#[cfg(test)]
fn string(bytes: &[u8], at: usize, encrypted: bool) -> Result<(String, usize)> {
    let (text, next, _) = string_bytes(bytes, at, encrypted, true)?;
    Ok((text, next))
}
impl Program {
    pub(crate) fn literal_bytes(&self, offset: u32) -> Option<&[u8]> {
        self.raw_strings.get(&offset).map(Vec::as_slice)
    }
    /// Bounded DC2/DC3WY original-code read for scanners which can inspect
    /// operand bytes. Reconstruct from retained exact literals rather than
    /// retaining a second whole script or re-encoding Unicode labels.
    pub(crate) fn native_byte(&self, pc:u32) -> Option<u8> {
        if !matches!(self.profile,Profile::Dc2PlusCommunication|Profile::Dc3WithYou) {return None}
        let encoding=self.profile.encoding();
        if (pc as usize)<encoding.prefix {
            return if pc<2 {Some(self.profile.version().to_le_bytes()[pc as usize])}else{Some(0)};
        }
        let index=self.instructions.partition_point(|instruction|instruction.offset<=pc).checked_sub(1)?;
        let instruction=self.instructions.get(index)?;
        if pc>=instruction.next {return None}
        let relative=(pc-instruction.offset) as usize;
        if relative==0 {return Some(instruction.opcode)}
        let extension=instruction.continuation.map(|_|instruction.next-9);
        if let Some(start)=extension.filter(|start|pc>=*start) {
            if pc==start {return encoding.continuation}
            let relative=(pc-start-1) as usize;return instruction.continuation?.get(relative/2).map(|value|value.to_le_bytes()[relative%2]);
        }
        match &instruction.arguments {
            Arguments::Bytes(values)=>values.get(relative-1).copied(),
            Arguments::Words(values)=>values.get((relative-1)/2).map(|value|value.to_le_bytes()[(relative-1)%2]),
            Arguments::SlotString{slot,..} if relative==1=>Some(*slot),
            Arguments::SlotString{..}|Arguments::String(_)=>{
                let start=if matches!(instruction.arguments,Arguments::SlotString{..}){2}else{1};
                let raw=self.literal_bytes(instruction.offset)?;let index=relative-start;
                if index==raw.len(){return Some(0)}
                raw.get(index).copied().map(|value|if instruction.opcode>encoding.plain_end&&instruction.opcode<=encoding.encrypted_end {value.wrapping_sub(32)}else{value})
            }
        }
    }
    /// Parse explicitly identified offset1/offset2 native layouts. Arithmetic,
    /// allocation, string reads and continuation consumption are bounded.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_SCRIPT_BYTES, "Circus script byte limit");
        let count = long(bytes, 0)? as usize;
        ensure!(count <= 65535, "Circus label count limit");
        let mut candidates = Vec::new();
        for profile in [
            Profile::Dc2PlusCommunication,
            Profile::Dc3WithYou,
            Profile::Dc3,
            Profile::Dc4,
            Profile::Dc4PlusHarmony,
        ] {
            let encoding = profile.encoding();
            let offset = count
                .checked_mul(encoding.index_stride)
                .and_then(|v| v.checked_add(4))
                .context("Circus index overflow")?;
            if encoding.index_stride == 6 && (count == 0 || long(bytes, 4)? != 3) {
                continue;
            }
            if offset
                .checked_add(encoding.prefix)
                .is_some_and(|end| end <= bytes.len())
                && word(bytes, offset)? == profile.version()
            {
                candidates.push((profile, offset));
            }
        }
        ensure!(
            candidates.len() == 1,
            "unsupported or ambiguous Circus MES header/version"
        );
        let (profile, code_file_offset) = candidates[0];
        let encoding = profile.encoding();
        let code = bytes
            .get(code_file_offset..)
            .context("Circus index exceeds script")?;
        if encoding.prefix == 3 {
            ensure!(code.get(2) == Some(&0), "Circus version suffix");
        }
        // The native runtime copies count words beginning at file offset 4:
        // first label 0 is the value 3 also used by format probes as a marker.
        // MesTextTool's editing view starts at 8 and intentionally treats the
        // combined index/metadata span differently; it is not VM indexing.
        let labels = (0..count)
            .map(|i| long(bytes, 4 + i * 4))
            .collect::<Result<Vec<_>>>()?;
        let label_metadata = (0..if encoding.index_stride == 6 { count } else { 0 })
            .map(|i| word(bytes, 4 + 4 * count + 2 * i))
            .collect::<Result<Vec<_>>>()?;
        let mut instructions = Vec::new();
        let mut raw_strings = std::collections::BTreeMap::new();
        let mut pos = encoding.prefix;
        while pos < code.len() {
            ensure!(
                instructions.len() < MAX_INSTRUCTIONS,
                "Circus instruction count limit"
            );
            let start = pos;
            let opcode = code[pos];
            ensure!(
                matches!(profile, Profile::Dc3WithYou | Profile::Dc3) || Some(opcode) != encoding.continuation,
                "orphan Circus argument continuation at {pos:#x}"
            );
            let arguments = match opcode {
                op if op <= encoding.bytes_end => {
                    let a = code
                        .get(pos + 1..pos + 3)
                        .context("truncated Circus byte operands")?;
                    pos += 3;
                    Arguments::Bytes(a.try_into()?)
                }
                op if op <= encoding.slot_end => {
                    let slot = *code.get(pos + 1).context("truncated Circus string slot")?;
                    let (text, next, raw) = string_bytes(code, pos + 2, false, false)?;
                    raw_strings.insert(start as u32, raw);
                    pos = next;
                    Arguments::SlotString { slot, text }
                }
                op if op <= encoding.encrypted_end => {
                    let (text, next, raw) = string_bytes(
                        code,
                        pos + 1,
                        opcode > encoding.plain_end,
                        matches!(profile, Profile::Dc4 | Profile::Dc4PlusHarmony),
                    )?;
                    raw_strings.insert(start as u32, raw);
                    pos = next;
                    Arguments::String(text)
                }
                _ => {
                    let args = words(code, pos + 1)?;
                    pos += 9;
                    Arguments::Words(args)
                }
            };
            let continuation = if encoding
                .continuation
                .is_some_and(|op| code.get(pos) == Some(&op))
            {
                let value = words(code, pos + 1)?;
                pos += 9;
                Some(value)
            } else {
                None
            };
            instructions.push(Instruction {
                offset: start as u32,
                next: pos as u32,
                opcode,
                arguments,
                continuation,
            });
        }
        Ok(Self {
            profile,
            code_file_offset,
            labels,
            label_metadata,
            instructions,
            raw_strings,
            original: None,
        })
    }
    /// Instruction lookup rejects entry into strings, arguments or extensions.
    pub fn instruction(&self, pc: u32) -> Result<&Instruction> {
        let index = self
            .instructions
            .binary_search_by_key(&pc, |op| op.offset)
            .map_err(|_| anyhow::anyhow!("Circus PC {pc:#x} is not an instruction boundary"))?;
        Ok(&self.instructions[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_native_space_escape_is_normalized_after_byte_decryption() {
        assert_eq!(
            string(&[b'A' - 32, 4, b'B' - 32, 0], 0, true).unwrap(),
            ("A B".into(), 4)
        );
        assert_eq!(string(b"A$B\0", 0, false).unwrap(), ("A$B".into(), 4));
    }
    fn fixture(code: &[u8]) -> Vec<u8> {
        let mut b = vec![];
        b.extend(2u32.to_le_bytes());
        b.extend(3u32.to_le_bytes());
        b.extend(0x80000003u32.to_le_bytes());
        b.extend([0; 4]);
        b.extend(0xabb6u16.to_le_bytes());
        b.push(0);
        b.extend(code);
        b
    }
    #[test]
    fn original_operands_signedness_encrypted_strings_and_continuation_boundaries() {
        let mut code = vec![0x72];
        for n in [-1i16, 4, 8, 12] {
            code.extend(n.to_le_bytes());
        }
        code.push(0x77);
        for n in [2i16, 3, 4, 5] {
            code.extend(n.to_le_bytes());
        }
        code.extend([0x69, b'A'.wrapping_sub(0x20), 0, 0, 0, 0]);
        let p = Program::parse(&fixture(&code)).unwrap();
        assert_eq!(p.profile, Profile::Dc4PlusHarmony);
        assert_eq!(p.labels, [3, 0x80000003]);
        assert_eq!(p.label_metadata, [0, 0]);
        assert_eq!(p.instructions.len(), 3);
        assert_eq!(
            p.instruction(3).unwrap().arguments,
            Arguments::Words([-1, 4, 8, 12])
        );
        assert_eq!(p.instruction(3).unwrap().continuation, Some([2, 3, 4, 5]));
        assert!(p.instruction(12).is_err());
        assert_eq!(
            p.instruction(21).unwrap().arguments,
            Arguments::String("A".into())
        );
    }
    #[test]
    fn bad_headers_truncation_strings_and_profiles_fail_visibly() {
        for code in [
            &[0x77, 0][..],
            &[0x3d, 1, b'x'][..],
            &[0x72, 0][..],
            &[0, 0][..],
        ] {
            assert!(Program::parse(&fixture(code)).is_err());
        }
        let mut b = fixture(&[0, 0, 0]);
        b[16] = 0;
        assert!(Program::parse(&b).is_err());
        let mut b = fixture(&[0, 0, 0]);
        b[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Program::parse(&b).is_err());
    }
    #[test]
    #[ignore = "requires original locally installed Circus MES files"]
    fn installed_mes_corpus() {
        let root = std::env::var_os("CRYSIRROR_CIRCUS_MES").expect("CRYSIRROR_CIRCUS_MES");
        let mut count = 0;
        for f in std::fs::read_dir(root).unwrap() {
            let p = f.unwrap().path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mes")) {
                Program::parse(&std::fs::read(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e:#}", p.display()));
                count += 1;
            }
        }
        assert!(count > 0);
        eprintln!("parsed {count} original Circus MES scripts");
    }
}

mod native_control;
pub mod native_live;
pub mod native_save;

/// `CIRCUS_TRACE=1` prints native instructions and resource jobs.
pub(crate) fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("CIRCUS_TRACE").is_some())
}
