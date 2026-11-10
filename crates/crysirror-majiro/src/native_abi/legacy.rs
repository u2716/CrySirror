// Copyright (c) 2026 CrySirror contributors. MIT.
//! Early Scarlett PE32 layout. Static reviewed dispatcher40BDB0 reads the
//! u32 hash through4079D0 and signed i16 argc through4079F0. Values still use
//! 16-byte records (tag+8, payload+12), strings+12 and512-byte CRT scratch.
//! This engine predates the SetEncryptionKey native; do not require an absent
//! password-call shape to identify its unrelated stack/value contract.
use super::{NativeAbi, Template, branch_call_compare, find, unique, writable};
use anyhow::{Context, Result, ensure};
use crysirror_resource::pe::PeData;

const DISPATCH: &str =
    "56 57 E8 ~read_hash 8B F0 E8 ~read_argc 8B 0D &thread 8B 79 08 2B F8 81 FE 43 0A DB 10";
const READ_HASH: &str = "8B 15 &thread 8B 0A 8B 01 83 C1 04 89 0A C3";
const READ_ARGC: &str = "8B 15 &thread 8B 0A 0F BF 01 83 C1 02 89 0A C3";
const TOP: &str = "A1 &thread 8B 48 08 8B 15 &state 8B 42 24 C1 E1 04 8D 44 01 F0 C3";
const PUSH: &str = "A1 &thread 8B 40 08 8B 0D &state 8B 51 24 C1 E0 04 03 C2 8B 54 24 04 89 50 0C C7 40 08 03 00 00 00 C7 40 04 00 00 00 00 E9 ~grow";
const GROW: &str = "A1 &thread FF 40 08 8B 0D &state 8B 15 &thread 8B 41 10 39 42 08";
const PAGE: &str =
    "53 56 57 E8 ~top 83 7C 24 10 03 8B 78 0C 8B 70 FC 7C 05 8B 58 EC EB 05 BB 01 00 00 00";
const UPPER: &str = "81 EC 00 02 00 00 E8 ~top 8B 40 0C 83 C0 0C 50 8D 4C 24 04 51";
const TEST: &str = "E8 ~mode 48 F7 D8 1B C0 F7 D8 50 E8 ~push 59 C3";
const MODE: &str = "A1 &startup_mode C3";

pub(super) fn inspect(pe: &PeData<'_>) -> Result<Option<NativeAbi>> {
    // The caller already validates PE32 and its executable-section scan budget.
    let dispatch = Template::new(DISPATCH)?;
    let mut accepted = None;
    let mut candidates = 0;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let code = pe.rva_slice(section.rva, section.file_size as usize)?;
        let base = pe.image_base() + u64::from(section.rva);
        for offset in dispatch.candidates(code) {
            let address = base + offset as u64;
            let Some(captures) = dispatch.matches(&code[offset..], address, &[]) else {
                continue;
            };
            candidates += 1;
            ensure!(candidates <= 2, "Majiro early dispatcher candidate budget");
            let thread = find(&captures, "thread")?;
            let state = thread
                .checked_add(4)
                .context("Majiro early state address overflow")?;
            if !writable(pe, thread) || !writable(pe, state) {
                continue;
            }
            let globals = [("thread", thread), ("state", state)];
            let hash_reader = find(&captures, "read_hash")?;
            let argc_reader = find(&captures, "read_argc")?;
            if !matches_at(pe, READ_HASH, hash_reader, &globals)?
                || !matches_at(pe, READ_ARGC, argc_reader, &globals)?
            {
                continue;
            }
            let Some(top) = unique(pe, &Template::new(TOP)?, &globals)? else {
                continue;
            };
            let push_template = Template::new(PUSH)?;
            let Some(push) = unique(pe, &push_template, &globals)? else {
                continue;
            };
            let push_captures = push_template
                .matches(pe.va_slice(push, push_template.size)?, push, &globals)
                .context("Majiro early push layout changed")?;
            if !matches_at(pe, GROW, find(&push_captures, "grow")?, &globals)? {
                continue;
            }
            let helpers = [("top", top), ("push", push)];
            let Some(page) = unique(pe, &Template::new(PAGE)?, &helpers)? else {
                continue;
            };
            // Several CRT functions have this stack/scratch prefix. Resolve the
            // registered uppercase branch first, then validate that exact leaf.
            let body = &code[offset..code.len().min(offset + 16384)];
            let Some(upper) = branch_call_compare(body, address, 0x266a3c79, &[0x81, 0xfe]) else {
                continue;
            };
            if !matches_at(pe, UPPER, upper, &helpers)? {
                continue;
            }
            let test_template = Template::new(TEST)?;
            let Some(test) = unique(pe, &test_template, &helpers)? else {
                continue;
            };
            let test_captures = test_template
                .matches(pe.va_slice(test, test_template.size)?, test, &helpers)
                .context("Majiro early startup layout changed")?;
            let mode = find(&test_captures, "mode")?;
            if !matches_at(pe, MODE, mode, &[])? {
                continue;
            }
            let code = &code[offset..code.len().min(offset + 16384)];
            if [(0x83d81f59, page), (0x266a3c79, upper), (0x6e6c641a, test)]
                .into_iter()
                .all(|(hash, target)| {
                    branch_call_compare(code, address, hash, &[0x81, 0xfe]) == Some(target)
                })
            {
                ensure!(
                    accepted.is_none(),
                    "Majiro ambiguous early native ABI structures"
                );
                accepted = Some(NativeAbi::Classic);
            }
        }
    }
    Ok(accepted)
}
fn matches_at(
    pe: &PeData<'_>,
    pattern: &'static str,
    address: u64,
    expected: &[(&'static str, u64)],
) -> Result<bool> {
    let template = Template::new(pattern)?;
    Ok(template
        .matches(pe.va_slice(address, template.size)?, address, expected)
        .is_some())
}
