//! Client-local restore must not replay old leases or drop another UI gesture.
use crysirror_core::{
    service::{buttons, InputFrame},
    state::ClientId,
};
use crysirror_scene::{input_routes::*, ui::*};
use serde_json::json;
#[test]
fn ui_client_restore_drops_only_owned_events_focus_and_capture_keeps_peer_release() {
    let mut ui = UiSystem::default();
    for (owner, x) in [(1, 0), (2, 20)] {
        let widget =
            serde_json::from_value(json!({"id":1,"kind":"button","rect":[x,0,10,10]})).unwrap();
        ui.apply_batch(ClientId(owner), &[UiCommand::Put { widget }])
            .unwrap();
    }
    ui.tick(
        InputFrame {
            buttons_down: buttons::MOUSE_LEFT,
            buttons_pressed: buttons::MOUSE_LEFT,
            pointer_x: 21,
            pointer_y: 1,
        },
        [40, 20],
    )
    .unwrap();
    assert!(ui.interaction(2, 1).unwrap().pressed);
    ui.events.push(UiEvent {
        widget: WidgetId { owner: 1, id: 1 },
        kind: "activate".into(),
        value: 1.,
        menu_bar: None,
    });
    ui.reset_client_input_after_restore(ClientId(1));
    assert!(ui.events.iter().all(|event| event.widget.owner != 1));
    assert!(ui.interaction(2, 1).unwrap().pressed);
    ui.tick(
        InputFrame {
            pointer_x: 21,
            pointer_y: 1,
            ..Default::default()
        },
        [40, 20],
    )
    .unwrap();
    assert!(ui
        .drain_events(2, None)
        .iter()
        .any(|event| event.kind == "activate"));
    ui.tick(
        InputFrame {
            buttons_down: buttons::MOUSE_LEFT,
            buttons_pressed: buttons::MOUSE_LEFT,
            pointer_x: 1,
            pointer_y: 1,
        },
        [40, 20],
    )
    .unwrap();
    assert!(ui.interaction(1, 1).unwrap().focused);
    ui.reset_client_input_after_restore(ClientId(1));
    assert!(!ui.interaction(1, 1).unwrap().focused);
}
#[test]
fn scoped_route_restore_invalidates_only_its_old_delivery_epoch_and_preserves_peer_lease() {
    let mut routes = InputRouting::default();
    for owner in [ClientId(1), ClientId(2)] {
        routes
            .apply_batch(
                owner,
                &[InputRouteCommand::Put {
                    receiver: serde_json::from_value(json!({"id":1,"priority":0,"sources":7}))
                        .unwrap(),
                }],
            )
            .unwrap();
        routes
            .dispatch(
                owner,
                InputRouteEvent {
                    source: InputRouteSource::Pointer,
                    code: 16,
                    phase: InputRoutePhase::Press,
                    point: [1, 1],
                    origin: None,
                },
            )
            .unwrap();
    }
    let one = routes.poll(ClientId(1)).unwrap().unwrap();
    let two = routes.poll(ClientId(2)).unwrap().unwrap();
    routes.reset_client_input_after_restore(ClientId(1));
    assert!(routes.acknowledge(ClientId(1), one.receipt, true).is_err());
    assert_eq!(routes.poll(ClientId(2)).unwrap(), Some(two.clone()));
    routes.acknowledge(ClientId(2), two.receipt, true).unwrap();
    routes.dispatch(ClientId(1), one.event).unwrap();
    let fresh = routes.poll(ClientId(1)).unwrap().unwrap();
    assert_ne!(fresh.receipt.epoch, one.receipt.epoch);
    assert_eq!(routes.stats(ClientId(1)).receivers, 1);
}
