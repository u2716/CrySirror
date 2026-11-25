// SPDX-License-Identifier: MPL-2.0
//! FacilityDefaultData, ScheduleFacillityPanelCtrl and ScheduleCalcAPI.
use super::*;

pub(crate) struct Room {
    pub index: usize,
    pub physical_slot:usize,
    pub name: String,
    pub count: usize,
    pub capacity: usize,
    pub minimum: usize,
    pub level: i32,
    pub thumbnail: String,
    pub matches: bool,
    pub selected_staff: bool,
    pub rank: usize,
    pub staff_rank: usize,
    pub status: &'static str,
}

pub(crate) fn tasks(s: &mut TjsState, slot: usize, built_only: bool) -> Result<Vec<Task>> {
    let id = s.schedule_slot(slot).context("empty work slot")?;
    let table = s.retail.table("facility.nei")?;
    let mut jobs = Vec::new();
    for r in (1..table.rows).take_while(|&r| table.exists(0, r)) {
        let facility = table.int(0, r);
        let work = table.int(1, r);
        if work == 0
            || table.string(14, r) != "〇"
            || !matches!(table.string(4, r).as_str(), "営業系" | "アイドル系")
        {
            continue;
        }
        if built_only && !s.retail.facilities.iter().any(|f| f.type_id == facility) {
            continue;
        }
        let contract = table.string(40, r);
        let status = &s.maids[&id];
        if !contract.is_empty()
            && !contract.split('|').any(|c| {
                ["Trainee", "Free", "Exclusive"]
                    .get(status.contract_type as usize)
                    .copied()
                    == Some(c.trim())
            })
        {
            continue;
        }
        ensure!(
            table.string(25, r).is_empty() && (41..=43).all(|c| table.string(c, r).is_empty()),
            "unsupported facility employment conditions for {work}"
        );
        jobs.push(Task {
            id: work,
            facility,
            row: r,
            name: table.string(24, r),
            icon: table.string(19, r),
        });
    }
    Ok(jobs)
}

pub(crate) fn rooms(s: &mut TjsState, slot: usize, shift: Shift, work: i32) -> Result<Vec<Room>> {
    let task = tasks(s, slot, true)?
        .into_iter()
        .find(|t| t.id == work)
        .context("unavailable facility work")?;
    let db = s.retail.table("facility.nei")?;
    let id = s.schedule_slot(slot).context("empty room selector slot")?;
    (0..s.retail.facilities.len())
        .map(|index| {
            let info = upgrade::info_for_shift(s, index, shift)?;
            let f = &s.retail.facilities[index];
            let row = row_id(&db, f.type_id)?;
            Ok(Room {
                physical_slot:f.slot.unwrap_or(index),
                index,
                name: info.name,
                count: info.staff,
                level: info.level,
                capacity: db.int(21, row).try_into()?,
                minimum: db.int(20, row).try_into()?,
                thumbnail: info.thumbnail,
                matches: f.type_id == task.facility,
                selected_staff: f.assigned[shift.index()].contains(&s.maids[&id].guid_id),
                rank: info.rank,
                staff_rank: info.staff_rank,
                status: info.status,
            })
        })
        .collect()
}

pub(crate) fn understaffed(
    s: &mut TjsState,
    slot: usize,
    shift: Shift,
    work: i32,
) -> Result<Option<String>> {
    let id = s.schedule_slot(slot).context("empty work slot")?;
    let guid = s.maids[&id].guid_id.clone();
    let room = rooms(s, slot, shift, work)?
        .into_iter()
        .find(|r| s.retail.facilities[r.index].assigned[shift.index()].contains(&guid));
    Ok(room.map(|r| {
        format!(
            "ROOM {:02} 「{}」\n稼動に必要なメイドが足りません\nあと{}人必要です",
            r.index,
            r.name,
            r.minimum.saturating_sub(r.count)
        )
    }))
}

/// AllocationMaid switches only this shift and leaves the assignment in the
/// chosen room when several instances share a type. A duplicate click is inert.
pub(crate) fn assign_room(
    s: &mut TjsState,
    slot: usize,
    shift: Shift,
    work: i32,
    index: usize,
) -> Result<bool> {
    let room = rooms(s, slot, shift, work)?
        .into_iter()
        .find(|r| r.index == index && r.matches)
        .context("incompatible work room")?;
    ensure!(
        room.count < room.capacity,
        "この施設にはこれ以上メイドを配置出来ません。"
    );
    let id = s.schedule_slot(slot).context("empty work slot")?;
    let guid = s.maids[&id].guid_id.clone();
    let i = shift.index();
    if s.retail.facilities[index].assigned[i].contains(&guid) {
        return Ok(false);
    }
    let mut staged = s.clone();
    for f in &mut staged.retail.facilities {
        f.assigned[i].remove(&guid);
    }
    staged.retail.facilities[index].before_assignment[i].remove(&guid);
    staged.retail.facilities[index].assigned[i].insert(guid);
    set_work(&mut staged, id, shift, work, None);
    if let Some(rooms) = facility::daily_assignments(&mut staged)? {
        staged.retail.facilities = rooms;
    }
    *s = staged;
    Ok(true)
}

pub(super) fn attributes(s: &TjsState, id: usize, fields: &[&str]) -> Result<Vec<f64>> {
    let m = &s.maids[&id];
    let default = character::Status::default();
    let extra = s.retail.characters.get(&m.guid_id).unwrap_or(&default);
    fields
        .iter()
        .map(|name| Ok(f64::from(character::parameter(m, extra, name)?)))
        .collect()
}

fn stat_bonus(values: &[f64], divisor: f64, limit: f64) -> f64 {
    let sum: f64 = values.iter().map(|v| (v / 500.).ceil()).sum();
    f64::from(((sum / divisor) as f32).clamp(0., limit as f32))
}

pub(super) fn pay(
    s: &mut TjsState,
    id: usize,
    room: usize,
    shift: Shift,
    factor: f64,
) -> Result<()> {
    let f = &s.retail.facilities[room];
    let facility_type = f.type_id;
    let db = s.retail.table("facility.nei")?;
    let row = row_id(&db, facility_type)?;
    let kind = db.string(4, row);
    let performer = kind == "アイドル系";
    let sales_work = kind == "営業系";
    s.club_gauge = (s.club_gauge
        + if performer {
            3
        } else if sales_work {
            2
        } else {
            0
        })
    .clamp(0, 100);
    if factor == 0. {
        return Ok(());
    }
    let percent = |bonus: f64| (100. + bonus) / 100.;
    let level = s
        .retail
        .facility_exp
        .get(&facility_type)
        .map_or(1, |xp| xp.level);
    let room_stats = percent(f64::from(db.int(22, row) + (level - 1).max(0) * 5));
    let room_recipe = percent(f64::from(recipe_experience(&mut s.retail, room)?));
    let fields = if performer {
        ["可憐", "気品", "魅惑", "ダンス", "ボーカル"]
    } else {
        ["可憐", "魅惑", "気品", "料理", "接待"]
    };
    let mut params = attributes(s, id, &fields)?;
    let mut crew = 0.;
    if performer {
        for &active in s.active_maids.iter().flatten() {
            crew += stat_bonus(
                &attributes(
                    s,
                    active,
                    &[
                        "可憐",
                        "気品",
                        "魅惑",
                        "お世話",
                        "接待",
                        "ダンス",
                        "ボーカル",
                    ],
                )?,
                2.,
                50.,
            );
        }
    } else {
        for &other in &s.stock {
            if other != id
                && s.retail.facilities[room].assigned[shift.index()]
                    .contains(&s.maids[&other].guid_id)
            {
                for (value, other_value) in params.iter_mut().zip(attributes(s, other, &fields)?) {
                    *value += other_value * 0.1;
                }
            }
        }
        for p in &mut params {
            *p = f64::from((*p as f32).clamp(0., 9999.));
        }
    }
    let skill = stat_bonus(&params, 3., 33.) * room_stats;
    let grade = s.player.current_salon_grade;
    let count_db = s.retail.table("schedule_entertain_number.nei")?;
    let count = count_db.int(1, row_id(&count_db, grade)?);
    let guests = s.retail.table("schedule_entertain_guest.nei")?;
    let rows: Vec<_> = (1..guests.rows)
        .filter(|&r| guests.int(0, r) == grade)
        .collect();
    ensure!(
        !rows.is_empty() && count >= 0,
        "missing guest distribution for club grade {grade}"
    );
    let mut cash = 0i64;
    let mut rating = 0i32;
    for _ in 0..count {
        let guest = rows[s.random(rows.len())];
        if !performer && !sales_work {
            continue;
        }
        let sales =
            f64::from(guests.int(1, guest)) * (percent(crew + skill) * room_recipe * factor);
        cash += (sales / 1000.).ceil() as i64 * 1000;
        rating +=
            (f64::from(guests.int(2, guest)) * (percent(skill) * room_recipe * factor)) as i32;
    }
    let m = s.maids.get_mut(&id).unwrap();
    if !s
        .retail
        .characters
        .get(&m.guid_id)
        .is_some_and(|c| c.sub_name.is_some())
    {
        m.evaluation = (m.evaluation + i64::from(rating)).clamp(0, 9999);
        m.total_evaluation = (m.total_evaluation + i64::from(rating)).clamp(0, 9_999_999_999);
    }
    m.others_play_count = (m.others_play_count + count).clamp(0, 9999);
    m.sales = (m.sales + cash).clamp(0, MONEY_MAX);
    m.total_sales = (m.total_sales + cash).clamp(0, MONEY_MAX);
    s.player.salon_evaluation = (s.player.salon_evaluation + rating).clamp(0, 999);
    s.player.money = (s.player.money + cash).clamp(0, MONEY_MAX);
    Ok(())
}
