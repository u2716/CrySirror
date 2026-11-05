// SPDX-License-Identifier: MPL-2.0
//! KCS natives that tune the original D3D render manager or its texture memory.
//! The shared renderer owns those decisions, so nothing below changes a frame.
use super::Ctx;
use anyhow::{ensure, Result};
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[541, 542, 568, 681, 858, 859, 879, 881];

#[derive(Default)]
pub(crate) struct State {
    /// ImageList this[36]: draw key → {channel, cutoff}.
    sound_links: BTreeMap<(u32, u32), [u32; 2]>,
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    cx.machine.preflight_native(false, &[])?;
    match cx.call.id {
        // 640870 → 680880 / 640650 → 680870: free the VRAM tile textures under a rectangle
        // or all of them. Tiles only cache; the Canvas keeps every draw it was given.
        541 | 542 => {}
        // 642820 / 6427C0 → 67FB80: depth range of the device viewport (and 859's extra
        // flag). Layer order is explicit here, so the depth range decides nothing.
        568 | 859 => {}
        // 64A410 → 673F90: drop a decoded image from the cache; resources are shared.
        681 => {}
        // 642800 → 6756F0: 66F5D0 queries pixel shader support and the result is dropped.
        858 => {}
        // 643CA0 → 311: link draw key (sender, a1) to sound channel a2 for 5F3A00, which
        // feeds channel time (212) to that draw's motion; -1 unlinks. Draws here have no
        // frame motion (native 312 note), so the link keeps only its state.
        879 => {
            ensure!(a.len() == 3, "CatSystem2 draw sound link arguments");
            let key = (a[0], cx.tasks.current() | (a[1] << 16));
            if a[2] == u32::MAX {
                cx.state.render_settings.sound_links.remove(&key);
            } else {
                cx.state.render_settings.sound_links.insert(key, [a[2], 1000]);
            }
        }
        // 643C70 → 313: the link's cutoff time.
        881 => {
            ensure!(a.len() == 3, "CatSystem2 draw sound cutoff arguments");
            let key = (a[0], cx.tasks.current() | (a[1] << 16));
            if let Some(link) = cx.state.render_settings.sound_links.get_mut(&key) {
                link[1] = a[2];
            }
        }
        id => anyhow::bail!("CatSystem2 render setting native {id} is not implemented"),
    }
    cx.machine.acknowledge(None)?;
    Ok(true)
}
