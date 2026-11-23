//! Shared slider capture and focus, independent of any native GUI adapter.
use crysirror_core::{
    service::{InputFrame, buttons},
    state::ClientId,
};
use crysirror_scene::ui::{UiCommand, UiSystem, WidgetId, WidgetSpec};
use serde_json::json;
fn slider(id: u32, x: i32) -> WidgetSpec {
    serde_json::from_value(
        json!({"id":id,"rect":[x,20,20,100],"kind":"slider","slider_axis":"vertical","value":0.4}),
    )
    .unwrap()
}
#[test]
fn vertical_capture_tracks_y_and_preserves_focus_and_other_clients() {
    let mut ui = UiSystem::default();
    ui.install(ClientId(1), &[slider(1, 10)]).unwrap();
    ui.install(ClientId(2), &[slider(1, 180)]).unwrap();
    ui.tick(
        InputFrame {
            pointer_x: 15,
            pointer_y: 45,
            buttons_down: buttons::MOUSE_LEFT,
            buttons_pressed: buttons::MOUSE_LEFT,
        },
        [240, 160],
    )
    .unwrap();
    assert!((ui.events.last().unwrap().value - 0.25).abs() < 1e-6);
    ui.tick(
        InputFrame {
            pointer_x: 200,
            pointer_y: 95,
            buttons_down: buttons::MOUSE_LEFT,
            ..Default::default()
        },
        [240, 160],
    )
    .unwrap();
    assert!((ui.events.last().unwrap().value - 0.75).abs() < 1e-6);
    assert_eq!(
        ui.events.last().unwrap().widget,
        WidgetId { owner: 1, id: 1 }
    );
    assert!(ui.interaction(1, 1).unwrap().pressed);
    assert_eq!(
        serde_json::to_value(&ui).unwrap()["clients"]["2"]["1"]["value"],
        json!(0.4f32)
    );
    ui.tick(
        InputFrame {
            pointer_x: -1,
            pointer_y: -1,
            ..Default::default()
        },
        [240, 160],
    )
    .unwrap();
    ui.apply_batch(ClientId(1), &[UiCommand::Focus { id: 1 }])
        .unwrap();
    ui.tick(
        InputFrame {
            pointer_x: -1,
            pointer_y: -1,
            buttons_down: buttons::DOWN,
            buttons_pressed: buttons::DOWN,
        },
        [240, 160],
    )
    .unwrap();
    assert!(ui.interaction(1, 1).unwrap().focused);
    assert!((ui.events.last().unwrap().value - 0.8).abs() < 1e-6);
    let saved = serde_json::to_vec(&ui).unwrap();
    let restored: UiSystem = serde_json::from_slice(&saved).unwrap();
    restored.validate().unwrap();
}
#[test]
fn vertical_axis_on_a_button_is_rejected_atomically() {
    let mut ui = UiSystem::default();
    ui.install(ClientId(1), &[slider(1, 10)]).unwrap();
    let before = serde_json::to_vec(&ui).unwrap();
    let bad = serde_json::from_value(
        json!({"id":2,"rect":[0,0,20,20],"kind":"button","slider_axis":"vertical"}),
    )
    .unwrap();
    assert!(
        ui.apply_batch(ClientId(1), &[UiCommand::Put { widget: bad }])
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&ui).unwrap(), before);
}

#[test]
fn buttons_navigate_sequentially_in_both_directions() {
    let mut ui = UiSystem::default();
    let widgets = (1..=3)
        .map(|id| {
            serde_json::from_value(json!({"id":id,"kind":"button","rect":[id*30,0,20,20]})).unwrap()
        })
        .collect::<Vec<_>>();
    ui.install(ClientId(1), &widgets).unwrap();
    ui.apply_batch(ClientId(1), &[UiCommand::Focus { id: 1 }])
        .unwrap();
    for (key, expected) in [
        (buttons::LEFT, 3),
        (buttons::UP, 2),
        (buttons::RIGHT, 3),
        (buttons::DOWN, 1),
    ] {
        ui.tick(
            InputFrame {
                pointer_x: -1,
                pointer_y: -1,
                buttons_down: key,
                buttons_pressed: key,
            },
            [200, 100],
        )
        .unwrap();
        assert!(ui.interaction(1, expected).unwrap().focused);
        ui.tick(
            InputFrame {
                pointer_x: -1,
                pointer_y: -1,
                ..Default::default()
            },
            [200, 100],
        )
        .unwrap();
    }
}

#[test]
fn pointer_focus_handoff_preserves_keyboard_selection_until_pointer_moves() {
    let mut ui = UiSystem::default();
    let widgets=(1..=2).map(|id|serde_json::from_value(json!({"id":id,"kind":"button","rect":[id*30,0,20,20],"focus_follows_pointer":true})).unwrap()).collect::<Vec<_>>();
    ui.install(ClientId(1), &widgets).unwrap();
    let frame = InputFrame {
        pointer_x: 35,
        pointer_y: 10,
        ..Default::default()
    };
    ui.tick(frame, [200, 100]).unwrap();
    assert!(ui.interaction(1, 1).unwrap().focused);
    ui.tick(
        InputFrame {
            buttons_down: buttons::DOWN,
            buttons_pressed: buttons::DOWN,
            ..frame
        },
        [200, 100],
    )
    .unwrap();
    assert!(ui.interaction(1, 2).unwrap().focused);
    ui.tick(frame, [200, 100]).unwrap();
    assert!(ui.interaction(1, 2).unwrap().focused);
    ui.tick(
        InputFrame {
            pointer_x: 36,
            ..frame
        },
        [200, 100],
    )
    .unwrap();
    assert!(ui.interaction(1, 1).unwrap().focused);
    ui.tick(
        InputFrame {
            pointer_x: -1,
            pointer_y: -1,
            ..Default::default()
        },
        [200, 100],
    )
    .unwrap();
    ui.tick(
        InputFrame {
            pointer_x: -1,
            pointer_y: -1,
            ..Default::default()
        },
        [200, 100],
    )
    .unwrap();
    assert!(!ui.interaction(1, 1).unwrap().focused && !ui.interaction(1, 2).unwrap().focused);
}
