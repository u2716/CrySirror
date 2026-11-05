// SPDX-License-Identifier: MPL-2.0
// Semantics adopted from siglus_rs crates/reallive/src/select.rs and the window
// geometry of textout.rs (MPL-2.0). Pixels are drawn by engine layers and glyph runs.
//! Native selections: `select_w` in a separate window, `select` in the active
//! window and `select_s` as `#SELBTN` buttons, including all option effects.
use super::*;
use crate::vm::{
    bytecode::{Select, select_effect},
    graphics::Image,
    settings::WindowAttr,
};
use crysirror_image::g00::Region;

const FIRST_WIDGET: u32 = 600_000;
const TEXT_LAYER: u32 = 605_000;
const SHADOW_LAYER: u32 = 605_001;
const BAR_LAYER: u32 = 605_002;
const FIRST_BUTTON_LAYER: u32 = 605_100;
/// Nine-slice pieces and solid backing of a text window, 16 ids per window.
pub(super) const NINE_SLICE_BASE: u32 = 630_000;
const MAX_CHOICES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Choice {
    /// Position in the source: the value stored on selection.
    index: usize,
    text: String,
    enabled: bool,
    /// Occupies a slot without text.
    blank: bool,
    colour: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ButtonStyle {
    name: Option<String>,
    back: Option<String>,
    moji_size: i32,
    default_colour: i32,
    select_colour: i32,
    /// (pattern, dx, dy) for normal, highlighted, pushed and disabled.
    frames: [(i32, i32, i32); 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Layout {
    Window { window: u32, restore: Option<u32> },
    Buttons(ButtonStyle),
}

#[derive(Debug)]
pub(super) struct Selection {
    layout: Layout,
    choices: Vec<Choice>,
    /// Absolute canvas rectangles.
    rects: Vec<[i32; 4]>,
    /// Window text origin, for glyph baselines.
    origin: [i32; 2],
    highlighted: Option<usize>,
    pressed: bool,
    drawn: Option<(Option<usize>, bool)>,
    layer_ids: Vec<u32>,
}

fn half_width(c: char) -> bool {
    c.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&c)
}

fn text_width(text: &str, cell: i32) -> i32 {
    text.chars().map(|c| if half_width(c) { cell / 2 } else { cell }).sum()
}

/// Pieces of a nine-slice frame as (source, destination) rectangles, upstream
/// `draw_type4`: twelve patterns, or a single picture cut into a 3 × 3 grid
/// with corners a third of its shorter side. Returns the pieces and the inner
/// backing rectangle.
fn nine_slice(image: &Image, window: [i32; 4], area: [i32; 4]) -> (Vec<([i32; 4], [i32; 4])>, [i32; 4]) {
    let sliced = image.regions.len() < 12;
    let (w, h) = (image.width, image.height);
    let corner = (w.min(h) / 3).max(1);
    let piece = |i: i32| -> [i32; 4] {
        if !sliced {
            let r: Region = image.region(i);
            return [r.x1, r.y1, r.x2 - r.x1 + 1, r.y2 - r.y1 + 1];
        }
        let (col, row) = (i % 3, match i / 3 {
            0 => 0,
            3 => 2,
            _ => 1,
        });
        let xs = [0, corner, w - corner, w];
        let ys = [0, corner, h - corner, h];
        [xs[col as usize], ys[row], xs[col as usize + 1] - xs[col as usize], ys[row + 1] - ys[row]]
    };
    let (tl, tc, tr) = (piece(0), piece(1), piece(2));
    let (ls, rs) = if sliced { (piece(3), piece(5)) } else { (piece(3), piece(6)) };
    let (bl, bc, br) = (piece(9), piece(10), piece(11));
    let [x, y, ww, wh] = window;
    let inner = [
        x + ls[2] - area[2],
        y + tc[3] - area[0],
        ww - ls[2] - rs[2] + area[2] + area[3],
        wh - tc[3] - bc[3] + area[0] + area[1],
    ];
    let pieces = vec![
        (tl, [x, y, tl[2], tl[3]]),
        (tr, [x + ww - tr[2], y, tr[2], tr[3]]),
        (bl, [x, y + wh - bl[3], bl[2], bl[3]]),
        (br, [x + ww - br[2], y + wh - br[3], br[2], br[3]]),
        (tc, [x + tl[2], y, ww - tl[2] - tr[2], tc[3]]),
        (bc, [x + bl[2], y + wh - bc[3], ww - bl[2] - br[2], bc[3]]),
        (ls, [x, y + tl[3], ls[2], wh - tl[3] - bl[3]]),
        (rs, [x + ww - rs[2], y + tr[3], rs[2], wh - tr[3] - br[3]]),
    ];
    (pieces, inner)
}

/// `draw_backing`: blended colour, or for filter 0 darken by the strength and
/// add the colour.
fn backing_layers(first_id: u32, rect: [i32; 4], attr: WindowAttr, order: i32) -> Vec<Value> {
    let colour = [attr.r, attr.g, attr.b, 255].map(|c| c.clamp(0, 255));
    let strength = attr.alpha.clamp(0, 255);
    let solid = |id: u32, colour: [i32; 4], blend: &str| {
        json!({"id":id,"parent":SCENE_ROOT,"rect":rect,"order":[998,order],
            "paint":{"kind":"solid","color":colour},"blend":blend,"opacity":strength})
    };
    if attr.filter == 0 {
        vec![solid(first_id, [255, 255, 255, 255], "subtract"), solid(first_id + 1, colour, "add")]
    } else {
        vec![solid(first_id, colour, "normal")]
    }
}

pub(super) fn window_attr(m: &Machine, prefix: &str) -> WindowAttr {
    if m.gameexe.int(&format!("{prefix}ATTR_MOD")).unwrap_or(0) != 0 {
        WindowAttr::from_slice(&m.gameexe.ints(&format!("{prefix}ATTR"))).unwrap_or_default()
    } else {
        m.sys.settings.window_attr
    }
}

/// A window's frame and text rectangle, upstream `textout::geometry`.
pub(super) struct FrameGeometry {
    pub(super) window: [i32; 4],
    pub(super) text: [i32; 2],
    pub(super) nine_slice: bool,
    main: Option<Rc<Image>>,
    area: [i32; 4],
}

impl RealLiveVm {
    pub(super) fn frame_geometry(&self, m: &Machine, window: u32, content: Option<[i32; 2]>) -> Result<FrameGeometry> {
        let exe = &m.gameexe;
        let prefix = format!("WINDOW.{window:03}.");
        let ints = |part: &str| exe.ints(&format!("{prefix}{part}"));
        let size = exe.int(&format!("{prefix}MOJI_SIZE")).unwrap_or(24).clamp(1, 512);
        let rep = ints("MOJI_REP");
        let count = ints("MOJI_CNT");
        let pad = ints("MOJI_POS");
        let pos = ints("POS");
        let minimum = ints("MOJI_MIN");
        let luby = exe.int(&format!("{prefix}LUBY_SIZE")).unwrap_or(0);
        let [top, bottom, left, right] = [0, 1, 2, 3].map(|i| pad.get(i).copied().unwrap_or(0));
        let cell = (size + rep.first().copied().unwrap_or(0)).max(1);
        let line = (size + rep.get(1).copied().unwrap_or(0) + luby).max(1);
        let (mut tw, mut th) = (
            count.first().copied().unwrap_or(25).max(1) * cell + size,
            count.get(1).copied().unwrap_or(3).max(1) * line,
        );
        let set = exe.int(&format!("{prefix}WAKU_SETNO")).unwrap_or(0);
        let pattern = if exe.int(&format!("{prefix}WAKU_MOD")).unwrap_or(0) != 0 {
            exe.int(&format!("{prefix}WAKU_NO")).unwrap_or(0)
        } else {
            m.sys.settings.waku_all
        };
        let key = |part: &str| format!("WAKU.{set:03}.{pattern:03}.{part}");
        let kind = exe.int(&key("TYPE")).or_else(|| exe.int(&format!("WAKU.{set:03}.TYPE")));
        let nine_slice = set >= 0 && matches!(kind, Some(3 | 4));
        let fit = set >= 0 && kind == Some(3);
        let (screen_w, screen_h) = (self.width as i32, self.height as i32);
        let loader = m.sys.gfx.loader.clone().context("RealLive resources")?;
        let image = |part: &str| -> Result<Option<Rc<Image>>> {
            if set < 0 {
                return Ok(None);
            }
            match exe.str(&key(part)).filter(|n| !n.is_empty()) {
                Some(n) => Ok(Some(loader.image(n)?)),
                None => Ok(None),
            }
        };
        let main = image("NAME")?;
        let back = image("BACK")?;
        if let (true, Some([cw, ch])) = (fit, content) {
            // Between MOJI_MIN and MOJI_CNT; long choices widen up to the screen.
            let min_cols = minimum.first().copied().unwrap_or(0);
            let min_rows = minimum.get(1).copied().unwrap_or(0);
            tw = cw.max(min_cols * cell).min((screen_w - left - right).max(cell));
            th = ((ch + line - 1) / line * line).clamp((min_rows * line).min(th), th);
        }
        let right = if fit { right.max(left) + (-rep.first().copied().unwrap_or(0)).max(0) } else { right };
        let (w, h) = match (&main, &back) {
            _ if nine_slice => (tw + left + right, th + top + bottom),
            (Some(i), _) | (None, Some(i)) => (i.width, i.height),
            _ => (tw + left + right, th + top + bottom),
        };
        let origin = pos.first().copied().unwrap_or(0);
        let px = pos.get(1).copied().unwrap_or(0);
        let py = pos.get(2).copied().unwrap_or(0);
        let x = if matches!(origin, 1 | 3) { screen_w - w - px } else { px };
        let y = if matches!(origin, 2 | 3) { screen_h - h - py } else { py };
        let (x, y) = (x.min(screen_w - w).max(0), y.min(screen_h - h).max(0));
        let mut area = [0; 4];
        for (slot, value) in area.iter_mut().zip(exe.ints(&key("AREA"))) {
            *slot = value;
        }
        Ok(FrameGeometry { window: [x, y, w, h], text: [x + left, y + top], nine_slice, main, area })
    }

    /// Layers of a type 3/4 frame, registered with the window's frame ids.
    pub(super) fn nine_slice_layers(&mut self, m: &Machine, window: u32, g: &FrameGeometry) -> Vec<Value> {
        let Some(image) = g.main.clone() else {
            return vec![];
        };
        let base = NINE_SLICE_BASE + window * 16;
        let (pieces, inner) = nine_slice(&image, g.window, g.area);
        let attr = window_attr(m, &format!("WINDOW.{window:03}."));
        let mut layers = backing_layers(base, inner, attr, 0);
        for (at, (source, dest)) in pieces.into_iter().enumerate() {
            if dest[2] <= 0 || dest[3] <= 0 {
                continue;
            }
            let (iw, ih) = (image.width.max(1) as f32, image.height.max(1) as f32);
            layers.push(json!({"id":base + 2 + at as u32,"parent":SCENE_ROOT,"rect":dest,"order":[998,2],
                "paint":{"kind":"image","asset":image.asset},
                "uv":[source[0] as f32 / iw, source[1] as f32 / ih, (source[0] + source[2]) as f32 / iw, (source[1] + source[3]) as f32 / ih]}));
        }
        self.waku_ids.extend(layers.iter().filter_map(|l| l["id"].as_u64()).map(|id| id as u32));
        self.waku_ids.sort_unstable();
        self.waku_ids.dedup();
        layers.into_iter().map(|layer| json!({"op":"put","layer":layer})).collect()
    }

    fn evaluate_choices(&mut self, m: &mut Machine, select: &Select) -> Result<(Vec<Choice>, Option<usize>)> {
        let mut choices = Vec::new();
        let mut cursor = None;
        for (index, option) in select.options.iter().enumerate() {
            let mut choice = Choice { index, text: m.eval_str(&option.text)?, enabled: true, blank: false, colour: None };
            let mut hidden = false;
            for condition in &option.conditions {
                let applies = match &condition.condition {
                    Some(expression) => m.eval_int(expression)? != 0,
                    None => true,
                };
                if !applies {
                    continue;
                }
                let argument = condition.argument.as_ref().map(|e| m.eval_int(e)).transpose()?;
                match condition.effect {
                    select_effect::COLOUR => choice.colour = Some(argument.unwrap_or(0)),
                    select_effect::TITLE => {
                        choice.enabled = false;
                        if let Some(colour) = argument {
                            choice.colour = Some(colour);
                        }
                    }
                    select_effect::HIDE => hidden = true,
                    select_effect::BLANK => {
                        choice.blank = true;
                        choice.enabled = false;
                    }
                    select_effect::CURSOR => cursor = Some(choices.len()),
                    other => bail!("unknown native choice condition {:?}", other as char),
                }
            }
            if !hidden {
                choices.push(choice);
            }
            ensure!(choices.len() <= MAX_CHOICES, "native selection option limit");
        }
        ensure!(!choices.is_empty(), "empty native selection");
        Ok((choices, cursor))
    }

    pub(super) fn selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, c: &Command) -> Result<Next> {
        let CommandKind::Select(select) = &c.kind else {
            bail!("selection without native choice data")
        };
        if m.should_set_savepoint(crate::vm::machine::SavepointKind::Selcom) {
            m.mark_savepoint();
        }
        let (choices, cursor) = self.evaluate_choices(m, select)?;
        let mut selection = match c.op.opcode {
            2 | 3 => self.button_selection(m, select, choices)?,
            0 => {
                let computed = select.window.as_ref().map(|e| m.eval_int(e)).transpose()?.unwrap_or(-1);
                let default = m.gameexe.int("DEFAULT_SEL_WINDOW").unwrap_or(-1);
                let chosen = if computed >= 0 { computed } else { default };
                self.window_selection(m, h, choices, u32::try_from(chosen).ok())?
            }
            _ => self.window_selection(m, h, choices, None)?,
        };
        selection.highlighted = cursor;
        self.draw_selection(m, h, &mut selection)?;
        let ops: Vec<Value> = selection.rects.iter().zip(&selection.choices).enumerate().map(|(i, (rect, choice))| {
            json!({"op":"put","widget":{"id":FIRST_WIDGET + i as u32,"kind":"button","rect":rect,
                "interaction_only":true,"keyboard_focusable":false,"enabled":choice.enabled}})
        }).collect();
        self.submit(h, json!({"ui":ops}))?;
        self.wait = Some(Wait::Choice(Box::new(selection)));
        Ok(Next::Advance)
    }

    fn window_selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, choices: Vec<Choice>, separate: Option<u32>) -> Result<Selection> {
        let active = self.window;
        let mut restore = None;
        let mut window = active;
        if let Some(chosen) = separate {
            window = chosen;
            restore = Some(active);
            // Only the selection window stays open while it is up.
            let open: Vec<u32> = self.pen.keys().copied().chain(
                self.waku_ids.iter().filter(|id| (20000..NINE_SLICE_BASE).contains(*id)).map(|id| (id - 20000) / 8),
            ).collect();
            for index in open {
                if index != window {
                    self.close_window(h, index)?;
                }
            }
            self.clear_window(h, window)?;
            self.window = window;
        }
        let prefix = format!("WINDOW.{window:03}.");
        let exe = &m.gameexe;
        let size = exe.int(&format!("{prefix}MOJI_SIZE")).unwrap_or(24).clamp(1, 512);
        let rep = exe.ints(&format!("{prefix}MOJI_REP"));
        let (rep_x, rep_y) = (rep.first().copied().unwrap_or(0), rep.get(1).copied().unwrap_or(0));
        let luby = exe.int(&format!("{prefix}LUBY_SIZE")).unwrap_or(0);
        let rows = exe.ints(&format!("{prefix}MOJI_CNT")).get(1).copied().unwrap_or(3).max(1);
        let cell = size + rep_x;
        let line = (size + rep_y + luby).max(1);
        // Options go below any text already shown; clear if they do not fit.
        let pen = self.pen.get(&window).copied().unwrap_or([0, 0]);
        let mut row = if pen == [0, 0] { 0 } else { pen[1] / line + i32::from(pen[0] > 0) };
        if row + choices.len() as i32 > rows {
            self.clear_window(h, window)?;
            row = 0;
        }
        let indent = if exe.int(&format!("{prefix}SELCOM_USE")).unwrap_or(0) != 0 {
            exe.int(&format!("{prefix}SELCOM_MOJIPOS")).unwrap_or(0).max(0)
        } else {
            0
        };
        let local: Vec<[i32; 4]> = choices.iter().enumerate().map(|(i, choice)| {
            let y = luby + (row + i as i32) * line;
            [indent, y - rep_y / 2, text_width(&choice.text, cell).max(cell), line]
        }).collect();
        let content = local.iter().fold([pen[0], pen[1]], |size, r| [size[0].max(r[0] + r[2]), size[1].max(r[1] + r[3])]);
        let g = self.frame_geometry(m, window, Some(content))?;
        let rects = local.iter().map(|r| [g.text[0] + r[0], g.text[1] + r[1], r[2], r[3]]).collect();
        let frame = if g.nine_slice {
            self.nine_slice_layers(m, window, &g)
        } else {
            self.waku_layers(m, &prefix, [g.window[0], g.window[1]])?
        };
        if !frame.is_empty() {
            self.submit(h, json!({ "layers": frame }))?;
        }
        self.toolbar.open.insert(window);
        Ok(Selection {
            layout: Layout::Window { window, restore },
            choices,
            rects,
            origin: g.text,
            highlighted: None,
            pressed: false,
            drawn: None,
            layer_ids: vec![],
        })
    }

    fn button_selection(&mut self, m: &mut Machine, select: &Select, choices: Vec<Choice>) -> Result<Selection> {
        let requested = select.window.as_ref().map(|e| m.eval_int(e)).transpose()?.unwrap_or(0);
        let exe = &m.gameexe;
        let set = if exe.get(&format!("SELBTN.{requested:03}.NAME")).is_some()
            || exe.get(&format!("SELBTN.{requested:03}.BASEPOS")).is_some()
        {
            requested
        } else {
            0
        };
        let key = |part: &str| format!("SELBTN.{set:03}.{part}");
        let pair = |part: &str| {
            let v = exe.ints(&key(part));
            (v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0))
        };
        let frame = |part: &str| {
            let v = exe.ints(&key(part));
            (v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0), v.get(2).copied().unwrap_or(0))
        };
        let named = |part: &str| exe.str(&key(part)).filter(|s| !s.is_empty()).map(str::to_owned);
        let default_colour = exe.int(&key("MOJIDEFAULTCOL")).unwrap_or(0);
        let mut select_colour = exe.int(&key("MOJISELECTCOL")).unwrap_or(1);
        if select_colour == default_colour {
            // Highlighting in the default colour would be invisible.
            select_colour = 1;
        }
        if select_colour < 0 {
            select_colour = default_colour;
        }
        let style = ButtonStyle {
            name: named("NAME"),
            back: named("BACK"),
            moji_size: exe.int(&key("MOJISIZE")).unwrap_or(24).max(1),
            default_colour,
            select_colour,
            frames: [frame("NORMAL"), frame("SELECT"), frame("PUSH"), frame("DONTSEL")],
        };
        let (base, rep, centre) = (pair("BASEPOS"), pair("REPPOS"), pair("CENTERING"));
        let loader = m.sys.gfx.loader.clone().context("RealLive resources")?;
        let size = match (&style.back, &style.name) {
            (Some(back), _) => {
                let image = loader.image(back)?;
                (image.width, image.height)
            }
            (None, Some(name)) => {
                let region = loader.image(name)?.region(style.frames[0].0);
                (region.x2 - region.x1 + 1, region.y2 - region.y1 + 1)
            }
            (None, None) => (style.moji_size * 12, style.moji_size * 3 / 2),
        };
        let count = choices.len() as i32;
        let x = if centre.0 != 0 { (self.width as i32 - ((count - 1) * rep.0 + size.0)) / 2 } else { base.0 };
        let y = if centre.1 != 0 { (self.height as i32 - ((count - 1) * rep.1 + size.1)) / 2 } else { base.1 };
        let rects = (0..count).map(|i| [x + rep.0 * i, y + rep.1 * i, size.0, size.1]).collect();
        Ok(Selection {
            layout: Layout::Buttons(style),
            choices,
            rects,
            origin: [0, 0],
            highlighted: None,
            pressed: false,
            drawn: None,
            layer_ids: vec![],
        })
    }

    fn draw_selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, s: &mut Selection) -> Result<()> {
        let state = (s.highlighted, s.pressed);
        if s.drawn == Some(state) {
            return Ok(());
        }
        s.drawn = Some(state);
        let mut layers: Vec<Value> = vec![];
        let mut glyphs: Vec<Value> = vec![];
        let mut shadows: Vec<Value> = vec![];
        let (run_size, shadow_on) = match &s.layout {
            Layout::Window { window, .. } => {
                let prefix = format!("WINDOW.{window:03}.");
                let exe = &m.gameexe;
                let size = exe.int(&format!("{prefix}MOJI_SIZE")).unwrap_or(24).clamp(1, 512);
                let rep = exe.ints(&format!("{prefix}MOJI_REP"));
                let (rep_x, rep_y) = (rep.first().copied().unwrap_or(0), rep.get(1).copied().unwrap_or(0));
                let dark = exe.int(&format!("{prefix}SELCOM_MOJIDARK")).unwrap_or(0) != 0;
                let ascent = h.font_metrics(&self.config.font_alias, size as f32, '字').map_err(err)?.ascent.round() as i32;
                for (i, (choice, r)) in s.choices.iter().zip(&s.rects).enumerate() {
                    if choice.blank {
                        continue;
                    }
                    let highlighted = s.highlighted == Some(i);
                    if highlighted {
                        layers.push(json!({"id":BAR_LAYER,"parent":SCENE_ROOT,"rect":[r[0] - 2, r[1], r[2] + 4, r[3]],"order":[1000,0],
                            "paint":{"kind":"solid","color":[255,255,255,255]},"opacity":64}));
                    }
                    let mut colour = commands_colour(&m.gameexe, choice.colour.unwrap_or(0));
                    if !choice.enabled || (dark && !highlighted && s.highlighted.is_some()) {
                        for c in &mut colour[..3] {
                            *c /= 2;
                        }
                    }
                    let mut x = r[0];
                    let baseline = r[1] + rep_y / 2 + ascent;
                    for ch in choice.text.chars() {
                        glyphs.push(json!({"character":ch.to_string(),"baseline":[x, baseline],"size":size,"color":colour,"bold":false}));
                        shadows.push(json!({"character":ch.to_string(),"baseline":[x, baseline],"size":size,"color":[0,0,0,255],"bold":false}));
                        x += if half_width(ch) { (size + rep_x) / 2 } else { size + rep_x };
                    }
                }
                (size, exe.int(&format!("{prefix}MOJI_SHADOW")).unwrap_or(1) != 0)
            }
            Layout::Buttons(style) => {
                let loader = m.sys.gfx.loader.clone().context("RealLive resources")?;
                let name = style.name.as_deref().map(|n| loader.image(n)).transpose()?;
                let back = style.back.as_deref().map(|n| loader.image(n)).transpose()?;
                let attr = m.sys.settings.window_attr;
                let shadow = commands_colour(&m.gameexe, 255);
                let ascent = h.font_metrics(&self.config.font_alias, style.moji_size as f32, '字').map_err(err)?.ascent.round() as i32;
                for (i, (choice, r)) in s.choices.iter().zip(&s.rects).enumerate() {
                    if choice.blank {
                        continue;
                    }
                    let highlighted = s.highlighted == Some(i);
                    let state = if !choice.enabled {
                        3
                    } else if highlighted && s.pressed {
                        2
                    } else if highlighted {
                        1
                    } else {
                        0
                    };
                    let id = FIRST_BUTTON_LAYER + i as u32 * 4;
                    if let Some(back) = &back {
                        let colour = [attr.r, attr.g, attr.b].map(|c| c.clamp(0, 255) as f32 / 255.);
                        layers.push(json!({"id":id,"parent":SCENE_ROOT,"rect":r,"order":[1000,0],"opacity":attr.alpha.clamp(0,255),
                            "paint":{"kind":"image","asset":back.asset},"filter":{"color_rate":1.0,"color":colour}}));
                    } else if name.is_none() {
                        layers.extend(backing_layers(id, *r, attr, 0));
                    }
                    if let Some(name) = &name {
                        let (pattern, dx, dy) = style.frames[state];
                        let region = name.region(pattern);
                        let (iw, ih) = (name.width.max(1) as f32, name.height.max(1) as f32);
                        let (rw, rh) = (region.x2 - region.x1 + 1, region.y2 - region.y1 + 1);
                        layers.push(json!({"id":id + 2,"parent":SCENE_ROOT,"rect":[r[0] + dx, r[1] + dy, rw, rh],"order":[1000,0],
                            "paint":{"kind":"image","asset":name.asset},
                            "uv":[region.x1 as f32 / iw, region.y1 as f32 / ih, (region.x2 + 1) as f32 / iw, (region.y2 + 1) as f32 / ih]}));
                    }
                    let colour_index = match choice.colour {
                        Some(colour) if !highlighted || !choice.enabled => colour,
                        _ if highlighted => style.select_colour,
                        _ => style.default_colour,
                    };
                    let colour = commands_colour(&m.gameexe, colour_index);
                    let size = style.moji_size;
                    let mut x = r[0] + (r[2] - text_width(&choice.text, size)) / 2;
                    let baseline = r[1] + (r[3] - size) / 2 + ascent;
                    for ch in choice.text.chars() {
                        glyphs.push(json!({"character":ch.to_string(),"baseline":[x, baseline],"size":size,"color":colour,"bold":false}));
                        shadows.push(json!({"character":ch.to_string(),"baseline":[x, baseline],"size":size,"color":shadow,"bold":false}));
                        x += if half_width(ch) { size / 2 } else { size };
                    }
                }
                (style.moji_size, true)
            }
        };
        let run = |id: u32, offset: i32, order: i32, glyphs: &[Value], width: u32, height: u32, font: &str| {
            json!({"id":id,"parent":SCENE_ROOT,"rect":[offset, offset, width, height],"order":[1000,order],
                "paint":{"kind":"glyph_run","font":font,"size":run_size,"color":[255,255,255,255],"glyphs":glyphs,
                    "reveal":{"unit":"unicode_scalar","interval_ms":0,"initial_visible":glyphs.len()}}})
        };
        if !glyphs.is_empty() {
            let font = self.config.font_alias.clone();
            if shadow_on {
                layers.push(run(SHADOW_LAYER, 1, 1, &shadows, self.width, self.height, &font));
            }
            layers.push(run(TEXT_LAYER, 0, 2, &glyphs, self.width, self.height, &font));
        }
        let live: Vec<u32> = layers.iter().filter_map(|l| l["id"].as_u64()).map(|id| id as u32).collect();
        let mut ops: Vec<Value> = s.layer_ids.iter().filter(|id| !live.contains(id)).map(|id| json!({"op":"remove","id":id})).collect();
        ops.extend(layers.into_iter().map(|layer| json!({"op":"put","layer":layer})));
        s.layer_ids = live;
        self.submit(h, json!({ "layers": ops }))
    }

    /// Returns true when an option was chosen.
    pub(super) fn poll_selection(&mut self, m: &mut Machine, h: &mut dyn HostServices, input: InputFrame, s: &mut Selection) -> Result<bool> {
        let count = s.choices.len();
        let pointer = [input.pointer_x, input.pointer_y];
        let hit = s.rects.iter().position(|r| pointer[0] >= r[0] && pointer[1] >= r[1] && pointer[0] < r[0] + r[2] && pointer[1] < r[1] + r[3]);
        let hovered = hit.filter(|i| s.choices[*i].enabled);
        if hovered.is_some() && hovered != s.highlighted {
            s.highlighted = hovered;
            self.play_numbered_se(m, h, 0)?;
        }
        let move_by = |from: Option<usize>, delta: isize, choices: &[Choice]| {
            let mut at = from.map_or(if delta > 0 { -1 } else { count as isize }, |i| i as isize);
            for _ in 0..count {
                at = (at + delta).rem_euclid(count as isize);
                if choices[at as usize].enabled {
                    return Some(at as usize);
                }
            }
            from
        };
        let edge = |bit: u64| input.buttons_pressed & bit != 0;
        let mut chosen = None;
        if !self.input_used {
            if edge(buttons::UP) || edge(buttons::LEFT) {
                s.highlighted = move_by(s.highlighted, -1, &s.choices);
            }
            if edge(buttons::DOWN) || edge(buttons::RIGHT) || edge(buttons::TAB) {
                s.highlighted = move_by(s.highlighted, 1, &s.choices);
            }
            if edge(buttons::ENTER) || edge(buttons::SPACE) {
                chosen = s.highlighted.filter(|i| s.choices[*i].enabled);
            }
            let digits = [buttons::KEY_1, buttons::KEY_2, buttons::KEY_3, buttons::KEY_4, buttons::KEY_5,
                buttons::KEY_6, buttons::KEY_7, buttons::KEY_8, buttons::KEY_9];
            for (n, bit) in digits.into_iter().enumerate() {
                if edge(bit) {
                    chosen = (0..count).filter(|i| s.choices[*i].enabled).nth(n).or(chosen);
                }
            }
        }
        s.pressed = input.buttons_down & buttons::MOUSE_LEFT != 0;
        let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_value(self.query(h, EngineQuery::UiEvents)?)?;
        for event in &events {
            if matches!(event.kind.as_str(), "activate" | "click") {
                if let Some(at) = event.widget.id.checked_sub(FIRST_WIDGET).map(|n| n as usize).filter(|n| *n < count) {
                    if s.choices[at].enabled {
                        chosen = Some(at);
                    }
                }
            }
        }
        let Some(at) = chosen else {
            self.draw_selection(m, h, s)?;
            return Ok(false);
        };
        self.play_numbered_se(m, h, 1)?;
        m.store = s.choices[at].index as i32;
        self.input_used = true;
        let widgets: Vec<Value> = (0..count).map(|i| json!({"op":"remove","id":FIRST_WIDGET + i as u32})).collect();
        let layers: Vec<Value> = s.layer_ids.iter().map(|id| json!({"op":"remove","id":id})).collect();
        self.submit(h, json!({"ui":widgets,"layers":layers}))?;
        if let Layout::Window { window, restore } = &s.layout {
            match restore {
                Some(restore) => {
                    self.close_window(h, *window)?;
                    self.window = *restore;
                }
                None => self.clear_window(h, *window)?,
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(regions: usize) -> Image {
        Image {
            asset: "frame".into(),
            width: 96,
            height: 48,
            regions: (0..regions).map(|_| Region::full(96, 48)).collect(),
        }
    }

    #[test]
    fn single_picture_frame_is_cut_into_a_grid_around_the_window() {
        let (pieces, inner) = nine_slice(&image(1), [10, 20, 200, 100], [4, 4, 4, 4]);
        assert_eq!(pieces.len(), 8);
        // Corners are a third of the shorter side and keep their size.
        assert_eq!(pieces[0], ([0, 0, 16, 16], [10, 20, 16, 16]));
        assert_eq!(pieces[3], ([80, 32, 16, 16], [194, 104, 16, 16]));
        // Edges stretch across the space the corners leave.
        assert_eq!(pieces[4].1, [26, 20, 168, 16]);
        assert_eq!(pieces[6].1, [10, 36, 16, 68]);
        // AREA lets the backing run under the frame edge.
        assert_eq!(inner, [22, 32, 176, 76]);
    }

    #[test]
    fn half_width_characters_take_half_a_cell() {
        assert_eq!(text_width("aｱ字", 28), 14 + 14 + 28);
    }

    #[test]
    fn subtractive_backing_darkens_then_adds_and_filter_one_blends() {
        let attr = WindowAttr { r: 10, g: 20, b: 30, alpha: 64, filter: 0 };
        let layers = backing_layers(5, [0, 0, 4, 4], attr, 0);
        assert_eq!(layers[0]["blend"], "subtract");
        assert_eq!(layers[1]["blend"], "add");
        assert_eq!(layers[1]["paint"]["color"], json!([10, 20, 30, 255]));
        let attr = WindowAttr { r: 10, g: 20, b: 30, alpha: 64, filter: 1 };
        let layers = backing_layers(5, [0, 0, 4, 4], attr, 0);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0]["blend"], "normal");
    }
}
