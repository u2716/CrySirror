//! Typed retained-font traversal without a font backend or native VM.
use crysirror_core::state::ClientId;
use crysirror_scene::{
    canvas::{CanvasCommand, CanvasSurface},
    layer::{LayerCommand, LayerSpec},
    ui::UiCommand,
    EngineSystems,
};
use serde_json::json;
use std::collections::BTreeSet;
fn text(id: u32, font: &str) -> LayerSpec {
    serde_json::from_value(
        json!({"id":id,"rect":[0,0,4,4],"paint":{"kind":"text","font":font,"size":12,"text":"A"}}),
    )
    .unwrap()
}
#[test]
fn retained_font_visitor_includes_nested_layers_unpresented_group_canvas_and_all_ui_clients_readonly(
) {
    let mut engine = EngineSystems::default();
    let nested = text(4, "snapshot");
    let snapshot: LayerSpec = serde_json::from_value(
        json!({"id":3,"rect":[0,0,4,4],"paint":{"kind":"snapshot","size":[4,4],"layers":[nested]}}),
    )
    .unwrap();
    engine
        .layers
        .apply_batch(
            ClientId(1),
            &[
                LayerCommand::Put {
                    layer: text(2, "layer"),
                },
                LayerCommand::Put { layer: snapshot },
            ],
        )
        .unwrap();
    let surface:CanvasSurface=serde_json::from_value(json!({"width":4,"height":4,"nodes":[{"kind":"group","offset":[0,0],"clip":[0,0,4,4],"opacity":255,"nodes":[{"kind":"draw","layer":text(5,"canvas")}]}]})).unwrap();
    engine
        .canvases
        .apply_batch(
            ClientId(2),
            &[
                CanvasCommand::Create {
                    id: 1,
                    width: 4,
                    height: 4,
                },
                CanvasCommand::Restore { id: 1, surface },
            ],
            &mut engine.layers,
        )
        .unwrap();
    let widget = serde_json::from_value(
        json!({"id":8,"kind":"label","rect":[0,0,4,4],"text":"W","font":"widget"}),
    )
    .unwrap();
    engine
        .ui
        .apply_batch(ClientId(1), &[UiCommand::Put { widget }])
        .unwrap();
    engine
        .ui
        .menus
        .put(
            2,
            serde_json::from_value(
                json!({"id":9,"rect":[0,0,4,1],"font":"menu","items":[{"id":1,"label":"File"}]}),
            )
            .unwrap(),
        )
        .unwrap();
    engine.ui.dialogs.open(1,serde_json::from_value(json!({"id":10,"font":"dialog","message":"M","buttons":[{"id":1,"label":"OK"}],"default":1})).unwrap()).unwrap();
    // This is the public snapshot representation, not a fabricated glyph/font.
    engine.ui.tooltips = serde_json::from_value(
        json!({"clients":{"2":{"7":{"spec":{"id":7,"text":"T","font":"tip"},"extent":[1,1]}}}}),
    )
    .unwrap();
    let before = serde_json::to_value(&engine).unwrap();
    let mut found = BTreeSet::new();
    engine.visit_font_dependencies(&mut |owner, font| {
        found.insert((owner.0, font.to_owned()));
    });
    assert_eq!(
        found,
        [
            (1, "layer"),
            (1, "snapshot"),
            (1, "widget"),
            (1, "dialog"),
            (2, "canvas"),
            (2, "menu"),
            (2, "tip")
        ]
        .into_iter()
        .map(|(owner, font)| (owner, font.to_owned()))
        .collect()
    );
    assert_eq!(serde_json::to_value(&engine).unwrap(), before);
}
