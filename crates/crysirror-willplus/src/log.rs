// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! `lua_Layers` message-log layer (`insertLogLayer` and the `*Log*` calls),
//! as the backlog scripts use it:
//!
//! * `getLogTextLine` -> (total lines, first shown line) and
//!   `getLogTextDrawCount` -> lines per view drive the scroll bar;
//! * `onWheelMouseLogLayer(set, key, lines, close)` scrolls (+ = up) and
//!   returns true when scrolling down from the bottom (the scripts close the
//!   log then); `pageUpLogText` scrolls one view up;
//! * `hitTestLogLayerText(x, y)` -> entry under the pointer (-1 none), whose
//!   rectangle `getLogTextRect` returns; `getLogTextRectIndex(entry)` returns
//!   any entry's; `getLogLineToIndex(line)` maps a line to its entry and
//!   `getIndexLogLayerText(entry)` returns the entry when the scene can be
//!   jumped back to (its line has a checkpoint), else -1;
//! * `LUA_PROCESS_LOG_VOICEPLAY(entry)` / `LUA_PROCESS_BACKGAME(entry)` and
//!   `insertLogPreviewLayer(set, key, entry, w, h, visible)` use entries.
//!
//! Layout (host policy; the packed executable's is not observable): a
//! speaker line, then the wrapped text, then one blank line per entry, in the
//! `CLegacyFontInfo` style registered for the log key.
use crate::layers::{Item, Kind, Layers, Params};
use crate::system::{int, num, text};
use crate::vm::LogEntry;
use anyhow::{Context, Result};
use crysirror_lua::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Name(usize),
    Text(usize, String),
    Blank(usize),
}
impl Line {
    pub fn entry(&self) -> usize {
        match self {
            Line::Name(e) | Line::Text(e, _) | Line::Blank(e) => *e,
        }
    }
}

/// Wrapped lines of `text` at `width` (same cell rules as the window).
pub fn wrap(text: &str, width: f64, size: f64) -> Result<Vec<String>> {
    let glyphs = crate::vm::layout_glyphs(text, width, size, 0., 0)?;
    let mut rows: BTreeMap<i64, String> = BTreeMap::new();
    for g in glyphs {
        let y = g["baseline"][1].as_i64().unwrap_or(0);
        rows.entry(y).or_default().push_str(g["character"].as_str().unwrap_or(""));
    }
    Ok(rows.into_values().collect())
}

pub fn display(entry: &LogEntry) -> Result<String> {
    crate::vm::Machine::message(&entry.text.replace("%K", "").replace("%P", ""))
}

pub struct Geometry {
    pub pos: [f64; 2],
    pub margins: [f64; 4],
    pub size: [f64; 2],
    pub pitch: f64,
    pub font: f64,
}
impl Geometry {
    pub fn width(&self) -> f64 {
        (self.size[0] - self.margins[0] - self.margins[2]).max(1.)
    }
    pub fn view(&self) -> usize {
        (((self.size[1] - self.margins[1] - self.margins[3]) / self.pitch).floor().max(1.)) as usize
    }
}

pub fn lines(log: &[LogEntry], geometry: &Geometry) -> Result<Vec<Line>> {
    let mut out = Vec::new();
    for (n, entry) in log.iter().enumerate() {
        if !entry.speaker.is_empty() {
            out.push(Line::Name(n));
        }
        for row in wrap(&display(entry)?, geometry.width(), geometry.font)? {
            out.push(Line::Text(n, row));
        }
        out.push(Line::Blank(n));
    }
    Ok(out)
}

/// Geometry of a log item (font size from its CLegacyFontInfo entry).
pub fn geometry(item: &Item, font: Option<f64>) -> Option<Geometry> {
    let Kind::Log { size, margins, line, .. } = &item.kind else { return None };
    let font = font.unwrap_or(20.);
    Some(Geometry { pos: [item.params.pos[0], item.params.pos[1]], margins: *margins, size: *size, pitch: (font * (1. + line)).round().max(1.), font })
}

fn entry_rect(lines: &[Line], top: usize, g: &Geometry, entry: usize) -> [f64; 4] {
    let first = lines.iter().position(|l| l.entry() == entry).unwrap_or(0);
    let count = lines.iter().filter(|l| l.entry() == entry && !matches!(l, Line::Blank(_))).count().max(1);
    let y = g.pos[1] + g.margins[1] + (first as f64 - top as f64) * g.pitch;
    [g.pos[0] + g.margins[0], y, g.width(), count as f64 * g.pitch]
}

fn four(v: [f64; 4]) -> Vec<Value> {
    v.iter().map(|x| Value::Int(x.round() as i64)).collect()
}

/// Log methods on `lua_Layers`; None when `method` is not one of them.
#[allow(clippy::too_many_arguments)]
pub fn call(layers: &mut Layers, log: &[LogEntry], fonts: &BTreeMap<(String, String), crate::vm::FontStyle>, method: &str, args: &[Value], redraw: &mut bool) -> Result<Option<Vec<Value>>> {
    let a = &args[1.min(args.len())..];
    if method == "insertLogLayer" {
        //(set, key, width, height, left, top, right, bottom, line margin, visible)
        let (set, key) = (text(a, 0)?, text(a, 1)?);
        let kind = Kind::Log { size: [num(a, 2)?, num(a, 3)?], margins: [num(a, 4)?, num(a, 5)?, num(a, 6)?, num(a, 7)?], line: num(a, 8)?, top: usize::MAX, hover: -1 };
        let visible = int(a, 9)? != 0;
        let target = layers.set_mut(&set)?;
        target.items.retain(|i| i.key != key);
        target.items.push(Item { key, kind, visible, params: Params::default(), effects: BTreeMap::new(), subs: BTreeMap::new(), animations: BTreeMap::new() });
        *redraw = true;
        return Ok(Some(vec![Value::Bool(true)]));
    }
    const METHODS: &[&str] = &["getLogTextDrawCount", "getLogTextLine", "onWheelMouseLogLayer", "pageUpLogText", "hitTestLogLayerText", "getLogTextRect", "getLogTextRectIndex", "getLogLineToIndex", "getIndexLogLayerText", "stopLogVoice"];
    if !METHODS.contains(&method) {
        return Ok(None);
    }
    let (set, key) = (text(a, 0)?, text(a, 1)?);
    let font = fonts.get(&(set.clone(), key.clone())).map(|f| f.size);
    let item = layers.item_mut(&set, &key).with_context(|| format!("AdvHD log layer {set}/{key}"))?;
    let g = geometry(item, font).with_context(|| format!("AdvHD {set}/{key} is not a log layer"))?;
    let all = lines(log, &g)?;
    let view = g.view();
    let bottom = all.len().saturating_sub(view);
    let Kind::Log { top, hover, .. } = &mut item.kind else { unreachable!() };
    if *top == usize::MAX || *top > bottom {
        *top = bottom;
    }
    let checkpoint = |entry: i64| usize::try_from(entry).ok().and_then(|e| log.get(e)).is_some_and(|e| e.checkpoint.is_some());
    Ok(Some(match method {
        "getLogTextDrawCount" => vec![Value::Int(view as i64)],
        "getLogTextLine" => vec![Value::Int(all.len() as i64), Value::Int(*top as i64)],
        "onWheelMouseLogLayer" => {
            let delta = int(a, 2)?;
            let at_bottom = *top >= bottom;
            *top = (*top as i64 - delta).clamp(0, bottom as i64) as usize;
            *redraw = true;
            vec![Value::Bool(delta < 0 && at_bottom)]
        }
        "pageUpLogText" => {
            *top = top.saturating_sub(view);
            *redraw = true;
            vec![Value::Int(*top as i64)]
        }
        "hitTestLogLayerText" => {
            let (x, y) = (num(a, 2)?, num(a, 3)?);
            let inside = x >= g.pos[0] && x < g.pos[0] + g.size[0] && y >= g.pos[1] + g.margins[1];
            let row = ((y - g.pos[1] - g.margins[1]) / g.pitch).floor();
            let line = if inside && row >= 0. && (row as usize) < view { all.get(*top + row as usize) } else { None };
            *hover = match line {
                Some(Line::Blank(_)) | None => -1,
                Some(line) => line.entry() as i64,
            };
            vec![Value::Int(*hover)]
        }
        "getLogTextRect" => four(if *hover >= 0 { entry_rect(&all, *top, &g, *hover as usize) } else { [0.; 4] }),
        "getLogTextRectIndex" => four(entry_rect(&all, *top, &g, usize::try_from(int(a, 2)?).unwrap_or(0))),
        "getLogLineToIndex" => {
            let line = usize::try_from(int(a, 2)?).ok().and_then(|l| all.get(l));
            vec![Value::Int(line.map_or(-1, |l| l.entry() as i64))]
        }
        "getIndexLogLayerText" => {
            let entry = int(a, 2)?;
            vec![Value::Int(if checkpoint(entry) { entry } else { -1 })]
        }
        "stopLogVoice" => vec![],
        _ => unreachable!(),
    }))
}
