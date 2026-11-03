// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original73FF40/77C270 string-bank policy. Byte replacement and C32
//! formatting are shared Text mechanisms; no pointer-valued varargs escape VM.
use crate::{native_interpreter::Interpreter, native_values::StringVariables};
use anyhow::{Result, ensure};

impl StringVariables {
    ///Inserted values are not recursively expanded. Missing IDs substitute an
    ///empty string; malformed/case-different markers remain literal bytes.
    pub(crate) fn expand(&self, source: &[u8], limit: usize) -> Result<Vec<u8>> {
        ensure!(
            !source.contains(&0),
            "CatSystem2 string expansion input NUL"
        );
        let mut spans = Vec::new();
        let mut at = 0;
        while at < source.len() {
            if source[at..].starts_with(b"$str") {
                if let Some((n, id)) = crate::native_values::integer_prefix(&source[at + 4..]) {
                    spans.push((at..at + 4 + n, self.get(id).unwrap_or_default()));
                    at += 4 + n;
                    continue;
                }
            }
            let token = [b"%LANG%".as_slice(), b"%LANG_M%"].into_iter().find(|token| source[at..].starts_with(token));
            if let (Some(token), false) = (token, self.language_prefix.is_empty()) {
                spans.push((at..at + token.len(), self.language_prefix.as_slice()));
                at += token.len();
                continue;
            }
            at += 1;
        }
        crysirror_text::byte_replace::replace_spans_bytes(source, &spans, limit, &|| Ok(()))
    }
}
///77C270 consumes at most four numeric varargs. Zero arguments copy the format
///literally, so `%%` changes only in the actual printf path. Evaluation commits
///once with the bank assignment; unsafe/unknown formats retain original pending.
pub(crate) fn assign(
    words: &[Vec<u8>],
    interpreter: &mut Interpreter,
    bank: &mut StringVariables,
) -> Result<()> {
    let Some(first) = words.first() else {
        return Ok(());
    };
    let mut proposed = interpreter.clone();
    let assignment = if first.eq_ignore_ascii_case(b"get") {
        if let Some(destination) = words.get(1) {
            let id = proposed.evaluate(destination)?;
            if let Some(source) = words.get(2) {
                let source = proposed.evaluate(source)?;
                Some((id, bank.get(source).unwrap_or_default().to_vec()))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        let id = proposed.evaluate(first)?;
        let value = if let Some(template) = words.get(1) {
            let args = words
                .iter()
                .skip(2)
                .take(4)
                .map(|s| {
                    proposed
                        .evaluate(s)
                        .map(crysirror_text::printf32::Printf32Argument::Word)
                })
                .collect::<Result<Vec<_>>>()?;
            if args.is_empty() {
                template.clone()
            } else {
                crysirror_text::printf32::format_printf32_bytes(template, &args, 2559)?.bytes
            }
        } else {
            Vec::new()
        };
        //740450 copies a C string, including `%c` NUL termination semantics.
        let end = value.iter().position(|b| *b == 0).unwrap_or(value.len());
        Some((id, value[..end].to_vec()))
    };
    if let Some((id, value)) = assignment {
        bank.assign(id, value)?;
    }
    *interpreter = proposed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(source: &[&[u8]]) -> Vec<Vec<u8>> {
        source.iter().map(|s| s.to_vec()).collect()
    }
    #[test]
    fn markers_use_original_numeric_prefix_and_one_pass() {
        let mut bank = StringVariables::default();
        bank.assign(0, b"zero".to_vec()).unwrap();
        bank.assign(1, b"$str2".to_vec()).unwrap();
        bank.assign(2, b"two".to_vec()).unwrap();
        bank.assign(u32::MAX, b"negative".to_vec()).unwrap();
        assert_eq!(
            bank.expand(
                b"$str1/$str+02tail/$str-1/$str0x/$str0x1/$str4294967296/$str7",
                256
            )
            .unwrap(),
            b"$str2/twotail/negative/zero/$str2/zero/"
        );
        assert_eq!(
            bank.expand(b"$str1.0/$STR2/$str+q/$str/$str-", 256)
                .unwrap(),
            b"$str1.0/$STR2/$str+q/$str/$str-"
        );
        assert_eq!(
            bank.expand(&bank.expand(b"$str1", 256).unwrap(), 256)
                .unwrap(),
            b"two"
        );
        assert!(bank.expand(b"$str1", 4).is_err());
    }
    #[test]
    fn format_copy_clear_and_c_string_extent() {
        let mut bank = StringVariables::default();
        let mut vm = Interpreter::new(8).unwrap();
        assign(&words(&[b"1", b"raw%%"]), &mut vm, &mut bank).unwrap();
        assign(&words(&[b"2", b"[%04d]/%%", b"7"]), &mut vm, &mut bank).unwrap();
        assign(&words(&[b"GET", b"3", b"2"]), &mut vm, &mut bank).unwrap();
        assert_eq!(bank.get(1), Some(b"raw%%".as_slice()));
        assert_eq!(bank.get(3), Some(b"[0007]/%".as_slice()));
        assign(&words(&[b"get", b"4", b"999"]), &mut vm, &mut bank).unwrap();
        assign(&words(&[b"5"]), &mut vm, &mut bank).unwrap();
        assign(&words(&[b"6", b"A%cB", b"0"]), &mut vm, &mut bank).unwrap();
        assert_eq!(bank.get(4), Some(b"".as_slice()));
        assert_eq!(bank.get(5), Some(b"".as_slice()));
        assert_eq!(bank.get(6), Some(b"A".as_slice()));
    }
    #[test]
    fn undefined_pointer_profiles_preserve_bank_and_expression_state() {
        let mut bank = StringVariables::default();
        let mut vm = Interpreter::new(8).unwrap();
        bank.assign(1, b"before".to_vec()).unwrap();
        assert!(assign(&words(&[b"1", b"%s", b"$0+=1"]), &mut vm, &mut bank).is_err());
        assert_eq!(bank.get(1), Some(b"before".as_slice()));
        assert_eq!(vm.flag(0), 0);
        assign(
            &words(&[b"2", b"%d%d%d%d", b"1", b"2", b"3", b"4", b"$0+=1"]),
            &mut vm,
            &mut bank,
        )
        .unwrap();
        assert_eq!(bank.get(2), Some(b"1234".as_slice()));
        assert_eq!(vm.flag(0), 0);
    }
}

///782FB0 normalizes unsigned-byte strcmp to -1/0/+1. Missing strings are
///empty; the destination expression is evaluated only when present.
pub(crate) fn compare(words: &[Vec<u8>], interpreter: &mut Interpreter) -> Result<()> {
    if let Some(destination) = words.first() {
        let mut proposed = interpreter.clone();
        let destination = proposed.evaluate(destination)?;
        let a = words.get(1).map(Vec::as_slice).unwrap_or_default();
        let b = words.get(2).map(Vec::as_slice).unwrap_or_default();
        let result = match a.cmp(b) {
            std::cmp::Ordering::Less => u32::MAX,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        proposed.set_flag(destination, result);
        *interpreter = proposed;
    }
    Ok(())
}
///77C4B0 stores/removes a per-player table entry.77C6D0 deliberately evaluates
///and discards its first argument, then uses its SECOND argument both as table
///key and destination string ID. This odd ABI is confirmed by x86, not guessed
///from a likely command name. Parent/child tables are independent.
pub(crate) fn table_command(
    set: bool,
    words: &[Vec<u8>],
    interpreter: &mut Interpreter,
    bank: &mut StringVariables,
    table: &mut StringVariables,
) -> Result<()> {
    let Some(first) = words.first() else {
        return Ok(());
    };
    let mut proposed = interpreter.clone();
    let key = proposed.evaluate(first)?;
    if set {
        if let Some(value) = words.get(1) {
            table.assign(key, value.clone())?;
        } else {
            table.remove(key);
        }
    } else if let Some(destination) = words.get(1) {
        let destination = proposed.evaluate(destination)?;
        bank.assign(
            destination,
            table.get(destination).unwrap_or_default().to_vec(),
        )?;
    }
    *interpreter = proposed;
    Ok(())
}
///785670 uses the raw row after its second ASCII space, not a reconstructed
///token list. It skips native CP932 lead/trail pairs while replacing spaces.
///Only byte-span assembly is a shared Text mechanism; lexical policy stays VM.
pub(crate) fn replace_space(
    op: &crate::fes_program::PendingOperation,
    interpreter: &mut Interpreter,
    bank: &mut StringVariables,
) -> Result<()> {
    let Some(destination) = op.arguments.first() else {
        return Ok(());
    };
    let mut proposed = interpreter.clone();
    let destination = proposed.evaluate(destination)?;
    let mut count = 0;
    let mut tail = None;
    for (at, b) in op.raw.iter().enumerate() {
        if *b == b' ' {
            count += 1;
            if count == 2 {
                tail = Some(at + 1);
                break;
            }
        }
    }
    if let Some(at) = tail.filter(|at| *at < op.raw.len()) {
        let replacement = &op.raw[at..];
        let source = bank.get(destination).ok_or_else(|| {
            anyhow::anyhow!("CatSystem2 replacespace missing native string pointer")
        })?;
        let mut at = 0;
        let mut spans = Vec::new();
        while at < source.len() {
            if matches!(source[at],0x81..=0x9f|0xe0..=0xfc) {
                ensure!(
                    at + 1 < source.len(),
                    "CatSystem2 replacespace truncated CP932 pair"
                );
                at += 2;
            } else {
                if source[at] == b' ' {
                    spans.push((at..at + 1, replacement));
                }
                at += 1;
            }
        }
        let result =
            crysirror_text::byte_replace::replace_spans_bytes(source, &spans, 2047, &|| Ok(()))?;
        bank.assign(destination, result)?;
    }
    *interpreter = proposed;
    Ok(())
}

#[cfg(test)]
mod table_tests {
    use super::*;
    fn w(source: &[&[u8]]) -> Vec<Vec<u8>> {
        source.iter().map(|s| s.to_vec()).collect()
    }
    #[test]
    fn table_lookup_retains_actual_second_argument_abi_and_parent_independence() {
        let mut vm = Interpreter::new(8).unwrap();
        let mut bank = StringVariables::default();
        let mut parent = StringVariables::default();
        let mut child = StringVariables::default();
        table_command(
            true,
            &w(&[b"2", b"parent"]),
            &mut vm,
            &mut bank,
            &mut parent,
        )
        .unwrap();
        table_command(true, &w(&[b"2", b"child"]), &mut vm, &mut bank, &mut child).unwrap();
        table_command(
            false,
            &w(&[b"$0+=1", b"2"]),
            &mut vm,
            &mut bank,
            &mut parent,
        )
        .unwrap();
        assert_eq!(vm.flag(0), 1);
        assert_eq!(bank.get(2), Some(b"parent".as_slice()));
        table_command(false, &w(&[b"999", b"2"]), &mut vm, &mut bank, &mut child).unwrap();
        assert_eq!(bank.get(2), Some(b"child".as_slice()));
        table_command(true, &w(&[b"2"]), &mut vm, &mut bank, &mut child).unwrap();
        table_command(false, &w(&[b"999", b"2"]), &mut vm, &mut bank, &mut child).unwrap();
        assert_eq!(bank.get(2), Some(b"".as_slice()));
        assert_eq!(parent.get(2), Some(b"parent".as_slice()));
    }
    #[test]
    fn compare_defaults_and_unsigned_encoding_order_match_original() {
        let mut vm = Interpreter::new(8).unwrap();
        compare(&w(&[b"0", b"\x82", b"z"]), &mut vm).unwrap();
        assert_eq!(vm.flag(0), 1);
        compare(&w(&[b"1", b"a", b"ab"]), &mut vm).unwrap();
        assert_eq!(vm.flag(1), u32::MAX);
        compare(&w(&[b"2"]), &mut vm).unwrap();
        assert_eq!(vm.flag(2), 0);
    }
    #[test]
    fn replace_uses_raw_tail_retains_pairs_and_rolls_back_bad_source() {
        let mut vm = Interpreter::new(8).unwrap();
        let mut bank = StringVariables::default();
        bank.assign(1, b"a b\x81 c ".to_vec()).unwrap();
        let op = crate::fes_program::PendingOperation {
            raw: b"replacespace 1 X Y".to_vec(),
            kind: crate::fes_program::PendingKind::Native,
            section: b"START".to_vec(),
            ordinal: 1,
            line: 1,
            byte_offset: 7,
            name: b"replacespace".to_vec(),
            arguments: w(&[b"1", b"X", b"Y"]),
        };
        replace_space(&op, &mut vm, &mut bank).unwrap();
        assert_eq!(bank.get(1), Some(b"aX Yb\x81 cX Y".as_slice()));
        bank.assign(1, b"bad\x81".to_vec()).unwrap();
        let mut op = op;
        op.arguments[0] = b"++$0".to_vec();
        assert!(replace_space(&op, &mut vm, &mut bank).is_err());
        assert_eq!(vm.flag(0), 0);
        assert_eq!(bank.get(1), Some(b"bad\x81".as_slice()));
    }
}
