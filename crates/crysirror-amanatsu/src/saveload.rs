// SPDX-License-Identifier: MPL-2.0
//! `AL.UI.SaveLoad.SaveLoadUI` (`am/pre/u` `SaveLoadUI`): `_tabNum` tabs of
//! `_dataNum` slots cloned from `_objTabNode`/`_objDataNode`, the auto slot
//! (`_autoDataComp`), slot selection through the slot toggles, the action
//! bubble of the selected slot (`SaveDataComponent.StartBubble` by
//! `GetStartBubbleKind`) and the Load/New/Override/Delete dialogs
//! (`SaveLoadDialog`, shown through the `Dialog` `ObjectChangeCtrl`).
//!
//! Slot files (`SaveInfo.GetFileName`): id < 100 is `{id:000}.sav`, the
//! auto slot is `a_000.sav`.
//!
//! SATD-amanatsu-port: the comment input field (`_inputNewComment`) keeps
//! the previous comment or stays empty, the first part of the slot time
//! text (`"{0}  {1}"`), tab/slot/bubble/dialog animations and the
//! `SaveLoadNewAnimation` are not ported.
use crate::level::Level;
use crysirror_unity::{managed::Value, scene6::Scene6};
use std::collections::HashMap;

pub const PREFAB: &str = "SaveLoadUI";
/// `SaveInfo.ID` of the auto slot.
pub const AUTO: usize = 100;

fn node_of(scene: &Scene6, path_id: i64) -> Option<usize> {
    scene.by_game_object.get(&path_id).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
}

/// `SaveInfo.GetFileName`.
pub fn file_name(id: usize) -> String {
    if id >= AUTO {
        crate::save::file_name("000", true)
    } else {
        crate::save::file_name(&format!("{id:03}"), false)
    }
}

/// What a slot shows of its `SaveData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlotData {
    pub time_zone: i32,
    pub time: String,
    pub comment: String,
}

/// A request to the session.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    Close,
    Load(usize),
    /// `Save` into a slot with a comment.
    Save(usize, String),
    Delete(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Dialog {
    Load = 0,
    New = 1,
    Override = 2,
    Delete = 3,
}

/// One `SaveDataComponent` (resolved onto its clone).
struct Slot {
    /// `_buttonObjChange._objects`: NoSaveData, EmptyData, SaveData,
    /// TitleNoSaveData.
    states: Vec<usize>,
    toggles: Vec<usize>,
    checks: Vec<usize>,
    time: Option<usize>,
    comment: Option<usize>,
    zone_mark: Option<(usize, Vec<Value>)>,
    new_mark: Option<usize>,
    bubble: Option<usize>,
    /// `_objNew`, `_objLoad`, `_objOverride`, `_objDelete`.
    objects: [Option<usize>; 4],
    buttons: [Option<usize>; 4],
}

struct DialogNodes {
    root: usize,
    yes: Option<usize>,
    no: Option<usize>,
    close: Option<usize>,
    new_time: Option<usize>,
    new_comment: Option<usize>,
    old_time: Option<usize>,
    old_comment: Option<usize>,
    /// `_isNewTimeZone`/`_isOldTimeZone` (`ImageSprite`: image, sprites).
    new_zone: Option<(usize, Vec<Value>)>,
    old_zone: Option<(usize, Vec<Value>)>,
}

/// An `ImageSprite` component by its path id: image node and sprites.
fn image_sprite(scene: &Scene6, path_id: i64) -> Option<(usize, Vec<Value>)> {
    let c = scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == path_id)?;
    Some((c.fields.pptr("_image").and_then(|i| node_of(scene, i.path_id))?, c.fields.array("_sprites")?.to_vec()))
}

fn set_zone(level: &mut Level, mark: &Option<(usize, Vec<Value>)>, zone: i32) {
    if let Some((image, sprites)) = mark {
        if let Some(Value::PPtr(p)) = sprites.get(zone.clamp(0, 3) as usize) {
            level.states.entry(*image).or_default().sprite = Some(*p);
        }
    }
}

pub struct SaveLoad {
    pub is_title: bool,
    pub tab: usize,
    pub selected: Option<usize>,
    data_num: usize,
    slots: Vec<Slot>,
    auto: Option<Slot>,
    tabs: Vec<(usize, Option<usize>)>,
    close: Option<usize>,
    dialog_root: Option<usize>,
    dialogs: Vec<DialogNodes>,
    dialog: Option<(Dialog, usize)>,
    pub infos: HashMap<usize, SlotData>,
    /// The newest slot (`GetLastData`: `IsNew`).
    newest: Option<usize>,
    /// The time text and zone of a new save (`_txtNewTime`).
    pub new_time: String,
    pub new_zone: i32,
}

/// Clone `template` under `parent`; template node → clone node.
fn clone_map(scene: &mut Scene6, template: usize, parent: usize) -> HashMap<usize, usize> {
    // `Scene6::clone_subtree` appends the subtree in this order.
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

fn slot(level: &Level, comp: &crysirror_unity::scene6::Component, map: &dyn Fn(usize) -> usize) -> Slot {
    let scene = &level.scene;
    let f = &comp.fields;
    let node = |k: &str| f.pptr(k).and_then(|p| node_of(scene, p.path_id)).map(map);
    let states = f
        .pptr("_buttonObjChange")
        .and_then(|p| scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == p.path_id))
        .and_then(|c| c.fields.array("_objects"))
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| if let Value::PPtr(p) = v { node_of(scene, p.path_id).map(map) } else { None })
        .collect();
    let toggle_check = |k: &str| -> Option<(usize, Option<usize>)> {
        let p = f.pptr(k)?;
        let t = node_of(scene, p.path_id)?;
        let check = scene.nodes[t].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle").and_then(|c| c.fields.pptr("graphic")).and_then(|g| node_of(scene, g.path_id)).map(map);
        Some((map(t), check))
    };
    let (mut toggles, mut checks) = (Vec::new(), Vec::new());
    for k in ["_tglSaveData", "_tglEmptyData"] {
        if let Some((t, c)) = toggle_check(k) {
            toggles.push(t);
            checks.extend(c);
        }
    }
    let zone_mark = f.pptr("_isSaveData").and_then(|p| scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == p.path_id)).and_then(|c| {
        let image = c.fields.pptr("_image").and_then(|i| node_of(scene, i.path_id)).map(map)?;
        Some((image, c.fields.array("_sprites")?.to_vec()))
    });
    let objects = ["_objNew", "_objLoad", "_objOverride", "_objDelete"].map(node);
    let buttons = ["_btnNew", "_btnLoad", "_btnOverride", "_btnDelete"].map(node);
    let bubble = objects[0].and_then(|o| scene.nodes[o].parent);
    Slot { states, toggles, checks, time: node("_txtTime"), comment: node("_txtComment"), zone_mark, new_mark: node("_objNewMark"), bubble, objects, buttons }
}

impl SaveLoad {
    /// `SaveLoadUI.Initialize` + `OpenExe(OpenParameter { IsTitle })`.
    pub fn new(level: &mut Level, is_title: bool) -> Option<Self> {
        let (ui_node, ui) = level.scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.script == "AL.UI.SaveLoad.SaveLoadUI").map(|c| (i, c.clone())))?;
        let _ = ui_node;
        let f = ui.fields.clone();
        let pnode = |level: &Level, k: &str| f.pptr(k).and_then(|p| node_of(&level.scene, p.path_id));
        let data_root = pnode(level, "_objDataRoot")?;
        let data_template = pnode(level, "_objDataNode")?;
        let tab_root = pnode(level, "_objTabRoot")?;
        let tab_template = pnode(level, "_objTabNode")?;
        let data_num = f.int("_dataNum").unwrap_or(10).clamp(1, 100) as usize;
        let tab_num = f.int("_tabNum").unwrap_or(10).clamp(1, 10) as usize;
        let data_comp = level.scene.nodes[data_template].components.iter().find(|c| c.script == "AL.UI.SaveLoad.SaveDataComponent")?.clone();
        let tab_comp = level.scene.nodes[tab_template].components.iter().find(|c| c.script == "AL.UI.SaveLoad.SaveDataTabComponent").cloned();
        let mut slots = Vec::new();
        for _ in 0..data_num {
            let map = clone_map(&mut level.scene, data_template, data_root);
            let s = slot(level, &data_comp, &|n| map.get(&n).copied().unwrap_or(n));
            level.set_active(map[&data_template], true);
            slots.push(s);
        }
        let mut tabs = Vec::new();
        for i in 0..tab_num {
            let map = clone_map(&mut level.scene, tab_template, tab_root);
            let m = |n: usize| map.get(&n).copied().unwrap_or(n);
            level.set_active(m(tab_template), true);
            let (mut toggle, mut texts, mut check) = (None, Vec::new(), None);
            if let Some(c) = &tab_comp {
                toggle = c.fields.pptr("_tglTag").and_then(|p| node_of(&level.scene, p.path_id)).map(m);
                check = toggle.and_then(|t| level.scene.nodes[t].components.iter().find(|c| c.script == "UnityEngine.UI.Toggle").and_then(|c| c.fields.pptr("graphic"))).and_then(|g| node_of(&level.scene, g.path_id)).map(m);
                texts = ["_textOn", "_textOff"].iter().filter_map(|k| c.fields.pptr(k).and_then(|p| node_of(&level.scene, p.path_id)).map(m)).collect();
            }
            // `SaveDataTabComponent.Initialize`: the tab number.
            for t in texts {
                level.states.entry(t).or_default().text = Some((i + 1).to_string());
            }
            tabs.push((toggle.unwrap_or(m(tab_template)), check));
        }
        level.set_active(data_template, false);
        level.set_active(tab_template, false);
        let auto = f.pptr("_autoDataComp").and_then(|p| level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == p.path_id).cloned()).map(|c| slot(level, &c, &|n| n));
        let mut dialogs = Vec::new();
        let mut dialog_root = None;
        for k in ["_dialogLoad", "_dialogNew", "_dialogOverride", "_dialogDelete"] {
            let Some(comp) = f.pptr(k).and_then(|p| level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.path_id == p.path_id).cloned()) else { continue };
            let root = level.scene.nodes.iter().position(|n| n.components.iter().any(|c| c.path_id == comp.path_id))?;
            dialog_root = level.scene.nodes[root].parent;
            let d = |k: &str| comp.fields.pptr(k).and_then(|p| node_of(&level.scene, p.path_id));
            let zone = |k: &str| comp.fields.pptr(k).and_then(|p| image_sprite(&level.scene, p.path_id));
            dialogs.push(DialogNodes { root, yes: d("_btnYes"), no: d("_btnNo"), close: d("_btnClose"), new_time: d("_txtNewTime"), new_comment: d("_inputNewComment"), old_time: d("_txtOldTime"), old_comment: d("_txtOldComment"), new_zone: zone("_isNewTimeZone"), old_zone: zone("_isOldTimeZone") });
        }
        if let Some(r) = dialog_root {
            level.set_active(r, false);
        }
        if let Some(n) = level.find("SaveLoadUI/Canvas/BackFrame/DontTouch") {
            level.set_active(n, false);
        }
        let close = pnode(level, "_btnClose");
        Some(Self { is_title, tab: 0, selected: None, data_num, slots, auto, tabs, close, dialog_root, dialogs, dialog: None, infos: HashMap::new(), newest: None, new_time: String::new(), new_zone: 0 })
    }

    /// New slot contents (`VisibleRefreshData` after reading the files).
    pub fn set_infos(&mut self, level: &mut Level, infos: HashMap<usize, SlotData>) {
        self.newest = infos.iter().filter(|(id, _)| **id < AUTO).max_by(|a, b| a.1.time.cmp(&b.1.time)).map(|(id, _)| *id);
        self.infos = infos;
        self.refresh(level);
    }

    fn show_slot(&self, level: &mut Level, slot: &Slot, id: usize, auto: bool) {
        let data = self.infos.get(&id);
        // `SaveDataComponent.Refresh`: data 2, auto empty 0, empty 1 (3 in
        // the title).
        let state = match (data.is_some(), auto, self.is_title) {
            (true, _, _) => 2,
            (false, true, _) => 0,
            (false, false, true) => 3,
            (false, false, false) => 1,
        };
        for (i, &n) in slot.states.iter().enumerate() {
            level.set_active(n, i == state);
        }
        let on = self.selected == Some(id);
        for &c in &slot.checks {
            level.set_active(c, on);
        }
        if let Some(n) = slot.time {
            level.states.entry(n).or_default().text = Some(data.map(|d| d.time.clone()).unwrap_or_default());
        }
        if let Some(n) = slot.comment {
            level.states.entry(n).or_default().text = Some(data.map(|d| d.comment.clone()).unwrap_or_default());
        }
        if let (Some((image, sprites)), Some(d)) = (&slot.zone_mark, data) {
            if let Some(Value::PPtr(p)) = sprites.get(d.time_zone.clamp(0, 3) as usize) {
                level.states.entry(*image).or_default().sprite = Some(*p);
            }
        }
        if let Some(n) = slot.new_mark {
            level.set_active(n, !auto && self.newest == Some(id));
        }
        // `StartBubble(GetStartBubbleKind)` on the selected slot: New on an
        // empty slot, Load/Override/Delete on data, Load/Delete in the
        // title, Load on the auto slot.
        let shown = match (on, data.is_some(), auto, self.is_title) {
            (false, ..) => None,
            (true, true, true, _) => Some([false, true, false, false]),
            (true, false, true, _) => None,
            (true, false, false, true) => None,
            (true, false, false, false) => Some([true, false, false, false]),
            (true, true, false, true) => Some([false, true, false, true]),
            (true, true, false, false) => Some([false, true, true, true]),
        };
        if let Some(b) = slot.bubble {
            level.set_active(b, shown.is_some());
        }
        for (i, o) in slot.objects.iter().enumerate() {
            if let Some(o) = o {
                level.set_active(*o, shown.is_some_and(|s| s[i]));
            }
        }
    }

    pub fn refresh(&mut self, level: &mut Level) {
        for (i, (_, check)) in self.tabs.iter().enumerate() {
            if let Some(c) = check {
                level.set_active(*c, i == self.tab);
            }
        }
        for (i, s) in self.slots.iter().enumerate() {
            self.show_slot(level, s, self.tab * self.data_num + i, false);
        }
        if let Some(a) = &self.auto {
            self.show_slot(level, a, AUTO, true);
        }
        if let Some(r) = self.dialog_root {
            level.set_active(r, self.dialog.is_some());
        }
        for (i, d) in self.dialogs.iter().enumerate() {
            level.set_active(d.root, self.dialog.is_some_and(|(k, _)| k as usize == i));
        }
    }

    fn open_dialog(&mut self, level: &mut Level, kind: Dialog, id: usize) {
        self.dialog = Some((kind, id));
        let data = self.infos.get(&id).cloned().unwrap_or_default();
        if let Some(d) = self.dialogs.get(kind as usize) {
            let mut text = |n: Option<usize>, s: &str| {
                if let Some(n) = n {
                    level.states.entry(n).or_default().text = Some(s.to_owned());
                }
            };
            text(d.old_time, &data.time);
            text(d.old_comment, &data.comment);
            text(d.new_time, &self.new_time);
            // SATD-amanatsu-port: no text entry; the previous comment.
            if let Some(input) = d.new_comment {
                let child = level.scene.nodes[input].components.iter().find(|c| c.script.ends_with("TMP_InputField")).and_then(|c| c.fields.pptr("m_TextComponent")).and_then(|p| node_of(&level.scene, p.path_id));
                text(child, &data.comment);
            }
            set_zone(level, &d.old_zone, data.time_zone);
            set_zone(level, &d.new_zone, self.new_zone);
        }
        self.refresh(level);
    }

    /// A click on a node of the UI.
    pub fn click(&mut self, level: &mut Level, node: usize) -> Action {
        let is = |n: Option<usize>| n == Some(node);
        if let Some((kind, id)) = self.dialog {
            let Some(d) = self.dialogs.get(kind as usize) else { return Action::None };
            if is(d.yes) {
                self.dialog = None;
                self.refresh(level);
                let comment = self.infos.get(&id).map(|d| d.comment.clone()).unwrap_or_default();
                return match kind {
                    Dialog::Load => Action::Load(id),
                    Dialog::New | Dialog::Override => Action::Save(id, comment),
                    Dialog::Delete => Action::Delete(id),
                };
            }
            if is(d.no) || is(d.close) {
                self.dialog = None;
                self.refresh(level);
            }
            return Action::None;
        }
        if is(self.close) {
            return Action::Close;
        }
        if let Some(i) = self.tabs.iter().position(|(t, _)| *t == node) {
            self.tab = i;
            self.selected = None;
            self.refresh(level);
            return Action::None;
        }
        let slots: Vec<(usize, &Slot)> = self.slots.iter().enumerate().map(|(i, s)| (self.tab * self.data_num + i, s)).chain(self.auto.iter().map(|s| (AUTO, s))).collect();
        for (id, s) in slots {
            if s.toggles.contains(&node) {
                self.selected = Some(id);
                self.refresh(level);
                return Action::None;
            }
            if let Some(b) = s.buttons.iter().position(|b| *b == Some(node)) {
                let kind = [Dialog::New, Dialog::Load, Dialog::Override, Dialog::Delete][b];
                self.open_dialog(level, kind, id);
                return Action::None;
            }
        }
        Action::None
    }

    /// Right click / Escape: the dialog, else the UI closes.
    pub fn back(&mut self, level: &mut Level) -> Action {
        if self.dialog.take().is_some() {
            self.refresh(level);
            return Action::None;
        }
        Action::Close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_file_names() {
        assert_eq!(file_name(0), "000.sav");
        assert_eq!(file_name(57), "057.sav");
        assert_eq!(file_name(AUTO), "a_000.sav");
    }
}
