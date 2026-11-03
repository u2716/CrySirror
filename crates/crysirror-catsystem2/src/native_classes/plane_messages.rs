// SPDX-License-Identifier: MPL-2.0
//! KCS natives that message Plane (class 2, handler 5E01F0) and click-controller
//! (class 6, handler 5DE8E0) tasks. Plane state lives in task_policy's NativePlane;
//! geometry and input routing go through the shared Layer and InputRouting services.
use super::Ctx;
use anyhow::{ensure, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[95, 346, 651, 653, 654, 655, 694];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = &cx.call.arguments;
    match cx.call.id {
        // 640690 (msg 71, "kx3" path), 63FC60 (msg 85, two names) and 63FD60
        // (msg 86): 5E01F0 sends 71/85/86 straight to LABEL_104 and no other
        // class handler has these cases, so the original changes nothing.
        346 | 654 | 655 => {
            ensure!(!a.is_empty(), "CatSystem2 native {} arguments", cx.call.id);
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        // 659D90 (msg 24, -1) and 659FE0 (msg 29): 5E01F0 forwards 24..40 to
        // 5DE8E0, whose switch has neither case (default returns 1); no other
        // class handles 24 or 29.
        651 | 694 => {
            ensure!(!a.is_empty(), "CatSystem2 native {} arguments", cx.call.id);
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        // 640DE0 → msg 84: this[170] = render type for later object creation.
        653 => {
            ensure!(a.len() == 2, "CatSystem2 Plane render type arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(plane) = cx.tasks.plane_mut(a[0]) {
                plane.render_type = a[1];
            }
            cx.machine.acknowledge(None)?;
        }
        // 640C00 → msg 89 (sender, event mode): the Plane binds its own click area
        // from its object geometry: left/top = base - position (676040/675B80),
        // extent this[44]/this[45], priority = (int) position z.
        95 => {
            ensure!(a.len() == 2, "CatSystem2 Plane button bind arguments");
            cx.machine.preflight_native(false, &[])?;
            let id = a[0];
            let Ok(plane) = cx.tasks.plane(id) else {
                // A class-6 controller would read the event-mode word as a RECT
                // pointer (5DE8E0 case 89); classes without case 89 ignore it.
                ensure!(cx.tasks.button(id).is_err(), "CatSystem2 native 95 sent to a plain click controller");
                cx.machine.acknowledge(None)?;
                return Ok(true);
            };
            let pose = plane.pose;
            // Without a render object 675B80/676040 write nothing; the Plane has
            // no geometry and binds an empty area.
            let pose = if plane.payload.is_some() { pose } else { crate::task_policy::PlanePose { size: [0.; 2], ..pose } };
            let left = (pose.base[0] - pose.position[0]) as i32;
            let top = (pose.base[1] - pose.position[1]) as i32;
            let right = (pose.base[0] - pose.position[0] + pose.size[0]) as i32;
            let bottom = (pose.base[1] - pose.position[1] + pose.size[1]) as i32;
            let full_canvas = left <= 0 && top <= 0 && right >= cx.canvas[0] as i32 && bottom >= cx.canvas[1] as i32;
            let native_rect = if full_canvas { [-100000, -100000, 100000, 100000] } else { [left, top, right, bottom] };
            let empty = left >= right || top >= bottom;
            let [l, t, r, b] = native_rect;
            let rect = if empty && !full_canvas { None } else { Some([i64::from(l), i64::from(t), i64::from(r) - i64::from(l), i64::from(b) - i64::from(t)]) };
            let target = cx.tasks.current();
            let mut proposed = cx.tasks.clone();
            if proposed.bind_plane_button(id, target, native_rect, a[1], full_canvas)? {
                let receiver = crysirror_scene::input_routes::InputReceiverSpec { id, priority: pose.position[2] as i32, sources: 7, rect, coverage: None, enabled: true, hover_enabled: true, drag_enabled: true };
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"input_routes":[{"op":"put","receiver":receiver}]}))?)?;
            }
            *cx.tasks = proposed;
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the Plane family"),
    }
    Ok(true)
}

#[allow(unused_variables)]
pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    Ok(())
}
