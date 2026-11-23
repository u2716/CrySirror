// SPDX-License-Identifier: MPL-2.0
//! KCS natives answered by the game window (WndProc 5316A0) and the render
//! manager's frame dump (VM+116, fields +0xA4..+0xAC). Pixels come from the
//! shared presenter capture; files go through the client storage.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    path::{Mount, VfsPath},
    service::HostServices,
    state::ClientId,
};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[9, 441, 565];

/// Render manager +0xA4 mode, +0xA8 frame counter, +0xAC folder (676A80).
#[derive(Default)]
pub(crate) struct State {
    dump_mode: u32,
    dump_counter: u32,
    dump_folder: String,
    dump_job: Option<(ImageJob, String)>,
    /// 0x433 → 0x410 tears the window down; the adapter halts after the call.
    pub halt: bool,
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 64F910: a1 != 0 posts WM_CLOSE (5316A0 case 0x10: beep, MessageBox
        // "00000042"/"00000043" yes/no, then 674010 + 0x410); a1 == 0 posts 0x433,
        // which runs 674010 (hide cursor) and 0x410 without asking.
        9 => {
            ensure!(a.len() == 1, "CatSystem2 window close arguments");
            ensure!(a[0] == 0, "CatSystem2 native 9 asks a yes/no close confirmation; the host has no confirmation dialog service");
            cx.machine.preflight_native(false, &[])?;
            cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&serde_json::json!({"ui":[{"op":"set_cursor_visible","visible":false}]}))?)?;
            cx.machine.acknowledge(None)?;
            cx.state.os_window.halt = true;
        }
        // 64F4E0 → 676A80(mode, folder, counter): a nonzero mode makes 675960 save
        // every presented frame as "%s\%s_%010d.bmp" (folder, folder, counter++).
        441 => {
            ensure!(a.len() == 3, "CatSystem2 frame dump arguments");
            cx.machine.preflight_native(false, &[])?;
            let folder = crysirror_resource::text_stream::decode_auto(&crate::native_values::format_bytes(cx.machine, a[1])?, 2048)?;
            let state = &mut cx.state.os_window;
            state.dump_mode = a[0];
            state.dump_folder = folder.replace('\\', "/").trim_matches('/').to_owned();
            state.dump_counter = a[2];
            state.dump_job = None;
            cx.machine.acknowledge(None)?;
        }
        // 64FEF0 → 6761E0: the frame dump mode.
        565 => {
            ensure!(a.is_empty(), "CatSystem2 frame dump mode arguments");
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(cx.state.os_window.dump_mode))?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the window family"),
    }
    Ok(true)
}

/// 675960 after each presented frame: capture, then write one numbered BMP.
pub(crate) fn tick(host: &mut dyn HostServices, client: ClientId, state: &mut State) -> Result<()> {
    if let Some((mut job, name)) = state.dump_job.take() {
        match job.poll().map_err(anyhow::Error::from)? {
            ImageProcessState::Pending => state.dump_job = Some((job, name)),
            ImageProcessState::Failed(e) => anyhow::bail!("CatSystem2 frame dump capture: {e}"),
            ImageProcessState::Ready(image) => {
                let [w, h] = image.size();
                let bmp = crysirror_image::bmp_encode::encode_bmp(w, h, &image.rgba(), crysirror_image::bmp_encode::BmpDepth::Bgr24)?;
                host.write_atomic(client, &VfsPath::new(Mount::Save, &name).context("CatSystem2 frame dump path")?, &bmp)?;
            }
        }
        return Ok(());
    }
    if state.dump_mode == 0 {
        return Ok(());
    }
    let folder = &state.dump_folder;
    let name = format!("{folder}/{folder}_{:010}.bmp", state.dump_counter);
    state.dump_counter = state.dump_counter.wrapping_add(1);
    state.dump_job = Some((host.begin_frame_capture(client).map_err(anyhow::Error::from)?, name));
    Ok(())
}
