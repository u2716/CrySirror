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
    /// 69CA30: unsigned reveal clock multiplier, in tenths; never a font scale.
    pub reveal_rate: u32,
    pub last_clock: u32,
    pub spacing: [f32; 2],
    pub color: [u8; 3],
    pub indent: bool,
    pub text: Vec<u8>,
    /// NVSTRING retains previous independently styled paragraph layouts.
    pub blocks: Vec<TextObject>,
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
    pub native_ids:std::collections::BTreeMap<u32,u32>,
}
impl Default for TextObject {
    fn default() -> Self {
        Self {
            log: None,
            sizes: [18., 22., 25., 28., 32.],
            margin: [0.; 4],
            frame: 0, reveal_rate:10,last_clock:0,
            spacing: [0.; 2],
            color: [255; 3],
            indent: false,
            text: Vec::new(),
            blocks: Vec::new(),
            drawing: false,
            skip: false,
            started: None,
            base_visible: 0,
            visible: 0,
            glyphs: Vec::new(),
            shaped: None,
            end: [0; 2],
            lines: 0,
            generation: 0,native_ids:Default::default(),
        }
    }
}
fn is_text_method(method: &[u8]) -> bool {
    matches!(method, b"layout" | b"str" | b"apend" | b"apendmark" | b"clear" | b"reset" | b"draw" | b"skip"
        | b"insert" | b"isdraw" | b"endpos" | b"linecnt" | b"userfont" | b"userfontobj" | b"delay" | b"mag" | b"strid")
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
    matches!(method, b"setlog" | b"remake" | b"max" | b"lineinfo" | b"pathinfo" | b"blockinfo" | b"blockcolor" | b"getpos" | b"pos" | b"strid")
}
fn log_blocks(rows: &[crate::native_log::Row]) -> Vec<std::ops::Range<usize>> {
    let mut blocks = Vec::new();
    let mut start = 0;
    for (index, row) in rows.iter().enumerate().skip(1) {
        if row.page_start { blocks.push(start..index); start = index; }
    }
    if !rows.is_empty() { blocks.push(start..rows.len()); }
    blocks
}
fn log_page(rows: &[crate::native_log::Row], top: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for (block, range) in log_blocks(rows).into_iter().skip(top).take(LOG_BLOCKS).enumerate() {
        let mut lines = 1;
        for (index, row) in rows[range].iter().enumerate() {
            if index != 0 { out.extend_from_slice(b"\\n"); lines += 1; }
            let body = row.text.strip_suffix(b"\\@").unwrap_or(&row.text);
            lines += body.windows(2).filter(|pair| pair == b"\\n").count();
            out.extend_from_slice(body);
        }
        for _ in lines..LOG_BLOCK_STARTS[block + 1] - LOG_BLOCK_STARTS[block] + 1 {
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
                text.log = (log != 0).then(|| LogView { id: log, top: log_blocks(logs.rows(log)).len().saturating_sub(LOG_BLOCKS) });
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
            b"strid" => {
                if args.len()>=3 {
                    let block=word(&mut staged,0)? as usize;
                    let key=word(&mut staged,1)?;
                    let destination=word(&mut staged,2)?;
                    let mut value=0;
                    if let Some(view)=&text.log {
                        let rows=logs.rows(view.id);
                        if let Some(range)=log_blocks(rows).get(view.top.saturating_add(block)).cloned(){
                            let mut body=Vec::new();for row in &rows[range]{body.extend_from_slice(&row.text);}
                            let source=crysirror_resource::text_stream::decode_auto(&body,65534)?;
                            value=strip_ids(&source).1.get(&key).copied().unwrap_or(0);
                        }
                    }
                    if destination!=u32::MAX{staged.set_flag(destination,value);}
                }
            }
            b"max" | b"getpos" => {
                let value = text.log.as_ref().map_or(0, |view| if method == b"max" { log_blocks(logs.rows(view.id)).len().saturating_sub(LOG_BLOCKS) } else { view.top });
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
fn members(bank: &Bank, target: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<Vec<u32>> {
    let reference = crate::fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
    let Some(count) = bank.count(&reference.name) else { return Ok(Vec::new()) };
    let range = if reference.index == -1 { 0..count as i32 } else { reference.index..reference.index + 1 };
    Ok(range.filter_map(|index| bank.lookup(&IndexedName { name: reference.name.clone(), index })).collect())
}
pub(crate) fn applies(bank: &Bank, target: &[u8], method: &[u8], vm: &Interpreter, strings: &StringVariables) -> Result<bool> {
    if !is_text_method(method) && !is_common_method(method) {
        return Ok(false);
    }
    let mut staged = vm.clone();
    let ids = members(bank, target, &mut staged, strings)?;
    let Some(id) = ids.first() else { return Ok(false) };
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
    let mag=if method==b"mag"{Some(args.first().map_or(Ok(10),|v|staged.evaluate(v))?)}else{None};
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
                    b"strid" => {
                        if matches!(object.declared_type,21|37)&&args.len()>=2 {
                            let key=staged.evaluate(&args[0])?;
                            let dest=staged.evaluate(&args[1])?;
                            if dest!=u32::MAX{staged.set_flag(dest,text.native_ids.get(&key).copied().unwrap_or(0));}
                        }
                    }
                    b"mag" => {
                        if matches!(object.declared_type,21|29|37) {
                            let value=mag.unwrap();
                            if text.reveal_rate!=value {text.reveal_rate=value;text.base_visible=text.visible;text.started=Some(text.last_clock);}
                        }
                    },
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
                    b"insert" => {
                        anyhow::ensure!(object.declared_type == 29, "CatSystem2 insert requires NVSTRING");
                        // 7516A0 -> 69DA40: retain the previous layout, then create
                        // a fresh paragraph at the previous paragraph's bottom.
                        let blocks = core::mem::take(&mut text.blocks);
                        let prefix: usize = blocks.iter().map(|paragraph| paragraph.visible).sum();
                        let mut block = (**text).clone();
                        text.blocks = blocks;
                        block.visible = block.visible.saturating_sub(prefix);
                        text.blocks.push(block);
                        text.text.clear();
                        text.glyphs.clear();
                        restart(text);
                    }
                    b"clear" => {
                        text.blocks.clear();
                        text.text.clear();
                        restart(text);
                    }
                    b"reset" => {
                        let source = if object.declared_type == 29 { Vec::new() } else { core::mem::take(&mut text.text) };
                        **text = TextObject { log: text.log.take(), text: source, generation: text.generation + 1, ..Default::default() };
                    },
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
    text.base_visible = text.blocks.iter().map(|block| block.visible).sum();
    text.visible = text.base_visible;
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
/// 6937B0/694780: `\\id<key>:<value>` metadata, first declaration wins.
/// Both numeric words accept unsigned decimal/hex and consume a trailing `;`.
fn strip_ids(source:&str)->(String,std::collections::BTreeMap<u32,u32>){
    fn number(bytes:&[u8])->(usize,u32){
        if !bytes.first().is_some_and(u8::is_ascii_digit){return (0,0);}
        let (mut at,base)=if bytes.starts_with(b"0x")||bytes.starts_with(b"0X"){(2,16u32)}else{(0,10)};
        let mut value=0u32;
        while let Some(byte)=bytes.get(at){
            let digit=match byte {b'0'..=b'9'=>u32::from(byte-b'0'),b'a'..=b'f' if base==16=>u32::from(byte-b'a'+10),b'A'..=b'F' if base==16=>u32::from(byte-b'A'+10),_=>break};
            value=value.wrapping_mul(base).wrapping_add(digit);at+=1;
        }
        if bytes.get(at)==Some(&b';'){at+=1;}
        (at,value)
    }
    let bytes=source.as_bytes();let mut out=Vec::new();let mut ids=std::collections::BTreeMap::new();let mut at=0;
    while at<bytes.len(){
        if bytes[at]==b'\\'&&bytes.get(at+1).is_some_and(|b|b.eq_ignore_ascii_case(&b'i'))&&bytes.get(at+2).is_some_and(|b|b.eq_ignore_ascii_case(&b'd')){
            at+=3;let (count,key)=number(&bytes[at..]);at+=count;let mut value=0;
            if count>0&&bytes.get(at)==Some(&b':'){at+=1;let (count,found)=number(&bytes[at..]);at+=count;value=found;}
            ids.entry(key).or_insert(value);
        }else{
            out.push(bytes[at]);
            if bytes[at]==b'\\'&&bytes.get(at+1)==Some(&b'\\'){at+=1;out.push(bytes[at]);}
            at+=1;
        }
    }
    (String::from_utf8(out).expect("ASCII control removal preserves UTF8"),ids)
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
                Some('c' | 'C') => {
                    // 6937B0 consumes the D/R/S selector as part of the colour
                    // command, then 694780's optional decimal/hex word. Leaving
                    // D behind draws a spurious 'd' before every default-colour
                    // message. Only the actual escape is consumed; literal text
                    // beginning with 'd' is unaffected.
                    if chars.peek().is_some_and(|c| matches!(c, 'd'|'D'|'r'|'R'|'s'|'S')) {
                        chars.next();
                    }
                    if chars.peek().is_some_and(char::is_ascii_digit) {
                        let first = chars.next().unwrap();
                        let hex = first=='0' && chars.peek().is_some_and(|c| matches!(c,'x'|'X'));
                        if hex { chars.next(); }
                        while chars.peek().is_some_and(|c| if hex {c.is_ascii_hexdigit()} else {c.is_ascii_digit()}) {
                            chars.next();
                        }
                        if chars.peek()==Some(&';') {chars.next();}
                    }
                },
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
                text.base_visible + (text.reveal_rate.wrapping_mul(clock.wrapping_sub(started))/10) as usize / text.frame as usize
            };
            text.visible = shown.min(total);
            if text.visible == total {
                text.drawing = false;
                text.skip = false;
            }
        }
        text.last_clock=clock;
        if text.visible != before {
            text.generation += 1;
        }
    }
    Ok(())
}
fn shape(text: &mut TextObject, width: i32, host: &dyn HostServices, font: &str) -> Result<()> {
    let visible = text.visible;
    shape_fragment(text, width, host, font)?;
    if text.blocks.is_empty() { return Ok(()); }
    let mut glyphs = Vec::new();
    let mut y = 0.;
    let mut lines = 0;
    for block in &mut text.blocks {
        shape_fragment(block, width, host, font)?;
        let size = block.sizes[2].max(1.);
        let metrics = host.font_metrics(font, size, 'あ')?;
        for mut glyph in block.glyphs.iter().take(block.visible).cloned() { glyph.baseline[1] += y; glyphs.push(glyph); }
        y += block.end[1] as f32 + metrics.ascent + metrics.descent + block.spacing[1];
        lines += block.lines;
    }
    for mut glyph in core::mem::take(&mut text.glyphs) { glyph.baseline[1] += y; glyphs.push(glyph); }
    text.end[1] += y as i32;
    text.lines += lines;
    text.visible = visible.min(glyphs.len());
    text.glyphs = glyphs;
    Ok(())
}
fn shape_fragment(text: &mut TextObject, width: i32, host: &dyn HostServices, font: &str) -> Result<()> {
    let source = crysirror_resource::text_stream::decode_auto(&text.text, 65534)?;
    let (source,ids)=strip_ids(&source);text.native_ids=ids;
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
            glyphs.push(crysirror_scene::layer::PositionedGlyph { image: None, style: None, font: None, horizontal_scale:None,font_width:None, cell_width: None,
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
