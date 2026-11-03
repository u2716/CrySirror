// SPDX-License-Identifier: GPL-2.0-or-later
//! Remaining ONScripter 848a608 script commands (`ScriptParser_command.cpp`,
//! `ONScripter_command.cpp`) over engine layers, glyph runs, UI, Media and Audio.
//! Screen copies use the presenter frame capture; native code and Lua never load.
use super::*;
use crysirror_core::service::MediaRequest;
use native::{SpriteAnimation, SPRITE2, TACHI};

const DRAW: u32 = 0x6400_0000;
const BAR: u32 = 0x6500_0000;
const NUMBER: u32 = 0x6500_1000;
const STRING_SPRITE: u32 = 0x6500_2000;
const MOVIE: u32 = 0x6500_3000;
const INPUT: u32 = 0x6300_0000;
const DIALOG: u32 = 0x6300_1000;
const MENU: u32 = 0x6300_2000;
const LOOKBACK: u32 = 0x6000_0100;
const MEDIA_ID: u32 = 900;
pub(super) const SAVE_SLOTS: i32 = 99;

/// One `savegame` slot: private engine format, not the original `.dat` layout.
#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    time: [i32; 5],
    label: String,
    state: State,
}

fn ints(args: &[Value]) -> Result<Vec<i32>> {
    args.iter().map(Value::int).collect()
}
fn sjis_bytes(text: &str) -> Vec<u8> {
    encoding_rs::SHIFT_JIS.encode(text).0.into_owned()
}
fn sjis_text(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned()
}
fn zenkaku(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32('０' as u32 + (c as u32 - '0' as u32)).unwrap(),
            '-' => 'ー',
            ' ' => '　',
            other => other,
        })
        .collect()
}
/// `itoa2`: full-width digits, right-aligned over `digits` columns when given.
pub(super) fn wide_number(value: i32, digits: Option<usize>) -> String {
    let text = value.to_string();
    let pad = digits.unwrap_or(0).saturating_sub(text.len());
    zenkaku(&format!("{}{text}", " ".repeat(pad)))
}
/// Expand `%n` and `$n` inside a text line, ONScripter's `addStringBuffer`.
pub(super) fn expand_text(text: &str, vars: &Variables, english: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '%' || c == '$' {
            let digits = rest[1..].bytes().take_while(u8::is_ascii_digit).count();
            if digits > 0 {
                if let Ok(n) = rest[1..1 + digits].parse::<u16>() {
                    if c == '%' {
                        let value = *vars.ints.get(&n).unwrap_or(&0);
                        if english {
                            out.push_str(&value.to_string());
                        } else {
                            out.push_str(&wide_number(value, None));
                        }
                    } else if let Some(value) = vars.strings.get(&n) {
                        out.push_str(value);
                    }
                    rest = &rest[1 + digits..];
                    continue;
                }
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}
/// `(base/ruby)` while `rubyon` is active. Returns plain text and ruby spans as
/// (first base character, base length, ruby text).
pub(super) fn split_ruby(text: &str) -> (String, Vec<(usize, usize, String)>) {
    let mut plain = String::new();
    let mut spans = vec![];
    let mut count = 0;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '(' {
            if let Some(end) = rest.find(')') {
                if let Some((base, ruby)) = rest[1..end].split_once('/') {
                    let len = base.chars().count();
                    spans.push((count, len, ruby.to_owned()));
                    plain.push_str(base);
                    count += len;
                    rest = &rest[end + 1..];
                    continue;
                }
            }
        }
        plain.push(c);
        count += 1;
        rest = &rest[c.len_utf8()..];
    }
    (plain, spans)
}
fn hex(value: &str) -> Result<[u8; 4]> {
    ensure!(value.len() == 7 && value.starts_with('#'), "native #RRGGBB required");
    Ok([
        u8::from_str_radix(&value[1..3], 16)?,
        u8::from_str_radix(&value[3..5], 16)?,
        u8::from_str_radix(&value[5..7], 16)?,
        255,
    ])
}

impl NscripterVm {
    fn next_random(&mut self) -> u32 {
        let mut x = self.state.native.rng | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state.native.rng = x;
        (x >> 16) as u32
    }
    fn sprite_rect(&self, id: u32) -> Option<[i64; 4]> {
        let r = &self.state.layers.get(&id)?["rect"];
        Some([r[0].as_i64()?, r[1].as_i64()?, r[2].as_i64()?, r[3].as_i64()?])
    }
    fn remove_layer(&mut self, id: u32) {
        self.state.layers.remove(&id);
        self.state.dirty.remove(&id);
        self.state.native.image_sources.remove(&id);
        self.state.native.animations.remove(&id);
        if self.state.published.contains(&id) {
            self.state.removed.insert(id);
        }
    }
    fn set_visible_range(&mut self, range: std::ops::Range<u32>, visible: bool) {
        let ids: Vec<u32> = self.state.layers.keys().copied().filter(|id| range.contains(id)).collect();
        for id in ids {
            let layer = self.state.layers.get_mut(&id).unwrap();
            if visible {
                if let Some(saved) = layer.as_object_mut().and_then(|o| o.remove("__hidden_visible")) {
                    layer["visible"] = saved;
                }
            } else {
                let saved = layer.get("visible").cloned().unwrap_or(json!(true));
                layer["__hidden_visible"] = saved;
                layer["visible"] = json!(false);
            }
            self.state.dirty.insert(id);
        }
    }
    fn apply_screen_filter(&mut self) {
        let filter = self.state.native.screen_filter.clone();
        let ids: Vec<u32> = self.state.layers.keys().copied().collect();
        for id in ids {
            let layer = self.state.layers.get_mut(&id).unwrap();
            match &filter {
                Some(value) => layer["filter"] = value.clone(),
                None => {
                    if let Some(object) = layer.as_object_mut() {
                        object.remove("filter");
                    }
                }
            }
            self.state.dirty.insert(id);
        }
    }
    fn play_audio(&mut self, path: &str, channel: u32, looping: bool, host: &mut dyn HostServices) -> Result<()> {
        self.state.native.channels.insert(channel, (path.to_owned(), looping));
        self.play(path, channel, looping, host)
    }
    fn stop_channel(&mut self, channel: u32, fade_ms: f32, host: &mut dyn HostServices) -> Result<()> {
        self.state.native.channels.remove(&channel);
        host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms })?;
        Ok(())
    }
    fn set_channel_gain(&mut self, channel: u32, volume: i32, host: &mut dyn HostServices) -> Result<()> {
        let gain = volume.clamp(0, 100) as f32 / 100.;
        host.submit_audio(self.client, AudioRequest::SetGain { channel, gain, fade_ms: 0. })?;
        Ok(())
    }
    fn modal_buttons(&mut self, id: u32, labels: &[String], host: &mut dyn HostServices) -> Result<()> {
        let row = u32::from(self.text_size()) + 12;
        let total = row * labels.len() as u32;
        let width = self.config.canvas[0] * 2 / 3;
        let rect = [
            ((self.config.canvas[0] - width) / 2) as i32,
            (self.config.canvas[1] as i32 - total as i32).max(0) / 2,
            width as i32,
            total as i32,
        ];
        let mut ui = vec![json!({"op":"put","widget":{"id":id,"rect":rect,"kind":"panel","flow":"column","modal":true,"z":31000,"color":[0,0,0,200]}})];
        for (index, text) in labels.iter().enumerate() {
            ui.push(json!({"op":"put","widget":{"id":id+1+index as u32,"parent":id,"rect":[0,0,width,row],"kind":"button","text":text,"font":self.config.font,"text_size":self.text_size(),"color":[0,0,0,0]}}));
        }
        self.submit(host, vec![], ui)
    }
    fn close_modal(&mut self, id: u32, host: &mut dyn HostServices) -> Result<()> {
        self.submit(host, vec![], vec![json!({"op":"remove","id":id})])
    }
    fn save_path(slot: i32) -> Result<VfsPath> {
        ensure!((1..=SAVE_SLOTS).contains(&slot), "native save slot");
        Ok(VfsPath::new(Mount::Save, &format!("save{slot}.json"))?)
    }
    fn read_save(&self, slot: i32, host: &mut dyn HostServices) -> Result<Option<SaveFile>> {
        let name = format!("save{slot}.json");
        let storage = host.storage(self.client)?;
        if storage.metadata(Mount::Save, &name)?.is_none() {
            return Ok(None);
        }
        let bytes = storage.read(&Self::save_path(slot)?)?;
        Ok(Some(serde_json::from_slice(&bytes).context("native save file")?))
    }
    pub(super) fn save_game(&mut self, slot: i32, host: &mut dyn HostServices) -> Result<()> {
        use chrono::{Datelike, Timelike};
        let now = chrono::Local::now();
        let mut state = self.state.clone();
        state.wait = Wait::None;
        let file = SaveFile {
            version: 1,
            time: [now.month() as i32, now.day() as i32, now.hour() as i32, now.minute() as i32, now.year()],
            label: self.state.native.save_label.clone(),
            state,
        };
        let bytes = serde_json::to_vec(&file)?;
        ensure!(bytes.len() <= 64 * 1024 * 1024, "native save byte budget");
        ensure!(self.save_write.is_none(), "native save already pending");
        self.save_write = Some(host.storage(self.client)?.begin_write_atomic(&Self::save_path(slot)?, Arc::from(bytes))?);
        Ok(())
    }
    pub(super) fn load_game(&mut self, slot: i32, host: &mut dyn HostServices) -> Result<bool> {
        let Some(file) = self.read_save(slot, host)? else {
            return Ok(false);
        };
        ensure!(file.version == 1, "native save version");
        let mut ops: Vec<Json> = self.state.published.iter().map(|id| json!({"op":"remove","id":id})).collect();
        if self.state.text_published {
            ops.push(json!({"op":"remove","id":TEXT}));
        }
        self.submit(host, ops, vec![])?;
        let channels = self.state.native.channels.keys().copied().collect::<Vec<_>>();
        for channel in channels {
            host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0. })?;
        }
        self.state = file.state;
        self.state.published.clear();
        self.state.removed.clear();
        self.state.dirty = self.state.layers.keys().copied().collect();
        self.state.text_published = false;
        self.state.native.window_published = false;
        self.state.native.shadow_published = false;
        self.flush(host)?;
        if !self.state.text.is_empty() {
            let text = std::mem::take(&mut self.state.text);
            self.state.glyphs.clear();
            self.state.cursor = CellCursor::at_start();
            self.text(&text, host)?;
            self.state.wait = Wait::None;
        }
        let channels = self.state.native.channels.clone();
        for (channel, (path, looping)) in channels {
            if looping {
                self.play(&path, channel, true, host)?;
            }
        }
        if let Some(label) = self.state.native.load_gosub.clone() {
            self.state.calls.push(Call { next: self.state.pc, params: vec![], cursor: 0 });
            self.state.pc = self.program.as_ref().unwrap().label(&label)?;
        }
        Ok(true)
    }
    fn slot_labels(&self, host: &mut dyn HostServices) -> Result<Vec<String>> {
        let count = self.state.native.declarations.get("savenumber")
            .and_then(|v| v.first()).map(Value::int).transpose()?.unwrap_or(9).clamp(1, SAVE_SLOTS);
        (1..=count).map(|slot| {
            Ok(match self.read_save(slot, host)? {
                Some(file) => format!("{slot:>2}  {:02}/{:02} {:02}:{:02}", file.time[0], file.time[1], file.time[2], file.time[3]),
                None => format!("{slot:>2}  ---"),
            })
        }).collect()
    }
    fn rmenu_items(&self) -> Vec<(String, String)> {
        let declared = self.state.native.declarations.get("rmenu");
        match declared {
            Some(values) if values.len() >= 2 => values.chunks(2).filter_map(|pair| {
                Some((pair[0].string().ok()?, pair.get(1)?.string().ok()?.to_ascii_lowercase()))
            }).collect(),
            _ => [("スキップ", "skip"), ("セーブ", "save"), ("ロード", "load"), ("バックログ", "lookback"), ("文章消去", "windowerase"), ("終了", "end")]
                .into_iter().map(|(a, b)| (a.to_owned(), b.to_owned())).collect(),
        }
    }
    /// Right-click during a click wait: `rmenu` items, ONS `executeSystemMenu`.
    pub(super) fn open_system_menu(&mut self, resume: Wait, host: &mut dyn HostServices) -> Result<()> {
        let labels: Vec<String> = self.rmenu_items().into_iter().map(|(label, _)| label).collect();
        self.modal_buttons(MENU, &labels, host)?;
        self.state.wait = Wait::Menu { kind: MenuKind::System, count: labels.len(), resume: Box::new(resume) };
        Ok(())
    }
    pub(super) fn system_call(&mut self, action: &str, resume: Wait, host: &mut dyn HostServices) -> Result<()> {
        match action {
            "skip" => {
                self.state.native.skip_mode = true;
                self.state.wait = resume;
            }
            "automode" => {
                self.state.native.automode = true;
                self.state.wait = resume;
            }
            "reset" => {
                let define = self.program.as_ref().unwrap().label("define")?;
                self.reset_to(define, host)?;
            }
            "end" => {
                self.state.halted = true;
                self.state.wait = Wait::None;
            }
            "windowerase" => {
                self.hide_text(host)?;
                self.state.wait = Wait::Erased { resume: Box::new(resume) };
            }
            "lookback" => self.open_lookback(resume, host)?,
            "save" | "load" => {
                let labels = self.slot_labels(host)?;
                self.modal_buttons(MENU, &labels, host)?;
                self.state.wait = Wait::Menu {
                    kind: if action == "save" { MenuKind::Save } else { MenuKind::Load },
                    count: labels.len(),
                    resume: Box::new(resume),
                };
            }
            other => bail!("native system call `{other}`"),
        }
        Ok(())
    }
    fn reset_to(&mut self, define: usize, host: &mut dyn HostServices) -> Result<()> {
        let ops: Vec<Json> = self.state.published.iter().map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(host, ops, vec![])?;
        self.clear_text(host)?;
        for channel in std::mem::take(&mut self.state.native.channels).into_keys() {
            host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms: 0. })?;
        }
        let aliases = std::mem::take(&mut self.state.vars.aliases);
        let globals: BTreeMap<u16, i32> = self.state.vars.ints.iter().filter(|(n, _)| **n >= 200).map(|(n, v)| (*n, *v)).collect();
        let strings: BTreeMap<u16, String> = self.state.vars.strings.iter().filter(|(n, _)| **n >= 200).map(|(n, v)| (*n, v.clone())).collect();
        let native = std::mem::take(&mut self.state.native);
        self.state = State::default();
        self.state.vars.aliases = aliases;
        self.state.vars.ints = globals;
        self.state.vars.strings = strings;
        self.state.native.declarations = native.declarations;
        self.state.native.effects = native.effects;
        self.state.native.reveal_masks = native.reveal_masks;
        self.state.native.globals_enabled = native.globals_enabled;
        self.state.native.filelog_enabled = native.filelog_enabled;
        self.state.native.files = native.files;
        self.state.pc = define;
        self.state.define = true;
        if let Some(label) = native.definereset_used.then_some(()).and(None::<String>) {
            self.state.pc = self.program.as_ref().unwrap().label(&label)?;
        }
        Ok(())
    }
    pub(super) fn open_lookback(&mut self, resume: Wait, host: &mut dyn HostServices) -> Result<()> {
        let count = self.state.native.history.len();
        if count == 0 {
            self.state.wait = resume;
            return Ok(());
        }
        self.hide_text(host)?;
        self.state.wait = Wait::Lookback { page: count - 1, resume: Box::new(resume) };
        self.show_lookback(count - 1, host)
    }
    pub(super) fn show_lookback(&mut self, page: usize, host: &mut dyn HostServices) -> Result<()> {
        let text = self.state.native.history.get(page).cloned().context("native lookback page")?;
        let options = self.layout();
        let mut cursor = CellCursor::at_start();
        let layout = layout_cells(&text, &options, &mut cursor)?;
        let size = self.text_size();
        let color = self.state.native.lookback_color.unwrap_or([255, 255, 0, 255]);
        let glyphs: Vec<Json> = layout.glyphs.into_iter().map(|g| json!({"character":g.character,"baseline":[g.position[0],g.position[1]+i32::from(size)],"color":color,"bold":false})).collect();
        let total = glyphs.len();
        self.submit(host, vec![json!({"op":"put","layer":{"id":LOOKBACK,"rect":self.window_rect(),"order":[10002,0],
            "paint":{"kind":"glyph_run","font":self.config.font,"size":size,"color":color,"glyphs":glyphs,"reveal":{"interval_ms":0,"initial_visible":total}}}})], vec![])
    }
    pub(super) fn close_lookback(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.submit(host, vec![json!({"op":"remove","id":LOOKBACK})], vec![])?;
        self.show_text(host)
    }
    /// Poll the waits this module introduced. Returns Some while still waiting.
    pub(super) fn poll_extended(&mut self, wait: Wait, host: &mut dyn HostServices, input: &mut InputFrame) -> Result<Option<VmYield>> {
        const PRESS: u64 = buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT;
        let pressed = input.buttons_pressed & PRESS != 0;
        match wait {
            Wait::Menu { kind, count, resume } => {
                let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(&host.query_engine(self.client, EngineQuery::UiEvents)?)?;
                let selected = events.iter().find(|e| e.kind == "activate" && e.widget.owner == self.client.0
                    && (MENU + 1..=MENU + count as u32).contains(&e.widget.id)).map(|e| (e.widget.id - MENU - 1) as usize);
                let cancel = input.buttons_pressed & buttons::MOUSE_RIGHT != 0;
                let Some(index) = selected else {
                    if cancel {
                        input.buttons_pressed &= !buttons::MOUSE_RIGHT;
                        self.close_modal(MENU, host)?;
                        self.state.wait = *resume;
                        return Ok(None);
                    }
                    self.state.wait = Wait::Menu { kind, count, resume };
                    return Ok(Some(VmYield::WaitForInput));
                };
                input.buttons_pressed &= !PRESS;
                self.close_modal(MENU, host)?;
                match kind {
                    MenuKind::System => {
                        let action = self.rmenu_items()[index].1.clone();
                        self.system_call(&action, *resume, host)?;
                    }
                    MenuKind::Save => {
                        self.state.wait = *resume;
                        self.save_game(index as i32 + 1, host)?;
                    }
                    MenuKind::Load => {
                        if !self.load_game(index as i32 + 1, host)? {
                            self.state.wait = *resume;
                        }
                    }
                }
                Ok(None)
            }
            Wait::Lookback { page, resume } => {
                let previous = input.buttons_pressed & (buttons::WHEEL_UP | buttons::PAGE_UP) != 0;
                let next = input.buttons_pressed & (buttons::WHEEL_DOWN | buttons::PAGE_DOWN) != 0;
                let leave = input.buttons_pressed & (PRESS | buttons::MOUSE_RIGHT) != 0
                    || (next && page + 1 >= self.state.native.history.len());
                if leave {
                    input.buttons_pressed &= !(PRESS | buttons::MOUSE_RIGHT);
                    self.close_lookback(host)?;
                    self.state.wait = *resume;
                    return Ok(None);
                }
                let target = if previous { page.saturating_sub(1) } else if next { page + 1 } else { page };
                if target != page {
                    self.show_lookback(target, host)?;
                }
                self.state.wait = Wait::Lookback { page: target, resume };
                Ok(Some(VmYield::WaitForInput))
            }
            Wait::Erased { resume } => {
                if pressed || input.buttons_pressed & buttons::MOUSE_RIGHT != 0 {
                    input.buttons_pressed &= !(PRESS | buttons::MOUSE_RIGHT);
                    self.show_text(host)?;
                    self.state.wait = *resume;
                    return Ok(None);
                }
                self.state.wait = Wait::Erased { resume };
                Ok(Some(VmYield::WaitForInput))
            }
            Wait::Dialog { variable } => {
                let events: Vec<Json> = serde_json::from_slice(&host.query_engine(self.client, EngineQuery::UiEvents)?)?;
                let result = events.iter().find(|e| e["kind"] == "dialog_result" && e["widget"]["id"] == DIALOG);
                let Some(event) = result else {
                    self.state.wait = Wait::Dialog { variable };
                    return Ok(Some(VmYield::WaitForInput));
                };
                let value = i32::from(event["value"].as_f64().unwrap_or(0.) == 1.);
                self.state.vars.ints.insert(variable, value);
                self.state.wait = Wait::None;
                Ok(None)
            }
            Wait::Input { variable } => {
                let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(&host.query_engine(self.client, EngineQuery::UiEvents)?)?;
                if !events.iter().any(|e| e.kind == "activate" && e.widget.owner == self.client.0 && e.widget.id == INPUT + 2) {
                    self.state.wait = Wait::Input { variable };
                    return Ok(Some(VmYield::WaitForInput));
                }
                let widget: Json = serde_json::from_slice(&host.query_engine(self.client, EngineQuery::UiWidget(INPUT + 1))?)?;
                let text = widget["text"].as_str().unwrap_or_default().to_owned();
                self.state.vars.strings.insert(variable, text);
                self.close_modal(INPUT, host)?;
                self.state.wait = Wait::None;
                input.buttons_pressed &= !PRESS;
                Ok(None)
            }
            Wait::Movie { click } => {
                let info = host.media_info(self.client, MEDIA_ID)?;
                let ended = matches!(info.phase, crysirror_core::service::MediaPhase::Ended | crysirror_core::service::MediaPhase::Failed);
                if ended || (click && pressed) {
                    input.buttons_pressed &= !PRESS;
                    host.media_request(self.client, MediaRequest::Remove { id: MEDIA_ID })?;
                    self.submit(host, vec![json!({"op":"remove","id":MOVIE})], vec![])?;
                    self.state.wait = Wait::None;
                    return Ok(None);
                }
                self.state.wait = Wait::Movie { click };
                Ok(Some(VmYield::BudgetExhausted))
            }
            Wait::Quake { until, amplitude, step } => {
                let now = host.now_ms();
                if now >= until {
                    self.shake_layers([0, 0], host)?;
                    self.state.wait = Wait::None;
                    return Ok(None);
                }
                let phase = (until - now) / step.max(1);
                let sign = if phase % 2 == 0 { 1 } else { -1 };
                self.shake_layers([amplitude[0] * sign, amplitude[1] * sign], host)?;
                self.state.wait = Wait::Quake { until, amplitude, step };
                Ok(Some(VmYield::BudgetExhausted))
            }
            Wait::Capture => {
                use crysirror_core::image_processing::ImageProcessState;
                let (mut job, to_background) = self.frame_capture.take().context("native screen capture job missing")?;
                match job.poll()? {
                    ImageProcessState::Pending => {
                        self.frame_capture = Some((job, to_background));
                        self.state.wait = Wait::Capture;
                        Ok(Some(VmYield::BudgetExhausted))
                    }
                    ImageProcessState::Failed(error) => bail!("native screen capture: {error}"),
                    ImageProcessState::Ready(image) => {
                        let [w, h] = image.size();
                        if to_background {
                            let path = host.present_surface(self.client, BG, w, h, &image.rgba())?;
                            self.layer(BG, json!({"id":BG,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"order":[-10000,0],"paint":{"kind":"image","asset":path.relative()}}));
                        } else {
                            self.screenshot = Some((w, h, image.rgba().to_vec()));
                        }
                        self.state.wait = Wait::None;
                        Ok(None)
                    }
                }
            }
            other => bail!("native wait {other:?} polled in the extended path"),
        }
    }
    fn shake_layers(&mut self, offset: [i32; 2], host: &mut dyn HostServices) -> Result<()> {
        let mut ops = vec![];
        for id in self.state.published.iter().chain(self.state.dirty.iter()) {
            let Some(layer) = self.state.layers.get(id) else { continue };
            let mut layer = layer.clone();
            if let (Some(x), Some(y)) = (layer["rect"][0].as_i64(), layer["rect"][1].as_i64()) {
                layer["rect"][0] = json!(x + i64::from(offset[0]));
                layer["rect"][1] = json!(y + i64::from(offset[1]));
                ops.push(json!({"op":"put","layer":layer}));
            }
        }
        self.submit(host, ops, vec![])
    }
    pub(super) fn update_animations(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.state.native.animations.is_empty() {
            return Ok(());
        }
        let now = host.now_ms();
        let mut ops = vec![];
        for (id, animation) in &self.state.native.animations {
            let Some(layer) = self.state.layers.get(id) else { continue };
            if !self.state.published.contains(id) || animation.sequence.frame_ms == 0 {
                continue;
            }
            let uv = match animation.fixed_cell {
                Some(cell) => {
                    let cells = f32::from(animation.sequence.cells.max(1));
                    [cell as f32 / cells, 0., (cell + 1) as f32 / cells, 1.]
                }
                None => animation.sequence.uv(now)?,
            };
            if layer["uv"] != json!(uv) {
                let mut layer = layer.clone();
                layer["uv"] = json!(uv);
                ops.push(json!({"op":"put","layer":layer}));
            }
        }
        if !ops.is_empty() {
            self.submit(host, ops, vec![])?;
        }
        Ok(())
    }

    /// Commands with no shared state beyond variables and declarations.
    pub(super) fn extended_command(&mut self, command: &str, t: &mut Tokens, host: &mut dyn HostServices) -> Result<bool> {
        match command {
            // ---- declarations with no runtime effect beyond being remembered
            "addkinsoku" | "setkinsoku" | "kinsoku" | "shadedistance" | "underline" | "zenkakko"
            | "soundpressplgin" | "windowchip" | "menu_automode" | "menu_click_def" | "menu_click_page"
            | "menu_full" | "menu_window" | "maxkaisoupage" | "spi" | "arc" | "ns2" | "ns3" | "nsadir"
            | "mode_ext" | "mode_saya" | "labellog" | "kidokuskip" | "kidokumode" | "linepage" | "linepage2"
            | "pagetag" | "checkkey" | "defaultfont" | "useescspc" | "usewheel" | "lookbacksp" | "logsp"
            | "logsp2" | "exec_dll_decl" | "effectcut" | "autosaveoff" | "tateyoko" | "indent"
            | "defaultspeed" | "windowback" | "automode" | "automode_time" | "rmode" | "roff"
            | "clickvoice" | "selectvoice" | "menuselectvoice" | "langen" | "langjp" | "showlangen" | "showlangjp"
            | "getmclick" | "getmouseover" | "getinsert" | "getzxc" | "getfunction" | "getpage" | "getpageup"
            | "getcursor" | "getenter" | "gettab" | "btndown" | "transbtn" | "transmode" | "movemousecursor"
            | "getreadlang" | "english" | "humanorder" | "spreload" | "lookbackflush" | "mp3save" => {
                let mut values = vec![];
                while t.peek().is_some() {
                    if !values.is_empty() {
                        t.comma()?;
                    }
                    let value = match t.peek() {
                        Some(Token::Word(w)) if !self.state.vars.aliases.contains_key(w) => {
                            let w = w.clone();
                            t.at += 1;
                            Value::Str(w)
                        }
                        _ => t.expr(&self.state.vars)?,
                    };
                    values.push(value);
                }
                match command {
                    "english" => self.state.native.english = values.first().map_or(true, |v| v.int().map_or(true, |n| n != 0)),
                    "defaultspeed" => {
                        if let [slow, medium, fast] = values.as_slice() {
                            self.state.native.default_speed = [slow.int()?, medium.int()?, fast.int()?];
                            self.state.speed = medium.int()?.clamp(0, 1000) as u64;
                        }
                    }
                    "lookbackflush" => self.state.native.history.clear(),
                    "maxkaisoupage" => self.state.native.max_history = values.first().map(Value::int).transpose()?.unwrap_or(20).clamp(1, 1000) as usize,
                    "automode" => self.state.native.automode = true,
                    "automode_time" => self.state.native.automode_ms = values.first().map(Value::int).transpose()?.unwrap_or(1000).clamp(0, 600000) as u64,
                    "roff" => self.state.native.right_menu_off = true,
                    "rmode" => self.state.native.right_menu_off = values.first().map(Value::int).transpose()?.unwrap_or(1) == 0,
                    "humanorder" => {}
                    "getmouseover" => {}
                    _ => {}
                }
                self.state.native.declarations.insert(command.into(), values);
            }
            // ---- arithmetic and strings
            "atoi" => {
                let dest = t.variable()?;
                t.comma()?;
                let text = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                let trimmed = text.trim_start();
                let sign = usize::from(trimmed.starts_with(['-', '+']));
                let digits = trimmed[sign..].bytes().take_while(u8::is_ascii_digit).count();
                let value = trimmed[..sign + digits].parse::<i32>().unwrap_or(0);
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "itoa" | "itoa2" => {
                let dest = t.variable()?;
                t.comma()?;
                let value = t.expr(&self.state.vars)?.int()?;
                let digits = if t.peek().is_some() {
                    t.comma()?;
                    Some(t.expr(&self.state.vars)?.int()?.clamp(0, 64) as usize)
                } else {
                    None
                };
                t.end()?;
                let text = if command == "itoa2" {
                    wide_number(value, digits)
                } else {
                    format!("{value:>width$}", width = digits.unwrap_or(0))
                };
                self.state.vars.set(dest, Value::Str(text))?;
            }
            "len" => {
                let dest = t.variable()?;
                t.comma()?;
                let text = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.state.vars.set(dest, Value::Int(sjis_bytes(&text).len() as i32))?;
            }
            "mid" => {
                let dest = t.variable()?;
                t.comma()?;
                let text = sjis_bytes(&t.expr(&self.state.vars)?.string()?);
                t.comma()?;
                let start = t.expr(&self.state.vars)?.int()?.max(0) as usize;
                t.comma()?;
                let len = t.expr(&self.state.vars)?.int()?.max(0) as usize;
                t.end()?;
                let begin = start.min(text.len());
                let end = begin.saturating_add(len).min(text.len());
                self.state.vars.set(dest, Value::Str(sjis_text(&text[begin..end])))?;
            }
            "split" => {
                let text = t.expr(&self.state.vars)?.string()?;
                t.comma()?;
                let delimiter = t.expr(&self.state.vars)?.string()?;
                ensure!(!delimiter.is_empty(), "native split delimiter");
                let mut parts = text.split(delimiter.as_str());
                while t.peek().is_some() {
                    t.comma()?;
                    let dest = self.state.vars.resolve(t.variable()?)?;
                    let part = parts.next().unwrap_or("");
                    let value = if matches!(dest, Token::IntVar(_) | Token::ArrayCell(..)) {
                        Value::Int(part.trim().parse().unwrap_or(0))
                    } else {
                        Value::Str(part.to_owned())
                    };
                    self.state.vars.set(dest, value)?;
                }
            }
            "rnd" | "rnd2" => {
                let dest = t.variable()?;
                t.comma()?;
                let a = t.expr(&self.state.vars)?.int()?;
                let (low, span) = if command == "rnd2" {
                    t.comma()?;
                    let b = t.expr(&self.state.vars)?.int()?;
                    (a, b.wrapping_sub(a).wrapping_add(1))
                } else {
                    (0, a)
                };
                t.end()?;
                ensure!(span > 0, "native random range");
                let value = low + (self.next_random() % span as u32) as i32;
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "sin" | "cos" | "tan" => {
                let dest = t.variable()?;
                t.comma()?;
                let degrees = f64::from(t.expr(&self.state.vars)?.int()?).to_radians();
                t.end()?;
                let value = match command {
                    "sin" => degrees.sin(),
                    "cos" => degrees.cos(),
                    _ => degrees.tan(),
                };
                self.state.vars.set(dest, Value::Int((value * 1000.) as i32))?;
            }
            "time" => {
                use chrono::Timelike;
                let now = chrono::Local::now();
                for (i, value) in [now.hour() as i32, now.minute() as i32, now.second() as i32].into_iter().enumerate() {
                    if i > 0 {
                        t.comma()?;
                    }
                    let dest = t.variable()?;
                    self.state.vars.set(dest, Value::Int(value))?;
                }
                t.end()?;
            }
            "getversion" => {
                let dest = t.variable()?;
                t.end()?;
                self.state.vars.set(dest, Value::Int(296))?;
            }
            "intlimit" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 3 && (0..10000).contains(&a[0]), "native intlimit");
                self.state.vars.limits.insert(a[0] as u16, [a[1], a[2]]);
            }
            "dim" => {
                let Some(Token::ArrayRef { no, dims }) = t.peek().cloned() else {
                    bail!("native dim requires an array expression")
                };
                t.at += 1;
                t.end()?;
                self.state.vars.declare_array(no, &dims)?;
            }
            "mov3" | "mov4" | "mov5" | "mov6" | "mov7" | "mov8" | "mov9" | "mov10" => {
                let count: u16 = command[3..].parse()?;
                let start = self.state.vars.resolve(t.variable()?)?;
                let Token::IntVar(first) = start else {
                    bail!("native {command} integer destination")
                };
                for i in 0..count {
                    t.comma()?;
                    let value = t.expr(&self.state.vars)?;
                    self.state.vars.set(Token::IntVar(first + i), value)?;
                }
                t.end()?;
            }
            "movl" => {
                let dest = t.take()?;
                let Token::ArrayRef { no, dims } = dest else {
                    bail!("native movl array destination")
                };
                let mut flat = self.state.vars.array_cell(no.clone(), &dims)?;
                while t.peek().is_some() {
                    t.comma()?;
                    let value = t.expr(&self.state.vars)?.int()?;
                    self.state.vars.set_cell(no.clone(), flat, value)?;
                    flat += 1;
                }
            }
            "tablegoto" => {
                let index = t.expr(&self.state.vars)?.int()?;
                let mut labels = vec![];
                while t.peek().is_some() {
                    t.comma()?;
                    labels.push(t.expr(&self.state.vars)?.string()?);
                }
                let label = usize::try_from(index).ok().and_then(|i| labels.get(i)).context("native tablegoto index")?;
                self.state.pc = self.program.as_ref().unwrap().label(label)?;
            }
            "jumpf" | "jumpb" => {
                t.end()?;
                let program = self.program.as_ref().unwrap();
                let pc = self.state.pc;
                self.state.pc = if command == "jumpf" {
                    program.tildes.iter().copied().find(|n| *n >= pc).context("native jumpf without ~")?
                } else {
                    program.tildes.iter().rev().copied().find(|n| *n < pc).context("native jumpb without ~")?
                };
            }
            "getparam2" => {
                let frame = self.state.calls.last_mut().context("getparam2 outside gosub")?;
                let value = frame.params.get(frame.cursor).cloned().context("missing gosub parameter")?;
                frame.cursor += 1;
                let dest = t.variable()?;
                self.state.vars.set(dest, value)?;
            }
            "getret" => {
                let dest = t.variable()?;
                t.end()?;
                self.state.vars.set(dest, Value::Int(0))?;
            }
            "getreg" => {
                let dest = t.variable()?;
                t.comma()?;
                let _key = t.expr(&self.state.vars)?;
                t.comma()?;
                let _value = t.expr(&self.state.vars)?;
                t.end()?;
                self.state.vars.set(dest, Value::Str(String::new()))?;
            }
            "fileexist" => {
                let dest = t.variable()?;
                t.comma()?;
                let name = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                let found = self.resolve_asset(&name, host).is_ok();
                self.state.vars.set(dest, Value::Int(i32::from(found)))?;
            }
            "savedir" | "definereset" | "savename_declared" => {
                let _ = Self::args(t, &self.state.vars)?;
                if command == "definereset" {
                    self.state.native.definereset_used = true;
                }
            }
            // ---- input state
            "gettimer" => {
                let dest = t.variable()?;
                t.end()?;
                let value = host.now_ms().saturating_sub(self.state.native.timer).min(i32::MAX as u64) as i32;
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "getbtntimer" => {
                let dest = t.variable()?;
                t.end()?;
                let value = host.now_ms().saturating_sub(self.state.native.button_started).min(i32::MAX as u64) as i32;
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "getmousepos" | "getcursorpos" | "getcursorpos2" => {
                let x = t.variable()?;
                t.comma()?;
                let y = t.variable()?;
                t.end()?;
                let (vx, vy) = if command == "getmousepos" {
                    (self.state.native.pointer[0], self.state.native.pointer[1])
                } else {
                    let r = self.window_rect();
                    (r[0] + self.state.cursor.position[0], r[1] + self.state.cursor.position[1])
                };
                self.state.vars.set(x, Value::Int(vx))?;
                self.state.vars.set(y, Value::Int(vy))?;
            }
            "isdown" | "isfull" | "ispage" | "isskip" | "checkpage" => {
                let dest = t.variable()?;
                let value = match command {
                    "isdown" => i32::from(self.state.native.mouse_down),
                    "isfull" => 0,
                    "ispage" => i32::from(matches!(self.state.wait, Wait::Click { page: true })),
                    "isskip" => {
                        if self.state.native.skip_mode { 1 } else if self.state.native.automode { 2 } else { 0 }
                    }
                    _ => {
                        t.comma()?;
                        let page = t.expr(&self.state.vars)?.int()?;
                        i32::from(usize::try_from(page).is_ok_and(|p| p <= self.state.native.history.len()))
                    }
                };
                t.end()?;
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "getlog" | "getlogtext" => {
                let dest = t.variable()?;
                t.comma()?;
                let page = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let count = self.state.native.history.len() as i32;
                let text = self.state.native.history.get((count - page - 1).max(0) as usize).cloned().unwrap_or_default();
                self.state.vars.set(dest, Value::Str(text))?;
            }
            "gettext" => {
                let dest = t.variable()?;
                t.end()?;
                let text = self.state.text.clone();
                self.state.vars.set(dest, Value::Str(text))?;
            }
            "gettag" | "gettaglog" => {
                while t.peek().is_some() {
                    let dest = self.state.vars.resolve(t.variable()?)?;
                    let empty = if matches!(dest, Token::StrVar(_)) { Value::Str(String::new()) } else { Value::Int(0) };
                    self.state.vars.set(dest, empty)?;
                    if t.peek().is_some() {
                        t.comma()?;
                    }
                }
            }
            "textgosub" | "pretextgosub" => {
                let label = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.program.as_ref().unwrap().label(&label)?;
                if command == "textgosub" {
                    self.state.native.text_gosub = Some(label);
                } else {
                    self.state.native.pretext_gosub = Some(label);
                }
            }
            // ---- text
            "locate" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2, "native locate");
                let size = i32::from(self.text_size());
                let spacing = self.state.native.spacing;
                self.state.cursor.position = [a[0] * (size + spacing[0]), a[1] * (size + spacing[1])];
            }
            "puttext" => {
                let text = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.text(&text, host)?;
            }
            "texthide" => {
                t.end()?;
                self.hide_text(host)?;
            }
            "textshow" => {
                t.end()?;
                self.show_text(host)?;
            }
            "erasetextwindow" => {
                let value = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                self.state.native.erase_window = value != 0;
            }
            "textspeeddefault" => {
                t.end()?;
                self.state.speed = self.state.native.default_speed[1].clamp(0, 1000) as u64;
            }
            "rubyon" | "rubyoff" => {
                let _ = Self::args(t, &self.state.vars)?;
                self.state.native.ruby = command == "rubyon";
            }
            "setwindow2" => {
                let value = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                if value.starts_with('#') {
                    self.state.native.window_color = Some(hex(&value)?);
                } else {
                    let rect = self.state.native.window_rect.context("native setwindow2 before setwindow")?;
                    self.state.native.window_color = None;
                    self.start_image(native::WINDOW, &value, json!({"rect":[rect[0],rect[1],null,null],"order":[9998,0],"visible":false}), false, 0, host)?;
                }
            }
            "lookbackcolor_set" => {
                let value = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.state.native.lookback_color = Some(hex(&value)?);
            }
            // ---- sprites
            "amsp" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 3 || a.len() == 4, "native amsp");
                let id = SPRITE + a[0] as u32;
                let layer = self.state.layers.get_mut(&id).context("amsp sprite missing")?;
                layer["rect"][0] = json!(a[1]);
                layer["rect"][1] = json!(a[2]);
                if let Some(opacity) = a.get(3) {
                    layer["opacity"] = json!((*opacity).clamp(0, 255));
                }
                self.state.dirty.insert(id);
            }
            "allsphide" | "allspresume" => {
                t.end()?;
                self.set_visible_range(SPRITE..SPRITE + 1000, command == "allspresume");
            }
            "allsp2hide" | "allsp2resume" => {
                t.end()?;
                self.set_visible_range(SPRITE2..SPRITE2 + 1000, command == "allsp2resume");
            }
            "lsp2" | "lsph2" | "lsp2add" | "lsph2add" | "lsp2sub" | "lsph2sub" => {
                let args = Self::args(t, &self.state.vars)?;
                ensure!(args.len() == 7 || args.len() == 8, "native {command} arguments");
                let n = args[0].int()?;
                ensure!((0..1000).contains(&n), "native sprite2 number");
                let name = args[1].string()?;
                let [x, y, sx, sy, rot] = [args[2].int()?, args[3].int()?, args[4].int()?, args[5].int()?, args[6].int()?];
                let opacity = args.get(7).map(Value::int).transpose()?.unwrap_or(255).clamp(0, 255);
                let blend = if command.ends_with("add") { "add" } else if command.ends_with("sub") { "subtract" } else { "normal" };
                self.start_image(SPRITE2 + n as u32, &name, json!({
                    "rect":[x,y,null,null],"order":[1000-n,1],"opacity":opacity,"blend":blend,
                    "visible":!command.starts_with("lsph"),
                    "transform":{"scale_x":sx as f32/100.,"scale_y":sy as f32/100.,"rotate":(rot as f32).to_radians()},
                }), false, 0, host)?;
            }
            "msp2" | "amsp2" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 6 || a.len() == 7, "native {command}");
                let id = SPRITE2 + a[0] as u32;
                let relative = command == "msp2";
                let layer = self.state.layers.get_mut(&id).context("sprite2 missing")?;
                let add = |value: &Json, delta: i64, relative: bool| if relative { value.as_i64().unwrap_or(0) + delta } else { delta };
                let center = [add(&layer["rect"][0], i64::from(a[1]), relative), add(&layer["rect"][1], i64::from(a[2]), relative)];
                layer["rect"][0] = json!(center[0]);
                layer["rect"][1] = json!(center[1]);
                let transform = &mut layer["transform"];
                let scale = |value: &Json, delta: i32| if relative { value.as_f64().unwrap_or(1.) + f64::from(delta) / 100. } else { f64::from(delta) / 100. };
                transform["scale_x"] = json!(scale(&transform["scale_x"], a[3]));
                transform["scale_y"] = json!(scale(&transform["scale_y"], a[4]));
                let rotate = f64::from(a[5]).to_radians();
                transform["rotate"] = json!(if relative { transform["rotate"].as_f64().unwrap_or(0.) + rotate } else { rotate });
                if let Some(opacity) = a.get(6) {
                    let base = if relative { layer["opacity"].as_i64().unwrap_or(255) } else { 0 };
                    layer["opacity"] = json!((base + i64::from(*opacity)).clamp(0, 255));
                }
                self.state.dirty.insert(id);
            }
            "vsp2" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2, "native vsp2");
                let id = SPRITE2 + a[0] as u32;
                self.state.layers.get_mut(&id).context("vsp2 sprite missing")?["visible"] = json!(a[1] == 1);
                self.state.dirty.insert(id);
            }
            "csp2" => {
                let n = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let ids: Vec<u32> = if n == -1 {
                    self.state.layers.keys().copied().filter(|id| (SPRITE2..SPRITE2 + 1000).contains(id)).collect()
                } else {
                    vec![SPRITE2 + n as u32]
                };
                for id in ids {
                    self.remove_layer(id);
                }
            }
            "cell" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2, "native cell");
                let id = SPRITE + a[0] as u32;
                let animation = self.state.native.animations.entry(id).or_insert_with(|| SpriteAnimation::still(1));
                animation.fixed_cell = Some(a[1].max(0) as u32);
            }
            "getspsize" => {
                let n = t.expr(&self.state.vars)?.int()?;
                t.comma()?;
                let w = t.variable()?;
                t.comma()?;
                let h = t.variable()?;
                let id = SPRITE + n as u32;
                let rect = self.sprite_rect(id).unwrap_or([0; 4]);
                self.state.vars.set(w, Value::Int(rect[2] as i32))?;
                self.state.vars.set(h, Value::Int(rect[3] as i32))?;
                if t.peek().is_some() {
                    t.comma()?;
                    let cells = t.variable()?;
                    let count = self.state.native.animations.get(&id).map_or(1, |a| i32::from(a.sequence.cells));
                    self.state.vars.set(cells, Value::Int(count))?;
                }
                t.end()?;
            }
            "getsppos" => {
                let n = t.expr(&self.state.vars)?.int()?;
                t.comma()?;
                let x = t.variable()?;
                t.comma()?;
                let y = t.variable()?;
                t.end()?;
                let rect = self.sprite_rect(SPRITE + n as u32).unwrap_or([0; 4]);
                self.state.vars.set(x, Value::Int(rect[0] as i32))?;
                self.state.vars.set(y, Value::Int(rect[1] as i32))?;
            }
            "getspmode" => {
                let dest = t.variable()?;
                t.comma()?;
                let n = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let visible = self.state.layers.get(&(SPRITE + n as u32)).is_some_and(|l| l["visible"].as_bool().unwrap_or(true));
                self.state.vars.set(dest, Value::Int(i32::from(visible)))?;
            }
            "humanz" => {
                let z = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                self.state.native.human_z = Some(z);
                for (no, id) in (TACHI..TACHI + 3).enumerate() {
                    if let Some(layer) = self.state.layers.get_mut(&id) {
                        layer["order"] = json!([1000 - z, no]);
                        self.state.dirty.insert(id);
                    }
                }
            }
            "strsp" | "strsph" | "spstr" => {
                if command == "spstr" {
                    let text = t.expr(&self.state.vars)?.string()?;
                    t.end()?;
                    self.state.native.declarations.insert("spstr".into(), vec![Value::Str(text)]);
                    return Ok(true);
                }
                let args = Self::args(t, &self.state.vars)?;
                ensure!(args.len() >= 12, "native strsp arguments");
                let n = args[0].int()?;
                let text = args[1].string()?;
                let v: Vec<i32> = args[2..12].iter().map(Value::int).collect::<Result<_>>()?;
                let (x, y, columns, _rows, w, h, pitch_x, pitch_y, bold) = (v[0], v[1], v[2].max(1), v[3], v[4], v[5], v[6], v[7], v[8]);
                let color = match args.get(12) {
                    Some(c) => hex(&c.string()?)?,
                    None => [255; 4],
                };
                let mut glyphs = vec![];
                let (mut column, mut row) = (0, 0);
                for character in text.chars() {
                    if character == '\n' || column >= columns {
                        column = 0;
                        row += 1;
                        if character == '\n' {
                            continue;
                        }
                    }
                    glyphs.push(json!({"character":character.to_string(),"baseline":[column*(w+pitch_x),row*(h+pitch_y)+h],"color":color,"bold":bold!=0}));
                    column += 1;
                }
                let total = glyphs.len();
                let id = STRING_SPRITE + n as u32;
                self.layer(id, json!({"id":id,"rect":[x,y,columns*(w+pitch_x),(row+1)*(h+pitch_y)],"order":[1000-n,2],"visible":command=="strsp",
                    "paint":{"kind":"glyph_run","font":self.config.font,"size":h,"color":color,"glyphs":glyphs,"reveal":{"interval_ms":0,"initial_visible":total}}}));
            }
            "bar" => {
                let a = Self::args(t, &self.state.vars)?;
                ensure!(a.len() == 8, "native bar");
                let v: Vec<i32> = a[..7].iter().map(Value::int).collect::<Result<_>>()?;
                let color = hex(&a[7].string()?)?;
                let width = if v[6] > 0 { (v[1].clamp(0, v[6]) as i64 * i64::from(v[4]) / i64::from(v[6])) as i32 } else { 0 };
                let id = BAR + v[0] as u32;
                self.layer(id, json!({"id":id,"rect":[v[2],v[3],width,v[5]],"order":[9000,v[0]],"paint":{"kind":"solid","color":color}}));
            }
            "barclear" => {
                t.end()?;
                let ids: Vec<u32> = self.state.layers.keys().copied().filter(|id| (BAR..BAR + 0x1000).contains(id)).collect();
                for id in ids {
                    self.remove_layer(id);
                }
            }
            "prnum" => {
                let a = Self::args(t, &self.state.vars)?;
                ensure!(a.len() == 7, "native prnum");
                let v: Vec<i32> = a[..6].iter().map(Value::int).collect::<Result<_>>()?;
                let color = hex(&a[6].string()?)?;
                let text = wide_number(v[1], Some(3));
                let glyphs: Vec<Json> = text.chars().enumerate().map(|(i, c)| json!({"character":c.to_string(),"baseline":[i as i32*v[4],v[5]],"color":color,"bold":false})).collect();
                let total = glyphs.len();
                let id = NUMBER + v[0] as u32;
                self.layer(id, json!({"id":id,"rect":[v[2],v[3],v[4]*3,v[5]*2],"order":[9000,1000+v[0]],
                    "paint":{"kind":"glyph_run","font":self.config.font,"size":v[5],"color":color,"glyphs":glyphs,"reveal":{"interval_ms":0,"initial_visible":total}}}));
            }
            "prnumclear" => {
                t.end()?;
                let ids: Vec<u32> = self.state.layers.keys().copied().filter(|id| (NUMBER..NUMBER + 0x1000).contains(id)).collect();
                for id in ids {
                    self.remove_layer(id);
                }
            }
            // ---- whole-screen effects
            "monocro" | "nega" => {
                let value = t.expr(&self.state.vars)?;
                t.end()?;
                self.state.native.screen_filter = if command == "nega" {
                    (value.int()? != 0).then(|| json!({"invert":1.0}))
                } else {
                    match value {
                        Value::Str(s) if s == "off" => None,
                        Value::Str(s) => {
                            let c = hex(&s)?;
                            Some(json!({"mono":1.0,"color_rate":0.5,"color":[c[0] as f32/255.,c[1] as f32/255.,c[2] as f32/255.]}))
                        }
                        Value::Int(0) => None,
                        Value::Int(_) => Some(json!({"mono":1.0})),
                    }
                };
                self.apply_screen_filter();
            }
            "quake" | "quakex" | "quakey" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2 && a[1] >= 0, "native quake");
                let step = (u64::from(a[1].unsigned_abs()) / (2 * u64::from(a[0].unsigned_abs()).max(1))).max(16);
                let amplitude = match command {
                    "quakex" => [a[0] * 4, 0],
                    "quakey" => [0, a[0] * 4],
                    _ => [a[0] * 4, a[0] * 4],
                };
                self.flush(host)?;
                self.state.wait = Wait::Quake { until: host.now_ms().saturating_add(a[1] as u64), amplitude, step };
            }
            // ---- draw family, as an overlay composed from engine layers
            "drawclear" => {
                t.end()?;
                let ids: Vec<u32> = self.state.layers.keys().copied().filter(|id| (DRAW..DRAW + 0x1000).contains(id)).collect();
                for id in ids {
                    self.remove_layer(id);
                }
                self.state.native.draw_next = 0;
            }
            "drawfill" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 3, "native drawfill");
                let id = DRAW + self.state.native.draw_next;
                self.state.native.draw_next += 1;
                let color = [a[0].clamp(0, 255), a[1].clamp(0, 255), a[2].clamp(0, 255), 255];
                self.layer(id, json!({"id":id,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"order":[20000,id-DRAW],"paint":{"kind":"solid","color":color}}));
            }
            "drawsp" | "drawsp2" | "drawsp3" | "drawbg" | "drawbg2" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                let id = DRAW + self.state.native.draw_next;
                self.state.native.draw_next += 1;
                let (source, rect, opacity, transform) = match command {
                    "drawbg" | "drawbg2" => {
                        let source = self.state.layers.get(&BG).context("native drawbg without background")?.clone();
                        let (x, y) = if command == "drawbg2" { (a[0], a[1]) } else { (0, 0) };
                        (source, [x, y], 255, Json::Null)
                    }
                    "drawsp" => {
                        ensure!(a.len() == 5, "native drawsp");
                        (self.state.layers.get(&(SPRITE + a[0] as u32)).context("drawsp sprite missing")?.clone(), [a[3], a[4]], a[2].clamp(0, 255), Json::Null)
                    }
                    "drawsp2" => {
                        ensure!(a.len() == 8, "native drawsp2");
                        (self.state.layers.get(&(SPRITE + a[0] as u32)).context("drawsp2 sprite missing")?.clone(), [a[3], a[4]], a[2].clamp(0, 255),
                         json!({"scale_x":a[5] as f32/100.,"scale_y":a[6] as f32/100.,"rotate":(a[7] as f32).to_radians()}))
                    }
                    _ => bail!("native drawsp3 needs a shared affine matrix mapping"),
                };
                let mut layer = source;
                layer["id"] = json!(id);
                layer["order"] = json!([20000, id - DRAW]);
                layer["opacity"] = json!(opacity);
                layer["visible"] = json!(true);
                layer["rect"][0] = json!(rect[0]);
                layer["rect"][1] = json!(rect[1]);
                if !transform.is_null() {
                    layer["transform"] = transform;
                }
                self.layer(id, layer);
            }
            "drawtext" => {
                t.end()?;
                self.show_text(host)?;
            }
            "draw" => {
                t.end()?;
                self.flush(host)?;
            }
            "blt" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 8, "native blt");
                let atlas = self.state.native.button_atlas.clone().context("native blt requires btndef")?;
                let [w, h] = host.image_size(self.client, &atlas)?;
                let id = DRAW + 0xf00;
                self.layer(id, json!({"id":id,"rect":[a[0],a[1],a[2],a[3]],"order":[19000,0],
                    "uv":[a[4] as f32/w as f32,a[5] as f32/h as f32,(a[4]+a[6]) as f32/w as f32,(a[5]+a[7]) as f32/h as f32],
                    "paint":{"kind":"image","asset":atlas}}));
                self.flush(host)?;
            }
            // ---- buttons
            "spbtn" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2, "native spbtn");
                let rect = self.sprite_rect(SPRITE + a[0] as u32).context("spbtn sprite missing")?;
                self.state.native.buttons.push(native::NativeButton { number: a[1], rect: rect.map(|v| v as i32), source: [0, 0], sprite: Some(SPRITE + a[0] as u32) });
            }
            "exbtn" | "exbtn_d" => {
                let args = Self::args(t, &self.state.vars)?;
                let (sprite, number) = if command == "exbtn_d" {
                    (None, 0)
                } else {
                    ensure!(args.len() == 3, "native exbtn");
                    (Some(args[0].int()?), args[1].int()?)
                };
                if let Some(sprite) = sprite {
                    let rect = self.sprite_rect(SPRITE + sprite as u32).context("exbtn sprite missing")?;
                    self.state.native.buttons.push(native::NativeButton { number, rect: rect.map(|v| v as i32), source: [0, 0], sprite: Some(SPRITE + sprite as u32) });
                }
            }
            "cellcheckspbtn" | "cellcheckexbtn" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() >= 2, "native {command}");
                let rect = self.sprite_rect(SPRITE + a[0] as u32).context("cellcheck sprite missing")?;
                self.state.native.buttons.push(native::NativeButton { number: a[1], rect: rect.map(|v| v as i32), source: [0, 0], sprite: Some(SPRITE + a[0] as u32) });
            }
            "existspbtn" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(!a.is_empty(), "native existspbtn");
                if a[0] >= 0 && self.state.native.buttons.iter().all(|b| b.sprite != Some(SPRITE + a[0] as u32)) {
                    if let Some(rect) = self.sprite_rect(SPRITE + a[0] as u32) {
                        self.state.native.buttons.push(native::NativeButton { number: a.get(1).copied().unwrap_or(a[0]), rect: rect.map(|v| v as i32), source: [0, 0], sprite: Some(SPRITE + a[0] as u32) });
                    }
                }
            }
            "bclear" | "bdef" => {
                if command == "bdef" {
                    let name = t.expr(&self.state.vars)?.string()?;
                    t.end()?;
                    self.state.native.button_atlas = Some(self.resolve_asset(&name, host)?);
                } else {
                    t.end()?;
                }
                self.state.native.buttons.clear();
            }
            "btime" | "btntime" | "btntime2" => {
                let ms = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                self.state.native.button_timeout = (ms > 0).then_some(ms as u64);
            }
            "btnwait2" | "textbtnwait" | "selectbtnwait" | "bexec" => {
                let Token::IntVar(variable) = self.state.vars.resolve(t.variable()?)? else {
                    bail!("native {command} integer destination")
                };
                t.end()?;
                if command != "textbtnwait" {
                    self.hide_text(host)?;
                }
                self.state.native.button_started = host.now_ms();
                self.state.native.keep_buttons = command == "btnwait2";
                self.show_buttons(host)?;
                self.state.wait = Wait::Buttons { variable };
            }
            "getcselnum" => {
                let dest = t.variable()?;
                t.end()?;
                let count = self.state.native.csel.len() as i32;
                self.state.vars.set(dest, Value::Int(count))?;
            }
            "getcselstr" => {
                let dest = t.variable()?;
                t.comma()?;
                let index = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let text = usize::try_from(index).ok().and_then(|i| self.state.native.csel.get(i)).map(|c| c.0.clone()).unwrap_or_default();
                self.state.vars.set(dest, Value::Str(text))?;
            }
            "csel" => {
                self.state.native.csel.clear();
                loop {
                    let text = t.expr(&self.state.vars)?.string()?;
                    t.comma()?;
                    let label = t.expr(&self.state.vars)?.string()?;
                    self.program.as_ref().unwrap().label(&label)?;
                    self.state.native.csel.push((text, label));
                    if t.peek().is_none() {
                        break;
                    }
                    t.comma()?;
                }
                let targets: Vec<String> = self.state.native.csel.iter().map(|c| c.1.clone()).collect();
                let labels: Vec<String> = self.state.native.csel.iter().map(|c| c.0.clone()).collect();
                self.show_choices(&labels, targets, None, false, host)?;
            }
            "cselbtn" => {
                let a = Self::args(t, &self.state.vars)?;
                ensure!(a.len() == 3, "native cselbtn");
                let index = a[0].int()? as usize;
                let (text, _) = self.state.native.csel.get(index).cloned().context("native cselbtn index")?;
                let number = a[1].int()?;
                let id = STRING_SPRITE + 0x800 + index as u32;
                let size = i32::from(self.text_size());
                let glyphs: Vec<Json> = text.chars().enumerate().map(|(i, c)| json!({"character":c.to_string(),"baseline":[i as i32*size,size],"color":[255,255,255,255],"bold":false})).collect();
                let total = glyphs.len();
                let rect = [0, a[2].int()?, size * total as i32, size + 8];
                self.layer(id, json!({"id":id,"rect":rect,"order":[9100,index],"paint":{"kind":"glyph_run","font":self.config.font,"size":size,"color":[255,255,255,255],"glyphs":glyphs,"reveal":{"interval_ms":0,"initial_visible":total}}}));
                self.state.native.buttons.push(native::NativeButton { number, rect, source: [0, 0], sprite: Some(id) });
            }
            "cselgoto" => {
                let index = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let label = usize::try_from(index).ok().and_then(|i| self.state.native.csel.get(i)).map(|c| c.1.clone()).context("native cselgoto index")?;
                self.state.pc = self.program.as_ref().unwrap().label(&label)?;
            }
            "yesnobox" | "okcancelbox" => {
                let Token::IntVar(variable) = self.state.vars.resolve(t.variable()?)? else {
                    bail!("native {command} integer destination")
                };
                t.comma()?;
                let message = t.expr(&self.state.vars)?.string()?;
                let title = if t.peek().is_some() {
                    t.comma()?;
                    t.expr(&self.state.vars)?.string()?
                } else {
                    String::new()
                };
                t.end()?;
                let (first, second) = if command == "yesnobox" { ("はい", "いいえ") } else { ("OK", "キャンセル") };
                self.submit(host, vec![], vec![json!({"op":"open_dialog","dialog":{"id":DIALOG,"title":title,"message":message,
                    "buttons":[{"id":1,"label":first},{"id":0,"label":second}],"default":1,"font":self.config.font}})])?;
                self.state.wait = Wait::Dialog { variable };
            }
            "input" => {
                let Token::StrVar(variable) = self.state.vars.resolve(t.variable()?)? else {
                    bail!("native input string destination")
                };
                let args = {
                    t.comma()?;
                    Self::args(t, &self.state.vars)?
                };
                let title = args.first().map(Value::string).transpose()?.unwrap_or_default();
                let limit = args.get(1).map(Value::int).transpose()?.unwrap_or(32).clamp(1, 1024);
                let width = self.config.canvas[0] as i32 * 2 / 3;
                let x = (self.config.canvas[0] as i32 - width) / 2;
                let y = self.config.canvas[1] as i32 / 3;
                let current = self.state.vars.strings.get(&variable).cloned().unwrap_or_default();
                let size = self.text_size();
                self.submit(host, vec![], vec![
                    json!({"op":"put","widget":{"id":INPUT,"rect":[x,y,width,160],"kind":"panel","modal":true,"z":31000,"color":[0,0,0,220]}}),
                    json!({"op":"put","widget":{"id":INPUT+3,"parent":INPUT,"kind":"label","rect":[8,8,width-16,36],"text":title,"font":self.config.font,"text_size":size}}),
                    json!({"op":"put","widget":{"id":INPUT+1,"parent":INPUT,"kind":"text_field","rect":[8,52,width-16,40],"text":current,"max_length":limit,"font":self.config.font,"text_size":size}}),
                    json!({"op":"put","widget":{"id":INPUT+2,"parent":INPUT,"kind":"button","rect":[width-128,104,120,40],"text":"OK","font":self.config.font,"text_size":size}}),
                ])?;
                self.state.wait = Wait::Input { variable };
            }
            // ---- audio
            "mp3" | "mp3loop" | "bgm_mp3" => {
                let path = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.play_audio(&path, 100, command == "mp3loop", host)?;
            }
            "mp3stop" | "loopbgmstop" => {
                t.end()?;
                self.stop_channel(100, 0., host)?;
            }
            "loopbgm" => {
                let intro = t.expr(&self.state.vars)?.string()?;
                t.comma()?;
                let looped = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.state.native.loop_after_intro = Some(looped);
                self.play_audio(&intro, 100, false, host)?;
            }
            "bgmfadeout" | "mp3fadeout" | "bgmfadein" | "mp3fadein" => {
                let ms = t.expr(&self.state.vars)?.int()?.clamp(0, 600000);
                t.end()?;
                if command.ends_with("out") {
                    self.state.native.fade_out_ms = ms as u64;
                } else {
                    self.state.native.fade_in_ms = ms as u64;
                }
            }
            "bgmvol" | "mp3vol" | "sevol" | "voicevol" | "defmp3vol" | "defsevol" | "defvoicevol" => {
                let volume = t.expr(&self.state.vars)?.int()?.clamp(0, 100);
                t.end()?;
                match command {
                    "bgmvol" | "mp3vol" | "defmp3vol" => {
                        self.state.native.volumes[0] = volume;
                        self.set_channel_gain(100, volume, host)?;
                    }
                    "sevol" | "defsevol" => self.state.native.volumes[1] = volume,
                    _ => self.state.native.volumes[2] = volume,
                }
            }
            "chvol" => {
                let a = ints(&Self::args(t, &self.state.vars)?)?;
                ensure!(a.len() == 2 && (0..50).contains(&a[0]), "native chvol");
                self.set_channel_gain(a[0] as u32, a[1], host)?;
            }
            "getbgmvol" | "getmp3vol" | "getsevol" | "getvoicevol" => {
                let dest = t.variable()?;
                t.end()?;
                let at = match command {
                    "getsevol" => 1,
                    "getvoicevol" => 2,
                    _ => 0,
                };
                let value = self.state.native.volumes[at];
                self.state.vars.set(dest, Value::Int(value))?;
            }
            "dwave" | "dwaveloop" => {
                let channel = t.expr(&self.state.vars)?.int()?;
                ensure!((0..50).contains(&channel), "native dwave channel");
                t.comma()?;
                let path = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.play_audio(&path, channel as u32 + 1, command == "dwaveloop", host)?;
            }
            "dwaveload" => {
                let channel = t.expr(&self.state.vars)?.int()?;
                ensure!((0..50).contains(&channel), "native dwave channel");
                t.comma()?;
                let path = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.state.native.loaded_waves.insert(channel as u32, path);
            }
            "dwaveplay" | "dwaveplayloop" => {
                let channel = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let path = self.state.native.loaded_waves.get(&(channel as u32)).cloned().context("native dwave channel not loaded")?;
                self.play_audio(&path, channel as u32 + 1, command == "dwaveplayloop", host)?;
            }
            "dwavestop" => {
                let channel = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                self.stop_channel(channel as u32 + 1, 0., host)?;
            }
            "bgm" | "bgmonce" => {
                let path = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.play_audio(&path, 100, command == "bgm", host)?;
            }
            "bgmstop" => {
                t.end()?;
                let fade = self.state.native.fade_out_ms as f32;
                self.stop_channel(100, fade, host)?;
            }
            // ---- movies
            "mpegplay" | "avi" | "movie" => {
                let args: Vec<Token> = t.values[t.at..].to_vec();
                let name = t.expr(&self.state.vars)?.string()?;
                let click = if command == "movie" {
                    args.iter().any(|a| matches!(a, Token::Word(w) if w == "click"))
                } else if t.peek().is_some() {
                    t.comma()?;
                    t.expr(&self.state.vars)?.int()? != 0
                } else {
                    true
                };
                t.at = t.values.len();
                let asset = self.resolve_asset(&name, host)?;
                let path = host.media_request(self.client, MediaRequest::Play { id: MEDIA_ID, asset, looping: false, audio: true, gain: 1.0 })?;
                self.submit(host, vec![json!({"op":"put","layer":{"id":MOVIE,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"order":[29000,0],"paint":{"kind":"image","asset":path.relative()}}})], vec![])?;
                self.state.wait = Wait::Movie { click };
            }
            // ---- save and load
            "saveon" | "saveoff" => {
                t.end()?;
                self.state.native.save_disabled = command == "saveoff";
            }
            "savepoint" => {
                t.end()?;
            }
            "savegame" | "savegame2" => {
                let slot = t.expr(&self.state.vars)?.int()?;
                if t.peek().is_some() {
                    t.comma()?;
                    self.state.native.save_label = t.expr(&self.state.vars)?.string()?;
                }
                t.end()?;
                self.save_game(slot, host)?;
            }
            "loadgame" => {
                let slot = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                ensure!(self.load_game(slot, host)?, "native save slot {slot} is empty");
            }
            "loadgosub" => {
                let label = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                self.program.as_ref().unwrap().label(&label)?;
                self.state.native.load_gosub = Some(label);
            }
            "savefileexist" => {
                let dest = t.variable()?;
                t.comma()?;
                let slot = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let exists = self.read_save(slot, host)?.is_some();
                self.state.vars.set(dest, Value::Int(i32::from(exists)))?;
            }
            "savetime" => {
                let slot = t.expr(&self.state.vars)?.int()?;
                let file = self.read_save(slot, host)?;
                let mut at = 0;
                while t.peek().is_some() {
                    t.comma()?;
                    let dest = t.variable()?;
                    let value = file.as_ref().map_or(0, |f| f.time[at.min(4)]);
                    self.state.vars.set(dest, Value::Int(value))?;
                    at += 1;
                }
            }
            "getsavestr" => {
                let dest = t.variable()?;
                t.comma()?;
                let slot = t.expr(&self.state.vars)?.int()?;
                t.end()?;
                let label = self.read_save(slot, host)?.map(|f| f.label).unwrap_or_default();
                self.state.vars.set(dest, Value::Str(label))?;
            }
            "systemcall" => {
                let action = match t.peek() {
                    Some(Token::Word(w)) => w.clone(),
                    _ => t.expr(&self.state.vars)?.string()?,
                };
                t.at = t.values.len();
                self.system_call(&action.to_ascii_lowercase(), Wait::None, host)?;
            }
            "reset" => {
                t.end()?;
                let define = self.program.as_ref().unwrap().label("define")?;
                self.reset_to(define, host)?;
            }
            "bgcopy" | "bgcpy" | "ofscopy" | "ofscpy" | "getscreenshot" => {
                let _ = Self::args(t, &self.state.vars)?;
                self.flush(host)?;
                self.frame_capture = Some((host.begin_frame_capture(self.client)?, command.starts_with("bg")));
                self.state.wait = Wait::Capture;
            }
            "savescreenshot" | "savescreenshot2" => {
                let name = t.expr(&self.state.vars)?.string()?;
                t.end()?;
                let (w, h, rgba) = self.screenshot.as_ref().context("native savescreenshot without getscreenshot")?;
                let bytes = crysirror_image::bmp_encode::encode_bmp(*w, *h, rgba, crysirror_image::bmp_encode::BmpDepth::Bgr24)?;
                ensure!(self.save_write.is_none(), "native save already pending");
                let path = VfsPath::new(Mount::Save, &asset(&name)?)?;
                self.save_write = Some(host.storage(self.client)?.begin_write_atomic(&path, Arc::from(bytes))?);
                if command == "savescreenshot" {
                    self.screenshot = None;
                }
            }
            "exec_dll" | "luacall" | "luasub" => bail!("native `{command}` hosts native code or Lua, which CrySirror never loads"),
            "getinsert_decl" => {}
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_numbers_and_text_variables_follow_ons_rules() {
        assert_eq!(wide_number(42, Some(4)), "　　４２");
        let mut vars = Variables::default();
        vars.ints.insert(3, 12);
        vars.strings.insert(1, "名".into());
        assert_eq!(expand_text("値%3と$1%", &vars, false), "値１２と名%");
        assert_eq!(expand_text("%3", &vars, true), "12");
    }

    #[test]
    fn ruby_spans_keep_base_positions() {
        let (plain, spans) = split_ruby("あ(漢字/かんじ)い(x)");
        assert_eq!(plain, "あ漢字い(x)");
        assert_eq!(spans, vec![(1, 2, "かんじ".to_owned())]);
    }
}
