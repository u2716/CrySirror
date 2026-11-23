// SPDX-License-Identifier: MPL-2.0
//! Native modal dialogs `0x213a..=0x213d` over the shared dialog and widget UI.
//! The script waits on the selector until the dialog closes, as the original
//! DialogBoxParam/MessageBox calls block the engine thread.

use crate::vm::CmvsVm;
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
use serde_json::{json, Value};

const DIALOG: u32 = 50_700;
const EDIT_ROOT: u32 = 50_710;
const EDIT_FIELD: u32 = 50_711;
const EDIT_OK: u32 = 50_712;
const EDIT_CANCEL: u32 = 50_713;
/// CMessageDlg 132: the exit confirmation strings of `0x213a` (0x14007cee0).
const EXIT_TITLE: &str = "ゲーム終了の確認";
const EXIT_MESSAGE: &str = "ゲームを終了させますか？";
/// Title of the 0x140085450 font selection dialog.
const FONT_TITLE: &str = "フォントの選択";

enum Kind {
    /// `0x213a` and `0x213b`: the chosen button is the result.
    Choice,
    /// `0x213c`: CStringEditDlg 133, reporting its edited text.
    Edit { output: u32, fallback: Vec<u8>, limit: usize },
    /// `0x213d`: the chosen face becomes the default message face.
    Font { faces: Vec<String> },
    CvnsFont { faces: Vec<String>, page: usize, font: String },
    Color(crysirror_scene::ui::color_picker::ColorPicker),
}

struct Pending {
    opcode: u16,
    pc: usize,
    kind: Kind,
}

#[derive(Default)]
pub(super) struct Dialogs {
    pending: Option<Pending>,
}

/// Outcome of a dialog selector for the adapter.
pub(super) enum Outcome {
    /// The dialog stays open; the selector runs again next frame.
    Waiting,
    Done,
    /// `0x213d` chose a face for the default message font.
    Face(Vec<u8>),
}

fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes).0.into_owned()
}

/// 0x439bc0 copies only double-byte CP932 characters from the ANSI edit buffer.
fn cvns_name(bytes: &[u8], limit: usize) -> Vec<u8> {
    let mut pairs = Vec::new();
    let mut input = bytes[..bytes.len().min(limit)].iter().copied();
    while let Some(byte) = input.next() {
        if matches!(byte, 0x81..=0x9f | 0xe0..=0xfc) {
            let Some(trail) = input.next() else { break; };
            pairs.extend([byte, trail]);
        }
    }
    pairs
}

fn submit(host: &mut dyn HostServices, client: ClientId, ops: Vec<Value>) -> Result<()> {
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({ "ui": ops }))?)?;
    Ok(())
}

fn events(host: &mut dyn HostServices, client: ClientId, root: u32) -> Result<Vec<Value>> {
    let bytes = host.query_engine(client, EngineQuery::UiSubtreeEvents(root))?;
    ensure!(bytes.len() <= 64 * 1024, "CMVS dialog event byte budget");
    Ok(serde_json::from_slice(&bytes)?)
}

impl Dialogs {
    /// Dispatches `0x213a..=0x213d`, opening the dialog on first entry.
    pub fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
    ) -> Result<Outcome> {
        if let Some(pending) = &self.pending {
            ensure!(
                pending.opcode == opcode && pending.pc == vm.pc(),
                "CMVS dialog selector changed while open"
            );
            return self.poll(vm, host, client);
        }
        let kind = match opcode {
            0x213a => {
                self.open_choice(host, client, font, EXIT_TITLE, EXIT_MESSAGE, &[(1, "はい"), (0, "いいえ")], 1, Some(0))?;
                Kind::Choice
            }
            0x213b | 0x29b1 => {
                let [mode, title, message] = vm.owner_arguments(opcode)?;
                let title = decode(vm.text_reference(title).context("CMVS message box title")?);
                let message = decode(vm.text_reference(message).context("CMVS message box text")?);
                // 0x14007ce20: mode 0 is MB_OK, other values MB_RETRYCANCEL. IDOK and IDRETRY give 1.
                if opcode == 0x213b && mode == 0 {
                    self.open_choice(host, client, font, &title, &message, &[(1, "OK")], 1, Some(1))?;
                } else {
                    self.open_choice(host, client, font, &title, &message, &[(1, "再試行"), (0, "キャンセル")], 1, Some(0))?;
                }
                Kind::Choice
            }
            0x213c | 0x29b2 => {
                let [title, prompt, initial, limit, output, _] = vm.owner_arguments(opcode)?;
                let title = decode(vm.text_reference(title).context("CMVS edit dialog title")?);
                let prompt = decode(vm.text_reference(prompt).context("CMVS edit dialog prompt")?);
                let fallback = vm.text_reference(initial).context("CMVS edit dialog text")?.to_vec();
                let limit = if opcode == 0x29b2 {
                    if limit == 0 { 255 } else { limit.min(255) as usize }
                } else { (limit as i32).clamp(1, 255) as usize };
                self.open_edit(host, client, canvas, font, &title, &prompt, &decode(&fallback), limit)?;
                Kind::Edit { output, fallback, limit }
            }
            0x29b7 => {
                let colors: [u32; 2] = vm.owner_arguments(opcode)?;
                let mut palette = [[0; 3]; 16];
                for (dst, value) in palette.iter_mut().zip(colors) {
                    *dst = super::cvns::rgb(value);
                }
                Kind::Color(crysirror_scene::ui::color_picker::ColorPicker::open(
                    host, client, EDIT_ROOT, canvas, [0; 3], &palette, font)?)
            }
            0x29b6 => {
                let faces = host.font_library(client)?.aliases()?;
                let current = faces.iter().position(|f| f == font).unwrap_or(0);
                let page = current / 4;
                self.open_font_page(host, client, font, &faces, page, current)?;
                Kind::CvnsFont { faces, page, font: font.to_owned() }
            }
            0x213d => {
                let mut faces = host.font_library(client)?.aliases()?;
                faces.truncate(7);
                ensure!(!faces.is_empty(), "CMVS font selection has no faces");
                let mut buttons: Vec<(u32, &str)> =
                    faces.iter().enumerate().map(|(index, face)| (index as u32 + 1, face.as_str())).collect();
                buttons.push((0, "キャンセル"));
                self.open_choice(host, client, font, FONT_TITLE, FONT_TITLE, &buttons, 1, Some(0))?;
                Kind::Font { faces }
            }
            _ => unreachable!("CMVS dialog selector"),
        };
        self.pending = Some(Pending { opcode, pc: vm.pc(), kind });
        vm.wait_owner_call(opcode, None)?;
        Ok(Outcome::Waiting)
    }

    fn open_font_page(&self, host: &mut dyn HostServices, client: ClientId, font: &str,
        faces: &[String], page: usize, selected: usize) -> Result<()> {
        let mut buttons: Vec<_> = faces.iter().skip(page * 4).take(4).enumerate()
            .map(|(i, f)| (i as u32 + 1, f.as_str())).collect();
        if page > 0 { buttons.push((5, "前へ")); }
        if (page + 1) * 4 < faces.len() { buttons.push((6, "次へ")); }
        buttons.push((0, "キャンセル"));
        let default = if faces.is_empty() { 0 } else { (selected % 4 + 1) as u32 };
        self.open_choice(host, client, font, FONT_TITLE, FONT_TITLE, &buttons, default, Some(0))
    }

    #[allow(clippy::too_many_arguments)]
    fn open_choice(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        font: &str,
        title: &str,
        message: &str,
        buttons: &[(u32, &str)],
        default: u32,
        cancel: Option<u32>,
    ) -> Result<()> {
        let buttons: Vec<Value> = buttons.iter().map(|(id, label)| json!({"id":id,"label":label})).collect();
        submit(host, client, vec![json!({"op":"open_dialog","dialog":{
            "id":DIALOG,"title":title,"message":message,"buttons":buttons,
            "default":default,"cancel":cancel,"font":font,"text_size":22}})])
    }

    #[allow(clippy::too_many_arguments)]
    fn open_edit(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
        title: &str,
        prompt: &str,
        initial: &str,
        limit: usize,
    ) -> Result<()> {
        let [width, height] = canvas.map(|value| value as f32);
        let panel = [width / 2. - 260., height / 2. - 110., 520., 220.];
        let at = |x: f32, y: f32, w: f32, h: f32| [x, y, w, h];
        let label = |id: u32, rect: [f32; 4], text: &str| {
            json!({"op":"put","widget":{"id":id,"parent":EDIT_ROOT,"rect":rect,"kind":"label",
                "text":text,"font":font,"text_size":20,"text_color":[0,0,0,255],"z":1}})
        };
        let button = |id: u32, rect: [f32; 4], text: &str| {
            json!({"op":"put","widget":{"id":id,"parent":EDIT_ROOT,"rect":rect,"kind":"button",
                "text":text,"font":font,"text_size":20,"color":[220,220,220,255],
                "text_color":[0,0,0,255],"z":2}})
        };
        submit(host, client, vec![
            json!({"op":"put","widget":{"id":EDIT_ROOT,"rect":panel,"kind":"panel","modal":true,
                "color":[240,240,240,255],"clip_children":true}}),
            label(EDIT_ROOT + 5, at(16., 10., 488., 28.), title),
            label(EDIT_ROOT + 6, at(16., 48., 488., 28.), prompt),
            json!({"op":"put","widget":{"id":EDIT_FIELD,"parent":EDIT_ROOT,"rect":at(16., 90., 488., 36.),
                "kind":"text_field","text":initial,"max_length":limit,"font":font,"text_size":22,
                "color":[255,255,255,255],"text_color":[0,0,0,255],"text_inset":[4,4],"z":3}}),
            button(EDIT_OK, at(280., 160., 100., 40.), "OK"),
            button(EDIT_CANCEL, at(400., 160., 100., 40.), "キャンセル"),
            json!({"op":"focus","id":EDIT_FIELD}),
        ])
    }

    fn poll(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId) -> Result<Outcome> {
        let pending = self.pending.as_mut().context("CMVS dialog state")?;
        let opcode = pending.opcode;
        if let Kind::Color(picker) = &mut pending.kind {
            use crysirror_scene::ui::color_picker::ColorSelection;
            match picker.poll(host, client)? {
                ColorSelection::Pending => {
                    vm.wait_owner_call(opcode, None)?;
                    return Ok(Outcome::Waiting);
                }
                ColorSelection::Cancelled => vm.complete_owner_call(2, Some(0))?,
                ColorSelection::Selected([r,g,b]) => vm.complete_owner_results(2,
                    &[(0,1),(4,u32::from_be_bytes([0,r,g,b]))])?,
            }
            self.pending = None;
            return Ok(Outcome::Done);
        }
        let pending = self.pending.as_ref().context("CMVS dialog state")?;
        if let Kind::Edit { output, fallback, limit } = &pending.kind {
            let mut choice = None;
            for event in events(host, client, EDIT_ROOT)? {
                let id = event["widget"]["id"].as_u64().map(|id| id as u32);
                if event["kind"] == "activate" && matches!(id, Some(EDIT_OK | EDIT_FIELD)) {
                    choice = Some(true);
                } else if event["kind"] == "activate" && id == Some(EDIT_CANCEL) {
                    choice = Some(false);
                }
            }
            let Some(accepted) = choice else {
                vm.wait_owner_call(opcode, None)?;
                return Ok(Outcome::Waiting);
            };
            let cvns = opcode == 0x29b2;
            let mut text = if cvns { fallback.clone() } else { Vec::new() };
            if accepted {
                let widget: Value = serde_json::from_slice(&host.query_engine(client, EngineQuery::UiWidget(EDIT_FIELD))?)?;
                let edited = widget["text"].as_str().unwrap_or_default();
                let edited = if cvns { edited } else { edited.trim() };
                let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(edited);
                text = bytes.into_owned();
                if cvns {
                    text = cvns_name(&text, *limit);
                }
                // 0x1400a6d50 keeps the initial text when the edit is empty.
                if text.is_empty() {
                    text.clone_from(fallback);
                }
                if !cvns { text.truncate(*limit); }
            }
            if cvns { vm.write_cvns_text(opcode, *output, &text)?; }
            else { vm.store_owner_string(*output, &text)?; }
            submit(host, client, vec![json!({"op":"remove","id":EDIT_ROOT})])?;
            vm.complete_owner_call(6, Some(u32::from(accepted)))?;
            self.pending = None;
            return Ok(Outcome::Done);
        }
        let mut choice = None;
        for event in events(host, client, DIALOG)? {
            if event["kind"] == "dialog_result" && event["widget"]["id"].as_u64() == Some(u64::from(DIALOG)) {
                choice = event["value"].as_f64().map(|value| value as u32);
            }
        }
        let Some(choice) = choice else {
            vm.wait_owner_call(opcode, None)?;
            return Ok(Outcome::Waiting);
        };
        let pending = self.pending.take().context("CMVS dialog state")?;
        match pending.kind {
            Kind::CvnsFont { faces, mut page, font } => {
                if choice == 5 || choice == 6 {
                    page = if choice == 5 { page - 1 } else { page + 1 };
                    self.open_font_page(host, client, &font, &faces, page, page * 4)?;
                    self.pending = Some(Pending { opcode, pc: vm.pc(), kind: Kind::CvnsFont { faces, page, font } });
                    vm.wait_owner_call(opcode, None)?;
                    return Ok(Outcome::Waiting);
                }
                vm.complete_owner_call(0, None)?;
                let face = choice.checked_sub(1).and_then(|i| faces.get(page * 4 + i as usize));
                Ok(face.map_or(Outcome::Done, |f| Outcome::Face(encoding_rs::SHIFT_JIS.encode(f).0.into_owned())))
            }
            Kind::Font { faces } => {
                vm.complete_owner_call(0, None)?;
                let face = (choice as usize).checked_sub(1).and_then(|index| faces.get(index));
                Ok(face.map_or(Outcome::Done, |face| {
                    Outcome::Face(encoding_rs::SHIFT_JIS.encode(face).0.into_owned())
                }))
            }
            _ => {
                let words = if matches!(opcode, 0x213b | 0x29b1) { 3 } else { 0 };
                vm.complete_owner_call(words, Some(choice))?;
                Ok(Outcome::Done)
            }
        }
    }
}
