// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn suspended_redraw_retains_previous_presentation_but_modal_can_be_shown() {
    let mut rt = Runtime::with_host(KrkrHost::new([4, 4], "default".into()));
    let global = rt.global_handle();
    rt.register_object_native(global, "awaitImage", |rt: &mut Runtime<KrkrHost>, _, _| {
        rt.request_suspend();
        Ok(Variant::Void)
    });
    natives::run_source(
        &mut rt,
        "redraw.tjs",
        "global.paintStage = 1; awaitImage(); global.paintStage = 2;",
        None,
    )
    .unwrap();
    assert_eq!(rt.global_member("paintStage").to_integer().unwrap(), 1);
    let config = serde_json::from_value(json!({"canvas":[4,4]})).unwrap();
    let mut vm = KrkrVm::new(ClientId(1), config).unwrap();
    vm.rt = Some(rt);
    assert!(
        vm.presentation_pending(),
        "the prefix of a redraw must not be presented"
    );
    vm.rt.as_mut().unwrap().host_mut().dialog_pending = Some(1);
    assert!(
        !vm.presentation_pending(),
        "a waiting modal must be presented to receive input"
    );
    vm.rt.as_mut().unwrap().host_mut().dialog_pending = None;
    vm.rt.as_mut().unwrap().resume_suspended().unwrap();
    assert_eq!(
        vm.rt
            .as_ref()
            .unwrap()
            .global_member("paintStage")
            .to_integer()
            .unwrap(),
        2
    );
    assert!(
        !vm.presentation_pending(),
        "the completed redraw must publish normally"
    );
}
