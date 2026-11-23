// SPDX-License-Identifier: MPL-2.0
//! Native history nodes (44C510), paging (432050/4316E0), and the retained
//! backlog UI (43AE50/432EC0/4330C0). Native text/cursor/page policy is data;
//! shared layout, image workers, layers, widgets and audio execute it.
use super::*;
use crysirror_core::image_processing::ImageResource;
const ROOT: u32 = 66000;
const VOICE: u32 = ROOT + 3000;
const BUTTON: u32 = ROOT + 2000;

#[derive(Clone)]
struct Record {
    //The original node payload begins with exactly these52 bytes, followed by
    //the compiled CP932 text. In particular voice is not a speaker caption.
    rect: [i32; 4],
    mode: i32,
    voice: String,
    raw: String,
}
#[derive(Clone, Default)]
pub(super) struct History {
    records: Vec<Record>,
    rows: i32,
    bytes: usize,
}
impl History {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn record(
        &mut self,
        raw: &str,
        mode: i32,
        voice: &str,
        window: &Window,
        size: i32,
    ) -> Result<()> {
        let f = window.native.fields;
        let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(raw);
        let (encoded_voice, _, invalid_voice) = encoding_rs::SHIFT_JIS.encode(voice);
        ensure!(
            !invalid && !invalid_voice && encoded.len() < 0x10000 && encoded_voice.len() < 32,
            "SugarPot history native record encoding/budget"
        );
        let count = encoded.len() + 53;
        ensure!(
            self.records.len() < 100000
                && self
                    .bytes
                    .checked_add(count + 2)
                    .is_some_and(|n| n <= 64 * 1024 * 1024),
            "SugarPot native history retained budget"
        );
        let groups = crate::text::cells::groups(
            raw,
            encoded.len(),
            size,
            f[15],
            f[17],
            f[16],
            crate::text::cells::Profile::Measure,
            false,
            [0, 0],
        )?;
        //44BEA0's terminal branch removes a trailing newline's counted row;
        //44C510 then adds one novel row only when a body glyph/newline existed,
        //or two ADV spacing rows. Speaker/voice-only nodes retain zero body.
        let terminal_row = if groups.row < f[16] {
            groups.row - i32::from(groups.last_break)
        } else {
            0
        };
        let occupied = terminal_row
            + if mode == 2 {
                i32::from(groups.ordinary)
            } else {
                2
            };
        let height = size + f[14];
        let advance = size + f[15];
        ensure!(height > 0, "SugarPot history line height");
        self.rows = self
            .rows
            .checked_add(occupied)
            .context("SugarPot history row count")?;
        if self
            .records
            .last()
            .is_some_and(|previous| previous.mode != mode)
        {
            self.rows = f[16] + 2;
        }
        let right = f[12]
            .checked_add(
                f[17]
                    .checked_mul(advance)
                    .context("SugarPot history width")?,
            )
            .context("SugarPot history right coordinate")?;
        let mut rect = [f[12], 0, right, 0];
        if f[16] >= self.rows - 1 {
            rect[1] = f[13] + (self.rows - occupied) * height;
            rect[3] = f[13] + (self.rows - if mode == 2 { 0 } else { 1 }) * height;
            if mode == 2 && page_end(raw) {
                self.rows = 0;
            }
            if mode != 2 {
                if let Some(previous) = self
                    .records
                    .last_mut()
                    .filter(|previous| page_end(&previous.raw))
                {
                    previous.raw.truncate(previous.raw.len() - 2);
                    self.bytes = self.bytes.saturating_sub(2);
                }
            }
        } else {
            self.rows = occupied;
            rect[1] = f[13];
            rect[3] = f[13] + (occupied - if mode == 2 { 0 } else { 1 }) * height;
            if let Some(previous) = self
                .records
                .last_mut()
                .filter(|previous| !page_end(&previous.raw))
            {
                previous.raw.push_str("\\f");
                self.bytes += 2;
            }
        }
        self.records.push(Record {
            rect,
            mode,
            voice: voice.to_owned(),
            raw: raw.to_owned(),
        });
        self.bytes += count;
        Ok(())
    }
    fn pages(&self) -> Vec<std::ops::Range<usize>> {
        //431F40 temporarily appends f to an unfinished final node. Treat that
        //terminal delimiter as a view, without mutating native saved text.
        let mut start = 0;
        let mut pages = vec![];
        for (index, record) in self.records.iter().enumerate() {
            if page_end(&record.raw) || index + 1 == self.records.len() {
                pages.push(start..index + 1);
                start = index + 1;
            }
        }
        pages
    }
}
fn page_end(text: &str) -> bool {
    text.ends_with("\\f") || text.ends_with("\\F")
}

pub(super) struct View {
    resume: Box<Wait>,
    native_wait: i32,
    pages: Vec<std::ops::Range<usize>>,
    page: usize,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    source: Option<Arc<dyn ImageResource>>,
    base: Option<UploadedImage>,
    dim: Option<UploadedImage>,
    painted: bool,
    text_layers: Vec<u32>,
    voices: Vec<u32>,
    buttons: Vec<(usize, UploadedImage, [i32; 2])>,
    button_next: usize,
    button_cells: BTreeMap<usize, u8>,
    hover_voice: Option<usize>,
    pending_voice: Option<String>,
    close_requested: bool,
}
impl View {
    pub(super) fn preparing(&self) -> bool {
        !self.painted
    }
    pub(super) fn resource_paths(&self) -> Vec<String> {
        self.base
            .iter()
            .chain(self.dim.iter())
            .map(|i| i.path.relative().to_owned())
            .collect()
    }
}
impl SugarPotVm {
    pub(super) fn history_record(&mut self, raw: &str) -> Result<()> {
        self.history.record(
            raw,
            self.mode,
            &self.last_voice_suffix,
            &self.backlog,
            self.font_size,
        )
    }
    pub(super) fn configure_backlog(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        values: &[Value],
    ) -> Result<bool> {
        let selector = self.number(args, 0, 0)?;
        if selector == 0 {
            self.open_backlog(h)?;
            return Ok(true);
        }
        let name = match selector {
            1 => "size",
            2 => "pos",
            4 => "txt",
            5 => "cel",
            6 => "frm",
            9 => "btn",
            10 => "mod",
            //445F40's reviewed default changes no state.
            _ => return Ok(true),
        };
        let rest = values.get(1..).unwrap_or_default();
        let n = |i| rest.get(i).map_or(Ok(0), Value::integer);
        let mut image_size = None;
        if matches!(name, "frm" | "btn") {
            let image = rest
                .first()
                .context("SugarPot backlog image argument")?
                .string()?;
            if !image.is_empty() {
                let Some(image) = self.image(h, image)? else {
                    return Ok(false);
                };
                image_size = Some(image.size);
                self.declared_images
                    .insert(image.path.relative().to_owned());
            }
        }
        if name == "btn" {
            let id = n(1)?;
            if !(0..=4).contains(&id) {
                return Ok(true);
            }
            self.backlog
                .buttons
                .retain(|previous| previous.get(1).and_then(|v| v.integer().ok()) != Some(id));
            self.backlog.buttons.push(rest.to_vec());
        } else if name == "mod" {
            self.backlog.properties.insert(name.into(), rest.to_vec());
        } else {
            storage::set_window(
                &mut self.backlog.native,
                name,
                rest,
                image_size,
                if name == "cel" { n(5)? } else { 0 },
            )?;
            if name == "txt" {
                let f = &mut self.backlog.native.fields;
                let height = self.font_size + f[14];
                ensure!(height != 0, "SugarPot backlog TXT zero native line height");
                f[16] = ((f[4] - 2 * f[13]) / height) as i8 as i32;
                self.backlog.text_cursor = [f[12], f[13]];
                self.backlog.text_cell = [0, 0];
            }
            self.backlog.properties.insert(name.into(), rest.to_vec());
        }
        Ok(true)
    }
    pub(super) fn open_backlog(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if self.backlog_view.is_some() {
            return Ok(true);
        }
        let allowed = match self.wait {
            Some(Wait::Text) => matches!(self.native_text.waiting, 8 | 16),
            Some(Wait::Selection) => true,
            _ => false,
        };
        if !allowed
            || !self
                .windows
                .get(self.current_window)
                .is_some_and(|w| w.visibility == 1)
        {
            return Ok(false);
        }
        let pages = self.history.pages();
        if pages.is_empty() {
            return Ok(false);
        }
        self.clear_window_controls(h)?;
        let resume = Box::new(self.wait.take().unwrap());
        let native_wait = self.native_text.waiting;
        self.native_text.waiting = i32::MIN;
        self.backlog_view = Some(View {
            resume,
            native_wait,
            page: pages.len() - 1,
            pages,
            capture: Some(h.begin_layer_prefix_image_over(
                self.client,
                [1024, 600],
                [2999, 0],
                [0, 0, 0, 255],
            )?),
            process: None,
            source: None,
            base: None,
            dim: None,
            painted: false,
            text_layers: vec![],
            voices: vec![],
            buttons: vec![],
            button_next: 0,
            button_cells: BTreeMap::new(),
            hover_voice: None,
            pending_voice: None,
            close_requested: false,
        });
        self.wait = Some(Wait::Backlog);
        Ok(true)
    }
    fn prepare_backlog(&mut self, h: &mut dyn HostServices, view: &mut View) -> Result<bool> {
        if view.source.is_none() {
            match view
                .capture
                .as_mut()
                .context("SugarPot backlog capture")?
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot backlog base capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    view.source = Some(resource);
                    view.capture = None;
                }
            }
        }
        if view.base.is_none() {
            view.base = Some(h.publish_image_resource(
                self.client,
                self.next_image,
                view.source.as_ref().unwrap().clone(),
            )?);
            self.next_image = self
                .next_image
                .checked_add(1)
                .context("SugarPot backlog image ID")?;
        }
        if view.dim.is_none() {
            if let Some(job) = &mut view.process {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot backlog RGB lookup: {e}"),
                    ImageProcessState::Ready(resource) => {
                        view.dim = Some(h.publish_image_resource(
                            self.client,
                            self.next_image,
                            resource,
                        )?);
                        self.next_image = self
                            .next_image
                            .checked_add(1)
                            .context("SugarPot backlog image ID")?;
                        view.process = None;
                    }
                }
            } else {
                let f = self.backlog.native.fields;
                let percentage = f[5] as i8 as i32;
                let color = f[0] as u32;
                //43AE50 computes FLOAT percentage, then truncates each signed
                //offset.43D000 clamps/adds stored RGB bytes; shared LUT workers
                //perform every pixel traversal and preserve the stage's alpha.
                let channel = |shift: u32| {
                    let background = ((color >> shift) & 255u32) as i32;
                    let ratio = (percentage as f32 / 100.) as f64;
                    let offset = (-((255 - background) as f64 * ratio)) as i32;
                    (0..256).map(|v| (v + offset).clamp(0, 255) as u8).collect()
                };
                let effect = crysirror_image::effects::ImageEffect::RgbLookup {
                    spec: crysirror_image::rgb_lut::RgbLookup {
                        red: channel(0),
                        green: channel(8),
                        blue: channel(16),
                        profile: crysirror_image::rgb_lut::RgbLookupProfile::StraightRgba,
                    },
                };
                view.process = Some(h.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(view.source.as_ref().unwrap().clone()),
                    serde_json::to_vec(&vec![effect])?.into(),
                )?);
                return Ok(false);
            }
        }
        if let Some(frame) = self.backlog.native.frame.clone() {
            let Some(image) = self.image(h, &frame.image)? else {
                return Ok(false);
            };
            self.declared_images
                .insert(image.path.relative().to_owned());
        }
        while view.button_next < self.backlog.buttons.len() {
            let button = self.backlog.buttons[view.button_next].clone();
            let name = button
                .first()
                .context("SugarPot backlog button image")?
                .string()?;
            if name.is_empty() {
                view.button_next += 1;
                continue;
            }
            let Some(image) = self.image(h, name)? else {
                return Ok(false);
            };
            ensure!(
                image.size[1] % 3 == 0,
                "SugarPot backlog three-cell button sheet"
            );
            let id = usize::try_from(
                button
                    .get(1)
                    .context("SugarPot backlog button ID")?
                    .integer()?,
            )?;
            let position = [
                button
                    .get(2)
                    .context("SugarPot backlog button X")?
                    .integer()? as i16 as i32,
                button
                    .get(3)
                    .context("SugarPot backlog button Y")?
                    .integer()? as i16 as i32,
            ];
            if !view.buttons.iter().any(|(old, _, _)| *old == id) {
                view.buttons.push((id, image, position));
            }
            view.button_next += 1;
        }
        self.paint_backlog(h, view, true, true)?;
        Ok(true)
    }
    fn backlog_button(
        &self,
        id: usize,
        image: &UploadedImage,
        position: [i32; 2],
        cell: u8,
    ) -> Json {
        json!({"op":"put","layer":{"id":BUTTON+id as u32,"rect":[position[0],position[1],image.size[0],image.size[1]/3],
            "order":[10004,id],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
            "uv":[0.,cell as f32/3.,1.,(cell as f32+1.)/3.],"paint":{"kind":"image","asset":image.path.relative()}}})
    }
    fn paint_backlog(
        &mut self,
        h: &mut dyn HostServices,
        view: &mut View,
        background: bool,
        replace_widgets: bool,
    ) -> Result<()> {
        let f = self.backlog.native.fields;
        let mut layers = view
            .text_layers
            .iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut ui = if replace_widgets {
            view.voices
                .iter()
                .map(|id| json!({"op":"remove","id":id}))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        view.text_layers.clear();
        if replace_widgets {
            view.voices.clear();
        }
        if background {
            ui.push(json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":11000,
                "kind":"panel","modal":true,"color":[0,0,0,0],"key_event_scope":true,"keyboard_navigation":false,
                "key_events":[{"buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"event":"bl_close"},
                    {"buttons":buttons::WHEEL_UP|buttons::PAGE_UP,"event":"bl_previous"},
                    {"buttons":buttons::WHEEL_DOWN|buttons::PAGE_DOWN,"event":"bl_next"},
                    {"buttons":buttons::HOME,"event":"bl_top"},{"buttons":buttons::END,"event":"bl_bottom"}]}}));
            layers.push(json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1024,600],"order":[10000,0],
                "blend":Self::native_image_blend(24,0),"paint":{"kind":"image","asset":view.base.as_ref().unwrap().path.relative()}}}));
            //43D000 affects only the CEL rectangle; the bare base stage remains
            //visible outside it. The backdrop is engine-captured, never a
            //private composed framebuffer or a legacy renderer upload.
            layers.push(
                json!({"op":"put","layer":{"id":ROOT+1,"rect":[0,0,1024,600],"order":[10001,0],
                "clip":[f[1]+f[6],f[2]+f[7],f[8],f[9]],"blend":Self::native_image_blend(24,0),
                "paint":{"kind":"image","asset":view.dim.as_ref().unwrap().path.relative()}}}),
            );
            if let Some(frame) = &self.backlog.native.frame {
                let path = self.resolve("grp", &frame.image, &["abm", "bmp", "dat"])?;
                let image = self
                    .images
                    .get(&path)
                    .context("SugarPot backlog prepared frame")?;
                layers.push(json!({"op":"put","layer":{"id":ROOT+2,"rect":[f[1]+frame.position[0],f[2]+frame.position[1],image.size[0],image.size[1]],
                    "order":[10002,0],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
                    "paint":{"kind":"image","asset":image.path.relative()}}}));
            }
            for (id, image, position) in &view.buttons {
                layers.push(self.backlog_button(*id, image, *position, 0));
                view.button_cells.insert(*id, 0);
                ui.push(
                    json!({"op":"put","widget":{"id":BUTTON+*id as u32,"parent":ROOT,
                    "rect":[position[0],position[1],image.size[0],image.size[1]/3],"z":2000+id,
                    "kind":"button","interaction_only":true,"keyboard_focusable":false}}),
                );
            }
        }
        let range = view
            .pages
            .get(view.page)
            .context("SugarPot backlog page")?
            .clone();
        ensure!(range.len() <= 1024, "SugarPot backlog page record budget");
        let options = crysirror_text::grid::CellLayoutOptions { cell_widths: Vec::new(),
            bounds: [32767, 32767],
            size: self.font_size,
            spacing: [f[15], f[14]],
            half_width: true,
            half_spacing: false,
            hanging: 0,
            forbidden_line_start: String::new(),
            indent_pairs: vec![],
        };
        let mut cursor = crysirror_text::grid::CellCursor::at_start();
        let mut cell = [0, 0];
        for (local, index) in range.clone().enumerate() {
            let record = &self.history.records[index];
            let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(&record.raw);
            ensure!(!invalid, "SugarPot backlog compiled source encoding");
            let groups = crate::text::cells::groups(
                &record.raw,
                encoded.len(),
                self.font_size,
                f[15],
                f[17],
                f[16],
                crate::text::cells::Profile::Backlog,
                false,
                cell,
            )?;
            let mut glyphs = vec![];
            //43AE50 passes a4=0 to43C560: a header resets X/counters but keeps
            //the current Y. Split at explicit header groups to ask the shared
            //cursor for that Y instead of privately duplicating layout math.
            for (at, group) in groups.cells.iter().enumerate() {
                let mut group = group.clone();
                if groups.headers.binary_search(&at).is_ok() {
                    group.reset_before = Some([0, cursor.position[1]]);
                }
                let placed = crysirror_text::grid::layout_cell_groups(
                    std::slice::from_ref(&group),
                    &options,
                    &mut cursor,
                    false,
                )?;
                ensure!(
                    placed.consumed_groups == 1,
                    "SugarPot backlog native cursor bounds"
                );
                glyphs.extend(placed.glyphs);
            }
            cell = [groups.column, groups.row];
            let color = if record.voice.is_empty() {
                [0, 255, 255, 255]
            } else if view.hover_voice == Some(local) {
                [255, 255, 0, 255]
            } else {
                [0, 255, 0, 255]
            };
            let shadow = self.font_style[0] as u8 as i32;
            let glyphs=glyphs.iter().map(|g|json!({"character":g.character,
                "baseline":[f[12]+g.position[0],f[13]+g.position[1]+self.font_size],
                "style":{"shadow":if shadow==0 {Json::Null}else{json!({"offset":[shadow,shadow],
                    "color":[self.font_style[1]as u8,(self.font_style[1]>>8)as u8,(self.font_style[1]>>16)as u8,255]})}}})).collect::<Vec<_>>();
            let id = ROOT + 10 + local as u32;
            layers.push(json!({"op":"put","layer":{"id":id,"rect":[f[1],f[2],f[3],f[4]],"order":[10003,local],
                "paint":{"kind":"glyph_run","font":self.config.font,"size":self.font_size,"color":color,"glyphs":glyphs}}}));
            view.text_layers.push(id);
            if replace_widgets
                && !record.voice.is_empty()
                && record.rect[2] > record.rect[0]
                && record.rect[3] > record.rect[1]
            {
                let id = VOICE + local as u32;
                ui.push(json!({"op":"put","widget":{"id":id,"parent":ROOT,
                    "rect":[record.rect[0],record.rect[1],record.rect[2]-record.rect[0],record.rect[3]-record.rect[1]],
                    "z":local,"kind":"button","interaction_only":true,"keyboard_focusable":false}}));
                view.voices.push(id);
            }
            if index + 1 < range.end {
                let count = if record.raw.is_empty() {
                    1
                } else if record.mode != 2 {
                    2
                } else if cell[0] != 0
                    && !record.raw.ends_with("\\i")
                    && !record.raw.ends_with("\\I")
                {
                    1
                } else {
                    0
                };
                for _ in 0..count {
                    let newline = crate::text::cells::groups(
                        "\\n",
                        2,
                        self.font_size,
                        f[15],
                        f[17],
                        f[16],
                        crate::text::cells::Profile::Body,
                        false,
                        cell,
                    )?;
                    let layout = crysirror_text::grid::layout_cell_groups(
                        &newline.cells,
                        &options,
                        &mut cursor,
                        false,
                    )?;
                    ensure!(
                        layout.consumed_groups == newline.cells.len(),
                        "SugarPot backlog page cursor bounds"
                    );
                    cell = [newline.column, newline.row];
                }
            }
        }
        self.submit(h, layers, ui)?;
        view.painted = true;
        Ok(())
    }
    pub(super) fn poll_backlog(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<()> {
        let mut view = self
            .backlog_view
            .take()
            .context("SugarPot backlog wait state")?;
        let result = self.backlog_step(h, input, &mut view);
        if !result.as_ref().is_ok_and(|closed| *closed) {
            self.backlog_view = Some(view);
        }
        result.map(|_| ())
    }
    fn backlog_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        view: &mut View,
    ) -> Result<bool> {
        view.close_requested |=
            input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0;
        if view.preparing() {
            self.prepare_backlog(h, view)?;
            return Ok(false);
        }
        if let Some(voice) = view.pending_voice.clone() {
            //4330C0 passes the stored history suffix directly to45B130,
            //without prepending the live VAR.cv prefix a second time.
            if !self.play_audio(h, &voice, 2, PcmBus::Voice, false)? {
                return Ok(false);
            }
            view.pending_voice = None;
        }
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let mut close = view.close_requested;
        let mut page = view.page;
        let mut next = input.buttons_pressed & (buttons::WHEEL_DOWN | buttons::PAGE_DOWN) != 0;
        let mut previous = input.buttons_pressed & (buttons::WHEEL_UP | buttons::PAGE_UP) != 0;
        let mut hover = None;
        let mut layers = vec![];
        for (id, image, position) in &view.buttons {
            let state: Json = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiInteraction(BUTTON + *id as u32))?,
            )?;
            let cell = if state["pressed"] == true {
                2
            } else if state["hovered"] == true {
                1
            } else {
                0
            };
            if view.button_cells.get(id) != Some(&cell) {
                layers.push(self.backlog_button(*id, image, *position, cell));
                view.button_cells.insert(*id, cell);
            }
            if state["activated"] == true {
                match id {
                    0 => previous = true,
                    1 => next = true,
                    2 => page = 0,
                    3 => page = view.pages.len() - 1,
                    4 => close = true,
                    _ => {}
                }
            }
        }
        for id in &view.voices {
            let state: Json = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiInteraction(*id))?,
            )?;
            let local = (*id - VOICE) as usize;
            if state["hovered"] == true {
                hover = Some(local);
            }
            if state["activated"] == true {
                let index = view.pages[view.page].start + local;
                view.pending_voice = Some(self.history.records[index].voice.clone());
            }
        }
        for event in events {
            if event["widget"]["owner"] != self.client.0 {
                continue;
            }
            match event["kind"].as_str().unwrap_or("") {
                "bl_close" => close = true,
                "bl_previous" => previous = true,
                "bl_next" => next = true,
                "bl_top" => page = 0,
                "bl_bottom" => page = view.pages.len() - 1,
                _ => {}
            }
        }
        if previous {
            page = page.saturating_sub(1);
        }
        if next {
            if page + 1 >= view.pages.len() {
                close = true;
            } else {
                page += 1;
            }
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        if close {
            let mut ids = view.text_layers.clone();
            ids.extend([ROOT, ROOT + 1]);
            if self.backlog.native.frame.is_some() {
                ids.push(ROOT + 2);
            }
            ids.extend(view.buttons.iter().map(|(id, _, _)| BUTTON + *id as u32));
            self.submit(
                h,
                ids.iter()
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect(),
                vec![json!({"op":"remove","id":ROOT})],
            )?;
            for image in view.base.iter().chain(view.dim.iter()) {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            self.stop_audio(h, 2, 0)?;
            self.native_text.waiting = view.native_wait;
            self.wait = Some((*view.resume).clone());
            input.buttons_pressed = 0;
            return Ok(true);
        }
        if page != view.page || hover != view.hover_voice {
            let changed = page != view.page;
            view.page = page;
            view.hover_voice = if changed { None } else { hover };
            self.paint_backlog(h, view, false, changed)?;
        }
        input.buttons_pressed = 0;
        Ok(false)
    }
}

pub(super) fn default_window() -> Window {
    //458870 zero-allocates292 bytes, then initializes this independent window
    //and a five-button sentinel array. Byte20 is dim32, not visibility24.
    let mut w = Window::default();
    w.native.fields = [
        0, 0, 0, 1024, 600, 32, 0, 0, 0, 0, 0, 0, 30, 24, 3, 0, 20, 24,
    ];
    w.text_cursor = [30, 24];
    w
}
