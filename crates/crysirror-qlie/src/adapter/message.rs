//! Native message escape/state policy. Shared cell layout, glyph coverage and
//! retained reveal clocks perform the reusable text work.
use super::*;
use crysirror_core::image_processing::GlyphCoverageQuality;
use crysirror_scene::layer::{
    GlyphOutline, GlyphOutlineKernel, GlyphShadow, LayerSpec, PositionedGlyph, PositionedGlyphStyle,
};
use crysirror_text::grid::{layout_cells_with_breaks, CellCursor, CellLayoutOptions};
//5051A4 ->4FF2B4 builds the original sixteen face/style passes. The
//eight radius-one neighbours plus four radius-two cardinals are exactly
//the shared integer Disk2 kernel, rather than a square radius-two border.
pub(super) fn native_glyph_style(flags: i32, aa: i32, color: i32) -> Result<PositionedGlyphStyle> {
    ensure!(
        flags & !15 == 0,
        "unsupported QLIE native font style bits {flags:#x}"
    );
    let color = color as u32;
    Ok(PositionedGlyphStyle {
        bold: flags & 1 != 0,
        raster: match aa {
            0 => GlyphCoverageQuality::Binary { threshold: 128 },
            1 => GlyphCoverageQuality::Supersample { factor: 2 },
            2 => GlyphCoverageQuality::Supersample { factor: 4 },
            3 => GlyphCoverageQuality::Supersample { factor: 8 },
            _ => bail!("unsupported QLIE font antialias mode {aa}"),
        },
        outline: if flags & 4 != 0 {
            Some(GlyphOutline {
                radius: if flags & 8 != 0 { 2 } else { 1 },
                kernel: if flags & 8 != 0 {
                    GlyphOutlineKernel::Disk
                } else {
                    GlyphOutlineKernel::Square
                },
                color: [(color >> 16) as u8, (color >> 8) as u8, color as u8, 255],
            })
        } else {
            None
        },
        shadow: if flags & 2 != 0 {
            Some(GlyphShadow {
                offset: [3., 3.],
                color: [0, 0, 0, 102],
            })
        } else {
            None
        },
    })
}
#[derive(Clone, Serialize, Deserialize)]
struct FontSlot {
    alias: String,
    style: PositionedGlyphStyle,
}
#[derive(Clone, Serialize, Deserialize)]
struct Style {
    font: String,
    font_slot: i32,
    glyph_style: Option<PositionedGlyphStyle>,
    ruby_font: Option<String>,
    ruby_style: Option<PositionedGlyphStyle>,
    size: [i32; 2],
    color: [u8; 4],
    pitch: [i32; 2],
    ruby_size: [i32; 2],
    ruby_position: [i32; 2],
    ruby_pitch: [i32; 2],
    ruby_color: [u8; 4],
    bold: bool,
    bold_override: Option<bool>,
    interval: u64,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            font: "default".into(),
            font_slot: 0,
            glyph_style: None,
            ruby_font: None,
            ruby_style: None,
            size: [24, 24],
            color: [255; 4],
            pitch: [0, 8],
            ruby_size: [10, 10],
            ruby_position: [0, -8],
            ruby_pitch: [0, 0],
            ruby_color: [255; 4],
            bold: false,
            bold_override: None,
            interval: 30,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Message {
    style: Style,
    defaults: Style,
    area: [i32; 4],
    margin: [i32; 2],
    cursor: CellCursor,
    glyphs: Vec<PositionedGlyph>,
    pub(super) layer: u32,
    pub(super) created: bool,
    pub(super) bound_parent: Option<u32>,
    initial: usize,
    auto_scroll: bool,
    click: u32,
    kinsoku_begin: String,
    kinsoku_end: String,
    kinsoku_length: i32,
    indent: String,
    indent_enabled: bool,
    /// Text added since the page was cleared (+ED0), read by `gt`.
    #[serde(default)]
    pub(super) text: String,
    #[serde(default)]
    pub(super) pictures: glyph_pictures::GlyphPictures,
    /// Picture glyphs held back until their sheets are published again.
    #[serde(skip)]
    pub(super) stashed_images: Vec<(usize, crysirror_scene::layer::GlyphImage)>,
    #[serde(skip)]
    pub(super) remap: BTreeMap<String, String>,
}
impl Message {
    /// TMLSaveData keeps the reveal position (MesPos/State): a page saved
    /// mid-reveal continues revealing from the glyphs already shown.
    pub(super) fn prepare_save(&mut self, visible: usize) {
        self.initial = visible.min(self.glyphs.len());
    }
    pub(super) fn reset_for_restore(&mut self) {
        // Picture sheets are republished by the rebuild; until then their
        // scalars draw with the font.
        if !self.pictures.all_live() {
            for (i, g) in self.glyphs.iter_mut().enumerate() {
                if let Some(image) = g.image.take() {
                    self.stashed_images.push((i, image));
                }
            }
        }
        self.initial = self.initial.min(self.glyphs.len());
        self.created = false;
        self.bound_parent = None;
    }
    pub(super) fn glyphs_mut(&mut self) -> &mut Vec<PositionedGlyph> {
        &mut self.glyphs
    }
    pub(super) fn surface_size(&self) -> [f32; 2] {
        [
            (self.area[2] - self.area[0]).max(1) as f32,
            (self.area[3] - self.area[1]).max(1) as f32,
        ]
    }
    pub(super) fn new(layer: u32, canvas: [u32; 2]) -> Self {
        Self {
            style: Style::default(),
            defaults: Style::default(),
            area: [0, 0, canvas[0] as i32, canvas[1] as i32],
            margin: [0, 0],
            cursor: CellCursor::at_start(),
            glyphs: Vec::new(),
            layer,
            created: false,
            bound_parent: None,
            initial: 0,
            auto_scroll: false,
            click: 0,
            kinsoku_begin: String::new(),
            kinsoku_end: String::new(),
            kinsoku_length: 0,
            indent: String::new(),
            indent_enabled: false,
            text: String::new(),
            pictures: Default::default(),
            stashed_images: Vec::new(),
            remap: BTreeMap::new(),
        }
    }
}
fn integer(a: &[Value], i: usize, default: i32) -> Result<i32> {
    a.get(i)
        .map(Value::integer)
        .transpose()
        .map(|v| v.unwrap_or(default))
}
fn glyphs(h: &dyn HostServices, message: &mut Message, text: &str) -> Result<()> {
    let size = message.style.size[1];
    let options = CellLayoutOptions {
        bounds: [
            (message.area[2] - message.area[0] - message.margin[0] * 2).clamp(1, 32767),
            (message.area[3] - message.area[1] - message.margin[1] * 2).clamp(1, 32767),
        ],
        size,
        spacing: message.style.pitch,
        half_width: true,
        half_spacing: false,
        hanging: message.kinsoku_length.max(0).saturating_mul(size / 2),
        forbidden_line_start: message.kinsoku_begin.clone(),
        indent_pairs: if message.indent_enabled {
            message
                .indent
                .chars()
                .filter_map(|c| match c {
                    '「' => Some(['「', '」']),
                    '（' => Some(['（', '）']),
                    '『' => Some(['『', '』']),
                    _ => None,
                })
                .collect()
        } else {
            vec![]
        },
        cell_widths: message.pictures.cell_widths(message.style.size[0]),
    };
    let mut remaining = text;
    while !remaining.is_empty() {
        let cursor = message.cursor.clone();
        let layout = layout_cells_with_breaks(
            remaining,
            &options,
            &mut message.cursor,
            &message.kinsoku_end,
        )
        .with_context(|| format!("QLIE message layout {remaining:?} at {cursor:?}"))?;
        for glyph in layout.glyphs {
            let ascent = h
                .font_metrics(&message.style.font, size as f32, glyph.character)?
                .ascent;
            let image = message.pictures.image(
                glyph.character,
                glyph.position,
                message.style.size,
                message.style.color,
            );
            message.glyphs.push(PositionedGlyph {
                image,
                style: message.style.glyph_style.map(|mut style| {
                    style.bold = message.style.bold;
                    style
                }),
                font: Some(message.style.font.clone()),
                character: glyph.character,
                baseline: [glyph.position[0] as f32, glyph.position[1] as f32 + ascent],
                size: Some(size as f32),
                color: Some(message.style.color),
                bold: Some(message.style.bold),
                horizontal_scale: Some(message.style.size[0] as f32 / size as f32),
                crop: None,
                font_width: None,
                cell_width: None,
            });
        }
        remaining = &remaining[layout.consumed..];
        if !remaining.is_empty() {
            ensure!(message.auto_scroll, "QLIE message text area overflow");
            let shift = (size + message.style.pitch[1]).max(1);
            for g in &mut message.glyphs {
                g.baseline[1] -= shift as f32;
            }
            message.cursor.position[1] -= shift;
        }
    }
    ensure!(message.glyphs.len() <= 65536, "QLIE message glyph budget");
    Ok(())
}
fn markup(
    h: &dyn HostServices,
    message: &mut Message,
    text: &str,
    fonts: &BTreeMap<i32, FontSlot>,
) -> Result<()> {
    let mut rest = text;
    while !rest.is_empty() {
        let Some(at) = rest.find('[') else {
            glyphs(h, message, rest)?;
            break;
        };
        glyphs(h, message, &rest[..at])?;
        let end = rest[at + 1..]
            .find(']')
            .context("QLIE message tag delimiter")?
            + at
            + 1;
        let body = &rest[at + 1..end];
        let args = crate::program::arguments(body)?;
        let tag = args.first().map(String::as_str).unwrap_or("");
        let n = |i: usize, default: i32| -> Result<i32> {
            args.get(i)
                .map(|v| crate::expression::parse_integer(v).context("QLIE message tag integer"))
                .transpose()
                .map(|v| v.unwrap_or(default))
        };
        match tag {
            "f" => {
                let index = n(1, 0)?;
                let slot = fonts
                    .get(&index)
                    .cloned()
                    .with_context(|| format!("QLIE font {index} is not configured"))?;
                message.style.font = slot.alias;
                message.style.font_slot = index;
                message.style.bold = slot.style.bold;
                message.style.bold_override = None;
                message.style.glyph_style = Some(slot.style);
            }
            "rf" => {
                let index = n(1, 0)?;
                let slot = fonts
                    .get(&index)
                    .cloned()
                    .with_context(|| format!("QLIE ruby font {index} is not configured"))?;
                message.style.ruby_font = Some(slot.alias);
                message.style.ruby_style = Some(slot.style);
            }
            "s" => {
                message.style.size = [n(1, 24)?.clamp(1, 512), n(2, n(1, 24)?)?.clamp(1, 512)];
            }
            "c" | "rc" => {
                let c = n(1, -1)? as u32;
                let color = [(c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8];
                if tag == "c" {
                    message.style.color = color;
                } else {
                    message.style.ruby_color = color;
                }
            }
            "pi" => message.style.pitch = [n(1, 0)?, n(2, 0)?],
            "rs" => {
                message.style.ruby_size = [n(1, 10)?.clamp(1, 512), n(2, n(1, 10)?)?.clamp(1, 512)]
            }
            "rp" => message.style.ruby_position = [n(1, 0)?, n(2, -8)?],
            "rpi" => message.style.ruby_pitch = [n(1, 0)?, n(2, 0)?],
            "n" => glyphs(h, message, "\n")?,
            "p" => {
                message.cursor.position = [n(1, 0)?, n(2, 0)?];
            }
            // 69E7AC [i,n]: advance the current line X (+3624) by n,
            // clamped at the line start; omitted n is ignored.
            "i" => {
                if args.len() >= 2 {
                    message.cursor.position[0] = (message.cursor.position[0]
                        + crate::expression::parse_integer(&args[1]).unwrap_or(0))
                    .max(0);
                }
            }
            "pc" | "pr" => {
                let text = args.get(1).context("QLIE aligned text argument")?;
                let mut preview = message.clone();
                preview.cursor.position[0] = 0;
                preview.glyphs.clear();
                glyphs(h, &mut preview, text)?;
                let width = preview.cursor.position[0];
                let percent = if tag == "pr" { 100 } else { n(2, 50)? };
                message.cursor.position[0] =
                    (((message.area[2] - message.area[0] - message.margin[0] * 2 - width)
                        * percent)
                        / 100
                        - 2)
                    .max(0);
                glyphs(h, message, text)?;
            }
            "cc" => {
                let character =
                    char::from_u32(n(1, 95)? as u32).context("QLIE inline character code")?;
                glyphs(h, message, &character.to_string())?;
            }
            "spd" => message.style.interval = n(1, message.defaults.interval as i32)?.max(0) as u64,
            "b" => {
                let bold = n(1, 1)? != 0;
                message.style.bold = bold;
                message.style.bold_override = Some(bold);
            }
            "rb" => {
                ensure!(args.len() == 3, "QLIE ruby ABI");
                let start = message.cursor.clone();
                let style = message.style.clone();
                glyphs(h, message, &args[1])?;
                let after = message.cursor.clone();
                let base_width = after.position[0] - start.position[0];
                let ruby_width = args[2].chars().count() as i32 * style.ruby_size[0];
                message.cursor = start;
                message.cursor.position[0] +=
                    (base_width - ruby_width) / 2 + style.ruby_position[0];
                // 69FDFC: a ruby run starting left of the line origin is
                // shifted right to start at zero, keeping its width.
                message.cursor.position[0] = message.cursor.position[0].max(0);
                message.cursor.position[1] += style.ruby_position[1] - style.ruby_size[1];
                // The shared layout cursor is nonnegative; ruby may extend
                // above the baseline by a semantic offset applied to its glyphs.
                let offset = message.cursor.position[1].min(0);
                message.cursor.position[1] -= offset;
                let first = message.glyphs.len();
                message.style.size = style.ruby_size;
                message.style.font = style
                    .ruby_font
                    .clone()
                    .unwrap_or_else(|| style.font.clone());
                message.style.glyph_style = style.ruby_style.or(style.glyph_style);
                if let Some(style) = message.style.glyph_style {
                    message.style.bold = style.bold;
                }
                message.style.pitch = style.ruby_pitch;
                message.style.color = style.ruby_color;
                glyphs(h, message, &args[2])?;
                for g in &mut message.glyphs[first..] {
                    g.baseline[1] += offset as f32;
                }
                message.style = style;
                message.cursor = after;
            }
            _ => bail!("unsupported QLIE message tag [{body}]"),
        }
        rest = &rest[end + 1..];
    }
    Ok(())
}
impl QlieVm {
    pub(super) fn publish_message(&mut self, h: &mut dyn HostServices, id: &str) -> Result<()> {
        self.put_parent(h, id)?;
        let o = &self.objects[id];
        let message = o.message.as_ref().unwrap();
        let x = message.area[0] + message.margin[0];
        let y = message.area[1] + message.margin[1];
        let rect = [
            x,
            y,
            (message.area[2] - x).max(1),
            (message.area[3] - y).max(1),
        ];
        let paint = json!({"kind":"glyph_run","font":message.style.font,"size":message.style.size[1],"glyphs":message.glyphs,"reveal":{"interval_ms":message.style.interval,"initial_visible":message.initial}});
        let cmd = if message.created {
            json!({"op":"set_paint","id":message.layer,"paint":paint})
        } else {
            json!({"op":"put","layer":{"id":message.layer,"parent":message.bound_parent.unwrap_or(o.layer),"visible":o.bundle.is_none()||message.bound_parent.is_some(),"rect":rect,"order":[o.position[2],0],"tie_order":u64::from(o.layer)<<32,"paint":paint}})
        };
        let mut commands = vec![cmd];
        if message.created {
            commands.push(json!({"op":"set_rect","id":message.layer,"rect":rect}));
        }
        self.submit(h, json!({"layers":commands}))?;
        self.objects
            .get_mut(id)
            .unwrap()
            .message
            .as_mut()
            .unwrap()
            .created = true;
        Ok(())
    }
    pub(super) fn message_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        ins: &Instruction,
        a: &[Value],
    ) -> Result<bool> {
        if self.objects[id].message.is_none() {
            let layer = self.next_layer;
            self.next_layer += 1;
            let mut message = Message::new(layer, self.config.canvas);
            let speed = self
                .objects
                .get("cfg")
                .and_then(|o| o.properties.get("ms"))
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .unwrap_or(30)
                .max(0) as u64;
            message.style.interval = speed;
            message.defaults.interval = speed;
            self.objects.get_mut(id).unwrap().message = Some(message);
        }
        let op = ins.op.as_str();
        match op {
            "o" | "t" | "n" => {
                let text = if op == "n" {
                    "[n]".into()
                } else if op == "t" {
                    ins.args.first().cloned().unwrap_or_default()
                } else {
                    a.first().map(Value::text).unwrap_or_default()
                };
                let layer = self.objects[id].message.as_ref().unwrap().layer;
                let created = self.objects[id].message.as_ref().unwrap().created;
                let visible = if created {
                    let layer: LayerSpec = serde_json::from_slice(&h.query_engine(
                        self.client,
                        crysirror_core::service::EngineQuery::Layer(layer),
                    )?)?;
                    layer
                        .paint
                        .glyph_reveal_status()
                        .map(|s| s.visible)
                        .unwrap_or(0)
                } else {
                    0
                };
                let mut fonts = BTreeMap::new();
                let library = h.font_library(self.client)?;
                for index in 0..16 {
                    let cfg = self.objects.get("cfg");
                    let number = |key: &str, default: i32| {
                        cfg.and_then(|o| o.properties.get(&format!("{key}/{index}")))
                            .and_then(|v| v.first())
                            .and_then(|v| v.integer().ok())
                            .unwrap_or(default)
                    };
                    let requested = cfg
                        .and_then(|o| {
                            o.properties
                                .get(&format!("font_alias/{index}"))
                                .or_else(|| o.properties.get(&format!("fn2/{index}")))
                        })
                        .and_then(|v| v.first())
                        .map(Value::text)
                        .unwrap_or_else(|| "default".into());
                    let alias = library
                        .resolve_alias(&requested)?
                        .or(library.resolve_alias("default")?)
                        .context("QLIE font slot and fallback unavailable")?;
                    fonts.insert(
                        index,
                        FontSlot {
                            alias,
                            style: native_glyph_style(
                                number("fst2", 1),
                                number("faa2", 2),
                                number("fsc2", 0),
                            )?,
                        },
                    );
                }
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                if let Some(slot) = fonts.get(&msg.style.font_slot) {
                    msg.style.font = slot.alias.clone();
                    msg.style.glyph_style = Some(slot.style);
                    msg.style.bold = msg.style.bold_override.unwrap_or(slot.style.bold);
                }
                msg.initial = visible;
                if msg.text.len() + text.len() <= 1 << 20 {
                    msg.text.push_str(&text);
                }
                let before = msg.glyphs.len();
                markup(h, msg, &text, &fonts)?;
                let added = msg.glyphs.len().saturating_sub(before);
                let auto = self.objects[id]
                    .properties
                    .get("ap")
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(0)
                    != 0;
                if auto && added > 0 {
                    let interval = self
                        .objects
                        .get("cfg")
                        .and_then(|o| o.properties.get("mtw"))
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(40)
                        .max(0);
                    if let Some(sys) = self.objects.get_mut("sys") {
                        let old = sys
                            .properties
                            .get("ap")
                            .and_then(|v| v.first())
                            .and_then(|v| v.integer().ok())
                            .unwrap_or(0);
                        sys.properties.insert(
                            "ap".into(),
                            vec![Value::Int(
                                old.saturating_add((added as i32).saturating_mul(interval)),
                            )],
                        );
                    }
                }
                self.publish_message(h, id)?;
            }
            "cc" => {
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                msg.defaults = msg.style.clone();
                msg.glyphs.clear();
                msg.text.clear();
                msg.cursor = CellCursor::at_start();
                msg.initial = 0;
                msg.click = 0;
                self.publish_message(h, id)?;
            }
            "ts" => {
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                msg.area = if a.len() == 2 {
                    [0, 0, integer(a, 0, 0)?, integer(a, 1, 0)?]
                } else {
                    ensure!(a.len() == 4, "QLIE text area ABI");
                    [
                        integer(a, 0, 0)?,
                        integer(a, 1, 0)?,
                        integer(a, 2, 0)?,
                        integer(a, 3, 0)?,
                    ]
                };
                ensure!(
                    msg.area[2] > msg.area[0] && msg.area[3] > msg.area[1],
                    "QLIE message area"
                );
            }
            "tp" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .cursor
                    .position = [integer(a, 0, 0)?, integer(a, 1, 0)?];
            }
            "?tp" => {
                for (i, value) in self.objects[id]
                    .message
                    .as_ref()
                    .unwrap()
                    .cursor
                    .position
                    .into_iter()
                    .enumerate()
                {
                    m.result(i, Value::Int(value));
                }
            }
            "?tmg" => {
                for (i, value) in self.objects[id]
                    .message
                    .as_ref()
                    .unwrap()
                    .margin
                    .into_iter()
                    .enumerate()
                {
                    m.result(i, Value::Int(value));
                }
            }
            "?tda" => {
                for (i, value) in self.objects[id]
                    .message
                    .as_ref()
                    .unwrap()
                    .area
                    .into_iter()
                    .enumerate()
                {
                    m.result(i, Value::Int(value));
                }
            }
            "tmg" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .margin = [integer(a, 0, 0)?, integer(a, 1, 0)?]
            }
            "as" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .auto_scroll = integer(a, 0, 0)? != 0
            }
            "nsw" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .indent = a.first().map(Value::text).unwrap_or_default();
            }
            "kwb" | "kwe" => {
                let text = match a.first() {
                    Some(Value::Int(0)) | None => String::new(),
                    Some(value) => value.text(),
                };
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                if op == "kwb" {
                    msg.kinsoku_begin = text;
                } else {
                    msg.kinsoku_end = text;
                }
            }
            "kwl" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .kinsoku_length = integer(a, 0, 0)?
            }
            "ns" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .message
                    .as_mut()
                    .unwrap()
                    .indent_enabled = integer(a, 0, 0)? != 0;
            }
            "ap" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            // 6F7484 queries of the message settings: ?as +3750, ?ap +3751,
            // ?kwb/?kwe +3556/+3560, ?kwl +3736, ?kwd +3752, ?nsw +3564,
            // ?ns +3728, ?tlt +3749, ?ts the text area size.
            "?as" => {
                let on = self.objects[id].message.as_ref().unwrap().auto_scroll;
                m.result(0, Value::Int(i32::from(on)));
            }
            "?ap" => {
                let on = self.objects[id]
                    .properties
                    .get("ap")
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(0)
                    == 1;
                m.result(0, Value::Int(i32::from(on)));
            }
            "?kwb" | "?kwe" | "?nsw" => {
                let msg = self.objects[id].message.as_ref().unwrap();
                let text = match op {
                    "?kwb" => msg.kinsoku_begin.clone(),
                    "?kwe" => msg.kinsoku_end.clone(),
                    _ => msg.indent.clone(),
                };
                m.result_text(0, text);
            }
            "?kwl" => {
                let length = self.objects[id].message.as_ref().unwrap().kinsoku_length;
                m.result(0, Value::Int(length));
            }
            "?ns" => {
                let on = self.objects[id].message.as_ref().unwrap().indent_enabled;
                m.result(0, Value::Int(i32::from(on)));
            }
            // kwd,v (+3752) and tlt,v (69E420, +3749) are kept settings.
            "kwd" | "tlt" => {
                let value = if op == "kwd" {
                    i32::from(integer(a, 0, 0)? == 1)
                } else {
                    integer(a, 0, 0)? & 0xFF
                };
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(format!("msg/{op}"), vec![Value::Int(value)]);
            }
            "?kwd" | "?tlt" => {
                let value = self.objects[id]
                    .properties
                    .get(&format!("msg/{}", &op[1..]))
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or(Value::Int(0));
                m.result(0, value);
            }
            "?ts" => {
                let area = self.objects[id].message.as_ref().unwrap().area;
                m.result(0, Value::Int(area[2] - area[0]));
                m.result(1, Value::Int(area[3] - area[1]));
            }
            "mc" | "sk" => {
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                msg.click = msg.click.saturating_add(1);
                let layer = msg.layer;
                if msg.created {
                    self.submit(h, json!({"layers":[{"op":"complete_reveal","id":layer}]}))?;
                }
            }
            "?mc" => {
                let msg = self.objects[id].message.as_ref().unwrap();
                let pending = if msg.created {
                    let layer: LayerSpec = serde_json::from_slice(&h.query_engine(
                        self.client,
                        crysirror_core::service::EngineQuery::Layer(msg.layer),
                    )?)?;
                    layer
                        .paint
                        .glyph_reveal_status()
                        .is_some_and(|s| !s.complete)
                } else {
                    false
                };
                m.result(0, Value::Int(i32::from(pending)));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
