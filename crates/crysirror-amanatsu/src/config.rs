// SPDX-License-Identifier: MPL-2.0
//! `AL.Config.ConfigWindow` (the `Config` prefab of the ADV Addressables
//! bundle) over the `Manager.Config` data persisted as `UserData/config.xml`
//! (`<Preferences>` with `Camera`, `Graphic`, `Text`, `GameSystem`, `HScene`,
//! `Sound`, `Voice` sections; volumes as `Volume[n] : Switch[b]`).
//!
//! The settings (`CameraSetting`, `ScreenSetting`, `GraphicSetting`,
//! `GameSetting`, `TextSetting`, `HSceneSetting`, `AudioSetting`,
//! `VoiceSetting`) bind toggle arrays whose index is the value (`[ON, OFF]`
//! pairs are true at index 0; `_tglAutoSaveTiming` [None, TimeZone, Day] is
//! `AutoSaveMode` 0–2), single toggles and sliders; the window scrolls its
//! `Content` under the `RectMask2D` viewport. The data lives in the engine
//! save mount (`user/config.xml`); a missing file is first read from the
//! installation's `UserData/config.xml`.
//!
//! Applied: volumes (mix groups, with a `voice_cNN` group per
//! personality under the voice master), `Text/FontSpeed` (message reveal),
//! `Text/WindowAlpha`, `GameSystem/AutoSaveMode`, `HScene/Visible` and
//! `Graphic/Bloom`/`Fog` (re-applied to the map when the window closes).
//!
//! SATD-amanatsu-port: the other values (camera, the other graphics
//! switches, H draw options, read skip) are stored but not applied; keys absent from the shipped file (`Urine`,
//! `UrineDraw`, `Ambient`, `SelfShadow`, `SSR`, `ReflectionProbe`,
//! `SunShaft`, `Rain`) use their field names; the screen dropdowns, colour
//! pickers, the per-personality voice rows' controls, section shortcuts, slider number
//! texts, reset (`btnInit`) defaults and the window fade are not ported.
use crate::level::Level;
use crysirror_unity::{managed::Value, scene6::Scene6, ugui};
use std::collections::HashMap;

pub const PREFAB: &str = "Config";

/// `config.xml`: ordered sections of ordered (key, text) entries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Config {
    /// The two-level `<Preferences>` document.
    pub fn parse(xml: &str) -> Self {
        let mut out = Config::default();
        let body = xml.split_once("<Preferences>").map_or("", |(_, b)| b);
        let mut rest = body;
        while let Some(open) = rest.find('<') {
            rest = &rest[open + 1..];
            if rest.starts_with('/') {
                // `</Section>` or `</Preferences>`.
                let Some(end) = rest.find('>') else { break };
                rest = &rest[end + 1..];
                continue;
            }
            let Some(end) = rest.find('>') else { break };
            let name = rest[..end].trim_end_matches('/').to_owned();
            rest = &rest[end + 1..];
            let close = format!("</{name}>");
            let Some(at) = rest.find(&close) else { continue };
            let inner = &rest[..at];
            if inner.contains('<') {
                let mut entries = Vec::new();
                let mut s = inner;
                while let Some(o) = s.find('<') {
                    s = &s[o + 1..];
                    let Some(e) = s.find('>') else { break };
                    let key = s[..e].to_owned();
                    s = &s[e + 1..];
                    let Some(c) = s.find(&format!("</{key}>")) else { break };
                    entries.push((key.clone(), unescape(&s[..c])));
                    s = &s[c + key.len() + 3..];
                }
                out.sections.push((name, entries));
            }
            rest = &rest[at + close.len()..];
        }
        out
    }

    pub fn to_xml(&self) -> String {
        let mut s = String::from("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Preferences>\r\n");
        for (name, entries) in &self.sections {
            s.push_str(&format!("  <{name}>\r\n"));
            for (k, v) in entries {
                s.push_str(&format!("    <{k}>{}</{k}>\r\n", escape(v)));
            }
            s.push_str(&format!("  </{name}>\r\n"));
        }
        s.push_str("</Preferences>");
        s
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.iter().find(|s| s.0 == section)?.1.iter().find(|e| e.0 == key).map(|e| e.1.as_str())
    }

    pub fn set(&mut self, section: &str, key: &str, value: String) {
        let i = match self.sections.iter().position(|s| s.0 == section) {
            Some(i) => i,
            None => {
                self.sections.push((section.to_owned(), Vec::new()));
                self.sections.len() - 1
            }
        };
        let entries = &mut self.sections[i].1;
        match entries.iter_mut().find(|e| e.0 == key) {
            Some(e) => e.1 = value,
            None => entries.push((key.to_owned(), value)),
        }
    }

    pub fn flag(&self, section: &str, key: &str, default: bool) -> bool {
        self.get(section, key).map_or(default, |v| v.eq_ignore_ascii_case("true"))
    }

    pub fn number(&self, section: &str, key: &str, default: f32) -> f32 {
        self.get(section, key).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
    }

    /// `Volume[n] : Switch[b]`.
    pub fn volume(&self, section: &str, key: &str) -> (i32, bool) {
        let v = self.get(section, key).unwrap_or("Volume[100] : Switch[True]");
        let between = |tag: &str| v.split_once(&format!("{tag}[")).and_then(|(_, r)| r.split_once(']')).map(|(x, _)| x.trim().to_owned());
        (between("Volume").and_then(|x| x.parse().ok()).unwrap_or(100), between("Switch").is_none_or(|x| x.eq_ignore_ascii_case("true")))
    }

    pub fn set_volume(&mut self, section: &str, key: &str, volume: i32, on: bool) {
        self.set(section, key, format!("Volume[{volume}] : Switch[{}]", if on { "True" } else { "False" }));
    }

    /// The linear gain of a sound volume (0 when switched off).
    pub fn gain(&self, section: &str, key: &str) -> f32 {
        let (v, on) = self.volume(section, key);
        if on {
            v.clamp(0, 100) as f32 / 100.
        } else {
            0.
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// What a toggle choice stores.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Store {
    /// `[ON, OFF]`: index 0 is true.
    Flag,
    /// A single toggle: on is true.
    Single,
    /// The index.
    Index,
    /// `[Windowed, FullScreen]` (or `[normal, inverted]`): index 1 is true.
    SecondFlag,
}

#[derive(Clone, Debug)]
enum Control {
    Choice { section: &'static str, key: &'static str, toggles: Vec<usize>, store: Store },
    Slider { section: &'static str, key: &'static str, node: usize, fields: crysirror_unity::managed::Fields, volume: Option<usize> },
}

/// (setting script, field, section, key, store) of the toggle arrays and
/// single toggles.
const CHOICES: &[(&str, &str, &str, &str, Store)] = &[
    ("CameraSetting", "_tglLook", "Camera", "Look", Store::Flag),
    // `[tglON 標準, tglOFF 反転]`: inverted at index 1.
    ("CameraSetting", "_tglInvertMoveX", "Camera", "InvertMoveX", Store::SecondFlag),
    ("CameraSetting", "_tglInvertMoveY", "Camera", "InvertMoveY", Store::SecondFlag),
    ("ScreenSetting", "_tglFullScreen", "Graphic", "FullScreen", Store::SecondFlag),
    ("GraphicSetting", "_qualityNumberToggle", "Graphic", "Quality", Store::Index),
    ("GraphicSetting", "_antialiasings", "Graphic", "Antialiasing", Store::Index),
    ("GraphicSetting", "_ambientToggles", "Graphic", "Ambient", Store::Flag),
    ("GraphicSetting", "_mapToggles", "Graphic", "Map", Store::Flag),
    ("GraphicSetting", "_shieldToggles", "Graphic", "Shield", Store::Flag),
    ("GraphicSetting", "_bloomToggle", "Graphic", "Bloom", Store::Single),
    ("GraphicSetting", "_depthOfFieldToggle", "Graphic", "DepthOfField", Store::Single),
    ("GraphicSetting", "_vignetteToggle", "Graphic", "Vignette", Store::Single),
    ("GraphicSetting", "_ssaoToggle", "Graphic", "SSAO", Store::Single),
    ("GraphicSetting", "_atmosphereToggle", "Graphic", "Fog", Store::Single),
    ("GraphicSetting", "_selfShadowToggle", "Graphic", "SelfShadow", Store::Single),
    ("GraphicSetting", "_ssrToggle", "Graphic", "SSR", Store::Single),
    ("GraphicSetting", "_rpToggle", "Graphic", "ReflectionProbe", Store::Single),
    ("GraphicSetting", "_sunShaftToggle", "Graphic", "SunShaft", Store::Single),
    ("GraphicSetting", "_rainToggle", "Graphic", "Rain", Store::Single),
    ("GameSetting", "_tglAutoSaveTiming", "GameSystem", "AutoSaveMode", Store::Index),
    ("GameSetting", "_tglAccessoryNum", "GameSystem", "LoadsAccessoryNum", Store::Index),
    ("TextSetting", "_tglSkip", "Text", "ReadSkip", Store::Flag),
    ("HSceneSetting", "_tglInitCamera", "HScene", "InitCamera", Store::Flag),
    ("HSceneSetting", "_tglDraw", "HScene", "Visible", Store::Flag),
    ("HSceneSetting", "_tglSon", "HScene", "Son", Store::Flag),
    ("HSceneSetting", "_tglSilhouette", "HScene", "SimpleBody", Store::Flag),
    ("HSceneSetting", "_tglSiruDraw", "HScene", "SiruDraw", Store::Index),
    ("HSceneSetting", "_tglSio", "HScene", "Sio", Store::Flag),
    ("HSceneSetting", "_tglSioDraw", "HScene", "SioDraw", Store::Index),
    ("HSceneSetting", "_tglUrine", "HScene", "Urine", Store::Flag),
    ("HSceneSetting", "_tglUrineDraw", "HScene", "UrineDraw", Store::Index),
    ("HSceneSetting", "_tglWeakStop", "HScene", "WeakStop", Store::Flag),
    ("HSceneSetting", "_tglEyeDir0", "HScene", "EyeDir0", Store::Flag),
    ("HSceneSetting", "_tglNeckDir0", "HScene", "NeckDir0", Store::Flag),
    ("HSceneSetting", "_tglEyeDir1", "HScene", "EyeDir1", Store::Flag),
    ("HSceneSetting", "_tglNeckDir1", "HScene", "NeckDir1", Store::Flag),
];

/// (setting script, field, section, key) of the plain sliders.
const SLIDERS: &[(&str, &str, &str, &str)] = &[
    ("CameraSetting", "_sliderSensitivityX", "Camera", "SensitivityX"),
    ("CameraSetting", "_sliderSensitivityY", "Camera", "SensitivityY"),
    ("TextSetting", "_sliderFontSpeed", "Text", "FontSpeed"),
    ("TextSetting", "_sliderTextWindowAlpha", "Text", "WindowAlpha"),
];

/// `AudioSetting` volume rows (`{Toggle, Slider, Image}`).
const VOLUMES: &[(&str, &str)] = &[("_master", "Master"), ("_bgm", "BGM"), ("_env", "ENV"), ("_systemSE", "SystemSE"), ("_gameSE", "GameSE")];

/// What a click in the window asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    None,
    /// A value changed (apply and persist).
    Changed,
    Close,
    /// `_btnTitle`: back to the title.
    Title,
}

pub struct ConfigUi {
    controls: Vec<Control>,
    content: Option<usize>,
    viewport: Option<usize>,
    content_pos: [f32; 2],
    scroll: f32,
    drag: Option<usize>,
    close: Option<usize>,
    title: Option<usize>,
    /// `SliderObservableToTMPro`: slider → its number text.
    labels: HashMap<usize, usize>,
}

fn node_of(scene: &Scene6, path_id: i64) -> Option<usize> {
    scene.by_game_object.get(&path_id).copied().or_else(|| scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
}

fn component<'a>(scene: &'a Scene6, n: usize, script: &str) -> Option<&'a crysirror_unity::managed::Fields> {
    scene.nodes[n].components.iter().find(|c| c.script.ends_with(script)).map(|c| &c.fields)
}

/// The Toggle's `graphic` node (shown while on).
fn toggle_graphic(scene: &Scene6, toggle: usize) -> Option<usize> {
    let f = component(scene, toggle, "Toggle")?;
    node_of(scene, f.pptr("graphic").or(f.pptr("Graphic"))?.path_id)
}

fn slider_fields(scene: &Scene6, n: usize) -> Option<crysirror_unity::managed::Fields> {
    scene.nodes[n].components.iter().find(|c| c.fields.get("m_FillRect").is_some()).map(|c| c.fields.clone())
}

impl ConfigUi {
    /// Read the settings' references and show the config's values.
    pub fn new(level: &mut Level, cfg: &Config) -> Self {
        let scene = &level.scene;
        let setting = |script: &str| scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.script.ends_with(&format!("Config.{script}"))).map(|c| (i, c.fields.clone())));
        let mut controls = Vec::new();
        for &(script, field, section, key, store) in CHOICES {
            let Some((_, f)) = setting(script) else { continue };
            let toggles: Vec<usize> = match f.get(field) {
                Some(Value::Array(a)) => a.iter().filter_map(|v| if let Value::PPtr(p) = v { node_of(scene, p.path_id) } else { None }).collect(),
                Some(Value::PPtr(p)) => node_of(scene, p.path_id).into_iter().collect(),
                _ => Vec::new(),
            };
            if !toggles.is_empty() {
                controls.push(Control::Choice { section, key, toggles, store });
            }
        }
        for &(script, field, section, key) in SLIDERS {
            let Some((_, f)) = setting(script) else { continue };
            let Some(node) = f.pptr(field).and_then(|p| node_of(scene, p.path_id)) else { continue };
            if let Some(fields) = slider_fields(scene, node) {
                controls.push(Control::Slider { section, key, node, fields, volume: None });
            }
        }
        if let Some((_, f)) = setting("AudioSetting") {
            for &(field, key) in VOLUMES {
                let Some(Value::Struct(row)) = f.get(field) else { continue };
                let toggle = row.pptr("Toggle").and_then(|p| node_of(scene, p.path_id));
                let Some(node) = row.pptr("Slider").and_then(|p| node_of(scene, p.path_id)) else { continue };
                if let Some(fields) = slider_fields(scene, node) {
                    controls.push(Control::Slider { section: "Sound", key, node, fields, volume: toggle });
                }
            }
        }
        // Rows this title does not offer (`GraphicSetting._objVisible`), the
        // dropdown `Blocker` saved in the prefab and the closed colour picker
        // menus.
        let mut hidden: Vec<usize> = Vec::new();
        if let Some((_, f)) = setting("GraphicSetting") {
            hidden.extend(f.array("_objVisible").unwrap_or(&[]).iter().filter_map(|v| if let Value::PPtr(p) = v { node_of(scene, p.path_id) } else { None }));
        }
        hidden.extend(scene.nodes.iter().enumerate().filter(|(_, n)| n.name == "menuPicker" || (n.name == "Blocker" && n.parent.is_some_and(|p| scene.nodes[p].parent.is_none()))).map(|(i, _)| i));
        let window = scene.nodes.iter().find_map(|n| n.components.iter().find(|c| c.script.ends_with("ConfigWindow")).map(|c| c.fields.clone()));
        let refer = |k: &str| window.as_ref().and_then(|w| w.pptr(k)).and_then(|p| node_of(scene, p.path_id));
        let content = refer("_mainWindow");
        let content_pos = content.and_then(|c| scene.nodes[c].xform.rect).map_or([0.; 2], |r| r.anchored_position);
        let mut labels = HashMap::new();
        for n in &scene.nodes {
            for c in n.components.iter().filter(|c| c.script.ends_with("SliderObservableToTMPro")) {
                if let (Some(sl), Some(t)) = (c.fields.pptr("_slider").and_then(|p| node_of(scene, p.path_id)), c.fields.pptr("_text").and_then(|p| node_of(scene, p.path_id))) {
                    labels.insert(sl, t);
                }
            }
        }
        for n in hidden {
            level.set_active(n, false);
        }
        // `GraphicSetting` rebuilds `_layoutGroups` after hiding effects: the
        // Effects row takes the height of its grid's visible rows.
        let scene = &level.scene;
        let grid_row = scene.nodes.iter().position(|n| n.name == "Root" && n.components.iter().any(|c| c.script.ends_with("GridLayoutGroup")) && n.parent.is_some_and(|p| scene.nodes[p].name == "Effects"));
        if let Some(root) = grid_row {
            let g = component(scene, root, "GridLayoutGroup").cloned().unwrap_or_default();
            let visible = scene.nodes[root].children.iter().filter(|&&c| level.is_active(c)).count();
            let cols = g.int("m_ConstraintCount").unwrap_or(3).max(1) as usize;
            let rows = visible.div_ceil(cols).max(1);
            let cell = match g.get("m_CellSize") {
                Some(Value::Struct(v)) => v.float("y").unwrap_or(28.) as f32,
                _ => 28.,
            };
            let spacing = match g.get("m_Spacing") {
                Some(Value::Struct(v)) => v.float("y").unwrap_or(8.) as f32,
                _ => 8.,
            };
            let row = scene.nodes[root].parent.unwrap_or(root);
            if let Some(r) = scene.nodes[row].xform.rect {
                let h = 4. + rows as f32 * cell + (rows - 1) as f32 * spacing;
                level.states.entry(row).or_default().size_delta = Some([r.size_delta[0], h]);
            }
        }
        let scene = &level.scene;
        let refer = |k: &str| window.as_ref().and_then(|w| w.pptr(k)).and_then(|p| node_of(scene, p.path_id));
        let ui = ConfigUi { controls, content, viewport: refer("_rectMask2D"), content_pos, scroll: 0., drag: None, close: refer("_btnClose"), title: refer("_btnTitle"), labels };
        ui.show(level, cfg);
        ui
    }

    /// The toggles' graphics and slider fills of the config's values.
    pub fn show(&self, level: &mut Level, cfg: &Config) {
        for c in &self.controls {
            match c {
                Control::Choice { section, key, toggles, store } => {
                    let on = chosen(cfg, section, key, *store, toggles.len());
                    for (i, &t) in toggles.iter().enumerate() {
                        let is_on = if *store == Store::Single { on == 0 } else { i == on };
                        if let Some(g) = toggle_graphic(&level.scene, t) {
                            level.set_active(g, is_on);
                        }
                    }
                }
                Control::Slider { section, key, node, fields, volume } => {
                    let value = match volume {
                        Some(_) => cfg.volume(section, key).0 as f32,
                        None => cfg.number(section, key, fields.float("m_Value").unwrap_or(0.) as f32),
                    };
                    // `_format` "0".
                    if let Some(&t) = self.labels.get(node) {
                        level.states.entry(t).or_default().text = Some(format!("{}", value.round() as i64));
                    }
                    let scene = level.scene.clone();
                    slider_visuals(&scene, fields, value, &mut level.states);
                    if let Some(t) = volume {
                        if let Some(g) = toggle_graphic(&scene, *t) {
                            level.set_active(g, cfg.volume(section, key).1);
                        }
                    }
                }
            }
        }
        if let Some(c) = self.content {
            level.states.entry(c).or_default().anchored_position = Some([self.content_pos[0], self.content_pos[1] + self.scroll]);
        }
    }

    /// One frame of pointer input: `pressed`/`down` of the left button and
    /// the wheel steps (positive scrolls down); `rects` from the overlay's
    /// layout.
    pub fn input(&mut self, level: &mut Level, rects: &HashMap<usize, ugui::Rect>, (x, y): (f32, f32), pressed: bool, down: bool, wheel: f32, cfg: &mut Config) -> Action {
        let mut action = Action::None;
        if wheel != 0. {
            if let (Some(c), Some(v)) = (self.content.and_then(|c| rects.get(&c)), self.viewport.and_then(|v| rects.get(&v))) {
                if v.contains(x, y) {
                    let max = (c.height() - v.height()).max(0.);
                    self.scroll = (self.scroll + wheel * 120.).clamp(0., max);
                }
            }
        }
        let inside_view = self.viewport.and_then(|v| rects.get(&v)).is_none_or(|v| v.contains(x, y));
        if pressed {
            if self.close.and_then(|n| rects.get(&n)).is_some_and(|r| r.contains(x, y)) {
                return Action::Close;
            }
            if self.title.filter(|&n| level.is_active(n)).and_then(|n| rects.get(&n)).is_some_and(|r| r.contains(x, y)) {
                return Action::Title;
            }
            for (ci, c) in self.controls.iter().enumerate() {
                match c {
                    Control::Choice { section, key, toggles, store } if inside_view => {
                        if let Some(i) = toggles.iter().position(|t| rects.get(t).is_some_and(|r| r.contains(x, y))) {
                            let value = match store {
                                Store::Flag => (i == 0).to_string(),
                                Store::SecondFlag => (i == 1).to_string(),
                                Store::Single => (!cfg.flag(section, key, true)).to_string(),
                                Store::Index => i.to_string(),
                            };
                            let value = if matches!(store, Store::Flag | Store::SecondFlag | Store::Single) { capital(&value) } else { value };
                            cfg.set(section, key, value);
                            action = Action::Changed;
                        }
                    }
                    Control::Slider { section, key, node, volume: Some(t), .. } if inside_view && rects.get(t).is_some_and(|r| r.contains(x, y)) && !rects.get(node).is_some_and(|r| r.contains(x, y)) => {
                        let (v, on) = cfg.volume(section, key);
                        cfg.set_volume(section, key, v, !on);
                        action = Action::Changed;
                    }
                    Control::Slider { node, .. } if inside_view && rects.get(node).is_some_and(|r| r.contains(x, y)) => self.drag = Some(ci),
                    _ => {}
                }
            }
        }
        if !down {
            self.drag = None;
        }
        if let Some(Control::Slider { section, key, node, fields, volume }) = self.drag.and_then(|i| self.controls.get(i)) {
            if let Some(r) = rects.get(node) {
                let (lo, hi) = (fields.float("m_MinValue").unwrap_or(0.) as f32, fields.float("m_MaxValue").unwrap_or(1.) as f32);
                let t = ((x - r.x0) / r.width().max(1.)).clamp(0., 1.);
                let mut v = lo + (hi - lo) * t;
                if fields.int("m_WholeNumbers").unwrap_or(0) != 0 || volume.is_some() {
                    v = v.round();
                }
                match volume {
                    Some(_) => {
                        let on = cfg.volume(section, key).1;
                        cfg.set_volume(section, key, v as i32, on);
                    }
                    None => cfg.set(section, key, format!("{v}")),
                }
                action = Action::Changed;
            }
        }
        self.show(level, cfg);
        action
    }
}

fn capital(v: &str) -> String {
    if v == "true" {
        "True".into()
    } else {
        "False".into()
    }
}

/// The selected toggle index of a choice (0 = on for a single toggle).
fn chosen(cfg: &Config, section: &str, key: &str, store: Store, n: usize) -> usize {
    match store {
        Store::Flag => usize::from(!cfg.flag(section, key, true)),
        Store::SecondFlag => usize::from(cfg.flag(section, key, true)),
        Store::Single => usize::from(!cfg.flag(section, key, true)),
        Store::Index => (cfg.number(section, key, 0.).max(0.) as usize).min(n.saturating_sub(1)),
    }
}

/// `Slider.UpdateVisuals`: the fill span and the handle position.
fn slider_visuals(scene: &Scene6, slider: &crysirror_unity::managed::Fields, value: f32, states: &mut HashMap<usize, ugui::NodeState>) {
    ugui::slider_value(scene, slider, value, states);
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Preferences>\r\n  <Camera>\r\n    <Look>True</Look>\r\n    <SensitivityX>0.5</SensitivityX>\r\n  </Camera>\r\n  <Sound>\r\n    <BGM>Volume[40] : Switch[True]</BGM>\r\n  </Sound>\r\n</Preferences>";

    #[test]
    fn round_trip_and_values() {
        let mut c = Config::parse(XML);
        assert!(c.flag("Camera", "Look", false));
        assert_eq!(c.number("Camera", "SensitivityX", 0.), 0.5);
        assert_eq!(c.volume("Sound", "BGM"), (40, true));
        assert_eq!(c.to_xml(), XML);
        c.set_volume("Sound", "BGM", 70, false);
        assert_eq!(c.gain("Sound", "BGM"), 0.);
        c.set("GameSystem", "AutoSaveMode", "1".into());
        let again = Config::parse(&c.to_xml());
        assert_eq!(again, c);
        assert_eq!(chosen(&again, "GameSystem", "AutoSaveMode", Store::Index, 3), 1);
    }
}
