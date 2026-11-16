// SPDX-License-Identifier: MPL-2.0
//! Developer tool: report whether a font file covers the given characters
//! through the engine's simple text measurement.
//!
//! cargo run --release -p crysirror-text --example glyph_probe -- FONT TEXT
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut text = crysirror_text::TextSystem::default();
    text.add_font("default", std::fs::read(&args[1])?)?;
    for ch in args[2].chars() {
        let result = text.measure_simple("default", 16.0, &ch.to_string());
        println!("U+{:04X} {}", ch as u32, match result { Ok(e) => format!("advance {}", e.advance), Err(e) => format!("error {e}") });
    }
    Ok(())
}
