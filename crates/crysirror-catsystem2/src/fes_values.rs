// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original759680 lazy tuple-component policy. This is expression/addressing
//! VM policy, not a geometry or text-layout algorithm.
use crate::native_interpreter::Interpreter;
use anyhow::{Result, ensure};

///Only the requested component is evaluated. Skipped components retain their
///side effects unapplied. Missing commas return None; empty existing components
///evaluate as native integer zero with an attached interpreter. Negative
///indices behave as component0, like the original signed ordinal branch.
pub(crate) fn component(
    source: &[u8],
    index: i32,
    interpreter: Option<&mut Interpreter>,
) -> Result<Option<u32>> {
    ensure!(
        source.len() <= 16 * 1024 * 1024 && !source.contains(&0),
        "CatSystem2 FES component input extent/NUL"
    );
    let mut at = 0;
    for _ in 0..index.max(0) {
        //6A56F0 is literal byte substring search, not CSV or an expression
        //lexer: parentheses, quotes and multibyte values do not hide commas.
        let Some(comma) = source[at..].iter().position(|b| *b == b',') else {
            return Ok(None);
        };
        at += comma + 1;
    }
    if let Some(interpreter) = interpreter {
        let tail = &source[at..];
        let end = tail.iter().position(|b| *b == b',').unwrap_or(tail.len());
        ensure!(
            end < 512,
            "CatSystem2 FES component native512-byte buffer extent"
        );
        Ok(Some(interpreter.evaluate(&tail[..end])?))
    } else {
        Ok(crate::native_values::integer_prefix(&source[at..]).map(|(_, value)| value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tuple_fields_are_lazy_empty_and_missing_have_distinct_original_results() {
        let mut vm = Interpreter::new(8).unwrap();
        assert_eq!(
            component(b"$0+=1,$0+=10,", 1, Some(&mut vm)).unwrap(),
            Some(10)
        );
        assert_eq!(vm.flag(0), 10);
        assert_eq!(component(b"1,,3", 1, Some(&mut vm)).unwrap(), Some(0));
        assert_eq!(component(b"1,2,", 2, Some(&mut vm)).unwrap(), Some(0));
        assert_eq!(component(b"1,2,", 3, Some(&mut vm)).unwrap(), None);
        assert_eq!(component(b"1+2,9", -1, Some(&mut vm)).unwrap(), Some(3));
        assert_eq!(component(b"($0+=2),9", 0, Some(&mut vm)).unwrap(), Some(12));
        assert_eq!(component(b"ignored,$0+=1", 2, Some(&mut vm)).unwrap(), None);
        assert_eq!(vm.flag(0), 12);
    }
    #[test]
    fn unattached_native_prefix_and_buffer_failure_do_not_evaluate_other_fields() {
        assert_eq!(component(b"-1,0x,+2tail", 0, None).unwrap(), Some(u32::MAX));
        assert_eq!(component(b"-1,0x,+2tail", 1, None).unwrap(), Some(0));
        assert_eq!(component(b"-1,0x,+2tail", 2, None).unwrap(), Some(2));
        assert_eq!(component(b"1.5,,2", 0, None).unwrap(), None);
        assert_eq!(component(b"1.5,,2", 1, None).unwrap(), None);
        let mut vm = Interpreter::new(8).unwrap();
        let source = [vec![b'0'; 512], b",++$0".to_vec()].concat();
        assert!(component(&source, 0, Some(&mut vm)).is_err());
        assert_eq!(vm.flag(0), 0);
        assert_eq!(component(&source, 1, Some(&mut vm)).unwrap(), Some(1));
        assert!(component(b"1\0,2", 0, None).is_err());
    }
}
