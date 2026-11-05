//!4359E0/4375E0/436050/436DB0 native panel policy. Both the dynamic1917
//!and fixed1918 lists use the engine's ordinary retained UI/menu controller;
//!scope3 is independent of the three script builders. No private renderer.
use super::*;
use crate::machine::{
    Button, MenuValue, TextPart,
    text_ops::panel::{PanelDeclarations, PanelReply},
};
use crysirror_core::service::buttons;
const ROOT: u32 = 0x3c00_0000;
const UI_ROOT: u32 = ROOT + 1;
const SCROLL: u32 = ROOT + 2;
const SCROLL_BOTTOM: u32 = ROOT + 3;
const THUMB: u32 = ROOT + 4;
const ROW_UI_ROOT: u32 = ROOT + 5;
const MENU_BASE: u32 = 0x36fc_0000;
const MENU_UI: u32 = MENU_BASE + 0xfffd;
#[derive(Clone)]
struct Build {
    ticket: (u64, u8),
    declarations: PanelDeclarations,
    group: Group,
    boundary: usize,
    render_rows: usize,
    viewport: Option<[i16; 4]>,
    scroll: i32,
    selected: i32,
    focus: i32,
    last_scroll: i32,
}
pub(super) struct NativePanel {
    declarations: PanelDeclarations,
    dynamic: Group,
    row_boundary: usize,
    render_rows: usize,
    viewport: Option<[i16; 4]>,
    scroll: i32,
    selected: i32,
    focus: i32,
    last_scroll: i32,
    states: Vec<i16>,
    pending: Option<PanelReply>,
    sounds: Vec<u16>,
    sound_assets: Vec<String>,
    input_pending: bool,
    voices: Vec<String>,
    voice_started: bool,
    finished: bool,
    active: bool,
    pending_build: Option<Build>,
    last_ticket: Option<(u64, u8)>,
    begin_ticket: Option<(u64, u8)>,
    refresh_ticket: Option<(u64, u8)>,
    scroll_changed: bool,
}
impl Default for NativePanel {
    fn default() -> Self {
        Self {
            declarations: Default::default(),
            dynamic: Default::default(),
            row_boundary: 0,
            render_rows: 0,
            viewport: None,
            scroll: 0,
            selected: -1,
            focus: -1,
            last_scroll: 0,
            states: vec![],
            pending: None,
            sounds: vec![],
            sound_assets: vec![],
            input_pending: false,
            voices: vec![],
            voice_started: false,
            finished: false,
            active: false,
            pending_build: None,
            last_ticket: None,
            begin_ticket: None,
            refresh_ticket: None,
            scroll_changed: false,
        }
    }
}
impl NativePanel {
    pub(super) fn ui_waiting(&self) -> bool {
        self.active
            && !self.finished
            && self.pending.is_none()
            && self.sounds.is_empty()
            && self.sound_assets.is_empty()
            && self.voices.is_empty()
            && self.pending_build.is_none()
    }
    fn max_scroll(&self) -> i32 {
        self.declarations.rows.len() as i32 - self.declarations.rectangles.len() as i32
    }
    fn geometry(&self) -> [i16; 8] {
        self.declarations
            .current_config
            .as_ref()
            .or(self.declarations.configs.last())
            .map_or([0; 8], |c| c.geometry)
    }
}
fn next_ticket(services: &mut Services) -> Result<(u64, u8)> {
    services.panel_serial = services
        .panel_serial
        .checked_add(1)
        .context("Circus panel generation overflow")?;
    Ok((services.panel_serial, 255))
}
fn queue(values: &mut Vec<MenuValue>, bank: i16, index: i16, value: i32) {
    values.push(MenuValue { bank, index, value });
}
fn row_group(
    declarations: &PanelDeclarations,
    fixed: &Group,
    scroll: i32,
    selected: i32,
) -> Result<Group> {
    let mut group = Group::default();
    ensure!(
        declarations.rectangles.len() <= 224,
        "Circus panel visible row budget"
    );
    for (ordinal, bounds) in declarations.rectangles.iter().copied().enumerate() {
        let index = scroll.wrapping_add(ordinal as i32) as u32 as usize;
        let empty = crate::machine::text_ops::panel::PanelRow {
            slot_id: -1,
            ..Default::default()
        };
        let row = declarations.rows.get(index).unwrap_or(&empty);
        let mut button = Button {
            bounds,
            action: row.action.clone(),
            ..Default::default()
        };
        for part in declarations.overlays.iter().chain(&row.states) {
            let mut part = part.clone();
            part.position[0] += i32::from(bounds[0]);
            part.position[1] += i32::from(bounds[1]);
            button.images.push(part.clone());
            if part.state == 2 && selected == index as i32 {
                part.state = 99;
                button.images.push(part);
            }
        }
        if let Some(part) = &row.image {
            let mut part = part.clone();
            part.position[0] += i32::from(bounds[0]);
            part.position[1] += i32::from(bounds[1]);
            button.images.push(part);
        }
        button.texts = panel_texts(row, [bounds[0], bounds[1]])?;
        group.buttons.push(button);
    }
    group.append_records(fixed);
    Ok(group)
}
impl Bridge<'_> {
    fn replace_panel_menu(&mut self, group: &Group) -> Result<bool> {
        self.with_menu_scope(3, |bridge| {
            if let Some(menu) = &mut bridge.services.menu {
                menu.active = false;
            }
            bridge.begin_menu(group)
        })
    }
    pub(super) fn copy_ui_to_panel(&mut self, group: &Group) -> Result<bool> {
        //4359E0 clears1918 then418D90 copies only its native record lists.
        let mut fixed = Group::default();
        fixed.append_records(group);
        self.services.panel_fixed = fixed;
        Ok(true)
    }
    pub(super) fn finalize_panel(
        &mut self,
        declarations: &PanelDeclarations,
        scroll: i32,
        selected: i32,
        focus: i32,
        ticket: (u64, u8),
    ) -> Result<bool> {
        let mut panel = self.services.panel.take().unwrap_or_default();
        let result =
            self.finalize_panel_inner(&mut panel, declarations, scroll, selected, focus, ticket);
        self.services.panel = Some(panel);
        result
    }
    fn finalize_panel_inner(
        &mut self,
        panel: &mut NativePanel,
        declarations: &PanelDeclarations,
        scroll: i32,
        selected: i32,
        focus: i32,
        ticket: (u64, u8),
    ) -> Result<bool> {
        if panel.last_ticket == Some(ticket) {
            return Ok(true);
        }
        if panel.pending_build.is_none() {
            let configured = declarations.has_viewport();
            let mut scroll = if configured { scroll } else { panel.scroll };
            if configured
                && (declarations.rows.len() as u32).wrapping_add(1)
                    <= (declarations.rectangles.len() as i32).wrapping_add(scroll) as u32
            {
                scroll = declarations.rows.len() as i32 - declarations.rectangles.len() as i32;
            }
            let (group, boundary, render_rows, viewport) = if configured {
                let group = if declarations.rectangles.is_empty() {
                    Group::default()
                } else {
                    row_group(declarations, &self.services.panel_fixed, scroll, selected)?
                };
                let g = declarations
                    .current_config
                    .as_ref()
                    .or(declarations.configs.last())
                    .context("Circus configured panel geometry")?
                    .geometry;
                (
                    group,
                    declarations.rectangles.len(),
                    declarations.rectangles.len(),
                    Some([g[4], g[5], g[6], g[7]]),
                )
            } else {
                //437602..437611 appends1918 to1917. It neither clears the
                //old dynamic lists nor consumes current config/row builders.
                let mut group = panel.dynamic.clone();
                let boundary = group.buttons.len();
                group.append_records(&self.services.panel_fixed);
                (group, boundary, panel.render_rows, panel.viewport)
            };
            panel.pending_build = Some(Build {
                ticket,
                declarations: declarations.clone(),
                group,
                boundary,
                render_rows,
                viewport,
                scroll,
                selected,
                focus: if configured {
                    if focus == 255 { -1 } else { focus }
                } else {
                    panel.focus
                },
                last_scroll: if configured { -1 } else { panel.last_scroll },
            });
        }
        let build = panel.pending_build.as_ref().unwrap();
        ensure!(
            build.ticket == ticket,
            "Circus panel finalization owner changed during retry"
        );
        if !self.replace_panel_menu(&build.group)? {
            return Ok(false);
        }
        self.publish_panel_bounds(build)?;
        let build = panel.pending_build.take().unwrap();
        panel.declarations = build.declarations;
        panel.dynamic = build.group;
        panel.row_boundary = build.boundary;
        panel.render_rows = build.render_rows;
        panel.viewport = build.viewport;
        panel.scroll = build.scroll;
        panel.selected = build.selected;
        panel.focus = build.focus;
        panel.last_scroll = build.last_scroll;
        panel.states = vec![99; panel.render_rows];
        panel.pending = None;
        panel.sounds.clear();
        panel.sound_assets.clear();
        panel.input_pending = false;
        panel.voices.clear();
        panel.voice_started = false;
        panel.finished = false;
        panel.last_ticket = Some(ticket);
        self.submit(
            vec![],
            vec![json!({"op":"set_enabled","id":MENU_UI,"enabled":panel.active})],
        )?;
        if panel.active {
            self.with_menu_scope(3, |bridge| {
                if let Some(menu) = &mut bridge.services.menu {
                    menu.active = true;
                }
                Ok(())
            })?;
            if panel.focus >= 0 && (panel.focus as usize) < panel.dynamic.buttons.len() {
                self.with_menu_scope(3,|bridge|bridge.select_native_menu_cursor(panel.focus as usize))?;
            }
        }
        Ok(true)
    }
    fn publish_panel_bounds(&mut self, build: &Build) -> Result<()> {
        self.remove_panel_auxiliary()?;
        let mut layers = Vec::new();
        let mut ui = Vec::new();
        //435820 draws ordinary1917 records via416190. The61..64 values
        //select4375E0's declaration branch; they do not install a scissor or
        //replace each button's original image-dependent hit testing.
        if build.declarations.has_viewport() {
            let g = build
                .declarations
                .current_config
                .as_ref()
                .or(build.declarations.configs.last())
                .unwrap()
                .geometry;
            if g[2] >= g[0] && g[3] >= g[1] {
                for (id, y, height) in [
                    (SCROLL, g[1], i32::from(g[3]) - i32::from(g[1])),
                    (SCROLL_BOTTOM, g[3], 1),
                ] {
                    ui.push(json!({"op":"put","widget":{"id":id,"parent":MENU_UI,"rect":[g[0],y,i32::from(g[2])-i32::from(g[0])+1,height],"kind":"slider","slider_axis":"vertical","interaction_only":true,"keyboard_focusable":false,"keyboard_navigation":false,"keyboard_activation":false,"value":0.,"key_events":[{"buttons":buttons::MOUSE_LEFT,"edge":"release","event":"panel_scroll"},{"buttons":buttons::MOUSE_LEFT,"edge":"held","delay_ms":200,"event":"panel_scroll"}]}}));
                }
            }
            if let Some(part) = &build.declarations.thumb {
                let mut part = part.clone();
                let denominator = build.declarations.rows.len() as i32
                    - build.declarations.rectangles.len() as i32;
                let y = (f64::from(i32::from(g[3]) - i32::from(g[1])) / f64::from(denominator))
                    * f64::from(build.scroll)
                    + f64::from(g[1])
                    - f64::from(part.source[3] / 2);
                part.position = [
                    i32::from(g[0]) + (i32::from(g[2]) - i32::from(g[0])) / 2
                        - i32::from(part.source[2] / 2),
                    crate::machine::menu_ops::native_ftol(y),
                ];
                self.image_layer(&mut layers, THUMB, &part, 102)?;
                if let Some(command) = layers
                    .last_mut()
                    .filter(|command| command["layer"]["id"] == THUMB)
                {
                    command["layer"]["parent"] = json!(SCENE_ROOT);
                    command["layer"]["visible"] = json!(true);
                }
            }
        }
        self.submit(layers, ui)?;
        Ok(())
    }
    fn remove_panel_auxiliary(&mut self) -> Result<()> {
        let mut layers = Vec::new();
        let layer: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(THUMB))?,
        )?;
        if !layer.is_null() {
            layers.push(json!({"op":"remove","id":THUMB}));
        }
        let mut ui = Vec::new();
        for id in [ROW_UI_ROOT, SCROLL, SCROLL_BOTTOM, UI_ROOT] {
            let widget: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::UiWidget(id))?,
            )?;
            if !widget.is_null() {
                ui.push(json!({"op":"remove","id":id}));
            }
        }
        if !layers.is_empty() || !ui.is_empty() {
            self.submit(layers, ui)?;
        }
        Ok(())
    }
    pub(super) fn activate_panel(&mut self) -> Result<bool> {
        let panel = self
            .services
            .panel
            .as_mut()
            .context("Circus panel activation before finalization")?;
        if panel.pending_build.is_some() {
            return Ok(false);
        }
        panel.active = true;
        panel.finished = false;
        panel.begin_ticket = None;
        let focus = panel.focus;
        let count = panel.dynamic.buttons.len();
        self.with_menu_scope(3, |bridge| {
            if let Some(menu) = &mut bridge.services.menu {
                menu.active = true;
            }
            Ok(())
        })?;
        self.submit(
            vec![],
            vec![json!({"op":"set_enabled","id":MENU_UI,"enabled":true})],
        )?;
        if focus >= 0 && (focus as usize) < count {
            self.with_menu_scope(3,|bridge|bridge.select_native_menu_cursor(focus as usize))?;
        }
        Ok(true)
    }
    pub(super) fn begin_panel(
        &mut self,
        declarations: &PanelDeclarations,
        fixed: &Group,
        scroll: i32,
        selected: i32,
    ) -> Result<bool> {
        if self
            .services
            .panel
            .as_ref()
            .is_some_and(|panel| panel.active && !panel.finished)
        {
            return Ok(true);
        }
        let ticket = if let Some(ticket) = self
            .services
            .panel
            .as_ref()
            .and_then(|panel| panel.begin_ticket)
        {
            ticket
        } else {
            let ticket = next_ticket(self.services)?;
            self.services
                .panel
                .get_or_insert_with(Default::default)
                .begin_ticket = Some(ticket);
            ticket
        };
        self.copy_ui_to_panel(fixed)?;
        if !self.finalize_panel(declarations, scroll, selected, -1, ticket)? {
            return Ok(false);
        }
        self.activate_panel()
    }
    pub(super) fn refresh_panel_template(
        &mut self,
        declarations: &PanelDeclarations,
    ) -> Result<bool> {
        if let Some(panel) = self.services.panel.as_ref() {
            let (scroll, selected, focus) = (panel.scroll, panel.selected, panel.focus);
            let ticket = if let Some(ticket) = panel.refresh_ticket {
                ticket
            } else {
                let mut ticket = next_ticket(self.services)?;
                ticket.1 = 254;
                self.services.panel.as_mut().unwrap().refresh_ticket = Some(ticket);
                ticket
            };
            if !self.finalize_panel(declarations, scroll, selected, focus, ticket)? {
                return Ok(false);
            }
            self.services.panel.as_mut().unwrap().refresh_ticket = None;
        }
        Ok(true)
    }
    pub(super) fn clear_panel(&mut self) -> Result<()> {
        self.remove_panel_auxiliary()?;
        self.with_menu_scope(3, |bridge| bridge.clear_menu())?;
        self.services.panel = None;
        self.services.panel_fixed = Group::default();
        Ok(())
    }
    pub(super) fn panel_process_pending(&self) -> bool {
        self.services.panel.as_ref().is_some_and(|panel| {
            panel.pending_build.is_some()
                || panel.pending.is_some()
                || !panel.sounds.is_empty()
                || !panel.sound_assets.is_empty()
                || !panel.voices.is_empty()
        })
    }
    fn panel_values(&mut self, values: Vec<MenuValue>) -> Result<()> {
        self.with_menu_scope(3, |bridge| {
            if let Some(menu) = &mut bridge.services.menu {
                menu.pending_values.extend(values);
            }
            Ok(())
        })
    }
    pub(super) fn panel_process_action(&mut self) -> Result<Option<String>> {
        if self.services.panel.is_none() {
            return Ok(None);
        }
        if !self.services.panel.as_ref().unwrap().active {
            self.activate_panel()?;
        }
        Ok(self.poll_panel_only()?.and_then(|reply| reply.action))
    }
    pub(super) fn poll_panel(&mut self) -> Result<Option<PanelReply>> {
        let reply = self.poll_panel_only()?;
        if self.panel_process_pending() {
            return Ok(None);
        }
        if let Some(reply) = reply {
            if reply.action.is_some() || reply.slot_id.is_some() {
                return Ok(Some(reply));
            }
            if let Some(action) = self.process_menu_fallback()? {
                return Ok(Some(PanelReply {
                    action: Some(action),
                    ..reply
                }));
            }
            return Ok(Some(reply));
        }
        Ok(None)
    }
    fn poll_panel_only(&mut self) -> Result<Option<PanelReply>> {
        let mut panel = self
            .services
            .panel
            .take()
            .context("Circus panel poll without retained owner")?;
        let result = self.poll_panel_inner(&mut panel);
        self.services.panel = Some(panel);
        result
    }
    fn complete_panel_scroll_build(
        &mut self,
        panel: &mut NativePanel,
    ) -> Result<Option<PanelReply>> {
        let build = panel
            .pending_build
            .as_ref()
            .context("Circus missing scroll rebuild owner")?
            .clone();
        if build.ticket.1 != 253 {
            return Ok(None);
        }
        let previous = panel.last_scroll;
        let sounds = std::mem::take(&mut panel.sounds);
        let sound_assets = std::mem::take(&mut panel.sound_assets);
        let voices = std::mem::take(&mut panel.voices);
        let ready = self.finalize_panel_inner(
            panel,
            &build.declarations,
            build.scroll,
            build.selected,
            build.focus,
            build.ticket,
        )?;
        panel.sounds = sounds;
        panel.sound_assets = sound_assets;
        panel.voices = voices;
        if !ready {
            return Ok(None);
        }
        panel.scroll_changed = previous != -1;
        panel.input_pending =
            !panel.sounds.is_empty() || !panel.sound_assets.is_empty() || !panel.voices.is_empty();
        if panel.focus >= 0 && (panel.focus as usize) < panel.dynamic.buttons.len() {
            self.with_menu_scope(3,|bridge|bridge.select_native_menu_cursor(panel.focus as usize))?;
        }
        self.panel_values(vec![
            MenuValue {
                bank: 99,
                index: 245,
                value: panel.focus,
            },
            MenuValue {
                bank: 99,
                index: 246,
                value: panel.scroll,
            },
        ])?;
        if panel.input_pending {
            return Ok(None);
        }
        Ok(Some(PanelReply {
            scroll_changed: previous != -1,
            scroll: panel.scroll,
            focus: panel.focus,
            slot_id: None,
            action: None,
        }))
    }
    pub(super) fn take_panel_scroll_changed(&mut self) -> bool {
        self.services
            .panel
            .as_mut()
            .is_some_and(|panel| std::mem::take(&mut panel.scroll_changed))
    }
    fn poll_panel_inner(&mut self, panel: &mut NativePanel) -> Result<Option<PanelReply>> {
        if panel.pending_build.is_some() {
            return self.complete_panel_scroll_build(panel);
        }
        if panel.finished {
            return Ok(None);
        }
        let mut values = Vec::new();
        if panel.pending.is_none() && !panel.input_pending {
            let events: Vec<Value> = self.with_menu_scope(3, |bridge| bridge.menu_events())?;
            let key = |names: &[&str]| {
                events.iter().any(|event| {
                    event["kind"]
                        .as_str()
                        .is_some_and(|kind| names.contains(&kind))
                })
            };
            let old_scroll = panel.scroll;
            let old_focus = panel.focus;
            let step = i32::from(panel.declarations.step);
            let mut scrollbar = false;
            if key(&["panel_scroll"]) {
                for id in [SCROLL, SCROLL_BOTTOM] {
                    let interaction: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiInteraction(id))?,
                    )?;
                    let Some(pointer) = interaction["pointer"].as_array() else {
                        continue;
                    };
                    let g = panel.geometry();
                    let x = pointer[0].as_f64().unwrap_or(f64::NAN);
                    let y = pointer[1].as_f64().unwrap_or(f64::NAN);
                    if x >= f64::from(g[0])
                        && x <= f64::from(g[2])
                        && y >= f64::from(g[1])
                        && y <= f64::from(g[3])
                    {
                        let value = crate::machine::menu_ops::native_ftol(
                            (y - f64::from(g[1]))
                                / (f64::from(i32::from(g[3]) - i32::from(g[1]))
                                    / (f64::from(panel.max_scroll()) + 1.)),
                        );
                        panel.scroll = value.min(panel.max_scroll()).max(0);
                        scrollbar = true;
                    }
                }
                queue(&mut values, 99, 245, -1);
                panel.focus = -1;
            }
            if key(&["panel_page_down"]) {
                panel.scroll = panel.scroll.wrapping_add(step);
                if (panel.declarations.rows.len() as u32).wrapping_add(1)
                    <= (panel.declarations.rectangles.len() as i32).wrapping_add(panel.scroll)
                        as u32
                {
                    panel.scroll = panel.max_scroll();
                    panel.last_scroll = -2;
                }
                panel.focus = -1;
                queue(&mut values, 99, 245, -1);
            }
            if key(&["panel_page_up"]) {
                panel.scroll = panel.scroll.wrapping_sub(step);
                if panel.scroll <= 0 {
                    panel.scroll = 0;
                    panel.last_scroll = -2;
                }
                panel.focus = -1;
                queue(&mut values, 99, 245, -1);
            }
            let action = if scrollbar {
                None
            } else {
                self.with_menu_scope(3, |bridge| bridge.tick_menu_events(&events))?
            };
            if !scrollbar {
                panel.focus=self.with_menu_scope(3,|bridge|Ok(bridge.native_menu_cursor()))?
                    .map_or(-1,|ordinal|ordinal as i32);
                if panel.focus == panel.row_boundary as i32
                    && key(&["menu_next", "menu_right", "menu_right_held"])
                {
                    panel.scroll = panel.scroll.wrapping_add(step);
                    if (panel.declarations.rows.len() as u32)
                        > (panel.declarations.rectangles.len() as i32).wrapping_add(panel.scroll)
                            as u32
                    {
                        //436DB0:4B32C0 is the immutable native "se\\200".
                        panel.sound_assets.push("se\\200".into());
                        if panel.scroll != panel.last_scroll {
                            panel.focus = old_focus;
                            queue(&mut values, 99, 245, old_focus);
                        }
                    } else {
                        panel.scroll = panel.max_scroll();
                    }
                } else if old_focus == 0 && key(&["menu_previous", "menu_left", "menu_left_held"]) {
                    panel.scroll = panel.scroll.wrapping_sub(step);
                    if panel.scroll > 0 {
                        panel.sound_assets.push("se\\200".into());
                        panel.focus = 0;
                        queue(&mut values, 99, 245, 0);
                    }
                    if panel.scroll < 0 {
                        panel.scroll = 0;
                    }
                }
            }
            for ordinal in 0..panel.render_rows {
                let state=self.with_menu_scope(3,|bridge|Ok(bridge.native_menu_control_state(ordinal)))?;
                if panel.states.get(ordinal).copied().unwrap_or(99) == 99 && state != 99 {
                    if let Some(config) = panel.declarations.configs.first() {
                        if config.sounds[1] != 0 {
                            panel.sounds.push(config.sounds[1]);
                        }
                        if !config.voices[1].is_empty() {
                            panel.voices.push(config.voices[1].clone());
                        }
                    }
                }
                if let Some(old) = panel.states.get_mut(ordinal) {
                    *old = state;
                }
            }
            if action.is_none() && panel.scroll != panel.last_scroll {
                let group = if panel.declarations.rectangles.is_empty() {
                    Group::default()
                } else {
                    row_group(
                        &panel.declarations,
                        &self.services.panel_fixed,
                        panel.scroll,
                        panel.selected,
                    )?
                };
                let mut ticket = next_ticket(self.services)?;
                ticket.1 = 253;
                panel.pending_build = Some(Build {
                    ticket,
                    declarations: panel.declarations.clone(),
                    group,
                    boundary: panel.declarations.rectangles.len(),
                    render_rows: panel.declarations.rectangles.len(),
                    viewport: panel.viewport,
                    scroll: panel.scroll,
                    selected: panel.selected,
                    focus: panel.focus,
                    last_scroll: panel.scroll,
                });
                return self.complete_panel_scroll_build(panel);
            }
            if let Some(action) = action {
                let ordinal = self.with_menu_scope(3, |bridge| {
                    Ok(bridge
                        .services
                        .menu
                        .as_ref()
                        .and_then(|menu| menu.activation_ordinal))
                })?;
                let mut slot_id = None;
                let valid = if let Some(ordinal) =
                    ordinal.filter(|ordinal| *ordinal < panel.row_boundary)
                {
                    let index = panel.scroll.wrapping_add(ordinal as i32) as u32 as usize;
                    if let Some(row) = panel
                        .declarations
                        .rows
                        .get(index)
                        .filter(|row| row.slot_id != -1)
                    {
                        slot_id = Some(row.slot_id);
                        true
                    } else {
                        false
                    }
                } else {
                    true
                };
                if valid {
                    if ordinal.is_some_and(|ordinal| ordinal < panel.render_rows) {
                        if let Some(config) = panel.declarations.configs.first() {
                            if config.sounds[0] != 0 {
                                panel.sounds.push(config.sounds[0]);
                            }
                            if !config.voices[0].is_empty() {
                                panel.voices.push(config.voices[0].clone());
                            }
                        }
                    }
                    if let Some(slot_id) = slot_id {
                        queue(&mut values, 99, 254, slot_id);
                    }
                    queue(&mut values, 9, 60, panel.scroll);
                    panel.pending = Some(PanelReply {
                        scroll_changed: false,
                        scroll: panel.scroll,
                        focus: panel.focus,
                        slot_id,
                        action: Some(action),
                    });
                } else {
                    self.with_menu_scope(3, |bridge| {
                        if let Some(menu) = &mut bridge.services.menu {
                            menu.active = true;
                        }
                        Ok(())
                    })?;
                    self.submit(
                        vec![],
                        vec![json!({"op":"set_enabled","id":MENU_UI,"enabled":true})],
                    )?;
                }
            }
            queue(&mut values, 99, 245, panel.focus);
            if panel.scroll != old_scroll {
                queue(&mut values, 99, 246, panel.scroll);
            }
            self.panel_values(values)?;
            panel.input_pending = true;
        }
        if let Some(name) = panel.sound_assets.first() {
            if !self.play_audio(2, name, false, false, 0, None)? {
                return Ok(None);
            }
            panel.sound_assets.remove(0);
            if !panel.sound_assets.is_empty() {
                return Ok(None);
            }
        }
        if let Some(&sound) = panel.sounds.first() {
            let Some(name) = self.sound_name(sound)? else {
                return Ok(None);
            };
            if !self.play_audio(2, &name, false, false, 0, None)? {
                return Ok(None);
            }
            panel.sounds.remove(0);
            if !panel.sounds.is_empty() {
                return Ok(None);
            }
        }
        if let Some(name) = panel.voices.first() {
            if !panel.voice_started {
                ensure!(
                    NativeHost::emit(self, &Event::StopAudio { channel: 8 })?,
                    "Circus panel voice stop"
                );
                panel.voice_started = true;
            }
            if !self.play_audio(8, name, true, false, 0, None)? {
                return Ok(None);
            }
            panel.voices.remove(0);
            panel.voice_started = false;
            if !panel.voices.is_empty() {
                return Ok(None);
            }
        }
        panel.input_pending = false;
        if let Some(reply) = panel.pending.take() {
            panel.finished = true;
            panel.active = false;
            return Ok(Some(reply));
        }
        Ok(Some(PanelReply {
            scroll_changed: false,
            scroll: panel.scroll,
            focus: panel.focus,
            slot_id: None,
            action: None,
        }))
    }
}
fn panel_texts(
    row: &crate::machine::text_ops::panel::PanelRow,
    origin: [i16; 2],
) -> Result<Vec<TextPart>> {
    let mut parts = vec![];
    for text in &row.rendered_texts()? {
        for (&state, (color, value)) in &text.variants {
            let position = if text.absolute {
                text.position
            } else {
                [
                    origin[0]
                        .checked_add(
                            text.position[0]
                                .checked_mul(2)
                                .context("Circus panel text x scale")?,
                        )
                        .context("Circus panel text x")?,
                    origin[1]
                        .checked_add(
                            text.position[1]
                                .checked_mul(2)
                                .context("Circus panel text y scale")?,
                        )
                        .context("Circus panel text y")?,
                ]
            };
            parts.push(TextPart {
                position,
                text: value.clone(),
                color: *color,
                size: 22,
                line_pitch: None,
                state,
            });
        }
    }
    ensure!(parts.len() <= 128, "Circus panel text layer namespace");
    Ok(parts)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::text_ops::panel::{PanelRow, PanelText};
    #[test]
    fn native_relative_text_doubles_offsets_but_absolute_does_not() {
        let row = PanelRow {
            texts: vec![
                PanelText {
                    position: [3, 4],
                    absolute: false,
                    variants: [(99, (7, "relative".into()))].into(),
                },
                PanelText {
                    position: [3, 4],
                    absolute: true,
                    variants: [(2, (129, "absolute".into()))].into(),
                },
            ],
            ..Default::default()
        };
        let parts = panel_texts(&row, [10, 20]).unwrap();
        assert_eq!(parts[0].position, [16, 28]);
        assert_eq!(parts[1].position, [3, 4]);
        assert_eq!(parts[1].state, 2);
    }
}
