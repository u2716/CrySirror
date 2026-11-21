// SPDX-License-Identifier: MPL-2.0
//! `AL.Scene.ActionMapScene` for a new game: `Initialize` opens
//! `MapSelectUI` (`SaveData.MapID` is -1); a map button
//! (`AL.MapUIInfo._id`) opens `MapConfirmDialog` (`DialogType.Normal`
//! while no map is set), Enter closes the UI with the id and
//! `MapSelectCallBack` runs `MapManager.ChangeMap(id, timeZone)` under the
//! scene fade; the map is then viewed from its first move-point camera.
use crate::{level::Level, map::MapInfo, scenes::Request};
use crysirror_unity::scene6::Scene6;
use std::collections::HashMap;

/// `am/pre/u` UI prefabs (`ActionMapUI.Create`, `MapSelectUI.Create`).
pub const UI_BUNDLE: &str = "lib/am/pre/u/000_00.unity3d";

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// `MapSelectUI` open.
    MapSelect,
    /// `MapConfirmDialog` for a map id.
    Confirm(i32),
    /// The map is shown.
    Map(i32),
}

/// What the controller asks of the session beyond scene requests.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    None,
    Scene(Request),
    /// Close `MapSelectUI` and `ChangeMap(id, time_zone)` behind the fade.
    ChangeMap { id: i32, time_zone: i32 },
    /// `_btnCharaEdit` (Add Heroine): open `CharaSelectUI`.
    OpenCharaSelect,
}

pub struct ActionMap {
    pub phase: Phase,
    /// `SaveData.Cycle` time zone (0 morning).
    pub time_zone: i32,
    /// Map button node → map id (`MapUIInfo._id`).
    buttons: HashMap<usize, i32>,
    dialog: Option<usize>,
    enter: Option<usize>,
    cancels: Vec<usize>,
    title: Option<usize>,
}

fn node_of(scene: &Scene6, path_id: i64) -> Option<usize> {
    scene.by_game_object.get(&path_id).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
}

impl ActionMap {
    /// `MapSelectUI.Initialize` over the instantiated prefab.
    pub fn new(select: &mut Level, maps: &[MapInfo]) -> Self {
        let scene = &select.scene;
        let mut buttons = HashMap::new();
        let mut dialog = None;
        let (mut enter, mut cancels, mut title) = (None, Vec::new(), None);
        let mut hidden = Vec::new();
        for (i, n) in scene.nodes.iter().enumerate() {
            for c in &n.components {
                match c.script.as_str() {
                    "AL.MapUIInfo" => {
                        let id = c.fields.int("_id").unwrap_or(-1) as i32;
                        let button = c.fields.pptr("_button").and_then(|p| node_of(scene, p.path_id)).unwrap_or(i);
                        buttons.insert(button, id);
                    }
                    "AL.MapConfirmDialog" => {
                        dialog = Some(i);
                        enter = c.fields.pptr("_btnEnter").and_then(|p| node_of(scene, p.path_id));
                        cancels = c.fields.array("_btnCancels").unwrap_or(&[]).iter().filter_map(|v| if let crysirror_unity::managed::Value::PPtr(p) = v { node_of(scene, p.path_id) } else { None }).collect();
                    }
                    "AL.MapSelectUI" => {
                        title = c.fields.pptr("_btnTitle").and_then(|p| node_of(scene, p.path_id));
                        // `CharaIconClear`: no heroine icons on a new game.
                        for info in c.fields.array("_charaInfos").unwrap_or(&[]) {
                            if let crysirror_unity::managed::Value::Struct(f) = info {
                                if let Some(n) = f.pptr("ObjBase").and_then(|p| node_of(scene, p.path_id)) {
                                    hidden.push(n);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // `MapInteractable`: maps without a list entry are locked, and the
        // list's open flag (last column) gates the rest.
        // SATD-amanatsu-port: the open flag comes from `map_info`, not from
        // save progress.
        for (&node, &id) in &buttons {
            let open = maps.iter().any(|m| m.id == id);
            select.interactable.insert(node, open);
        }
        if let Some(d) = dialog {
            select.set_active(d, false);
        }
        for n in hidden {
            select.set_active(n, false);
        }
        Self { phase: Phase::MapSelect, time_zone: 0, buttons, dialog, enter, cancels, title }
    }

    /// A Button click inside `MapSelectUI`.
    pub fn click(&mut self, select: &mut Level, node: usize) -> Action {
        match self.phase.clone() {
            Phase::MapSelect => {
                if let Some(&id) = self.buttons.get(&node) {
                    // `Next`: the confirmation while `OpenParameter.MapID`
                    // has no value.
                    if let Some(d) = self.dialog {
                        select.set_active(d, true);
                    }
                    self.phase = Phase::Confirm(id);
                    select.log.push(format!("map select {id}: confirm"));
                } else if select.scene.nodes[node].name == "btnHeroin" {
                    return Action::OpenCharaSelect;
                } else if Some(node) == self.title {
                    select.log.push("SATD-amanatsu-port: MapSelectUI title button (ExitDialog to title) not ported".into());
                } else {
                    select.log.push(format!("SATD-amanatsu-port: MapSelectUI button {} not ported", select.scene.path(node)));
                }
                Action::None
            }
            Phase::Confirm(id) => {
                if Some(node) == self.enter {
                    if let Some(d) = self.dialog {
                        select.set_active(d, false);
                    }
                    self.phase = Phase::Map(id);
                    return Action::ChangeMap { id, time_zone: self.time_zone };
                }
                if self.cancels.contains(&node) {
                    if let Some(d) = self.dialog {
                        select.set_active(d, false);
                    }
                    self.phase = Phase::MapSelect;
                }
                Action::None
            }
            Phase::Map(_) => Action::None,
        }
    }

    /// Right click / Escape: cancel the dialog.
    pub fn back(&mut self, select: &mut Level) {
        if let Phase::Confirm(_) = self.phase {
            if let Some(d) = self.dialog {
                select.set_active(d, false);
            }
            self.phase = Phase::MapSelect;
        }
    }
}
