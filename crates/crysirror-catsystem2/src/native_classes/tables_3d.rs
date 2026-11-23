// SPDX-License-Identifier: MPL-2.0
//! Section tables, natives 683..685 (787F50 family), read by the 3D view from
//! `config.int/3DTable.txt`: `#section` lines followed by `key value` lines.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[594, 595, 596, 597, 683, 684, 685];

type Table = BTreeMap<Vec<u8>, Vec<(Vec<u8>, Vec<u8>)>>;

/// 78CE10: tone name → RGB percent, image name → tone name; names without case.
#[derive(Clone, Debug, Default, PartialEq)]
struct ToneTable {
    tones: BTreeMap<Vec<u8>, [u32; 3]>,
    images: BTreeMap<Vec<u8>, Vec<u8>>,
}

/// 78D750: spaces go; `#name(r,g,b)` opens a tone, any other line is an image in the
/// current one. Before the first, the tone is `__UNKNOWN__` at 100%.
fn parse_tones(bytes: &[u8]) -> ToneTable {
    let mut table = ToneTable::default();
    let mut current = (b"__unknown__".to_vec(), [100u32; 3]);
    let lines: Vec<Vec<u8>> = bytes.split(|b| matches!(*b, b'\r' | b'\n' | 0)).map(<[u8]>::to_vec).collect();
    for line in strip_comments(lines) {
        let line: Vec<u8> = line.into_iter().filter(|b| !matches!(*b, b' ' | b'\t')).collect();
        if line.is_empty() {
            continue;
        }
        if let Some(at) = line.iter().position(|b| *b == b'#') {
            let rest = &line[at + 1..];
            if let Some(open) = rest.iter().position(|b| *b == b'(') {
                let inner = &rest[open + 1..];
                let inner = &inner[..inner.iter().position(|b| *b == b')').unwrap_or(inner.len())];
                let mut words = inner.split(|b| *b == b',').map(|w| std::str::from_utf8(w).ok().and_then(|t| t.parse::<i32>().ok()).unwrap_or(0) as u32);
                current = (rest[..open].to_ascii_lowercase(), [words.next().unwrap_or(0), words.next().unwrap_or(0), words.next().unwrap_or(0)]);
                continue;
            }
        }
        table.tones.entry(current.0.clone()).or_insert(current.1);
        table.images.insert(line.to_ascii_lowercase(), current.0.clone());
    }
    table
}

#[derive(Default)]
pub(crate) struct State {
    tones: BTreeMap<u32, ToneTable>,
    tables: BTreeMap<u32, Table>,
    load: Option<(u32, crysirror_tasks::Task<Option<Vec<u8>>>)>,
}

/// 7876B0: remove `/* */` spans, which may cross lines, and `//` line tails.
fn strip_comments(lines: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut inside = false;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let mut kept = Vec::with_capacity(line.len());
        let mut at = 0;
        while at < line.len() {
            if inside {
                if line[at..].starts_with(b"*/") {
                    inside = false;
                    at += 2;
                } else {
                    at += 1;
                }
            } else if line[at..].starts_with(b"/*") {
                inside = true;
                at += 2;
            } else if line[at..].starts_with(b"//") {
                break;
            } else {
                kept.push(line[at]);
                at += 1;
            }
        }
        out.push(kept);
    }
    out
}

/// 787170 + 7877C0 + 787F50: lines end at CR, LF or CRLF; tabs read as spaces and
/// outer spaces go. A `#` line opens (or reopens) its section, full text including
/// `#`. Other lines of an open section are `key value`, split by 788570 at the first
/// space with further spaces skipped.
fn parse(bytes: &[u8]) -> Table {
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut at = 0;
    while at < bytes.len() && bytes[at] != 0 {
        match bytes[at] {
            b'\r' | b'\n' => {
                if bytes[at] == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
                    at += 1;
                }
                lines.push(core::mem::take(&mut line));
            }
            b'\t' => line.push(b' '),
            byte => line.push(byte),
        }
        at += 1;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    let mut table = Table::new();
    let mut section: Option<Vec<u8>> = None;
    for line in strip_comments(lines) {
        let start = line.iter().position(|b| *b != b' ').unwrap_or(line.len());
        let end = line.iter().rposition(|b| *b != b' ').map_or(start, |p| p + 1);
        let line = &line[start..end];
        if line.is_empty() {
            continue;
        }
        if line[0] == b'#' {
            table.entry(line.to_vec()).or_default();
            section = Some(line.to_vec());
        } else if let Some(name) = &section {
            let split = line.iter().position(|b| *b == b' ').unwrap_or(line.len());
            let value = line[split..].iter().position(|b| *b != b' ').map_or(&[][..], |p| &line[split + p..]);
            table.get_mut(name).expect("open section").push((line[..split].to_vec(), value.to_vec()));
        }
    }
    table
}

/// 788330: the `index`th record with this key in the exact section.
fn cell<'a>(table: &'a Table, section: &[u8], key: &[u8], index: u32) -> Option<&'a [u8]> {
    table.get(section)?.iter().filter(|(k, _)| k == key).nth(index as usize).map(|(_, v)| v.as_slice())
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 64D870: a missing file answers 0; its readers then answer 0.
        594 => {
            ensure!(a.len() == 1, "CatSystem2 tone table load arguments");
            cx.machine.preflight_native(true, &[])?;
            if cx.state.tables_3d.load.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[0])?).context("CatSystem2 tone table name")?;
                let storage = cx.host.storage(cx.client)?;
                let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                    Ok(permit) => permit,
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                };
                let task = permit.spawn(move |_| {
                    let startup = crate::native_settings::optional_asset(storage.as_ref(), "config/startup.xml")?.context("CatSystem2 startup missing for tone table")?;
                    let startup = crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup, encoding_rs::SHIFT_JIS)?;
                    crate::native_settings::optional_native_asset(storage.as_ref(), &startup, &name)
                })?;
                cx.state.tables_3d.load = Some((cx.call.pc, task));
                return Ok(false);
            }
            let Some(bytes) = cx.state.tables_3d.load.as_mut().unwrap().1.try_take()? else { return Ok(false) };
            cx.state.tables_3d.load = None;
            let id = match bytes {
                Some(bytes) => {
                    let id = cx.handles.allocate()?;
                    cx.state.tables_3d.tones.insert(id, parse_tones(&bytes));
                    id
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(id))?;
        }
        595 => {
            cx.machine.preflight_native(false, &[])?;
            cx.state.tables_3d.tones.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 64DA30 → 78D3E0 by tone name, 64D9B0 → 78D230 by image name; R, G, B into VM words.
        596 | 597 => {
            ensure!(a.len() == 5, "CatSystem2 tone table query arguments");
            let name = crate::native_values::format_bytes(cx.machine, a[1])?.to_ascii_lowercase();
            let found = cx.state.tables_3d.tones.get(&a[0]).and_then(|table| {
                let tone = if cx.call.id == 596 { Some(&name) } else { table.images.get(&name) };
                tone.and_then(|tone| table.tones.get(tone)).copied()
            });
            match found {
                Some([r, g, b]) => cx.machine.commit_native(Some(1), &[(a[2], r.to_le_bytes().to_vec()), (a[3], g.to_le_bytes().to_vec()), (a[4], b.to_le_bytes().to_vec())])?,
                None => {
                    cx.machine.preflight_native(true, &[])?;
                    cx.machine.acknowledge(Some(0))?;
                }
            }
        }
        // 650850: 5D49A0 kind 19 names `archive/member` directly; a missing file is 0.
        683 => {
            ensure!(a.len() == 1, "CatSystem2 section table load arguments");
            cx.machine.preflight_native(true, &[])?;
            if cx.state.tables_3d.load.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[0])?).context("CatSystem2 section table name")?;
                let storage = cx.host.storage(cx.client)?;
                let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                    Ok(permit) => permit,
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                };
                let task = permit.spawn(move |_| {
                    let startup = crate::native_settings::optional_asset(storage.as_ref(), "config/startup.xml")?.context("CatSystem2 startup missing for section table")?;
                    let startup = crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup, encoding_rs::SHIFT_JIS)?;
                    crate::native_settings::optional_native_asset(storage.as_ref(), &startup, &name)
                })?;
                cx.state.tables_3d.load = Some((cx.call.pc, task));
                return Ok(false);
            }
            let Some(bytes) = cx.state.tables_3d.load.as_mut().unwrap().1.try_take()? else { return Ok(false) };
            cx.state.tables_3d.load = None;
            let id = match bytes {
                Some(bytes) => {
                    let id = cx.handles.allocate()?;
                    cx.state.tables_3d.tables.insert(id, parse(&bytes));
                    id
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(id))?;
        }
        684 => {
            ensure!(a.len() == 1, "CatSystem2 section table free arguments");
            cx.machine.preflight_native(false, &[])?;
            cx.state.tables_3d.tables.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 650980: copies the value, terminated, and answers 1.
        685 => {
            ensure!(a.len() == 5, "CatSystem2 section table query arguments");
            let section = crate::native_values::format_bytes(cx.machine, a[1])?;
            let key = crate::native_values::format_bytes(cx.machine, a[2])?;
            let value = cx.state.tables_3d.tables.get(&a[0]).and_then(|table| cell(table, &section, &key, a[3])).map(<[u8]>::to_vec);
            match (value, a[4]) {
                (Some(mut value), target) if target != 0 => {
                    value.push(0);
                    cx.machine.commit_native(Some(1), &[(target, value)])?;
                }
                _ => {
                    cx.machine.preflight_native(true, &[])?;
                    cx.machine.acknowledge(Some(0))?;
                }
            }
        }
        id => anyhow::bail!("CatSystem2 section table native {id} is not implemented"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_group_images_and_ignore_spaces() {
        let table = parse_tones(b"// c\n#Night (50, 60,70)\nBG01\n bg02 \n#day(100,100,100)\nbg03\n");
        assert_eq!(table.images[&b"bg02"[..]], b"night".to_vec());
        assert_eq!(table.tones[&b"night"[..]], [50, 60, 70]);
        assert_eq!(table.tones[&b"day"[..]], [100, 100, 100]);
    }

    #[test]
    fn sections_keys_comments_and_occurrences() {
        let table = parse(b"// head\r\n#a\r\nmodel  x.kx2 // tail\n\tmodel y\r/* gone\nstill */f_enable 1\n#b\nmodel z\n");
        assert_eq!(cell(&table, b"#a", b"model", 0), Some(&b"x.kx2"[..]));
        assert_eq!(cell(&table, b"#a", b"model", 1), Some(&b"y"[..]));
        assert_eq!(cell(&table, b"#a", b"f_enable", 0), Some(&b"1"[..]));
        assert_eq!(cell(&table, b"#b", b"model", 0), Some(&b"z"[..]));
        assert_eq!(cell(&table, b"a", b"model", 0), None);
    }
}
