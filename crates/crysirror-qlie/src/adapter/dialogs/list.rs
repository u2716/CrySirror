//! Native list/memo dialog. UI owns editing, focus and clipboard transport;
//! the adapter retains the original list's copy-back/readonly policy.
use super::*;
use crysirror_core::service::{
    DesktopRequest, DesktopRequestJob, DesktopRequestState, DesktopResponse,
};
pub(super) struct ListPicker {
    source: String,
    lines: Vec<String>,
    offset: usize,
    count: usize,
    editable: bool,
    width: f32,
    clipboard: Option<Box<dyn DesktopRequestJob>>,
}
impl QlieVm {
    fn list_dialog_read_fields(
        &self,
        h: &mut dyn HostServices,
        root: u32,
        picker: &mut ListPicker,
    ) -> Result<()> {
        if picker.editable {
            for row in 0..picker.count {
                if picker.offset + row >= picker.lines.len() {
                    break;
                }
                let spec: serde_json::Value = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::UiWidget(root + 32 + row as u32))?,
                )?;
                picker.lines[picker.offset + row] = spec["text"]
                    .as_str()
                    .context("QLIE list dialog edited row")?
                    .to_owned();
            }
        }
        Ok(())
    }
    fn list_dialog_rows(
        &mut self,
        h: &mut dyn HostServices,
        root: u32,
        picker: &ListPicker,
    ) -> Result<()> {
        let commands=(0..picker.count).map(|row|json!({"op":"put","widget":{"id":root+32+row as u32,"parent":root,"kind":if picker.editable{"text_field"}else{"label"},"rect":[20,60+row*28,picker.width-40.,26],"font":"default","text_size":18,"text":picker.lines.get(picker.offset+row).cloned().unwrap_or_default(),"max_length":16384,"keyboard_focusable":picker.editable,"enabled":!picker.editable||picker.offset+row<picker.lines.len()}})).collect::<Vec<_>>();
        self.submit(h, json!({"ui":commands}))
    }
    pub(in crate::adapter) fn list_dialog(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool> {
        if self.dialog.is_none() {
            ensure!(a.len() == 9, "QLIE list dialog ABI");
            ensure!(
                a[4].text().is_empty(),
                "unsupported QLIE list dialog secondary action"
            );
            let source = a[0].text();
            let object = self
                .objects
                .get(&source)
                .context("QLIE list dialog source")?;
            ensure!(
                matches!(object.kind.as_str(), "list" | "table"),
                "QLIE list dialog source kind"
            );
            let root = 0x40000000;
            let field = root + 3;
            let button = root + 4;
            let width = (a[7].integer()?.max(320) as f32)
                .min(self.config.canvas[0].saturating_sub(32).max(160) as f32);
            let height = (a[8].integer()?.max(240) as f32)
                .min(self.config.canvas[1].saturating_sub(32).max(160) as f32);
            let x = (self.config.canvas[0] as f32 - width) / 2.;
            let y = (self.config.canvas[1] as f32 - height) / 2.;
            let count = ((height - 156.) / 28.).floor().max(1.) as usize;
            let picker = ListPicker {
                source,
                lines: object.lines.clone(),
                offset: 0,
                count,
                editable: a[6].integer()? == 1,
                width,
                clipboard: None,
            };
            self.submit(h,json!({"ui":[
                {"op":"put","widget":{"id":root,"kind":"panel","rect":[x,y,width,height],"modal":true,"z":i32::MAX,"color":[36,40,48,255],"clip_children":true,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+1,"parent":root,"kind":"label","rect":[20,12,width-40.,32],"text":if a[2].text().is_empty(){a[1].text()}else{a[2].text()},"font":"default","text_size":24,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+6,"parent":root,"kind":"button","rect":[20,height-88.,52,28],"text":"↑","font":"default"}},
                {"op":"put","widget":{"id":root+7,"parent":root,"kind":"button","rect":[80,height-88.,52,28],"text":"↓","font":"default"}},
                {"op":"put","widget":{"id":root+2,"parent":root,"kind":"label","rect":[144,height-88.,width-164.,28],"text":"","font":"default","text_size":16,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+5,"parent":root,"kind":"button","rect":[20,height-48.,(width-176.).max(100.),32],"text":a[3].text(),"visible":!a[3].text().is_empty(),"font":"default","text_size":18}},
                {"op":"put","widget":{"id":button,"parent":root,"kind":"button","rect":[width-140.,height-48.,120,32],"text":a[5].text(),"font":"default","text_size":18}},
                {"op":"focus","id":button}
            ]}))?;
            self.list_dialog_rows(h, root, &picker)?;
            self.dialog = Some(TextDialog {
                root,
                field,
                button,
                font: None,
                list: Some(picker),
            });
            return Ok(false);
        }
        let mut dialog = self
            .dialog
            .take()
            .context("QLIE list dialog continuation")?;
        let Some(picker) = &mut dialog.list else {
            self.dialog = Some(dialog);
            bail!("QLIE list dialog continuation owner");
        };
        if let Some(job) = &mut picker.clipboard {
            match job.poll() {
                Ok(DesktopRequestState::Pending) => {}
                Ok(DesktopRequestState::Ready(DesktopResponse::ClipboardWritten)) => {
                    picker.clipboard = None;
                    self.submit(
                        h,
                        json!({"ui":[{"op":"set_text","id":dialog.root+2,"text":"Copied"}]}),
                    )?;
                }
                Ok(DesktopRequestState::Ready(_)) => bail!("QLIE clipboard completion type"),
                Err(error) => {
                    picker.clipboard = None;
                    self.submit(h,json!({"ui":[{"op":"set_text","id":dialog.root+2,"text":format!("Clipboard: {error}")}]}))?;
                }
            }
        }
        let events: Vec<serde_json::Value> = serde_json::from_slice(
            &h.query_engine(self.client, EngineQuery::UiSubtreeEvents(dialog.root))?,
        )?;
        let mut close = false;
        for event in events {
            let id = event["widget"]["id"]
                .as_u64()
                .or_else(|| event["widget"].as_u64())
                .unwrap_or(0) as u32;
            let kind = event["kind"].as_str().unwrap_or("");
            if id == dialog.button && kind == "activate" || kind == "cancel" {
                close = true;
                break;
            }
            if kind != "activate" {
                continue;
            }
            if id == dialog.root + 6 || id == dialog.root + 7 {
                self.list_dialog_read_fields(h, dialog.root, picker)?;
                picker.offset = if id == dialog.root + 6 {
                    picker.offset.saturating_sub(picker.count)
                } else {
                    picker
                        .offset
                        .saturating_add(picker.count)
                        .min(picker.lines.len().saturating_sub(picker.count))
                };
                self.list_dialog_rows(h, dialog.root, picker)?;
            } else if id == dialog.root + 5 && picker.clipboard.is_none() {
                self.list_dialog_read_fields(h, dialog.root, picker)?;
                match h.begin_desktop_request(self.client,DesktopRequest::ClipboardWrite{text:picker.lines.join("\r\n")}) {
                    Ok(job)=>picker.clipboard=Some(job),
                    Err(error)=>self.submit(h,json!({"ui":[{"op":"set_text","id":dialog.root+2,"text":format!("Clipboard: {error}")}]}))?,
                }
            }
        }
        if !close {
            self.dialog = Some(dialog);
            return Ok(false);
        }
        self.list_dialog_read_fields(h, dialog.root, picker)?;
        if picker.editable {
            let object = self
                .objects
                .get_mut(&picker.source)
                .context("QLIE edited list destination")?;
            object.lines = std::mem::take(&mut picker.lines);
            object.table = object
                .lines
                .iter()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.to_owned()))
                .collect();
        }
        self.submit(h, json!({"ui":[{"op":"remove","id":dialog.root}]}))?;
        m.result(0, Value::Int(1));
        Ok(true)
    }
}
