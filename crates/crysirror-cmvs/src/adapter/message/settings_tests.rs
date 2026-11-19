// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::vm::{VmProfile, VmYield};
use crysirror_core::service::MemoryAssetReader;

fn prepare(code: &[u8], text: &[u8]) -> CmvsVm {
    let mut file = b"PS2A".to_vec();
    for word in [
        48,
        1,
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
    CmvsVm::prepare(
        VmProfile::Pc20160201X64,
        &MemoryAssetReader(Arc::from(file)),
    )
    .unwrap()
}

fn push_word(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&[0x40, 0x04, 4, 0]);
    code.extend_from_slice(&value.to_le_bytes());
}

fn call(code: &mut Vec<u8>, opcode: u16, mode: u32, value: u32) {
    push_word(code, value);
    push_word(code, mode);
    code.extend_from_slice(&opcode.to_le_bytes());
}

fn dispatch(messages: &mut Messages, vm: &mut CmvsVm, expected: u16) {
    let VmYield::SystemCall { opcode, .. } = vm.run(20).unwrap() else {
        panic!("expected an owner call");
    };
    assert_eq!(opcode, expected);
    messages.text_setting(vm, opcode).unwrap();
}

#[test]
fn text_settings_round_trip_through_the_native_stack_and_clamp_values() {
    let cases = [
        (0x22df, (-5i32) as u32, 2),
        (0x22e0, 9, 1),
        (0x22e1, 42, 10),
        (0x22e2, u32::MAX, 0),
        (0x22e3, 42, 4),
        (0x22e4, 27, 1),
        (0x22e6, 0, 0),
        (0x22e7, u32::MAX, u32::MAX),
        (0x22e8, 123, 123),
        (0x22f8, 456, 456),
    ];
    let mut code = Vec::new();
    for (opcode, value, _) in cases {
        call(&mut code, opcode, 1, value);
        call(&mut code, opcode, 0, 0xdead_beef);
        code.extend_from_slice(&[0, 2, 0x0f, 1, 0, 0, 0, 0, 0x0f, 2, 0x30, 4, 0, 0x20]);
    }
    let mut vm = prepare(&code, &[]);
    let mut messages = Messages::default();
    for (index, (opcode, _, result)) in cases.into_iter().enumerate() {
        dispatch(&mut messages, &mut vm, opcode);
        dispatch(&mut messages, &mut vm, opcode);
        assert_eq!(vm.run(20).unwrap(), VmYield::Frame);
        assert_eq!(vm.parameter(4), Some(result), "{opcode:#x}");
        assert_eq!(vm.parameter(4 * (index + 2)), None, "{opcode:#x}");
    }
}

#[test]
fn face_query_writes_loaded_text_and_bold_preserves_existing_glyphs() {
    let mut code = Vec::new();
    call(&mut code, 0x22de, 1, 0);
    call(&mut code, 0x22de, 0, 9);
    call(&mut code, 0x22e0, 1, 1);
    code.extend_from_slice(&[0, 0x20]);
    let mut vm = prepare(&code, b"New Face\0................\0");
    let mut messages = Messages::default();
    let mut text = Text::new([0; 2], [64; 2], [64; 2], Arc::from("default"), false);
    output::push(&mut text, u16::from(b'A'), [0, 0], 24, 0).unwrap();
    messages.texts[0] = Some(text);
    dispatch(&mut messages, &mut vm, 0x22de);
    dispatch(&mut messages, &mut vm, 0x22de);
    assert_eq!(vm.text_reference(9), Some(b"New Face".as_slice()));
    dispatch(&mut messages, &mut vm, 0x22e0);

    let text = messages.texts[0].as_mut().unwrap();
    output::push(text, u16::from(b'B'), [24, 0], 24, 0).unwrap();
    assert!(!text.glyphs[0].bold);
    assert!(text.glyphs[1].bold);
    assert_eq!(vm.run(20).unwrap(), VmYield::Frame);
    assert_eq!(vm.parameter(4), None);
}
