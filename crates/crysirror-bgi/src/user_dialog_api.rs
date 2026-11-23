//! Parsed native input forms rendered by shared engine widgets; the VM owns
//! their byte limits, destination addresses and accepted/cancelled writes.
use super::*;
use crysirror_bgi_vm::{UserDialogRequest as Request, UserDialogResponse as Response};
use crysirror_core::service::EngineQuery;
const ROOT: u32 = 0xbb00_0000;
const PANEL: u32 = ROOT + 1;
const OK: u32 = ROOT + 2;
const CANCEL: u32 = ROOT + 3;
const PREVIOUS: u32 = ROOT + 4;
const NEXT: u32 = ROOT + 5;
const FIELD: u32 = ROOT + 64;
const OPTION: u32 = ROOT + 1024;
#[derive(Clone)]
struct Field {
    label: String,
    text: String,
    limit: usize,
    numeric: bool,
}
pub(crate) struct DialogWork {
    request: u64,
    dialog: Request,
    fields: Vec<Field>,
    page: usize,
    rows: usize,
    options: Vec<String>,
}
impl DialogWork {
    fn new(request: u64, dialog: Request, screen: [u32; 2]) -> Result<Self> {
        let (mut fields, mut options) = (Vec::new(), Vec::new());
        match &dialog {
            Request::Input {
                initial,
                max_bytes,
                numeric,
                ..
            } => fields.push(Field {
                label: String::new(),
                text: initial.clone(),
                limit: *max_bytes,
                numeric: *numeric,
            }),
            Request::TwoField {
                first_label,
                first_initial,
                first_max_bytes,
                first_numeric,
                second_label,
                second_initial,
                second_max_bytes,
                second_numeric,
                ..
            } => {
                fields.push(Field {
                    label: first_label.clone(),
                    text: first_initial.clone(),
                    limit: *first_max_bytes,
                    numeric: *first_numeric,
                });
                fields.push(Field {
                    label: second_label.clone(),
                    text: second_initial.clone(),
                    limit: *second_max_bytes,
                    numeric: *second_numeric,
                });
            }
            Request::Segmented {
                segment_count,
                max_bytes_per_segment,
                numeric,
                ..
            } => {
                ensure!(*segment_count <= 32, "BGI segmented input field limit");
                for index in 0..*segment_count {
                    fields.push(Field {
                        label: (index + 1).to_string(),
                        text: String::new(),
                        limit: *max_bytes_per_segment,
                        numeric: *numeric,
                    });
                }
            }
            Request::Selection {
                options: values, ..
            } => {
                ensure!(values.len() <= 4096, "BGI native dialog selection count");
                options = values.clone();
            }
            Request::DateFields {
                fields: values,
                month_index,
                day_index,
            } => {
                for (index, text) in values.iter().enumerate() {
                    fields.push(Field {
                        label: (index + 1).to_string(),
                        text: text.clone(),
                        limit: 9,
                        numeric: false,
                    });
                }
                fields.push(Field {
                    label: "Month".into(),
                    text: if *month_index < 0 {
                        String::new()
                    } else {
                        (month_index + 1).to_string()
                    },
                    limit: 2,
                    numeric: true,
                });
                fields.push(Field {
                    label: "Day".into(),
                    text: if *day_index < 0 {
                        String::new()
                    } else {
                        (day_index + 1).to_string()
                    },
                    limit: 2,
                    numeric: true,
                });
            }
        }
        let rows = ((screen[1].saturating_sub(220)) / 42).max(1) as usize;
        ensure!(
            fields.len() <= rows.max(6),
            "BGI input form exceeds available engine viewport"
        );
        for field in &mut fields {
            field.text = super::user_api::truncate_sjis(&field.text, field.limit);
        }
        Ok(Self {
            request,
            dialog,
            fields,
            page: 0,
            rows,
            options,
        })
    }
    fn title(&self) -> &str {
        match &self.dialog {
            Request::Input { title, .. }
            | Request::TwoField { title, .. }
            | Request::Segmented { title, .. }
            | Request::Selection { title, .. } => title,
            Request::DateFields { .. } => "Date",
        }
    }
    fn prompt(&self) -> &str {
        match &self.dialog {
            Request::Segmented { prompt, .. } | Request::Selection { prompt, .. } => prompt,
            _ => "",
        }
    }
    fn response(&self) -> Option<Response> {
        Some(match &self.dialog {
            Request::Input { .. } => Response::Input(self.fields[0].text.clone()),
            Request::TwoField { .. } => {
                Response::TwoField([self.fields[0].text.clone(), self.fields[1].text.clone()])
            }
            Request::Segmented { .. } => Response::Segmented(
                self.fields
                    .iter()
                    .map(|f| f.text.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            Request::Selection { .. } => return None,
            Request::DateFields { .. } => {
                let month = self.fields[4]
                    .text
                    .parse::<i32>()
                    .ok()
                    .filter(|n| (1..=12).contains(n))
                    .map(|n| n - 1)
                    .unwrap_or(-1);
                let day = self.fields[5]
                    .text
                    .parse::<i32>()
                    .ok()
                    .filter(|n| (1..=31).contains(n))
                    .map(|n| n - 1)
                    .unwrap_or(-1);
                Response::DateFields {
                    fields: std::array::from_fn(|index| self.fields[index].text.clone()),
                    month_index: month,
                    day_index: day,
                }
            }
        })
    }
}
impl BgiVm {
    fn present_user_form(&self, host: &mut dyn HostServices, work: &DialogWork) -> Result<()> {
        let font = self
            .config
            .font
            .as_deref()
            .context("BGI input form requires engine font")?;
        let screen = self.graph.screen;
        let w = screen[0].saturating_sub(64).min(720).max(32);
        let h = screen[1].saturating_sub(64).max(32);
        let x = (screen[0] - w) / 2;
        let y = (screen[1] - h) / 2;
        let mut commands = vec![
            json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,screen[0],screen[1]],"color":[0,0,0,160],"z":2147479000,"modal":true,"keyboard_focusable":false}}),
            json!({"op":"put","widget":{"id":PANEL,"parent":ROOT,"rect":[x,y,w,h],"color":[36,36,44,255],"keyboard_focusable":false}}),
            json!({"op":"put","widget":{"id":ROOT+8,"parent":PANEL,"kind":"label","rect":[16,12,w.saturating_sub(32),36],"font":font,"text_size":24,"text":work.title()}}),
            json!({"op":"put","widget":{"id":ROOT+9,"parent":PANEL,"kind":"label","rect":[16,50,w.saturating_sub(32),40],"font":font,"text":work.prompt()}}),
        ];
        if matches!(&work.dialog, Request::Selection { .. }) {
            for (index, text) in work
                .options
                .iter()
                .enumerate()
                .skip(work.page * work.rows)
                .take(work.rows)
            {
                commands.push(json!({"op":"put","widget":{"id":OPTION+index as u32,"parent":PANEL,"kind":"button","rect":[16,96+(index%work.rows)*42,w.saturating_sub(32),36],"font":font,"text":text}}));
            }
            if work.page > 0 {
                commands.push(json!({"op":"put","widget":{"id":PREVIOUS,"parent":PANEL,"kind":"button","rect":[16,h.saturating_sub(44),120,32],"font":font,"text":"Previous"}}));
            }
            if (work.page + 1) * work.rows < work.options.len() {
                commands.push(json!({"op":"put","widget":{"id":NEXT,"parent":PANEL,"kind":"button","rect":[144,h.saturating_sub(44),120,32],"font":font,"text":"Next"}}));
            }
        } else {
            for (index, field) in work.fields.iter().enumerate() {
                let y = 96 + index * 54;
                commands.push(json!({"op":"put","widget":{"id":FIELD+index as u32,"parent":PANEL,"kind":"text_field","rect":[16,y,w.saturating_sub(32),42],"font":font,"text":field.text,"max_length":field.limit.min(65536),"text_color":[10,10,10,255],"color":[250,250,250,255]}}));
                if !field.label.is_empty() {
                    commands.push(json!({"op":"put","widget":{"id":ROOT+128+index as u32,"parent":PANEL,"kind":"label","rect":[16,y.saturating_sub(18),w.saturating_sub(32),18],"font":font,"text_size":14,"text":field.label}}));
                }
            }
            commands.push(json!({"op":"put","widget":{"id":OK,"parent":PANEL,"kind":"button","rect":[w.saturating_sub(256),h.saturating_sub(44),112,32],"font":font,"text":"OK"}}));
        }
        commands.push(json!({"op":"put","widget":{"id":CANCEL,"parent":PANEL,"kind":"button","rect":[w.saturating_sub(128),h.saturating_sub(44),112,32],"font":font,"text":"Cancel"}}));
        let focus = if !work.fields.is_empty() {
            FIELD
        } else if !work.options.is_empty() {
            OPTION + (work.page * work.rows) as u32
        } else {
            CANCEL
        };
        commands.push(json!({"op":"focus","id":focus}));
        self.submit(host, json!({"ui":commands}))
    }
    fn form_button(&self, host: &mut dyn HostServices, id: u32) -> Result<bool> {
        let state: Value = serde_json::from_slice(
            &host.query_engine(self.client, EngineQuery::UiInteraction(id))?,
        )?;
        Ok(state
            .get("activated")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }
    pub(crate) fn user_dialog_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        request: u64,
        dialog: &Request,
    ) -> Result<bool> {
        if self.user.dialog.is_some_and(|(owner, _)| owner != request) {
            return Ok(false);
        }
        if self.dialog_work.is_none() {
            let work = DialogWork::new(request, dialog.clone(), self.graph.screen)?;
            self.present_user_form(host, &work)?;
            self.user.dialog = Some((request, 0));
            self.dialog_work = Some(work);
            return Ok(false);
        }
        let mut work = self.dialog_work.take().unwrap();
        ensure!(
            work.request == request,
            "BGI input dialog continuation owner"
        );
        let mut cancelled = self.form_button(host, CANCEL)?;
        cancelled |= self.input.buttons_pressed & crysirror_core::service::buttons::ESCAPE != 0;
        if cancelled {
            self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
            self.user.dialog = None;
            vm.commit_user_dialog(None)?;
            return Ok(true);
        }
        if matches!(&work.dialog, Request::Selection { .. }) {
            let mut change_page = None;
            if work.page > 0 && self.form_button(host, PREVIOUS)? {
                change_page = Some(work.page - 1);
            }
            if (work.page + 1) * work.rows < work.options.len() && self.form_button(host, NEXT)? {
                change_page = Some(work.page + 1);
            }
            if let Some(page) = change_page {
                self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
                work.page = page;
                self.present_user_form(host, &work)?;
                self.dialog_work = Some(work);
                return Ok(false);
            }
            for (index, option) in work
                .options
                .iter()
                .enumerate()
                .skip(work.page * work.rows)
                .take(work.rows)
            {
                if self.form_button(host, OPTION + index as u32)? {
                    let response = Some(Response::Selection(option.clone()));
                    vm.commit_user_dialog(response)?;
                    self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
                    self.user.dialog = None;
                    return Ok(true);
                }
            }
        } else {
            for (index, field) in work.fields.iter_mut().enumerate() {
                let widget: Value = serde_json::from_slice(
                    &host.query_engine(self.client, EngineQuery::UiWidget(FIELD + index as u32))?,
                )?;
                let text = widget
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or(&field.text);
                let mut value = super::user_api::truncate_sjis(text, field.limit);
                if field.numeric {
                    value.retain(|c| c.is_ascii_digit());
                }
                if value != text {
                    self.submit(
                        host,
                        json!({"ui":[{"op":"set_text","id":FIELD+index as u32,"text":value}]}),
                    )?;
                }
                field.text = value;
            }
            if self.form_button(host, OK)? {
                vm.commit_user_dialog(work.response())?;
                self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
                self.user.dialog = None;
                return Ok(true);
            }
        }
        self.dialog_work = Some(work);
        Ok(false)
    }
    pub(crate) fn prompt_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        request: u64,
        text: &str,
        default_yes: Option<bool>,
    ) -> Result<bool> {
        if self.user.dialog.is_some_and(|(owner, _)| owner != request) {
            return Ok(false);
        }
        if self.user.dialog.is_none() {
            self.user.next_dialog = (self.user.next_dialog % 65535) + 1;
            let id = self.user.next_dialog;
            let font = self
                .config
                .font
                .as_deref()
                .context("BGI host prompt requires engine font")?;
            let buttons = if default_yes.is_some() {
                vec![json!({"id":1,"label":"Yes"}), json!({"id":0,"label":"No"})]
            } else {
                vec![json!({"id":1,"label":"OK"})]
            };
            self.submit(host,json!({"ui":[{"op":"open_dialog","dialog":{"id":id,"title":self.user.caption.as_deref().unwrap_or("CrySirror"),"message":text,"buttons":buttons,"default":u32::from(default_yes.unwrap_or(true)),"cancel":u32::from(default_yes.is_none()),"font":font,"text_size":24}}]}))?;
            self.user.dialog = Some((request, id));
            return Ok(false);
        }
        let (_, id) = self.user.dialog.unwrap();
        let events: Vec<Value> =
            serde_json::from_slice(&host.query_engine(self.client, EngineQuery::DialogEvents)?)?;
        let Some(event) = events.iter().find(|event| {
            event
                .get("widget")
                .and_then(|w| w.get("id"))
                .and_then(Value::as_u64)
                == Some(u64::from(id))
        }) else {
            return Ok(false);
        };
        let accepted = event.get("value").and_then(Value::as_f64).unwrap_or(0.) != 0.;
        vm.commit_prompt(accepted)?;
        self.user.dialog = None;
        Ok(true)
    }
}
