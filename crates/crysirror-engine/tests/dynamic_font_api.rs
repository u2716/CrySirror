//! The public Text object reaches shared asynchronous fonts without a native VM.
use crysirror_engine::{App, Value};
use crysirror_runtime::Runtime;
use serde_json::{Value as Json, json};
use std::time::{Duration, Instant};
fn call(app: &mut App, name: &str, args: Json) -> anyhow::Result<Json> {
    let cx = app.context();
    let text = cx.singleton("Text")?;
    Ok(cx
        .call(
            text,
            name,
            &args
                .as_array()
                .unwrap()
                .iter()
                .cloned()
                .map(Value::from_json)
                .collect::<Vec<_>>(),
        )?
        .to_json())
}

#[test]
fn unit_flow_is_a_public_font_independent_engine_operation() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("assets")).unwrap();
    std::fs::write(d.path().join("project.json"),br#"{"schema":2,"name":"unit flow","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"}]}"#).unwrap();
    let mut app = App::new(
        Runtime::load(&d.path().join("project.json"), &d.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    let items = json!([
        {"kind":"glyph","character":"A","size":13,"units":1,"style":7},
        {"kind":"glyph","character":"あ","size":13,"units":2},
        {"kind":"move","x":80},
        {"kind":"glyph","character":"B","size":9,"units":1},
        {"kind":"glyph","character":"C","size":20,"units":1}
    ]);
    let result = call(
        &mut app,
        "layoutUnitFlow",
        json!([items,{"spacing":[-2,3],"wrap_units":4}]),
    )
    .unwrap();
    assert_eq!(result["extent"], json!([89, 43]));
    assert_eq!(result["glyphs"][3]["position"], json!([0, 23]));
    assert_eq!(result["glyphs"][0]["style"], 7);
    assert!(
        call(
            &mut app,
            "layoutUnitFlow",
            json!([[{"kind":"glyph","character":"AB","size":12,"units":1}]])
        )
        .is_err()
    );
    assert!(call(&mut app, "layoutUnitFlow", json!([[],{"wrap_pixels":24}])).is_err());
    assert_eq!(app.runtime().unwrap().now_ms(), 0);
}

#[test]
fn public_alias_and_scalar_metrics_use_authorized_shared_faces_without_font_files() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("assets")).unwrap();
    let mut bytes = b"TVP pre-rendered font\x1a".to_vec();
    bytes.extend([1, 2]);
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(36u32.to_le_bytes());
    bytes.extend(38u32.to_le_bytes());
    bytes.extend((b'A' as u16).to_le_bytes());
    bytes.extend(58u32.to_le_bytes());
    for value in [2i16, 2, -1, 2, 3, 0, 4, 0] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend([32, 0x43]);
    std::fs::write(d.path().join("assets/authored.tft"), bytes).unwrap();
    std::fs::write(d.path().join("project.json"),br#"{"schema":2,"name":"font metrics","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b"}]}"#).unwrap();
    let mut app = App::new(
        Runtime::load(&d.path().join("project.json"), &d.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    assert!(
        call(&mut app, "resolveFontAlias", json!(["missing"]))
            .unwrap()
            .is_null()
    );
    let ticket = call(
        &mut app,
        "beginPrecomputedFont",
        json!(["authored", "authored.tft", 24]),
    )
    .unwrap();
    let start = Instant::now();
    let info = loop {
        let status = call(&mut app, "fontStatus", json!([ticket])).unwrap();
        if status["state"] == "ready" {
            break status["font"].clone();
        }
        assert_eq!(status["state"], "pending", "{status}");
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::yield_now();
    };
    assert_eq!(
        call(&mut app, "resolveFontAlias", json!(["authored"])).unwrap(),
        info["alias"]
    );
    assert_eq!(call(&mut app,"glyphId",json!([info["alias"],"A"])).unwrap(),json!(1));
    assert!(call(&mut app,"fontStyle",json!([info["alias"]])).unwrap().is_null());
    assert!(call(&mut app,"glyphId",json!([info["alias"],"AA"])).is_err());
    let metrics = call(&mut app, "fontMetrics", json!([info["alias"], 24, "A"])).unwrap();
    assert_eq!(metrics["advance"], 4.);
    assert!(call(&mut app, "fontMetrics", json!([info["alias"], 32, "A"])).is_err());
    let bounds_ticket=call(&mut app,"beginGlyphBounds",json!(["authored",[{"character":"A","size":24,"baseline":[1,2]},{"character":"A","size":24,"baseline":[4,5]}]])).unwrap();
    let start = Instant::now();
    let geometry = loop {
        let status = call(&mut app, "glyphBoundsStatus", json!([bounds_ticket])).unwrap();
        if status["state"] == "ready" {
            break status;
        }
        assert_eq!(status["state"], "pending", "{status}");
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::yield_now();
    };
    assert_eq!(geometry["glyphs"][0]["origin"], json!([0, 0]));
    assert_eq!(geometry["glyphs"][0]["size"], json!([2, 2]));
    assert_eq!(geometry["glyphs"][1]["origin"], json!([3, 3]));
    assert_eq!(geometry["digest"], info["digest"]);
    assert_eq!(
        call(&mut app, "glyphBoundsStatus", json!([bounds_ticket])).unwrap(),
        geometry
    );
    for scalar in ["", "AA"] {
        assert!(call(&mut app, "fontMetrics", json!([info["alias"], 24, scalar])).is_err());
    }
    let b = app.runtime().unwrap().client_id("b").unwrap();
    app.context().state().unwrap().client = b;
    assert!(call(&mut app, "glyphBoundsStatus", json!([bounds_ticket])).is_err());
    assert!(call(&mut app, "releaseGlyphBounds", json!([bounds_ticket])).is_err());
    assert!(
        call(&mut app, "resolveFontAlias", json!([info["alias"]]))
            .unwrap()
            .is_null()
    );
    assert!(call(&mut app, "fontMetrics", json!([info["alias"], 24, "A"])).is_err());
    assert!(call(&mut app,"glyphId",json!([info["alias"],"A"])).is_err());
    assert!(call(&mut app,"fontStyle",json!([info["alias"]])).is_err());
}
#[test]
#[ignore = "requires licensed runtime system font; font bytes are never committed"]
fn text_object_registers_shared_fonts_and_checks_owner_before_measurement() {
    let d = tempfile::tempdir().unwrap();
    for name in ["a", "b"] {
        std::fs::create_dir_all(d.path().join("assets").join(name)).unwrap();
    }
    std::fs::copy(
        std::env::var("CRYSIRROR_TEST_FONT").unwrap(),
        d.path().join("assets/a/runtime.ttf"),
    )
    .unwrap();
    std::fs::write(d.path().join("project.json"),serde_json::to_vec(&json!({"schema":2,"name":"dynamic font API","content_revision":"1","canvas":[256,64],"assets":"assets","clients":[{"name":"a","adapter":"crysirror-script-v1","game":"a","asset_prefix":"a"},{"name":"b","adapter":"crysirror-script-v1","game":"b","asset_prefix":"b"}]})).unwrap()).unwrap();
    let mut app = App::new(
        Runtime::load(&d.path().join("project.json"), &d.path().join("state")).unwrap(),
        "a",
    )
    .unwrap();
    app.init().unwrap();
    app.start().unwrap();
    let ticket = call(&mut app, "beginFont", json!(["game", "runtime.ttf"])).unwrap();
    let start = Instant::now();
    let font = loop {
        let status = call(&mut app, "fontStatus", json!([ticket])).unwrap();
        if status["state"] == "ready" {
            break status["font"].clone();
        }
        assert_ne!(status["state"], "failed", "{status}");
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::yield_now();
    };
    let alias = font["alias"].as_str().unwrap();
    assert_eq!(
        call(&mut app, "resolveFontAlias", json!(["game"])).unwrap(),
        alias
    );
    assert!(
        call(&mut app, "fontMetrics", json!([alias, 24, "A"])).unwrap()["advance"]
            .as_f64()
            .unwrap()
            > 0.
    );
    let metrics = call(&mut app, "measureSimple", json!([alias, 24, "Hello"])).unwrap();
    assert!(metrics["advance"].as_f64().unwrap() > 0.);
    assert_eq!(
        call(&mut app, "resolveFont", json!(["game"])).unwrap(),
        font
    );
    let second = app.runtime().unwrap().client_id("b").unwrap();
    app.context().state().unwrap().client = second;
    assert!(call(&mut app, "fontStatus", json!([ticket])).is_err());
    assert!(call(&mut app, "measureSimple", json!([alias, 24, "Hello"])).is_err());
    assert!(
        call(&mut app, "resolveFont", json!([alias]))
            .unwrap()
            .is_null()
    );
    let first = app.runtime().unwrap().client_id("a").unwrap();
    app.context().state().unwrap().client = first;
    call(&mut app, "releaseFontJob", json!([ticket])).unwrap();
    call(&mut app, "releaseFont", json!([font["handle"]])).unwrap();
    assert!(call(&mut app, "measureSimple", json!([alias, 24, "Hello"])).is_err());
}
