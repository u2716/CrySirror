// Native seed/output/table policy from xsystem4 src/hll/Math.c, a85467d78e,
// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. The generator is the existing shared Resource kernel.
use crate::vm::external::{Argument, HllReturn};
use anyhow::{ensure, Context, Result};
use crysirror_resource::legacy_stream::Mt19937_1998;
use std::collections::BTreeMap;
const MAX_TABLES: usize = 4096;
const MAX_TABLE_VALUES: usize = 1_000_000;
#[derive(Clone, Debug)]
struct Table {
    values: Vec<i32>,
    cursor: usize,
}
#[derive(Clone, Debug)]
pub(crate) struct Random {
    mt: Mt19937_1998,
    tables: BTreeMap<i32, Table>,
}
impl Default for Random {
    fn default() -> Self {
        Self {
            mt: Mt19937_1998::new(4357),
            tables: BTreeMap::new(),
        }
    }
}
impl Random {
    pub(crate) fn set_seed(&mut self, seed: i32) {
        self.mt = Mt19937_1998::new(seed as u32 | 1);
        self.mt.next_u32();
    }
    fn fraction(mt: &mut Mt19937_1998) -> f32 {
        (f64::from(mt.next_u32()) * (1.0 / 4294967296.0)) as f32
    }
    fn shuffle(
        mt: &mut Mt19937_1998,
        values: &mut [i32],
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<()> {
        // Original loop samples float*i (not i+1); f32 rounding can produce i.
        // Preserve its native permutation policy, not a different fair shuffle.
        for i in (1..values.len()).rev() {
            check()?;
            let selected = (Self::fraction(mt) * i as f32) as usize;
            values.swap(i, selected);
        }
        check()
    }
    #[cfg(test)]
    fn dispatch(&mut self, name: &[u8], kind: i32, args: &[Argument]) -> Result<Option<HllReturn>> {
        self.dispatch_checked(name, kind, args, &|| Ok(()))
    }
    pub(crate) fn dispatch_checked(
        &mut self,
        name: &[u8],
        kind: i32,
        args: &[Argument],
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<Option<HllReturn>> {
        check()?;
        let integer = |argument: &Argument| match argument {
            Argument::Scalar { kind: 10, bits } => Some(*bits),
            _ => None,
        };
        let value = match name {
            b"SetSeed" => {
                ensure!(kind == 0 && args.len() == 1, "System4 Math.SetSeed ABI");
                let seed = integer(&args[0]).context("System4 Math.SetSeed integer argument")?;
                self.set_seed(seed);
                HllReturn::Void
            }
            b"Rand" => {
                ensure!(kind == 10 && args.is_empty(), "System4 Math.Rand ABI");
                HllReturn::Scalar((self.mt.next_u32() & 0x7fff_ffff) as i32)
            }
            b"RandF" => {
                ensure!(kind == 11 && args.is_empty(), "System4 Math.RandF ABI");
                HllReturn::Scalar(Self::fraction(&mut self.mt).to_bits() as i32)
            }
            b"RandTableInit" => {
                ensure!(
                    kind == 0 && args.len() == 2,
                    "System4 Math.RandTableInit ABI"
                );
                let id = integer(&args[0]).context("System4 Math.RandTableInit table ID")?;
                let size =
                    usize::try_from(integer(&args[1]).context("System4 Math.RandTableInit size")?)?;
                let old = self.tables.get(&id).map_or(0, |t| t.values.len());
                let total: usize = self.tables.values().map(|table| table.values.len()).sum();
                ensure!(
                    size <= MAX_TABLE_VALUES && total - old + size <= MAX_TABLE_VALUES,
                    "System4 native random table value budget"
                );
                ensure!(
                    self.tables.contains_key(&id) || self.tables.len() < MAX_TABLES,
                    "System4 native random table count budget"
                );
                let mut values = (0..i32::try_from(size)?).collect::<Vec<_>>();
                let mut mt = self.mt.clone();
                Self::shuffle(&mut mt, &mut values, check)?;
                self.mt = mt;
                self.tables.insert(id, Table { values, cursor: 0 });
                HllReturn::Void
            }
            b"RandTable" => {
                ensure!(kind == 10 && args.len() == 1, "System4 Math.RandTable ABI");
                let id = integer(&args[0]).context("System4 Math.RandTable table ID")?;
                // Source explicitly returns zero for a missing table ID.
                let Some(table) = self.tables.get(&id) else {
                    return Ok(Some(HllReturn::Scalar(0)));
                };
                ensure!(
                    !table.values.is_empty(),
                    "System4 native random table empty read is undefined"
                );
                let needs_shuffle = table.cursor >= table.values.len();
                if needs_shuffle {
                    let mut values = table.values.clone();
                    let mut mt = self.mt.clone();
                    Self::shuffle(&mut mt, &mut values, check)?;
                    self.mt = mt;
                    self.tables.insert(id, Table { values, cursor: 0 });
                }
                let table = self.tables.get_mut(&id).unwrap();
                let value = table.values[table.cursor];
                table.cursor += 1;
                HllReturn::Scalar(value)
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn int(value: i32) -> Argument {
        Argument::Scalar {
            kind: 10,
            bits: value,
        }
    }
    fn scalar(random: &mut Random, name: &[u8], kind: i32, args: &[Argument]) -> i32 {
        let Some(HllReturn::Scalar(value)) = random.dispatch(name, kind, args).unwrap() else {
            panic!("scalar")
        };
        value
    }
    #[test]
    fn original_seed_default_mask_discard_and_clone_use_shared_word_stream() {
        let mut random = Random::default();
        assert_eq!(
            scalar(&mut random, b"Rand", 10, &[]),
            (3510405877u32 & 0x7fff_ffff) as i32
        );
        random.dispatch(b"SetSeed", 0, &[int(0)]).unwrap();
        assert_eq!(
            scalar(&mut random, b"Rand", 10, &[]),
            (4182529786u32 & 0x7fff_ffff) as i32
        );
        let mut clone = random.clone();
        assert_eq!(
            scalar(&mut random, b"RandF", 11, &[]),
            scalar(&mut clone, b"RandF", 11, &[])
        );
        assert!(random.dispatch(b"Rand", 11, &[]).is_err());
        assert!(random
            .dispatch(b"SetSeedByCurrentTime", 0, &[])
            .unwrap()
            .is_none());
    }
    #[test]
    fn original_random_table_permutations_and_rollover_match_c_policy_oracle() {
        let mut random = Random::default();
        random.dispatch(b"SetSeed", 0, &[int(1)]).unwrap();
        random
            .dispatch(b"RandTableInit", 0, &[int(9), int(8)])
            .unwrap();
        // Pinned original MT C kernel plus original Math.c shuffle loop,
        // independently compiled in ignored verification target (not Rust RNG).
        let expected = [7, 0, 1, 5, 2, 4, 3, 6, 5, 7, 0, 2, 3, 1, 6, 4];
        for value in expected {
            assert_eq!(scalar(&mut random, b"RandTable", 10, &[int(9)]), value)
        }
        assert_eq!(scalar(&mut random, b"RandTable", 10, &[int(-800)]), 0);
        random
            .dispatch(b"RandTableInit", 0, &[int(9), int(0)])
            .unwrap();
        assert!(random.dispatch(b"RandTable", 10, &[int(9)]).is_err());
        assert!(random
            .dispatch(b"RandTableInit", 0, &[int(9), int(-1)])
            .is_err());
        assert!(random
            .dispatch(b"RandTableInit", 0, &[int(9), int(1_000_001)])
            .is_err());
    }
    #[test]
    fn cancelled_table_preparation_keeps_previous_stream_and_table() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut random = Random::default();
        random
            .dispatch(b"RandTableInit", 0, &[int(1), int(3)])
            .unwrap();
        let mut before = random.clone();
        let checks = AtomicUsize::new(0);
        assert!(random
            .dispatch_checked(b"RandTableInit", 0, &[int(1), int(100)], &|| {
                if checks.fetch_add(1, Ordering::Relaxed) > 10 {
                    anyhow::bail!("cancelled")
                }
                Ok(())
            })
            .is_err());
        for _ in 0..8 {
            assert_eq!(
                scalar(&mut random, b"RandTable", 10, &[int(1)]),
                scalar(&mut before, b"RandTable", 10, &[int(1)])
            );
            assert_eq!(
                scalar(&mut random, b"Rand", 10, &[]),
                scalar(&mut before, b"Rand", 10, &[])
            );
        }
    }
}
