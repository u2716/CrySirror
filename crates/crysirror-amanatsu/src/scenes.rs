// SPDX-License-Identifier: MPL-2.0
//! Build-scene controllers ported from `AL.Scene.InitScene`,
//! `AL.Scene.LogoScene` and `AL.Title.TitleScene` (+ `SelectMenuAnimation`,
//! `PressAnyAnimation`). Each runs once per frame over its [`Level`]; the
//! native `async UniTask` steps are explicit stages.
use crate::{level::Level, sound::Call};
use crysirror_unity::{events::PersistentCall, managed::Value};

/// `ILLGAMES.Settings.SceneSettings` ids (`SceneSettings_00_Master`).
pub mod scene_id {
    pub const LOGO: i32 = 1;
    pub const TITLE: i32 = 4;
    pub const CUSTOM: i32 = 5;
    pub const ACTION_MAP: i32 = 6;
    pub const MEMORY: i32 = 7;
}

/// What a scene asks of the session this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    None,
    /// `SceneSettings.Load(id)`: fade in, then load the scene.
    Load(i32),
    /// `Application.Quit` (through `ExitDialog`).
    Quit,
    /// `TitleScene.OnLoad`: `SaveLoadUI` (`IsTitle`); its Load runs
    /// `LoadSaveFileInternal`: load a
    /// save and enter the action map.
    LoadSave,
    /// `ConfigWindow.Load`.
    Config,
}

pub struct Input {
    pub now: u64,
    pub fading: bool,
    pub click: Option<(f32, f32)>,
    pub right_click: bool,
    pub escape: bool,
    pub f1: bool,
}

/// `LogoScene.Start`: fade out, brand call, wait `_waitTime` (skippable
/// with a click unless `_clickWait`), then the title.
pub struct Logo {
    wait_ms: u64,
    skippable: bool,
    shown_at: Option<u64>,
}

/// `PropertyData._sseID` / `_bgmID.Title` values of `PropertyData`
/// (`OK_s`, `OK_l`, `Cancel`).
#[derive(Clone, Copy, Debug)]
pub struct SoundIds {
    pub ok_s: i32,
    pub ok_l: i32,
    pub cancel: i32,
    pub title_bgm: i32,
    /// `_bgmID` Morning, Afternoon, EarlyEvening, Evening (`PlayBGMBYActionMap`).
    pub zone_bgm: [i32; 4],
    /// `_sseID.TimeZoneChange`.
    pub time_zone_se: i32,
}

impl Logo {
    pub fn new(level: &Level) -> Self {
        let f = level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.Scene.LogoScene").map(|c| c.fields.clone()).unwrap_or_default();
        Self {
            wait_ms: (f.float("_waitTime").unwrap_or(2.) * 1000.) as u64,
            skippable: f.int("_clickWait").unwrap_or(0) == 0,
            shown_at: None,
        }
    }

    pub fn step(&mut self, level: &mut Level, input: &Input) -> Request {
        if input.fading {
            return Request::None;
        }
        if self.shown_at.is_none() {
            // A random brand call of `sd/dt/bc`.
            level.sounds.push(Call::RandomClip("sd/dt/bc/".into()));
        }
        let shown = *self.shown_at.get_or_insert(input.now);
        let skip = self.skippable && input.click.is_some();
        if skip || input.now >= shown + self.wait_ms {
            level.log.push(format!("logo done ({})", if skip { "skipped" } else { "waited" }));
            return Request::Load(scene_id::TITLE);
        }
        Request::None
    }
}

/// `TitleScene.MenuIndex`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    PressAny = 0,
    Menu = 1,
    GamePlay = 2,
    CharaCreation = 3,
    Network = 4,
}

/// `PressAnyAnimation.State`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressAny {
    None,
    Blink,
    FadeOut { until: u64 },
}

pub struct Title {
    menu_objects: Vec<Option<usize>>,
    sub_groups: Vec<Option<usize>>,
    buttons: Vec<Option<usize>>,
    menu_layout: Option<usize>,
    press_any_fadeout_ms: u64,
    /// `SelectMenuAnimation._secondMenuTime` / `_backWaitTime` (ms).
    second_menu_ms: u64,
    back_wait_ms: u64,
    pub select: Menu,
    press_any: PressAny,
    /// `SelectMenuAnimation` lock: raycasts blocked until this time.
    locked_until: Option<u64>,
    started: bool,
    ids: SoundIds,
}

fn refs(level: &Level, fields: &crysirror_unity::managed::Fields, name: &str) -> Vec<Option<usize>> {
    fields.array(name).unwrap_or(&[]).iter().map(|v| if let Value::PPtr(p) = v { level.owner(p.path_id) } else { None }).collect()
}

impl Title {
    pub fn new(level: &mut Level, ids: SoundIds, memories: bool) -> Self {
        let f = level.scene.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.Title.TitleScene").map(|c| c.fields.clone()).unwrap_or_default();
        let ptr = |name: &str| f.pptr(name).filter(|p| !p.is_null()).and_then(|p| level.owner(p.path_id));
        let menu_layout = ptr("_selectMenuAnimation");
        // `PressAnyAnimation._fadeout`: the longest of its PlayAnimationUIs.
        let fadeout = ptr("_pressAnyAnimation")
            .and_then(|n| level.scene.nodes[n].components.iter().find(|c| c.script == "AL.Title.PressAnyAnimation"))
            .and_then(|c| c.fields.pptr("_fadeout"))
            .and_then(|p| level.owner(p.path_id))
            .map(|n| {
                level.scene.nodes[n].components.iter().filter_map(|c| c.fields.float("_duration")).fold(0f64, f64::max)
            })
            .unwrap_or(0.);
        let sma = menu_layout.and_then(|n| level.scene.nodes[n].components.iter().find(|c| c.script == "AL.Title.SelectMenuAnimation")).map(|c| c.fields.clone()).unwrap_or_default();
        let mut t = Self {
            second_menu_ms: sma.int("_secondMenuTime").unwrap_or(0).max(0) as u64,
            back_wait_ms: sma.int("_backWaitTime").unwrap_or(0).max(0) as u64,
            menu_objects: refs(level, &f, "_menuObject"),
            sub_groups: refs(level, &f, "_menuCanvasGroup"),
            buttons: refs(level, &f, "_menuButtons"),
            menu_layout,
            press_any_fadeout_ms: (fadeout * 1000.) as u64,
            select: Menu::PressAny,
            press_any: PressAny::None,
            locked_until: None,
            started: false,
            ids,
        };
        // Start: Voice.StopAll, Sound.Stop, then PlayBGMAsync(_bgmID.Title).
        level.sounds.push(Call::StopAll);
        level.sounds.push(Call::Bgm(ids.title_bgm));
        // Memories shows once a short movie was played (`GlobalData.Flags`).
        if let Some(Some(m)) = t.menu_objects.get(6) {
            level.set_active(*m, memories);
        }
        // SATD-amanatsu-port: the first-run language dialog
        // (`LanguageDLG`, `Localize.Translate`) stays hidden.
        if let Some(n) = level.find("Title/Canvas/Information/languageDLG") {
            level.set_active(n, false);
        }
        // SelectMenuAnimation.Initialize: lock the menu.
        t.lock(level, None);
        t
    }

    fn lock(&mut self, level: &mut Level, until: Option<u64>) {
        if let Some(n) = self.menu_layout {
            level.set_blocks_raycasts(n, false);
        }
        self.locked_until = until.or(Some(u64::MAX));
    }

    fn unlock(&mut self, level: &mut Level) {
        if let Some(n) = self.menu_layout {
            level.set_blocks_raycasts(n, true);
        }
        self.locked_until = None;
    }

    fn transitioning(&self, now: u64) -> bool {
        self.locked_until.is_some_and(|t| t != u64::MAX && now < t)
    }

    /// `OnMenuButtonsInteractable`: every `ButtonIndex` button.
    fn menu_buttons_interactable(&self, level: &mut Level, on: bool) {
        for b in self.buttons.iter().flatten() {
            level.set_interactable(*b, on);
        }
    }

    pub fn step(&mut self, level: &mut Level, input: &Input) -> Request {
        if !self.started {
            // After the opening fade (`StartFadeAsync(Out)`): PressAny blinks.
            // SATD-amanatsu-port: BGM, title call voice and the TOS check.
            if input.fading {
                return Request::None;
            }
            self.started = true;
            self.press_any = PressAny::Blink;
        }
        if let Some(t) = self.locked_until {
            if t != u64::MAX && input.now >= t {
                self.unlock(level);
            }
        }
        if let PressAny::FadeOut { until } = self.press_any {
            if input.now >= until {
                // PlayMenuAnimation continuation: Menu, SelectMenuAnimation.OnStart.
                self.press_any = PressAny::None;
                self.select = Menu::Menu;
                self.unlock(level);
            }
        }
        // `<Start>b__33_6` on Update while no fade runs.
        if input.fading {
            return Request::None;
        }
        if input.escape {
            // SATD-amanatsu-port: `ExitDialog` confirmation UI.
            level.sounds.push(Call::SystemSe(self.ids.ok_l));
            return Request::Quit;
        }
        if input.f1 {
            level.sounds.push(Call::SystemSe(self.ids.ok_l));
            return Request::Config;
        }
        if input.right_click {
            return self.on_back(level, input.now);
        }
        if self.select == Menu::PressAny && self.press_any == PressAny::Blink {
            // PlayMenuAnimation: press-any fades out, then the menu opens.
            self.press_any = PressAny::FadeOut { until: input.now + self.press_any_fadeout_ms };
            return Request::None;
        }
        Request::None
    }

    /// A Button click's script calls (`TitleScene` methods).
    pub fn call(&mut self, level: &mut Level, call: &PersistentCall, now: u64) -> Request {
        if call.type_name != "AL.Title.TitleScene" {
            level.log.push(format!("unported script call {}.{}", call.type_name, call.method));
            return Request::None;
        }
        match call.method.as_str() {
            "OnGamePlay" | "OnCustom" | "OnNetWork" => {
                level.sounds.push(Call::SystemSe(self.ids.ok_s));
                match call.method.as_str() {
                    "OnGamePlay" => self.on_next(level, Menu::GamePlay, 0, 6, now),
                    "OnCustom" => self.on_next(level, Menu::CharaCreation, 1, 7, now),
                    _ => self.on_next(level, Menu::Network, 2, 8, now),
                }
            }
            "OnBack" => return self.on_back(level, now),
            "OnStart" | "OnCustomMale" | "OnCustomFemale" | "OnMemory" | "OnQuit" | "OnOption" => {
                level.sounds.push(Call::SystemSe(self.ids.ok_l));
                return match call.method.as_str() {
                    "OnStart" => Request::Load(scene_id::ACTION_MAP),
                    "OnCustomMale" | "OnCustomFemale" => Request::Load(scene_id::CUSTOM),
                    "OnMemory" => Request::Load(scene_id::MEMORY),
                    "OnQuit" => Request::Quit,
                    _ => Request::Config,
                };
            }
            "OnLoad" => {
                level.sounds.push(Call::SystemSe(self.ids.ok_s));
                return Request::LoadSave;
            }
            "OnTOS" | "OnManual" | "OnLanguage" | "OnUploader" | "OnDownloader" => {
                level.sounds.push(Call::SystemSe(self.ids.ok_s));
                level.log.push(format!("TitleScene.{} (not ported)", call.method));
            }
            // SATD-amanatsu-port: TOS viewer,
            // uploader/downloader; OnManual opens an external URL.
            m => level.log.push(format!("TitleScene.{m} (not ported)")),
        }
        Request::None
    }

    /// OnGamePlay/OnCustom/OnNetWork: `SelectMenuAnimation.OnNext`, the sub
    /// menu's group and its Back button become interactable.
    fn on_next(&mut self, level: &mut Level, menu: Menu, group: usize, back: usize, now: u64) {
        self.select = menu;
        self.lock(level, Some(now + self.second_menu_ms));
        if let Some(Some(g)) = self.sub_groups.get(group) {
            level.set_group_interactable(*g, true);
        }
        self.menu_buttons_interactable(level, false);
        if let Some(Some(b)) = self.buttons.get(back) {
            level.set_interactable(*b, true);
        }
    }

    /// `TitleScene.OnBack`.
    fn on_back(&mut self, level: &mut Level, now: u64) -> Request {
        if self.transitioning(now) {
            return Request::None;
        }
        let (group, back) = match self.select {
            Menu::GamePlay => (0, 6),
            Menu::CharaCreation => (1, 7),
            Menu::Network => (2, 8),
            _ => return Request::None,
        };
        self.select = Menu::Menu;
        self.lock(level, Some(now + self.back_wait_ms));
        // The Back button's own m_OnClick (its Animator calls); its
        // TitleScene.OnBack returns while SelectMenuAnimation runs.
        if let Some(Some(b)) = self.buttons.get(back) {
            let calls = level.on_click(*b);
            let _ = level.invoke(&calls);
        }
        self.menu_buttons_interactable(level, true);
        if let Some(Some(g)) = self.sub_groups.get(group) {
            level.set_group_interactable(*g, false);
        }
        level.sounds.push(Call::SystemSe(self.ids.cancel));
        Request::None
    }

    pub fn menu_open(&self) -> bool {
        self.select != Menu::PressAny
    }
}
