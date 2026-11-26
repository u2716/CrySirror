// SPDX-License-Identifier: MPL-2.0
//! GameInShop, GameInShopMain and ShopItem standalone purchase controls.
use super::*;
use crate::tjs::{TjsState, retail::shop::Catalog};
use crysirror_unity::reader::Reader;

pub(super) const CAMERA_OFFSET: f32 = 1920. * (181. / 256. - 0.5);
const LISTS: [&str; 3] = ["MainCategoryViewer", "SubCategoryViewer", "ItemList"];
type Rect = [f32; 4];

#[derive(Default, Clone, Copy)]
pub(super) struct Grid {
    pub clip: Rect,
    pub step: f32,
    pub x: f32,
}
#[derive(Default, Clone, Copy)]
pub(super) struct Layout {
    grids: [Grid; 3],
    info_origin: [f32; 2],
    info_offset: [f32; 2],
}
impl Layout {
    pub fn read(&mut self, class: &str, path: &str, offset: [f32; 3], data: &[u8]) -> Result<()> {
        if class == "ItemInfoWnd" {
            self.info_origin = [960. + offset[0], 540. - offset[1]];
            let mut r = Reader::new(data);
            r.seek(32)?;
            self.info_offset = r.f32s::<2>()?;
        }
        let Some(index) = LISTS
            .iter()
            .position(|name| path.contains(&format!("/{name}/Content")))
        else {
            return Ok(());
        };
        self.grids[index].read(class, path, offset, data)
    }
}

impl Grid {
    pub fn read(&mut self, class: &str, path: &str, offset: [f32; 3], data: &[u8]) -> Result<()> {
        let mut r = Reader::new(data);
        if class == "UIPanel" && (path.ends_with("/Content") || path.ends_with("/Contents") || path.ends_with("/PlayableInfo/PlayableInfo")) {
            r.seek(172)?;
            let [x, y, w, h] = r.f32s::<4>()?;
            r.seek(204)?;
            let [dx, dy] = r.f32s::<2>()?;
            self.clip = [
                960. + offset[0] + x + dx - w / 2.,
                540. - offset[1] - y - dy - h / 2.,
                w,
                h,
            ];
        } else if class == "UIGrid" {
            r.seek(52)?;
            self.step = r.f32()?;
            self.x = offset[0];
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Main(usize),
    Sub(usize),
    Buy(i32),
    Icon(i32),
    TryOn,
    Restore,
    Camera,
    Scroll(usize, bool),
    Exit,
}

#[derive(Default)]
pub(super) struct Screen {
    catalog: Catalog,
    base: Vec<Widget>,
    tooltip: Vec<Widget>,
    icons: Vec<(Rect, i32, f32, String)>,
    tooltip_first: usize,
    hover: Option<i32>,
    button: Vec<Widget>,
    item: Vec<Widget>,
    layout: Layout,
    selected: usize,
    last_sub: std::collections::BTreeMap<String, usize>,
    offset: [usize; 3],
    max_offset: [usize; 3],
    hits: Vec<(Rect, Action)>,
    pressed: Option<Action>,
    pub dialog: Option<quit_dialog::QuitDialog>,
    pending: Option<i32>,
    selected_item: Option<i32>,
    wardrobe: Option<Box<crate::chara::Maid>>,
    pub initial_camera: Option<Orbit>,
}
fn inside(r: Rect, input: &InputFrame) -> bool {
    let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}
impl Screen {
    fn load(
        s: &mut TjsState,
        bg: i32,
        base: Vec<Widget>,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Self> {
        let catalog = Catalog::open(s, bg)?;
        ensure!(
            !catalog.categories.is_empty(),
            "no shop lineup for background {bg}"
        );
        for g in assets.shop_layout.grids {
            ensure!(
                g.step > 0. && g.clip[3] >= g.step,
                "missing serialized shop grid"
            );
        }
        let (tooltip, base) = base
            .into_iter()
            .partition(|w| w.path.contains("/Window-ItemInfo/"));
        Ok(Self {
            tooltip,
            catalog,
            base,
            layout: assets.shop_layout,
            button: assets.scene(op, host, client, "TrainingCategory")?,
            item: assets.scene(op, host, client, "ShopItem")?,
            ..Default::default()
        })
    }

    fn widgets(
        &mut self,
        s: &TjsState,
        assets: &mut Assets,
        op: &Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        files: &GameFiles,
    ) -> Result<Vec<Widget>> {
        let cat = &self.catalog.categories[self.selected];
        let mut out: Vec<_> = self
            .base
            .iter()
            .filter(|w| !w.path.ends_with("/Arrow"))
            .cloned()
            .collect();
        if !self.catalog.fitting_tab(&cat.1) {
            out.retain(|w| !w.path.contains("/TrialWear") && !w.path.ends_with("/ViweReset"));
        }
        self.hits.clear();
        self.icons.clear();
        self.hover = None;
        for w in &mut out {
            if w.path.ends_with("/WorkingFunds/Value")
                && let Some(l) = &mut w.label
            {
                l.set_text(&daily::money(s.player.money));
            }
            if let Some(rect) = w.collider {
                if w.path.ends_with("/Parent/OK") {
                    self.hits.push((rect, Action::Exit));
                }
                match w.path.rsplit('/').next() {
                    Some("TrialWear") if self.selected_item.is_some() => {
                        self.hits.push((rect, Action::TryOn))
                    }
                    Some("TrialWearReset") if self.wardrobe.is_some() => {
                        self.hits.push((rect, Action::Restore))
                    }
                    Some("ViweReset") => self.hits.push((rect, Action::Camera)),
                    _ => {}
                }
                for (i, list) in LISTS.iter().enumerate() {
                    if w.path.contains(list)
                        && (w.path.ends_with("/Up") || w.path.ends_with("/Down"))
                    {
                        self.hits
                            .push((rect, Action::Scroll(i, w.path.ends_with("/Down"))));
                    }
                }
            }
        }
        let cat = &self.catalog.categories[self.selected];
        let mut mains = Vec::new();
        let mut subs = Vec::new();
        for (i, (m, _)) in self.catalog.categories.iter().enumerate() {
            if !mains
                .iter()
                .any(|&j: &usize| self.catalog.categories[j].0 == *m)
            {
                mains.push(i);
            }
            if *m == cat.0 {
                subs.push(i);
            }
        }
        let items = self.catalog.in_category(cat);
        for (list, count) in [mains.len(), subs.len(), items.len()]
            .into_iter()
            .enumerate()
        {
            let g = self.layout.grids[list];
            let capacity = (g.clip[3] / g.step).floor() as usize;
            self.max_offset[list] = count.saturating_sub(capacity);
            self.offset[list] = self.offset[list].min(self.max_offset[list]);
            for (row, i) in (self.offset[list]..count).take(capacity).enumerate() {
                let dy = g.clip[1] + (row as f32 + 0.5) * g.step - 540.;
                let (label, action, selected) = match list {
                    0 => (
                        &self.catalog.categories[mains[i]].0,
                        Action::Main(mains[i]),
                        self.catalog.categories[mains[i]].0 == cat.0,
                    ),
                    1 => (
                        &self.catalog.categories[subs[i]].1,
                        Action::Sub(subs[i]),
                        subs[i] == self.selected,
                    ),
                    _ => (
                        &self.catalog.items[&items[i]].name,
                        Action::Buy(items[i]),
                        false,
                    ),
                };
                for template in if list == 2 { &self.item } else { &self.button } {
                    let mut w = template.clone();
                    let leaf = template.path.rsplit('/').next().unwrap_or("");
                    if list == 2 {
                        let id = items[i];
                        let owned = self.catalog.owns(s, id);
                        let enabled = self.catalog.eligible(s, id);
                        let price = self.catalog.price(s, id)?;
                        if (w.path.contains("/BuyButton") && (owned || !enabled))
                            || (w.path.contains("/Purchased") && !owned)
                            || (w.path.contains("/Impossible") && (owned || enabled))
                            || (leaf == "Frame" && self.selected_item != Some(id))
                        {
                            continue;
                        }
                        if leaf == "ItemIcon" {
                            let icon = &self.catalog.items[&id].icon;
                            let icon = if files.exists(&format!("{}.tex", icon.to_lowercase())) {
                                icon
                            } else {
                                "icon_none"
                            };
                            w.asset = assets.schedule_icon(op, host, client, files, icon)?;
                        }
                        if leaf == "Price"
                            && let Some(l) = &mut w.label
                        {
                            l.set_text(&daily::money(price));
                            if !owned && (!enabled || price > s.player.money) {
                                w.tint = [255, 0, 0, 255];
                            }
                        }
                        if leaf == "ItemName"
                            && let Some(l) = &mut w.label
                        {
                            l.set_text(label);
                        }
                    } else if let Some(l) = &mut w.label {
                        l.set_text(label);
                    }
                    business::move_by(&mut w, g.x, dy);
                    if leaf == "ItemIcon"
                        && let Some(rect) = w.collider
                    {
                        self.icons
                            .push((rect, items[i], 540. + dy, w.asset.clone()));
                        if self.catalog.parts(items[i]) {
                            self.hits.push((rect, Action::Icon(items[i])));
                        }
                    }
                    if let Some(rect) = w.collider {
                        let active = match action {
                            Action::Buy(id) => {
                                leaf == "BuyButton" && self.catalog.price(s, id)? <= s.player.money
                            }
                            _ => leaf == "Button",
                        };
                        if active {
                            self.hits.push((rect, action));
                        }
                    }
                    if selected
                        && leaf == "Button"
                        && let Some(rect) = w.collider
                        && let Some(template) = self
                            .base
                            .iter()
                            .find(|w| w.path.contains(LISTS[list]) && w.path.ends_with("/Arrow"))
                    {
                        let mut arrow = template.clone();
                        let shift =
                            rect[1] + rect[3] / 2. - 18. - (arrow.rect[1] + arrow.rect[3] / 2.);
                        business::move_by(&mut arrow, 0., shift);
                        arrow.depth += 100;
                        out.push(arrow);
                    }
                    w.depth += 100;
                    w.path = format!("Shop/{list}/{i}/{}", w.path);
                    out.push(w);
                }
            }
        }
        out.sort_by_key(|w| w.depth);
        self.tooltip_first = out.len();
        Ok(out)
    }

    fn show_info(&mut self, widgets: &mut Vec<Widget>, input: &InputFrame) {
        let hovered = self.icons.iter().find(|(r, ..)| inside(*r, input));
        if self.hover == hovered.map(|v| v.1) {
            return;
        }
        widgets.truncate(self.tooltip_first);
        self.hover = hovered.map(|v| v.1);
        let Some((_, id, y, icon)) = hovered else {
            return;
        };
        let item = &self.catalog.items[id];
        let [_, origin_y] = self.layout.info_origin;
        let [offset_x, offset_y] = self.layout.info_offset;
        let local_y = origin_y + 600. - y;
        let flip = local_y + offset_y - 90. < -412.;
        for mut w in self.tooltip.iter().cloned() {
            if let Some(label) = &mut w.label {
                if w.path.ends_with("/Title") {
                    label.set_text(&item.name);
                }
                if w.path.ends_with("/Info") {
                    label.set_text(&item.description);
                }
            }
            if w.path.ends_with("/Icon") {
                w.asset = icon.clone();
            }
            business::move_by(
                &mut w,
                7. - 337. + offset_x,
                y - origin_y - if flip { -offset_y } else { offset_y },
            );
            w.depth += 1000;
            widgets.push(w);
        }
    }

    fn action(&mut self, input: &InputFrame, released: u64) -> Option<Action> {
        for (list, g) in self.layout.grids.iter().enumerate() {
            if inside(g.clip, input)
                && input.buttons_pressed & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) != 0
            {
                return Some(Action::Scroll(
                    list,
                    input.buttons_pressed & buttons::WHEEL_DOWN != 0,
                ));
            }
        }
        let hit = self
            .hits
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, input))
            .map(|(_, a)| *a);
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            self.pressed = hit;
        }
        if released & buttons::MOUSE_LEFT != 0 && self.pressed.take() == hit {
            hit
        } else {
            None
        }
    }

    fn select(&mut self, a: Action) {
        match a {
            Action::Icon(id) => {
                self.selected_item = if self.selected_item == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            Action::Main(i) => {
                self.selected_item = None;
                let main = &self.catalog.categories[i].0;
                self.selected = self.last_sub.get(main).copied().unwrap_or(i);
                self.offset[1..].fill(0);
            }
            Action::Sub(i) => {
                self.selected_item = None;
                self.selected = i;
                self.last_sub
                    .insert(self.catalog.categories[i].0.clone(), i);
                self.offset[2] = 0;
            }
            Action::Scroll(i, down) => {
                self.offset[i] = if down {
                    (self.offset[i] + 1).min(self.max_offset[i])
                } else {
                    self.offset[i].saturating_sub(1)
                };
            }
            _ => {}
        }
    }
}

impl Game {
    pub(super) fn open_shop(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        now: u64,
    ) -> Result<(String, String)> {
        let bg = self
            .script
            .tag_backup
            .get("bg_id")
            .map(|v| v.parse::<i32>())
            .transpose()?
            .unwrap_or(0);
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        let table = s.retail.table("game_in_shop_bg_setting.nei")?;
        let row = (1..table.rows)
            .find(|&r| table.exists(0, r) && table.int(0, r) == bg)
            .context("unknown shop background")?;
        ensure!(
            table.string(2, row).is_empty(),
            "shop stage environment not implemented for bg_id {bg}"
        );
        self.shop = Screen::load(
            s,
            bg,
            std::mem::take(&mut self.widgets),
            &mut self.assets,
            op,
            host,
            client,
        )?;
        self.widgets = self
            .shop
            .widgets(s, &mut self.assets, op, host, client, &self.files)?;
        let mut bgm = table.string(3, row);
        if !bgm.contains('.') {
            bgm.push_str(".ogg");
        }
        let lead = s
            .active_maids
            .first()
            .copied()
            .flatten()
            .filter(|id| s.maids.get(id).is_some_and(|m| m.leader))
            .or_else(|| {
                s.stock
                    .iter()
                    .copied()
                    .find(|id| s.maids.get(id).is_some_and(|m| m.leader))
            })
            .or_else(|| s.stock.first().copied())
            .context("shop needs a stock character")?;
        self.actors.reset_for_day(s);
        *s.active_maids
            .first_mut()
            .context("missing character slot zero")? = Some(lead);
        drop(vm);
        self.actors
            .activate_stock(&self.script, lead, 0, Some(true))?;
        use crate::kag::{AdvUi, EyeLook};
        let mut effects = self.actors.start_motion(
            &self.script,
            0,
            "h_kaiwa_tati_001.ks",
            "*会話立ち待機",
            Some(0),
            None,
            true,
            &self.files,
            op,
            now,
        )?;
        effects.extend([
            AdvUi::Face {
                man: false,
                slot: 0,
                name: "通常".into(),
            },
            AdvUi::FaceBlend {
                man: false,
                slot: 0,
                name: "無し".into(),
            },
            AdvUi::Eye {
                man: false,
                slot: 0,
                look: EyeLook::Camera("目と顔を向ける".into()),
                blend_s: 0.,
            },
            AdvUi::CameraChase {
                man: false,
                slot: 0,
                bone: "Bip01 Head".into(),
                offset: [0.01597595, -0.22453693, 0.043105565],
                once: true,
                seconds: 0.,
            },
        ]);
        self.edit_motion_effects(effects, op, now)?;
        self.adv_camera.1 = 1.6;
        self.adv_camera.2 = [173.7126, 9.999997];
        Ok((table.string(1, row), bgm))
    }

    pub(super) fn update_shop(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(dialog) = &mut self.shop.dialog {
            let Some(choice) = dialog.update(&self.widgets, input, released, now) else {
                return Ok(());
            };
            self.widgets.truncate(dialog.first);
            self.shop.dialog = None;
            let Some(id) = self.shop.pending.take() else {
                return Ok(());
            };
            if choice != quit_dialog::Choice::Ok {
                return Ok(());
            }
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let s = &mut vm.runtime.host_mut().state;
            self.shop.catalog.buy(s, id)?;
            self.sound.play(
                host,
                client,
                &self.files,
                Kind::System,
                "SE025.ogg",
                0.,
                false,
            )?;
            self.widgets = self
                .shop
                .widgets(s, &mut self.assets, op, host, client, &self.files)?;
            let dialog = self.assets.scene(op, host, client, "SystemDialog")?;
            let message = format!("{}を購入しました", self.shop.catalog.items[&id].name);
            let d = quit_dialog::QuitDialog::show(&mut self.widgets, dialog, now, &message)?;
            self.widgets
                .retain(|w| !w.path.contains("SystemDialog/Base/Cancel"));
            self.shop.dialog = Some(d);
            return Ok(());
        }
        self.shop.show_info(&mut self.widgets, &input);
        let Some(a) = self.shop.action(&input, released) else {
            return Ok(());
        };
        if matches!(a, Action::TryOn | Action::Restore | Action::Exit) {
            let model = self
                .actors
                .actor(false, 0)
                .context("missing shop subject")?;
            if a == Action::TryOn {
                let id = self.shop.selected_item.context("no fitting item")?;
                let mut preview = model.clone();
                for file in self.shop.catalog.preview_menus(id)? {
                    let menu =
                        crate::menu::MenuFile::parse(&self.files.read(op.storage.as_ref(), file)?)?;
                    preview.set_prop_file(&menu.category, file, false)?;
                }
                preview.set_mask_mode(&self.shop.catalog.items[&id].mask_mode)?;
                preview.all_proc_prop(&self.files, op.storage.as_ref())?;
                self.shop
                    .wardrobe
                    .get_or_insert_with(|| Box::new(model.clone()));
                *model = preview;
            } else if let Some(original) = self.shop.wardrobe.take() {
                model.props = original.props;
                model.slots = original.slots;
                for slot in model.slots.values_mut() {
                    slot.dirty = true;
                }
                model.body_morphs = original.body_morphs;
                model.blend_sets = original.blend_sets;
                model.set_mask_mode("None")?;
            }
        }
        if a == Action::Camera
            && let Some((target, distance, angles)) = self.shop.initial_camera
        {
            self.adv_camera = (target, distance, angles);
            let mut camera =
                crate::bg::orbit_camera_offset(target, angles, distance, 35., CAMERA_OFFSET);
            camera["op"] = json!("set_camera");
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"world":[camera]}))?)?;
        }
        if a == Action::Exit {
            self.actors.motions.clear();
            for effect in [
                crate::kag::AdvUi::Eye {
                    man: false,
                    slot: 0,
                    look: crate::kag::EyeLook::Camera("無し".into()),
                    blend_s: 0.,
                },
                crate::kag::AdvUi::CharaVisible {
                    man: false,
                    slot: 0,
                    visible: false,
                },
            ] {
                self.actors
                    .apply(effect, &self.script, &self.files, &mut self.assets, op, now)?;
            }
            return self.resume_script(op, host, client, now);
        }
        let vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &vm.runtime.host().state;
        if let Action::Buy(id) = a {
            let it = &self.shop.catalog.items[&id];
            let message = format!(
                "{}\nを購入しますか?\n資金 -{}CR",
                it.name,
                daily::money(self.shop.catalog.price(s, id)?)
            );
            let dialog = self.assets.scene(op, host, client, "SystemDialog")?;
            self.shop.dialog = Some(quit_dialog::QuitDialog::show(
                &mut self.widgets,
                dialog,
                now,
                &message,
            )?);
            self.shop.pending = Some(id);
        } else {
            self.shop.select(a);
            self.widgets = self
                .shop
                .widgets(s, &mut self.assets, op, host, client, &self.files)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
