//! dlgs message dialogs (700C20) on engine-owned modal UI.
//!
//! d1/d2/d3,message,title,button... use TImoMessageDialog: its buttons set
//! ModalResult-like codes 1/2/3 (Button1Click..3) and closing leaves 0; d1
//! pushes no result. sd1/sd2/sd3,message,type,button... use MessageDlg:
//! button words map through 700A10 to TMsgDlgBtn (shown in enum order with
//! the RTL captions) and the result is the Delphi mr* value; sd1 pushes no
//! result. `[n]` in a message is a line break (503E40). Every dialog ends
//! with the 250 ms input reset (5C765C).
use super::*;

pub(in crate::adapter) struct ChoiceDialog {
    pub root: u32,
    pub buttons: Vec<(u32, i32)>,
    pub push: bool,
}
/// TMsgDlgBtn order, caption and ModalResult.
const MSG_BUTTONS: [(&str, &str, i32); 11] = [
    ("yes", "Yes", 6),
    ("no", "No", 7),
    ("ok", "OK", 1),
    ("cancel", "Cancel", 2),
    ("abort", "Abort", 3),
    ("retry", "Retry", 4),
    ("ignore", "Ignore", 5),
    ("all", "All", 8),
    ("notoall", "No to All", 9),
    ("yestoall", "Yes to All", 10),
    ("close", "Close", 11),
];
/// 700A10: button word to TMsgDlgBtn index, else the slot default.
fn msg_button(word: &str, default: usize) -> usize {
    let word = word.to_lowercase();
    let index = MSG_BUTTONS.iter().position(|(w, _, _)| *w == word);
    // mbClose is enum value 11 (after mbHelp); the table skips mbHelp.
    index.unwrap_or(default)
}
/// 700904: dialog type word to its caption (mtWarning..mtConfirmation).
fn msg_caption(word: &str) -> &'static str {
    match word.to_lowercase().as_str() {
        "error" => "Error",
        "information" => "Information",
        "warning" => "Warning",
        "confirmation" => "Confirm",
        _ => "Information",
    }
}

impl QlieVm {
    pub(super) fn choice_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        if self.choice.is_none() {
            let text = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
            let message = text(0).replace("[n]", "\n");
            let (title, buttons, push): (String, Vec<(String, i32)>, bool) = match op {
                "d1" | "d2" | "d3" => {
                    let count = usize::from(op.as_bytes()[1] - b'0');
                    ensure!(a.len() == 2 + count, "QLIE message dialog ABI");
                    let title = text(1).replace("[n]", "\n");
                    let buttons = (0..count).map(|i| (text(2 + i), i as i32 + 1)).collect();
                    (title, buttons, op != "d1")
                }
                _ => {
                    let count = usize::from(op.as_bytes()[2] - b'0');
                    ensure!(a.len() == 2 + count, "QLIE standard message dialog ABI");
                    let defaults = [[2, 0, 0], [0, 1, 0], [0, 1, 3]][count - 1];
                    let mut set: Vec<usize> =
                        (0..count).map(|i| msg_button(&text(2 + i), defaults[i])).collect();
                    set.sort_unstable();
                    set.dedup();
                    let buttons = set
                        .into_iter()
                        .map(|i| (MSG_BUTTONS[i].1.to_owned(), MSG_BUTTONS[i].2))
                        .collect();
                    (msg_caption(&text(1)).to_owned(), buttons, op != "sd1")
                }
            };
            let root = 0x40000100;
            let width = (self.config.canvas[0] as f32 - 32.).min(640.).max(240.);
            let height = (self.config.canvas[1] as f32 - 32.).min(240.).max(160.);
            let x = (self.config.canvas[0] as f32 - width) / 2.;
            let y = (self.config.canvas[1] as f32 - height) / 2.;
            let mut ops = vec![
                json!({"op":"put","widget":{"id":root,"rect":[x,y,width,height],"kind":"panel","modal":true,"z":i32::MAX,"color":[36,40,48,255],"keyboard_focusable":false}}),
                json!({"op":"put","widget":{"id":root+1,"parent":root,"rect":[20,12,width-40.,32],"kind":"label","text":title,"font":"default","text_size":22,"keyboard_focusable":false}}),
                json!({"op":"put","widget":{"id":root+2,"parent":root,"rect":[20,52,width-40.,height-124.],"kind":"label","text":message,"font":"default","text_size":22,"keyboard_focusable":false}}),
            ];
            let count = buttons.len() as f32;
            let button_width = ((width - 40.) / count - 12.).min(160.);
            let mut ids = Vec::new();
            for (i, (caption, code)) in buttons.into_iter().enumerate() {
                let id = root + 3 + i as u32;
                let bx = width - 20. - (count - i as f32) * (button_width + 12.) + 12.;
                ops.push(json!({"op":"put","widget":{"id":id,"parent":root,"rect":[bx,height-60.,button_width,44],"kind":"button","text":caption,"font":"default","text_size":20}}));
                ids.push((id, code));
            }
            ops.push(json!({"op":"focus","id":ids[0].0}));
            self.submit(h, json!({ "ui": ops }))?;
            self.choice = Some(ChoiceDialog { root, buttons: ids, push });
            return Ok(false);
        }
        let d = self.choice.as_ref().unwrap();
        let events: serde_json::Value = serde_json::from_slice(
            &h.query_engine(self.client, EngineQuery::UiSubtreeEvents(d.root))?,
        )?;
        let mut chosen = None;
        for event in events.as_array().context("QLIE message dialog event receipt")? {
            let widget = event["widget"]["id"].as_u64().or_else(|| event["widget"].as_u64());
            if event["kind"] == "activate" {
                if let Some((_, code)) = d.buttons.iter().find(|(id, _)| Some(*id as u64) == widget) {
                    chosen = Some(*code);
                    break;
                }
            }
        }
        let Some(code) = chosen else {
            return Ok(false);
        };
        let (root, push) = (d.root, d.push);
        self.submit(h, json!({"ui":[{"op":"remove","id":root}]}))?;
        self.choice = None;
        if push {
            m.result(0, Value::Int(code));
        }
        self.block_input(h.now_ms(), 250);
        Ok(true)
    }
}
