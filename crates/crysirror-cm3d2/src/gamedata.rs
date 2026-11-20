// SPDX-License-Identifier: MPL-2.0
//! Game tables of `csv.arc`, decrypted with the installed plug-in DLL's key
//! (`native_key`), and the static data the C# builds from them once:
//! class experience tables (`MaidParam.Initialize`), `Trophy.CreateData`,
//! `Shop.CreateData`, `PlayerParam.CreateSalonGradeData`,
//! `scenario_get_item_list.nei` and `Yotogi.CreateYotogiData`'s skills.
use crate::{
    binio::OrderedMap,
    nei::Table,
    params::{parse_enum, CLASS_SLOTS, MAID_CLASS, PROPENSITY, YOTOGI_CLASS},
};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet};

/// `Trophy.Data`.
#[derive(Clone, Debug, Default)]
pub struct Trophy {
    pub id: i32,
    pub rarity: i32,
    pub name: String,
    pub maid_point: i32,
    pub info_text: String,
    pub bonus_text: String,
    pub effect_draw_item_name: String,
    pub title_mask: bool,
    pub lineup: Vec<i32>,
}

/// `Shop.ItemData`/`Shop.ItemDataGroup` (`need_ids` set for groups).
#[derive(Clone, Debug, Default)]
pub struct ShopItem {
    pub id: i32,
    pub name: String,
    pub icon_file: String,
    pub detail_text: String,
    /// `TBody.MaskMode` name ("None" when empty).
    pub mask_mode: String,
    pub main_category: String,
    pub sub_category: String,
    pub price: i32,
    pub menus: Vec<String>,
    pub need_ids: Option<Vec<i32>>,
}

/// `Yotogi.SkillData` fields used outside the yotogi play scene.
#[derive(Clone, Debug, Default)]
pub struct Skill {
    pub id: i32,
    pub category: String,
    pub name: String,
    pub icon_file_name: String,
    pub start_call_file: String,
    pub start_call_file2: String,
    pub exec_need_hp: i32,
    pub add_yotogi_class_exp: i32,
    pub exp_table: Vec<i32>,
    pub only_one: bool,
    pub exec_seikeiken: [bool; 4],
    /// Tonus, Contact, Trust, Lover, Slave.
    pub exec_condition: [bool; 5],
    pub drunk_skill: bool,
    pub propensity_type: i32,
    pub exec_stage: Vec<String>,
    pub get: SkillGet,
}

/// `Yotogi.SkillData.GetConditionData`.
#[derive(Clone, Debug, Default)]
pub struct SkillGet {
    pub seikeiken: [bool; 4],
    pub inyoku: i32,
    pub m_value: i32,
    pub hentai: i32,
    pub housi: i32,
    pub lovely: i32,
    pub elegance: i32,
    pub charm: i32,
    pub maid_class: i32,
    pub maid_class_level: i32,
    pub yotogi_class: i32,
    pub yotogi_class_level: i32,
}

/// `Yotogi.Category` order of `skill_data_list`.
pub const SKILL_CATEGORIES: &[&str] = &["淫欲", "変態", "奉仕", "M性", "交換", "乱交", "詰られ"];

pub struct GameData {
    tables: HashMap<String, Table>,
    pub maid_class_exp: Vec<Vec<i32>>,
    pub yotogi_class_exp: Vec<Vec<i32>>,
    pub trophies: BTreeMap<i32, Trophy>,
    pub shop: OrderedMap<i32, ShopItem>,
    pub shop_categories: Vec<(String, Vec<String>)>,
    pub set_cards: Vec<(Vec<String>, Vec<i32>)>,
    /// `SalonGradeData`: need exp and lineup ids per grade.
    pub salon_grades: Vec<(i32, Vec<i32>)>,
    pub scenario_items: Vec<String>,
    /// Skills per `SKILL_CATEGORIES` index, by id.
    pub skills: Vec<BTreeMap<i32, Skill>>,
    pub schedule: crate::schedule_data::ScheduleData,
    /// `MaidRandomName` last/first/player name arrays (from the installed
    /// Assembly-CSharp, see [`GameData::set_random_names`]).
    pub random_names: [Vec<String>; 3],
    /// `param.Correction` per feature/propensity.
    pub feature_correction: BTreeMap<i32, crate::schedule::Correction>,
    pub propensity_correction: BTreeMap<i32, crate::schedule::Correction>,
    pub yotogi: crate::yotogi_data::YotogiData,
}

impl GameData {
    /// Decrypts every `.nei` and builds the static data.
    pub fn load(files: &crate::files::GameFiles, storage: &dyn crysirror_core::service::ClientStorage, key: &[u8; 16]) -> Result<Self> {
        let mut tables = HashMap::new();
        for name in files.names_with_ext(".nei") {
            let bytes = files.read(storage, &name)?;
            tables.insert(name.clone(), Table::open(&bytes, key).with_context(|| format!("table {name}"))?);
        }
        Self::from_tables(tables)
    }

    pub fn from_tables(tables: HashMap<String, Table>) -> Result<Self> {
        let mut d = Self {
            tables,
            maid_class_exp: vec![Vec::new(); CLASS_SLOTS],
            yotogi_class_exp: vec![Vec::new(); CLASS_SLOTS],
            trophies: BTreeMap::new(),
            shop: OrderedMap::default(),
            shop_categories: Vec::new(),
            set_cards: Vec::new(),
            salon_grades: Vec::new(),
            scenario_items: Vec::new(),
            skills: vec![BTreeMap::new(); SKILL_CATEGORIES.len()],
            schedule: crate::schedule_data::ScheduleData::empty(),
            random_names: Default::default(),
            feature_correction: BTreeMap::new(),
            propensity_correction: BTreeMap::new(),
            yotogi: Default::default(),
        };
        for (file, names, yotogi) in [("maid_class_exp_list.nei", MAID_CLASS, false), ("yotogi_class_exp_list.nei", YOTOGI_CLASS, true)] {
            let t = d.table(file)?;
            let mut lists = vec![Vec::new(); CLASS_SLOTS];
            for y in 1..t.rows {
                if !t.exists(0, y) {
                    continue;
                }
                let c = parse_enum(names, &t.string(0, y)).with_context(|| format!("{file} class {}", t.string(0, y)))?;
                lists[c as usize] = (1..).take_while(|x| t.exists(*x, y)).map(|x| t.int(x, y)).collect();
            }
            if yotogi { d.yotogi_class_exp = lists } else { d.maid_class_exp = lists }
        }
        d.load_trophies()?;
        d.load_shop()?;
        let t = d.table("salongrade_setting.nei")?;
        let mut grades = Vec::new();
        for y in 1..t.rows {
            if t.exists(0, y) {
                let items = t.string(2, y);
                let ids = if items.is_empty() { Vec::new() } else { items.split(',').map(|s| s.trim().parse().context("salon grade item id")).collect::<Result<_>>()? };
                grades.push((t.int(1, y), ids));
            }
        }
        d.salon_grades = grades;
        let t = d.table("scenario_get_item_list.nei")?;
        d.scenario_items = (0..t.rows).filter(|y| t.exists(0, *y)).map(|y| format!("{}.menu", t.string(0, y).to_lowercase())).collect();
        d.load_skills()?;
        d.yotogi = crate::yotogi_data::YotogiData::load(&d)?;
        let schedule = crate::schedule_data::ScheduleData::load(|n| d.table(n))?;
        d.schedule = schedule;
        // Correction.Create.
        for (file, names, feature) in [("feature_correction.nei", crate::params::FEATURE, true), ("propensity_correction.nei", PROPENSITY, false)] {
            let t = d.table(file)?;
            let mut map = BTreeMap::new();
            for y in 1..t.rows {
                if t.exists(0, y) {
                    let k = parse_enum(names, &t.string(0, y)).with_context(|| format!("{file}: {}", t.string(0, y)))?;
                    let mut c = crate::schedule::Correction::default();
                    for (i, v) in c.0.iter_mut().enumerate() {
                        *v = t.int(1 + i, y);
                    }
                    map.insert(k, c);
                }
            }
            if feature { d.feature_correction = map } else { d.propensity_correction = map }
        }
        Ok(d)
    }

    /// Tables-free data for unit tests.
    #[cfg(test)]
    pub fn from_tables_for_test() -> Self {
        Self {
            tables: HashMap::new(),
            maid_class_exp: vec![Vec::new(); CLASS_SLOTS],
            yotogi_class_exp: vec![Vec::new(); CLASS_SLOTS],
            trophies: BTreeMap::new(),
            shop: OrderedMap::default(),
            shop_categories: Vec::new(),
            set_cards: Vec::new(),
            salon_grades: Vec::new(),
            scenario_items: Vec::new(),
            skills: vec![BTreeMap::new(); SKILL_CATEGORIES.len()],
            schedule: crate::schedule_data::ScheduleData::empty(),
            random_names: Default::default(),
            feature_correction: BTreeMap::new(),
            propensity_correction: BTreeMap::new(),
            yotogi: Default::default(),
        }
    }

    /// `MaidRandomName.last_name_array`/`first_name_array`/`player_name_array`
    /// evaluated from the installed `Assembly-CSharp.dll`.
    pub fn set_random_names(&mut self, asm: &crysirror_dotnet::Assembly) -> Result<()> {
        let st = asm.eval_cctor("MaidRandomName")?;
        for (i, f) in ["last_name_array", "first_name_array", "player_name_array"].iter().enumerate() {
            let v = st.fields.get(*f).with_context(|| format!("MaidRandomName.{f}"))?;
            let items = st.items(v).with_context(|| format!("MaidRandomName.{f} is not an array"))?;
            self.random_names[i] = items.iter().filter_map(|x| x.str().map(str::to_owned)).collect();
            anyhow::ensure!(!self.random_names[i].is_empty(), "MaidRandomName.{f} is empty");
        }
        Ok(())
    }

    pub fn table(&self, name: &str) -> Result<&Table> {
        self.tables.get(&name.to_ascii_lowercase()).with_context(|| format!("CM3D2 table {name} is missing"))
    }

    fn load_trophies(&mut self) -> Result<()> {
        let t = self.table("trophy_list.nei")?;
        let mut out = BTreeMap::new();
        for y in 1..t.rows {
            if !t.exists(0, y) {
                continue;
            }
            let tr = Trophy {
                id: t.int(0, y),
                rarity: t.int(1, y),
                name: t.string(2, y),
                maid_point: t.int(3, y),
                info_text: t.string(4, y),
                bonus_text: t.string(5, y),
                effect_draw_item_name: t.string(6, y),
                title_mask: t.string(7, y) == "○",
                lineup: (8..t.columns).filter(|x| t.exists(*x, y)).map(|x| t.int(x, y)).collect(),
            };
            anyhow::ensure!(!out.contains_key(&tr.id), "trophy overlap id {}", tr.id);
            out.insert(tr.id, tr);
        }
        self.trophies = out;
        Ok(())
    }

    fn load_shop(&mut self) -> Result<()> {
        let t = self.table("game_in_shop_category_list.nei")?;
        let mut cats: Vec<(String, Vec<String>)> = Vec::new();
        for y in 1..t.rows {
            if !t.exists(0, y) {
                continue;
            }
            let (main, sub) = (t.string(0, y), t.string(1, y));
            let i = match cats.iter().position(|(m, _)| *m == main) {
                Some(i) => i,
                None => {
                    cats.push((main, Vec::new()));
                    cats.len() - 1
                }
            };
            if !cats[i].1.contains(&sub) {
                cats[i].1.push(sub);
            }
        }
        let t = self.table("game_in_shop_itemlist.nei")?;
        let listed: HashSet<i32> = (1..t.rows).filter(|y| t.exists(0, *y)).map(|y| t.int(0, y)).collect();
        let base = |t: &Table, y: usize| {
            let mask = t.string(4, y);
            ShopItem {
                id: t.int(0, y),
                name: t.string(1, y),
                icon_file: format!("{}.tex", t.string(2, y)),
                detail_text: t.string(3, y).replace("《改行》", "\n"),
                mask_mode: if mask.is_empty() { "None".into() } else { mask },
                main_category: t.string(5, y),
                sub_category: t.string(6, y),
                ..Default::default()
            }
        };
        let mut shop = OrderedMap::default();
        let t = self.table("game_in_shop_item_setting.nei")?;
        for y in 1..t.rows {
            if t.exists(0, y) && listed.contains(&t.int(0, y)) {
                let mut item = base(t, y);
                item.price = t.int(7, y);
                item.menus = (8..t.columns).filter(|x| t.exists(*x, y)).map(|x| format!("{}.menu", t.string(x, y).to_lowercase())).collect();
                anyhow::ensure!(!shop.contains(&item.id), "shop item {} listed twice", item.id);
                shop.insert(item.id, item);
            }
        }
        let t = self.table("game_in_shop_item_groups_setting.nei")?;
        for y in 1..t.rows {
            if t.exists(0, y) && listed.contains(&t.int(0, y)) {
                let mut item = base(t, y);
                item.need_ids = Some((7..t.columns).filter(|x| t.exists(*x, y)).map(|x| t.int(x, y)).collect());
                anyhow::ensure!(!shop.contains(&item.id), "shop item {} listed twice", item.id);
                shop.insert(item.id, item);
            }
        }
        let t = self.table("game_in_shop_setcard_setting.nei")?;
        let mut cards = Vec::new();
        for y in 1..t.rows {
            if !t.exists(0, y) {
                continue;
            }
            let menus = t.string(0, y).split(',').map(|m| format!("{}.menu", m.to_lowercase())).collect();
            let mut ids = Vec::new();
            for x in (1..t.columns).take_while(|x| t.exists(*x, y)) {
                let id: i32 = t.string(x, y).trim().parse().context("set card shop id")?;
                anyhow::ensure!(shop.contains(&id), "set card shop item {id} does not exist");
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            cards.push((menus, ids));
        }
        self.shop = shop;
        self.shop_categories = cats;
        self.set_cards = cards;
        Ok(())
    }

    fn load_skills(&mut self) -> Result<()> {
        let list = self.table("yotogi_skil_list.nei")?;
        let ids: HashSet<i32> = (1..list.rows).filter(|y| list.exists(0, *y)).map(|y| list.int(0, y)).collect();
        let detail = self.table("yotogi_skil_detail.nei")?;
        let acq = self.table("yotogi_skil_acquisition.nei")?;
        let mut out = vec![BTreeMap::new(); SKILL_CATEGORIES.len()];
        let mut seen = HashSet::new();
        for y in 1..detail.rows {
            if !ids.contains(&detail.int(0, y)) {
                continue;
            }
            let s = skill(detail, acq, y)?;
            anyhow::ensure!(seen.insert(s.id), "yotogi skill {} registered twice", s.id);
            let c = SKILL_CATEGORIES.iter().position(|c| *c == s.category).with_context(|| format!("Yotogi.Category {}", s.category))?;
            out[c].insert(s.id, s);
        }
        self.skills = out;
        Ok(())
    }

    /// `Yotogi.GetSkillData`.
    pub fn skill(&self, id: i32) -> Option<&Skill> {
        self.skills.iter().find_map(|c| c.get(&id))
    }

    /// `Trophy.trophy_list[id].maid_point` summed over owned trophies.
    pub fn trophy_bonus_maid_point(&self, have: impl Iterator<Item = i32>) -> i32 {
        have.filter_map(|id| self.trophies.get(&id)).map(|t| t.maid_point).sum()
    }
}

/// `Yotogi.SkillData(csv, csv_acq, y, …)`.
fn skill(t: &Table, acq: &Table, y: usize) -> Result<Skill> {
    let mut n = 0;
    let mut next = || {
        n += 1;
        n - 1
    };
    let mut s = Skill { id: t.int(next(), y), category: t.string(next(), y), name: t.string(next(), y), icon_file_name: t.string(next(), y), ..Default::default() };
    for f in [&mut s.start_call_file, &mut s.start_call_file2] {
        *f = t.string(next(), y);
        if !f.is_empty() {
            f.push_str(".ks");
        }
    }
    s.exec_need_hp = t.int(next(), y);
    s.add_yotogi_class_exp = t.int(next(), y);
    let (a, b) = (t.int(next(), y), t.int(next(), y));
    s.exp_table = vec![0, a, b];
    s.only_one = t.string(next(), y) == "○";
    for i in 0..4 {
        s.exec_seikeiken[i] = t.string(next(), y) == "○";
    }
    for i in 0..5 {
        s.exec_condition[i] = t.string(next(), y) == "○";
    }
    s.drunk_skill = t.string(next(), y) == "○";
    let p = t.string(next(), y);
    if !p.is_empty() {
        s.propensity_type = parse_enum(PROPENSITY, &p).with_context(|| format!("param.Propensity {p}"))?;
    }
    let mut x = n;
    while x < t.columns {
        let head = t.string(x, 0);
        if head.is_empty() {
            break;
        }
        if t.string(x, y) == "○" {
            s.exec_stage.push(head);
        }
        x += 1;
    }
    anyhow::ensure!(acq.int(0, y) == s.id, "skill acquisition and detail id lines differ at {y}");
    let mut n = 2;
    let mut next = || {
        n += 1;
        n - 1
    };
    let g = &mut s.get;
    for i in 0..4 {
        g.seikeiken[i] = acq.string(next(), y) == "○";
    }
    for f in [&mut g.inyoku, &mut g.m_value, &mut g.hentai, &mut g.housi, &mut g.lovely, &mut g.elegance, &mut g.charm] {
        *f = acq.int(next(), y);
    }
    let x = next();
    if acq.exists(x, y) && acq.exists(1, y) {
        g.maid_class = parse_enum(MAID_CLASS, &acq.string(x, y)).with_context(|| format!("MaidClassType {}", acq.string(x, y)))?;
        g.maid_class_level = acq.int(next(), y);
    } else {
        next();
    }
    let x = next();
    if acq.exists(x, y) && acq.exists(1, y) {
        g.yotogi_class = parse_enum(YOTOGI_CLASS, &acq.string(x, y)).with_context(|| format!("YotogiClassType {}", acq.string(x, y)))?;
        g.yotogi_class_level = acq.int(next(), y);
    }
    Ok(s)
}
