#![cfg(feature = "native-text")]
use crysirror_system4::{
    ain::{Function, Program},
    vm::{heap::Object, Machine, Step},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn machine(opcode: u16) -> Machine {
    let mut code = Vec::new();
    code.extend(opcode.to_le_bytes());
    code.extend(0x2fu16.to_le_bytes());
    let p = Program {
        version: 5,
        code,
        functions: vec![Function {
            address: 0,
            name: b"main".to_vec(),
            is_label: false,
            return_kind: 0,
            return_structure: -1,
            arguments: 0,
            crc: 0,
            variables: vec![],
        }],
        ..Default::default()
    };
    let mut m = Machine::new(Arc::new(p)).unwrap();
    while !m.entered_main {
        assert_eq!(m.step().unwrap(), Step::Advanced)
    }
    m
}
fn string(m: &mut Machine, bytes: &[u8]) -> i32 {
    m.heap
        .alloc(Object::String(Arc::new(bytes.to_vec())))
        .unwrap()
}
#[test]
fn substring_releases_source_before_allocating_original_result_slot() {
    let mut m = machine(0x8f);
    let source = string(&mut m, &[b'a', 0x81, b'B', b'c']);
    let sequence = m.heap.slot(source).unwrap().sequence;
    m.stack = vec![source, 1, 1];
    assert_eq!(m.step().unwrap(), Step::Advanced);
    assert_eq!(m.pc, 2);
    assert_eq!(m.stack, [source]);
    assert!(m.heap.slot(source).unwrap().sequence > sequence);
    assert_eq!(m.heap.string(source).unwrap().as_slice(), &[0x81, b'B']);
}
#[test]
fn borrowed_character_mutation_keeps_heap_identity_and_cow_reference() {
    let mut m = machine(0x58);
    let slot = string(&mut m, b"abc");
    let snapshot = m.heap.string(slot).unwrap();
    let sequence = m.heap.slot(slot).unwrap().sequence;
    m.stack = vec![slot, 1, 0x4281];
    assert_eq!(m.step().unwrap(), Step::Advanced);
    assert_eq!(m.stack, [0x4281]);
    assert_eq!(m.heap.slot(slot).unwrap().sequence, sequence);
    assert_eq!(snapshot.as_slice(), b"abc");
    assert_eq!(
        m.heap.string(slot).unwrap().as_slice(),
        &[b'a', 0x81, b'B', b'c']
    );
    let mut c = machine(0x57);
    let slot = string(&mut c, &[0xa5]);
    c.stack = vec![slot, 0];
    c.step().unwrap();
    assert_eq!(c.stack, [-91]);
    assert!(c.heap.slot(slot).is_ok());
}
#[test]
fn find_and_length_temporary_release_follow_native_ownership() {
    let mut m = machine(0x8e);
    // S_FIND opcode is independently verified below against the decoder name;
    // this guards an authored opcode fixture from silently testing another op.
    assert_eq!(m.program.instruction(0).unwrap().name, "S_FIND");
    let source = string(&mut m, &[0x81, b'x', b'x']);
    let needle = string(&mut m, b"x");
    m.stack = vec![source, needle];
    m.step().unwrap();
    assert_eq!(m.stack, [1]);
    assert!(m.heap.slot(source).is_err());
    assert!(m.heap.slot(needle).is_err());
    let mut length = machine(0x74);
    let slot = string(&mut length, &[0x81, b'x', b'a']);
    length.stack = vec![slot];
    length.step().unwrap();
    assert_eq!(length.stack, [2]);
    assert!(length.heap.slot(slot).is_err());
}
#[test]
fn numeric_compaction_quirk_and_promoted_float_are_shared_operations() {
    let mut m = machine(0x83);
    let slot = string(&mut m, &[0x82, 0x50, b'2']);
    m.stack = vec![slot];
    m.step().unwrap();
    assert_eq!(m.stack, [122]);
    assert!(m.heap.slot(slot).is_err());
    let mut f = machine(0x93);
    f.stack = vec![1.25f32.to_bits() as i32, -1];
    f.step().unwrap();
    assert_eq!(f.heap.string(f.stack[0]).unwrap().as_slice(), b"1.250000");
}
#[test]
fn malformed_index_and_cancel_leave_original_pc_operands_and_string() {
    let mut m = machine(0x58);
    let slot = string(&mut m, b"abc");
    m.stack = vec![slot, -1, 123];
    assert!(m.step().is_err());
    assert_eq!(m.pc, 0);
    assert_eq!(m.stack, [slot, -1, 123]);
    assert_eq!(m.heap.string(slot).unwrap().as_slice(), b"abc");
    let mut scan = machine(0x74);
    let slot = string(&mut scan, b"a long text");
    scan.stack = vec![slot];
    let calls = AtomicUsize::new(0);
    assert!(scan
        .step_with_check(&|| {
            if calls.fetch_add(1, Ordering::Relaxed) >= 4 {
                anyhow::bail!("authored cancellation")
            }
            Ok(())
        })
        .is_err());
    assert_eq!(scan.pc, 0);
    assert_eq!(scan.stack, [slot]);
    assert_eq!(scan.heap.string(slot).unwrap().as_slice(), b"a long text");
}
