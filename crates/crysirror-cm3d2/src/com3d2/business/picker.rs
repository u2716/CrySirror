// SPDX-License-Identifier: MPL-2.0
//! Single-character selection used by YotogiMain and facility improvement.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::com3d2) enum PickMode {
    Yotogi,
    Facility,
}

impl PickMode {
    pub fn from_tag(tag: Option<&String>) -> Result<Self> {
        match tag.map(String::as_str) {
            Some("Yotogi") => Ok(Self::Yotogi),
            Some("Facility") => Ok(Self::Facility),
            mode => bail!("unsupported standalone SceneCharacterSelect type {mode:?}"),
        }
    }
    pub fn camera(self) -> (&'static str, [f32; 3], f32, [f32; 2]) {
        match self {
            Self::Yotogi => (
                "Theater",
                [12.39468, 5.495444, 22.72065],
                0.1,
                [-6.047941, 14.65832],
            ),
            Self::Facility => (
                "EmpireClub_Rotary",
                [6.673284, 3.031431, 20.00305],
                0.2,
                [562.5145, -4.906824],
            ),
        }
    }
}

#[derive(Default)]
pub(super) struct Picker {
    candidates: Vec<usize>,
    pub(super) selected: Option<usize>,
    pub(super) dialog: Option<quit_dialog::QuitDialog>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PickExit {
    Select(usize),
    Cancel,
}
impl PickExit {
    pub(super) fn finish(self, s: &mut TjsState) -> Result<&'static str> {
        match self {
            Self::Select(id) => {
                let guid = &s
                    .maids
                    .get(&id)
                    .context("missing selected employee")?
                    .guid_id;
                let pos = s
                    .chara_guid_list
                    .iter()
                    .position(|g| g == guid)
                    .context("employee no longer in the character selection list")?;
                s.chara_guid_list.remove(pos);
                if s.active_maids.is_empty() {
                    s.active_maids.push(None);
                }
                s.active_maids[0] = Some(id);
                Ok("label")
            }
            Self::Cancel => {
                s.chara_guid_list.clear();
                Ok("cancel_label")
            }
        }
    }
}

impl Panel {
    pub(super) fn init_picker(&mut self, mode: PickMode, allow_cancel: bool, s: &TjsState) {
        self.picker.candidates = s
            .chara_guid_list
            .iter()
            .filter_map(|guid| {
                s.stock
                    .iter()
                    .copied()
                    .find(|id| s.maids.get(id).is_some_and(|m| &m.guid_id == guid))
            })
            .collect();
        s.sort_maid_standard(&mut self.picker.candidates);
        self.picker.selected = self.picker.candidates.first().copied();
        self.base
            .retain(|w| allow_cancel || !w.path.ends_with("/ButtonParent/Cancel"));
        for w in &mut self.base {
            if w.path.ends_with("/ExplanatoryText/Value") {
                text(
                    w,
                    match mode {
                        PickMode::Yotogi => "夜伽を行うメイドを選択してください。",
                        PickMode::Facility => "施設強化を行うメイドを選択してください。",
                    },
                );
            }
        }
    }

    pub(super) fn picker_widgets(
        &mut self,
        s: &TjsState,
        assets: &mut Assets,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Vec<Widget>> {
        self.bindings.clear();
        let mut widgets = self.base.clone();
        for w in &widgets {
            if w.path.ends_with("/ButtonParent/Cancel") {
                self.bindings.insert(w.path.clone(), Action::Cancel);
            } else if w.path.ends_with("/ButtonParent/OK") && self.picker.selected.is_some() {
                self.bindings.insert(w.path.clone(), Action::Ok);
            }
        }
        self.staff_widgets(
            self.picker.candidates.clone(),
            s,
            assets,
            host,
            client,
            &mut widgets,
        )?;
        widgets.sort_by_key(|w| w.depth);
        Ok(widgets)
    }

    pub(super) fn picker_input(
        &mut self,
        mode: PickMode,
        widgets: &mut Vec<Widget>,
        s: &TjsState,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
        pressed: &mut Option<usize>,
    ) -> Result<Option<PickExit>> {
        if let Some(dialog) = self.picker.dialog.as_mut() {
            if let Some(choice) = dialog.update(widgets, input, released, now) {
                widgets.truncate(dialog.first);
                self.picker.dialog = None;
                if choice == quit_dialog::Choice::Ok {
                    return Ok(Some(PickExit::Cancel));
                }
            }
            return Ok(None);
        }
        let action = self.click(
            widgets,
            Scene::CharacterSelect(mode),
            &assets.operations_layout,
            &input,
            released,
            pressed,
        );
        match action {
            Some(Action::Ok) => return Ok(self.picker.selected.map(PickExit::Select)),
            Some(Action::Cancel) if mode == PickMode::Yotogi => {
                let dialog = assets.scene(op, host, client, "SystemDialog")?;
                self.picker.dialog = Some(quit_dialog::QuitDialog::show(
                    widgets,
                    dialog,
                    now,
                    "まだ夜伽を実行していないメイドがいます。\n夜伽を終了しますか？",
                )?);
            }
            Some(Action::Cancel) => return Ok(Some(PickExit::Cancel)),
            Some(Action::Choose(id)) => {
                self.picker.selected = Some(id);
                *widgets = self.picker_widgets(s, assets, host, client)?;
            }
            None if scroll(
                &mut self.staff_scroll,
                &input,
                assets.operations_layout.staff,
                self.picker.candidates.len(),
            ) =>
            {
                *widgets = self.picker_widgets(s, assets, host, client)?;
            }
            None => {}
            _ => bail!("unexpected character selection action {action:?}"),
        }
        Ok(None)
    }
}

impl Game {
    pub(in crate::com3d2) fn open_picker(
        &mut self,
        mode: PickMode,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        self.actors.reset_for_day(s);
        self.actors.slots.retain(|&(man, _), _| man);
        s.active_maids.fill(None);
        let mut panel = Panel {
            base: std::mem::take(&mut self.widgets),
            plate: self.assets.scene(op, host, client, "PickerPlate")?,
            ..Default::default()
        };
        panel.init_picker(
            mode,
            self.script
                .tag_backup
                .get("cancel_label")
                .is_some_and(|s| !s.is_empty()),
            s,
        );
        self.widgets = panel.picker_widgets(s, &mut self.assets, host, client)?;
        self.business = panel;
        self.camera_control = true;
        self.camera_move = None;
        self.dialogue = None;
        self.message_open = false;
        self.adv_cover = None;
        self.world_cover = None;
        self.sound.stop(host, client, Kind::Voice, 0.)?;
        self.sound.play(
            host,
            client,
            &self.files,
            Kind::Bgm,
            "BGM015.ogg",
            0.5,
            true,
        )?;
        Ok(())
    }

    pub(in crate::com3d2) fn update_picker(
        &mut self,
        mode: PickMode,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        let (exit, key) = {
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let s = &mut vm.runtime.host_mut().state;
            let Some(exit) = self.business.picker_input(
                mode,
                &mut self.widgets,
                s,
                &mut self.assets,
                op,
                host,
                client,
                input,
                released,
                now,
                &mut self.pressed,
            )?
            else {
                return Ok(());
            };
            (exit, exit.finish(s)?)
        };
        if let PickExit::Select(id) = exit {
            self.actors
                .activate_stock(&self.script, id, 0, Some(true))?;
            self.actors.apply(
                crate::kag::AdvUi::AllProcProp {
                    man: false,
                    slot: 0,
                },
                &self.script,
                &self.files,
                &mut self.assets,
                op,
                now,
            )?;
        }
        self.business = Panel::default();
        self.resume_at(key, op, host, client, now)
    }
}
