// SPDX-License-Identifier: MPL-2.0
//! Static yotogi tables: `Yotogi.StageData` (`yotogi_stage_list.nei` +
//! `yotogi_stage_setting.nei`), `Yotogi.SkillData.Command` per skill
//! (`yotogi_skil_command_data.nei` + `yotogi_skil_command_status.nei`),
//! `YotogiSkillSelectWaitMotion` and `UndressingUnit.UndressingData`.
use crate::{
    gamedata::GameData,
    nei::Table,
    params::{parse_enum, PERSONAL, PROPENSITY},
};
use anyhow::{bail, ensure, Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet};

/// `Yotogi.Stage` in enum order.
pub const STAGES: &[&str] = &[
    "サロン",
    "主人公の部屋",
    "プレイルーム",
    "プール",
    "初代バスルーム",
    "ＳＭルーム",
    "プレイルーム2",
    "中庭",
    "大浴場",
    "メイド部屋",
    "花魁風プレイルーム",
    "新ペントハウス",
    "屋外",
];

/// `Yotogi.SexualType`.
pub const SEXUAL_TYPES: &[&str] = &["Null", "Mouth", "Throat", "Nipple", "Front", "Back", "Curi"];
/// `Yotogi.SkillCommandType`.
pub const COMMAND_TYPES: &[&str] = &["挿入", "継続", "単発", "絶頂", "止める"];
pub const CMD_INSERT: i32 = 0;
pub const CMD_CONTINUE: i32 = 1;
pub const CMD_SINGLE: i32 = 2;
pub const CMD_CLIMAX: i32 = 3;
pub const CMD_STOP: i32 = 4;
/// `param.Condition` / `param.ConditionSpecial` enum names.
pub const CONDITION: &[&str] = &["Null", "Tonus", "Contact", "Trust", "Lover", "Slave"];
pub const CONDITION_SPECIAL: &[&str] = &["Null", "Drunk"];

/// `Yotogi.StageData`.
#[derive(Clone, Debug)]
pub struct StageData {
    pub sort_id: i32,
    pub draw_name: String,
    /// `Yotogi.Stage` index.
    pub stage: usize,
    pub stage_name: String,
    pub need_grade: i32,
    pub thumbnail_name: String,
    pub prefab_name: String,
    pub bgm_file: String,
    pub select_camera_center: [f32; 3],
    pub select_camera_radius: f32,
    pub select_camera_rotate: [f32; 2],
    pub skill_chara_pos: [f32; 3],
    pub skill_chara_rotate: [f32; 3],
    pub skill_camera_rotate: [f32; 2],
}

/// `Yotogi.SkillData.Command.Data` (`Basic` and `Status`).
#[derive(Clone, Debug, Default)]
pub struct Command {
    pub skill_id: i32,
    /// The setting table row (unique within the skill).
    pub id: i32,
    pub name: String,
    pub group_name: String,
    pub command_type: i32,
    pub request_skill_lv: i32,
    pub normal_file: String,
    pub insert_file: String,
    pub rc_file: String,
    pub rrc_file: String,
    pub rr_file: String,
    pub sexual_type: i32,
    pub seikan_arousal_file: String,
    pub propensity_sabun: i32,
    pub propensity_arousal_file: String,
    pub insert_release: bool,
    pub request_insert: bool,
    pub active_command: bool,
    pub expression: String,
    pub status: CommandStatus,
}

/// `Command.Data.Status`.
#[derive(Clone, Debug, Default)]
pub struct CommandStatus {
    pub excitement: i32,
    pub mind: i32,
    pub reason: i32,
    pub skill_exp: i32,
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
    /// mouth, throat, nipple, front, back, curi.
    pub sexual: [i32; 6],
}

/// `YotogiSkillSelectWaitMotion.Data`.
#[derive(Clone, Debug, Default)]
pub struct WaitMotion {
    pub condition: i32,
    pub condition_sp: i32,
    pub motion_script_file: String,
    pub motion_script_label: String,
    pub face_blend_name: String,
    pub face_name: String,
}

/// `UndressingUnit.UndressingData`.
#[derive(Clone, Debug, Default)]
pub struct Undressing {
    pub unit_name: String,
    pub name: String,
    pub target_slots: Vec<String>,
    pub call_scenario: String,
}

#[derive(Clone, Debug, Default)]
pub struct YotogiData {
    /// `Yotogi.stage_data_list` by stage index.
    pub stages: BTreeMap<usize, StageData>,
    /// Commands of each listed skill.
    pub commands: HashMap<i32, Vec<Command>>,
    pub wait_motions: HashMap<i32, Vec<WaitMotion>>,
    pub undressing: Vec<Undressing>,
}

impl YotogiData {
    pub fn load(d: &GameData) -> Result<Self> {
        let list = d.table("yotogi_skil_list.nei")?;
        let ids: HashSet<i32> = (1..list.rows).filter(|y| list.exists(0, *y)).map(|y| list.int(0, y)).collect();
        let mut out = Self::default();
        // Yotogi.CreateYotogiData: command block rows per listed skill.
        let blocks = |t: &Table| -> Result<HashMap<i32, usize>> {
            let mut m = HashMap::new();
            let mut j = 0;
            while j < t.rows {
                if t.exists(0, j) {
                    ensure!(t.exists(0, j + 1), "{}のスキルID設定がありません", t.string(0, j));
                    let id = t.int(0, j + 1);
                    if ids.contains(&id) {
                        m.insert(id, j);
                    }
                    j += 1;
                }
                j += 1;
            }
            Ok(m)
        };
        let setting = d.table("yotogi_skil_command_data.nei")?;
        let status = d.table("yotogi_skil_command_status.nei")?;
        let basic_rows = blocks(setting)?;
        let status_rows = blocks(status)?;
        for (id, by) in &basic_rows {
            let sy = *status_rows.get(id).with_context(|| format!("yotogi skill {id}: command status block missing"))?;
            out.commands.insert(*id, commands(setting, status, *id, *by, sy).with_context(|| format!("yotogi skill {id} commands"))?);
        }
        let stage_list = d.table("yotogi_stage_list.nei")?;
        let listed: HashSet<usize> = (1..stage_list.rows)
            .filter(|y| stage_list.exists(0, *y))
            .filter_map(|y| STAGES.iter().position(|s| *s == stage_list.string(0, y)))
            .collect();
        let t = d.table("yotogi_stage_setting.nei")?;
        for y in 1..t.rows {
            if t.exists(0, y) {
                let s = stage(t, y)?;
                if listed.contains(&s.stage) {
                    out.stages.insert(s.stage, s);
                }
            }
        }
        let t = d.table("yotogi_skil_select_wait_motion.nei")?;
        for y in 1..t.rows {
            let personal = parse_enum(PERSONAL, &t.string(0, y)).context("YotogiSkillSelectWaitMotion personal enum parse error.")?;
            let mut m = WaitMotion::default();
            let mut x = 1;
            if t.exists(x, y) {
                m.condition = parse_enum(CONDITION, &t.string(x, y)).context("Condition enum parse error.")?;
                x += 2;
            } else if t.exists(x + 1, y) {
                x += 1;
                m.condition_sp = parse_enum(CONDITION_SPECIAL, &t.string(x, y)).context("ConditionSpecial enum parse error.")?;
                x += 1;
            } else {
                bail!("yotogi_skil_select_wait_motion.nei row {y}: no condition");
            }
            m.motion_script_file = format!("{}.ks", t.string(x, y));
            m.motion_script_label = t.string(x + 1, y);
            m.face_blend_name = t.string(x + 2, y);
            m.face_name = t.string(x + 3, y);
            out.wait_motions.entry(personal).or_default().push(m);
        }
        let t = d.table("yotogi_undressing.nei")?;
        for y in 1..t.rows {
            if !t.exists(0, y) {
                continue;
            }
            let slots = t.string(2, y);
            let target_slots = if slots.is_empty() { Vec::new() } else { slots.split(',').map(str::to_owned).collect() };
            for s in &target_slots {
                ensure!(crate::chara::SLOTS.iter().any(|(n, _)| n == s), "TBody.SlotID enum parse error.\n{s}");
            }
            out.undressing.push(Undressing { unit_name: t.string(0, y), name: t.string(1, y), target_slots, call_scenario: t.string(3, y) });
        }
        Ok(out)
    }

    /// `Yotogi.GetPlayableStageList(grade)`: (stage, enabled) by sort id.
    pub fn playable_stages(&self, grade: i32) -> Vec<(&StageData, bool)> {
        let mut v: Vec<_> = self.stages.values().map(|s| (s, s.need_grade <= grade)).collect();
        v.sort_by_key(|(s, _)| s.sort_id);
        v
    }

    /// `Yotogi.GetStageData(bg_prefab_name)`.
    pub fn stage_by_prefab(&self, prefab: &str) -> Option<&StageData> {
        self.stages.values().find(|s| s.prefab_name == prefab)
    }

    /// `YotogiSkillSelectWaitMotion.GetData`.
    pub fn wait_motion(&self, personal: i32, condition: i32, condition_sp: i32) -> Result<&WaitMotion> {
        let list = self.wait_motions.get(&personal).context("YotogiSkillSelectWaitMotion: personal missing")?;
        let hit = if condition_sp == 0 { list.iter().find(|m| m.condition == condition) } else { list.iter().find(|m| m.condition_sp == condition_sp) };
        hit.filter(|m| m.motion_script_file != ".ks").context("YotogiSkillSelectWaitMotion: no motion")
    }
}

fn floats<const N: usize>(text: &str, what: &str) -> Result<[f32; N]> {
    let v: Vec<f32> = text.split(',').map(|s| s.trim().parse::<f32>()).collect::<Result<_, _>>().with_context(|| format!("{what}: {text}"))?;
    ensure!(v.len() == N, "{what}");
    let mut a = [0.; N];
    a.copy_from_slice(&v);
    Ok(a)
}

/// `Yotogi.StageData(csv, y)`.
fn stage(t: &Table, y: usize) -> Result<StageData> {
    let stage_name = t.string(3, y);
    let stage = STAGES.iter().position(|s| *s == stage_name).with_context(|| format!("Yotogi.Stage\nenum parse error.[{stage_name}]"))?;
    let cam: [f32; 6] = floats(&t.string(7, y), "ステージカメラ設定が不正です")?;
    let chara: [f32; 6] = floats(&t.string(8, y), "カス子位置情報が不正です")?;
    let skill: [f32; 2] = floats(&t.string(9, y), "スキル選択時カメラデータが不正です")?;
    Ok(StageData {
        need_grade: t.int(0, y),
        sort_id: t.int(1, y),
        draw_name: t.string(2, y),
        stage,
        stage_name,
        thumbnail_name: t.string(4, y),
        prefab_name: t.string(5, y),
        bgm_file: format!("{}.ogg", t.string(6, y)),
        select_camera_center: [cam[0], cam[1], cam[2]],
        select_camera_radius: cam[3],
        select_camera_rotate: [cam[4], cam[5]],
        skill_chara_pos: [chara[0], chara[1], chara[2]],
        skill_chara_rotate: [chara[3], chara[4], chara[5]],
        skill_camera_rotate: skill,
    })
}

/// `Yotogi.SkillData.Command` (both cell x origins are 0 in the original).
fn commands(setting: &Table, status: &Table, skill_id: i32, basic_y: usize, status_y: usize) -> Result<Vec<Command>> {
    let count = (0..).take_while(|i| setting.exists(2, basic_y + 1 + i)).count();
    ensure!(count > 0, "{}のコマンド設定は不正です", setting.string(0, basic_y));
    let mut out = Vec::with_capacity(count);
    for no in 1..=count {
        let y = basic_y + no;
        let mut c = Command { skill_id, id: y as i32, ..Default::default() };
        // Basic: the group is the nearest group cell at or above the row.
        let gy = (0..=y).rev().find(|r| setting.exists(1, *r)).context("表示グループの特定に失敗しました")?;
        c.group_name = setting.string(1, gy);
        let mut x = 2;
        let mut next = || {
            x += 1;
            x - 1
        };
        c.name = setting.string(next(), y);
        let ty = setting.string(next(), y);
        c.command_type = parse_enum(COMMAND_TYPES, &ty).with_context(|| format!("SkillCommandType {ty}"))?;
        c.request_skill_lv = setting.int(next(), y);
        for f in [&mut c.normal_file, &mut c.insert_file, &mut c.rc_file, &mut c.rrc_file, &mut c.rr_file] {
            *f = format!("{}.ks", setting.string(next(), y));
        }
        let sexual = setting.string(next(), y);
        if !sexual.is_empty() {
            c.sexual_type = parse_enum(SEXUAL_TYPES, &sexual).with_context(|| format!("enum parse error.[{sexual}]"))?;
        }
        c.seikan_arousal_file = setting.string(next(), y);
        ensure!(c.sexual_type == 0 || !c.seikan_arousal_file.is_empty(), "{}\n性感帯ファイル指定が不正です", c.name);
        let p = setting.string(next(), y);
        if !p.is_empty() {
            c.propensity_sabun = parse_enum(PROPENSITY, &p).with_context(|| format!("enum parse error.[{p}]"))?;
            c.propensity_arousal_file = setting.string(next(), y);
            ensure!(!c.propensity_arousal_file.is_empty(), "{}\n性癖ファイル指定が不正です", c.name);
        } else {
            next();
        }
        c.insert_release = setting.string(next(), y) != "○";
        c.request_insert = setting.string(next(), y) == "○" || c.command_type != CMD_SINGLE;
        let ex = next();
        c.expression = if setting.exists(ex, y) { setting.string(ex, y) } else { String::new() };
        c.active_command = !c.expression.is_empty();
        // Status: column 1 skipped, name at 2, values from 3.
        let sy = status_y + no;
        let sname = status.string(2, sy);
        ensure!(sname == c.name, "{sname}:{}コマンドの名前が不一致です", c.name);
        let v = |i: usize| status.int(3 + i, sy);
        let s = &mut c.status;
        (s.excitement, s.mind, s.reason, s.skill_exp, s.inyoku, s.m_value, s.hentai) = (v(0), v(1), v(2), v(3), v(4), v(5), v(6));
        (s.housi, s.frustration, s.lovely, s.elegance, s.charm, s.reception, s.teach_rate) = (v(7), v(8), v(9), v(10), v(11), v(12), v(13));
        s.sexual = [v(14), v(15), v(16), v(17), v(18), v(19)];
        out.push(c);
    }
    Ok(out)
}
