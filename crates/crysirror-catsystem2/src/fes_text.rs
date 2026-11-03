// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original STRING/STRING8 objects (classes 21, 37): a Plane (74B480) carrying a
//! message buffer (752EC0 str, 752F90 apend), layout words (`layout` family), and a
//! timed reveal (753620 draw, 752DC0 isdraw, 753680 skip). Glyphs are published as
//! the engine's GlyphRun; shaping, rasterization and fonts stay in CrySirror.
use crate::{
    fes_object_bank::Bank, fes_references::IndexedName, native_class_state::State,
    native_interpreter::Interpreter, native_values::StringVariables,
};
use anyhow::{Context, Result};
use crysirror_core::service::HostServices;

#[derive(Clone, Debug)]
pub(crate) struct TextObject {
    /// LOGSTRING: the message log this object pages through.
    pub log: Option<LogView>,
    pub sizes: [f32; 5],
    pub margin: [f32; 4],
    /// `layout frame`: 60Hz frames per character; zero or less shows the page at once.
    pub frame: i32,
    pub spacing: [f32; 2],
    pub color: [u8; 3],
    pub indent: bool,
    pub text: Vec<u8>,
    pub drawing: bool,
    pub skip: bool,
    pub started: Option<u32>,
    pub base_visible: usize,
    pub visible: usize,
    pub glyphs: Vec<crysirror_scene::layer::PositionedGlyph>,
    pub shaped: Option<(Vec<u8>, [i32; 2])>,
    pub end: [i32; 2],
    pub lines: u32,
    pub generation: u64,
}
impl Default for TextObject {
    fn default() -> Self {
        Self {
            log: None,
            sizes: [18., 22., 25., 28., 32.],
            margin: [0.; 4],
            frame: 0,
            spacing: [0.; 2],
            color: [255; 3],
            indent: false,
            text: Vec::new(),
            drawing: false,
            skip: false,
            started: None,
            base_visible: 0,
            visible: 0,
            glyphs: Vec::new(),
            shaped: None,
            end: [0; 2],
            lines: 0,
            generation: 0,
        }
    }
}
fn is_text_method(method: &[u8]) -> bool {
    matches!(method, b"layout" | b"str" | b"apend" | b"apendmark" | b"clear" | b"reset" | b"draw" | b"skip"
        | b"isdraw" | b"endpos" | b"linecnt" | b"userfont" | b"userfontobj" | b"delay")
}
fn is_common_method(method: &[u8]) -> bool {
    matches!(method, b"pos" | b"pos2" | b"getpos" | b"getpos2" | b"getsize" | b"page")
}
/// A LOGSTRING's window on its log: `COUNT` blocks from `top`.
#[derive(Clone, Debug, Default)]
pub(crate) struct LogView {
    pub id: u32,
    pub top: usize,
}
/// LOGSTRING blocks visible at once (hist_B's OBJECT COUNT).
const LOG_BLOCKS: usize = 4;
/// First text line of each block: the screen's 155-pixel block pitch in 29-pixel lines.
const LOG_BLOCK_STARTS: [usize; LOG_BLOCKS + 1] = [0, 5, 11, 16, 21];
pub(crate) fn is_log_method(method: &[u8]) -> bool {
    matches!(method, b"setlog" | b"remake" | b"max" | b"lineinfo" | b"pathinfo" | b"blockinfo" | b"blockcolor" | b"getpos" | b"pos")
}
fn log_page(rows: &[crate::native_log::Row], top: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for (block, row) in rows.iter().skip(top).take(LOG_BLOCKS).enumerate() {
        // The page-wait mark `\\@` closes a message; it is not text.
        let body = row.text.strip_suffix(b"\\@").unwrap_or(&row.text);
        let lines = 1 + body.windows(2).filter(|pair| pair == b"\\n").count();
        out.extend_from_slice(body);
        for _ in lines - 1..LOG_BLOCK_STARTS[block + 1] - LOG_BLOCK_STARTS[block] {
            out.extend_from_slice(b"\\n");
        }
    }
    out
}
/// 75FDD0 and family on a LOGSTRING; `false` when the target is not one.
pub(crate) fn log_command(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables, logs: &crate::native_log::Logs) -> Result<bool> {
    let method = arguments.first().context("CatSystem2 log method missing")?.clone();
    let args = &arguments[1..];
    let mut staged = vm.clone();
    let mut handled = false;
    for id in members(bank, target, &mut staged, strings)? {
        let State::Plane(plane) = &mut bank.objects.get_mut(&id).context("CatSystem2 log member missing")?.state else { continue };
        if !plane.log_string {
            continue;
        }
        let Some(text) = plane.text.as_mut() else { continue };
        handled = true;
        let word = |vm: &mut Interpreter, at: usize| args.get(at).map_or(Ok(0), |source| vm.evaluate(source));
        match method.as_slice() {
            b"setlog" => {
                let log = word(&mut staged, 0)?;
                text.log = (log != 0).then(|| LogView { id: log, top: logs.rows(log).len().saturating_sub(LOG_BLOCKS) });
            }
            b"remake" | b"pos" => {
                if method == b"pos" {
                    let top = word(&mut staged, 0)? as usize;
                    if let Some(view) = &mut text.log { view.top = top; }
                }
                if let Some(view) = &text.log {
                    text.text = log_page(logs.rows(view.id), view.top);
                    text.drawing = true;
                    text.skip = true;
                    text.started = None;
                    text.generation += 1;
                }
            }
            b"max" | b"getpos" => {
                let value = text.log.as_ref().map_or(0, |view| if method == b"max" { logs.rows(view.id).len().saturating_sub(LOG_BLOCKS) } else { view.top });
                let flag = word(&mut staged, 0)?;
                staged.set_flag(flag, value as u32);
            }
            // 75E920/75E9C0: no recorded voice or jump target for a row yet.
            b"lineinfo" | b"pathinfo" | b"blockinfo" => {
                for at in 1..args.len() {
                    let flag = word(&mut staged, at)?;
                    if flag != u32::MAX { staged.set_flag(flag, if method == b"blockinfo" { 0 } else { u32::MAX }); }
                }
            }
            // 75EB40: highlight colour of a block; the page is drawn in one run.
            _ => {}
        }
    }
    if handled {
        *vm = staged;
    }
    Ok(handled)
}
/// Methods accepted on every class until their capability exists: animation lists
/// (setanm/anm), face composition (attr/refex) and SAVEPANEL's save pages.
fn is_inert_method(method: &[u8]) -> bool {
    matches!(method, b"setanm" | b"anm" | b"attr" | b"refex" | b"ptype" | b"dispid" | b"setsave" | b"getsavestr" | b"getsavedate" | b"setnum" | b"mode")
}
fn members(bank: &Bank, target: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<Vec<u32>> {
    let reference = crate::fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
    let Some(count) = bank.count(&reference.name) else { return Ok(Vec::new()) };
    let range = if reference.index == -1 { 0..count as i32 } else { reference.index..reference.index + 1 };
    Ok(range.filter_map(|index| bank.lookup(&IndexedName { name: reference.name.clone(), index })).collect())
}
pub(crate) fn applies(bank: &Bank, target: &[u8], method: &[u8], vm: &Interpreter, strings: &StringVariables) -> Result<bool> {
    if !is_text_method(method) && !is_common_method(method) && !is_inert_method(method) {
        return Ok(false);
    }
    let mut staged = vm.clone();
    let ids = members(bank, target, &mut staged, strings)?;
    let Some(id) = ids.first() else { return Ok(false) };
    if is_inert_method(method) {
        return Ok(true);
    }
    Ok(match &bank.objects[id].state {
        State::Plane(plane) if plane.text.is_some() => true,
        State::Plane(_) | State::CPlane(_) | State::Image(_) | State::CImage(_) => is_common_method(method),
        _ => false,
    })
}
/// Everything after the first two words of a row: 766090 reads the row's raw tail, so
/// message text keeps its spaces.
fn raw_tail(raw: &[u8]) -> Vec<u8> {
    let mut rest = raw;
    for _ in 0..2 {
        let start = rest.iter().position(|b| !matches!(b, b' ' | b'\t')).unwrap_or(rest.len());
        rest = &rest[start..];
        let end = rest.iter().position(|b| matches!(b, b' ' | b'\t')).unwrap_or(rest.len());
        rest = &rest[end..];
    }
    let start = rest.iter().position(|b| !matches!(b, b' ' | b'\t')).unwrap_or(rest.len());
    rest[start..].to_vec()
}
pub(crate) fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], raw: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<()> {
    let method = arguments.first().context("CatSystem2 text method missing")?.clone();
    let args = &arguments[1..];
    let mut staged = vm.clone();
    let owner = bank.owner;
    if is_inert_method(&method) {
        return Ok(());
    }
    for id in members(bank, target, &mut staged, strings)? {
        let object = bank.objects.get_mut(&id).context("CatSystem2 text member missing")?;
        match method.as_slice() {
            b"pos2" => {
                // 749610: the position comes from an HG3 frame, resolved by the next class frame.
                // A second word is the frame, as in `pl_name pos2 $str900 100`.
                let raw = match (args.first(), args.get(1)) {
                    (Some(name), Some(frame)) => Some([name.as_slice(), b":", staged.evaluate(frame)?.to_string().as_bytes()].concat()),
                    (Some(name), None) => Some(name.clone()),
                    _ => None,
                };
                let reference = raw.map(|raw| crate::native_class_state::MetadataReference::parse(&raw)).transpose()?;
                match &mut object.state {
                    State::Plane(plane) | State::CPlane(plane) => plane.position_reference = reference,
                    State::Image(image) | State::CImage(image) => image.position_reference = reference,
                    _ => {}
                }
            }
            // 74DB80 / 74DC10: a slider's `page` is its range, `pos`/`getpos` its value.
            b"page" | b"pos" | b"getpos" if matches!(&object.state, State::Image(image) if image.slider.is_some()) => {
                let State::Image(image) = &mut object.state else { continue };
                let slider = image.slider.as_mut().unwrap();
                match method.as_slice() {
                    b"page" => {
                        let low = args.first().map_or(Ok(0), |source| staged.evaluate(source))? as i32;
                        let high = args.get(1).map_or(Ok(0), |source| staged.evaluate(source))? as i32;
                        slider.range = [low, high];
                    }
                    b"pos" => {
                        if let Some(source) = args.first() {
                            let value = staged.evaluate(source)? as i32;
                            slider.set_value(value);
                        }
                    }
                    _ => {
                        if let Some(source) = args.first() {
                            let flag = staged.evaluate(source)?;
                            staged.set_flag(flag, slider.value as u32);
                        }
                    }
                }
            }
            b"page" => {}
            b"pos" | b"getpos" | b"getpos2" | b"getsize" => {
                let (position, size) = match &mut object.state {
                    State::Plane(plane) | State::CPlane(plane) => (&mut plane.pose.position[..], Some(plane.size)),
                    State::Image(image) | State::CImage(image) => (&mut image.pose.position[..], Some(image.size)),
                    _ => continue,
                };
                if method == b"pos" {
                    for (i, word) in args.iter().take(position.len().min(3)).enumerate() {
                        position[i] = staged.evaluate(word)? as i32 as f32;
                    }
                } else {
                    let values = if method != b"getsize" {
                        [position[0] as i32, position[1] as i32, position.get(2).copied().unwrap_or(0.) as i32]
                    } else {
                        let size = size.unwrap_or_default();
                        [size[0], size[1], 0]
                    };
                    for (word, value) in args.iter().zip(values) {
                        let flag = staged.evaluate(word)?;
                        staged.set_flag(flag, value as u32);
                    }
                }
            }
            _ => {
                let State::Plane(plane) = &mut object.state else { continue };
                let Some(text) = plane.text.as_mut() else { continue };
                match method.as_slice() {
                    b"layout" => layout(text, args, &mut staged)?,
                    b"str" => {
                        text.text = raw_tail(raw);
                        restart(text);
                    }
                    b"apend" => {
                        text.base_visible = text.visible;
                        text.text.extend(raw_tail(raw));
                        text.drawing = false;
                        text.generation += 1;
                    }
                    b"apendmark" => text.generation += 1,
                    b"clear" => {
                        text.text.clear();
                        restart(text);
                    }
                    b"reset" => **text = TextObject { log: text.log.take(), text: core::mem::take(&mut text.text), generation: text.generation + 1, ..Default::default() },
                    b"draw" => {
                        text.drawing = true;
                        text.skip = false;
                        text.started = None;
                    }
                    b"skip" => text.skip = true,
                    b"isdraw" => {
                        staged.set_player_local(owner, 0, u32::from(text.drawing))?;
                    }
                    b"endpos" => {
                        for (word, value) in args.iter().skip(1).zip(text.end) {
                            let flag = staged.evaluate(word)?;
                            staged.set_flag(flag, value as u32);
                        }
                    }
                    b"linecnt" => {
                        if let Some(word) = args.first() {
                            let flag = staged.evaluate(word)?;
                            staged.set_flag(flag, text.lines);
                        }
                    }
                    // userfont maps private code points to image cells; delay holds the
                    // next reveal. Neither alters ordinary message text layout.
                    _ => {}
                }
            }
        }
    }
    *vm = staged;
    Ok(())
}
fn restart(text: &mut TextObject) {
    text.drawing = false;
    text.skip = false;
    text.started = None;
    text.base_visible = 0;
    text.visible = 0;
    text.shaped = None;
    text.generation += 1;
}
fn layout(text: &mut TextObject, args: &[Vec<u8>], vm: &mut Interpreter) -> Result<()> {
    let Some(key) = args.first() else { return Ok(()) };
    let mut number = |i: usize, vm: &mut Interpreter| -> Result<Option<i32>> {
        args.get(i).map(|word| vm.evaluate(word).map(|v| v as i32)).transpose()
    };
    match key.as_slice() {
        b"size" => {
            for i in 0..5 {
                if let Some(value) = number(i + 1, vm)? {
                    text.sizes[i] = value as f32;
                }
            }
        }
        b"margin" => {
            for i in 0..4 {
                if let Some(value) = number(i + 1, vm)? {
                    text.margin[i] = value as f32;
                }
            }
        }
        b"frame" => text.frame = number(1, vm)?.unwrap_or(0),
        b"move" => {
            for i in 0..2 {
                if let Some(value) = number(i + 1, vm)? {
                    text.spacing[i] = value as f32;
                }
            }
        }
        b"color" => {
            for i in 0..3 {
                if let Some(value) = number(i + 1, vm)? {
                    text.color[i] = value as u8;
                }
            }
        }
        b"indent" => text.indent = number(1, vm)?.unwrap_or(0) != 0,
        // Font selection, kinsoku tables and ruby spacing: their words are ignored here,
        // never evaluated, because they carry file names and literal characters.
        _ => {}
    }
    text.shaped = None;
    Ok(())
}
/// Original inline escapes: `\n` breaks a line, `[base/ruby]` shows its base, and other
/// backslash sequences are control codes with no glyphs of their own.
pub(crate) fn plain_lines(source: &str) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut chars = source.chars().peekable();
    let mut ruby = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => lines.push(String::new()),
                Some(_) | None => {}
            },
            '\n' => lines.push(String::new()),
            '\r' => {}
            '[' => {}
            '/' if !ruby && source.contains('[') => ruby = true,
            ']' => ruby = false,
            _ if ruby => {}
            _ => lines.last_mut().unwrap().push(c),
        }
    }
    lines
}
/// One owner frame: shape changed text and advance its reveal clock.
pub(crate) fn advance(bank: &mut Bank, clock: u32, host: &dyn HostServices, font: &str) -> Result<()> {
    for object in bank.objects.values_mut() {
        let State::Plane(plane) = &mut object.state else { continue };
        let width = plane.size[0];
        let Some(text) = plane.text.as_mut() else { continue };
        let key = (text.text.clone(), plane.size);
        if text.shaped.as_ref() != Some(&key) {
            shape(text, width, host, font)?;
            text.shaped = Some(key);
            text.generation += 1;
        }
        let total = text.glyphs.len();
        let before = text.visible;
        if text.drawing {
            let started = *text.started.get_or_insert(clock);
            let shown = if text.skip || text.frame <= 0 {
                total
            } else {
                text.base_visible + (clock.saturating_sub(started) as usize) / text.frame as usize
            };
            text.visible = shown.min(total);
            if text.visible == total {
                text.drawing = false;
                text.skip = false;
            }
        }
        if text.visible != before {
            text.generation += 1;
        }
    }
    Ok(())
}
fn shape(text: &mut TextObject, width: i32, host: &dyn HostServices, font: &str) -> Result<()> {
    let source = crysirror_resource::text_stream::decode_auto(&text.text, 65534)?;
    let size = text.sizes[2].max(1.);
    let metrics = host.font_metrics(font, size, 'あ').context("CatSystem2 message font metrics")?;
    let line_height = metrics.ascent + metrics.descent + text.spacing[1];
    let right = width as f32 - text.margin[2];
    let mut glyphs = Vec::new();
    let mut pen = [text.margin[0], text.margin[1]];
    let mut lines = 1;
    for (i, line) in plain_lines(&source).iter().enumerate() {
        if i > 0 {
            pen = [text.margin[0], pen[1] + line_height];
            lines += 1;
        }
        let mut hanging = false;
        for character in line.chars() {
            let advance = host.font_metrics(font, size, character)?.advance + text.spacing[0];
            if pen[0] + advance > right && pen[0] > text.margin[0] {
                pen = [text.margin[0] + if hanging { size } else { 0. }, pen[1] + line_height];
                lines += 1;
            }
            if text.indent && pen[0] == text.margin[0] && matches!(character, '「' | '『' | '（') {
                hanging = true;
            }
            glyphs.push(crysirror_scene::layer::PositionedGlyph {
                crop: None,
                character,
                baseline: [pen[0], pen[1] + metrics.ascent],
                size: None,
                color: None,
                bold: None,
            });
            pen[0] += advance;
        }
    }
    text.end = [pen[0] as i32, pen[1] as i32];
    text.lines = if glyphs.is_empty() { 0 } else { lines };
    text.glyphs = glyphs;
    text.visible = text.visible.min(text.glyphs.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_break_lines_and_ruby_shows_its_base() {
        assert_eq!(plain_lines("a\\nb"), ["a", "b"]);
        assert_eq!(plain_lines("[漢字/かんじ]x"), ["漢字x"]);
        assert_eq!(plain_lines("a/b"), ["a/b"]);
        assert_eq!(plain_lines("x\\@y"), ["xy"]);
    }
    #[test]
    fn raw_tail_keeps_message_spaces() {
        assert_eq!(raw_tail(b"str apend hello big  world"), b"hello big  world");
        assert_eq!(raw_tail(b"str clear"), b"");
    }
}
