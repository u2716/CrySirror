//! Authored native configuration grammar; no game data in fixtures.

use crysirror_system4::bootstrap::{Bootstrap, Document, Value};
#[test]
fn native_ini_lists_sparse_assignment_literal_backslashes_and_formations() {
    let bytes = br#"; native comment
CodeName="game.ain"
GameName="Example"
ViewWidth=1024 ViewHeight=768
SaveFolder="SaveData"
MainVM="Sys42VM" UseJoypad=true
DefaultVolumeRate=75
VolumeValancer[2]="voice",60
VolumeValancer[0]="BGM"
DLL="DLL\native.dll"
Formation "preferences" { speed=3 list={1,2,{"nested",},} }
"#;
    let d = Document::parse(bytes).unwrap();
    assert!(d.warnings.is_empty());
    assert_eq!(
        d.entries.iter().find(|e| e.name == "DLL").unwrap().value,
        Value::String("DLL\\native.dll".into())
    );
    assert!(matches!(
        d.entries.last().unwrap().value,
        Value::Formation(_)
    ));
    let b = Bootstrap::from_document(&d).unwrap();
    assert_eq!(b.program, "game.ain");
    assert_eq!(b.canvas, [1024, 768]);
    assert!(b.joypad);
    assert_eq!(b.native_vm.as_deref(), Some("Sys42VM"));
    assert_eq!(
        b.mixers
            .iter()
            .map(|m| (m.name.as_str(), m.volume))
            .collect::<Vec<_>>(),
        vec![("BGM", 75), ("Unnamed", 0), ("voice", 60)]
    );
}
#[test]
fn native_ini_duplicate_order_and_invalid_inputs_are_explicit() {
    let d = Document::parse(br#"CodeName="old.ain" CodeName="new.ain" x=1 x[0]=2"#).unwrap();
    assert_eq!(Bootstrap::from_document(&d).unwrap().program, "new.ain");
    assert_eq!(d.warnings.len(), 1);
    for bytes in [
        b"CodeName=\"a.ain\"\0".as_slice(),
        b"x={1,}",
        b"Formation \"empty\" {}",
        b"x[1024]=1",
        b"x=\"unterminated\n",
        b"x=-1",
        b"x=true,",
        b"x={{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{1}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}",
    ] {
        // {1,} itself is valid; all remaining malformed grammar is rejected.
        if bytes == b"x={1,}" {
            assert!(Document::parse(bytes).is_ok());
        } else {
            assert!(Document::parse(bytes).is_err(), "{bytes:?}");
        }
    }
    for bytes in [
        br#"CodeName="../game.ain""#.as_slice(),
        br#"CodeName="g.ain" ViewWidth=0"#,
        br#"CodeName="g.ain" UseJoypad=1"#,
        br#"CodeName="g.ain" VolumeValancer={1}"#,
        br#"CodeName="g.exe""#,
    ] {
        assert!(
            Bootstrap::from_document(&Document::parse(bytes).unwrap()).is_err(),
            "{bytes:?}"
        );
    }
}
#[test]
fn cp932_configuration_is_decoded_without_loss() {
    let (bytes, _, err) =
        encoding_rs::SHIFT_JIS.encode("CodeName=\"戦国ランス.ain\" GameName=\"戦国ランス\"");
    assert!(!err);
    let b = Bootstrap::from_document(&Document::parse(&bytes).unwrap()).unwrap();
    assert_eq!(b.game_name, "戦国ランス");
    assert_eq!(b.program, "戦国ランス.ain");
    assert!(Document::parse(&[0x81]).is_err());
}
#[test]
#[ignore = "reads caller-selected original System40.ini from owner game"]
fn owner_rance7_bootstrap_is_native_no_generated_manifest() {
    let root = std::path::PathBuf::from(std::env::var("CRYSIRROR_SYSTEM4_ROOT").unwrap());
    let b = Bootstrap::from_document(
        &Document::parse(&std::fs::read(root.join("System40.ini")).unwrap()).unwrap(),
    )
    .unwrap();
    println!("native_ini={b:?}");
    assert_eq!(b.canvas, [800, 600]);
    assert_eq!(b.program, "戦国ランス.ain");
    assert!(root.join(b.program).is_file());
    assert_eq!(b.mixers.len(), 2);
}
