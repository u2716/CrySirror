//! Disassemble one original AIN function (names resolved for calls):
//! `cargo run -p crysirror-system4 --example ain_function -- GAME.ain NAME`.
use anyhow::{Context, Result};
fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("ain_function GAME.ain NAME")?;
    let wanted = std::env::args().nth(2).context("function name")?;
    let program = crysirror_system4::ain::Program::parse(&std::fs::read(path)?)?;
    let start = program
        .functions
        .iter()
        .find(|f| String::from_utf8_lossy(&f.name) == wanted)
        .context("function not found")?
        .address;
    let mut pc = start;
    loop {
        let i = program.instruction(pc)?;
        let note = match i.name {
            "CALLFUNC" | "CALLMETHOD" => program.functions.get(i.arguments[0] as usize).map(|f| String::from_utf8_lossy(&f.name).into_owned()),
            "CALLHLL" => program.libraries.get(i.arguments[0] as usize).and_then(|l| {
                l.functions.get(i.arguments[1] as usize).map(|f| format!("{}.{}", String::from_utf8_lossy(&l.name), String::from_utf8_lossy(&f.name)))
            }),
            "S_PUSH" => program.strings.get(i.arguments[0] as usize).map(|s| format!("{:?}", encoding_rs::SHIFT_JIS.decode(s).0)),
            _ => None,
        };
        println!("{:#x} {} {:?} {}", i.pc, i.name, i.arguments, note.unwrap_or_default());
        if matches!(i.name, "ENDFUNC") || i.next as usize >= program.code.len() {
            break;
        }
        pc = i.next;
    }
    Ok(())
}
