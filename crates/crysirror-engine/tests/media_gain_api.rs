//! Independent public Media amplitude controls preserve engine clock and scope.
use crysirror_core::{
    service::{HostServices, MediaRequest},
    state::ClientId,
};
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
#[test]
fn public_media_gain_retains_exact_controls_without_restarting_or_advancing_clock() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    std::fs::write(
        root.path().join("assets/movie.mpg"),
        include_bytes!("../../crysirror-video/tests/fixtures/media_av.mpg"),
    )
    .unwrap();
    std::fs::write(root.path().join("project.json"),br#"{"schema":2,"name":"media-gain","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"app","adapter":"crysirror-script-v1","game":"gain"},{"name":"peer","adapter":"crysirror-script-v1","game":"other"}]}"#).unwrap();
    let rt = Runtime::load(
        &root.path().join("project.json"),
        &root.path().join("state"),
    )
    .unwrap();
    let mut app = App::new(rt, "app").unwrap();
    let media = app.context().singleton("Media").unwrap();
    let id = Value::Int(9);
    app.context()
        .call(
            media,
            "play",
            &[
                id.clone(),
                Value::Str("movie.mpg".into()),
                Value::from_json(serde_json::json!({"audio":false})),
            ],
        )
        .unwrap();
    app.context()
        .call(media, "setGain", &[id.clone(), Value::Float(0.375)])
        .unwrap();
    assert_eq!(
        app.context()
            .call(media, "gain", &[id.clone()])
            .unwrap()
            .as_f64()
            .unwrap(),
        0.375
    );
    let before = app
        .context()
        .call(media, "status", &[id.clone()])
        .unwrap()
        .to_json();
    for bad in [f64::NAN, -0.1, 4.000001, 1e300] {
        assert!(
            app.context()
                .call(media, "setGain", &[id.clone(), Value::Float(bad)])
                .is_err()
        );
        assert_eq!(
            app.context()
                .call(media, "gain", &[id.clone()])
                .unwrap()
                .as_f64()
                .unwrap(),
            0.375
        );
    }
    let rt = app.runtime_mut().unwrap();
    assert!(
        rt.host
            .media_request(ClientId(2), MediaRequest::SetGain { id: 9, gain: 0. })
            .is_err()
    );
    assert_eq!(rt.host.media.gain(ClientId(1), 9), Some(0.375));
    assert_eq!(rt.now_ms(), 0);
    let after = app
        .context()
        .call(media, "status", &[id.clone()])
        .unwrap()
        .to_json();
    assert_eq!(before["published_frames"], after["published_frames"]);
    assert_eq!(before["position_ms"], after["position_ms"]);
    assert_eq!(before["audio_started"], after["audio_started"]);
    assert!(matches!(
        app.context().call(media, "gain", &[Value::Int(8)]).unwrap(),
        Value::Null
    ));
    app.context().call(media, "remove", &[id.clone()]).unwrap();
    assert!(matches!(
        app.context().call(media, "gain", &[id]).unwrap(),
        Value::Null
    ));
}
