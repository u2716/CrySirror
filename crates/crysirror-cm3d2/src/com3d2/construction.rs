// SPDX-License-Identifier: MPL-2.0
//! Construction panel of the standalone SceneFacilityManagement level.
use super::*;
use crate::tjs::{
    TjsState,
    retail::{
        construction::{Catalog, SLOTS},
        operations::upgrade,
    },
};
use widgets::power::{Layout, Rect};
mod information;
mod rename;
use crate::tjs::retail::wardrobe::{self, Mode};
const WARDROBE: &str = "/Window Facility Heroine Costume";
use information::{MENU, MINI, RECIPE_VIEW, RECIPES, Tab};

const DETAILS: &str = "/Window Facility Details";
const ROOM: &str = "/Window Facility Details/Parent Facility Button/temp Image Facility Info";
const INFO: &str = "/Window Facility Details/Facility Info";
const TYPES: &str = "/Window Facility Type List";
const VIEW: &str = "/Window Facility Type List/Scroll View/Viewport";
const OPTION: &str = "/Window Facility Type List/Scroll View/Viewport/Parent Button/temp Toggle";
#[derive(Clone, Copy, PartialEq, Debug)]
enum Action {
    Slot(usize),
    Place,
    Type(i32),
    ShowTab(Tab),
    InspectSlot(usize),
    Rename(usize),
    ChangeMode(Mode),
    EditCostume,
    ScrollRecipes(bool),
    Destroy,
    Reset,
    Ok,
    Cancel,
    Details,
    Scroll(bool),
}
#[derive(Clone, Copy)]
enum Confirm {
    EnterTypes,
    Reserve(Option<i32>),
    Clear,
    Notice,
}
pub(super) struct Screen {
    layout: Layout,
    catalog: Catalog,
    slot: usize,
    tab: Tab,
    inspected: Option<usize>,
    recipe_scroll: usize,
    editing: Option<rename::Edit>,
    begin_edit: bool,
    preview: Option<(String, String)>,
    picking: bool,
    choice: Option<i32>,
    scroll: usize,
    show_info: bool,
    bindings: Vec<(Rect, Action)>,
    pressed: Option<Action>,
    confirm: Option<Confirm>,
    pub dialog: Option<quit_dialog::QuitDialog>,
}
fn hit(r: Rect, i: &InputFrame) -> bool {
    let [x, y, w, h] = r;
    let (px, py) = (i.pointer_x as f32, i.pointer_y as f32);
    px >= x && px < x + w && py >= y && py < y + h
}
fn text(w: &mut Widget, v: &str) {
    if let Some(l) = &mut w.label {
        l.set_text(v);
    }
}
impl Screen {
    fn load(
        s: &mut TjsState,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<Self> {
        let layout = assets.facility_canvas(
            op,
            host,
            c,
            "SceneFacilityManagement",
            "Canvas uGUI (SceneFacilityManagement)",
        )?;
        for path in [ROOM, OPTION, VIEW, "/Button OK", "/Button Cancel"] {
            layout.rect(path)?;
        }
        Ok(Self {
            layout,
            catalog: Catalog::load(&mut s.retail)?,
            slot: 0,
            tab: Tab::Build,
            inspected: None,
            recipe_scroll: 0,
            editing: None,
            begin_edit: false,
            preview: None,
            picking: false,
            choice: None,
            scroll: 0,
            show_info: false,
            bindings: vec![],
            pressed: None,
            confirm: None,
            dialog: None,
        })
    }
    fn widgets(
        &mut self,
        s: &mut TjsState,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<Vec<Widget>> {
        self.bindings.clear();
        let mut out = Vec::new();
        let listing = self.tab != Tab::Build || self.picking;
        let selected = Catalog::effective(&s.retail, self.slot);
        let cost = self
            .catalog
            .total(&s.retail, self.choice.map(|id| (self.slot, id)))?;
        for mut w in self.layout.base.iter().cloned() {
            if w.path.starts_with(ROOM) || w.path.starts_with(OPTION) || w.path.starts_with(INFO) {
                continue;
            }
            if w.path.starts_with(WARDROBE)
                || w.path.starts_with(MINI)
                || w.path.starts_with(RECIPES)
                || (self.tab != Tab::Build && w.path.starts_with("/Window Money Parameter"))
                || (self.picking && w.path.starts_with(MENU))
                || (listing && w.path.starts_with(DETAILS))
                || (!listing && w.path.starts_with(TYPES))
                || (!self.picking && w.path == "/Button Cancel")
            {
                continue;
            }
            if w.path.ends_with("/Text Number Now") {
                text(&mut w, &daily::money(s.player.money));
            }
            if w.path.ends_with("/Text Number Cost") {
                text(&mut w, &daily::money(cost));
            }
            if w.path.starts_with(MENU) {
                if let Some(tab) = Tab::for_path(&w.path) {
                    if w.path.ends_with("/Checkmark") && tab != self.tab {
                        continue;
                    }
                    if w.path.ends_with("/Background") {
                        self.bindings.push((w.rect, Action::ShowTab(tab)));
                    }
                } else if w.path.contains("Toggle Facility Facility") {
                    continue;
                }
            }
            let action = match w.path.rsplit('/').next().unwrap_or("") {
                "Button OK" if !self.picking || self.choice.is_some() => Some(Action::Ok),
                "Button Cancel" => Some(Action::Cancel),
                "Button Create" => Some(Action::Place),
                "Button Destroy" if selected.is_some() => Some(Action::Destroy),
                "Button RESET" if !s.retail.construction.is_empty() => Some(Action::Reset),
                "Background" if w.path.contains("Toggle Visible Facility Info") => {
                    Some(Action::Details)
                }
                _ => None,
            };
            if let Some(action) = action {
                self.bindings.push((w.rect, action));
            }
            out.push(w);
        }
        if listing {
            let choices: Vec<_> = match self.tab {
                Tab::Build => self
                    .catalog
                    .choices(s, self.slot)
                    .into_iter()
                    .map(|v| (v, Action::Type(v.id)))
                    .collect(),
                Tab::Catalog => self
                    .catalog
                    .available(s)
                    .map(|v| (v, Action::Type(v.id)))
                    .collect(),
                Tab::Wardrobe => (0..SLOTS)
                    .filter_map(|slot| Catalog::effective(&s.retail, slot).map(|id| (slot, id)))
                    .map(|(slot, id)| Ok((self.catalog.get(id)?, Action::InspectSlot(slot))))
                    .collect::<Result<_>>()?,
                Tab::Current => (0..SLOTS)
                    .filter_map(|slot| Catalog::live(&s.retail, slot).map(|id| (slot, id)))
                    .map(|(slot, id)| Ok((self.catalog.get(id)?, Action::InspectSlot(slot))))
                    .collect::<Result<_>>()?,
            };
            let first = self.layout.rect(OPTION)?;
            let view = self.layout.rect(VIEW)?;
            let step = first[3] + self.layout.list_spacing;
            ensure!(step > 0., "invalid facility type list spacing");
            let visible = ((view[3] + self.layout.list_spacing) / step)
                .floor()
                .max(1.) as usize;
            self.scroll = self.scroll.min(choices.len().saturating_sub(visible));
            for (row, (def, action)) in choices
                .into_iter()
                .skip(self.scroll)
                .take(visible)
                .enumerate()
            {
                let rect = [first[0], first[1] + row as f32 * step, first[2], first[3]];
                for mut w in self.layout.instantiate(OPTION, rect)? {
                    if w.path.ends_with("/Checkmark")
                        && (self.choice != Some(def.id)
                            || matches!(action, Action::InspectSlot(slot) if self.inspected != Some(slot)))
                    {
                        continue;
                    }
                    let label = if let Action::InspectSlot(slot) = action
                        && self.tab == Tab::Wardrobe
                    {
                        self.catalog.display_name(&s.retail, slot)?.to_owned()
                    } else if let Action::InspectSlot(slot) = action {
                        let i = s
                            .retail
                            .facility_index(slot)
                            .context("missing current room")?;
                        upgrade::info(s, i)?.name
                    } else {
                        def.name.clone()
                    };
                    text(&mut w, &label);
                    out.push(w);
                }
                self.bindings.push((rect, action));
            }
            for w in &out {
                if w.path.ends_with("/Button Slider Movement") {
                    self.bindings
                        .push((w.rect, Action::Scroll(w.rect[1] > view[1] + view[3] / 2.)));
                }
            }
            if let Some(id) = self.choice {
                self.show_information(s, id, assets, op, host, c, &mut out)?;
            }
        } else {
            for slot in 0..SLOTS {
                let rect = self.layout.cell(ROOM, slot, SLOTS)?;
                self.bindings.push((rect, Action::Slot(slot)));
                let live = Catalog::live(&s.retail, slot);
                let id = Catalog::effective(&s.retail, slot).or(live);
                let def = id.map(|id| self.catalog.get(id)).transpose()?;
                let pending = s
                    .retail
                    .construction
                    .get(&slot)
                    .map(|v| v.as_ref().map(|p| p.id));
                let info = if self.show_info || slot == self.slot {
                    match (pending, id, s.retail.facility_index(slot)) {
                        (Some(Some(ty)), _, _) => Some(upgrade::planned_info(s, ty)?),
                        (_, Some(_), Some(i)) => Some(upgrade::info(s, i)?),
                        _ => None,
                    }
                } else {
                    None
                };
                for mut w in self.layout.instantiate(ROOM, rect)? {
                    if (w.path.contains("/Button Change Name") && def.is_none())
                        || w.path.ends_with("/Image Check Mark")
                        || (w.path.contains("/Parent Info") && !self.show_info)
                        || (w.path.ends_with("/Image Change")
                            && !pending.is_some_and(|v| v.is_some()))
                        || (w.path.ends_with("/Image Delete")
                            && !(pending == Some(None) && live.is_some()))
                    {
                        continue;
                    }
                    if w.path.ends_with("/Button Change Name") {
                        self.bindings.push((w.rect, Action::Rename(slot)));
                    }
                    if w.path == ROOM {
                        if let Some(def) = def {
                            assets.facility_thumbnail(op, host, c, &mut w, &def.thumbnail)?;
                        } else {
                            w.tint = [0, 0, 0, 64];
                        }
                    }
                    if w.path.ends_with("/Facility Name/Text")
                        || w.path.ends_with("/InputField Facility Name/Text")
                    {
                        text(
                            &mut w,
                            self.editing
                                .as_ref()
                                .filter(|e| e.slot == slot)
                                .map(|e| e.value.as_str())
                                .unwrap_or(self.catalog.display_name(&s.retail, slot)?),
                        );
                    }
                    if self.set_room_field(
                        &mut w,
                        info.as_ref(),
                        rect,
                        self.layout.rect(ROOM)?,
                        &mut out,
                    )? {
                        continue;
                    }
                    out.push(w);
                }
                if slot == self.slot {
                    for mut w in self.layout.instantiate(INFO, self.layout.rect(INFO)?)? {
                        if self.set_room_field(
                            &mut w,
                            info.as_ref(),
                            self.layout.rect(INFO)?,
                            self.layout.rect(INFO)?,
                            &mut out,
                        )? {
                            continue;
                        }
                        if w.path.ends_with("/Text Facility Name") {
                            text(&mut w, self.catalog.display_name(&s.retail, slot)?);
                        }
                        if w.path.ends_with("/Image Facility Thumbnail") {
                            if let Some(def) = def {
                                assets.facility_thumbnail(op, host, c, &mut w, &def.thumbnail)?;
                            } else {
                                w.tint = [0, 0, 0, 64];
                            }
                        }
                        out.push(w);
                    }
                }
            }
        }
        Ok(out)
    }
    fn wardrobe_widgets(
        &mut self,
        s: &mut TjsState,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let slot = self.inspected.context("missing wardrobe room")?;
        let mode = wardrobe::settings(&s.retail, slot)?.mode;
        for mut w in self
            .layout
            .instantiate(WARDROBE, self.layout.rect(WARDROBE)?)?
        {
            if let Some(child) = w.path.split("/Parent Radio Buttons/").nth(1) {
                let target = match child.split('/').next() {
                    Some("Toggle") => Mode::Default,
                    Some("Toggle (2)") => Mode::Edited,
                    Some("Toggle (1)") => Mode::Heroine,
                    _ => continue,
                };
                if child.ends_with("/Background") {
                    self.bindings.push((w.rect, Action::ChangeMode(target)));
                }
                if child.ends_with("/Checkmark") && mode != target {
                    continue;
                }
                if child.ends_with("/Button Costume Setting") && mode == Mode::Edited {
                    self.bindings.push((w.rect, Action::EditCostume));
                }
            }
            if w.path.ends_with("/image thumbnail") {
                let asset = match mode {
                    Mode::Default => {
                        let ty = self.choice.context("missing facility type")?;
                        let name = wardrobe::defaults(&mut s.retail, ty, false)?.1;
                        let (files, _) = s.retail.editor_files()?;
                        if name.is_empty() || !files.exists(&format!("{name}.tex")) {
                            continue;
                        }
                        assets.schedule_icon(op, host, c, &files, &name)?
                    }
                    Mode::Heroine => continue,
                    Mode::Edited => {
                        let guid = &wardrobe::settings(&s.retail, slot)?.guid;
                        if guid.is_empty() {
                            continue;
                        }
                        if self.preview.as_ref().is_none_or(|(key, _)| key != guid) {
                            use crysirror_core::path::{Mount, VfsPath};
                            let mut path =
                                VfsPath::new(Mount::Save, &format!("Thumb/f_cos_tmp_{guid}.png"))?;
                            if !op.storage.exists(&path) {
                                path =
                                    VfsPath::new(Mount::Save, &format!("Thumb/f_cos_{guid}.png"))?;
                            }
                            if !op.storage.exists(&path) {
                                continue;
                            }
                            let image = crysirror_image::decode_standard(&op.storage.read(&path)?)?;
                            let asset = host.upload_rgba(
                                c,
                                0x4a000000,
                                image.width,
                                image.height,
                                &image.rgba,
                            )?;
                            self.preview = Some((guid.clone(), asset.relative().to_owned()));
                        }
                        self.preview
                            .as_ref()
                            .context("missing facility preview")?
                            .1
                            .clone()
                    }
                };
                w.asset = asset;
            }
            out.push(w);
        }
        Ok(())
    }
    fn set_room_field(
        &self,
        w: &mut Widget,
        info: Option<&upgrade::RoomInfo>,
        destination: Rect,
        source: Rect,
        out: &mut Vec<Widget>,
    ) -> Result<bool> {
        if w.path.ends_with("/RawImage") {
            let count = info.map_or(0, |v| {
                if w.path.contains("parent stars Rank/") {
                    v.rank
                } else {
                    v.staff_rank
                }
            });
            for index in 0..count {
                let mut star = w.clone();
                star.rect = self.layout.cell(&w.path, index, count)?;
                star.rect[0] += destination[0] - source[0];
                star.rect[1] += destination[1] - source[1];
                out.push(star);
            }
            return Ok(true);
        }
        if w.path.contains("Text Level of facility") {
            text(
                w,
                &info.map_or_else(|| "--".into(), |v| v.level.to_string()),
            );
        }
        if w.path.contains("Text Number of people") {
            text(
                w,
                &info.map_or_else(|| "---".into(), |v| v.staff.to_string()),
            );
        }
        if w.path.contains("Text Status") {
            text(w, info.map_or("---", |v| v.status));
        }
        Ok(false)
    }
    fn alert(
        &mut self,
        confirm: Confirm,
        message: &str,
        widgets: &mut Vec<Widget>,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        let notice = matches!(confirm, Confirm::Notice);
        let dialog = assets.scene(
            op,
            host,
            c,
            if notice {
                "SystemDialog"
            } else {
                "YesNoDialog"
            },
        )?;
        self.dialog = Some(quit_dialog::QuitDialog::show(
            widgets, dialog, now, message,
        )?);
        if notice {
            widgets.retain(|w| !w.path.contains("SystemDialog/Base/Cancel"));
        }
        self.confirm = Some(confirm);
        Ok(())
    }
    fn input(
        &mut self,
        s: &mut TjsState,
        widgets: &mut Vec<Widget>,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<bool> {
        if let Some(d) = &mut self.dialog {
            let Some(choice) = d.update(widgets, input, released, now) else {
                return Ok(false);
            };
            widgets.truncate(d.first);
            self.dialog = None;
            let confirm = self
                .confirm
                .take()
                .context("missing facility confirmation action")?;
            if choice == quit_dialog::Choice::Cancel {
                return Ok(false);
            }
            let mut notice = None;
            match confirm {
                Confirm::EnterTypes => {
                    self.picking = true;
                    self.choice = None;
                    self.scroll = 0;
                }
                Confirm::Reserve(id) => {
                    let name = if let Some(id) = id {
                        self.catalog.get(id)?.name.clone()
                    } else {
                        self.catalog.display_name(&s.retail, self.slot)?.to_owned()
                    };
                    self.catalog.reserve(s, self.slot, id)?;
                    self.picking = false;
                    self.choice = None;
                    notice = Some(if id.is_some() {
                        format!("『{name}』\nを設定しました。")
                    } else {
                        format!("『{name}』\nを撤去予定に設定しました。")
                    });
                }
                Confirm::Clear => {
                    s.retail.construction.clear();
                    self.picking = false;
                    self.choice = None;
                    notice = Some("施設の設定をリセットしました。".into());
                }
                Confirm::Notice => return Ok(false),
            }
            *widgets = self.widgets(s, assets, op, host, c)?;
            if let Some(msg) = notice {
                self.alert(Confirm::Notice, &msg, widgets, assets, op, host, c, now)?;
            }
            return Ok(false);
        }
        if self.editing.is_some() && self.update_name(s, widgets, assets, op, host, c, &input)? {
            return Ok(false);
        }
        let target = self
            .bindings
            .iter()
            .rev()
            .find(|(r, _)| hit(*r, &input))
            .map(|(_, a)| *a);
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            self.pressed = target;
        }
        let wheel = input.buttons_pressed & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) != 0;
        let down = input.buttons_pressed & buttons::WHEEL_DOWN != 0;
        let listing = self.picking || self.tab != Tab::Build;
        let action = if listing
            && self.choice.is_some()
            && wheel
            && hit(self.layout.rect(RECIPE_VIEW)?, &input)
        {
            Some(Action::ScrollRecipes(down))
        } else if listing
            && hit(self.layout.rect(VIEW)?, &input)
            && input.buttons_pressed & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) != 0
        {
            Some(Action::Scroll(
                input.buttons_pressed & buttons::WHEEL_DOWN != 0,
            ))
        } else if released & buttons::MOUSE_LEFT != 0 && self.pressed.take() == target {
            target
        } else {
            None
        };
        let Some(a) = action else { return Ok(false) };
        let current = Catalog::effective(&s.retail, self.slot)
            .map(|id| self.catalog.get(id))
            .transpose()?;
        let current_name = self.catalog.display_name(&s.retail, self.slot)?.to_owned();
        let prompt = match a {
            Action::Slot(slot) => {
                self.slot = slot;
                None
            }
            Action::ChangeMode(mode) => {
                wardrobe::settings_mut(&mut s.retail, self.inspected.context("missing room")?)?
                    .mode = mode;
                None
            }
            Action::EditCostume => {
                self.begin_edit = true;
                return Ok(true);
            }
            Action::Rename(slot) => {
                self.start_name(s, host, c, slot)?;
                None
            }
            Action::ShowTab(tab) => {
                if self.tab == tab {
                    return Ok(false);
                }
                self.tab = tab;
                self.scroll = 0;
                self.choice = None;
                self.inspected = None;
                self.recipe_scroll = 0;
                self.show_info = false;
                None
            }
            Action::InspectSlot(slot) => {
                self.inspected = Some(slot);
                self.choice = if self.tab == Tab::Wardrobe {
                    Catalog::effective(&s.retail, slot)
                } else {
                    Catalog::live(&s.retail, slot)
                };
                self.recipe_scroll = 0;
                None
            }
            Action::ScrollRecipes(down) => {
                self.recipe_scroll = if down {
                    self.recipe_scroll.saturating_add(1)
                } else {
                    self.recipe_scroll.saturating_sub(1)
                };
                None
            }
            Action::Type(id) => {
                self.recipe_scroll = 0;
                self.choice = Some(id);
                None
            }
            Action::Details => {
                self.show_info = !self.show_info;
                None
            }
            Action::Scroll(down) => {
                self.scroll = if down {
                    self.scroll + 1
                } else {
                    self.scroll.saturating_sub(1)
                };
                None
            }
            Action::Cancel => {
                self.picking = false;
                self.choice = None;
                None
            }
            Action::Reset => Some((
                Confirm::Clear,
                "施設の設定をリセットします。\nよろしいですか？".into(),
            )),
            Action::Place => Some(match current {
                Some(f) if !f.removable => (
                    Confirm::Notice,
                    format!("『{}』\nは変更出来ない施設です。", current_name),
                ),
                Some(_) => (
                    Confirm::EnterTypes,
                    format!(
                        "『{}』\nが配置されています。\n施設を変更しますか？",
                        current_name
                    ),
                ),
                None => (Confirm::EnterTypes, "空き部屋に施設を配置しますか？".into()),
            }),
            Action::Destroy => {
                let f = current.context("no facility to destroy")?;
                Some(if f.removable {
                    (
                        Confirm::Reserve(None),
                        format!(
                            "『{}』\nを撤去します。\nよろしいですか？\n※施設強化及び服装設定もリセットされます",
                            current_name
                        ),
                    )
                } else {
                    (
                        Confirm::Notice,
                        format!("『{}』\nは撤去出来ない施設です。", current_name),
                    )
                })
            }
            Action::Ok if !self.picking => return Ok(true),
            Action::Ok => {
                let id = self.choice.context("missing construction choice")?;
                let name = &self.catalog.get(id)?.name;
                Some(
                    if self.catalog.total(&s.retail, Some((self.slot, id)))? > s.player.money {
                        (Confirm::Notice, "資金が不足する設定は出来ません。".into())
                    } else {
                        (
                            Confirm::Reserve(Some(id)),
                            match current {
                                None => format!("『{name}』\nを設定します。\nよろしいですか？"),
                                Some(_) => format!(
                                    "『{}』\nを、\n『{name}』\nに設定します。\nよろしいですか？",
                                    current_name
                                ),
                            },
                        )
                    },
                )
            }
        };
        if let Some((action, message)) = prompt {
            self.alert(action, &message, widgets, assets, op, host, c, now)?;
        } else {
            *widgets = self.widgets(s, assets, op, host, c)?;
        }
        Ok(false)
    }
}
impl Game {
    pub(super) fn open_construction(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<()> {
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        let mut ui = Screen::load(s, &mut self.assets, op, host, c)?;
        if self.script.tag_backup.contains_key("update_costume") {
            let slot = s
                .retail
                .edit_facility
                .context("facility editor return without selected room")?;
            wardrobe::finish(s, slot)?;
            ui.tab = Tab::Wardrobe;
            ui.inspected = Some(slot);
            ui.choice = Catalog::effective(&s.retail, slot);
            self.actors.reset_for_day(s);
        }
        self.widgets = ui.widgets(s, &mut self.assets, op, host, c)?;
        self.construction = Some(ui);
        Ok(())
    }
    pub(super) fn update_construction(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(start) = self.leaving {
            if now.saturating_sub(start) >= 500 {
                if let Some(ui) = &self.construction
                    && ui.begin_edit
                {
                    let slot = ui.inspected.context("missing edit room")?;
                    let id = {
                        let mut vm = self
                            .script
                            .tjs
                            .lock()
                            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
                        wardrobe::prepare(&mut vm.runtime.host_mut().state, slot)?
                    };
                    if let Some(id) = id {
                        self.actors
                            .activate_stock(&self.script, id, 0, Some(true))?;
                        let vm = self
                            .script
                            .tjs
                            .lock()
                            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
                        *self
                            .actors
                            .actor(false, 0)
                            .context("missing wardrobe actor")? =
                            (*vm.runtime.host().state.retail.stock_models[&id]).clone();
                        self.actors.show_editor_stock()?;
                    }
                    self.resume_at("label_edit", op, host, c, now)?;
                } else if self.script.tag_backup.contains_key("label") {
                    self.resume_script(op, host, c, now)?;
                } else {
                    self.script.jump_label(
                        &self.files,
                        op.storage.as_ref(),
                        "*施設の管理画面を抜ける",
                    )?;
                    self.script.enabled = true;
                    self.script.exec();
                    self.enter(Scene::Script, op, host, c, now)?;
                }
            }
            return Ok(());
        }
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let done = self
            .construction
            .as_mut()
            .context("missing construction screen")?
            .input(
                &mut vm.runtime.host_mut().state,
                &mut self.widgets,
                &mut self.assets,
                op,
                host,
                c,
                input,
                released,
                now,
            )?;
        drop(vm);
        if done {
            self.leaving = Some(now);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
