//! D.C.III (DC3.exe, MES 0x8a71) to DC3WY (0xa09f) opcode translation.
//!
//! Evidence: the 23 system scripts both installations ship (main, load,
//! save, config, advmenu, cgmode, cg_*, sce_*, music, ...) were aligned
//! instruction-by-instruction on identical operands; every mapping below
//! with a count came from those aligned pairs (strong matches). Others were
//! placed by reading DC3.exe 424FE0 against DC3WY 4233A0:
//!  * 0x07 (text click wait, process state 7) -> 0x06;
//!  * 0x2e (slot string between 0x2d->0x3b and 0x2f->0x3d) -> 0x3c;
//!  * 0x37/0x38 (voice name + 42E6D0 playback) -> 0x46/0x47;
//!  * 0x3c (music stop + process state 60 movie name) -> 0x4b;
//!  * 0x44 (choice button text, +32 bytes into the 1388-byte template) -> 0x54;
//!  * 0x4d ("" / "-" releases the two overlay images) -> 0x5d;
//!  * 0x4f/0x50 (speaker / dialogue, encrypted) -> 0x61/0x62;
//!  * 0x5c/0x5e (41AC90 / 41AA40 word records) -> 0x72/0x74;
//!  * 0x6b (portrait position table, 10 dwords per entry; 0x10 flag scale)
//!    -> 0x81; 0x6c (a3==0 x/y pair) -> 0x82.
//! 0x05 is the byte-form music command (index, mode; 255 stop, 254
//! replay-current, 10/11 no-op, optional following 0x06 start token); it is
//! rewritten to DC3WY's word-form 0x67 (index, mode).
//! Opcodes not listed here keep their original number behind UNTRANSLATED
//! and fail visibly at execution.
use crate::{Arguments, Instruction, Profile, Program};
use anyhow::{bail, ensure, Result};

///Marker opcode for an original instruction with no reviewed DC3WY mapping.
pub const UNTRANSLATED: u8 = 0xee;
///D.C.III 0x6C (a3 == 0): the face draw origin (C63CEC/C63CF0) that
///4202E0 uses for the slot 19/20 face layers.
pub const FACE_ORIGIN: u8 = 0xe0;

///D.C.III image slots onto the DC3WY slots with the same role (from the
///resources each loads in the two script sets): background 1, portraits
///3..6 -> 2..5 ('?' substitution, 0x2F/0x6B index 0..3), UI chips 11/12 ->
///14/15, chip/mask 13 -> 16 (MSK), 17/18 unchanged (0x27 petal/rain
///atlases), face 19 -> 25, and slot 16 (423290's fog) by resource:
///SYSTEM\FOG* -> 20 (object 54), EFF\FOGX* (3072x1200 scroll bands) -> 21,
///others -> 22.
fn slot(original: u8, name: &str) -> Option<u8> {
    Some(match original {
        0 | 1 | 17 | 18 => original,
        3..=6 => original - 1,
        11 => 14,
        12 => 15,
        13 => 16,
        16 if name.to_ascii_uppercase().starts_with("SYSTEM\\FOG") => 20,
        16 if name.to_ascii_uppercase().starts_with("EFF\\FOGX") => 21,
        16 => 22,
        19 => 25,
        //0x2D system index k loads slot 22+k (MESWIN, ADVMENU_CHIP, ...);
        //DC3WY's 0x3B loads 28+k.
        22..=28 => original + 6,
        _ => return None,
    })
}

fn map(opcode: u8) -> Option<u8> {
    Some(match opcode {
        0x00 | 0x02 | 0x03 | 0x04 | 0x08 | 0x09 | 0x0a | 0x0e | 0x10 | 0x12 | 0x13 | 0x14 | 0x15
        | 0x16 | 0x18 | 0x1a | 0x1b | 0x1c | 0x1e | 0x20 | 0x22 | 0x26 | 0x27 => opcode,
        0x07 => 0x06,
        0x2c => 0x3a,
        0x2d => 0x3b,
        0x2e => 0x3c,
        0x2f => 0x3d,
        0x30 => 0x3f,
        0x32 => 0x41,
        0x34..=0x3c => opcode + 15,
        0x3e => 0x4d,
        0x42 => 0x51,
        0x43 => 0x52,
        0x44 => 0x54,
        0x45 => 0x55,
        0x4a => 0x5a,
        0x4d => 0x5d,
        0x4f => 0x61,
        0x50 => 0x62,
        0x51 => 0x63,
        0x53..=0x55 => opcode + 21,
        0x56 => 0x6c,
        0x59..=0x64 => opcode + 22,
        0x66..=0x69 => opcode + 22,
        0x6b => 0x81,
        0x6c => 0x82,
        0x6e => 0x86,
        0x6f => 0x87,
        _ => return None,
    })
}

impl Program {
    ///Rewrite a D.C.III program into DC3WY opcode numbering, keeping every
    ///native offset, label and literal unchanged.
    pub fn into_dc3wy(mut self) -> Result<Self> {
        ensure!(self.profile == Profile::Dc3, "DC3 translation requires a 0x8a71 program");
        let mut translated = std::collections::BTreeMap::new();
        for instruction in &mut self.instructions {
            let original = instruction.opcode;
            translated.insert(instruction.offset, original);
            if original == 0x05 {
                let Arguments::Bytes([index, mode]) = instruction.arguments else {
                    bail!("DC3 music command operands");
                };
                instruction.opcode = 0x67;
                instruction.arguments = Arguments::Words([index.into(), mode.into(), 0, 0]);
                continue;
            }
            if original == 0x6c {
                instruction.opcode = FACE_ORIGIN;
                continue;
            }
            instruction.opcode = map(original).unwrap_or(UNTRANSLATED);
            //Image parts (0x55 -> 0x6A, 0x5A -> 0x70) name their source slot
            //in continuation word 2 (0x5A keeps flag bits 6/7 above it); a
            //0x55 slider (word 3 == 5) has no slot.
            if matches!(original, 0x55 | 0x5a) {
                if let Some(words) = &mut instruction.continuation {
                    let slider = original == 0x55 && words[3] == 5;
                    if !slider {
                        let flags = if original == 0x5a { words[2] & !0x3f } else { 0 };
                        let number = u8::try_from(words[2] & if original == 0x5a { 0x3f } else { 0xff }).unwrap_or(u8::MAX);
                        match slot(number, "") {
                            Some(mapped) => words[2] = flags | i16::from(mapped),
                            None => instruction.opcode = UNTRANSLATED,
                        }
                    }
                }
            }
            //0x2C's slot operand selects the image role (42BA70 state 44).
            if original == 0x2c {
                if let Arguments::SlotString { slot: number, text } = &mut instruction.arguments {
                    match slot(*number, text) {
                        Some(mapped) => *number = mapped,
                        None => instruction.opcode = UNTRANSLATED,
                    }
                }
            }
        }
        self.profile = Profile::Dc3WithYou;
        self.original = Some(translated);
        Ok(self)
    }
    ///The D.C.III opcode an instruction was translated from.
    pub fn original_opcode(&self, instruction: &Instruction) -> Option<u8> {
        self.original.as_ref()?.get(&instruction.offset).copied()
    }
}
