// SPDX-License-Identifier: MPL-2.0
//! `AL.Memory` (回想): the short movie list (`MemoryManager`: the newest
//! `memory/list/` bundle's `Memory_Thumbnail` MemoryPack data, titles
//! from `sm_timeline`, unlocked by `GlobalData.Flags`), the heroine card
//! list (`EntryFileListCtrl`), the time zone toggles and play
//! (`MovieManager.ClickMemory`/`PlayShortMove`), back to the title
//! (`BackTitle`).
//!
//! SATD-amanatsu-port: the card list's sources/sorting/personal view, the
//! hover frames (`OnPointMemory`), the grid's scrolling (`LoopGridView`
//! shows its first rows), the title's `Localize` text and the play
//! button's interactable text swap are not ported.
use crate::{charaselect::Grid, level::Level, unity::Unity};
use anyhow::{Context, Result};
use crysirror_unity::{managed::Value, memorypack::Reader};

/// `MemoryThumbnailData` entry: memory id, short movie id, thumbnail
/// sprite (bundle, asset).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thumbnail {
    pub id: i32,
    pub movie: i32,
    pub bundle: String,
    pub asset: String,
}

/// `MemoryManager.MemoryLoad`: the newest bundle under `memory/list/`,
/// its `Memory_Thumbnail` (a dictionary of `ThumbnailData`), by id.
pub fn thumbnails(unity: &mut Unity) -> Result<Vec<Thumbnail>> {
    let mut bundles: Vec<String> = unity.bundle_files().into_iter().filter_map(|k| k.strip_prefix("bundle:lib/memory/list/").and_then(|r| r.split_once('#')).map(|(b, _)| format!("lib/memory/list/{b}"))).collect();
    bundles.sort();
    bundles.dedup();
    let bundle = bundles.last().context("memory/list bundle")?;
    let (key, id) = unity.bundle_asset(bundle, "Memory_Thumbnail", crysirror_unity::class_id::TEXT_ASSET)?.context("Memory_Thumbnail")?;
    parse_thumbnails(&unity.text_asset(&key, id)?)
}

/// `MemoryThumbnailData`: [1 member] count, then (key, [4 members] id,
/// short movie id, bundle, asset).
pub fn parse_thumbnails(b: &[u8]) -> Result<Vec<Thumbnail>> {
    let mut r = Reader::new(b);
    r.object_header()?.context("null MemoryThumbnailData")?;
    let n = r.i32()?;
    anyhow::ensure!((0..=4096).contains(&n), "MemoryThumbnailData count {n}");
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let _key = r.i32()?;
        r.object_header()?.context("null ThumbnailData")?;
        let id = r.i32()?;
        let movie = r.i32()?;
        let bundle = r.string()?.unwrap_or_default();
        let asset = r.string()?.unwrap_or_default();
        out.push(Thumbnail { id, movie, bundle, asset });
    }
    out.sort_by_key(|t| t.id);
    Ok(out)
}

/// `MemoryManager.GetTitle`: `sm_timeline` id → name.
pub fn titles(unity: &mut Unity) -> Result<std::collections::HashMap<i32, String>> {
    let (key, id) = unity.bundle_asset("lib/sm/lis/000_00.unity3d", "sm_timeline[000_00]", crysirror_unity::class_id::TEXT_ASSET)?.context("sm_timeline")?;
    let t = crate::tables::Table::parse(&unity.text_asset(&key, id)?)?;
    // `TableData.Key`: ID 5, Name 6.
    let (Some(i), Some(n)) = (t.keys.iter().position(|&k| k == 5), t.keys.iter().position(|&k| k == 6)) else { anyhow::bail!("sm_timeline columns {:?}", t.keys) };
    Ok(t.rows.iter().filter_map(|r| Some((r.get(i)?.parse().ok()?, r.get(n)?.clone()))).collect())
}

/// One listed memory.
pub struct Entry {
    pub thumbnail: Thumbnail,
    pub title: String,
    /// `AlreadyRead`: the short movie was played.
    pub unlocked: bool,
    /// Uploaded thumbnail image.
    pub image: Option<String>,
}

pub enum Outcome {
    None,
    /// `BackTitle.LoadTitle`.
    Back,
    /// `PlayShortMove`: short movie, time zone, heroine card.
    Play { movie: i32, zone: i32, card: String },
}

pub struct Memory {
    entries: Vec<Entry>,
    grid: Option<Grid>,
    selected: Option<usize>,
    /// (`Selected` frame, `Selectable` frame) of each item.
    frames: Vec<(Option<usize>, Option<usize>)>,
    title: Option<usize>,
    /// Morning (1), evening (2), night (3): toggle and checkmark nodes.
    toggles: Vec<(usize, Option<usize>, i32)>,
    zone: i32,
    play: Option<usize>,
    back: Option<usize>,
    cards: Option<Grid>,
    card: Option<usize>,
    card_image: Option<usize>,
    card_images: Vec<Option<String>>,
}

fn named_child(level: &Level, node: usize, name: &str) -> Option<usize> {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if level.scene.nodes[n].name == name {
            return Some(n);
        }
        stack.extend(&level.scene.nodes[n].children);
    }
    None
}

impl Memory {
    pub fn new(level: &mut Level, entries: Vec<Entry>, cards: &[(String, Option<String>)]) -> Self {
        let scene = &level.scene;
        let pptr_node = |fields: &crysirror_unity::managed::Fields, k: &str| fields.pptr(k).and_then(|p| crate::charaselect::node_of(scene, p.path_id));
        let manager = scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.Memory.MovieManager").map(|c| c.fields.clone());
        let title = manager.as_ref().and_then(|m| pptr_node(m, "_title"));
        let play = manager.as_ref().and_then(|m| pptr_node(m, "_pleyButton"));
        let toggles: Vec<(usize, Option<usize>, i32)> = [("_toggleMorning", 1), ("_togglEvening", 2), ("_togglNight", 3)]
            .iter()
            .filter_map(|(k, zone)| {
                let t = pptr_node(manager.as_ref()?, k)?;
                let check = scene.nodes[t].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle").and_then(|c| c.fields.pptr("graphic")).and_then(|p| crate::charaselect::node_of(scene, p.path_id));
                Some((t, check, *zone))
            })
            .collect();
        let back = scene.nodes.iter().position(|n| n.components.iter().any(|c| c.script == "AL.Memory.BackTitle"));
        let card_image = level.find("Canvas/SelectChara/BackgroundImage/charaFrame/charaCard");
        let template = scene.nodes.iter().position(|n| n.components.iter().any(|c| c.script == "AL.Memory.MemoryItem"));
        let mut grid = template.and_then(|t| Grid::from_template(level, t));
        let mut frames = Vec::new();
        if let Some(g) = grid.as_mut() {
            for (i, e) in entries.iter().enumerate() {
                let node = g.add(level, &i.to_string(), None);
                if let Some(t) = named_child(level, node, "MemoryThumbnail") {
                    let st = level.states.entry(t).or_default();
                    st.image = e.image.clone();
                    // The locked material: the thumbnail greyed out.
                    if !e.unlocked {
                        st.color = Some([0.35, 0.35, 0.35, 1.]);
                    }
                }
                let selected = named_child(level, node, "MemorySelectedFrame");
                let selectable = named_child(level, node, "MemorySelectableFrame");
                for n in [selected, named_child(level, node, "MemoryOnPointFrame")].into_iter().flatten() {
                    level.set_active(n, false);
                }
                if let Some(s) = selectable {
                    level.set_active(s, e.unlocked);
                }
                frames.push((selected, selectable));
            }
        }
        let mut cards_grid = Grid::find(level, "AL.EntryScene.EntryFileListCtrl", None);
        if let Some(g) = cards_grid.as_mut() {
            for (card, img) in cards {
                g.add(level, card, img.clone());
            }
        }
        let mut m = Memory { entries, grid, selected: None, frames, title, toggles, zone: 1, play, back, cards: cards_grid, card: None, card_image, card_images: cards.iter().map(|c| c.1.clone()).collect() };
        m.set_zone(level, 1);
        m.set_card(level, (!cards.is_empty()).then_some(0));
        m.play_active(level);
        if let Some(t) = m.title {
            level.states.entry(t).or_default().text = Some(String::new());
        }
        m
    }

    pub fn describe(&self) -> String {
        let grid = self.grid.as_ref().map(|g| g.describe()).unwrap_or_else(|| "no grid".into());
        format!("{} memories ({} unlocked, {} thumbnails; {grid}), {} cards", self.entries.len(), self.entries.iter().filter(|e| e.unlocked).count(), self.entries.iter().filter(|e| e.image.is_some()).count(), self.card_images.len())
    }

    /// `PlayActive`: the play button takes clicks with a memory and a
    /// card selected.
    fn play_active(&self, level: &mut Level) {
        if let Some(p) = self.play {
            level.set_interactable(p, self.selected.is_some() && self.card.is_some());
        }
    }

    fn set_zone(&mut self, level: &mut Level, zone: i32) {
        self.zone = zone;
        for (_, check, z) in &self.toggles {
            if let Some(c) = check {
                level.set_active(*c, *z == zone);
            }
        }
    }

    fn set_card(&mut self, level: &mut Level, card: Option<usize>) {
        self.card = card;
        if let Some(g) = &self.cards {
            g.highlight(level, card);
        }
        if let (Some(n), Some(i)) = (self.card_image, card) {
            level.states.entry(n).or_default().image = self.card_images.get(i).cloned().flatten();
        }
    }

    /// A click on `node` of the scene.
    pub fn click(&mut self, level: &mut Level, node: usize) -> Outcome {
        let within = |level: &Level, target: Option<usize>| {
            let mut n = Some(node);
            while let Some(x) = n {
                if Some(x) == target {
                    return true;
                }
                n = level.scene.nodes[x].parent;
            }
            false
        };
        if within(level, self.back) {
            return Outcome::Back;
        }
        if within(level, self.play) {
            let (Some(i), Some(c)) = (self.selected, self.card) else { return Outcome::None };
            let card = self.cards.as_ref().and_then(|g| g.items.get(c)).map(|(_, card)| card.clone()).unwrap_or_default();
            return Outcome::Play { movie: self.entries[i].thumbnail.movie, zone: self.zone, card };
        }
        if let Some((_, _, zone)) = self.toggles.iter().find(|(t, _, _)| within(level, Some(*t))).copied() {
            self.set_zone(level, zone);
            return Outcome::None;
        }
        if let Some(i) = self.grid.as_ref().and_then(|g| g.item_of(level, node)) {
            // `ClickMemory`: locked memories ignore the click; a second
            // click on the selection clears it.
            if !self.entries[i].unlocked {
                return Outcome::None;
            }
            self.selected = if self.selected == Some(i) { None } else { Some(i) };
            for (j, (sel, _)) in self.frames.iter().enumerate() {
                if let Some(s) = sel {
                    level.set_active(*s, Some(j) == self.selected);
                }
            }
            if let Some(t) = self.title {
                level.states.entry(t).or_default().text = Some(self.selected.map(|i| self.entries[i].title.clone()).unwrap_or_default());
            }
            self.play_active(level);
            return Outcome::None;
        }
        if let Some(i) = self.cards.as_ref().and_then(|g| g.item_of(level, node)) {
            self.set_card(level, Some(i));
            self.play_active(level);
        }
        Outcome::None
    }
}

/// The ObjectChangeCtrl lists of the card list start on their first
/// object (as in `CharaSelectUI`).
pub fn reset_object_ctrls(level: &mut Level) {
    let ctrls: Vec<Vec<usize>> = level
        .scene
        .nodes
        .iter()
        .flat_map(|n| &n.components)
        .filter(|c| c.script == "AL.ObjectChangeCtrl")
        .map(|c| c.fields.array("_objects").unwrap_or(&[]).iter().filter_map(|v| if let Value::PPtr(p) = v { crate::charaselect::node_of(&level.scene, p.path_id) } else { None }).collect())
        .collect();
    for objs in ctrls {
        for (i, o) in objs.iter().enumerate() {
            level.set_active(*o, i == 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_list() {
        // Two entries as in `memory/list/000_00` `Memory_Thumbnail`.
        let mut b = vec![1, 2, 0, 0, 0];
        for (key, id, movie) in [(1, 1, 0), (2, 2, 1)] {
            b.extend((key as i32).to_le_bytes());
            b.push(4);
            b.extend((id as i32).to_le_bytes());
            b.extend((movie as i32).to_le_bytes());
            for s in ["memory/thumbnail/000_00.unity3d", "sp_al_memories_15_000"] {
                b.extend((!(s.len() as i32)).to_le_bytes());
                b.extend((s.len() as i32).to_le_bytes());
                b.extend(s.as_bytes());
            }
        }
        let t = parse_thumbnails(&b).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[1].id, t[1].movie, t[1].asset.as_str()), (2, 1, "sp_al_memories_15_000"));
    }
}
