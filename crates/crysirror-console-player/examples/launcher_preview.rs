// SPDX-License-Identifier: MPL-2.0
//! Developer tool: write the console launcher project for named games, to
//! render it with the desktop player (`crysirror render`).
//!
//! cargo run --release -p crysirror-console-player --example launcher_preview -- \
//!     ROOT WIDTH HEIGHT FONT [vita|switch] GAME_DIR...
use crysirror_console_player::{games::Game, launcher};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let display = (args[2].parse()?, args[3].parse()?);
    let font = std::fs::read(&args[4])?;
    let vita = args[5] == "vita";
    // Each GAME is a game folder (its name is the id) or a bare id.
    let games: Vec<Game> = args[6..]
        .iter()
        .map(|arg| {
            let path = PathBuf::from(arg);
            let id = path.file_name().map_or(arg.clone(), |n| n.to_string_lossy().into_owned());
            Game { id, dir: path }
        })
        .collect();
    let style = crysirror_library::library::Style {
        confirm: if vita { "" } else { "A" },
        cross: vita,
        add_hint: format!("Copy each game folder to {}", if vita { "ux0:data/CrySirror/games" } else { "sdmc:/switch/CrySirror/games" }),
    };
    let dir = launcher::prepare(&root, &games, display, Some(&font), &style)?;
    println!("{}", dir.display());
    Ok(())
}
