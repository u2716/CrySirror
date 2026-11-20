// SPDX-License-Identifier: MPL-2.0
//! Reads the installed Custom Maid 3D 2 archives named by `CRYSIRROR_CM3D2_GAME`.
use crysirror_core::state::ClientId;
use crysirror_core::service::AssetReader;
use crysirror_resource::{read_all, DecodeContext, FormatRegistry};
use std::sync::Arc;

#[test]
#[ignore = "requires the original CM3D2 GameData directory"]
fn cm3d2_archives_list_and_open_entries() -> anyhow::Result<()> {
    let game = std::path::PathBuf::from(std::env::var_os("CRYSIRROR_CM3D2_GAME").expect("game root"));
    let mut registry = FormatRegistry::default();
    crysirror_formats_arc::register(&mut registry)?;
    let cx = DecodeContext::new(ClientId(1));
    for (name, count, sample) in [("script.arc", 5061, "ck_motion/sex/ck_sex_fera_dt.ks"), ("csv.arc", 44, ""), ("tjs.arc", 3, "")] {
        let bytes = std::fs::read(game.join("GameData").join(name))?;
        let source: Arc<dyn AssetReader> = Arc::new(crysirror_core::service::MemoryAssetReader(bytes.into()));
        let (id, archive) = registry.open(source, Some("kiss/warc"), &cx)?;
        assert_eq!(id, "kiss/warc");
        assert_eq!(archive.entries().len(), count, "{name}");
        for entry in archive.entries() {
            let data = read_all(archive.open(entry.id, &cx)?.as_ref(), 64 << 20)?;
            assert_eq!(data.len() as u64, entry.size, "{}", entry.path);
        }
        if !sample.is_empty() {
            assert!(archive.entries().iter().any(|entry| entry.path == sample));
        }
    }
    Ok(())
}
