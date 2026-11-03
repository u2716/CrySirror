// Copyright (c) 2026 CrySirror contributors.
// Distributed under the GNU Affero General Public License, version 3 or
// (at your option) any later version, https://www.gnu.org/licenses/agpl-3.0.html.
//! KiriKiri executable-embedded options (`XOPT_EMBED_AREA_`): the release
//! configuration a shipped krkr2 executable carries, e.g. `debugwin="no"`,
//! which `System.getArgument("-debugwin")` reports. The area is the marker,
//! a 32-bit capacity, then `name=value` lines up to a NUL; values are TJS
//! string literals. What follows the NUL (the configurable option names) is
//! not an option and is ignored. Only data is read; the executable never runs.
use anyhow::{Context, Result, ensure};

const MARKER: &[u8] = b"XOPT_EMBED_AREA_";
/// The area is fixed-size in the executable; anything longer is not an area.
const MAX_AREA: usize = 64 * 1024;
const MAX_OPTIONS: usize = 256;

/// `(name, value)` pairs, names without the leading `-`.
pub fn parse(exe: &[u8]) -> Result<Vec<(String, String)>> {
    let Some(at) = exe.windows(MARKER.len()).position(|w| w == MARKER) else {
        return Ok(Vec::new());
    };
    let body = exe
        .get(at + MARKER.len() + 4..)
        .context("truncated embedded option area")?;
    let text = &body[..body.len().min(MAX_AREA)];
    let end = text.iter().position(|b| *b == 0).context("unterminated embedded option area")?;
    let text = std::str::from_utf8(&text[..end]).context("embedded options are not text")?;
    let mut out = Vec::new();
    for line in text.split(['\n', '\r']).map(str::trim).filter(|l| !l.is_empty()) {
        ensure!(out.len() < MAX_OPTIONS, "embedded option limit");
        let (name, value) = line.split_once('=').unwrap_or((line, "yes"));
        out.push((name.trim().trim_start_matches('-').to_owned(), unquote(value.trim())?));
    }
    Ok(out)
}

/// `System.exeBits` of the original process: krkrz reports the pointer width of
/// the running executable, so it follows the PE machine and never the host. Yuzusoft
/// and other scripts branch on it to select plugins and the motion title path.
pub fn pe_bits(exe: &[u8]) -> Option<i64> {
    let lfanew = u32::from_le_bytes(exe.get(0x3c..0x40)?.try_into().ok()?) as usize;
    if exe.get(..2)? != b"MZ" || exe.get(lfanew..lfanew.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes(exe.get(lfanew + 4..lfanew + 6)?.try_into().ok()?) {
        0x014c => Some(32),
        0x8664 | 0xaa64 => Some(64),
        _ => None,
    }
}

/// A TJS string literal (`"..."` or `'...'` with backslash escapes), or a
/// bare value as written.
fn unquote(v: &str) -> Result<String> {
    let Some(q) = v.chars().next().filter(|c| *c == '"' || *c == '\'') else {
        return Ok(v.to_owned());
    };
    ensure!(v.len() >= 2 && v.ends_with(q), "unterminated embedded option string");
    let inner = &v[1..v.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next().context("dangling escape in embedded option")? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'x' | 'X' => {
                let mut code = 0u32;
                let mut digits = 0;
                while digits < 4 && let Some(d) = chars.peek().and_then(|c| c.to_digit(16)) {
                    code = code * 16 + d;
                    chars.next();
                    digits += 1;
                }
                ensure!(digits > 0, "empty hex escape in embedded option");
                out.push(char::from_u32(code).context("invalid escaped character")?);
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_quoted_and_escaped_values_before_the_name_list() {
        let mut exe = b"MZ....".to_vec();
        exe.extend(MARKER);
        exe.extend(0x0fecu32.to_le_bytes());
        exe.extend(b"debugwin=\"\\x6E\\x6F\"\nhkupdaterect=\"\"\n\0forcelog,debugwin\0\0\0");
        assert_eq!(
            parse(&exe).unwrap(),
            [("debugwin".into(), "no".into()), ("hkupdaterect".into(), String::new())]
        );
    }

    #[test]
    fn exe_bits_follow_the_pe_machine() {
        let pe = |machine: u16| {
            let mut exe = vec![0; 0x80];
            exe[..2].copy_from_slice(b"MZ");
            exe[0x3c] = 0x40;
            exe[0x40..0x44].copy_from_slice(b"PE\0\0");
            exe[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
            exe
        };
        assert_eq!(pe_bits(&pe(0x014c)), Some(32));
        assert_eq!(pe_bits(&pe(0x8664)), Some(64));
        assert_eq!(pe_bits(&pe(0x01c0)), None);
        assert_eq!(pe_bits(b"MZ truncated"), None);
    }

    #[test]
    fn executables_without_an_area_have_no_options() {
        assert!(parse(b"MZ no options here").unwrap().is_empty());
    }
}
