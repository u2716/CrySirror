// SPDX-License-Identifier: MPL-2.0
//! Native save-slot labels mapped to the shared retained menu.

use crate::vm::{SaveMenu, SaveSlot, VmProfile};
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_resource::{
    pe::{PeData, PeResourceName},
    pe_menu::MenuTemplateItem,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const MENU: u32 = 130;
const LOAD_FIRST: u32 = 32736;
const SAVE_FIRST: u32 = 32746;
const VISIBLE_SLOTS: u32 = 10;

pub(super) struct NativeMenu {
    published: bool,
    cvns: bool,
    template: Value,
}

impl NativeMenu {
    pub(super) fn published(&self) -> bool {
        self.published
    }

    pub(super) fn hide(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        // A restored save can ask to hide a menu this session has not published.
        if !std::mem::take(&mut self.published) {
            return Ok(());
        }
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({
                "ui":[{"op":"remove_menu","id":MENU}]
            }))?,
        )?;
        Ok(())
    }

    pub(super) fn prepare(profile: VmProfile, pe: &PeData<'_>, canvas: [u32; 2]) -> Result<Self> {
        let menus = pe.menu_templates()?;
        let template = menus
            .iter()
            .find(|menu| menu.name == PeResourceName::Id(MENU) && menu.language == 1041)
            .context("CMVS native menu 130 missing")?;
        let mut reserved = BTreeSet::new();
        collect_ids(&template.template.items, &mut reserved)?;
        if profile != VmProfile::Cvns20040515X86 {
            ensure!(
                (LOAD_FIRST..SAVE_FIRST + VISIBLE_SLOTS).all(|id| reserved.contains(&id)),
                "CMVS native save menu slot IDs differ"
            );
        }
        let mut ids = (1..=65535).rev().filter(|id| !reserved.contains(id));
        let items = items(&template.template.items, &mut ids)?;
        if profile != VmProfile::Cvns20040515X86 {
            let root = items
                .first()
                .and_then(|item| item["children"].as_array())
                .context("CMVS menu root missing")?;
            ensure!(root.len() >= 2, "CMVS menu needs load and save groups");
        }
        Ok(Self {
            published: false,
            cvns: profile == VmProfile::Cvns20040515X86,
            template: json!({"id":MENU,"rect":[0,0,canvas[0],24],"font":"default",
            "context":true,"items":items}),
        })
    }

    pub(super) fn publish(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slots: &SaveMenu,
        enabled: bool,
        checks: &[(u32, bool)],
    ) -> Result<()> {
        let mut menu = self.template.clone();
        if !self.cvns {
            if let Some(items) = menu["items"].as_array_mut() {
                apply_checks(items, checks);
            }
        }
        let main = menu["items"][0]["children"]
            .as_array_mut()
            .context("CMVS menu root")?;
        main[0]["enabled"] = Value::Bool(true);
        main[1]["enabled"] = Value::Bool(enabled);
        if !self.cvns {
            apply_slots(main, slots);
        }
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"ui":[{"op":"set_menu","menu":menu}]}))?,
        )?;
        self.published = true;
        Ok(())
    }

    pub fn update_cvns_setting(&mut self, changes: &[(u32, bool, bool)]) {
        fn update(items: &mut [Value], changes: &[(u32, bool, bool)]) {
            for item in items {
                if let Some((_, enabled, checked)) = changes.iter().find(|(id, _, _)| item["id"].as_u64() == Some(u64::from(*id))) {
                    item["enabled"] = json!(enabled);
                    item["checked"] = json!(checked);
                }
                if let Some(children) = item["children"].as_array_mut() {
                    update(children, changes);
                }
            }
        }
        if let Some(items) = self.template["items"].as_array_mut() {
            update(items, changes);
        }
    }

    /// Update the native template even while the context menu is hidden.
    /// A later scan with fewer slots leaves the other item labels intact.
    pub fn update_cvns_slots(&mut self, slots: &SaveMenu) -> Result<()> {
        let root = self.template["items"][0]["children"].as_array_mut().context("CVNS menu root")?;
        cvns_slots(root, slots);
        Ok(())
    }
}

fn cvns_slots(items: &mut [Value], slots: &SaveMenu) {
    for item in items {
        if let Some(id) = item["id"].as_u64() {
            // The native loop writes load then save at each index. Where
            // the ID ranges overlap, the later load index wins.
            let slot = [(33100, true), (33300, false)].into_iter().find_map(|(first, load)| {
                let index = id.checked_sub(first)? as usize;
                slots.slots.get(index).map(|slot| (index, load, slot))
            });
            if let Some((index, load, slot)) = slot {
                let label = match slot {
                    SaveSlot::Recognized(header) => encoding_rs::SHIFT_JIS.decode(&header.label).0,
                    _ => "---- 未定義 ----".into(),
                };
                item["label"] = json!(format!("{:02} : {label}", (index + 1) % 100));
                item["enabled"] = json!(!load || matches!(slot, SaveSlot::Recognized(_)));
                item["checked"] = json!(slots.latest == Some(index as u16));
            }
        }
        if let Some(children) = item["children"].as_array_mut() {
            cvns_slots(children, slots);
        }
    }
}

fn collect_ids(items: &[MenuTemplateItem], used: &mut BTreeSet<u32>) -> Result<()> {
    for item in items {
        if let Some(id) = item.id.filter(|&id| id != 0) {
            ensure!(
                (1..=65535).contains(&id) && used.insert(id),
                "duplicate native CMVS menu command"
            );
        }
        collect_ids(&item.children, used)?;
    }
    Ok(())
}

fn items(native: &[MenuTemplateItem], ids: &mut impl Iterator<Item = u32>) -> Result<Vec<Value>> {
    native
        .iter()
        .map(|item| {
            let id = match item.id {
                Some(id) if id != 0 => id,
                _ => ids
                    .next()
                    .context("CMVS menu synthetic ID range exhausted")?,
            };
            let (label, shortcut) = item.text.split_once('\t').unwrap_or((&item.text, ""));
            let label = label
                .rsplit_once("(&")
                .filter(|(_, tail)| tail.ends_with(')'))
                .map_or(label, |(label, _)| label);
            Ok(
                json!({"id":id,"label":label,"shortcut":shortcut,"enabled":item.state & 3 == 0,
            "checked":item.state & 8 != 0,"separator":item.id == Some(0) && item.text.is_empty(),
            "children":items(&item.children, ids)?}),
            )
        })
        .collect()
}

/// Settings selectors set the check state of their native menu items.
fn apply_checks(items: &mut [Value], checks: &[(u32, bool)]) {
    for item in items {
        let id = item["id"].as_u64().map(|id| id as u32);
        if let Some(&(_, checked)) = checks.iter().find(|(check, _)| Some(*check) == id) {
            item["checked"] = Value::Bool(checked);
        }
        if let Some(children) = item["children"].as_array_mut() {
            apply_checks(children, checks);
        }
    }
}

fn apply_slots(items: &mut [Value], slots: &SaveMenu) {
    for item in items {
        if let Some(id) = item["id"].as_u64().map(|id| id as u32) {
            if (LOAD_FIRST..SAVE_FIRST + VISIBLE_SLOTS).contains(&id) {
                let loading = id < SAVE_FIRST;
                let index = ((id - LOAD_FIRST) % VISIBLE_SLOTS) as usize;
                let slot = slots.slots.get(index);
                item["checked"] = Value::Bool(slots.latest == Some(index as u16));
                item["enabled"] = Value::Bool(if loading {
                    matches!(slot, Some(SaveSlot::Recognized(_)))
                } else {
                    slot.is_some()
                });
                if let Some(SaveSlot::Recognized(header)) = slot {
                    item["label"] =
                        Value::String(encoding_rs::SHIFT_JIS.decode(&header.label).0.into_owned());
                }
            }
        }
        if let Some(children) = item["children"].as_array_mut() {
            apply_slots(children, slots);
        }
    }
}
