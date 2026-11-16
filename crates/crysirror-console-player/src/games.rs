// SPDX-License-Identifier: MPL-2.0
//! Game directories under the console root: `<root>/games/<id>/`.
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub id: String,
    pub dir: PathBuf,
}

/// Game directories, sorted by id. Hidden entries are skipped.
pub fn list(root: &Path) -> Result<Vec<Game>> {
    let games = root.join("games");
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for entry in std::fs::read_dir(&games).with_context(|| format!("list {}", games.display()))? {
        let entry = entry?;
        let Ok(id) = entry.file_name().into_string() else { continue };
        // Horizon fsdev metadata can be unreliable; a directory is whatever
        // read_dir can open.
        let dir = entry.path().is_dir() || std::fs::read_dir(entry.path()).is_ok();
        seen.push(format!("{id}:{dir}"));
        if id.starts_with('.') || !dir {
            continue;
        }
        out.push(Game { id, dir: entry.path() });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    if out.is_empty() && !seen.is_empty() {
        bail!("no game directory in {} (entries: {})", games.display(), seen.join(", "));
    }
    Ok(out)
}

/// The game named in `<root>/selected-game`, else the only game present.
pub fn select(root: &Path) -> Result<Game> {
    let games = list(root)?;
    if let Ok(name) = std::fs::read_to_string(root.join("selected-game")) {
        let name = name.trim();
        return games
            .into_iter()
            .find(|g| g.id == name)
            .with_context(|| format!("selected-game names a missing game: {name}"));
    }
    match games.len() {
        0 => bail!("no game in {}/games", root.display()),
        1 => Ok(games.into_iter().next().unwrap()),
        n => bail!("{n} games installed; name one in {}/selected-game", root.display()),
    }
}

/// Per-adapter, per-game save root (host storage policy in platform/*/host.toml).
pub fn save_root(root: &Path, adapter: &str, game: &Game) -> PathBuf {
    root.join("saves").join(adapter).join(&game.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection() {
        let t = std::env::temp_dir().join(format!("crysirror-games-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(t.join("games/b")).unwrap();
        assert_eq!(select(&t).unwrap().id, "b");
        std::fs::create_dir_all(t.join("games/a")).unwrap();
        assert!(select(&t).is_err());
        std::fs::write(t.join("selected-game"), "a\n").unwrap();
        assert_eq!(select(&t).unwrap().id, "a");
        assert_eq!(save_root(&t, "siglus", &select(&t).unwrap()), t.join("saves/siglus/a"));
        std::fs::remove_dir_all(&t).unwrap();
    }
}
