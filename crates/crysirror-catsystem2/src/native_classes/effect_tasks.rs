// SPDX-License-Identifier: MPL-2.0
//! Image transition tasks, classes 26 wipe, 27 draw, 28, 29 wipe2, 30 wipe3 and 31
//! (handlers 5EFD50, 5F0A00, 5F11F0, 5F1AA0, 5F23E0, 5F2F10). The original mixes bitmaps
//! each frame and uploads the result to the render manager's VRAM atlas (677090),
//! which Planes sample. Here VRAM is the main ImageList Canvas: the frame's image is
//! prepared by the shared transition worker and placed there as one draw record.
//! When the target of a mix is the list's own work bitmap, the old picture is laid
//! over the live Canvas with the same weights instead of reading the Canvas back.
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[196, 197, 200, 203];

/// Draw records above every script draw: priority is the key's high half.
const KEY_BASE: u32 = 0x7f00_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// 26 (5EFF10): clamp(phase - 256 + r, 0, 256), phase = f*512/d.
    Soft,
    /// 27 (5F0560): one weight f*256/d.
    Fade,
    /// 28 (5F13C0): the Soft table scaling the first image's alpha.
    SoftAlpha { inverse: bool },
    /// 29 (5F1C70): r >= 256 - f*256/d picks the second image.
    Hard,
    /// 30 (5F2670): each rule level ramps over `vague` frames after its own start.
    Ramp,
    /// 31 (5F31A0): the Ramp table scaling the first image's alpha.
    RampAlpha { inverse: bool },
}

#[derive(Clone)]
enum Second {
    Image(Arc<dyn ImageResource>),
    /// The list's work bitmap: its live Canvas is what the frame blends toward.
    List,
    None,
}

struct Effect {
    kind: Kind,
    duration: u32,
    vague: u32,
    first: Arc<dyn ImageResource>,
    second: Second,
    rule: Option<Arc<dyn ImageResource>>,
    /// Source corners of first, second and rule, in that order, already offset.
    corners: [[i32; 2]; 3],
    destination: [i32; 2],
    size: [i32; 2],
    speed: f32,
}

enum Phase {
    /// In submission order: first, second when it is an image, rule when present.
    Crop { jobs: Vec<Option<ImageJob>>, done: Vec<Option<Arc<dyn ImageResource>>> },
    Run { first: Arc<dyn ImageResource>, second: Option<Arc<dyn ImageResource>>, rule: Option<Arc<dyn ImageResource>>, started: u64, job: Option<(ImageJob, bool)> },
}

#[derive(Default)]
struct Task {
    effect: Option<Effect>,
    phase: Option<Phase>,
    /// Which of the two alternating image slots the list draw uses now.
    slot: bool,
    placed: Option<String>,
}

#[derive(Default)]
pub(crate) struct State {
    tasks: BTreeMap<u32, Task>,
    /// Destroyed while running: their overlay leaves on the next tick.
    retiring: Vec<(u32, Task)>,
    noted: bool,
}

fn kind_of(class: u32, flag: u32) -> Option<Kind> {
    Some(match class {
        26 => Kind::Soft,
        27 => Kind::Fade,
        28 => Kind::SoftAlpha { inverse: flag == 0 },
        29 => Kind::Hard,
        30 => Kind::Ramp,
        31 => Kind::RampAlpha { inverse: flag == 0 },
        _ => return None,
    })
}

/// Stored bitmap extent, by 68F5A0, of a handle or the list's work bitmap.
fn image(cx: &Ctx, handle: u32) -> Result<Arc<dyn ImageResource>> {
    Ok(cx.handles.images.get(&handle).with_context(|| format!("CatSystem2 transition image handle {handle:#x}"))?.image.clone())
}

/// 5EFB30, 5F07D0, 5F0FE0, 5F1880, 5F21A0, 5F2CE0: read the task's parameter block.
fn parameters(cx: &mut Ctx, class: u32, at: u32) -> Result<Effect> {
    use crate::native_surface::LIST_BITMAP;
    // Word layouts from 0x6C: duration, [vague,] first+x+y, [second+x+y,] [rule,] dest x+y,
    // (width, height written by the loader), [flag,] offset x+y.
    let count = match class { 26 | 29 => 14, 27 => 11, 28 => 12, 30 => 15, _ => 13 };
    let bytes = cx.machine.read_bytes(at, count * 4)?;
    let w = |i: usize| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    let i = |i: usize| w(i) as i32;
    let (duration, vague, first, fx, fy, second, sx, sy, rule, dest, flag, offset) = match class {
        26 | 29 => (w(0), 0, w(1), i(2), i(3), w(4), i(5), i(6), w(7), [i(8), i(9)], 0, [i(12), i(13)]),
        27 => (w(0), 0, w(1), i(2), i(3), w(4), i(5), i(6), 0, [i(7), i(8)], 0, [0, 0]),
        28 => (w(0), 0, w(1), i(2), i(3), 0, 0, 0, w(4), [i(5), i(6)], w(9), [i(10), i(11)]),
        30 => (w(0), w(1), w(2), i(3), i(4), w(5), i(6), i(7), w(8), [i(9), i(10)], 0, [i(13), i(14)]),
        _ => (w(0), w(1), w(2), i(3), i(4), 0, 0, 0, w(5), [i(6), i(7)], w(10), [i(11), i(12)]),
    };
    let kind = kind_of(class, flag).context("CatSystem2 transition class")?;
    let first_image = image(cx, first)?;
    let list = second & 0xff00_0000 == LIST_BITMAP;
    let second_image = if second == 0 { Second::None } else if list { Second::List } else { Second::Image(image(cx, second)?) };
    let rule_image = if rule == 0 { None } else { Some(image(cx, rule)?) };
    let first_size = first_image.size();
    let mut extent = [first_size[0] as i32 - fx, first_size[1] as i32 - fy];
    if let Second::Image(other) = &second_image {
        let size = other.size();
        extent = [extent[0].min(size[0] as i32 - sx), extent[1].min(size[1] as i32 - sy)];
    }
    if let Some(mask) = &rule_image {
        let size = mask.size();
        extent = [extent[0].min(size[0] as i32 - offset[0]), extent[1].min(size[1] as i32 - offset[1])];
    }
    extent = [(extent[0] - offset[0]).max(0), (extent[1] - offset[1]).max(0)];
    Ok(Effect {
        kind,
        duration: duration.max(1),
        vague: vague.max(1),
        first: first_image,
        second: second_image,
        rule: rule_image,
        corners: [[fx + offset[0], fy + offset[1]], [sx + offset[0], sy + offset[1]], offset],
        destination: [dest[0] + offset[0], dest[1] + offset[1]],
        size: extent,
        speed: 1.,
    })
}

fn crop(cx: &mut Ctx, source: &Arc<dyn ImageResource>, corner: [i32; 2], size: [i32; 2]) -> Result<ImageJob> {
    let input = ImageInput::Composite {
        base: Box::new(ImageInput::Solid { size: [size[0] as u32, size[1] as u32], color: [0; 4] }),
        layers: vec![ImageCompositeLayer { input: Box::new(ImageInput::Resource(source.clone())), source: [corner[0], corner[1], size[0], size[1]], destination: [0, 0], clip: None, opacity: 255, math: ImageCompositeMath::Copy }],
    };
    Ok(cx.host.image_metadata(cx.client)?.begin_process(input, Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    if cx.call.arguments.first().is_some_and(|task| cx.tasks.opaque_class(*task) == Some(18)) {
        super::shader_tasks::raster_native(cx)?;
        return Ok(true);
    }
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 effect task native {} arguments", cx.call.id);
    let id = a[0];
    let class = cx.tasks.opaque_class(id);
    let known = class.is_some_and(|class| (26..=31).contains(&class));
    match cx.call.id {
        // 641050: message 13 asks the class, then its parameter message takes a VM block.
        196 => {
            ensure!(a.len() == 2, "CatSystem2 effect parameter arguments");
            cx.machine.preflight_native(false, &[])?;
            if known {
                let effect = parameters(cx, class.unwrap(), a[1])?;
                let task = cx.state.effect_tasks.tasks.entry(id).or_default();
                task.effect = Some(effect);
                task.phase = None;
            } else if class.is_some() {
                note(cx, class.unwrap())?;
            }
            cx.machine.acknowledge(None)?;
        }
        // 6413F0 → 257: running, start = now, speed by effspd (live config 77780).
        197 => {
            cx.machine.preflight_native(false, &[])?;
            if known {
                let speed = match cx.settings.as_ref().map_or(Ok(0), |settings| settings.configuration(b"effspd", 0, 0))? {
                    1 => 1.5,
                    2 => 2.,
                    3 => 3.,
                    4 => 4.,
                    _ => 1.,
                };
                let Some(mut effect) = cx.state.effect_tasks.tasks.get_mut(&id).and_then(|task| task.effect.take()) else {
                    cx.machine.acknowledge(None)?;
                    return Ok(true);
                };
                // 5F2BC0 (class 31) does not scale time.
                effect.speed = if matches!(effect.kind, Kind::RampAlpha { .. }) { 1. } else { speed };
                let mut jobs = Vec::new();
                if effect.size[0] > 0 && effect.size[1] > 0 {
                    let (first, second, rule) = (effect.first.clone(), effect.second.clone(), effect.rule.clone());
                    jobs.push(crop(cx, &first, effect.corners[0], effect.size)?);
                    if let Second::Image(other) = &second {
                        jobs.push(crop(cx, other, effect.corners[1], effect.size)?);
                    }
                    if let Some(mask) = &rule {
                        jobs.push(crop(cx, mask, effect.corners[2], effect.size)?);
                    }
                }
                let task = cx.state.effect_tasks.tasks.get_mut(&id).unwrap();
                task.effect = Some(effect);
                let done = jobs.iter().map(|_| None).collect();
                task.phase = Some(Phase::Crop { jobs: jobs.into_iter().map(Some).collect(), done });
            }
            cx.machine.acknowledge(None)?;
        }
        // 641290 → 259: the running flag.
        200 => {
            cx.machine.preflight_native(true, &[])?;
            let running = cx.state.effect_tasks.tasks.get(&id).is_some_and(|task| task.phase.is_some());
            cx.machine.acknowledge(Some(u32::from(running)))?;
        }
        // 641020 → 261: a running task finishes on its final picture.
        203 => {
            cx.machine.preflight_native(false, &[])?;
            if cx.state.effect_tasks.tasks.get(&id).is_some_and(|task| task.phase.is_some()) {
                finish(cx.host, cx.client, cx.tasks, id, cx.state.effect_tasks.tasks.get_mut(&id).unwrap())?;
            }
            cx.machine.acknowledge(None)?;
        }
        other => bail!("CatSystem2 native {other} has no handler in the effect family"),
    }
    Ok(true)
}

fn note(cx: &mut Ctx, class: u32) -> Result<()> {
    if !core::mem::replace(&mut cx.state.effect_tasks.noted, true) {
        cx.host.append_log(cx.client, &format!("CatSystem2 effect task class {class} has no capability yet; its parameters are dropped"))?;
    }
    Ok(())
}

fn key(effect: &Effect) -> u32 {
    let [x, y] = effect.destination;
    KEY_BASE | ((x.wrapping_mul(31).wrapping_add(y) as u32) & 0xffff)
}

/// The 256 rule weights of a frame.
fn weights(effect: &Effect, frame: u32) -> Vec<u16> {
    let (d, v) = (effect.duration, effect.vague);
    let clamped = frame.min(d);
    let phase = (clamped as i64 * 512 / d as i64).clamp(0, 512);
    let fade = (clamped as i64 * 256 / d as i64).clamp(0, 256);
    (0..256i64)
        .map(|r| {
            let value = match effect.kind {
                Kind::Soft | Kind::SoftAlpha { .. } => (phase - 256 + r).clamp(0, 256),
                Kind::Fade => fade,
                Kind::Hard => if r >= 256 - fade { 256 } else { 0 },
                Kind::Ramp | Kind::RampAlpha { .. } => {
                    // 5F25C0: start[r] = d - r*d/255.
                    let start = d as i64 - r * d as i64 / 255;
                    if (frame as i64) < start {
                        0
                    } else if frame as i64 > start + v as i64 {
                        256
                    } else {
                        (frame as i64 - start) * 256 / v as i64
                    }
                }
            };
            value as u16
        })
        .collect()
}

fn end_frame(effect: &Effect) -> u32 {
    match effect.kind {
        Kind::Ramp | Kind::RampAlpha { .. } => effect.duration + effect.vague,
        _ => effect.duration,
    }
}

fn begin_frame(host: &mut dyn HostServices, client: ClientId, effect: &Effect, first: &Arc<dyn ImageResource>, second: &Option<Arc<dyn ImageResource>>, rule: &Option<Arc<dyn ImageResource>>, frame: u32) -> Result<ImageJob> {
    let mut table = weights(effect, frame);
    let operation = match effect.kind {
        Kind::SoftAlpha { inverse: false } | Kind::RampAlpha { inverse: false } => "alpha_scale",
        Kind::SoftAlpha { inverse: true } | Kind::RampAlpha { inverse: true } => "alpha_scale_inverse",
        _ if second.is_some() => "mix",
        // Old picture over the live Canvas: its alpha is what remains of the old picture.
        _ => {
            table.iter_mut().for_each(|w| *w = 256 - *w);
            "alpha_scale"
        }
    };
    let effects = serde_json::to_vec(&json!([{"kind":"transition","spec":{"progress":0.,"softness":0.,"blur":0.,"rule_table":{"weights":table,"channel":2,"operation":operation}}}]))?;
    let input = ImageInput::Transition { from: first.clone(), to: second.clone().unwrap_or_else(|| first.clone()), mask: rule.clone() };
    Ok(host.image_metadata(client)?.begin_process(input, Arc::from(effects)).map_err(anyhow::Error::from)?)
}

fn place(host: &mut dyn HostServices, client: ClientId, tasks: &mut crate::task_policy::NativeTaskPolicy, id: u32, effect: &Effect, task_slot: &mut bool, placed: &mut Option<String>, image: Arc<dyn ImageResource>) -> Result<()> {
    let Some(list) = tasks.main_image_list() else { return Ok(()) };
    *task_slot = !*task_slot;
    let slot = 0x5500_0000u32.checked_add(id.checked_mul(2).context("CatSystem2 effect image slot")?).context("CatSystem2 effect image slot")? + u32::from(*task_slot);
    let upload = host.publish_image_resource(client, slot, image)?;
    let path = upload.path.relative().to_owned();
    let publication = tasks.list_publication(list, client)?;
    publication.put(crate::native_image_publication::Draw { key: key(effect), image: path.clone(), size: upload.size, position: [effect.destination[0] as f32, effect.destination[1] as f32], pivot: [0.; 2], alpha: 255, blend: crysirror_scene::layer::Blend::Normal, visible: true, clip: None })?;
    publication.publish(host, client, Vec::new())?;
    if let Some(old) = placed.replace(path) {
        host.release_image_resource(client, &old)?;
    }
    Ok(())
}

/// 5EFEE0 and its siblings: the final picture stays in VRAM. Toward the live list
/// that is simply the list; toward an image, that image's region.
fn finish(host: &mut dyn HostServices, client: ClientId, tasks: &mut crate::task_policy::NativeTaskPolicy, id: u32, task: &mut Task) -> Result<()> {
    let phase = task.phase.take();
    let Some(effect) = task.effect.as_ref() else { return Ok(()) };
    match (&effect.second, phase) {
        (Second::List, _) => {
            if let Some(list) = tasks.main_image_list() {
                let publication = tasks.list_publication(list, client)?;
                if publication.remove(key(effect)) {
                    publication.publish(host, client, Vec::new())?;
                }
            }
            if let Some(old) = task.placed.take() {
                host.release_image_resource(client, &old)?;
            }
        }
        (Second::Image(_), Some(Phase::Run { second: Some(second), .. })) => {
            let (mut slot, mut placed) = (task.slot, task.placed.take());
            place(host, client, tasks, id, effect, &mut slot, &mut placed, second)?;
            task.slot = slot;
            task.placed = placed;
        }
        _ => {}
    }
    Ok(())
}

/// 5EFA10 and its siblings, once per frame.
pub(crate) fn tick(host: &mut dyn HostServices, client: ClientId, tasks: &mut crate::task_policy::NativeTaskPolicy, state: &mut State, now_ms: u64) -> Result<()> {
    for (id, mut task) in core::mem::take(&mut state.retiring) {
        finish(host, client, tasks, id, &mut task)?;
    }
    for (id, task) in &mut state.tasks {
        match task.phase.take() {
            None => {}
            Some(Phase::Crop { mut jobs, mut done }) => {
                let mut waiting = false;
                for (slot, out) in jobs.iter_mut().zip(done.iter_mut()) {
                    let Some(job) = slot.as_mut() else { continue };
                    match job.poll().map_err(anyhow::Error::from)? {
                        ImageProcessState::Pending => waiting = true,
                        ImageProcessState::Failed(e) => bail!("CatSystem2 transition crop: {e}"),
                        ImageProcessState::Ready(image) => {
                            *out = Some(image);
                            *slot = None;
                        }
                    }
                }
                if waiting {
                    task.phase = Some(Phase::Crop { jobs, done });
                    continue;
                }
                let Some(effect) = task.effect.as_ref() else { continue };
                if done.is_empty() {
                    continue;
                }
                let mut parts = done.into_iter().flatten();
                let first = parts.next().unwrap();
                let second = if matches!(effect.second, Second::Image(_)) { parts.next() } else { None };
                let rule = if effect.rule.is_some() { parts.next() } else { None };
                task.phase = Some(Phase::Run { first, second, rule, started: now_ms, job: None });
            }
            Some(Phase::Run { first, second, rule, started, job }) => {
                let Some(effect) = task.effect.as_ref() else { continue };
                let frame = (now_ms.saturating_sub(started) as f32 * effect.speed) as u32;
                let ended = frame >= end_frame(effect);
                let job = match job {
                    Some((mut job, last)) => match job.poll().map_err(anyhow::Error::from)? {
                        ImageProcessState::Pending => Some((job, last)),
                        ImageProcessState::Failed(e) => bail!("CatSystem2 transition frame: {e}"),
                        ImageProcessState::Ready(image) => {
                            let (mut slot, mut placed) = (task.slot, task.placed.take());
                            place(host, client, tasks, *id, effect, &mut slot, &mut placed, image)?;
                            task.slot = slot;
                            task.placed = placed;
                            if last {
                                task.phase = Some(Phase::Run { first, second, rule, started, job: None });
                                let is_alpha = matches!(effect.kind, Kind::SoftAlpha { .. } | Kind::RampAlpha { .. });
                                if is_alpha {
                                    task.phase = None;
                                } else {
                                    finish(host, client, tasks, *id, task)?;
                                }
                                continue;
                            }
                            None
                        }
                    },
                    None => None,
                };
                let job = match job {
                    Some(job) => Some(job),
                    None => {
                        let alpha = matches!(effect.kind, Kind::SoftAlpha { .. } | Kind::RampAlpha { .. });
                        if ended && !alpha {
                            task.phase = Some(Phase::Run { first, second, rule, started, job: None });
                            finish(host, client, tasks, *id, task)?;
                            continue;
                        }
                        Some((begin_frame(host, client, effect, &first, &second, &rule, frame.min(end_frame(effect)))?, ended))
                    }
                };
                task.phase = Some(Phase::Run { first, second, rule, started, job });
            }
        }
    }
    Ok(())
}

/// Native 67 destroyed task `id`. A picture already in VRAM stays there; an
/// unfinished overlay of the live list goes with the task.
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    let _ = (host, client);
    if let Some(task) = state.tasks.remove(&id).filter(|task| task.phase.is_some()) {
        state.retiring.push((id, task));
    }
    Ok(())
}
