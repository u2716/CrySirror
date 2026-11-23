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
    /// `_btnNextTimeZone`: the zone advanced to this value.
    NextTimeZone(i32),
    /// `_btnSaveLoad`: open `SaveLoadUI`.
    OpenSaveLoad,
}

pub struct ActionMap {
    pub phase: Phase,
    /// `SaveData.Cycle` time zone (0 morning).
    pub time_zone: i32,
    /// `SaveData.MapID` (a map is loaded behind the UI).
    pub map: Option<i32>,
    /// Map button node → map id (`MapUIInfo._id`).
    buttons: HashMap<usize, i32>,
    dialog: Option<usize>,
    enter: Option<usize>,
    cancels: Vec<usize>,
    title: Option<usize>,
    config: Option<usize>,
    next_zone: Option<usize>,
    save_load: Option<usize>,
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
        let (mut enter, mut cancels, mut title, mut config, mut next_zone, mut save_load) = (None, Vec::new(), None, None, None, None);
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
                        config = c.fields.pptr("_btnConfig").and_then(|p| node_of(scene, p.path_id));
                        next_zone = c.fields.pptr("_btnNextTimeZone").and_then(|p| node_of(scene, p.path_id));
                        save_load = c.fields.pptr("_btnSaveLoad").and_then(|p| node_of(scene, p.path_id));
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
            select.set_interactable(node, open);
        }
        if let Some(d) = dialog {
            select.set_active(d, false);
        }
        for n in hidden {
            select.set_active(n, false);
        }
        Self { phase: Phase::MapSelect, time_zone: 0, map: None, buttons, dialog, enter, cancels, title, config, next_zone, save_load }
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
                } else if Some(node) == self.next_zone {
                    // SATD-amanatsu-port: with a map loaded behind the UI the
                    // original reloads it in the new zone at once; here the
                    // next map choice loads it.
                    self.time_zone = (self.time_zone + 1) % 4;
                    return Action::NextTimeZone(self.time_zone);
                } else if Some(node) == self.save_load {
                    return Action::OpenSaveLoad;
                } else if Some(node) == self.config {
                    return Action::Scene(Request::Config);
                } else if Some(node) == self.title {
                    return Action::Scene(Request::ConfirmTitle);
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

/// Manager.Game.ReturnTitle's normal Yes/No ConfirmDialog. Read the strings
/// from the selected installation language and bind its prefab's UIText/TMP
/// references; the same prefab also contains an inactive warning theme.
pub(crate) fn title_dialog(unity: &mut crate::unity::Unity) -> anyhow::Result<Level> {
    use anyhow::{Context, ensure};
    let language = if unity.list_dir("UserData").iter().any(|name| name == "setup.xml") {
        let bytes = unity.read_all("UserData/setup.xml")?;
        let xml = if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
            ensure!(bytes.len() % 2 == 0, "invalid UTF-16 setup.xml");
            String::from_utf16(&bytes.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect::<Vec<_>>())?
        } else {
            String::from_utf8(bytes)?
        };
        xml.split_once("<Language>").and_then(|(_, x)| x.split_once("</Language>"))
            .map(|(s, _)| s.trim().parse::<u32>()).transpose()?.unwrap_or(0)
    } else { 0 };
    let pack = unity.find_script_object(&format!("lib/localization/info/{language}.unity3d#"), "Localize.Translate.LanguagePack")?
        .context("return-title LanguagePack")?;
    let config = pack.fields("_config").context("language config")?;
    let mut level = Level::prefab(unity, crate::talk::ADV_BUNDLE, "ConfirmDialog")?;
    let panel = level.find("ConfirmDialog/Dialog_Panel").context("normal ConfirmDialog theme")?;
    let warning = level.find("ConfirmDialog/Dialog_Panel_Del").context("warning ConfirmDialog theme")?;
    level.set_active(warning, false);
    crate::talk::show(&mut level, panel);
    let theme = level.scene.nodes[panel].components.iter().find(|c| c.script == "AL.Dialog.ConfirmDialogTheme")
        .context("ConfirmDialogTheme component")?.fields.clone();
    let node = |p: crysirror_unity::PPtr| node_of(&level.scene, p.path_id).context("ConfirmDialog UI reference");
    let sentence = node(theme.pptr("_txtSentence").context("ConfirmDialog sentence")?)?;
    let warn = node(theme.pptr("_txtSentenceWarning").context("ConfirmDialog warning")?)?;
    let buttons = theme.array("_btnContents").context("ConfirmDialog button list")?.iter()
        .map(|v| match v { crysirror_unity::managed::Value::PPtr(p) => node(*p), _ => anyhow::bail!("invalid ConfirmDialog button") })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let [ok, yes, no] = buttons.as_slice() else { anyhow::bail!("unsupported ConfirmDialog button layout") };
    level.set_active(*ok, false);
    level.set_active(*yes, true);
    level.set_active(*no, true);
    level.set_active(warn, false);
    for (owner, key) in [(sentence, "_titleDialog"), (*yes, "_dialogYes"), (*no, "_dialogNo")] {
        let text = level.scene.nodes[owner].components.iter().find(|c| c.script == "ILLGAMES.Unity.UI.UIText")
            .and_then(|c| c.fields.pptr("_tmpText")).and_then(|p| node_of(&level.scene, p.path_id))
            .context("ConfirmDialog text target")?;
        level.states.entry(text).or_default().text = Some(config.string(key).with_context(|| format!("language {key}"))?.to_owned());
    }
    Ok(level)
}
