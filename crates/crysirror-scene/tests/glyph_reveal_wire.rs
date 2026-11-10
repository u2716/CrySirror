//! Shared JSON and positional binary clocks round trip without a native VM.
use crysirror_scene::layer::{GlyphReveal, GlyphRevealUnit};
#[test]
fn optional_rate_keeps_sparse_json_and_complete_binary_fields() {
    for rate in [None, Some(60)] {
        let reveal = GlyphReveal {
            unit: GlyphRevealUnit::UnicodeScalar,
            interval_ms: 40,
            rate_per_second: rate,
            initial_visible: 3,
            prefix_steps: None,
            elapsed_ms: 127,
            forced_complete: false,
            paused: true,
        };
        let json = serde_json::to_value(&reveal).unwrap();
        assert_eq!(json.get("rate_per_second").is_some(), rate.is_some());
        assert_eq!(serde_json::from_value::<GlyphReveal>(json).unwrap(), reveal);
        // A following field exposes accidental omission/misalignment, rather
        // than just testing an isolated value's encoded length.
        let packet = (reveal, 0x12345678u32, vec![7u8, 8, 9]);
        let bytes = bincode::serialize(&packet).unwrap();
        let decoded: (GlyphReveal, u32, Vec<u8>) = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded, packet);
    }
}

#[test]
fn changing_rate_retains_cluster_boundary_and_forced_completion() {
    let mut reveal = GlyphReveal {
        unit: GlyphRevealUnit::UnicodeScalar,
        interval_ms: 0,
        rate_per_second: Some(20),
        initial_visible: 0,
        prefix_steps: Some(vec![2, 2, 5, 8]),
        elapsed_ms: 100,
        forced_complete: false,
        paused: true,
    };
    assert_eq!(reveal.visible_count(8), 2);
    reveal.change_rate(8, 40).unwrap();
    assert_eq!(reveal.visible_count(8), 2);
    assert_eq!(reveal.prefix_steps, Some(vec![5, 8]));
    assert!(reveal.paused);
    reveal.elapsed_ms = 25;
    assert_eq!(reveal.visible_count(8), 5);
    reveal.forced_complete = true;
    reveal.change_rate(8, 60).unwrap();
    assert_eq!(reveal.visible_count(8), 8);
    assert_eq!(reveal.prefix_steps, Some(vec![]));
    let before = reveal.clone();
    assert!(reveal.change_rate(8, 0).is_err());
    assert_eq!(reveal, before);
}
