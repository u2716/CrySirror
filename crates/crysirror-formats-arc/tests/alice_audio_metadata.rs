// Copyright (c) 2026 CrySirror contributors. GPL-3.0-or-later.
//! Resource formats without a native VM or audio device.
use crysirror_core::{service::MemoryAssetReader, state::ClientId};
use crysirror_formats_arc::alice_audio_metadata::{
    MusicLoopCatalog, MusicMetadataVersion, WaveRoutingCatalog,
};
use crysirror_resource::DecodeContext;
fn music(text: &[u8]) -> MusicLoopCatalog {
    MusicLoopCatalog::load(
        &MemoryAssetReader(
            text.iter()
                .map(|b| b.rotate_left(4))
                .collect::<Vec<_>>()
                .into(),
        ),
        &DecodeContext::new(ClientId(9)),
    )
    .unwrap()
}
#[test]
fn old_and_new_music_rows_preserve_native_restart_defaults_and_first_duplicates() {
    let old = music(b"//comment\n1,2,30,80,-1\n1,0,20,90,-1,unused\n2 ,0,0,90,-1\n");
    assert_eq!(old.version, MusicMetadataVersion::FiveFields);
    assert_eq!(old.entries.len(), 2);
    let e = old.entry(1).unwrap();
    assert_eq!(
        (
            e.loop_count,
            e.loop_start,
            e.loop_end,
            e.mixer_group,
            e.volume_percent
        ),
        (2, 30, 80, -1, 100)
    );
    let new = music(b"1,2,30,80,-1\n2,3,4,50,1,77\n2,0,0,20,0,100\n");
    assert_eq!(new.version, MusicMetadataVersion::SixFields);
    assert_eq!(new.entries.len(), 2);
    assert!(new.entry(1).is_none());
    let e = new.entry(2).unwrap();
    assert_eq!((e.loop_count, e.mixer_group, e.volume_percent), (3, 1, 77));
}
#[test]
fn source_chunk_boundary_entry_limit_and_undefined_integer_overflow_are_bounded() {
    let mut text = vec![b'x'; 255];
    text.extend(b"4,0,0,12,-1\n");
    assert_eq!(music(&text).entry(4).unwrap().loop_end, 12);
    let text = (0..105)
        .map(|i| format!("{i},1,0,12,-1\n"))
        .collect::<String>();
    assert_eq!(music(text.as_bytes()).entries.len(), 100);
    let overflow = b"1,2147483648,0,5,-1\n"
        .iter()
        .map(|b| b.rotate_left(4))
        .collect::<Vec<_>>();
    assert!(MusicLoopCatalog::load(
        &MemoryAssetReader(overflow.into()),
        &DecodeContext::new(ClientId(9))
    )
    .is_err());
}
fn wave(version: u32, groups: &[i32]) -> Vec<u8> {
    let (header, stride) = if version == 3 { (36, 12) } else { (40, 16) };
    let mut b = vec![0; (header + stride * groups.len()).max(48)];
    b[..4].copy_from_slice(b"XI2\0");
    b[8..12].copy_from_slice(&(groups.len() as i32 + 1).to_le_bytes());
    b[12..16].copy_from_slice(&version.to_le_bytes());
    for (i, group) in groups.iter().enumerate() {
        b[header + i * stride + 8..header + i * stride + 12].copy_from_slice(&group.to_le_bytes());
    }
    b
}
#[test]
fn wave_versions_keep_one_based_routes_and_raw_fields_and_reject_bad_tables() {
    let cx = DecodeContext::new(ClientId(9));
    for version in [3, 4] {
        let b = wave(version, &[2, 7]);
        let c = WaveRoutingCatalog::load(&MemoryAssetReader(b.into()), &cx).unwrap();
        assert_eq!(c.version, version);
        assert!(c.entry(0).is_none());
        assert_eq!(c.entry(1).unwrap().mixer_group, 2);
        assert_eq!(c.entry(2).unwrap().mixer_group, 7);
        assert_eq!(
            c.entry(1).unwrap().words.len(),
            if version == 3 { 3 } else { 4 }
        );
        assert!(c.entry(3).is_none());
    }
    for b in [wave(5, &[]), wave(4, &[1, 2])[..49].to_vec(), vec![0; 48]] {
        assert!(WaveRoutingCatalog::load(&MemoryAssetReader(b.into()), &cx).is_err());
    }
    let mut cx = DecodeContext::new(ClientId(9));
    cx.limits.max_entries = 1;
    assert!(WaveRoutingCatalog::load(&MemoryAssetReader(wave(4, &[1, 2]).into()), &cx).is_err());
}
#[test]
#[ignore = "reads owner-installed audio metadata as runtime input, no game files committed"]
fn owner_rance7_audio_metadata() {
    let root = std::path::PathBuf::from(std::env::var("CRYSIRROR_SYSTEM4_ROOT").unwrap());
    let cx = DecodeContext::new(ClientId(9));
    let music = MusicLoopCatalog::load(
        &MemoryAssetReader(std::fs::read(root.join("戦国ランスBA.bgi")).unwrap().into()),
        &cx,
    )
    .unwrap();
    let wave = WaveRoutingCatalog::load(
        &MemoryAssetReader(std::fs::read(root.join("戦国ランスWA.wai")).unwrap().into()),
        &cx,
    )
    .unwrap();
    println!(
        "music_entries={} music_version={:?} wave_entries={} wave_version={}",
        music.entries.len(),
        music.version,
        wave.entries.len(),
        wave.version
    );
    assert!(!music.entries.is_empty());
    assert!(!wave.entries.is_empty());
}
