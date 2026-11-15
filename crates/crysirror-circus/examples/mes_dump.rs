//! Diagnostic listing of a native MES program: `mes_dump FILE.mes [START_PC] [COUNT]`.
use anyhow::Result;
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("FILE.mes");
    let program = crysirror_circus::Program::parse(&std::fs::read(&path)?)?;
    let parse = |s: Option<String>, d: u32| s.map_or(d, |s| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap());
    let start = parse(args.next(), 0);
    let count = args.next().map_or(usize::MAX, |s| s.parse().unwrap());
    for instruction in program.instructions.iter().filter(|i| i.offset >= start).take(count) {
        println!("{:#06x} {:#04x} {:?}{}", instruction.offset, instruction.opcode, instruction.arguments, instruction.continuation.map(|c| format!(" +{c:?}")).unwrap_or_default());
    }
    Ok(())
}
