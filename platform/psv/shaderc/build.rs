// SPDX-License-Identifier: MPL-2.0
use std::{fmt::Write, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=CRYSIRROR_SHADER_SOURCES");
    let dir = PathBuf::from(
        std::env::var_os("CRYSIRROR_SHADER_SOURCES")
            .expect("export shared shaders first; set CRYSIRROR_SHADER_SOURCES"),
    );
    println!(
        "cargo:rerun-if-changed={}",
        dir.join("programs.txt").display()
    );
    let index = std::fs::read_to_string(dir.join("programs.txt")).expect("exported programs.txt");
    let mut paths: Vec<_> = index
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|name| {
            assert!(
                name.ends_with(".cg") && !name.contains(['/', '\\']),
                "shader file name"
            );
            dir.join(name)
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no Cg programs");
    let mut source = String::from("pub const SOURCES: &[(&str,bool,&str)] = &[\n");
    for path in paths {
        let path = path.canonicalize().unwrap();
        let stem = path.file_stem().unwrap().to_str().unwrap();
        assert!(
            stem.ends_with("_v") || stem.ends_with("_f"),
            "Cg stage suffix: {stem}"
        );
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(
            source,
            "({:?},{},include_str!({:?})),",
            stem,
            stem.ends_with("_f"),
            path.to_str().unwrap()
        )
        .unwrap();
    }
    source.push_str("];\npub const INCLUDES: &[(&str,&str)] = &[];\n");
    std::fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("sources.rs"),
        source,
    )
    .unwrap();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("vita") {
        for lib in ["SceShaccCg_stub", "m"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }
    }
}
