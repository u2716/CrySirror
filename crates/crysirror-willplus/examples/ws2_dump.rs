//! Diagnostic listing of one WS2 entry: `ws2_dump FILE.arc NAME.ws2`
//! (`*` lists every WS2 entry, each under a `== NAME` header).
//! Applies the AdvHD ror-2 byte obfuscation and the V3 layout.
use crysirror_willplus::program::{Operand, Profile, Program};
use std::sync::Arc;

fn entry(bytes: &[u8], wanted: &str) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let count = u32::from_le_bytes(bytes[0..4].try_into()?) as usize;
    let index = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
    let base = 8 + index;
    let mut at = 8;
    for _ in 0..count {
        let size = u32::from_le_bytes(bytes[at..at + 4].try_into()?) as usize;
        let offset = u32::from_le_bytes(bytes[at + 4..at + 8].try_into()?) as usize;
        at += 8;
        let mut units = Vec::new();
        loop {
            let unit = u16::from_le_bytes(bytes[at..at + 2].try_into()?);
            at += 2;
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        let name = String::from_utf16(&units)?;
        if name.eq_ignore_ascii_case(wanted) || (wanted == "*" && name.to_ascii_lowercase().ends_with(".ws2")) {
            out.push((name, bytes[base + offset..base + offset + size].to_vec()));
        }
    }
    anyhow::ensure!(!out.is_empty(), "{wanted} not found");
    Ok(out)
}

fn show(bytes: &[u8], operand: &Operand) -> String {
    match operand {
        Operand::Byte(v) => format!("b{v}"),
        Operand::Short(v) => format!("h{v}"),
        Operand::Integer(v) => format!("i{v}"),
        Operand::Address(v) => format!("@{v:#x}"),
        Operand::FloatBits(v) => format!("f{}", f32::from_bits(*v)),
        Operand::String(range) => format!("{:?}", encoding_rs::SHIFT_JIS.decode(&bytes[range.clone()]).0),
        Operand::Array(items) => format!("[{}]", items.iter().map(|o| show(bytes, o)).collect::<Vec<_>>().join(", ")),
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let arc = std::fs::read(args.next().expect("FILE.arc"))?;
    let wanted = args.next().expect("NAME.ws2");
    for (name, raw) in entry(&arc, &wanted)? {
    if wanted == "*" {
        println!("== {name}");
    }
    let bytes: Arc<[u8]> = raw.iter().map(|b| b.rotate_right(2)).collect::<Vec<_>>().into();
    let program = Program::parse(Profile::V3, bytes.clone(), || Ok(()))?;
    for i in program.instructions() {
        let ops: Vec<String> = i.operands.iter().map(|o| show(&bytes, o)).collect();
        let choices: Vec<String> = i.choices.iter().map(|c| format!("{{{} -> {:#04x} {}}}", c.operands.iter().map(|o| show(&bytes, o)).collect::<Vec<_>>().join(" "), c.continuation.opcode, c.continuation.operands.iter().map(|o| show(&bytes, o)).collect::<Vec<_>>().join(" "))).collect();
        println!("{:#07x} {:#04x} {} {}", i.pc, i.opcode, ops.join(" "), choices.join(" "));
    }
    }
    Ok(())
}
