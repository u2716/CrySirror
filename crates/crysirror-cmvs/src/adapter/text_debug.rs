// SPDX-License-Identifier: MPL-2.0
//! Text debug records `0x23a4..=0x23a8` (engine +4768, 0x140019f50..0x14001a8f0) and the
//! modal CTextDebugDlg 147 of `0x23a6` (0x1400157f0, procedure 0x140015ea0). Testers mark
//! a script line with a corrected name and text and a remark; records are kept in
//! `<computer>_<n>.dvg` beside the save slots, one CP932 CSV line each. The host exposes
//! no computer name, so the file uses the original's `GetComputerNameA` fallback
//! `default`.

use crate::vm::CmvsVm;
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
use serde_json::{json, Value};

const ROOT: u32 = 50_740;
const NAME: u32 = ROOT + 1;
const TEXT: u32 = ROOT + 2;
const REMARK: u32 = ROOT + 3;
const CONFIRM: u32 = ROOT + 4;
const ABORT: u32 = ROOT + 5;
const UNDO: u32 = ROOT + 6;
/// Radio buttons 1052..=1055; the middle two are disabled in the template.
const KINDS: [u32; 4] = [ROOT + 7, ROOT + 8, ROOT + 9, ROOT + 10];
const NOTICE: u32 = ROOT + 20;
const COMPUTER: &str = "default";
const TITLE: &str = "テキストデバッグ";
const CHECK_TITLE: &str = "テキストチェック";
const SINGLE_BYTE: &str = "半角文字が含まれています。";
const COMMA: &str = "コメントに半角[,]が含まれています。";
const KIND_LABELS: [&str; 4] = ["名前＆テキスト", "未設定", "未設定", "その他"];

/// One 0x4d8-byte record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Record {
    /// +12: 2 for records the dialog creates.
    origin: i32,
    /// +16: the checked radio button.
    kind: i32,
    /// +20: the script line number.
    line: i32,
    script: Vec<u8>,
    /// +56, +88 and +136: voice, original name and original text.
    voice: Vec<u8>,
    name: Vec<u8>,
    text: Vec<u8>,
    /// +392: the voice again; `0x23a7` returns it.
    voice_copy: Vec<u8>,
    /// +424 and +472: the corrections.
    fixed_name: Vec<u8>,
    fixed_text: Vec<u8>,
    /// +728, with CRLF stored as `\n` in the file.
    remark: Vec<u8>,
}

fn lead(byte: u8) -> bool {
    matches!(byte, 0x81..=0x9f | 0xe0..=0xfc)
}

/// Splits one CSV field at a single-byte comma, stepping over CP932 pairs.
fn field<'a>(input: &mut &'a [u8]) -> &'a [u8] {
    let mut end = 0;
    while end < input.len() && input[end] != b',' {
        end += if lead(input[end]) { 2 } else { 1 };
    }
    let end = end.min(input.len());
    let value = &input[..end];
    *input = input.get(end + 1..).unwrap_or_default();
    value
}

/// `atoi` as 0x14001fb20 reads the numeric fields.
fn number(bytes: &[u8]) -> i32 {
    let text = std::str::from_utf8(bytes).unwrap_or_default().trim();
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
    digits.parse().unwrap_or(0)
}

impl Record {
    /// 0x140019fb0: ten comma-separated fields; the voice also fills +392.
    fn parse(line: &[u8]) -> Self {
        let mut rest = line;
        let origin = number(field(&mut rest));
        let kind = number(field(&mut rest));
        let script = field(&mut rest).to_vec();
        let line = number(field(&mut rest));
        let voice = field(&mut rest).to_vec();
        let name = field(&mut rest).to_vec();
        let text = field(&mut rest).to_vec();
        let fixed_name = field(&mut rest).to_vec();
        let fixed_text = field(&mut rest).to_vec();
        let remark = field(&mut rest).to_vec();
        let mut decoded = Vec::with_capacity(remark.len());
        let mut index = 0;
        while index < remark.len() {
            if lead(remark[index]) {
                decoded.extend_from_slice(&remark[index..(index + 2).min(remark.len())]);
                index += 2;
            } else if remark[index..].starts_with(b"\\n") {
                decoded.extend_from_slice(b"\r\n");
                index += 2;
            } else {
                decoded.push(remark[index]);
                index += 1;
            }
        }
        Self { origin, kind, line, script, voice_copy: voice.clone(), voice, name, text, fixed_name, fixed_text, remark: decoded }
    }

    /// 0x14001a570: `%d,%d,%s,%d,%s,%s,%s,%s,%s,%s\r\n`, CRLF in the remark as `\n`.
    fn write(&self, out: &mut Vec<u8>) {
        let mut remark = Vec::with_capacity(self.remark.len());
        let mut index = 0;
        while index < self.remark.len() {
            if lead(self.remark[index]) {
                remark.extend_from_slice(&self.remark[index..(index + 2).min(self.remark.len())]);
                index += 2;
            } else if self.remark[index..].starts_with(b"\r\n") {
                remark.extend_from_slice(b"\\n");
                index += 2;
            } else {
                remark.push(self.remark[index]);
                index += 1;
            }
        }
        out.extend_from_slice(format!("{},{},", self.origin, self.kind).as_bytes());
        out.extend_from_slice(&self.script);
        out.extend_from_slice(format!(",{},", self.line).as_bytes());
        for value in [&self.voice, &self.name, &self.text, &self.fixed_name, &self.fixed_text] {
            out.extend_from_slice(value);
            out.push(b',');
        }
        out.extend_from_slice(&remark);
        out.extend_from_slice(b"\r\n");
    }
}

/// Parses a `.dvg` file until an empty line or one not starting with a digit (0x14001a340).
pub(super) fn parse_file(bytes: &[u8]) -> Vec<Record> {
    let mut records = Vec::new();
    for line in bytes.split(|&byte| byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if !line.first().is_some_and(u8::is_ascii_digit) {
            break;
        }
        records.push(Record::parse(line));
    }
    records
}

/// The line `0x23a5` describes: +20..+136 of the record list owner.
#[derive(Clone, Default)]
struct Current {
    line: i32,
    script: Vec<u8>,
    voice: Vec<u8>,
    name: Vec<u8>,
    text: Vec<u8>,
}

struct Open {
    kind: i32,
    panel: [f32; 4],
    notices: Vec<&'static str>,
}

#[derive(Default)]
pub(super) struct TextDebug {
    records: Vec<Record>,
    current: Current,
    /// +392, the `<n>` of the file name (`0x23a8`).
    file: u32,
    open: Option<Open>,
}

pub(super) enum Request {
    /// `0x23a4`: read this file and append its records.
    Load(String),
    /// `0x23a6` saved: write these bytes.
    Save(String, Vec<u8>),
    Waiting,
    Done,
}

fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes).0.into_owned()
}

fn encode(text: &str, limit: usize) -> Vec<u8> {
    let mut bytes = encoding_rs::SHIFT_JIS.encode(text).0.into_owned();
    // Edit controls stop at the DDX limit without splitting a pair.
    let mut end = 0;
    while end < bytes.len() {
        let step = if lead(bytes[end]) { 2 } else { 1 };
        if end + step > limit {
            break;
        }
        end += step;
    }
    bytes.truncate(end);
    bytes
}

fn submit(host: &mut dyn HostServices, client: ClientId, ops: Vec<Value>) -> Result<()> {
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({ "ui": ops }))?)?;
    Ok(())
}

impl TextDebug {
    pub fn file_name(&self, prefix: &str) -> String {
        format!("{prefix}{COMPUTER}_{}.dvg", self.file)
    }

    pub fn append(&mut self, records: Vec<Record>) {
        self.records.extend(records);
    }

    fn find(&self, line: i32) -> Option<usize> {
        self.records.iter().position(|record| record.line == line)
    }

    /// `0x23a4..=0x23a8`; `prefix` is the save directory prefix the slots use.
    #[allow(clippy::too_many_arguments)]
    pub fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
        prefix: &str,
    ) -> Result<Request> {
        match opcode {
            0x23a4 => return Ok(Request::Load(self.file_name(prefix))),
            0x23a5 => {
                let [line, voice, name, text] = vm.owner_arguments(opcode)?;
                let string = |reference| vm.text_reference(reference).map(<[u8]>::to_vec).context("CMVS text debug string");
                // 0x140099290 takes the main script's name (+14616) without its extension.
                let mut script = vm.main_script_name().to_vec();
                if let Some(dot) = script.iter().rposition(|&byte| byte == b'.') {
                    script.truncate(dot);
                }
                self.current = Current { line: line as i32, script, voice: string(voice)?, name: string(name)?, text: string(text)? };
                vm.complete_owner_call(4, None)?;
            }
            0x23a6 => {
                if self.open.is_some() {
                    return self.poll(vm, host, client, font, prefix);
                }
                let record = self.find(self.current.line).map(|index| &self.records[index]);
                let kind = record.map_or(0, |record| record.kind);
                let panel = self.open(host, client, canvas, font, record.cloned(), kind)?;
                self.open = Some(Open { kind, panel, notices: Vec::new() });
                vm.wait_owner_call(opcode, None)?;
                return Ok(Request::Waiting);
            }
            0x23a7 => {
                let [line, voice, name, text] = vm.owner_arguments(opcode)?;
                let found = self.find(line as i32).map(|index| self.records[index].clone());
                if let Some(record) = &found {
                    vm.store_owner_string(voice, &record.voice_copy)?;
                    vm.store_owner_string(name, &record.fixed_name)?;
                    vm.store_owner_string(text, &record.fixed_text)?;
                }
                vm.complete_owner_call(4, Some(u32::from(found.is_some())))?;
            }
            _ => {
                let [file] = vm.owner_arguments(opcode)?;
                self.file = file;
                vm.complete_owner_call(1, None)?;
            }
        }
        Ok(Request::Done)
    }

    #[allow(clippy::too_many_arguments)]
    fn open(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        font: &str,
        record: Option<Record>,
        kind: i32,
    ) -> Result<[f32; 4]> {
        // Dialog units of the 316×179 template, at two pixels each.
        let [width, height] = canvas.map(|value| value as f32);
        let panel = [width / 2. - 316., height / 2. - 179., 632., 358.];
        let at = |x: f32, y: f32, w: f32, h: f32| [panel[0] + 2. * x, panel[1] + 2. * y, 2. * w, 2. * h];
        let label = |id: u32, rect: [f32; 4], text: &str| {
            json!({"op":"put","widget":{"id":id,"parent":ROOT,"rect":rect,"kind":"label",
                "text":text,"font":font,"text_size":16,"text_color":[0,0,0,255],"z":1}})
        };
        let edit = |id: u32, rect: [f32; 4], text: &str, limit: usize| {
            json!({"op":"put","widget":{"id":id,"parent":ROOT,"rect":rect,"kind":"text_field",
                "text":text,"max_length":limit,"font":font,"text_size":16,"color":[255,255,255,255],
                "text_color":[0,0,0,255],"text_inset":[4,4],"z":3}})
        };
        let current = &self.current;
        let (name, text, remark) = match &record {
            Some(record) => (decode(&record.fixed_name), decode(&record.fixed_text), decode(&record.remark)),
            None => (decode(&current.name), decode(&current.text), String::new()),
        };
        let mut ops = vec![
            json!({"op":"put","widget":{"id":ROOT,"rect":panel,"kind":"panel","modal":true,
                "color":[240,240,240,255],"clip_children":true}}),
            label(ROOT + 11, at(8., 2., 200., 10.), TITLE),
            label(ROOT + 12, at(11., 16., 33., 8.), "【番号】"),
            label(ROOT + 13, at(46., 13., 45., 14.), &current.line.to_string()),
            label(ROOT + 14, at(92., 16., 49., 8.), "【ファイル】"),
            label(ROOT + 15, at(141., 13., 56., 14.), &decode(&current.script)),
            label(ROOT + 16, at(198., 16., 33., 8.), "【音声】"),
            label(ROOT + 17, at(231., 13., 71., 14.), &decode(&current.voice)),
            label(ROOT + 18, at(11., 32., 73., 8.), "【名前＆テキスト】"),
            label(ROOT + 19, at(11., 93., 33., 8.), "【備考】"),
            edit(NAME, at(14., 45., 94., 14.), &name, 30),
            edit(TEXT, at(14., 62., 288., 26.), &text, 254),
            edit(REMARK, at(14., 105., 288., 48.), &remark, 254),
        ];
        ops.extend(self.kind_buttons(font, panel, kind));
        for (id, x, title) in [(UNDO, 7., "元に戻す(&U)"), (CONFIRM, 96., "確定(&R)"), (ABORT, 170., "中止(&S)")] {
            ops.push(json!({"op":"put","widget":{"id":id,"parent":ROOT,"rect":at(x, 158., 50., 14.),
                "kind":"button","text":title,"font":font,"text_size":16,"color":[220,220,220,255],
                "text_color":[0,0,0,255],"z":2}}));
        }
        ops.push(json!({"op":"focus","id":TEXT}));
        submit(host, client, ops)?;
        Ok(panel)
    }

    fn kind_buttons(&self, font: &str, panel: [f32; 4], kind: i32) -> Vec<Value> {
        let at = |x: f32, y: f32, w: f32, h: f32| [panel[0] + 2. * x, panel[1] + 2. * y, 2. * w, 2. * h];
        [(119., 66.), (188., 35.), (228., 35.), (266., 34.)]
            .into_iter()
            .enumerate()
            .map(|(index, (x, w))| {
                let mark = if kind == index as i32 { "●" } else { "○" };
                let enabled = index == 0 || index == 3;
                json!({"op":"put","widget":{"id":KINDS[index],"parent":ROOT,"rect":at(x, 45., w, 10.),
                    "kind":"button","text":format!("{mark}{}", KIND_LABELS[index]),"font":font,"text_size":14,
                    "color":[240,240,240,255],"enabled":enabled,
                    "text_color":if enabled { [0,0,0,255] } else { [128,128,128,255] },"z":2}})
            })
            .collect()
    }

    fn text_of(host: &mut dyn HostServices, client: ClientId, id: u32, limit: usize) -> Result<Vec<u8>> {
        let widget: Value = serde_json::from_slice(&host.query_engine(client, EngineQuery::UiWidget(id))?)?;
        Ok(encode(widget["text"].as_str().unwrap_or_default(), limit))
    }

    fn poll(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId, font: &str, prefix: &str) -> Result<Request> {
        let open = self.open.as_mut().context("CMVS text debug dialog state")?;
        // A shown check message blocks the dialog like the original MessageBoxA.
        if !open.notices.is_empty() {
            let bytes = host.query_engine(client, EngineQuery::UiSubtreeEvents(NOTICE))?;
            ensure!(bytes.len() <= 64 * 1024, "CMVS dialog event byte budget");
            let events: Vec<Value> = serde_json::from_slice(&bytes)?;
            if events.iter().any(|event| event["kind"] == "dialog_result") {
                open.notices.remove(0);
                if let Some(next) = open.notices.first() {
                    show_notice(host, client, font, next)?;
                }
            }
            vm.wait_owner_call(0x23a6, None)?;
            return Ok(Request::Waiting);
        }
        let bytes = host.query_engine(client, EngineQuery::UiSubtreeEvents(ROOT))?;
        ensure!(bytes.len() <= 64 * 1024, "CMVS dialog event byte budget");
        let events: Vec<Value> = serde_json::from_slice(&bytes)?;
        let mut action = None;
        let previous = open.kind;
        for event in &events {
            if event["kind"] != "activate" {
                continue;
            }
            match event["widget"]["id"].as_u64().map(|id| id as u32) {
                Some(id) if id == KINDS[0] || id == KINDS[3] => {
                    open.kind = if id == KINDS[0] { 0 } else { 3 };
                }
                Some(CONFIRM) => action = Some(true),
                Some(ABORT) => action = Some(false),
                _ => {}
            }
        }
        let (kind, panel) = (open.kind, open.panel);
        if kind != previous {
            submit(host, client, self.kind_buttons(font, panel, kind))?;
        }
        let Some(confirmed) = action else {
            vm.wait_owner_call(0x23a6, None)?;
            return Ok(Request::Waiting);
        };
        if !confirmed {
            return self.close(vm, host, client, None);
        }
        // 0x1400159c0: name and text must be double-byte throughout; the remark may not
        // hold a single-byte comma.
        let name = Self::text_of(host, client, NAME, 30)?;
        let text = Self::text_of(host, client, TEXT, 254)?;
        let remark = Self::text_of(host, client, REMARK, 254)?;
        let wide = |bytes: &[u8]| {
            let mut index = 0;
            while index < bytes.len() {
                if !lead(bytes[index]) {
                    return false;
                }
                index += 2;
            }
            true
        };
        let mut notices = Vec::new();
        if !(wide(&name) && wide(&text)) {
            notices.push(SINGLE_BYTE);
        }
        let mut rest = remark.as_slice();
        if field(&mut rest).len() != remark.len() {
            notices.push(COMMA);
        }
        if !notices.is_empty() {
            show_notice(host, client, font, notices[0])?;
            self.open.as_mut().context("CMVS text debug dialog state")?.notices = notices;
            vm.wait_owner_call(0x23a6, None)?;
            return Ok(Request::Waiting);
        }
        let current = self.current.clone();
        match self.find(current.line) {
            Some(index) => {
                let record = &mut self.records[index];
                record.fixed_name = name;
                record.fixed_text = text;
                record.remark = remark;
                record.kind = kind;
            }
            None => {
                // 0x14001a8f0 keeps the list ordered by line.
                let at = self.records.iter().position(|record| record.line > current.line).unwrap_or(self.records.len());
                self.records.insert(at, Record {
                    origin: 2,
                    kind,
                    line: current.line,
                    script: current.script,
                    voice_copy: current.voice.clone(),
                    voice: current.voice,
                    name: current.name,
                    text: current.text,
                    fixed_name: name,
                    fixed_text: text,
                    remark,
                });
            }
        }
        let mut bytes = Vec::new();
        for record in &self.records {
            record.write(&mut bytes);
        }
        let path = self.file_name(prefix);
        self.close(vm, host, client, (!bytes.is_empty()).then_some((path, bytes)))
    }

    fn close(&mut self, vm: &mut CmvsVm, host: &mut dyn HostServices, client: ClientId, save: Option<(String, Vec<u8>)>) -> Result<Request> {
        submit(host, client, vec![json!({"op":"remove","id":ROOT})])?;
        self.open = None;
        // 0x140087850 reports the completed modal loop, whichever button closed it.
        vm.complete_owner_call(0, Some(1))?;
        Ok(save.map_or(Request::Done, |(path, bytes)| Request::Save(path, bytes)))
    }
}

fn show_notice(host: &mut dyn HostServices, client: ClientId, font: &str, message: &str) -> Result<()> {
    submit(host, client, vec![json!({"op":"open_dialog","dialog":{
        "id":NOTICE,"title":CHECK_TITLE,"message":message,"buttons":[{"id":1,"label":"OK"}],
        "default":1,"cancel":1,"font":font,"text_size":20}})])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_through_the_dvg_line() {
        let record = Record {
            origin: 2,
            kind: 3,
            line: 120,
            script: b"sn1010".to_vec(),
            voice: b"v001".to_vec(),
            name: vec![0x83, 0x41],
            text: vec![0x82, 0xa0],
            voice_copy: b"v001".to_vec(),
            fixed_name: vec![0x83, 0x43],
            fixed_text: vec![0x82, 0xa2],
            remark: b"a\r\nb".to_vec(),
        };
        let mut bytes = Vec::new();
        record.write(&mut bytes);
        assert!(bytes.windows(4).any(|window| window == b"a\\nb"));
        assert_eq!(parse_file(&bytes), vec![record]);
        assert!(parse_file(b"x,1\r\n").is_empty());
    }
}
