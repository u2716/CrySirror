// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Decompile one TJS2 bytecode file: `decompile <file> [object-index]`.
//! With an index the object is disassembled instead.
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or_else(|| anyhow::anyhow!("usage: decompile <file> [object-index]"))?;
    let bytes = std::fs::read(&path)?;
    let file = crysirror_tjs2::bytecode::BytecodeFile::parse(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    match args.next() {
        Some(index) if index == "list" => {
            for (i, o) in file.objects.iter().enumerate() {
                println!("#{i} parent={:?} {:?} {}", o.parent, o.context_type, o.name(&file).unwrap_or(""));
            }
        }
        Some(index) => println!("{}", file.disassemble_object_full(index.parse()?).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => {
            let out = crysirror_tjs2::decompile::decompile(&file, &Default::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
            for s in &out.sources {
                println!("{}", s.text);
            }
        }
    }
    Ok(())
}
