//! Native text controls translated to shared encoded-cell placement/GlyphRun.
//! No glyph rasterizer, font cache, typewriter or hit-test engine lives here.
use super::*;
use crate::machine::{
    TextPart,
    text_ops::{TextReply, TextRequest},
};
use crysirror_text::{
    grid::{encoded_cell_run, encoded_ruby_run},
    reveal::pixel_curtain::PixelCurtain,
    unit_flow::{UnitFlowItem, UnitFlowOptions, layout_unit_flow},
};

#[path="text_stream.rs"]
mod streaming;

const ROOT: u32 = 0x3900_0000;
const BODY: u32 = ROOT + 0x1000;
const NAME: u32 = ROOT + 2;
const WINDOW: u32 = ROOT + 3;

#[derive(Serialize, Deserialize)]
pub(super) struct NativeText {
    #[serde(default = "default_extra_sprite_rate")]
    pub(super) extra_sprite_rate: f32,
    #[serde(skip)]
    controls_refreshing:bool,
    #[serde(default)]
    streams:streaming::NativeTextStreams,
    pub active: bool,
    window: bool,
    curtain: Option<PixelCurtain>,
    rows: Vec<u32>,
    decorations: Vec<u32>,
    row_heights: Vec<u32>,
}
fn default_extra_sprite_rate() -> f32 {
    1.
}
impl Default for NativeText {
    fn default() -> Self {
        Self {
            extra_sprite_rate: 1.,
            active: false,
            controls_refreshing:false,
            streams:streaming::NativeTextStreams::default(),
            window: false,
            curtain: None,
            rows: Vec::new(),
            decorations: Vec::new(),
            row_heights: Vec::new(),
        }
    }
}
impl Bridge<'_> {
    pub(super) fn set_native_dialogue_visibility(&mut self,visible:bool)->Result<()> {
        let mut layers=Vec::new();
        if self.services.text.active {layers.push(json!({"op":"set_visible","id":ROOT,"visible":visible}));}
        if self.services.text.window {layers.push(json!({"op":"set_visible","id":WINDOW,"visible":visible}));}
        self.submit(layers,vec![])
    }
}
impl NativeText {
    pub(super) fn animating(&self) -> bool {
        self.streams.animating()||self.curtain.as_ref().is_some_and(|c| !c.complete())
    }
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.extra_sprite_rate.is_finite(),
            "Circus text drawable rate"
        );
        self.streams.validate()?;
        if let Some(curtain) = &self.curtain {
            curtain.validate()?;
            ensure!(
                curtain.visible_widths().len() == self.rows.len()
                    && self.row_heights.len() == self.rows.len(),
                "Circus saved curtain row identity"
            );
        }
        ensure!(
            self.rows.len() <= 4096
                && self
                    .rows
                    .iter()
                    .enumerate()
                    .all(|(n, id)| *id == BODY + n as u32),
            "Circus saved text layer identity"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Run(String),
    Size(i16),
    DefaultFont,
    FontWidth(i16),
    Color(i16),
    Rgb([u8; 3]),
    ColorUnchanged,
    Newline,
    Ruby { reading: String, base: String },
}

///43E490 consumes native @ size/color controls, not HTML or Unicode casing.
///Known preprocessing commands must have been scheduled by their semantic
///owner. Other selectors retain43E490's explicit default-font behavior.
fn tokens(text: &str) -> Result<Vec<Token>> {
    tokens_with_defaults(text,false)
}
fn scheduled_control(text:&str)->bool {
    if ["@t","@cmd","@d","@muf","@mu","@lse","@se","@ht","@hy","@re1"].iter().any(|prefix|text.starts_with(prefix)){return true}
    for prefix in ["@wb","@w","@bt","@by","@bi","@bo"] {
        if text.strip_prefix(prefix).and_then(|tail|tail.bytes().next()).is_some_and(|digit|(b'1'..=b'5').contains(&digit)){return true}
    }
    text.strip_prefix("@b").and_then(|tail|tail.bytes().next()).is_some_and(|selector|(b'1'..=b'5').contains(&selector)||matches!(selector,b'c'|b'f'))
}
fn tokens_with_defaults(text:&str,native_defaults:bool)->Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut plain = String::new();
    let mut at = 0;
    let flush = |out: &mut Vec<Token>, plain: &mut String| {
        if !plain.is_empty() {
            out.push(Token::Run(std::mem::take(plain)));
        }
    };
    while at < text.len() {
        let rest = &text[at..];
        let ch = rest.chars().next().unwrap();
        if ch == '\n' {
            flush(&mut out, &mut plain);
            out.push(Token::Newline);
            at += 1;
            continue;
        }
        if ch == '｛' {
            flush(&mut out, &mut plain);
            let after = at + ch.len_utf8();
            let end = text[after..]
                .find('｝')
                .context("Circus unterminated native ruby")?
                + after;
            let (reading, base) = text[after..end]
                .split_once('／')
                .context("Circus native ruby missing separator")?;
            ensure!(
                !base.is_empty() && !reading.is_empty(),
                "Circus empty native ruby"
            );
            out.push(Token::Ruby {
                reading: reading.into(),
                base: base.into(),
            });
            at = end + '｝'.len_utf8();
            continue;
        }
        if ch != '@' {
            plain.push(ch);
            at += ch.len_utf8();
            continue;
        }
        flush(&mut out, &mut plain);
        ensure!(rest.len() >= 2, "Circus incomplete native text control");
        let control = rest.as_bytes()[1];
        let scheduled=!native_defaults&&scheduled_control(rest);
        let size = match control {
            b'm' if !scheduled => Some(22),
            b'l' if !scheduled => Some(32),
            b's' if !scheduled => Some(20),
            b'i' => Some(18),
            b'r' if !scheduled => Some(12),
            b'y' => Some(16),
            b'z' => Some(10),
            b'w' if !scheduled =>
            {
                flush(&mut out, &mut plain);
                out.push(Token::FontWidth(44));
                at += 2;
                continue;
            }
            _ => None,
        };
        if let Some(size) = size {
            out.push(Token::Size(size));
            at += 2;
            continue;
        }
        if control == b'c' && !scheduled {
            let value = *rest
                .as_bytes()
                .get(2)
                .context("Circus incomplete native color")?;
            if value.is_ascii_digit() {
                let digits = if rest.as_bytes().get(3).is_some_and(u8::is_ascii_digit) {
                    2
                } else {
                    1
                };
                out.push(Token::Color(rest[2..2 + digits].parse()?));
                at += 2 + digits;
                continue;
            }
            let color = match value {
                b'r' => [255, 0, 0],
                b'g' => [0, 255, 0],
                b'b' => [0, 0, 255],
                b'w' => [255; 3],
                b'k' => [0; 3],
                _=>{ensure!(value.is_ascii(),"Circus native color selector splits encoded character");out.push(Token::ColorUnchanged);at+=3;continue},
            };
            out.push(Token::Rgb(color));
            at += 3;
            continue;
        }
        if !scheduled {
            //43E490's LABEL119 default selects the22px font and advances
            //exactly two native bytes. Known unscheduled inline commands
            //are kept distinct from this original renderer default branch.
            ensure!(rest.as_bytes()[1].is_ascii(),"Circus native text control splits encoded character");
            out.push(Token::DefaultFont);at+=2;continue
        }
        bail!(
            "Circus native inline command at byte{at} requires its scheduled semantic side effect: {rest}"
        );
    }
    flush(&mut out, &mut plain);
    Ok(out)
}

fn default_palette(index: i16) -> Result<[u8; 3]> {
    // Original static COLORREF table4B31B0, decoded BGR -> logical RGB.
    const COLORS: [[u8; 3]; 14] = [
        [0, 0, 0],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [0, 0, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
        [40, 133, 78],
        [255, 204, 204],
        [0, 119, 68],
        [127, 255, 127],
        [204, 204, 204],
        [255, 170, 170],
    ];
    ensure!((0..64).contains(&index), "Circus text palette index");
    Ok(COLORS.get(index as usize).copied().unwrap_or([0; 3]))
}

/// Allocation metrics differ from actual glyph advances: the native prepass
/// only grows size, counts @w single-byte cells twice and keeps color selector
/// bytes in its width. Shared unit flow performs the placement arithmetic.
#[derive(Clone, Debug)]
struct NativeLineMetrics {
    width: u32,
    height: i16,
    seed: i16,
}
fn native_line_metrics(text: &str, initial: i16) -> Result<Vec<NativeLineMetrics>> {
    native_line_metrics_with_defaults(text,initial,false)
}
fn native_line_metrics_with_defaults(text:&str,initial:i16,native_defaults:bool)->Result<Vec<NativeLineMetrics>> {
    ensure!(
        (1..=512).contains(&initial),
        "Circus native initial font size"
    );
    let mut lines = Vec::new();
    let mut seed = initial;
    for line in text.split('\n') {
        let mut size = seed as u16;
        let mut last_size = seed;
        let mut wide = false;
        let mut ruby_min = 0;
        let mut items = Vec::new();
        let parsed = tokens_with_defaults(line,native_defaults)?;
        let height = if parsed.iter().any(|token| matches!(token, Token::Size(32))) {
            32
        } else {
            seed
        };
        for token in parsed {
            match token {
                Token::Size(value) => {
                    size = size.max(value as u16);
                    last_size = value;
                    wide = false;
                }
                Token::DefaultFont=>{last_size=22;wide=false;},
                Token::FontWidth(_) => {
                    size = size.max(22);
                    last_size = 22;
                    wide = true;
                }
                Token::Color(value) => {
                    wide = false;
                    for character in value.to_string().chars() {
                        items.push(UnitFlowItem::Glyph {
                            character,
                            size,
                            units: 1,
                            style: 0,
                        });
                    }
                }
                Token::Rgb(_) | Token::ColorUnchanged => {
                    wide = false;
                    items.push(UnitFlowItem::Glyph {
                        character: 'c',
                        size,
                        units: 1,
                        style: 0,
                    });
                }
                Token::Run(value) => {
                    for character in value.chars() {
                        let mut buffer = [0; 4];
                        let (bytes, _, error) =
                            encoding_rs::SHIFT_JIS.encode(character.encode_utf8(&mut buffer));
                        ensure!(!error, "Circus cell allocation encoding");
                        items.push(UnitFlowItem::Glyph {
                            character,
                            size,
                            units: if wide || bytes.len() > 1 { 2 } else { 1 },
                            style: 0,
                        });
                    }
                }
                Token::Ruby { reading, base } => {
                    let (reading, _, error) = encoding_rs::SHIFT_JIS.encode(&reading);
                    ensure!(!error, "Circus ruby allocation reading");
                    if ruby_min < 12 * reading.len() {
                        ruby_min = 12 * (reading.len() >> 1);
                    }
                    let (base, _, error) = encoding_rs::SHIFT_JIS.encode(&base);
                    ensure!(!error, "Circus ruby allocation base");
                    items.push(UnitFlowItem::Move {
                        x: Some((base.len() * usize::from(size / 2)) as i64),
                        y: None,
                        relative: true,
                    });
                }
                Token::Newline => unreachable!(),
            }
        }
        let flow = layout_unit_flow(&items, UnitFlowOptions::default())?;
        let width = u32::try_from(flow.cursor[0])?
            .max(u32::try_from(ruby_min)?)
            .checked_add(4)
            .context("Circus text allocation width overflow")?;
        lines.push(NativeLineMetrics {
            width,
            height,
            seed,
        });
        seed = last_size;
    }
    Ok(lines)
}
#[cfg(test)]
fn native_canvas_width(text: &str) -> Result<u32> {
    Ok(native_line_metrics(text, 22)?
        .iter()
        .map(|line| line.width)
        .max()
        .unwrap_or(4))
}

impl Bridge<'_> {
    ///A published input/animation owner can be shown while its opcode waits.
    ///Asset workers and higher modal owners always take precedence over an
    ///older visible menu; their pending frames are never presentation ACKs.
    pub(super) fn native_ui_waiting(&self)->bool {
        if self.services.loading.is_some()||self.services.menu_image_job.is_some()||self.services.text.controls_refreshing{return false}
        if self.services.history_preview.pending(){return false}
        if self.services.history.active(){return self.services.history.ui_waiting()}
        if let Some(waiting)=self.services.system.modal_ui_waiting(){return waiting}
        if let Some(profile)=&self.services.profile{return profile.ui_waiting()}
        if self.services.movie.active(){return self.services.movie.ui_waiting()}
        if let Some(panel)=&self.services.panel{return panel.ui_waiting()}
        self.services.menu.as_ref().is_some_and(|menu|menu.ui_waiting())
    }
    pub(super) fn text_layer_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        if self.services.text.active {
            ids.extend([ROOT, NAME]);
            ids.extend(self.services.text.rows.iter().copied());
            ids.extend(self.services.text.decorations.iter().copied());
        }
        if self.services.text.window {
            ids.push(WINDOW);
        }
        ids
    }
    pub(super) fn native_text(&mut self, request: &TextRequest) -> Result<Option<TextReply>> {
        match request {
            TextRequest::History(request)=>return Ok(self.native_history(request)?.map(TextReply::History)),
            TextRequest::RefreshDialogueControls{group} => {
                if !self.services.text.controls_refreshing {self.clear_menu()?;self.services.text.controls_refreshing=true;}
                if let Some(group)=group {if !self.begin_menu(group)? {return Ok(None)}}
                self.services.text.controls_refreshing=false;
            }
            TextRequest::Movie(request) => return self.native_movie(request),
            TextRequest::Comment(request) => {
                return Ok(self.native_comment(request)?.then_some(TextReply::Done));
            }
            TextRequest::ClearDialogue => {
                let mut commands = self
                    .services
                    .text
                    .rows
                    .iter()
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect::<Vec<_>>();
                let name: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::Layer(NAME))?,
                )?;
                if !name.is_null() {
                    commands.push(json!({"op":"remove","id":NAME}));
                }
                self.submit(commands, vec![])?;
                self.services.text.rows.clear();
                self.services.text.row_heights.clear();
                self.services.text.curtain = None;
            }
            TextRequest::ClearUi => {
                self.clear_panel()?;
                self.clear_menu()?;
            }
            TextRequest::PanelBegin {
                declarations,
                fixed,
                scroll,
                selected,
            } => {
                if !self.begin_panel(declarations, fixed, *scroll, *selected)? {
                    return Ok(None);
                }
            }
            TextRequest::PanelPoll => return Ok(self.poll_panel()?.map(TextReply::Panel)),
            TextRequest::Clear => {
                self.clear_panel()?;
                self.clear_menu()?;
                if self.services.text.active {
                    self.submit(vec![json!({"op":"remove","id":ROOT})], vec![])?;
                }
                if self.services.text.window {
                    self.submit(vec![json!({"op":"remove","id":WINDOW})], vec![])?;
                }
                self.services.text = NativeText::default();
            }
            TextRequest::ImmediateImage(part) => {
                self.root()?;
                let mut layers = Vec::new();
                self.image_layer(&mut layers, WINDOW, part, 110)?;
                if let Some(layer) = layers.iter_mut().rev().find(|command|command["layer"]["id"]==WINDOW) {
                    layer["layer"]["visible"] = json!(self.services.native_window_visible);
                    layer["layer"]["parent"] = json!(SCENE_ROOT);
                }
                self.submit(layers, vec![])?;
                self.services.text.window = true;
            }
            TextRequest::InlineEvent(event) => {
                if !self.native_inline_event(event)? {
                    return Ok(None);
                }
            }
            TextRequest::Input => {
                let revealing = self
                    .services
                    .text
                    .curtain
                    .as_ref()
                    .is_some_and(|c| !c.complete());
                if revealing {
                    self.services.text.curtain.as_mut().unwrap().finish();
                    self.publish_text_curtain()?;
                }
                // Retire this native edge after the reveal/input operation,
                // so an asynchronous next-line load cannot deliver it twice.
                self.native_system(crate::machine::system_ops::SystemRequest::Gesture(
                    crate::machine::gesture_ops::GestureRequest::ConsumeInput{
                        mask:crysirror_core::service::buttons::MOUSE_LEFT
                            |crysirror_core::service::buttons::ENTER
                            |crysirror_core::service::buttons::PAGE_DOWN,
                    }))?;
                return Ok(Some(TextReply::Input {
                    advance: !revealing,
                }));
            }
            TextRequest::CommentRecord => {
                return Ok(Some(TextReply::CommentRecord(
                    self.services.comments.native_record(),
                )));
            }
            TextRequest::FinishDrawableQueues => self.finish_text_motion(false)?,
            TextRequest::MenuEntryFinish => {
                self.finish_text_motion(true)?;
            }
            TextRequest::MovementStatus => {
                return Ok(Some(TextReply::Input {
                    advance: !self.menu_moving()?,
                }));
            }
            TextRequest::Status => {
                return Ok(Some(TextReply::Input {
                    advance: self
                        .services
                        .text
                        .curtain
                        .as_ref()
                        .is_none_or(PixelCurtain::complete),
                }));
            }
            TextRequest::GalleryBegin(group) => {
                if !self.services.text.controls_refreshing {self.clear_menu()?;self.services.text.controls_refreshing=true;}
                if !self.begin_menu(group)? {return Ok(None);}
                self.services.text.controls_refreshing=false;
            }
            TextRequest::GalleryPoll => {
                if self.input.buttons_pressed & crysirror_core::service::buttons::MOUSE_RIGHT != 0 {
                    self.clear_menu()?;
                    return Ok(Some(TextReply::GalleryInput { button: None }));
                }
                let Some(action) = self.tick_menu()? else {
                    return Ok(None);
                };
                return Ok(Some(TextReply::GalleryInput {
                    button: Some(
                        action
                            .parse()
                            .context("Circus native gallery button ordinal")?,
                    ),
                }));
            }
            TextRequest::GalleryResume => self.resume_menu()?,
            TextRequest::UpdateRow {
                ordinal,
                action,
                text,
            } => self.update_menu_row(*ordinal, action, text)?,
            TextRequest::Dialogue {
                speaker,
                body,
                body_template,
                name_template,
                body_group,
                name_group,
                speed,
            } => {
                ensure!((0..=100).contains(speed), "Circus native text speed");
                let mut layers = Vec::new();
                if self.services.text.active {
                    layers.push(json!({"op":"remove","id":ROOT}));
                }
                layers.push(json!({"op":"put","layer":{"id":ROOT,"parent":SCENE_ROOT,"visible":self.services.native_window_visible,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[110,0]}}));
                let mut widths = Vec::new();
                let mut rows = Vec::new();
                let mut row_heights = Vec::new();
                let mut decorations = Vec::new();
                for (n, group) in [body_group, name_group].into_iter().enumerate() {
                    if let Some(group) = group {
                        for (index, original) in group.images.iter().enumerate() {
                            ensure!(index < 4096, "Circus message-window image budget");
                            let mut part = original.clone();
                            part.position[0] += i32::from(group.position[0]);
                            part.position[1] += i32::from(group.position[1]);
                            let id = ROOT + 0x2000 + n as u32 * 4096 + index as u32;
                            self.image_layer(&mut layers, id, &part, 109)?;
                            if let Some(layer) =
                                layers.iter_mut().rev().find(|value| value["layer"]["id"] == id)
                            {
                                layer["layer"]["parent"] = json!(ROOT);
                                layer["layer"]["visible"] = json!(matches!(part.state, 0 | 99));
                                decorations.push(id);
                            }
                        }
                    }
                }
                if let Some(template) = body_template {
                    let mut y = template.position[1];
                    let mut color_prefix = String::new();
                    let line_metrics = native_line_metrics(body, template.size)?;
                    for (n, line) in body.split('\n').enumerate() {
                        ensure!(n < 4096, "Circus message row budget");
                        let parsed = tokens(line)?;
                        let metric = &line_metrics[n];
                        let height = metric.height + 12;
                        let mut part = template.clone();
                        part.position[1] = y;
                        part.size = metric.seed;
                        part.text = format!("{color_prefix}{line}");
                        let id = BODY + n as u32;
                        self.text_layers(&mut layers, ROOT, id, std::slice::from_ref(&part), 110)?;
                        widths.push(metric.width);
                        row_heights.push(height as u32);
                        rows.push(id);
                        layers.last_mut().unwrap()["layer"]["source_crop"] =
                            json!([0, 0, 0, height]);
                        y = y
                            .checked_add(template.line_pitch.unwrap_or(height))
                            .context("Circus message row position overflow")?;
                        for token in parsed {
                            match token {
                                Token::Color(index) => color_prefix = format!("@c{index}"),
                                Token::Rgb(rgb) => {
                                    color_prefix = match rgb {
                                        [255, 0, 0] => "@cr",
                                        [0, 255, 0] => "@cg",
                                        [0, 0, 255] => "@cb",
                                        [255, 255, 255] => "@cw",
                                        _ => "@ck",
                                    }
                                    .into()
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if let Some(template) = name_template {
                    let mut part = template.clone();
                    part.text = speaker.clone();
                    self.text_layers(&mut layers, ROOT, NAME, std::slice::from_ref(&part), 110)?;
                }
                let step = u32::try_from(2 * speed + 8)?;
                let mut curtain = PixelCurtain::new(widths, step)?;
                if step >= 50 {
                    curtain.finish();
                }
                self.root()?;
                self.submit(layers, vec![])?;
                self.services.text = NativeText {
                    controls_refreshing:false,
                    streams:std::mem::take(&mut self.services.text.streams),
                    extra_sprite_rate: self.services.text.extra_sprite_rate,
                    active: true,
                    window: self.services.text.window,
                    curtain: Some(curtain),
                    rows,
                    decorations,
                    row_heights,
                };
                self.publish_text_curtain()?;
            }
        }
        Ok(Some(TextReply::Done))
    }

    pub(super) fn advance_text(&mut self) -> Result<()> {
        self.advance_text_streams()?;
        if self.services.system.pointer_motion_ticks!=0 && self.host.window_focused(self.client)?!=Some(false) {
            self.submit(vec![],vec![json!({"op":"advance_pointer_motion","ticks":1})])?;
            self.services.system.pointer_motion_ticks-=1;
        }
        if let Some(curtain) = &mut self.services.text.curtain {
            if !curtain.complete() {
                curtain.advance_ticks(1)?;
                self.publish_text_curtain()?;
            }
        }
        Ok(())
    }
    fn publish_text_curtain(&mut self) -> Result<()> {
        let Some(curtain) = &self.services.text.curtain else {
            return Ok(());
        };
        let commands = self
            .services
            .text
            .rows
            .iter()
            .zip(curtain.visible_widths()).zip(&self.services.text.row_heights)
            .map(|((id, width),height)| json!({"op":"set_source_crop","id":id,"crop":[0,0,width,height]}))
            .collect();
        self.submit(commands, vec![])
    }

    fn finish_text_motion(&mut self, release44: bool) -> Result<()> {
        let mut commands = vec![];
        //440AB0 addresses the dedicated overlay text object25080, rather
        //than every message group. Its retained carrier150 owns no glyph
        //curtain; the primary actor is absent until a native declaration.
        let id = 0x3a04_0000u32 + 150;
        let layer: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(id))?,
        )?;
        if !layer.is_null() {
            let status: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(id))?,
            )?;
            if !status.is_null() {
                commands.push(
                    json!({"op":"sequence","id":id,"operation":{"kind":"finish","cancel":false}}),
                );
            }
        }
        if let Some(image) = release44
            .then(|| self.services.images.remove(&44))
            .flatten()
        {
            for object in self.services.visual.roots() {
                let id = super::visual::content_layer(object);
                let layer: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::Layer(id))?,
                )?;
                if layer["paint"]["kind"] == "image"
                    && layer["paint"]["asset"] == image.path.relative()
                {
                    commands.push(json!({"op":"set_paint","id":id,"paint":{"kind":"solid","color":[0,0,0,0]}}));
                }
            }
            self.services.image_names.remove(&44);
            self.services.image_extents.remove(&44);
            self.host
                .release_image_resource(self.client, image.path.relative())?;
        }
        self.submit(commands, vec![])
    }
    pub(super) fn comment_glyph_rows(
        &mut self,
        layers: &mut Vec<Value>,
        parent: u32,
        base: u32,
        value: &str,
        origin: [i16; 2],
        color: i16,
    ) -> Result<Vec<(u32, u32, u32)>> {
        let metrics = native_line_metrics(value, 22)?;
        let mut parts = vec![];
        let mut y = origin[1];
        let mut carry = String::new();
        for (n, line) in value.split('\n').enumerate() {
            let metric = &metrics[n];
            let id = base + n as u32;
            let part = TextPart {
                position: [origin[0], y],
                text: format!("{carry}{line}"),
                color,
                size: metric.seed,
                line_pitch: None,
                state: 99,
            };
            self.text_layers(layers, parent, id, std::slice::from_ref(&part), 112)?;
            let height = u32::try_from(metric.height + 12)?;
            parts.push((id, metric.width, height));
            y = y
                .checked_add(i16::try_from(height)?)
                .context("Circus comment row y overflow")?;
            for token in tokens(line)? {
                match token {
                    Token::Color(index) => carry = format!("@c{index}"),
                    Token::Rgb(rgb) => {
                        carry = format!(
                            "@c{}",
                            match rgb {
                                [255, 0, 0] => 'r',
                                [0, 255, 0] => 'g',
                                [0, 0, 255] => 'b',
                                [255, 255, 255] => 'w',
                                _ => 'k',
                            }
                        )
                    }
                    _ => {}
                }
            }
        }
        Ok(parts)
    }
    pub(super) fn text_layers(
        &mut self,
        layers: &mut Vec<Value>,
        parent: u32,
        base: u32,
        parts: &[TextPart],
        order: i32,
    ) -> Result<()> {
        self.text_layers_with_defaults(layers,parent,base,parts,order,false)
    }
    pub(in crate::adapter) fn history_text_layers(&mut self,layers:&mut Vec<Value>,parent:u32,base:u32,parts:&[TextPart],order:i32)->Result<()> {
        self.text_layers_with_defaults(layers,parent,base,parts,order,true)
    }
    fn text_layers_with_defaults(&mut self,layers:&mut Vec<Value>,parent:u32,base:u32,parts:&[TextPart],order:i32,native_defaults:bool)->Result<()> {
        let metrics = self.host.text_measurement(self.client)?;
        for (n, part) in parts.iter().enumerate() {
            ensure!(n < 4096, "Circus retained text part budget");
            let color_index = if part.color > 128 {
                part.color - 128
            } else {
                part.color
            };
            let palette = |index: i16| -> Result<[u8; 3]> {
                Ok(self
                    .services
                    .palette
                    .get(&(u8::try_from(index)?))
                    .copied()
                    .unwrap_or(default_palette(index)?))
            };
            let mut rgb = palette(color_index)?;
            let mut size = part.size;
            let mut font_width = None;
            let mut origin = [2, 0];
            let mut glyphs = Vec::new();
            let parsed = tokens_with_defaults(&part.text,native_defaults)?;
            //43E490 gives a line32px height when @l is present; otherwise22.
            let line_metrics = native_line_metrics_with_defaults(&part.text, part.size,native_defaults)?;
            let mut line_index = 0;
            let mut maximum = line_metrics[0].height;
            for token in parsed {
                match token {
                    Token::Size(value) => {
                        size = value;
                        font_width = None;
                    }
                    Token::DefaultFont=>{size=22;font_width=None;},
                    Token::FontWidth(width) => {
                        size = 22;
                        font_width = Some(width);
                    }
                    Token::Color(value) => rgb = palette(value)?,
                    Token::Rgb(value) => rgb = value,
                    Token::ColorUnchanged=>{},
                    Token::Newline => {
                        origin = [
                            2,
                            origin[1] + i32::from(part.line_pitch.unwrap_or(maximum + 12)),
                        ];
                        font_width = None;
                        line_index += 1;
                        maximum = line_metrics[line_index].height;
                    }
                    Token::Ruby { reading, base } => {
                        let (base_cells, reading_cells, advance) = encoded_ruby_run(
                            &base,
                            &reading,
                            origin,
                            22,
                            12,
                            -9,
                            encoding_rs::SHIFT_JIS,
                        )?;
                        for (cells, font_size) in [(base_cells, 22), (reading_cells, 12)] {
                            let ascent = metrics
                                .measure_simple(self.font, font_size as f32, " ")?
                                .ascent;
                            for cell in cells {
                                let mut buffer = [0; 4];
                                let (_, cell_width) = encoded_cell_run(
                                    cell.character.encode_utf8(&mut buffer),
                                    [0, 0],
                                    font_size,
                                    encoding_rs::SHIFT_JIS,
                                )?;
                                glyphs.push(json!({"character":cell.character.to_string(),"baseline":[cell.position[0] as f32,cell.position[1] as f32+10.+ascent],"size":font_size,"cell_width":cell_width,"crop":[0,origin[1],line_metrics[line_index].width,maximum+12],"color":[rgb[0],rgb[1],rgb[2],255]}));
                            }
                        }
                        origin[0] += advance;
                    }
                    Token::Run(value) => {
                        let (cells, advance) = encoded_cell_run(
                            &value,
                            origin,
                            i32::from(size),
                            encoding_rs::SHIFT_JIS,
                        )?;
                        let ascent = metrics.measure_simple(self.font, size as f32, " ")?.ascent;
                        for cell in cells {
                            let mut buffer = [0; 4];
                            let (_, cell_width) = encoded_cell_run(
                                cell.character.encode_utf8(&mut buffer),
                                [0, 0],
                                i32::from(size),
                                encoding_rs::SHIFT_JIS,
                            )?;
                            glyphs.push(json!({"character":cell.character.to_string(),"baseline":[cell.position[0] as f32,cell.position[1] as f32+f32::from(maximum-size)+10.+ascent],"size":size,"font_width":font_width,"cell_width":cell_width,"crop":[0,origin[1],line_metrics[line_index].width,maximum+12],"color":[rgb[0],rgb[1],rgb[2],255]}));
                        }
                        origin[0] += advance;
                    }
                }
            }
            let [x, y] = part.position;
            let outline = (part.color > 128)
                .then(|| json!({"radius":1,"color":[0,0,0,255],"kernel":"square"}));
            let width = line_metrics
                .iter()
                .map(|line| line.width)
                .max()
                .unwrap_or(4);
            layers.push(json!({"op":"put","layer":{"id":base+n as u32,"parent":parent,"rect":[x,y,width,720],"visible":part.state==99||part.state==0,"paint":{"kind":"glyph_run","font":self.font,"size":22,"color":[255,255,255,255],"glyphs":glyphs,"outline":outline},"order":[order,0]}}));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_allocation_metrics_remain_separate_from_glyph_cell_advances() {
        assert_eq!(native_canvas_width("@wAB").unwrap(), 48);
        assert_eq!(native_canvas_width("@wA@mB").unwrap(), 37);
        assert_eq!(native_canvas_width("@sあ").unwrap(), 26);
        assert_eq!(native_canvas_width("@c12A").unwrap(), 37);
        assert_eq!(native_canvas_width("AB\n@lあ").unwrap(), 36);
        let lines = native_line_metrics("@sあ\nい", 22).unwrap();
        assert_eq!((lines[0].height, lines[0].width), (22, 26));
        assert_eq!(
            (lines[1].seed, lines[1].height, lines[1].width),
            (20, 20, 24)
        );
    }
    #[test]
    fn native_size_palette_and_multibyte_ruby_are_distinct() {
        assert_eq!(
            tokens("@s甲@c12乙\n｛よみ／読み｝").unwrap(),
            vec![
                Token::Size(20),
                Token::Run("甲".into()),
                Token::Color(12),
                Token::Run("乙".into()),
                Token::Newline,
                Token::Ruby {
                    reading: "よみ".into(),
                    base: "読み".into()
                }
            ]
        );
    }
    #[test]
    fn nonempty_side_effect_controls_cannot_silently_disappear() {
        assert!(tokens("@t35@d1台詞").is_err());
        assert!(tokens("@cmd5台詞").is_err());
        assert!(tokens("@mu3台詞").is_err());
        assert!(tokens("@c").is_err());
    }
}
