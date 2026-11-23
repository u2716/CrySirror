// SPDX-License-Identifier: MPL-2.0
//! Developer tool: search a locally installed KiriKiri game's scripts.
//! Output is for the developer's terminal only; never commit it.
//!
//! cargo run --release -p crysirror-launch --example krkr_grep -- GAME_DIR NEEDLE [LIMIT]
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).ok_or_else(|| anyhow::anyhow!("GAME_DIR"))?;
    let needle = args.get(2).ok_or_else(|| anyhow::anyhow!("NEEDLE"))?;
    let limit = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(200);
    for line in crysirror_launch::detect::kirikiri_script_search(std::path::Path::new(dir), needle, limit)? {
        println!("{line}");
    }
    Ok(())
}
