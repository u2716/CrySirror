use crysirror_core::{
    service::{InputObservation, buttons},
    state::ClientId,
};
use crysirror_system4::input::Input;
fn observation(revision: u64, down: u64, pressed: u64) -> InputObservation {
    InputObservation {
        client: ClientId(1),
        revision,
        time_ms: revision,
        buttons_down: down,
        buttons_pressed: pressed,
        pointer: Some([25, 37]),
        pointer_inside: Some(true),
        focused: Some(true),
    }
}
#[test]
fn native_key_clear_stays_cleared_until_a_new_physical_event() {
    let mut input = Input::default();
    let held = observation(
        1,
        buttons::ENTER | buttons::CTRL,
        buttons::ENTER | buttons::CTRL,
    );
    input.observe(held);
    assert_eq!(input.key(13).unwrap(), 1);
    input.clear(true);
    assert_eq!(input.key(13).unwrap(), 0);
    assert_eq!(input.key(17).unwrap(), 1);
    input.observe(held);
    assert_eq!(input.key(13).unwrap(), 0);
    input.observe(observation(2, buttons::ENTER | buttons::CTRL, 0));
    assert_eq!(input.key(13).unwrap(), 0);
    input.observe(observation(3, 0, 0));
    input.observe(observation(4, buttons::ENTER, buttons::ENTER));
    assert_eq!(input.key(13).unwrap(), 1);
    input.clear(false);
    assert_eq!(input.key(13).unwrap(), 0);
    assert!(input.key(0x42).is_err());
    assert_eq!(input.key(999).unwrap(), 0);
    input.observe(observation(5, 0, buttons::ENTER));
    assert_eq!(
        input.key(13).unwrap(),
        0,
        "same-frame released key is not held"
    );
}
#[test]
fn native_pointer_focus_and_wheel_use_real_owner_observation() {
    let mut input = Input::default();
    let o = observation(1, 0, buttons::WHEEL_UP);
    input.observe(o);
    assert_eq!(input.wheel(), [1, 0]);
    input.clear_wheel();
    input.observe(o);
    assert_eq!(input.wheel(), [0, 0]);
    assert_eq!(Input::pointer(o).unwrap(), ([25, 37], 1));
    let mut unknown = o;
    unknown.focused = None;
    assert!(Input::pointer(unknown).is_err());
    assert!(Input::validate_scope(o, ClientId(2)).is_err());
}
