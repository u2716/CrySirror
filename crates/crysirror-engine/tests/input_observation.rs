//! Public APIs are exercised without any native VM, window or SDK.
use crysirror_core::service::{buttons,InputFrame,PointerObservation};
use crysirror_engine::{input_api::ActionMap,App,Value};
use crysirror_runtime::Runtime;
use serde_json::json;

#[test]
fn public_observation_and_platform_status_use_real_owner_capabilities() {
    let root=tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    let path=root.path().join("project.json");
    std::fs::write(&path,serde_json::to_vec(&json!({"schema":2,"name":"input observation","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"app","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap()).unwrap();
    let runtime=Runtime::load(&path,&root.path().join("state")).unwrap();
    let mut app=App::new(runtime,"app").unwrap();
    let input=app.context().singleton("Input").unwrap();
    let platform=app.context().singleton("Platform").unwrap();
    assert_eq!(app.context().call(input,"observation",&[]).unwrap().to_json(),json!(null));
    {
        let rt=app.runtime_mut().unwrap();
        rt.host.pointer_observation=PointerObservation {position:Some([0,0]),inside:Some(true)};
        rt.tick(16,InputFrame {buttons_down:buttons::KEY_C|buttons::MOUSE_MIDDLE,buttons_pressed:buttons::KEY_C,..Default::default()}).unwrap();
    }
    let observed=app.context().call(input,"observation",&[]).unwrap().to_json();
    assert_eq!(observed["pointer"],json!([0,0]));
    assert_eq!(observed["focused"],json!(null));
    assert_eq!(observed["buttons_down"],(buttons::KEY_C|buttons::MOUSE_MIDDLE).to_string());
    assert!(observed["revision"].is_string()&&observed["time_ms"].is_string());
    assert_eq!(app.context().call(input,"observation",&[]).unwrap().to_json(),observed);
    assert!(matches!(app.context().call(input,"pointerActivity",&[]).unwrap(),Value::Null));
    app.runtime_mut().unwrap().host.note_pointer_activity().unwrap();
    assert_eq!(app.context().call(input,"pointerActivity",&[]).unwrap().to_json(),json!("1"));
    // A live pointer event is not a new admission and leaves the retained frame.
    assert_eq!(app.context().call(input,"observation",&[]).unwrap().to_json(),observed);
    let unavailable=app.context().call(platform,"integration",&[Value::from("steam")]).unwrap().to_json();
    assert_eq!(unavailable,json!({"client":1,"name":"steam","available":false,"backend":null}));
    assert!(app.context().call(platform,"integration",&[Value::from("unknown-sdk")]).is_err());
}

#[test]
fn added_keys_use_the_existing_action_map_and_vk_translation_keeps_unknown_distinct() {
    for (name,mask,vk) in [("c",buttons::KEY_C,0x43),("l",buttons::KEY_L,0x4c),("z",buttons::KEY_Z,0x5a),("mouse_middle",buttons::MOUSE_MIDDLE,4)] {
        let mut actions=ActionMap::default();
        actions.bind("test",&[name.into()]).unwrap();
        let input=InputFrame {buttons_down:mask,buttons_pressed:mask,..Default::default()};
        actions.begin_frame(input);assert!(actions.is_down("test").unwrap()&&actions.pressed("test").unwrap());
        actions.end_frame(input);
        assert_eq!(buttons::windows_virtual_key(vk),Some(mask));
    }
    assert_eq!(buttons::windows_virtual_key(104),None);
    assert_eq!(buttons::windows_virtual_key(0xa2),None);
}
