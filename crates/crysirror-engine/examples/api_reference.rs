// SPDX-License-Identifier: MPL-2.0
//! Regenerate public API artifacts without compiling or executing test cases.
#[path = "../tests/api_docs.rs"]
mod reference;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docs = reference::generate();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("docs/api");
    std::fs::create_dir_all(&dir)?;
    for (name, content) in [
        ("REFERENCE.md", &docs.markdown),
        ("api.json", &docs.json),
        ("crysirror.d.ts", &docs.typescript),
    ] {
        std::fs::write(dir.join(name), content)?;
    }
    println!("{}", docs.fingerprint);
    Ok(())
}
