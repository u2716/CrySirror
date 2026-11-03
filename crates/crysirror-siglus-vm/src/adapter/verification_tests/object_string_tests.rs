//! Authored native bytecode and controllable engine completions, without fonts.
use super::*;
use crate::runtime::{
    constants::{cd, fm, op},
    forms::codes,
    globals::ObjectState,
};
use std::sync::atomic::Ordering;

fn chain(index: i32, op: i32) -> [i32; 5] {
    [
        codes::ELM_GLOBAL_FRONT,
        codes::ELM_STAGE_OBJECT,
        codes::ELM_ARRAY,
        index,
        op,
    ]
}
fn object(vm: &SiglusVm, index: usize) -> Option<&ObjectState> {
    vm.state
        .vm
        .as_ref()?
        .ctx
        .globals
        .stage_forms
        .values()
        .find_map(|stage| stage.object_lists.get(&1)?.get(index))
}
fn installed(vm: &SiglusVm, host: &Host) -> Vec<crysirror_scene::layer::LayerSpec> {
    vm.state
        .published
        .iter()
        .filter_map(|id| host.engine.layers.get(ClientId(1), *id).cloned())
        .collect()
}
fn install(host: &mut Host, scene: crate::fixture::SceneBuilder) {
    host.files.insert(
        "Asset/Scene.pck".into(),
        crate::fixture::pack_scene(&scene.finish()),
    );
}
#[test]
fn native_string_waits_atomically_then_rebuilds_and_retires_original_generations() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_CREATE_STRING),
        2,
        "A#2CB",
        &[1, 10, 20],
        fm::VOID,
    );
    scene.command(
        &chain(52, codes::ELM_OBJECT_SET_STRING_PARAM),
        1,
        &[24, -4, 2, 0, 4, 1, 3, 6],
        fm::VOID,
    );
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_SET_STRING),
        0,
        "A#2CB",
        &[],
        fm::VOID,
    );
    scene.command(&chain(52, codes::ELM_OBJECT_GET_STRING), 0, &[], fm::STR);
    scene.command(&chain(52, codes::ELM_OBJECT_GET_SIZE_X), 0, &[], fm::INT);
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    host.glyph_pending.store(true, Ordering::SeqCst);
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::BudgetExhausted
    );
    assert!(object(&vm, 52).is_none_or(|o| o.object_type == 0));
    assert_eq!(vm.state.vm.as_ref().unwrap().ctx.native_media_revision, 0);
    assert!(vm.state.published.is_empty());
    assert_eq!(host.glyph_requests.lock().unwrap().len(), 1);
    let pending = vm.snapshot().unwrap();
    vm.restore(&pending).unwrap();
    host.glyph_pending.store(false, Ordering::SeqCst);
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::Halted
    );
    let o = object(&vm, 52).unwrap();
    assert_eq!(
        (o.object_type, o.base.disp, o.base.x, o.base.y),
        (3, 1, 10, 20)
    );
    assert_eq!(o.string_value.as_deref(), Some("A#2CB"));
    assert_eq!(o.string_param.fuchi_color, 6);
    assert_eq!(o.runtime.native_text.as_ref().unwrap().generation, 3);
    assert_eq!(vm.object_strings.len(), 1);
    assert_eq!(vm.state.io.text_sizes.len(), 1);
    assert_eq!(vm.diagnostics()["str_stack"], serde_json::json!(["A#2CB"]));
    assert_eq!(
        vm.diagnostics()["int_stack"],
        serde_json::json!([0]),
        "pinned GET_SIZE excludes String backends"
    );
    let layers = installed(&vm, &host);
    let glyphs = layers
        .iter()
        .filter_map(|layer| match &layer.paint {
            crysirror_scene::layer::Paint::GlyphRun {
                glyphs,
                color,
                blur,
                ..
            } => Some((glyphs[0].character, *color, blur.is_some())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(glyphs.len(), 6);
    assert_eq!(
        glyphs.iter().map(|g| g.0).collect::<Vec<_>>(),
        vec!['A', 'A', 'A', 'B', 'B', 'B']
    );
    assert_eq!(glyphs[2], ('A', [0, 0, 255, 255], false));
    assert_eq!(glyphs[5], ('B', [255, 0, 0, 255], false));
    let saved = vm.snapshot().unwrap();
    let before = serde_json::to_value(&layers).unwrap();
    vm.restore(&saved).unwrap();
    host.now = 16;
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::Halted
    );
    assert_eq!(serde_json::to_value(installed(&vm, &host)).unwrap(), before);
}

#[test]
fn native_string_param_seven_argument_form_recaptures_native_defaults() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_CREATE_STRING),
        1,
        "A",
        &[1],
        fm::VOID,
    );
    scene.command(
        &chain(52, codes::ELM_OBJECT_SET_STRING_PARAM),
        1,
        &[24, 0, 0, 0, 0, 1, 0, 6],
        fm::VOID,
    );
    scene.command(
        &chain(52, codes::ELM_OBJECT_SET_STRING_PARAM),
        0,
        &[24, 0, 0, 0, 0, 1, 0],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    assert_eq!(
        object(&vm, 52).unwrap().string_param.fuchi_color,
        1,
        "seven-argument form starts from MWND defaults"
    );
}

#[test]
fn native_string_frame_callback_pending_snapshot_keeps_one_clock_and_commit() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command(
        &chain(52, codes::ELM_OBJECT_CREATE_RECT),
        2,
        &[0, 0, 4, 3, 255, 0, 0, 255, 1, 0, 0],
        fm::VOID,
    );
    let mut start = chain(52, codes::ELM_OBJECT_FRAME_ACTION).to_vec();
    start.push(codes::ELM_FRAMEACTION_START);
    scene.element(&start);
    scene.int(32);
    scene.string("pulse");
    scene.code.push(cd::COMMAND);
    for v in [0, 2, fm::STR, fm::INT, 0, fm::VOID] {
        scene.i32(v);
    }
    scene.int(77);
    scene.command(&[codes::ELM_GLOBAL_TIMEWAIT], 0, &[1000], fm::VOID);
    scene.code.push(cd::EOF);
    let callback = scene.code.len();
    for (id, form) in [codes::FM_FRAMEACTION, codes::FM_OBJECT]
        .into_iter()
        .enumerate()
    {
        scene.code.push(cd::DEC_PROP);
        scene.i32(form);
        scene.i32(id as i32);
    }
    scene.code.push(cd::ARG);
    let g = [codes::ELM_GLOBAL_G, -1, 0];
    scene.element(&g);
    scene.command(&g, 0, &[], fm::INT);
    scene.int(1);
    scene.code.push(cd::OPERATE_2);
    scene.i32(fm::INT);
    scene.i32(fm::INT);
    scene.code.push(op::PLUS);
    scene.code.push(cd::COMMAND);
    for v in [1, 1, fm::INT, 0, fm::VOID] {
        scene.i32(v);
    }
    scene.command_string(
        &chain(53, codes::ELM_OBJECT_CREATE_STRING),
        1,
        "A",
        &[1],
        fm::VOID,
    );
    scene.code.push(cd::RETURN);
    scene.i32(0);
    let mut bytes = scene.finish();
    let table = bytes.len();
    bytes.extend((callback as i32).to_le_bytes());
    let index = bytes.len();
    bytes.extend(0i32.to_le_bytes());
    bytes.extend(5i32.to_le_bytes());
    let name = bytes.len();
    for w in "pulse".encode_utf16() {
        bytes.extend(w.to_le_bytes());
    }
    for (field, value) in [
        (19, table),
        (20, 1),
        (21, index),
        (22, 1),
        (23, name),
        (24, 1),
    ] {
        bytes[field * 4..field * 4 + 4].copy_from_slice(&(value as i32).to_le_bytes());
    }
    host.files
        .insert("Asset/Scene.pck".into(), crate::fixture::pack_scene(&bytes));
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    host.glyph_pending.store(true, Ordering::SeqCst);
    host.now = 16;
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::BudgetExhausted
    );
    assert_eq!(vm.state.native_frame_now, Some(16));
    assert!(object(&vm, 53).is_none_or(|o| o.object_type == 0));
    assert_eq!(
        vm.state
            .vm
            .as_ref()
            .unwrap()
            .ctx
            .globals
            .int_lists
            .get(&(codes::ELM_GLOBAL_G as u32))
            .and_then(|v| v.first())
            .copied()
            .unwrap_or(0),
        0
    );
    let saved = vm.snapshot().unwrap();
    vm.restore(&saved).unwrap();
    host.glyph_pending.store(false, Ordering::SeqCst);
    host.now = 99;
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    assert_eq!(vm.state.native_frame_now, None);
    assert_eq!(
        vm.state.vm.as_ref().unwrap().ctx.globals.int_lists[&(codes::ELM_GLOBAL_G as u32)][0],
        1
    );
    assert_eq!(object(&vm, 53).unwrap().object_type, 3);
    assert_eq!(vm.diagnostics()["int_stack"], serde_json::json!([77]));
}

#[test]
fn native_string_original_save_keeps_wire_codec_and_reconstructs_engine_intent() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_CREATE_STRING),
        1,
        "A",
        &[1],
        fm::VOID,
    );
    scene.command(&[codes::ELM_GLOBAL_SAVEPOINT], 0, &[], fm::INT);
    scene.command(&[codes::ELM_GLOBAL_TIMEWAIT], 0, &[1000], fm::VOID);
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let original = vm.export_original_save().unwrap();
    let frame = serde_json::to_value(installed(&vm, &host)).unwrap();
    vm.import_original_save(&original).unwrap();
    assert!(object(&vm, 52).unwrap().runtime.native_text.is_none());
    assert_eq!(vm.object_strings.len(), 0);
    // The original save contains no CrySirror generation/font lease. A host
    // query may defer reconstruction, without inventing an empty string draw.
    host.glyph_pending.store(true, Ordering::SeqCst);
    assert_eq!(
        vm.step(&mut host, InputFrame::default(), 5000).unwrap(),
        VmYield::BudgetExhausted
    );
    assert_eq!(serde_json::to_value(installed(&vm, &host)).unwrap(), frame);
    host.glyph_pending.store(false, Ordering::SeqCst);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    assert!(object(&vm, 52).unwrap().runtime.native_text.is_some());
    assert_eq!(serde_json::to_value(installed(&vm, &host)).unwrap(), frame);
    let rebuilt = vm.export_original_save().unwrap();
    assert_eq!(rebuilt.len(), original.len());
    // Native ensure_runtime_slot_for_save intentionally refreshes its civil
    // timestamp (eight i32 fields). It is not the simulated VM clock. Compare
    // every remaining byte, including the entire original packed payload.
    assert!(
        rebuilt[..8] == original[..8] && rebuilt[40..] == original[40..],
        "original header metadata/payload changed outside the native save timestamp"
    );
}

#[test]
fn native_string_restored_recipe_generation_is_validated_before_replacing_live_vm() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_CREATE_STRING),
        1,
        "A",
        &[1],
        fm::VOID,
    );
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let before = vm.snapshot().unwrap();
    let mut invalid: Machine = binary().deserialize(&before).unwrap();
    let native = invalid.vm.as_mut().unwrap();
    let o = native
        .ctx
        .globals
        .stage_forms
        .values_mut()
        .find_map(|stage| stage.object_lists.get_mut(&1))
        .unwrap()
        .get_mut(52)
        .unwrap();
    o.runtime.native_text.as_mut().unwrap().generation = 0;
    assert!(vm.restore(&binary().serialize(&invalid).unwrap()).is_err());
    assert_eq!(vm.snapshot().unwrap(), before);
}

#[test]
fn same_string_assignment_recaptures_current_font_but_non_string_storage_never_prepares_a_font() {
    let (mut vm, mut host) = machine(false);
    let mut scene = crate::fixture::SceneBuilder::default();
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_CREATE_STRING),
        1,
        "A",
        &[1],
        fm::VOID,
    );
    scene.command_string(
        &[codes::ELM_GLOBAL_SCRIPT, codes::ELM_SCRIPT_SET_FONT_NAME],
        0,
        "NativeTestFamily",
        &[],
        fm::VOID,
    );
    scene.command_string(
        &chain(52, codes::ELM_OBJECT_SET_STRING),
        0,
        "A",
        &[],
        fm::VOID,
    );
    scene.command(
        &chain(53, codes::ELM_OBJECT_CREATE_RECT),
        0,
        &[0, 0, 4, 3, 255, 0, 0, 255],
        fm::VOID,
    );
    scene.command_string(
        &chain(53, codes::ELM_OBJECT_SET_STRING),
        0,
        "B",
        &[],
        fm::VOID,
    );
    scene.command(&chain(53, codes::ELM_OBJECT_GET_STRING), 0, &[], fm::STR);
    scene.code.push(cd::EOF);
    install(&mut host, scene);
    vm.step(&mut host, InputFrame::default(), 5000).unwrap();
    let requests = host.glyph_requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].font, "default");
    assert_eq!(requests[1].font, "message-font-alt");
    assert_eq!(requests[1].glyphs[0].baseline, [0., 6.]);
    assert_eq!(
        object(&vm, 52)
            .unwrap()
            .runtime
            .native_text
            .as_ref()
            .unwrap()
            .capture
            .font_name,
        "NativeTestFamily"
    );
    assert_eq!(object(&vm, 53).unwrap().object_type, 1);
    assert!(object(&vm, 53).unwrap().runtime.native_text.is_none());
    assert_eq!(vm.diagnostics()["str_stack"], serde_json::json!(["B"]));
}
