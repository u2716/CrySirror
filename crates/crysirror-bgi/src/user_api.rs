//! UserB0 lifecycle and native byte/choice policy over retained engine UI.
use super::*;
use crysirror_core::service::EngineQuery;
const EDIT_ID: u32 = 0xb000_0020;
const DEFAULT_CAPTION: &str =
    "Ethornell - BURIKO General Interpreter ( Version : 1.622 - Compatibility : 1.72 )";
#[derive(Default)]
pub(crate) struct UserState {
    pub(super) caption: Option<String>,
    pub(super) dialog: Option<(u64, u32)>,
    pub(super) next_dialog: u32,
    cursor_object: Option<(i32, i32, i32)>,
    cursor_visible: bool,
    cursor_initialized: bool,
    idle_timeout: u64,
    idle_deadline: u64,
    last_pointer: Option<[i32; 2]>,
    text_action_serial: u32,
    edit: EditState,
}
struct EditState {
    active: bool,
    visible: bool,
    focused: bool,
    rect: [i32; 4],
    font: i32,
    size: i32,
    max_chars: usize,
    scale: i32,
    color: u32,
    text: String,
    hide_on_enter: bool,
    printable: bool,
}
impl Default for EditState {
    fn default() -> Self {
        Self {
            active: false,
            visible: false,
            focused: false,
            rect: [0; 4],
            font: 0,
            size: 16,
            max_chars: 255,
            scale: 100,
            color: 0,
            text: String::new(),
            hide_on_enter: false,
            printable: true,
        }
    }
}
pub(crate) fn handles(group: u8, id: u16) -> bool {
    group == 0xb0 && matches!(id,0x02..=0x06|0x20..=0x29|0x80..=0x83)
}
pub(super) fn truncate_sjis(text: &str, limit: usize) -> String {
    let mut result = String::new();
    let mut bytes = 0;
    for ch in text.chars() {
        let size = encoding_rs::SHIFT_JIS.encode(&ch.to_string()).0.len();
        if bytes + size > limit {
            break;
        }
        bytes += size;
        result.push(ch);
    }
    result
}
impl BgiVm {
    fn publish_edit(&self, host: &mut dyn HostServices) -> Result<()> {
        let e = &self.user.edit;
        if !e.active {
            return Ok(());
        }
        let font = self.font_alias(host, e.font)?;
        self.submit(host,json!({"ui":[{"op":"put","widget":{"id":EDIT_ID,"rect":e.rect,"kind":"text_field","z":i32::MAX-1024,"visible":e.visible,"text":e.text,"font":font,"text_size":e.size,"max_length":e.max_chars.max(e.text.chars().count()),"color":[255,255,255,255],"text_color":[(e.color>>16)as u8,(e.color>>8)as u8,e.color as u8,255]}}]}))?;
        if e.focused {
            self.submit(host, json!({"ui":[{"op":"focus","id":EDIT_ID}]}))?;
        }
        Ok(())
    }
    pub(crate) fn tick_user(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let pointer = [self.input.pointer_x, self.input.pointer_y];
        if !self.user.cursor_initialized {
            self.user.cursor_visible = true;
            self.user.cursor_initialized = true;
        }
        let moved = self.user.last_pointer != Some(pointer);
        self.user.last_pointer = Some(pointer);
        if self.user.idle_timeout != 0 && moved {
            self.user.idle_deadline = now.saturating_add(self.user.idle_timeout);
        }
        let visible = self.user.idle_timeout == 0 || now < self.user.idle_deadline;
        let mut graph_changed = false;
        if let Some((object, x, y)) = self.user.cursor_object {
            if moved {
                self.graph.position(
                    object,
                    pointer[0].wrapping_add(x),
                    pointer[1].wrapping_add(y),
                );
                graph_changed = true;
            }
            if self.user.cursor_visible != visible {
                self.graph.tree.set_enabled(object, visible);
                self.graph.objects.entry(object).or_default().native.enabled = u32::from(visible);
                graph_changed = true;
            }
            if self.user.cursor_visible != visible || moved {
                self.submit(
                    host,
                    json!({"ui":[{"op":"set_cursor_visible","visible":false}]}),
                )?;
            }
        } else if self.user.cursor_visible != visible {
            self.submit(
                host,
                json!({"ui":[{"op":"set_cursor_visible","visible":visible}]}),
            )?;
        }
        self.user.cursor_visible = visible;
        if graph_changed {
            self.publish_graph(host)?;
        }
        if self.user.edit.active {
            let widget: Value = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::UiWidget(EDIT_ID))?,
            )?;
            let candidate = widget
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or(&self.user.edit.text);
            let before = &self.user.edit.text;
            let accepted = if (!self.user.edit.printable
                || before.chars().count() >= self.user.edit.max_chars)
                && candidate.chars().count() > before.chars().count()
            {
                before.clone()
            } else {
                truncate_sjis(candidate, 255)
            };
            let accepted = if accepted.chars().count() > self.user.edit.max_chars
                && accepted.chars().count() > before.chars().count()
            {
                before.clone()
            } else {
                accepted
            };
            if accepted != candidate {
                self.submit(
                    host,
                    json!({"ui":[{"op":"set_text","id":EDIT_ID,"text":accepted}]}),
                )?;
            }
            self.user.edit.text = accepted;
            let interaction: Value = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::UiInteraction(EDIT_ID))?,
            )?;
            self.user.edit.focused = interaction
                .get("focused")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let action: Value = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::UiTextAction(EDIT_ID))?,
            )?;
            let serial = action.get("serial").and_then(Value::as_u64).unwrap_or(0) as u32;
            if serial != 0 && serial != self.user.text_action_serial {
                self.user.text_action_serial = serial;
                let kind = action.get("kind").and_then(Value::as_str).unwrap_or("");
                if matches!(kind, "submit" | "tab") {
                    self.user.edit.focused = false;
                    self.submit(host, json!({"ui":[{"op":"blur","id":EDIT_ID}]}))?;
                    if self.user.edit.hide_on_enter || kind == "tab" {
                        self.user.edit.visible = false;
                        self.submit(
                            host,
                            json!({"ui":[{"op":"set_visible","id":EDIT_ID,"visible":false}]}),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn user_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
        request: u64,
    ) -> Result<bool> {
        let a = &call.args;
        let mut results = Vec::new();
        let mut writes = Vec::new();
        match call.id {
            0x02 | 0x03 => {
                if self.system.fullscreen {
                    results.push(0);
                } else {
                    let size = host
                        .window_geometry(self.client)?
                        .context("BGI window geometry is unavailable")?
                        .inner_size;
                    let [w, h] = [i32::try_from(size[0])?, i32::try_from(size[1])?];
                    let [screen_w, screen_h] =
                        [self.graph.screen[0] as i32, self.graph.screen[1] as i32];
                    let (point, valid) = if call.id == 0x02 {
                        ([(screen_w - w).max(0) / 2, (screen_h - h).max(0) / 2], true)
                    } else {
                        (
                            [a[0], a[1]],
                            a[0] >= 0
                                && a[1] >= 0
                                && a[0].saturating_add(w) <= screen_w
                                && a[1].saturating_add(h) <= screen_h,
                        )
                    };
                    if valid {
                        self.submit(
                            host,
                            json!({"ui":[{"op":"set_window_position","position":point}]}),
                        )?;
                    }
                    results.push(i32::from(valid));
                }
            }
            0x04 => {
                let object = a[0];
                if object != 0 {
                    self.graph.require(object)?;
                    self.user.cursor_object = Some((object, a[1], a[2]));
                    self.graph.position(
                        object,
                        self.input.pointer_x.wrapping_add(a[1]),
                        self.input.pointer_y.wrapping_add(a[2]),
                    );
                    self.publish_graph(host)?;
                } else {
                    self.user.cursor_object = None;
                }
                self.submit(host,json!({"ui":[{"op":"set_cursor_visible","visible":object==0&&self.user.cursor_visible}]}))?;
            }
            0x05 => {
                let was = self.user.idle_timeout != 0;
                self.user.idle_timeout = u64::try_from(a[0]).unwrap_or_default();
                self.user.idle_deadline = host.now_ms().saturating_add(self.user.idle_timeout);
                if !was && self.user.idle_timeout != 0 {
                    self.user.cursor_visible = true;
                }
                if was && self.user.idle_timeout == 0 {
                    self.user.cursor_visible = true;
                    self.submit(host,json!({"ui":[{"op":"set_cursor_visible","visible":self.user.cursor_object.is_none()}]}))?;
                }
            }
            0x06 => results.push(i32::from(self.user.cursor_visible)),
            0x20 => {
                ensure!(
                    a[2] >= 8
                        && a[3] >= 8
                        && a[2] as u32 <= self.graph.screen[0]
                        && a[3] as u32 <= self.graph.screen[1],
                    "BGI edit control size"
                );
                ensure!(
                    (8..=64).contains(&a[5]) && (1..=256).contains(&a[6]),
                    "BGI edit font size/character limit"
                );
                let _ = self.font_alias(host, a[4])?;
                let e = &mut self.user.edit;
                e.active = true;
                e.visible = true;
                e.focused = a[7] != 0;
                e.rect = [a[0], a[1], a[2], a[3]];
                e.font = a[4];
                e.size = a[5];
                e.max_chars = a[6] as usize;
                self.publish_edit(host)?;
            }
            0x21 => {
                if self.user.edit.active {
                    self.submit(host, json!({"ui":[{"op":"remove","id":EDIT_ID}]}))?;
                }
                self.user.edit.active = false;
                self.user.edit.visible = false;
                self.user.edit.focused = false;
                results.push(0);
            }
            0x22 => {
                let accepted = (25..=200).contains(&a[0]);
                if accepted {
                    self.user.edit.scale = a[0];
                }
                results.push(i32::from(accepted));
            }
            0x23 => results.push(i32::from(self.user.edit.active && self.user.edit.visible)),
            0x24 => {
                if self.user.edit.active {
                    self.user.edit.visible = a[0] != 0;
                    self.submit(
                        host,
                        json!({"ui":[{"op":"set_visible","id":EDIT_ID,"visible":a[0]!=0}]}),
                    )?;
                }
            }
            0x25 => {
                let color = a[0] as u32;
                self.user.edit.color =
                    ((color & 0xff) << 16) | (color & 0xff00) | ((color >> 16) & 0xff);
                self.publish_edit(host)?;
            }
            0x26 => {
                self.user.edit.text = truncate_sjis(&vm.string(a[0])?, 255);
                self.publish_edit(host)?;
            }
            0x27 => {
                self.tick_user(host)?;
                let mut bytes = encoding_rs::SHIFT_JIS
                    .encode(&self.user.edit.text)
                    .0
                    .into_owned();
                let count = bytes.len();
                bytes.push(0);
                writes.push(crysirror_bgi_vm::HostMemoryWrite {
                    address: a[0] as u32,
                    bytes,
                });
                results.push(count as i32);
            }
            0x28 => self.user.edit.hide_on_enter = a[0] != 0,
            0x29 => self.user.edit.printable = a[0] != 0,
            0x83 => {
                self.user.caption = if a[0] == 0 {
                    None
                } else {
                    Some(vm.string(a[0])?)
                }
            }
            0x80..=0x82 => {
                if let Some((owner, _)) = self.user.dialog {
                    if owner != request {
                        return Ok(false);
                    }
                }
                if self.user.dialog.is_none() {
                    let message = vm.string(a[0])?;
                    let title = self.user.caption.as_deref().unwrap_or(DEFAULT_CAPTION);
                    self.user.next_dialog = (self.user.next_dialog % 65535) + 1;
                    let id = self.user.next_dialog;
                    let yes_no = call.id == 0x81;
                    let ok_only = call.id == 0x80;
                    let first = if ok_only {
                        true
                    } else {
                        a[if call.id == 0x81 { 1 } else { 2 }] != 0
                    };
                    let buttons = if ok_only {
                        vec![json!({"id":1,"label":"OK"})]
                    } else if yes_no || a[1] != 1 {
                        vec![json!({"id":1,"label":"Yes"}), json!({"id":0,"label":"No"})]
                    } else {
                        vec![
                            json!({"id":1,"label":"OK"}),
                            json!({"id":0,"label":"Cancel"}),
                        ]
                    };
                    let font = self
                        .config
                        .font
                        .as_deref()
                        .context("BGI native dialog requires an engine font alias")?;
                    self.submit(host,json!({"ui":[{"op":"open_dialog","dialog":{"id":id,"title":title,"message":message,"buttons":buttons,"default":if first{1}else{0},"cancel":if ok_only{1}else{0},"font":font,"text_size":24}}]}))?;
                    self.user.dialog = Some((request, id));
                    return Ok(false);
                }
                let (_, id) = self.user.dialog.unwrap();
                let events: Vec<Value> = serde_json::from_slice(
                    &host.query_engine(self.client, EngineQuery::DialogEvents)?,
                )?;
                let Some(event) = events.iter().find(|event| {
                    event
                        .get("widget")
                        .and_then(|w| w.get("id"))
                        .and_then(Value::as_u64)
                        == Some(u64::from(id))
                }) else {
                    return Ok(false);
                };
                let selected = event.get("value").and_then(Value::as_f64).unwrap_or(0.) != 0.;
                self.user.dialog = None;
                if !ok_only(call.id) {
                    results.push(i32::from(selected));
                }
            }
            _ => bail!(
                "BGI UserB0 selector {:02x} requires native semantic mapping",
                call.id
            ),
        }
        ensure!(results.len() == call.outputs, "BGI UserB0 result count");
        vm.commit_call_writes(&results, writes)?;
        self.calls += 1;
        Ok(true)
    }
}
fn ok_only(id: u16) -> bool {
    id == 0x80
}
