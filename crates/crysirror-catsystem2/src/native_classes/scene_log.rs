// SPDX-License-Identifier: MPL-2.0
//! Natives 699/1034 (6510C0, 6515C0): rebuild the message log from the visit history, reading
//! each visited scene again on a worker. Called after a load, so the backlog comes back.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use std::sync::Arc;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[699, 1034];

/// One visited scene's readable lines, voice commands already checked against the disk.
struct Source {
    scene: Vec<u8>,
    history_index: usize,
    inputs: Vec<(Vec<u8>, Vec<u8>, String, bool)>,
}

#[derive(Default)]
pub(crate) struct State {
    job: Option<(u32, crysirror_tasks::Task<Vec<Source>>)>,
}

/// 6A56F0 with 8D4724 ("＠" in Shift-JIS): a name line shows what follows the mark.
fn display_name(line: &[u8]) -> &[u8] {
    match line.windows(2).position(|pair| pair == [0x81, 0x97]) {
        Some(at) => &line[at + 2..],
        None => line,
    }
}

/// 651C00: a command whose first word is `voice` names files; the first that exists counts.
fn voice_file(storage: &dyn crysirror_core::service::ClientStorage, catalog: &crate::native_resource_paths::Catalog, line: &[u8], voice: &[u8]) -> Result<Option<String>> {
    let mut words = line.split(|b| *b == b' ').filter(|w| !w.is_empty());
    if words.next() != Some(voice) {
        return Ok(None);
    }
    for word in words {
        let name = String::from_utf8_lossy(word).into_owned();
        for path in catalog.candidates(storage, "sound", &name, &|| Ok(()))? {
            if crate::native_resource_paths::resolve(storage, &path)?.is_some() {
                return Ok(Some(name));
            }
        }
    }
    Ok(None)
}

/// 6510C0's inner loops: kind 32 text, kind 33 name, kind 48 voice commands.
fn read_inputs(storage: &dyn crysirror_core::service::ClientStorage, catalog: &crate::native_resource_paths::Catalog, scene: &crate::scene::Scene, count: u32, voice: &[u8], novel_command: &[u8], novel_mode: &mut bool) -> Result<Vec<(Vec<u8>, Vec<u8>, String, bool)>> {
    let mut out = Vec::new();
    for &(length, index) in scene.input_ranges.iter().take(count as usize) {
        let (mut name, mut text, mut file) = (Vec::new(), Vec::new(), None);
        let mut page_start = !*novel_mode;
        for line in scene.lines.iter().skip(index as usize).take(length as usize) {
            match line.kind {
                2 => page_start = !*novel_mode,
                3 if *novel_mode => page_start = true,
                32 => text.extend_from_slice(&line.bytes),
                33 => name.extend_from_slice(display_name(&line.bytes)),
                48 => {
                    if file.is_none() { file = voice_file(storage, catalog, &line.bytes, voice)?; }
                    let mut words = line.bytes.split(|b| *b == b' ').filter(|word| !word.is_empty());
                    if words.next().is_some_and(|word| word.eq_ignore_ascii_case(novel_command)) {
                        *novel_mode = words.next().and_then(|word| std::str::from_utf8(word).ok()).and_then(|word| word.parse::<i32>().ok()).is_some_and(|value| value != 0);
                    }
                }
                _ => {}
            }
        }
        out.push((name, text, file.unwrap_or_default(), page_start));
    }
    Ok(out)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(a.len() >= 4, "CatSystem2 log rebuild arguments");
    cx.machine.preflight_native(false, &[])?;
    let Some(limit) = (a[0] != 0).then(|| cx.tasks.logs.limit(a[0])).flatten() else {
        cx.machine.acknowledge(None)?;
        return Ok(true);
    };
    let Some(scene) = cx.handles.scenes.get(&a[1]) else {
        cx.machine.acknowledge(None)?;
        return Ok(true);
    };
    if cx.state.scene_log.job.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
        // 72EFB0: walk back from the newest visit until the limit is covered.
        let mut first = 0;
        let mut covered = 0usize;
        for (at, entry) in scene.history.iter().enumerate().rev() {
            if at == 0 || covered >= limit {
                first = if covered >= limit { at + 1 } else { 0 };
                break;
            }
            covered = covered.saturating_add((entry.value as usize).saturating_sub(1));
        }
        let visits: Vec<(usize, Vec<u8>, u32)> = scene.history.iter().enumerate().skip(first.min(scene.history.len())).map(|(index, entry)| (index, entry.name.clone(), entry.value)).collect();
        let voice = crate::native_values::format_bytes(cx.machine, a[2])?;
        let novel_command = crate::native_values::format_bytes(cx.machine, a[3])?;
        let storage = cx.host.storage(cx.client)?;
        let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
            Err(e) => return Err(e),
        };
        let task = permit.spawn(move |cancel| {
            let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
            let mut out = Vec::with_capacity(visits.len());
            let mut novel_mode = false;
            for (history_index, name, count) in visits {
                cancel.check()?;
                let stem = crate::native_scene::stem(&name).to_vec();
                let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &format!("scene.int/{}.cst", String::from_utf8_lossy(&stem)))?;
                let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? else { continue };
                let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), crate::kcs::MAX_SCRIPT_BYTES as u64)?;
                let scene = Arc::new(crate::scene::Scene::parse(&bytes)?);
                out.push(Source { scene: name, history_index, inputs: read_inputs(storage.as_ref(), &catalog, &scene, count, &voice, &novel_command, &mut novel_mode)? });
            }
            Ok(out)
        })?;
        cx.state.scene_log.job = Some((cx.call.pc, task));
        return Ok(false);
    }
    let Some(sources) = cx.state.scene_log.job.as_mut().unwrap().1.try_take()? else { return Ok(false) };
    cx.state.scene_log.job = None;
    cx.tasks.logs.clear(a[0]);
    for source in sources {
        let scene = String::from_utf8_lossy(&source.scene).into_owned();
        for (line, (name, text, voice, page_start)) in source.inputs.into_iter().enumerate() {
            // 73FF40: $-variables as of now, as the original's rebuild does.
            let name = cx.strings.expand(&name, 4096)?;
            let text = cx.strings.expand(&text, 16384)?;
            cx.tasks.logs.add(a[0], crate::native_log::Row { scene: scene.clone(), line: line as u32, flags: u32::from(!text.is_empty()) | u32::from(!name.is_empty()) * 2 | u32::from(!voice.is_empty()) * 4, name, text, voice, page_start, history_index: source.history_index as i32 });
        }
    }
    cx.machine.acknowledge(None)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_lines_show_what_follows_the_mark() {
        assert_eq!(display_name(b"abc\x81\x97def"), b"def");
        assert_eq!(display_name(b"abc"), b"abc");
    }
}
