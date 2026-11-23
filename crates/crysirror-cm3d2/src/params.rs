// SPDX-License-Identifier: MPL-2.0
//! `param.Status` (maid) and `param_player.Status` (player) with their
//! original `BinaryWriter` layouts, the `SimpleExperienceSystem` levels, and
//! the enums with their `EnumConvert.GetString` texts. The operations of
//! `MaidParam`/`PlayerParam` live in [`crate::maid_param`].
use crate::binio::{OrderedMap, OrderedSet, Reader, Writer};
use anyhow::Result;

/// `param.Feature` (index = value).
pub const FEATURE: &[&str] = &[
    "Null", "元気な笑顔", "朗らかな魅力", "優美", "純真無垢", "天使", "魅了", "大人びた魅力", "魔性の魅力", "凜とした", "高貴", "恥ずかしがり屋", "素直",
    "妖艶な魅力", "上品", "優雅", "とことん尽くす", "傅く", "過去の秘密", "疲労", "過去の過ち",
];
/// `param.Propensity`.
pub const PROPENSITY: &[&str] = &["Null", "淫乱", "M女", "尻穴好き", "二穴好き", "変態", "奉仕好き", "中出し好き", "飲精好き"];
/// `param.MaidClassType` names (0..7 enabled; the arrays hold 16 slots).
pub const MAID_CLASS: &[&str] = &["Novice", "Lovely", "Elegance", "Sexy", "Innocent", "Charm", "Ready"];
pub const MAID_CLASS_TEXT: &[&str] = &["ノービスメイド", "ラブリーメイド", "エレガンスメイド", "セクシーメイド", "イノセントメイド", "チャームメイド", "レディーメイド"];
/// `param.YotogiClassType`.
pub const YOTOGI_CLASS: &[&str] = &["Debut", "Rudeness", "Slave", "Abnormal", "Service", "Escort", "Soap"];
pub const YOTOGI_CLASS_TEXT: &[&str] = &["デビューメイド", "ルードネスメイド", "スレイブメイド", "アブノーマルメイド", "サービスメイド", "エスコートメイド", "ソープメイド"];
pub const CLASS_SLOTS: usize = 16;
/// `param.Seikeiken`: No_No, Yes_No, No_Yes, Yes_Yes.
pub const SEIKEIKEN: &[&str] = &["No_No", "Yes_No", "No_Yes", "Yes_Yes"];
pub const SEIKEIKEN_TEXT: &[&str] = &["処女", "経験あり", "後ろ経験あり", "両方経験あり"];
/// `param.Personal`.
pub const PERSONAL: &[&str] = &["Pure", "Cool", "Pride"];
pub const PERSONAL_TEXT: &[&str] = &["純真で健気な妹系", "クールで寡黙", "プライドが高く負けず嫌い"];
/// `param.ContractType`.
pub const CONTRACT_TEXT: &[&str] = &["育成中", "フリー", "専属"];
/// `param.Condition`: Null, Tonus, Contact, Trust, Lover, Slave.
pub const CONDITION_TEXT: &[&str] = &["", "緊張", "お近づき", "信頼", "恋人", "愛奴"];
/// `param.ConditionSpecial`: Null, Drunk.
pub const CONDITION_SPECIAL_TEXT: &[&str] = &["なし", "酔い"];

/// `Enum.Parse` over a name table.
pub fn parse_enum(names: &[&str], s: &str) -> Option<i32> {
    names.iter().position(|n| *n == s).map(|i| i as i32).or_else(|| s.trim().parse().ok())
}

/// `EnumConvert.GetString` over a text table (out of range → empty).
pub fn enum_text(texts: &[&str], v: i32) -> String {
    usize::try_from(v).ok().and_then(|i| texts.get(i)).map(|s| (*s).to_owned()).unwrap_or_default()
}

/// `SimpleExperienceSystem`: levels from a per-level need table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exp {
    pub current: i32,
    pub total: i32,
    pub next: i32,
    pub level: i32,
    /// Need of level n at index n − 1; not part of `Serialize`.
    pub need: Vec<i32>,
}

impl Exp {
    pub fn set_list(&mut self, need: Vec<i32>) {
        self.need = need;
        self.update();
    }
    fn need_exp(&self, level: i32) -> i32 {
        self.need.get((level - 1) as usize).copied().unwrap_or(0)
    }
    pub fn max_level(&self) -> i32 {
        self.need.len() as i32
    }
    pub fn next_level_exp(&self, level: i32) -> i32 {
        let level = level + 1;
        if level <= 0 || self.max_level() < level { 0 } else { self.need_exp(level) }
    }
    pub fn max_level_need(&self) -> i32 {
        (0..self.max_level()).map(|i| self.next_level_exp(i)).sum()
    }
    pub fn add(&mut self, exp: i32) {
        self.total += exp;
        self.update();
    }
    pub fn set_level(&mut self, level: i32) {
        self.total = 0;
        let level = level.clamp(0, self.max_level());
        for i in 1..=level {
            self.total += self.need_exp(i);
        }
        self.update();
    }
    pub fn update(&mut self) {
        let max = self.max_level_need();
        self.total = self.total.min(max).max(0);
        let mut rest = self.total;
        for i in 1..=self.max_level() {
            let need = self.need_exp(i);
            rest -= need;
            if rest < 0 {
                self.level = i - 1;
                self.next = rest.abs();
                self.current = need + rest;
                return;
            }
        }
        self.level = self.max_level();
        self.current = 0;
        self.next = 0;
    }
    fn write(&self, w: &mut Writer) {
        w.i32(self.current);
        w.i32(self.total);
        w.i32(self.next);
        w.i32(self.level);
    }
    fn read(&mut self, r: &mut Reader) -> Result<()> {
        self.current = r.i32()?;
        self.total = r.i32()?;
        self.next = r.i32()?;
        self.level = r.i32()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkillData {
    pub id: i32,
    pub play_count: u32,
    pub exp: Exp,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkData {
    pub id: i32,
    pub play_count: u32,
    pub level: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassData {
    pub is_have: bool,
    pub exp: Exp,
}

impl ClassData {
    /// `MaidClassData.Clear`: not owned, level 0 (the need table stays).
    pub fn clear(&mut self) {
        self.is_have = false;
        self.exp.set_level(0);
    }
    pub fn level(&self) -> i32 {
        self.exp.level
    }
}

/// `param.Status.MaidClassBonusStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bonus {
    pub hp: i32,
    pub mind: i32,
    pub reception: i32,
    pub care: i32,
    pub lovely: i32,
    pub inyoku: i32,
    pub elegance: i32,
    pub m_value: i32,
    pub charm: i32,
    pub hentai: i32,
    pub housi: i32,
    pub teach_rate: i32,
}

impl Bonus {
    fn fields(&mut self) -> [&mut i32; 12] {
        [&mut self.hp, &mut self.mind, &mut self.reception, &mut self.care, &mut self.lovely, &mut self.inyoku, &mut self.elegance, &mut self.m_value, &mut self.charm, &mut self.hentai, &mut self.housi, &mut self.teach_rate]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    pub height: i32,
    pub weight: i32,
    pub bust: i32,
    pub waist: i32,
    pub hip: i32,
    pub cup: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sexual {
    pub mouth: i32,
    pub throat: i32,
    pub nipple: i32,
    pub front: i32,
    pub back: i32,
    pub curi: i32,
}

/// `param.Status`.
#[derive(Clone, Debug, PartialEq)]
pub struct MaidStatus {
    pub guid_id: String,
    pub create_time: String,
    pub create_time_num: u64,
    pub employment_day: i32,
    pub maid_point: i32,
    pub last_name: String,
    pub first_name: String,
    pub profile: String,
    pub free_comment: String,
    pub init_seikeiken: i32,
    pub seikeiken: i32,
    pub personal: i32,
    pub contract_type: i32,
    pub maid_class: Vec<ClassData>,
    pub current_maid_class: i32,
    pub yotogi_class: Vec<ClassData>,
    pub current_yotogi_class: i32,
    pub feature: OrderedSet<i32>,
    pub propensity: OrderedSet<i32>,
    pub body: Body,
    pub condition: i32,
    pub condition_special: i32,
    pub yotogi_play_count: i32,
    pub others_play_count: i32,
    pub likability: i32,
    pub study_rate: i32,
    /// Not serialized.
    pub cur_excite: i32,
    pub cur_hp: i32,
    pub hp: i32,
    pub cur_mind: i32,
    pub mind: i32,
    pub cur_reason: i32,
    pub reason: i32,
    pub reception: i32,
    pub care: i32,
    pub lovely: i32,
    pub inyoku: i32,
    pub elegance: i32,
    pub m_value: i32,
    pub charm: i32,
    pub hentai: i32,
    pub housi: i32,
    pub teach_rate: i32,
    pub sexual: Sexual,
    pub play_number: i32,
    pub frustration: i32,
    pub popular_rank: i32,
    pub evaluation: i64,
    pub total_evaluation: i64,
    pub sales: i64,
    pub total_sales: i64,
    pub is_first_name_call: bool,
    pub is_rental_maid: bool,
    pub noon_work_id: i32,
    pub night_work_id: i32,
    pub skill_data: OrderedMap<i32, SkillData>,
    pub work_data: OrderedMap<i32, WorkData>,
    pub generic_flag: OrderedMap<String, i32>,
    pub employment: bool,
    pub leader: bool,
    pub eye_parts_tab: i32,
    pub parts_dic: OrderedMap<String, String>,
    pub bonus: Bonus,
}

impl Default for MaidStatus {
    fn default() -> Self {
        Self {
            guid_id: String::new(),
            create_time: String::new(),
            create_time_num: 0,
            employment_day: 0,
            maid_point: 0,
            last_name: String::new(),
            first_name: String::new(),
            profile: String::new(),
            free_comment: String::new(),
            init_seikeiken: 0,
            seikeiken: 0,
            personal: 0,
            contract_type: 0,
            maid_class: vec![ClassData::default(); CLASS_SLOTS],
            current_maid_class: 0,
            yotogi_class: vec![ClassData::default(); CLASS_SLOTS],
            current_yotogi_class: 0,
            feature: OrderedSet::default(),
            propensity: OrderedSet::default(),
            body: Body::default(),
            condition: 0,
            condition_special: 0,
            yotogi_play_count: 0,
            others_play_count: 0,
            likability: 0,
            study_rate: 0,
            cur_excite: 0,
            cur_hp: 0,
            hp: 0,
            cur_mind: 0,
            mind: 0,
            cur_reason: 0,
            reason: 0,
            reception: 0,
            care: 0,
            lovely: 0,
            inyoku: 0,
            elegance: 0,
            m_value: 0,
            charm: 0,
            hentai: 0,
            housi: 0,
            teach_rate: 0,
            sexual: Sexual::default(),
            play_number: 0,
            frustration: 0,
            popular_rank: 0,
            evaluation: 0,
            total_evaluation: 0,
            sales: 0,
            total_sales: 0,
            is_first_name_call: true,
            is_rental_maid: false,
            noon_work_id: 0,
            night_work_id: 0,
            skill_data: OrderedMap::default(),
            work_data: OrderedMap::default(),
            generic_flag: OrderedMap::default(),
            employment: false,
            leader: false,
            eye_parts_tab: 0,
            parts_dic: OrderedMap::default(),
            bonus: Bonus::default(),
        }
    }
}

/// The `MaidParam.StatusAccess` view: bonus-added, capped values.
impl MaidStatus {
    pub fn a_cur_excite(&self) -> i32 {
        self.cur_excite.min(999)
    }
    pub fn a_hp(&self) -> i32 {
        (self.hp + self.bonus.hp).min(999)
    }
    pub fn a_cur_hp(&self) -> i32 {
        self.cur_hp.min(self.a_hp())
    }
    pub fn a_mind(&self) -> i32 {
        (self.mind + self.bonus.mind).min(999)
    }
    pub fn a_cur_mind(&self) -> i32 {
        self.cur_mind.min(self.a_mind())
    }
    pub fn a_reason(&self) -> i32 {
        self.reason.min(999)
    }
    pub fn a_cur_reason(&self) -> i32 {
        self.cur_reason.min(self.a_reason())
    }
    pub fn a_study_rate(&self) -> i32 {
        self.study_rate.min(1000)
    }
    pub fn a_reception(&self) -> i32 {
        (self.reception + self.bonus.reception).min(9999)
    }
    pub fn a_care(&self) -> i32 {
        (self.care + self.bonus.care).min(9999)
    }
    pub fn a_lovely(&self) -> i32 {
        (self.lovely + self.bonus.lovely).min(9999)
    }
    pub fn a_inyoku(&self) -> i32 {
        (self.inyoku + self.bonus.inyoku).min(9999)
    }
    pub fn a_elegance(&self) -> i32 {
        (self.elegance + self.bonus.elegance).min(9999)
    }
    pub fn a_m_value(&self) -> i32 {
        (self.m_value + self.bonus.m_value).min(9999)
    }
    pub fn a_charm(&self) -> i32 {
        (self.charm + self.bonus.charm).min(9999)
    }
    pub fn a_hentai(&self) -> i32 {
        (self.hentai + self.bonus.hentai).min(9999)
    }
    pub fn a_housi(&self) -> i32 {
        (self.housi + self.bonus.housi).min(9999)
    }
    pub fn a_teach_rate(&self) -> i32 {
        (self.teach_rate + self.bonus.teach_rate).min(1000)
    }
    pub fn a_frustration(&self) -> i32 {
        self.frustration.min(100)
    }
    pub fn inran_gap(&self) -> i32 {
        self.a_inyoku() + self.a_lovely()
    }
    pub fn m_value_gap(&self) -> i32 {
        self.a_m_value() + self.a_elegance()
    }
    pub fn hentai_gap(&self) -> i32 {
        self.a_hentai() + self.a_charm()
    }
    /// `status.GetGenericFlag`: absent flags read 0.
    pub fn flag(&self, k: &str) -> i32 {
        self.generic_flag.get(k).copied().unwrap_or(0)
    }
    pub fn is_feature(&self, f: i32) -> bool {
        self.feature.contains(&f)
    }
    pub fn is_propensity(&self, p: i32) -> bool {
        self.propensity.contains(&p)
    }

    /// `param.Status.Serialize`.
    pub fn write(&self, w: &mut Writer) {
        w.str(&self.guid_id);
        w.str(&self.create_time);
        w.u64(self.create_time_num);
        w.i32(self.employment_day);
        w.i32(self.maid_point);
        w.str(&self.last_name);
        w.str(&self.first_name);
        w.str(&self.profile);
        w.str(&self.free_comment);
        for v in [self.init_seikeiken, self.seikeiken, self.personal, self.contract_type] {
            w.i32(v);
        }
        w.i32(self.maid_class.len() as i32);
        for c in &self.maid_class {
            w.bool(c.is_have);
            c.exp.write(w);
        }
        w.i32(self.current_maid_class);
        w.i32(self.yotogi_class.len() as i32);
        for c in &self.yotogi_class {
            w.bool(c.is_have);
            c.exp.write(w);
        }
        w.i32(self.current_yotogi_class);
        w.i32(self.feature.len() as i32);
        for f in self.feature.keys() {
            w.i32(*f);
        }
        w.i32(self.propensity.len() as i32);
        for p in self.propensity.keys() {
            w.i32(*p);
        }
        let b = &self.body;
        for v in [b.height, b.weight, b.bust, b.waist, b.hip] {
            w.i32(v);
        }
        w.str(&b.cup);
        w.i32(self.condition);
        w.i32(self.condition_special);
        for v in [
            self.yotogi_play_count, self.others_play_count, self.likability, self.study_rate, self.cur_hp, self.hp, self.cur_mind, self.mind, self.cur_reason,
            self.reason, self.reception, self.care, self.lovely, self.inyoku, self.elegance, self.m_value, self.charm, self.hentai, self.housi, self.teach_rate,
        ] {
            w.i32(v);
        }
        let s = &self.sexual;
        for v in [s.mouth, s.throat, s.nipple, s.front, s.back, s.curi] {
            w.i32(v);
        }
        w.i32(self.play_number);
        w.i32(self.frustration);
        w.i32(self.popular_rank);
        for v in [self.evaluation, self.total_evaluation, self.sales, self.total_sales] {
            w.i64(v);
        }
        w.bool(self.is_first_name_call);
        w.bool(self.is_rental_maid);
        w.i32(self.noon_work_id);
        w.i32(self.night_work_id);
        w.i32(self.skill_data.len() as i32);
        for (k, s) in self.skill_data.iter() {
            w.i32(*k);
            w.i32(s.id);
            w.u32(s.play_count);
            s.exp.write(w);
        }
        w.i32(self.work_data.len() as i32);
        for (k, d) in self.work_data.iter() {
            w.i32(*k);
            w.i32(d.id);
            w.u32(d.play_count);
            w.i32(d.level);
        }
        w.i32(self.generic_flag.len() as i32);
        for (k, v) in self.generic_flag.iter() {
            w.str(k);
            w.i32(*v);
        }
        w.bool(self.employment);
        w.bool(self.leader);
        w.i32(self.eye_parts_tab);
        w.i32(self.parts_dic.len() as i32);
        for (k, v) in self.parts_dic.iter() {
            w.str(k);
            w.str(v);
        }
        let mut bonus = self.bonus;
        for v in bonus.fields() {
            w.i32(*v);
        }
    }

    /// `param.Status.Deserialize`. Skill need tables come from the yotogi
    /// skill data afterwards (`skill_exp_table`); class need tables are kept.
    pub fn read(&mut self, r: &mut Reader, version: i32) -> Result<()> {
        self.guid_id = r.str()?;
        self.create_time = r.str()?;
        self.create_time_num = r.u64()?;
        self.employment_day = r.i32()?;
        self.maid_point = r.i32()?;
        self.last_name = r.str()?;
        self.first_name = r.str()?;
        self.profile = r.str()?;
        self.free_comment = r.str()?;
        self.init_seikeiken = r.i32()?;
        self.seikeiken = r.i32()?;
        self.personal = r.i32()?;
        self.contract_type = r.i32()?;
        for list in [0, 1] {
            let n = r.count()?;
            anyhow::ensure!(n <= CLASS_SLOTS, "class data count {n}");
            for i in 0..n {
                let c = if list == 0 { &mut self.maid_class[i] } else { &mut self.yotogi_class[i] };
                c.is_have = r.bool()?;
                c.exp.read(r)?;
            }
            if list == 0 {
                self.current_maid_class = r.i32()?;
            } else {
                self.current_yotogi_class = r.i32()?;
            }
        }
        self.feature.clear();
        for _ in 0..r.count()? {
            self.feature.add(r.i32()?);
        }
        self.propensity.clear();
        for _ in 0..r.count()? {
            self.propensity.add(r.i32()?);
        }
        self.body = Body { height: r.i32()?, weight: r.i32()?, bust: r.i32()?, waist: r.i32()?, hip: r.i32()?, cup: r.str()? };
        self.condition = r.i32()?;
        self.condition_special = r.i32()?;
        for f in [
            &mut self.yotogi_play_count, &mut self.others_play_count, &mut self.likability, &mut self.study_rate, &mut self.cur_hp, &mut self.hp, &mut self.cur_mind,
            &mut self.mind, &mut self.cur_reason, &mut self.reason, &mut self.reception, &mut self.care, &mut self.lovely, &mut self.inyoku, &mut self.elegance,
            &mut self.m_value, &mut self.charm, &mut self.hentai, &mut self.housi, &mut self.teach_rate,
        ] {
            *f = r.i32()?;
        }
        self.sexual = Sexual { mouth: r.i32()?, throat: r.i32()?, nipple: r.i32()?, front: r.i32()?, back: r.i32()?, curi: r.i32()? };
        self.play_number = r.i32()?;
        self.frustration = r.i32()?;
        self.popular_rank = r.i32()?;
        self.evaluation = r.i64()?;
        self.total_evaluation = r.i64()?;
        self.sales = r.i64()?;
        self.total_sales = r.i64()?;
        self.is_first_name_call = if version <= 6 { true } else { r.bool()? };
        self.is_rental_maid = if version <= 8 { false } else { r.bool()? };
        self.noon_work_id = r.i32()?;
        self.night_work_id = r.i32()?;
        self.skill_data.clear();
        for _ in 0..r.count()? {
            let _key = r.i32()?;
            let mut s = SkillData { id: r.i32()?, play_count: r.u32()?, ..Default::default() };
            s.exp.read(r)?;
            self.skill_data.insert(s.id, s);
        }
        self.work_data.clear();
        for _ in 0..r.count()? {
            let key = r.i32()?;
            let d = WorkData { id: r.i32()?, play_count: r.u32()?, level: r.i32()? };
            self.work_data.insert(key, d);
        }
        self.generic_flag.clear();
        for _ in 0..r.count()? {
            let k = r.str()?;
            let v = r.i32()?;
            self.generic_flag.insert(k, v);
        }
        self.employment = r.bool()?;
        self.leader = r.bool()?;
        self.eye_parts_tab = r.i32()?;
        self.parts_dic.clear();
        for _ in 0..r.count()? {
            let k = r.str()?;
            let v = r.str()?;
            self.parts_dic.insert(k, v);
        }
        for v in self.bonus.fields() {
            *v = r.i32()?;
        }
        Ok(())
    }
}

/// `param_player.Status.ScheduleData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScheduleData {
    pub maid_guid: String,
    pub noon_success_level: i32,
    pub night_success_level: i32,
    pub communication: bool,
    pub backup_status: OrderedMap<String, i32>,
}

/// `param_player.Status.NightWorkState`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NightWorkState {
    pub work_id: i32,
    pub called_maid_guid: String,
    pub finish: bool,
}

/// `param_player.Status.ShopItemStatus`.
pub const SHOP_NEW: i32 = 0;
pub const SHOP_NOT_PURCHASED: i32 = 1;
pub const SHOP_PURCHASED: i32 = 2;

pub const MONEY_MAX: i64 = 9_999_999_999;
pub const SALON_GRADE_MAX: i32 = 5;
pub const SCHEDULE_SLOTS: usize = 6;
pub const INIT_SALON_LOAN: i64 = -10_000_000;
pub const INIT_SALON_LOAN_MIN: i64 = -7_500_000;

/// `param_player.Status`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerStatus {
    /// `MaidRandomName.GetPlayerName` draws from a name table in
    /// Assembly-CSharp; the name entry of `SceneUserEdit` sets it.
    pub player_name: String,
    pub scenario_phase: i32,
    pub phase_days: i32,
    pub days: i32,
    pub shop_use_money: i64,
    pub money: i64,
    pub init_salon_loan: i64,
    pub salon_loan: i64,
    pub salon_clean: i32,
    pub salon_beautiful: i32,
    pub salon_evaluation: i32,
    pub is_first_name_call: bool,
    pub current_salon_grade: i32,
    pub best_salon_grade: i32,
    pub schedule_slot: Vec<ScheduleData>,
    pub generic_flag: OrderedMap<String, i32>,
    pub night_works_state: OrderedMap<i32, NightWorkState>,
    pub shop_lineup: OrderedMap<i32, i32>,
    pub have_item: OrderedMap<String, bool>,
    pub have_trophy: OrderedSet<i32>,
    pub maid_class_open: OrderedSet<i32>,
    pub yotogi_class_open: OrderedSet<i32>,
}

impl Default for PlayerStatus {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            scenario_phase: 0,
            phase_days: 0,
            days: 0,
            shop_use_money: 0,
            money: 0,
            init_salon_loan: INIT_SALON_LOAN,
            salon_loan: INIT_SALON_LOAN,
            salon_clean: 0,
            salon_beautiful: 0,
            salon_evaluation: 0,
            is_first_name_call: false,
            current_salon_grade: 1,
            best_salon_grade: 1,
            schedule_slot: vec![ScheduleData::default(); SCHEDULE_SLOTS],
            generic_flag: OrderedMap::default(),
            night_works_state: OrderedMap::default(),
            shop_lineup: OrderedMap::default(),
            have_item: OrderedMap::default(),
            have_trophy: OrderedSet::default(),
            maid_class_open: OrderedSet::default(),
            yotogi_class_open: OrderedSet::default(),
        }
    }
}

impl PlayerStatus {
    pub fn flag(&self, k: &str) -> i32 {
        self.generic_flag.get(k).copied().unwrap_or(0)
    }
    pub fn is_shop_lineup(&self, id: i32) -> bool {
        self.shop_lineup.contains(&id)
    }
    pub fn shop_item_status(&self, id: i32) -> Option<i32> {
        self.shop_lineup.get(&id).copied()
    }

    /// `param_player.Status.Serialize`.
    pub fn write(&self, w: &mut Writer) {
        w.str(&self.player_name);
        w.i32(self.scenario_phase);
        w.i32(self.phase_days);
        w.i32(self.days);
        w.i64(self.shop_use_money);
        w.i64(self.money);
        w.i64(self.init_salon_loan);
        w.i64(self.salon_loan);
        w.i32(self.salon_clean);
        w.i32(self.salon_beautiful);
        w.i32(self.salon_evaluation);
        w.bool(self.is_first_name_call);
        w.i32(self.current_salon_grade);
        w.i32(self.best_salon_grade);
        for s in &self.schedule_slot {
            w.str(&s.maid_guid);
            w.i32(s.noon_success_level);
            w.i32(s.night_success_level);
            w.bool(s.communication);
            w.i32(s.backup_status.len() as i32);
            for (k, v) in s.backup_status.iter() {
                w.str(k);
                w.i32(*v);
            }
        }
        w.i32(self.generic_flag.len() as i32);
        for (k, v) in self.generic_flag.iter() {
            w.str(k);
            w.i32(*v);
        }
        w.i32(self.night_works_state.len() as i32);
        for (k, n) in self.night_works_state.iter() {
            w.i32(*k);
            w.i32(n.work_id);
            w.str(&n.called_maid_guid);
            w.bool(n.finish);
        }
        w.i32(self.shop_lineup.len() as i32);
        for (k, v) in self.shop_lineup.iter() {
            w.i32(*k);
            w.i32(*v);
        }
        w.i32(self.have_item.len() as i32);
        for (k, v) in self.have_item.iter() {
            w.str(k);
            w.bool(*v);
        }
        for set in [&self.have_trophy, &self.maid_class_open, &self.yotogi_class_open] {
            w.i32(set.len() as i32);
            for v in set.keys() {
                w.i32(*v);
            }
        }
    }

    /// `param_player.Status.Deserialize`.
    pub fn read(&mut self, r: &mut Reader, version: i32) -> Result<()> {
        self.player_name = r.str()?;
        self.scenario_phase = r.i32()?;
        self.phase_days = r.i32()?;
        self.days = r.i32()?;
        self.shop_use_money = r.i64()?;
        self.money = r.i64()?;
        self.init_salon_loan = if version <= 8 { INIT_SALON_LOAN } else { r.i64()? };
        self.salon_loan = r.i64()?;
        self.salon_clean = r.i32()?;
        self.salon_beautiful = r.i32()?;
        self.salon_evaluation = r.i32()?;
        self.is_first_name_call = if version <= 5 { true } else { r.bool()? };
        self.current_salon_grade = r.i32()?;
        self.best_salon_grade = r.i32()?;
        for s in &mut self.schedule_slot {
            s.maid_guid = r.str()?;
            s.noon_success_level = r.i32()?;
            s.night_success_level = r.i32()?;
            s.communication = r.bool()?;
            s.backup_status.clear();
            for _ in 0..r.count()? {
                let k = r.str()?;
                let v = r.i32()?;
                s.backup_status.insert(k, v);
            }
        }
        self.generic_flag.clear();
        for _ in 0..r.count()? {
            let k = r.str()?;
            let v = r.i32()?;
            self.generic_flag.insert(k, v);
        }
        self.night_works_state.clear();
        for _ in 0..r.count()? {
            let k = r.i32()?;
            let n = NightWorkState { work_id: r.i32()?, called_maid_guid: r.str()?, finish: r.bool()? };
            self.night_works_state.insert(k, n);
        }
        self.shop_lineup.clear();
        for _ in 0..r.count()? {
            let k = r.i32()?;
            let v = r.i32()?;
            self.shop_lineup.insert(k, v);
        }
        self.have_item.clear();
        for _ in 0..r.count()? {
            let k = r.str()?;
            let v = r.bool()?;
            self.have_item.insert(k, v);
        }
        self.have_trophy.clear();
        for _ in 0..r.count()? {
            self.have_trophy.add(r.i32()?);
        }
        self.maid_class_open.clear();
        self.yotogi_class_open.clear();
        if version <= 9 {
            return Ok(());
        }
        for _ in 0..r.count()? {
            self.maid_class_open.add(r.i32()?);
        }
        for _ in 0..r.count()? {
            self.yotogi_class_open.add(r.i32()?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_levels_follow_the_need_table() {
        let mut e = Exp::default();
        e.set_list(vec![0, 20, 25]);
        assert_eq!((e.level, e.current, e.next), (1, 0, 20));
        e.add(30);
        assert_eq!((e.level, e.current, e.next), (2, 10, 15));
        e.add(1000);
        assert_eq!((e.total, e.level, e.next), (45, 3, 0));
        e.set_level(1);
        assert_eq!(e.total, 0);
    }

    #[test]
    fn maid_status_round_trips() {
        let mut m = MaidStatus { last_name: "苗字".into(), seikeiken: 3, evaluation: 7, ..Default::default() };
        m.feature.add(18);
        m.generic_flag.insert("a".into(), 2);
        m.skill_data.insert(5, SkillData { id: 5, play_count: 1, exp: Exp::default() });
        let mut w = Writer::default();
        m.write(&mut w);
        let mut back = MaidStatus::default();
        back.read(&mut Reader::new(&w.bytes), 100).unwrap();
        assert_eq!(back, m);
        let mut p = PlayerStatus::default();
        p.have_item.insert("x.menu".into(), true);
        p.have_trophy.add(3);
        let mut w = Writer::default();
        p.write(&mut w);
        let mut back = PlayerStatus::default();
        back.read(&mut Reader::new(&w.bytes), 100).unwrap();
        assert_eq!(back, p);
    }
}
