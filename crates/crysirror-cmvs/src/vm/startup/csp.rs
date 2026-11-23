// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::vm::save_slot::{Slot, csp1};
use anyhow::{Context, Result, ensure};

/// The live CSS0 suffix and common-script callbacks are overlaid while staging.
/// Publication does not run further decoding or resource lookups.
pub(crate) struct RestoreSeed {
    machine: Machine,
    control: Control,
    strings: u32,
    history_limit: u32,
}

pub(crate) struct RestoredVm {
    machine: Machine,
    control: Control,
    history: History,
    save_title: Vec<u8>,
    scene_title: Vec<u8>,
    popup: bool,
    info: [Option<u32>; 3],
    pub(crate) header: Box<[u8; 600]>,
    pub(crate) records: Vec<u8>,
}

impl RestoreSeed {
    pub(crate) fn prepare(self, slot: Slot) -> Result<RestoredVm> {
        let records = slot.checkpoint;
        let machine = self
            .machine
            .from_csp_execution(slot.main, &records, self.strings)?;
        let mut control = self.control;
        let mut history = History::cvns();
        history.set_save_limit(self.history_limit);
        let mut popup = None;
        let mut save_title = None;
        let mut scene_title = None;
        csp1::visit(&records, |tag, _, data| {
            match tag {
                0x100 | 0xff => {
                    let end = data
                        .iter()
                        .position(|&b| b == 0)
                        .context("unterminated CSP1 title")?;
                    ensure!(end < 256, "CSP1 title exceeds destination");
                    if tag == 0x100 {
                        save_title = Some(data[..end].to_vec());
                    } else {
                        scene_title = Some(data[..end].to_vec());
                    }
                }
                0x109 | 0x10b => control.restore_csp_record(tag, data, &machine)?,
                0x204 => popup = Some(u32::from_le_bytes(data.try_into()?) != 0),
                0x540 => history = History::from_csp_record(data, self.history_limit)?,
                _ => {}
            }
            Ok(())
        })?;
        let word = |at: usize| {
            Some(u32::from_le_bytes(
                slot.header[at..at + 4].try_into().unwrap(),
            ))
        };
        let info = [word(528), word(532), word(532)];
        Ok(RestoredVm {
            machine,
            control,
            history,
            popup: popup.context("CSP1 popup state missing")?,
            save_title: save_title.context("CSP1 save title missing")?,
            scene_title: scene_title.context("CSP1 scene title missing")?,
            info,
            header: Box::new(slot.header),
            records,
        })
    }
}

impl CmvsVm {
    pub(crate) fn cvns_restore_seed(&self) -> RestoreSeed {
        RestoreSeed {
            machine: self.machine.clone(),
            control: self.control.clone(),
            strings: self.cvns.persistent_start[2],
            history_limit: self.history.save_limit(),
        }
    }

    pub(crate) fn publish_cvns_vm(&mut self, restore: RestoredVm, windows: [Option<bool>; 8]) {
        self.machine = restore.machine;
        self.control = restore.control;
        self.history = restore.history;
        self.save_title = restore.save_title;
        self.scene_title = restore.scene_title;
        self.popup_enabled = restore.popup;
        // The successful tail of sub_431ce0 explicitly enables the save menu.
        self.second_save_group = true;
        self.save_information = restore.info;
        self.message_windows = windows;
        self.title = Some(self.scene_title.clone());
        self.io_generation = self.io_generation.wrapping_add(1);
    }
}
