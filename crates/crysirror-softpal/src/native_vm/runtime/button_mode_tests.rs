use super::*;

pub(super) fn button_scene() -> (ScriptRuntime, SpriteSystem, PalInputState) {
    let mut runtime = ScriptRuntime::boot(0, ScriptRuntimeConfig::default());
    let mut sprites = SpriteSystem::new();
    let mut desc = SpriteDesc::new(SceneTextureId(1), 40, 40);
    desc.visible = true;
    let handle = sprites.create(desc);
    runtime.game_buttons.insert(
        (2, 7),
        GameButtonEntry {
            registration_order: 1,
            native_flags: 1,
            hover_callbacks: [None; 2],
            handle,
            name: "test".into(),
            visible: true,
            enabled: true,
            locked: false,
            toggle: 0,
            alpha: 255,
            slider_offset: 0,
            base_position: [0; 2],
            slider_position: [0; 2],
            hit_rect: None,
            gosub_point: Some(123),
            anim_resource: None,
            anim_play_flag: 0,
        },
    );
    let mut input = PalInputState::new();
    input.set_coordinate_space(1280, 720, 1280, 720);
    input.handle_cursor_moved(10.0, 10.0);
    (runtime, sprites, input)
}

#[test]
fn btn_mode_release_captures_press_but_dispatches_only_once_on_release() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    runtime.button_release_mode = true;
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    let press = runtime.update_button_input_state(&mut sprites, &input);
    assert!(press.consumed_mouse_capture);
    assert!(
        !press.consumed_any(),
        "capturing must not resolve the ADV wait"
    );
    assert_eq!(
        runtime.pressed_button,
        Some((2, 7)),
        "slider captures on push"
    );
    assert!(runtime.pending_gosub_points.is_empty());
    assert!(runtime.button_push_queue.is_empty());

    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, false);
    let release = runtime.update_button_input_state(&mut sprites, &input);
    assert!(release.consumed_mouse_capture && !release.consumed_any());
    assert!(runtime.pending_gosub_points.is_empty());
    input.begin_frame();
    assert!(runtime.update_button_input_state(&mut sprites, &input).consumed_push_edge);
    assert_eq!(runtime.pressed_button, None);
    assert_eq!(
        runtime
            .pending_gosub_points
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [123]
    );
    assert_eq!(runtime.pop_latched_button_push(2), Some(7));
    assert_eq!(runtime.pop_latched_button_push(2), None);
    let consumed = input.without_mouse_edges();
    assert!(!consumed.mouse_pull(PalMouseButton::Left));
    input.begin_frame();
    assert!(!runtime
        .update_button_input_state(&mut sprites, &input)
        .consumed_any());
}

#[test]
fn btn_mode_release_requires_hover_capture_and_cancels_when_dragged_outside() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    runtime.button_release_mode = true;
    // A release edge alone is insufficient for normal buttons.
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, false);
    assert!(!runtime
        .update_button_input_state(&mut sprites, &input)
        .consumed_any());
    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    runtime.update_button_input_state(&mut sprites, &input);
    input.begin_frame();
    input.handle_cursor_moved(100.0, 100.0);
    runtime.update_button_input_state(&mut sprites, &input);
    assert_eq!(
        runtime.pressed_button,
        Some((2, 7)),
        "slider capture survives leaving"
    );
    input.begin_frame();
    input.handle_cursor_moved(10.0, 10.0);
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, false);
    assert!(!runtime
        .update_button_input_state(&mut sprites, &input)
        .consumed_any());
    assert!(runtime.pending_gosub_points.is_empty());
    // Native captures hover while already held, including entry from outside.
    input.begin_frame();
    input.handle_cursor_moved(100.0, 100.0);
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    runtime.update_button_input_state(&mut sprites, &input);
    input.begin_frame();
    input.handle_cursor_moved(10.0, 10.0);
    runtime.update_button_input_state(&mut sprites, &input);
    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, false);
    assert!(
        runtime
            .update_button_input_state(&mut sprites, &input)
            .consumed_mouse_capture
    );
    assert!(runtime.pending_gosub_points.is_empty());
    input.begin_frame();
    assert!(runtime.update_button_input_state(&mut sprites, &input).consumed_push_edge);
}

#[test]
fn btn_mode_switch_keeps_default_push_and_does_not_invent_keyboard_button_clicks() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    assert!(!runtime.button_release_mode);
    input.handle_keyboard_event("Enter", true);
    assert!(!runtime
        .update_button_input_state(&mut sprites, &input)
        .consumed_any());
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    assert!(
        runtime
            .update_button_input_state(&mut sprites, &input)
            .consumed_mouse_capture
    );
    assert_eq!(runtime.pop_latched_button_push(2), None);
    input.begin_frame();
    assert!(runtime.update_button_input_state(&mut sprites, &input).consumed_push_edge);
    assert_eq!(runtime.pop_latched_button_push(2), Some(7));
    runtime.set_pal_time(100);
    input.begin_frame();
    runtime.button_release_mode = true;
    runtime.update_button_input_state(&mut sprites, &input);
    runtime.button_release_mode = false;
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, false);
    assert!(!runtime
        .update_button_input_state(&mut sprites, &input)
        .consumed_any());
    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    assert!(
        runtime
            .update_button_input_state(&mut sprites, &input)
            .consumed_mouse_capture
    );
}

#[test]
fn btn_mode_uninit_clears_pending_capture_without_resetting_mode() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    runtime.button_release_mode = true;
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Left, true);
    runtime.update_button_input_state(&mut sprites, &input);
    runtime.stack.push(2);
    runtime.ext_btn_uninit(Some(&mut sprites));
    assert_eq!(runtime.pressed_button, None);
    assert_eq!(runtime.button_release_capture, None);
    assert!(runtime.button_release_mode);
}

#[test]
fn system_shortcuts_dispatch_first_slot_and_share_idle_gate() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    input.handle_cursor_moved(100.0, 100.0);
    for slot in [0, 4, 5, 16] {
        runtime.system_buttons.insert(slot, GameSystemButtonEntry {
            image: 100 + slot, state: 0, enabled: true,
        });
    }
    input.handle_keyboard_event("F1", true);
    input.handle_keyboard_event("Escape", true);
    runtime.update_button_input_state(&mut sprites, &input);
    assert_eq!(runtime.pending_gosub_points.pop_front(), Some(104));
    assert!(runtime.pending_gosub_points.is_empty());
    input.begin_frame();
    input.handle_keyboard_event("F1", false);
    input.handle_keyboard_event("Escape", false);
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Right, true);
    runtime.set_pal_time(49);
    runtime.update_button_input_state(&mut sprites, &input);
    assert!(runtime.pending_gosub_points.is_empty());
    runtime.set_pal_time(50);
    runtime.update_button_input_state(&mut sprites, &input);
    assert_eq!(runtime.pending_gosub_points.pop_front(), Some(100));
}

#[test]
fn system_repeat_uses_native_mask_and_release_profile() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    runtime.system_buttons.insert(17, GameSystemButtonEntry {
        image: 117, state: 1, enabled: true,
    });
    input.handle_keyboard_event("ArrowUp", true);
    input.sample_repeat(0);
    input.begin_frame();
    input.sample_repeat(501);
    runtime.set_pal_time(501);
    runtime.update_button_input_state(&mut sprites, &input);
    assert!(runtime.pending_gosub_points.is_empty(), "old EXE rejects stable held keys");
    runtime.input_registered_profile = true;
    runtime.update_button_input_state(&mut sprites, &input);
    assert!(runtime.pending_gosub_points.is_empty(), "state is a bit mask, not a repeat boolean");
    runtime.system_buttons.get_mut(&17).unwrap().state = 0x80;
    runtime.update_button_input_state(&mut sprites, &input);
    assert_eq!(runtime.pending_gosub_points.pop_front(), Some(117));
    input.begin_frame();
    input.handle_keyboard_event("ArrowUp", false);
    runtime.set_pal_time(600);
    runtime.system_buttons.insert(0, GameSystemButtonEntry {
        image: 120, state: 0, enabled: true,
    });
    runtime.input_release_shortcuts = true;
    runtime.button_release_mode = true;
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Right, true);
    runtime.update_button_input_state(&mut sprites, &input);
    assert!(runtime.pending_gosub_points.is_empty());
    input.begin_frame();
    input.handle_mouse_button_event(crate::native_vm::event::MouseButton::Right, false);
    runtime.update_button_input_state(&mut sprites, &input);
    assert_eq!(runtime.pending_gosub_points.pop_front(), Some(120));
}

#[test]
fn disabled_front_button_occludes_and_equal_priority_uses_registration_order() {
    let (mut runtime, mut sprites, input) = button_scene();
    let mut desc = SpriteDesc::new(SceneTextureId(2), 40, 40);
    desc.visible = true;
    let handle = sprites.create(desc);
    let mut front = runtime.game_buttons[&(2, 7)].clone();
    front.registration_order = 2;
    front.handle = handle;
    front.enabled = false;
    runtime.game_buttons.insert((2, 1), front);
    assert_eq!(runtime.button_hit_at(&sprites, 10, 10, 2), None);
    runtime.game_buttons.get_mut(&(2, 1)).unwrap().enabled = true;
    assert_eq!(runtime.button_hit_at(&sprites, 10, 10, 2), Some((2, 1)));
    let _ = input;
}

#[test]
fn synthetic_reaction_waits_two_polls_and_cannot_replace_pending() {
    let (mut runtime, mut sprites, mut input) = button_scene();
    runtime.stack.extend([7, 2]);
    runtime.ext_btn_set_hit();
    input.handle_cursor_moved(100.0, 100.0);
    assert!(runtime.update_button_input_state(&mut sprites, &input).consumed_mouse_capture);
    assert!(runtime.pending_gosub_points.is_empty());
    runtime.begin_button_reaction(2, 99, true, None);
    assert_eq!(runtime.pending_button_reactions[&2].index, 7);
    input.begin_frame();
    assert!(runtime.update_button_input_state(&mut sprites, &input).consumed_push_edge);
    assert_eq!(runtime.pending_gosub_points.pop_front(), Some(123));
    runtime.set_pal_time(100);
    input.begin_frame();
    runtime.update_button_input_state(&mut sprites, &input);
    assert!(runtime.pending_gosub_points.is_empty());
}
