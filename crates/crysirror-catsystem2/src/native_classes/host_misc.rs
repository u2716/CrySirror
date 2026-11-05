// SPDX-License-Identifier: MPL-2.0
//! KCS natives answered by the window, the motion manager and the image codec that
//! keep little state of their own.
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::path::{Mount, VfsPath};
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[76, 201, 241, 315, 316, 317, 514, 576, 577, 598, 628, 649, 650, 688, 689, 740, 741, 846, 903, 1009, 1049];

#[derive(Default)]
pub(crate) struct State {
    /// Handles of 315 snapshots (6584E0 {set, buffer, size}). Scripts only keep and free them.
    snapshots: BTreeMap<u32, u32>,
    /// B91BFC+50024: 628 selects the window menu with the extra entry.
    pub window_menu: u32,
    minimize_noted: bool,
    fes_state_noted: bool,
    /// 903: pending read of the `.tag` cue tables on a worker.
    tags: Option<(u32, crysirror_tasks::Task<u32>)>,
}

/// `pcm_tag.int/<sound>.tag`: 24-byte rows of { name[20], value }. 5EAEA0 message 214 answers
/// the value of the name on that channel's table, 0 without one.
fn tag_value(bytes: &[u8], name: &[u8]) -> Option<u32> {
    bytes.chunks_exact(24).find_map(|row| {
        let end = row[..20].iter().position(|b| *b == 0).unwrap_or(20);
        (&row[..end] == name).then(|| u32::from_le_bytes(row[20..24].try_into().unwrap()))
    })
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 659F20: the calling task's parent → its id (+0xC, then +4).
        76 => {
            cx.machine.preflight_native(true, &[])?;
            let id = cx.tasks.parent_of(cx.tasks.current())?;
            cx.machine.acknowledge(Some(id))?;
        }
        // 641360 → 13: class 18 takes 264 and class 25 takes 273 when a1 == 0. Scripts
        // create no class 25.
        201 => {
            ensure!(a.len() == 3, "CatSystem2 effect parameter arguments");
            cx.machine.preflight_native(false, &[])?;
            if cx.tasks.opaque_class(a[0]) == Some(18) {
                super::shader_tasks::raster_native(cx)?;
            } else {
                cx.machine.acknowledge(None)?;
            }
        }
        // 649DA0: the image goes into the save record under key a1, answering 1. The original
        // writes its own HG-2 (690210 type 4); this adapter's record holds PNG, which only
        // its own image reader reads back.
        241 => {
            ensure!(a.len() == 3, "CatSystem2 save image arguments");
            cx.machine.preflight_native(true, &[])?;
            let image = cx.handles.images.get(&a[2]).map(|frame| frame.image.clone());
            let saved = match (a[0], image) {
                (0, _) | (_, None) => 0,
                (container, Some(image)) => {
                    let [width, height] = image.size();
                    let png = crysirror_image::encode_png(&crysirror_image::ImageSurface { width, height, rgba: image.rgba().to_vec() })?;
                    cx.saves.get_mut(&container).context("CatSystem2 save image container")?.append(a[1], &png)?;
                    1
                }
            };
            cx.machine.acknowledge(Some(saved))?;
        }
        576 | 688 | 689 => {
            super::view3d::save(cx)?;
        }
        // 649C30 → 343 → 7582E0: a running FES child's local variables under key a1.
        649 => {
            ensure!(a.len() == 3, "CatSystem2 FES save arguments");
            cx.machine.preflight_native(true, &[])?;
            let bytes = cx.interpreter.as_ref().and_then(|vm| vm.player_bank_bytes(a[2]));
            let saved = match bytes {
                Some(bytes) if a[0] != 0 && cx.tasks.fes(a[2]).is_ok() => {
                    cx.saves.get_mut(&a[0]).context("CatSystem2 FES save container")?.append(a[1], &bytes)?;
                    1
                }
                _ => 0,
            };
            cx.machine.acknowledge(Some(saved))?;
        }
        // 6491D0 → 344 → 758280: the variables come back, and a child with a `_LOAD_` section
        // resumes there.
        650 => {
            ensure!(a.len() == 3, "CatSystem2 FES load arguments");
            cx.machine.preflight_native(true, &[])?;
            let record = (a[0] != 0).then(|| cx.saves.get(&a[0]).and_then(|save| save.first(a[1]).map(<[u8]>::to_vec))).flatten();
            let loaded = match record {
                Some(bytes) => {
                    let vm = cx.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?;
                    if vm.restore_player_bank(a[2], &bytes) {
                        cx.tasks.set_fes_section(a[2], b"_LOAD_")?;
                    }
                    1
                }
                None => 0,
            };
            cx.machine.acknowledge(Some(loaded))?;
        }
        // 644F80 → sound message 214.
        903 => {
            ensure!(a.len() == 2, "CatSystem2 sound tag arguments");
            cx.machine.preflight_native(true, &[])?;
            #[cfg(not(feature = "runtime"))]
            bail!("CatSystem2 sound tags need the runtime feature");
            #[cfg(feature = "runtime")]
            {
                if cx.state.host_misc.tags.as_ref().is_none_or(|(pc, _)| *pc != cx.call.pc) {
                    let name = crate::native_values::format_bytes(cx.machine, a[1])?;
                    let sounds = cx.kcs_sound.loaded_names((a[0] != u32::MAX).then_some(a[0]));
                    let storage = cx.host.storage(cx.client)?;
                    let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                        Ok(permit) => permit,
                        Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                        Err(e) => return Err(e),
                    };
                    let task = permit.spawn(move |_| {
                        for sound in sounds {
                            let stem = String::from_utf8_lossy(crate::native_scene::stem(&sound)).into_owned();
                            let path = VfsPath::new(Mount::Asset, &format!("pcm_tag.int/{stem}.tag"))?;
                            let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? else { continue };
                            let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 1024 * 1024)?;
                            if let Some(value) = tag_value(&bytes, &name) {
                                return Ok(value);
                            }
                        }
                        Ok(0)
                    })?;
                    cx.state.host_misc.tags = Some((cx.call.pc, task));
                    return Ok(false);
                }
                let Some(value) = cx.state.host_misc.tags.as_mut().unwrap().1.try_take()? else { return Ok(false) };
                cx.state.host_misc.tags = None;
                cx.machine.acknowledge(Some(value))?;
            }
        }
        315 => {
            ensure!(!a.is_empty(), "CatSystem2 motion snapshot arguments");
            cx.machine.preflight_native(true, &[])?;
            if a[0] == 0 {
                cx.machine.acknowledge(Some(0))?;
            } else {
                let id = cx.handles.allocate()?;
                cx.state.host_misc.snapshots.insert(id, a[0]);
                cx.machine.acknowledge(Some(id))?;
            }
        }
        316 => {
            ensure!(a.len() == 2, "CatSystem2 motion snapshot refresh arguments");
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(u32::from(a[0] != 0)))?;
        }
        317 => {
            ensure!(a.len() == 1, "CatSystem2 motion snapshot free arguments");
            cx.machine.preflight_native(false, &[])?;
            cx.state.host_misc.snapshots.remove(&a[0]);
            cx.machine.acknowledge(None)?;
        }
        // 6589F0 → 789AA0: whether the set holds that motion; output pose resets either way.
        514 => {
            ensure!(a.len() == 2, "CatSystem2 motion name query arguments");
            cx.machine.preflight_native(true, &[])?;
            let name = crate::native_values::format_bytes(cx.machine, a[1])?;
            let name = crate::native_scene::stem(&name).to_vec();
            let wide: Vec<u16> = name.iter().map(|b| u16::from(*b)).collect();
            let found = match cx.handles.motions.get_mut(&a[0]) {
                Some(set) => set.contains_name(&wide)?,
                None => false,
            };
            cx.machine.acknowledge(Some(u32::from(found)))?;
        }
        // 6501B0 → root message 40: stores root+652, which nothing in the program reads.
        577 => {
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        // 64B600 → 7908B0: comma separated text; the first non-empty part goes to the buffer.
        598 => {
            ensure!(a.len() == 2, "CatSystem2 text split arguments");
            let text = crate::native_values::format_bytes(cx.machine, a[0])?;
            match text.split(|b| *b == b',').next().filter(|part| !part.is_empty()) {
                Some(part) if a[1] != 0 => {
                    let mut part = part.to_vec();
                    part.push(0);
                    cx.machine.commit_native(Some(1), &[(a[1], part)])?;
                }
                _ => {
                    cx.machine.preflight_native(true, &[])?;
                    cx.machine.acknowledge(Some(0))?;
                }
            }
        }
        // 650200: 0x466 rebuilds a menu the host window does not have.
        628 => {
            ensure!(a.len() == 1, "CatSystem2 window menu arguments");
            cx.machine.preflight_native(false, &[])?;
            cx.state.host_misc.window_menu = a[0];
            cx.machine.acknowledge(None)?;
        }
        // 647240: B91BFC+0xF78C is set only for the exclusive fullscreen mode, which
        // this host never enters, so the answer is the windowed 0.
        741 => {
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(0))?;
        }
        // 64FA60: where the pointer goes to leave the window, under its client area in
        // canvas units (B91BFC+63432 selects the height source; all equal without a frame).
        740 => {
            ensure!(a.len() == 2, "CatSystem2 pointer exit arguments");
            cx.machine.commit_native(None, &[(a[0], 0u32.to_le_bytes().to_vec()), (a[1], cx.canvas[1].to_le_bytes().to_vec())])?;
        }
        // 650190: 0x46A → WM_COMMAND 0x9C55 → SC_MINIMIZE.
        846 => {
            cx.machine.preflight_native(false, &[])?;
            if !core::mem::replace(&mut cx.state.host_misc.minimize_noted, true) {
                cx.host.append_log(cx.client, "CatSystem2 native 846: the host has no minimize request; the window stays")?;
            }
            cx.machine.acknowledge(None)?;
        }
        // 64C2A0: encode by extension: bmp type 1, png type 7, anything else JPEG (type 5).
        1009 => {
            ensure!(a.len() == 2, "CatSystem2 image save arguments");
            cx.machine.preflight_native(true, &[])?;
            if a[0] == 0 {
                cx.machine.acknowledge(Some(0))?;
                return Ok(true);
            }
            let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 image save name")?;
            let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            let image = cx.handles.images.get(&a[0]).context("CatSystem2 image save handle")?.image.clone();
            let [width, height] = image.size();
            let bytes = match extension.as_str() {
                "bmp" => crysirror_image::bmp_encode::encode_bmp(width, height, &image.rgba(), crysirror_image::bmp_encode::BmpDepth::Bgr24)?,
                "png" => crysirror_image::encode_png(&crysirror_image::ImageSurface { width, height, rgba: image.rgba().to_vec() })?,
                _ => crysirror_image::encode_jpeg(&crysirror_image::ImageSurface { width, height, rgba: image.rgba().to_vec() }, 90)?,
            };
            let path = VfsPath::new(Mount::Save, &name.replace('\\', "/")).context("CatSystem2 image save path")?;
            cx.host.write_atomic(cx.client, &path, &bytes)?;
            cx.machine.acknowledge(Some(1))?;
        }
        // 655DF0: parse the format string as message text and keep its plain glyphs.
        1049 => {
            ensure!(a.len() == 2, "CatSystem2 plain text arguments");
            let text = crate::native_values::format_bytes(cx.machine, a[1])?;
            if a[0] == 0 {
                cx.machine.preflight_native(true, &[])?;
                cx.machine.acknowledge(Some(0))?;
            } else {
                let mut plain = super::text_window::plain_text(&text);
                let length = plain.len().min(4095);
                plain.truncate(length);
                plain.push(0);
                cx.machine.commit_native(Some(length as u32), &[(a[0], plain)])?;
            }
        }
        id => bail!("CatSystem2 native {id} is not implemented"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_rows_answer_by_exact_name() {
        let mut bytes = b"01".to_vec();
        bytes.resize(20, 0);
        bytes.extend(0x69au32.to_le_bytes());
        assert_eq!(tag_value(&bytes, b"01"), Some(0x69a));
        assert_eq!(tag_value(&bytes, b"02"), None);
    }
}
