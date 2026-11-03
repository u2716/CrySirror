use crysirror_system4::assets::{Class, Volume, classify};
#[test]
fn native_archive_classification_uses_original_program_base_and_volume_letter() {
    assert_eq!(
        classify("戦国ランス.ain", "戦国ランスGA.ald"),
        Some((Class::Cg, 0))
    );
    assert_eq!(classify("game.ain", "GAMEwZ.ALD"), Some((Class::Sound, 25)));
    assert_eq!(classify("game.ain", "gameBA.ald"), Some((Class::Music, 0)));
    for file in [
        "otherGA.ald",
        "gameGA.ald.bak",
        "gameG0.ald",
        "gameVA.ald",
        "game.ald",
    ] {
        assert_eq!(classify("game.ain", file), None);
    }
    assert!(
        Volume {
            class: Class::Cg,
            volume: 0,
            source: "gameGA.ald".into(),
            mount: "__system4/cg/0".into()
        }
        .validate()
        .is_ok()
    );
    assert!(
        Volume {
            class: Class::Cg,
            volume: 26,
            source: "gameGA.ald".into(),
            mount: "__system4/cg/0".into()
        }
        .validate()
        .is_err()
    );
    assert!(
        Volume {
            class: Class::Cg,
            volume: 0,
            source: "../gameGA.ald".into(),
            mount: "__system4/cg/0".into()
        }
        .validate()
        .is_err()
    );
}
