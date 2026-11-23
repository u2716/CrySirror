//! Native input-query policy composed from engine-owned modal TextField/UI.
//! Text/IME admission, focus, editing, painting and events belong to shared UI.
use super::*;
mod choice;
mod color;
pub(super) use color::ColorDialog;
mod file;
pub(super) use file::FilePending;
mod font;
mod list;
pub(super) use choice::ChoiceDialog;
use crysirror_core::service::EngineQuery;
pub(super) struct TextDialog {
    root: u32,
    field: u32,
    button: u32,
    font: Option<font::FontPicker>,
    list: Option<list::ListPicker>,
}
impl QlieVm {
    pub(super) fn dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        if op == "sl" {
            return self.list_dialog(h, m, a);
        }
        if op == "f" {
            return self.font_dialog(h, m, a);
        }
        if op == "c" {
            return self.color_dialog_method(h, m, a);
        }
        if matches!(op, "o" | "s") {
            return self.file_dialog_method(h, m, op, a);
        }
        if matches!(op, "d1" | "d2" | "d3" | "sd1" | "sd2" | "sd3") {
            return self.choice_dialog_method(h, m, op, a);
        }
        ensure!(op == "t", "unsupported QLIE desktop dialog \\{op}");
        if self.dialog.is_none() {
            ensure!(a.len() == 4, "QLIE text input dialog ABI");
            let root = 0x40000000;
            let field = root + 3;
            let button = root + 4;
            let width = (self.config.canvas[0] as f32 - 32.).min(640.).max(64.);
            let height = (self.config.canvas[1] as f32 - 32.).min(256.).max(64.);
            let x = (self.config.canvas[0] as f32 - width) / 2.;
            let y = (self.config.canvas[1] as f32 - height) / 2.;
            self.submit(h,json!({"ui":[
                {"op":"put","widget":{"id":root,"rect":[x,y,width,height],"kind":"panel","modal":true,"z":i32::MAX,"color":[36,40,48,255],"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+1,"parent":root,"rect":[20,12,width-40.,32],"kind":"label","text":a[1].text(),"font":"default","text_size":24,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+2,"parent":root,"rect":[20,58,width-40.,54],"kind":"label","text":a[2].text(),"font":"default","text_size":22,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":field,"parent":root,"rect":[20,118,width-40.,44],"kind":"text_field","text":a[0].text(),"font":"default","text_size":24,"max_length":4096}},
                {"op":"put","widget":{"id":button,"parent":root,"rect":[width-156.,height-68.,136,44],"kind":"button","text":a[3].text(),"font":"default","text_size":22}},
                {"op":"focus","id":field}
            ]}))?;
            self.dialog = Some(TextDialog {
                root,
                field,
                button,
                font: None,
                list: None,
            });
            return Ok(false);
        }
        let d = self.dialog.as_ref().unwrap();
        let events: serde_json::Value = serde_json::from_slice(
            &h.query_engine(self.client, EngineQuery::UiSubtreeEvents(d.root))?,
        )?;
        let mut done = None;
        for event in events.as_array().context("QLIE text input event receipt")? {
            let widget = event["widget"]["id"]
                .as_u64()
                .or_else(|| event["widget"].as_u64());
            if (widget == Some(d.field as u64) && event["kind"] == "submit")
                || (widget == Some(d.button as u64) && event["kind"] == "activate")
            {
                done = Some(true);
                break;
            }
            if widget == Some(d.field as u64) && event["kind"] == "cancel" {
                done = Some(false);
                break;
            }
        }
        let Some(accepted) = done else {
            return Ok(false);
        };
        let widget: serde_json::Value =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiWidget(d.field))?)?;
        let value = if accepted {
            widget["text"]
                .as_str()
                .context("QLIE engine text field value")?
                .to_owned()
        } else {
            String::new()
        };
        let root = d.root;
        self.submit(h, json!({"ui":[{"op":"remove","id":root}]}))?;
        self.dialog = None;
        m.result_text(0, value);
        m.result(0, Value::Int(i32::from(accepted)));
        Ok(true)
    }
}
