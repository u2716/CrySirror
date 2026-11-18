// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The configuration screen's tab pages (sub_41A2B0): `status.lwg`
//! (CDivParam 0x406650, shown by sub_407500) and `desig.lwg` (CDivDesig
//! 0x4063E0, shown by sub_407250).
//!
//! The status page has three columns. Each shows the image `c_K_S` (K the
//! column, S = variable 5015 + 10K), a 200x100 text box at (319,347),
//! (60,347), (578,347) and a 250x100 box at (299,440), (40,440),
//! (558,440), 16-pixel text in the APINI-independent palette at 0x4EB02C.
//! The first box is the name, the state name of S, three named values
//! (variables 5011..5013 + 10K) and the stamina line (5014 + 10K) whose
//! colour code depends on it (below 11, below 31, otherwise); the second is
//! the count line (5010 + 10K) and three lines of counters (5161.. + 100K).
//! The format strings and names are the executable's own (.rdata of the
//! verified build), read at start-up rather than copied.
//!
//! The designation page shows `e_N` and `p_N` for each N in 1..9 whose
//! variable 7100 + N is set, otherwise `d_N`.
use crate::memory::Memory;
use crate::res::join;
use crate::scene::Ctx;
use crate::sprite::{Point, Sprite, pt};

/// Text of the status page read from the executable.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StatusText {
    pub names: [Vec<u8>; 3],
    pub states: [Vec<u8>; 4],
    pub values: [Vec<u8>; 3],
    pub name_line: Vec<u8>,
    pub state_line: Vec<u8>,
    pub value_line: Vec<u8>,
    /// Stamina below 11, below 31, otherwise.
    pub stamina_lines: [Vec<u8>; 3],
    pub count_line: Vec<u8>,
    pub counter_lines: [Vec<u8>; 3],
    pub palette: [u32; 10],
}

/// Verified build (1.11.0.3) addresses.
mod va {
    pub const NAMES: u32 = 0x4EB000;
    pub const STATES: u32 = 0x4EB00C;
    pub const VALUES: u32 = 0x4EB01C;
    pub const PALETTE: u32 = 0x4EB02C;
    pub const NAME_LINE: u32 = 0x4C8CC4;
    pub const STATE_LINE: u32 = 0x4C8CCC;
    pub const VALUE_LINE: u32 = 0x4C8CD8;
    pub const STAMINA: [u32; 3] = [0x4C8CE8, 0x4C8D00, 0x4C8D18];
    pub const COUNT_LINE: u32 = 0x4C8D28;
    pub const COUNTER_LINES: [u32; 3] = [0x4C8D44, 0x4C8D64, 0x4C8D84];
}

/// Maps a virtual address of a PE image to its file bytes.
struct Image<'a> {
    bytes: &'a [u8],
    base: u32,
    sections: Vec<(u32, u32, u32, u32)>,
}
impl<'a> Image<'a> {
    fn new(bytes: &'a [u8]) -> Option<Self> {
        let rd16 = |o: usize| bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let rd32 = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let pe = rd32(0x3C)? as usize;
        (bytes.get(pe..pe + 4)? == b"PE\0\0").then_some(())?;
        let count = usize::from(rd16(pe + 6)?);
        let optional = usize::from(rd16(pe + 20)?);
        let base = rd32(pe + 24 + 28)?;
        let first = pe + 24 + optional;
        let sections = (0..count)
            .map(|i| {
                let s = first + 40 * i;
                Some((rd32(s + 12)?, rd32(s + 8)?, rd32(s + 20)?, rd32(s + 16)?))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self { bytes, base, sections })
    }
    fn at(&self, va: u32) -> Option<usize> {
        let rva = va.checked_sub(self.base)?;
        self.sections.iter().find(|s| rva >= s.0 && rva < s.0 + s.1.max(s.3)).map(|s| (s.2 + rva - s.0) as usize)
    }
    fn u32(&self, va: u32) -> Option<u32> {
        let o = self.at(va)?;
        Some(u32::from_le_bytes(self.bytes.get(o..o + 4)?.try_into().ok()?))
    }
    fn string(&self, va: u32) -> Option<Vec<u8>> {
        let o = self.at(va)?;
        let rest = self.bytes.get(o..)?;
        let end = rest.iter().take(256).position(|&b| b == 0)?;
        Some(rest[..end].to_vec())
    }
}

impl StatusText {
    pub fn from_exe(exe: &[u8]) -> Option<Self> {
        let img = Image::new(exe)?;
        let table = |va: u32, i: u32| img.u32(va + 4 * i).and_then(|p| img.string(p));
        Some(Self {
            names: [table(va::NAMES, 0)?, table(va::NAMES, 1)?, table(va::NAMES, 2)?],
            states: [table(va::STATES, 0)?, table(va::STATES, 1)?, table(va::STATES, 2)?, table(va::STATES, 3)?],
            values: [table(va::VALUES, 0)?, table(va::VALUES, 1)?, table(va::VALUES, 2)?],
            name_line: img.string(va::NAME_LINE)?,
            state_line: img.string(va::STATE_LINE)?,
            value_line: img.string(va::VALUE_LINE)?,
            stamina_lines: [img.string(va::STAMINA[0])?, img.string(va::STAMINA[1])?, img.string(va::STAMINA[2])?],
            count_line: img.string(va::COUNT_LINE)?,
            counter_lines: [img.string(va::COUNTER_LINES[0])?, img.string(va::COUNTER_LINES[1])?, img.string(va::COUNTER_LINES[2])?],
            palette: std::array::from_fn(|i| img.u32(va::PALETTE + 4 * i as u32).unwrap_or(0)),
        })
    }
}

/// Extras tables of the verified build: the CG view's 6 pages × 12 entries
/// of 6 image numbers (.rdata 0x4CB9F8) and the scene view's 12 entries of
/// 29 words (.data 0x4EB298: flag variable, thumbnail, start value, title).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GalleryTables {
    pub cg: Vec<u32>,
    pub scenes: Vec<Vec<u16>>,
}
impl GalleryTables {
    pub fn from_exe(exe: &[u8]) -> Option<Self> {
        let img = Image::new(exe)?;
        let cg = (0..6 * 72).map(|i| img.u32(0x4CB9F8 + 4 * i)).collect::<Option<Vec<_>>>()?;
        let at = img.at(0x4EB298)?;
        let raw = exe.get(at..at + 12 * 58)?;
        let scenes = raw.chunks_exact(58).map(|e| e.chunks_exact(2).map(|w| u16::from_le_bytes([w[0], w[1]])).collect()).collect();
        Some(Self { cg, scenes })
    }
}

/// A `wsprintfA` subset: `%s`, `%[width]d`, `%%`; any other character
/// after `%` is copied without the `%`, as wsprintfA does.
pub fn wsprintf(format: &[u8], args: &[Arg]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut args = args.iter();
    let mut i = 0;
    while i < format.len() {
        let c = format[i];
        if c != b'%' {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        let mut width = 0usize;
        while i < format.len() && format[i].is_ascii_digit() {
            width = width * 10 + usize::from(format[i] - b'0');
            i += 1;
        }
        match format.get(i) {
            Some(b's') => {
                if let Some(Arg::Text(t)) = args.next() {
                    out.extend_from_slice(t);
                }
            }
            Some(b'd') => {
                if let Some(Arg::Int(v)) = args.next() {
                    out.extend(format!("{v:>width$}").into_bytes());
                }
            }
            Some(&other) => out.push(other),
            None => {}
        }
        i += 1;
    }
    out
}

pub enum Arg<'a> {
    Text(&'a [u8]),
    Int(i32),
}

pub struct Column {
    pub image: Sprite,
    pub upper: Vec<u8>,
    pub lower: Vec<u8>,
    pub upper_at: Point,
    pub lower_at: Point,
}

#[derive(Default)]
pub struct StatusPages {
    pub tab: usize,
    pub status_lwg: Option<String>,
    pub desig_lwg: Option<String>,
    pub status_bg: Sprite,
    pub desig_bg: Sprite,
    pub columns: Vec<Column>,
    /// (`e_N` or `d_N`, `p_N`) per N.
    pub marks: Vec<(Sprite, Sprite)>,
    pub text: Option<StatusText>,
    images: Vec<(String, Sprite)>,
}

const UPPER: [(i32, i32); 3] = [(319, 347), (60, 347), (578, 347)];
const LOWER: [(i32, i32); 3] = [(299, 440), (40, 440), (558, 440)];

impl StatusPages {
    pub fn open(cx: &mut Ctx) -> Self {
        let sys = cx.apini.dir(crate::apini::Dir::System).to_owned();
        let mut p = Self { text: cx.apini.status_text.clone(), ..Default::default() };
        for (name, slot) in [("status", 0), ("desig", 1)] {
            let lwg = join(&sys, name);
            let Some(layout) = cx.res.lwg_of(&lwg).cloned() else { continue };
            for l in &layout.layers {
                if let Some((c, at)) = cx.lwg_layer(&lwg, &l.name) {
                    let s = Sprite { content: Some(c), pos: at, visible: true, ..Default::default() };
                    if l.name == "bg" {
                        if slot == 0 { p.status_bg = s } else { p.desig_bg = s }
                    } else {
                        p.images.push((format!("{name}/{}", l.name), s));
                    }
                }
            }
            if slot == 0 { p.status_lwg = Some(lwg) } else { p.desig_lwg = Some(lwg) }
        }
        p
    }
    fn image(&self, page: &str, name: &str) -> Sprite {
        let key = format!("{page}/{name}");
        self.images.iter().find(|(k, _)| *k == key).map(|(_, s)| s.clone()).unwrap_or_default()
    }
    pub fn show(&mut self, tab: usize) {
        self.tab = tab;
    }
    /// sub_407500 / sub_407250 from the current variables.
    pub fn refresh(&mut self, mem: &Memory) {
        let var = |i: usize| i32::from(mem.vars[i]);
        self.columns.clear();
        self.marks.clear();
        if self.tab == 1 {
            let Some(t) = self.text.clone() else { return };
            for k in 0..3 {
                let state = var(5015 + 10 * k);
                let image = self.image("status", &format!("c_{k}_{state}"));
                let mut upper = wsprintf(&t.name_line, &[Arg::Text(&t.names[k])]);
                let state_name = t.states.get(state.clamp(0, 3) as usize).cloned().unwrap_or_default();
                upper.extend(wsprintf(&t.state_line, &[Arg::Text(&state_name)]));
                for j in 0..3 {
                    upper.extend(wsprintf(&t.value_line, &[Arg::Text(&t.values[j]), Arg::Int(var(5011 + 10 * k + j))]));
                }
                let stamina = var(5014 + 10 * k);
                let line = if stamina < 11 { &t.stamina_lines[0] } else if stamina < 31 { &t.stamina_lines[1] } else { &t.stamina_lines[2] };
                upper.extend(wsprintf(line, &[Arg::Int(stamina)]));
                let base = 5161 + 100 * k;
                let mut lower = wsprintf(&t.count_line, &[Arg::Int(var(5010 + 10 * k))]);
                lower.extend(wsprintf(&t.counter_lines[0], &[Arg::Int(var(base)), Arg::Int(var(base + 3)), Arg::Int(var(base + 11))]));
                lower.extend(wsprintf(&t.counter_lines[1], &[Arg::Int(var(base + 1)), Arg::Int(var(base + 4)), Arg::Int(var(base + 12))]));
                let sum = var(base + 20) + var(base + 21) + var(base + 22);
                lower.extend(wsprintf(&t.counter_lines[2], &[Arg::Int(var(base + 2)), Arg::Int(var(base + 10)), Arg::Int(sum)]));
                self.columns.push(Column { image, upper, lower, upper_at: pt(UPPER[k].0, UPPER[k].1), lower_at: pt(LOWER[k].0, LOWER[k].1) });
            }
        } else if self.tab == 2 {
            for n in 1..=9 {
                if var(7100 + n) != 0 {
                    self.marks.push((self.image("desig", &format!("e_{n}")), self.image("desig", &format!("p_{n}"))));
                } else {
                    self.marks.push((self.image("desig", &format!("d_{n}")), Sprite::default()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wsprintf_subset() {
        let out = wsprintf(b"%s%\x81\x40|%4d|%2d/x|%3d", &[Arg::Text(b"ab"), Arg::Int(7), Arg::Int(12), Arg::Int(5)]);
        assert_eq!(out, b"ab\x81\x40|   7|12/x|  5".to_vec());
    }
}
