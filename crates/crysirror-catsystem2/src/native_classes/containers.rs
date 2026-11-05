// SPDX-License-Identifier: MPL-2.0
//! KCS natives that move native state into and out of CSS save containers
//! (78B600 family; records appended by 78B750, looked up by 78B790).
use super::Ctx;
use anyhow::{ensure, Context, Result};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[240, 250, 385, 386, 609, 610, 978, 979, 981, 993];

#[derive(Default)]
pub(crate) struct State {}

/// Task message 176: FES (class 34, 62BED0) answers its player bank this[109],
/// stored by message 331. Other classes leave the out value null.
fn bank_owner(cx: &Ctx, task: u32) -> Option<u32> {
    cx.tasks.fes(task).ok().map(|_| task)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 643510 / 6436D0: with an interpreter (VM+104), 728D60 copies the flags
        // into the backup allocation and 728D80 copies them back.
        609 | 610 => {
            ensure!(a.is_empty(), "CatSystem2 flag backup arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(interpreter) = cx.interpreter.as_mut() {
                if cx.call.id == 609 {
                    interpreter.backup_flags();
                } else {
                    interpreter.restore_flags()?;
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 649CF0: container, key, task, first word, word count. 728A00 copies the
        // clipped range; 78B750 appends it (an empty range is still a record).
        240 => {
            ensure!(a.len() == 5, "CatSystem2 bank save arguments");
            cx.machine.preflight_native(true, &[])?;
            let words = bank_owner(cx, a[2]).and_then(|owner| cx.interpreter.as_ref()?.player_words(owner, a[3], a[4]));
            let saved = match (a[0], words) {
                (0, _) | (_, None) => 0,
                (container, Some(words)) => {
                    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
                    match cx.saves.get_mut(&container) {
                        Some(save) => {
                            save.append(a[1], &bytes)?;
                            1
                        }
                        // A stale handle dereferences freed memory in the original.
                        None => anyhow::bail!("CatSystem2 bank save into unknown container {container}"),
                    }
                }
            };
            cx.machine.acknowledge(Some(saved))?;
        }
        // 649240: container, key, task, first word. The record's size >> 2 words
        // are written back by 728A60, which also marks the bank changed.
        250 => {
            ensure!(a.len() == 4, "CatSystem2 bank load arguments");
            cx.machine.preflight_native(true, &[])?;
            let owner = bank_owner(cx, a[2]).filter(|_| a[0] != 0);
            let record = owner.and_then(|_| cx.saves.get(&a[0]).and_then(|save| save.first(a[1])).map(<[u8]>::to_vec));
            let loaded = match (owner, record) {
                (Some(owner), Some(bytes)) => {
                    let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
                    match cx.interpreter.as_mut().and_then(|i| i.write_player_words(owner, a[3], &words)) {
                        Some(_) => 1,
                        None => 0,
                    }
                }
                _ => 0,
            };
            cx.machine.acknowledge(Some(loaded))?;
        }
        // 649EF0: container, key, scene, list (0 first, else second), include data.
        // 72D120 sizes and writes both lists; one becomes the record.
        385 => {
            ensure!(a.len() == 5, "CatSystem2 scene list save arguments");
            cx.machine.preflight_native(true, &[])?;
            let list = cx.handles.scenes.get(&a[2]).filter(|_| a[0] != 0).map(|scene| {
                crate::native_scene::serialize_entries(if a[3] != 0 { &scene.history } else { &scene.names }, a[4] != 0)
            });
            let saved = match list {
                Some(bytes) => {
                    cx.saves.get_mut(&a[0]).with_context(|| format!("CatSystem2 scene list save into unknown container {}", a[0]))?.append(a[1], &bytes)?;
                    1
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(saved))?;
        }
        // 649480: container, key, scene, list. 72D320 replaces that list only. Its
        // first-list path reads data at offset + null, so a record there with data
        // would fault in the original.
        386 => {
            ensure!(a.len() == 4, "CatSystem2 scene list load arguments");
            cx.machine.preflight_native(true, &[])?;
            let record = (a[0] != 0).then(|| cx.saves.get(&a[0]).and_then(|save| save.first(a[1])).map(<[u8]>::to_vec)).flatten();
            let loaded = match (record, cx.handles.scenes.get_mut(&a[2])) {
                (Some(bytes), Some(scene)) => {
                    let list = crate::native_scene::parse_entries(&bytes)?;
                    if a[3] != 0 {
                        scene.history = list;
                    } else {
                        ensure!(list.iter().all(|e| e.blob.is_none()), "CatSystem2 first scene list record carries data (72D320 faults)");
                        scene.names = list;
                    }
                    1
                }
                _ => 0,
            };
            cx.machine.acknowledge(Some(loaded))?;
        }
        // 6422A0 -> 78B7E0 + 72D9C0: container a1 nested becomes the last history entry's data.
        978 => {
            ensure!(a.len() == 2, "CatSystem2 scene entry data arguments");
            cx.machine.preflight_native(false, &[])?;
            let nested = cx.saves.get(&a[1]).map(|save| save.nested());
            if let (Some(bytes), Some(entry)) = (nested, cx.handles.scenes.get_mut(&a[0]).and_then(|scene| scene.history.last_mut())) {
                entry.blob = Some(bytes);
            }
            cx.machine.acknowledge(None)?;
        }
        // 641B80 -> 72DA40 + 78B820: history entry a1's data as a new container; none answers 0.
        979 => {
            ensure!(a.len() == 2, "CatSystem2 scene entry container arguments");
            cx.machine.preflight_native(true, &[])?;
            let blob = cx.handles.scenes.get(&a[0]).and_then(|scene| scene.history.get(a[1] as usize)).and_then(|entry| entry.blob.clone());
            let id = match blob {
                Some(bytes) => {
                    let id = *cx.next_save;
                    *cx.next_save = id.checked_add(1).context("CatSystem2 CSS handle overflow")?;
                    cx.saves.insert(id, crate::native_save::Container::from_nested(&bytes)?);
                    id
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(id))?;
        }
        // 6421A0 -> 72DA90: data of scene a1 onto scene a0, entry by entry over both lists.
        981 => {
            ensure!(a.len() == 2, "CatSystem2 scene data copy arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(source) = cx.handles.scenes.get(&a[1]).cloned() {
                if let Some(target) = cx.handles.scenes.get_mut(&a[0]) {
                    for (to, from) in target.names.iter_mut().zip(&source.names).chain(target.history.iter_mut().zip(&source.history)) {
                        to.blob = from.blob.clone();
                    }
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 641760 -> 72DC00: scene a0's history becomes scene a1's names and values only.
        993 => {
            ensure!(a.len() == 2, "CatSystem2 scene history copy arguments");
            cx.machine.preflight_native(false, &[])?;
            let copy: Option<Vec<_>> = if a[0] == a[1] {
                Some(Vec::new())
            } else {
                cx.handles.scenes.get(&a[1]).map(|source| source.history.iter().map(|e| crate::native_scene::SceneEntry { name: e.name.clone(), value: e.value, ..Default::default() }).collect())
            };
            if let (Some(copy), Some(target)) = (copy, cx.handles.scenes.get_mut(&a[0])) {
                target.history = copy;
            }
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the container family"),
    }
    Ok(true)
}
