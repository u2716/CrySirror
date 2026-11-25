// SPDX-License-Identifier: MPL-2.0
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) enum Mode {
    Default,
    Edited,
    #[default]
    Heroine,
}

#[derive(Clone, Default)]
pub(crate) struct Wardrobe {
    pub mode: Mode,
    pub edits: Option<BTreeMap<String, Option<String>>>,
    pub guid: String,
    pub(super) unforced: std::collections::BTreeSet<String>,
}

impl Wardrobe {
    pub(crate) fn forced(&self, name: &str) -> bool {
        self.edits
            .as_ref()
            .is_some_and(|items| items.contains_key(name))
            && !self.unforced.contains(name)
    }

    pub(crate) fn set_button(&mut self, name: &str, active: bool) {
        // 1.0.4 SetButtonActive writes acckamisub after *every* button, including bulk updates.
        for key in [name, "acckamisub"] {
            if self
                .edits
                .as_ref()
                .is_some_and(|items| items.contains_key(key))
            {
                if active {
                    self.unforced.remove(key);
                } else {
                    self.unforced.insert(key.into());
                }
            }
        }
    }
}
pub(crate) fn settings(s: &State, slot: usize) -> Result<&Wardrobe> {
    if let Some(order) = s.construction.get(&slot) {
        return Ok(&order
            .as_ref()
            .context("room scheduled for demolition")?
            .wardrobe);
    }
    Ok(&s.facilities[s.facility_index(slot).context("empty wardrobe room")?].costume)
}
pub(crate) fn settings_mut(s: &mut State, slot: usize) -> Result<&mut Wardrobe> {
    if s.construction.contains(&slot) {
        return Ok(&mut s
            .construction
            .get_mut(&slot)
            .and_then(Option::as_mut)
            .context("room scheduled for demolition")?
            .wardrobe);
    }
    let i = s.facility_index(slot).context("empty wardrobe room")?;
    Ok(&mut s.facilities[i].costume)
}
pub(crate) fn enabled(s: &mut State) -> Result<Vec<&'static str>> {
    let t = s.table("facility_costume_mpn_enable_list.nei")?;
    (1..t.rows)
        .take_while(|&r| t.exists(0, r))
        .map(|r| character::mpn_name(&t.string(0, r)))
        .collect()
}
pub(crate) fn defaults(
    s: &mut State,
    id: i32,
    editing: bool,
) -> Result<(BTreeMap<String, Option<String>>, String)> {
    let t = s.table("facility_default_costume.nei")?;
    let mut result = BTreeMap::new();
    let Some(col) = (1..t.columns).find(|&c| t.int(c, 3) == id) else {
        return Ok((result, String::new()));
    };
    let allowed = enabled(s)?;
    for r in (4..t.rows).take_while(|&r| t.exists(0, r)) {
        let file = t.string(col, r);
        let Ok(name) = character::mpn_name(&t.string(0, r)) else {
            return Ok((result, file));
        };
        if !allowed.contains(&name) {
            continue;
        }
        let prop = if file.is_empty() && !editing {
            None
        } else if file.is_empty() || file.eq_ignore_ascii_case("del") {
            preset::delete_menu(name).map(str::to_owned)
        } else {
            Some(format!("{file}.menu"))
        };
        result.insert(name.into(), prop);
    }
    Ok((result, String::new()))
}
pub(crate) fn prepare(s: &mut TjsState, slot: usize) -> Result<Option<usize>> {
    let ty =
        construction::Catalog::effective(&s.retail, slot).context("missing costume subject")?;
    let actors = s
        .active_maids
        .iter()
        .flatten()
        .chain(s.stock.iter())
        .copied()
        .collect::<Vec<_>>();
    let id = actors
        .iter()
        .copied()
        .find(|i| s.maids.get(i).is_some_and(|m| m.leader))
        .or_else(|| actors.first().copied());
    let Some(id) = id else { return Ok(None) };
    let edits = match settings(&s.retail, slot)?.edits.clone() {
        Some(v) => v,
        None => defaults(&mut s.retail, ty, true)?.0,
    };
    let guid = if settings(&s.retail, slot)?.guid.is_empty() {
        Some(s.guid())
    } else {
        None
    };
    let mut model = (**s
        .retail
        .stock_models
        .get(&id)
        .context("missing wardrobe model")?)
    .clone();
    for (name, file) in &edits {
        if let Some(file) = file {
            model.set_prop_file(name, file, true)?;
        }
    }
    let (files, storage) = s.retail.editor_files()?;
    model.all_proc_prop(&files, storage.as_ref())?;
    s.retail.stock_models.insert(id, Arc::new(model));
    let w = settings_mut(&mut s.retail, slot)?;
    if let Some(guid) = guid {
        w.guid = guid;
    }
    w.edits = Some(edits);
    s.active_maids[0] = Some(id);
    s.retail.edit_facility = Some(slot);
    Ok(Some(id))
}
pub(crate) fn finish(s: &mut TjsState, slot: usize) -> Result<()> {
    let id = s.active_maids[0].context("missing wardrobe editor character")?;
    let mut model = (**s
        .retail
        .stock_models
        .get(&id)
        .context("missing wardrobe model")?)
    .clone();
    let mut edits = settings(&s.retail, slot)?.edits.clone().unwrap_or_default();
    for mpn in enabled(&mut s.retail)? {
        let i = model
            .prop_names
            .iter()
            .position(|&v| v == mpn)
            .context("missing costume property")?;
        edits.insert(mpn.into(), Some(model.props[i].temp_file.clone()));
        model.reset_prop(mpn)?;
    }
    let (files, storage) = s.retail.editor_files()?;
    model.all_proc_prop(&files, storage.as_ref())?;
    model.visible = false;
    s.retail.stock_models.insert(id, Arc::new(model));
    settings_mut(&mut s.retail, slot)?.edits = Some(edits);
    s.retail.edit_facility = None;
    Ok(())
}

#[cfg(test)]
pub(crate) fn assert_equipped_uniform(
    vm: &mut crate::tjs::Tjs,
    id: usize,
    slot: usize,
) -> Result<()> {
    let s = &mut vm.runtime.host_mut().state;
    let i = s.retail.facility_index(slot).context("missing test cafe")?;
    let shift = if s.player.flag("時間帯") == 3 {
        operations::Shift::Night
    } else {
        operations::Shift::Day
    };
    let roster_slot = (0..s.player.schedule_slot.len())
        .find(|&n| s.schedule_slot(n) == Some(id))
        .context("missing employee on roster")?;
    operations::paid::assign_room(s, roster_slot, shift, 3002, i)?;
    s.active_maids[0] = Some(id);
    vm.eval("SetMaidCostumeFacility(0)")?;
    let s = &mut vm.runtime.host_mut().state;
    assert!(s.retail.appearances[&id].slots["skirt"].temp.is_none());
    settings_mut(&mut s.retail, slot)?.set_button("skirt", true);
    vm.eval("SetMaidCostumeFacility(0)")?;
    let s = &vm.runtime.host().state;
    assert_eq!(
        s.retail.appearances[&id].slots["skirt"]
            .temp
            .as_ref()
            .unwrap()
            .0,
        "dress243_skrt_i_.menu"
    );
    Ok(())
}
