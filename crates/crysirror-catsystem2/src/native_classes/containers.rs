// SPDX-License-Identifier: MPL-2.0
//! KCS natives that move native state into and out of CSS save containers
//! (78B600 family; records appended by 78B750, looked up by 78B790).
use super::Ctx;
use anyhow::{ensure, Result};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[240, 250, 609, 610];

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
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the container family"),
    }
    Ok(true)
}
