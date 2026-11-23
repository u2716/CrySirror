// SPDX-License-Identifier: MPL-2.0
use crysirror_cmvs::{
    vm::{decode_at, CmvsVm, InstructionKind, VmProfile, VmYield},
    Script,
};
use crysirror_core::service::MemoryAssetReader;
use std::sync::Arc;

fn script_file(compressed: bool, label: u32) -> Vec<u8> {
    let mut payload = label.to_le_bytes().to_vec();
    payload.extend_from_slice(&[0x40, 0x04, 4, 0, 0x78, 0x56, 0x34, 0x12, 0x1f, 0x20]);
    payload.extend_from_slice(&[0xbb, 0x82, 0xa0, 0]);
    let decoded_len = payload.len() as u32;
    let mut stored = if compressed {
        payload
            .chunks(8)
            .flat_map(|group| std::iter::once(0xff).chain(group.iter().copied()))
            .collect::<Vec<_>>()
    } else {
        payload
    };
    // This key yields a rotate-by-four and XOR byte of 0x24.
    for byte in &mut stored {
        *byte = (byte.rotate_left(4) ^ 0x24).wrapping_add(0x7c);
    }
    let mut file = b"PS2A".to_vec();
    for word in [
        48_u32,
        0x205b8,
        0x0030_0120,
        1,
        10,
        1,
        3,
        0,
        if compressed { stored.len() as u32 } else { 0 },
        decoded_len,
        0,
    ] {
        file.extend_from_slice(&word.to_le_bytes());
    }
    file.extend(stored);
    file
}

#[test]
fn reads_both_disk_layouts_and_decodes_native_operands() {
    for compressed in [false, true] {
        let source = MemoryAssetReader(Arc::from(script_file(compressed, 8)));
        let script = Script::read(&source).unwrap();
        assert_eq!(script.version_raw(), 0x205b8);
        assert_eq!(script.labels().collect::<Vec<_>>(), [8]);
        assert_eq!(script.initial_data(), &[0xbb]);
        assert_eq!(script.text_bytes(), &[0x82, 0xa0, 0]);
        let entry = decode_at(script.code(), script.entry_pc() as usize).unwrap();
        assert_eq!(entry.end, 8);
        assert_eq!(
            entry.kind,
            InstructionKind::StackImmediate {
                bytes: 4,
                raw: 0x12345678,
            }
        );
        assert_eq!(
            decode_at(script.code(), entry.end).unwrap().kind,
            InstructionKind::System { command: 0x1f }
        );
    }
}

#[test]
fn rejects_out_of_range_labels_truncated_files_and_unknown_opcodes() {
    assert!(Script::decode(script_file(false, 9)).is_err());
    let mut truncated = script_file(true, 8);
    truncated.pop();
    assert!(Script::decode(truncated).is_err());
    let error = decode_at(&[0x30, 0x04, 0xff, 0xff], 2).unwrap_err();
    assert_eq!(error.pc, 2);
    assert_eq!(error.opcode, Some(0xffff));
}

#[test]
fn enforces_one_term_budget_across_nested_expressions() {
    let mut code = 0x0200_u16.to_le_bytes().to_vec();
    for _ in 0..64 {
        code.extend_from_slice(&0x0200_u16.to_le_bytes());
        for _ in 0..64 {
            code.extend_from_slice(&0x0100_u16.to_le_bytes());
            code.extend_from_slice(&0_u32.to_le_bytes());
        }
        code.extend_from_slice(&0x020f_u16.to_le_bytes());
    }
    code.extend_from_slice(&0x020f_u16.to_le_bytes());
    let error = decode_at(&code, 0).unwrap_err();
    assert_eq!(error.reason, "instruction expression term budget");
}

fn startup_code(version: u32, code: &[u8], text: &[u8]) -> MemoryAssetReader {
    let mut file = b"PS2A".to_vec();
    for word in [
        48,
        version,
        0,
        0,
        code.len() as u32,
        0,
        text.len() as u32,
        0,
        0,
        (code.len() + text.len()) as u32,
        0,
    ] {
        file.extend_from_slice(&word.to_le_bytes());
    }
    file.extend(
        code.iter()
            .chain(text)
            .map(|byte| byte.rotate_left(1).wrapping_add(0x7c)),
    );
    MemoryAssetReader(Arc::from(file))
}

fn push_integer(code: &mut Vec<u8>, raw: u32) {
    integer_result(code, raw);
    code.extend_from_slice(&0x0430_u16.to_le_bytes());
}

fn integer_result(code: &mut Vec<u8>, raw: u32) {
    code.extend_from_slice(&[0x00, 0x02, 0x00, 0x01]);
    code.extend_from_slice(&raw.to_le_bytes());
    code.extend_from_slice(&0x020f_u16.to_le_bytes());
}

fn push_system_integer(code: &mut Vec<u8>, slot: u32) {
    code.extend_from_slice(&[0x00, 0x02, 0x0f, 0x01]);
    code.extend_from_slice(&slot.to_le_bytes());
    code.extend_from_slice(&[0x0f, 0x02, 0x30, 0x04]);
}

#[test]
fn unrestricted_gesture_controls_use_native_stack_cleanup_and_result_slots() {
    let mut code = Vec::new();
    push_integer(&mut code, 2);
    code.extend_from_slice(&0x2364_u16.to_le_bytes());
    push_integer(&mut code, 128);
    code.extend_from_slice(&0x236b_u16.to_le_bytes());
    push_integer(&mut code, 129);
    code.extend_from_slice(&0x236b_u16.to_le_bytes());
    code.extend_from_slice(&0x236a_u16.to_le_bytes());
    for slot in [0, 4, 5] {
        push_system_integer(&mut code, slot);
    }
    code.extend_from_slice(&0x2000_u16.to_le_bytes());
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &startup_code(1, &code, &[])).unwrap();
    assert_eq!(vm.run(32).unwrap(), VmYield::Frame);
    assert_eq!(vm.parameter(12), Some(u32::MAX));
    assert_eq!(vm.parameter(8), Some(0));
    assert_eq!(vm.parameter(4), Some(0));
    assert_eq!(vm.parameter(16), None);
    vm.begin_frame(16).unwrap();
}

#[test]
fn indirect_call_and_dynamic_return_follow_the_x64_stack_abi() {
    let mut code = Vec::new();
    push_integer(&mut code, 42);
    let target_operand = code.len() + 4;
    integer_result(&mut code, 0);
    code.extend_from_slice(&0x0416_u16.to_le_bytes());
    let resume_pc = code.len();
    code.extend_from_slice(&0x2000_u16.to_le_bytes());
    let target = code.len() as u32;
    code[target_operand..target_operand + 4].copy_from_slice(&target.to_le_bytes());
    // The x64 instruction multiplies AL/AX's 16-bit word by four in place.
    integer_result(&mut code, 0x0000_4001);
    code.extend_from_slice(&0x0413_u16.to_le_bytes());
    let source = startup_code(1, &code, &[]);
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    assert_eq!(vm.run(4).unwrap(), VmYield::BudgetExhausted);
    assert_eq!(vm.pc(), target as usize);
    assert_eq!(vm.parameter(4), Some(resume_pc as u32));
    assert_eq!(vm.parameter(8), Some(42));
    assert_eq!(vm.run(3).unwrap(), VmYield::Frame);
    assert_eq!(vm.pc(), resume_pc + 2);
    assert_eq!(vm.parameter(4), None);
}

#[test]
fn frame_strings_can_use_reserved_bytes_and_resolve_as_native_arguments() {
    let mut code = Vec::new();
    let word = |code: &mut Vec<u8>, v: u16| code.extend_from_slice(&v.to_le_bytes());
    word(&mut code, 0x0440);
    word(&mut code, 8);
    code.extend_from_slice(&0_u32.to_le_bytes());
    word(&mut code, 0x0201);
    word(&mut code, 0x0127);
    code.extend_from_slice(&2_u32.to_le_bytes());
    word(&mut code, 0x0120);
    code.extend_from_slice(&0_u32.to_le_bytes());
    word(&mut code, 0x0170);
    word(&mut code, 0x020f);
    // This native table entry returns without consuming the stack.
    word(&mut code, 0x2024);
    word(&mut code, 0x2000);
    let source = startup_code(1, &code, b"text\0");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    assert_eq!(vm.run(4).unwrap(), VmYield::Frame);
    assert_eq!(vm.text_reference(0xc000_0002), Some(&b"text"[..]));
    assert_eq!(vm.parameter(8), Some(u32::from_le_bytes(*b"\0\0te")));
    assert_eq!(vm.parameter(4), None); // The final reserved byte stayed unknown.
}

#[test]
fn string_syscalls_keep_byte_offsets_and_native_stack_cleanup() {
    let mut text = Vec::new();
    let mut entry = |bytes: &[u8]| {
        let address = text.len() as u32;
        text.extend_from_slice(bytes);
        text.push(0);
        address
    };
    let source = entry(b"Ab\x81B.ab");
    let prefix = entry(b"aB");
    let needle = entry(b"Bignored");
    let pair = entry(b"\x81B");
    let number = entry(b"-1x2");
    let date = entry(b"2026/11/18");
    let extension = entry(b"cpz");
    let pattern = entry(b"[%s4]%%");
    let number_pattern = entry(b"%d+4/%z");
    let padded = entry(b"mask%z3a.pb3");
    let upper_hex = entry(b"%X4");
    let wide = entry(b"%S3");
    let pad_existing = entry(b"x%r!");
    let byte_pattern = entry(b"%c3");
    let octal = entry(b"%lo4");
    let long_decimal = entry(b"%t12");
    let double = entry(b"%f+4.3");
    let implicit_long = entry(b"%t");
    let bank = 0x8000_0000;
    let cases: &[(u16, &[u32], Option<u32>)] = &[
        (0x20f0, &[bank, source], None),
        (0x20f1, &[source, prefix], Some(0)),
        (0x20f2, &[bank, prefix], None),
        (0x20f3, &[bank + 1, source, 2, 1], Some(1)),
        (0x20f4, &[bank + 2, pair, source], Some(2)),
        (0x20f5, &[source, prefix], Some(1)),
        (0x20f6, &[number], Some((-102i32) as u32)),
        (0x20f7, &[source, needle], Some(3)),
        (0x20f8, &[bank], Some(9)),
        (0x20f9, &[bank, needle], Some(8)),
        (0x20fa, &[bank + 3, (-42i32) as u32], None),
        (0x20fb, &[date], Some(48 * 11_111_111 + 20_261_118)),
        (0x20fc, &[bank + 5, padded, 7], None),
        (0x20fd, &[bank + 4, pattern, prefix], None),
        (0x20ff, &[bank, extension], None),
        (0x20fc, &[bank + 7, upper_hex, 0xab], None),
        (0x20fd, &[bank + 8, wide, prefix], None),
        (0x20f0, &[bank + 9, needle], None),
        (0x20fc, &[bank + 9, pad_existing, 0], None),
        (0x20fc, &[bank + 10, byte_pattern, u32::from(b'A')], None),
        (0x20fc, &[bank + 11, octal, 0b101_101], None),
        (0x20fc, &[bank + 12, long_decimal, u32::MAX], None),
        (0x20fc, &[bank + 13, double, u32::MAX], None),
        (0x20fc, &[bank + 14, implicit_long, 7], None),
    ];
    let mut code = Vec::new();
    for &(opcode, args, result) in cases {
        for &argument in args.iter().rev() {
            push_integer(&mut code, argument);
        }
        code.extend_from_slice(&opcode.to_le_bytes());
        if result.is_some() {
            push_system_integer(&mut code, 0);
        }
        code.extend_from_slice(&0x2000_u16.to_le_bytes());
    }
    for argument in [7, number_pattern, bank + 6] {
        push_integer(&mut code, argument);
    }
    let invalid_pc = code.len();
    code.extend_from_slice(&0x20fc_u16.to_le_bytes());
    let script = startup_code(1, &code, &text);
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &script).unwrap();
    let mut stack_bytes = 0;
    for &(opcode, _, result) in cases {
        assert_eq!(vm.run(32).unwrap(), VmYield::Frame, "{opcode:#x}");
        if let Some(result) = result {
            stack_bytes += 4;
            assert_eq!(vm.parameter(4), Some(result), "{opcode:#x}");
        }
        assert_eq!(vm.parameter(stack_bytes + 4), None, "{opcode:#x}");
    }
    for (index, expected) in [
        b"Ab\x81B.cpz".as_slice(),
        b"\x81B",
        b"Ab",
        b"-42",
        b"[aB  ]%",
        b"mask007a.pb3",
        b"",
        b"\x82\x4f\x82\x4f\x82\x60\x82\x61",
        b"\x82\x81\x82\x61  ",
        b"xignored!",
        b"A3",
        b"0055",
        b"  4294967295",
        b"  +0.000",
        b"0000000000000000000",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(vm.text_reference(bank + index as u32), Some(expected));
    }
    let error = vm.run(32).unwrap_err();
    assert_eq!((error.pc, error.opcode), (invalid_pc, Some(0x20fc)));
    assert_eq!(vm.parameter(4), Some(bank + 6));
    assert_eq!(vm.text_reference(bank + 6), Some(&b""[..]));
}

#[test]
fn string_syscalls_preserve_loaded_text_and_bank_bytes_after_a_terminator() {
    let text = b"2026/11/18\0X\0";
    let bank = 0x8000_0000;
    let mut code = Vec::new();
    for args in [[0, bank], [11, bank], [11, 0]] {
        for argument in args {
            push_integer(&mut code, argument);
        }
        code.extend_from_slice(&0x20f0_u16.to_le_bytes());
    }
    // A string expression reads the same updated text segment as syscalls.
    code.extend_from_slice(&[0x01, 0x02, 0x21, 0x01]);
    integer_result(&mut code, 1);
    code.extend_from_slice(&[0x20, 0x01, 0, 0, 0, 0, 0x70, 0x01, 0x0f, 0x02]);
    for reference in [0, bank] {
        push_integer(&mut code, reference);
        code.extend_from_slice(&0x20fb_u16.to_le_bytes());
        push_system_integer(&mut code, 0);
    }
    code.extend_from_slice(&0x2000_u16.to_le_bytes());
    let file = startup_code(1, &code, text);
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &file).unwrap();
    assert_eq!(vm.run(32).unwrap(), VmYield::Frame);
    for reference in [0, bank, bank + 1] {
        assert_eq!(vm.text_reference(reference), Some(&b"X"[..]));
    }
    let raw_sum =
        88 * 10_000_000 + 50 * 100_000 + 54 * 10_000 + 49 * 1000 + 49 * 100 + 49 * 10 + 56;
    assert_eq!(vm.parameter(4), Some(raw_sum));
    assert_eq!(vm.parameter(8), Some(raw_sum));
    let unmodified = CmvsVm::prepare(VmProfile::Pc20160201X64, &file).unwrap();
    assert_eq!(unmodified.text_reference(0), Some(&text[..10]));
    assert_eq!(unmodified.text_reference(bank), Some(&[][..]));
}

#[test]
fn arithmetic_syscalls_keep_native_wrapping_selectors_and_result_slots() {
    let cases: &[(u16, &[u32], &[(u32, u32)])] = &[
        (0x20d5, &[10, 25], &[(0, 18)]),
        (0x20d6, &[5, 0], &[(0, 5)]),
        (0x20d6, &[65536, 2], &[(0, 0)]),
        (0x20d7, &[2000, 500, 0], &[(0, 1000)]),
        (0x20d7, &[400, 500, 1], &[(0, 100)]),
        (0x20d7, &[400, 500, 2], &[(0, 300)]),
        (0x20d7, &[1, 0, 3], &[(0, 300)]),
        (0x20d7, &[u32::MAX, 500, 1], &[(0, i32::MIN as u32)]),
        (0x20d8, &[90, 100], &[(1, 0), (2, 99)]),
        (0x20d9, &[0, 100], &[(1, 100), (2, 0)]),
        (0x20da, &[1, 2, 1, 10], &[(4, 89), (5, 8)]),
        (0x20db, &[1, 2, 1, 10], &[(4, 8999), (5, 8)]),
        (0x20dc, &[0], &[(1, 4096), (2, 0)]),
        (0x201b, &[], &[(0, 0)]),
        (0x20e9, &[], &[(0, 0)]),
        (0x23b5, &[], &[(0, 0)]),
    ];
    let mut code = Vec::new();
    for &(opcode, args, expected) in cases {
        for &argument in args.iter().rev() {
            push_integer(&mut code, argument);
        }
        code.extend_from_slice(&opcode.to_le_bytes());
        for &(slot, _) in expected.iter().rev() {
            push_system_integer(&mut code, slot);
        }
        code.extend_from_slice(&0x2000_u16.to_le_bytes());
    }

    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &startup_code(1, &code, &[])).unwrap();
    let mut expected_stack_words = 0;
    for &(opcode, _, expected) in cases {
        assert_eq!(vm.run(32).unwrap(), VmYield::Frame, "{opcode:#x}");
        for (index, &(_, value)) in expected.iter().enumerate() {
            assert_eq!(vm.parameter(4 * (index + 1)), Some(value), "{opcode:#x}");
        }
        expected_stack_words += expected.len();
        assert_eq!(vm.parameter(4 * (expected_stack_words + 1)), None);
    }
}

#[test]
fn rotate_3d_uses_native_axis_order_and_hundredth_truncation() {
    let mut code = Vec::new();
    for argument in [90.0f32, 90.0, 90.0, 3.4375, -2.1875, 1.125] {
        push_integer(&mut code, argument.to_bits());
    }
    code.extend_from_slice(&0x20e0_u16.to_le_bytes());
    for slot in [10u32, 9, 8] {
        code.extend_from_slice(&[0x02, 0x02, 0x0f, 0x01]);
        code.extend_from_slice(&slot.to_le_bytes());
        code.extend_from_slice(&[0x0f, 0x02, 0x30, 0x04]);
    }
    code.extend_from_slice(&0x2000_u16.to_le_bytes());

    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &startup_code(1, &code, &[])).unwrap();
    assert_eq!(vm.run(32).unwrap(), VmYield::Frame);
    for (index, expected) in [-1.12f32, 3.43, -2.18].into_iter().enumerate() {
        assert_eq!(vm.parameter(4 * (index + 1)), Some(expected.to_bits()));
    }
    assert_eq!(vm.parameter(16), None);
}

#[test]
fn typed_expression_arrays_and_result_banks_preserve_native_conversion() {
    let mut code = Vec::new();
    let word = |code: &mut Vec<u8>, v: u16| code.extend_from_slice(&v.to_le_bytes());
    let scalar = |code: &mut Vec<u8>, tag: u16, bits: u32| {
        word(code, tag);
        code.extend_from_slice(&bits.to_le_bytes());
    };
    word(&mut code, 0x0440);
    word(&mut code, 24);
    code.extend_from_slice(&0_u32.to_le_bytes());
    // base + stride * row + 4 * column puts the float at byte offset 16.
    word(&mut code, 0x0202);
    scalar(&mut code, 0x0137, 0);
    word(&mut code, 8);
    integer_result(&mut code, 1);
    integer_result(&mut code, 2);
    scalar(&mut code, 0x012a, 12.75_f32.to_bits());
    word(&mut code, 0x0170);
    word(&mut code, 0x020f);
    word(&mut code, 0x0202);
    scalar(&mut code, 0x010f, 8);
    scalar(&mut code, 0x0132, 16);
    word(&mut code, 0x0170);
    word(&mut code, 0x020f);
    word(&mut code, 0x0200);
    scalar(&mut code, 0x010f, 8);
    word(&mut code, 0x020f);
    word(&mut code, 0x0430);
    word(&mut code, 0x2000);
    let source = startup_code(1, &code, &[]);
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    assert_eq!(vm.run(10).unwrap(), VmYield::Frame);
    assert_eq!(vm.parameter(4), Some(12));
    assert_eq!(vm.parameter(12), Some(12.75_f32.to_bits()));
}

#[test]
fn explicit_vm_profile_prepares_from_only_a_script_and_keeps_unhandled_arguments() {
    let mut code = Vec::new();
    push_integer(&mut code, 10);
    code.extend_from_slice(&0x201f_u16.to_le_bytes());
    code.extend_from_slice(&0x2000_u16.to_le_bytes());
    push_integer(&mut code, 0);
    push_integer(&mut code, 3);
    code.extend_from_slice(&0x2013_u16.to_le_bytes());
    push_integer(&mut code, 5);
    let pc = code.len();
    code.extend_from_slice(&0x22b3_u16.to_le_bytes());

    let source = startup_code(0x12345, &code, b"music\\\0");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    assert_eq!(vm.run(1).unwrap(), VmYield::BudgetExhausted);
    assert_eq!(vm.run(20).unwrap(), VmYield::Frame);
    assert_eq!(vm.settings().archive_mode, Some(10));
    assert_eq!(vm.parameter(4), None);
    let boundary = VmYield::SystemCall { pc, opcode: 0x22b3 };
    assert_eq!(vm.run(20).unwrap(), boundary);
    assert_eq!(vm.run(20).unwrap(), boundary);
    assert_eq!(vm.parameter(4), Some(5));
    assert_eq!(vm.parameter(8), None);
    let source = vm.resources().sources(0x2013).next().unwrap();
    let crysirror_cmvs::vm::ResourceSource::Directory(path) = source else {
        panic!("BGM source should be registered as a directory");
    };
    assert_eq!(path.relative(), "music");
}

#[test]
fn profile_2021_registers_keyed_archives_and_yields_new_syscalls() {
    let mut code = Vec::new();
    for word in [0x11_u32, 0x22, 0x33, 0x44] {
        push_integer(&mut code, word);
    }
    code.extend_from_slice(&[0x01, 0x02]);
    code.extend_from_slice(&0x120_u16.to_le_bytes());
    code.extend_from_slice(&0_u32.to_le_bytes());
    code.extend_from_slice(&0x20f_u16.to_le_bytes());
    code.extend_from_slice(&0x430_u16.to_le_bytes());
    push_integer(&mut code, 0);
    code.extend_from_slice(&0x2268_u16.to_le_bytes());
    push_integer(&mut code, 1);
    let unresolved = code.len();
    code.extend_from_slice(&0x20e4_u16.to_le_bytes());

    let source = startup_code(0x32083, &code, b"data\\pack\\script.cpz\0");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20211007X64, &source).unwrap();
    assert_eq!(
        vm.run(40).unwrap(),
        VmYield::SystemCall {
            pc: unresolved,
            opcode: 0x20e4
        }
    );
    let sources = vm.resources().script_sources();
    let [crysirror_cmvs::vm::ResourceSource::KeyedArchive(path, words)] = sources.as_slice() else {
        panic!("script archive should keep its registered words");
    };
    assert_eq!(path.relative(), "data/pack/script.cpz");
    assert_eq!(*words, [0x44, 0x33, 0x22, 0x11]);

    let source = startup_code(0x27947, &code, b"data\\pack\\script.cpz\0");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    let _ = vm.run(40);
    assert!(vm.resources().script_sources().is_empty());
}

#[test]
fn invalid_fill_preserves_the_previous_global_and_does_not_advance() {
    let mut code = Vec::new();
    for (value, count) in [(7, 1), (9, 2)] {
        push_integer(&mut code, value);
        push_integer(&mut code, count);
        push_integer(&mut code, 4095);
        code.extend_from_slice(&0x2092_u16.to_le_bytes());
    }
    let source = startup_code(0, &code, b"");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    let error = vm.run(20).unwrap_err();
    assert_eq!(error.pc, code.len() - 2);
    assert_eq!(vm.pc(), error.pc);
    assert_eq!(vm.integer(4095), Some(7));
    assert_eq!(vm.parameter(4), Some(4095));
    assert_eq!(vm.parameter(8), Some(2));
    assert_eq!(vm.parameter(12), Some(9));
}

#[test]
fn common_script_return_restores_the_caller_and_rejects_a_stale_image() {
    let mut code = Vec::new();
    push_integer(&mut code, 0);
    push_integer(&mut code, 1);
    code.extend_from_slice(&0x2081_u16.to_le_bytes());
    let resume = code.len();
    code.extend_from_slice(&0x2000_u16.to_le_bytes());
    let source = startup_code(1, &code, b"common.ps3\0");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    assert_eq!(
        vm.run(20).unwrap(),
        VmYield::SystemCall {
            pc: resume - 2,
            opcode: 0x2081
        }
    );

    let request = vm.pending_script().unwrap().unwrap();
    assert_eq!(request.common_slot(), Some(1));
    assert_eq!(request.name(), b"common.ps3");
    let common = startup_code(1, &[0x14, 0x04, 0, 0], b"");
    let stale = request.clone().prepare(&common).unwrap();
    vm.complete_common_script(request.prepare(&common).unwrap())
        .unwrap();
    assert_eq!(vm.script_name(), b"common.ps3");
    assert!(vm.complete_common_script(stale).is_err());
    assert_eq!(vm.run(2).unwrap(), VmYield::Frame);
    assert_eq!(vm.script_name(), b"start.ps3");
    assert_eq!(vm.pc(), resume + 2);
    assert_eq!(vm.parameter(4), None);
}

#[test]
fn automatic_callback_enters_on_the_next_native_frame_and_resumes_the_caller() {
    let mut code = Vec::new();
    let op = |code: &mut Vec<u8>, value: u16| code.extend_from_slice(&value.to_le_bytes());
    for value in [0, 0, 0] {
        push_integer(&mut code, value);
    }
    let target_operand = code.len() + 4;
    push_integer(&mut code, 0);
    push_integer(&mut code, 6);
    op(&mut code, 0x2088);
    push_integer(&mut code, 1);
    op(&mut code, 0x2089);
    op(&mut code, 0x2000);
    let resume = code.len();
    op(&mut code, 0x2000);
    let target = code.len() as u32;
    code[target_operand..target_operand + 4].copy_from_slice(&target.to_le_bytes());
    for value in [42, 1, 5] {
        push_integer(&mut code, value);
    }
    op(&mut code, 0x2092);
    op(&mut code, 0x0414);
    op(&mut code, 0);

    let source = startup_code(1, &code, b"");
    let mut vm = CmvsVm::prepare(VmProfile::Pc20160201X64, &source).unwrap();
    vm.begin_frame(0).unwrap();
    assert_eq!(vm.run(30).unwrap(), VmYield::Frame);
    assert_eq!(vm.pc(), resume);
    assert_eq!(vm.integer(5), Some(0));
    vm.begin_frame(16).unwrap();
    assert_eq!(vm.pc(), target as usize);
    assert_eq!(vm.run(30).unwrap(), VmYield::Frame);
    assert_eq!(vm.pc(), resume + 2);
    assert_eq!(vm.integer(5), Some(42));
    assert_eq!(vm.parameter(4), None);
}

#[test]
fn ps2_profile_keeps_native_no_effect_slots_distinct_from_later_handlers() {
    let mut code = Vec::new();
    push_integer(&mut code, 10);
    for opcode in [0x201fu16, 0x2000, 0x2320] {
        code.extend_from_slice(&opcode.to_le_bytes());
    }
    let reader = startup_code(0, &code, &[]);
    let mut vm = CmvsVm::prepare(VmProfile::Pc20070119X86, &reader).unwrap();
    assert_eq!(vm.script_name(), b"start.ps2");
    assert_eq!(vm.run(16).unwrap(), VmYield::Frame);
    assert_eq!(vm.settings().archive_mode, None);
    assert_eq!(vm.parameter(4), Some(10));
    assert_eq!(vm.run(1).unwrap_err().reason, "outside CMVS 2007 syscall table");
}
