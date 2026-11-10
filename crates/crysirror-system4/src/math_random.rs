// Native seed/output/table policy from xsystem4 src/hll/Math.c, a85467d78e,
// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. The generator is the existing shared Resource kernel.
use crate::vm::external::{Argument, HllReturn};
use anyhow::{Context, Result, ensure};
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
struct ArrayTable {
    slot: i32,
    sequence: u32,
    cursor: usize,
}
#[derive(Clone, Debug)]
pub(crate) struct Random {
    mt: Mt19937_1998,
    tables: BTreeMap<i32, Table>,
    mode: i32,
    array_tables: BTreeMap<i32, ArrayTable>,
}
impl Default for Random {
    fn default() -> Self {
        Self {
            mt: Mt19937_1998::new(4357),
            tables: BTreeMap::new(),
            mode: 0,
            array_tables: BTreeMap::new(),
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
        // Original Math14E0 uses the full raw MT fraction in x87, multiplied
        // by i (not i+1). Do not round the fraction through the RandF ABI.
        for i in (1..values.len()).rev() {
            check()?;
            let selected = (f64::from(mt.next_u32()) / 4294967296.0 * i as f64).trunc() as usize;
            values.swap(i, selected);
        }
        check()
    }
    /// Original Math.dll19A0/1A00 retain a VM integer-array interface. The
    /// table is an alias: initialization writes0..len and every wrap shuffles
    /// the same live VM array. Heap sequence checks replace native dangling
    /// pointers with an explicit error. All data/RNG changes commit together.
    pub(crate) fn dispatch_array_checked(
        &mut self,
        name: &[u8],
        kind: i32,
        args: &[Argument],
        heap: &mut crate::vm::heap::Heap,
        check: &(dyn Fn() -> Result<()> + Sync),
    ) -> Result<Option<HllReturn>> {
        use crate::vm::heap::PageKind;
        if !matches!(name, b"RandTable2Init" | b"RandTable2") {
            return Ok(None);
        }
        check()?;
        let (id, init_slot) = match (name, kind, args) {
            (
                b"RandTable2Init",
                0,
                [
                    Argument::Scalar { kind: 10, bits: id },
                    Argument::Page { kind: 14, slot },
                ],
            ) => (*id, Some(*slot)),
            (b"RandTable2", 10, [Argument::Scalar { kind: 10, bits: id }]) => (*id, None),
            _ => anyhow::bail!("Math.RandTable2 argument/return ABI"),
        };
        if !(0..=256).contains(&id) {
            return Ok(Some(if init_slot.is_some() {
                HllReturn::Void
            } else {
                HllReturn::Scalar(-1)
            }));
        }
        let binding = if let Some(slot) = init_slot {
            ArrayTable {
                slot,
                sequence: heap.slot(slot)?.sequence,
                cursor: 0,
            }
        } else if let Some(table) = self.array_tables.get(&id) {
            table.clone()
        } else {
            return Ok(Some(HllReturn::Scalar(-1)));
        };
        ensure!(
            heap.slot(binding.slot)?.sequence == binding.sequence,
            "Math.RandTable2 expired VM array"
        );
        let page = heap.page(binding.slot)?;
        ensure!(
            matches!(
                page.kind,
                PageKind::Array {
                    kind: 14,
                    rank: 1,
                    ..
                }
            ),
            "Math.RandTable2 integer-array profile"
        );
        ensure!(
            page.values.len() <= MAX_TABLE_VALUES,
            "Math.RandTable2 array budget"
        );
        let mut values = page.values.clone();
        let mut mt = self.mt.clone();
        let mut cursor = binding.cursor;
        let returned = if init_slot.is_some() {
            for (n, value) in values.iter_mut().enumerate() {
                check()?;
                *value = n as i32;
            }
            HllReturn::Void
        } else {
            let value = *values
                .get(cursor)
                .context("Math.RandTable2 empty/shrunk native array")?;
            cursor += 1;
            HllReturn::Scalar(value)
        };
        if init_slot.is_some() || cursor >= values.len() {
            for n in (1..values.len()).rev() {
                check()?;
                let selected =
                    (f64::from(mt.next_u32()) / 4294967296.0 * n as f64).trunc() as usize;
                values.swap(n, selected);
            }
            cursor = 0;
        }
        check()?;
        heap.page_mut(binding.slot)?.values = values;
        self.mt = mt;
        self.array_tables
            .insert(id, ArrayTable { cursor, ..binding });
        Ok(Some(returned))
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
            // Original Math.dll18F0 retains every integer; 1280/1300 select
            // the four-sample distribution only for exactly1, else uniform.
            b"SetRandMode" => {
                ensure!(kind == 0 && args.len() == 1, "Math.SetRandMode ABI");
                self.mode = integer(&args[0]).context("Math.SetRandMode integer")?;
                HllReturn::Void
            }
            b"Rand" => {
                ensure!(kind == 10 && args.is_empty(), "System4 Math.Rand ABI");
                let value = if self.mode == 1 {
                    let a = f64::from(self.mt.next_u32()) * 0.5;
                    let b = f64::from(self.mt.next_u32()) * 0.5;
                    let c = f64::from(self.mt.next_u32()) * 0.5;
                    let d = f64::from(self.mt.next_u32()) * 0.5;
                    ((a + b - c - d).trunc() as i64) as u32
                } else {
                    self.mt.next_u32()
                };
                HllReturn::Scalar((value & 0x7fff_ffff) as i32)
            }
            b"RandF" => {
                ensure!(kind == 11 && args.is_empty(), "System4 Math.RandF ABI");
                let value = if self.mode == 1 {
                    let a = f64::from(self.mt.next_u32()) / 4294967296.0 * 0.5;
                    let b = f64::from(self.mt.next_u32()) / 4294967296.0 * 0.5;
                    let c = f64::from(self.mt.next_u32()) / 4294967296.0 * 0.5;
                    let d = f64::from(self.mt.next_u32()) / 4294967296.0 * 0.5;
                    (a + b - c - d) as f32
                } else {
                    Self::fraction(&mut self.mt)
                };
                HllReturn::Scalar(value.to_bits() as i32)
            }
            b"RandTableInit" => {
                ensure!(
                    kind == 0 && args.len() == 2,
                    "System4 Math.RandTableInit ABI"
                );
                let id = integer(&args[0]).context("System4 Math.RandTableInit table ID")?;
                if !(0..=256).contains(&id){return Ok(Some(HllReturn::Void));}
                let requested=integer(&args[1]).context("System4 Math.RandTableInit size")?;
                let size=if requested>0{requested as usize}else{self.tables.get(&id).map_or(0,|t|t.values.len())};
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
                // Native1480 leaves an existing allocation and values intact
                // for nonpositive sizes; the following14E0 still reshuffles.
                let mut values=if requested>0{(0..i32::try_from(size)?).collect::<Vec<_>>()}else{self.tables.get(&id).map_or_else(Vec::new,|t|t.values.clone())};
                let mut mt = self.mt.clone();
                Self::shuffle(&mut mt, &mut values, check)?;
                self.mt = mt;
                self.tables.insert(id, Table { values, cursor: 0 });
                HllReturn::Void
            }
            b"RandTable" => {
                ensure!(kind == 10 && args.len() == 1, "System4 Math.RandTable ABI");
                let id = integer(&args[0]).context("System4 Math.RandTable table ID")?;
                if !(0..=256).contains(&id){return Ok(Some(HllReturn::Scalar(-1)));}
                let table=self.tables.get(&id).context("native Math.RandTable read before initialization")?;
                ensure!(!table.values.is_empty(),"native Math.RandTable empty read is undefined");
                let value=table.values[table.cursor];
                if table.cursor+1==table.values.len(){
                    let mut values=table.values.clone();let mut mt=self.mt.clone();
                    Self::shuffle(&mut mt,&mut values,check)?;
                    self.mt=mt;self.tables.insert(id,Table{values,cursor:0});
                }else{self.tables.get_mut(&id).unwrap().cursor+=1;}
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
        assert!(
            random
                .dispatch(b"SetSeedByCurrentTime", 0, &[])
                .unwrap()
                .is_none()
        );
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
        assert_eq!(scalar(&mut random, b"RandTable", 10, &[int(-800)]), -1);
        random
            .dispatch(b"RandTableInit", 0, &[int(9), int(0)])
            .unwrap();
        // Original1480 does not clear an old allocation at size0/negative;
        // 1900 still invokes14E0, resetting and shuffling the retained table.
        assert_eq!(random.tables[&9].values.len(),8);
        assert_eq!(random.tables[&9].cursor,0);
        random.dispatch(b"RandTableInit",0,&[int(9),int(-1)]).unwrap();
        assert_eq!(random.tables[&9].values.len(),8);
        assert!(random.dispatch(b"RandTable",10,&[int(100)]).is_err());
        assert!(
            random
                .dispatch(b"RandTableInit", 0, &[int(9), int(1_000_001)])
                .is_err()
        );
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
        assert!(
            random
                .dispatch_checked(b"RandTableInit", 0, &[int(1), int(100)], &|| {
                    if checks.fetch_add(1, Ordering::Relaxed) > 10 {
                        anyhow::bail!("cancelled")
                    }
                    Ok(())
                })
                .is_err()
        );
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
    #[test]
    fn native_four_sample_mode_consumes_four_words_and_unknown_modes_are_uniform() {
        let mut random = Random::default();
        let mut expected = Mt19937_1998::new(4357);
        let words = std::array::from_fn::<_, 4, _>(|_| f64::from(expected.next_u32()) * 0.5);
        let value = (words[0] + words[1] - words[2] - words[3]).trunc() as i64;
        random.dispatch(b"SetRandMode", 0, &[int(1)]).unwrap();
        assert_eq!(
            scalar(&mut random, b"Rand", 10, &[]),
            (value as u32 & 0x7fff_ffff) as i32
        );
        random.dispatch(b"SetRandMode", 0, &[int(76)]).unwrap();
        assert_eq!(
            scalar(&mut random, b"Rand", 10, &[]),
            (expected.next_u32() & 0x7fff_ffff) as i32
        );
    }
    #[test]
    fn native_array_table_keeps_aliases_visible_and_reshuffles_on_the_last_read() {
        use crate::vm::heap::{Heap, Object, Page, PageKind};
        let mut heap = Heap::new(0).unwrap();
        let slot = heap
            .alloc(Object::Page(Some(Page {
                kind: PageKind::Array {
                    kind: 14,
                    structure: -1,
                    rank: 1,
                },
                values: vec![99, 88, 77, 66],
            })))
            .unwrap();
        let mut random = Random::default();
        random
            .dispatch_array_checked(
                b"RandTable2Init",
                0,
                &[int(2), Argument::Page { kind: 14, slot }],
                &mut heap,
                &|| Ok(()),
            )
            .unwrap();
        let mut values = heap.page(slot).unwrap().values.clone();
        values.sort();
        assert_eq!(values, [0, 1, 2, 3]);
        let first = heap.page(slot).unwrap().values[0];
        let Some(HllReturn::Scalar(value)) = random
            .dispatch_array_checked(b"RandTable2", 10, &[int(2)], &mut heap, &|| Ok(()))
            .unwrap()
        else {
            panic!("value")
        };
        assert_eq!(value, first);
        heap.page_mut(slot).unwrap().values[1] = 42;
        let Some(HllReturn::Scalar(value)) = random
            .dispatch_array_checked(b"RandTable2", 10, &[int(2)], &mut heap, &|| Ok(()))
            .unwrap()
        else {
            panic!("value")
        };
        assert_eq!(value, 42);
        for _ in 0..2 {
            random
                .dispatch_array_checked(b"RandTable2", 10, &[int(2)], &mut heap, &|| Ok(()))
                .unwrap();
        }
        assert_eq!(
            random.array_tables[&2].cursor, 0,
            "rollover belongs to final read"
        );
        assert!(heap.page(slot).unwrap().values.contains(&42));
        heap.free(slot).unwrap();
        let reused = heap
            .alloc(Object::Page(Some(Page {
                kind: PageKind::Array {
                    kind: 14,
                    structure: -1,
                    rank: 1,
                },
                values: vec![0; 4],
            })))
            .unwrap();
        assert_eq!(reused, slot);
        assert!(
            random
                .dispatch_array_checked(b"RandTable2", 10, &[int(2)], &mut heap, &|| Ok(()))
                .is_err()
        );
    }
}
