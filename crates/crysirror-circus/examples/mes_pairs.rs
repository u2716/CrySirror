//! Diagnostic: count (opcode, operands) pairs for one opcode across a MES directory.
use anyhow::Result;
use std::collections::BTreeMap;

fn main() -> Result<()> {
    let dir = std::env::args().nth(1).expect("MES directory");
    let opcode = u8::from_str_radix(std::env::args().nth(2).expect("opcode hex").trim_start_matches("0x"), 16)?;
    let mut pairs: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("mes")) != Some(true) {
            continue;
        }
        let Ok(program) = crysirror_circus::Program::parse(&std::fs::read(&path)?) else { continue };
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        for i in program.instructions.iter().filter(|i| i.opcode == opcode) {
            let key = format!("{:?}", i.arguments);
            let entry = pairs.entry(key).or_insert((0, format!("{name}@{:#x}", i.offset)));
            entry.0 += 1;
        }
    }
    for (key, (count, site)) in pairs {
        println!("{count:5} {key} first={site}");
    }
    Ok(())
}
