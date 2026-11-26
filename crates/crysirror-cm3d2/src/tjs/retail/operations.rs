// SPDX-License-Identifier: MPL-2.0
//! Standalone ScheduleScene, WorkResultScene and GameMain day transitions.
pub(crate) mod paid;
pub(crate) mod upgrade;
use super::*;
use crate::params::{Exp, MONEY_MAX, WorkData};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shift {
    Day,
    Night,
}
impl Shift {
    fn index(self) -> usize {
        usize::from(self == Self::Night)
    }
    pub(crate) fn work(self, m: &crate::params::MaidStatus) -> i32 {
        if self == Self::Day {
            m.noon_work_id
        } else {
            m.night_work_id
        }
    }
    pub(crate) fn receipt_phases(self, s: &TjsState, slot: usize) -> (&'static str, &'static str) {
        let (_, result, bonus) = self.phases();
        let comm = s
            .retail
            .slot_communication
            .get(&slot)
            .is_some_and(|c| c[self.index()]);
        (
            if self == Self::Day {
                "Morning"
            } else {
                "NoonBonus"
            },
            if comm { bonus } else { result },
        )
    }
    fn phases(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Day => ("_Schedule_Noon_Resulted", "Noon", "NoonBonus"),
            Self::Night => ("_Schedule_Night_Resulted", "Night", "NightBonus"),
        }
    }
}

pub(crate) struct Task {
    pub id: i32,
    pub name: String,
    pub icon: String,
    facility: i32,
    row: usize,
}

pub(crate) fn basic_tasks(s: &mut TjsState, slot: usize) -> Result<Vec<Task>> {
    let id = s.schedule_slot(slot).context("empty schedule slot")?;
    let m = &s.maids[&id];
    if s.retail
        .characters
        .get(&m.guid_id)
        .is_some_and(|c| c.sub_name.is_some())
    {
        return Ok(Vec::new());
    }
    let contract = m.contract_type;
    let table = s.retail.table("schedule_work_noon.nei")?;
    let mut tasks = Vec::new();
    for row in (1..table.rows).take_while(|&r| !table.string(1, r).is_empty()) {
        let work = table.int(0, row);
        if !s.retail.enabled("schedule_work_noon_enabled", work)?
            || !table.string(5, row).is_empty()
        {
            continue;
        }
        let cond = table.string(20, row);
        if !cond.is_empty()
            && !cond.split('|').any(|v| {
                ["Trainee", "Free", "Exclusive"]
                    .get(contract as usize)
                    .copied()
                    == Some(v.trim())
            })
        {
            continue;
        }
        ensure!(
            (21..=23).all(|col| table.string(col, row).is_empty()),
            "unsupported basic training eligibility for {work}"
        );
        tasks.push(Task {
            id: work,
            name: table.string(1, row),
            icon: table.string(4, row),
            facility: table.int(3, row),
            row,
        });
    }
    Ok(tasks)
}

fn set_work(s: &mut TjsState, id: usize, shift: Shift, work: i32, training_room: Option<i32>) {
    let maid = s.maids.get_mut(&id).unwrap();
    match shift {
        Shift::Day => maid.noon_work_id = work,
        Shift::Night => maid.night_work_id = work,
    }
    if let Some(index) =
        training_room.and_then(|room| s.retail.facilities.iter().position(|f| f.type_id == room))
    {
        for facility in &mut s.retail.facilities {
            facility.assigned[shift.index()].remove(&maid.guid_id);
        }
        s.retail.facilities[index].before_assignment[shift.index()].remove(&maid.guid_id);
        s.retail.facilities[index].assigned[shift.index()].insert(maid.guid_id.clone());
    }
}

pub(crate) fn assign(s: &mut TjsState, slot: usize, shift: Shift, work: i32) -> Result<()> {
    let task = basic_tasks(s, slot)?
        .into_iter()
        .find(|t| t.id == work)
        .with_context(|| format!("unavailable basic training {work}"))?;
    let id = s.schedule_slot(slot).context("missing schedule maid")?;
    let mut staged = s.clone();
    set_work(&mut staged, id, shift, work, Some(task.facility));
    if let Some(facilities) = facility::daily_assignments(&mut staged)? {
        staged.retail.facilities = facilities;
    }
    *s = staged;
    Ok(())
}

pub(crate) fn communication(s: &TjsState, id: usize, shift: Shift) -> bool {
    s.retail
        .characters
        .get(&s.maids[&id].guid_id)
        .is_some_and(|status| status.communication[shift.index()])
}
pub(crate) fn toggle_communication(s: &mut TjsState, slot: usize, shift: Shift) -> Result<()> {
    let id = s.schedule_slot(slot).context("empty communication slot")?;
    let work = shift.work(&s.maids[&id]);
    ensure!(
        basic_tasks(s, slot)?.iter().any(|t| t.id == work) || paid::tasks(s,slot,true)?.iter().any(|t|t.id==work),
        "unsupported communication assignment {work}"
    );
    let enabled = !communication(s, id, shift);
    // The two NGUI radio groups are independent; optionCanBeNone is set.
    for i in facility::roster(s) {
        if i == id || (enabled && communication(s, i, shift)) {
            s.retail
                .characters
                .entry(s.maids[&i].guid_id.clone())
                .or_default()
                .communication[shift.index()] = i == id && enabled;
        }
    }
    Ok(())
}

pub(crate) fn candidates(s: &TjsState) -> Vec<usize> {
    let occupied = facility::roster(s).into_iter().collect::<BTreeSet<_>>();
    let mut ids: Vec<_> = s
        .stock
        .iter()
        .copied()
        .filter(|i| !occupied.contains(i) && s.maids.contains_key(i))
        .collect();
    s.sort_maid_standard(&mut ids);
    ids
}

/// MaidStatusListCtrl hides scheduled staff from the click selector.
pub(crate) fn choose_staff(s: &mut TjsState, slot: usize, id: usize) -> Result<()> {
    ensure!(
        slot < 40 && candidates(s).contains(&id),
        "unavailable schedule staff or slot"
    );
    let mut staged = s.clone();
    staged
        .player
        .schedule_slot
        .get_mut(slot)
        .context("invalid schedule slot")?
        .maid_guid = staged.maids[&id].guid_id.clone();
    fill_work(&mut staged)?;
    if let Some(rooms) = facility::daily_assignments(&mut staged)? {
        staged.retail.facilities = rooms;
    }
    *s = staged;
    Ok(())
}

/// UIDragDropMaidStatus.ChangeSlotData exchanges the two maid GUIDs. Work
/// IDs follow their employees; result backups belong to the original slots.
pub(crate) fn swap_staff(s: &mut TjsState, from: usize, to: usize) -> Result<()> {
    ensure!(from < 40 && to < 40 && s.player.schedule_slot.len() >= 40, "invalid schedule drop");
    if from == to { return Ok(()); }
    let mut staged=s.clone();
    let a=staged.player.schedule_slot[from].maid_guid.clone();
    let b=staged.player.schedule_slot[to].maid_guid.clone();
    staged.player.schedule_slot[to].maid_guid=a;
    staged.player.schedule_slot[from].maid_guid=b;
    if let Some(rooms)=facility::daily_assignments(&mut staged)? {staged.retail.facilities=rooms;}
    *s=staged;
    Ok(())
}

pub(crate) fn removal_prompt(s: &TjsState, slot: usize) -> Option<String> {
    let m = &s.maids[&s.schedule_slot(slot)?];
    if m.leader && facility::roster(s).len() == 1 {
        return None;
    }
    Some(format!(
        "名前：{} {}\nスケジュールから削除して\n宜しいですか。",
        m.last_name, m.first_name
    ))
}

pub(crate) fn remove_staff(s: &mut TjsState, slot: usize) -> Result<()> {
    ensure!(
        removal_prompt(s, slot).is_some(),
        "staff cannot be removed from this slot"
    );
    let mut staged = s.clone();
    staged.player.schedule_slot[slot].maid_guid.clear();
    if let Some(rooms) = facility::daily_assignments(&mut staged)? {
        staged.retail.facilities = rooms;
    }
    *s = staged;
    Ok(())
}

fn fill_work(s: &mut TjsState) -> Result<()> {
    for id in s.stock.clone() {
        for shift in [Shift::Day, Shift::Night] {
            let m = s.maids.get(&id).context("missing stock staff status")?;
            if shift.work(m) != 0 {
                continue;
            }
            if s.retail
                .characters
                .get(&m.guid_id)
                .is_some_and(|x| x.sub_name.is_some())
            {
                let defs = s.retail.table("schedule_define.nei")?;
                let work = define(&defs, "subMaidInitTaskId")?;
                let rooms = s.retail.table("facility.nei")?;
                let room = (1..rooms.rows)
                    .find(|&r| rooms.int(1, r) == work)
                    .map(|r| rooms.int(0, r));
                set_work(s, id, shift, work, room);
                continue;
            }
            let personal_id = m.personal;
            let personal = s.retail.table("maid_status_personal_list.nei")?;
            let name = personal.string(1, row_id(&personal, personal_id)?);
            let table = s.retail.table(if shift == Shift::Day {
                "schedule_init_noon.nei"
            } else {
                "schedule_init_night.nei"
            })?;
            let col = (2..table.columns)
                .find(|&c| table.string(c, 0) == name)
                .with_context(|| format!("missing schedule weights for {name}"))?;
            let choices: Vec<_> = (1..table.rows)
                .filter(|&r| table.int(0, r) > 0 && table.int(col, r) > 0)
                .map(|r| (table.int(0, r), table.int(col, r) as usize))
                .collect();
            let total = choices.iter().map(|(_, weight)| weight).sum();
            ensure!(total > 0, "empty schedule weights for {name}");
            let mut roll = s.random(total);
            let work = choices
                .into_iter()
                .find_map(|(id, w)| {
                    if roll < w {
                        Some(id)
                    } else {
                        roll -= w;
                        None
                    }
                })
                .context("invalid schedule weights")?;
            let training = s.retail.table("schedule_work_noon.nei")?;
            let room = (1..training.rows)
                .find(|&r| training.int(0, r) == work && training.string(5, r).is_empty())
                .map(|r| training.int(3, r));
            set_work(s, id, shift, work, room);
        }
    }
    Ok(())
}

pub(crate) fn open_schedule(s: &mut TjsState) -> Result<()> {
    let mut staged = s.clone();
    daily::fill_empty_roster(&mut staged)?;
    fill_work(&mut staged)?;
    *s = staged;
    Ok(())
}

fn fatigue(s: &mut State) -> Result<i32> {
    let features = s.table("maid_status_feature_list.nei")?;
    (1..features.rows)
        .find(|&r| features.string(1, r) == "疲労")
        .map(|r| features.int(0, r))
        .context("missing fatigue feature")
}

pub(crate) fn end_day(s: &mut TjsState) -> Result<()> {
    let tired = fatigue(&mut s.retail)?;
    let enabled = s
        .retail
        .enabled("maid_status_feature_enabled_list", tired)?;
    ensure!(
        s.stock.iter().all(|id| s.maids.contains_key(id)),
        "missing staff status"
    );
    for id in &s.stock {
        let m = s.maids.get_mut(id).unwrap();
        m.feature.remove(&tired);
        if enabled && m.cur_hp <= 0 && m.cur_mind <= 0 {
            m.feature.add(tired);
        }
        m.cur_hp = m.a_hp().clamp(0, 999);
        m.cur_mind = m.a_mind().clamp(0, 999);
    }
    Ok(())
}

pub(crate) fn start_day(s: &mut TjsState) -> Result<()> {
    if s.player.days == 0 {
        return day::start_before_hiring(s);
    }
    ensure!(
        !s.retail.compatibility && s.retail.legacy_files.is_empty(),
        "unsupported linked day transition"
    );
    let mut staged = s.clone();
    super::construction::advance(&mut staged)?;
    // GameMain calls DayStartManage on both sides of advancing PlayerStatus.days.
    for pass in 0..2 {
        daily::restore(&mut staged);
        daily::fill_empty_roster(&mut staged)?;
        fill_work(&mut staged)?;
        for shift in [Shift::Day, Shift::Night] {
            staged
                .player
                .generic_flag
                .insert(shift.phases().0.into(), 0);
        }
        for slot in 0..40 {
            daily::backup(&mut staged, slot, "Start")?;
        }
        if pass == 0 {
            if !staged.stock.iter().any(|id| staged.maids[id].leader)
                && let Some(id) = staged
                    .active_maids
                    .iter()
                    .flatten()
                    .next()
                    .or(staged.stock.first())
            {
                staged.maids.get_mut(id).context("missing leader")?.leader = true;
            }
            staged.player.days = staged
                .player
                .days
                .checked_add(1)
                .context("schedule day overflow")?;
        }
    }
    *s = staged;
    Ok(())
}

fn define(t: &Table, name: &str) -> Result<i32> {
    (0..t.rows)
        .rfind(|&r| t.string(0, r) == name)
        .map(|r| t.int(1, r))
        .with_context(|| format!("missing schedule define {name}"))
}

fn trait_bonus(s: &mut State, maid: &crate::params::MaidStatus) -> Result<BTreeMap<String, i32>> {
    let mut bonus = BTreeMap::new();
    for (kind, ids) in [("feature", &maid.feature), ("propensity", &maid.propensity)] {
        if ids.is_empty() {
            continue;
        }
        let names = s.table(&format!("maid_status_{kind}_list.nei"))?;
        let values = s.table(&format!("maid_status_{kind}_correction.nei"))?;
        for &id in ids.keys() {
            let name = names.string(1, row_id(&names, id)?);
            if let Some(r) = (1..values.rows).find(|&r| values.string(0, r) == name) {
                for (k, v) in stock::corrections(&values.string(1, r))? {
                    *bonus.entry(k).or_default() += v;
                }
            }
        }
    }
    Ok(bonus)
}

fn add_job_exp(
    s: &mut State,
    maid: &mut crate::params::MaidStatus,
    c: &mut character::Status,
    delta: i32,
) -> Result<()> {
    let id = maid.current_maid_class;
    let Some(prev) = c.job_levels.get(&id).copied() else {
        return Ok(());
    };
    let list = s.table("maid_status_jobclass_list.nei")?;
    let name = list.string(1, row_id(&list, id)?);
    let table = s.table("maid_status_jobclass_experiences.nei")?;
    let row = (1..table.rows)
        .find(|&r| table.string(0, r) == name)
        .context("missing job experience table")?;
    let mut xp = Exp::default();
    xp.set_list(
        (1..table.columns)
            .take_while(|&col| table.exists(col, row))
            .map(|col| table.int(col, row))
            .collect(),
    );
    xp.add(*c.class_exp[0].get(&id).context("missing job experience")? + delta);
    c.class_exp[0].insert(id, xp.total);
    c.job_levels.insert(id, xp.level);
    if xp.level != prev {
        let jobs = c
            .job_levels
            .iter()
            .map(|(&id, &lv)| Ok((list.string(1, row_id(&list, id)?), lv)))
            .collect::<Result<_>>()?;
        stock::class_bonus(s, maid, c, &jobs)?;
    }
    Ok(())
}

fn recipe_experience(s: &mut State, room: usize) -> Result<i32> {
    let facility = s.facilities.get(room).context("missing room for recipe")?;
    let facility_type = facility.type_id;
    let materials=facility.recipes.values().flatten().copied().collect::<Vec<_>>();
    let mut gain = upgrade::additional(s,&materials)?[2];
    let recipes = s.table("facility_power_up.nei")?;
    if let Some(row) = (1..recipes.rows).find(|&r| recipes.int(0, r) == facility_type) {
        let definitions = s.table("facility_power_up_list.nei")?;
        for recipe in recipes.string(1, row).split(',').filter(|v| !v.is_empty()) {
            gain += definitions.int(11, row_id(&definitions, recipe.trim().parse()?)?);
        }
    }
    Ok(gain)
}

/// FacilityManager shares experience by facility type. New rooms have empty
/// material slots, but their recipes already contribute the base parameters.
fn award_room_exp(s: &mut State, room: usize) -> Result<()> {
    let facility_type=s.facilities[room].type_id;
    let rooms = s.table("facility.nei")?;
    let mut gain = rooms.int(13, row_id(&rooms, facility_type)?);
    gain += recipe_experience(s, room)?;
    if !s.facility_exp.contains_key(&facility_type) {
        let levels = s.table("facility_level_table.nei")?;
        let row = (1..levels.rows).find(|&r| levels.int(0, r) == facility_type);
        let mut xp = Exp::default();
        xp.set_list(
            (0..5)
                .map(|i| {
                    row.filter(|&r| levels.exists(i + 2, r))
                        .map_or(i as i32 * 100, |r| levels.int(i + 2, r))
                })
                .collect(),
        );
        s.facility_exp.insert(facility_type, xp);
    }
    s.facility_exp.get_mut(&facility_type).unwrap().add(gain);
    Ok(())
}

fn apply_work(s: &mut TjsState, slot: usize, shift: Shift) -> Result<()> {
    let id = s.schedule_slot(slot).context("empty work result slot")?;
    let mut m = s.maids[&id].clone();
    let work = shift.work(&m);
    let training = basic_tasks(s,slot)?.into_iter().find(|t|t.id == work);
    let power_up = work == upgrade::job(s)?;
    let paid = training.is_none() && !power_up;
    let task = match training {
        Some(t) => t,
        None if power_up => upgrade::task(work),
        None => paid::tasks(s,slot,false)?.into_iter().find(|v| v.id == work)
            .with_context(|| format!("unsupported standalone work result {work} in slot {slot}"))?,
    };
    let room = s.retail.facilities.iter().position(|f| f.assigned[shift.index()].contains(&m.guid_id));
    let mut c = s
        .retail
        .characters
        .get(&m.guid_id)
        .cloned()
        .unwrap_or_default();
    let defs = s.retail.table("schedule_define.nei")?;
    let comm = c.communication[shift.index()];
    let success = if comm {
        1
    } else if m.feature.contains(&fatigue(&mut s.retail)?) {
        0
    } else {
        let roll = 1 + s.random(99) as i32;
        if roll <= define(&defs, "workSuccessLvMissIncidence")? {
            0
        } else if roll > 100 - define(&defs, "workSuccessLvPerfectIncidence")? {
            2
        } else {
            1
        }
    };
    let facilities = s.retail.table("facility.nei")?;
    let fr = row_id(&facilities, task.facility)?;
    let success = if power_up { if s.retail.upgrades.contains_key(&m.guid_id) {1} else {3} } else if paid && room.is_none_or(|i|
        s.retail.facilities[i].assigned[shift.index()].len() < facilities.int(20,fr) as usize) { 3 } else { success };
    let slot_state = &mut s.player.schedule_slot[slot];
    match shift {
        Shift::Day => slot_state.noon_success_level = success,
        Shift::Night => slot_state.night_success_level = success,
    }
    s.retail.slot_communication.entry(slot).or_default()[shift.index()] = comm;
    m.generic_flag
        .insert("_communication".into(), i32::from(comm));
    let count = m
        .work_data
        .get(&work)
        .map_or(0, |w| w.play_count)
        .checked_add(u32::from(success != 3))
        .context("work count overflow")?;
    let level = if count < define(&defs, "noonWorkLv2Exp")? as u32 {
        1
    } else if count < define(&defs, "noonWorkLv3Exp")? as u32 {
        2
    } else {
        3
    };
    if success != 3 { m.work_data.insert(
        work,
        WorkData {
            id: work,
            play_count: count,
            level,
        },
    ); }
    let factor = if success == 3 { 0. } else if comm {
        1.3
    } else {
        [0.5, 1., 1.5][success as usize]
    };
    let level_bonus = (level - 1) * 10;
    let room_bonus = (s
        .retail
        .facility_exp
        .get(&task.facility)
        .map_or(1, |xp| xp.level)
        - 1)
    .max(0)
        * 5;
    let room_factor = (100 + facilities.int(22, fr)) as f64 / 100.;
    let bonus = trait_bonus(&mut s.retail, &m)?;
    let t = if paid { facilities } else { s.retail.table("schedule_work_noon.nei")? };
    let offset = if paid {20} else {0};
    let base = |col| if power_up {0} else {t.int(col + offset, task.row)};
    let value = |col, name| {
        (base(col) as f64
            * ((100 + level_bonus + room_bonus + bonus.get(name).copied().unwrap_or(0)) as f64
                / 100.)
            * room_factor
            * factor)
            .ceil() as i32
    };
    let divisor = define(&defs, "dispatchIncomCoefficient")?;
    ensure!(divisor > 0, "invalid dispatch income coefficient");
    let base_income =
        (base(18) as f64 * ((100 + level_bonus) as f64 / 100.) * factor).ceil();
    let income = (base_income * ((m.a_care() as f64 / divisor as f64).ceil() / 100. + 1.)) as i64;
    if c.sub_name.is_none() {
        m.study_rate = (m.study_rate + base(6)).clamp(0, 1000);
        for (i, (field, name)) in [
            (&mut m.reception, "接待"),
            (&mut m.care, "お世話"),
            (&mut m.lovely, "可憐"),
            (&mut m.elegance, "気品"),
            (&mut m.charm, "魅惑"),
            (&mut m.teach_rate, "指導"),
            (&mut c.cooking, "料理"),
            (&mut c.vocal, "ボーカル"),
            (&mut c.dance, "ダンス"),
            (&mut c.appeal, "AP"),
        ]
        .into_iter()
        .enumerate()
        {
            *field = (*field + value(7 + i, name)).clamp(0, 9999);
        }
        c.appeal = c
            .appeal
            .min(200 + ((c.dance + c.bonus_dance).clamp(0, 9999) / 1000) * 200);
        add_job_exp(&mut s.retail, &mut m, &mut c, value(17, ""))?;
    }
    m.sales = (m.sales + income).clamp(0, MONEY_MAX);
    m.total_sales = (m.total_sales + income).clamp(0, MONEY_MAX);
    s.player.money = (s.player.money + income).clamp(0, MONEY_MAX);
    s.retail.characters.insert(m.guid_id.clone(), c);
    s.maids.insert(id, m);
    let (_, phase, bonus_phase) = shift.phases();
    if (paid || power_up) && let Some(index) = room { paid::pay(s,id,index,shift,factor)?; }
    daily::backup(s, slot, phase)?;
    if let Some(index)=room {award_room_exp(&mut s.retail,index)?;}
    daily::backup(s, slot, bonus_phase)
}

/// WorkResultScene reads this difference before writing the bonus snapshot.
fn play_delta(s: &TjsState, slot: usize, shift: Shift) -> i32 {
    let backup = &s.player.schedule_slot[slot].backup_status;
    let (before, after) = match shift {
        Shift::Day => ("Morning_yotogi_play_count", "NoonBonus_yotogi_play_count"),
        Shift::Night => ("NoonBonus_yotogi_play_count", "NightBonus_yotogi_play_count"),
    };
    backup.get(after).copied().unwrap_or(0) - backup.get(before).copied().unwrap_or(0)
}

pub(crate) fn receipt(s: &mut TjsState, slot: usize, shift: Shift) -> Result<(i32, (&'static str, &'static str))> {
    let id = s.schedule_slot(slot).context("empty result slot")?;
    let work = shift.work(&s.maids[&id]);
    if facility::night_roles(&mut s.retail)?.get(&work).is_some_and(|r| r == "HaveSex") {
        let phases = if shift == Shift::Day { ("Morning", "NoonBonus") } else { ("NoonBonus", "NightBonus") };
        return Ok((i32::from(play_delta(s, slot, shift) != 0), phases));
    }
    let data = &s.player.schedule_slot[slot];
    let level = if shift == Shift::Day { data.noon_success_level } else { data.night_success_level };
    Ok((level, shift.receipt_phases(s, slot)))
}

pub(crate) fn settle(s: &mut TjsState, shift: Shift) -> Result<()> {
    if s.player.flag(shift.phases().0) > 0
        && (shift == Shift::Day || s.player.flag(Shift::Day.phases().0) > 0)
    {
        return Ok(());
    }
    let mut staged = s.clone();
    let night_roles = facility::night_roles(&mut staged.retail)?;
    for period in [Shift::Day, Shift::Night] {
        if staged.player.flag(period.phases().0) == 0 {
            for slot in 0..40 {
                let Some(id) = staged.schedule_slot(slot) else { continue };
                let work = period.work(&staged.maids[&id]);
                if night_roles.get(&work).is_some_and(|r| r == "HaveSex") {
                    let played = play_delta(&staged, slot, period) > 0;
                    let flags = &mut staged.maids.get_mut(&id).unwrap().generic_flag;
                    flags.insert("_YotogiPlayed".into(), i32::from(played));
                    daily::backup(&mut staged, slot, period.phases().2)?;
                    let m = staged.maids.get_mut(&id).unwrap();
                    m.generic_flag.insert(format!("_PlayedNightWorkId{}", m.night_work_id), 1);
                } else {
                    apply_work(&mut staged, slot, period)?;
                }
            }
            staged
                .player
                .generic_flag
                .insert(period.phases().0.into(), 1);
        }
        if period == shift {
            break;
        }
    }
    for (threshold,trophy) in [(5,1180),(10,1190)] {
        if staged.retail.upgrade_count >= threshold { award(&mut staged,trophy)?; }
    }
    *s = staged;
    Ok(())
}

pub(crate) fn income(s: &TjsState) -> [i64; 6] {
    let Some(i) = (0..40).find(|&i| s.schedule_slot(i).is_some()) else {
        return [0; 6];
    };
    let backup = &s.player.schedule_slot[i].backup_status;
    let old = i64::from(*backup.get("Start_clubMoney").unwrap_or(&0));
    let evaluation = i64::from(*backup.get("Start_clubEvaluation").unwrap_or(&0));
    let current = i64::from(s.player.salon_evaluation);
    [
        old,
        s.player.money - old,
        s.player.money,
        evaluation,
        current - evaluation,
        current,
    ]
}

pub(crate) fn portrait(
    s: &TjsState,
    id: usize,
) -> Option<&std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>> {
    s.retail
        .characters
        .get(&s.maids.get(&id)?.guid_id)?
        .thumbnail_icon
        .as_ref()
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires COM3D2_ROOT standalone installation"]
    fn installed_cafe_sales_follow_room_and_shift() -> Result<()> {
        let (_tmp, mut vm) = super::super::tests::installed_tjs()?;
        let s = &mut vm.runtime.host_mut().state;
        s.player.current_salon_grade = 1;
        s.player.schedule_slot.resize_with(40, Default::default);
        for id in 0..2 {
            let m = crate::params::MaidStatus {
                personal: 100,
                guid_id: format!("cafe-{id}"),
                ..Default::default()
            };
            s.player.schedule_slot[id].maid_guid = m.guid_id.clone();
            s.maids.insert(id, m);
        }
        s.stock = vec![0, 1];
        s.active_maids.fill(None);
        let cafe = s.retail.facilities.len();
        for _ in 0..2 {
            s.retail.facilities.push(facility::Facility {type_id: 120, ..Default::default()});
        }
        paid::assign_room(s, 0, Shift::Day, 3002, cafe)?;
        paid::assign_room(s, 1, Shift::Night, 3002, cafe)?;
        paid::assign_room(s, 1, Shift::Day, 3002, cafe+1)?;
        let other = s.maids.get_mut(&1).unwrap();
        other.lovely = 9999;
        other.charm = 9999;
        other.elegance = 9999;
        other.reception = 9999;
        s.retail.characters.entry(other.guid_id.clone()).or_default().cooking = 9999;
        assert_eq!(paid::attributes(s,0,&["可憐","魅惑","気品","料理","接待"])?,[0.;5]);
        // The seeded guest indices 7 and 2 are 70,000 and 90,000 at grade 1.
        // Another worker in the wrong room or on the other shift adds zero.
        for same_room in [false, true] {
            let mut shift = s.clone();
            if same_room {
                paid::assign_room(&mut shift, 1, Shift::Day, 3002, cafe)?;
                assert!(!shift.retail.facilities[cafe+1].assigned[0].contains("cafe-1"));
            }
            shift.seed(7);
            let cash = shift.player.money;
            paid::pay(&mut shift,0,cafe,Shift::Day,1.)?;
            // Two steps per stat for the co-worker, /3, then the cafe's +20%.
            // Each sale rounds up to a thousand before the employee receives it.
            let expected = if same_room { 73_000 + 94_000 } else { 70_000 + 90_000 };
            assert_eq!(shift.player.money-cash,expected);
            assert_eq!(shift.maids[&0].sales,expected);
            assert_eq!(shift.maids[&0].others_play_count,2);
            assert_eq!(shift.maids[&0].evaluation,2);
        }
        close_daily(s,true)?;
        let cash = s.player.money;
        settle(s, Shift::Day)?;
        assert_eq!(s.player.money,cash);
        assert_eq!(s.player.schedule_slot[0].noon_success_level,3);
        assert_eq!(s.player.schedule_slot[1].noon_success_level,3);
        Ok(())
    }

    #[test]
    #[ignore = "requires COM3D2_ROOT standalone installation"]
    fn installed_eight_basic_jobs_on_both_shifts() -> Result<()> {
        let (_tmp, mut vm) = super::super::tests::installed_tjs()?;
        let initial = &mut vm.runtime.host_mut().state;
        initial
            .player
            .schedule_slot
            .resize_with(40, Default::default);
        for m in initial.maids.values_mut() {
            m.personal = 100;
        }
        open_schedule(initial)?;
        // Each job has its own installed base gain, before room/class bonuses.
        for (job, key, base) in [
            (2001, "care", 140),
            (2002, "charm", 180),
            (2003, "elegance", 180),
            (2004, "lovely", 110),
            (2005, "charm", 230),
            (2006, "cooking", 240),
            (2007, "vocal", 260),
            (2008, "dance", 260),
        ] {
            for shift in [Shift::Day, Shift::Night] {
                let mut s = initial.clone();
                let id = s.schedule_slot(0).unwrap();
                let m = s.maids.get_mut(&id).unwrap();
                m.study_rate = 100;
                s.retail
                    .characters
                    .entry(m.guid_id.clone())
                    .or_default()
                    .communication = [true; 2];
                assign(&mut s, 0, shift, job)?;
                close_daily(&mut s, true)?;
                if shift == Shift::Night {
                    s.player
                        .generic_flag
                        .insert("_Schedule_Noon_Resulted".into(), 1);
                    daily::backup(&mut s, 0, "NoonBonus")?;
                }
                let before = s.player.money;
                settle(&mut s, shift)?;
                let b = &s.player.schedule_slot[0].backup_status;
                let (from, to) = shift.receipt_phases(&s, 0);
                assert_eq!(
                    b.get(&format!("{to}_{key}")).unwrap()
                        - b.get(&format!("{from}_{key}")).unwrap(),
                    (f64::from(base) * 1.3).ceil() as i32,
                    "{job} {shift:?}"
                );
                assert_eq!(s.maids[&id].study_rate, 90);
                assert_eq!(s.maids[&id].work_data.get(&job).unwrap().play_count, 1);
                assert_eq!(s.player.money, before);
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires COM3D2_ROOT standalone installation"]
    fn installed_training_settlement_and_second_day() -> Result<()> {
        let (_tmp, mut vm) = super::super::tests::installed_tjs()?;
        let s = &mut vm.runtime.host_mut().state;
        let stock = std::mem::take(&mut s.stock);
        s.active_maids.fill(None);
        s.player.schedule_slot.resize_with(40, Default::default);
        start_day(s)?;
        assert_eq!(s.player.days, 1);
        s.stock = stock;
        for m in s.maids.values_mut() {
            m.personal = 100;
        }
        open_schedule(s)?;
        let m = &s.maids[&0];
        assert!((2001..=2003).contains(&m.noon_work_id));
        assert_eq!(m.night_work_id, 10000);
        assert_eq!(basic_tasks(s, 0)?.len(), 8);
        assign(s, 0, Shift::Day, 2001)?;
        assign(s, 0, Shift::Night, 2003)?;
        assert!(assign(s, 39, Shift::Day, 2001).is_err());
        assert!(assign(s, 0, Shift::Day, 99999).is_err());
        let m = s.maids.get_mut(&0).unwrap();
        m.current_maid_class = 10;
        m.study_rate = 100;
        m.work_data.insert(
            2001,
            WorkData {
                id: 2001,
                play_count: 19,
                level: 2,
            },
        );
        m.feature.add(10);
        m.hp = 110;
        m.mind = 90;
        m.cur_hp = 0;
        m.cur_mind = 0;
        let c = s.retail.characters.entry(m.guid_id.clone()).or_default();
        c.job_levels.insert(10, 1);
        c.class_exp[0].insert(10, 0);
        c.communication = [true, false];
        s.player.money = 123456;
        s.player.salon_evaluation = 321;
        close_daily(s, true)?;
        let start_care = s.maids[&0].care;
        let start_class = s.maids[&0].current_maid_class;
        // Failure in the last occupied cell must not settle or advance the RNG of the first.
        s.player.schedule_slot[39].maid_guid = "character-1".into();
        s.maids.get_mut(&1).unwrap().noon_work_id = 99999;
        assert!(settle(s, Shift::Day).is_err());
        assert_eq!(s.maids[&0].work_data.get(&2001).unwrap().play_count, 19);
        assert_eq!(s.player.flag("_Schedule_Noon_Resulted"), 0);
        s.player.schedule_slot[39].maid_guid.clear();
        s.maids.get_mut(&1).unwrap().noon_work_id = 2002;
        settle(s, Shift::Day)?;
        assert_eq!(
            s.maids[&0].work_data.get(&2001).unwrap(),
            &WorkData {
                id: 2001,
                play_count: 20,
                level: 3
            }
        );
        assert_eq!(
            s.maids[&0].care - start_care,
            (140.0_f64 * 1.30 * 1.30).ceil() as i32
        );
        assert_eq!(s.maids[&0].study_rate, 90);
        assert_eq!(
            s.retail.characters["character-0"].class_exp[0][&start_class],
            16
        );
        assert_eq!(s.player.schedule_slot[0].noon_success_level, 1);
        assert_eq!(
            (
                s.retail.facility_exp[&100].total,
                s.retail.facility_exp[&100].level
            ),
            (15, 2)
        );
        assert_eq!(s.retail.slot_communication[&0], [true, false]);
        let tired = fatigue(&mut s.retail)?;
        s.maids.get_mut(&0).unwrap().feature.add(tired);
        settle(s, Shift::Night)?;
        assert_eq!(s.player.schedule_slot[0].night_success_level, 0);
        assert_eq!(s.retail.characters["character-0"].vocal, 32);
        assert_eq!(s.retail.facility_exp[&100].total, 30);
        assert_eq!(s.maids[&0].study_rate, 80);
        let after = s.maids[&0].clone();
        settle(s, Shift::Night)?;
        assert_eq!(s.maids[&0], after);
        assert_eq!(income(s), [0, 123456, 123456, 0, 321, 321]);
        let b = &s.player.schedule_slot[0].backup_status;
        for phase in ["Morning", "Noon", "NoonBonus", "Night", "NightBonus"] {
            assert_eq!(
                b.iter()
                    .filter(|(k, _)| k.starts_with(&format!("{phase}_")))
                    .count(),
                32
            );
        }
        s.maids.get_mut(&1).unwrap().feature.add(tired);
        s.maids.get_mut(&1).unwrap().cur_mind = 1;
        end_day(s)?;
        assert!(s.maids[&0].feature.contains(&tired));
        assert!(!s.maids[&1].feature.contains(&tired));
        assert_eq!(s.maids[&0].cur_hp, s.maids[&0].a_hp());
        assert_eq!(s.player.days, 1);
        s.maids.get_mut(&0).unwrap().night_work_id = 2002;
        start_day(s)?;
        assert_eq!(s.player.days, 2);
        assert!(s.maids[&0].leader);
        assert_eq!(s.maids[&0].night_work_id, 2003);
        assert_eq!(s.player.flag("_Schedule_Night_Resulted"), 0);
        assert_eq!(income(s), [123456, 0, 123456, 321, 0, 321]);
        Ok(())
    }
}
