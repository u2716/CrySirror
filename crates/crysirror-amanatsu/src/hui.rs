// SPDX-License-Identifier: MPL-2.0
//! The HScene posture selection (`PostureSelect`): `AL.H.PostureMainSelecter`
//! (a category button cloned from `_prefab` per category with usable
//! postures, sprites from `_viewDatas[category]._sprites` (normal,
//! selected), `_buttonHide`/`_buttonVisible` folding `_hideTarget`) and
//! `AL.H.PostureSubSelecter` (the postures of the chosen category cloned
//! from `_clone`, `_select` marking the current posture, `_textName` and
//! the `_selectPosture` icon naming the category).
//!
//! SATD-amanatsu-port: the sub list scroll (`LoopListView2`, scrollbar),
//! the selection animations and the ViewData `_spriteStates` swaps are
//! not ported.
use crate::{h::Posture, level::Level};
use crysirror_unity::{managed::Value, scene6::Scene6, PPtr};
use std::collections::HashMap;

pub const ROOT: &str = "HScene/Canvas/ccr_dropShadow/PostureSelect";

fn node_of(scene: &Scene6, path_id: i64) -> Option<usize> {
    scene.by_game_object.get(&path_id).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
}

fn component<'a>(scene: &'a Scene6, script: &str) -> Option<(usize, &'a crysirror_unity::scene6::Component)> {
    scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.script == script).map(|c| (i, c)))
}

/// Clone `template` under `parent`; template node → clone node.
fn clone_map(scene: &mut Scene6, template: usize, parent: usize) -> HashMap<usize, usize> {
    let mut order = Vec::new();
    let mut stack = vec![template];
    while let Some(n) = stack.pop() {
        order.push(n);
        stack.extend(scene.nodes[n].children.iter().rev());
    }
    let base = scene.nodes.len();
    scene.clone_subtree(template, parent);
    order.into_iter().enumerate().map(|(i, n)| (n, base + i)).collect()
}

/// What a click asks of the H scene.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    /// `ChangePosture` to the posture (file, id).
    Change(String, i32),
}

pub struct PostureUi {
    /// Usable postures by category (file, id, name).
    lists: Vec<Vec<(String, i32, String)>>,
    category: usize,
    current: (String, i32),
    /// Category → (button node, image node).
    main: Vec<(usize, usize, usize)>,
    sprites: Vec<[Option<PPtr>; 2]>,
    labels: Vec<String>,
    hide: Option<usize>,
    show: Option<usize>,
    hide_target: Option<usize>,
    sub_template: Option<usize>,
    sub_content: Option<usize>,
    /// Template nodes of the sub item: text, button, select mark.
    sub_parts: [Option<usize>; 3],
    sub_items: Vec<(usize, usize, Option<usize>, (String, i32))>,
    name_text: Option<usize>,
    icon: Option<(usize, Vec<Value>)>,
    /// `LoopListView2` (`mArrangeType` TopToBottom): item padding.
    sub_padding: f32,
}

impl PostureUi {
    /// `PostureMainSelecter.Initialize` + `SelectionPosture(current)`.
    pub fn new(level: &mut Level, postures: &[&Posture], current: &Posture) -> Option<Self> {
        let (_, main) = component(&level.scene, "AL.H.PostureMainSelecter")?;
        let mf = main.fields.clone();
        let (_, sub) = component(&level.scene, "AL.H.PostureSubSelecter")?;
        let sf = sub.fields.clone();
        let pnode = |level: &Level, f: &crysirror_unity::managed::Fields, k: &str| f.pptr(k).and_then(|p| node_of(&level.scene, p.path_id));
        let item_root = pnode(level, &mf, "_itemRoot")?;
        let prefab = pnode(level, &mf, "_prefab")?;
        let mut sprites = Vec::new();
        let mut labels = Vec::new();
        for v in mf.array("_viewDatas").unwrap_or(&[]) {
            let Value::Struct(d) = v else { continue };
            let s = d.array("_sprites").unwrap_or(&[]);
            let sp = |i: usize| if let Some(Value::PPtr(p)) = s.get(i) { Some(*p) } else { None };
            sprites.push([sp(0), sp(1)]);
            labels.push(d.string("_text").unwrap_or("").to_owned());
        }
        let mut lists: Vec<Vec<(String, i32, String)>> = vec![Vec::new(); labels.len().max(7)];
        for p in postures {
            if let Some(l) = lists.get_mut(p.mode.max(0) as usize) {
                l.push((p.file.clone(), p.id, p.name.clone()));
            }
        }
        // One button per category with postures, before the Close item.
        let close_item = level.scene.nodes[item_root].children.iter().copied().find(|&c| c != prefab && level.scene.nodes[c].name == "Close");
        let mut main_items = Vec::new();
        for (c, l) in lists.iter().enumerate() {
            if l.is_empty() {
                continue;
            }
            let map = clone_map(&mut level.scene, prefab, item_root);
            let root = map[&prefab];
            let button = level.scene.nodes[root].children.iter().copied().find(|&k| level.scene.nodes[k].name == "Button").unwrap_or(root);
            level.set_active(root, true);
            main_items.push((c, button, button));
        }
        if let Some(close) = close_item {
            let kids = &mut level.scene.nodes[item_root].children;
            kids.retain(|&k| k != close);
            kids.push(close);
        }
        level.set_active(prefab, false);
        let sub_template = pnode(level, &sf, "_clone");
        let sub_content = pnode(level, &sf, "_content");
        let sub_parts = match sub_template.and_then(|t| level.scene.nodes[t].components.iter().find(|c| c.script == "AL.H.PostureSubSelectComponent").cloned()) {
            Some(c) => ["_text", "_button", "_select"].map(|k| c.fields.pptr(k).and_then(|p| node_of(&level.scene, p.path_id))),
            None => [None; 3],
        };
        if let Some(t) = sub_template {
            level.set_active(t, false);
        }
        let sub_padding = sub_template
            .and_then(|t| {
                level.scene.nodes.iter().flat_map(|n| &n.components).filter(|c| c.script.ends_with("LoopListView2")).find_map(|c| {
                    c.fields.array("mItemPrefabDataList")?.iter().find_map(|e| match e {
                        Value::Struct(d) if d.pptr("mItemPrefab").and_then(|p| node_of(&level.scene, p.path_id)) == Some(t) => d.float("mPadding"),
                        _ => None,
                    })
                })
            })
            .unwrap_or(0.) as f32;
        let icon = sf.fields("_selectPosture").and_then(|s| Some((s.pptr("_image").and_then(|p| node_of(&level.scene, p.path_id))?, s.array("_sprites")?.to_vec())));
        let mut ui = Self {
            lists,
            category: current.mode.max(0) as usize,
            current: (current.file.clone(), current.id),
            main: main_items,
            sprites,
            labels,
            hide: pnode(level, &mf, "_buttonHide"),
            show: pnode(level, &mf, "_buttonVisible"),
            hide_target: pnode(level, &mf, "_hideTarget"),
            sub_template,
            sub_content,
            sub_parts,
            sub_items: Vec::new(),
            name_text: pnode(level, &sf, "_textName"),
            icon,
            sub_padding,
        };
        if let Some(n) = ui.show {
            level.set_active(n, false);
        }
        ui.select_category(level, ui.category);
        Some(ui)
    }

    fn refresh_main(&self, level: &mut Level) {
        for &(c, _, image) in &self.main {
            if let Some(Some(p)) = self.sprites.get(c).map(|s| s[usize::from(c == self.category)]) {
                level.states.entry(image).or_default().sprite = Some(p);
            }
        }
    }

    /// `PostureSubSelecter.InitializeScrollSelection` for a category.
    fn select_category(&mut self, level: &mut Level, category: usize) {
        self.category = category;
        self.refresh_main(level);
        for (root, ..) in self.sub_items.drain(..) {
            level.set_active(root, false);
        }
        if let Some(n) = self.name_text {
            level.states.entry(n).or_default().text = Some(self.labels.get(category).cloned().unwrap_or_default());
        }
        if let Some((image, sprites)) = &self.icon {
            if let Some(Value::PPtr(p)) = sprites.get(category) {
                level.states.entry(*image).or_default().sprite = Some(*p);
            }
        }
        let (Some(template), Some(content)) = (self.sub_template, self.sub_content) else { return };
        let rect = level.scene.nodes[template].xform.rect;
        for (i, (file, id, name)) in self.lists.get(category).cloned().unwrap_or_default().into_iter().enumerate() {
            let map = clone_map(&mut level.scene, template, content);
            let m = |n: Option<usize>| n.map(|n| map.get(&n).copied().unwrap_or(n));
            let root = map[&template];
            level.set_active(root, true);
            // `LoopListView2` TopToBottom: one item height plus padding each.
            if let Some(r) = rect {
                let y = r.anchored_position[1] - i as f32 * (r.size_delta[1] + self.sub_padding);
                level.states.entry(root).or_default().anchored_position = Some([r.anchored_position[0], y]);
            }
            if let Some(t) = m(self.sub_parts[0]) {
                level.states.entry(t).or_default().text = Some(name.clone());
            }
            let select = m(self.sub_parts[2]);
            if let Some(s) = select {
                level.set_active(s, (file.as_str(), id) == (self.current.0.as_str(), self.current.1));
            }
            self.sub_items.push((root, m(self.sub_parts[1]).unwrap_or(root), select, (file, id)));
        }
    }

    /// The posture changed (marks the new current one).
    pub fn set_current(&mut self, level: &mut Level, file: &str, id: i32) {
        self.current = (file.to_owned(), id);
        for (_, _, select, key) in &self.sub_items {
            if let Some(s) = select {
                level.set_active(*s, (key.0.as_str(), key.1) == (file, id));
            }
        }
    }

    pub fn click(&mut self, level: &mut Level, node: usize) -> Action {
        if let Some(&(c, ..)) = self.main.iter().find(|m| m.1 == node) {
            self.select_category(level, c);
            return Action::None;
        }
        if let Some((_, _, _, key)) = self.sub_items.iter().find(|s| s.1 == node) {
            return Action::Change(key.0.clone(), key.1);
        }
        let fold = |level: &mut Level, hidden: bool, hide: Option<usize>, show: Option<usize>, target: Option<usize>| {
            if let Some(t) = target {
                level.set_active(t, !hidden);
            }
            if let Some(s) = show {
                level.set_active(s, hidden);
            }
            let _ = hide;
        };
        if Some(node) == self.hide {
            fold(level, true, self.hide, self.show, self.hide_target);
        } else if Some(node) == self.show {
            fold(level, false, self.hide, self.show, self.hide_target);
        }
        Action::None
    }
}

impl PostureUi {
    /// Hide the cloned buttons and items (the lists are rebuilt for a new
    /// place).
    pub fn dispose(self, level: &mut Level) {
        for (_, button, _) in self.main {
            if let Some(root) = level.scene.nodes[button].parent {
                level.set_active(root, false);
            }
        }
        for (root, ..) in self.sub_items {
            level.set_active(root, false);
        }
    }
}

/// What a click in the point mode asks.
#[derive(Clone, Debug, PartialEq)]
pub enum PointAction {
    None,
    Back,
    /// Move to the H point (index).
    Move(usize),
}

/// `AL.H.UI.PointSetting` (`Canvas Point`): the main canvas gives way to
/// the point canvas and each other H point gets a marker.
/// SATD-amanatsu-port: the HPoint particle markers and colliders are not
/// rendered; each point is marked by a clone of its place type's filter
/// button at the point's screen position, and the filters do not filter.
pub struct PointUi {
    markers: Vec<(usize, usize)>,
    back: Option<usize>,
}

/// `Filter` buttons by `HPoint._placeType`.
const FILTERS: [&str; 7] = ["Button Floor", "Button Chair", "Button Desk", "Button BeachChair", "Button Poolside", "Button Pool", "Button Sea"];

impl PointUi {
    /// `points`: (index, place type, screen position) of the points to mark.
    pub fn open(level: &mut Level, points: &[(usize, i32, [f32; 2])], height: f32) -> Option<Self> {
        let canvas = level.find("HScene/Canvas Point")?;
        let filter = level.find("HScene/Canvas Point/ccr_dropShadow/Filter")?;
        // `FadeCanvas` open.
        crate::talk::show(level, canvas);
        if let Some(main) = level.find("HScene/Canvas") {
            level.set_active(main, false);
        }
        let mut markers = Vec::new();
        for &(index, place, [x, y]) in points {
            let Some(name) = FILTERS.get(place.max(0) as usize) else { continue };
            let Some(template) = level.scene.nodes[filter].children.iter().copied().find(|&c| level.scene.nodes[c].name == *name) else { continue };
            let node = level.scene.clone_subtree(template, canvas);
            level.set_active(node, true);
            let st = level.states.entry(node).or_default();
            st.anchor_min = Some([0., 0.]);
            st.anchor_max = Some([0., 0.]);
            st.anchored_position = Some([x, height - y]);
            markers.push((node, index));
        }
        Some(Self { markers, back: level.find("HScene/Canvas Point/ccr_dropShadow/Back") })
    }

    pub fn click(&self, node: usize) -> PointAction {
        if Some(node) == self.back {
            return PointAction::Back;
        }
        self.markers.iter().find(|m| m.0 == node).map_or(PointAction::None, |m| PointAction::Move(m.1))
    }

    pub fn close(self, level: &mut Level) {
        for (node, _) in self.markers {
            level.set_active(node, false);
        }
        if let Some(c) = level.find("HScene/Canvas Point") {
            level.set_active(c, false);
        }
        if let Some(main) = level.find("HScene/Canvas") {
            level.set_active(main, true);
        }
    }
}
