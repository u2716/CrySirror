// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native SAVEPANEL part records (76D3E0/75B300/762620). These descriptions
//! contain original source/number policy; shared Image/Text services draw them.
use crate::{fes_references::IndexedName, native_interpreter::Interpreter, native_save::Container, native_values::StringVariables};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) enum Source { Flag(u32), Date(usize), Literal, RecordString(u32) }
#[derive(Clone, Debug, Default)]
pub(crate) struct TextStyle {
    /// 660110 adds eight before creating its original font.
    pub font_size: u32,
    pub columns: u32,
    pub fill_color: u32,
    pub edge_color: u32,
    pub flags: u32,
    pub max_lines: u32,
    pub line_pitch: u32,
    pub font: Vec<u8>,
}
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Norm,
    SavedImage { key: u32, size: Option<[u32; 2]>, clip: Option<[i32; 4]> },
    /// The native ptype IMAGE/NUM_I bitmap pointer is null until a native
    /// bitmap setter supplies one. Do not invent a bitmap from an object ID.
    Image,
    ObjectImage { target: IndexedName },
    NumberObject { target: IndexedName },
    NumberImage,
    Text { numeric: bool, style: TextStyle },
}
#[derive(Clone, Debug)]
pub(crate) struct Part {
    pub kind: Kind,
    pub source: Source,
    pub decimal_div: u32,
    pub raw_frame: bool,
    pub value: u32,
    /// setnum updates the live renderer value even when the saved source is a
    /// flag/date; the next setsave rebuilds it from that original source.
    pub number_override: Option<u32>,
}
impl Part {
    fn source_number(&self, flags: &BTreeMap<u32, u32>, date: [u32; 6]) -> u32 {
        match self.source {
            Source::Flag(index) => flags.get(&index).copied().unwrap_or(0),
            Source::Date(index) => date.get(index).copied().unwrap_or(0),
            Source::Literal | Source::RecordString(_) => self.value,
        }
    }
    pub(crate) fn number(&self, flags: &BTreeMap<u32, u32>, date: [u32; 6]) -> u32 {
        self.number_override.unwrap_or_else(|| self.source_number(flags, date))
    }
    pub(crate) fn frame(&self, flags: &BTreeMap<u32, u32>, date: [u32; 6]) -> Result<u32> {
        let number = self.number(flags, date);
        if self.decimal_div == 0 { Ok(number) } else {
            // Original renderer uses signed IDIV for this optional digit.
            let divisor = self.decimal_div as i32;
            let modulus = divisor.wrapping_mul(10);
            ensure!(modulus != 0 && !(number as i32 == i32::MIN && modulus == -1), "CatSystem2 panel decimal remainder fault");
            let remainder = number as i32 % modulus;
            ensure!(!(remainder == i32::MIN && divisor == -1), "CatSystem2 panel decimal division fault");
            Ok((remainder / divisor) as u32)
        }
    }
    pub(crate) fn text(&self, flags: &BTreeMap<u32, u32>, date: [u32; 6], saved: Option<&Container>) -> Vec<u8> {
        if let Source::RecordString(key) = self.source {
            return saved.and_then(|saved| saved.first(key)).filter(|bytes| bytes.len() < 256).map(|bytes| bytes.split(|byte| *byte == 0).next().unwrap_or_default().to_vec()).unwrap_or_default();
        }
        // 75CD60 writes the numeric renderer word, not STR's text allocation.
        (self.source_number(flags, date) as i32).to_string().into_bytes()
    }
}

fn source(raw: &[u8], vm: &mut Interpreter) -> Result<(Source, u32, u32)> {
    ensure!(raw.len() < 96 && !raw.contains(&0), "CatSystem2 panel source extent");
    let at = raw.iter().position(|byte| matches!(*byte, b'.' | b':')).unwrap_or(raw.len());
    let name = &raw[..at];
    let mut index = 0;
    let mut end = at;
    if raw.get(at) == Some(&b'.') {
        end = raw[at + 1..].iter().position(|byte| *byte == b':').map_or(raw.len(), |offset| at + 1 + offset);
        index = vm.evaluate(&raw[at + 1..end])?;
    }
    let has_divider = raw.get(end) == Some(&b':');
    let divider = if has_divider { vm.evaluate(&raw[end + 1..])? } else { 0 };
    let source = match name {
        b"FLAG" => Source::Flag(index),
        b"DATE_Y" => Source::Date(0), b"DATE_M" => Source::Date(1), b"DATE_D" => Source::Date(2),
        b"DATE_H" => Source::Date(3), b"DATE_N" => Source::Date(4), b"DATE_S" => Source::Date(5),
        _ => return Ok((Source::Literal, vm.evaluate(name)?, divider)),
    };
    let divider = if matches!(source, Source::Date(_)) && !has_divider { u32::MAX } else { divider };
    Ok((source, 0, divider))
}

/// Arguments start at the ptype kind (NORM/NUM_O/...). Fields are evaluated
/// once before installing any member's record.
pub(crate) fn parse(words: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables, count: &impl Fn(&[u8]) -> Option<u32>) -> Result<(u32, Part)> {
    ensure!(words.len() >= 2, "CatSystem2 panel ptype kind/id arguments");
    let id = vm.evaluate(&words[1])?;
    let args = &words[2..];
    let mut part = Part { kind: Kind::Norm, source: Source::Flag(0), decimal_div: 0, raw_frame: false, value: 0, number_override: None };
    part.kind = match words[0].as_slice() {
        b"NORM" => Kind::Norm,
        b"IMAGE" => Kind::Image,
        b"SIMAGE" => {
            ensure!(!args.is_empty(), "CatSystem2 SIMAGE record key");
            let mut numeric = |at: usize, default| args.get(at).map_or(Ok(default), |arg| vm.evaluate(arg));
            let key = numeric(0, 0)?;
            let size = if args.len() >= 2 { Some([numeric(1, 0)?, numeric(2, 0)?]) } else { None };
            if let Some(size) = size { ensure!(size.iter().all(|value| *value <= 8192), "CatSystem2 panel image size budget"); }
            let clip = if let Some([width, height]) = size {
                let rect = [numeric(3, 0)? as i32, numeric(4, 0)? as i32, numeric(5, 0)? as i32, numeric(6, 0)? as i32];
                Some(if rect[2] == 0 || rect[3] == 0 { [0, 0, width.wrapping_sub(1) as i32, height.wrapping_sub(1) as i32] } else { rect })
            } else { None };
            Kind::SavedImage { key, size, clip }
        }
        b"IMAGEOBJ" => {
            let raw = args.first().context("CatSystem2 IMAGEOBJ reference")?;
            // 76D3E0 case2 does not call75B300: the numeric renderer word is
            // the zeroed record's literal zero, not saved FLAG0.
            part.source = Source::Literal;
            Kind::ObjectImage { target: crate::fes_references::evaluate(raw, vm, strings, count)? }
        }
        b"NUM_O" | b"NUM_O2" => {
            let raw = args.first().context("CatSystem2 NUM_O object reference")?;
            let target = crate::fes_references::evaluate(raw, vm, strings, count)?;
            let raw = args.get(1).context("CatSystem2 NUM_O numeric source")?;
            (part.source, part.value, part.decimal_div) = source(raw, vm)?;
            part.raw_frame = words[0] == b"NUM_O2";
            Kind::NumberObject { target }
        }
        b"NUM_I" => Kind::NumberImage,
        b"NUM_S" => Kind::Text { numeric: true, style: TextStyle::default() },
        b"STR" | b"SSTR" => {
            let raw = args.first().context("CatSystem2 panel STR source")?;
            (part.source, part.value, part.decimal_div) = source(raw, vm)?;
            if matches!(part.source, Source::Literal) { part.source = Source::RecordString(part.value); }
            let mut number = |at: usize, default| args.get(at).map_or(Ok(default), |raw| vm.evaluate(raw));
            Kind::Text { numeric: false, style: TextStyle {
                font_size: number(1, 0)?, columns: number(2, 0)?, fill_color: number(3, 0)?,
                flags: number(4, 0)?, edge_color: number(5, 0)?, max_lines: number(6, 1)?, line_pitch: number(7, 0)?,
                font: args.get(8).map_or(Ok(Vec::new()), |font| strings.expand(font, 64))?,
            } }
        }
        other => anyhow::bail!("CatSystem2 unsupported panel ptype {}", String::from_utf8_lossy(other)),
    };
    Ok((id, part))
}
