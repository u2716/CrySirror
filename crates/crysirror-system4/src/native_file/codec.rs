// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Native File.dll10002210/10002498 and array helper chains
// were read in full; xsystem4 File.c supplies additional type descriptions.
//! Native typed pages only. Shared storage and compression own all I/O.
use super::*;
use crate::vm::heap::{Object, PageKind, element_kind};
use std::collections::BTreeSet;
pub(super) fn transfer(
    machine: &mut Machine,
    slot: i32,
    stream: &mut Stream,
    reading: bool,
    check: &dyn Fn() -> Result<()>,
) -> Result<bool> {
    let mut budget = 8_000_000usize;
    walk(
        machine,
        slot,
        stream,
        reading,
        &mut budget,
        &mut BTreeSet::new(),
        0,
        check,
    )
}
fn walk(
    m: &mut Machine,
    slot: i32,
    s: &mut Stream,
    reading: bool,
    budget: &mut usize,
    path: &mut BTreeSet<i32>,
    depth: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<bool> {
    check()?;
    ensure!(depth <= 256, "System4 File page depth budget");
    if slot == -1 || m.heap.page_opt(slot)?.is_none() {
        return Ok(true);
    }
    ensure!(path.insert(slot), "System4 File cyclic owned page");
    let page = m.heap.page(slot)?;
    ensure!(page.values.len() <= *budget, "System4 File value budget");
    *budget -= page.values.len();
    let kinds = match page.kind {
        PageKind::Structure(index) => {
            let fields = &m
                .program
                .structures
                .get(index)
                .context("System4 File structure index")?
                .members;
            ensure!(
                fields.len() == page.values.len(),
                "System4 File structure extent"
            );
            fields.iter().map(|v| v.kind).collect::<Vec<_>>()
        }
        PageKind::Array { kind, rank, .. } => {
            vec![if rank > 1 { kind } else { element_kind(kind)? }; page.values.len()]
        }
        _ => anyhow::bail!("System4 File expects a structure or array page"),
    };
    let values = page.values.clone();
    for (index, (kind, bits)) in kinds.into_iter().zip(values).enumerate() {
        check()?;
        if matches!(kind, 13 | 14..=17 | 50) {
            if !walk(m, bits, s, reading, budget, path, depth + 1, check)? {
                path.remove(&slot);
                return Ok(false);
            }
        } else if matches!(kind, 10 | 11 | 47) {
            if reading {
                let Some(data) = s.bytes.get(s.cursor..s.cursor.saturating_add(4)) else {
                    path.remove(&slot);
                    return Ok(false);
                };
                let value = i32::from_le_bytes(data.try_into().unwrap());
                m.heap.set(slot, index as i32, value)?;
                s.cursor += 4;
            } else {
                append(s, &bits.to_le_bytes())?;
            }
        } else if kind == 12 {
            if reading {
                let Some(rest) = s.bytes.get(s.cursor..) else {
                    path.remove(&slot);
                    return Ok(false);
                };
                let Some(end) = rest.iter().position(|byte| *byte == 0) else {
                    path.remove(&slot);
                    return Ok(false);
                };
                let value = Arc::new(rest[..end].to_vec());
                m.release_string_now(bits)?;
                let value = m.heap.alloc(Object::String(value))?;
                m.heap.set(slot, index as i32, value)?;
                s.cursor += end + 1;
            } else {
                let value = m.heap.string(bits)?;
                let value = value.split(|b| *b == 0).next().unwrap();
                append(s, value)?;
                append(s, &[0])?;
            }
        }
        // The reviewed DLL explicitly skips other typed fields without
        // advancing the byte cursor; do not import later long/function types.
    }
    path.remove(&slot);
    Ok(true)
}
fn append(stream: &mut Stream, bytes: &[u8]) -> Result<()> {
    let end = stream
        .cursor
        .checked_add(bytes.len())
        .context("System4 File byte overflow")?;
    ensure!(end <= LIMIT, "System4 File byte budget");
    let target = Arc::make_mut(&mut stream.bytes);
    if target.len() < end {
        target.resize(end, 0);
    }
    target[stream.cursor..end].copy_from_slice(bytes);
    stream.cursor = end;
    Ok(())
}
