// SPDX-License-Identifier: MPL-2.0
//! Window-system selectors of the original Windows build over shared host services:
//! shell open `0x201c`, pointer regions `0x20cb`, pointer warp `0x20cc` and the
//! class cursor `0x2099`.

use crate::vm::CmvsVm;
use anyhow::{Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices},
    state::ClientId,
    Error,
};
use crysirror_image::gdi_region::{GdiRegion, PolygonFill};
use serde_json::json;
use std::sync::Arc;

const CURSOR: u32 = 0x434d_5643;

/// What the adapter must do after a host selector.
pub(super) enum Host {
    Done,
    /// Read and decode a cursor file on a worker, then select it.
    LoadCursor(u32, VfsPath),
}

#[derive(Default)]
pub(super) struct HostState {
    /// `0x2099` class cursor index; 0 is the system arrow.
    cursor: u32,
}

/// A script path below the game directory, with Windows separators.
fn game_path(bytes: &[u8]) -> Option<VfsPath> {
    let (text, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    if invalid || text.is_empty() || text.contains(':') || text.starts_with(['\\', '/']) {
        return None;
    }
    VfsPath::new(Mount::Asset, &text.replace('\\', "/")).ok()
}

impl HostState {
    pub fn command(
        &mut self,
        opcode: u16,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Option<Host>> {
        match opcode {
            0x201c => {
                // 0x140098aa0 runs ShellExecuteA(open) only when the fourth word is -1.
                let [file, _, _, mode] = vm.owner_arguments(opcode)?;
                if mode == u32::MAX {
                    let file = vm.text_reference(file).context("CMVS shell file")?.to_vec();
                    match game_path(&file) {
                        // The player shows the game's own documents; it never runs programs.
                        Some(path) => match host.open_document(client, &path) {
                            Ok(()) | Err(Error::Unsupported(_) | Error::NotFound) => {}
                            Err(error) => return Err(error.into()),
                        },
                        None => {
                            let target = encoding_rs::SHIFT_JIS.decode(&file).0.into_owned();
                            host.append_log(client, &format!("CMVS shell request not executed: {target}"))?;
                        }
                    }
                }
                vm.complete_owner_call(4, None)?;
            }
            0x20cb => {
                let [kind, a, b, c, d, e, f] = vm.owner_arguments(opcode)?;
                let [a, b, c, d, e, f] = [a, b, c, d, e, f].map(|word| word as i32);
                let region = match kind {
                    1 => Some(GdiRegion::Rect { rect: [a, b, c, d] }),
                    2 => Some(GdiRegion::Ellipse { rect: [a, b, c, d] }),
                    3 => Some(GdiRegion::RoundRect { rect: [a, b, c, d], corner: [e, f] }),
                    // Points come from consecutive integer variables (0x14009db40).
                    4 => {
                        let count = b.clamp(0, 256) as usize;
                        let mut points = Vec::with_capacity(count);
                        for index in 0..count {
                            let base = a.wrapping_add(2 * index as i32);
                            let word = |offset: i32| {
                                usize::try_from(base.wrapping_add(offset))
                                    .ok()
                                    .and_then(|variable| vm.integer(variable))
                                    .map_or(0, |value| value as i32)
                            };
                            points.push([word(0), word(1)]);
                        }
                        Some(GdiRegion::Polygon { points, fill: PolygonFill::Winding })
                    }
                    _ => None,
                };
                let inside = region.is_some_and(|region| region.contains(vm.pointer()));
                vm.complete_owner_call(7, Some(u32::from(inside)))?;
            }
            0x20cc => {
                let [x, y] = vm.owner_arguments(opcode)?;
                // SetCursorPos fails without effect where the pointer cannot be moved.
                match host.warp_pointer(client, x as i32, y as i32) {
                    Ok(()) | Err(Error::Unsupported(_)) => {}
                    Err(error) => return Err(error.into()),
                }
                vm.complete_owner_call(2, None)?;
            }
            0x2099 => {
                let [index] = vm.owner_arguments(opcode)?;
                vm.complete_owner_call(1, None)?;
                if index == self.cursor {
                    return Ok(Some(Host::Done));
                }
                if index == 0 {
                    self.cursor = 0;
                    let select = json!({"ui":[{"op":"set_cursor","cursor":{"kind":"standard","shape":"default"}}]});
                    host.submit_engine(client, 1, &serde_json::to_vec(&select)?)?;
                    return Ok(Some(Host::Done));
                }
                // 0x1400a5580 keeps the current cursor when the index has no registration.
                let Some(path) = vm.cursor_name(index).and_then(game_path) else {
                    return Ok(Some(Host::Done));
                };
                return Ok(Some(Host::LoadCursor(index, path)));
            }
            _ => return Ok(None),
        }
        Ok(Some(Host::Done))
    }

    /// Reads a cursor file under the game directory on a worker.
    pub fn read_cursor(storage: &Arc<dyn ClientStorage>, path: &VfsPath) -> Result<Option<crysirror_image::cursor::CursorImage>> {
        let Some(path) = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage.as_ref(), path)? else {
            return Ok(None);
        };
        let source = storage.open_asset(&path)?;
        let bytes = crysirror_resource::read_all(source.as_ref(), 4 * 1024 * 1024)?;
        Ok(crysirror_image::cursor::decode_cursor(&bytes).ok())
    }

    /// Selects a decoded cursor. A file LoadImageA cannot read leaves the arrow.
    pub fn select_cursor(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: u32,
        image: Option<crysirror_image::cursor::CursorImage>,
    ) -> Result<()> {
        self.cursor = index;
        let ops = match image {
            Some(image) => {
                let frames: Vec<_> = image
                    .frames
                    .iter()
                    .map(|frame| json!({"width":frame.image.width,"height":frame.image.height,
                        "hotspot":frame.hotspot,"rgba":frame.image.rgba}))
                    .collect();
                let steps: Vec<_> = image.steps.iter().map(|&(frame, ms)| [frame as u32, ms]).collect();
                vec![
                    json!({"op":"define_cursor","id":CURSOR,"cursor":{"frames":frames,"steps":steps}}),
                    json!({"op":"set_cursor","cursor":{"kind":"custom","id":CURSOR}}),
                ]
            }
            None => vec![json!({"op":"set_cursor","cursor":{"kind":"standard","shape":"default"}})],
        };
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({ "ui": ops }))?)?;
        Ok(())
    }
}
