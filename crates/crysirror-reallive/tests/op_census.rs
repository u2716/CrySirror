// SPDX-License-Identifier: MPL-2.0
//! Owner-media diagnostic: lists every distinct `(modtype, module, opcode)`
//! used by a private game's Seen.txt with its occurrence count.
//! `CRYSIRROR_REALLIVE_CENSUS=<game dir> cargo test --release -p crysirror-reallive
//! --test op_census -- --ignored --nocapture`. Never reads data unless asked.
use crysirror_reallive::vm::{Archive, Gameexe, bytecode::Element};
use std::collections::BTreeMap;

#[test]
#[ignore = "needs owner-supplied RealLive game directory"]
fn census() {
    let Some(dir) = std::env::var_os("CRYSIRROR_REALLIVE_CENSUS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let find = |name: &str| {
        std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
            .map(|e| e.path())
            .unwrap()
    };
    let ini = std::fs::read(find("Gameexe.ini")).unwrap();
    let mut exe = Gameexe::from_bytes(&ini, "sjis".parse().unwrap());
    let seen = std::fs::read(find("Seen.txt")).unwrap();
    let mut archive = Archive::from_parts(
        seen.clone(),
        Default::default(),
        exe.str("REGNAME").unwrap_or(""),
        "sjis".parse().unwrap(),
    )
    .unwrap();
    if let Some(nls) = archive.probable_encoding() {
        exe = Gameexe::from_bytes(&ini, nls);
        archive = Archive::from_parts(seen, Default::default(), exe.str("REGNAME").unwrap_or(""), nls)
            .unwrap();
    }
    let mut counts: BTreeMap<(u8, u8, u16), u64> = BTreeMap::new();
    let numbers: Vec<i32> = archive.numbers().collect();
    for n in numbers {
        let Ok(s) = archive.scenario(n) else {
            eprintln!("scenario {n} failed to parse");
            continue;
        };
        for e in &s.script.elements {
            if let Element::Command(c) = e {
                *counts.entry((c.op.modtype, c.op.module, c.op.opcode)).or_default() += 1;
            }
        }
    }
    for ((t, m, o), n) in counts {
        println!("OP {t}:{m}:{o} {n}");
    }
}
