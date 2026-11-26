// SPDX-License-Identifier: MPL-2.0
//! FacilityPowerUpManager's per-room recipes and per-employee receipt.
use super::*;

pub(crate) struct Recipe {
    pub id: i32,
    pub name: String,
    pub categories: Vec<i32>,
    pub conditions: Vec<String>,
    pub base: [i32; 3],
    pub eligible: bool,
}

#[derive(Clone)]
pub(crate) struct Choice {
    pub room: usize,
    pub recipe: i32,
    pub materials: Vec<i32>,
}

pub(crate) fn job(s: &mut TjsState) -> Result<i32> {
    define(
        s.retail.table("schedule_define.nei")?.as_ref(),
        "faclilityPowerUpWorkId",
    )
}

pub(crate) fn task(work: i32) -> Task {
    Task {
        id: work,
        name: "施設強化".into(),
        icon: "schedule_icon_shisetsu".into(),
        facility: 100,
        row: 0,
    }
}
pub(crate) fn assign(s: &mut TjsState, slot: usize, shift: Shift) -> Result<()> {
    let work = job(s)?;
    let id = s
        .schedule_slot(slot)
        .context("upgrade assignment requires a roster slot")?;
    let mut staged = s.clone();
    set_work(&mut staged, id, shift, work, None);
    if let Some(rooms) = facility::daily_assignments(&mut staged)? {
        staged.retail.facilities = rooms;
    }
    *s = staged;
    Ok(())
}
pub(crate) fn rooms(s: &mut TjsState) -> Result<Vec<String>> {
    let db = s.retail.table("facility.nei")?;
    s.retail
        .facilities
        .iter()
        .map(|f| {
            Ok(f.name
                .clone()
                .unwrap_or(db.string(2, row_id(&db, f.type_id)?)))
        })
        .collect()
}

fn ids(csv: &str) -> Result<Vec<i32>> {
    csv.split(',')
        .filter(|v| !v.is_empty())
        .map(|v| Ok(v.trim().parse()?))
        .collect()
}

pub(crate) fn recipes(s: &mut TjsState, room: usize) -> Result<Vec<Recipe>> {
    let ty = s
        .retail
        .facilities
        .get(room)
        .context("unknown upgrade room")?
        .type_id;
    recipes_for_type(s, ty)
}

pub(crate) fn recipes_for_type(s: &mut TjsState, ty: i32) -> Result<Vec<Recipe>> {
    let list = s.retail.table("facility_power_up.nei")?;
    let Some(r) = (1..list.rows).find(|&r| list.int(0, r) == ty) else {
        return Ok(Vec::new());
    };
    let defs = s.retail.table("facility_power_up_list.nei")?;
    let staff = s.maid(0);
    ids(&list.string(1, r))?
        .into_iter()
        .map(|id| {
            let r = row_id(&defs, id)?;
            let mut eligible = s.player.current_salon_grade >= defs.int(7, r);
            let cond = defs.string(8, r);
            if let Some(m) = staff
                && !cond.is_empty()
            {
                let (name, n) = cond
                    .split_once(',')
                    .context("malformed recipe status condition")?;
                let default = character::Status::default();
                let c = s.retail.characters.get(&m.guid_id).unwrap_or(&default);
                eligible &= character::parameter(m, c, name)? >= n.trim().parse::<i32>()?;
            }
            Ok(Recipe {
                id,
                name: defs.string(1, r),
                categories: ids(&defs.string(2, r))?,
                conditions: (3..7)
                    .map(|col| defs.string(col, r))
                    .filter(|s| !s.is_empty())
                    .collect(),
                base: std::array::from_fn(|i| defs.int(i + 9, r)),
                eligible,
            })
        })
        .collect()
}

pub(crate) fn options(s: &mut TjsState, category: i32) -> Result<Vec<(i32, String)>> {
    let db = s.retail.table("facility_material.nei")?;
    Ok((1..db.rows)
        .filter(|&r| db.int(2, r) == category && s.retail.materials.contains(&db.int(0, r)))
        .map(|r| (db.int(0, r), db.string(1, r)))
        .collect())
}

pub(crate) fn previous(s: &TjsState, room: usize, recipe: i32) -> &[i32] {
    s.retail
        .facilities
        .get(room)
        .and_then(|r| r.recipes.get(&recipe))
        .map_or(&[], Vec::as_slice)
}
pub(crate) fn choose(s: &mut TjsState, room: usize, recipe: &Recipe) -> Result<Choice> {
    let prior = previous(s, room, recipe.id).to_vec();
    let mut materials = Vec::new();
    for (slot, &cat) in recipe.categories.iter().enumerate() {
        let enabled = options(s, cat)?;
        materials.push(
            prior
                .get(slot)
                .copied()
                .filter(|v| enabled.iter().any(|(id, _)| id == v))
                .or_else(|| enabled.first().map(|(id, _)| *id))
                .unwrap_or(-1),
        );
    }
    Ok(Choice {
        room,
        recipe: recipe.id,
        materials,
    })
}
pub(crate) fn additional(s: &mut State, materials: &[i32]) -> Result<[i32; 3]> {
    let db = s.table("facility_material.nei")?;
    let mut total = [0; 3];
    for &id in materials {
        if let Some(r) = (1..db.rows).find(|&r| db.exists(0, r) && db.int(0, r) == id) {
            for (i, n) in total.iter_mut().enumerate() {
                *n += db.int(i + 3, r);
            }
        }
    }
    Ok(total)
}

impl Choice {
    pub(crate) fn enabled(&self, s: &mut TjsState) -> Result<bool> {
        let Some(recipe) = recipes(s, self.room)?
            .into_iter()
            .find(|r| r.id == self.recipe)
        else {
            return Ok(false);
        };
        if !recipe.eligible || recipe.categories.len() != self.materials.len() {
            return Ok(false);
        }
        for (&cat, &mat) in recipe.categories.iter().zip(&self.materials) {
            if !options(s, cat)?.iter().any(|(id, _)| *id == mat) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(crate) fn apply(&self, s: &mut TjsState) -> Result<()> {
        ensure!(self.enabled(s)?, "unavailable facility upgrade");
        let guid = s
            .maid(0)
            .context("no employee selected to perform the upgrade")?
            .guid_id
            .clone();
        let count = s
            .retail
            .upgrade_count
            .checked_add(1)
            .context("upgrade count overflow")?;
        s.retail.facilities[self.room]
            .recipes
            .insert(self.recipe, self.materials.clone());
        s.retail
            .upgrades
            .entry(guid)
            .or_insert_with(|| self.clone());
        s.retail.upgrade_count = count;
        Ok(())
    }
}

pub(crate) struct Receipt {
    pub room: usize,
    pub name: String,
    pub thumbnail: String,
    pub recipe: Recipe,
    pub additional: [i32; 3],
    pub names: [String; 3],
}

pub(crate) fn receipt(s: &mut TjsState, id: usize) -> Result<Option<Receipt>> {
    let guid = &s
        .maids
        .get(&id)
        .context("missing receipt employee")?
        .guid_id;
    let Some(choice) = s.retail.upgrades.get(guid).cloned() else {
        return Ok(None);
    };
    let room = choice.room;
    let ty = s
        .retail
        .facilities
        .get(room)
        .context("missing upgraded room")?
        .type_id;
    let db = s.retail.table("facility.nei")?;
    let r = row_id(&db, ty)?;
    let recipe = recipes(s, room)?
        .into_iter()
        .find(|r| r.id == choice.recipe)
        .context("missing upgraded recipe")?;
    let params = s.retail.table("facility_parameters.nei")?;
    Ok(Some(Receipt {
        room,
        name: s.retail.facilities[room]
            .name
            .clone()
            .unwrap_or_else(|| db.string(2, r)),
        thumbnail: db.string(18, r),
        recipe,
        additional: additional(&mut s.retail, &choice.materials)?,
        names: std::array::from_fn(|i| params.string(1, i + 1)),
    }))
}

pub(crate) struct RoomInfo {
    pub name: String,
    pub thumbnail: String,
    pub level: i32,
    pub staff: usize,
    pub rank: usize,
    pub staff_rank: usize,
    pub status: &'static str,
}

pub(crate) fn info(s: &mut TjsState, room: usize) -> Result<RoomInfo> {
    let shift = if s.player.flag("時間帯") == 3 {
        Shift::Night
    } else {
        Shift::Day
    };
    info_for_shift(s, room, shift)
}

pub(super) fn info_for_shift(s: &mut TjsState, room: usize, shift: Shift) -> Result<RoomInfo> {
    let db = s.retail.table("facility.nei")?;
    let f = s.retail.facilities.get(room).context("unknown facility")?;
    room_info(s, &db, f, shift)
}

pub(crate) fn planned_info(s: &mut TjsState, ty: i32) -> Result<RoomInfo> {
    let db = s.retail.table("facility.nei")?;
    let room = facility::Facility {
        type_id: ty,
        ..Default::default()
    };
    room_info(s, &db, &room, Shift::Day)
}

fn room_info(
    s: &TjsState,
    db: &crate::nei::Table,
    f: &facility::Facility,
    shift: Shift,
) -> Result<RoomInfo> {
    let r = row_id(db, f.type_id)?;
    let members = &f.assigned[shift.index()];
    let kind = db.string(4, r);
    let (fields, divisor): (&[&str], f64) = match kind.as_str() {
        "無し" => (
            &[
                "可憐",
                "気品",
                "魅惑",
                "お世話",
                "接待",
                "料理",
                "ボーカル",
                "ダンス",
                "淫欲",
                "M性",
                "変態",
                "奉仕",
            ],
            6.,
        ),
        "営業系" => (
            &[
                "可憐", "気品", "魅惑", "接待", "淫欲", "M性", "変態", "奉仕",
            ],
            4.,
        ),
        "接待系" => (&["可憐", "気品", "魅惑", "お世話", "料理"], 3.),
        "アイドル系" => (&["可憐", "気品", "魅惑", "ボーカル", "ダンス"], 3.),
        _ => (&[], 1.),
    };
    let mut score = 0;
    for id in &s.stock {
        if members.contains(&s.maids[id].guid_id) {
            let p = paid::attributes(s, *id, fields)?;
            score += (p.iter().map(|v| (v / 500.).ceil()).sum::<f64>() / divisor) as i32;
        }
    }
    let staff_rank = 1 + ids(&db.string(9, r))?
        .into_iter()
        .take_while(|threshold| score > *threshold)
        .count();
    let operating = members.len() >= db.int(20, r) as usize;
    Ok(RoomInfo {
        name: f.name.clone().unwrap_or_else(|| db.string(2, r)),
        thumbnail: db.string(18, r),
        level: s.retail.facility_exp.get(&f.type_id).map_or(1, |p| p.level),
        staff: members.len(),
        rank: db.int(8, r).try_into()?,
        staff_rank,
        status: if !operating {
            "未稼働"
        } else if db.string(14, r) == "〇" {
            "営業可能"
        } else {
            "稼働中"
        },
    })
}
