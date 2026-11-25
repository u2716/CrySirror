// SPDX-License-Identifier: MPL-2.0
//! Standalone MaidManagementMain stock mutations.
use super::*;

pub(crate) struct Access {
    pub edit: bool,
    pub lead: bool,
    pub dismiss: bool,
}

pub(crate) fn access(s: &mut TjsState, id: usize) -> Result<Access> {
    let m = s.maids.get(&id).context("unknown stock character")?;
    let extra = s.retail.characters.get(&m.guid_id);
    if let Some(name) = extra.and_then(|x| x.sub_name.clone()) {
        let t = s.retail.table("maid_status_submaid_list.nei")?;
        return Ok(Access {
            edit: t.string(3, stock::named(&t, 1, &name)?) == "〇",
            lead: false,
            dismiss: false,
        });
    }
    let day = s.player.flag("時間帯") != 3;
    Ok(Access {
        edit: true,
        lead: day && !m.leader && m.contract_type != 0 && extra.is_none_or(|x| x.old.is_none()),
        dismiss: day && !m.leader && !s.retail.main_characters.contains(&id),
    })
}

pub(crate) fn count(s: &TjsState) -> usize {
    s.stock
        .iter()
        .filter(|id| {
            s.maids.get(id).is_some_and(|m| {
                s.retail
                    .characters
                    .get(&m.guid_id)
                    .is_none_or(|x| x.sub_name.is_none())
            })
        })
        .count()
}

pub(crate) fn lead(s: &mut TjsState, id: usize) -> Result<()> {
    ensure!(access(s, id)?.lead, "character cannot be appointed");
    for &i in &s.stock {
        s.maids.get_mut(&i).context("missing stock status")?.leader = i == id;
    }
    Ok(())
}

pub(crate) fn dismiss(s: &mut TjsState, id: usize) -> Result<()> {
    ensure!(
        s.stock.len() >= 2 && access(s, id)?.dismiss,
        "character cannot be dismissed"
    );
    let m = s.maids.remove(&id).context("missing stock status")?;
    s.stock.retain(|&i| i != id);
    for v in &mut s.active_maids {
        if *v == Some(id) {
            *v = None;
        }
    }
    s.bodies.remove(&id);
    s.retail.stock_models.remove(&id);
    s.retail.characters.remove(&m.guid_id);
    s.player.money += 10_000;
    Ok(())
}

pub(crate) fn employ(s: &mut TjsState, price: i64) -> Result<usize> {
    ensure!(
        price >= 0 && s.player.flag("時間帯") != 3 && count(s) < 200 && s.player.money >= price,
        "employment is unavailable"
    );
    let mut staged = s.clone();
    stock::initialize(&mut staged, "Muku")?;
    let id = staged.active_maids[0].context("missing new stock character")?;
    staged.player.money -= price;
    *s = staged;
    Ok(id)
}

pub(crate) fn edit(s: &mut TjsState, id: usize) -> Result<()> {
    ensure!(access(s, id)?.edit, "character cannot be edited");
    facility::activate(s, 0, id, true);
    Ok(())
}
