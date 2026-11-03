//! Native array find ownership/callback fixtures, independent of game data.
use crysirror_system4::{
    ain::{Function, HllFunction, Library, Program, Variable},
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
        structure: -1,
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
fn op(code: &mut Vec<u8>, opcode: u16, args: &[i32]) -> u32 {
    let address = code.len() as u32;
    code.extend(opcode.to_le_bytes());
    for x in args {
        code.extend(x.to_le_bytes());
    }
    address
}
fn fixture(kind: i32, borrowed: bool, callback: bool) -> Machine {
    let mut code = Vec::new();
    op(&mut code, if borrowed { 0xd5 } else { 0xaf }, &[]);
    op(&mut code, 0x2f, &[]);
    let comparator = code.len() as u32;
    if callback {
        op(&mut code, 0x5a, &[0, 0]);
        op(&mut code, 0x5e, &[0]);
        op(&mut code, 0x5e, &[1]);
        op(&mut code, 0x18, &[]);
        op(&mut code, 0x2f, &[]);
    }
    let mut functions = vec![function(0, 0, vec![])];
    if callback {
        functions.push(function(comparator, 2, vec![variable(10), variable(10)]));
    }
    let mut p = Program {
        version: 5,
        code,
        globals: vec![variable(kind), variable(12)],
        functions,
        ..Default::default()
    };
    if callback {
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
                    structure: -1,
                    rank: 1,
                },
                values,
            }),
        )
        .unwrap();
    slot
}
fn string(m: &mut Machine, bytes: &[u8]) -> i32 {
    m.heap
        .alloc(Object::String(Arc::new(bytes.to_vec())))
        .unwrap()
}
fn run(m: &mut Machine) -> anyhow::Result<Step> {
    for _ in 0..10000 {
        let s = m.step()?;
        if s != Step::Advanced {
            return Ok(s);
        }
    }
    anyhow::bail!("authored find did not converge")
}
#[test]
fn integer_find_clips_range_and_uses_raw_bits_with_resumable_clone() {
    let mut m = fixture(15, false, false);
    let nan = 0x7fc0_0001;
    array(&mut m, 15, vec![0, nan, 0x7fc0_0002]);
    m.stack = vec![0, 0, -9, 999, nan, 0];
    assert_eq!(m.step().unwrap(), Step::Advanced);
    let mut clone = m.clone();
    for m in [&mut m, &mut clone] {
        assert_eq!(run(m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [1]);
    }
}
#[test]
fn owned_string_find_releases_key_but_keeps_array_slots_and_uses_c_prefix() {
    let mut m = fixture(16, false, false);
    let a = string(&mut m, b"one");
    let b = string(&mut m, b"two\0tail");
    let key = string(&mut m, b"two\0other");
    array(&mut m, 16, vec![a, b]);
    m.stack = vec![0, 0, 0, 2, key, 0];
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [1]);
    assert!(m.heap.slot(key).is_err());
    assert_eq!(&**m.heap.string(b).unwrap(), b"two\0tail");
}
#[test]
fn empty_allocated_string_array_and_null_page_have_original_distinct_key_lifetime() {
    for allocated in [false, true] {
        let mut m = fixture(16, false, false);
        if allocated {
            array(&mut m, 16, vec![]);
        }
        let key = string(&mut m, b"key");
        m.stack = vec![0, 0, 0, 99, key, 0];
        assert_eq!(run(&mut m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [-1]);
        assert_eq!(m.heap.slot(key).is_err(), allocated);
    }
}
#[test]
fn shorthand_borrows_needle_and_never_releases_global_string() {
    let mut m = fixture(16, true, false);
    let key = m.heap.get(0, 1).unwrap();
    m.heap
        .set_string(key, Arc::new(b"needle".to_vec()))
        .unwrap();
    let value = string(&mut m, b"needle");
    array(&mut m, 16, vec![value]);
    m.stack = vec![0, 0, 0, 1, 0, 1];
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [0]);
    assert_eq!(&**m.heap.string(key).unwrap(), b"needle");
}
#[test]
fn comparator_wait_retains_search_and_reads_later_element_after_real_callback_completion() {
    let mut m = fixture(14, false, true);
    let slot = array(&mut m, 14, vec![1, 2, 3]);
    m.stack = vec![0, 0, 0, 3, 7, 1];
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!("native comparator skipped")
    };
    assert_eq!(m.frames.last().unwrap().function, 1);
    m.heap.set(slot, 1, 7).unwrap(); // legal value mutation, not page reallocation
    let mut clone = m.clone();
    for m in [&mut m, &mut clone] {
        m.complete_hll(&call, HllReturn::Void).unwrap();
        let Step::External(second) = run(m).unwrap() else {
            panic!("second comparator skipped")
        };
        m.complete_hll(&second, HllReturn::Void).unwrap();
        assert_eq!(run(m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [1]);
        assert_eq!(m.frames.len(), 0);
    }
}
#[test]
fn comparator_page_reallocation_is_explicit_failure_not_searching_new_storage() {
    for needle in [7, 1] {
        let mut m = fixture(14, false, true);
        let slot = array(&mut m, 14, vec![1, 2]);
        m.stack = vec![0, 0, 0, 2, needle, 1];
        let Step::External(call) = run(&mut m).unwrap() else {
            panic!()
        };
        let mut replacement = m.heap.page(slot).unwrap().clone();
        replacement.values.pop();
        m.heap.set_page(slot, Some(replacement)).unwrap();
        m.complete_hll(&call, HllReturn::Void).unwrap();
        assert!(run(&mut m)
            .unwrap_err()
            .to_string()
            .contains("changed array layout"));
        // No result is acknowledged, even when the callback matched just
        // before it invalidated the retained original C page pointer.
        assert_eq!(m.stack, [i32::from(needle == 1)]);
    }
}

#[test]
fn string_reference_comparator_preserves_array_and_search_key_across_waits() {
    let mut m = fixture(16, false, true);
    let p = Arc::make_mut(&mut m.program);
    let address = p.code.len() as u32;
    op(&mut p.code, 0x5a, &[0, 0]);
    for index in [0, 1] {
        op(&mut p.code, 5, &[]);
        op(&mut p.code, 0, &[index]);
        op(&mut p.code, 0x46, &[]);
    }
    op(&mut p.code, 0x49, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions[1].address = address;
    p.functions[1].variables = vec![variable(20), variable(20)];
    let first = string(&mut m, b"no");
    let second = string(&mut m, b"match");
    let key = string(&mut m, b"match");
    array(&mut m, 16, vec![first, second]);
    m.stack = vec![0, 0, 0, 2, key, 1];
    for _ in 0..2 {
        let Step::External(call) = run(&mut m).unwrap() else {
            panic!("native callback skipped")
        };
        assert!(m.heap.string(key).is_ok());
        m.complete_hll(&call, HllReturn::Void).unwrap();
    }
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [1]);
    assert!(m.heap.slot(key).is_err());
    assert_eq!(&**m.heap.string(first).unwrap(), b"no");
    assert_eq!(&**m.heap.string(second).unwrap(), b"match");
    assert_eq!(m.heap.slot(first).unwrap().references, 1);
    assert_eq!(m.heap.slot(second).unwrap().references, 1);
}
