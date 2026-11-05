//! CrySirror widgets for the existing host-runtime dialog protocol.
use super::*;
use crysirror_core::service::EngineQuery;
const ROOT: u32 = 0xb611_0000;
const PANEL: u32 = ROOT + 1;
const OK: u32 = ROOT + 2;
const CANCEL: u32 = ROOT + 3;
const FIELD: u32 = ROOT + 64;
const OPTION: u32 = ROOT + 1024;
struct Field {
    text: String,
    limit: usize,
    numeric: bool,
    label: String,
}
pub(super) struct Dialog {
    request: u64,
    payload: Option<Value>,
    fields: Vec<Field>,
    options: Vec<String>,
    prompt: bool,
    title: String,
    message: String,
    page: usize,
    rows: usize,
}
fn bounded_text(text: &str, limit: usize, numeric: bool) -> String {
    let mut out = String::new();
    let mut count = 0;
    for ch in text.chars() {
        if numeric && !(ch.is_ascii_digit() || (out.is_empty() && ch == '-')) {
            continue;
        }
        let len = encoding_rs::SHIFT_JIS.encode(&ch.to_string()).0.len();
        if count + len > limit {
            break;
        }
        count += len;
        out.push(ch);
    }
    out
}
impl IntegratedVm {
    pub(super) fn open_dialog(
        &mut self,
        host: &mut dyn HostServices,
        event: HostEvent,
        payload: Option<String>,
    ) -> Result<()> {
        let payload: Option<Value> = payload
            .map(|text| serde_json::from_str(&text))
            .transpose()?;
        ensure!(
            event.kind != 5 || payload.is_some(),
            "BGI missing native dialog payload"
        );
        let mut fields = Vec::new();
        let mut options = Vec::new();
        if let Some(value) = &payload {
            match value["type"].as_str().context("BGI dialog type")? {
                "input" => {
                    for field in value["fields"].as_array().context("BGI input fields")? {
                        let limit = field["max_bytes"]
                            .as_u64()
                            .context("BGI field byte limit")?
                            as usize;
                        ensure!(limit <= 65536, "BGI input field byte budget");
                        fields.push(Field {
                            text: field["initial"].as_str().unwrap_or_default().into(),
                            limit,
                            numeric: field["numeric"].as_bool().unwrap_or(false),
                            label: field["label"].as_str().unwrap_or_default().into(),
                        });
                    }
                }
                "segmented" => {
                    let segments = value["segments"].as_u64().context("BGI segment count")?;
                    ensure!(segments <= 32, "BGI segment field budget");
                    let limit = value["max_bytes"]
                        .as_u64()
                        .context("BGI segment byte limit")?
                        as usize;
                    ensure!(limit <= 65536, "BGI segment byte budget");
                    for index in 0..segments {
                        fields.push(Field {
                            text: String::new(),
                            limit,
                            numeric: value["numeric"].as_bool().unwrap_or(false),
                            label: (index + 1).to_string(),
                        });
                    }
                }
                "date" => {
                    for text in value["fields"].as_array().context("BGI date fields")? {
                        fields.push(Field {
                            text: text.as_str().context("BGI date field")?.into(),
                            limit: 10,
                            numeric: false,
                            label: String::new(),
                        });
                    }
                    for (key, label) in [("month", "Month"), ("day", "Day")] {
                        fields.push(Field {
                            text: (value[key].as_i64().unwrap_or(-1) + 1).max(0).to_string(),
                            limit: 2,
                            numeric: true,
                            label: label.into(),
                        });
                    }
                }
                "select" => {
                    for option in value["options"]
                        .as_array()
                        .context("BGI selection options")?
                    {
                        ensure!(options.len() < 4096, "BGI dialog selection budget");
                        options.push(option.as_str().context("BGI option text")?.to_owned());
                    }
                }
                kind => bail!("BGI unsupported host dialog shape {kind}"),
            }
        }
        let title = payload
            .as_ref()
            .and_then(|value| value["title"].as_str())
            .unwrap_or("CrySirror")
            .to_owned();
        let message = payload
            .as_ref()
            .and_then(|value| value["prompt"].as_str())
            .unwrap_or(&event.text)
            .to_owned();
        let rows = ((self.size[1].saturating_sub(240)) / 48).max(1) as usize;
        ensure!(
            fields.len() <= rows.max(6),
            "BGI input dialog exceeds logical viewport"
        );
        let work = Dialog {
            request: event.request_id,
            payload,
            fields,
            options,
            prompt: event.kind == 2,
            title,
            message,
            page: 0,
            rows,
        };
        self.show_dialog(host, &work)?;
        self.dialog = Some(work);
        Ok(())
    }
    fn show_dialog(&self, host: &mut dyn HostServices, dialog: &Dialog) -> Result<()> {
        let font = self.config.font.as_deref().unwrap_or("default");
        let w = self.size[0].saturating_sub(48).clamp(32, 800);
        let h = self.size[1].saturating_sub(48).max(32);
        let x = self.size[0].saturating_sub(w) / 2;
        let y = self.size[1].saturating_sub(h) / 2;
        let mut commands = vec![
            json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,self.size[0],self.size[1]],"color":[0,0,0,160],"z":2147479000,"modal":true,"keyboard_focusable":false}}),
            json!({"op":"put","widget":{"id":PANEL,"parent":ROOT,"rect":[x,y,w,h],"color":[36,36,44,255],"keyboard_focusable":false}}),
            json!({"op":"put","widget":{"id":ROOT+8,"parent":PANEL,"kind":"label","rect":[16,12,w.saturating_sub(32),36],"font":font,"text_size":24,"text":dialog.title}}),
            json!({"op":"put","widget":{"id":ROOT+9,"parent":PANEL,"kind":"label","rect":[16,52,w.saturating_sub(32),96],"font":font,"text":dialog.message}}),
        ];
        for (index, field) in dialog.fields.iter().enumerate() {
            let top = 152 + index * 48;
            commands.push(json!({"op":"put","widget":{"id":FIELD+index as u32,"parent":PANEL,"kind":"text_field","rect":[16,top,w.saturating_sub(32),40],"font":font,"text":field.text,"max_length":field.limit.max(1),"text_color":[10,10,10,255],"color":[250,250,250,255]}}));
            if !field.label.is_empty() {
                commands.push(json!({"op":"put","widget":{"id":ROOT+128+index as u32,"parent":PANEL,"kind":"label","rect":[16,top.saturating_sub(16),w.saturating_sub(32),16],"font":font,"text_size":12,"text":field.label}}));
            }
        }
        for (index, text) in dialog
            .options
            .iter()
            .enumerate()
            .skip(dialog.page * dialog.rows)
            .take(dialog.rows)
        {
            commands.push(json!({"op":"put","widget":{"id":OPTION+index as u32,"parent":PANEL,"kind":"button","rect":[16,152+(index%dialog.rows)*48,w.saturating_sub(32),40],"font":font,"text":text}}));
        }
        if dialog.options.is_empty() {
            commands.push(json!({"op":"put","widget":{"id":OK,"parent":PANEL,"kind":"button","rect":[w.saturating_sub(256),h.saturating_sub(44),112,32],"font":font,"text":if dialog.prompt {"OK / Yes"} else {"OK"}}}));
        } else {
            if dialog.page > 0 {
                commands.push(json!({"op":"put","widget":{"id":ROOT+4,"parent":PANEL,"kind":"button","rect":[16,h.saturating_sub(44),100,32],"font":font,"text":"Previous"}}));
            }
            if (dialog.page + 1) * dialog.rows < dialog.options.len() {
                commands.push(json!({"op":"put","widget":{"id":ROOT+5,"parent":PANEL,"kind":"button","rect":[120,h.saturating_sub(44),100,32],"font":font,"text":"Next"}}));
            }
        }
        commands.push(json!({"op":"put","widget":{"id":CANCEL,"parent":PANEL,"kind":"button","rect":[w.saturating_sub(128),h.saturating_sub(44),112,32],"font":font,"text":if dialog.prompt {"Cancel / No"} else {"Cancel"}}}));
        // Explicit UI focus edits a TextField. Buttons use the UI's modal
        // navigation focus, which is assigned automatically on the next tick.
        if !dialog.fields.is_empty() {
            commands.push(json!({"op":"focus","id":FIELD}));
        }
        self.submit(host, json!({"ui":commands}))
    }
    fn button(&self, host: &mut dyn HostServices, id: u32) -> Result<bool> {
        let state: Value = serde_json::from_slice(
            &host.query_engine(self.client, EngineQuery::UiInteraction(id))?,
        )?;
        Ok(state["activated"].as_bool().unwrap_or(false))
    }
    pub(super) fn poll_dialog(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Some(mut dialog) = self.dialog.take() else {
            return Ok(());
        };
        let cancelled =
            self.input.buttons_pressed & buttons::ESCAPE != 0 || self.button(host, CANCEL)?;
        let mut answer = cancelled.then(|| json!({"accepted":false}));
        if !cancelled {
            if !dialog.options.is_empty() {
                let mut new_page = None;
                if dialog.page > 0 && self.button(host, ROOT + 4)? {
                    new_page = Some(dialog.page - 1);
                }
                if (dialog.page + 1) * dialog.rows < dialog.options.len()
                    && self.button(host, ROOT + 5)?
                {
                    new_page = Some(dialog.page + 1);
                }
                if let Some(page) = new_page {
                    self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
                    dialog.page = page;
                    self.show_dialog(host, &dialog)?;
                }
                for index in (dialog.page * dialog.rows)
                    ..((dialog.page + 1) * dialog.rows).min(dialog.options.len())
                {
                    if self.button(host, OPTION + index as u32)? {
                        answer = Some(json!({"accepted":true,"index":index}));
                        break;
                    }
                }
            } else {
                for (index, field) in dialog.fields.iter_mut().enumerate() {
                    let widget: Value =
                        serde_json::from_slice(&host.query_engine(
                            self.client,
                            EngineQuery::UiWidget(FIELD + index as u32),
                        )?)?;
                    let text = widget["text"].as_str().unwrap_or(&field.text);
                    let value = bounded_text(text, field.limit, field.numeric);
                    if value != text {
                        self.submit(
                            host,
                            json!({"ui":[{"op":"set_text","id":FIELD+index as u32,"text":value}]}),
                        )?;
                    }
                    field.text = value;
                }
                if self.button(host, OK)?
                    || (dialog.prompt && self.input.buttons_pressed & buttons::ENTER != 0)
                {
                    let mut response = json!({"accepted":true,"texts":dialog.fields.iter().map(|field|field.text.as_str()).collect::<Vec<_>>()});
                    if dialog
                        .payload
                        .as_ref()
                        .is_some_and(|value| value["type"] == "date")
                    {
                        ensure!(dialog.fields.len() == 6, "BGI date form shape");
                        response["month"] =
                            json!(dialog.fields[4].text.parse::<i32>().unwrap_or(1) - 1);
                        response["day"] =
                            json!(dialog.fields[5].text.parse::<i32>().unwrap_or(1) - 1);
                    }
                    answer = Some(response);
                }
            }
        }
        if let Some(response) = answer {
            self.submit(host, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
            if dialog.prompt {
                self.controls.push_back(WorkerCommand::Request {
                    id: dialog.request,
                    result: i32::from(!cancelled),
                });
            } else {
                self.controls.push_back(WorkerCommand::Dialog {
                    id: dialog.request,
                    response: response.to_string(),
                });
            }
        } else {
            self.dialog = Some(dialog);
        }
        Ok(())
    }
}
