// SPDX-License-Identifier: MPL-2.0
//! Script source files (67E7D0). The extension is replaced in turn by
//! `ms` (UTF8_I, 693580), `mls` (MLS 693380: "MalieScenario" + zlib, read
//! through MBC_I) and `txt` (MBC_I, 694090: code page 932), then the name
//! is tried as given (MBC_I). Decoders reproduce the original's character
//! values, including its lenient UTF-8 and EOF (-1) handling.
use super::lex::{file_start, text, W};
use anyhow::Result;
use std::io::Read;

/// Bounded byte access to the game's files; `None` when absent.
pub trait Source {
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>>;
}

/// Characters of the first file found, or `None`.
pub fn open(source: &mut dyn Source, name: &[u16]) -> Result<Option<Vec<i64>>> {
    // 6900B0: after the last '.' of the file name component.
    let start = file_start(name);
    let stem: W = match name[start..].iter().rposition(|&c| c == 46) {
        Some(p) => name[..start + p + 1].to_vec(),
        None => {
            let mut v = name.to_vec();
            v.push(46);
            v
        }
    };
    let with = |ext: &str| text(&stem) + ext;
    if let Some(b) = source.read(&normalize(&with("ms")))? {
        return Ok(Some(utf8(&b)));
    }
    if let Some(b) = source.read(&normalize(&with("mls")))? {
        if b.len() >= 13 && &b[..13] == b"MalieScenario" {
            let mut out = Vec::new();
            // ZLibIn_Full stops the stream on an inflate error.
            let _ = flate2::read::ZlibDecoder::new(&b[13..]).take(64 * 1024 * 1024).read_to_end(&mut out);
            return Ok(Some(cp932(&out)));
        }
    }
    if let Some(b) = source.read(&normalize(&with("txt")))? {
        return Ok(Some(cp932(&b)));
    }
    Ok(source.read(&normalize(&text(name)))?.map(|b| cp932(&b)))
}

/// Windows path text to an asset-relative path.
fn normalize(p: &str) -> String {
    let p = p.replace('\\', "/");
    let mut p = p.as_str();
    while let Some(r) = p.strip_prefix("./") {
        p = r;
    }
    p.trim_start_matches('/').to_owned()
}

struct Bytes<'a> {
    b: &'a [u8],
    at: usize,
}
impl Bytes<'_> {
    fn get(&mut self) -> i64 {
        let v = self.b.get(self.at).map_or(-1, |&v| i64::from(v));
        self.at += 1;
        v
    }
    fn done(&self) -> bool {
        self.at >= self.b.len()
    }
}

/// 693580.
fn utf8(b: &[u8]) -> Vec<i64> {
    let mut r = Bytes { b, at: 0 };
    let mut out = Vec::new();
    while !r.done() {
        let c = r.get();
        let v = if c < 0x80 {
            c
        } else if c < 0xE0 {
            (r.get() & 0x3F) + ((c & 0x1F) << 6)
        } else if c < 0xF0 {
            let b1 = r.get();
            (r.get() & 0x3F) + (((b1 & 0x3F) + ((c & 0xF) << 6)) << 6)
        } else if c < 0xF8 {
            let b1 = r.get();
            let b2 = r.get();
            (r.get() & 0x3F) + (((b2 & 0x3F) + (((b1 & 0x3F) + ((c & 7) << 6)) << 6)) << 6)
        } else if c >= 0xFE {
            c
        } else {
            let (b6, b7, b8, b9, top) = if c >= 0xFC {
                let b11 = r.get();
                let (b6, b7, b8, b9) = (r.get(), r.get(), r.get(), r.get());
                (b6, b7, b8, b9, (b11 & 0x3F) + ((c & 1) << 6))
            } else {
                let (b6, b7, b8, b9) = (r.get(), r.get(), r.get(), r.get());
                (b6, b7, b8, b9, c & 2)
            };
            (b9 & 0x3F) + (((b8 & 0x3F) + (((b7 & 0x3F) + (((b6 & 0x3F) + (top << 6)) << 6)) << 6)) << 6)
        };
        out.push(v);
    }
    out
}

/// 694090: CP932 one character at a time; an unconvertible sequence yields
/// 0 (MultiByteToWideChar leaves the zeroed buffer).
fn cp932(b: &[u8]) -> Vec<i64> {
    let mut r = Bytes { b, at: 0 };
    let mut out = Vec::new();
    while !r.done() {
        let c = r.get();
        let lead = (0x81..=0x9F).contains(&c) || (0xE0..=0xFC).contains(&c);
        let bytes: Vec<u8> = if lead {
            vec![c as u8, r.get() as u8]
        } else if c < 0x80 {
            out.push(c);
            continue;
        } else {
            vec![c as u8]
        };
        let (s, _, bad) = encoding_rs::SHIFT_JIS.decode(&bytes);
        let mut units = s.encode_utf16();
        out.push(if bad { 0 } else { units.next().map_or(0, i64::from) });
    }
    out
}
