// SPDX-License-Identifier: MPL-2.0
//! Character package lists of `dc1.psarc/spawn-character.bin`.
//!
//! `*dev-spawn-characters*` is `{count, array}` of 0x14-byte entries
//! `{name, StringId, spawn info, packages, models}`; both lists are
//! `{count, array of {string, StringId}}`. Packages name the skeleton
//! (`elena-base`, `flynn-base`, `npc-normal-base`), the animation packages
//! (`anim-elena`) and weapons (`pistol-base`, `beretta-pistol`); models are
//! the art groups (`elena-winter`). The player's entry lists only
//! `anim-proto-multiplayer`: its models (`hero-…`) use the rig in
//! `proto.pak`.
use crate::dc::{Dc, sid};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub name: String,
    pub packages: Vec<String>,
    pub models: Vec<String>,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Character>> {
    let mut dc = Dc::default();
    let module = dc.load(sid("spawn-character"), bytes)?.clone();
    let m = &dc.mem;
    let root = module.entries.iter().find(|(n, _)| *n == sid("*dev-spawn-characters*")).map(|(_, a)| *a).context("spawn-character.bin without *dev-spawn-characters*")?;
    let text = |at: u32| -> String { m.u32(at).ok().filter(|p| *p != 0).and_then(|p| m.cstr(p).ok()).unwrap_or_default() };
    let list = |at: u32| -> Result<Vec<String>> {
        if at == 0 {
            return Ok(Vec::new());
        }
        let n = m.u32(at)?;
        ensure!(n <= 1024, "ND character list size");
        let a = m.u32(at + 4)?;
        Ok((0..n).map(|i| text(a + 8 * i)).filter(|s| !s.is_empty()).collect())
    };
    let count = m.u32(root)?;
    ensure!(count <= 1024, "ND character count");
    let array = m.u32(root + 4)?;
    (0..count)
        .map(|i| {
            let e = array + 0x14 * i;
            Ok(Character { name: text(e), packages: list(m.u32(e + 0xc)?)?, models: list(m.u32(e + 0x10)?)? })
        })
        .collect()
}

/// The character an art group belongs to: by model name, then by a listed
/// model the art group extends (`flynn-heist` → `flynn`).
pub fn character_of<'a>(characters: &'a [Character], art: &str) -> Option<&'a Character> {
    characters.iter().find(|c| c.models.iter().any(|m| m == art)).or_else(|| {
        characters.iter().filter(|c| c.models.iter().any(|m| art.starts_with(&format!("{m}-")))).max_by_key(|c| c.models.iter().filter(|m| art.starts_with(&format!("{m}-"))).map(String::len).max())
    })
}

/// Where a skinned art group without its own skeleton gets one: the
/// skeleton package (`None` for the player rig in `proto.pak`) and the
/// character's animation packages. Matched by model name, then by a
/// listed model the art group extends (`flynn-heist` → `flynn`), then
/// `hero-…` as the player and `…pistol…` as `pistol-base`.
pub fn rig_of(characters: &[Character], art: &str) -> Option<(Option<String>, Vec<String>)> {
    let found = character_of(characters, art);
    let anims = |c: &Character| c.packages.iter().filter(|p| p.starts_with("anim-")).cloned().collect::<Vec<_>>();
    match found {
        Some(c) if c.name == "Player" => Some((None, anims(c))),
        Some(c) => c.packages.iter().find(|p| p.ends_with("-base") && *p != "pistol-base").map(|b| (Some(b.clone()), anims(c))),
        None if art.starts_with("hero-") => Some((None, Vec::new())),
        None if art.contains("pistol") => Some((Some("pistol-base".into()), Vec::new())),
        None => None,
    }
}
