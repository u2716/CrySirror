// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original759E50 OBJECT-row layout. These are VM DWORDs/bytes and logical
//! references, never pointers, general image state or a second scene graph.
use crate::{fes_values, native_interpreter::Interpreter};
use anyhow::{ensure, Result};

use crate::fes_references::IndexedName;

#[derive(Clone, Debug)]
pub(crate) struct ObjectValues {
    bytes: [u8; 896],
    ///None is an observed missing native parent. Presence is independently
    ///represented by the original bit0x40. Some holds an adapter logical ID,
    ///never the original executable's object pointer or an engine device ID.
    pub parent: Option<u32>,
}
impl ObjectValues {
    pub fn word(&self, index: usize) -> u32 {
        u32::from_le_bytes(self.bytes[index * 4..index * 4 + 4].try_into().unwrap())
    }
    fn set_word(&mut self, index: usize, value: u32) {
        self.bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn bit(&mut self, word: usize, mask: u32) {
        self.set_word(word, self.word(word) | mask);
    }
    pub fn string(&self, offset: usize) -> &[u8] {
        let bytes = &self.bytes[offset..];
        &bytes[..bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len())]
    }
    fn set_string(&mut self, offset: usize, capacity: usize, value: &[u8]) -> Result<()> {
        ensure!(
            value.len() < capacity && !value.contains(&0),
            "CatSystem2 OBJECT string exceeds reviewed native field extent/NUL"
        );
        self.bytes[offset..offset + capacity].fill(0);
        self.bytes[offset..offset + value.len()].copy_from_slice(value);
        Ok(())
    }
}

///Decode one complete admitted row transactionally. References use the caller's
///actual name/array/string-bank policy (758D40); parent lookup must distinguish
///known absence from unsupported lookup. No callback may publish resources.
///Target index has already been resolved once, before this native field phase.
///The caller must still publish class-specific shared resources before accepting
///a DEFINE/OBJECT declaration; decoding this structure alone is not admission.
pub(crate) fn project(
    fields: &[(u8, Vec<u8>)],
    target_index: i32,
    interpreter: &mut Interpreter,
    reference: &mut impl FnMut(&[u8], &mut Interpreter) -> Result<IndexedName>,
    parent: &mut impl FnMut(&IndexedName) -> Result<Option<u32>>,
) -> Result<ObjectValues> {
    ensure!(fields.len() <= 66, "CatSystem2 OBJECT field count");
    let mut vm = interpreter.clone();
    let mut out = ObjectValues {
        bytes: [0; 896],
        parent: None,
    };
    out.set_word(59, 255);
    for (field, raw) in fields {
        ensure!(
            *field < 66 && raw.len() < 512 && !raw.contains(&0),
            "CatSystem2 OBJECT field code/value extent/NUL"
        );
        match *field {
            0 => {
                out.set_string(12, 32, raw)?;
                out.bit(0, 1);
            }
            1 => scalar(&mut out, &mut vm, raw, 11, 0, 0x400)?,
            2..=33 => scalar(
                &mut out,
                &mut vm,
                raw,
                usize::from(*field) + 10,
                2,
                1u32 << (*field - 2),
            )?,
            34 => {
                //POS has a separate presence bit for each provided component.
                for component in 0..3 {
                    let Some(value) = fes_values::component(raw, component, Some(&mut vm))? else {
                        break;
                    };
                    out.set_word(44 + component as usize, value);
                    out.bit(0, 2 << component);
                }
            }
            35 => {
                let value = checked_reference(reference(raw, &mut vm)?)?;
                out.set_string(200, 32, &value.name)?;
                out.set_word(58, value.index as u32);
                out.bit(0, 0x10);
            }
            36 => scalar(&mut out, &mut vm, raw, 59, 0, 0x20)?,
            37 => {
                let value = checked_reference(reference(raw, &mut vm)?)?;
                out.parent = parent(&value)?;
                ensure!(
                    out.parent != Some(0),
                    "CatSystem2 OBJECT parent logical zero is reserved for null"
                );
                //The original DWORD60 is a process pointer. Leave its numeric
                //storage zero, retaining only the explicit logical reference.
                out.bit(0, 0x40);
            }
            38 => tuple(&mut out, &mut vm, raw, 47, 3, 0x80)?,
            39 => tuple(&mut out, &mut vm, raw, 69, 2, 0x100)?,
            40 => tuple(&mut out, &mut vm, raw, 71, 2, 0x200)?,
            41 => {
                scalar(&mut out, &mut vm, raw, 73, 0, 0x800)?;
                if target_index == -1 && out.word(0) & 0x800 != 0 {
                    out.set_word(218, 1);
                }
            }
            42 => scalar(&mut out, &mut vm, raw, 74, 0, 0x1000000)?,
            43 => scalar(&mut out, &mut vm, raw, 75, 0, 0x1000)?,
            44 => scalar(&mut out, &mut vm, raw, 76, 0, 0x2000)?,
            45 => {
                out.set_string(308, 32, raw)?;
                out.bit(0, 0x4000);
            }
            46 => scalar(&mut out, &mut vm, raw, 85, 0, 0x8000)?,
            47 => scalar(&mut out, &mut vm, raw, 86, 0, 0x10000)?,
            48 => scalar(&mut out, &mut vm, raw, 87, 0, 0x20000)?,
            49..=52 => {
                let mut value = checked_reference(reference(raw, &mut vm)?)?;
                //758D40 still evaluates the index before the special name0.
                if value.name == b"0" {
                    value.name.clear();
                    value.index = 0;
                }
                let direction = usize::from(*field - 49);
                out.set_string(656 + direction * 36, 32, &value.name)?;
                out.set_word(172 + direction * 9, value.index as u32);
                out.bit(0, 0x40000 << direction);
            }
            53 | 54 => {
                let value = checked_reference(reference(raw, &mut vm)?)?;
                let (offset, mask) = if *field == 53 {
                    (352, 0x400000)
                } else {
                    (480, 0x800000)
                };
                //Original evaluates the reference index, then copies only its
                //name; no invented index word is written into the row.
                out.set_string(offset, 128, &value.name)?;
                out.bit(0, mask);
            }
            55 => scalar(&mut out, &mut vm, raw, 152, 0, 0x2000000)?,
            56 => tuple(&mut out, &mut vm, raw, 153, 3, 0x4000000)?,
            57 => {
                out.set_string(800, 64, raw)?;
                out.bit(0, 0x8000000);
            }
            58 => {
                out.set_string(624, 32, raw)?;
                out.bit(0, 0x10000000);
            }
            59 => scalar(&mut out, &mut vm, raw, 216, 0, 0x20000000)?,
            60 => scalar(&mut out, &mut vm, raw, 217, 0, 0x40000000)?,
            61 => scalar(&mut out, &mut vm, raw, 219, 0, 0x80000000)?,
            62..=65 => scalar(
                &mut out,
                &mut vm,
                raw,
                usize::from(*field) + 158,
                1,
                1 << (*field - 62),
            )?,
            _ => unreachable!(),
        }
    }
    *interpreter = vm;
    Ok(out)
}
fn checked_reference(value: IndexedName) -> Result<IndexedName> {
    ensure!(
        !value.name.is_empty() && value.name.len() < 32 && !value.name.contains(&0),
        "CatSystem2 OBJECT resolved name extent/NUL"
    );
    Ok(value)
}
fn scalar(
    out: &mut ObjectValues,
    vm: &mut Interpreter,
    raw: &[u8],
    at: usize,
    mask_word: usize,
    mask: u32,
) -> Result<()> {
    if let Some(value) = fes_values::component(raw, 0, Some(vm))? {
        out.set_word(at, value);
        out.bit(mask_word, mask);
    }
    Ok(())
}
fn tuple(
    out: &mut ObjectValues,
    vm: &mut Interpreter,
    raw: &[u8],
    at: usize,
    count: i32,
    mask: u32,
) -> Result<()> {
    for component in 0..count {
        let Some(value) = fes_values::component(raw, component, Some(vm))? else {
            break;
        };
        out.set_word(at + component as usize, value);
        if component == 0 {
            out.bit(0, mask);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reference(raw: &[u8], vm: &mut Interpreter) -> Result<IndexedName> {
        crate::fes_references::evaluate(raw, vm, &Default::default(), &|_| Some(1))
    }
    fn project_fields(fields: &[(u8, Vec<u8>)], vm: &mut Interpreter) -> Result<ObjectValues> {
        project(fields, -1, vm, &mut reference, &mut |value| {
            ensure!(
                value.name == b"parent" && value.index == 3,
                "unknown parent"
            );
            Ok(Some(23))
        })
    }
    #[test]
    fn whole_native_field_layout_preserves_masks_raw_bytes_and_logical_parent() {
        let mut vm = Interpreter::new(8).unwrap();
        let fields: Vec<_> = (0..66)
            .map(|field| {
                let value = match field {
                    0 => b"logo.hg3".to_vec(),
                    35 => b"plane[2]".to_vec(),
                    37 => b"parent[3]".to_vec(),
                    45 => b"default".to_vec(),
                    49 => b"0[99]".to_vec(),
                    50..=52 => b"key[5]".to_vec(),
                    53 | 54 => b"anchor[6]".to_vec(),
                    57 => b"table".to_vec(),
                    58 => b"voice".to_vec(),
                    34 | 38..=40 | 56 => b"11,22,33,++$7".to_vec(),
                    _ => field.to_string().into_bytes(),
                };
                (field, value)
            })
            .collect();
        let out = project_fields(&fields, &mut vm).unwrap();
        assert_eq!(
            (out.word(0), out.word(1), out.word(2)),
            (u32::MAX, 15, u32::MAX)
        );
        assert_eq!(out.string(12), b"logo.hg3");
        for id in 2..=33 {
            assert_eq!(out.word(id + 10), id as u32);
        }
        assert_eq!((out.word(44), out.word(45), out.word(46)), (11, 22, 33));
        assert_eq!((out.word(47), out.word(48), out.word(49)), (11, 22, 33));
        assert_eq!((out.word(69), out.word(70)), (11, 22));
        assert_eq!((out.word(71), out.word(72)), (11, 22));
        assert_eq!(out.word(58), 2);
        assert_eq!(out.string(200), b"plane");
        assert_eq!(out.parent, Some(23));
        assert_eq!(out.word(60), 0);
        assert_eq!(out.word(218), 1);
        assert_eq!(out.word(59), 36);
        assert_eq!(out.string(656), b"");
        assert_eq!(out.word(172), 0);
        assert_eq!(out.string(692), b"key");
        assert_eq!(out.word(181), 5);
        assert_eq!(out.string(352), b"anchor");
        assert_eq!(out.word(120), u32::from_le_bytes(*b"anch"));
        assert_eq!((out.word(153), out.word(154), out.word(155)), (11, 22, 33));
        assert_eq!(out.word(216), 59);
        assert_eq!(out.word(217), 60);
        assert_eq!(out.word(219), 61);
        assert_eq!(out.word(223), 65);
        assert_eq!(vm.flag(7), 0, "unused tuple tail is never evaluated");
    }
    #[test]
    fn tuple_side_effect_order_missing_cells_defaults_and_failures_are_atomic() {
        let mut vm = Interpreter::new(8).unwrap();
        let empty = project_fields(&[], &mut vm).unwrap();
        assert_eq!(
            (empty.word(0), empty.word(59), empty.word(218)),
            (0, 255, 0)
        );
        let out = project_fields(
            &[
                (34, b"++$0,$0+=10".to_vec()),
                (38, b"8,".to_vec()),
                (53, b"anchor[++$0]".to_vec()),
                (61, b"++$0".to_vec()),
            ],
            &mut vm,
        )
        .unwrap();
        assert_eq!((out.word(44), out.word(45), out.word(46)), (1, 11, 0));
        assert_eq!((out.word(47), out.word(48), out.word(49)), (8, 0, 0));
        assert_eq!(vm.flag(0), 13);
        assert_eq!(out.word(219), 13);
        let before = vm.flag(0);
        for fields in [
            vec![(34, b"++$0".to_vec()), (0, vec![b'a'; 32])],
            vec![(34, b"++$0".to_vec()), (66, b"1".to_vec())],
            vec![(34, b"++$0".to_vec()), (37, b"missing".to_vec())],
        ] {
            assert!(project_fields(&fields, &mut vm).is_err());
            assert_eq!(vm.flag(0), before);
        }
        let null = project(
            &[(37, b"missing".to_vec())],
            0,
            &mut vm,
            &mut reference,
            &mut |_| Ok(None),
        )
        .unwrap();
        assert_eq!(null.word(0), 0x40);
        assert_eq!(null.parent, None);
    }
}
