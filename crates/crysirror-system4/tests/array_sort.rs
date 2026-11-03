//! Original native sort/member/callback ownership, independent of game media.
use crysirror_system4::{
    ain::{Function, HllFunction, Library, Program, Structure, Variable},
    vm::{
        external::HllReturn,
        heap::{Object, Page, PageKind},
        Machine, Step,
    },
};
use std::sync::Arc;
fn variable(kind: i32) -> Variable {
    Variable {
        name: vec![],
        kind,
        structure: 0,
        rank: 1,
        group: None,
    }
}
fn function(address: u32, arguments: u32, variables: Vec<Variable>) -> Function {
    Function {
        name: if address == 0 {
            b"main".to_vec()
        } else {
            b"compare".to_vec()
        },
        address,
        arguments,
        variables,
        is_label: false,
        return_kind: 10,
        return_structure: -1,
        crc: 0,
    }
}
fn op(code: &mut Vec<u8>, opcode: u16, args: &[i32]) {
    code.extend(opcode.to_le_bytes());
    for a in args {
        code.extend(a.to_le_bytes());
    }
}
fn fixture(kind: i32, member: bool, custom: bool) -> Machine {
    let mut code = Vec::new();
    op(&mut code, if member { 0xf1 } else { 0x90 }, &[]);
    op(&mut code, 0x2f, &[]);
    let address = code.len() as u32;
    if custom {
        op(&mut code, 0x5a, &[0, 0]);
        if kind == 16 {
            op(&mut code, 0, &[0]);
        } else {
            for parameter in [0, 1] {
                op(&mut code, 0x5e, &[parameter]);
                if kind == 17 {
                    op(&mut code, 0, &[0]);
                    op(&mut code, 0x2, &[]);
                }
            }
            op(&mut code, 0xa, &[]);
        }
        op(&mut code, 0x2f, &[]);
    }
    let element = crysirror_system4::vm::heap::element_kind(kind).unwrap();
    let mut p = Program {
        version: 5,
        code,
        globals: vec![variable(kind)],
        functions: vec![function(0, 0, vec![])],
        ..Default::default()
    };
    if kind == 17 {
        p.structures = vec![Structure {
            name: vec![],
            constructor: -1,
            destructor: -1,
            members: vec![variable(10)],
        }];
    }
    if custom {
        p.functions.push(function(
            address,
            2,
            vec![
                variable(if element == 13 { 20 } else { element }),
                variable(if element == 13 { 20 } else { element }),
            ],
        ));
        p.libraries = vec![Library {
            name: b"authored-fence".to_vec(),
            functions: vec![HllFunction {
                name: b"wait".to_vec(),
                return_kind: 0,
                arguments: vec![],
            }],
        }];
    }
    let mut m = Machine::new(Arc::new(p)).unwrap();
    while !m.entered_main {
        assert_eq!(m.step().unwrap(), Step::Advanced);
    }
    m
}
fn array(m: &mut Machine, kind: i32, values: Vec<i32>) -> i32 {
    let slot = m.heap.get(0, 0).unwrap();
    m.heap
        .set_page(
            slot,
            Some(Page {
                kind: PageKind::Array {
                    kind,
                    structure: 0,
                    rank: 1,
                },
                values,
            }),
        )
        .unwrap();
    slot
}
fn string(m: &mut Machine, b: &[u8]) -> i32 {
    m.heap.alloc(Object::String(Arc::new(b.to_vec()))).unwrap()
}
fn run(m: &mut Machine) -> anyhow::Result<Step> {
    for _ in 0..100_000 {
        let s = m.step()?;
        if s != Step::Advanced {
            return Ok(s);
        }
    }
    anyhow::bail!("sort did not converge")
}
#[test]
fn primitive_sort_preserves_heap_identity_raw_values_and_resumable_state() {
    for (kind, values, expected) in [
        (14, vec![7, -4, 7, 2], vec![-4, 2, 7, 7]),
        (58, vec![9, 2, 8, 1], vec![1, 2, 8, 9]),
    ] {
        let mut m = fixture(kind, false, false);
        let slot = array(&mut m, kind, values.clone());
        let seq = m.heap.slot(slot).unwrap().sequence;
        m.stack.extend([0, 0, 0]);
        m.step().unwrap();
        for _ in 0..4 {
            m.step().unwrap();
        }
        let mut fork = m.clone();
        assert_eq!(run(&mut m).unwrap(), Step::Halted);
        assert_eq!(run(&mut fork).unwrap(), Step::Halted);
        assert_eq!(m.heap.page(slot).unwrap().values, expected);
        assert_eq!(fork.heap.page(slot).unwrap().values, expected);
        assert_eq!(m.heap.slot(slot).unwrap().sequence, seq);
        assert!(m.stack.is_empty());
    }
}
#[test]
fn floating_comparator_preserves_native_nan_ties_and_bit_values() {
    let mut m = fixture(15, false, false);
    let values = [3.0f32, -4.0, 1.0, 2.0]
        .map(|v| v.to_bits() as i32)
        .to_vec();
    let slot = array(&mut m, 15, values);
    m.stack.extend([0, 0, 0]);
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(
        m.heap.page(slot).unwrap().values,
        [-4.0f32, 1.0, 2.0, 3.0].map(|v| v.to_bits() as i32)
    );
    let mut m = fixture(15, false, false);
    let nan = 0x7fc00123i32;
    let slot = array(&mut m, 15, vec![nan, 0]);
    m.stack.extend([0, 0, 0]);
    run(&mut m).unwrap();
    assert_eq!(m.heap.page(slot).unwrap().values, [nan, 0]);
}
#[test]
fn strings_sort_on_c_prefix_without_releasing_or_replacing_original_slots() {
    let mut m = fixture(16, false, false);
    let z = string(&mut m, b"z");
    let a1 = string(&mut m, b"a\0hidden1");
    let a2 = string(&mut m, b"a\0hidden2");
    let slot = array(&mut m, 16, vec![z, a1, a2]);
    m.stack.extend([0, 0, 0]);
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.heap.page(slot).unwrap().values, [a1, a2, z]);
    for s in [z, a1, a2] {
        assert_eq!(m.heap.slot(s).unwrap().references, 1);
    }
}
#[test]
fn member_sort_is_stable_by_native_word_not_float_numeric_interpretation() {
    let mut m = fixture(17, true, false);
    let mut slots = vec![];
    for v in [2, -4, 2, 1] {
        slots.push(
            m.heap
                .alloc(Object::Page(Some(Page {
                    kind: PageKind::Structure(0),
                    values: vec![v],
                })))
                .unwrap(),
        );
    }
    let slot = array(&mut m, 17, slots.clone());
    m.stack.extend([0, 0, 0]);
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(
        m.heap.page(slot).unwrap().values,
        [slots[1], slots[3], slots[0], slots[2]]
    );
}
#[test]
fn custom_comparator_crosses_original_hll_wait_and_publishes_only_after_final_receipt() {
    let mut m = fixture(14, false, true);
    let input = vec![5, 1, 4, 2];
    let slot = array(&mut m, 14, input.clone());
    m.stack.extend([0, 0, 1]);
    let mut calls = 0;
    loop {
        match run(&mut m).unwrap() {
            Step::Halted => break,
            Step::External(c) => {
                calls += 1;
                assert_eq!(m.heap.page(slot).unwrap().values, input);
                assert_eq!(m.step().unwrap(), Step::External(c.clone()));
                m.complete_hll(&c, HllReturn::Void).unwrap();
            }
            _ => panic!(),
        }
    }
    assert!(calls >= 3);
    assert_eq!(m.heap.page(slot).unwrap().values, [1, 2, 4, 5]);
    assert!(m.stack.is_empty());
}
#[test]
fn invalid_member_retains_native_operands_and_callback_reallocation_cannot_publish_stale_values() {
    let mut m = fixture(17, true, false);
    let slot = array(&mut m, 17, vec![]);
    m.stack.extend([0, 0, 9]);
    assert!(m.step().is_err());
    assert_eq!(m.pc, 0);
    assert_eq!(m.stack, [0, 0, 9]);
    assert!(m.heap.page(slot).unwrap().values.is_empty());
    let mut m = fixture(14, false, true);
    let slot = array(&mut m, 14, vec![3, 2]);
    m.stack.extend([0, 0, 1]);
    let Step::External(c) = run(&mut m).unwrap() else {
        panic!()
    };
    m.heap
        .set_page(
            slot,
            Some(Page {
                kind: PageKind::Array {
                    kind: 14,
                    structure: 0,
                    rank: 1,
                },
                values: vec![99],
            }),
        )
        .unwrap();
    m.complete_hll(&c, HllReturn::Void).unwrap();
    let e = run(&mut m).unwrap_err();
    assert!(format!("{e:#}").contains("array layout"));
    assert_eq!(m.heap.page(slot).unwrap().values, [99]);
}

#[test]
fn custom_string_comparators_receive_owned_copies_and_stable_equal_keys() {
    let mut m = fixture(16, false, true);
    let first = string(&mut m, b"b");
    let second = string(&mut m, b"a");
    let third = string(&mut m, b"c");
    let slot = array(&mut m, 16, vec![first, second, third]);
    let baseline = m.heap.live_slots();
    m.stack.extend([0, 0, 1]);
    let mut calls = 0;
    loop {
        match run(&mut m).unwrap() {
            Step::Halted => break,
            Step::External(c) => {
                calls += 1;
                let frame = m.frames.last().unwrap();
                let page = m.heap.page(frame.local_page).unwrap();
                for original_copy in &page.values[..2] {
                    assert!(![first, second, third].contains(original_copy));
                    assert_eq!(m.heap.slot(*original_copy).unwrap().references, 1);
                }
                m.complete_hll(&c, HllReturn::Void).unwrap();
            }
            _ => panic!(),
        }
    }
    assert!(calls > 0);
    assert_eq!(m.heap.page(slot).unwrap().values, [first, second, third]);
    for s in [first, second, third] {
        assert_eq!(m.heap.slot(s).unwrap().references, 1);
    }
    // Main local page is retired on halt; no copied comparator string leaks.
    assert_eq!(m.heap.live_slots(), baseline - 1);
}
#[test]
fn custom_struct_comparator_preserves_borrowed_identity_and_native_refcounts() {
    let mut m = fixture(17, false, true);
    let mut objects = vec![];
    for value in [4, 1, 4, 2] {
        objects.push(
            m.heap
                .alloc(Object::Page(Some(Page {
                    kind: PageKind::Structure(0),
                    values: vec![value],
                })))
                .unwrap(),
        );
    }
    let slot = array(&mut m, 17, objects.clone());
    m.stack.extend([0, 0, 1]);
    loop {
        match run(&mut m).unwrap() {
            Step::Halted => break,
            Step::External(c) => {
                m.complete_hll(&c, HllReturn::Void).unwrap();
            }
            _ => panic!(),
        }
    }
    assert_eq!(
        m.heap.page(slot).unwrap().values,
        [objects[1], objects[3], objects[0], objects[2]]
    );
    for object in objects {
        assert_eq!(m.heap.slot(object).unwrap().references, 1);
    }
}
#[test]
fn empty_custom_sort_does_not_resolve_or_invoke_invalid_comparator() {
    let mut m = fixture(14, false, false);
    array(&mut m, 14, vec![]);
    m.stack.extend([0, 0, -999]);
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert!(m.stack.is_empty());
}

#[test]
fn member_string_sort_uses_nul_keys_and_float_member_sort_uses_raw_signed_words() {
    let mut m = fixture(17, true, false);
    let mut program = (*m.program).clone();
    program.structures[0].members[0].kind = 12;
    m.program = Arc::new(program);
    let mut objects = vec![];
    for bytes in [b"z".as_slice(), b"a\0tail1", b"a\0tail2"] {
        let value = string(&mut m, bytes);
        objects.push(
            m.heap
                .alloc(Object::Page(Some(Page {
                    kind: PageKind::Structure(0),
                    values: vec![value],
                })))
                .unwrap(),
        );
    }
    let slot = array(&mut m, 17, objects.clone());
    m.stack.extend([0, 0, 0]);
    run(&mut m).unwrap();
    assert_eq!(
        m.heap.page(slot).unwrap().values,
        [objects[1], objects[2], objects[0]]
    );
    let mut m = fixture(17, true, false);
    let mut program = (*m.program).clone();
    program.structures[0].members[0].kind = 11;
    m.program = Arc::new(program);
    let positive = m
        .heap
        .alloc(Object::Page(Some(Page {
            kind: PageKind::Structure(0),
            values: vec![0],
        })))
        .unwrap();
    let negative = m
        .heap
        .alloc(Object::Page(Some(Page {
            kind: PageKind::Structure(0),
            values: vec![i32::MIN],
        })))
        .unwrap();
    let slot = array(&mut m, 17, vec![positive, negative]);
    m.stack.extend([0, 0, 0]);
    run(&mut m).unwrap();
    assert_eq!(m.heap.page(slot).unwrap().values, [negative, positive]);
}
#[test]
fn cancelled_continuation_and_reused_heap_slot_cannot_commit_sorted_values() {
    let mut m = fixture(14, false, false);
    let input = vec![9, 4, 7, 2, 1];
    let slot = array(&mut m, 14, input.clone());
    m.stack.extend([0, 0, 0]);
    m.step().unwrap();
    let error = m
        .step_with_check(&|| anyhow::bail!("authored cancelled"))
        .unwrap_err();
    assert!(format!("{error:#}").contains("authored cancelled"));
    assert_eq!(m.heap.page(slot).unwrap().values, input);
    assert_eq!(m.pc, 0);
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.heap.page(slot).unwrap().values, [1, 2, 4, 7, 9]);
    let mut m = fixture(14, false, true);
    let slot = array(&mut m, 14, vec![3, 2]);
    let original = m.heap.slot(slot).unwrap().sequence;
    m.stack.extend([0, 0, 1]);
    let Step::External(c) = run(&mut m).unwrap() else {
        panic!()
    };
    m.heap.free(slot).unwrap();
    let replacement = m
        .heap
        .alloc(Object::Page(Some(Page {
            kind: PageKind::Array {
                kind: 14,
                structure: 0,
                rank: 1,
            },
            values: vec![99, 100],
        })))
        .unwrap();
    assert_eq!(replacement, slot);
    assert_ne!(m.heap.slot(slot).unwrap().sequence, original);
    m.complete_hll(&c, HllReturn::Void).unwrap();
    assert!(format!("{:#}", run(&mut m).unwrap_err()).contains("array identity"));
    assert_eq!(m.heap.page(slot).unwrap().values, [99, 100]);
}
