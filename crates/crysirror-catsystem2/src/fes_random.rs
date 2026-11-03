// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!Complete reviewed77BBB0/77BC80 DWORD range/table policy. Random generation
//!uses the shared MTsplit kernel and owned session seeds, never a private RNG.
use crate::{motion::RandomStreams, native_interpreter::Interpreter};
use anyhow::{ensure, Context, Result};
use crysirror_core::state::ClientId;
pub(crate) fn apply(
    table: bool,
    args: &[Vec<u8>],
    vm: &mut Interpreter,
    random: Option<&mut RandomStreams>,
    client: ClientId,
) -> Result<()> {
    let need = if table { 4 } else { 3 };
    let mut staged = vm.clone();
    let mut words = Vec::new();
    for raw in args.iter().take(need) {
        words.push(staged.evaluate(raw)?);
    }
    //The original short-argument branches evaluate available prefix words and
    //return0 without drawing random data. Extra words are not evaluated.
    if words.len() < need {
        *vm = staged;
        return Ok(());
    }
    let random = random.context("CatSystem2 rand requires actual owned seed completion")?;
    ensure!(
        random.belongs_to(client),
        "CatSystem2 foreign FES random stream"
    );
    let mut rng = random.clone();
    if table {
        let [first, last, start, _ignored]: [u32; 4] = words[..4].try_into().unwrap();
        let width = last.wrapping_sub(first).wrapping_add(1);
        ensure!(
            width != 0,
            "CatSystem2 randtbl undefined native zero-width divisor"
        );
        if (first as i32) <= (last as i32) {
            let count = (i64::from(last as i32) - i64::from(first as i32) + 1) as u64;
            ensure!(
                count <= 65536,
                "CatSystem2 randtbl initialization exceeds bounded native-call budget"
            );
            for offset in 0..count as u32 {
                staged.set_flag(first.wrapping_add(offset), start.wrapping_add(offset));
            }
        }
        //Native performs1001 PAIRS, not Fisher-Yates. The fourth argument is
        //evaluated above but intentionally does not control this count.
        for _ in 0..1001 {
            let a = first.wrapping_add(rng.integer_word(client)? % width);
            let b = first.wrapping_add(rng.integer_word(client)? % width);
            let old_a = staged.flag(a);
            let old_b = staged.flag(b);
            staged.set_flag(a, old_b);
            staged.set_flag(b, old_a);
        }
    } else {
        let [destination, min, max]: [u32; 3] = words[..3].try_into().unwrap();
        let width = max.wrapping_sub(min).wrapping_add(1);
        ensure!(
            width != 0,
            "CatSystem2 rand undefined native zero-width divisor"
        );
        staged.set_flag(
            destination,
            min.wrapping_add(rng.integer_word(client)? % width),
        );
    }
    *vm = staged;
    *random = rng;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn args(words: &[&[u8]]) -> Vec<Vec<u8>> {
        words.iter().map(|word| word.to_vec()).collect()
    }
    #[test]
    fn inclusive_wrapping_range_prefix_tail_and_failure_keep_original_stream_order() -> Result<()> {
        let client = ClientId(1);
        let mut vm = Interpreter::new(32)?;
        let mut random = RandomStreams::new(client, 11, 17);
        let first = crysirror_resource::legacy_stream::Mt19937Split::first_u32(17);
        assert!(apply(
            false,
            &args(&[b"++$1", b"0", b"-1"]),
            &mut vm,
            Some(&mut random),
            client
        )
        .is_err());
        assert_eq!(
            vm.flag(1),
            0,
            "failed native profile must not commit prefix side effects"
        );
        apply(
            false,
            &args(&[b"5", b"-3", b"-1", b"++$9"]),
            &mut vm,
            Some(&mut random),
            client,
        )?;
        assert_eq!(vm.flag(5), (-3i32 as u32).wrapping_add(first % 3));
        assert_eq!(vm.flag(9), 0);
        apply(false, &args(&[b"++$9"]), &mut vm, None, client)?;
        assert_eq!(vm.flag(9), 1);
        assert!(apply(
            false,
            &args(&[b"5", b"0", b"9"]),
            &mut vm,
            Some(&mut random),
            ClientId(2)
        )
        .is_err());
        Ok(())
    }
    #[test]
    fn table_uses_ignored_fourth_expression_and_exactly1001_pairs_of_shared_draws() -> Result<()> {
        let client = ClientId(1);
        let mut vm = Interpreter::new(32)?;
        let mut random = RandomStreams::new(client, 23, 31);
        apply(
            true,
            &args(&[b"10", b"15", b"100", b"++$9"]),
            &mut vm,
            Some(&mut random),
            client,
        )?;
        assert_eq!(vm.flag(9), 1);
        let mut values = (10..=15).map(|i| vm.flag(i)).collect::<Vec<_>>();
        values.sort_unstable();
        assert_eq!(values, (100..=105).collect::<Vec<_>>());
        let mut reference = crysirror_resource::legacy_stream::Mt19937Split::new(31);
        for _ in 0..2002 {
            reference.next_u32();
        }
        assert_eq!(
            random.integer_word(client)?,
            reference.next_u32(),
            "native table consumes exactly2002 words regardless of fourth value"
        );
        Ok(())
    }
}
