//! Shared JSON and positional binary clocks round trip without a native VM.
use crysirror_scene::layer::{GlyphReveal,GlyphRevealUnit};
#[test]
fn optional_rate_keeps_sparse_json_and_complete_binary_fields() {
    for rate in [None,Some(60)] {
        let reveal=GlyphReveal{unit:GlyphRevealUnit::UnicodeScalar,interval_ms:40,rate_per_second:rate,initial_visible:3,elapsed_ms:127,forced_complete:false,paused:true};
        let json=serde_json::to_value(&reveal).unwrap();
        assert_eq!(json.get("rate_per_second").is_some(),rate.is_some());
        assert_eq!(serde_json::from_value::<GlyphReveal>(json).unwrap(),reveal);
        // A following field exposes accidental omission/misalignment, rather
        // than just testing an isolated value's encoded length.
        let packet=(reveal,0x12345678u32,vec![7u8,8,9]);
        let bytes=bincode::serialize(&packet).unwrap();
        let decoded:(GlyphReveal,u32,Vec<u8>)=bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded,packet);
    }
}
