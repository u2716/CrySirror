//! Diagnostic census of native MES opcodes: `mes_census DIR`. For each opcode
//! prints the count and the distinct first operands (selectors).
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
fn main() -> Result<()> {
    let dir = std::env::args().nth(1).expect("MES directory");
    let mut census: BTreeMap<u8, (usize, BTreeSet<i32>, BTreeSet<String>)> = BTreeMap::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("mes")) != Some(true) {
            continue;
        }
        let program = match crysirror_circus::Program::parse(&std::fs::read(&path)?) {
            Ok(program) => program,
            Err(error) => { eprintln!("{}: {error:#}", path.display()); continue; }
        };
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        for i in &program.instructions {
            let selector = match &i.arguments {
                crysirror_circus::Arguments::Bytes(v) => i32::from(v[0]),
                crysirror_circus::Arguments::Words(v) => i32::from(v[0]),
                crysirror_circus::Arguments::SlotString { slot, .. } => i32::from(*slot),
                crysirror_circus::Arguments::String(_) => -1,
            };
            let entry = census.entry(i.opcode).or_default();
            entry.0 += 1;
            entry.1.insert(selector);
            if entry.2.len() < 3 { entry.2.insert(format!("{name}@{:#x}", i.offset)); }
        }
    }
    for (opcode, (count, selectors, sites)) in census {
        let selectors: Vec<_> = selectors.into_iter().take(40).collect();
        println!("{opcode:#04x} n={count} sel={selectors:?} at={sites:?}");
    }
    Ok(())
}
