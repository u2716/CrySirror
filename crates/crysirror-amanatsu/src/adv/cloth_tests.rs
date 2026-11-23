// SPDX-License-Identifier: MPL-2.0
use super::ClothRun;
use crate::chara::ClothPart;

#[test]
fn set_animation_preserves_dither_on_each_mesh_of_only_the_selected_actor() {
    use serde_json::json;
    let animation = json!({"morphs": {"blink": 0.3}, "layers": [{"clip":"idle"}]});
    let mut world = vec![
        json!({"op": "set_animation", "id": 41, "animation": animation}),
        json!({"op": "set_animation", "id": 42, "animation": animation}),
        json!({"op": "set_animation", "id": 43, "animation": animation}),
        json!({"op": "despawn", "id": 42}),
    ];
    super::apply_actor_dither(&mut world, &[41, 42], 0.25);
    for cmd in &world[..2] {
        assert_eq!(cmd["animation"]["dither"], 0.25);
        assert_eq!(cmd["animation"]["morphs"], animation["morphs"]);
        assert_eq!(cmd["animation"]["layers"], animation["layers"]);
    }
    assert_eq!(world[2]["animation"], animation);
    assert_eq!(world[3], json!({"op":"despawn", "id":42}));
}

#[test]
fn underwater_override_preserves_each_requested_slot_until_clear() {
    let part = |root: &str, mask| ClothPart {
        list: "co_top",
        root: root.to_owned(),
        variants: vec![format!("{root}-a"), format!("{root}-b")],
        hide_category: mask,
    };
    let mut cloth = ClothRun {
        parts: vec![part("top", 1), part("gloves", 2)],
        states: vec![0, 0],
        underwater: true,
        accessories: vec![],
        accessories_hidden: false,
        acs_slots: vec![],
        male: false,
        son_config: false,
        son: false,
        hide_moz: false,
        gomu: false,
        body_nodes: vec![],
        entities: vec![],
        hidden_nodes: vec![],
        layers: Default::default(),
    };
    assert!(cloth.hidden().iter().any(|node| node == "top"));
    assert!(!cloth.hidden().iter().any(|node| node == "gloves"));
    cloth.states[0] = 1;
    assert!(cloth.hidden().iter().any(|node| node == "top"));
    cloth.underwater = false;
    let hidden = cloth.hidden();
    assert!(!hidden.iter().any(|node| node == "top" || node == "top-b" || node == "gloves-a"));
    assert!(hidden.iter().any(|node| node == "top-a"));
    assert!(hidden.iter().any(|node| node == "gloves-b"));
}
