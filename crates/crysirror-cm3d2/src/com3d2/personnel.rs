// SPDX-License-Identifier: MPL-2.0
//! Level 7, MaidManagement and MaidManagementMain.
use super::*;
use crate::tjs::{TjsState, retail::personnel as staff};
use crysirror_unity::reader::Reader;

#[derive(Default, Clone, Copy)]
pub(super) struct Layout {
    grid: shop::Grid,
    price: i64,
    step: f32,
}
impl Layout {
    pub fn read(&mut self, class: &str, path: &str, pos: [f32; 3], data: &[u8]) -> Result<()> {
        if path.contains("/CharacterSelectPanel/") {
            self.grid.read(class, path, pos, data)?;
        }
        let mut r = Reader::new(data);
        if class == "MaidManagementMain" {
            r.seek(44)?;
            self.price = r.i32()? as i64;
        }
        if class == "UIGrid" && path.ends_with("/Tower/Grid") {
            r.seek(52)?;
            self.step = r.f32()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
enum Action {
    Select(usize),
    Edit,
    Lead,
    Employ,
    Dismiss,
    Cancel,
    Scroll(bool),
    Status,
    Class,
}
#[derive(Clone, Copy)]
enum Message {
    Employ,
    Dismiss,
    Notice,
}
pub(super) struct Screen {
    layout: Layout,
    base: Vec<Widget>,
    plate: Vec<Widget>,
    hits: Vec<([f32; 4], Action)>,
    selected: Option<usize>,
    offset: usize,
    pressed: Option<Action>,
    leave: &'static str,
    modal: Option<(Message, quit_dialog::QuitDialog)>,
}
const BUTTONS: [(&str, Action); 5] = [
    ("雇用", Action::Employ),
    ("解雇", Action::Dismiss),
    ("メイド長", Action::Lead),
    ("エディット", Action::Edit),
    ("クラス変更", Action::Class),
];
impl Screen {
    pub fn clip(&self, path: &str) -> Option<[f32; 4]> {
        path.starts_with("personnel-plate/")
            .then_some(self.layout.grid.clip)
    }
    fn draw(
        &mut self,
        out: &mut Vec<Widget>,
        s: &mut TjsState,
        assets: &mut Assets,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<()> {
        let day = s.player.flag("時間帯") != 3;
        let mut ids = s.stock.clone();
        s.sort_maid_standard(&mut ids);
        if self.selected.is_none_or(|id| !ids.contains(&id)) {
            self.selected = ids.first().copied();
        }
        let access = self.selected.map(|id| staff::access(s, id)).transpose()?;
        let buttons: Vec<_> = BUTTONS
            .iter()
            .filter(|(_, a)| day || matches!(a, Action::Edit | Action::Class))
            .collect();
        out.clear();
        self.hits.clear();
        for mut w in self.base.iter().cloned() {
            let act = if let Some((_, tail)) = w.path.split_once("/Tower/Grid/") {
                let name = tail.split('/').next().unwrap();
                let Some((index, (_, act))) = buttons
                    .iter()
                    .copied()
                    .enumerate()
                    .find(|(_, b)| b.0 == name)
                else {
                    continue;
                };
                // The serialized grid includes transfer and ranking; UIGrid packs only active children.
                let original = [
                    "雇用",
                    "移籍",
                    "解雇",
                    "メイド長",
                    "エディット",
                    "ランキング",
                    "クラス変更",
                ]
                .iter()
                .position(|&n| n == name)
                .unwrap();
                let root = name == tail;
                business::move_by(
                    &mut w,
                    0.,
                    (index as f32 - original as f32) * self.layout.step,
                );
                let enabled = match act {
                    Action::Employ => staff::count(s) < 200,
                    Action::Edit => access.as_ref().is_some_and(|a| a.edit),
                    Action::Dismiss => access.as_ref().is_some_and(|a| a.dismiss),
                    Action::Lead => access.as_ref().is_some_and(|a| a.lead),
                    Action::Class => self.selected.is_some(),
                    _ => false,
                };
                if !enabled {
                    w.tint = [128, 128, 128, 255];
                }
                (enabled && root).then_some(*act)
            } else if w.path.ends_with("/ButtonParent/Cancel") {
                Some(Action::Cancel)
            } else if w.path.ends_with("/Tower/ステータス") {
                Some(Action::Status)
            } else if w.path.ends_with("/Scroll Bar/Up") {
                Some(Action::Scroll(false))
            } else if w.path.ends_with("/Scroll Bar/Down") {
                Some(Action::Scroll(true))
            } else {
                None
            };
            if let (Some(a), Some(r)) = (act, w.collider) {
                self.hits.push((r, a));
            }
            out.push(w);
        }
        let g = self.layout.grid;
        let visible = (g.clip[3] / g.step).ceil() as usize;
        self.offset = self.offset.min(ids.len().saturating_sub(visible));
        for (i, &id) in ids.iter().skip(self.offset).take(visible).enumerate() {
            for mut w in self.plate.iter().cloned() {
                if !business::prepare_plate(
                    &mut w,
                    id,
                    self.selected == Some(id),
                    s,
                    assets,
                    host,
                    c,
                )? {
                    continue;
                }
                business::move_by(&mut w, g.x, g.clip[1] + (i as f32 + 0.5) * g.step - 540.);
                w.depth += 200;
                if w.path.ends_with("/Button")
                    && let Some(mut r) = w.collider
                {
                    r[3] = r[3].min(g.clip[1] + g.clip[3] - r[1]);
                    self.hits.push((r, Action::Select(id)));
                }
                w.path = format!("personnel-plate/{id}/{}", w.path);
                out.push(w);
            }
        }
        Ok(())
    }
}
impl Game {
    pub(super) fn open_personnel(
        &mut self,
        op: &mut Opener,
        h: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<daily::Period> {
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        let period = if s.player.flag("時間帯") == 3 {
            daily::Period::Night
        } else {
            daily::Period::Daytime
        };
        let layout = self.assets.personnel_layout;
        ensure!(
            layout.price > 0
                && layout.step > 0.
                && layout.grid.step > 0.
                && layout.grid.clip[3] > 0.,
            "incomplete level 7 layout"
        );
        let backup = if self.script.tag_backup.contains_key("loop") {
            self.personnel_backup
        } else {
            None
        };
        let mut screen = Screen {
            layout,
            plate: self.assets.scene(op, h, c, "PickerPlate")?,
            base: std::mem::take(&mut self.widgets),
            selected: backup.map(|v| v.0),
            offset: backup.map_or(0, |v| v.1),
            hits: vec![],
            pressed: None,
            leave: "label",
            modal: None,
        };
        screen.draw(&mut self.widgets, s, &mut self.assets, h, c)?;
        self.personnel = Some(screen);
        self.message_open = false;
        self.dialogue = None;
        self.adv_camera = ([0., 1.5, 0.], 2., [180., 10.]);
        self.camera_control = true;
        self.camera_move = None;
        self.actors.reset_positions();
        self.sound.stop(h, c, Kind::Voice, 0.)?;
        self.sound
            .play(h, c, &self.files, Kind::Bgm, period.bgm(), 0.5, true)?;
        Ok(period)
    }
    pub(super) fn update_personnel(
        &mut self,
        op: &mut Opener,
        h: &mut dyn HostServices,
        c: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        let screen = self
            .personnel
            .as_mut()
            .context("missing personnel screen")?;
        if let Some(start) = self.leaving {
            let key = screen.leave;
            if now.saturating_sub(start) >= 500 {
                return self.resume_at(key, op, h, c, now);
            }
            return Ok(());
        }
        if now.saturating_sub(self.entered) < 500 {
            return Ok(());
        }
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        if let Some((msg, dlg)) = &mut screen.modal {
            let Some(choice) = dlg.update(&self.widgets, input, released, now) else {
                return Ok(());
            };
            self.widgets.truncate(dlg.first);
            let msg = *msg;
            screen.modal = None;
            if choice == quit_dialog::Choice::Ok {
                match msg {
                    Message::Employ => {
                        let id = staff::employ(s, screen.layout.price)?;
                        self.personnel_backup = Some((id, usize::MAX));
                        screen.leave = "new_edit_label";
                        self.leaving = Some(now);
                    }
                    Message::Dismiss => {
                        staff::dismiss(s, screen.selected.context("no stock selection")?)?;
                        screen.selected = None;
                    }
                    Message::Notice => {}
                }
            }
        } else {
            let hit = screen
                .hits
                .iter()
                .rev()
                .find(|(r, _)| {
                    let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
                    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
                })
                .map(|(_, a)| *a);
            if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                screen.pressed = hit;
            }
            if released & buttons::MOUSE_LEFT == 0 {
                return Ok(());
            }
            let Some(act) = screen.pressed.take().filter(|a| Some(*a) == hit) else {
                return Ok(());
            };
            let id = screen.selected;
            let message = match act {
                Action::Select(id) => {
                    screen.selected = Some(id);
                    None
                }
                Action::Cancel => {
                    screen.leave = "label";
                    self.leaving = Some(now);
                    None
                }
                Action::Edit => {
                    let id = id.context("missing selected employee")?;
                    staff::edit(s, id)?;
                    screen.leave = "edit_label";
                    self.personnel_backup = Some((id, screen.offset));
                    self.leaving = Some(now);
                    None
                }
                Action::Lead => {
                    staff::lead(s, id.context("missing selected employee")?)?;
                    None
                }
                Action::Scroll(down) => {
                    screen.offset = if down {
                        screen.offset.saturating_add(1)
                    } else {
                        screen.offset.saturating_sub(1)
                    };
                    None
                }
                Action::Employ if s.player.money < screen.layout.price => Some((
                    Message::Notice,
                    format!(
                        "新しいメイドを雇用するには\n{}CRが必要です。",
                        screen.layout.price
                    ),
                )),
                Action::Employ => Some((
                    Message::Employ,
                    format!(
                        "新しいメイドを雇用しますか?\n資金 -{}CR",
                        screen.layout.price
                    ),
                )),
                Action::Dismiss if s.stock.len() >= 2 => {
                    let m = &s.maids[&id.context("missing selected employee")?];
                    Some((
                        Message::Dismiss,
                        format!(
                            "{} {}\nを解雇しますか?\n資金 +10,000CR",
                            m.last_name, m.first_name
                        ),
                    ))
                }
                Action::Dismiss => None,
                Action::Status | Action::Class => {
                    bail!("unsupported COM3D2 MaidManagementMain {act:?} panel")
                }
            };
            if let Some((msg, text)) = message {
                let widgets = self.assets.scene(op, h, c, "SystemDialog")?;
                let dlg = quit_dialog::QuitDialog::show(&mut self.widgets, widgets, now, &text)?;
                if matches!(msg, Message::Notice) {
                    self.widgets.retain(|w| {
                        !(w.path.contains("SystemDialog/") && w.path.ends_with("/Cancel"))
                    });
                }
                screen.modal = Some((msg, dlg));
                return Ok(());
            }
        }
        screen.draw(&mut self.widgets, s, &mut self.assets, h, c)
    }
}

#[test]
#[ignore = "requires 1.0.4 stand-alone level 7, KAG and stock models"]
fn installed_personnel_buttons_and_both_callers() -> Result<()> {
    let mut menu = installed_menu::Menu::new()?;
    for (night, label) in [(true, "*メイド管理へ"), (false, "*メイド管理")] {
        menu.open(night, label, Scene::Personnel)?;
        let hit = |menu: &installed_menu::Menu, a| {
            menu.game
                .personnel
                .as_ref()
                .unwrap()
                .hits
                .iter()
                .find(|(_, action)| *action == a)
                .map(|(r, _)| *r)
        };
        assert_eq!(hit(&menu, Action::Employ).is_some(), !night);
        assert!(hit(&menu, Action::Edit).is_some());
        if night {
            menu.click(hit(&menu, Action::Cancel).unwrap())?;
            menu.wait(Scene::Daily(daily::Period::Night))?;
        } else {
            let price = menu.game.personnel.as_ref().unwrap().layout.price;
            assert_eq!(price, 3_000_000);
            menu.game
                .script
                .tjs
                .lock()
                .unwrap()
                .runtime
                .host_mut()
                .state
                .player
                .money = price;
            menu.click(hit(&menu, Action::Employ).unwrap())?;
            let confirm = menu
                .game
                .widgets
                .iter()
                .find(|w| w.path.ends_with("/Ok") && w.path.contains("SystemDialog"))
                .and_then(|w| w.collider)
                .context("missing employment confirm")?;
            menu.click(confirm)?;
            let vm = menu.game.script.tjs.lock().unwrap();
            let s = &vm.runtime.host().state;
            assert_eq!(s.stock.len(), 2);
            assert_eq!(s.player.money, 0);
            assert_eq!(s.active_maids[0], s.stock.last().copied());
            drop(vm);
            menu.tick(500)?;
            assert_eq!(menu.game.scene, Scene::Script);
        }
    }
    Ok(())
}
