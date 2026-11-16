//! Diagnostic: decode every WS2 entry of WillPlus ARC files with each layout
//! profile (raw and with the AdvHD ror-2 byte obfuscation) and count opcodes.
//! `ws2_census FILE.arc...`
use crysirror_willplus::program::{Profile, Program};
use std::collections::BTreeMap;
use std::sync::Arc;

fn entries(bytes: &[u8]) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let count = u32::from_le_bytes(bytes[0..4].try_into()?) as usize;
    let index = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
    let base = 8 + index;
    let mut at = 8;
    let mut out = Vec::new();
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
        out.push((String::from_utf16(&units)?, bytes[base + offset..base + offset + size].to_vec()));
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let mut ok: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures: BTreeMap<String, String> = BTreeMap::new();
    let mut histogram: BTreeMap<u8, usize> = BTreeMap::new();
    let mut total = 0;
    for path in std::env::args().skip(1) {
        for (name, raw) in entries(&std::fs::read(&path)?)? {
            if !name.to_ascii_lowercase().ends_with(".ws2") {
                continue;
            }
            total += 1;
            for rotate in [false, true] {
                let bytes: Vec<u8> = if rotate { raw.iter().map(|b| b.rotate_right(2)).collect() } else { raw.clone() };
                let bytes: Arc<[u8]> = bytes.into();
                for profile in [Profile::V1, Profile::V2, Profile::V3] {
                    let key = format!("{profile:?}/{}", if rotate { "ror2" } else { "raw" });
                    match Program::parse(profile, bytes.clone(), || Ok(())) {
                        Ok(program) => {
                            *ok.entry(key.clone()).or_default() += 1;
                            if key == std::env::var("WS2_PROFILE").unwrap_or_default() {
                                for i in program.instructions() {
                                    *histogram.entry(i.opcode).or_default() += 1;
                                }
                            }
                        }
                        Err(error) => {
                            failures.entry(key).or_insert_with(|| format!("{name}: {error:#}"));
                        }
                    }
                }
            }
        }
    }
    println!("ws2 files {total}");
    for (key, count) in &ok {
        println!("{key}: {count} ok");
    }
    for (key, failure) in &failures {
        println!("{key}: first failure {failure}");
    }
    for (opcode, count) in &histogram {
        println!("{opcode:#04x} {count}");
    }
    Ok(())
}
