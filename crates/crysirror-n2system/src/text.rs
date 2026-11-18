// SPDX-License-Identifier: MPL-2.0
// N2 markup policy follows NitroSharp Syntax/Markup/MarkupParser.cs e3869842
// and legacy Text/Dialogue.cs 36701328 (MIT). Font measurement, group placement,
// retained glyph coverage and reveal timing are shared CrySirror services.
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::service::TextMeasurement;
use crysirror_text::grid::{
    CellCursor, CellGlyph, CellGroup, CellLayoutOptions, layout_cell_groups,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Style {
    #[serde(default)]
    pub vertical_glyphs: bool,
    pub font: String,
    pub size: u16,
    pub color: [u8; 4],
    pub outline: Option<[u8; 4]>,
    pub bold: bool,
    pub italic_font: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Part {
    Text(String, Style),
    Ruby(String, String, Style),
    Halt,
    NoBreak,
    Voice {
        character: String,
        source: String,
        stop: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub vertical_glyphs: bool,
    pub parts: Vec<Part>,
    pub bounds: [u32; 2],
    pub spacing: [i32; 2],
    pub margin: [i32; 2],
    pub shown: usize,
    pub disable_animation: bool,
    pub line_read: bool,
    pub thread: Option<u32>,
    pub order: i32,
    pub revision: u64,
}

pub fn color(text: &str) -> Result<[u8; 4]> {
    if let Ok(rgb) = u32::from_str_radix(text.trim_start_matches('#'), 16) {
        return Ok([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]);
    }
    Ok(match text.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0, 255],
        "white" | "whited" => [255; 4],
        "gray" => [128, 128, 128, 255],
        "red" => [255, 0, 0, 255],
        "green" => [0, 128, 0, 255],
        "blue" => [0, 0, 255, 255],
        "yellow" => [255, 255, 0, 255],
        _ => bail!("N2System markup color {text}"),
    })
}
pub fn parse(markup: &str, style: Style) -> Result<Vec<Part>> {
    ensure!(markup.len() <= 1024 * 1024, "N2System markup byte capacity");
    let mut parser = Parser {
        text: markup,
        at: 0,
        count: 0,
    };
    parser.content(None, style, 0)
}
struct Parser<'a> {
    text: &'a str,
    at: usize,
    count: usize,
}
impl Parser<'_> {
    fn content(&mut self, end: Option<&str>, mut style: Style, depth: u32) -> Result<Vec<Part>> {
        ensure!(depth <= 64, "N2System markup nesting capacity");
        let mut parts = Vec::new();
        while self.at < self.text.len() {
            self.count += 1;
            ensure!(self.count <= 65536, "N2System markup element capacity");
            let rest = &self.text[self.at..];
            if !rest.starts_with('<') {
                let mut text = String::new();
                while self.at < self.text.len() && !self.text[self.at..].starts_with('<') {
                    let rest = &self.text[self.at..];
                    if rest.starts_with("//") {
                        self.at += rest.find('\n').map_or(rest.len(), |n| n + 1);
                        continue;
                    }
                    let c = rest.chars().next().unwrap();
                    self.at += c.len_utf8();
                    if matches!(c, '&' | '\\') && self.text[self.at..].starts_with(['.', ',']) {
                        let c = self.text[self.at..].chars().next().unwrap();
                        self.at += c.len_utf8();
                        text.push(c);
                    } else {
                        text.push(c);
                    }
                }
                if !text.is_empty() {
                    parts.push(Part::Text(text, style.clone()));
                }
                continue;
            }
            let tail = rest.find('>').context("N2System unterminated markup tag")?;
            let mut tag = &rest[1..tail];
            self.at += tail + 1;
            let closing = tag.starts_with('/');
            if closing {
                tag = &tag[1..];
            }
            let split = tag.find(char::is_whitespace).unwrap_or(tag.len());
            let name = tag[..split].to_ascii_lowercase();
            if closing {
                ensure!(
                    end == Some(name.as_str()),
                    "N2System mismatched markup closing tag {name}"
                );
                return Ok(parts);
            }
            let attributes = attributes(&tag[split..])?;
            let old = style.clone();
            match name.as_str() {
                "font" => {
                    if let Some(size) = attributes.get("size") {
                        style.size = size.parse().context("N2System markup font size")?;
                        ensure!(
                            (1..=512).contains(&style.size),
                            "N2System markup size range"
                        );
                    }
                    if let Some(value) = attributes.get("incolor") {
                        style.color = color(value)?;
                    }
                    if let Some(value) = attributes.get("outcolor") {
                        style.outline = Some(color(value)?);
                    }
                    parts.extend(self.content(Some("font"), style.clone(), depth + 1)?);
                }
                "span" => {
                    // Reference markup uses `value`; this game's scripts write
                    // `SPAN size=0`. Size 0 leaves the inherited size untouched
                    // (unverified against the native 407D40/46F1F0 parser).
                    let size = attributes
                        .get("value")
                        .or_else(|| attributes.get("size"))
                        .with_context(|| format!("N2System span size missing in tag {tag:?}"))?;
                    let size: u16 = size.parse().context("N2System span size")?;
                    if size != 0 {
                        ensure!((1..=512).contains(&size), "N2System span size range");
                        style.size = size;
                    }
                    parts.extend(self.content(Some("span"), style.clone(), depth + 1)?);
                }
                "ruby" => {
                    let reading = attributes
                        .get("text")
                        .context("N2System ruby reading missing")?
                        .clone();
                    let children = self.content(Some("ruby"), style.clone(), depth + 1)?;
                    let mut base = String::new();
                    for child in children {
                        match child {
                            Part::Text(text, _) => base.push_str(&text),
                            _ => bail!("N2System ruby contains a control element"),
                        }
                    }
                    parts.push(Part::Ruby(base, reading, style.clone()));
                }
                "i" => {
                    if let Some(font) = &style.italic_font {
                        style.font = font.clone();
                    }
                    parts.extend(self.content(Some("i"), style.clone(), depth + 1)?);
                }
                "u" => parts.extend(self.content(Some("u"), style.clone(), depth + 1)?),
                "br" => parts.push(Part::Text("\n".into(), style.clone())),
                "k" => parts.push(Part::Halt),
                "?" => parts.push(Part::NoBreak),
                "voice" => parts.push(Part::Voice {
                    character: attributes
                        .get("name")
                        .context("N2System voice name missing")?
                        .clone(),
                    source: attributes
                        .get("src")
                        .context("N2System voice source missing")?
                        .clone(),
                    stop: attributes.get("mode").is_some_and(|m| m == "off"),
                }),
                "pre" => {
                    let rest = &self.text[self.at..];
                    let closing = rest
                        .to_ascii_lowercase()
                        .find("</pre>")
                        .context("N2System unclosed markup PRE")?;
                    parts.push(Part::Text(rest[..closing].into(), style.clone()));
                    self.at += closing + 6;
                }
                _ => bail!("N2System unknown markup tag {name}"),
            }
            style = old;
        }
        ensure!(end.is_none(), "N2System unclosed markup element");
        Ok(parts)
    }
}
fn attributes(mut text: &str) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    while !text.trim().is_empty() {
        text = text.trim_start();
        let equal = text
            .find('=')
            .context("N2System markup attribute missing '='")?;
        let key = text[..equal].trim().to_ascii_lowercase();
        text = text[equal + 1..].trim_start();
        let (value, rest) = if let Some(quoted) = text.strip_prefix('"') {
            let end = quoted
                .find('"')
                .context("N2System unterminated markup attribute")?;
            (&quoted[..end], &quoted[end + 1..])
        } else {
            let end = text.find(char::is_whitespace).unwrap_or(text.len());
            (&text[..end], &text[end..])
        };
        ensure!(
            out.len() < 32 && !key.is_empty() && out.insert(key, value.into()).is_none(),
            "N2System duplicate/excess markup attributes"
        );
        text = rest;
    }
    Ok(out)
}

pub struct Layout {
    pub glyphs: Vec<Json>,
    pub size: [u32; 2],
    pub text: String,
    pub lines: u32,
    pub reveal_positions: Vec<f32>,
}
/// Native runs describe formatting and ruby clusters; the shared cell-group
/// service owns wrapping and bounds. Metrics/kerning are the host's immutable
/// font view. This function produces logical glyphs, never a composed screen.
pub fn layout(page: &Page, measurement: &dyn TextMeasurement) -> Result<Layout> {
    let mut groups = Vec::new();
    let mut styles = Vec::new();
    let mut text = String::new();
    let mut max_size = 1;
    let mut nowrap = false;
    let mut reveal_positions = Vec::new();
    let mut character_position = 0usize;
    for part in page.parts.iter().take(page.shown) {
        match part {
            Part::NoBreak => nowrap = true,
            Part::Text(value, style) => {
                max_size = max_size.max(i32::from(style.size));
                for c in value.chars() {
                    if c == '\r' {
                        continue;
                    }
                    if c == '\n' {
                        groups.push(CellGroup {
                            glyphs: vec![],
                            advance: 0,
                            reset_before: None,
                            automatic_wrap: false,
                            break_before: false,
                            break_after: true,
                        });
                        text.push(c);
                        continue;
                    }
                    let metrics = measurement.measure_simple(
                        &style.font,
                        style.size as f32,
                        &c.to_string(),
                    )?;
                    groups.push(CellGroup {
                        glyphs: vec![CellGlyph {
                            character: c,
                            position: [0, metrics.ascent.ceil() as i32],
                        }],
                        advance: metrics.advance.ceil() as i32 + page.spacing[0],
                        reset_before: None,
                        automatic_wrap: !nowrap,
                        break_before: false,
                        break_after: false,
                    });
                    styles.push(style.clone());
                    reveal_positions.push(character_position as f32);
                    character_position += 1;
                    text.push(c);
                }
            }
            Part::Ruby(base, reading, style) => {
                let b = measurement.measure_simple(&style.font, style.size as f32, base)?;
                let ruby_size = (style.size / 2).max(1);
                let r = measurement.measure_simple(&style.font, ruby_size as f32, reading)?;
                let width = b.advance.max(r.advance);
                let mut glyphs = Vec::new();
                let ruby_start = character_position;
                let base_count = base.chars().count();
                let ruby_count = reading.chars().count();
                for g in measurement.position_simple(&style.font, style.size, base)? {
                    glyphs.push(CellGlyph {
                        character: g.character,
                        position: [
                            ((width - b.advance) / 2. + g.baseline[0]).round() as i32,
                            g.baseline[1].round() as i32,
                        ],
                    });
                    styles.push(style.clone());
                    reveal_positions.push(character_position as f32);
                    character_position += 1;
                }
                let mut ruby = style.clone();
                ruby.size = ruby_size;
                for (index, g) in measurement
                    .position_simple(&style.font, ruby_size, reading)?
                    .into_iter()
                    .enumerate()
                {
                    glyphs.push(CellGlyph {
                        character: g.character,
                        position: [
                            ((width - r.advance) / 2. + g.baseline[0]).round() as i32,
                            (g.baseline[1] - f32::from(style.size) * 0.6).round() as i32,
                        ],
                    });
                    styles.push(ruby.clone());
                    reveal_positions.push(
                        ruby_start as f32
                            + index as f32 / ruby_count.saturating_sub(1).max(1) as f32
                                * base_count.saturating_sub(1) as f32,
                    );
                }
                groups.push(CellGroup {
                    glyphs,
                    advance: width.ceil() as i32 + page.spacing[0],
                    reset_before: None,
                    automatic_wrap: !nowrap,
                    break_before: false,
                    break_after: false,
                });
                text.push_str(base);
                max_size = max_size.max(i32::from(style.size));
            }
            Part::Voice { .. } | Part::Halt => {}
        }
        ensure!(styles.len() <= 65536, "N2System positioned glyph capacity");
    }
    let options = CellLayoutOptions {
        bounds: [page.bounds[0].min(32767) as i32, 32767],
        size: max_size,
        // LoadText's positive line-spacing argument is an absolute baseline
        // pitch, rather than extra space added to the selected font size.
        spacing: [
            0,
            if page.spacing[1] > 0 {
                page.spacing[1] - max_size
            } else {
                0
            },
        ],
        half_width: false,
        half_spacing: false,
        hanging: 0,
        forbidden_line_start: String::new(),
        indent_pairs: Vec::new(),
        cell_widths: Vec::new(),
    };
    let mut cursor = CellCursor::at_start();
    let laid = layout_cell_groups(&groups, &options, &mut cursor, false)?;
    ensure!(
        laid.consumed_groups == groups.len(),
        "N2System text layout vertical capacity"
    );
    let mut glyphs = Vec::new();
    let mut extent = [1u32; 2];
    for (glyph, style) in laid.glyphs.into_iter().zip(styles) {
        let baseline = [
            glyph.position[0] + page.margin[0],
            glyph.position[1] + page.margin[1],
        ];
        extent[0] = extent[0].max((baseline[0] + i32::from(style.size)).max(0) as u32);
        extent[1] = extent[1].max((baseline[1] + i32::from(style.size) / 3).max(0) as u32);
        let outline = style
            .outline
            .map(|color| json!({"radius":1,"color":color,"kernel":"disk"}));
        glyphs.push(json!({"character":glyph.character,"baseline":baseline,"font":style.font,"size":style.size,
            "color":style.color,"bold":style.bold,"style":{"bold":style.bold,"outline":outline,"upright_in_clockwise_line":style.vertical_glyphs}}));
    }
    Ok(Layout {
        glyphs,
        size: extent,
        text,
        reveal_positions,
        lines: if groups.is_empty() {
            0
        } else {
            (cursor.position[1] / (options.size + options.spacing[1])) as u32 + 1
        },
    })
}
