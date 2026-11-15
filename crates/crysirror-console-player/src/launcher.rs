// SPDX-License-Identifier: MPL-2.0
//! Console use of the shared game library launcher (crysirror-launch
//! `library`): shown when several games are installed and none is named in
//! `<root>/selected-game`. The project is written to `<root>/cache/launcher`
//! (covers cached in `<root>/cache/covers`) and run by the shared runtime and
//! renderer like any authored project; the chosen game is read back from
//! the launcher's save directory.
use crate::games::Game;
use anyhow::Result;
use crysirror_library::library;
use std::path::{Path, PathBuf};

/// Whether to show the launcher instead of opening a game directly: no
/// `selected-game` file and not exactly one installed game.
pub fn wanted(root: &Path, games: &[Game]) -> bool {
    !root.join("selected-game").exists() && games.len() != 1
}

/// Write the launcher project for `games`; returns its directory.
pub fn prepare(root: &Path, games: &[Game], display: (u32, u32), font: Option<&[u8]>, style: &library::Style) -> Result<PathBuf> {
    let covers = root.join("cache").join("covers");
    let entries: Vec<library::Entry> = games
        .iter()
        .map(|game| library::Entry { id: game.id.clone(), ..library::Entry::folder(&game.dir, Some(&covers)) })
        .collect();
    let dir = root.join("cache").join("launcher");
    library::write(&dir, &library::build(&entries, display, font, style)?)?;
    Ok(dir)
}

/// The game the launcher chose, read (and removed) from its save directory.
pub fn choice(state: &Path, games: &[Game]) -> Option<Game> {
    let id = library::take_choice(state)?;
    games.iter().find(|g| g.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_only_without_selection_and_single_game() {
        let root = std::env::temp_dir().join(format!("crysirror-console-launcher-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let games: Vec<Game> = ["a", "b"].iter().map(|id| Game { id: id.to_string(), dir: root.join(id) }).collect();
        assert!(wanted(&root, &games) && !wanted(&root, &games[..1]) && wanted(&root, &[]));
        std::fs::write(root.join("selected-game"), "a").unwrap();
        assert!(!wanted(&root, &games));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
