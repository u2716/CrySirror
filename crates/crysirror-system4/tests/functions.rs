//! Authored native bytecode; no game, font, DLL execution or renderer.
use crysirror_system4::{
    ain::{Function, HllFunction, Library, Program, Structure, Variable},
    vm::{external::HllReturn, Machine, Step},
};
use std::sync::Arc;
fn variable(kind: i32) -> Variable {
    Variable {
        name: vec![],
        kind,
        structure: -1,
        rank: 0,
        group: None,
    }
}
fn function(name: &str, address: u32, arguments: u32, variables: Vec<Variable>) -> Function {
    Function {
        name: name.as_bytes().to_vec(),
        address,
        arguments,
        variables,
        return_kind: 0,
        return_structure: -1,
        is_label: false,
        crc: 0,
    }
}
fn op(p: &mut Program, code: u16, args: &[i32]) -> u32 {
    let address = p.code.len() as u32;
    p.code.extend(code.to_le_bytes());
    for x in args {
        p.code.extend(x.to_le_bytes());
    }
    address
}
fn program() -> Program {
    Program {
        version: 5,
        message_function: -1,
        ..Default::default()
    }
}
fn fence(p: &mut Program) {
    p.libraries = vec![Library {
        name: b"authored-unsupported".to_vec(),
        functions: vec![HllFunction {
            name: b"wait".to_vec(),
            return_kind: 0,
            arguments: vec![],
        }],
    }];
    op(p, 0x5a, &[0, 0]);
}
fn run(machine: &mut Machine) -> anyhow::Result<Step> {
    for _ in 0..10000 {
        let result = machine.step()?;
        if result != Step::Advanced {
            return Ok(result);
        }
    }
    anyhow::bail!("authored VM convergence limit")
}
#[test]
fn message_dispatch_preserves_original_id_count_bytes_and_waiting_owned_frame() {
    let mut p = program();
    op(&mut p, 0x59, &[0]);
    op(&mut p, 0, &[99]);
    op(&mut p, 0x2f, &[]);
    let address = p.code.len() as u32;
    fence(&mut p);
    op(&mut p, 0x2f, &[]);
    p.functions = vec![
        function("main", 0, 0, vec![]),
        function(
            "message",
            address,
            3,
            vec![variable(10), variable(10), variable(12)],
        ),
    ];
    p.message_function = 1;
    p.messages = vec![vec![0x82, 0xa0, 0, b'x'], b"second".to_vec()];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!("message bypassed native function")
    };
    let page = m.frames.last().unwrap().local_page;
    let text = m.heap.get(page, 2).unwrap();
    assert_eq!(m.heap.get(page, 0).unwrap(), 0);
    assert_eq!(m.heap.get(page, 1).unwrap(), 2);
    assert_eq!(&**m.heap.string(text).unwrap(), &[0x82, 0xa0, 0, b'x']);
    let mut clone = m.clone();
    for m in [&mut m, &mut clone] {
        assert_eq!(m.step().unwrap(), Step::External(call.clone()));
        m.complete_hll(&call, HllReturn::Void).unwrap();
        assert_eq!(run(m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [99]);
        assert_eq!(m.heap.live_slots(), 1);
    }
}
#[test]
fn message_disabled_is_native_no_handler_and_bad_abi_fails_at_original_pc() {
    let mut p = program();
    op(&mut p, 0x59, &[-1]);
    op(&mut p, 0x2f, &[]);
    p.functions = vec![function("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p.clone())).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    p.message_function = 0;
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(run(&mut m).unwrap_err().to_string().contains("MSG"));
    assert_eq!(m.pc, 0);
    assert!(m.stack.is_empty());
    assert_eq!(m.heap.live_slots(), 2);
}
#[test]
fn function_assignment_returns_same_string_slot_and_uses_c_prefix_zero_fallback() {
    for (name, expected) in [
        (b"callee\0tail".to_vec(), 1),
        (b"main".to_vec(), 0),
        (b"absent".to_vec(), 0),
    ] {
        let mut p = program();
        op(&mut p, 4, &[]); // global page
        op(&mut p, 0, &[0]);
        op(&mut p, 0x41, &[0]);
        op(&mut p, 0, &[17]); // compiler-only type
        let assign = op(&mut p, 0x6e, &[]);
        op(&mut p, 0x42, &[]);
        op(&mut p, 0x2f, &[]);
        let callee = op(&mut p, 0x2f, &[]);
        p.strings = vec![name];
        p.globals = vec![variable(27)];
        p.functions = vec![
            function("main", 0, 0, vec![]),
            function("callee", callee, 0, vec![]),
        ];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        while m.pc != assign {
            assert_eq!(m.step().unwrap(), Step::Advanced);
        }
        let original = m.stack[m.stack.len() - 2];
        let seq = m.heap.slot(original).unwrap().sequence;
        assert_eq!(m.step().unwrap(), Step::Advanced);
        assert_eq!(m.stack, [original]);
        assert_eq!(m.heap.slot(original).unwrap().sequence, seq);
        assert_eq!(m.heap.get(0, 0).unwrap(), expected);
        assert_eq!(run(&mut m).unwrap(), Step::Halted);
        assert_eq!(m.heap.live_slots(), 1);
    }
}
#[test]
fn native_negative_ojmp_keeps_string_and_sjump_releases_destructor_before_label_entry() {
    let mut p = program();
    op(&mut p, 5, &[]);
    op(&mut p, 0, &[0]);
    op(&mut p, 0, &[0]);
    op(&mut p, 0x76, &[]); // owned structure local
    op(&mut p, 0x19, &[]);
    op(&mut p, 1, &[]);
    op(&mut p, 0x41, &[0]);
    let call = op(&mut p, 0x65, &[]);
    let jump = op(&mut p, 0x64, &[]);
    op(&mut p, 0, &[999]);
    op(&mut p, 0x2f, &[]);
    let label = op(&mut p, 0, &[77]);
    op(&mut p, 0x2f, &[]);
    let destructor = p.code.len() as u32;
    fence(&mut p);
    op(&mut p, 0x2f, &[]);
    let mut local = variable(13);
    local.structure = 0;
    p.functions = vec![
        function("main", 0, 0, vec![local]),
        function("scenario", label, 0, vec![variable(12)]),
        function("destructor", destructor, 0, vec![]),
    ];
    p.functions[1].is_label = true;
    p.structures = vec![Structure {
        name: vec![],
        constructor: -1,
        destructor: 2,
        members: vec![],
    }];
    p.object_jump = Some(-1);
    p.strings = vec![b"scenario\0tail".to_vec()];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    while m.pc != call {
        assert_eq!(m.step().unwrap(), Step::Advanced);
    }
    let original = m.stack.clone();
    assert_eq!(m.step().unwrap(), Step::Advanced);
    assert_eq!(m.pc, jump);
    assert_eq!(m.stack, original);
    let Step::External(receipt) = run(&mut m).unwrap() else {
        panic!("destructor was skipped")
    };
    assert_eq!(m.frames.last().unwrap().function, 2);
    assert_eq!(m.frames.first().unwrap().function, 0);
    assert!(m.stack.is_empty());
    let mut clone = m.clone();
    for m in [&mut m, &mut clone] {
        m.complete_hll(&receipt, HllReturn::Void).unwrap();
        assert_eq!(run(m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [77]);
        assert_eq!(m.heap.live_slots(), 1);
    }
}
#[test]
fn native_ojmp_resolver_is_called_and_may_wait_before_returning_its_original_string() {
    let mut p = program();
    op(&mut p, 0x41, &[0]);
    op(&mut p, 0x65, &[]);
    op(&mut p, 0x64, &[]);
    op(&mut p, 0x2f, &[]);
    let resolver = p.code.len() as u32;
    fence(&mut p);
    op(&mut p, 5, &[]);
    op(&mut p, 0, &[0]);
    op(&mut p, 0x46, &[]); // S_REF: result gets a fresh owned slot
    op(&mut p, 0x2f, &[]);
    let label = op(&mut p, 0, &[24]);
    op(&mut p, 0x2f, &[]);
    p.functions = vec![
        function("main", 0, 0, vec![]),
        function("resolver", resolver, 1, vec![variable(12)]),
        function("target", label, 0, vec![]),
    ];
    p.functions[2].is_label = true;
    p.strings = vec![b"target".to_vec()];
    p.object_jump = Some(1);
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!("resolver was guessed instead of called")
    };
    assert_eq!(m.frames.last().unwrap().function, 1);
    m.complete_hll(&call, HllReturn::Void).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [24]);
    assert_eq!(m.heap.live_slots(), 1);
}
#[test]
fn explicit_empty_label_table_never_falls_back_to_native_function_lookup() {
    let mut p = program();
    op(&mut p, 0x41, &[0]);
    let call = op(&mut p, 0x65, &[]);
    op(&mut p, 0x2f, &[]);
    p.functions = vec![function("main", 0, 0, vec![])];
    p.strings = vec![b"main".to_vec()];
    p.scenario_label_section = true;
    p.object_jump = Some(-1);
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(run(&mut m).is_err());
    assert_eq!(m.pc, call);
    assert_eq!(m.stack.len(), 1);
    assert_eq!(&**m.heap.string(m.stack[0]).unwrap(), b"main");
}
