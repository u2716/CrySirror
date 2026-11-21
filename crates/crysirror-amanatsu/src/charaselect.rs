// SPDX-License-Identifier: MPL-2.0
//! `AL.UI.CharaSelect.CharaSelectUI` (Add Heroine): the heroine cards of the
//! installation in the `EntryFileListCtrl` grid (`LoopGridView`: fixed
//! columns, item size and padding; items cloned from `_clone`), selection
//! through the item toggles, 登録 (`_enters`) registering the selected card
//! in the left `MainSelectChara` list, 閉じる closing the UI.
//!
//! SATD-amanatsu-port: the 自作/他作 sources (`UserData/chara`), sorting,
//! pair editing, release/delete dialogs, the personality text and the
//! selected heroine's parameter panel are not ported.
use crate::level::Level;
use crysirror_unity::{managed::Value, scene6::Scene6};

fn node_of(scene: &Scene6, path_id: i64) -> Option<usize> {
    scene.by_game_object.get(&path_id).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
}

/// Path of `node` relative to `root`.
fn relative(scene: &Scene6, root: usize, mut node: usize) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    while node != root {
        parts.push(scene.nodes[node].name.clone());
        node = scene.nodes[node].parent?;
    }
    parts.reverse();
    Some(parts)
}

fn descend(scene: &Scene6, mut node: usize, path: &[String]) -> Option<usize> {
    for p in path {
        node = *scene.nodes[node].children.iter().find(|&&c| &scene.nodes[c].name == p)?;
    }
    Some(node)
}

/// One grid: the item template and where clones go.
struct Grid {
    template: usize,
    content: usize,
    columns: usize,
    size: [f32; 2],
    padding: [f32; 2],
    /// Relative paths of the item's raw image, highlight and toggle.
    raw: Option<Vec<String>>,
    highlight: Option<Vec<String>>,
    entry_icon: Option<Vec<String>>,
    /// The item toggle's `graphic` (checkmark, shown while on).
    check: Option<Vec<String>>,
    /// Rows the viewport shows (SATD-amanatsu-port: no scrolling).
    rows: usize,
    items: Vec<(usize, String)>,
}

impl Grid {
    fn find(level: &Level, ctrl_class: &str, list_ctrl: Option<i64>) -> Option<Self> {
        let scene = &level.scene;
        let ctrl = scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == ctrl_class && list_ctrl.is_none_or(|id| c.path_id == id))?;
        let template = ctrl.fields.pptr("_clone").and_then(|p| node_of(scene, p.path_id))?;
        let content = scene.nodes[template].parent?;
        let grid = scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script.ends_with("LoopGridView") && c.fields.array("mItemPrefabDataList").is_some_and(|l| l.iter().any(|e| matches!(e, Value::Struct(s) if s.pptr("mItemPrefab").and_then(|p| node_of(scene, p.path_id)) == Some(template)))));
        let v2 = |name: &str, d: [f32; 2]| grid.and_then(|g| g.fields.fields(name)).map_or(d, |s| [s.float("x").unwrap_or(d[0] as f64) as f32, s.float("y").unwrap_or(d[1] as f64) as f32]);
        let info = scene.nodes[template].components.iter().find(|c| c.script.ends_with("EntryFileInfoComponent"));
        let rel = |field: &str| info.and_then(|i| i.fields.pptr(field)).and_then(|p| node_of(scene, p.path_id)).and_then(|n| relative(scene, template, n));
        let toggle = info.and_then(|i| i.fields.pptr("_toggle")).and_then(|p| node_of(scene, p.path_id));
        let check = toggle
            .and_then(|t| scene.nodes[t].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle"))
            .and_then(|c| c.fields.pptr("graphic"))
            .and_then(|p| node_of(scene, p.path_id))
            .and_then(|n| relative(scene, template, n));
        // The viewport (content's parent) height in canvas units.
        let viewport = {
            // The nearest sized ancestor (viewports stretch in their scroll view).
            let mut n = scene.nodes[content].parent;
            let mut h = 0.;
            while let Some(v) = n {
                if let Some(r) = scene.nodes[v].xform.rect.filter(|r| r.size_delta[1] > 1. && (r.anchor_max[1] - r.anchor_min[1]).abs() < 1e-3) {
                    h = r.size_delta[1];
                    break;
                }
                n = scene.nodes[v].parent;
            }
            h
        };
        let mut g = Self {
            template,
            content,
            columns: grid.and_then(|g| g.fields.int("mFixedRowOrColumnCount")).unwrap_or(4).max(1) as usize,
            size: v2("mItemSize", [101., 140.]),
            padding: v2("mItemPadding", [10., 10.]),
            raw: rel("_rawImage"),
            highlight: rel("_imgHighlight"),
            entry_icon: rel("_imgEntryIcon"),
            check,
            rows: 0,
            items: Vec::new(),
        };
        g.rows = usize::MAX;
        if viewport > 0. {
            g.rows = (((viewport + g.padding[1]) / (g.size[1] + g.padding[1])).floor() as usize).max(1);
        }
        Some(g)
    }

    /// Clone an item for `card` showing `image` (an uploaded asset).
    fn add(&mut self, level: &mut Level, card: &str, image: Option<String>) -> usize {
        let i = self.items.len();
        let node = level.scene.clone_subtree(self.template, self.content);
        level.set_active(node, true);
        let (col, row) = (i % self.columns, i / self.columns);
        let st = level.states.entry(node).or_default();
        // LoopGridView lays items from the content's top-left corner.
        st.anchored_position = Some([col as f32 * (self.size[0] + self.padding[0]) + self.size[0] / 2., -(row as f32 * (self.size[1] + self.padding[1])) - self.size[1] / 2.]);
        if let (Some(raw), Some(img)) = (&self.raw, image) {
            if let Some(r) = descend(&level.scene, node, raw) {
                level.states.entry(r).or_default().image = Some(img);
            }
        }
        for p in [&self.highlight, &self.entry_icon, &self.check].into_iter().flatten() {
            if let Some(h) = descend(&level.scene, node, p) {
                level.set_active(h, false);
            }
        }
        if row >= self.rows {
            level.set_active(node, false);
        }
        self.items.push((node, card.to_owned()));
        node
    }

    fn item_of(&self, level: &Level, mut node: usize) -> Option<usize> {
        loop {
            if let Some(i) = self.items.iter().position(|(n, _)| *n == node) {
                return Some(i);
            }
            node = level.scene.nodes[node].parent?;
        }
    }

    fn highlight(&self, level: &mut Level, selected: Option<usize>) {
        for (i, (n, _)) in self.items.iter().enumerate() {
            for p in [&self.highlight, &self.check].into_iter().flatten() {
                if let Some(h) = descend(&level.scene, *n, p) {
                    level.set_active(h, Some(i) == selected);
                }
            }
        }
    }
}

pub enum Outcome {
    None,
    Close,
}

/// The entered heroines' fixed slots (`MainListCardInfoComponent`,
/// `Card (1)`…): (slot node, raw card image node).
struct Slots {
    slots: Vec<(usize, Option<usize>)>,
    filled: usize,
}

impl Slots {
    fn find(level: &mut Level) -> Self {
        let scene = &level.scene;
        let mut slots: Vec<(u32, usize, Option<usize>, Vec<usize>)> = Vec::new();
        for (i, n) in scene.nodes.iter().enumerate() {
            let Some(c) = n.components.iter().find(|c| c.script.ends_with("MainListCardInfoComponent")) else { continue };
            let num = n.name.trim_start_matches("Card").trim().trim_matches(|c| c == '(' || c == ')').parse().unwrap_or(0);
            let raw = c.fields.pptr("_rawCard").and_then(|p| node_of(scene, p.path_id));
            let hide = ["_objPairRoot", "_objCancelRoot"].iter().filter_map(|f| c.fields.pptr(f).and_then(|p| node_of(scene, p.path_id))).collect();
            slots.push((num, i, raw, hide));
        }
        slots.sort_by_key(|s| s.0);
        for (_, _, raw, hide) in &slots {
            for h in hide {
                level.set_active(*h, false);
            }
            if let Some(r) = raw {
                level.set_active(*r, false);
            }
        }
        Self { slots: slots.into_iter().map(|s| (s.1, s.2)).collect(), filled: 0 }
    }

    fn add(&mut self, level: &mut Level, image: Option<String>) {
        let Some(&(_, raw)) = self.slots.get(self.filled) else { return };
        self.filled += 1;
        if let (Some(r), Some(img)) = (raw, image) {
            level.set_active(r, true);
            level.states.entry(r).or_default().image = Some(img);
        }
    }
}

pub struct CharaSelect {
    cards: Grid,
    entered: Slots,
    selected: Option<usize>,
    enter: Option<usize>,
    close: Option<usize>,
}

impl CharaSelect {
    /// `cards`: (card path, uploaded thumbnail) of the selectable cards;
    /// `registered`: the heroines already entered.
    pub fn new(level: &mut Level, cards: &[(String, Option<String>)], registered: &[(String, Option<String>)]) -> Option<Self> {
        let main = level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script.ends_with("CharaSelectMainView")).map(|c| c.fields.clone())?;
        let selecter = main.pptr("_charaSelecter").and_then(|p| level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == p.path_id)).map(|c| c.fields.clone())?;
        let list_ctrl = selecter.pptr("_listCtrl").map(|p| p.path_id);
        let mut grid = Grid::find(level, "AL.EntryScene.EntryFileListCtrl", list_ctrl)?;
        let enter = selecter.array("_enters").and_then(|a| a.first()).and_then(|v| if let Value::PPtr(p) = v { node_of(&level.scene, p.path_id) } else { None });
        let close = main.pptr("_btnClose").and_then(|p| node_of(&level.scene, p.path_id));
        // Initial view: dialogs closed, each `ObjectChangeCtrl` on its
        // first object.
        let ctrls: Vec<Vec<usize>> = level
            .scene
            .nodes
            .iter()
            .flat_map(|n| &n.components)
            .filter(|c| c.script == "AL.ObjectChangeCtrl")
            .map(|c| c.fields.array("_objects").unwrap_or(&[]).iter().filter_map(|v| if let Value::PPtr(p) = v { node_of(&level.scene, p.path_id) } else { None }).collect())
            .collect();
        for objs in ctrls {
            for (i, o) in objs.iter().enumerate() {
                level.set_active(*o, i == 0);
            }
        }
        for name in ["AllDelete", "Delete", "Pair"] {
            for n in (0..level.scene.nodes.len()).filter(|&n| level.scene.nodes[n].name == name && level.scene.nodes[n].components.iter().any(|c| c.script.ends_with("CharaSelectDialog"))).collect::<Vec<_>>() {
                level.set_active(n, false);
            }
        }
        for (card, img) in cards {
            grid.add(level, card, img.clone());
        }
        let mut entered = Slots::find(level);
        for (_, img) in registered {
            entered.add(level, img.clone());
        }
        Some(Self { cards: grid, entered, selected: None, enter, close })
    }

    /// The lists found (diagnostics).
    pub fn describe(&self, level: &Level) -> String {
        let g = |g: &Grid| format!("{} → {} ({} cols, {} rows)", level.scene.path(g.template), level.scene.path(g.content), g.columns, g.rows);
        format!("cards {}; {} entry slots", g(&self.cards), self.entered.slots.len())
    }

    /// A click inside the UI; `register` receives the entered card.
    pub fn click(&mut self, level: &mut Level, node: usize, register: &mut dyn FnMut(&str) -> Option<String>) -> Outcome {
        if Some(node) == self.close {
            return Outcome::Close;
        }
        if Some(node) == self.enter {
            if let Some(i) = self.selected.take() {
                let card = self.cards.items[i].1.clone();
                if let Some(img) = register(&card) {
                    self.entered.add(level, Some(img));
                }
                self.cards.highlight(level, None);
            }
            return Outcome::None;
        }
        if let Some(i) = self.cards.item_of(level, node) {
            self.selected = if self.selected == Some(i) { None } else { Some(i) };
            self.cards.highlight(level, self.selected);
        }
        Outcome::None
    }
}
