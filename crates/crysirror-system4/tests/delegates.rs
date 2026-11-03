//! Authored original AIN instruction streams, not a hosted VM or game fixture.
use crysirror_system4::{
    ain::{Function, FunctionType, HllFunction, Library, Program, Structure, Variable},
    vm::{
        external::HllReturn,
        heap::{Object, Page, PageKind},
        Machine, Step,
    },
};
use std::sync::Arc;
fn v(kind: i32, structure: i32) -> Variable {
    Variable {
        name: vec![],
        kind,
        structure,
        rank: 0,
        group: None,
    }
}
fn f(name: &str, address: u32, kind: i32, variables: Vec<Variable>) -> Function {
    Function {
        name: name.as_bytes().to_vec(),
        address,
        is_label: false,
        return_kind: kind,
        return_structure: -1,
        arguments: variables.len() as u32,
        variables,
        crc: 0,
    }
}
fn op(code: &mut Vec<u8>, opcode: u16, args: &[i32]) -> u32 {
    let pc = code.len() as u32;
    code.extend(opcode.to_le_bytes());
    for arg in args {
        code.extend(arg.to_le_bytes());
    }
    pc
}
fn push(code: &mut Vec<u8>, value: i32) {
    op(code, 0, &[value]);
}
fn global(code: &mut Vec<u8>, index: i32) {
    op(code, 0x5d, &[index]);
}
fn gate(code: &mut Vec<u8>) -> u32 {
    op(code, 0x5a, &[0, 0])
}
fn base() -> Program {
    Program {
        version: 5,
        libraries: vec![Library {
            name: b"authored_test".to_vec(),
            functions: vec![HllFunction {
                name: b"Yield".to_vec(),
                return_kind: 0,
                arguments: vec![],
            }],
        }],
        ..Default::default()
    }
}
fn run(machine: &mut Machine) -> Step {
    for _ in 0..20000 {
        let step = machine.step().unwrap();
        if step != Step::Advanced {
            return step;
        }
    }
    panic!("delegate stream did not converge")
}
fn external(machine: &mut Machine) -> crysirror_system4::vm::ExternalCall {
    let Step::External(call) = run(machine) else {
        panic!("expected original external receipt")
    };
    call
}
fn complete(machine: &mut Machine, call: &crysirror_system4::vm::ExternalCall) {
    machine.complete_hll(call, HllReturn::Void).unwrap();
}
fn method(code: &mut Vec<u8>, delegate: i32, object: i32, function: i32, opcode: u16) {
    global(code, delegate);
    if object < 0 {
        push(code, -1);
    } else {
        global(code, object);
    }
    push(code, function);
    op(code, opcode, &[]);
}
#[test]
fn complete_delegate_collection_family_preserves_order_duplicates_identity_and_weak_copies() {
    let mut p = base();
    p.globals = vec![v(63, -1), v(63, -1), v(13, 0)];
    p.structures = vec![Structure {
        name: b"obj".to_vec(),
        constructor: -1,
        destructor: -1,
        members: vec![],
    }];
    method(&mut p.code, 0, 2, 1, 0xf2);
    method(&mut p.code, 0, 2, 1, 0xf3);
    method(&mut p.code, 0, 2, 2, 0xf3);
    global(&mut p.code, 0);
    op(&mut p.code, 0xf5, &[]); //2
    method(&mut p.code, 0, 2, 2, 0xf6); //true
    method(&mut p.code, 0, 2, 1, 0xf7);
    global(&mut p.code, 0);
    op(&mut p.code, 0xf5, &[]); //1
    global(&mut p.code, 1);
    global(&mut p.code, 0);
    op(&mut p.code, 0xfa, &[]);
    op(&mut p.code, 1, &[]);
    global(&mut p.code, 0);
    op(&mut p.code, 0xf8, &[]);
    global(&mut p.code, 1);
    op(&mut p.code, 0xf5, &[]); //1, independent assignment
    global(&mut p.code, 0);
    global(&mut p.code, 1);
    op(&mut p.code, 0xfb, &[]);
    op(&mut p.code, 1, &[]);
    global(&mut p.code, 0);
    global(&mut p.code, 1);
    op(&mut p.code, 0xfe, &[]);
    op(&mut p.code, 1, &[]);
    global(&mut p.code, 0);
    op(&mut p.code, 0xf5, &[]); //0
    global(&mut p.code, 1);
    op(&mut p.code, 0xf9, &[]);
    op(&mut p.code, 0x7a, &[]);
    op(&mut p.code, 0xf5, &[]); //copied id,1
    op(&mut p.code, 0x66, &[]);
    op(&mut p.code, 0xfc, &[]); //pop cloned delegate only
    global(&mut p.code, 2);
    push(&mut p.code, 1);
    op(&mut p.code, 0xfd, &[]);
    op(&mut p.code, 0xfc, &[]);
    op(&mut p.code, 0x100, &[]);
    op(&mut p.code, 0xfc, &[]);
    let end = gate(&mut p.code);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let call = external(&mut m);
    assert_eq!(call.pc, end);
    assert_eq!(m.stack, [2, 1, 1, 1, 0, 1]);
    let delegate = m.heap.get(0, 1).unwrap();
    let object = m.heap.get(0, 2).unwrap();
    assert_eq!(m.heap.page(delegate).unwrap().values[..2], [object, 2]);
    assert_eq!(m.heap.slot(object).unwrap().references, 1);
    assert_eq!(m.heap.live_slots(), 5); //global,2 original delegates,object,main local
}
#[test]
fn expired_object_generations_are_collected_without_holding_or_calling_reused_slots() {
    let mut p = base();
    p.globals = vec![v(63, -1), v(13, 0)];
    p.structures = vec![Structure {
        name: b"obj".to_vec(),
        constructor: -1,
        destructor: -1,
        members: vec![],
    }];
    method(&mut p.code, 0, 1, 1, 0xf3);
    gate(&mut p.code);
    method(&mut p.code, 0, 1, 1, 0xf6); //false after original generation retired
    method(&mut p.code, 0, 1, 1, 0xf3); //same slot, new generation allowed
    global(&mut p.code, 0);
    op(&mut p.code, 0xf5, &[]);
    gate(&mut p.code);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let call = external(&mut m);
    let object = m.heap.get(0, 1).unwrap();
    let seq = m.heap.slot(object).unwrap().sequence;
    m.heap.free(object).unwrap();
    let reused = m
        .heap
        .alloc(Object::Page(Some(Page {
            kind: PageKind::Structure(0),
            values: vec![],
        })))
        .unwrap();
    assert_eq!(reused, object);
    assert_ne!(m.heap.slot(reused).unwrap().sequence, seq);
    complete(&mut m, &call);
    external(&mut m);
    assert_eq!(m.stack, [0, 1]);
    let delegate = m.heap.get(0, 0).unwrap();
    assert_eq!(
        m.heap.page(delegate).unwrap().values,
        [object, 1, m.heap.slot(reused).unwrap().sequence as i32]
    );
}
fn callbacks(argument: i32, return_kind: i32) -> (Program, u32, u32, u32) {
    let mut p = base();
    p.globals = vec![v(63, -1)];
    method(&mut p.code, 0, -1, 1, 0xf3);
    method(&mut p.code, 0, -1, 2, 0xf3);
    global(&mut p.code, 0);
    if argument == 12 {
        op(&mut p.code, 0x41, &[0]);
        p.strings = vec![b"original".to_vec()];
    } else {
        push(&mut p.code, 7);
    }
    op(&mut p.code, 0xff, &[0]);
    let call = op(&mut p.code, 0xf4, &[0, 0]);
    op(&mut p.code, 0x2c, &[call as i32]);
    let end = gate(&mut p.code);
    op(&mut p.code, 0x2f, &[]);
    p.code[call as usize + 6..call as usize + 10].copy_from_slice(&(end as i32).to_le_bytes());
    let first = p.code.len() as u32;
    gate(&mut p.code);
    if return_kind != 0 {
        global(&mut p.code, 0);
        op(&mut p.code, 1, &[]);
        op(&mut p.code, 0x5e, &[0]);
        push(&mut p.code, 2);
        op(&mut p.code, 0xb, &[]);
    }
    op(&mut p.code, 0x2f, &[]);
    let second = p.code.len() as u32;
    gate(&mut p.code);
    if return_kind != 0 {
        op(&mut p.code, 0x5e, &[0]);
        push(&mut p.code, 3);
        op(&mut p.code, 0xb, &[]);
    }
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("main", 0, 0, vec![]),
        f("one", first, return_kind, vec![v(argument, -1)]),
        f("two", second, return_kind, vec![v(argument, -1)]),
    ];
    p.delegates = vec![FunctionType {
        name: b"callback".to_vec(),
        return_kind,
        return_structure: -1,
        arguments: 1,
        variables: vec![v(argument, -1)],
    }];
    (p, end, first, second)
}
#[test]
fn delegate_calls_resume_original_callbacks_with_last_result_and_identical_cloned_continuation() {
    let (p, end, first, second) = callbacks(10, 10);
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let call = external(&mut m);
    assert_eq!(call.pc, first);
    assert_eq!(m.stack, [7, m.heap.get(0, 0).unwrap(), 1]);
    assert_eq!(
        m.heap.get(m.frames.last().unwrap().local_page, 0).unwrap(),
        7
    );
    for mut resumed in [m.clone(), m] {
        complete(&mut resumed, &call);
        let next = external(&mut resumed);
        assert_eq!(next.pc, second);
        assert_eq!(
            resumed
                .heap
                .get(resumed.frames.last().unwrap().local_page, 0)
                .unwrap(),
            7
        );
        complete(&mut resumed, &next);
        let done = external(&mut resumed);
        assert_eq!(done.pc, end);
        assert_eq!(resumed.stack, [21]);
        assert_eq!(resumed.frames.len(), 1);
    }
}
#[test]
fn delegate_owned_arguments_are_copied_per_callback_then_released_after_the_original_chain() {
    let (p, end, first, second) = callbacks(12, 0);
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let first_call = external(&mut m);
    assert_eq!(first_call.pc, first);
    let original = m.stack[0];
    let copy = m.heap.get(m.frames.last().unwrap().local_page, 0).unwrap();
    assert_ne!(copy, original);
    assert_eq!(m.heap.string(copy).unwrap().as_slice(), b"original");
    m.heap
        .set_string(copy, Arc::new(b"changed".to_vec()))
        .unwrap();
    assert_eq!(m.heap.string(original).unwrap().as_slice(), b"original");
    complete(&mut m, &first_call);
    let second_call = external(&mut m);
    assert_eq!(second_call.pc, second);
    let second_copy = m.heap.get(m.frames.last().unwrap().local_page, 0).unwrap();
    assert_eq!(m.heap.string(second_copy).unwrap().as_slice(), b"original");
    complete(&mut m, &second_call);
    let done = external(&mut m);
    assert_eq!(done.pc, end);
    assert!(m.stack.is_empty());
    assert!(m.heap.slot(original).is_err());
    assert!(m.heap.slot(second_copy).is_err());
}
#[test]
fn delegate_method_name_signature_matching_and_native_self_subtraction_order_are_retained() {
    let mut p = base();
    p.strings = vec![b"one\0ignored".to_vec(), b"wrong".to_vec()];
    for index in [0, 1] {
        op(&mut p.code, 0x41, &[index]);
        push(&mut p.code, 0);
        op(&mut p.code, 0x101, &[]);
    }
    for function in [1, 2, 3] {
        method(&mut p.code, 0, -1, function, 0xf3);
    }
    global(&mut p.code, 0);
    global(&mut p.code, 0);
    op(&mut p.code, 0xfe, &[]);
    op(&mut p.code, 1, &[]);
    global(&mut p.code, 0);
    op(&mut p.code, 0xf5, &[]);
    gate(&mut p.code);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(63, -1)];
    p.functions = vec![
        f("main", 0, 0, vec![]),
        f("one", 0, 10, vec![]),
        f("wrong", 0, 0, vec![]),
    ];
    p.delegates = vec![FunctionType {
        name: vec![],
        return_kind: 10,
        return_structure: -1,
        arguments: 0,
        variables: vec![],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    external(&mut m);
    assert_eq!(m.stack, [1, 0, 1]);
    let dg = m.heap.get(0, 0).unwrap();
    assert_eq!(m.heap.page(dg).unwrap().values, [-1, 2, 0]);
}
#[test]
fn invalid_delegate_type_or_page_fails_at_original_pc_without_consuming_operands() {
    for operation in [0xff, 0xf4] {
        let mut p = base();
        push(&mut p.code, 42);
        let failed = op(
            &mut p.code,
            operation,
            &if operation == 0xff {
                vec![9]
            } else {
                vec![9, 0]
            },
        );
        p.functions = vec![f("main", 0, 0, vec![])];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        loop {
            match m.step() {
                Ok(Step::Advanced) => continue,
                Err(e) => {
                    assert!(format!("{e:#}").contains("delegate type index"));
                    assert_eq!(m.pc, failed);
                    assert_eq!(m.stack, [42]);
                    break;
                }
                x => panic!("{x:?}"),
            }
        }
    }
    let mut p = base();
    push(&mut p.code, 0);
    let failed = op(&mut p.code, 0xf5, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    loop {
        if let Err(e) = m.step() {
            assert!(format!("{e:#}").contains("delegate triples"));
            assert_eq!(m.pc, failed);
            assert_eq!(m.stack, [0]);
            break;
        }
    }
}

#[test]
fn delegate_method_frames_bind_the_actual_object_without_converting_weak_entries_to_owners() {
    let (mut p, _, first, _) = callbacks(10, 10);
    p.globals.push(v(13, 0));
    p.structures = vec![Structure {
        name: b"obj".to_vec(),
        constructor: -1,
        destructor: -1,
        members: vec![v(10, -1)],
    }];
    // Both same-width object pushes become original SH_GLOBALREF instructions.
    for pc in [6usize, 26] {
        p.code[pc..pc + 2].copy_from_slice(&0x5du16.to_le_bytes());
        p.code[pc + 2..pc + 6].copy_from_slice(&1i32.to_le_bytes());
    }
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let object = m.heap.get(0, 1).unwrap();
    m.heap.set(object, 0, 41).unwrap();
    let call = external(&mut m);
    assert_eq!(call.pc, first);
    assert_eq!(m.frames.last().unwrap().structure_page, object);
    assert_eq!(m.heap.slot(object).unwrap().references, 1);
    for mut resumed in [m.clone(), m] {
        complete(&mut resumed, &call);
        let call = external(&mut resumed);
        assert_eq!(resumed.frames.last().unwrap().structure_page, object);
        assert_eq!(resumed.heap.get(object, 0).unwrap(), 41);
        assert_eq!(resumed.heap.slot(object).unwrap().references, 1);
        complete(&mut resumed, &call);
        external(&mut resumed);
        assert_eq!(resumed.heap.slot(object).unwrap().references, 1);
    }
}
