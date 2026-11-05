// SPDX-License-Identifier: MPL-2.0
//! Screen effect tasks (classes 35..58) that register one frame hook with the render
//! manager (67FE70). Scene transitions are the classes this title uses: rdraw (38, 5F8F50),
//! rwipe (39, 5F9920), rwmos (50, 5FB110) and rfmos (51). Each fades the previous frame out
//! under a z band; here they are shared screen transitions, which also cover what the
//! original keeps above the band (see HANDOFF). A transition starts when the script hands over its
//! parameters, the last point before the new picture is presented.
//! wbreak (52) and the wave wipes (57, 58) use the same timing, but fade instead of their own
//! mesh motion. blur (35) and mosaic (36) run until disabled, as `Paint::Backdrop` layers at their
//! band's upper depth. Raster (18) distorts its parent Plane.
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[520, 521, 522, 523, 524, 527, 528, 530, 531, 853, 854, 855, 856, 857, 860, 861, 905, 967, 968, 970, 971, 972];

fn effect_layer(task: u32) -> Result<u32> {
    0x1800_0000u32.checked_add(task).filter(|v| *v < 0x1a00_0000).context("CatSystem2 screen effect layer range")
}

const MASK_SLOT: u32 = 0x4b80_0000;

#[derive(Clone, Debug, Default, PartialEq)]
struct Effect {
    duration_ms: u64,
    /// Task +116 with its end time (+260..+264).
    until_ms: Option<u64>,
    /// blur and mosaic: +116 without an end time, and the layer that shows it.
    enabled: bool,
    /// The layer to put when enabled, and blur's opacity for the frames after the first
    /// (+532 skips the first feedback).
    spec: Option<serde_json::Value>,
    depth: f32,
    opacity_next: Option<u8>,
}

/// Class 18 (5EDC00, 5EDED0, tick 5EDD30): a wave over its parent Plane.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Raster {
    /// +0x70 mode: raster_x, raster_y, raster_f; +0x7C amplitude, +0x80 waves per pixel,
    /// +0x84 period in frames, +0x8C time.
    vertical: bool,
    amplitude: f32,
    per_pixel: f32,
    period: f32,
    time: f32,
    running: bool,
    last_ms: u64,
}

/// Class 52 (5FDA10, hook 5FE650): the captured screen breaks into triangles flying outward.
#[derive(Clone, Debug, PartialEq)]
struct Shard {
    /// The triangle's corners relative to its centre, and its texture coordinates.
    corners: [[f32; 3]; 3],
    uvs: [[f32; 2]; 3],
    position: [f32; 3],
    velocity: [f32; 2],
    spin: [f32; 3],
    /// +64: the running rotation.
    matrix: [[f32; 3]; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Shatter {
    shards: Vec<Shard>,
    size: [f32; 2],
    band: f32,
    duration: f32,
    start_ms: Option<u64>,
    elapsed: f32,
}

fn rotate(matrix: &[[f32; 3]; 3], turn: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            out[row][column] = (0..3).map(|k| matrix[row][k] * turn[k][column]).sum();
        }
    }
    out
}

/// 5FDA10: two triangles per cell; each drifts away from the centre, faster with distance.
fn shatter(size: [f32; 2], cell: [u32; 2], speed: f32, spin_degrees: f32, mut draw: impl FnMut() -> Result<f32>) -> Result<Vec<Shard>> {
    let (cell_w, cell_h) = (cell[0].max(1) as f32, cell[1].max(1) as f32);
    let centre = [size[0] * 0.5, size[1] * 0.5];
    let farthest = (centre[0] * centre[0] + centre[1] * centre[1]).sqrt().max(1.);
    let spin = spin_degrees.to_radians();
    let mut out = Vec::new();
    let mut y = 0.;
    while y < size[1] {
        let mut x = 0.;
        while x < size[0] {
            let (r, b) = (x + cell_w, y + cell_h);
            for triangle in [[[x, y], [r, y], [x, b]], [[r, y], [x, b], [r, b]]] {
                let c = [(triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3., (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.];
                let away = [c[0] - centre[0], c[1] - centre[1]];
                let distance = (away[0] * away[0] + away[1] * away[1]).sqrt();
                let tilt = distance / farthest * std::f32::consts::FRAC_PI_4;
                let heading = away[0].atan2(-away[1]);
                let thrust = draw()? * speed * tilt.sin();
                let mut spins = [0.; 3];
                for value in &mut spins {
                    *value = spin * 0.5 - draw()? * spin;
                }
                out.push(Shard {
                    corners: triangle.map(|p| [p[0] - c[0], p[1] - c[1], 0.]),
                    uvs: triangle.map(|p| [p[0] / size[0], p[1] / size[1]]),
                    position: [c[0], c[1], 0.],
                    velocity: [thrust * heading.sin(), -thrust * heading.cos()],
                    spin: spins,
                    matrix: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                });
            }
            x = r;
        }
        y += cell_h;
    }
    Ok(out)
}

/// Classes 57 (5FEE20, 5FFA60) and 58 (6002F0, 601030): a grid over the captured screen.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Sheet {
    /// The page rolls up over a cylinder and leaves along its tangent.
    Curl { radius: f32, arc: f32 },
    /// Each cell turns over as the front passes; its back shows what lies beneath.
    Flip { sweep: f32 },
}

#[derive(Clone, Debug, PartialEq)]
struct Grid {
    sheet: Sheet,
    size: [f32; 2],
    cell: [f32; 2],
    angle: f32,
    band: f32,
    duration: f32,
    start_ms: Option<u64>,
}

/// 67E910 and 67EA30: a row vector turned about X, and about Z.
fn turn_x(p: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]
}
fn turn_z(p: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos, p[2]]
}
/// 678170 with its 2000-unit near distance, about the screen centre.
fn projected(p: [f32; 3]) -> [f32; 2] {
    let scale = 2000. / (p[2] + 2000.).max(1.);
    [p[0] * scale, p[1] * scale]
}

impl Grid {
    /// Every triangle's corners and texture coordinates at time `t` in 0..=1.
    fn triangles(&self, t: f32) -> Vec<([[f32; 2]; 3], [[f32; 2]; 3])> {
        let [w, h] = self.size;
        let centre = [w * 0.5, h * 0.5];
        // The rotated extent the moving front travels across.
        let turned = |x: f32, y: f32| turn_z([x - centre[0], y - centre[1], 0.], -self.angle);
        let corners = [turned(0., 0.), turned(w, 0.), turned(0., h), turned(w, h)];
        let top = corners.iter().map(|c| c[1]).fold(f32::MAX, f32::min);
        let bottom = corners.iter().map(|c| c[1]).fold(f32::MIN, f32::max);
        let mut out = Vec::new();
        let (cell_w, cell_h) = (self.cell[0].max(1.), self.cell[1].max(1.));
        let mut y = 0.;
        while y < h {
            let mut x = 0.;
            while x < w {
                let (r, b) = ((x + cell_w).min(w), (y + cell_h).min(h));
                let quad = [[x, y], [r, y], [x, b], [r, y], [r, b], [x, b]];
                let uv = quad.map(|p| [p[0] / w, p[1] / h]);
                match self.sheet {
                    Sheet::Curl { radius, arc } => {
                        let front = t * ((top - radius) - bottom) + bottom;
                        let circumference = radius * std::f32::consts::TAU;
                        let arc_length = arc * radius;
                        let arc_end = turn_x([0., 0., radius], arc);
                        let tangent = turn_x([0., 1., 0.], arc);
                        let place = |p: [f32; 2]| -> [f32; 2] {
                            let v = turned(p[0], p[1]);
                            if v[1] < front {
                                return p;
                            }
                            let along = v[1] - front;
                            let q = if arc_length <= along {
                                let rest = along - arc_length;
                                [v[0] + arc_end[0] + tangent[0] * rest, front + arc_end[1] + tangent[1] * rest, arc_end[2] - radius + tangent[2] * rest]
                            } else {
                                let roll = turn_x([0., 0., radius], along / circumference.max(1.) * std::f32::consts::TAU);
                                [v[0] + roll[0], front + roll[1], roll[2] - radius]
                            };
                            let flat = projected(q);
                            let back = turn_z([flat[0], flat[1], 0.], self.angle);
                            [back[0] + centre[0], back[1] + centre[1]]
                        };
                        let placed = quad.map(place);
                        out.push(([placed[0], placed[1], placed[2]], [uv[0], uv[1], uv[2]]));
                        out.push(([placed[3], placed[4], placed[5]], [uv[3], uv[4], uv[5]]));
                    }
                    Sheet::Flip { sweep } => {
                        let front = t * (top - bottom) + bottom;
                        let c = [(x + r) * 0.5, (y + b) * 0.5];
                        let along = turned(c[0], c[1])[1] - front;
                        let turn = if along < 0. { 0. } else if sweep <= along { std::f32::consts::PI } else { along / sweep * std::f32::consts::PI };
                        // Past a quarter turn only the back would show, and it is what lies below.
                        if turn < std::f32::consts::FRAC_PI_2 {
                            let place = |p: [f32; 2]| -> [f32; 2] {
                                let local = turn_z([p[0] - c[0], p[1] - c[1], 0.], -self.angle);
                                let flat = projected(turn_z(turn_x(local, turn), self.angle));
                                [flat[0] + c[0], flat[1] + c[1]]
                            };
                            let placed = quad.map(place);
                            out.push(([placed[0], placed[1], placed[2]], [uv[0], uv[1], uv[2]]));
                            out.push(([placed[3], placed[4], placed[5]], [uv[3], uv[4], uv[5]]));
                        }
                    }
                }
                x = r;
            }
            y = (y + cell_h).min(h);
        }
        out
    }
}

fn mesh_layer(task: u32) -> Result<u32> {
    0x1a00_0000u32.checked_add(task).filter(|v| *v < 0x1c00_0000).context("CatSystem2 screen mesh layer range")
}

#[derive(Default)]
pub(crate) struct State {
    effects: BTreeMap<u32, Effect>,
    rasters: BTreeMap<u32, Raster>,
    shatters: BTreeMap<u32, Shatter>,
    grids: BTreeMap<u32, Grid>,
}

fn put_effect(cx: &mut Ctx, task: u32) -> Result<()> {
    let id = effect_layer(task)?;
    let Some(effect) = cx.state.shader_tasks.effects.get(&task) else { return Ok(()) };
    let Some(mut spec) = effect.spec.clone() else { return Ok(()) };
    spec["id"] = json!(id);
    let depth = effect.depth;
    cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":id},{"op":"put","layer":spec},{"op":"set_depth","id":id,"depth":depth}]}))?)?;
    Ok(())
}

/// Natives 196, 197, 200, 201 and 203 addressed to a class 18 task.
pub(crate) fn raster_native(cx: &mut Ctx) -> Result<()> {
    let a = cx.call.arguments.clone();
    let task = a[0];
    let word = |cx: &Ctx, at: u32| cx.machine.read_word(at).map(f32::from_bits);
    let returns = cx.call.id == 200;
    if !returns {
        cx.machine.preflight_native(false, &[])?;
    }
    match cx.call.id {
        // 263 → 5EDE20.
        196 => {
            ensure!(a.len() == 2, "CatSystem2 raster parameters");
            let mode = cx.machine.read_word(a[1])?;
            let amplitude = word(cx, a[1] + 8)?;
            let wave = word(cx, a[1] + 12)?;
            let period = word(cx, a[1] + 16)? / 60. * cx.settings.as_ref().map_or(60., |s| s.fps as f32);
            let raster = cx.state.shader_tasks.rasters.entry(task).or_default();
            raster.vertical = mode == 1;
            raster.amplitude = amplitude;
            raster.per_pixel = if wave != 0. { 1. / wave } else { 0. };
            raster.period = period.max(1.);
        }
        // 257.
        197 => {
            let now = cx.host.now_ms();
            let raster = cx.state.shader_tasks.rasters.entry(task).or_default();
            raster.running = true;
            raster.time = 0.;
            raster.last_ms = now;
        }
        // 264 when a1 == 0: new amplitude, wave and period while it runs.
        201 => {
            ensure!(a.len() == 3, "CatSystem2 raster update");
            if a[1] == 0 {
                let amplitude = word(cx, a[2] + 8)?;
                let wave = word(cx, a[2] + 12)?;
                let period = word(cx, a[2] + 16)? / 60. * cx.settings.as_ref().map_or(60., |s| s.fps as f32);
                let raster = cx.state.shader_tasks.rasters.entry(task).or_default();
                raster.amplitude = amplitude;
                raster.per_pixel = if wave != 0. { 1. / wave } else { 0. };
                raster.period = period.max(1.);
            }
        }
        // 259.
        200 => {
            cx.machine.preflight_native(true, &[])?;
            let running = cx.state.shader_tasks.rasters.get(&task).is_some_and(|raster| raster.running);
            cx.machine.acknowledge(Some(u32::from(running)))?;
            return Ok(());
        }
        // 258.
        _ => {
            if let Some(raster) = cx.state.shader_tasks.rasters.get_mut(&task) {
                raster.running = false;
            }
            if let Some(payload) = cx.tasks.parent_of(task).ok().and_then(|parent| cx.tasks.plane(parent).ok().and_then(|plane| plane.payload)) {
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"set_filter","id":payload,"filter":null}]}))?)?;
            }
        }
    }
    cx.machine.acknowledge(None)?;
    Ok(())
}

fn start(cx: &mut Ctx, task: u32, duration: u32, effect: serde_json::Value) -> Result<()> {
    // 5DCCE0: the task clock counts frames.
    let duration_ms = (u64::from(duration) * 1000 / 60).clamp(1, 600_000);
    cx.state.shader_tasks.effects.entry(task).or_default().duration_ms = duration_ms;
    cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"transition","transition":{"effect":effect,"duration_ms":duration_ms}}]}))?)?;
    Ok(())
}

/// 5F9620: the rule image; 905 reads it the other way round.
fn rule_mask(cx: &mut Ctx, image: u32, inverted: bool) -> Result<serde_json::Value> {
    let Some(frame) = cx.handles.images.get(&image) else { return Ok(json!({"kind":"crossfade"})) };
    let slot = MASK_SLOT | (image & 0x7f_ffff);
    let asset = if inverted {
        let [width, height] = frame.image.size();
        let mut rgba = frame.image.rgba().to_vec();
        for pixel in rgba.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] {
                *channel = 255 - *channel;
            }
        }
        cx.host.upload_rgba(cx.client, slot, width, height, &rgba)?.relative().to_owned()
    } else {
        let image = frame.image.clone();
        cx.host.publish_image_resource(cx.client, slot, image)?.path.relative().to_owned()
    };
    Ok(json!({"kind":"mask","asset":asset,"fade":4}))
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 screen effect native {} arguments", cx.call.id);
    let id = cx.call.id;
    let class = cx.tasks.opaque_class(a[0]);
    // 642A10: 377 answers +116; the hook clears it at the end time (phase 5).
    if id == 520 {
        cx.machine.preflight_native(true, &[])?;
        let now = cx.host.now_ms();
        let mut running = false;
        if let Some(effect) = cx.state.shader_tasks.effects.get_mut(&a[0]) {
            running = effect.enabled;
            match effect.until_ms {
                Some(until) if now < until => running = true,
                _ => effect.until_ms = None,
            }
        }
        cx.machine.acknowledge(Some(u32::from(running)))?;
        return Ok(true);
    }
    cx.machine.preflight_native(false, &[])?;
    let wanted = match id {
        521 | 522 => 35,
        523 | 524 => 36,
        527 | 528 => 38,
        530 | 531 | 905 => 39,
        853..=855 => 50,
        856 | 857 => 51,
        860 | 861 => 52,
        967 | 968 => 57,
        _ => 58,
    };
    // 5DC9D0: another class has no such case.
    if class != Some(wanted) {
        cx.machine.acknowledge(None)?;
        return Ok(true);
    }
    match id {
        // 642890 → 384: duration a1, z band a2..a3.
        527 => {
            ensure!(a.len() == 4, "CatSystem2 rdraw arguments");
            start(cx, a[0], a[1], json!({"kind":"crossfade"}))?;
        }
        // 642F60 → 387 / 642EB0 → 387 with its reverse word set: plus rule image a4.
        530 | 905 => {
            ensure!(a.len() == 5, "CatSystem2 rwipe arguments");
            let effect = rule_mask(cx, a[4], id == 905)?;
            start(cx, a[0], a[1], effect)?;
        }
        // 642DF0 → 516 / 642940 → 519: duration a1, band a2..a3, cell sizes after them.
        853 | 856 => {
            ensure!(a.len() >= 6, "CatSystem2 mosaic transition arguments");
            start(cx, a[0], a[1], json!({"kind":"mosaic","size":a[4].max(a[5]).max(1)}))?;
        }
        // 642DB0 → 517: rebuilds the original's grid from the parameters already held.
        854 => {}
        // 642900, 642FD0, 642E70, 6429D0 → 385, 388, 518, 520: +116 = a1, end time = now + duration.
        528 | 531 | 855 | 857 => {
            ensure!(a.len() == 2, "CatSystem2 screen effect enable arguments");
            let now = cx.host.now_ms();
            let effect = cx.state.shader_tasks.effects.entry(a[0]).or_default();
            effect.until_ms = (a[1] != 0).then_some(now + effect.duration_ms);
        }
        // 642CD0 → 521 (wbreak), 642B50 → 390, 6426D0 → 394: duration a1.
        // 642CD0: a2 unused, a3 band, a1 duration, a4..a5 cell, a6 speed, a7 spin.
        860 => {
            ensure!(a.len() == 8, "CatSystem2 wbreak arguments");
            let size = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            // A local generator: the session streams (B93150) are not reachable from here, so a
            // given break differs from the original's draw for draw.
            let mut seed = a[0].wrapping_mul(2654435761) | 1;
            let shards = shatter(size, [a[4], a[5]], f32::from_bits(a[6]), f32::from_bits(a[7]), || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                Ok((seed >> 8) as f32 / (1u32 << 24) as f32)
            })?;
            let duration = a[1].max(1) as f32;
            cx.state.shader_tasks.shatters.insert(a[0], Shatter { shards, size, band: a[3] as i32 as f32, duration, start_ms: None, elapsed: 0. });
            cx.state.shader_tasks.effects.entry(a[0]).or_default().duration_ms = (u64::from(a[1]) * 1000 / 60).clamp(1, 600_000);
        }
        // 642B50: a1 duration, a2 angle, a3 radius, a4 arc, a5..a6 band.
        // 6426D0: a1 duration, a2..a3 cell, a4 unused, a5 sweep, a6 angle, a8..a9 band.
        967 | 971 => {
            ensure!(a.len() >= 2, "CatSystem2 timed effect arguments");
            let size = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            let grid = if cx.call.id == 967 {
                ensure!(a.len() == 7, "CatSystem2 curl arguments");
                Grid { sheet: Sheet::Curl { radius: f32::from_bits(a[3]), arc: f32::from_bits(a[4]) }, size, cell: [16., 16.], angle: f32::from_bits(a[2]), band: a[6] as i32 as f32, duration: a[1].max(1) as f32, start_ms: None }
            } else {
                ensure!(a.len() == 10, "CatSystem2 flip arguments");
                Grid { sheet: Sheet::Flip { sweep: f32::from_bits(a[5]) }, size, cell: [a[2] as f32, a[3] as f32], angle: f32::from_bits(a[6]), band: a[9] as i32 as f32, duration: a[1].max(1) as f32, start_ms: None }
            };
            cx.state.shader_tasks.grids.insert(a[0], grid);
            cx.state.shader_tasks.effects.entry(a[0]).or_default().duration_ms = (u64::from(a[1]) * 1000 / 60).clamp(1, 600_000);
        }
        968 | 972 if cx.state.shader_tasks.grids.contains_key(&a[0]) => {
            ensure!(a.len() == 2, "CatSystem2 sheet enable arguments");
            let (capture, mesh) = (effect_layer(a[0])?, mesh_layer(a[0])?);
            let now = cx.host.now_ms();
            let [w, h] = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            let grid = cx.state.shader_tasks.grids.get_mut(&a[0]).unwrap();
            let band = grid.band;
            if a[1] != 0 {
                grid.start_ms = Some(now);
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[
                    {"op":"remove","id":capture},
                    {"op":"put","layer":{"id":capture,"rect":[0,0,w,h],"paint":{"kind":"backdrop","history":true},"opacity":0}},
                    {"op":"set_depth","id":capture,"depth":band},
                    {"op":"put","layer":{"id":mesh,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,0]}}},
                    {"op":"set_depth","id":mesh,"depth":band - 1.},
                ]}))?)?;
            } else if grid.start_ms.take().is_some() {
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":capture},{"op":"remove","id":mesh}]}))?)?;
            }
            let duration = cx.state.shader_tasks.effects.get(&a[0]).map_or(0, |effect| effect.duration_ms);
            cx.state.shader_tasks.effects.entry(a[0]).or_default().until_ms = (a[1] != 0).then_some(now + duration);
        }
        // 642690 → 393: nothing but the original's mesh rebuild.
        970 => {}
        // 642D70, 642BE0, 642780 → 522, 391, 395.
        // 642D70 → 522: the capture layer goes in below the band, and the clock starts.
        861 if cx.state.shader_tasks.shatters.contains_key(&a[0]) => {
            ensure!(a.len() == 2, "CatSystem2 wbreak enable arguments");
            let (capture, mesh) = (effect_layer(a[0])?, mesh_layer(a[0])?);
            let now = cx.host.now_ms();
            let [w, h] = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            let shatter = cx.state.shader_tasks.shatters.get_mut(&a[0]).unwrap();
            let band = shatter.band;
            if a[1] != 0 {
                shatter.start_ms = Some(now);
                shatter.elapsed = 0.;
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[
                    {"op":"remove","id":capture},
                    {"op":"put","layer":{"id":capture,"rect":[0,0,w,h],"paint":{"kind":"backdrop","history":true},"opacity":0}},
                    {"op":"set_depth","id":capture,"depth":band},
                    {"op":"put","layer":{"id":mesh,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,0]}}},
                    {"op":"set_depth","id":mesh,"depth":band - 1.},
                ]}))?)?;
            } else if shatter.start_ms.take().is_some() {
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":capture},{"op":"remove","id":mesh}]}))?)?;
            }
            let duration = cx.state.shader_tasks.effects.get(&a[0]).map_or(0, |effect| effect.duration_ms);
            cx.state.shader_tasks.effects.entry(a[0]).or_default().until_ms = (a[1] != 0).then_some(now + duration);
        }
        861 | 968 | 972 => {
            ensure!(a.len() == 2, "CatSystem2 timed effect enable arguments");
            let now = cx.host.now_ms();
            let effect = cx.state.shader_tasks.effects.entry(a[0]).or_default();
            effect.until_ms = (a[1] != 0).then_some(now + effect.duration_ms);
        }
        // 642540 → 378: matrix a1, offset a2..a4, ARGB a5, band a6..a7. Each frame the last
        // captured band returns through that matrix at that alpha, so it trails.
        521 => {
            ensure!(a.len() == 8, "CatSystem2 blur arguments");
            let matrix: Vec<f32> = cx.machine.read_bytes(a[1], 64)?.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            ensure!(matrix[3] == 0. && matrix[7] == 0. && matrix[11] == 0. && matrix[15] == 1., "CatSystem2 blur matrix with a perspective row");
            let [w, h] = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            let f = |at: usize| f32::from_bits(a[at]);
            let alpha = (a[5] >> 24) as u8;
            let effect = cx.state.shader_tasks.effects.entry(a[0]).or_default();
            effect.depth = f(7);
            effect.opacity_next = Some(alpha);
            effect.spec = Some(json!({"id":0,"rect":[f(2),f(3),w,h],"paint":{"kind":"backdrop","history":true},"opacity":0,"transform":{"pivot_x":w/2.,"pivot_y":h/2.,"local_matrix":matrix}}));
            if effect.enabled {
                put_effect(cx, a[0])?;
            }
        }
        // 642A60 → 380: rectangle a1..a4, cell size a5..a6, band a7..a8.
        523 => {
            ensure!(a.len() == 10, "CatSystem2 mosaic arguments");
            let f = |at: usize| f32::from_bits(a[at]);
            let [w, h] = [cx.canvas[0] as f32, cx.canvas[1] as f32];
            let (cell_w, cell_h) = (f(5).max(1.), f(6).max(1.));
            let effect = cx.state.shader_tasks.effects.entry(a[0]).or_default();
            effect.depth = f(8);
            effect.opacity_next = None;
            let uv = [f(1) / w, f(2) / h, (f(1) + f(3)) / w, (f(2) + f(4)) / h];
            effect.spec = Some(json!({"id":0,"rect":[f(1),f(2),f(3),f(4)],"uv":uv,"paint":{"kind":"backdrop","history":false},"filter":{"distortion":{"kind":"mosaic","cut":(cell_w / w).clamp(1e-5,1.),"aspect":(cell_h / h) / (cell_w / w)}}}));
            if effect.enabled {
                put_effect(cx, a[0])?;
            }
        }
        // 642610 → 379 / 642AD0 → 381: on or off.
        522 | 524 => {
            ensure!(a.len() == 2, "CatSystem2 screen effect enable arguments");
            let enabled = a[1] != 0;
            let effect = cx.state.shader_tasks.effects.entry(a[0]).or_default();
            effect.enabled = enabled;
            if enabled {
                put_effect(cx, a[0])?;
            } else {
                let layer = effect_layer(a[0])?;
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":layer}]}))?)?;
            }
        }
        _ => bail!("CatSystem2 screen effect native {id} is not implemented"),
    }
    cx.machine.acknowledge(None)?;
    Ok(true)
}

pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    // +532: blur's first enabled frame shows no feedback.
    for (task, effect) in &mut state.effects {
        if effect.enabled {
            if let Some(opacity) = effect.opacity_next.take() {
                host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"set_opacity","id":effect_layer(*task)?,"opacity":opacity}]}))?)?;
            }
        }
    }
    // 5FE650 case 3: spin and drift by the frames since the last draw, then fade.
    for (task, shatter) in &mut state.shatters {
        let Some(start) = shatter.start_ms else { continue };
        let elapsed = (now_ms.saturating_sub(start) as f32 * 60. / 1000.).min(shatter.duration);
        let step = elapsed - shatter.elapsed;
        shatter.elapsed = elapsed;
        let mesh = mesh_layer(*task)?;
        if elapsed >= shatter.duration {
            shatter.start_ms = None;
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":effect_layer(*task)?},{"op":"remove","id":mesh}]}))?)?;
            continue;
        }
        let alpha = 1. - elapsed / shatter.duration;
        let [w, h] = shatter.size;
        let mut part = crysirror_puppet::FramePart { id: "shards".into(), texture: 0, positions: Vec::new(), uvs: Vec::new(), indices: Vec::new(), colors: Vec::new(), blend: crysirror_puppet::PartBlend::Normal, masks: Vec::new(), invert_mask: false, visible: true };
        for shard in &mut shatter.shards {
            let (sx, cx) = (shard.spin[0] * step).sin_cos();
            let (sy, cy) = (shard.spin[1] * step).sin_cos();
            let (sz, cz) = (shard.spin[2] * step).sin_cos();
            shard.matrix = rotate(&shard.matrix, &[[1., 0., 0.], [0., cx, sx], [0., -sx, cx]]);
            shard.matrix = rotate(&shard.matrix, &[[cy, 0., -sy], [0., 1., 0.], [sy, 0., cy]]);
            shard.matrix = rotate(&shard.matrix, &[[cz, sz, 0.], [-sz, cz, 0.], [0., 0., 1.]]);
            shard.position[0] += shard.velocity[0] * step;
            shard.position[1] += shard.velocity[1] * step;
            let base = part.positions.len() as u32;
            for (corner, uv) in shard.corners.iter().zip(&shard.uvs) {
                let m = &shard.matrix;
                let x = corner[0] * m[0][0] + corner[1] * m[1][0] + shard.position[0];
                let y = corner[0] * m[0][1] + corner[1] * m[1][1] + shard.position[1];
                part.positions.push([x, y]);
                part.uvs.push(*uv);
                part.colors.push([1., 1., 1., alpha]);
            }
            part.indices.extend([base, base + 1, base + 2]);
        }
        let frame = crysirror_puppet::PuppetFrame { size: [w, h], parts: vec![part], textures: vec!["backdrop:frame".to_owned()] };
        host.present_puppet_frame(client, mesh, &serde_json::to_vec(&frame)?)?;
    }
    // 5FFA60, 601030: the grid at this moment of its travel; 5FF8C0 case 5 ends it.
    for (task, grid) in &mut state.grids {
        let Some(start) = grid.start_ms else { continue };
        let elapsed = now_ms.saturating_sub(start) as f32 * 60. / 1000.;
        let mesh = mesh_layer(*task)?;
        if elapsed >= grid.duration {
            grid.start_ms = None;
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":effect_layer(*task)?},{"op":"remove","id":mesh}]}))?)?;
            continue;
        }
        let mut part = crysirror_puppet::FramePart { id: "sheet".into(), texture: 0, positions: Vec::new(), uvs: Vec::new(), indices: Vec::new(), colors: Vec::new(), blend: crysirror_puppet::PartBlend::Normal, masks: Vec::new(), invert_mask: false, visible: true };
        for (corners, uvs) in grid.triangles(elapsed / grid.duration) {
            let base = part.positions.len() as u32;
            part.positions.extend(corners);
            part.uvs.extend(uvs);
            part.colors.extend([[1., 1., 1., 1.]; 3]);
            part.indices.extend([base, base + 1, base + 2]);
        }
        let frame = crysirror_puppet::PuppetFrame { size: grid.size, parts: vec![part], textures: vec!["backdrop:frame".to_owned()] };
        host.present_puppet_frame(client, mesh, &serde_json::to_vec(&frame)?)?;
    }
    // 5EDD30: the raster clock wraps at its period.
    for (task, raster) in &mut state.rasters {
        if !raster.running {
            continue;
        }
        raster.time = (raster.time + now_ms.saturating_sub(raster.last_ms) as f32 * 60. / 1000.) % raster.period;
        raster.last_ms = now_ms;
        let Ok(parent) = tasks.parent_of(*task) else { continue };
        let Ok(plane) = tasks.plane(parent) else { continue };
        let Some(payload) = plane.payload else { continue };
        let size = plane.pose.size;
        let (across, along) = if raster.vertical { (size[1], size[0]) } else { (size[0], size[1]) };
        if across < 1. {
            continue;
        }
        let distortion = json!({"kind":"wave","vertical":raster.vertical,"amplitude":(raster.amplitude / across).clamp(-1., 1.),"cycles":raster.per_pixel * along,"phase":raster.time / raster.period});
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"set_filter","id":payload,"filter":{"distortion":distortion}}]}))?)?;
    }
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    state.rasters.remove(&id);
    let sheet_running = state.grids.remove(&id).is_some_and(|grid| grid.start_ms.is_some());
    if state.shatters.remove(&id).is_some_and(|shatter| shatter.start_ms.is_some()) || sheet_running {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":effect_layer(id)?},{"op":"remove","id":mesh_layer(id)?}]}))?)?;
    }
    if state.effects.remove(&id).is_some_and(|effect| effect.enabled) {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":effect_layer(id)?}]}))?)?;
    }
    Ok(())
}

#[cfg(test)]
mod shatter_tests {
    use super::shatter;

    #[test]
    fn a_grid_makes_two_triangles_per_cell_moving_outward() {
        let shards = shatter([100., 100.], [50, 50], 10., 0., || Ok(1.)).unwrap();
        assert_eq!(shards.len(), 8);
        let top_left = &shards[0];
        assert!(top_left.velocity[0] < 0. && top_left.velocity[1] < 0.);
        let bottom_right = &shards[7];
        assert!(bottom_right.velocity[0] > 0. && bottom_right.velocity[1] > 0.);
    }
}

#[cfg(test)]
mod grid_tests {
    use super::{Grid, Sheet};

    fn grid(sheet: Sheet) -> Grid {
        Grid { sheet, size: [100., 100.], cell: [50., 50.], angle: 0., band: 0., duration: 10., start_ms: None }
    }

    #[test]
    fn a_curl_leaves_the_page_flat_before_the_front_arrives() {
        let triangles = grid(Sheet::Curl { radius: 10., arc: 1. }).triangles(0.);
        assert_eq!(triangles.len(), 8);
        assert_eq!(triangles[0].0[0], [0., 0.]);
    }

    #[test]
    fn a_flip_has_dropped_every_cell_by_the_end() {
        let sheet = Sheet::Flip { sweep: 10. };
        assert_eq!(grid(sheet).triangles(0.).len(), 8);
        assert_eq!(grid(sheet).triangles(1.).len(), 8 - 8);
    }
}

