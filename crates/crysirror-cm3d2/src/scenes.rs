// SPDX-License-Identifier: MPL-2.0
//! Per-scene MonoBehaviour logic ported from Assembly-CSharp (`SceneFirst`,
//! `SceneLogo`, `SceneWarning`, `SceneADV`, `TitleMgr`/`TitleCtrl`). Each
//! controller keeps the original modes, durations and transitions.
use crate::adapter::{Game, Opener};
use anyhow::{bail, Context, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    FadeIn,
    ClickWait,
    Loading,
    FadeOut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Title {
    started_ms: u64,
    /// `TitleCtrl.m_currentActiveButton`.
    active: usize,
    hovered: Option<usize>,
    pointer: Option<[i32; 2]>,
    closing: Option<u64>,
    /// Continue: the title panel's 0.5 s `TweenAlpha` (out, then the load
    /// panel; in again after a cancel) and the open load panel.
    panel: TitlePanel,
}

#[derive(Clone, Debug, PartialEq)]
enum TitlePanel {
    Shown,
    FadeOut(u64),
    Load(Box<crate::saveload::SaveLoad>),
    FadeIn(u64),
}

/// `UserEditMgr`/`UserEditAPI` state.
#[derive(Clone, Debug, PartialEq)]
pub struct UserEdit {
    /// Head menus (`m_listHead`): button name = menu file.
    heads: Vec<String>,
    /// Panel opened (`OnCharaLoaded` → `OpenUserEditPanel`).
    open: bool,
    /// Close request time (`BaseClosePanel`).
    closing: Option<u64>,
    /// `FadeOut(1)` of `EndNextScene` started.
    leaving: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Active {
    First,
    Logo(Mode),
    Warning(Mode),
    /// `SceneADV`: the ADV KAG drives this scene.
    Adv,
    /// `SceneSingleEffect`: the ADV KAG with `startmvpeffect`.
    SingleEffect,
    Title(Title),
    UserEdit(UserEdit),
    /// `SceneEdit`.
    Edit(Box<crate::scene_edit::Edit>),
    /// `SceneStartDaily`.
    StartDaily(crate::daily::StartDaily),
    /// `SceneDaily` (menu and results).
    Daily(crate::daily::Daily),
    /// `SceneCharacterSelect`.
    CharaSelect(crate::charselect::CharaSelect),
    /// `SceneYotogi` (stays while the ADV KAG runs between its screens).
    Yotogi(Box<crate::yotogi::Yotogi>),
    /// `SceneMaidManagement`.
    MaidManagement(Box<crate::maid_management::MaidManagement>),
    /// `SceneShop`.
    Shop(Box<crate::shop::Shop>),
    /// `SceneTrophy`.
    Trophy(Box<crate::trophy::SceneTrophy>),
    /// `SceneStaffRoll`.
    StaffRoll(Box<crate::staffroll::StaffRoll>),
}

/// `TitleMgr.ButtonType` in enum order.
pub const TITLE_BUTTONS: [&str; 4] = ["Start", "Continue", "Config", "End"];
/// `TitleCtrl` constants.
const BUTTON_START_X: f32 = -600.;
const BUTTON_MOVE_S: f32 = 0.7;
const BUTTON_INTERVAL_S: f32 = 0.3;

/// The `Start()` of the scene's controller script.
pub fn start(game: &mut Game, opener: &mut Opener, scene: &str) -> Result<Active> {
    Ok(match scene {
        "_cm3d2" => {
            // SceneFirst: FadeOut(0) then wait for CharacterMgr.
            game.fade_to(1., 0.);
            Active::First
        }
        "SceneLogo" | "SceneWarning" => {
            game.set_main_active("__GameMain__/SystemUI Root/SystemShortcut", false)?;
            game.fade_to(1., 0.);
            game.fade_to(0., 1.);
            if scene == "SceneLogo" {
                Active::Logo(Mode::FadeIn)
            } else {
                Active::Warning(Mode::FadeIn)
            }
        }
        // SceneToTitle.Start: FadeOut(0), Deserialize(-1); the restored
        // script calls the title scene.
        "SceneToTitle" => {
            game.fade_to(1., 0.);
            game.load_slot(opener, -1)?;
            Active::Adv
        }
        // SceneSingleEffect.Start: registers `startmvpeffect`, then Exec.
        "SceneSingleEffect" => {
            game.adv.exec();
            Active::SingleEffect
        }
        "SceneADV" => {
            game.adv.enabled = true;
            game.adv.exec();
            Active::Adv
        }
        "SceneTitle" => {
            // TitleMgr.Init: FadeOut(0), FadeIn(3), buttons start at x=-600.
            game.set_main_active("__GameMain__/SystemUI Root/SystemShortcut", true)?;
            // MainCamera.SetTargetPos/SetDistance/SetAroundAngle of TitleMgr.Init.
            let mut camera = crate::bg::orbit_camera([-0.4801364, 3.612486, 21.63433], [161.3175, -1.490669], 8.699998, game.fov);
            camera["op"] = serde_json::json!("set_camera");
            game.world.push(camera);
            // SoundMgr.PlayBGM("BGM001.ogg", 0).
            game.sound_ops.push(crate::sound::SoundOp::Play { kind: crate::sound::Kind::Bgm, file: "BGM001.ogg".into(), fade_s: 0., looping: true });
            game.fade_to(1., 0.);
            game.fade_to(0., 3.);
            let level = game.level.as_mut().context("title scene")?;
            for (i, name) in TITLE_BUTTONS.iter().enumerate() {
                let group = level
                    .scene
                    .find(&format!("UI Root/TitlePanel/ButtonGroup/{name}Group"))
                    .with_context(|| format!("title button group {name}"))?;
                let base = level.scene.nodes[group].position;
                level.states.entry(group).or_default().position = Some([BUTTON_START_X, base[1], base[2]]);
                let button = level.scene.child(group, name).context("title button")?;
                level.states.entry(button).or_default().click = Some("TitleMgr.ClickButton".into());
                let cursor = level.scene.child(button, "SelectCursor");
                if let Some(c) = cursor {
                    level.states.entry(c).or_default().active = Some(i == 0);
                }
            }
            // TitleMgr: Serialize(-1, "タイトルへ戻る") for SceneToTitle.
            game.save_slot(opener, -1, "タイトルへ戻る")?;
            Active::Title(Title { started_ms: game.now, active: 0, hovered: None, pointer: None, closing: None, panel: TitlePanel::Shown })
        }
        // SATD-cm3d2-game-logic: remaining scene controllers.
        "SceneUserEdit" => {
            // UserEditAPI.Init.
            game.fade_to(1., 0.);
            game.adv_ui(crate::kag::AdvUi::Window(false), opener)?;
            game.camera = ([0.08306749, 1.299064, 0.0249091], 2.2, [177.9044, 2.546583]);
            game.sound_ops.push(crate::sound::SoundOp::Play { kind: crate::sound::Kind::Bgm, file: "BGM009.ogg".into(), fade_s: 5., looping: true });
            game.adv_ui(crate::kag::AdvUi::SetBg("Shitsumu".into()), opener)?;
            let storage = opener.storage.clone();
            let mut heads = Vec::new();
            for path in game.files.menu_files.clone() {
                if !path.to_ascii_lowercase().contains("/man/") {
                    continue;
                }
                let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
                let menu = crate::menu::MenuFile::parse(&game.files.read(storage.as_ref(), &name)?)?;
                // SceneEdit.GetMenuItemSetUP: man items with an icon, MPN head.
                let man = menu.first("メニューフォルダ").is_some_and(|f| f.eq_ignore_ascii_case("man"));
                let icon = menu.first("icon").or_else(|| menu.first("icons"));
                if man && icon.is_some() && menu.category.eq_ignore_ascii_case("head") {
                    heads.push(name);
                }
            }
            // DeactivateCharaAll; man 0 visible, posture man_porse01.
            for c in &mut game.characters {
                c.visible = false;
            }
            let now = game.now;
            if let Some(man) = game.man(0) {
                man.visible = true;
                man.position = [0.; 3];
                man.rotation = [0.; 3];
                man.cross_fade("man_porse01.anm", true, false, 0.5, now, f32::MAX);
                man.chinko_visible = false;
            }
            // SetTargetOffset(Screen.width / 3 · 2 − Screen.width / 2).
            game.camera_offset = crate::ngui::WIDTH / 6.;
            game.all_pos = [0.; 3];
            game.all_rot = [0.; 3];
            game.fade_to(0., 1.);
            Active::UserEdit(UserEdit { heads, open: false, closing: None, leaving: false })
        }
        "SceneEdit" => Active::Edit(Box::new(crate::scene_edit::start(game, opener)?)),
        "SceneStartDaily" => Active::StartDaily(crate::daily::start_daily(game, opener)?),
        "SceneDaily" => Active::Daily(crate::daily::start_daily_scene(game, opener)?),
        "SceneCharacterSelect" => Active::CharaSelect(crate::charselect::start(game, opener)?),
        "SceneYotogi" => Active::Yotogi(Box::new(crate::yotogi::start(game, opener)?)),
        "SceneStaffRoll" => Active::StaffRoll(Box::new(crate::staffroll::start(game, opener)?)),
        "SceneTrophy" => Active::Trophy(Box::new(crate::trophy::start(game, opener)?)),
        "SceneShop" => Active::Shop(Box::new(crate::shop::start(game, opener)?)),
        "SceneMaidManagement" => Active::MaidManagement(Box::new(crate::maid_management::start(game, opener)?)),
        // SATD-cm3d2-game-logic: remaining scene controllers.
        other => bail!("CM3D2 scene {other} has no ported controller"),
    })
}

/// iTween `easeOutExpo` (the iTween default ease type).
pub(crate) fn ease_out_expo(t: f32) -> f32 {
    if t >= 1. {
        1.
    } else {
        1. - 2f32.powf(-10. * t)
    }
}

/// The `Update()` of the active controller.
pub fn update(game: &mut Game, opener: &mut Opener) -> Result<()> {
    let now = game.now;
    let click = game.any_click();
    let fade_done = game.fade.done(now);
    let next: Option<Active> = match game.active.clone() {
        Active::First => {
            game.pending_scene = Some("SceneLogo".into());
            None
        }
        Active::Logo(mode) => match mode {
            Mode::FadeIn if fade_done => Some(Active::Logo(Mode::ClickWait)),
            Mode::ClickWait if click => {
                game.play_system(crate::sound::SeType::IClick);
                game.fade_to(1., 1.);
                Some(Active::Logo(Mode::FadeOut))
            }
            Mode::FadeOut if fade_done => {
                game.pending_scene = Some("SceneWarning".into());
                None
            }
            _ => None,
        },
        Active::Warning(mode) => match mode {
            Mode::FadeIn if fade_done => Some(Active::Warning(Mode::ClickWait)),
            // CharacterMgr.LoadDefault: the default maid presets load here;
            // see the crate's SATD record for the character manager.
            Mode::ClickWait if click => {
                game.play_system(crate::sound::SeType::IClick);
                Some(Active::Warning(Mode::Loading))
            }
            Mode::Loading => {
                // CharacterMgr.LoadDefault, then VisibleAll(false) once loaded.
                game.load_default_characters(opener)?;
                game.visible_all(false);
                game.fade_to(1., 1.);
                Some(Active::Warning(Mode::FadeOut))
            }
            Mode::FadeOut if fade_done => {
                let storage = opener.storage.clone();
                game.adv.load(&game.files, storage.as_ref(), "AllMain.ks", "")?;
                game.adv.exec();
                Some(Active::Adv)
            }
            _ => None,
        },
        Active::Adv | Active::SingleEffect => None,
        Active::Shop(mut s) => {
            crate::shop::update(game, opener, &mut s)?;
            match &game.active {
                Active::Shop(_) => Some(Active::Shop(s)),
                _ => None,
            }
        }
        Active::StaffRoll(mut t) => {
            crate::staffroll::update(game, opener, &mut t)?;
            match &game.active {
                Active::StaffRoll(_) => Some(Active::StaffRoll(t)),
                _ => None,
            }
        }
        Active::Trophy(mut t) => {
            crate::trophy::update(game, opener, &mut t)?;
            match &game.active {
                Active::Trophy(_) => Some(Active::Trophy(t)),
                _ => None,
            }
        }
        Active::MaidManagement(mut m) => {
            crate::maid_management::update(game, opener, &mut m)?;
            match &game.active {
                Active::MaidManagement(_) => Some(Active::MaidManagement(m)),
                _ => None,
            }
        }
        Active::Yotogi(mut y) => {
            crate::yotogi::update(game, opener, &mut y)?;
            match &game.active {
                // A KAG request changed the scene's queued calls meanwhile.
                Active::Yotogi(_) => Some(Active::Yotogi(y)),
                _ => None,
            }
        }
        Active::CharaSelect(mut s) => {
            crate::charselect::update(game, opener, &mut s)?;
            if game.active == Active::Adv { None } else { Some(Active::CharaSelect(s)) }
        }
        Active::Daily(mut d) => {
            crate::daily::update_daily_scene(game, opener, &mut d)?;
            if game.active == Active::Adv { None } else { Some(Active::Daily(d)) }
        }
        Active::StartDaily(mut s) => {
            crate::daily::update_start_daily(game, opener, &mut s)?;
            if game.active == Active::Adv { None } else { Some(Active::StartDaily(s)) }
        }
        Active::Edit(mut e) => {
            crate::scene_edit::update(game, opener, &mut e)?;
            Some(Active::Edit(e))
        }
        Active::UserEdit(mut u) => {
            if !u.open {
                // UserEditMgr.OpenUserEditPanel → UserEditCtrl.CreateViewer.
                open_user_edit(game, opener, &u)?;
                u.open = true;
            }
            for (si, node, handler) in std::mem::take(&mut game.events) {
                match handler.as_str() {
                    "UserEditMgr.ClickPreset" => {
                        let name = game.scene_mut(si).map(|l| l.scene.nodes[node].name.clone()).unwrap_or_default();
                        let storage = opener.storage.clone();
                        let files = game.files.clone();
                        if let Some(man) = game.man(0) {
                            // UserEditMgr.ClickPreset: SetProp(head) + AllProcProp.
                            if man.file("head").map(|m| !m.eq_ignore_ascii_case(&name)).unwrap_or(true) {
                                man.set_prop("head", &name);
                                man.all_proc_prop(&files, storage.as_ref())?;
                            }
                        }
                    }
                    "UserEditMgr.CloseUserEditPanel" if u.closing.is_none() => u.closing = Some(now),
                    "UserEditMgr.OnChangeName" => {
                        let name = game.scene_mut(si).and_then(|l| l.states.get(&node).and_then(|s| s.text.clone())).unwrap_or_default();
                        game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.runtime.host_mut().state.player.player_name = name;
                    }
                    "UserEditMgr.OnColorValueChange" => {
                        let level = game.level.as_ref().context("user edit")?;
                        let v = |c: &str| level.scene.find(&format!("UI Root/UserEditPanel/Color/{c}/Slider")).and_then(|n| level.states.get(&n).and_then(|s| s.value)).unwrap_or(1.);
                        let color = [v("R"), v("G"), v("B"), 1.];
                        if let Some(man) = game.man(0) {
                            man.set_man_color(color);
                        }
                    }
                    "UserEditMgr.OnAbdomenValueChange" => {
                        let level = game.level.as_mut().context("user edit")?;
                        let Some(slider) = level.scene.find("UI Root/UserEditPanel/Abdomen/Slider") else { continue };
                        let v = level.states.get(&slider).and_then(|s| s.value).unwrap_or(0.);
                        // ToLabelValue: round to 2 places, × 100.
                        let rounded = (v * 100.).round() / 100.;
                        if let Some(n) = level.scene.find("UI Root/UserEditPanel/Abdomen/Value/Number") {
                            level.states.entry(n).or_default().text = Some(((rounded * 100.) as i32).to_string());
                        }
                        if let Some(man) = game.man(0) {
                            // MaidProp Hara min 0..max 100 (SATD-cm3d2-game-logic: bone morph).
                            man.set_value("Hara", (rounded * 100. + 0.5) as i32)?;
                        }
                    }
                    other => game.log.push(format!("UserEdit event {other} not ported")),
                }
            }
            if let Some(since) = u.closing {
                // BasePanelMgr fade-out (0.5 s), then EndNextScene FadeOut(1).
                let level = game.level.as_mut().context("user edit")?;
                if let Some(panel) = level.scene.find("UI Root/UserEditPanel") {
                    level.states.entry(panel).or_default().alpha = Some(1. - ((now - since) as f32 / 500.).min(1.));
                }
                if !u.leaving && now >= since + 500 {
                    game.fade_to(1., 1.);
                    u.leaving = true;
                } else if u.leaving && fade_done {
                    let label = game.adv.tag_backup.get("label").cloned().context("SceneCall label for user edit")?;
                    let storage = opener.storage.clone();
                    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                    game.adv.exec();
                    // UserEditAPI.Uninit.
                    if let Some(man) = game.man(0) {
                        man.visible = false;
                        man.chinko_visible = true;
                    }
                    game.camera_offset = 0.;
                    u.closing = None;
                    game.active = Active::UserEdit(u);
                    return Ok(());
                }
            }
            Some(Active::UserEdit(u))
        }
        Active::Title(mut t) => {
            let elapsed = (now - t.started_ms) as f32 / 1000.;
            let level = game.level.as_mut().context("title scene")?;
            for (i, name) in TITLE_BUTTONS.iter().enumerate() {
                let Some(group) = level.scene.find(&format!("UI Root/TitlePanel/ButtonGroup/{name}Group")) else { continue };
                let base = level.scene.nodes[group].position;
                let local = ((elapsed - i as f32 * BUTTON_INTERVAL_S) / BUTTON_MOVE_S).clamp(0., 1.);
                let x = BUTTON_START_X * (1. - ease_out_expo(local));
                level.states.entry(group).or_default().position = Some([x, base[1], base[2]]);
            }
            // TitleCtrl.OnContinue → CloseTitlePanel(OpenLoadPanel).
            match t.panel.clone() {
                TitlePanel::Shown => {}
                TitlePanel::FadeOut(since) | TitlePanel::FadeIn(since) => {
                    let f = ((now - since) as f32 / 500.).min(1.);
                    let out = matches!(t.panel, TitlePanel::FadeOut(_));
                    let panel = level.scene.find("UI Root/TitlePanel").context("TitlePanel")?;
                    level.states.entry(panel).or_default().alpha = Some(if out { 1. - f } else { f });
                    if f >= 1. {
                        if out {
                            level.states.entry(panel).or_default().active = Some(false);
                            t.panel = TitlePanel::Load(Box::new(crate::saveload::open(game, opener, crate::saveload::View::Load, true)?));
                        } else {
                            t.panel = TitlePanel::Shown;
                        }
                    }
                    game.active = Active::Title(t);
                    return Ok(());
                }
                TitlePanel::Load(mut s) => {
                    match crate::saveload::update(game, opener, &mut s)? {
                        crate::saveload::Outcome::Running => t.panel = TitlePanel::Load(s),
                        // CallbackAfterFadeOut = OpenTitlePanel.
                        crate::saveload::Outcome::Closed => {
                            let level = game.level.as_mut().context("title scene")?;
                            let panel = level.scene.find("UI Root/TitlePanel").context("TitlePanel")?;
                            let st = level.states.entry(panel).or_default();
                            st.active = Some(true);
                            st.alpha = Some(0.);
                            t.panel = TitlePanel::FadeIn(now);
                        }
                        crate::saveload::Outcome::Loaded => t.panel = TitlePanel::Shown,
                    }
                    game.active = Active::Title(t);
                    return Ok(());
                }
            }
            if let Some(since) = t.closing {
                // TweenAlpha.Begin(panel, 3, 0) while FadeOut(3) runs.
                if let Some(panel) = level.scene.find("UI Root/TitlePanel") {
                    level.states.entry(panel).or_default().alpha = Some(1. - ((now - since) as f32 / 3000.).min(1.));
                }
                if fade_done {
                    // SceneMgr.CloseScene: jump to the SceneCall label.
                    let label = game.adv.tag_backup.get("label").cloned().context("SceneCall label for the title")?;
                    let storage = opener.storage.clone();
                    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                    game.adv.exec();
                    game.active = Active::Title(Title { closing: None, ..t });
                    return Ok(());
                }
            } else {
                // TitleMouseOver.OnHover(true) follows UICamera's topmost collider.
                let hover = crate::uiev::pick(&game.ui.hits, game.input.pointer_x as f32, game.input.pointer_y as f32)
                    .filter(|hit| hit.scene == 1 && level.states.get(&hit.node).and_then(|state| state.click.as_deref()) == Some("TitleMgr.ClickButton"))
                    .and_then(|hit| TITLE_BUTTONS.iter().position(|name| level.scene.nodes[hit.node].name == *name));
                let mut activated = None;
                game.events.retain(|(scene,node,handler)| {
                    if *scene == 1 && handler == "TitleMgr.ClickButton" {
                        activated = TITLE_BUTTONS.iter().position(|name| level.scene.nodes[*node].name == *name);
                        false
                    } else {
                        true
                    }
                });
                let pointer = [game.input.pointer_x, game.input.pointer_y];
                let pointer_selection = (hover != t.hovered || t.pointer != Some(pointer)).then_some(hover).flatten();
                t.hovered = hover;
                t.pointer = Some(pointer);
                let available = TITLE_BUTTONS.map(|name| game.ui.hits.iter().any(|hit|
                    hit.scene == 1 && level.scene.nodes[hit.node].name == name &&
                    level.states.get(&hit.node).and_then(|s| s.click.as_deref()) == Some("TitleMgr.ClickButton")));
                let keys = game.input.buttons_pressed;
                use crysirror_core::service::buttons;
                let has_modal = game.main.scene.find("__GameMain__/SystemUI Root/SystemDialog").is_some_and(|n|
                    game.main.states.get(&n).and_then(|s| s.active).unwrap_or(game.main.scene.nodes[n].active));
                let accepts_keys = !game.cm_system.open && !has_modal && game.input_focus.is_none();
                let movement = if accepts_keys {
                    match (keys & buttons::UP != 0, keys & buttons::DOWN != 0) {
                        (true, false) => -1, (false, true) => 1, _ => 0,
                    }
                } else { 0 };
                let focus = pointer_selection.unwrap_or(t.active);
                let selection = if movement != 0 {
                    (1..=TITLE_BUTTONS.len()).map(|step|
                        (focus as isize + movement * step as isize).rem_euclid(TITLE_BUTTONS.len() as isize) as usize
                    ).find(|&index| available[index])
                } else { pointer_selection };
                let focused = selection.unwrap_or(focus);
                if accepts_keys && keys & buttons::ENTER != 0 && available[focused] && activated.is_none() {
                    activated = Some(focused);
                }
                let selection = activated.or(selection);
                if let Some(selected) = selection {
                    t.active = selected;
                    for (i, name) in TITLE_BUTTONS.iter().enumerate() {
                        if let Some(c) = level.scene.find(&format!("UI Root/TitlePanel/ButtonGroup/{name}Group/{name}/SelectCursor")) {
                            level.states.entry(c).or_default().active = Some(i == selected);
                        }
                    }
                }
                if let Some(button) = activated {
                    let button_node = level.scene.find(&format!("UI Root/TitlePanel/ButtonGroup/{0}Group/{0}", TITLE_BUTTONS[button]));
                    if let Some(f) = button_node.and_then(|n| level.scene.script(n, "UIButton")) {
                        if let Some(file) = crate::sound::button_click(f.int("clickMouseSe").unwrap_or(0), f.string("clickMouseSeFile").unwrap_or("")) {
                            game.sound_ops.push(crate::sound::SoundOp::Play { kind: crate::sound::Kind::System, file, fade_s: 0., looping: false });
                        }
                    }
                    match TITLE_BUTTONS[button] {
                        "Start" => {
                            game.fade_to(1., 3.);
                            t.closing = Some(now);
                        }
                        "Continue" => t.panel = TitlePanel::FadeOut(now),
                        // TitleCtrl.OnConfig: ConfigMgr.OpenConfigPanel.
                        "Config" => game.open_config()?,
                        // TitleCtrl.OnEnd: GameMain.ToApplicationQuit.
                        _ => game.quit_requested = true,
                    }
                }
            }
            Some(Active::Title(t))
        }
    };
    if let Some(next) = next {
        game.active = next;
    }
    Ok(())
}

/// `UserEditCtrl.CreateViewer`: head preset units in their table, initial
/// values, Ok button, panel fade-in.
fn open_user_edit(game: &mut Game, opener: &mut Opener, u: &UserEdit) -> Result<()> {
    let prefab_key = game
        .unity
        .resource_of(opener, "SceneUserEdit/Prefab/HeadPresetUnit", crysirror_unity::class_id::GAME_OBJECT)?
        .context("Resources SceneUserEdit/Prefab/HeadPresetUnit")?;
    let prefab = game.unity.prefab(opener, &prefab_key)?;
    let storage = opener.storage.clone();
    let mut icons = Vec::new();
    for head in &u.heads {
        let menu = crate::menu::MenuFile::parse(&game.files.read(storage.as_ref(), head)?)?;
        icons.push(menu.first("icon").or_else(|| menu.first("icons")).unwrap_or("").to_owned());
    }
    let level = game.level.as_mut().context("user edit level")?;
    let parent = level.scene.find("UI Root/UserEditPanel/Head/HeadPresetUnitParent").context("HeadPresetUnitParent")?;
    level.scene.nodes[parent].children.clear();
    let mut created = Vec::new();
    for (head, icon) in u.heads.iter().zip(icons) {
        let node = level.scene.instantiate(&prefab, prefab.roots[0], Some(parent));
        level.scene.nodes[node].name = head.clone();
        level.scene.nodes[node].position = [0.; 3];
        level.scene.nodes[node].scale = [1.; 3];
        level.states.entry(node).or_default().click = Some("UserEditMgr.ClickPreset".into());
        created.push((node, icon));
    }
    for (node, pos) in crate::ngui::table_positions(&level.scene, &level.states, parent) {
        level.states.entry(node).or_default().position = Some(pos);
    }
    let ok = level.scene.find("UI Root/UserEditPanel/Ok").context("Ok")?;
    level.states.entry(ok).or_default().click = Some("UserEditMgr.CloseUserEditPanel".into());
    let panel = level.scene.find("UI Root/UserEditPanel").context("UserEditPanel")?;
    level.states.entry(panel).or_default().active = Some(true);
    // InitData: name, fat 0.5 (Hara 50 of 0..100), colour from ManColor.
    let name = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.runtime.host().state.player.player_name.clone();
    if let Some(input) = level.scene.find("UI Root/UserEditPanel/Name/InputField") {
        level.states.entry(input).or_default().text = Some(name.clone());
        if let Some(l) = level.scene.child(input, "Label") {
            level.states.entry(l).or_default().text = Some(name);
        }
    }
    // InitData: GetFat = (Hara − min)/(max − min), ManColor r/g/b.
    let (fat, color) = game.man(0).map(|m| (m.value("Hara") as f32 / 100., m.man_color)).unwrap_or((0.5, [0., 0.09, 1., 1.]));
    let level = game.level.as_mut().context("user edit level")?;
    let mut set_slider = |path: &str, v: f32| {
        let Some(n) = level.scene.find(path) else { return };
        level.states.entry(n).or_default().value = Some(v);
        let f = level.scene.script(n, "UISlider").cloned().unwrap_or_default();
        let bar = level.scene.script(n, "UISprite").cloned().unwrap_or_default();
        let w = bar.int("mWidth").unwrap_or(0) as f32;
        let px = crate::ngui::pivot_x(bar.int("mPivot").unwrap_or(4));
        if let Some(t) = f.pptr("thumb").and_then(|t| level.scene.find_ref(n, t.path_id)) {
            let p = level.scene.nodes[t].position;
            level.states.entry(t).or_default().position = Some([(v - px) * w, p[1], p[2]]);
        }
    };
    set_slider("UI Root/UserEditPanel/Abdomen/Slider", fat);
    set_slider("UI Root/UserEditPanel/Color/R/Slider", color[0]);
    set_slider("UI Root/UserEditPanel/Color/G/Slider", color[1]);
    set_slider("UI Root/UserEditPanel/Color/B/Slider", color[2]);
    if let Some(n) = level.scene.find("UI Root/UserEditPanel/Abdomen/Value/Number") {
        level.states.entry(n).or_default().text = Some((((fat * 100.).round() / 100. * 100.) as i32).to_string());
    }
    for (node, icon) in created {
        if !icon.is_empty() {
            game.pending_textures.push((1, node, icon));
        }
    }
    Ok(())
}
