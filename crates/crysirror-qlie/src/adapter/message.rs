//! Native message escape/state policy. Shared cell layout, glyph coverage and
//! retained reveal clocks perform the reusable text work.
use super::*;
use crysirror_scene::layer::{LayerSpec, PositionedGlyph};
use crysirror_text::grid::{layout_cells_with_breaks, CellCursor, CellLayoutOptions};
#[derive(Clone)]
struct Style {
    font: String,
    ruby_font: Option<String>,
    size: [i32; 2],
    color: [u8; 4],
    pitch: [i32; 2],
    ruby_size: [i32; 2],
    ruby_position: [i32; 2],
    ruby_pitch: [i32; 2],
    ruby_color: [u8; 4],
    bold: bool,
    interval: u64,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            font: "default".into(),
            ruby_font: None,
            size: [24, 24],
            color: [255; 4],
            pitch: [0, 8],
            ruby_size: [10, 10],
            ruby_position: [0, -8],
            ruby_pitch: [0, 0],
            ruby_color: [255; 4],
            bold: false,
            interval: 30,
        }
    }
}
#[derive(Clone)]
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
}
impl Message {
    pub(super) fn surface_size(&self) -> [f32; 2] {
        [
            (self.area[2] - self.area[0]).max(1) as f32,
            (self.area[3] - self.area[1]).max(1) as f32,
        ]
    }
    fn new(layer: u32, canvas: [u32; 2]) -> Self {
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
    };
    let mut remaining = text;
    while !remaining.is_empty() {
        let layout = layout_cells_with_breaks(
            remaining,
            &options,
            &mut message.cursor,
            &message.kinsoku_end,
        )?;
        for glyph in layout.glyphs {
            let ascent = h
                .font_metrics(&message.style.font, size as f32, glyph.character)?
                .ascent;
            message.glyphs.push(PositionedGlyph {
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
    fonts: &BTreeMap<i32, String>,
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
                let alias = fonts
                    .get(&index)
                    .cloned()
                    .or_else(|| (index == 0).then(|| "default".into()))
                    .with_context(|| format!("QLIE font {index} is not configured"))?;
                message.style.font = alias;
            }
            "rf" => {
                let index = n(1, 0)?;
                message.style.ruby_font = Some(
                    fonts
                        .get(&index)
                        .cloned()
                        .or_else(|| (index == 0).then(|| "default".into()))
                        .with_context(|| format!("QLIE ruby font {index} is not configured"))?,
                );
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
            "b" => message.style.bold = n(1, 1)? != 0,
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
    fn publish_message(&mut self, h: &mut dyn HostServices, id: &str) -> Result<()> {
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
            json!({"op":"put","layer":{"id":message.layer,"parent":message.bound_parent.unwrap_or(o.layer),"visible":o.bundle.is_none()||message.bound_parent.is_some(),"rect":rect,"order":[i32::MAX,0],"paint":paint}})
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
                let fonts = self
                    .objects
                    .get("cfg")
                    .map(|cfg| {
                        cfg.properties
                            .iter()
                            .filter_map(|(key, value)| {
                                key.strip_prefix("font_alias/")
                                    .and_then(|n| n.parse::<i32>().ok())
                                    .map(|n| (n, value[0].text()))
                            })
                            .collect::<BTreeMap<_, _>>()
                    })
                    .unwrap_or_default();
                let msg = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
                msg.initial = visible;
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
