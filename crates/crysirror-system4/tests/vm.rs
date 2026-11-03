use crysirror_system4::{
    ain::{Function, HllFunction, InitialValue, Library, Program, Structure, Variable},
    vm::{Machine, Step},
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
fn f(name: &str, address: u32, args: u32, vars: Vec<Variable>) -> Function {
    Function {
        name: name.as_bytes().to_vec(),
        address,
        is_label: false,
        return_kind: 10,
        return_structure: -1,
        arguments: args,
        variables: vars,
        crc: 0,
    }
}
fn op(code: &mut Vec<u8>, opcode: u16, args: &[i32]) -> u32 {
    let pc = code.len() as u32;
    code.extend(opcode.to_le_bytes());
    for a in args {
        code.extend(a.to_le_bytes());
    }
    pc
}
fn run(m: &mut Machine) -> anyhow::Result<Step> {
    for _ in 0..10000 {
        let s = m.step()?;
        if s != Step::Advanced {
            return Ok(s);
        }
    }
    anyhow::bail!("authored VM failed to converge")
}
#[test]
fn native_math_pure_hll_runs_with_machine_worker_state_and_clone() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x5a, &[0, 0]);
    op(&mut p.code, 0x5a, &[0, 1]);
    op(&mut p.code, 0x5a, &[0, 1]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    p.libraries = vec![Library {
        name: b"Math".to_vec(),
        functions: vec![
            HllFunction {
                name: b"SetSeed".to_vec(),
                return_kind: 0,
                arguments: vec![(b"seed".to_vec(), 10)],
            },
            HllFunction {
                name: b"Rand".to_vec(),
                return_kind: 10,
                arguments: vec![],
            },
        ],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    // Stop only after first native Rand has published its result and complete
    // the rest from a clone. There is no owner HLL roundtrip or private RNG.
    while m.stack != [(4182529786u32 & 0x7fff_ffff) as i32] {
        assert_eq!(m.step().unwrap(), Step::Advanced);
    }
    let mut clone = m.clone();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(run(&mut clone).unwrap(), Step::Halted);
    assert_eq!(
        m.stack,
        [
            (4182529786u32 & 0x7fff_ffff) as i32,
            (2180050607u32 & 0x7fff_ffff) as i32
        ]
    );
    assert_eq!(m.stack, clone.stack);
}
#[test]
fn native_math_host_time_seed_still_retains_original_external_call() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x5a, &[0, 0]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    p.libraries = vec![Library {
        name: b"Math".to_vec(),
        functions: vec![HllFunction {
            name: b"SetSeedByCurrentTime".to_vec(),
            return_kind: 0,
            arguments: vec![],
        }],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!("host-time capability was fabricated")
    };
    assert_eq!(call.pc, 0);
    assert_eq!(m.pc, 0);
    assert!(m.stack.is_empty());
    assert_eq!(m.step().unwrap(), Step::External(call));
}
#[test]
fn native_nested_peek_lock_returns_real_depth_and_clone_retains_it() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for code in [3, 3, 4, 4, 4] {
        op(&mut p.code, 0x63, &[code]);
    }
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    while m.stack.len() < 2 {
        assert_eq!(m.step().unwrap(), Step::Advanced)
    }
    assert_eq!(m.stack, [1, 2]);
    let mut clone = m.clone();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(run(&mut clone).unwrap(), Step::Halted);
    assert_eq!(m.stack, [1, 2, 1, 0, 0]);
    assert_eq!(clone.stack, m.stack);
}
#[test]
fn native_function_introspection_uses_original_frame_names_and_owned_lookup_string() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x63, &[0x13]);
    op(&mut p.code, 0x63, &[0x1b]);
    op(&mut p.code, 0, &[-1]);
    op(&mut p.code, 0x63, &[0x13]);
    op(&mut p.code, 0x42, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [1]);
    assert_eq!(m.heap.live_slots(), 1);
}
#[test]
fn native_call_moves_arguments_and_return_values() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[8]);
    op(&mut p.code, 0x30, &[1]);
    op(&mut p.code, 0x2f, &[]);
    let callee = p.code.len() as u32;
    op(&mut p.code, 5, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 2, &[]);
    op(&mut p.code, 0, &[3]);
    op(&mut p.code, 9, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("main", 0, 0, vec![]),
        f("add", callee, 1, vec![v(10, -1)]),
    ];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [11]);
    assert_eq!(m.frames.len(), 0);
    assert_eq!(m.heap.live_slots(), 1);
}
#[test]
fn globals_allocated_before_initializers_and_constructors() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    // constructor sees the later global's initial value, writes member0.
    op(&mut p.code, 0x5b, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x5d, &[1]);
    op(&mut p.code, 0x19, &[]);
    op(&mut p.code, 1, &[]);
    op(&mut p.code, 0x2f, &[]);
    let main = p.code.len() as u32;
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 2, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("unused", 0, 0, vec![]),
        f("ctor", 0, 0, vec![]),
        f("main", main, 0, vec![]),
    ];
    p.main = 2;
    p.globals = vec![v(13, 0), v(10, -1)];
    p.initial_values = vec![InitialValue::Integer {
        global: 1,
        kind: 10,
        value: 73,
    }];
    p.structures = vec![Structure {
        name: vec![],
        constructor: 1,
        destructor: -1,
        members: vec![v(10, -1)],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [73]);
    assert!(m.entered_main);
}
#[test]
fn external_fence_and_capture_preserve_exact_pc_stack_identity() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[123]);
    let pc = op(&mut p.code, 0x63, &[22]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let s = run(&mut m).unwrap();
    assert!(matches!(s, Step::External(_)));
    assert_eq!(m.pc, pc);
    assert_eq!(m.stack, [123]);
    let mut restored = m.clone();
    for _ in 0..3 {
        assert_eq!(restored.step().unwrap(), s);
        assert_eq!(restored.pc, pc);
        assert_eq!(restored.stack, [123]);
    }
    assert_eq!(
        restored
            .heap
            .slot(restored.frames[0].local_page)
            .unwrap()
            .sequence,
        m.heap.slot(m.frames[0].local_page).unwrap().sequence
    );
}
#[test]
fn unknown_instruction_fails_without_advance_or_ack() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x4a, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let e = run(&mut m).unwrap_err();
    assert!(format!("{e:#}").contains("SF_CREATE"));
    assert_eq!(m.pc, 0);
    assert!(m.stack.is_empty());
    assert_eq!(m.instructions, 0);
}
#[test]
#[ignore = "owner-installed original AIN; does not execute original PE or HLL library"]
fn owner_rance7_bootstrap() {
    let p = Program::parse(&std::fs::read(std::env::var("CRYSIRROR_TEST_AIN").unwrap()).unwrap())
        .unwrap();
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let dll = std::fs::read(std::env::var("CRYSIRROR_TEST_OUTPUTLOG").unwrap()).unwrap();
    let mut bridge = crysirror_system4::adapter::Bridge::default();
    bridge.debug_log =
        Some(crysirror_system4::adapter::AbsentDebugLog::verify_native_library(&dll).unwrap());
    let mut host = ClockHost(0);
    for _ in 0..2_000_000 {
        match m.step() {
            Ok(Step::Advanced) => {}
            Ok(s) => {
                if let Step::External(ref c) = s {
                    match bridge.dispatch(&mut m, c, &mut host) {
                        Ok(_) => {
                            if let crysirror_system4::vm::ExternalTarget::Hll {
                                library,
                                function,
                            } = c.target
                            {
                                let l = &m.program.libraries[library];
                                println!(
                                    "completed reviewed absent-interface branch {}.{} pc={:#x}",
                                    String::from_utf8_lossy(&l.name),
                                    String::from_utf8_lossy(&l.functions[function].name),
                                    c.pc
                                );
                            }
                            continue;
                        }
                        Err(e) => println!("bridge wall: {e:#}"),
                    }
                }
                println!("{} {s:?}", m.location());
                assert_eq!(m.pc, 0xa4a);
                assert_eq!(m.instructions, 1540);
                assert!(!m.entered_main);
                assert_eq!(bridge.absent_debug_calls, 2);
                assert!(matches!(
                    s,
                    Step::External(crysirror_system4::vm::ExternalCall {
                        target: crysirror_system4::vm::ExternalTarget::System(6),
                        ..
                    })
                ));
                let saved_stack = m.stack.clone();
                assert_eq!(m.step().unwrap(), s);
                assert_eq!(m.stack, saved_stack);
                if let Step::External(c) = s {
                    if let crysirror_system4::vm::ExternalTarget::Hll { library, function } =
                        c.target
                    {
                        let l = &m.program.libraries[library];
                        println!(
                            "HLL {}.{} declaration={:?} args_tail={:?}",
                            String::from_utf8_lossy(&l.name),
                            String::from_utf8_lossy(&l.functions[function].name),
                            l.functions[function],
                            m.stack.iter().rev().take(12).collect::<Vec<_>>()
                        );
                    }
                }
                return;
            }
            Err(e) => panic!("{} unexpected VM error={e:#}", m.location()),
        }
    }
    panic!("owner bootstrap instruction budget");
}

#[derive(Default)]
struct ClockHost(u64);
impl crysirror_core::service::FileSystem for ClockHost {
    fn read(
        &mut self,
        _: crysirror_core::state::ClientId,
        _: &crysirror_core::path::VfsPath,
    ) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::NotFound)
    }
    fn write_atomic(
        &mut self,
        _: crysirror_core::state::ClientId,
        _: &crysirror_core::path::VfsPath,
        _: &[u8],
    ) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
}
impl crysirror_core::service::HostServices for ClockHost {
    fn submit(
        &mut self,
        _: crysirror_core::state::ClientId,
        _: &[crysirror_core::state::Command],
    ) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported("test scene"))
    }
    fn now_ms(&self) -> u64 {
        self.0
    }
}
#[test]
fn shared_clock_sleep_retains_operand_until_actual_deadline() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[25]);
    op(&mut p.code, 0x63, &[21]);
    op(&mut p.code, 0x63, &[13]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(c) = run(&mut m).unwrap() else {
        panic!()
    };
    let mut bridge = crysirror_system4::adapter::Bridge::default();
    let mut host = ClockHost(100);
    assert_eq!(
        bridge.dispatch(&mut m, &c, &mut host).unwrap(),
        crysirror_system4::adapter::Dispatch::WaitUntil(125)
    );
    assert_eq!(m.stack, [25]);
    host.0 = 124;
    assert_eq!(
        bridge.dispatch(&mut m, &c, &mut host).unwrap(),
        crysirror_system4::adapter::Dispatch::WaitUntil(125)
    );
    host.0 = 125;
    bridge.dispatch(&mut m, &c, &mut host).unwrap();
    let Step::External(c) = run(&mut m).unwrap() else {
        panic!()
    };
    bridge.dispatch(&mut m, &c, &mut host).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [125]);
}
#[test]
fn native_reference_arguments_alias_and_keep_global_page_alive() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x30, &[1]);
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x2f, &[]);
    let at = p.code.len() as u32;
    op(&mut p.code, 5, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 3, &[]);
    op(&mut p.code, 0, &[7]);
    op(&mut p.code, 0x19, &[]);
    op(&mut p.code, 1, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("main", 0, 0, vec![]),
        f("write-ref", at, 2, vec![v(18, -1), v(0, -1)]),
    ];
    p.globals = vec![v(10, -1)];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [7]);
    assert_eq!(m.heap.slot(0).unwrap().references, 1);
    assert_eq!(m.heap.live_slots(), 1);
}
#[test]
fn new_and_delete_execute_real_constructor_destructor_continuations() {
    let mut p = Program {
        version: 5,
        main: 1,
        ..Default::default()
    };
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x76, &[]);
    op(&mut p.code, 0x77, &[]);
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x2f, &[]);
    let ctor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0, &[41]);
    op(&mut p.code, 0x19, &[]);
    op(&mut p.code, 1, &[]);
    op(&mut p.code, 0x2f, &[]);
    let dtor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x31, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("unused", 0, 0, vec![]),
        f("main", 0, 0, vec![]),
        f("ctor", ctor, 0, vec![]),
        f("dtor", dtor, 0, vec![]),
    ];
    p.globals = vec![v(10, -1)];
    p.structures = vec![Structure {
        name: vec![],
        constructor: 2,
        destructor: 3,
        members: vec![v(10, -1)],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [42]);
    assert_eq!(m.heap.live_slots(), 1);
}
#[test]
fn original_zero_function_allocates_multidimensional_page_tree_before_main() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 0, &[3]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 0x51, &[]);
    op(&mut p.code, 0x2f, &[]);
    let main = p.code.len() as u32;
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 2, &[]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 2, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.main = 1;
    p.functions = vec![f("0", 0, 0, vec![]), f("main", main, 0, vec![])];
    p.globals = vec![v(14, -1)];
    p.globals[0].rank = 2;
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [0]);
    let parent = m.heap.get(0, 0).unwrap();
    let page = m.heap.page(parent).unwrap();
    assert_eq!(page.values.len(), 2);
    assert_ne!(page.values[0], page.values[1]);
    for child in &page.values {
        assert_eq!(m.heap.page(*child).unwrap().values, [0, 0, 0]);
    }
}
#[test]
fn invalid_debug_dll_is_not_authorized_as_a_stub() {
    assert!(
        crysirror_system4::adapter::AbsentDebugLog::verify_native_library(&[0; 33280]).is_err()
    );
}
#[test]
fn adapter_boot_and_native_execution_use_shared_worker_before_owner_completion() {
    use crysirror_core::{
        path::{Mount, VfsPath},
        service::{ClientStorage, FileSystem, HostServices},
        state::{ClientId, Command},
        vm::{VmAdapter, VmYield},
        Error,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Storage {
        bytes: Vec<u8>,
        owner: std::thread::ThreadId,
        reads: Arc<AtomicUsize>,
    }
    impl ClientStorage for Storage {
        fn read(&self, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            assert_ne!(std::thread::current().id(), self.owner);
            self.reads.fetch_add(1, Ordering::Relaxed);
            Ok(self.bytes.clone())
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
            Err(Error::Unsupported("test listing"))
        }
    }
    struct Host {
        storage: Arc<dyn ClientStorage>,
        owner: std::thread::ThreadId,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::Unsupported("owner I/O"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
    }
    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("test scene"))
        }
        fn now_ms(&self) -> u64 {
            assert_eq!(std::thread::current().id(), self.owner);
            321
        }
        fn input_observation(
            &self,
            client: ClientId,
        ) -> crysirror_core::Result<crysirror_core::service::InputObservation> {
            assert_eq!(client, ClientId(7));
            // Real Runtime returns NotFound before its first observation.
            // An actual VmAdapter InputFrame may still drive worker startup.
            Err(Error::NotFound)
        }
        fn storage(&mut self, client: ClientId) -> crysirror_core::Result<Arc<dyn ClientStorage>> {
            assert_eq!(client, ClientId(7));
            Ok(self.storage.clone())
        }
    }
    let mut code = Vec::new();
    op(&mut code, 0x63, &[13]);
    op(&mut code, 0x2f, &[]);
    let mut bytes = b"VERS".to_vec();
    bytes.extend(5u32.to_le_bytes());
    bytes.extend(b"CODE");
    bytes.extend((code.len() as u32).to_le_bytes());
    bytes.extend(code);
    bytes.extend(b"FUNC");
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(b"main\0");
    for n in [0, 10, u32::MAX, 0, 0, 0] {
        bytes.extend(n.to_le_bytes());
    }
    bytes.extend(b"MAIN");
    bytes.extend(0u32.to_le_bytes());
    let owner = std::thread::current().id();
    let reads = Arc::new(AtomicUsize::new(0));
    let storage = Arc::new(Storage {
        bytes,
        owner,
        reads: reads.clone(),
    });
    let mut host = Host { storage, owner };
    let mut adapter = crysirror_system4::worker_adapter::NativeAdapter::new(
        ClientId(7),
        crysirror_system4::worker_adapter::Config {
            canvas: None,
            font_aliases: ["default".into(), "default".into()],
            program: "test.ain".into(),
            ini: None,
            volumes: Vec::new(),
            reviewed_output_log_dll: None,
        },
    )
    .unwrap();
    let mut halted = false;
    for _ in 0..1000 {
        match adapter.step(&mut host, Default::default(), 32).unwrap() {
            VmYield::Halted => {
                halted = true;
                break;
            }
            _ => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
    assert!(halted);
    assert_eq!(reads.load(Ordering::Relaxed), 1);
    let diagnostic = String::from_utf8(adapter.diagnostics()).unwrap();
    assert!(diagnostic.contains("\"clock_calls\":1"));
    assert!(diagnostic.contains("\"error\":null"));
    assert!(adapter.snapshot().is_err());
}
#[test]
fn local_and_copy_heap_identity_is_reserved_before_owned_children() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x63, &[6]);
    p.functions = vec![f("main", 0, 0, vec![v(12, -1)])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.frames[0].local_page, 1);
    assert_eq!(m.heap.get(1, 0).unwrap(), 2);
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x98, &[0]);
    op(&mut p.code, 0x63, &[6]);
    p.functions = vec![f("main", 0, 0, vec![])];
    p.globals = vec![v(13, 0)];
    p.structures = vec![
        Structure {
            name: vec![],
            constructor: -1,
            destructor: -1,
            members: vec![v(13, 1)],
        },
        Structure {
            name: vec![],
            constructor: -1,
            destructor: -1,
            members: vec![],
        },
    ];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.stack, [4]);
    assert_eq!(m.heap.get(4, 0).unwrap(), 5);
}
#[test]
fn struct_array_allocates_and_constructs_each_element_before_next_identity() {
    let mut p = Program {
        version: 5,
        main: 1,
        ..Default::default()
    };
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 0x51, &[]);
    op(&mut p.code, 0x2f, &[]);
    let main = p.code.len() as u32;
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x2f, &[]);
    let ctor = p.code.len() as u32;
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("0", 0, 0, vec![]),
        f("main", main, 0, vec![]),
        f("ctor", ctor, 0, vec![v(12, -1)]),
    ];
    p.globals = vec![v(17, 0)];
    p.structures = vec![Structure {
        name: vec![],
        constructor: 2,
        destructor: -1,
        members: vec![],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    let array = m.heap.page(m.heap.get(0, 0).unwrap()).unwrap();
    assert_eq!(array.values, [3, 4]);
    assert_eq!(m.heap.slot(3).unwrap().sequence, 4);
    assert_eq!(m.heap.slot(4).unwrap().sequence, 7);
}
#[test]
fn hll_value_argument_cleanup_releases_without_native_destructors() {
    use crysirror_system4::{
        ain::{HllFunction, Library},
        vm::external::HllReturn,
    };
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x76, &[]);
    op(&mut p.code, 0x5a, &[0, 0]);
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x2f, &[]);
    let dtor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x31, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![]), f("dtor", dtor, 0, vec![])];
    p.globals = vec![v(10, -1)];
    p.structures = vec![Structure {
        name: vec![],
        constructor: -1,
        destructor: 1,
        members: vec![],
    }];
    p.libraries = vec![Library {
        name: b"Authored".to_vec(),
        functions: vec![HllFunction {
            name: b"Consume".to_vec(),
            return_kind: 0,
            arguments: vec![(b"object".to_vec(), 13)],
        }],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!()
    };
    assert!(m.complete_hll(&call, HllReturn::Scalar(1)).is_err());
    assert_eq!(m.step().unwrap(), Step::External(call.clone()));
    m.complete_hll(&call, HllReturn::Void).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack, [0]);
    assert_eq!(m.heap.live_slots(), 1);
}

#[test]
fn native_s_mod_uses_shared_byte_format_and_exact_string_argument_release() {
    for (template, arg_kind, arg_string, expected, live_slots) in [
        (b"mixer%03d".as_slice(), 2, None, b"mixer007".as_slice(), 2),
        (
            b"[%4s]".as_slice(),
            4,
            Some(b"ab".as_slice()),
            b"[  ab]".as_slice(),
            2,
        ),
        (
            b"unchanged%d".as_slice(),
            4,
            Some(b"unused".as_slice()),
            b"unchanged%d".as_slice(),
            3,
        ),
    ] {
        let mut p = Program {
            version: 5,
            ..Default::default()
        };
        p.strings.push(template.to_vec());
        op(&mut p.code, 0x41, &[0]);
        if let Some(value) = arg_string {
            p.strings.push(value.to_vec());
            op(&mut p.code, 0x41, &[1]);
        } else {
            op(&mut p.code, 0, &[7]);
        }
        op(&mut p.code, 0, &[arg_kind]);
        op(&mut p.code, 0x94, &[]);
        op(&mut p.code, 0x2f, &[]);
        p.functions = vec![f("main", 0, 0, vec![])];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        assert_eq!(run(&mut m).unwrap(), Step::Halted);
        assert_eq!(m.heap.string(m.stack[0]).unwrap().as_slice(), expected);
        assert_eq!(
            m.heap.live_slots(),
            live_slots,
            "matched %s releases; unmatched string argument remains as in original native source"
        );
    }
}
#[test]
fn unsupported_native_s_mod_does_not_consume_stack_heap_or_pc() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    p.strings.push(b"%D".to_vec());
    op(&mut p.code, 0x41, &[0]);
    op(&mut p.code, 0, &[7]);
    op(&mut p.code, 0, &[2]);
    let pc = op(&mut p.code, 0x94, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    for _ in 0..100 {
        if m.pc == pc {
            break;
        }
        m.step().unwrap();
    }
    assert_eq!(m.pc, pc);
    let stack = m.stack.clone();
    let live = m.heap.live_slots();
    let count = m.instructions;
    assert!(format!("{:#}", m.step().unwrap_err()).contains("unsupported byte format conversion"));
    assert_eq!(m.pc, pc);
    assert_eq!(m.stack, stack);
    assert_eq!(m.heap.live_slots(), live);
    assert_eq!(m.instructions, count);
}

#[test]
fn struct_assignment_keeps_destination_identity_and_resumes_destructor_before_deep_copy() {
    use crysirror_system4::vm::heap::{Object, Page, PageKind};
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x5d, &[1]);
    op(&mut p.code, 0, &[0]); // compiler struct type
    op(&mut p.code, 0x4e, &[]);
    let after = op(&mut p.code, 0x63, &[6]); // inspect without destroying globals
    let dtor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 0x31, &[]);
    op(&mut p.code, 0x63, &[13]); // pending external call inside destructor
    op(&mut p.code, 1, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![]), f("dtor", dtor, 0, vec![])];
    p.globals = vec![v(13, 0), v(13, 0), v(10, -1), v(12, -1)];
    p.structures = vec![
        Structure {
            name: b"parent".to_vec(),
            constructor: -1,
            destructor: 1,
            members: vec![v(12, -1), v(13, 1), v(14, -1), v(20, -1)],
        },
        Structure {
            name: b"child".to_vec(),
            constructor: -1,
            destructor: -1,
            members: vec![v(10, -1)],
        },
    ];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let dst = m.heap.get(0, 0).unwrap();
    let src = m.heap.get(0, 1).unwrap();
    let sequence = m.heap.slot(dst).unwrap().sequence;
    m.heap.reference(dst).unwrap(); // an aliased destination is still overwritten in-place
    let text = m.heap.get(src, 0).unwrap();
    m.heap
        .set_string(text, Arc::new(b"original bytes".to_vec()))
        .unwrap();
    let child = m.heap.get(src, 1).unwrap();
    m.heap.set(child, 0, 73).unwrap();
    let array = m.heap.get(src, 2).unwrap();
    m.heap
        .set_page(
            array,
            Some(Page {
                kind: PageKind::Array {
                    kind: 14,
                    structure: -1,
                    rank: 1,
                },
                values: vec![3, 4],
            }),
        )
        .unwrap();
    let reference = m.heap.get(0, 3).unwrap();
    m.heap.reference(reference).unwrap();
    m.heap.set(src, 3, reference).unwrap();
    let Step::External(call) = run(&mut m).unwrap() else {
        panic!()
    };
    assert_eq!(
        call.target,
        crysirror_system4::vm::ExternalTarget::System(13)
    );
    assert!(
        m.stack.is_empty(),
        "SR_ASSIGN result appears only after the destructor/copy"
    );
    assert_eq!(m.heap.get(0, 2).unwrap(), 1);
    assert_eq!(m.heap.slot(dst).unwrap().sequence, sequence);
    assert_eq!(m.heap.slot(dst).unwrap().references, 2);
    let saved = m.clone();
    for mut resumed in [m, saved] {
        crysirror_system4::adapter::Bridge::default()
            .dispatch(&mut resumed, &call, &mut ClockHost(123))
            .unwrap();
        let Step::External(done) = run(&mut resumed).unwrap() else {
            panic!()
        };
        assert_eq!(done.pc, after);
        assert_eq!(resumed.stack, [src]);
        assert_eq!(resumed.heap.slot(dst).unwrap().sequence, sequence);
        assert_eq!(resumed.heap.slot(dst).unwrap().references, 2);
        let copied_text = resumed.heap.get(dst, 0).unwrap();
        assert_ne!(copied_text, text);
        assert!(Arc::ptr_eq(
            &resumed.heap.string(copied_text).unwrap(),
            &resumed.heap.string(text).unwrap()
        ));
        let copied_child = resumed.heap.get(dst, 1).unwrap();
        assert_ne!(copied_child, child);
        assert_eq!(resumed.heap.get(copied_child, 0).unwrap(), 73);
        resumed.heap.set(copied_child, 0, 99).unwrap();
        assert_eq!(resumed.heap.get(child, 0).unwrap(), 73);
        let copied_array = resumed.heap.get(dst, 2).unwrap();
        assert_ne!(copied_array, array);
        assert_eq!(resumed.heap.page(copied_array).unwrap().values, [3, 4]);
        assert_eq!(resumed.heap.get(dst, 3).unwrap(), reference);
        assert_eq!(resumed.heap.slot(reference).unwrap().references, 3);
        assert!(matches!(
            resumed.heap.slot(copied_child).unwrap().object,
            Object::Page(Some(_))
        ));
    }
}

#[test]
fn struct_self_assignment_is_a_noop_and_invalid_null_assignment_keeps_native_pc_operands() {
    for null in [false, true] {
        let mut p = Program {
            version: 5,
            ..Default::default()
        };
        if null {
            op(&mut p.code, 0, &[-1]);
        } else {
            op(&mut p.code, 0x5d, &[0]);
        }
        op(&mut p.code, 0x5d, &[0]);
        op(&mut p.code, 0, &[987]); // ignored compiler metadata
        let pc = op(&mut p.code, 0x4e, &[]);
        op(&mut p.code, 0x63, &[6]);
        let dtor = p.code.len() as u32;
        op(&mut p.code, 4, &[]);
        op(&mut p.code, 0, &[1]);
        op(&mut p.code, 0x31, &[]);
        op(&mut p.code, 0x2f, &[]);
        p.functions = vec![f("main", 0, 0, vec![]), f("dtor", dtor, 0, vec![])];
        p.globals = vec![v(13, 0), v(10, -1)];
        p.structures = vec![Structure {
            name: vec![],
            constructor: -1,
            destructor: 1,
            members: vec![v(10, -1)],
        }];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        let slot = m.heap.get(0, 0).unwrap();
        let seq = m.heap.slot(slot).unwrap().sequence;
        for _ in 0..100 {
            if m.entered_main && m.pc == pc {
                break;
            }
            m.step().unwrap();
        }
        let before = m.stack.clone();
        let instructions = m.instructions;
        let slots = m.heap.live_slots();
        if null {
            assert!(format!("{:#}", m.step().unwrap_err()).contains("null pointer"));
            assert_eq!(m.pc, pc);
            assert_eq!(m.stack, before);
            assert_eq!(m.instructions, instructions);
            assert_eq!(m.heap.live_slots(), slots);
        } else {
            assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
            assert_eq!(m.stack, [slot]);
            assert_eq!(m.heap.get(0, 1).unwrap(), 0);
            assert_eq!(m.heap.slot(slot).unwrap().sequence, seq);
        }
    }
}

#[test]
fn native_assert_checks_the_condition_and_releases_only_successful_operands() {
    for condition in [0, 1] {
        let mut p = Program {
            version: 5,
            ..Default::default()
        };
        p.strings = vec![b"valid(input)".to_vec(), b"authored.jaf".to_vec()];
        op(&mut p.code, 0, &[condition]);
        op(&mut p.code, 0x41, &[0]);
        op(&mut p.code, 0x41, &[1]);
        op(&mut p.code, 0, &[27]);
        let pc = op(&mut p.code, 0x6f, &[]);
        op(&mut p.code, 0x2f, &[]);
        p.functions = vec![f("main", 0, 0, vec![])];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        for _ in 0..100 {
            if m.entered_main && m.pc == pc {
                break;
            }
            m.step().unwrap();
        }
        let stack = m.stack.clone();
        let count = m.instructions;
        let slots = m.heap.live_slots();
        if condition == 0 {
            let e = format!("{:#}", m.step().unwrap_err());
            assert!(e.contains("authored.jaf:27: valid(input)"));
            assert_eq!(m.pc, pc);
            assert_eq!(m.stack, stack);
            assert_eq!(m.instructions, count);
            assert_eq!(m.heap.live_slots(), slots);
        } else {
            assert_eq!(run(&mut m).unwrap(), Step::Halted);
            assert!(m.stack.is_empty());
            assert_eq!(m.heap.live_slots(), 1);
        }
    }
}
#[test]
fn native_compound_values_use_float_bits_and_saturating_long_assignment() {
    for (opcode, initial, rhs, expected) in [
        (
            0x25,
            1.5f32.to_bits() as i32,
            2.25f32.to_bits() as i32,
            3.75f32.to_bits() as i32,
        ),
        (0xa3, i32::MAX, 100, i32::MAX),
        (0xa4, 1, 100, 0),
        (0xa5, 100000, 100000, i32::MAX),
        (0xa6, 99, 0, 0),
        (0xa8, 7, 3, 3),
    ] {
        let mut p = Program {
            version: 5,
            ..Default::default()
        };
        p.globals = vec![v(10, -1)];
        p.initial_values = vec![InitialValue::Integer {
            global: 0,
            kind: 10,
            value: initial,
        }];
        op(&mut p.code, 4, &[]);
        op(&mut p.code, 0, &[0]);
        op(&mut p.code, 0, &[rhs]);
        op(&mut p.code, opcode, &[]);
        op(&mut p.code, 0x2f, &[]);
        p.functions = vec![f("main", 0, 0, vec![])];
        let mut m = Machine::new(Arc::new(p)).unwrap();
        assert_eq!(run(&mut m).unwrap(), Step::Halted);
        assert_eq!(m.stack, [expected], "native opcode{opcode:#x}");
        assert_eq!(m.heap.get(0, 0).unwrap(), expected);
    }
}
#[test]
fn native_string_compare_is_nul_terminated_and_append_reuses_right_slot_after_release() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    p.strings = vec![b"a\0tail".to_vec(), b"a".to_vec()];
    op(&mut p.code, 0x41, &[0]);
    op(&mut p.code, 0x41, &[1]);
    op(&mut p.code, 0x49, &[]); // S_EQUALE
    op(&mut p.code, 0, &[i32::MIN]);
    op(&mut p.code, 0x6a, &[]); // I_STRING
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.stack[0], 1);
    assert_eq!(
        m.heap.string(m.stack[1]).unwrap().as_slice(),
        b"-2147483648"
    );
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    p.globals = vec![v(12, -1)];
    p.strings = vec![b"A".to_vec(), b"B".to_vec()];
    p.initial_values = vec![InitialValue::String {
        global: 0,
        value: b"A".to_vec(),
    }];
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0x41, &[1]);
    op(&mut p.code, 0x45, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert_eq!(run(&mut m).unwrap(), Step::Halted);
    assert_eq!(m.heap.string(m.stack[0]).unwrap().as_slice(), b"AB");
    assert_eq!(
        m.heap.string(m.heap.get(0, 0).unwrap()).unwrap().as_slice(),
        b"AB"
    );
}
#[test]
fn native_array_free_runs_each_destructor_and_preserves_nullable_root_identity() {
    let mut p = Program {
        version: 5,
        main: 1,
        ..Default::default()
    };
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0, &[2]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 0x51, &[]);
    op(&mut p.code, 0x2f, &[]);
    let main = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0x53, &[]); // A_FREE
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[0]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 0x54, &[]); // A_NUMOF
    op(&mut p.code, 0x63, &[6]);
    let dtor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 0x31, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.functions = vec![
        f("0", 0, 0, vec![]),
        f("main", main, 0, vec![]),
        f("dtor", dtor, 0, vec![]),
    ];
    p.globals = vec![v(17, 0), v(10, -1)];
    p.structures = vec![Structure {
        name: vec![],
        constructor: -1,
        destructor: 2,
        members: vec![v(12, -1)],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let array = m.heap.get(0, 0).unwrap();
    let sequence = m.heap.slot(array).unwrap().sequence;
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.stack, [0]);
    assert_eq!(m.heap.get(0, 1).unwrap(), 2);
    assert!(m.heap.page_opt(array).unwrap().is_none());
    assert_eq!(m.heap.slot(array).unwrap().sequence, sequence);
    assert_eq!(m.heap.slot(array).unwrap().references, 1);
    assert_eq!(m.heap.live_slots(), 3); // global, original array slot, current main local
}

#[test]
fn native_realloc_keeps_root_and_existing_children_and_alloc_replaces_contents() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    // Original rank-two allocation: one row containing two values.
    for x in [0, 0, 1, 2, 2] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    // Reallocate only the outer dimension. The old child retains its native
    // extent, while the newly constructed child uses the new inner dimension.
    for x in [0, 0, 2, 5, 2] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x52, &[]);
    // Pause at the real external clock receipt without completing it.
    // GetFuncStackName is now a pure native operation and is not a sentinel.
    let checkpoint = op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(14, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let root = m.heap.get(0, 0).unwrap();
    let seq = m.heap.slot(root).unwrap().sequence;
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.pc, checkpoint);
    let page = m.heap.page(root).unwrap();
    assert_eq!(page.values.len(), 2);
    assert_eq!(m.heap.page(page.values[0]).unwrap().values.len(), 2);
    assert_eq!(m.heap.page(page.values[1]).unwrap().values.len(), 5);
    assert_eq!(m.heap.slot(root).unwrap().sequence, seq);
    assert_eq!(m.heap.slot(root).unwrap().references, 1);
    assert!(m.stack.is_empty());
}
#[test]
fn native_realloc_zero_and_rank_zero_alloc_are_distinct_from_allocated_empty_page() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    // A_REALLOC(null,0) preserves null. A_ALLOC(0 items) creates a real empty
    // array page; a later rank-zero A_ALLOC releases that page back to null.
    for x in [0, 0, 0, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x52, &[]);
    for x in [0, 1, 0, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    for x in [0, 2, 3, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    for x in [0, 2, 0] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(14, -1), v(14, -1), v(14, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert!(m
        .heap
        .page_opt(m.heap.get(0, 0).unwrap())
        .unwrap()
        .is_none());
    assert!(m
        .heap
        .page(m.heap.get(0, 1).unwrap())
        .unwrap()
        .values
        .is_empty());
    assert!(m
        .heap
        .page_opt(m.heap.get(0, 2).unwrap())
        .unwrap()
        .is_none());
    assert!(m.stack.is_empty());
}
#[test]
fn native_array_replace_and_realloc_shrink_release_owned_strings() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for x in [0, 0, 4, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    for x in [0, 0, 2, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x52, &[]);
    for x in [0, 0, 1, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(16, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let root = m.heap.get(0, 0).unwrap();
    let seq = m.heap.slot(root).unwrap().sequence;
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.heap.page(root).unwrap().values.len(), 1);
    assert!(m
        .heap
        .string(m.heap.get(root, 0).unwrap())
        .unwrap()
        .is_empty());
    assert_eq!(m.heap.live_slots(), 4);
    assert_eq!(m.heap.slot(root).unwrap().sequence, seq);
}
#[test]
fn native_array_push_insert_erase_pop_reverse_preserve_original_insertion_rule() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for value in [10, 20, 30] {
        for x in [0, 0, value] {
            op(&mut p.code, 0, &[x]);
        }
        op(&mut p.code, 0x84, &[]);
    }
    // Native insert clamps a beyond-end index to len-1, not len.
    for x in [0, 0, 999, 25] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x89, &[]);
    for x in [0, 0] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0xb0, &[]);
    for x in [0, 0, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x88, &[]);
    for x in [0, 0] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x85, &[]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(14, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(
        m.heap.page(m.heap.get(0, 0).unwrap()).unwrap().values,
        [30, 20]
    );
    assert_eq!(m.stack, [1]);
}
#[test]
fn native_structure_shorthand_pushback_deep_copies_without_constructor_replay() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x81, &[0, 0]);
    op(&mut p.code, 0xcf, &[0, 0, 99]);
    op(&mut p.code, 0xd1, &[0, 0]);
    op(&mut p.code, 0xcf, &[0, 0, 77]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(17, 0)];
    p.functions = vec![f("main", 0, 0, vec![v(13, 0)])];
    p.structures = vec![Structure {
        name: b"copy".to_vec(),
        constructor: -1,
        destructor: -1,
        members: vec![v(10, -1)],
    }];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    let source = m.heap.get(m.frames.last().unwrap().local_page, 0).unwrap();
    let array = m.heap.get(0, 0).unwrap();
    let copy = m.heap.get(array, 0).unwrap();
    assert_ne!(copy, source);
    assert_eq!(m.heap.get(source, 0).unwrap(), 77);
    assert_eq!(m.heap.get(copy, 0).unwrap(), 99);
}
#[test]
fn native_shorthand_string_references_keep_destination_identity_and_literal_nul_semantics() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    // Write the native local string slot using the original literal, then
    // assign its string data into the existing global string slot.
    op(&mut p.code, 0x5e, &[0]);
    op(&mut p.code, 0xe4, &[0]);
    op(&mut p.code, 0x5d, &[0]);
    op(&mut p.code, 0xe5, &[0]);
    op(&mut p.code, 0xd8, &[0, 1]);
    op(&mut p.code, 0xef, &[0, 1]);
    op(&mut p.code, 0xeb, &[0]);
    op(&mut p.code, 0xdf, &[0]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(12, -1)];
    p.functions = vec![f("main", 0, 0, vec![v(12, -1)])];
    p.strings = vec![b"ab\0c".to_vec(), b"ab\0different".to_vec()];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let global = m.heap.get(0, 0).unwrap();
    let seq = m.heap.slot(global).unwrap().sequence;
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.heap.get(0, 0).unwrap(), global);
    assert_eq!(m.heap.slot(global).unwrap().sequence, seq);
    assert_eq!(m.heap.string(global).unwrap().as_slice(), b"ab\0c");
    assert_eq!(&m.stack[..3], &[1, 0, 0]);
    let alias = *m.stack.last().unwrap();
    assert_ne!(alias, global);
    assert_eq!(m.heap.string(alias).unwrap().as_slice(), b"ab\0c");
}
#[test]
fn native_shorthand_conditional_jump_and_global_assign_use_original_operand_order() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    op(&mut p.code, 0x8c, &[0, 7]);
    let branch = op(&mut p.code, 0xb8, &[0, 9, 0]);
    op(&mut p.code, 0xce, &[0, -1]);
    let jump = op(&mut p.code, 0x2c, &[0]);
    let target = p.code.len() as i32;
    op(&mut p.code, 0xbc, &[0, 0]);
    let stop = p.code.len() as i32;
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.code[branch as usize + 10..branch as usize + 14].copy_from_slice(&target.to_le_bytes());
    p.code[jump as usize + 2..jump as usize + 6].copy_from_slice(&stop.to_le_bytes());
    p.globals = vec![v(10, -1)];
    p.functions = vec![f("main", 0, 0, vec![v(10, -1)])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(m.heap.get(0, 0).unwrap(), 7);
    assert!(m.stack.is_empty());
}
#[test]
fn native_array_fill_clips_and_copy_overlap_is_forward_not_a_snapshot() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for value in [1, 2, 3, 4] {
        for x in [0, 0, value] {
            op(&mut p.code, 0, &[x]);
        }
        op(&mut p.code, 0x84, &[]);
    }
    for x in [0, 0, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x5d, &[0]);
    for x in [0, 3] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x55, &[]);
    for x in [0, 0, -1, 3, 9] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x56, &[]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(14, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    assert_eq!(
        m.heap.page(m.heap.get(0, 0).unwrap()).unwrap().values,
        [9, 9, 1, 1]
    );
    assert_eq!(m.stack, [3, 2]);
}
#[test]
fn native_array_fill_deep_copies_strings_and_finalizes_the_input_value() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for x in [0, 0, 3, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    for x in [0, 0, 0, 3] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x41, &[0]);
    op(&mut p.code, 0x56, &[]);
    op(&mut p.code, 0x63, &[0x0d]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(16, -1)];
    p.functions = vec![f("main", 0, 0, vec![])];
    p.strings = vec![b"value".to_vec()];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    assert!(matches!(run(&mut m).unwrap(), Step::External(_)));
    let page = m.heap.page(m.heap.get(0, 0).unwrap()).unwrap();
    let mut ids = page.values.clone();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3);
    for id in ids {
        assert_eq!(m.heap.string(id).unwrap().as_slice(), b"value");
        assert_eq!(m.heap.slot(id).unwrap().references, 1);
    }
    assert_eq!(m.heap.live_slots(), 6);
    assert_eq!(m.stack, [3]);
}
#[test]
fn native_array_fill_resumes_each_original_destructor_before_publishing_the_replacement() {
    let mut p = Program {
        version: 5,
        ..Default::default()
    };
    for x in [0, 0, 2, 1] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x51, &[]);
    for x in [0, 0, 0, 2, 0] {
        op(&mut p.code, 0, &[x]);
    }
    op(&mut p.code, 0x76, &[]);
    op(&mut p.code, 0x56, &[]);
    let after = op(&mut p.code, 0x63, &[6]);
    op(&mut p.code, 0x2f, &[]);
    let dtor = p.code.len() as u32;
    op(&mut p.code, 4, &[]);
    op(&mut p.code, 0, &[1]);
    op(&mut p.code, 0x31, &[]);
    op(&mut p.code, 0x63, &[13]);
    op(&mut p.code, 1, &[]);
    op(&mut p.code, 0x2f, &[]);
    p.globals = vec![v(17, 0), v(10, -1)];
    p.structures = vec![Structure {
        name: b"item".to_vec(),
        constructor: -1,
        destructor: 1,
        members: vec![],
    }];
    p.functions = vec![f("main", 0, 0, vec![]), f("dtor", dtor, 0, vec![])];
    let mut m = Machine::new(Arc::new(p)).unwrap();
    let Step::External(first) = run(&mut m).unwrap() else {
        panic!()
    };
    assert_eq!(m.heap.get(0, 1).unwrap(), 1);
    assert!(m.stack.is_empty());
    for mut resumed in [m.clone(), m] {
        let mut call = first.clone();
        loop {
            crysirror_system4::adapter::Bridge::default()
                .dispatch(&mut resumed, &call, &mut ClockHost(123))
                .unwrap();
            let Step::External(next) = run(&mut resumed).unwrap() else {
                panic!()
            };
            if next.pc == after {
                break;
            }
            call = next;
        }
        assert_eq!(resumed.heap.get(0, 1).unwrap(), 3);
        assert_eq!(resumed.stack, [2]);
        assert_eq!(
            resumed
                .heap
                .page(resumed.heap.get(0, 0).unwrap())
                .unwrap()
                .values
                .len(),
            2
        );
    }
}
