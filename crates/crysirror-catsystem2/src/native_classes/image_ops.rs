// SPDX-License-Identifier: MPL-2.0
//! KCS natives answered by the ImageList task (class 15, handler 5F3AE0).
//! A task of another class has no case for these messages (base handler 0).
use super::Ctx;
use anyhow::{ensure, Result};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[342, 464, 534, 625];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 ImageList native {} arguments", cx.call.id);
    cx.machine.preflight_native(false, &[])?;
    if cx.tasks.image_list(a[0]).is_err() {
        cx.machine.acknowledge(None)?;
        return Ok(true);
    }
    match cx.call.id {
        // 6440A0 → 292: this[35] = 1; with a manager 743EE0 marks every page dirty
        // and recomposes (743600), then this[31] = 0.
        342 => {
            ensure!(a.len() == 1, "CatSystem2 ImageList refresh arguments");
            let list = cx.tasks.image_list_mut(a[0])?;
            list.field_140 = 1;
            if list.manager.is_some() {
                list.field_124 = 0;
                republish(cx, a[0])?;
            }
        }
        // 6440E0 → 300 → 744540(key): restart the draw's frame motion (746850)
        // and mark its old and new rectangles dirty for the next composition.
        464 => {
            ensure!(a.len() == 2, "CatSystem2 ImageList draw refresh arguments");
            if cx.tasks.image_list(a[0])?.manager.is_some() {
                republish(cx, a[0])?;
            }
        }
        // 644060 → 295(key, start, play): 746510/746830 store the draw's motion
        // time (+0x38) and play flag (+0x50), then 744330 steps every draw that
        // has a motion (+0x48). Draws from 257/732 (643910/6444C0 → 280) never
        // carry one (message word 14 stays zero), so nothing changes on screen.
        625 => {
            ensure!(a.len() == 3, "CatSystem2 ImageList draw motion arguments");
        }
        // 643E40 → 304: this[35] = argument.
        534 => {
            ensure!(a.len() == 2, "CatSystem2 ImageList compose flag arguments");
            cx.tasks.image_list_mut(a[0])?.field_140 = a[1];
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the ImageList family"),
    }
    cx.machine.acknowledge(None)?;
    Ok(true)
}

/// The shared Canvas presents the list's draw records; republishing them is the
/// recomposition. Draw records carry no frame motion here (native 312 note).
fn republish(cx: &mut Ctx, id: u32) -> Result<()> {
    #[cfg(feature = "runtime")]
    {
        let publication = cx.tasks.list_publication(id, cx.client)?;
        publication.publish(cx.host, cx.client, Vec::new())?;
    }
    #[cfg(not(feature = "runtime"))]
    anyhow::bail!("CatSystem2 ImageList {id} publication needs the runtime");
    #[cfg(feature = "runtime")]
    Ok(())
}
