// SPDX-License-Identifier: MPL-2.0
//! KCS natives answered by the INPUT task (class 11, constructor 5E6270,
//! handler 5E6F50, tick 5E66C0). Scripts reach it either through message 34 to
//! the root (5DE390 returns the INPUT task id) or through a click controller's
//! 5DE8E0 forwarding. Receivers, hover markers and pointer movement live in the
//! shared InputRouting / host pointer services; only the original policy is here.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[106, 109, 149, 150, 286, 394, 401, 404, 937];

/// INPUT this[1305..1308]: message 137/138 pointer glide (start, remaining
/// milliseconds, target in virtual canvas coordinates).
#[derive(Clone, Copy, Debug)]
struct Glide {
    start_ms: u64,
    remaining: i64,
    target: [i32; 2],
}

#[derive(Default)]
pub(crate) struct State {
    glide: Option<Glide>,
    /// INPUT this[1309]: the software cursor state 677130 last accepted.
    cursor_visible: Option<bool>,
}

fn route(cx: &mut Ctx, commands: serde_json::Value) -> Result<()> {
    cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({ "input_routes": commands }))?)?;
    Ok(())
}

fn receiver(cx: &mut Ctx, id: u32) -> Result<Option<crysirror_scene::input_routes::InputReceiverSpec>> {
    Ok(serde_json::from_slice(&cx.host.query_engine(cx.client, crysirror_core::service::EngineQuery::InputRouteReceiver(id))?)?)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 648920 → 5DE8E0 case 93 → INPUT 132: set this controller's hover
        // marker without an enter/leave message; marking clears every other one.
        // An unbound controller (this[27] == -1) returns before forwarding.
        106 => {
            ensure!(a.len() == 2, "CatSystem2 hover marker arguments");
            cx.machine.preflight_native(false, &[])?;
            let bound = cx.tasks.button(a[0]).is_ok_and(|b| b.binding_target.is_some());
            if bound && receiver(cx, a[0])?.is_some() {
                route(cx, json!([{"op":"set_hover_marker","id":a[0],"marked":a[1]!=0}]))?;
            } else if bound && a[1] != 0 {
                route(cx, json!([{"op":"clear_hover_markers","reevaluate":false}]))?;
            }
            cx.machine.acknowledge(None)?;
        }
        // 648A10 → 5DE8E0 case 95 → INPUT 136: the source bits of the last
        // key event 5E7A30 offered (touch 16 reads as 2). 5DD130 reports a
        // delivered message, so the native answers the out value.
        109 => {
            ensure!(a.len() == 1, "CatSystem2 last input source arguments");
            ensure!(cx.tasks.button(a[0]).is_ok() || cx.tasks.plane(a[0]).is_ok(), "CatSystem2 native 109 target has no 5DE8E0 case 95");
            cx.machine.preflight_native(true, &[])?;
            let source = if cx.input.last_source == 16 { 2 } else { cx.input.last_source };
            cx.machine.acknowledge(Some(source))?;
        }
        // 656720 → INPUT 125: 70F6D0 pointer, 676990 to virtual canvas, into two VM words.
        149 => {
            ensure!(a.len() == 2, "CatSystem2 pointer position arguments");
            let observation = cx.host.input_observation(cx.client)?;
            ensure!(observation.client == cx.client, "CatSystem2 foreign input observation");
            cx.input.observe(&observation)?;
            let [x, y] = cx.input.fes_position();
            cx.machine.commit_native(None, &[(a[0], x.to_le_bytes().to_vec()), (a[1], y.to_le_bytes().to_vec())])?;
        }
        // 6569F0 → INPUT 137(ms, x | y << 16): only with setconf "autocur"
        // (B91BFC+77792), a foreground window and a target inside the virtual
        // canvas (677240) does the pointer start gliding toward the target.
        150 => {
            ensure!(a.len() == 3, "CatSystem2 pointer glide arguments");
            cx.machine.preflight_native(false, &[])?;
            let autocur = cx.settings.as_ref().context("CatSystem2 configuration unavailable for autocur")?.configuration(b"autocur", 0, 0)?;
            let (x, y) = (i32::from(a[0] as u16), i32::from(a[1] as u16));
            let inside = x < cx.canvas[0] as i32 && y < cx.canvas[1] as i32;
            if autocur != 0 && inside && cx.host.window_focused(cx.client)? != Some(false) {
                cx.state.input_task.glide = Some(Glide { start_ms: cx.host.now_ms(), remaining: i64::from(a[2] as i32), target: [x, y] });
            }
            cx.machine.acknowledge(None)?;
        }
        // 6568B0 → INPUT 145: clear every hover marker, forget the last pointer
        // position (this[1233..1234] = -1) so the next tick re-evaluates hover.
        286 => {
            ensure!(a.is_empty(), "CatSystem2 hover reset arguments");
            cx.machine.preflight_native(false, &[])?;
            route(cx, json!([{"op":"clear_hover_markers","reevaluate":true}]))?;
            cx.machine.acknowledge(None)?;
        }
        // 648B20 → 5DE8E0 case 97 → INPUT 146: the controller's hit mask becomes
        // the image's nonzero-alpha pixels (5E65B0), anchored at the receiver's
        // top-left (5E7860); a null handle removes it.
        394 => {
            ensure!(a.len() == 2, "CatSystem2 button mask arguments");
            cx.machine.preflight_native(false, &[])?;
            let Some(mut spec) = receiver(cx, a[0])? else {
                cx.machine.acknowledge(None)?;
                return Ok(true);
            };
            if a[1] == 0 {
                spec.coverage = None;
            } else {
                #[cfg(feature = "runtime")]
                {
                    let frame = cx.handles.images.get(&a[1]).with_context(|| format!("CatSystem2 button mask image handle {}", a[1]))?;
                    let rect = spec.rect.context("CatSystem2 pixel mask on a global click controller")?;
                    ensure!(rect.iter().all(|v| v.abs() < 100000) && rect[2] > 0 && rect[3] > 0 && rect[2] <= 8192 && rect[3] <= 8192, "CatSystem2 pixel mask needs a bounded controller rectangle");
                    let [iw, ih] = frame.image.size();
                    let rgba = frame.image.rgba();
                    let (w, h) = (rect[2] as u32, rect[3] as u32);
                    let mut coverage = vec![0u8; (w * h) as usize];
                    for y in 0..h.min(ih) {
                        for x in 0..w.min(iw) {
                            if rgba[((y * iw + x) * 4 + 3) as usize] != 0 {
                                coverage[(y * w + x) as usize] = 255;
                            }
                        }
                    }
                    spec.coverage = Some(crysirror_scene::ui::WidgetHitMask { size: [w, h], coverage: coverage.into(), threshold: 1, strict_bounds: false });
                }
                #[cfg(not(feature = "runtime"))]
                anyhow::bail!("CatSystem2 button pixel mask needs runtime image handles");
            }
            route(cx, json!([{"op":"put","receiver":spec}]))?;
            cx.machine.acknowledge(None)?;
        }
        // 656570 → INPUT 149: 677130 shows/hides the software cursor; this[1309]
        // keeps the accepted state for the idle auto-hide in 5E66C0.
        401 => {
            ensure!(a.len() == 1, "CatSystem2 cursor visibility arguments");
            cx.machine.preflight_native(false, &[])?;
            let visible = a[0] != 0;
            cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"ui":[{"op":"set_cursor_visible","visible":visible}]}))?)?;
            cx.state.input_task.cursor_visible = Some(visible);
            cx.machine.acknowledge(None)?;
        }
        // 656900 → INPUT 151: every key slot is polled again (memset32 of 80 ones).
        404 => {
            ensure!(a.is_empty(), "CatSystem2 key slot reset arguments");
            cx.machine.preflight_native(false, &[])?;
            let settings = cx.settings.as_mut().context("CatSystem2 key bindings unavailable")?;
            settings.input.polled.iter_mut().for_each(|slot| *slot = true);
            cx.machine.acknowledge(None)?;
        }
        // 6564D0 → INPUT 157 → 70F300 → 710240: the mouse device forgets its
        // accumulated wheel direction/notches (+0x604/+0x608, 6 up, 7 down).
        937 => {
            ensure!(a.is_empty(), "CatSystem2 wheel reset arguments");
            cx.machine.preflight_native(false, &[])?;
            let codes = [crysirror_core::service::buttons::WHEEL_UP, crysirror_core::service::buttons::WHEEL_DOWN].map(|m| m.trailing_zeros());
            route(cx, json!([{"op":"discard_queued","codes":codes}]))?;
            cx.machine.acknowledge(None)?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in the INPUT family"),
    }
    Ok(true)
}

/// 5E66C0's glide step: x += (target - x) / remaining, then remaining becomes
/// remaining + start - now (at least 1); a step taken with remaining <= 1 ends it.
pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    _tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    let Some(mut glide) = state.glide else { return Ok(()) };
    if glide.remaining <= 0 {
        state.glide = None;
        return Ok(());
    }
    let Some([x, y]) = host.input_observation(client)?.pointer else { return Ok(()) };
    let step = |from: i32, to: i32| from + ((i64::from(to) - i64::from(from)) / glide.remaining) as i32;
    host.warp_pointer(client, step(x, glide.target[0]), step(y, glide.target[1]))?;
    if glide.remaining <= 1 {
        state.glide = None;
    } else {
        let elapsed = now_ms.saturating_sub(glide.start_ms) as i64;
        glide.remaining = (glide.remaining - elapsed).max(1);
        state.glide = Some(glide);
    }
    Ok(())
}
