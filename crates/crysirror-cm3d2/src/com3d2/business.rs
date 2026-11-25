// SPDX-License-Identifier: MPL-2.0
//! SceneDaily's local schedule and its three KAG result callbacks (Unity 5.6).
use super::*;
use crate::tjs::{
    TjsState,
    retail::operations::{self, Shift},
};
use crysirror_unity::reader::Reader;

mod picker;
pub(super) use picker::PickMode;

const SLOTS: &str = "/SelectedMaidViewer/Contents/SelectedMaidUnitParent";
const ROW: &str = "/MaidStatusUnit/";
const CGRID: &str = "/MainCategoryViewer/Content/CategoryUnitParent";
const STAFFGRID: &str = "/CharacterSelectPanel/Contents/MaidSkillUnitParent";
const RGRID: &str = "/MaidStatusViewer/BG/Contents/SelectedMaidUnitParent";
const TGRID: &str = "/UnitViewer/Content/CategoryUnitParent";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResultPanel {
    Work(Shift),
    Income,
}
impl ResultPanel {
    pub fn asset(self) -> &'static str {
        if self == Self::Income {
            "ResultIncomePanel"
        } else {
            "ResultWorkPanel"
        }
    }
    pub fn background(self) -> &'static str {
        match self {
            Self::Work(Shift::Day) => "ShinShitsumu_ChairRot",
            Self::Work(Shift::Night) => "ShinShitsumu_ChairRot_Night",
            Self::Income => "ShinShitsumu_Night",
        }
    }
}
pub(super) fn scene(tag: Option<&String>) -> Result<Scene> {
    Ok(match tag.map(String::as_str) {
        Some("ResultWorkDaytime") => Scene::Result(ResultPanel::Work(Shift::Day)),
        Some("ResultWorkNight") => Scene::Result(ResultPanel::Work(Shift::Night)),
        Some("ResultIncome") => Scene::Result(ResultPanel::Income),
        _ => Scene::Daily(daily::Period::from_tag(tag)?),
    })
}

fn maid_widget(p: &str) -> bool {
    if !p.contains("MaidStatus/") {
        return p.ends_with("/MaidStatus");
    }
    !p.ends_with("/SelectCursor")
        && !p.ends_with("/Engage")
        && !p.ends_with("/Reader")
        && !p.ends_with("/Ranking/Icon")
}
pub(super) fn wanted(scene: &str, p: &str) -> bool {
    match scene {
        "SceneCharacterSelect" => p.starts_with("UI Root/Parent/") && (
            p.ends_with("/ButtonParent/Cancel") || p.ends_with("/ButtonParent/OK") ||
            p.contains("/ExplanatoryText") || p.ends_with("/CharacterSelectPanel/BG") ||
            p.ends_with("/CharacterSelectPanel/Contents") || p.ends_with(STAFFGRID)),
        "ResultIncomePanel" => p.contains("/ResultIncomePanel/") && !p.contains("/BonusIncome"),
        "StaffPlate" | "PickerPlate" => {
            p.ends_with("/Button")
                || p.contains("/LeftPlate/") && (scene == "PickerPlate" || !p.contains("/Select")) && !p.ends_with("/Engage")
        }
        "TrainingCategory" => p.ends_with("/Button") || p.ends_with("/Label"),
        "TrainingUnit" | "WorkUnit" => {
            p.ends_with("/Title") || p.ends_with("/Main") || p.ends_with("/Main/SelectCursor")
        }
        "ResultMaidUnit" => {
            maid_widget(p)
                || p.contains("/UpParam/PanelFacilityParam/")
                || p.ends_with("/UpParam/LabelWaring")
                || p.contains("/CommuNoon")
                || p.ends_with("/Task")
                || (p.contains("/UpParam/PanelBasicParam/")
                    || p.contains("/UpParam/PanelSubParam/"))
                    && (p.contains("/TitleBar")
                        || p.ends_with("/Param")
                        || p.ends_with("/ParamParent")
                        || p.ends_with("/Param/Value"))
        }
        "ResultWorkPanel" => {
            p.ends_with(RGRID)
                || p.ends_with("/MaidStatusViewer/BG/Contents")
                || p.contains("/ResultWorkPanel/")
                    && (p.ends_with("/Ok")
                        || p.ends_with("/MaidStatusViewer/BG")
                        || p.contains("/MaidStatusViewer/Header/"))
        }
        "SchedulePanel" => {
            p.contains("/SchedulePanel/")
                && (p.ends_with("/SchedulePanel/Title")
                    || p.ends_with("/SelectedMaidViewer/Ok")
                    || p.ends_with("/SelectedMaidViewer/BG")
                    || p.contains("/SelectedMaidViewer/Header/")
                    || p.ends_with("/SelectedMaidViewer/Contents")
                    || p.ends_with(SLOTS)
                    || p.ends_with("/MainCategoryViewer/Base")
                    || p.ends_with("/MainCategoryViewer/Content")
                    || p.ends_with(CGRID)
                    || p.ends_with("/CharacterSelectPanel/BG")
                    || p.ends_with("/CharacterSelectPanel/Contents")
                    || p.ends_with(STAFFGRID)
                    || p.ends_with("/UnitViewer/Content")
                    || p.ends_with(TGRID)
                    || p.ends_with("/UnitViewer/Base")
                    || p.contains(SLOTS)
                        && p.contains(ROW)
                        && (maid_widget(p)
                            || p.contains("/CommuNoon")
                            || p.contains("/CommuNight")
                            || ["DaytimeTask", "NightTask"].iter().any(|kind| {
                                p.ends_with(&format!("/{kind}"))
                                    || p.contains(&format!("/{kind}/NamePlate"))
                                    || p.ends_with(&format!("/{kind}/SelectCursor"))
                            })))
        }
        _ => false,
    }
}

#[derive(Default, Clone, Copy)]
struct Grid {
    origin: [f32; 2],
    step: f32,
    clip: [f32; 4],
}
#[derive(Default, Clone, Copy)]
struct ParamGrid {
    origin: [f32; 2],
    template: [f32; 2],
    columns: usize,
    cell: [f32; 2],
}
#[derive(Default)]
pub(super) struct Layout {
    roster: Grid,
    tasks: Grid,
    results: Grid,
    category: Grid,
    staff: Grid,
    params: [ParamGrid; 4],
}
impl Layout {
    pub fn read(&mut self, class: &str, path: &str, offset: [f32; 3], data: &[u8]) -> Result<()> {
        // ScheduleMaidStatusUnit reparents its Param template under this UIGrid.
        if let Some(i) = ["/PanelBasicParam/","/PanelSubParam/","/PanelFacilityParam/Panel_Basic/","/PanelFacilityParam/Panel_Add/"].iter().position(|p|path.contains(p)) {
            let g = &mut self.params[i];
            if path.ends_with("/Param") && class == "UILabel" {
                g.template = [offset[0], -offset[1]];
            }
            if path.ends_with("/ParamParent") && class == "UIGrid" {
                let mut r = Reader::new(data);
                r.seek(32)?;
                let arrangement = r.i32()?;
                r.seek(44)?;
                let count = r.i32()?;
                g.cell = r.f32s::<2>()?;
                g.columns = match (arrangement, count) {
                    (0, n) if n > 0 => n.try_into()?,
                    (1, 0) => 1,
                    _ => bail!("unsupported receipt grid {path}"),
                };
                g.origin = [offset[0], -offset[1]];
            }
            return Ok(());
        }
        let (g, grid, panel) = if path.contains("/SelectedMaidViewer/Contents") {
            (&mut self.roster, SLOTS, "/SelectedMaidViewer/Contents")
        } else if path.contains("/CharacterSelectPanel/Contents") {
            (&mut self.staff, STAFFGRID, "/CharacterSelectPanel/Contents")
        } else if path.contains("/MainCategoryViewer/Content") {
            (&mut self.category, CGRID, "/MainCategoryViewer/Content")
        } else if path.contains("/UnitViewer/Content") {
            (&mut self.tasks, TGRID, "/UnitViewer/Content")
        } else if path.contains("/MaidStatusViewer/BG/Contents") {
            (&mut self.results, RGRID, "/MaidStatusViewer/BG/Contents")
        } else {
            return Ok(());
        };
        let mut r = Reader::new(data);
        if class == "UIGrid" && path.ends_with(grid) {
            r.seek(52)?;
            g.step = r.f32()?;
            g.origin = [offset[0], -offset[1]];
            ensure!(
                g.step.is_finite() && g.step > 0.,
                "invalid retail grid {path}"
            );
        }
        if class == "UIPanel" && path.ends_with(panel) {
            r.seek(172)?;
            let [x, y, w, h] = r.f32s::<4>()?;
            r.seek(204)?;
            let [dx, dy] = r.f32s::<2>()?;
            g.clip = [
                960. + offset[0] + x + dx - w / 2.,
                540. - offset[1] - y - dy - h / 2.,
                w,
                h,
            ];
        }
        Ok(())
    }
}

pub(super) fn prepare_plate(w:&mut Widget,id:usize,selected:bool,s:&TjsState,assets:&mut Assets,host:&mut dyn HostServices,client:ClientId)->Result<bool>{
    let m=&s.maids[&id];
                    if w.path.contains("/Select") && !selected { return Ok(false); }
                    if w.path.ends_with("/LeaderPlate") && !m.leader {
                        return Ok(false);
                    }
                    if let Some(rank) = w
                        .path
                        .rsplit_once("/Rank")
                        .and_then(|(_, r)| r.parse::<i32>().ok())
                        && rank != m.popular_rank
                    {
                        return Ok(false);
                    }
                    if w.path.ends_with("/TextLastName") {
                        text(w, &m.last_name);
                    }
                    if w.path.ends_with("/TextFirstName") {
                        text(w, &m.first_name);
                    }
                    if w.path.ends_with("/RankLabel") {
                        text(
                            w,
                            &if (1..=99).contains(&m.popular_rank) {
                                m.popular_rank.to_string()
                            } else {
                                "-".into()
                            },
                        );
                    }
                    if w.path.ends_with("/TextTypeName") {
                        text(
                            w,
                            ["育成中", "フリー", "専属"]
                                .get(m.contract_type as usize)
                                .copied()
                                .unwrap_or(""),
                        );
                    }
                    if w.path.ends_with("/Thum") {
                        let Some(asset) = assets.schedule_portrait(host, client, s, id)? else {
                            return Ok(false);
                        };
                        w.asset = asset;
                    }
    Ok(true)
}
pub(super) fn move_by(w: &mut Widget, dx: f32, dy: f32) {
    w.rect[0] += dx;
    w.rect[1] += dy;
    if let Some(c) = &mut w.collider {
        c[0] += dx;
        c[1] += dy;
    }
}
fn text(w: &mut Widget, value: &str) {
    if let Some(l) = &mut w.label {
        l.set_text(value);
    }
}
pub(super) fn contains([x, y, w, h]: [f32; 4], i: &InputFrame) -> bool {
    (x..x + w).contains(&(i.pointer_x as f32)) && (y..y + h).contains(&(i.pointer_y as f32))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Ok,
    Cancel,
    Staff(usize),
    Choose(usize),
    Shift(usize, Shift),
    Training,
    WorkCategory,
    AssignUpgrade,
    OpenRooms(i32),
    Room(usize),
    CloseRooms,
    Communication(usize, Shift),
    Assign(i32),
}
struct RosterDrag {
    slot: usize,
    origin: [i32;2],
    offset: [f32;2],
    moving: bool,
}

#[derive(Default)]
pub(super) struct Panel {
    daily: Option<Vec<Widget>>,
    base: Vec<Widget>,
    unit: Vec<Widget>,
    selection: Option<(usize, Shift)>,
    stamp: Option<Widget>,
    plate: Vec<Widget>,
    category: Vec<Widget>,
    paid_unit: Vec<Widget>,
    work_category: bool,
    room_work: Option<i32>,
    room_notice: Option<quit_dialog::QuitDialog>,
    staff_slot: Option<usize>,
    staff_scroll: usize,
    picker: picker::Picker,
    training: bool,
    hover: Option<Action>,
    drag: Option<RosterDrag>,
    remove_pressed: Option<usize>,
    remove_dialog: Option<(usize, quit_dialog::QuitDialog)>,
    bindings: std::collections::BTreeMap<String, Action>,
    opened: u64,
    close: Option<u64>,
    row_scroll: usize,
    task_scroll: usize,
}
impl Panel {
    pub fn in_schedule(&self) -> bool {
        self.daily.is_some()
    }
    pub fn alpha(&self, scene: Scene, now: u64, entered: u64) -> Option<u8> {
        if !(matches!(scene, Scene::Daily(_)) && self.in_schedule()
            || matches!(scene, Scene::Result(_)))
        {
            return None;
        }
        let start = if self.in_schedule() {
            self.opened
        } else {
            entered
        };
        let v = self
            .close
            .map_or(now.saturating_sub(start) as f32 / 250., |t| {
                1. - now.saturating_sub(t) as f32 / 250.
            });
        Some((255. * v.clamp(0., 1.)).round() as u8)
    }
    pub fn clip(&self, scene: Scene, path: &str, l: &Layout) -> Option<[f32; 4]> {
        if path.starts_with("roster/") && matches!(scene, Scene::Daily(_)) {
            Some(l.roster.clip)
        } else if path.starts_with("staff/") && matches!(scene, Scene::Daily(_) | Scene::CharacterSelect(_)) {
            Some(l.staff.clip)
        } else if path.starts_with("task/") && matches!(scene, Scene::Daily(_)) {
            Some(l.tasks.clip)
        } else if path.starts_with("result/") && matches!(scene, Scene::Result(_)) {
            Some(l.results.clip)
        } else {
            None
        }
    }
    fn hit(
        &self,
        widgets: &[Widget],
        scene: Scene,
        l: &Layout,
        input: &InputFrame,
    ) -> Option<usize> {
        widgets.iter().enumerate().rev().find_map(|(i, w)| {
            (self.bindings.contains_key(&w.path)
                && w.collider.is_some_and(|r| contains(r, input))
                && self
                    .clip(scene, &w.path, l)
                    .is_none_or(|r| contains(r, input)))
            .then_some(i)
        })
    }
    fn update_hover(
        &mut self,
        widgets: &[Widget],
        scene: Scene,
        layout: &Layout,
        input: &InputFrame,
    ) {
        self.hover = self
            .hit(widgets, scene, layout, input)
            .and_then(|i| self.bindings.get(&widgets[i].path))
            .copied();
    }
    pub fn tooltip_visible(&self, path: &str) -> bool {
        if !path.starts_with("roster/") {
            return true;
        }
        let Some((button, _)) = path.rsplit_once("/NamePlate") else {
            return true;
        };
        self.remove_dialog.is_none()
            && self
                .bindings
                .get(button)
                .is_some_and(|a| Some(*a) == self.hover)
    }
    fn click(
        &self,
        widgets: &[Widget],
        scene: Scene,
        l: &Layout,
        input: &InputFrame,
        released: u64,
        pressed: &mut Option<usize>,
    ) -> Option<Action> {
        if (input.buttons_pressed | released) & buttons::MOUSE_LEFT == 0 {
            return None;
        }
        let hit = self.hit(widgets, scene, l, input);
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            *pressed = hit;
        }
        if released & buttons::MOUSE_LEFT == 0 || pressed.take() != hit {
            return None;
        }
        self.bindings.get(&widgets[hit?].path).copied()
    }
    pub fn dialog(&self) -> Option<&quit_dialog::QuitDialog> {
        self.remove_dialog.as_ref().map(|(_, d)| d).or(self.picker.dialog.as_ref()).or(self.room_notice.as_ref())
    }
    pub fn drag_offset(&self, path:&str) -> Option<[f32;2]> {
        let drag=self.drag.as_ref().filter(|drag|drag.moving)?;
        let n=path.strip_prefix("roster/")?.split('/').next()?.parse::<usize>().ok()?;
        (n==drag.slot).then_some(drag.offset)
    }

    fn roster_drag(&mut self,s:&mut TjsState,widgets:&[Widget],scene:Scene,layout:&Layout,
        input:&InputFrame,released:u64,pressed:&mut Option<usize>) -> Result<bool> {
        let left=buttons::MOUSE_LEFT;
        if input.buttons_pressed & left != 0 {
            self.drag=self.hit(widgets,scene,layout,input).and_then(|i|match self.bindings.get(&widgets[i].path) {
                Some(Action::Staff(slot)) => Some(RosterDrag {slot:*slot,origin:[input.pointer_x,input.pointer_y],offset:[0.;2],moving:false}),
                _=>None
            });
        }
        let Some(drag)=&mut self.drag else {return Ok(false)};
        let dx=input.pointer_x as f32-drag.origin[0] as f32;
        let dy=input.pointer_y as f32-drag.origin[1] as f32;
        // Level 3 UICamera.mouseDragThreshold is four. This prefab's
        // UIDragDropMaidStatus has no direction or press-and-hold restriction.
        drag.moving |= dx*dx+dy*dy > 16.;
        drag.offset=[dx,dy];
        let from=drag.slot;
        let moving=drag.moving;
        if released & left == 0 {return Ok(moving);}
        self.drag=None;
        if !moving {return Ok(false);}
        *pressed=None;
        let target=self.hit(widgets,scene,layout,input).and_then(|i|
            widgets[i].path.strip_prefix("roster/")?.split('/').next()?.parse::<usize>().ok());
        if let Some(to)=target && from!=to {
            operations::swap_staff(s,from,to)?;
            let remap=|n:usize| if n==from {to} else if n==to {from} else {n};
            self.selection=self.selection.map(|(slot,shift)|(remap(slot),shift));
            self.staff_slot=self.staff_slot.map(remap);
        }
        Ok(true)
    }

    fn right_click_staff(
        &mut self,
        widgets: &[Widget],
        scene: Scene,
        layout: &Layout,
        input: &InputFrame,
        released: u64,
    ) -> Option<usize> {
        if (input.buttons_pressed | released) & buttons::MOUSE_RIGHT == 0 {
            return None;
        }
        let hovered = self.hit(widgets, scene, layout, input).and_then(|i| {
            match self.bindings[&widgets[i].path] {
                Action::Staff(slot) => Some(slot),
                _ => None,
            }
        });
        if input.buttons_pressed & buttons::MOUSE_RIGHT != 0 {
            self.remove_pressed = hovered;
        }
        if released & buttons::MOUSE_RIGHT != 0 && self.remove_pressed.take() == hovered {
            hovered
        } else {
            None
        }
    }
    fn resolve_removal(
        &mut self,
        s: &mut TjsState,
        widgets: &mut Vec<Widget>,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<bool> {
        let (slot, dialog) = self
            .remove_dialog
            .as_mut()
            .context("missing removal dialog")?;
        let Some(choice) = dialog.update(widgets, input, released, now) else {
            return Ok(false);
        };
        let slot = *slot;
        widgets.truncate(dialog.first);
        self.remove_dialog = None;
        if choice == quit_dialog::Choice::Cancel {
            return Ok(false);
        }
        operations::remove_staff(s, slot)?;
        if self.staff_slot == Some(slot)
            || self.selection.is_some_and(|(selected, _)| selected == slot)
        {
            self.staff_slot = None;
            self.selection = None;
            self.training = false;
        }
        Ok(true)
    }
    fn activate(&mut self, action: Action, s: &mut TjsState, now: u64) -> Result<()> {
        match action {
            Action::Cancel => bail!("unexpected schedule cancel"),
            Action::AssignUpgrade => {let (slot,shift)=self.selection.context("missing assignment cell")?; operations::upgrade::assign(s,slot,shift)?;}
            Action::WorkCategory => {
                self.training = false;
                self.work_category = true;
                self.task_scroll = 0;
            }
            Action::OpenRooms(id) => self.room_work = Some(id),
            Action::CloseRooms => self.room_work = None,
            Action::Room(index) => {
                let (slot, shift) = self.selection.context("missing facility slot")?;
                operations::paid::assign_room(s, slot, shift, self.room_work.context("missing room selector")?, index)?;
            }
            Action::Ok => {
                if (0..40).any(|i| s.schedule_slot(i).is_some()) {
                    self.close = Some(now);
                }
            }
            Action::Staff(slot) => {
                self.work_category = false;
                self.staff_slot = Some(slot);
                self.selection = None;
                self.training = false;
                self.staff_scroll = 0;
            }
            Action::Choose(id) => {
                operations::choose_staff(
                    s,
                    self.staff_slot.context("missing roster selection")?,
                    id,
                )?;
            }
            Action::Shift(slot, shift) => {
                self.staff_slot = None;
                self.selection = Some((slot, shift));
                self.task_scroll = 0;
                let work = shift.work(&s.maids[&s.schedule_slot(slot).context("missing slot")?]);
                self.work_category = work==operations::upgrade::job(s)? || operations::paid::tasks(s,slot,true)?.iter().any(|t| t.id == work);
                self.training = operations::basic_tasks(s, slot)?
                    .iter()
                    .any(|t| t.id == work);
            }
            Action::Communication(slot, shift) => operations::toggle_communication(s, slot, shift)?,
            Action::Training => {
                self.work_category = false;
                self.training = true;
                self.task_scroll = 0;
            }
            Action::Assign(work) => {
                let (slot, shift) = self.selection.context("no selected shift")?;
                operations::assign(s, slot, shift, work)?;
            }
        }
        Ok(())
    }
}

fn setup_income(widgets: &mut [Widget], s: &TjsState) {
    let [working, income, total, before, change, after] = operations::income(s);
    let income = income as i32;
    for w in widgets {
        let Some((name, "Value")) = w.path.rsplit_once('/') else {
            continue;
        };
        let value = match name.rsplit('/').next().unwrap_or("") {
            "WorkingFunds" => daily::money(working),
            "ClubIncome" => format!(
                "{}{}",
                if income >= 0 { "+" } else { "" },
                daily::money(i64::from(income))
            ),
            "TotalAmount" => daily::money(total),
            "ClientEvaluation" => before.to_string(),
            "AcquisitionEvaluation" => format!("{}{change}", if change >= 0 { "+" } else { "-" }),
            "Result" => after.to_string(),
            _ => continue,
        };
        text(w, &value);
    }
}

fn task_info(s: &mut TjsState, work: i32) -> Result<(String, String)> {
    if work==operations::upgrade::job(s)? {let task=operations::upgrade::task(work);return Ok((task.name,task.icon));}
    for (file, icon) in [
        ("schedule_work_noon.nei", 4),
        ("schedule_work_night.nei", 3),
        ("schedule_work_easyyotogi.nei", 3),
        ("facility.nei", 19),
    ] {
        let t = s.retail.table(file)?;
        let facility = file == "facility.nei";
        if let Some(r) =
            (1..t.rows).find(|&r| t.exists(0, r) && t.int(usize::from(facility), r) == work)
        {
            return Ok((
                t.string(if facility { 24 } else { 1 }, r),
                t.string(icon, r),
            ));
        }
    }
    bail!("unknown standalone schedule work {work}")
}

fn maid_label(w: &mut Widget, m: &crate::params::MaidStatus) {
    if w.path.ends_with("/LastName") {
        text(w, &m.last_name)
    } else if w.path.ends_with("/FirstName") {
        text(w, &m.first_name)
    } else if w.path.ends_with("/Ranking/Value") {
        text(
            w,
            &if m.popular_rank > 0 {
                m.popular_rank.to_string()
            } else {
                "-".into()
            },
        )
    } else if w.path.ends_with("/ContractType/Value") {
        text(
            w,
            ["育成中", "フリー", "専属"]
                .get(m.contract_type as usize)
                .copied()
                .unwrap_or(""),
        )
    }
}

impl Panel {
    fn populate(
        &mut self,
        result: Option<Shift>,
        s: &mut TjsState,
        assets: &mut Assets,
        op: &Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        files: &GameFiles,
    ) -> Result<Vec<Widget>> {
        let mut out = Vec::new();
        self.bindings.clear();
        let grid = if result.is_some() {
            assets.operations_layout.results
        } else {
            assets.operations_layout.roster
        };
        ensure!(
            grid.step > 0. && grid.clip[3] > 0.,
            "missing native roster grid"
        );
        let available = if let Some((slot, _)) = self.selection {
            operations::basic_tasks(s, slot)?
        } else {
            Vec::new()
        };
        let upgrade_job=operations::upgrade::job(s)?;
        let paid = if let Some((slot,_)) = self.selection { let mut tasks=operations::paid::tasks(s,slot,true)?;tasks.insert(0,operations::upgrade::task(upgrade_job));tasks } else {Vec::new()};
        for mut w in self.base.iter().filter(|w| !w.path.contains(ROW)).cloned() {
            if w.path.ends_with("/Ok") {
                self.bindings.insert(w.path.clone(), Action::Ok);
            }
            if w.path.ends_with("/Frame_Time/Label") {
                text(
                    &mut w,
                    if result == Some(Shift::Day) {
                        "昼"
                    } else {
                        "夜"
                    },
                );
            }
            if w.path.ends_with("/UnitViewer/Base") && !(self.training || self.work_category)
                || w.path.ends_with("/MainCategoryViewer/Base") && available.is_empty() && paid.is_empty()
                || w.path.ends_with("/CharacterSelectPanel/BG") && self.staff_slot.is_none()
            {
                continue;
            }
            out.push(w);
        }
        let template: Vec<_> = if result.is_some() {
            self.unit.clone()
        } else {
            self.base
                .iter()
                .filter(|w| w.path.contains(ROW))
                .cloned()
                .collect()
        };
        let visible = (grid.clip[3] / grid.step).ceil() as usize + 1;
        for slot in self.row_scroll..(self.row_scroll + visible).min(40) {
            let Some(id) = s.schedule_slot(slot) else {
                if result.is_none() {
                    let mut w = template
                        .iter()
                        .find(|w| w.path.ends_with("/MaidStatus"))
                        .context("missing empty roster button")?
                        .clone();
                    move_by(&mut w, 0., (slot - self.row_scroll) as f32 * grid.step);
                    w.path = format!("roster/{slot}/{}", w.path);
                    self.bindings.insert(w.path.clone(), Action::Staff(slot));
                    out.push(w);
                }
                continue;
            };
            let [day, night] = [s.maids[&id].noon_work_id, s.maids[&id].night_work_id];
            let tasks = operations::basic_tasks(s, slot)?;
            let eligible = [day, night].map(|work| tasks.iter().any(|task| task.id == work));
            let day = task_info(s, day)?;
            let night = task_info(s, night)?;
            let portrait = assets.schedule_portrait(host, client, s, id)?;
            let paid_tasks = operations::paid::tasks(s, slot, true)?;
            let eligible = [eligible[0] || paid_tasks.iter().any(|t|t.id==s.maids[&id].noon_work_id), eligible[1] || paid_tasks.iter().any(|t|t.id==s.maids[&id].night_work_id)];
            let receipt = result.map(|shift| operations::receipt(s, slot, shift)).transpose()?;
            let work = result.map(|shift|shift.work(&s.maids[&id]));
            let paid_receipt = work.is_some_and(|work|paid_tasks.iter().any(|t|t.id==work));
            let is_upgrade=work==Some(operations::upgrade::job(s)?);
            let upgrade=if is_upgrade {operations::upgrade::receipt(s,id)?}else{None};
            let mut warning=None;
            if let (Some(shift),Some((success,phases)),Some(work))=(result,receipt,work) {
                if is_upgrade && success==3 {warning=Some("施設の強化がキャンセルされました".into());}
                else if !paid_receipt && success==0 && shift==Shift::Night && phases.1=="NightBonus" && !eligible[1] {
                    warning=Some("夜伽がキャンセルされました".to_owned());
                } else if paid_receipt && success==3 {
                    warning=operations::paid::understaffed(s,slot,shift,work)?;
                }
            }
            let m = &s.maids[&id];
            for mut w in template.clone() {
                let mut action = (result.is_none() && w.path.ends_with("/MaidStatus"))
                    .then_some(Action::Staff(slot));
                let shift = if w.path.contains("/DaytimeTask") {
                    Some(Shift::Day)
                } else if w.path.contains("/NightTask") {
                    Some(Shift::Night)
                } else if w.path.ends_with("/Task") {
                    result
                } else {
                    None
                };
                if let Some(shift) = shift {
                    let (name, icon) = if shift == Shift::Day { &day } else { &night };
                    if w.path.ends_with("/SelectCursor") && self.selection != Some((slot, shift)) {
                        continue;
                    }
                    if w.path.ends_with("/NamePlate/Name") {
                        text(&mut w, name);
                    }
                    if w.path.ends_with("/DaytimeTask")
                        || w.path.ends_with("/NightTask")
                        || w.path.ends_with("/Task")
                    {
                        w.asset = assets.schedule_icon(op, host, client, files, icon)?;
                        if result.is_none() {
                            action = Some(Action::Shift(slot, shift));
                        }
                    }
                } else if w.path.contains("/CommuNoon") || w.path.contains("/CommuNight") {
                    let shift = result.unwrap_or(if w.path.contains("/CommuNoon") {
                        Shift::Day
                    } else {
                        Shift::Night
                    });
                    if !eligible[usize::from(shift == Shift::Night)] {
                        continue;
                    }
                    let checked = operations::communication(s, id, shift);
                    if w.path.contains("/SelectCursor") && !checked {
                        continue;
                    }
                    // Each roster row clones the same prefab component IDs. Resolve
                    // the radio graphic per row before the common widget renderer.
                    w.toggle = None;
                    if w.path.ends_with("/CommuNoon") || w.path.ends_with("/CommuNight") {
                        if result.is_none() {
                            action = Some(Action::Communication(slot, shift));
                        }
                    }
                } else if w.path.ends_with("/Figure/Icon") {
                    let Some(asset) = &portrait else { continue };
                    w.asset = asset.clone();
                } else if w.path.contains("/UpParam/") {
                    if w.path.contains("/PanelFacilityParam/") {
                        let Some(v)=upgrade.as_ref() else {continue};
                        if w.path.ends_with("/RoomNo"){text(&mut w,&format!("ROOM {:02}",v.room+1));}
                        else if w.path.ends_with("/InfoParent/RoomName"){text(&mut w,&v.name);}
                        else if w.path.ends_with("/InfoParent/RecipeNo"){text(&mut w,&format!("RECIPE {:04}",v.recipe.id));}
                        else if w.path.ends_with("/InfoParent/Recipe"){text(&mut w,&v.recipe.name);}
                        else if w.path.ends_with("/Thumbnail"){assets.facility_thumbnail(op,host,client,&mut w,&v.thumbnail)?;}
                    } else if w.path.ends_with("/LabelWaring") {
                        let Some(message)=&warning else {continue};
                        text(&mut w,message);
                    } else if warning.is_some() || is_upgrade {continue;}
                    if w.path.ends_with("/Param") || w.path.ends_with("/Param/Value") {
                        continue;
                    }
                } else {
                    maid_label(&mut w, m);
                }
                let dy = (slot - self.row_scroll) as f32 * grid.step;
                if result.is_some() {
                    move_by(&mut w, grid.origin[0], grid.origin[1] + dy);
                } else {
                    move_by(&mut w, 0., dy);
                }
                w.path = format!(
                    "{}/{slot}/{}",
                    if result.is_some() { "result" } else { "roster" },
                    w.path
                );
                if let Some(action) = action {
                    self.bindings.insert(w.path.clone(), action);
                }
                let is_result_task = result.is_some() && w.path.ends_with("/Task");
                let center = [w.rect[0] + w.rect[2] / 2., w.rect[1] + w.rect[3] / 2.];
                out.push(w);
                if is_result_task {
                    let level = receipt.context("missing work receipt")?.0;
                    let name = match level {
                        0 | 3 => "sippai",
                        1 => "seikou",
                        2 => "daiseikou",
                        _ => bail!("unknown success level {level}"),
                    };
                    let mut stamp = self.stamp.clone().context("missing result stamp prefab")?;
                    stamp.asset = assets.schedule_icon(
                        op,
                        host,
                        client,
                        files,
                        &format!("CM3D2_work_result_mark_{name}"),
                    )?;
                    move_by(&mut stamp, center[0] - 960., center[1] - 540.);
                    stamp.path = format!("result/{slot}/{}", stamp.path);
                    out.push(stamp);
                }
            }
            if let Some(v)=upgrade {
                for (index,values) in [v.recipe.base,v.additional].iter().enumerate() {
                    let group=if index==0 {"/PanelFacilityParam/Panel_Basic/"}else{"/PanelFacilityParam/Panel_Add/"};
                    let fields=v.names.iter().zip(values).map(|(name,n)|(name.as_str(),name.as_str(),*n,false));
                    self.param_values(slot,fields,group,assets.operations_layout.params[2+index],assets.operations_layout.results,&mut out)?;
                }
            } else if let Some((_, phases)) = receipt && warning.is_none() && !is_upgrade {
                self.result_values(s, slot, phases, paid_receipt, &assets.operations_layout, &mut out)?;
            }
        }
        if self.staff_slot.is_some() {
            self.staff_widgets(operations::candidates(s), s, assets, host, client, &mut out)?;
        }
        let categories = [(51,Action::Training,!available.is_empty()),(52,Action::WorkCategory,!paid.is_empty())];
        for (index,(category,action,_)) in categories.into_iter().filter(|c| c.2).enumerate() {
            let g = assets.operations_layout.category;
            ensure!(
                g.step > 0. && g.clip[3] > 0.,
                "missing training category grid"
            );
            let names = s.retail.table("schedule_work_night_category_list.nei")?;
            let row = (1..names.rows)
                .find(|&r| names.int(0, r) == category)
                .context("missing basic training category")?;
            for mut w in self.category.clone() {
                if w.path.ends_with("/Label") {
                    text(&mut w, &names.string(1, row));
                }
                if w.path.ends_with("/Button") {
                    w.path = format!("category/{category}/{}",w.path);
                    self.bindings.insert(w.path.clone(), action);
                }
                move_by(&mut w, g.origin[0], g.clip[1] - 540. + g.step * (index as f32 + 0.5));
                out.push(w);
            }
        }
        if (self.training || self.work_category)
            && let Some((slot, shift)) = self.selection
        {
            let grid = assets.operations_layout.tasks;
            ensure!(
                grid.step > 0. && grid.clip[3] > 0.,
                "missing native task grid"
            );
            let selected = shift.work(
                &s.maids[&s
                    .schedule_slot(slot)
                    .context("missing active schedule slot")?],
            );
            // ResetNGUI aligns the populated grid to the top of its scroll view.
            let tasks = if self.work_category { paid } else {available};
            for (i, task) in tasks
                .into_iter()
                .skip(self.task_scroll)
                .take((grid.clip[3] / grid.step).ceil() as usize + 1)
                .enumerate()
            {
                let template = if self.work_category {&self.paid_unit} else {&self.unit};
                for mut w in template.iter().cloned() {
                    if w.path.ends_with("/SelectCursor") && task.id != selected {
                        continue;
                    }
                    if w.path.ends_with("/Title") {
                        text(&mut w, &task.name);
                    }
                    let button = w.path.ends_with("/Main");
                    if button {
                        w.asset = assets.schedule_icon(op, host, client, files, &task.icon)?;
                    }
                    move_by(
                        &mut w,
                        grid.origin[0],
                        grid.clip[1] - 540. + i as f32 * grid.step,
                    );
                    w.path = format!("task/{}/{}", task.id, w.path);
                    if button {
                        self.bindings
                            .insert(w.path.clone(), if task.id==upgrade_job {Action::AssignUpgrade} else if self.work_category {Action::OpenRooms(task.id)} else {Action::Assign(task.id)});
                    }
                    out.push(w);
                }
            }
        }
        out.sort_by_key(|w| w.depth);
        Ok(out)
    }
    fn room_widgets(&mut self, s: &mut TjsState, slot: usize, shift: Shift, work: i32, assets: &mut Assets,
        op: &mut Opener, host: &mut dyn HostServices, client: ClientId) -> Result<Vec<Widget>> {
        let rooms = operations::paid::rooms(s,slot,shift,work)?;
        let style = self.base.iter().find_map(|w| w.label.as_ref()).context("missing schedule label")?;
        let id=s.schedule_slot(slot).context("missing room selector employee")?;
        let portrait=assets.schedule_portrait(host,client,s,id)?;
        let widgets = assets.room_list(op,host,client,style,&rooms,portrait.as_deref())?;
        self.bindings.clear();
        for w in &widgets {
            if w.collider.is_none() {continue;}
            if w.path.ends_with("/Button OK") {
                self.bindings.insert(w.path.clone(),Action::CloseRooms);
            } else if let Some(index) = w.path.strip_prefix("room/").and_then(|x| x.parse::<usize>().ok()) {
                self.bindings.insert(w.path.clone(),Action::Room(index));
            }
        }
        Ok(widgets)
    }
    fn staff_widgets(&mut self, candidates: Vec<usize>, s: &TjsState, assets: &mut Assets,
        host: &mut dyn HostServices, client: ClientId, out: &mut Vec<Widget>) -> Result<()> {
            let g = assets.operations_layout.staff;
            ensure!(
                g.step > 0. && g.clip[3] > 0.,
                "missing staff selection grid"
            );

            for (i, id) in candidates
                .into_iter()
                .skip(self.staff_scroll)
                .take((g.clip[3] / g.step).ceil() as usize + 1)
                .enumerate()
            {
                for mut w in self.plate.clone() {
                    if !prepare_plate(&mut w, id, self.picker.selected==Some(id), s, assets, host, client)? {continue;}
                    let button = w.path.ends_with("/Button");
                    move_by(
                        &mut w,
                        g.origin[0],
                        g.clip[1] - 540. + g.step * (i as f32 + 0.5),
                    );
                    w.path = format!("staff/{id}/{}", w.path);
                    if button {
                        self.bindings.insert(w.path.clone(), Action::Choose(id));
                    }
                    out.push(w);
                }
            }
        Ok(())
    }
    fn result_values(
        &self,
        s: &TjsState,
        slot: usize,
        phases: (&str, &str),
        paid: bool,
        layout: &Layout,
        out: &mut Vec<Widget>,
    ) -> Result<()> {
        let snapshot = &s.player.schedule_slot[slot].backup_status;
        let (prev, current) = phases;
        let diff = |key: &str| -> i32 {
            let get = |phase: &str| {
                snapshot
                    .get(&format!("{phase}_{key}"))
                    .copied()
                    .unwrap_or(0)
            };
            get(current).wrapping_sub(get(prev))
        };
        let sales =
            (i64::from(diff("sales_top10")) * 1_000_000_000 + i64::from(diff("sales_top0"))) as i32;
        let fields = [
            ("care", "お世話", diff("care")),
            ("lovely", "可　燐", diff("lovely")),
            ("cooking", "料　理", diff("cooking")),
            ("reception", "接　待", diff("reception")),
            ("elegance", "気　品", diff("elegance")),
            ("vocal", "ボーカル", diff("vocal")),
            ("teach_rate", "指　導", diff("teach_rate") / 10),
            ("charm", "魅　惑", diff("charm")),
            ("dance", "ダンス", diff("dance")),
            ("sales", "取得資金", sales),
            ("evaluation", "取得客評価", if paid {diff("evaluation")} else {0}),
            ("others_play_count", "接客回数", if paid {diff("others_play_count")} else {0}),
            ("maidClassExp", "ジョブメイド経験値", diff("maidClassExp")),
        ];
        for (panel, group) in ["/PanelBasicParam/","/PanelSubParam/"].into_iter().enumerate() {
            let rows=fields.iter().enumerate().filter(move |(i,_)|usize::from(*i>=9)==panel)
                .map(|(_, &(key,name,delta))|(key,name,delta,paid && key=="sales"));
            self.param_values(slot,rows,group,layout.params[panel],layout.results,out)?;
        }
        Ok(())
    }

    fn param_values<'a>(&self,slot:usize,fields:impl Iterator<Item=(&'a str,&'a str,i32,bool)>,group:&str,grid:ParamGrid,results:Grid,out:&mut Vec<Widget>)->Result<()> {
        for (n,(key,name,delta,money)) in fields.filter(|v|v.2>0).enumerate() {
            ensure!(grid.columns>0,"missing {group} receipt grid");
            let dx = results.origin[0] + grid.origin[0] - grid.template[0]
                + (n % grid.columns) as f32 * grid.cell[0];
            let dy = results.origin[1]
                + (slot - self.row_scroll) as f32 * results.step
                + grid.origin[1]
                - grid.template[1]
                + (n / grid.columns) as f32 * grid.cell[1];
            for mut w in self
                .unit
                .iter()
                .filter(|w| {
                    w.path.contains(group)
                        && (w.path.ends_with("/Param") || w.path.ends_with("/Param/Value"))
                })
                .cloned()
            {
                let value = if w.path.ends_with("/Value") {
                    if money {format!("+ {}",daily::money(i64::from(delta)))} else { format!("+ {delta}") }
                } else {
                    name.into()
                };
                text(&mut w, &value);
                move_by(&mut w, dx, dy);
                w.path = format!("result/{slot}/{key}/{}", w.path);
                out.push(w);
            }
        }
        Ok(())
    }
}

impl Game {
    pub(super) fn begin_schedule(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        now: u64,
    ) -> Result<()> {
        let base = self.assets.scene(op, host, client, "SchedulePanel")?;
        let unit = self.assets.scene(op, host, client, "TrainingUnit")?;
        let mut panel = Panel {
            base,
            unit,
            plate: self.assets.scene(op, host, client, "StaffPlate")?,
            paid_unit: self.assets.scene(op,host,client,"WorkUnit")?,
            category: self.assets.scene(op, host, client, "TrainingCategory")?,
            opened: now,
            ..Default::default()
        };
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        operations::open_schedule(s)?;
        let widgets = panel.populate(None, s, &mut self.assets, op, host, client, &self.files)?;
        panel.daily = Some(std::mem::replace(&mut self.widgets, widgets));
        self.business = panel;
        self.pressed = None;
        Ok(())
    }
    pub(super) fn update_schedule(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(t) = self.business.close {
            if now.saturating_sub(t) >= 250 {
                self.widgets = self.business.daily.take().context("missing daily panel")?;
                self.business = Default::default();
                self.pressed = None;
            }
            return Ok(());
        }
        if now.saturating_sub(self.business.opened) < 250 {
            return Ok(());
        }
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state;
        if self.business.remove_dialog.is_some() {
            if self
                .business
                .resolve_removal(s, &mut self.widgets, input, released, now)?
            {
                self.widgets = self.business.populate(
                    None,
                    s,
                    &mut self.assets,
                    op,
                    host,
                    client,
                    &self.files,
                )?;
            }
            return Ok(());
        }
        if let Some(work) = self.business.room_work {
            if let Some(notice)=&mut self.business.room_notice {
                if notice.update(&self.widgets,input,released,now).is_some() {
                    self.widgets.truncate(notice.first);
                    self.business.room_notice=None;
                }
                return Ok(());
            }
            let action = self.business.click(&self.widgets,self.scene,&self.assets.operations_layout,&input,released,&mut self.pressed);
            if let Some(action) = action {
                let (slot,shift) = self.business.selection.context("missing room selector shift")?;
                if let Action::Room(index)=action && operations::paid::rooms(s,slot,shift,work)?.get(index).is_some_and(|r|r.count>=r.capacity) {
                    let dialog=self.assets.scene(op,host,client,"SystemDialog")?;
                    let notice=quit_dialog::QuitDialog::show(&mut self.widgets,dialog,now,"この施設にはこれ以上メイドを配置出来ません。")?;
                    self.widgets.retain(|w|!w.path.contains("SystemDialog/Base/Cancel"));
                    self.business.room_notice=Some(notice);
                    return Ok(());
                }
                self.business.activate(action, s, now)?;
                self.widgets = if self.business.room_work.is_some() {
                    self.business.room_widgets(s,slot,shift,work,&mut self.assets,op,host,client)?
                } else {
                    self.business.populate(None,s,&mut self.assets,op,host,client,&self.files)?
                };
            }
            return Ok(());
        }
        if self.business.roster_drag(s,&self.widgets,self.scene,&self.assets.operations_layout,&input,released,&mut self.pressed)? {
            if released & buttons::MOUSE_LEFT != 0 {
                self.widgets=self.business.populate(None,s,&mut self.assets,op,host,client,&self.files)?;
            }
            return Ok(());
        }
        if let Some(slot) = self.business.right_click_staff(
            &self.widgets,
            self.scene,
            &self.assets.operations_layout,
            &input,
            released,
        ) && let Some(message) = operations::removal_prompt(s, slot)
        {
            let dialog = self.assets.scene(op, host, client, "SystemDialog")?;
            self.business.remove_dialog = Some((
                slot,
                quit_dialog::QuitDialog::show(&mut self.widgets, dialog, now, &message)?,
            ));
            self.pressed = None;
            return Ok(());
        }
        self.business.update_hover(
            &self.widgets,
            self.scene,
            &self.assets.operations_layout,
            &input,
        );
        let action = self.business.click(
            &self.widgets,
            self.scene,
            &self.assets.operations_layout,
            &input,
            released,
            &mut self.pressed,
        );
        match action {
            Some(action) => {
                self.business.activate(action, s, now)?;
                if let Some(work) = self.business.room_work {
                    let (slot,shift) = self.business.selection.context("missing room selector shift")?;
                    self.widgets = self.business.room_widgets(s,slot,shift,work,&mut self.assets,op,host,client)?;
                    return Ok(());
                }
                if self.business.close.is_some() {
                    return Ok(());
                }
            }
            None => {
                let layout = &self.assets.operations_layout;
                let moved = if let Some((slot, _)) = self.business.selection
                    && (self.business.training || self.business.work_category)
                    && contains(layout.tasks.clip, &input)
                {
                    scroll(
                        &mut self.business.task_scroll,
                        &input,
                        layout.tasks,
                        if self.business.work_category {1+operations::paid::tasks(s,slot,true)?.len()} else {operations::basic_tasks(s, slot)?.len()},
                    )
                } else if self.business.staff_slot.is_some() && contains(layout.staff.clip, &input)
                {
                    scroll(
                        &mut self.business.staff_scroll,
                        &input,
                        layout.staff,
                        operations::candidates(s).len(),
                    )
                } else {
                    scroll(&mut self.business.row_scroll, &input, layout.roster, 40)
                };
                if !moved {
                    return Ok(());
                }
            }
        }
        self.widgets =
            self.business
                .populate(None, s, &mut self.assets, op, host, client, &self.files)?;
        Ok(())
    }
    pub(super) fn open_result(
        &mut self,
        panel: ResultPanel,
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
        self.message_open = false;
        self.dialogue = None;
        self.camera_control = true;
        self.camera_move = None;
        self.adv_cover = None;
        self.world_cover = None;
        self.business = Default::default();
        self.actors.reset_for_day(s);
        self.actors.slots.retain(|&(man, _), _| man);
        s.active_maids.fill(None);
        if let ResultPanel::Work(shift) = panel {
            operations::settle(s, shift)?;
            self.business.base = std::mem::take(&mut self.widgets);
            self.business.unit = self.assets.scene(op, host, client, "ResultMaidUnit")?;
            self.business.stamp = self
                .assets
                .scene(op, host, client, "ResultStamp")?
                .into_iter()
                .find(|w| w.path == "ResultWorkIcon");
            self.widgets = self.business.populate(
                Some(shift),
                s,
                &mut self.assets,
                op,
                host,
                client,
                &self.files,
            )?;
            self.sound.play(
                host,
                client,
                &self.files,
                Kind::Se,
                "SE002.ogg",
                0.,
                false,
            )?;
        } else {
            setup_income(&mut self.widgets, s);
            let ok = self
                .widgets
                .iter()
                .find(|w| w.path.ends_with("/Ok") && w.collider.is_some())
                .context("missing result OK button")?;
            self.business.bindings.insert(ok.path.clone(), Action::Ok);
        }
        self.sound
            .play(host, client, &self.files, Kind::Bgm, "BGM015.ogg", 1., true)?;
        self.adv_camera = ([0.08306749, 1.299064, 0.0249091], 2.2, [177.9044, 2.546583]);
        Ok(())
    }
    pub(super) fn update_result(
        &mut self,
        panel: ResultPanel,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if let Some(t) = self.leaving {
            if now.saturating_sub(t) >= 1000 {
                self.resume_script(op, host, client, now)?;
            }
        } else if let Some(t) = self.business.close {
            if now.saturating_sub(t) >= 250 {
                if panel == ResultPanel::Income
                    || self
                        .script
                        .tjs
                        .lock()
                        .map_err(|_| anyhow::anyhow!("TJS lock"))?
                        .runtime
                        .host()
                        .state
                        .player
                        .flag("時間帯")
                        != 3
                {
                    self.sound.stop(host, client, Kind::Bgm, 1.)?;
                }
                self.leaving = Some(now);
            }
        } else if now.saturating_sub(self.entered) >= 1000 {
            if self.business.click(
                &self.widgets,
                self.scene,
                &self.assets.operations_layout,
                &input,
                released,
                &mut self.pressed,
            ) == Some(Action::Ok)
            {
                self.business.close = Some(now);
            } else if let ResultPanel::Work(shift) = panel
                && scroll(
                    &mut self.business.row_scroll,
                    &input,
                    self.assets.operations_layout.results,
                    40,
                )
            {
                let mut vm = self
                    .script
                    .tjs
                    .lock()
                    .map_err(|_| anyhow::anyhow!("TJS lock"))?;
                self.widgets = self.business.populate(
                    Some(shift),
                    &mut vm.runtime.host_mut().state,
                    &mut self.assets,
                    op,
                    host,
                    client,
                    &self.files,
                )?;
            }
        }
        Ok(())
    }
}
fn scroll(first: &mut usize, input: &InputFrame, g: Grid, count: usize) -> bool {
    if !contains(g.clip, input) {
        return false;
    }
    let rows = (g.clip[3] / g.step).floor().max(1.) as usize;
    let was = *first;
    if input.buttons_pressed & buttons::WHEEL_UP != 0 {
        *first = first.saturating_sub(1);
    }
    if input.buttons_pressed & buttons::WHEEL_DOWN != 0 {
        *first = (*first + 1).min(count.saturating_sub(rows));
    }
    *first != was
}

#[cfg(test)]
mod tests;
