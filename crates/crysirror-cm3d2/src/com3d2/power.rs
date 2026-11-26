// SPDX-License-Identifier: MPL-2.0
//! Standalone FacilityPowerUpManager and its serialized Unity UI controls.
use super::*;
use crate::tjs::{TjsState, retail::operations::upgrade};
use widgets::power::{CONDITION, Layout, MATERIAL, RECIPE, ROOM, Rect, VIEW};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Room(usize),
    Recipe(i32),
    Dropdown(usize),
    Material(usize, i32),
    Apply,
    Exit,
    Scroll(bool),
}

#[derive(Default)]
pub(super) struct Screen {
    layout: Layout,
    thumbnails: Vec<Widget>,
    room: Option<usize>,
    choice: Option<upgrade::Choice>,
    dropdown: Option<usize>,
    scroll: usize,
    bindings: Vec<(Rect, Action)>,
    pressed: Option<Action>,
    pub dialog: Option<quit_dialog::QuitDialog>,
    pending: Option<Action>,
    return_key: Option<&'static str>,
}

fn inside([x, y, w, h]: Rect, input: &InputFrame) -> bool {
    let (px, py) = (input.pointer_x as f32, input.pointer_y as f32);
    px >= x && px < x + w && py >= y && py < y + h
}

fn label(w: &mut Widget, value: &str) {
    if let Some(l) = w.label.as_mut() {
        l.set_text(value);
    }
}

impl Screen {
    fn load(
        s: &mut TjsState,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Self> {
        let layout = assets.upgrade_canvas(op, host, client)?;
        let template = layout
            .base
            .iter()
            .find(|w| w.path.ends_with("/Image Facility Thumbnail"))
            .context("missing facility preview thumbnail")?;
        let mut thumbnails = Vec::new();
        for i in 0..upgrade::rooms(s)?.len() {
            let mut w = template.clone();
            assets.facility_thumbnail(op, host, client, &mut w, &upgrade::info(s, i)?.thumbnail)?;
            thumbnails.push(w);
        }
        Ok(Self {
            layout,
            thumbnails,
            ..Default::default()
        })
    }

    fn room_info(&self, s: &mut TjsState, room: usize, out: &mut Vec<Widget>) -> Result<()> {
        let info = upgrade::info(s, room)?;
        for mut w in self
            .layout
            .base
            .iter()
            .filter(|w| w.path.starts_with("/Image Facility Mini Info"))
            .cloned()
        {
            if w.path.ends_with("/Image Facility Thumbnail") {
                w = self.thumbnails[room].clone();
            }
            if w.path.ends_with("/InputField Facility Name/Text") {
                label(&mut w, &info.name);
            }
            if w.path.ends_with("/Text Number of people (1)") {
                label(&mut w, &info.staff.to_string());
            }
            if w.path.ends_with("/Text Level of facility (1)") {
                label(&mut w, &info.level.to_string());
            }
            if w.path.ends_with("/Text Status (1)") {
                label(&mut w, info.status);
            }
            if w.path.ends_with("/RawImage") {
                let count = if w.path.contains("parent stars Rank/") {
                    info.rank
                } else {
                    info.staff_rank
                };
                for i in 0..count {
                    let mut star = w.clone();
                    star.rect = self.layout.cell(&w.path, i, count)?;
                    out.push(star);
                }
            } else {
                out.push(w);
            }
        }
        Ok(())
    }
    fn list_item(
        &mut self,
        out: &mut Vec<Widget>,
        path: &str,
        index: usize,
        count: usize,
        title: &str,
        selected: bool,
        action: Option<Action>,
    ) -> Result<()> {
        let rect = self.layout.cell(path, index, count)?;
        for mut w in self.layout.instantiate(path, rect)? {
            if w.path.ends_with("/Checkmark") && !selected {
                continue;
            }
            label(&mut w, title);
            out.push(w);
        }
        if let Some(a) = action {
            self.bindings.push((rect, a));
        }
        Ok(())
    }

    fn effects(
        &self,
        s: &mut TjsState,
        recipe: &upgrade::Recipe,
        choice: &upgrade::Choice,
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let old = upgrade::previous(s, choice.room, choice.recipe).to_vec();
        let old = upgrade::additional(&mut s.retail, &old)?;
        let new = upgrade::additional(&mut s.retail, &choice.materials)?;
        let names = s.retail.table("facility_parameters.nei")?;
        for side in ["Previous", "Next"] {
            for group in ["Default", "Additional"] {
                let path = format!(
                    "/Window Material List/Window {side} Params/Parent {group} Effect/Parent Text/temp Text"
                );
                let values = if group == "Default" {
                    recipe.base
                } else if side == "Previous" {
                    old
                } else {
                    new
                };
                let shown: Vec<_> = (0..3)
                    .filter(|&i| {
                        values[i] > 0 || (side == "Next" && group == "Additional" && old[i] > 0)
                    })
                    .collect();
                for (row, &i) in shown.iter().enumerate() {
                    for mut w in self
                        .layout
                        .instantiate(&path, self.layout.cell(&path, row, shown.len())?)?
                    {
                        if w.path.ends_with("/Image")
                            && (if group == "Default" { i } else { row }) % 2 != 0
                        {
                            continue;
                        }
                        if w.label.is_some() {
                            label(
                                &mut w,
                                &format!("{}  +{}", names.string(1, i + 1), values[i]),
                            );
                            if group == "Additional" && side == "Next" {
                                w.tint = match new[i].cmp(&old[i]) {
                                    std::cmp::Ordering::Greater => [0, 255, 255, 255],
                                    std::cmp::Ordering::Less => [255, 0, 255, 255],
                                    std::cmp::Ordering::Equal => [255; 4],
                                };
                            }
                        }
                        out.push(w);
                    }
                }
            }
        }
        Ok(())
    }

    fn material_widgets(
        &mut self,
        s: &mut TjsState,
        recipe: &upgrade::Recipe,
        choice: &upgrade::Choice,
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let categories = s.retail.table("facility_material_category.nei")?;
        let mut overlay = Vec::new();
        for (i, &cat) in recipe.categories.iter().enumerate() {
            let opts = upgrade::options(s, cat)?;
            let rect = self.layout.cell(MATERIAL, i, recipe.categories.len())?;
            let current = opts
                .iter()
                .find(|(id, _)| Some(id) == choice.materials.get(i));
            let category = (1..categories.rows)
                .find(|&r| categories.int(0, r) == cat)
                .context("missing material category")?;
            let mut items = self.layout.instantiate(MATERIAL, rect)?;
            let item_label = items
                .iter()
                .find(|w| w.path.ends_with("/Item Label"))
                .context("missing material option label")?
                .clone();
            let template = items
                .iter()
                .find(|w| w.path.ends_with("/Template"))
                .context("missing material dropdown template")?
                .clone();
            items.retain(|w| {
                !w.path.contains("/Template") && (!w.path.ends_with("/Image Plus Mark") || i > 0)
            });
            for mut w in items {
                if w.path.ends_with("/Label") {
                    label(&mut w, current.map_or("無し", |(_, name)| name));
                }
                if w.path.ends_with("/text category") {
                    label(&mut w, &format!("CATEGORY{}", i + 1));
                }
                if w.path.ends_with("/Text Title") {
                    label(&mut w, &categories.string(1, category));
                }
                out.push(w);
            }
            if !opts.is_empty() && self.dropdown.is_none() {
                self.bindings.push((rect, Action::Dropdown(i)));
            }
            if self.dropdown == Some(i) {
                let mut bg = template;
                let item = self
                    .layout
                    .rect(&format!("{MATERIAL}/Template/Viewport/Content/Item"))?;
                bg.rect[3] = item[3] * opts.len() as f32;
                // Unity Dropdown flips the list when the anchored bottom would leave the canvas.
                if bg.rect[1] + bg.rect[3] > 1080. {
                    bg.rect[1] = rect[1] - bg.rect[3];
                }
                let bounds = bg.rect;
                overlay.push(bg);
                for (j, (id, name)) in opts.iter().enumerate() {
                    let r = [
                        bounds[0],
                        bounds[1] + item[3] * j as f32,
                        bounds[2],
                        item[3],
                    ];
                    let mut w = item_label.clone();
                    w.rect = r;
                    label(&mut w, name);
                    overlay.push(w);
                    self.bindings.push((r, Action::Material(i, *id)));
                }
            }
        }
        self.effects(s, recipe, choice, out)?;
        out.extend(overlay);
        Ok(())
    }

    fn populate(&mut self, s: &mut TjsState) -> Result<Vec<Widget>> {
        self.bindings.clear();
        let enabled = self
            .choice
            .as_ref()
            .map(|c| c.enabled(s))
            .transpose()?
            .unwrap_or(false);
        let mut out = Vec::new();
        for mut w in self.layout.base.clone() {
            let path = w.path.as_str();
            if path.contains("/temp ")
                || path.starts_with("/Image Facility Mini Info")
                || path == VIEW
            {
                continue;
            }
            match path {
                "/Button OK" if enabled => self.bindings.push((w.rect, Action::Apply)),
                "/Button OK" => w.tint = w.tint.map(|v| ((v as f32) * 0.5).round() as u8),
                "/Button EXIT" => self.bindings.push((w.rect, Action::Exit)),
                _ => {}
            }
            out.push(w);
        }
        let names = upgrade::rooms(s)?;
        for (i, name) in names.iter().enumerate() {
            self.list_item(
                &mut out,
                ROOM,
                i,
                names.len(),
                name,
                self.room == Some(i),
                Some(Action::Room(i)),
            )?;
        }
        if let Some(room) = self.room {
            self.room_info(s, room, &mut out)?;
            let recipes = upgrade::recipes(s, room)?;
            let view = self.layout.rect(VIEW)?;
            let first = self.layout.cell(RECIPE, 0, 1)?;
            let step = self.layout.cell(RECIPE, 1, 2)?[1] - first[1];
            let visible = ((view[3] + step - first[3]) / step).floor().max(1.) as usize;
            self.scroll = self.scroll.min(recipes.len().saturating_sub(visible));
            if recipes.is_empty() {
                self.list_item(
                    &mut out,
                    RECIPE,
                    0,
                    1,
                    "レシピは存在しません。",
                    false,
                    None,
                )?;
            }
            for (i, recipe) in recipes.iter().skip(self.scroll).take(visible).enumerate() {
                self.list_item(
                    &mut out,
                    RECIPE,
                    i,
                    visible,
                    &recipe.name,
                    self.choice.as_ref().is_some_and(|c| c.recipe == recipe.id),
                    Some(Action::Recipe(recipe.id)),
                )?;
            }
            let moves = self
                .layout
                .base
                .iter()
                .filter(|w| w.path.ends_with("/Button Slider Movement"))
                .collect::<Vec<_>>();
            for w in moves {
                self.bindings
                    .push((w.rect, Action::Scroll(w.rect[1] > view[1] + view[3] / 2.)));
            }
            if let Some(choice) = self.choice.clone() {
                let recipe = recipes
                    .iter()
                    .find(|r| r.id == choice.recipe)
                    .context("missing selected recipe")?;
                for (i, condition) in recipe.conditions.iter().enumerate() {
                    for mut w in self.layout.instantiate(
                        CONDITION,
                        self.layout.cell(CONDITION, i, recipe.conditions.len())?,
                    )? {
                        if w.path.ends_with("/image background") && i % 2 == 0 {
                            continue;
                        }
                        if w.path.ends_with("/text number") {
                            label(&mut w, &(i + 1).to_string());
                        } else {
                            label(&mut w, condition);
                        }
                        out.push(w);
                    }
                }
                self.material_widgets(s, recipe, &choice, &mut out)?;
            }
        }
        for (i, w) in out.iter_mut().enumerate() {
            w.depth = i as i32;
        }
        Ok(out)
    }

    fn action(&mut self, input: &InputFrame, released: u64) -> Option<Action> {
        let hit = self
            .bindings
            .iter()
            .rev()
            .find(|(rect, a)| {
                inside(*rect, input)
                    && (self.dropdown.is_none() || matches!(a, Action::Material(..)))
            })
            .map(|(_, a)| *a);
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            self.pressed = hit;
        }
        if released & buttons::MOUSE_LEFT != 0 && self.pressed.take() == hit {
            return hit;
        }
        None
    }

    fn select(&mut self, s: &mut TjsState, action: Action) -> Result<()> {
        match action {
            Action::Room(i) => {
                self.room = Some(i);
                self.choice = None;
                self.dropdown = None;
                self.scroll = 0;
            }
            Action::Recipe(id) => {
                let room = self.room.context("choose an upgrade room first")?;
                let recipe = upgrade::recipes(s, room)?
                    .into_iter()
                    .find(|r| r.id == id)
                    .context("unknown facility recipe")?;
                self.choice = Some(upgrade::choose(s, room, &recipe)?);
                self.dropdown = None;
            }
            Action::Dropdown(i) => self.dropdown = Some(i),
            Action::Material(i, id) => {
                *self
                    .choice
                    .as_mut()
                    .and_then(|c| c.materials.get_mut(i))
                    .context("missing material slot")? = id;
                self.dropdown = None;
            }
            Action::Scroll(down) => {
                self.scroll = if down {
                    self.scroll.saturating_add(1)
                } else {
                    self.scroll.saturating_sub(1)
                }
            }
            _ => bail!("facility confirmation requires a dialog"),
        }
        Ok(())
    }
}

impl Game {
    pub(super) fn open_power(
        &mut self,
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
        self.main_light = Default::default();
        self.camera_control = false;
        self.adv_camera = ([0., 1.5, 0.], 2., [180., 10.]);
        self.camera_move = None;
        self.adv_cover = None;
        self.world_cover = None;
        self.message_open = false;
        self.dialogue = None;
        self.business = Default::default();
        self.power = Screen::load(s, &mut self.assets, op, host, client)?;
        self.widgets = self.power.populate(s)?;
        self.daily = Default::default();
        self.daily.tutorial_first = self.widgets.len();
        let flag = "__チュートリアルフラグ_SceneFacilityPowerUp";
        if s.player.flag(flag) <= 0 {
            let db = s.retail.table("tutorial_list.nei")?;
            let r = (1..db.rows)
                .find(|&r| db.string(0, r) == "SceneFacilityPowerUp")
                .context("missing facility upgrade tutorial")?;
            self.daily.pages = (2..db.columns)
                .step_by(2)
                .filter(|&col| db.exists(col, r))
                .map(|col| (db.string(col, r), db.string(col + 1, r)))
                .filter(|(file, _)| !file.is_empty())
                .collect();
            self.assets.tutorial(
                op,
                host,
                client,
                &self.daily.pages,
                0,
                &self.files,
                &mut self.widgets,
            )?;
            s.player.generic_flag.insert(flag.into(), 1);
        }
        Ok(())
    }

    pub(super) fn update_power(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(t) = self.leaving {
            if now.saturating_sub(t) >= 500 {
                let key = self
                    .power
                    .return_key
                    .take()
                    .context("missing facility return destination")?;
                self.resume_at(key, op, host, client, now)?;
            }
            return Ok(());
        }
        if !self.daily.pages.is_empty() {
            if let Some(t) = self.daily.closing_help {
                if now.saturating_sub(t) >= 500 {
                    self.widgets.truncate(self.daily.tutorial_first);
                    self.daily.pages.clear();
                    self.daily.closing_help = None;
                }
                return Ok(());
            }
            let hit = self.daily.hit(&self.widgets, &input);
            if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                self.pressed = hit;
            }
            if released & buttons::MOUSE_LEFT != 0
                && self.pressed.take() == hit
                && let Some(i) = hit
            {
                let path = &self.widgets[i].path;
                if path.ends_with("/Button OK") {
                    self.daily.closing_help = Some(now);
                } else if let Some((_, tail)) = path.split_once("/page-") {
                    let page = tail
                        .split('/')
                        .next()
                        .context("tutorial page")?
                        .parse::<usize>()?;
                    if page != self.daily.page {
                        self.widgets.truncate(self.daily.tutorial_first);
                        self.assets.tutorial(
                            op,
                            host,
                            client,
                            &self.daily.pages,
                            page,
                            &self.files,
                            &mut self.widgets,
                        )?;
                        self.daily.page = page;
                    }
                }
            }
            return Ok(());
        }
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        self.power.input(
            s,
            &mut self.widgets,
            &mut self.assets,
            op,
            host,
            client,
            input,
            released,
            now,
        )?;
        if self.power.return_key.is_some() {
            self.leaving = Some(now);
        }
        Ok(())
    }
}

impl Screen {
    fn input(
        &mut self,
        s: &mut TjsState,
        widgets: &mut Vec<Widget>,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(dialog) = self.dialog.as_mut() {
            if let Some(choice) = dialog.update(widgets, input, released, now) {
                widgets.truncate(dialog.first);
                self.dialog = None;
                let action = self
                    .pending
                    .take()
                    .context("missing pending facility action")?;
                if choice == quit_dialog::Choice::Ok {
                    self.return_key = Some(if action == Action::Apply {
                        self.choice.as_ref().context("missing upgrade")?.apply(s)?;
                        "label_power_up"
                    } else {
                        "label"
                    });
                }
            }
            return Ok(());
        }
        if self.dropdown.is_none()
            && inside(self.layout.rect(VIEW)?, &input)
            && input.buttons_pressed & (buttons::WHEEL_DOWN | buttons::WHEEL_UP) != 0
        {
            self.select(
                s,
                Action::Scroll(input.buttons_pressed & buttons::WHEEL_DOWN != 0),
            )?;
            *widgets = self.populate(s)?;
            return Ok(());
        }
        let Some(action) = self.action(&input, released) else {
            if self.dropdown.is_some() && released & buttons::MOUSE_LEFT != 0 {
                self.dropdown = None;
                *widgets = self.populate(s)?;
            }
            return Ok(());
        };
        if matches!(action, Action::Apply | Action::Exit) {
            let msg = if action == Action::Apply {
                "施設の強化を実行しますか？"
            } else {
                "施設の強化をせずに終了します。\nよろしいですか？"
            };
            let buttons = assets.scene(op, host, client, "YesNoDialog")?;
            self.dialog = Some(quit_dialog::QuitDialog::show(widgets, buttons, now, msg)?);
            self.pending = Some(action);
        } else {
            self.select(s, action)?;
            *widgets = self.populate(s)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tjs::retail::{
        daily_status,
        operations::{self, Shift},
    };
    fn click(screen: &mut Screen, a: Action) -> Result<()> {
        let &(r, _) = screen
            .bindings
            .iter()
            .find(|(_, x)| *x == a)
            .with_context(|| format!("missing {a:?}"))?;
        let mut input = InputFrame {
            pointer_x: (r[0] + r[2] / 2.) as i32,
            pointer_y: (r[1] + r[3] / 2.) as i32,
            buttons_pressed: buttons::MOUSE_LEFT,
            ..Default::default()
        };
        ensure!(
            (0..1920).contains(&input.pointer_x) && (0..1080).contains(&input.pointer_y),
            "offscreen {a:?} {r:?}"
        );
        assert_eq!(screen.action(&input, 0), None);
        input.buttons_pressed = 0;
        assert_eq!(screen.action(&input, buttons::MOUSE_LEFT), Some(a));
        Ok(())
    }

    #[test]
    #[ignore = "requires COM3D2_ROOT standalone installation"]
    fn installed_manual_facility_upgrade() -> Result<()> {
        let (_tmp, mut rt, c, mut op, _, mut s, mut assets) = daily::installed_panel(&[])?;
        operations::start_day(&mut s)?;
        s.stock.push(0);
        let m = s.maids.entry(0).or_default();
        m.guid_id = "upgrade-worker".into();
        m.personal = 100;
        operations::open_schedule(&mut s)?;
        upgrade::assign(&mut s, 0, Shift::Day)?;
        s.active_maids = vec![Some(0)];
        let mut ui = Screen::load(&mut s, &mut assets, &mut op, &mut rt.host, c)?;
        let room = upgrade::rooms(&mut s)?
            .iter()
            .position(|name| name == "トレーニングルーム")
            .unwrap();
        let widgets = ui.populate(&mut s)?;
        assert!(widgets.iter().any(|w| {
            w.label
                .as_ref()
                .is_some_and(|l| l.text == "トレーニングルーム")
        }));
        for a in [Action::Room(room), Action::Recipe(10)] {
            click(&mut ui, a)?;
            ui.select(&mut s, a)?;
            ui.populate(&mut s)?;
        }
        let widgets = ui.populate(&mut s)?;
        assert!(
            widgets
                .iter()
                .any(|w| w.label.as_ref().is_some_and(|l| l.text == "無し"))
        );
        assert!(!ui.bindings.iter().any(|(_, a)| *a == Action::Apply));
        assert!(ui.choice.as_ref().unwrap().apply(&mut s).is_err());
        assert_eq!(s.retail.upgrade_count, 0);
        // An independent inventory fixture represents two already purchased materials.
        s.retail.materials.extend([10, 20]);
        ui.select(&mut s, Action::Recipe(11))?;
        let widgets = ui.populate(&mut s)?;
        assert!(widgets.iter().any(|w| {
            w.label
                .as_ref()
                .is_some_and(|l| l.text == "ボーカルが500以上")
        }));
        assert!(!ui.bindings.iter().any(|(_, a)| *a == Action::Apply));
        for a in [
            Action::Recipe(10),
            Action::Dropdown(0),
            Action::Material(0, 20),
        ] {
            click(&mut ui, a)?;
            ui.select(&mut s, a)?;
            ui.populate(&mut s)?;
        }
        let mut widgets = ui.populate(&mut s)?;
        assert!(widgets.iter().any(|w| {
            w.tint == [0, 255, 255, 255]
                && w.label
                    .as_ref()
                    .is_some_and(|l| l.text == "施設EXPアップ  +2")
        }));
        click(&mut ui, Action::Apply)?;
        let yesno = assets.scene(&mut op, &mut rt.host, c, "YesNoDialog")?;
        let dialog =
            quit_dialog::QuitDialog::show(&mut widgets, yesno, 0, "施設の強化を実行しますか？")?;
        assert!(widgets[dialog.first..].iter().any(
            |w| w.path.ends_with("/Message") && w.label.as_ref().unwrap().text.contains("実行")
        ));
        ui.choice.as_ref().unwrap().apply(&mut s)?;
        assert_eq!(upgrade::previous(&s, room, 10), &[20]);
        assert!(s.retail.materials.contains(&20));
        assert_eq!(s.retail.upgrade_count, 1);
        assert_eq!(s.retail.upgrades["upgrade-worker"].room, room);
        operations::settle(&mut s, Shift::Day)?;
        assert_eq!(s.maids[&0].work_data.get(&3000).unwrap().play_count, 1);
        assert_eq!(s.player.schedule_slot[0].noon_success_level, 1);
        let recipe = upgrade::recipes(&mut s, room)?
            .into_iter()
            .find(|r| r.id == 10)
            .unwrap();
        let choice = upgrade::choose(&mut s, room, &recipe)?;
        assert_eq!(choice.materials, [20]);
        daily_status(&mut s, true)?;
        assert!(s.retail.upgrades.is_empty());
        assert_eq!(upgrade::previous(&s, room, 10), &[20]);
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn fixture_confirm_upgrade(
    s: &mut TjsState,
    assets: &mut Assets,
    op: &mut Opener,
    host: &mut dyn HostServices,
    client: ClientId,
) -> Result<&'static str> {
    let room = upgrade::rooms(s)?
        .iter()
        .position(|name| name == "トレーニングルーム")
        .context("missing installed training room")?;
    let mut screen = Screen::load(s, assets, op, host, client)?;
    let mut widgets = screen.populate(s)?;
    let mut clock = 0;
    for a in [Action::Room(room), Action::Recipe(10), Action::Apply] {
        let (r, _) = screen
            .bindings
            .iter()
            .find(|(_, v)| *v == a)
            .context("missing facility control")?;
        let mut input = InputFrame {
            pointer_x: (r[0] + r[2] / 2.) as i32,
            pointer_y: (r[1] + r[3] / 2.) as i32,
            ..Default::default()
        };
        input.buttons_pressed = buttons::MOUSE_LEFT;
        screen.input(s, &mut widgets, assets, op, host, client, input, 0, clock)?;
        input.buttons_pressed = 0;
        clock += 20;
        screen.input(
            s,
            &mut widgets,
            assets,
            op,
            host,
            client,
            input,
            buttons::MOUSE_LEFT,
            clock,
        )?;
    }
    let first = screen
        .dialog
        .as_ref()
        .context("missing upgrade confirmation")?
        .first;
    let r = widgets[first..]
        .iter()
        .find(|w| w.path.ends_with("/Base/Ok"))
        .and_then(|w| w.collider)
        .context("missing yes button")?;
    let mut input = InputFrame {
        pointer_x: (r[0] + r[2] / 2.) as i32,
        pointer_y: (r[1] + r[3] / 2.) as i32,
        buttons_pressed: buttons::MOUSE_LEFT,
        ..Default::default()
    };
    screen.input(
        s,
        &mut widgets,
        assets,
        op,
        host,
        client,
        input,
        0,
        clock + 400,
    )?;
    input.buttons_pressed = 0;
    screen.input(
        s,
        &mut widgets,
        assets,
        op,
        host,
        client,
        input,
        buttons::MOUSE_LEFT,
        clock + 420,
    )?;
    screen
        .return_key
        .context("facility confirmation did not release KAG")
}
