// SPDX-License-Identifier: MPL-2.0
//! Per-scene state records, natives 991/992/994 (5B0B10 family): the local flag
//! ranges and the strings named by `config/localstr.xml` are captured, deflated and
//! chained onto a history entry's data so the backlog can jump back into a scene.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;
use std::io::{Read, Write};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[991, 992, 994];

#[derive(Default)]
pub(crate) struct State {
    /// `localstr` string ids per config name.
    lists: BTreeMap<Vec<u8>, Vec<u32>>,
    load: Option<(u32, crysirror_tasks::Task<Vec<u32>>)>,
}

/// 5B0B10: `localstr` is a comma list of decimal ids; a missing file lists nothing.
fn read_list(storage: &dyn crysirror_core::service::ClientStorage, name: &str) -> Result<Vec<u32>> {
    let startup = crate::native_settings::optional_asset(storage, "config/startup.xml")?.context("CatSystem2 startup missing for scene records")?;
    let startup = crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup, encoding_rs::SHIFT_JIS)?;
    let bytes = match crate::native_settings::optional_asset(storage, &format!("config/{name}"))? {
        Some(bytes) => Some(bytes),
        None => crate::native_settings::optional_native_asset(storage, &startup, &format!("config.int/{name}"))?,
    };
    let Some(bytes) = bytes else { return Ok(Vec::new()) };
    let document = crysirror_resource::xml::XmlDocument::parse_with_fallback(&bytes, encoding_rs::SHIFT_JIS)?;
    let Some(node) = document.unique_child_path("localstr")? else { return Ok(Vec::new()) };
    Ok(node.text.split(',').filter(|t| !t.trim().is_empty()).map(|t| crate::native_values::integer(t.trim())).collect())
}

/// 5B0B10's second half: every local range, in table order.
fn tracked_flags(cx: &Ctx) -> Vec<u32> {
    cx.settings.as_ref().map_or(Vec::new(), |settings| {
        settings.flag_ranges[0].iter().flat_map(|[start, count]| *start..start.saturating_add(*count)).collect()
    })
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("CatSystem2 scene record header")?.try_into().unwrap()))
}

/// 5B0EC0: { next, packed end, flag bytes, string bytes, time, zlib(flags, strings) }, then
/// the entry's previous chain.
fn encode(flags: &[(u32, u32)], strings: &[u8], time: u32, previous: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut plain = Vec::with_capacity(flags.len() * 8 + strings.len());
    for (id, value) in flags {
        plain.extend(id.to_le_bytes());
        plain.extend(value.to_le_bytes());
    }
    plain.extend(strings);
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    zlib.write_all(&plain)?;
    let packed = zlib.finish()?;
    let end = packed.len() as u32 + 20;
    let mut out = Vec::with_capacity(packed.len() + 20 + previous.map_or(0, <[u8]>::len));
    out.extend((if previous.is_some() { end } else { 0 }).to_le_bytes());
    out.extend(end.to_le_bytes());
    out.extend(((flags.len() * 8) as u32).to_le_bytes());
    out.extend((strings.len() as u32).to_le_bytes());
    out.extend(time.to_le_bytes());
    out.extend(packed);
    if let Some(previous) = previous {
        out.extend(previous);
    }
    Ok(out)
}

struct Record {
    time: u32,
    flags: Vec<(u32, u32)>,
    strings: Vec<u8>,
}

/// 5B10C0's walk: the first record of the chain at or before `limit`.
fn select(blob: &[u8], limit: Option<u32>) -> Result<Option<Record>> {
    let mut at = 0;
    loop {
        let next = word(blob, at)? as usize;
        let time = word(blob, at + 16)?;
        if limit.is_none_or(|limit| time <= limit) {
            let end = word(blob, at + 4)? as usize;
            let flag_bytes = word(blob, at + 8)? as usize;
            let string_bytes = word(blob, at + 12)? as usize;
            let packed = blob.get(at + 20..at + end).context("CatSystem2 scene record data")?;
            let mut plain = Vec::with_capacity(flag_bytes + string_bytes);
            flate2::read::ZlibDecoder::new(packed).read_to_end(&mut plain)?;
            ensure!(plain.len() == flag_bytes + string_bytes, "CatSystem2 scene record size");
            let flags = plain[..flag_bytes].chunks_exact(8).map(|c| (u32::from_le_bytes(c[..4].try_into().unwrap()), u32::from_le_bytes(c[4..].try_into().unwrap()))).collect();
            return Ok(Some(Record { time, flags, strings: plain[flag_bytes..].to_vec() }));
        }
        if next == 0 {
            return Ok(None);
        }
        at += next;
    }
}

/// 5B12A0's walk: offset of the first record older than `time`.
fn first_older(blob: &[u8], time: u32) -> Result<Option<usize>> {
    let mut at = 0;
    loop {
        if word(blob, at + 16)? < time {
            return Ok(Some(at));
        }
        let next = word(blob, at)? as usize;
        if next == 0 {
            return Ok(None);
        }
        at += next;
    }
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 642300 → 5B0EC0: prepend a record to the last history entry.
        991 => {
            ensure!(a.len() == 3, "CatSystem2 scene record store arguments");
            cx.machine.preflight_native(false, &[])?;
            if !cx.handles.scenes.contains_key(&a[0]) {
                cx.machine.acknowledge(None)?;
                return Ok(true);
            }
            let Some(strings) = list_for(cx, a[2])? else { return Ok(false) };
            let mut bank = crate::native_values::StringVariables::default();
            bank.mark_created();
            for id in &strings {
                if let Some(text) = cx.strings.get(*id) {
                    bank.assign(*id, text.to_vec())?;
                }
            }
            let flags: Vec<(u32, u32)> = tracked_flags(cx).into_iter().map(|id| (id, cx.interpreter.as_ref().map_or(0, |i| i.flag(id)))).collect();
            let scene = cx.handles.scenes.get_mut(&a[0]).unwrap();
            if let Some(entry) = scene.history.last_mut() {
                let packed = encode(&flags, &bank.serialize().unwrap_or_default(), a[1], entry.blob.as_deref())?;
                entry.blob = Some(packed);
            }
            cx.machine.acknowledge(None)?;
        }
        // 641F60 → 5B10C0: search back from *a1 for a record at or before *a2 in that
        // entry, restore it, and write where it was found.
        992 => {
            ensure!(a.len() == 4, "CatSystem2 scene record restore arguments");
            let index = cx.machine.read_word(a[1])? as i32;
            let limit = cx.machine.read_word(a[2])?;
            if index < 0 || !cx.handles.scenes.contains_key(&a[0]) {
                cx.machine.preflight_native(true, &[])?;
                cx.machine.acknowledge(Some(0))?;
                return Ok(true);
            }
            let Some(strings) = list_for(cx, a[3])? else { return Ok(false) };
            let scene = &cx.handles.scenes[&a[0]];
            let mut found = None;
            let mut at = index;
            while at >= 0 {
                if let Some(blob) = scene.history.get(at as usize).and_then(|entry| entry.blob.as_deref()).filter(|blob| !blob.is_empty()) {
                    if let Some(record) = select(blob, (at == index).then_some(limit))? {
                        found = Some((at, record));
                        break;
                    }
                }
                at -= 1;
            }
            let Some((at, record)) = found else {
                cx.machine.preflight_native(true, &[])?;
                cx.machine.acknowledge(Some(0))?;
                return Ok(true);
            };
            if let Some(interpreter) = cx.interpreter.as_mut() {
                for (id, value) in &record.flags {
                    interpreter.set_flag(*id, *value);
                }
            }
            let mut bank = crate::native_values::StringVariables::default();
            bank.deserialize(&record.strings)?;
            for id in &strings {
                match bank.get(*id) {
                    Some(text) => cx.strings.assign(*id, text.to_vec())?,
                    None => cx.strings.remove(*id),
                }
            }
            cx.machine.commit_native(Some(1), &[(a[1], (at as u32).to_le_bytes().to_vec()), (a[2], record.time.to_le_bytes().to_vec())])?;
        }
        // 641940 → 5B12A0, 72DB70: entry a1's records newer than a2 go (the survivors are
        // written to the last entry, as 72D9C0 addresses it), its value becomes a2, and
        // the entries after it are erased.
        994 => {
            ensure!(a.len() == 3, "CatSystem2 scene record rewind arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(scene) = cx.handles.scenes.get_mut(&a[0]) {
                let keep = match scene.history.get(a[1] as usize).and_then(|entry| entry.blob.as_deref()).filter(|blob| !blob.is_empty()) {
                    Some(blob) => first_older(blob, a[2])?.filter(|at| *at > 0).map(|at| blob[at..].to_vec()),
                    None => None,
                };
                if let (Some(keep), Some(last)) = (keep, scene.history.last_mut()) {
                    last.blob = Some(keep);
                }
                if let Some(entry) = scene.history.get_mut(a[1] as usize) {
                    entry.value = a[2];
                }
                scene.history.truncate(a[1] as usize + 1);
            }
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 scene record native {id} is not implemented"),
    }
    Ok(true)
}

/// The string list of a config name, read once on a worker. None while it loads.
fn list_for(cx: &mut Ctx, name: u32) -> Result<Option<Vec<u32>>> {
    let name = crate::native_values::format_bytes(cx.machine, name)?;
    if let Some(list) = cx.state.scene_records.lists.get(&name) {
        return Ok(Some(list.clone()));
    }
    if cx.state.scene_records.load.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
        let text = String::from_utf8(name.clone()).context("CatSystem2 scene record config name")?;
        let storage = cx.host.storage(cx.client)?;
        let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
            Err(e) => return Err(e),
        };
        let task = permit.spawn(move |_| read_list(storage.as_ref(), &text))?;
        cx.state.scene_records.load = Some((cx.call.pc, task));
        return Ok(None);
    }
    let Some(list) = cx.state.scene_records.load.as_mut().unwrap().1.try_take()? else { return Ok(None) };
    cx.state.scene_records.load = None;
    cx.state.scene_records.lists.insert(name, list.clone());
    Ok(Some(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_chain_newest_first_and_select_by_time() {
        let first = encode(&[(1, 10)], &[], 5, None).unwrap();
        let chained = encode(&[(1, 20)], &[], 9, Some(&first)).unwrap();
        assert_eq!(select(&chained, None).unwrap().unwrap().flags, vec![(1, 20)]);
        assert_eq!(select(&chained, Some(6)).unwrap().unwrap().flags, vec![(1, 10)]);
        assert!(select(&chained, Some(4)).unwrap().is_none());
        assert_eq!(first_older(&chained, 7).unwrap().map(|at| word(&chained[at..], 16).unwrap()), Some(5));
    }
}
