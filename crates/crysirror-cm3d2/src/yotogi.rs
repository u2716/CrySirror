// SPDX-License-Identifier: MPL-2.0
//! `SceneYotogi`: `YotogiManager` over the `WfScreen` framework with its
//! children `YotogiStageSelectManager`, `YotogiSkillSelectManager`,
//! `YotogiPlayManager`, `YotogiResultManager` and `YotogiNullManager`, and
//! the views they drive (`YotogiParameterViewer`, `YotogiParamBasicBar`,
//! `YotogiParamScroll`, `YotogiSkillContainerViewer`, `YotogiSkillUnit`,
//! `YotogiCommandFactory`, `UndressingUnit`).
use crate::{
    adapter::{Game, Opener},
    gamedata::GameData,
    kag::{AdvUi, EyeLook},
    maid_param::{MaidParam, PlayerParam},
    params::MaidStatus,
    sound::{Kind, SoundOp},
    tjs::TjsState,
    wf::{self, Child, FadeStatus, Manager, Screens},
    yotogi_data::*,
};
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;

const STAGE: &str = "UI Root/StageSelectPanel";
const SKILL: &str = "UI Root/SkillSelectPanel";
const PLAY: &str = "UI Root/YotogiPlayPanel";
const RESULT: &str = "UI Root/ResultPanel";
const NULL: &str = "UI Root/NullPanel";
const PARAMETER_VIEWER: &str = "SceneYotogi/Common/Prefab/ParameterViewer";
const SKILL_GROUP: &str = "UI Root/YotogiPlayPanel/CommandViewer/SkillViewer/MaskGroup/SkillGroup";
const BASIC_BAR: &str = "UI Root/YotogiPlayPanel/StatusViewer/ParameterViewer/MaskGroup/ParameterParent";
const PARAM_VIEW: &str = "UI Root/YotogiPlayPanel/CommandViewer/SkillViewer/MaskGroup/ParamView";
const UNDRESS: &str = "UI Root/YotogiPlayPanel/UndressingViewer/UndressingViewer/MaskGroup/UndressParent";
const MAN_ALPHA: &str = "UI Root/YotogiPlayPanel/UndressingViewer/UndressingViewer/MaskGroup/PlayerModelTransparency/Slider";
const CONTROL_MASKS: [&str; 2] = ["UI Root/YotogiPlayPanel/CommandViewer/SkillViewer/ControlMaskt", "UI Root/YotogiPlayPanel/UndressingViewer/UndressingViewer/ControlMaskt"];
/// `kSkillMaxSlot`.
const SLOTS: usize = 7;
/// `YotogiPlayManager.kRepeatWaitTime` (ms).
const REPEAT_WAIT_MS: i32 = 200;
/// `YotogiResultManager.AnimeTime`.
const RESULT_ANIME_S: f32 = 1.5;
/// `YotogiParamScroll.WaitTime` / `MoveSpeedTime`.
const SCROLL_WAIT_MS: u64 = 2500;
const SCROLL_MOVE_S: f32 = 0.7;
/// `YotogiSkillIcon.IconSize`.
const ICON_SIZE: f32 = 80.;

/// `YotogiPlay.Param`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayParam {
    pub cur_excite: i32,
    pub cur_mind: i32,
    pub cur_reason: i32,
    pub inyoku: i32,
    pub m_value: i32,
    pub hentai: i32,
    pub housi: i32,
    pub frustration: i32,
    pub lovely: i32,
    pub elegance: i32,
    pub charm: i32,
    pub reception: i32,
    pub teach_rate: i32,
    pub yotogi_type_level: i32,
    pub yotogi_type_total_exp: i32,
    pub sexual: [i32; 6],
}

impl PlayParam {
    /// `YotogiPlay.Param.Create`.
    pub fn create(st: &MaidStatus) -> Self {
        let class = st.yotogi_class.get(st.current_yotogi_class as usize);
        let s = &st.sexual;
        Self {
            cur_excite: st.a_cur_excite(),
            cur_mind: st.a_cur_mind(),
            cur_reason: st.a_cur_reason(),
            inyoku: st.a_inyoku(),
            m_value: st.a_m_value(),
            hentai: st.a_hentai(),
            housi: st.a_housi(),
            frustration: st.a_frustration(),
            lovely: st.a_lovely(),
            elegance: st.a_elegance(),
            charm: st.a_charm(),
            reception: st.a_reception(),
            teach_rate: st.a_teach_rate(),
            yotogi_type_level: class.map_or(0, |c| c.level()),
            yotogi_type_total_exp: class.map_or(0, |c| c.exp.total),
            sexual: [s.mouth, s.throat, s.nipple, s.front, s.back, s.curi],
        }
    }

    fn sub(self, b: Self) -> Self {
        let mut sexual = [0; 6];
        for (i, v) in sexual.iter_mut().enumerate() {
            *v = self.sexual[i] - b.sexual[i];
        }
        Self {
            cur_excite: self.cur_excite - b.cur_excite,
            cur_mind: self.cur_mind - b.cur_mind,
            cur_reason: self.cur_reason - b.cur_reason,
            inyoku: self.inyoku - b.inyoku,
            m_value: self.m_value - b.m_value,
            hentai: self.hentai - b.hentai,
            housi: self.housi - b.housi,
            frustration: self.frustration - b.frustration,
            lovely: self.lovely - b.lovely,
            elegance: self.elegance - b.elegance,
            charm: self.charm - b.charm,
            reception: self.reception - b.reception,
            teach_rate: self.teach_rate - b.teach_rate,
            yotogi_type_level: self.yotogi_type_level - b.yotogi_type_level,
            yotogi_type_total_exp: self.yotogi_type_total_exp - b.yotogi_type_total_exp,
            sexual,
        }
    }
}

/// `Yotogi.ExcitementStatus` of `YotogiPlay.GetExcitementStatus`.
pub fn excitement_status(excite: i32) -> usize {
    match excite {
        e if e < 0 => 0,
        e if e < 100 => 1,
        e if e < 200 => 2,
        _ => 3,
    }
}

/// `YotogiPlay.GetRCType` as the `Yotogi.RCType` name (`None` = `RCNull`).
pub fn rc_type(before: i32, current: i32) -> Option<&'static str> {
    if 0 <= before && current <= -1 {
        Some("RC_1")
    } else if before <= 199 && 200 <= current {
        Some("RC2")
    } else if before <= 99 && 100 <= current {
        Some("RC1")
    } else if before <= -1 && 0 <= current {
        Some("RC0")
    } else {
        None
    }
}

/// `Yotogi.RRCType` names from `RRC_4` (index 1) to `RRC10`.
const RRC: [&str; 15] = ["RRCNull", "RRC_4", "RRC_3", "RRC_2", "RRC_1", "RRC1", "RRC2", "RRC3", "RRC4", "RRC5", "RRC6", "RRC7", "RRC8", "RRC9", "RRC10"];

/// `YotogiPlay.GetRRCType`.
pub fn rrc_type(before: i32, current: i32) -> Option<&'static str> {
    if before == current {
        match current {
            -100 => return Some("RRC_4"),
            300 => return Some("RRC10"),
            _ => {}
        }
    }
    let mut num = if before >= current { -100 } else { 300 };
    let step = if before >= current { 25 } else { -25 };
    let mut t: i32 = if num != 300 { 1 } else { 14 };
    for _ in 0..16 {
        if num == 0 || num == 100 || num == 200 {
            num += step;
        }
        if 0 < num {
            if before < num && num <= current {
                return RRC.get(t as usize).copied();
            }
        } else if (before <= num && num < current) || (current <= num && num < before) {
            return RRC.get(t as usize).copied();
        }
        num += step;
        t += step / step.abs();
    }
    None
}

/// `YotogiPlay.GetRRType` as the `Yotogi.RRType` name.
pub fn rr_type(current: i32) -> &'static str {
    let mut n = current.abs() / 50;
    if current < 0 {
        n = -(n + 1).min(2);
    } else {
        n = n.min(5);
    }
    ["RRNull", "RR_2", "RR_1", "RR1", "RR2", "RR3", "RR4", "RR5", "RR6"][(3 + n) as usize]
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PlayingSkill {
    skill: Option<i32>,
    exp: i32,
    backup_total_exp: i32,
    is_play: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum TweenKind {
    Scale { from: [f32; 3], to: [f32; 3] },
    Position { from: [f32; 3], to: [f32; 3] },
    /// `YotogiParamScroll` move with its `oncomplete` callback.
    Scroll { from: [f32; 3], to: [f32; 3] },
    /// `FluctuationOfParameterUnit.UpdateValue` on the label.
    Count { from: i32, to: i32 },
    /// `YotogiSkillUnit.UpdateValue`: total exp of the skill on the unit.
    SkillExp { skill: i32, from: i32, to: i32, level: i32 },
}

#[derive(Clone, Debug, PartialEq)]
struct Tween {
    node: usize,
    start: u64,
    seconds: f32,
    ease: Ease,
    kind: TweenKind,
}

/// iTween ease types used here (`easeOutExpo` is iTween's default).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ease {
    OutSine,
    OutQuint,
    OutExpo,
}

impl Ease {
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::OutSine => ease_out_sine(t),
            Ease::OutQuint => ease_out_quint(t),
            Ease::OutExpo => ease_out_expo(t),
        }
    }
}

fn ease_out_sine(t: f32) -> f32 {
    (t * std::f32::consts::FRAC_PI_2).sin()
}
fn ease_out_quint(t: f32) -> f32 {
    1. - (1. - t).powi(5)
}
/// iTween's default `easeOutExpo`.
fn ease_out_expo(t: f32) -> f32 {
    if t >= 1. { 1. } else { 1. - 2f32.powf(-10. * t) }
}

/// `YotogiParamScroll.State`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Scroll {
    #[default]
    Null,
    CallIn,
    FadeIn(u64),
    Wait(u64),
    FadeOut(u64),
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Play {
    next_flag: bool,
    is_reset_item: bool,
    is_swoon: bool,
    zettyo_count: i32,
    repeat_voice: String,
    repeat_time: i32,
    level: i32,
    no: i32,
    insert: bool,
    backup: PlayParam,
    played: BTreeSet<i32>,
    /// Valid command indices for Normal / Insert.
    valid: [Vec<usize>; 2],
    /// Command factory: (group title node, [(command node, command index)]).
    groups: Vec<(String, usize, Vec<(usize, usize)>)>,
    control_mask_until: Option<u64>,
    scroll: Scroll,
    /// `UndressingUnit.is_undressing_` per unit node.
    undressing: Vec<(usize, String, bool)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct State {
    /// `GetMaid(0)` character index.
    maid: usize,
    next_label: String,
    edit_label: String,
    skills: [PlayingSkill; SLOTS],
    backup_param: PlayParam,
    backup_skills: Vec<i32>,
    backup_propensity: Vec<i32>,
    /// `TagYotogiCall`s waiting for this frame's update.
    calls: Vec<(String, String, Option<String>)>,
    stage_units: Vec<(usize, usize)>,
    stage: usize,
    /// ParameterViewer prefab copies of SkillSelect, Play, Result.
    viewers: [usize; 3],
    /// Category button node and its skill ids.
    categories: Vec<(usize, Vec<i32>)>,
    skill_units: Vec<(usize, i32)>,
    slots: [Option<i32>; SLOTS],
    play: Play,
    tweens: Vec<Tween>,
    dialog: Option<&'static str>,
    edit_requested: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Yotogi {
    wf: Manager,
    s: State,
}

fn lock<R>(game: &Game, f: impl FnOnce(&mut TjsState, &GameData) -> Result<R>) -> Result<R> {
    let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
    let state = &mut tjs.runtime.host_mut().state;
    let data = state.data()?;
    f(state, &data)
}

fn maid_status(game: &Game, maid: usize) -> Result<MaidStatus> {
    lock(game, |s, _| s.maids.get(&maid).cloned().context("yotogi maid parameters"))
}

/// A `MaidParam` operation on the yotogi maid.
fn with_param<R>(game: &Game, maid: usize, f: impl FnOnce(&mut MaidParam, &mut PlayerParam) -> Result<R>) -> Result<R> {
    lock(game, |s, data| {
        let st = s.maids.get_mut(&maid).context("yotogi maid parameters")?;
        let mut p = MaidParam { st, data };
        let mut pl = PlayerParam { st: &mut s.player, data };
        f(&mut p, &mut pl)
    })
}

fn eval(game: &Game, exp: &str) -> Result<crysirror_tjs2::runtime::Variant> {
    game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.eval(exp)
}

/// `ScriptManager.ReplacePersonal` for maid 0.
fn replace_personal(game: &Game, name: &str) -> Result<String> {
    if !name.contains('?') {
        return Ok(name.to_owned());
    }
    let personal = lock(game, |s, _| Ok(s.maid(0).map_or(0, |m| m.personal)))?;
    Ok(name.replace('?', ["C", "B", "A"].get(personal as usize).copied().unwrap_or("")))
}

/// `ScriptManager.LoadYotogiScript(file)`: enable, load and `Exec` now,
/// as the original runs the tags inside the call. Requests addressed to this
/// scene (`talkrepeat`, `controlmask`) wait in `Game::yotogi_ops` for the
/// end of the scene update.
fn load_yotogi_script(game: &mut Game, opener: &mut Opener, file: &str) -> Result<()> {
    let storage = opener.storage.clone();
    game.yotogi_kag.enabled = true;
    game.yotogi_kag.load(&game.files, storage.as_ref(), file, "")?;
    game.yotogi_kag.exec();
    game.yotogi_kag.update(game.now, false, false, &game.files, storage.as_ref())?;
    if std::mem::take(&mut game.yotogi_kag.voice_wait) {
        let len = game.voice.as_ref().map_or(0, |v| v.2);
        game.yotogi_kag.set_wait(game.now, len, true);
    }
    game.sound_ops.append(&mut game.yotogi_kag.sound);
    for op in std::mem::take(&mut game.yotogi_kag.ui) {
        match op {
            AdvUi::RepeatVoice(_) | AdvUi::ControlMask => game.yotogi_ops.push(op),
            op => game.adv_ui(op, opener)?,
        }
    }
    Ok(())
}

fn voice_stop_all(game: &mut Game) {
    game.sound_ops.push(SoundOp::Stop { kind: Kind::Voice, fade_s: 0. });
}

/// `MainLight.Reset`, `ResetCharaPosAll`, `MainCamera.Reset(Target)`,
/// `VoiceStopAll`.
fn reset_view(game: &mut Game, opener: &mut Opener) -> Result<()> {
    game.adv_ui(AdvUi::LightMain { rotation: None, intensity: None }, opener)?;
    game.adv_ui(AdvUi::AllResetPos, opener)?;
    game.adv_ui(AdvUi::CameraReset, opener)?;
    game.camera_control = true;
    voice_stop_all(game);
    Ok(())
}

fn data(game: &Game) -> Result<std::sync::Arc<GameData>> {
    lock(game, |s, _| s.data())
}

fn is_all_chara_busy(game: &Game) -> bool {
    game.active_maids.iter().chain(game.active_men.iter()).flatten().filter_map(|i| game.characters.get(*i)).any(|c| c.visible && c.busy())
}

/// `TweenHash` targets replace the running tween of the same kind.
fn tween(s: &mut State, now: u64, node: usize, seconds: f32, ease: Ease, kind: TweenKind) {
    let same = |a: &TweenKind, b: &TweenKind| std::mem::discriminant(a) == std::mem::discriminant(b);
    s.tweens.retain(|t| !(t.node == node && same(&t.kind, &kind)));
    s.tweens.push(Tween { node, start: now, seconds, ease, kind });
}

/// `SceneYotogi` start: `YotogiManager.Awake` + `Start`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<Yotogi> {
    let child = |name: &'static str, root: &str, fade: f32| Child { name, root: root.into(), fade_time: fade, status: FadeStatus::Null };
    let mut wf = Manager::new(game, vec![child("StageSelect", STAGE, 0.5), child("SkillSelect", SKILL, 0.5), child("Play", PLAY, 0.5), child("Result", RESULT, 0.5), child("Null", NULL, 0.)]);
    let maid = game.active_maids[0].context("SceneYotogi: no maid in slot 0")?;
    let mut s = State { maid, ..Default::default() };
    game.yotogi_kag.enabled = false;
    awake(game, opener, &mut s)?;
    // CreateBackUpStatus.
    let st = maid_status(game, maid)?;
    s.backup_param = PlayParam::create(&st);
    s.backup_propensity = st.propensity.iter().map(|(k, _)| *k).collect();
    let tag = game.adv.tag_backup.clone();
    let mut y = Yotogi { wf: wf.clone(), s };
    if tag.get("name").map(String::as_str) == Some("SceneYotogi") {
        let start = tag.get("start").context("SceneCallにstartの設定がされていませんでした")?.clone();
        let label = tag.get("label").context("SceneCallにlabelの設定がされていませんでした")?.clone();
        y.s.next_label = label;
        if start == "SkillSelect" {
            if let Some(e) = tag.get("edit_label") {
                y.s.edit_label = e.clone();
            }
        }
        anyhow::ensure!(["Null", "StageSelect", "SkillSelect", "Play", "Result"].contains(&start.as_str()), "[{start}]を特定できませんでした");
        wf.call_screen(game, opener, &mut y.s, &start)?;
    } else {
        wf.call_screen(game, opener, &mut y.s, "StageSelect")?;
    }
    y.wf = wf;
    Ok(y)
}

/// The `Awake`s: parameter viewers, category buttons, slot clicks.
fn awake(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    let set_pos = |game: &mut Game, n: usize, p: [f32; 3]| wf::state(game, n).position = Some(p);
    // YotogiSkillSelectManager.Awake.
    let root = wf::node(game, SKILL)?;
    let v = wf::create_prefab(game, opener, root, PARAMETER_VIEWER)?;
    set_pos(game, v, [782., 449., 0.]);
    s.viewers[0] = v;
    let costume = wf::child(game, v, "ChangeCostume")?;
    wf::state(game, costume).click = Some("YotogiSkillSelectManager.OnClickEdit".into());
    let parent = wf::node(game, &format!("{SKILL}/CategoryViewer/Content/CategoryUnitParent"))?;
    wf::clear_children(game, parent);
    for (i, name) in crate::gamedata::SKILL_CATEGORIES.iter().enumerate() {
        let obj = wf::create_prefab(game, opener, parent, "SceneYotogi/SkillSelect/Prefab/CategoryBtn")?;
        game.level.as_mut().unwrap().scene.nodes[obj].name = (*name).into();
        let label = wf::child(game, obj, "Label")?;
        wf::state(game, label).text = Some((*name).into());
        let button = wf::child(game, obj, "Button")?;
        wf::state(game, button).click = Some(format!("YotogiSkillSelectManager.OnClickFromCategoryButton:{i}"));
        s.categories.push((obj, Vec::new()));
    }
    wf::reposition(game, parent);
    wf::reset_scroll(game, &format!("{SKILL}/CategoryViewer/Content"))?;
    // YotogiSkillContainerViewer: DeselectAll, icon right clicks.
    wf::set_click(game, &format!("{SKILL}/SkillContainerViewer/DeselectAll"), "YotogiSkillSelectManager.OnClickAllReset")?;
    for i in 0..SLOTS {
        let p = format!("{SKILL}/SkillContainerViewer/SkillParent/SkillIconUnit{i}");
        wf::set_click(game, &format!("{p}/SkillIconDrag"), format!("YotogiSkillContainerViewer.OnRightClickIcon:{i}"))?;
        let n = wf::node(game, &format!("{p}/SkillName"))?;
        wf::state(game, n).alpha = Some(0.);
    }
    // YotogiPlayManager.Awake.
    let mask = wf::node(game, &format!("{PLAY}/ParameterViewer/YotogiParameterViewer/MaskGroup"))?;
    let v = wf::create_prefab(game, opener, mask, PARAMETER_VIEWER)?;
    set_cost_visible(game, v, false)?;
    s.viewers[1] = v;
    for i in 0..SLOTS {
        let unit = game.level.as_ref().unwrap().scene.nodes[wf::node(game, &format!("{SKILL_GROUP}/SkillParent"))?].children[i];
        let icon = wf::child(game, unit, "Icon")?;
        wf::state(game, icon).click = Some(format!("YotogiPlayManager.OnSkillIconClick:{i}"));
        let name = wf::child(game, unit, "SkillName")?;
        wf::state(game, name).alpha = Some(0.);
    }
    // YotogiResultManager.Awake.
    let rv = wf::node(game, &format!("{RESULT}/ParameterViewer"))?;
    let v = wf::create_prefab(game, opener, rv, PARAMETER_VIEWER)?;
    set_pos(game, v, [0., 360., 0.]);
    set_cost_visible(game, v, false)?;
    s.viewers[2] = v;
    wf::set_active(game, &format!("{RESULT}/CommonPanel/UnderButtonGroup/ViweReset"), false)?;
    // AttributeViewer starts hidden (SetAttributeViewer).
    wf::set_active(game, "UI Root/AttributeViewer", false)?;
    Ok(())
}

/// `YotogiParameterViewer.SetCostumeButtonVisible`.
fn set_cost_visible(game: &mut Game, viewer: usize, on: bool) -> Result<()> {
    let bg = wf::child(game, viewer, "BG")?;
    let width = game.level.as_ref().unwrap().scene.script(bg, "UISprite").and_then(|f| f.int("mWidth")).unwrap_or(0) as f32;
    wf::state(game, bg).size = Some([width, if on { 900. } else { 850. }]);
    let b = wf::child(game, viewer, "ChangeCostume")?;
    wf::state(game, b).active = Some(on);
    Ok(())
}

/// `YotogiParameterViewer.UpdateTextCommon` + `UpdateTextParam`
/// (`current`: the prefab's `CurrentValueDraw`, true).
fn update_viewer(game: &mut Game, viewer: usize, maid: usize, current: bool) -> Result<()> {
    let st = maid_status(game, maid)?;
    let class = st.yotogi_class.get(st.current_yotogi_class as usize).cloned().unwrap_or_default();
    let text = |game: &mut Game, path: &str, t: String| -> Result<()> {
        let n = wf::child(game, viewer, path)?;
        wf::state(game, n).text = Some(t);
        Ok(())
    };
    use crate::params::{enum_text, CONTRACT_TEXT, YOTOGI_CLASS_TEXT};
    text(game, "ContractType/Name", enum_text(CONTRACT_TEXT, st.contract_type))?;
    text(game, "MaidName/Name", format!("{}\n{}", st.last_name, st.first_name))?;
    text(game, "NumberGroup/YotogiPlayCount/Value", st.yotogi_play_count.to_string())?;
    text(game, "NumberGroup/OthersPlayCount/Value", st.others_play_count.to_string())?;
    text(game, "YotogiClass/Name", enum_text(YOTOGI_CLASS_TEXT, st.current_yotogi_class))?;
    text(game, "YotogiClass/Level/Value", class.level().to_string())?;
    text(game, "YotogiClass/Exp/Exp", class.exp.current.to_string())?;
    text(game, "YotogiClass/Exp/RequiredExp", class.exp.next_level_exp(class.level()).to_string())?;
    let pick = |cur: i32, max: i32| if current { cur } else { max }.to_string();
    text(game, "NumberGroup/Hp/Value", pick(st.a_cur_hp(), st.a_hp()))?;
    text(game, "NumberGroup/Likability/Value", st.likability.to_string())?;
    text(game, "NumberGroup/Reason/Value", pick(st.a_cur_reason(), st.a_reason()))?;
    text(game, "NumberGroup/Mind/Value", pick(st.a_cur_mind(), st.a_mind()))?;
    text(game, "NumberGroup/Lovely/Value", st.a_lovely().to_string())?;
    text(game, "NumberGroup/Elegance/Value", st.a_elegance().to_string())?;
    text(game, "NumberGroup/Charm/Value", st.a_charm().to_string())?;
    text(game, "NumberGroup/Housi/Value", st.a_housi().to_string())?;
    text(game, "NumberGroup/Inyoku/Value", st.a_inyoku().to_string())?;
    text(game, "NumberGroup/MValue/Value", st.a_m_value().to_string())?;
    text(game, "NumberGroup/Hentai/Value", st.a_hentai().to_string())?;
    text(game, "NumberGroup/StudyRate/Value", (st.a_study_rate() / 10).to_string())?;
    text(game, "NumberGroup/TeachRate/Value", (st.a_teach_rate() / 10).to_string())?;
    Ok(())
}

/// Load a `.tex` image into a `UI2DSprite` (`ImportCM.LoadTexture`).
fn load_sprite(game: &mut Game, node: usize, file: &str, size: Option<[f32; 2]>) {
    let st = wf::state(game, node);
    st.size = size;
    game.pending_textures.push((1, node, file.into()));
}

impl Yotogi {
    pub fn call(&mut self, name: String, label: String, edit_label: Option<String>) {
        self.s.calls.push((name, label, edit_label));
    }

    /// Current screen and its fade status (diagnostics).
    pub fn diagnostics(&self) -> String {
        let cur = self.wf.current().unwrap_or("");
        format!("{cur} {:?} next_label={} skills={:?} no={}", self.wf.status(cur), self.s.next_label, self.s.skills.iter().map(|p| p.skill).collect::<Vec<_>>(), self.s.play.no)
    }

    /// `YotogiManager.SetRepeatVoiceFile`.
    pub fn set_repeat_voice(&mut self, file: String) {
        self.s.play.repeat_voice = file;
        self.s.play.repeat_time = REPEAT_WAIT_MS;
    }

    /// `YotogiManager.TagControlMask`.
    pub fn control_mask(&mut self, game: &mut Game) -> Result<()> {
        if self.wf.current() == Some("Play") {
            set_control_mask(game, &mut self.s, true, false)?;
        }
        Ok(())
    }
}

fn set_control_mask(game: &mut Game, s: &mut State, on: bool, endless: bool) -> Result<()> {
    s.play.control_mask_until = if on { Some(if endless { u64::MAX } else { game.now + 3000 }) } else { None };
    for p in CONTROL_MASKS {
        wf::set_active(game, p, on)?;
    }
    Ok(())
}

/// `YotogiManager` update: queued script calls, the screens, tweens.
pub fn update(game: &mut Game, opener: &mut Opener, y: &mut Yotogi) -> Result<()> {
    for (name, label, edit) in std::mem::take(&mut y.s.calls) {
        anyhow::ensure!(["Null", "StageSelect", "SkillSelect", "Play", "Result"].contains(&name.as_str()), "@yotogicall : [{name}]を特定できませんでした");
        y.s.next_label = label;
        if name == "SkillSelect" {
            if let Some(e) = edit {
                y.s.edit_label = e;
            }
        }
        y.wf.call_screen(game, opener, &mut y.s, &name)?;
    }
    y.wf.update(game, opener, &mut y.s)?;
    for op in std::mem::take(&mut game.yotogi_ops) {
        match op {
            AdvUi::RepeatVoice(f) => y.set_repeat_voice(f),
            AdvUi::ControlMask => y.control_mask(game)?,
            _ => {}
        }
    }
    update_tweens(game, &mut y.s)?;
    Ok(())
}

fn update_tweens(game: &mut Game, s: &mut State) -> Result<()> {
    let now = game.now;
    let mut keep = Vec::new();
    for t in std::mem::take(&mut s.tweens) {
        if now < t.start {
            keep.push(t);
            continue;
        }
        let f = if t.seconds <= 0. { 1. } else { ((now - t.start) as f32 / 1000. / t.seconds).min(1.) };
        let e = t.ease.apply(f);
        let lerp3 = |a: [f32; 3], b: [f32; 3]| [a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e, a[2] + (b[2] - a[2]) * e];
        let lerp = |a: i32, b: i32| (a as f32 + (b - a) as f32 * e).round() as i32;
        match &t.kind {
            TweenKind::Scale { from, to } => wf::state(game, t.node).scale = Some(lerp3(*from, *to)),
            TweenKind::Position { from, to } | TweenKind::Scroll { from, to } => wf::state(game, t.node).position = Some(lerp3(*from, *to)),
            TweenKind::Count { from, to } => {
                let v = lerp(*from, *to);
                wf::state(game, t.node).text = Some(if v > 0 { format!("+{v}") } else { v.to_string() });
            }
            TweenKind::SkillExp { skill, from, to, level } => {
                let total = lerp(*from, *to);
                let need = data(game)?.skill(*skill).map(|d| d.exp_table.clone()).unwrap_or_default();
                let mut exp = crate::params::Exp { total, ..Default::default() };
                exp.set_list(need);
                if *level < exp.level {
                    let up = wf::child(game, t.node, "levelup")?;
                    if !game.level.as_ref().unwrap().states.get(&up).and_then(|s| s.active).unwrap_or(false) {
                        game.sound_ops.push(SoundOp::Play { kind: Kind::System, file: "SE008.ogg".into(), fade_s: 0., looping: false });
                        wf::state(game, up).active = Some(true);
                    }
                }
                update_gauge(game, t.node, &exp)?;
            }
        }
        if f < 1. {
            keep.push(t);
        } else if matches!(t.kind, TweenKind::Scroll { .. }) {
            // YotogiParamScroll oncomplete callbacks.
            scroll_tween_done(game, s)?;
        }
    }
    keep.append(&mut s.tweens);
    s.tweens = keep;
    Ok(())
}

/// `YotogiSkillUnit.UpdateGage`.
fn update_gauge(game: &mut Game, unit: usize, exp: &crate::params::Exp) -> Result<()> {
    for i in 0..3 {
        let star = wf::child(game, unit, &format!("Description/StarGroup/Star{i}"))?;
        wf::state(game, star).active = Some(i as i32 <= exp.level - 1);
    }
    let next = exp.next_level_exp(exp.level);
    let v = if next != 0 { exp.current as f32 / next as f32 } else { 0. };
    let v = if v.is_nan() { 0. } else if v.is_infinite() { 1. } else { v };
    let g = wf::child(game, unit, "Description/Gauge")?;
    wf::state(game, g).scale = Some([v, 1., 1.]);
    Ok(())
}

impl Screens for State {
    fn on_call(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        let _ = wf;
        match name {
            "StageSelect" => stage_on_call(game, opener, self),
            "SkillSelect" => skill_on_call(game, opener, self),
            "Play" => play_on_call(game, opener, self),
            "Result" => result_on_call(game, opener, self),
            _ => Ok(()),
        }
    }

    fn is_call_fade_in(&mut self, _wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<bool> {
        match name {
            "StageSelect" => {
                let Some(m) = game.characters.get(self.maid) else { return Ok(true) };
                if m.busy() {
                    return Ok(false);
                }
                let m = game.characters.get_mut(self.maid).unwrap();
                let sets = m.blend_sets.clone();
                m.face.anime("通常", &sets);
                m.face.blend("無し");
                m.visible = false;
                Ok(true)
            }
            "SkillSelect" => {
                let man = game.active_men[0];
                if man.and_then(|i| game.characters.get(i)).is_some_and(|c| c.busy()) || game.characters.get(self.maid).is_some_and(|c| c.busy()) {
                    return Ok(false);
                }
                if let Some(c) = man.and_then(|i| game.characters.get_mut(i)) {
                    c.visible = false;
                }
                if let Some(c) = game.characters.get_mut(self.maid) {
                    c.offset_position = [0.; 3];
                }
                init_maid_gesture(game, opener, self)?;
                Ok(true)
            }
            "Play" => play_is_call_fade_in(game, opener, self),
            _ => Ok(true),
        }
    }

    fn on_finish(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        match name {
            "StageSelect" => {
                if let Some(m) = game.characters.get_mut(self.maid) {
                    m.visible = true;
                }
                if !self.next_label.is_empty() {
                    wf.call_screen(game, opener, self, "Null")?;
                }
            }
            "SkillSelect" => {
                game.adv_ui(AdvUi::AllPos { position: Some([0.; 3]), rotation: Some([0.; 3]) }, opener)?;
                game.camera_offset = 0.;
                game.stop_motion_scripts();
                if !self.next_label.is_empty() {
                    wf.call_screen(game, opener, self, "Null")?;
                }
            }
            "Play" => play_on_finish(wf, game, opener, self)?,
            "Result" => result_on_finish(game, opener, self)?,
            "Null" => {
                if !self.next_label.is_empty() {
                    let label = std::mem::take(&mut self.next_label);
                    let storage = opener.storage.clone();
                    game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
                    game.adv.exec();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_fade_end(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        if name == "Result" && !is_all_chara_busy(game) {
            reset_view(game, opener)?;
            for i in game.active_maids.iter().chain(game.active_men.iter()).flatten().copied().collect::<Vec<_>>() {
                game.characters[i].visible = false;
            }
            if let Some(m) = game.characters.get_mut(self.maid) {
                m.visible = true;
            }
            if !self.next_label.is_empty() {
                wf.call_screen(game, opener, self, "Null")?;
            }
        }
        Ok(())
    }

    fn update(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()> {
        let status = wf.status(name);
        match name {
            "Null" if status == FadeStatus::Wait => {
                wf.finish(game, opener, self, "Null")?;
            }
            "StageSelect" | "SkillSelect" | "Play" | "Result" if status == FadeStatus::Wait => {
                let events: Vec<_> = std::mem::take(&mut game.events);
                for (_, node, handler) in events {
                    self.event(wf, game, opener, name, node, &handler)?;
                }
                if name == "Play" {
                    play_update(wf, game, opener, self)?;
                }
            }
            _ => {}
        }
        if name == "Play" {
            scroll_update(game, self)?;
        }
        Ok(())
    }
}

impl State {
    fn event(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, screen: &str, node: usize, handler: &str) -> Result<()> {
        let (method, arg) = handler.split_once(':').map_or((handler, None), |(m, a)| (m, Some(a)));
        match (screen, method) {
            (_, "SystemDialog.Ok") | (_, "YotogiSkillSelectManager.SwoonOk") => {
                game.set_main_active("__GameMain__/SystemUI Root/SystemDialog", false)?;
                match self.dialog.take() {
                    Some("swoon_confirm") | Some("swoon") if !self.next_label.is_empty() => {
                        wf.finish(game, opener, self, screen)?;
                    }
                    Some("swoon") => game.adv_ui(AdvUi::Fade3d { to: 0., seconds: 0. }, opener)?,
                    _ => {}
                }
            }
            (_, "YotogiSkillSelectManager.SwoonCancel") => {
                game.set_main_active("__GameMain__/SystemUI Root/SystemDialog", false)?;
                self.dialog = None;
            }
            ("StageSelect", "YotogiStageUnit.UpdateBG") => {
                if let Some(&(_, stage)) = self.stage_units.iter().find(|u| u.0 == node) {
                    select_stage(game, opener, self, stage)?;
                }
            }
            ("StageSelect", "YotogiStageSelectManager.OnClickOK") => {
                if !self.next_label.is_empty() {
                    wf.finish(game, opener, self, "StageSelect")?;
                }
            }
            ("SkillSelect", "YotogiSkillSelectManager.OnClickFromCategoryButton") => {
                let i: usize = arg.and_then(|a| a.parse().ok()).context("category")?;
                set_category_skill(game, opener, self, i)?;
            }
            ("SkillSelect", "YotogiSkillSelectManager.OnClickFromSkillUnit") => {
                if let Some(&(_, id)) = self.skill_units.iter().find(|u| u.0 == node) {
                    if let Some(free) = self.slots.iter().position(Option::is_none) {
                        self.slots[free] = Some(id);
                        update_slot(game, self, free)?;
                        on_skill_change(game, self)?;
                    }
                }
            }
            ("SkillSelect", "YotogiSkillSelectManager.OnClickAllReset") => {
                self.slots = [None; SLOTS];
                for i in 0..SLOTS {
                    update_slot(game, self, i)?;
                }
                on_skill_change(game, self)?;
            }
            ("SkillSelect", "YotogiSkillContainerViewer.OnRightClickIcon") => {
                // Only a right click removes (UICamera.currentKey == Mouse1).
                if game.input.buttons_pressed & crysirror_core::service::buttons::MOUSE_RIGHT != 0 {
                    let i: usize = arg.and_then(|a| a.parse().ok()).context("slot")?;
                    self.slots[i] = None;
                    // Close the gap: later skills move forward.
                    let mut list: Vec<i32> = self.slots.iter().flatten().copied().collect();
                    list.resize(SLOTS, -1);
                    for (k, v) in list.into_iter().enumerate() {
                        self.slots[k] = (v >= 0).then_some(v);
                        update_slot(game, self, k)?;
                    }
                    on_skill_change(game, self)?;
                }
            }
            ("SkillSelect", "YotogiSkillSelectManager.OnClickEdit") => {
                if !self.edit_label.is_empty() {
                    self.next_label = self.edit_label.clone();
                    wf.finish(game, opener, self, "SkillSelect")?;
                }
            }
            ("SkillSelect", "YotogiSkillSelectManager.OnClickNext") => {
                set_play_skill_array(game, self)?;
                let hp = maid_status(game, self.maid)?.a_cur_hp();
                if hp <= 0 {
                    game.show_system_dialog("体力が0以下のため気絶します。\n本当に実行しますか？", "YotogiSkillSelectManager.SwoonOk")?;
                    game.set_main_active("__GameMain__/SystemUI Root/SystemDialog/Base/Cancel", true)?;
                    if let Some(n) = game.main.scene.find("__GameMain__/SystemUI Root/SystemDialog/Base/Cancel") {
                        game.main.states.entry(n).or_default().click = Some("YotogiSkillSelectManager.SwoonCancel".into());
                    }
                    self.dialog = Some("swoon_confirm");
                } else if !self.next_label.is_empty() {
                    wf.finish(game, opener, self, "SkillSelect")?;
                }
            }
            ("Play", "YotogiCommandFactory.OnClick") => {
                let cmd = self.play.groups.iter().flat_map(|g| g.2.iter()).find(|c| c.0 == node).map(|c| c.1);
                if let Some(c) = cmd {
                    on_click_command(game, opener, self, c)?;
                }
            }
            ("Play", "YotogiPlayManager.OnSkillIconClick") => {
                let i: i32 = arg.and_then(|a| a.parse().ok()).context("icon")?;
                // Only a clickable (not yet played) icon reaches here.
                // `num = i - 1` with `num != -1`: the first icon does nothing.
                let playable = self.skills.get(i as usize).is_some_and(|p| p.skill.is_some() && !p.is_play) && self.play.no != i;
                if playable && i != 0 {
                    self.play.no = i - 1;
                    on_next_skill_move(wf, game, opener, self)?;
                }
            }
            ("Play", "YotogiPlayManager.OnClickNext") => {
                self.play.no = -1;
                if let Some(i) = self.skills.iter().position(|p| p.skill.is_some() && !p.is_play) {
                    self.play.no = i as i32 - 1;
                }
                on_next_skill_move(wf, game, opener, self)?;
            }
            ("Play", "YotogiPlayManager.OnClickViweReset") => {
                if let Some((t, d, a)) = game.yotogi_saved_camera {
                    game.adv_ui(AdvUi::CameraAnimation { from: None, to: (t, d, a), delay_ms: 0, ms: 500, ease: "easeOutCubic".into() }, opener)?;
                }
            }
            ("Play", "UndressingUnit.OnClickEvent") => undress_click(game, opener, self, node)?,
            ("Play", "UndressingUnit.OnClickEventAllOFF") => undress_all(game, opener, self, node, false)?,
            ("Play", "UndressingUnit.OnClickEventAllON") => undress_all(game, opener, self, node, true)?,
            ("Play", "YotogiPlayManager.ChangeSliderValue") => {
                let v = game.level.as_ref().unwrap().states.get(&node).and_then(|s| s.value).unwrap_or(1.);
                game.man_alpha = (v * 100.).ceil() as i32;
                game.man_alpha_update();
            }
            ("Result", "YotogiResultManager.OnClickNext") => {
                if !self.next_label.is_empty() {
                    wf.finish(game, opener, self, "Result")?;
                }
            }
            _ => game.log.push(format!("SceneYotogi {screen} event {handler} not ported")),
        }
        Ok(())
    }
}

// ---- StageSelect ----------------------------------------------------------

fn stage_on_call(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    wf::set_click(game, &format!("{STAGE}/Ok"), "YotogiStageSelectManager.OnClickOK")?;
    if let Some(m) = game.characters.get_mut(s.maid) {
        m.visible = true;
    }
    reset_view(game, opener)?;
    let grade = lock(game, |st, _| Ok(st.player.best_salon_grade))?;
    let d = data(game)?;
    let list: Vec<(usize, bool, String, i32, String)> =
        d.yotogi.playable_stages(grade).into_iter().map(|(st, on)| (st.stage, on, st.draw_name.clone(), st.need_grade, st.thumbnail_name.clone())).collect();
    let parent = wf::node(game, &format!("{STAGE}/StageSelectViewer/StageViewer/Contents/StageUnitParent"))?;
    wf::clear_children(game, parent);
    s.stage_units.clear();
    for (stage, enabled, draw, need, thumb) in list {
        let unit = wf::create_prefab(game, opener, parent, "SceneYotogi/StageSelect/Prefab/StageUnit")?;
        let p = wf::child(game, unit, "Parent")?;
        let name = wf::child(game, p, "Name")?;
        wf::state(game, name).text = Some(if enabled { draw } else { "??????".into() });
        for i in 0..5 {
            let star = wf::child(game, p, &format!("StarGroup/Star{}", i + 1))?;
            wf::state(game, star).active = Some(i < need);
        }
        let mut file = format!("{thumb}.tex");
        if !enabled {
            file = "cm3d2_stageselectpreview_nowprinting.tex".into();
        }
        if !game.files.exists(&file) {
            file = "cm3d2_stageselectpreview_dammy.tex".into();
        }
        if game.files.exists(&file) {
            let icon = wf::child(game, p, "Icon")?;
            load_sprite(game, icon, &file, None);
        }
        wf::state(game, p).enabled = Some(enabled);
        wf::state(game, p).click = Some("YotogiStageUnit.UpdateBG".into());
        s.stage_units.push((p, stage));
    }
    wf::reposition(game, parent);
    wf::reset_scroll(game, &format!("{STAGE}/StageSelectViewer/StageViewer/Contents"))?;
    // UIWFTabPanel.Select(first): runs the first unit's onClick.
    if let Some(&(_, stage)) = s.stage_units.first() {
        select_stage(game, opener, s, stage)?;
    }
    Ok(())
}

/// `YotogiStageUnit.UpdateBG` + `OnSelectEvent`.
fn select_stage(game: &mut Game, opener: &mut Opener, s: &mut State, stage: usize) -> Result<()> {
    let d = data(game)?;
    let st = d.yotogi.stages.get(&stage).context("stage data")?;
    game.log.push(format!("yotogi stage {} bg {} (was {:?})", st.stage_name, st.prefab_name, game.bg.as_ref().map(|b| &b.0)));
    game.adv_ui(AdvUi::SetBg(st.prefab_name.clone()), opener)?;
    lock(game, |t, _| {
        t.stage_name = st.stage_name.clone();
        t.stage_prefab = st.prefab_name.clone();
        Ok(())
    })?;
    s.stage = stage;
    // CameraMain.SetFromScriptOnTarget(center, radius, rotate).
    game.adv_ui(AdvUi::Camera { target: st.select_camera_center, distance: st.select_camera_radius, around: st.select_camera_rotate }, opener)?;
    game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: st.bgm_file.clone(), fade_s: 1., looping: true });
    // UIWFTabButton selection cursor of the clicked unit.
    for &(p, u) in &s.stage_units.clone() {
        if let Ok(c) = wf::child(game, p, "SelectCursor") {
            wf::state(game, c).alpha = Some(if u == stage { 1. } else { 0. });
        }
    }
    Ok(())
}

// ---- SkillSelect ----------------------------------------------------------

fn skill_on_call(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    reset_view(game, opener)?;
    let d = data(game)?;
    let stage_name = lock(game, |t, _| Ok(t.stage_name.clone()))?;
    let stage = if stage_name.is_empty() { 2 } else { STAGES.iter().position(|x| *x == stage_name).with_context(|| format!("Yotogi.Stage enum convert error.\n{stage_name}"))? };
    s.stage = stage;
    let sd = d.yotogi.stages.get(&stage).context("stage data")?.clone();
    game.sound_ops.push(SoundOp::Play { kind: Kind::Bgm, file: sd.bgm_file.clone(), fade_s: 1.5, looping: true });
    let maid = s.maid;
    // New skills whose conditions hold (without class requirements).
    let skills = with_param(game, maid, |p, _| {
        for cat in &d.skills {
            for (id, sk) in cat {
                if !p.st.skill_data.contains(id) && p.is_check_get_skill(sk, false, false) {
                    p.set_new_get_skill(*id);
                }
            }
        }
        // CreateBackUpSkillID.
        let ids: Vec<i32> = p.st.skill_data.iter().map(|(k, _)| *k).collect();
        let hp = p.st.a_hp();
        p.set_cur_hp(hp);
        let (mind, reason) = (p.st.a_mind(), p.st.a_reason());
        p.set_cur_mind(mind);
        p.set_cur_reason(reason);
        Ok(ids)
    })?;
    s.backup_skills = skills.clone();
    let prefab = lock(game, |t, _| Ok(t.stage_prefab.clone()))?;
    if !prefab.is_empty() {
        game.adv_ui(AdvUi::SetBg(prefab), opener)?;
    }
    if let Some(m) = game.active_men[0] {
        game.characters[m].visible = true;
    }
    game.characters[maid].visible = true;
    for (man, slot) in [(true, 0), (false, 0)] {
        game.adv_ui(AdvUi::AllProcProp { man, slot }, opener)?;
    }
    wf::set_click(game, &format!("{SKILL}/CommonPanel/UnderButtonGroup/Next"), "YotogiSkillSelectManager.OnClickNext")?;
    wf::set_active(game, &format!("{SKILL}/CommonPanel/UnderButtonGroup/ViweReset"), false)?;
    update_viewer(game, s.viewers[0], maid, true)?;
    // YotogiSkillContainerViewer.Init + hp.
    s.slots = [None; SLOTS];
    for i in 0..SLOTS {
        update_slot(game, s, i)?;
    }
    let st = maid_status(game, maid)?;
    let hp = st.a_cur_hp();
    wf::set_text(game, &format!("{SKILL}/SkillContainerViewer/HpGroup/Values/MaxHp"), hp.to_string())?;
    set_current_hp(game, s, hp)?;
    // Categories: owned skills runnable on this stage by this maid.
    for c in &mut s.categories {
        c.1.clear();
    }
    for id in &skills {
        let Some(sk) = d.skill(*id) else { continue };
        if sk.exec_stage.iter().any(|x| *x == STAGES[stage]) && is_exec_maid(sk, &st) {
            let ci = crate::gamedata::SKILL_CATEGORIES.iter().position(|c| *c == sk.category).unwrap_or(0);
            s.categories[ci].1.push(*id);
        }
    }
    for c in &mut s.categories {
        c.1.sort();
    }
    for (obj, list) in s.categories.clone() {
        let b = wf::child(game, obj, "Button")?;
        wf::state(game, b).enabled = Some(!list.is_empty());
    }
    if let Some(first) = s.categories.iter().position(|c| !c.1.is_empty()) {
        set_category_skill(game, opener, s, first)?;
    } else {
        on_skill_change(game, s)?;
    }
    Ok(())
}

/// `Yotogi.SkillData.IsExecMaid`.
fn is_exec_maid(sk: &crate::gamedata::Skill, st: &MaidStatus) -> bool {
    // exec_condition order Tonus..Slave = Condition 1..5.
    let cond = (st.condition >= 1) && sk.exec_condition.get(st.condition as usize - 1).copied().unwrap_or(false);
    let seikeiken = sk.exec_seikeiken.get(st.seikeiken as usize).copied().unwrap_or(false);
    if !cond || !seikeiken {
        return false;
    }
    let drunk = st.condition_special == 1;
    sk.drunk_skill == drunk
}

/// `YotogiSkillSelectManager.InitMaidGesture`.
fn init_maid_gesture(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    let d = data(game)?;
    let sd = d.yotogi.stages.get(&s.stage).context("stage data")?.clone();
    game.adv_ui(AdvUi::AllPos { position: Some(sd.skill_chara_pos), rotation: Some(sd.skill_chara_rotate) }, opener)?;
    game.adv_ui(AdvUi::ItemMaskMode { man: false, slot: 0, mode: "None".into() }, opener)?;
    game.adv_ui(AdvUi::Eye { man: false, slot: 0, look: EyeLook::Camera("目と顔を向ける".into()), blend_s: 0.5 }, opener)?;
    let st = maid_status(game, s.maid)?;
    let wm = d.yotogi.wait_motion(st.personal, st.condition, st.condition_special)?.clone();
    game.motion_blend = false;
    game.adv_ui(AdvUi::MotionScript { slot: 0, file: wm.motion_script_file.clone(), label: wm.motion_script_label.clone() }, opener)?;
    game.motion_blend = true;
    if let Some(m) = game.characters.get_mut(s.maid) {
        let sets = m.blend_sets.clone();
        m.face.anime(&wm.face_name, &sets);
        m.face.blend("無し");
        m.face.mouth_open = false;
    }
    // SetTargetPos(head bone − offset): the head bone position comes from
    // the posed skeleton, which the CPU pose does not provide yet
    // (SATD-cm3d2-game-logic); the standing head height of the default
    // body stands in.
    let head = [sd.skill_chara_pos[0], sd.skill_chara_pos[1] + 1.45, sd.skill_chara_pos[2]];
    let target = [head[0] - 0.00503452, head[1] - 0.02758344, head[2] + 0.0292511];
    game.adv_ui(AdvUi::Camera { target, distance: 1.1, around: sd.skill_camera_rotate }, opener)?;
    // SetTargetOffset(Screen.width / 256 · 154 − Screen.width / 2).
    game.camera_offset = crate::ngui::WIDTH / 256. * 154. - crate::ngui::WIDTH / 2.;
    Ok(())
}

fn set_category_skill(game: &mut Game, opener: &mut Opener, s: &mut State, ci: usize) -> Result<()> {
    let parent = wf::node(game, &format!("{SKILL}/SkillViewer/Contents/SkillUnitParent"))?;
    wf::clear_children(game, parent);
    s.skill_units.clear();
    let d = data(game)?;
    let st = maid_status(game, s.maid)?;
    for id in s.categories[ci].1.clone() {
        let unit = wf::create_prefab(game, opener, parent, "SceneYotogi/SkillSelect/Prefab/SkillUnit")?;
        let button = wf::child(game, unit, "Button")?;
        wf::state(game, button).click = Some("YotogiSkillSelectManager.OnClickFromSkillUnit".into());
        set_skill_unit(game, &d, &st, unit, id)?;
        s.skill_units.push((button, id));
    }
    wf::reposition(game, parent);
    wf::reset_scroll(game, &format!("{SKILL}/SkillViewer/Contents"))?;
    // Arrow follows the selected category; tab selection state.
    for (i, (obj, _)) in s.categories.clone().into_iter().enumerate() {
        if let Ok(f) = wf::child(game, obj, "Frame") {
            wf::state(game, f).alpha = Some(if i == ci { 1. } else { 0. });
        }
    }
    let arrow = wf::node(game, &format!("{SKILL}/CategoryViewer/Arrow"))?;
    let obj = s.categories[ci].0;
    let level = game.level.as_ref().unwrap();
    let pos = |n: usize| level.states.get(&n).and_then(|x| x.position).unwrap_or(level.scene.nodes[n].position);
    // Arrow parent is CategoryViewer; the button sits under Content/CategoryUnitParent.
    let content = level.scene.nodes[obj].parent.and_then(|p| level.scene.nodes[p].parent).context("category content")?;
    let unit_parent = level.scene.nodes[obj].parent.unwrap();
    let y = pos(content)[1] + pos(unit_parent)[1] + pos(obj)[1] + 18.;
    let from = pos(arrow);
    let to = [from[0], y, from[2]];
    let now = game.now;
    tween(s, now, arrow, 0.3, Ease::OutQuint, TweenKind::Position { from, to });
    on_skill_change(game, s)?;
    Ok(())
}

/// `YotogiSkillUnit.SetSkillData`.
fn set_skill_unit(game: &mut Game, d: &GameData, st: &MaidStatus, unit: usize, id: i32) -> Result<()> {
    let sk = d.skill(id).context("skill")?;
    let set = |game: &mut Game, path: &str, t: String| -> Result<()> {
        let n = wf::child(game, unit, path)?;
        wf::state(game, n).text = Some(t);
        Ok(())
    };
    set(game, "Description/Name", sk.name.clone())?;
    set(game, "Description/UnderGroup/AcquiredExperience/Value", sk.add_yotogi_class_exp.to_string())?;
    set(game, "Description/UnderGroup/CostOfHp/Value", sk.exec_need_hp.to_string())?;
    let icon = wf::child(game, unit, "Icon")?;
    let file = if game.files.exists(&format!("{}.tex", sk.icon_file_name)) { format!("{}.tex", sk.icon_file_name) } else { "icon_none.tex".into() };
    load_sprite(game, icon, &file, Some([ICON_SIZE, ICON_SIZE]));
    let sd = st.skill_data.get(&id).cloned().unwrap_or_default();
    let new = wf::child(game, unit, "IconNew")?;
    wf::state(game, new).active = Some(sd.play_count == 0);
    let up = wf::child(game, unit, "levelup")?;
    wf::state(game, up).active = Some(false);
    update_gauge(game, unit, &sd.exp)
}

/// `YotogiSkillSelectSlot.SetSkillData` + `UpdateData`.
fn update_slot(game: &mut Game, s: &State, i: usize) -> Result<()> {
    let p = format!("{SKILL}/SkillContainerViewer/SkillParent/SkillIconUnit{i}");
    let icon = wf::node(game, &format!("{p}/SkillIconDrag"))?;
    let d = data(game)?;
    let st = maid_status(game, s.maid)?;
    match s.slots[i].and_then(|id| d.skill(id)) {
        None => {
            let st = wf::state(game, icon);
            st.sprite2d = Some(None);
            st.collider = Some(false);
            wf::set_text(game, &format!("{p}/SkillName/Val"), "")?;
        }
        Some(sk) => {
            let file = if game.files.exists(&format!("{}.tex", sk.icon_file_name)) { format!("{}.tex", sk.icon_file_name) } else { "icon_none.tex".into() };
            wf::state(game, icon).collider = Some(true);
            load_sprite(game, icon, &file, Some([ICON_SIZE, ICON_SIZE]));
            wf::set_text(game, &format!("{p}/SkillName/Val"), sk.name.clone())?;
        }
    }
    let level = s.slots[i].and_then(|id| st.skill_data.get(&id)).map_or(0, |x| x.exp.level).min(3);
    for k in 0..3 {
        wf::set_active(game, &format!("{p}/StarGroup/Star{k}"), (k as i32) < level)?;
    }
    Ok(())
}

/// `YotogiSkillContainerViewer.SetCurrentHp`.
fn set_current_hp(game: &mut Game, s: &mut State, hp: i32) -> Result<()> {
    let base = format!("{SKILL}/SkillContainerViewer/HpGroup");
    let max_node = wf::node(game, &format!("{base}/Values/MaxHp"))?;
    let max: i32 = game.level.as_ref().unwrap().states.get(&max_node).and_then(|x| x.text.as_deref()).and_then(|t| t.parse().ok()).unwrap_or(0);
    let cur = hp.min(max);
    let n = wf::node(game, &format!("{base}/Values/CurrentHp"))?;
    let st = wf::state(game, n);
    st.text = Some(cur.to_string());
    st.color = Some(if cur <= 0 { [1., 0., 0., 1.] } else { [1.; 4] });
    let g = wf::node(game, &format!("{base}/Gauge"))?;
    let from = game.level.as_ref().unwrap().states.get(&g).and_then(|x| x.scale).unwrap_or([1.; 3]);
    let v = if max > 0 { cur.max(0) as f32 / max as f32 } else { 0. };
    let now = game.now;
    tween(s, now, g, 0.4, Ease::OutSine, TweenKind::Scale { from, to: [v, 1., 1.] });
    Ok(())
}

/// `YotogiSkillSelectManager.OnSkillChangeEvent`.
fn on_skill_change(game: &mut Game, s: &mut State) -> Result<()> {
    let d = data(game)?;
    let set: Vec<i32> = s.slots.iter().flatten().copied().collect();
    let max_hp = maid_status(game, s.maid)?.a_hp();
    let hp = max_hp - set.iter().filter_map(|id| d.skill(*id)).map(|k| k.exec_need_hp).sum::<i32>();
    let open = set.len() != SLOTS && 0 < hp;
    for (button, id) in s.skill_units.clone() {
        let only_one = d.skill(id).is_some_and(|k| k.only_one);
        let on = open && !(only_one && set.contains(&id));
        wf::state(game, button).enabled = Some(on);
    }
    set_current_hp(game, s, hp)?;
    with_param(game, s.maid, |p, _| {
        p.set_cur_hp(hp);
        Ok(())
    })?;
    update_viewer(game, s.viewers[0], s.maid, true)?;
    wf::set_enabled(game, &format!("{SKILL}/CommonPanel/UnderButtonGroup/Next"), !set.is_empty())?;
    Ok(())
}

/// `YotogiManager.SetPlaySkillArray`.
fn set_play_skill_array(game: &mut Game, s: &mut State) -> Result<()> {
    let set: Vec<i32> = s.slots.iter().flatten().copied().collect();
    let st = maid_status(game, s.maid)?;
    for i in 0..SLOTS {
        let skill = set.get(i).copied();
        s.skills[i] = PlayingSkill { skill, exp: 0, backup_total_exp: skill.and_then(|id| st.skill_data.get(&id)).map_or(0, |x| x.exp.total), is_play: false };
    }
    Ok(())
}

// ---- Play -----------------------------------------------------------------

fn play_on_call(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    s.play.is_swoon = false;
    s.play.is_reset_item = false;
    s.play.insert = false;
    if !s.play.next_flag {
        clear_voice(game, s);
        s.play.zettyo_count = 0;
        wf::set_click(game, &format!("{PLAY}/CommonPanel/UnderButtonGroup/ViweReset"), "YotogiPlayManager.OnClickViweReset")?;
        eval(game, "global.__skill_command_file = new Array();")?;
        wf::set_click(game, &format!("{PLAY}/CommonPanel/UnderButtonGroup/Next"), "YotogiPlayManager.OnClickNext")?;
        let st = with_param(game, s.maid, |p, _| {
            p.st.cur_excite = 0;
            let (mind, reason) = (p.st.a_mind(), p.st.a_reason());
            p.set_cur_mind(mind);
            p.set_cur_reason(reason);
            Ok(p.st.clone())
        })?;
        let _ = st;
        s.play.no = -1;
        let n = wf::node(game, MAN_ALPHA)?;
        wf::state(game, n).value = Some(game.man_alpha as f32 / 100.);
        wf::state(game, n).click = Some("YotogiPlayManager.ChangeSliderValue".into());
        init_undressing(game, s)?;
        let drunk = maid_status(game, s.maid)?.condition_special == 1;
        if drunk {
            with_param(game, s.maid, |_, pl| Ok(pl.add_have_trophy(420)))?;
        }
    }
    s.play.next_flag = false;
    next_skill(game, opener, s)?;
    reset_world(game, opener, s)?;
    Ok(())
}

fn play_is_call_fade_in(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<bool> {
    if !s.play.is_reset_item {
        if is_all_chara_busy(game) {
            return Ok(false);
        }
        game.motion_blend = true;
        s.play.is_reset_item = true;
        // UnVisibleMaidAndMan(false, true).
        for i in game.active_men.iter().flatten().copied().collect::<Vec<_>>() {
            game.characters[i].visible = false;
        }
        let d = data(game)?;
        let skill = current_skill(s).and_then(|id| d.skill(id)).cloned();
        if let Some(sk) = &skill {
            if !sk.start_call_file.is_empty() {
                call_normal_file(game, opener, &sk.start_call_file, "")?;
            }
        }
        game.yotogi_saved_camera = Some(game.camera);
        if let Some(m) = game.active_men[0] {
            game.characters[m].visible = true;
        }
        if let Some(sk) = &skill {
            if !sk.start_call_file2.is_empty() {
                let excite = maid_status(game, s.maid)?.a_cur_excite();
                call_normal_file(game, opener, &sk.start_call_file2, &format!("*{}", rr_type(excite)))?;
            }
        }
        for (man, list) in [(false, game.active_maids.to_vec()), (true, game.active_men.to_vec())] {
            for (slot, c) in list.into_iter().enumerate() {
                if c.is_some_and(|i| game.characters[i].visible) {
                    game.adv_ui(AdvUi::AllProcProp { man, slot }, opener)?;
                }
            }
        }
        return Ok(false);
    }
    Ok(!is_all_chara_busy(game) && game.yotogi_kag.current_label() == "*finish")
}

fn current_skill(s: &State) -> Option<i32> {
    usize::try_from(s.play.no).ok().and_then(|i| s.skills.get(i)).and_then(|p| p.skill)
}

/// `YotogiPlayManager.CallNormalFile`.
fn call_normal_file(game: &mut Game, opener: &mut Opener, file: &str, label: &str) -> Result<String> {
    let text = replace_personal(game, file)?;
    eval(game, &format!("global.__skill_first_file = '{text}';"))?;
    eval(game, &format!("global.__skill_first_file_label = '{label}';"))?;
    load_yotogi_script(game, opener, "yotogi_skill_call_main.ks")?;
    Ok(text)
}

/// `YotogiPlayManager.CallCommandFile`.
fn call_command_file(game: &mut Game, opener: &mut Opener, file: &str, label: &str) -> Result<()> {
    if file.is_empty() || file == ".ks" {
        return Ok(());
    }
    let file = replace_personal(game, file)?;
    let file = match file.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.ks"),
        None => format!("{file}.ks"),
    };
    let count = match eval(game, "global.__skill_command_file.count;")? {
        crysirror_tjs2::runtime::Variant::Integer(n) => n,
        _ => 0,
    };
    let add = format!("global.__skill_command_file.add(%['file'=>'{file}','label'=>'{label}']);");
    if count <= 0 {
        eval(game, "global.__skill_command_file.clear();")?;
        eval(game, &add)?;
        load_yotogi_script(game, opener, "yotogi_skill_command_call_main.ks")?;
    } else {
        eval(game, &add)?;
    }
    Ok(())
}

fn clear_voice(game: &mut Game, s: &mut State) {
    voice_stop_all(game);
    s.play.repeat_time = 0;
}

/// `YotogiPlayManager.NextSkill`.
fn next_skill(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<bool> {
    let _ = opener;
    clear_voice(game, s);
    s.play.repeat_voice.clear();
    s.play.repeat_time = REPEAT_WAIT_MS;
    set_control_mask(game, s, false, false)?;
    s.play.played.clear();
    s.play.no += 1;
    let st = with_param(game, s.maid, |p, _| {
        let mind = p.st.a_mind();
        let v = if p.st.a_cur_mind() <= 0 { mind / 2 } else { mind };
        p.set_cur_mind(v);
        Ok(p.st.clone())
    })?;
    set_bar(game, 0, st.a_cur_excite(), false, s)?;
    set_bar(game, 1, st.a_cur_mind(), false, s)?;
    set_bar(game, 2, st.a_cur_reason(), false, s)?;
    update_viewer(game, s.viewers[1], s.maid, true)?;
    scroll_init(game, s)?;
    let Some(id) = current_skill(s) else { return Ok(false) };
    update_skill_tower(game, s)?;
    s.skills[s.play.no as usize].is_play = true;
    let d = data(game)?;
    let commands = d.yotogi.commands.get(&id).context("skill commands")?;
    reserve_commands(game, opener, s, commands)?;
    s.play.level = st.skill_data.get(&id).map_or(0, |x| x.exp.level);
    s.play.valid = [Vec::new(), Vec::new()];
    for (i, c) in commands.iter().enumerate() {
        if s.play.level >= c.request_skill_lv {
            if c.command_type == CMD_INSERT {
                s.play.valid[1].push(i);
                s.play.valid[0].push(i);
            } else if c.request_insert {
                s.play.valid[1].push(i);
            } else {
                s.play.valid[0].push(i);
            }
        }
    }
    update_command(game, s)?;
    Ok(true)
}

/// `YotogiParamBasicBar.SetCurrent*` (0 excite, 1 mind, 2 reason).
fn set_bar(game: &mut Game, kind: usize, cur: i32, anime: bool, s: &mut State) -> Result<()> {
    let st = maid_status(game, s.maid)?;
    let (name, v, color) = match kind {
        0 => {
            let v = (cur + 100) as f32 / 300. - 1. / 3.;
            let c = if 200 <= cur { [1., 0., 0., 1.] } else if 100 <= cur { [1., 0.5, 0., 1.] } else if cur < 0 { [0., 0.5, 1., 1.] } else { [1., 1., 0., 1.] };
            ("Excited", v, c)
        }
        _ => {
            let max = if kind == 1 { st.a_mind() } else { st.a_reason() };
            let v = if max != 0 { cur as f32 / max as f32 } else { 0. };
            ("Mind", v, if v as f64 <= 0.3 { [1., 0., 0., 1.] } else { [0., 0.5, 1., 1.] })
        }
    };
    let name = if kind == 2 { "Reason" } else { name };
    wf::set_text(game, &format!("{BASIC_BAR}/{name}/Value"), cur.to_string())?;
    let g = wf::node(game, &format!("{BASIC_BAR}/{name}/GaugeGroup/Gauge"))?;
    wf::state(game, g).color = Some(color);
    let to = [v, 1., 1.];
    if anime {
        let from = game.level.as_ref().unwrap().states.get(&g).and_then(|x| x.scale).unwrap_or(game.level.as_ref().unwrap().scene.nodes[g].scale);
        let now = game.now;
        tween(s, now, g, 0.4, Ease::OutSine, TweenKind::Scale { from, to });
    } else {
        s.tweens.retain(|t| t.node != g);
        wf::state(game, g).scale = Some(to);
    }
    Ok(())
}

/// `YotogiCommandFactory.ReserveSkillCommand`: titles and commands per
/// group, ids 100·group (+5 per command) as `localPosition.z`.
fn reserve_commands(game: &mut Game, opener: &mut Opener, s: &mut State, commands: &[Command]) -> Result<()> {
    let hidden = wf::node(game, &format!("{SKILL_GROUP}/CommandParent/HiddenTree"))?;
    let unit = wf::node(game, &format!("{SKILL_GROUP}/CommandParent/CommandUnit"))?;
    wf::clear_children(game, hidden);
    wf::clear_children(game, unit);
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, c) in commands.iter().enumerate() {
        match groups.iter_mut().find(|g| g.0 == c.group_name) {
            Some(g) => g.1.push(i),
            None => groups.push((c.group_name.clone(), vec![i])),
        }
    }
    s.play.groups.clear();
    for (k, (name, list)) in groups.into_iter().enumerate() {
        let id = (k * 100) as f32;
        let title = wf::create_prefab(game, opener, hidden, "SceneYotogi/Yotogi/Prefab/CommandCategoryTitle")?;
        let p = game.level.as_ref().unwrap().scene.nodes[title].position;
        let st = wf::state(game, title);
        st.text = Some(name.clone());
        st.position = Some([p[0], p[1], id]);
        let mut items = Vec::new();
        let mut z = id + 5.;
        for i in list {
            let c = &commands[i];
            let node = wf::create_prefab(game, opener, hidden, "SceneYotogi/Yotogi/Prefab/Command")?;
            let p = game.level.as_ref().unwrap().scene.nodes[node].position;
            let label = wf::child(game, node, "Name")?;
            wf::state(game, label).text = Some(c.name.clone());
            if c.active_command {
                wf::state(game, label).color = Some([1., 0., 0., 1.]);
            }
            let st = wf::state(game, node);
            st.position = Some([p[0], p[1], z]);
            st.click = Some("YotogiCommandFactory.OnClick".into());
            items.push((node, i));
            z += 5.;
        }
        s.play.groups.push((name, title, items));
    }
    Ok(())
}

/// Move a node under another parent (`Transform.SetParent`).
fn reparent(game: &mut Game, node: usize, parent: usize) {
    let scene = &mut game.level.as_mut().unwrap().scene;
    if let Some(old) = scene.nodes[node].parent {
        scene.nodes[old].children.retain(|c| *c != node);
    }
    scene.nodes[node].parent = Some(parent);
    scene.nodes[parent].children.push(node);
}

/// `YotogiPlayManager.UpdateCommand` (`ClearCommand` + `AddCommand`) and
/// the factory's `LateUpdate` grid reposition by `localPosition.z`.
fn update_command(game: &mut Game, s: &mut State) -> Result<()> {
    let hidden = wf::node(game, &format!("{SKILL_GROUP}/CommandParent/HiddenTree"))?;
    let unit = wf::node(game, &format!("{SKILL_GROUP}/CommandParent/CommandUnit"))?;
    for (_, title, items) in s.play.groups.clone() {
        reparent(game, title, hidden);
        for (n, _) in items {
            reparent(game, n, hidden);
        }
    }
    let st = maid_status(game, s.maid)?;
    if st.a_cur_mind() > 0 {
        let d = data(game)?;
        let id = current_skill(s).context("playing skill")?;
        let commands = d.yotogi.commands.get(&id).context("skill commands")?;
        let state = usize::from(s.play.insert);
        for &i in &s.play.valid[state] {
            let c = &commands[i];
            if c.command_type == CMD_CLIMAX && st.a_cur_excite() < 80 {
                continue;
            }
            if c.active_command && (s.play.played.contains(&c.id) || !eval_bool(game, &c.expression)?) {
                continue;
            }
            let Some((_, title, items)) = s.play.groups.iter().find(|g| g.0 == c.group_name).cloned() else { continue };
            let Some(&(node, _)) = items.iter().find(|x| x.1 == i) else { continue };
            reparent(game, node, unit);
            if !game.level.as_ref().unwrap().scene.nodes[unit].children.contains(&title) {
                reparent(game, title, unit);
            }
        }
    }
    // SortOriginal: by localPosition.z.
    let level = game.level.as_mut().unwrap();
    let mut children = level.scene.nodes[unit].children.clone();
    let z = |n: &usize| level.states.get(n).and_then(|x| x.position).map_or(0., |p| p[2]);
    children.sort_by(|a, b| z(a).total_cmp(&z(b)));
    level.scene.nodes[unit].children = children;
    wf::reposition(game, unit);
    Ok(())
}

/// `Command.Data.EvalExpression`.
fn eval_bool(game: &Game, exp: &str) -> Result<bool> {
    if exp.is_empty() {
        return Ok(true);
    }
    game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.eval_bool(exp)
}

/// `YotogiPlayManager.UpdateSkillTower`.
fn update_skill_tower(game: &mut Game, s: &mut State) -> Result<()> {
    let parent = wf::node(game, &format!("{SKILL_GROUP}/SkillParent"))?;
    let d = data(game)?;
    for i in 0..SLOTS {
        let unit = game.level.as_ref().unwrap().scene.nodes[parent].children[i];
        let icon = wf::child(game, unit, "Icon")?;
        let arrow = wf::child(game, unit, "Arrow")?;
        let name = wf::child(game, unit, "SkillName")?;
        wf::state(game, name).alpha = Some(0.);
        wf::state(game, arrow).active = Some(false);
        let Some(sk) = s.skills[i].skill.and_then(|id| d.skill(id)) else {
            wf::state(game, icon).active = Some(false);
            continue;
        };
        let label = wf::child(game, name, "Val").or_else(|_| wf::child(game, name, "Label"));
        if let Ok(l) = label {
            wf::state(game, l).text = Some(sk.name.clone());
        }
        wf::state(game, icon).active = Some(true);
        let current = s.play.no == i as i32;
        if s.skills[i].is_play || current {
            let st = wf::state(game, icon);
            st.collider = Some(false);
            st.color = Some(if current { [1.; 4] } else { [0.4, 0.4, 0.4, 0.4] });
            if current {
                wf::state(game, arrow).active = Some(true);
            }
        } else {
            let st = wf::state(game, icon);
            st.collider = Some(true);
            st.color = None;
        }
        let file = if game.files.exists(&format!("{}.tex", sk.icon_file_name)) { format!("{}.tex", sk.icon_file_name) } else { "icon_none.tex".into() };
        load_sprite(game, icon, &file, Some([ICON_SIZE, ICON_SIZE]));
    }
    Ok(())
}

/// `YotogiPlayManager.ApplyExecCommandStatus`.
fn apply_command_status(game: &mut Game, s: &mut State, c: &Command) -> Result<()> {
    let correction = lock(game, |t, _| Ok(t.correction(s.maid)))?;
    let exp = with_param(game, s.maid, |p, _| {
        let level = p.st.skill_data.get(&c.skill_id).map_or(0, |x| x.exp.level);
        let st = p.st.clone();
        let ceil = |v: f32| if v >= 0. { v.ceil() as i32 } else { v.floor() as i32 };
        let a = |cmd: i32, cor: i32| {
            let mut v = (cmd as f32 * (100. + cor as f32) + ((level - 1) * 10) as f32) / 100.;
            let rate = st.a_study_rate() / 10;
            let k = 100. - ((100 - rate) / 10 * 5) as f32;
            v *= k * 0.01;
            ceil(v)
        };
        let b = |cmd: i32, cor: i32| {
            let r = if st.a_cur_reason() <= 0 { 20. } else { 0. };
            ceil(cmd as f32 * (100. + cor as f32 + st.a_frustration() as f32 + r) / 100.)
        };
        let f3 = |cmd: i32| if 0 < st.a_cur_reason() { cmd } else { ceil(cmd as f32 * 1.5 + -2.) };
        let x = &c.status;
        let k = correction.0;
        p.add_status("興奮値", b(x.excitement, k[0]));
        p.add_status("可憐", f3(x.lovely));
        p.add_status("気品", f3(x.elegance));
        p.add_status("魅惑", f3(x.charm));
        p.add_status("現在精神力", a(x.mind, k[2]));
        p.add_status("現在理性", a(x.reason, k[1]));
        p.add_status("淫欲", a(x.inyoku, k[8]));
        p.add_status("M性", a(x.m_value, k[11]));
        p.add_status("変態", a(x.hentai, k[9]));
        p.add_status("奉仕", a(x.housi, k[10]));
        p.add_status("接待", a(x.reception, k[3]));
        p.add_status("指導", a(x.teach_rate, k[12]));
        let sx = st.sexual.clone();
        for (part, cur, v) in [("口", sx.mouth, x.sexual[0]), ("喉", sx.throat, x.sexual[1]), ("乳首", sx.nipple, x.sexual[2]), ("前穴", sx.front, x.sexual[3]), ("後穴", sx.back, x.sexual[4]), ("クリ", sx.curi, x.sexual[5])] {
            if cur != 1000 {
                p.add_sexual(part, v, 999);
            }
        }
        Ok(x.skill_exp)
    })?;
    s.skills[s.play.no as usize].exp += exp;
    Ok(())
}

/// `YotogiPlayManager.OnClickCommand`.
fn on_click_command(game: &mut Game, opener: &mut Opener, s: &mut State, ci: usize) -> Result<()> {
    let d = data(game)?;
    let id = current_skill(s).context("playing skill")?;
    let c = d.yotogi.commands.get(&id).and_then(|v| v.get(ci)).context("command")?.clone();
    let skill = d.skill(id).context("skill")?.clone();
    s.play.played.insert(c.id);
    s.play.backup = PlayParam::create(&maid_status(game, s.maid)?);
    apply_command_status(game, s, &c)?;
    update_viewer(game, s.viewers[1], s.maid, true)?;
    let st = maid_status(game, s.maid)?;
    set_bar(game, 1, st.a_cur_mind(), true, s)?;
    set_bar(game, 2, st.a_cur_reason(), true, s)?;
    set_bar(game, 0, st.a_cur_excite(), true, s)?;
    let diff = PlayParam::create(&st).sub(s.play.backup);
    let list: Vec<(&str, i32)> = [
        ("興奮", diff.cur_excite),
        ("精神", diff.cur_mind),
        ("理性", diff.cur_reason),
        ("淫欲", diff.inyoku),
        ("M性", diff.m_value),
        ("変態", diff.hentai),
        ("奉仕", diff.housi),
        ("可憐", diff.lovely),
        ("気品", diff.elegance),
        ("魅惑", diff.charm),
        ("接待", diff.reception),
        ("指導", diff.teach_rate / 10),
    ]
    .into_iter()
    .filter(|x| x.1 != 0)
    .collect();
    scroll_call_fade_in(game, s, &list)?;
    eval(game, "global.__skill_command_file.clear();")?;
    clear_voice(game, s);
    let mut normal = true;
    let mut drunk = false;
    let mut propensity_arousal = false;
    let mut propensity_have = false;
    let mut sexual_arousal = false;
    let mut sexual_have = false;
    if skill.drunk_skill {
        drunk = true;
        normal = false;
    }
    if normal && c.propensity_sabun != 0 {
        if st.is_propensity(c.propensity_sabun) {
            propensity_have = true;
            normal = false;
        } else if with_param(game, s.maid, |p, _| p.is_get_possible_propensity(c.propensity_sabun))? {
            normal = false;
            propensity_arousal = true;
        }
    }
    if normal && c.sexual_type != 0 && 200 <= st.a_cur_excite() {
        let (cur, trophy, part) = match c.sexual_type {
            1 => (st.sexual.mouth, Some(430), 0),
            2 => (st.sexual.throat, Some(440), 1),
            3 => (st.sexual.nipple, Some(450), 2),
            4 => (st.sexual.front, None, 3),
            5 => (st.sexual.back, None, 4),
            _ => (st.sexual.curi, Some(480), 5),
        };
        if cur == 999 {
            with_param(game, s.maid, |p, pl| {
                let sx = &mut p.st.sexual;
                *[&mut sx.mouth, &mut sx.throat, &mut sx.nipple, &mut sx.front, &mut sx.back, &mut sx.curi][part] = 1000;
                if let Some(t) = trophy {
                    pl.add_have_trophy(t);
                }
                Ok(())
            })?;
            sexual_arousal = true;
            normal = false;
        } else if cur >= 1000 {
            sexual_have = true;
        }
    }
    let _ = normal;
    let ty = c.command_type;
    if propensity_arousal {
        set_control_mask(game, s, true, false)?;
        with_param(game, s.maid, |p, _| Ok(p.set_propensity(c.propensity_sabun, true)))?;
        call_command_file(game, opener, &c.propensity_arousal_file, "")?;
    } else if sexual_arousal {
        set_control_mask(game, s, true, false)?;
        call_command_file(game, opener, &c.seikan_arousal_file, "")?;
    } else {
        match ty {
            CMD_INSERT | CMD_CONTINUE => {
                if drunk {
                    play_drunk_continue(game, opener, s, &c)?;
                } else if propensity_have || sexual_have {
                    play_proclivity_continue(game, opener, s, &c)?;
                } else {
                    play_normal_continue(game, opener, s, &c)?;
                }
                play_rr_file(game, opener, s, &c)?;
            }
            CMD_SINGLE => {
                let label = if drunk {
                    drunk_single_label(&st, s.play.backup.cur_excite)
                } else if propensity_have || sexual_have {
                    format!("*性癖SA{}", excitement_status(s.play.backup.cur_excite) + 1)
                } else {
                    normal_single_label(&st, s.play.level, s.play.backup.cur_excite)
                };
                call_command_file(game, opener, &c.normal_file, &label)?;
                play_rr_file(game, opener, s, &c)?;
            }
            CMD_CLIMAX => {
                play_normal_climax(game, opener, s, &c)?;
                s.play.zettyo_count += 1;
                if 10 <= s.play.zettyo_count {
                    with_param(game, s.maid, |_, pl| Ok(pl.add_have_trophy(410)))?;
                }
            }
            CMD_STOP => {
                let label = format!("*YAM{}", excitement_status(maid_status(game, s.maid)?.a_cur_excite()) + 1);
                call_command_file(game, opener, &c.normal_file, &label)?;
                play_rr_file(game, opener, s, &c)?;
            }
            _ => bail!("SkillCommandType {ty}"),
        }
    }
    match ty {
        CMD_INSERT => s.play.insert = true,
        CMD_STOP => s.play.insert = false,
        CMD_CLIMAX if c.insert_release => s.play.insert = false,
        _ => {}
    }
    update_command(game, s)
}

fn play_rr_file(game: &mut Game, opener: &mut Opener, s: &mut State, c: &Command) -> Result<()> {
    let excite = maid_status(game, s.maid)?.a_cur_excite();
    call_command_file(game, opener, &c.rr_file, &format!("*{}", rr_type(excite)))
}

fn play_insert(game: &mut Game, opener: &mut Opener, s: &State, c: &Command) -> Result<()> {
    let label = format!("*SOU{}", excitement_status(s.play.backup.cur_excite) + 1);
    call_command_file(game, opener, &c.normal_file, &label)
}

fn play_insert2(game: &mut Game, opener: &mut Opener, s: &State, c: &Command) -> Result<()> {
    let label = format!("*SOC{}", excitement_status(s.play.backup.cur_excite) + 1);
    call_command_file(game, opener, &c.insert_file, &label)
}

/// `PlayNormalContinue`.
fn play_normal_continue(game: &mut Game, opener: &mut Opener, s: &mut State, c: &Command) -> Result<()> {
    if !s.play.insert {
        play_insert(game, opener, s, c)?;
    }
    let st = maid_status(game, s.maid)?;
    let (before, cur) = (s.play.backup.cur_excite, st.a_cur_excite());
    let mut file = String::new();
    let mut label = String::new();
    if let Some(rc) = rc_type(before, cur) {
        file = c.rc_file.clone();
        let cond = st.condition;
        label = if cond == 5 {
            format!("*愛奴{rc}")
        } else if s.play.level == 3 {
            if cond == 4 { format!("*慣れ恋人{rc}") } else { format!("*緊張orお近づきor信頼{rc}") }
        } else if s.play.level != 2 {
            match cond {
                1 => format!("*不慣れ緊張{rc}"),
                2 => format!("*不慣れお近づき{rc}"),
                4 => format!("*不慣れ恋人{rc}"),
                3 => format!("*フラット、変化進行{rc}"),
                _ => String::new(),
            }
        } else {
            format!("*フラット、変化進行{rc}")
        };
    } else {
        if s.play.insert {
            play_insert2(game, opener, s, c)?;
        }
        if let Some(rrc) = rrc_type(before, cur) {
            file = c.rrc_file.clone();
            label = if s.play.level == 1 || s.play.level == 2 { format!("*不慣れ{rrc}") } else { format!("*慣れ{rrc}") };
        }
    }
    if !file.is_empty() {
        call_command_file(game, opener, &file, &label)?;
    }
    Ok(())
}

/// `PlayDrunkContinue`.
fn play_drunk_continue(game: &mut Game, opener: &mut Opener, s: &mut State, c: &Command) -> Result<()> {
    if !s.play.insert {
        play_insert(game, opener, s, c)?;
    }
    let st = maid_status(game, s.maid)?;
    if let Some(rc) = rc_type(s.play.backup.cur_excite, st.a_cur_excite()) {
        let label = match st.condition {
            1 => format!("*酔い緊張{rc}"),
            2 => format!("*酔いお近づき{rc}"),
            3 => format!("*酔い信頼{rc}"),
            4 => format!("*酔い恋人{rc}"),
            5 => format!("*酔い愛奴{rc}"),
            _ => String::new(),
        };
        call_command_file(game, opener, &c.rc_file, &label)?;
    } else if s.play.insert {
        play_insert2(game, opener, s, c)?;
    }
    Ok(())
}

/// `PlayProclivityContinue`.
fn play_proclivity_continue(game: &mut Game, opener: &mut Opener, s: &mut State, c: &Command) -> Result<()> {
    if !s.play.insert {
        play_insert(game, opener, s, c)?;
    }
    let st = maid_status(game, s.maid)?;
    let (before, cur) = (s.play.backup.cur_excite, st.a_cur_excite());
    if let Some(rc) = rc_type(before, cur) {
        call_command_file(game, opener, &c.rc_file, &format!("*性癖{rc}"))?;
    } else {
        if s.play.insert {
            play_insert2(game, opener, s, c)?;
        }
        if let Some(rrc) = rrc_type(before, cur) {
            call_command_file(game, opener, &c.rrc_file, &format!("*性癖{rrc}"))?;
        }
    }
    Ok(())
}

/// `PlayNormalClimax`.
fn play_normal_climax(game: &mut Game, opener: &mut Opener, s: &mut State, c: &Command) -> Result<()> {
    let before = excitement_status(s.play.backup.cur_excite);
    let after = excitement_status(maid_status(game, s.maid)?.a_cur_excite());
    anyhow::ensure!(before != 0, "PlayNormalClimax error.");
    call_command_file(game, opener, &c.normal_file, &format!("*SYA{before}"))?;
    call_command_file(game, opener, &c.rr_file, &format!("*ZIG{}", after.max(1)))
}

/// `PlayNormalSingle` labels.
fn normal_single_label(st: &MaidStatus, level: i32, before: i32) -> String {
    let e = excitement_status(before) as i32 + 1;
    let base = if st.condition == 5 {
        20
    } else if level == 1 || level == 2 {
        if st.condition == 4 {
            4
        } else if level == 2 || st.condition == 3 {
            8
        } else {
            0
        }
    } else if level == 3 {
        if st.condition == 4 { 16 } else { 12 }
    } else {
        return String::new();
    };
    format!("*SA{}", base + e)
}

/// `PlayDrunkSingle` labels.
fn drunk_single_label(st: &MaidStatus, before: i32) -> String {
    let e = excitement_status(before) as i32 + 1;
    let base = match st.condition {
        1 | 2 => 0,
        3 => 4,
        4 | 5 => 8,
        _ => return String::new(),
    };
    format!("*酔いSA{}", base + e)
}

/// `YotogiPlayManager.Update` once the screen waits.
fn play_update(wf: &mut Manager, game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    if s.play.control_mask_until.is_some_and(|t| t <= game.now) {
        set_control_mask(game, s, false, false)?;
    }
    if !s.play.repeat_voice.is_empty() && !game.voice_playing() {
        let dt = game.now.saturating_sub(game.last_now) as i32;
        s.play.repeat_time += dt;
        if REPEAT_WAIT_MS <= s.play.repeat_time {
            s.play.repeat_time = 0;
            game.sound_ops.push(SoundOp::Play { kind: Kind::Voice, file: s.play.repeat_voice.clone(), fade_s: 0., looping: true });
        }
    }
    if !s.play.is_swoon || !game.fade3d.done(game.now) {
        return Ok(());
    }
    with_param(game, s.maid, |_, pl| Ok(pl.add_have_trophy(350)))?;
    call_normal_file(game, opener, "?_YOTOGI_SYSTEM_KIZETU.ks", "")?;
    game.show_system_dialog("気絶しました。\n夜伽を終了します。", "SystemDialog.Ok")?;
    s.dialog = Some("swoon");
    s.play.is_swoon = false;
    let _ = wf;
    Ok(())
}

fn is_swoon(game: &Game, s: &State) -> Result<bool> {
    let st = maid_status(game, s.maid)?;
    Ok(st.a_cur_mind() <= 0 && st.a_cur_hp() <= 0)
}

/// `YotogiPlayManager.OnNextSkillMove`.
fn on_next_skill_move(wf: &mut Manager, game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    if s.play.is_swoon {
        return Ok(());
    }
    game.yotogi_kag.enabled = false;
    s.play.is_swoon = is_swoon(game, s)?;
    if s.play.is_swoon || s.play.no == -1 {
        if maid_status(game, s.maid)?.a_cur_hp() <= 0 {
            s.play.is_swoon = true;
        }
        if s.play.is_swoon {
            set_control_mask(game, s, true, true)?;
            game.adv_ui(AdvUi::Fade3d { to: 1., seconds: 0.5 }, opener)?;
            return Ok(());
        }
        eval(game, "global.__skill_command_file.clear();")?;
        s.play.repeat_voice.clear();
        clear_voice(game, s);
        if !s.next_label.is_empty() {
            wf.finish(game, opener, s, "Play")?;
        }
    } else {
        s.play.next_flag = true;
        wf.finish(game, opener, s, "Play")?;
    }
    Ok(())
}

fn play_on_finish(wf: &mut Manager, game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    if !s.play.next_flag {
        apply_status(game, s)?;
        if is_swoon(game, s)? {
            game.adv_ui(AdvUi::Fade3d { to: 0., seconds: 0. }, opener)?;
        }
        if !s.next_label.is_empty() {
            wf.call_screen(game, opener, s, "Null")?;
        }
    } else {
        game.adv_ui(AdvUi::AllPos { position: Some([0.; 3]), rotation: Some([0.; 3]) }, opener)?;
        wf.call_screen(game, opener, s, "Play")?;
    }
    Ok(())
}

/// `YotogiManager.ApplyStatus`.
fn apply_status(game: &mut Game, s: &mut State) -> Result<()> {
    let d = data(game)?;
    let skills = s.skills.clone();
    with_param(game, s.maid, |p, _| {
        let mut class_exp = 0;
        for ps in &skills {
            let Some(id) = ps.skill else { continue };
            let Some(sk) = d.skill(id) else { continue };
            if ps.is_play {
                let sd = p.st.skill_data.get_mut(&id).with_context(|| format!("未取得のスキルID[{id}]のデータを参照しようとしました"))?;
                sd.play_count += 1;
                sd.exp.add(ps.exp);
                class_exp += sk.add_yotogi_class_exp;
                let flag = format!("夜伽_カテゴリー_実行回数_{}", sk.category);
                let v = p.st.generic_flag.get(&flag).copied().unwrap_or(0);
                p.st.generic_flag.insert(flag, v + 1);
            }
        }
        p.add_status("夜伽プレイ回数", 1);
        let cur = p.st.current_yotogi_class as usize;
        if let Some(c) = p.st.yotogi_class.get_mut(cur) {
            c.exp.add(class_exp);
        }
        p.update_class_bonus()?;
        p.add_status("習得率", -50);
        for cat in &d.skills {
            for (id, sk) in cat {
                if !p.st.skill_data.contains(id) && p.is_check_get_skill(sk, true, true) {
                    p.set_new_get_skill(*id);
                }
            }
        }
        Ok(())
    })?;
    for ps in &mut s.skills {
        ps.exp = 0;
    }
    Ok(())
}

/// `YotogiManager.ResetWorld`.
fn reset_world(game: &mut Game, opener: &mut Opener, s: &State) -> Result<()> {
    let _ = s;
    game.adv_ui(AdvUi::DelPrefabBg(None), opener)?;
    for c in game.characters.iter_mut() {
        if !c.man {
            c.bust_move = [1., 1.];
        }
        c.ik.remove("左手");
        c.ik.remove("右手");
    }
    const PROPS: [&str; 9] = ["handitem", "kousoku_upper", "kousoku_lower", "skirt", "panz", "mizugi", "onepiece", "accanl", "accvag"];
    for (man, list) in [(false, game.active_maids.to_vec()), (true, game.active_men.to_vec())] {
        for (slot, c) in list.into_iter().enumerate() {
            if !c.is_some_and(|i| game.characters[i].visible) {
                continue;
            }
            if !man {
                // The original opens/closes maid_ (slot 0), not each maid.
                game.adv_ui(AdvUi::OpenMouth { man: false, slot: 0, open: false }, opener)?;
            }
            game.adv_ui(AdvUi::DelPrefabChara { man, slot, name: None }, opener)?;
            game.adv_ui(AdvUi::ItemReset { man, slot, props: PROPS.iter().map(|p| (*p).to_owned()).collect() }, opener)?;
            game.adv_ui(AdvUi::AllProcProp { man, slot }, opener)?;
        }
    }
    Ok(())
}

// ---- Undressing -----------------------------------------------------------

fn init_undressing(game: &mut Game, s: &mut State) -> Result<()> {
    let parent = wf::node(game, UNDRESS)?;
    let d = data(game)?;
    let children = game.level.as_ref().unwrap().scene.nodes[parent].children.clone();
    s.play.undressing.clear();
    let maid = game.characters.get(s.maid).cloned().unwrap_or_default();
    for n in children {
        let name = game.level.as_ref().unwrap().scene.nodes[n].name.clone();
        let Some(unit) = name.strip_prefix("UndressUnit") else { continue };
        let data = d.yotogi.undressing.iter().find(|u| u.unit_name == unit).with_context(|| format!("{name}\nデータを特定できませんでした"))?;
        let enabled = data.target_slots.is_empty() || data.target_slots.iter().any(|t| maid.slots.iter().any(|(k, v)| k.eq_ignore_ascii_case(t) && v.model.is_some()));
        let st = wf::state(game, n);
        st.enabled = Some(enabled);
        st.color = None;
        s.play.undressing.push((n, unit.to_owned(), false));
    }
    Ok(())
}

/// `UndressingUnit.OnClickEvent`; `call` runs the click event.
fn undress_toggle(game: &mut Game, opener: &mut Opener, s: &mut State, node: usize, call: bool) -> Result<()> {
    let d = data(game)?;
    let Some(k) = s.play.undressing.iter().position(|u| u.0 == node) else { return Ok(()) };
    let unit = s.play.undressing[k].1.clone();
    let data = d.yotogi.undressing.iter().find(|u| u.unit_name == unit).context("undressing data")?.clone();
    if let Some(m) = game.characters.get_mut(s.maid) {
        for t in &data.target_slots {
            let loaded = m.slots.iter().any(|(k, v)| k.eq_ignore_ascii_case(t) && v.model.is_some());
            if loaded {
                let key = crate::chara::SLOTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)).map(|(n, _)| *n).unwrap_or("");
                let visible = m.force_hide.get(key).copied().unwrap_or(true);
                m.set_mask(t, !visible)?;
                break;
            }
        }
    }
    s.play.undressing[k].2 = !s.play.undressing[k].2;
    let on = s.play.undressing[k].2;
    let base = game.level.as_ref().unwrap().scene.script(node, "UIButton").and_then(|b| b.floats("mDefaultColor").or_else(|| b.floats("defaultColor")).map(|c| [c[0], c[1], c[2], c[3]]));
    if on {
        let mut c = base.unwrap_or([1.; 4]);
        c[3] = 0.5;
        wf::state(game, node).color = Some(c);
        if call {
            undressing_event(game, opener, s, &data)?;
        }
    } else {
        wf::state(game, node).color = None;
    }
    Ok(())
}

fn undress_click(game: &mut Game, opener: &mut Opener, s: &mut State, node: usize) -> Result<()> {
    undress_toggle(game, opener, s, node, true)
}

/// `OnClickEventAllOFF` (`on` false) / `OnClickEventAllON`.
fn undress_all(game: &mut Game, opener: &mut Opener, s: &mut State, node: usize, on: bool) -> Result<()> {
    let d = data(game)?;
    let me = s.play.undressing.iter().find(|u| u.0 == node).map(|u| u.1.clone()).unwrap_or_default();
    if on {
        game.adv_ui(AdvUi::ItemMaskMode { man: false, slot: 0, mode: "None".into() }, opener)?;
    }
    let mut i = 0;
    while let Some(k) = s.play.undressing.iter().position(|u| u.1 == i.to_string()) {
        let (n, _, undressing) = s.play.undressing[k].clone();
        let enabled = game.level.as_ref().unwrap().states.get(&n).and_then(|x| x.enabled).unwrap_or(true);
        if enabled {
            if !on && !undressing {
                undress_toggle(game, opener, s, n, false)?;
            } else if on && undressing {
                s.play.undressing[k].2 = false;
                wf::state(game, n).color = None;
            }
        }
        i += 1;
    }
    if !on {
        // The maid of this unit: SetMaskMode on the yotogi maid.
        if let Some(m) = game.characters.get_mut(s.maid) {
            m.set_mask_mode("Nude")?;
        }
    }
    if let Some(data) = d.yotogi.undressing.iter().find(|u| u.unit_name == me).cloned() {
        undressing_event(game, opener, s, &data)?;
    }
    Ok(())
}

/// `YotogiPlayManager.OnUndressingEvent`.
fn undressing_event(game: &mut Game, opener: &mut Opener, s: &mut State, data: &Undressing) -> Result<()> {
    if !data.call_scenario.is_empty() && maid_status(game, s.maid)?.condition_special != 1 {
        call_command_file(game, opener, &data.call_scenario, "")?;
    }
    Ok(())
}

// ---- ParamScroll ----------------------------------------------------------

/// Parent, base, hide and end positions of `YotogiParamScroll`.
fn scroll_geometry(game: &Game) -> Result<(usize, [f32; 3], [f32; 3], [f32; 3])> {
    let view = wf::node(game, PARAM_VIEW)?;
    let parent = wf::child(game, view, "Parent")?;
    let mask = wf::child(game, view, "Mask")?;
    let level = game.level.as_ref().unwrap();
    let base = level.scene.nodes[parent].position;
    let clip = level.scene.script(mask, "UIPanel").and_then(|f| f.floats("mClipRange").map(|c| [c[0], c[1], c[2], c[3]])).unwrap_or([0.; 4]);
    Ok((parent, base, [base[0] - clip[2], base[1], base[2]], [base[0], base[1] + clip[3], base[2]]))
}

fn scroll_labels(game: &Game, parent: usize) -> Vec<usize> {
    game.level.as_ref().unwrap().scene.nodes[parent].children.clone()
}

/// `YotogiParamScroll.Init`.
fn scroll_init(game: &mut Game, s: &mut State) -> Result<()> {
    let (parent, _, hide, _) = scroll_geometry(game)?;
    for l in scroll_labels(game, parent) {
        wf::state(game, l).alpha = Some(0.);
    }
    s.tweens.retain(|t| t.node != parent);
    wf::state(game, parent).position = Some(hide);
    s.play.scroll = Scroll::Null;
    Ok(())
}

/// `YotogiParamScroll.CallFadeIn`.
fn scroll_call_fade_in(game: &mut Game, s: &mut State, list: &[(&str, i32)]) -> Result<()> {
    let (parent, _, hide, _) = scroll_geometry(game)?;
    let labels = scroll_labels(game, parent);
    for (i, l) in labels.iter().enumerate() {
        let st = wf::state(game, *l);
        match list.get(i) {
            Some((name, v)) => {
                st.alpha = Some(1.);
                st.text = Some(format!("{name}\u{3000}{v}"));
            }
            None => st.alpha = Some(0.),
        }
    }
    s.tweens.retain(|t| t.node != parent);
    wf::state(game, parent).position = Some(hide);
    s.play.scroll = Scroll::CallIn;
    Ok(())
}

fn scroll_update(game: &mut Game, s: &mut State) -> Result<()> {
    let (parent, base, hide, end) = scroll_geometry(game)?;
    let now = game.now;
    match s.play.scroll {
        Scroll::CallIn => {
            tween(s, now, parent, SCROLL_MOVE_S, Ease::OutSine, TweenKind::Scroll { from: hide, to: base });
            s.play.scroll = Scroll::FadeIn(now);
        }
        Scroll::Wait(until) if until <= now => {
            tween(s, now, parent, SCROLL_MOVE_S, Ease::OutSine, TweenKind::Scroll { from: base, to: end });
            s.play.scroll = Scroll::FadeOut(now);
        }
        _ => {}
    }
    Ok(())
}

/// `OnEndFadeIn` / `OnEndFadeOut`.
fn scroll_tween_done(game: &mut Game, s: &mut State) -> Result<()> {
    match s.play.scroll {
        Scroll::FadeIn(_) => s.play.scroll = Scroll::Wait(game.now + SCROLL_WAIT_MS),
        Scroll::FadeOut(_) => {
            let (parent, _, hide, _) = scroll_geometry(game)?;
            wf::state(game, parent).position = Some(hide);
            s.play.scroll = Scroll::Null;
        }
        _ => {}
    }
    Ok(())
}

// ---- Result ---------------------------------------------------------------

fn result_on_call(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    wf::set_click(game, &format!("{RESULT}/CommonPanel/UnderButtonGroup/Next"), "YotogiResultManager.OnClickNext")?;
    let fluct = wf::node(game, &format!("{RESULT}/AcquiredParameterViewer/FluctuationOfParameter/FluctuationOfParameterParent"))?;
    let acq_skill = wf::node(game, &format!("{RESULT}/AcquiredParameterViewer/AcquiredSkill/AcquiredSkillParent"))?;
    let acq_attr = wf::node(game, &format!("{RESULT}/AcquiredParameterViewer/AcquiredAttribute/AcquiredAttributeParent"))?;
    let units = wf::node(game, &format!("{RESULT}/SkillViewer/SkillUnitParent"))?;
    for p in [fluct, acq_skill, acq_attr, units] {
        wf::clear_children(game, p);
    }
    update_viewer(game, s.viewers[2], s.maid, true)?;
    let d = data(game)?;
    let st = maid_status(game, s.maid)?;
    let now = game.now;
    let mut seen = BTreeSet::new();
    for ps in s.skills.clone() {
        let Some(id) = ps.skill else { continue };
        if !seen.insert(id) || game.level.as_ref().unwrap().scene.nodes[units].children.len() >= SLOTS {
            continue;
        }
        let unit = wf::create_prefab(game, opener, units, "SceneYotogi/SkillSelect/Prefab/SkillUnit")?;
        let b = wf::child(game, unit, "Button")?;
        wf::state(game, b).active = Some(false);
        let bg = wf::child(game, unit, "Description/BG")?;
        wf::state(game, bg).alpha = Some(1.);
        set_skill_unit(game, &d, &st, unit, id)?;
        let total = st.skill_data.get(&id).map_or(0, |x| x.exp.total);
        let add = total - ps.backup_total_exp;
        if 0 < add {
            // SetExpAnime: the gauge restarts from the backup total.
            let mut exp = crate::params::Exp { total: ps.backup_total_exp, ..Default::default() };
            exp.set_list(d.skill(id).map(|k| k.exp_table.clone()).unwrap_or_default());
            update_gauge(game, unit, &exp)?;
            s.tweens.push(Tween { node: unit, start: now, seconds: RESULT_ANIME_S, ease: Ease::OutExpo, kind: TweenKind::SkillExp { skill: id, from: ps.backup_total_exp, to: total, level: exp.level } });
        }
    }
    let diff = PlayParam::create(&st).sub(s.backup_param);
    let mut count = 0;
    for (name, v) in [
        ("夜伽クラスレベル", diff.yotogi_type_level),
        ("夜伽クラスEXP", diff.yotogi_type_total_exp),
        ("奉仕", diff.housi),
        ("淫欲", diff.inyoku),
        ("M性", diff.m_value),
        ("変態", diff.hentai),
        ("可憐", diff.lovely),
        ("気品", diff.elegance),
        ("魅惑", diff.charm),
        ("指導", diff.teach_rate / 10),
    ] {
        if count >= 14 || v == 0 {
            continue;
        }
        let u = wf::create_prefab(game, opener, fluct, "SceneYotogi/Result/Prefab/FluctuationOfParameterUnit")?;
        count += 1;
        let n = wf::child(game, u, "Name")?;
        wf::state(game, n).text = Some(name.into());
        let val = wf::child(game, u, "Value")?;
        wf::state(game, val).text = Some("0".into());
        let bg = wf::child(game, u, "BGGray")?;
        wf::state(game, bg).active = Some((count + 1) / 2 % 2 == 0);
        s.tweens.push(Tween { node: val, start: now, seconds: RESULT_ANIME_S, ease: Ease::OutExpo, kind: TweenKind::Count { from: 0, to: v } });
    }
    for p in st.propensity.iter().map(|(k, _)| k).filter(|p| !s.backup_propensity.contains(p)).take(5) {
        let u = wf::create_prefab(game, opener, acq_attr, "SceneYotogi/Result/Prefab/AcquiredAttributeUnit")?;
        let n = wf::child(game, u, "Name")?;
        let text = crate::params::PROPENSITY.get(*p as usize).map(|t| t.replace('二', "2")).unwrap_or_default();
        wf::state(game, n).text = Some(text);
    }
    let mut k = 0;
    for (id, _) in st.skill_data.iter() {
        if s.backup_skills.contains(id) || k >= 8 {
            continue;
        }
        let Some(sk) = d.skill(*id) else { continue };
        if sk.drunk_skill {
            continue;
        }
        let u = wf::create_prefab(game, opener, acq_skill, "SceneYotogi/Result/Prefab/AcquiredSkillUnit")?;
        k += 1;
        let n = wf::child(game, u, "Name")?;
        wf::state(game, n).text = Some(sk.name.clone());
        let bg = wf::child(game, u, "BGGray")?;
        wf::state(game, bg).active = Some(k % 2 != 0);
    }
    for p in [fluct, acq_skill, acq_attr, units] {
        wf::reposition(game, p);
    }
    if 0 < st.a_cur_mind() {
        let total = diff.inyoku + diff.housi + diff.hentai + diff.m_value;
        let file = if st.a_frustration() < 50 {
            (400 <= total).then_some("?_GIG_0001.ks")
        } else {
            (total < 400).then_some("?_GIG_0002.ks")
        };
        if let Some(f) = file {
            eval(game, &format!("global.__skill_first_file = '{f}';"))?;
            eval(game, "global.__skill_first_file_label = '';")?;
            load_yotogi_script(game, opener, "yotogi_skill_call_main.ks")?;
        }
    }
    // PlayerParam.CheckTrophyFromYotogiResult.
    with_param(game, s.maid, |p, pl| {
        let st = &*p.st;
        let mut t = Vec::new();
        if 50 <= st.yotogi_play_count {
            t.push(240);
        }
        if 100 <= st.yotogi_play_count {
            t.push(250);
        }
        for (v, id) in [(st.a_inyoku(), 310), (st.a_m_value(), 320), (st.a_hentai(), 330), (st.a_housi(), 340)] {
            if 9999 <= v {
                t.push(id);
            }
        }
        if !pl.st.have_trophy.contains(&370) && st.skill_data.iter().any(|(_, x)| 3 <= x.exp.level) {
            t.push(370);
        }
        if st.seikeiken == 0 && [st.a_inyoku(), st.a_m_value(), st.a_hentai(), st.a_housi()].iter().all(|v| 9999 <= *v) {
            t.push(490);
        }
        for id in t {
            pl.add_have_trophy(id);
        }
        Ok(())
    })
}

fn result_on_finish(game: &mut Game, opener: &mut Opener, s: &mut State) -> Result<()> {
    let _ = s;
    voice_stop_all(game);
    for kag in game.motion_kags.values_mut() {
        kag.clear_motion();
    }
    for slot in 0..game.active_maids.len() {
        if game.active_maids[slot].is_none() {
            continue;
        }
        game.adv_ui(AdvUi::Face { man: false, slot, name: "通常".into() }, opener)?;
        game.adv_ui(AdvUi::FaceBlend { man: false, slot, name: "無し".into() }, opener)?;
        game.adv_ui(AdvUi::ItemMaskMode { man: false, slot, mode: "None".into() }, opener)?;
        if let Some(c) = game.chara(false, slot) {
            for (body, mat) in [("body", 0), ("head", 5)] {
                for prop in ["_MainTex", "_ShadowTex"] {
                    c.tex_mul_del(body, mat, prop);
                }
            }
        }
    }
    reset_world(game, opener, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excitement_classifiers_follow_yotogi_play() {
        assert_eq!([-1, 0, 99, 100, 199, 200].map(excitement_status), [0, 1, 1, 2, 2, 3]);
        assert_eq!(rc_type(10, -5), Some("RC_1"));
        assert_eq!(rc_type(150, 250), Some("RC2"));
        assert_eq!(rc_type(50, 120), Some("RC1"));
        assert_eq!(rc_type(-20, 5), Some("RC0"));
        assert_eq!(rc_type(10, 40), None);
        // Thresholds every 25 skipping 0/100/200; only rises cross positive ones.
        assert_eq!(rrc_type(0, 30), Some("RRC1"));
        assert_eq!(rrc_type(-10, -60), Some("RRC_2"));
        assert_eq!(rrc_type(50, 20), None);
        assert_eq!(rrc_type(-100, -100), Some("RRC_4"));
        assert_eq!(rrc_type(300, 300), Some("RRC10"));
        assert_eq!([-100, -1, 0, 49, 50, 299].map(rr_type), ["RR_2", "RR_1", "RR1", "RR1", "RR2", "RR6"]);
    }

    #[test]
    fn single_command_labels() {
        let mut st = MaidStatus { condition: 1, ..Default::default() };
        assert_eq!(normal_single_label(&st, 1, -5), "*SA1");
        st.condition = 4;
        assert_eq!(normal_single_label(&st, 2, 150), "*SA7");
        st.condition = 3;
        assert_eq!(normal_single_label(&st, 3, 250), "*SA16");
        st.condition = 5;
        assert_eq!(normal_single_label(&st, 1, 0), "*SA22");
        assert_eq!(drunk_single_label(&st, 120), "*酔いSA11");
    }
}
