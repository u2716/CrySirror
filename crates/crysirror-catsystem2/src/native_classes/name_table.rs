// SPDX-License-Identifier: MPL-2.0
//! Name tables, natives 633..640 (833560 family): `config.int/nametable.csv` maps a
//! speaker's aliases to a row id and to display strings. Rows come from the tab
//! separated reader 7278E0/728110; a row whose first cell starts with a digit counts.
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[633, 634, 635, 636, 639, 640];

/// 0x310-byte row: id, aliases (cell 1 then cells 4..), +0x110 cell 2, +0x210 cell 3.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Row {
    id: u32,
    aliases: Vec<Vec<u8>>,
    prefix: Vec<u8>,
    format: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct State {
    tables: BTreeMap<u32, Vec<Row>>,
    load: Option<(u32, crysirror_tasks::Task<Option<Vec<u8>>>)>,
}

/// 728110 with separator TAB: `"` opens a quoted cell in which `""` is a quote and
/// line ends stay; an unquoted cell ends at TAB, CR or LF.
fn records(bytes: &[u8]) -> Vec<Vec<Vec<u8>>> {
    let mut rows = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let mut cells = Vec::new();
        while at < bytes.len() && !matches!(bytes[at], b'\r' | b'\n') {
            let mut cell = Vec::new();
            let quoted = bytes[at] == b'"';
            if quoted {
                at += 1;
            }
            while at < bytes.len() && bytes[at] != 0 {
                let byte = bytes[at];
                if quoted && byte == b'"' {
                    if bytes.get(at + 1) == Some(&b'"') {
                        cell.push(b'"');
                        at += 2;
                        continue;
                    }
                    at += 1;
                    break;
                }
                if !quoted && matches!(byte, b'\t' | b'\r' | b'\n') {
                    break;
                }
                if matches!(byte, 0x81..=0x9f | 0xe0..=0xfc) && at + 1 < bytes.len() {
                    cell.extend_from_slice(&bytes[at..at + 2]);
                    at += 2;
                    continue;
                }
                cell.push(byte);
                at += 1;
            }
            cells.push(cell);
            if bytes.get(at) == Some(&b'\t') {
                at += 1;
            }
            if bytes.get(at) == Some(&0) {
                at = bytes.len();
            }
        }
        if bytes.get(at) == Some(&b'\r') && bytes.get(at + 1) == Some(&b'\n') {
            at += 1;
        }
        at += 1;
        rows.push(cells);
    }
    rows
}

/// 8335E0: a line needs a digit first and a cell 1 under 256 bytes.
fn parse(bytes: &[u8]) -> Vec<Row> {
    let cut = |cell: &Vec<u8>| if cell.len() < 256 { cell.clone() } else { Vec::new() };
    records(bytes)
        .into_iter()
        .filter_map(|cells| {
            let first = cells.first()?;
            if !first.first()?.is_ascii_digit() {
                return None;
            }
            let id = crate::native_values::integer(&String::from_utf8_lossy(first)) as u32;
            let name = cells.get(1)?;
            if name.len() >= 256 {
                return None;
            }
            let mut aliases = vec![name.clone()];
            aliases.extend(cells.iter().skip(4).cloned());
            Some(Row { id, aliases, prefix: cells.get(2).map_or(Vec::new(), cut), format: cells.get(3).map_or(Vec::new(), cut) })
        })
        .collect()
}

/// 64E250: expand the row's format over the input name, then keep what followed a comma.
fn expand(input: &[u8], format: &[u8], prefix: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut cursor = 0;
    while at < format.len() {
        let byte = format[at];
        if byte != b'%' {
            out.push(byte);
            cursor += 1;
            at += 1;
            continue;
        }
        match format.get(at + 1).copied().unwrap_or(0) {
            b'f' => out.extend_from_slice(prefix),
            b'c' => {
                out.push(input.get(cursor).copied().unwrap_or(0));
                cursor += 1;
            }
            b'n' => {
                let start = input.iter().position(|b| b.is_ascii_digit() || *b == b',').unwrap_or(input.len());
                let digits = input[start..].iter().take_while(|b| b.is_ascii_digit()).copied().collect::<Vec<_>>();
                out.extend_from_slice(&digits);
            }
            b'r' => out.extend_from_slice(input.get(cursor..).unwrap_or(&[])),
            b's' => out.extend_from_slice(input),
            // Any other letter leaves the percent sign and is then copied as text.
            _ => {
                out.push(b'%');
                cursor += 1;
                at += 1;
                continue;
            }
        }
        at += 2;
    }
    let mut index = 0usize;
    while let Some(&byte) = input.get(index) {
        if byte as i8 <= 0 {
            break;
        }
        if byte == b',' {
            out.extend_from_slice(&input[index..]);
            break;
        }
        index += 1;
        if index >= input.get(index).map_or(0, |b| *b as i8 as i32).max(0) as usize {
            break;
        }
    }
    out
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 name table native {} arguments", cx.call.id);
    match cx.call.id {
        // 64DE90: 5D3C20 and 5D5330 read the file; a missing file answers 0.
        633 => {
            cx.machine.preflight_native(true, &[])?;
            if cx.state.name_table.load.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[0])?).context("CatSystem2 name table path")?;
                let storage = cx.host.storage(cx.client)?;
                let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                    Ok(permit) => permit,
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                };
                let task = permit.spawn(move |cancel| {
                    let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
                    for path in catalog.candidates(storage.as_ref(), "fes", &name, &|| cancel.check())? {
                        cancel.check()?;
                        if let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? {
                            return Ok(Some(crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 16 * 1024 * 1024)?));
                        }
                    }
                    Ok(None)
                })?;
                cx.state.name_table.load = Some((cx.call.pc, task));
                return Ok(false);
            }
            let Some(bytes) = cx.state.name_table.load.as_mut().unwrap().1.try_take()? else { return Ok(false) };
            cx.state.name_table.load = None;
            let id = match bytes {
                Some(bytes) => {
                    let id = cx.handles.allocate()?;
                    cx.state.name_table.tables.insert(id, parse(&bytes));
                    id
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(id))?;
        }
        634 => {
            cx.machine.preflight_native(false, &[])?;
            cx.state.name_table.tables.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 64E200 → 833B70: first row with an alias equal to the name, else -1.
        635 => {
            cx.machine.preflight_native(true, &[])?;
            let name = crate::native_values::format_bytes(cx.machine, a[1])?;
            let found = cx.state.name_table.tables.get(&a[0]).and_then(|rows| rows.iter().position(|row| row.aliases.contains(&name)));
            cx.machine.acknowledge(Some(found.map_or(u32::MAX, |index| index as u32)))?;
        }
        // 64E190 → 833AB0: row id, else -1.
        636 => {
            cx.machine.preflight_native(true, &[])?;
            let id = cx.state.name_table.tables.get(&a[0]).and_then(|rows| rows.get(a[1] as usize)).map_or(u32::MAX, |row| row.id);
            cx.machine.acknowledge(Some(id))?;
        }
        // 64DFF0: the VM name, cut to the length of cell 2, equals it (6A6BB0 ignores case).
        639 => {
            cx.machine.preflight_native(true, &[])?;
            let row = cx.state.name_table.tables.get(&a[0]).and_then(|rows| rows.get(a[1] as usize)).cloned();
            let matched = match row {
                Some(row) => {
                    let name = cx.machine.read_cstring(a[2], 65534)?;
                    name.len() >= row.prefix.len() && name[..row.prefix.len()].eq_ignore_ascii_case(&row.prefix)
                }
                None => false,
            };
            cx.machine.acknowledge(Some(u32::from(matched)))?;
        }
        // 64E0C0: expanded name into a VM buffer, answering 1.
        640 => {
            let row = cx.state.name_table.tables.get(&a[0]).and_then(|rows| rows.get(a[1] as usize)).cloned();
            match row {
                Some(row) => {
                    let input = cx.machine.read_cstring(a[2], 65534)?;
                    let mut out = expand(&input, &row.format, &row.prefix);
                    out.push(0);
                    cx.machine.commit_native(Some(1), &[(a[3], out)])?;
                }
                None => {
                    cx.machine.preflight_native(true, &[])?;
                    cx.machine.acknowledge(Some(0))?;
                }
            }
        }
        id => bail!("CatSystem2 native {id} has no handler in the name table family"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_need_a_leading_digit_and_collect_aliases() {
        let rows = parse(b"// comment\r\n1\tname\tpre\tfmt\talias\r\n\r\nx\ty\r\n2\t\"a\"\"b\"\r\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].aliases, vec![b"name".to_vec(), b"alias".to_vec()]);
        assert_eq!(rows[0].prefix, b"pre");
        assert_eq!(rows[1].aliases[0], b"a\"b");
    }
    #[test]
    fn formats_expand_percent_codes() {
        assert_eq!(expand(b"ab12", b"%f-%s", b"P"), b"P-ab12");
        assert_eq!(expand(b"ab12", b"%n", b""), b"12");
        assert_eq!(expand(b"ab", b"%x", b""), b"%x");
    }
}
