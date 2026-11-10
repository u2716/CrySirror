// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy message records, retained shared GlyphRuns and native click waits.
//! Shared Text owns cell layout, font metrics and reveal; workers never borrow
//! HostServices. Native encoded rows, names and mode policy remain here.
use crate::{
    config::{Names, integer},
    machine::{Machine, NativeCall, Payload},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{EngineQuery, HostServices, InputFrame, TextMeasurement},
    state::ClientId,
};
use crysirror_scene::layer::LayerSpec;
use crysirror_tasks::{Task, TaskPool};
use crysirror_text::grid::{CellCursor, CellLayoutOptions, layout_cells};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
mod background;
mod cutin;
pub use cutin::CutinPolicy;
mod draw;
pub(crate) use draw::DrawView;
mod timing;
mod window_visibility;

#[derive(Clone, Debug)]
pub struct TextFace {
    pub font: String,
    pub size: u16,
    pub spacing: i32,
    pub color: [u8; 4],
}
#[derive(Clone, Debug)]
pub struct MessageWindow {
    pub id: u32,
    pub asset: String,
    pub rect: [i32; 4],
    pub source_size: [u32; 2],
    /// Native percent0..100; exact /256 profile is processed by engine jobs.
    pub percent: u16,
    /// Native notTransWindowUse bits:2 draws a lower source plane before
    /// the main window,1 draws source row+height after it. These are original
    /// asset-stack semantics, not a renderer or pixel operation.
    pub stacked_planes: u8,
    /// Engine source whose alpha is the original mask, or shared CornerKey
    /// extraction when native DWQ metadata proves the image has no mask.
    pub resource: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
}
#[derive(Clone, Debug)]
pub struct MessageBindings {
    pub client: ClientId,
    pub root: u32,
    pub body_layer: u32,
    pub name_layer: u32,
    pub order: i32,
    pub canvas: [u32; 2],
    pub window: Option<MessageWindow>,
}
#[derive(Clone, Debug)]
pub struct MessageProfile {
    pub client: ClientId,
    pub root: u32,
    pub body_layer: u32,
    pub name_layer: u32,
    pub order: i32,
    pub canvas: [u32; 2],
    pub body: TextFace,
    pub name: TextFace,
    pub print_origin: [i32; 2],
    pub print_next: [i32; 2],
    pub lprint_origin: [i32; 2],
    pub lprint_next: [i32; 2],
    pub name_origin: [i32; 2],
    pub name_enabled: bool,
    pub auto_name: bool,
    pub name_center: i32,
    pub name_length: i32,
    pub window_rows: usize,
    pub full_rows: usize,
    pub reveal_per_second: u32,
    pub name_colors: Arc<BTreeMap<Vec<u8>, [u8; 4]>>,
    pub window: Option<MessageWindow>,
    pub name_window: Option<MessageWindow>,
    /// Original MyMessage one-byte-to-two-byte resource table, supplied by a
    /// reviewed module/config worker. No source/font table is bundled here.
    pub single_byte_map: Option<Arc<[char]>>,
    pub advance_buttons: u64,
    pub cannot_click: bool,
    /// Native Cutin class and mode must both be active before PRINT uses its
    /// selector. Inactive selectors are inert, matching SetMessageData.
    pub cutin_mode: i32,
    pub cutin: Option<Arc<CutinPolicy>>,
    pub cutin_window_percent: u16,
    /// CMyFont ruby policy from the game setup; None when rubiFlag is 0.
    pub ruby: Option<RubyProfile>,
    pub inline: InlineProfile,
    pub body_shadow: Option<[u8; 4]>,
    pub name_shadow: Option<[u8; 4]>,
}
/// Native text substitutions and command colour table, read from the installed
/// module rather than copying its data into the engine. Names are session-owned.
#[derive(Clone, Debug, Default)]
pub struct InlineProfile {
    pub names: [Vec<u8>; 2],
    pub flower: Vec<u8>,
    pub replacements: Vec<Vec<u8>>,
    pub colors: BTreeMap<u16, [u8; 4]>,
    pub gaiji: BTreeMap<u16, u8>,
    pub user_gaiji:BTreeMap<u16,u8>,
    pub images: BTreeMap<(u8, u16), String>,
}
impl InlineProfile {
    pub(crate) fn mugen2(module: &[u8]) -> Result<Self> {
        let pe = crysirror_resource::pe::PeData::parse(module)?;
        let text = |va, n| -> Result<Vec<u8>> {
            let data = pe.va_slice(va, n)?;
            Ok(data[..data.iter().position(|b| *b == 0).unwrap_or(data.len())].to_vec())
        };
        let mut result = Self {
            flower: text(0x575a58, 20)?,
            ..Default::default()
        };
        for (va, limit, gaiji) in [(0x576278, 100, false), (0x575bf8, 20, true)] {
            for (i, entry) in pe.va_slice(va, limit * 8)?.chunks_exact(8).enumerate() {
                let code = u32::from_le_bytes(entry[..4].try_into()?) as u16;
                if code == 0x8f49 {
                    break;
                }
                let value = u32::from_le_bytes(entry[4..].try_into()?);
                if gaiji {
                    result.gaiji.insert(code, i as u8 + 5);
                } else {
                    result.colors.insert(
                        code,
                        [value as u8, (value >> 8) as u8, (value >> 16) as u8, 255],
                    );
                }
            }
        }
        for i in 0..10 {
            result.replacements.push(text(0x576598 + i * 34, 34)?);
        }
        Ok(result)
    }
}
/// Game-setup ruby policy (MUGEN2 init 0x546931..0x546a9e; defaults are the
/// .data values percent40, sukima1, colour255, deltaY0, colourFix0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubyProfile {
    pub percent: i32,
    pub delta_y: i32,
    pub spacing: i32,
    pub color: [u8; 4],
    pub fixed_color: bool,
}
impl RubyProfile {
    pub fn from_game_setup(game: &Names) -> Result<Option<Self>> {
        let n = |key: &str, default: i32| -> Result<i32> {
            Ok(game.integer(key.as_bytes())?.unwrap_or(default))
        };
        if n("rubiFlag", 0)? == 0 {
            return Ok(None);
        }
        let percent = n("rubiPercent", 40)?;
        ensure!((1..=100).contains(&percent), "NNN ruby percent profile");
        let byte = |key: &str| -> Result<u8> { Ok(n(key, 255)?.clamp(0, 255) as u8) };
        Ok(Some(Self {
            percent,
            delta_y: n("rubiDeltaY", 0)?,
            spacing: n("rubiSukima", 1)?,
            color: [
                byte("rubiColorR")?,
                byte("rubiColorG")?,
                byte("rubiColorB")?,
                255,
            ],
            fixed_color: n("rubiColorFixFlag", 0)? != 0,
        }))
    }
}
/// One `#［N...］` annotation: base cells `[start, start+len)` of its row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubyRun {
    pub start: usize,
    pub len: usize,
    pub text: Vec<char>,
}
impl MessageProfile {
    /// Defaults and key bindings are verified against complete legacy native
    /// CCommonPrintMessage constructor0x500fa0, not a modern struct layout.
    pub fn from_legacy_setup(
        setup: &Names,
        bindings: MessageBindings,
        font: &str,
        speed: usize,
        name_colors: Option<&Names>,
        single_byte_map: Option<Arc<[char]>>,
        advance_buttons: u64,
    ) -> Result<Self> {
        ensure!(speed < 5, "NNN native message speed index");
        let n = |key: &str, default: i32| -> Result<i32> {
            Ok(setup.integer(key.as_bytes())?.unwrap_or(default))
        };
        let size = n("fontSize", 24)?;
        let x = n("printX", 100)?;
        let y = n("printY", 100)?;
        let dx = n("nextX", 0)?;
        let dy = n("nextY", size + 6)?;
        let lx = n("LPrintX", 100)?;
        let ly = n("LPrintY", 100)?;
        let ldx = n("LNextX", dx)?;
        let ldy = n("LNextY", dy)?;
        ensure!(ldy > 0, "NNN native full message line step");
        let custom = n("colorCustomFlag", 0)? != 0;
        let color = |prefix: &str| -> Result<[u8; 4]> {
            Ok(if custom {
                [
                    n(&format!("{prefix}ColorR"), 255)? as u8,
                    n(&format!("{prefix}ColorG"), 255)? as u8,
                    n(&format!("{prefix}ColorB"), 255)? as u8,
                    255,
                ]
            } else {
                [255; 4]
            })
        };
        let body_color = if custom {
            [
                n("colorR", 255)? as u8,
                n("colorG", 255)? as u8,
                n("colorB", 255)? as u8,
                255,
            ]
        } else {
            [255; 4]
        };
        let use_colors = n("nameColorChangeFlag", 1)? != 0;
        let auto = n("autoNameWindowFlag", 0)? != 0;
        let colors = if use_colors || auto {
            Self::parse_name_colors(
                name_colors.context("NNN original name-color file not prepared")?,
            )?
        } else {
            BTreeMap::new()
        };
        let rates = [20, 40, 60, 120, 10000];
        let rate = if n("customMessageSpeed", 0)? != 0 {
            n(&format!("messageSpeed{}", speed + 1), rates[speed])?
        } else {
            rates[speed]
        };
        let profile = Self {
            client: bindings.client,
            root: bindings.root,
            body_layer: bindings.body_layer,
            name_layer: bindings.name_layer,
            order: bindings.order,
            canvas: bindings.canvas,
            body: TextFace {
                font: font.into(),
                size: u16::try_from(size)?,
                spacing: n("space", 1)?,
                color: body_color,
            },
            name: TextFace {
                font: font.into(),
                size: u16::try_from(n("nameFontSize", 20)?)?,
                spacing: n("nameSpace", 1)?,
                color: color("name")?,
            },
            print_origin: [x, y],
            print_next: [dx, dy],
            lprint_origin: [lx, ly],
            lprint_next: [ldx, ldy],
            name_origin: [n("nameX", x - dx)?, n("nameY", y - dy)?],
            name_enabled: n("nameFlag", 1)? != 0,
            auto_name: auto,
            name_center: n("nameCenterFlag", 1)?,
            name_length: n("nameLengthMax", 5)?,
            window_rows: usize::try_from(n("windowMessageNumber", 4)?)?,
            full_rows: usize::try_from(n(
                "fullMessageNumber",
                (bindings.canvas[1] as i32 - ly) / ldy - 1,
            )?)?,
            reveal_per_second: u32::try_from(rate)?,
            name_colors: Arc::new(colors),
            window: bindings.window,
            name_window: None,
            single_byte_map,
            advance_buttons,
            cannot_click: false,
            cutin_mode: 0,
            cutin: None,
            cutin_window_percent: 100,
            ruby: None,
            inline: InlineProfile::default(),
            // The reviewed legacy printer passes packed RRGGBB directly to
            // SetTextColor(COLORREF), unlike the corrected later source.
            body_shadow: Some(if custom {
                [
                    n("shadeColorB", 0)? as u8,
                    n("shadeColorG", 0)? as u8,
                    n("shadeColorR", 0)? as u8,
                    255,
                ]
            } else {
                [0, 0, 0, 255]
            }),
            name_shadow: Some(if custom {
                [
                    n("nameShadeColorB", 0)? as u8,
                    n("nameShadeColorG", 0)? as u8,
                    n("nameShadeColorR", 0)? as u8,
                    255,
                ]
            } else {
                [0, 0, 0, 255]
            }),
        };
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.cutin_window_percent<=100,"NNN Cutin window percent bounds");
        ensure!(
            self.client.0 != 0
                && self.root != 0
                && self.body_layer != self.name_layer
                && self.root != self.body_layer
                && self.root != self.name_layer,
            "NNN text identities"
        );
        if self.cutin.is_some() {
            let ids=cutin::node_ids(self)?;
            ensure!(!ids.iter().any(|id|*id==self.root||*id==self.name_layer||self.window.iter().chain(self.name_window.iter()).any(|w|w.id==*id)),"NNN Cutin layer identity collision");
        }
        ensure!(
            self.canvas.iter().all(|n| (1..=8192).contains(n))
                && self.window_rows > 0
                && self.window_rows <= 64
                && self.full_rows > 0
                && self.full_rows <= 64,
            "NNN text canvas/row limits"
        );
        ensure!(
            (1..=100000).contains(&self.reveal_per_second) && self.advance_buttons != 0,
            "NNN reveal/input profile"
        );
        ensure!(
            (0..=2).contains(&self.name_center) && (0..=256).contains(&self.name_length),
            "NNN name alignment"
        );
        for face in [&self.body, &self.name] {
            ensure!(
                !face.font.is_empty()
                    && (1..=512).contains(&face.size)
                    && (-511..=4096).contains(&face.spacing),
                "NNN text face"
            );
        }
        for p in [
            self.print_origin,
            self.print_next,
            self.lprint_origin,
            self.lprint_next,
            self.name_origin,
        ] {
            ensure!(
                p.iter().all(|v| (-32767..=32767).contains(v)),
                "NNN text position"
            );
        }
        for w in self.window.iter().chain(self.name_window.iter()) {
            ensure!(
                w.percent <= 100 && w.stacked_planes <= 3,
                "NNN message window percent/plane bounds"
            );
            if let Some(image) = &w.resource {
                ensure!(
                    image.client() == self.client && image.size() == w.source_size,
                    "NNN scoped message-window resource"
                );
            } else {
                ensure!(
                    w.percent == 100 && w.stacked_planes == 0,
                    "NNN translucent window requires engine resource"
                );
            }
            ensure!(
                w.id != self.root
                    && w.id != self.body_layer
                    && w.id != self.name_layer
                    && w.rect[2] > 0
                    && w.rect[3] > 0
                    && !w.asset.is_empty()
                    && w.source_size[0] >= w.rect[2] as u32
                    && w.source_size[1] >= w.rect[3] as u32,
                "NNN message window binding"
            );
        }
        ensure!(
            self.window
                .as_ref()
                .zip(self.name_window.as_ref())
                .is_none_or(|(a, b)| a.id != b.id),
            "NNN window layer collision"
        );
        if let Some(map) = &self.single_byte_map {
            ensure!(map.len() == 192, "NNN original single-byte mapping size");
        }
        Ok(())
    }
    /// The name-color file is groups of original encoded name,R,G,B tokens.
    pub fn parse_name_colors(names: &Names) -> Result<BTreeMap<Vec<u8>, [u8; 4]>> {
        ensure!(names.tokens.len() % 4 == 0, "NNN name color tuple size");
        let mut out = BTreeMap::new();
        for row in names.tokens.chunks_exact(4) {
            out.entry(row[0].clone()).or_insert([
                integer(&row[1])? as u8,
                integer(&row[2])? as u8,
                integer(&row[3])? as u8,
                255,
            ]);
        }
        Ok(out)
    }
}
/// Read original character-mapping data from the reviewed owner module. This
/// never executes PE code, bundles the table or infers a foreign module ABI.
pub fn mugen2_single_byte_map(module: &[u8]) -> Result<Arc<[char]>> {
    let digest = crysirror_resource::pe::PeData::parse(module)?
        .sha256()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    ensure!(
        digest == crate::native_bootstrap::MUGEN2_MODULE_SHA256,
        "NNN unreviewed character map module"
    );
    let table = module
        .get(0x173c70..0x173c70 + 384)
        .context("NNN original character map bounds")?;
    ensure!(
        module.get(0x175108..0x175108 + 384) == Some(table),
        "NNN original character map copies differ"
    );
    let mut out = Vec::with_capacity(192);
    for pair in table.chunks_exact(2) {
        let text = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(pair)
            .context("NNN original character map encoding")?;
        let mut cells = text.chars();
        let c = cells
            .next()
            .context("NNN original character map empty cell")?;
        ensure!(
            cells.next().is_none(),
            "NNN original character map scalar width"
        );
        out.push(c);
    }
    Ok(Arc::from(out))
}
/// Decode exactly one native display cell at a time. Unknown markup fails
/// before publication; encoded rows never become silently replaced Unicode.
pub fn plain_cells(raw: &[u8], single_byte_map: Option<&[char]>) -> Result<String> {
    Ok(cells_with_ruby(raw, single_byte_map)?.0)
}
fn decode_pair(pair: &[u8]) -> Result<char> {
    let text = encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(pair)
        .context("NNN invalid Shift_JIS display cell")?;
    let mut chars = text.chars();
    let c = chars.next().context("NNN empty encoded display cell")?;
    ensure!(chars.next().is_none(), "NNN encoded cell is not one scalar");
    Ok(c)
}
/// CMyMessage::MakeMessage (two-byte code) display cells plus ruby records.
/// `#` reads one little-endian code word; `［` (81 6D) starts ruby whose
/// base length is an ASCII 1..9 byte or full-width ０..９ pair, then two-byte
/// ruby cells up to `］` (81 6E). Malformed ruby is consumed without a record,
/// as natively. Other `#` commands still fail before publication.
pub fn cells_with_ruby(
    raw: &[u8],
    single_byte_map: Option<&[char]>,
) -> Result<(String, Vec<RubyRun>)> {
    let (text, ruby, _) = styled_cells(raw, single_byte_map, None)?;
    Ok((text, ruby))
}
fn styled_cells(
    raw: &[u8],
    single_byte_map: Option<&[char]>,
    inline: Option<&InlineProfile>,
) -> Result<(String, Vec<RubyRun>, Vec<Option<[u8; 4]>>)> {
    ensure!(
        raw.len() <= 254 && !raw.contains(&0),
        "NNN message row bytes"
    );
    let mut out = String::new();
    let mut cells = 0usize;
    let mut ruby = Vec::new();
    let mut colors = Vec::new();
    let mut color = None;
    let mut at = 0;
    while at < raw.len() {
        let b = raw[at];
        if b == b'#' {
            let code = raw
                .get(at + 1..at + 3)
                .context("NNN truncated inline markup")?;
            let command = u16::from_be_bytes([code[0], code[1]]);
            at += 3;
            if command != 0x816d {
                let policy = inline.context("NNN inline command profile not prepared")?;
                if command == 0x89fc {
                    out.push('\n');
                    continue;
                }
                if let Some(c) = policy.colors.get(&command) {
                    color = Some(*c);
                    continue;
                }
                let replacement = match command {
                    0x90a9 => Some(&policy.names[0]),
                    0x96bc => Some(&policy.names[1]),
                    0x89d4 => Some(&policy.flower),
                    0x824f..=0x8258 => policy.replacements.get((command - 0x824f) as usize),
                    _ => None,
                };
                if let Some(bytes) = replacement {
                    let text = plain_cells(bytes, single_byte_map)?;
                    let count = text.chars().count();
                    if cells + count <= 256 {
                        cells += count;
                        colors.extend(std::iter::repeat_n(color, count));
                        out.push_str(&text);
                    }
                    continue;
                }
                if let Some(dat) = policy.gaiji.get(&command).or_else(||policy.user_gaiji.get(&command)) {
                    out.push(char::from_u32(0xf0000 + u32::from(*dat)).unwrap());
                    colors.push(color);
                    cells += 1;
                    continue;
                }
                // Native43B8E0/GetCommand=-1 consumes the complete command and
                // continues. This is the original unknown-token policy, not an
                // unsupported syscall acknowledgement.
                continue;
            }
            let length = match raw.get(at) {
                Some(digit @ b'1'..=b'9') => {
                    at += 1;
                    i32::from(digit - b'0')
                }
                _ => {
                    let pair = raw.get(at..at + 2).context("NNN truncated ruby length")?;
                    at += 2;
                    i32::from(u16::from_be_bytes([pair[0], pair[1]])) - 0x824f
                }
            };
            if !(1..=9).contains(&length) {
                continue;
            }
            let mut text = Vec::new();
            let mut closed = false;
            while text.len() < 100 {
                let Some(pair) = raw.get(at..at + 2) else {
                    break;
                };
                at += 2;
                if pair == [0x0d, 0x0a] {
                    break;
                }
                if pair == [0x81, 0x6e] {
                    closed = !text.is_empty();
                    break;
                }
                text.push(decode_pair(pair)?);
            }
            if closed {
                let len = length as usize;
                ensure!(cells >= len, "NNN ruby base precedes the row");
                ruby.push(RubyRun {
                    start: cells - len,
                    len,
                    text,
                });
            }
            continue;
        }
        if b < 0x20 {
            at += 1;
            continue;
        }
        if (0x20..=0x7f).contains(&b) || (0xa0..=0xdf).contains(&b) {
            let map = single_byte_map.context("NNN original one-byte cell mapping not prepared")?;
            out.push(
                *map.get((b - 0x20) as usize)
                    .context("NNN single-byte cell mapping bounds")?,
            );
            at += 1;
        } else {
            let pair = raw
                .get(at..at + 2)
                .context("NNN incomplete Shift_JIS display cell")?;
            out.push(decode_pair(pair)?);
            at += 2;
        }
        cells += 1;
        colors.push(color);
    }
    Ok((out, ruby, colors))
}
/// CMyFont::MakePic two-byte ruby placement for one row: x of each ruby cell
/// relative to the row origin. `cell` is fontSize+space, `cells` the row
/// length; ruby width is rubySize+rubiSukima.
pub fn ruby_positions(runs: &[RubyRun], cell: i32, cells: usize, ruby_width: i32) -> Vec<Vec<i32>> {
    let right_max = cell * cells as i32 - ruby_width;
    let mut left_limit = 0;
    runs.iter()
        .map(|run| {
            let length = run.text.len() as i32;
            let mut x =
                run.start as i32 * cell + run.len as i32 * cell / 2 - ruby_width * length / 2;
            let mut last = x + ruby_width * (length - 1);
            if x < left_limit {
                last += left_limit - x;
                x = left_limit;
            }
            if last > right_max {
                x -= last - right_max;
                last = right_max;
            }
            if x < left_limit {
                x = left_limit;
            }
            if last < x {
                last = x;
            }
            left_limit = (last + ruby_width).min(right_max);
            let step = (last - x) / (length - 1).max(1);
            (0..length).map(|i| x + i * step).collect()
        })
        .collect()
}
#[derive(Clone, Debug)]
pub struct NativeMessageState {
    pub mode: u32,
    pub serial: u32,
    pub rows: Vec<Vec<u8>>,
    /// Native RAM has 64 rows independently of m_messageKosuu. PRINT can
    /// hide rows whose bytes still contribute to a Cutin window's extent.
    /// SaveMessage writes only visible rows; this is not a larger save block.
    pub row_storage: Vec<Vec<u8>>,
    pub cutin_rows: Vec<[i32; 2]>,
    pub window_visible: bool,
    pub body_cells: usize,
    pub next_window_override: Option<bool>,
}
impl Default for NativeMessageState {
    fn default() -> Self {
        Self {
            mode: 0x22220001,
            serial: u32::MAX,
            rows: Vec::new(),
            row_storage: vec![Vec::new();64],
            cutin_rows: Vec::new(),
            window_visible: false,
            body_cells: 0,
            next_window_override: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MessagePlan {
    pub next: NativeMessageState,
    pub batch: Value,
}
pub(crate) fn positioned(
    text: &str,
    face: &TextFace,
    origin: [i32; 2],
    metrics: &dyn TextMeasurement,
) -> Result<Vec<Value>> {
    positioned_with_step(text, face, origin, i32::from(face.size), metrics)
}
fn positioned_with_step(
    text: &str,
    face: &TextFace,
    origin: [i32; 2],
    next_y: i32,
    metrics: &dyn TextMeasurement,
) -> Result<Vec<Value>> {
    let extent = metrics.measure_simple(&face.font, f32::from(face.size), "")?;
    let options = CellLayoutOptions {
        cell_widths: Vec::new(),
        bounds: [32767, 32767],
        size: i32::from(face.size),
        spacing: [face.spacing, next_y - i32::from(face.size)],
        half_width: false,
        half_spacing: false,
        hanging: 0,
        forbidden_line_start: String::new(),
        indent_pairs: vec![],
    };
    let result = layout_cells(text, &options, &mut CellCursor::at_start())?;
    ensure!(
        result.consumed == text.len(),
        "NNN native row exceeded shared cell layout"
    );
    Ok(result.glyphs.into_iter().map(|g|json!({"character":g.character,"baseline":[origin[0] as f32+g.position[0] as f32,origin[1] as f32+g.position[1] as f32+extent.ascent]})).collect())
}
fn bind_inline_images(
    glyphs: &mut [Value],
    face: &TextFace,
    inline: &InlineProfile,
    metrics: &dyn TextMeasurement,
) -> Result<()> {
    let ascent = metrics
        .measure_simple(&face.font, face.size as f32, "")?
        .ascent;
    for glyph in glyphs {
        let scalar = glyph["character"]
            .as_str()
            .and_then(|s| s.chars().next())
            .context("NNN inline scalar absent")? as u32;
        if !(0xf0001..=0xf0036).contains(&scalar) {
            continue;
        }
        let number = (scalar - 0xf0000) as u8;
        let path = inline
            .images
            .get(&(number, face.size))
            .context("NNN inline gaiji image is missing from the installed font sheet")?;
        glyph["image"] = json!({"asset":path,"rect":[glyph["baseline"][0],glyph["baseline"][1].as_f64().unwrap()-f64::from(ascent),u32::from(face.size)+1,face.size],"uv":[0.,0.,1.,1.],"tint":glyph.get("color").cloned().unwrap_or(json!(face.color))});
    }
    Ok(())
}
impl NativeMessageState {
    /// CPU-only preparation for a scoped worker. Native PRINT/LPRINT replace
    /// rows, APPEND retains mode and rows; legacy APPEND hides the window.
    pub fn prepare(
        &self,
        call: &NativeCall,
        profile: &MessageProfile,
        metrics: &dyn TextMeasurement,
    ) -> Result<MessagePlan> {
        profile.validate()?;
        ensure!(call.identify == 0x66660006, "NNN message callback identify");
        let Payload::Message {
            kind,
            serial,
            cutin,
            text,
        } = &call.payload
        else {
            anyhow::bail!("NNN message payload")
        };
        ensure!(
            (0x22220001..=0x22220003).contains(kind) && call.code == *kind,
            "NNN message native code"
        );
        ensure!(profile.cutin_mode==0||profile.cutin.is_some(),"NNN Cutin mode without character table");
        ensure!(self.rows.len()<=64&&self.row_storage.len()<=64&&self.cutin_rows.len()<=64,
            "NNN native message RAM row bounds");
        ensure!(
            text.len() <= 1024 * 1024 && !text.contains(&0),
            "NNN native message byte limit"
        );
        let mut next = self.clone();
        next.row_storage.resize(64,Vec::new());
        next.cutin_rows.resize(64,[0;2]);
        for (stored,row) in next.row_storage.iter_mut().zip(&next.rows) {*stored=row.clone();}
        let append = *kind == 0x22220003;
        if append && matches!(next.mode, 0x22220004 | 0x22220005) {
            next.mode = 0x22220001;
        }
        if !append {
            next.mode = *kind;
            next.rows.clear();
            next.cutin_rows.fill([0;2]);
        }
        next.window_visible = self.next_window_override.unwrap_or(*kind == 0x22220001);
        next.next_window_override = None;
        next.serial = *serial;
        let mut previous = if append { self.body_cells } else { 0 };
        let mut removed = 0usize;
        let mut at = 0;
        let mut inserted=0usize;
        let mut first=true;
        while at < text.len() {
            let end = text[at..]
                .iter()
                .position(|b| *b == 13)
                .map_or(text.len(), |n| at + n);
            let bytes = &text[at..end];
            let suppress=first&&profile.cutin_mode>0&&*cutin!=0
                &&profile.cutin.as_ref().is_some_and(|policy|policy.suppress_name(*cutin,bytes));
            first=false;
            if !suppress {
                if next.rows.len()==64 {
                    next.scroll_storage(1);
                    next.rows.remove(0);
                    removed+=1;
                }
                let row=bytes[..bytes.len().min(254)].to_vec();
                next.row_storage[next.rows.len()]=row.clone();
                next.cutin_rows[next.rows.len()]=[0;2];
                next.rows.push(row);
                inserted+=1;
            }
            if end == text.len() {
                break;
            }
            ensure!(
                text.get(end + 1) == Some(&10),
                "NNN message requires native CRLF row separator"
            );
            at = end + 2;
        }
        let max = if next.mode == 0x22220001 {
            profile.window_rows
        } else {
            profile.full_rows
        };
        if profile.cutin_mode>0&&*cutin!=0 {
            // The source stamps before applying its visible row limit. Never
            // reproduce its out-of-array writes when a speaker was suppressed.
            let count=inserted.min(next.rows.len());
            for k in 0..count {let index=next.rows.len()-1-k;next.cutin_rows[index]=[*cutin,k as i32+1];}
        }
        if next.rows.len() > max {
            if next.mode == 0x22220001 {
                next.rows.truncate(max);
            } else {
                let drop = next.rows.len() - max;
                next.rows.drain(..drop);
                next.scroll_storage(drop);
                removed += drop;
            }
        }
        if append && removed > 0 {
            for row in self.rows.iter().take(removed) {
                previous = previous.saturating_sub(
                    plain_cells(row, profile.single_byte_map.as_deref())?
                        .chars()
                        .count(),
                );
            }
        }
        Self::layout_rows(next, previous, profile, metrics)
    }
    fn scroll_storage(&mut self,count:usize) {
        // MessageScroll copies every native slot, including hidden rows, then
        // clears the final slot. Rotating visible rows alone loses that state.
        self.row_storage.drain(..count);
        self.row_storage.resize(64,Vec::new());
        self.cutin_rows.copy_within(count..64,0);
        self.cutin_rows[64-count..].fill([0;2]);
    }
    /// Original SetMessageByLoad supplies fixed native rows directly. This
    /// neither receives a PRINT callback nor inserts/truncates rows again.
    pub(crate) fn prepare_saved(
        rows: Vec<Vec<u8>>,
        mode: u32,
        serial: u32,
        window_visible: bool,
        profile: &MessageProfile,
        metrics: &dyn TextMeasurement,
        cutin_rows: Vec<[i32; 2]>,
        mut row_storage: Vec<Vec<u8>>,
    ) -> Result<MessagePlan> {
        profile.validate()?;
        ensure!(
            matches!(mode, 0x22220001 | 0x22220002),
            "NNN saved message requires native PRINT/LPRINT wait state"
        );
        ensure!(
            rows.len() <= 64 && rows.iter().all(|row| row.len() <= 254 && !row.contains(&0)),
            "NNN saved fixed message rows"
        );
        ensure!(row_storage.len()<=64&&row_storage.iter().all(|r|r.len()<=254&&!r.contains(&0)),"NNN saved message RAM bounds");
        row_storage.resize(64,Vec::new());
        for (stored,row) in row_storage.iter_mut().zip(&rows) {*stored=row.clone();}
        let next = Self {
            mode,
            serial,
            rows,
            row_storage,
            cutin_rows,
            window_visible,
            body_cells: 0,
            next_window_override: None,
        };
        // Original InitByLoad resets per-row printed counts, independent of
        // the current live message or how many characters it had revealed.
        Self::layout_rows(next, 0, profile, metrics)
    }
    fn layout_rows(
        mut next: Self,
        previous: usize,
        profile: &MessageProfile,
        metrics: &dyn TextMeasurement,
    ) -> Result<MessagePlan> {
        next.row_storage.resize(64,Vec::new());
        for (stored,row) in next.row_storage.iter_mut().zip(&next.rows) {*stored=row.clone();}
        let has_name = profile.cutin_mode==0 && next.mode == 0x22220001
            && profile.name_enabled
            && !next.rows.is_empty()
            && (!profile.auto_name || profile.name_colors.contains_key(&next.rows[0]));
        let mut name_glyphs = Vec::new();
        let mut name_color = profile.name.color;
        if has_name {
            let raw = &next.rows[0];
            let (name, _, name_colors) = styled_cells(
                raw,
                profile.single_byte_map.as_deref(),
                Some(&profile.inline),
            )?;
            let mut origin = profile.name_origin;
            if profile.name_center != 0 {
                let spare = (profile.name_length - name.chars().count() as i32).max(0);
                let divisor = if profile.name_center == 1 { 2 } else { 1 };
                origin[0] +=
                    (spare * (i32::from(profile.name.size) + profile.name.spacing)) / divisor;
            }
            name_color = profile
                .name_colors
                .get(raw)
                .copied()
                .unwrap_or(profile.name.color);
            name_glyphs = positioned(&name, &profile.name, origin, metrics)?;
            for (glyph, color) in name_glyphs.iter_mut().zip(name_colors) {
                if let Some(color) = color {
                    glyph["color"] = json!(color);
                }
            }
            bind_inline_images(&mut name_glyphs, &profile.name, &profile.inline, metrics)?;
        }
        let mut body = Vec::new();
        let offset = usize::from(has_name);
        let (origin, step) = if next.mode == 0x22220001 {
            (profile.print_origin, profile.print_next)
        } else {
            (profile.lprint_origin, profile.lprint_next)
        };
        // Ruby glyphs share the body run and reveal clock: each appears on
        // the tick of the base cell under its x, never adding a tick.
        let mut prefixes = Vec::new();
        let mut cell_count = 0usize;
        let mut row_ranges=Vec::new();
        let mut cutin_windows=Vec::new();
        let mut previous_cutin_rows=0;
        for (row, raw) in next.rows.iter().skip(offset).enumerate() {
            let (cells, ruby, colors) = styled_cells(
                raw,
                profile.single_byte_map.as_deref(),
                Some(&profile.inline),
            )?;
            let mut row_origin = [
                origin[0] + row as i32 * step[0],
                origin[1] + row as i32 * step[1],
            ];
            let mut face=profile.body.clone();
            if profile.cutin_mode!=0 {
                let [selector,remaining]=next.cutin_rows.get(row+offset).copied().unwrap_or([0;2]);
                if selector>0 {
                    let policy=profile.cutin.as_ref().context("NNN Cutin layout table absent")?;
                    (face,row_origin)=policy.face_origin(selector,row+offset,&profile.body)?;
                    if remaining>previous_cutin_rows {
                        ensure!(remaining>0&&row+offset+remaining as usize<=64,"NNN Cutin group exceeds native message RAM");
                        let cells=next.row_storage[row+offset..row+offset+remaining as usize].iter()
                            .map(|raw|plain_cells(raw,profile.single_byte_map.as_deref()).map(|s|s.chars().count()))
                            .collect::<Result<Vec<_>>>()?.into_iter().max().unwrap_or(0);
                        if let Some(window)=policy.window(selector,remaining,cells,profile,row)? {cutin_windows.push(window);}
                    }
                }
                previous_cutin_rows=remaining;
            }
            let mut base =
                positioned_with_step(&cells, &face, row_origin, step[1], metrics)?;
            for glyph in &mut base {glyph["size"]=json!(face.size);}
            for (glyph, color) in base.iter_mut().zip(colors) {
                if let Some(color) = color {
                    glyph["color"] = json!(color);
                }
            }
            bind_inline_images(&mut base, &face, &profile.inline, metrics)?;
            let row_start=body.len();
            let count = base.len();
            let mut extra: Vec<Vec<Value>> = vec![Vec::new(); count];
            if let (Some(policy), false) = (&profile.ruby, ruby.is_empty()) {
                let size = i32::from(face.size) * policy.percent / 100;
                ensure!(size > 0, "NNN ruby font size");
                let cell = i32::from(face.size) + face.spacing;
                let ascent = metrics
                    .measure_simple(&profile.body.font, size as f32, "")?
                    .ascent;
                let body_ascent = metrics
                    .measure_simple(&face.font, face.size as f32, "")?
                    .ascent;
                let mut previous_end = std::collections::BTreeMap::<i32, i32>::new();
                for run in &ruby {
                    ensure!(run.start + run.len <= count, "NNN ruby base range");
                    let baseline = base[run.start]["baseline"][1].as_f64().unwrap() as f32;
                    let first = base
                        .iter()
                        .position(|g| g["baseline"][1].as_f64() == Some(f64::from(baseline)))
                        .unwrap();
                    let end = base
                        .iter()
                        .rposition(|g| g["baseline"][1].as_f64() == Some(f64::from(baseline)))
                        .unwrap()
                        + 1;
                    ensure!(
                        run.start + run.len <= end,
                        "NNN ruby base crosses an inline newline"
                    );
                    let start_x = base[first]["baseline"][0].as_f64().unwrap() as i32;
                    let local = RubyRun {
                        start: run.start - first,
                        len: run.len,
                        text: run.text.clone(),
                    };
                    let mut xs = ruby_positions(&[local], cell, end - first, size + policy.spacing)
                        .remove(0);
                    let limit = previous_end.entry(baseline as i32).or_default();
                    if xs.first().is_some_and(|x| *x < *limit) {
                        let shift = *limit - xs[0];
                        for x in &mut xs {
                            *x += shift;
                        }
                    }
                    if let Some(last) = xs.last() {
                        *limit = *last + size + policy.spacing;
                    }
                    for (c, x) in run.text.iter().zip(xs) {
                        let column = first
                            + usize::try_from(x.max(0) / cell.max(1))
                                .unwrap_or(0)
                                .min(end - first - 1);
                        let color = if policy.fixed_color {
                            json!(policy.color)
                        } else {
                            base[column]
                                .get("color")
                                .cloned()
                                .unwrap_or(json!(profile.body.color))
                        };
                        extra[column].push(json!({"character":c,"baseline":[start_x+x,(baseline-body_ascent)-size as f32+policy.delta_y as f32+ascent],"size":size as f32,"color":color}));
                    }
                }
            }
            for (glyph, ruby) in base.into_iter().zip(extra) {
                if let (Some(image), Some(color)) = (glyph.get("image"), profile.body_shadow) {
                    let mut shadow = glyph.clone();
                    shadow["image"]["rect"][0] = json!(image["rect"][0].as_f64().unwrap() + 1.);
                    shadow["image"]["rect"][1] = json!(image["rect"][1].as_f64().unwrap() + 1.);
                    shadow["image"]["tint"] = json!(color);
                    body.push(shadow);
                }
                body.push(glyph);
                body.extend(ruby);
                cell_count += 1;
                prefixes.push(body.len());
            }
            row_ranges.push([row_start,body.len()]);
        }
        next.body_cells = cell_count;
        let shown = previous.min(prefixes.len());
        let initial = if shown == 0 { 0 } else { prefixes[shown - 1] };
        let reveal = if body.len() == cell_count {
            json!({"interval_ms":0,"initial_visible":previous.min(body.len()),"rate_per_second":profile.reveal_per_second})
        } else {
            json!({"interval_ms":0,"initial_visible":initial,"rate_per_second":profile.reveal_per_second,"prefix_steps":prefixes[shown..]})
        };
        if let Some(color) = profile.name_shadow {
            let mut expanded = Vec::new();
            for glyph in name_glyphs {
                if let Some(image) = glyph.get("image") {
                    let mut shadow = glyph.clone();
                    shadow["image"]["rect"][0] = json!(image["rect"][0].as_f64().unwrap() + 1.);
                    shadow["image"]["rect"][1] = json!(image["rect"][1].as_f64().unwrap() + 1.);
                    shadow["image"]["tint"] = json!(color);
                    expanded.push(shadow);
                }
                expanded.push(glyph);
            }
            name_glyphs = expanded;
        }
        for (glyphs, color) in [
            (&mut body, profile.body_shadow),
            (&mut name_glyphs, profile.name_shadow),
        ] {
            if let Some(color) = color {
                for glyph in glyphs {
                    glyph["style"] = json!({"shadow":{"offset":[1.,1.],"color":color}});
                }
            }
        }
        let mut layers = vec![
            json!({"op":"put","layer":{"id":profile.body_layer,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order,1],"paint":{"kind":"glyph_run","font":profile.body.font,"size":profile.body.size,"color":profile.body.color,"glyphs":body,"reveal":reveal}}}),
            json!({"op":"put","layer":{"id":profile.name_layer,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order,2],"paint":{"kind":"glyph_run","font":profile.name.font,"size":profile.name.size,"color":name_color,"glyphs":name_glyphs}}}),
        ];
        if profile.cutin_mode!=0 {
            layers[0]["layer"]["visible"]=json!(false);
            for (row,range) in row_ranges.iter().enumerate() {
                layers.push(json!({"op":"put","layer":{"id":cutin::node(profile,row,false)?,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order,11+row*2],"paint":{"kind":"glyph_view","source":profile.body_layer,"range":range}}}));
            }
            layers.extend(cutin_windows);
        }
        // Retire rows/windows from the previous message in the same batch.
        if profile.cutin.is_some() {
            let mut removals=cutin::node_ids(profile)?.into_iter().map(|id|json!({"op":"remove","id":id})).collect::<Vec<_>>();
            removals.append(&mut layers);layers=removals;
        }
        for (w, visible) in profile
            .window
            .iter()
            .map(|w| (w, next.window_visible&&profile.cutin_mode==0))
            .chain(
                profile
                    .name_window
                    .iter()
                    .map(|w| (w, next.window_visible && has_name&&profile.cutin_mode==0)),
            )
        {
            let uv = [
                0.,
                0.,
                w.rect[2] as f32 / w.source_size[0] as f32,
                w.rect[3] as f32 / w.source_size[1] as f32,
            ];
            layers.insert(0,json!({"op":"put","layer":{"id":w.id,"parent":profile.root,"rect":w.rect,"uv":uv,"visible":visible&&(w.percent>0||w.stacked_planes!=0),"order":[profile.order,0],"paint":{"kind":"image","asset":w.asset}}}));
        }
        Ok(MessagePlan {
            next,
            batch: json!({"layers":layers}),
        })
    }
}
/// Pending native message owns its one worker and publication receipt. Failed
/// preparation/publication never acknowledges the original VM instruction.
pub struct NativeTextBridge {
    pub profile: MessageProfile,
    pub state: NativeMessageState,
    pub api_calls: u64,
    pool: TaskPool,
    worker: Option<Task<MessagePlan>>,
    ready: Option<MessagePlan>,
    pending: Option<NativeCall>,
    published: bool,
    blocked_click_at: Option<u64>,
    layers_present: bool,
    windows_present: bool,
    background: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    window_job: Option<crysirror_core::image_processing::ImageJob>,
    window_prepared: bool,
    window_path: Option<String>,
    window_candidate_path: Option<String>,
    restored_wait_armed: bool,
    window_on: bool,
    hidden_layers: BTreeMap<u32,bool>,
    window_visibility_changed: bool,
    window_toggle_enabled: bool,
    window_off_mouse: u64,
    timing: timing::Timing,
    disable_auto_requested: bool,
    native_skip_allowed: bool,
    wheel_down_click: bool,
    window_variant: i32,
    presented_reveal_complete: Option<bool>,
    background_refresh: Option<background::Refresh>,
}
/// Immutable native layout/resource seed. It never owns the live message or
/// asks a restore preparation host to mutate the Scene.
#[derive(Clone)]
pub(crate) struct TextRestoreSeed {
    profile: MessageProfile,
    retained_serial: u32,
    pool: TaskPool,
    previous_window_path: Option<String>,
    window_on: bool,
    window_off_mouse: u64,
    timing: timing::Timing,
    wheel_down_click: bool,
    window_variant: i32,
    cutin_rows: Vec<[i32; 2]>,
    row_storage: Vec<Vec<u8>>,
}
pub(crate) struct SavedTextPreparation {
    bridge: NativeTextBridge,
}
pub(crate) struct PreparedSavedText {
    pub bridge: NativeTextBridge,
    pub batch: Value,
    pub release_images: Vec<String>,
}
impl TextRestoreSeed {
    pub(crate) fn bind_message_storage(&mut self,rows:Vec<Vec<u8>>)->Result<()> {
        ensure!(rows.len()==64&&rows.iter().all(|r|r.len()<=254&&!r.contains(&0)),"NNN restored message RAM bounds");
        self.row_storage=rows;Ok(())
    }
    pub(crate) fn bind_cutin(&mut self,mode:i32,rows:&[[i32;2];64])->Result<()> {
        ensure!(mode==0||self.profile.cutin.is_some(),"NNN saved Cutin character policy absent");
        self.profile.cutin_mode=mode;
        self.cutin_rows=rows.to_vec();
        Ok(())
    }
    pub(crate) fn retained_serial(&self) -> u32 {
        self.retained_serial
    }
    pub(crate) fn window_variant(&self) -> i32 {
        self.window_variant
    }
    pub(crate) fn bind_player_names(&mut self, names: [Vec<u8>; 2]) {
        self.profile.inline.names = names;
    }
    pub(crate) fn bind_windows(&mut self, number: i32, windows: Vec<MessageWindow>) -> Result<()> {
        let mut profile = self.profile.clone();
        profile.window = windows.first().cloned();
        profile.name_window = windows.get(1).cloned();
        profile.validate()?;
        self.profile = profile;
        self.window_variant = number;
        Ok(())
    }
    pub(crate) fn dormant(
        &self,
        rows: Vec<Vec<u8>>,
        mode: u32,
        serial: u32,
        window_visible: bool,
        cannot_click: bool,
    ) -> Result<NativeTextBridge> {
        ensure!(
            matches!(mode, 0x22220001..=0x22220005)
                && rows.len() <= 64
                && rows.iter().all(|row| row.len() <= 254 && !row.contains(&0)),
            "NNN saved latent message state"
        );
        let mut profile = self.profile.clone();
        profile.cannot_click = cannot_click;
        let mut bridge = NativeTextBridge::from_pool(profile, self.pool.clone())?;
        bridge.timing = self.timing.clone();
        bridge.wheel_down_click = self.wheel_down_click;
        bridge.window_variant = self.window_variant;
        bridge.timing.start(0, false, false, 7000)?;
        bridge.window_on = self.window_on;
        bridge.window_off_mouse = self.window_off_mouse;
        bridge.state = NativeMessageState {
            mode,
            serial,
            rows,
            row_storage: self.row_storage.clone(),
            cutin_rows: self.cutin_rows.clone(),
            window_visible,
            body_cells: 0,
            next_window_override: None,
        };
        Ok(bridge)
    }
    pub(crate) fn previous_window_path(&self) -> Option<&str> {
        self.previous_window_path.as_deref()
    }


    pub(crate) fn begin(
        &self,
        rows: Vec<Vec<u8>>,
        mode: u32,
        serial: u32,
        window_visible: bool,
        cannot_click: bool,
        background: Arc<dyn crysirror_core::image_processing::ImageResource>,
        host: &mut dyn HostServices,
    ) -> Result<SavedTextPreparation> {
        let metrics = host.text_measurement(self.profile.client)?;
        let mut profile = self.profile.clone();
        profile.cannot_click = cannot_click;
        let permit = self.pool.reserve()?;
        let worker_profile = profile.clone();
        let cutin_rows=self.cutin_rows.clone();
        let row_storage=self.row_storage.clone();
        let worker = permit.spawn(move |cancel| {
            cancel.check()?;
            let plan = NativeMessageState::prepare_saved(
                rows,
                mode,
                serial,
                window_visible,
                &worker_profile,
                metrics.as_ref(),
                cutin_rows,
                row_storage,
            )?;
            cancel.check()?;
            Ok(plan)
        })?;
        let mut bridge = NativeTextBridge::from_pool(profile, self.pool.clone())?;
        bridge.timing = self.timing.clone();
        bridge.wheel_down_click = self.wheel_down_click;
        bridge.window_variant = self.window_variant;
        bridge.timing.start(0, false, false, 7000)?;
        bridge.window_on = self.window_on;
        bridge.window_off_mouse = self.window_off_mouse;
        bridge.set_background(background)?;
        bridge.window_path = self.previous_window_path.clone();
        bridge.worker = Some(worker);
        Ok(SavedTextPreparation { bridge })
    }
}
impl SavedTextPreparation {
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(worker) = &mut self.bridge.worker {
            let Some(plan) = worker.try_take()? else {
                return Ok(false);
            };
            self.bridge.worker = None;
            // A restore cannot publish an unresolved native mask/window as
            // ordinary alpha. All visible windows need actual shared resources.
            for window in self
                .bridge
                .profile
                .window
                .iter()
                .chain(self.bridge.profile.name_window.iter())
            {
                let visible = plan.batch["layers"].as_array().unwrap().iter().any(|node| {
                    node["layer"]["id"].as_u64() == Some(u64::from(window.id))
                        && node["layer"]["visible"].as_bool() == Some(true)
                });
                ensure!(
                    !visible || window.resource.is_some(),
                    "NNN saved visible native window resource is unresolved"
                );
            }
            self.bridge.ready = Some(plan);
        }
        ensure!(
            self.bridge.ready.is_some(),
            "NNN saved text result already consumed"
        );
        self.bridge.prepare_windows(host)
    }
    pub(crate) fn finish(mut self) -> Result<PreparedSavedText> {
        ensure!(
            self.bridge.worker.is_none()
                && self.bridge.window_job.is_none()
                && self.bridge.window_prepared,
            "NNN saved text is not ready"
        );
        let mut plan = self
            .bridge
            .ready
            .take()
            .context("NNN saved text plan absent")?;
        let mut release_images = Vec::new();
        if let Some(path) = self.bridge.window_candidate_path.take() {
            if let Some(previous) = self.bridge.window_path.replace(path.clone()) {
                if previous != path {
                    release_images.push(previous);
                }
            }
        } else if let Some(previous) = self.bridge.window_path.take() {
            release_images.push(previous);
        }
        self.bridge.hidden_layers=self.bridge.project_window_visibility(&mut plan.batch)?;
        self.bridge.state = plan.next;
        self.bridge.published = true;
        self.bridge.layers_present = true;
        self.bridge.windows_present =
            self.bridge.profile.window.is_some() || self.bridge.profile.name_window.is_some();
        // The first post-commit poll arms the new wait and blocks the load click.
        self.bridge.restored_wait_armed = false;
        self.bridge.blocked_click_at = None;
        Ok(PreparedSavedText {
            bridge: self.bridge,
            batch: plan.batch,
            release_images,
        })
    }
}
impl NativeTextBridge {
    pub(crate) fn resume_after_menu(&mut self, now: u64) {
        self.timing.arm(now);
        self.blocked_click_at = Some(now);
    }
    pub(crate) fn change_speed(&mut self, index: usize, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            !self.dependency_pending(),
            "NNN message speed change during glyph preparation"
        );
        let rate = self.timing.normal_speed(index)?;
        if !self.timing.script() {
            self.profile.reveal_per_second = rate;
            if self.layers_present {
                let mut layer: LayerSpec = serde_json::from_slice(&host.query_engine(
                    self.profile.client,
                    EngineQuery::Layer(self.profile.body_layer),
                )?)?;
                if let crysirror_scene::layer::Paint::GlyphRun {
                    glyphs,
                    reveal: Some(reveal),
                    ..
                } = &mut layer.paint
                {
                    reveal.change_rate(glyphs.len(), rate)?;
                    host.submit_engine(
                        self.profile.client,
                        1,
                        &serde_json::to_vec(&json!({"layers":[{"op":"put","layer":layer}]}))?,
                    )?;
                }
            }
        }
        Ok(())
    }
    pub(crate) fn change_window_percent(&mut self, percent: i32) -> Result<()> {
        ensure!(
            (0..=100).contains(&percent) && !self.dependency_pending(),
            "NNN window transparency change during image preparation"
        );
        self.profile.cutin_window_percent=percent as u16;
        for window in self
            .profile
            .window
            .iter_mut()
            .chain(self.profile.name_window.iter_mut())
        {
            window.percent = percent as u16;
        }
        Ok(())
    }
    pub fn new(profile: MessageProfile) -> Result<Self> {
        Self::from_pool(profile, TaskPool::new("nnn-native-text", 1, 1)?)
    }
    fn from_pool(profile: MessageProfile, pool: TaskPool) -> Result<Self> {
        profile.validate()?;
        let timing = timing::Timing::new(profile.reveal_per_second);
        Ok(Self {
            profile,
            state: NativeMessageState::default(),
            api_calls: 0,
            pool,
            worker: None,
            ready: None,
            pending: None,
            published: false,
            blocked_click_at: None,
            layers_present: false,
            windows_present: false,
            background: None,
            window_job: None,
            window_prepared: false,
            window_path: None,
            window_candidate_path: None,
            restored_wait_armed: false,
            window_on: true,
            hidden_layers:BTreeMap::new(),
            window_visibility_changed:false,
            window_toggle_enabled:true,
            window_off_mouse:crysirror_core::service::buttons::MOUSE_RIGHT,
            timing,
            disable_auto_requested: false,
            native_skip_allowed: false,
            wheel_down_click: false,
            window_variant: 0,
            presented_reveal_complete: None,
            background_refresh: None,
        })
    }
    pub(crate) fn restored_layer_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        if self.layers_present {
            ids.extend([self.profile.body_layer, self.profile.name_layer]);
            if self.profile.cutin.is_some() {ids.extend(cutin::node_ids(&self.profile).expect("validated Cutin layer identities"));}
        }
        if self.windows_present {
            ids.extend(
                self.profile
                    .window
                    .iter()
                    .chain(self.profile.name_window.iter())
                    .map(|window| window.id),
            );
        }
        ids
    }
    pub(crate) fn restore_seed(&self) -> Result<TextRestoreSeed> {
        ensure!(
            self.worker.is_none()
                && self.ready.is_none()
                && self.window_job.is_none()
                && self.background_refresh.is_none(),
            "NNN text restore requires drained preparation dependencies"
        );
        Ok(TextRestoreSeed {
            profile: self.profile.clone(),
            retained_serial: self.state.serial,
            pool: self.pool.clone(),
            previous_window_path: self.window_path.clone(),
            window_on: self.window_on,
            window_off_mouse:self.window_off_mouse,
            timing: self.timing.clone(),
            wheel_down_click: self.wheel_down_click,
            window_variant: self.window_variant,
            cutin_rows: self.state.cutin_rows.clone(),
            row_storage: self.state.row_storage.clone(),
        })
    }
    pub(crate) fn bind_windows(&mut self, number: i32, windows: Vec<MessageWindow>) -> Result<()> {
        ensure!(
            self.pending.is_none()
                && self.worker.is_none()
                && self.ready.is_none()
                && self.window_job.is_none()
                && self.background_refresh.is_none(),
            "NNN window variant changed during native message preparation"
        );
        let mut profile = self.profile.clone();
        profile.window = windows.first().cloned();
        profile.name_window = windows.get(1).cloned();
        profile.validate()?;
        self.profile = profile;
        self.window_variant = number;
        self.window_prepared = false;
        Ok(())
    }
    pub(crate) fn configure_native_timing(&mut self, setup: &Names) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "NNN timing configuration during native message"
        );
        self.wheel_down_click = setup.integer(b"wheelDownIsClick")?.unwrap_or(0) != 0;
        self.window_off_mouse=match setup.integer(b"windowOffMouseButton")?.unwrap_or(1) {
            -1=>0,0=>crysirror_core::service::buttons::MOUSE_LEFT,
            1=>crysirror_core::service::buttons::MOUSE_RIGHT,
            2=>crysirror_core::service::buttons::MOUSE_MIDDLE,
            _=>anyhow::bail!("NNN window-off mouse button profile"),
        };
        self.timing.configure(setup)
    }
    pub(crate) fn start_native_controls(
        &mut self,
        script_auto: i32,
        system_auto: bool,
        cannot_click: bool,
        has_voice: bool,
        voice_length_ms: u32,
    ) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "NNN native message controls changed during publication"
        );
        let rate = self
            .timing
            .start(script_auto, system_auto, has_voice, voice_length_ms)?;
        self.profile.reveal_per_second = rate;
        self.profile.cannot_click = cannot_click;
        self.disable_auto_requested = false;
        self.presented_reveal_complete = None;
        Ok(())
    }
    pub(crate) fn system_auto(&mut self, enabled: bool) {
        self.timing.set_automatic(enabled)
    }
    pub(crate) fn take_disable_auto_request(&mut self) -> bool {
        std::mem::take(&mut self.disable_auto_requested)
    }
    pub(crate) fn shared_reveal_complete(&self, host: &mut dyn HostServices) -> Result<bool> {
        let bytes = host.query_engine(
            self.profile.client,
            EngineQuery::Layer(self.profile.body_layer),
        )?;
        let layer: LayerSpec =
            serde_json::from_slice(&bytes).context("NNN retained message layer absent")?;
        Ok(layer
            .paint
            .glyph_reveal_status()
            .context("NNN message shared reveal absent")?
            .complete)
    }
    pub(crate) fn set_presented_reveal_complete(&mut self, complete: Option<bool>) {
        self.presented_reveal_complete = complete;
    }
    pub(crate) fn accept_first_presentation(&mut self, now: u64) {
        self.blocked_click_at = Some(now);
        self.timing.arm(now);
    }
    pub(crate) fn set_native_skip(&mut self, allowed: bool) {
        self.native_skip_allowed = allowed;
    }
    pub fn pending_message(&self) -> bool {
        self.pending.is_some()
    }
    pub fn dependency_pending(&self) -> bool {
        self.worker.is_some() || self.window_job.is_some() || self.background_refresh.is_some()
    }
    pub fn set_next_window_visibility(&mut self, visible: bool) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "NNN cannot override a pending native message"
        );
        self.state.next_window_override = Some(visible);
        Ok(())
    }
    /// Stage with the parent's effect operations so one original ClearAll
    /// callback can commit one atomic shared engine batch.
    pub fn set_background(
        &mut self,
        image: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<()> {
        ensure!(
            image.client() == self.profile.client
                && image.size() == self.profile.canvas
                && image.opaque(),
            "NNN message background scope/opaque XRGB dimensions"
        );
        if let Some(previous) = &self.background {
            if Arc::ptr_eq(previous, &image) {
                return Ok(());
            }
            ensure!(
                self.pending.is_none(),
                "NNN changing message background needs native frame/window dependency recomposition"
            );
        }
        self.background = Some(image);
        Ok(())
    }
    fn prepare_windows(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        use crysirror_core::image_processing::ImageProcessState;
        if self.window_prepared {
            return Ok(true);
        }
        let plan = self
            .ready
            .as_ref()
            .context("NNN window composition without prepared message")?;
        let mut windows = Vec::new();
        for window in self
            .profile
            .window
            .iter()
            .chain(self.profile.name_window.iter())
        {
            let node = plan.batch["layers"]
                .as_array()
                .context("NNN message layer batch")?
                .iter()
                .find(|node| node["layer"]["id"].as_u64() == Some(u64::from(window.id)))
                .context("NNN message window layer missing")?;
            if node["layer"]["visible"].as_bool() == Some(true)
                && (window.percent > 0 || window.stacked_planes != 0)
            {
                windows.push(window.clone());
            }
        }
        let has_resources = windows.iter().any(|window| window.resource.is_some());
        if !has_resources {
            self.window_prepared = true;
            return Ok(true);
        }
        ensure!(
            windows.iter().all(|window| window.resource.is_some()),
            "NNN mixed resolved/unresolved message window resources"
        );
        if self.window_job.is_none() {
            let background = self
                .background
                .clone()
                .context("NNN native window requires completed engine background frame")?;
            self.window_job = Some(host.image_metadata(self.profile.client)?.begin_process(
                background::composite_input(background, &windows)?,
                b"[]".as_slice().into(),
            )?);
            self.api_calls += 1;
        }
        match self.window_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN shared native window composition: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.profile.client && image.size() == self.profile.canvas,
                    "NNN window composition resource scope"
                );
                let published =
                    host.publish_image_resource(self.profile.client, windows[0].id, image)?;
                self.api_calls += 1;
                let path = published.path.relative().to_owned();
                let plan = self.ready.as_mut().unwrap();
                for node in plan.batch["layers"].as_array_mut().unwrap() {
                    let Some(window) = windows
                        .iter()
                        .find(|window| node["layer"]["id"].as_u64() == Some(u64::from(window.id)))
                    else {
                        continue;
                    };
                    let [x, y, w, h] = window.rect;
                    let [cw, ch] = self.profile.canvas;
                    node["layer"]["uv"] = json!([
                        x as f32 / cw as f32,
                        y as f32 / ch as f32,
                        (x + w) as f32 / cw as f32,
                        (y + h) as f32 / ch as f32
                    ]);
                    node["layer"]["paint"] = json!({"kind":"image","asset":path});
                }
                self.window_candidate_path = Some(path);
                self.window_prepared = true;
                self.window_job = None;
                Ok(true)
            }
        }
    }
    pub fn clear_operations(&self) -> Result<Vec<Value>> {
        ensure!(
            self.pending.is_none() && self.worker.is_none() && self.ready.is_none(),
            "NNN cannot clear a pending native message"
        );
        Ok(if self.layers_present {
            let mut nodes=vec![
                json!({"op":"remove","id":self.profile.body_layer}),
                json!({"op":"remove","id":self.profile.name_layer}),
            ];
            if self.profile.cutin.is_some() {nodes.extend(cutin::node_ids(&self.profile)?.into_iter().map(|id|json!({"op":"remove","id":id})));}
            nodes
        } else {
            vec![]
        })
    }
    /// Native NewGame hides presentation without erasing original message rows.
    /// Window resources may outlive a prior glyph-only CLR, so track them apart.
    pub(crate) fn hide_operations(&self) -> Result<Vec<Value>> {
        let mut operations = self.clear_operations()?;
        if self.windows_present {
            for window in self
                .profile
                .window
                .iter()
                .chain(self.profile.name_window.iter())
            {
                operations.push(json!({"op":"remove","id":window.id}));
            }
        }
        Ok(operations)
    }
    pub(crate) fn accept_hide(&mut self) -> Result<()> {
        let _ = self.hide_operations()?;
        self.layers_present = false;
        self.windows_present = false;
        self.state.window_visible = false;
        Ok(())
    }
    /// Call only after the staged shared clear batch successfully publishes.
    pub fn accept_clear(&mut self) -> Result<()> {
        let _ = self.clear_operations()?;
        self.state.rows.clear();
        self.state.body_cells = 0;
        self.layers_present = false;
        Ok(())
    }
    /// Original message Clear discards rows and glyphs, preserving mode and
    /// window policy. This standalone form owns its one shared batch.
    pub fn clear(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let layers = self.clear_operations()?;
        if !layers.is_empty() {
            host.submit_engine(
                self.profile.client,
                1,
                &serde_json::to_vec(&json!({"layers":layers}))?,
            )?;
            self.api_calls += 1;
        }
        self.accept_clear()
    }
    /// Publish original message GlyphRuns/window without consuming input or
    /// advancing its receipt. The native screen-effect15 pass follows this
    /// phase, and only its complete frame can deliver the pending input edge.
    pub(crate) fn prepare_publication(
        &mut self,
        machine: &Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        if self.published {
            ensure!(
                self.pending.as_ref() == Some(call),
                "NNN published message receipt changed"
            );
            return Ok(true);
        }
        if self.pending.is_none() {
            ensure!(
                matches!(call.payload, Payload::Message { .. }),
                "NNN text bridge requires message"
            );
            let metrics = host.text_measurement(self.profile.client)?;
            let permit = self.pool.reserve()?;
            let profile = self.profile.clone();
            let state = self.state.clone();
            let record = call.clone();
            self.worker = Some(permit.spawn(move |cancel| {
                cancel.check()?;
                let plan = state.prepare(&record, &profile, metrics.as_ref())?;
                cancel.check()?;
                Ok(plan)
            })?);
            self.pending = Some(call.clone());
            return Ok(false);
        }
        ensure!(
            self.pending.as_ref() == Some(call),
            "NNN stale text pending receipt"
        );
        if let Some(worker) = &mut self.worker {
            let Some(plan) = worker.try_take()? else {
                return Ok(false);
            };
            self.ready = Some(plan);
            self.worker = None;
        }
        if self.ready.is_some() && !self.prepare_windows(host)? {
            return Ok(false);
        }
        if let Some(plan) = &self.ready {
            // Retain a completed plan until the whole engine batch commits.
            // SetMessageMode unconditionally calls WindowOn for every new
            // PRINT/LPRINT/APPEND, independently of the saved window flag.
            // A restored wait instead retains the live WindowOn state above.
            host.submit_engine(self.profile.client, 1, &serde_json::to_vec(&plan.batch)?)?;
            self.api_calls += 1;
            self.window_visibility_changed |= !self.window_on;
            self.window_on = true;
            self.hidden_layers.clear();
            self.state = plan.next.clone();
            if let Some(path) = self.window_candidate_path.take() {
                if let Some(previous) = self.window_path.replace(path) {
                    host.release_image_resource(self.profile.client, &previous)?;
                }
            }
            self.ready = None;
            self.published = true;
            self.layers_present = true;
            self.windows_present =
                self.profile.window.is_some() || self.profile.name_window.is_some();
            self.blocked_click_at = Some(host.now_ms());
            self.timing.arm(host.now_ms());
        }
        Ok(self.published)
    }
    /// Native F4/F5 PRINT callbacks still update encoded rows, APPEND and
    /// window policy (531f/5320 dispatch), but never enter the message mode or
    /// publish a new screen. Layout is the same shared worker as ordinary
    /// PRINT; its Layer DTO is deliberately not submitted.
    pub(crate) fn try_commit_skipped(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        ensure!(
            !self.published,
            "NNN script skip cannot replace a live message wait"
        );
        if self.pending.is_none() {
            ensure!(
                matches!(call.payload, Payload::Message { .. }),
                "NNN skipped message payload"
            );
            let metrics = host.text_measurement(self.profile.client)?;
            let permit = self.pool.reserve()?;
            let profile = self.profile.clone();
            let state = self.state.clone();
            let record = call.clone();
            self.worker = Some(permit.spawn(move |cancel| {
                cancel.check()?;
                let plan = state.prepare(&record, &profile, metrics.as_ref())?;
                cancel.check()?;
                Ok(plan)
            })?);
            self.pending = Some(call.clone());
            return Ok(false);
        }
        ensure!(
            self.pending.as_ref() == Some(call),
            "NNN skipped message receipt changed"
        );
        if let Some(worker) = &mut self.worker {
            let Some(plan) = worker.try_take()? else {
                return Ok(false);
            };
            self.ready = Some(plan);
            self.worker = None;
        }
        let next = &self
            .ready
            .as_ref()
            .context("NNN skipped message plan absent")?
            .next;
        machine.acknowledge(call)?;
        self.state = next.clone();
        self.ready = None;
        self.pending = None;
        self.window_prepared = false;
        self.presented_reveal_complete = None;
        Ok(true)
    }
    /// WM_F4/F5's SetYoyaku ends an already published native wait. It neither
    /// reveals hidden glyphs nor clears its rows: the next real Print owns
    /// those presentation operations.
    pub(crate) fn finish_native_action(
        &mut self,
        machine: &mut Machine,
        call: Option<&NativeCall>,
        now: u64,
    ) -> Result<()> {
        ensure!(
            self.published && !self.dependency_pending() && self.ready.is_none(),
            "NNN native key action requires published message boundary"
        );
        if let Some(call) = call {
            ensure!(
                self.pending.as_ref() == Some(call),
                "NNN native key action message receipt changed"
            );
            machine.acknowledge(call)?;
        } else {
            ensure!(
                self.pending.is_none(),
                "NNN restored key action has live receipt"
            );
        }
        self.pending = None;
        self.published = false;
        self.window_prepared = false;
        self.blocked_click_at = Some(now);
        self.restored_wait_armed = false;
        Ok(())
    }
    pub fn try_commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        if !self.prepare_publication(machine, call, host)? {
            return Ok(false);
        }
        if !self.poll_message_input(host, input)? {
            return Ok(false);
        }
        machine.acknowledge(call)?;
        self.pending = None;
        self.published = false;
        self.window_prepared = false;
        self.blocked_click_at = Some(host.now_ms());
        Ok(true)
    }
    fn poll_message_input(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        ensure!(self.published, "NNN message publication absent");
        let bytes = host.query_engine(
            self.profile.client,
            EngineQuery::Layer(self.profile.body_layer),
        )?;
        let layer: LayerSpec =
            serde_json::from_slice(&bytes).context("NNN owned message layer disappeared")?;
        let complete = layer
            .paint
            .glyph_reveal_status()
            .context("NNN message layer has no shared reveal")?
            .complete
            && self.presented_reveal_complete.unwrap_or(true);
        if self.poll_window_controls(host,input,complete)? {return Ok(false);}
        if self.native_skip_allowed {
            if !complete {
                host.submit_engine(
                    self.profile.client,
                    1,
                    &serde_json::to_vec(
                        &json!({"layers":[{"op":"complete_reveal","id":self.profile.body_layer}]}),
                    )?,
                )?;
                self.api_calls += 1;
            }
            return Ok(true);
        }
        if self.timing.poll(host.now_ms(), complete)? {
            return Ok(true);
        }
        let click = !self.profile.cannot_click
            && !self.timing.script()
            && input.buttons_pressed & self.profile.advance_buttons != 0
            && self.blocked_click_at != Some(host.now_ms());
        if !click {
            return Ok(false);
        }
        self.disable_auto_requested = true;
        if !complete {
            host.submit_engine(
                self.profile.client,
                1,
                &serde_json::to_vec(
                    &json!({"layers":[{"op":"complete_reveal","id":self.profile.body_layer}]}),
                )?,
            )?;
            self.api_calls += 1;
            self.blocked_click_at = Some(host.now_ms());
            return Ok(false);
        }
        Ok(true)
    }
    /// Resume the already-restored native wait at its saved nextPC. There is
    /// no synthetic NativeCall and no VM acknowledgement to replay.
    pub(crate) fn restored_input_armed(&self) -> bool {
        self.restored_wait_armed
    }
    pub(crate) fn poll_restored_input(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        ensure!(
            self.pending.is_none()
                && self.worker.is_none()
                && self.ready.is_none()
                && self.window_job.is_none(),
            "NNN restored text dependencies are not complete"
        );
        if !self.restored_wait_armed {
            self.restored_wait_armed = true;
            self.blocked_click_at = Some(host.now_ms());
            self.timing.arm(host.now_ms());
            return Ok(false);
        }
        if !self.poll_message_input(host, input)? {
            return Ok(false);
        }
        self.published = false;
        self.window_prepared = false;
        self.blocked_click_at = Some(host.now_ms());
        self.restored_wait_armed = false;
        Ok(true)
    }
}

#[cfg(test)]
mod native_hide_tests {
    use super::*;
    #[test]
    fn newgame_hides_glyphs_and_windows_but_preserves_original_rows_and_append_cells() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"native hide","content_revision":"1","assets":"assets","canvas":[16,16],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        let profile = MessageProfile::from_legacy_setup(
            &Names::parse(b"fontSize 4 nameFontSize 4 printX 0 printY 0 nextY 4 LPrintX 0 LPrintY 0 LNextY 4 fullMessageNumber 3 windowMessageNumber 3 nameX 0 nameY 0 nameCenterFlag 0", false).unwrap(),
            MessageBindings {
                client: ClientId(1),
                root: 1,
                body_layer: 200,
                name_layer: 201,
                order: 30,
                canvas: [16, 16],
                window: Some(MessageWindow {
                    id: 202,
                    asset: "authored.png".into(),
                    rect: [0, 0, 2, 1],
                    source_size: [2, 1],
                    percent: 100,
                    stacked_planes: 0,
                    resource: None,
                }),
            },
            "unused-no-font-decoding",
            2,
            Some(&Names { tokens: vec![] }),
            None,
            crysirror_core::service::buttons::MOUSE_LEFT,
        )
        .unwrap();
        assert_eq!(profile.full_rows, 3);
        assert_eq!(profile.body.size, 4);
        let mut text = NativeTextBridge::new(profile).unwrap();
        text.state.rows = vec![vec![0x82, 0xa0]];
        text.state.body_cells = 1;
        text.state.window_visible = true;
        text.layers_present = true;
        text.windows_present = true;
        let put = |id| json!({"op":"put","layer":{"id":id,"rect":[0,0,2,1],"paint":{"kind":"solid","color":[1,2,3,255]}}});
        rt.host
            .submit_engine(
                ClientId(1),
                1,
                &serde_json::to_vec(&json!({"layers":[put(200),put(201),put(202)]})).unwrap(),
            )
            .unwrap();
        let rows = text.state.rows.clone();
        let ops = text.hide_operations().unwrap();
        assert_eq!(ops.len(), 3);
        rt.host
            .submit_engine(
                ClientId(1),
                1,
                &serde_json::to_vec(&json!({"layers":ops})).unwrap(),
            )
            .unwrap();
        text.accept_hide().unwrap();
        assert_eq!(text.state.rows, rows);
        assert_eq!(text.state.body_cells, 1);
        assert!(!text.state.window_visible);
        for id in [200, 201, 202] {
            assert!(rt.host.engine.layers.get(ClientId(1), id).is_none());
        }
        assert!(text.hide_operations().unwrap().is_empty());
        // A prior glyph-only CLR does not imply its retained window was removed.
        rt.host
            .submit_engine(
                ClientId(1),
                1,
                &serde_json::to_vec(&json!({"layers":[put(202)]})).unwrap(),
            )
            .unwrap();
        text.windows_present = true;
        let ops = text.hide_operations().unwrap();
        assert_eq!(ops.len(), 1);
        rt.host
            .submit_engine(
                ClientId(1),
                1,
                &serde_json::to_vec(&json!({"layers":ops})).unwrap(),
            )
            .unwrap();
        text.accept_hide().unwrap();
        assert!(rt.host.engine.layers.get(ClientId(1), 202).is_none());
        assert_eq!(text.state.rows, rows);
        assert_eq!(rt.host.now_ms(), 0);
    }
}

#[cfg(test)]
mod window_plane_tests {
    use super::*;
    use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState};
    use std::time::{Duration, Instant};
    fn finish(mut job: ImageJob) -> Arc<dyn crysirror_core::image_processing::ImageResource> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match job.poll().unwrap() {
                ImageProcessState::Ready(image) => return image,
                ImageProcessState::Failed(error) => panic!("{error}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        }
    }
    #[test]
    fn original_stacked_window_order_and_full_percent_blt_use_real_shared_image_pixels() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"window plane oracle","content_revision":"1","assets":"game","canvas":[1,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let images = rt.host.image_metadata(ClientId(1)).unwrap();
        let background = finish(
            images
                .begin_process(
                    ImageInput::Solid {
                        size: [1, 1],
                        color: [20, 40, 60, 255],
                    },
                    b"[]".as_slice().into(),
                )
                .unwrap(),
        );
        let bytes = crysirror_image::encode_png(&crysirror_image::ImageSurface {
            width: 1,
            height: 3,
            rgba: vec![100, 120, 140, 128, 200, 100, 50, 128, 10, 220, 30, 128],
        })
        .unwrap();
        let source = finish(
            images
                .begin_process(
                    ImageInput::Encoded {
                        name: "window.png".into(),
                        bytes: bytes.into(),
                        size: [1, 3],
                    },
                    b"[]".as_slice().into(),
                )
                .unwrap(),
        );
        let raw = rt
            .host
            .publish_image_resource(ClientId(1), 202, source.clone())
            .unwrap();
        for (flags, percent, expected) in [
            (0, 100, [60, 80, 100, 255]),
            (1, 100, [130, 90, 75, 255]),
            (2, 100, [105, 95, 97, 255]),
            (3, 100, [128, 112, 71, 255]),
            (3, 0, [107, 115, 47, 255]),
        ] {
            let names=crate::config::Names::parse(b"fontSize 4 nameFontSize 4 printX 0 printY 0 nextY 4 LPrintX 0 LPrintY 0 LNextY 4 fullMessageNumber 3 windowMessageNumber 3 nameFlag 0 nameColorChangeFlag 0",false).unwrap();
            let profile = MessageProfile::from_legacy_setup(
                &names,
                MessageBindings {
                    client: ClientId(1),
                    root: 1,
                    body_layer: 200,
                    name_layer: 201,
                    order: 30,
                    canvas: [1, 1],
                    window: Some(MessageWindow {
                        id: 202,
                        asset: raw.path.relative().into(),
                        rect: [0, 0, 1, 1],
                        source_size: [1, 3],
                        percent,
                        stacked_planes: flags,
                        resource: Some(source.clone()),
                    }),
                },
                "fixture",
                2,
                None,
                None,
                crysirror_core::service::buttons::MOUSE_LEFT,
            )
            .unwrap();
            let text = NativeTextBridge::new(profile).unwrap();
            let mut prep = text
                .restore_seed()
                .unwrap()
                .begin(
                    vec![],
                    0x22220001,
                    7,
                    true,
                    false,
                    background.clone(),
                    &mut rt.host,
                )
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while !prep.poll(&mut rt.host).unwrap() {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            let done = prep.finish().unwrap();
            let window = done.batch["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["layer"]["id"] == 202)
                .unwrap();
            assert_eq!(window["layer"]["visible"], true);
            let path = window["layer"]["paint"]["asset"].as_str().unwrap();
            assert_eq!(
                rt.assets.surface(ClientId(1), path).unwrap().rgba.as_ref(),
                expected,
                "flags={flags} percent={percent}"
            );
        }
    }
    #[test]
    fn ruby_markup_is_consumed_and_placed_like_native_make_pic() {
        // MUGEN2 prologue row: "　バビロン　までは　#［５How many miles to Babylon?］なんマイル？"
        let mut raw = b"\x81\x40\x83\x6f\x83\x72\x83\x8d\x83\x93\x81\x40\x82\xdc\x82\xc5\x82\xcd\x81\x40#\x81\x6d\x82\x54".to_vec();
        for c in "Ｈｏｗ　ｍａｎｙ　ｍｉｌｅｓ　ｔｏ　Ｂａｂｙｌｏｎ？".chars()
        {
            raw.extend(encoding_rs::SHIFT_JIS.encode(&c.to_string()).0.iter());
        }
        raw.extend(b"\x81\x6e\x82\xc8\x82\xf1\x83\x7d\x83\x43\x83\x8b\x81\x48");
        let (cells, ruby) = cells_with_ruby(&raw, None).unwrap();
        assert_eq!(cells, "　バビロン　までは　なんマイル？");
        assert_eq!(ruby.len(), 1);
        assert_eq!((ruby[0].start, ruby[0].len, ruby[0].text.len()), (5, 5, 26));
        // size26, space0, percent43 -> ruby11, sukima0: x=130+65-143=52,
        // last=52+275=327 <= 26*16-11, step=275/25=11.
        let xs = &ruby_positions(&ruby, 26, 16, 11)[0];
        assert_eq!((xs[0], xs[1], xs[25]), (52, 63, 327));
        // ASCII digit length and malformed (unclosed) ruby are consumed.
        let (cells, ruby) = cells_with_ruby(b"\x82\xa0#\x81\x6d1\x82\xa0", None).unwrap();
        assert_eq!((cells.as_str(), ruby.len()), ("あ", 0));
        assert!(cells_with_ruby(b"#\x90\x46", None).is_err());
    }
    #[test]
    fn ruby_is_pushed_inside_the_row_and_after_the_previous_run() {
        let run = |start, len, n| RubyRun {
            start,
            len,
            text: vec!['a'; n],
        };
        // Two runs overlapping: the second starts at the first's leftLimit.
        let xs = ruby_positions(&[run(0, 1, 4), run(1, 1, 4)], 20, 4, 10);
        assert_eq!(xs[0], vec![0, 10, 20, 30]);
        assert_eq!(xs[1][0], 40);
        // A run past the right edge is pushed back to rightMax.
        let xs = ruby_positions(&[run(3, 1, 4)], 20, 4, 10);
        assert_eq!(*xs[0].last().unwrap(), 70);
    }
}
#[cfg(test)]
mod inline_command_tests {
    use super::*;
    fn raw(s: &str) -> Vec<u8> {
        encoding_rs::SHIFT_JIS.encode(s).0.into_owned()
    }
    #[test]
    fn native_inline_names_colors_newlines_and_unknown_consumption_are_distinct() {
        let mut profile = InlineProfile::default();
        profile.names = [raw("姓"), raw("名")];
        profile.colors.insert(0x90d4, [255, 0, 0, 255]);
        let (text, ruby, colors) =
            styled_cells(&raw("#姓#名#赤本文#改次#謎終"), None, Some(&profile)).unwrap();
        assert_eq!(text, "姓名本文\n次終");
        assert!(ruby.is_empty());
        assert_eq!(
            colors,
            vec![
                None,
                None,
                Some([255, 0, 0, 255]),
                Some([255, 0, 0, 255]),
                Some([255, 0, 0, 255]),
                Some([255, 0, 0, 255])
            ]
        );
    }
}
#[cfg(test)]
mod inline_image_plan_tests {
    use super::*;
    struct Metrics;
    impl TextMeasurement for Metrics {
        fn measure_simple(
            &self,
            _: &str,
            size: f32,
            _: &str,
        ) -> crysirror_core::Result<crysirror_core::service::TextExtent> {
            Ok(crysirror_core::service::TextExtent {
                advance: 0.,
                line_height: size,
                ascent: size * 0.75,
                descent: -size * 0.25,
            })
        }
    }
    #[test]
    fn image_face_and_shadow_share_one_native_reveal_step_and_parse_as_shared_layers() {
        let mut profile = MessageProfile::from_legacy_setup(
            &Names::parse(b"nameFlag 0 nameColorChangeFlag 0", false).unwrap(),
            MessageBindings {
                client: ClientId(1),
                root: 1,
                body_layer: 200,
                name_layer: 201,
                order: 30,
                canvas: [800, 600],
                window: None,
            },
            "fixture",
            2,
            None,
            None,
            1,
        )
        .unwrap();
        profile.inline.gaiji.insert(0x9053, 5);
        profile
            .inline
            .images
            .insert((5, 24), "gaiji/cell.png".into());
        let bytes = encoding_rs::SHIFT_JIS.encode("甲#心乙").0.into_owned();
        let state = NativeMessageState {
            mode: 0x22220001,
            rows: vec![bytes],
            ..Default::default()
        };
        let plan = NativeMessageState::layout_rows(state, 0, &profile, &Metrics).unwrap();
        assert_eq!(plan.next.body_cells, 3);
        let layer: crysirror_scene::layer::LayerSpec =
            serde_json::from_value(plan.batch["layers"][0]["layer"].clone()).unwrap();
        let crysirror_scene::layer::Paint::GlyphRun { glyphs, reveal, .. } = layer.paint else {
            panic!()
        };
        assert_eq!(glyphs.len(), 4);
        assert_eq!(reveal.unwrap().prefix_steps, Some(vec![1, 3, 4]));
        let shadow = glyphs[1].image.as_ref().unwrap();
        let face = glyphs[2].image.as_ref().unwrap();
        assert_eq!(face.asset, "gaiji/cell.png");
        assert_eq!(shadow.rect[0], face.rect[0] + 1.);
        assert_eq!(shadow.tint, [0, 0, 0, 255]);
    }
}
