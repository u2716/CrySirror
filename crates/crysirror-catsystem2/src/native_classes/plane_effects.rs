// SPDX-License-Identifier: MPL-2.0
//! Plane transforms and child objects that 5E01F0 keeps beside its render object:
//! the world matrix (57), the history trail (79/80, ticked by 5DFE50), the pattern
//! grid of child planes (46) and the movie, effect and model objects (43/44/69).
use super::Ctx;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
use crysirror_scene::layer::LayerSpec;
use serde_json::json;
use std::collections::BTreeMap;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[81, 344, 398, 399, 433, 701, 702, 988, 989];

const GHOST_LAYERS: u32 = 0x5700_0000;

/// One 128-byte history record; id and render object words are derived here.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Record {
    position: [f32; 3],
    base: [f32; 3],
    scale: [f32; 3],
    matrix: [f32; 16],
    argb: u32,
    visible: bool,
}

/// Plane fields +264..+320.
#[derive(Clone, Debug)]
struct Trail {
    count: u32,
    stride: u32,
    end_alpha: i32,
    end_scale: [f32; 3],
    drift: [f32; 3],
    z_step: i32,
    /// +312: 0 plain; 1..5 turn each copy along the path and stretch it by its speed.
    mode: u32,
    records: Vec<Record>,
    published: bool,
}

#[derive(Default)]
pub(crate) struct State {
    trails: BTreeMap<u32, Trail>,
    /// Plane → the object 67C6F0/67C700 answer; scripts play it with native 266.
    movies: BTreeMap<u32, u32>,
    /// Plane → its 67CE50 mesh, drawn in place of the Plane's quad.
    models: BTreeMap<u32, Kx2Run>,
}

struct Kx2Run {
    model: Kx2,
    /// The Plane's image, kept once the mesh has replaced its paint.
    asset: Option<String>,
}

/// 67CE50: `KX2\0`, the mesh's bounds, then triangles of three {x, y, z, ARGB} corners.
#[derive(Clone, Debug, PartialEq)]
struct Kx2 {
    minimum: [f32; 2],
    maximum: [f32; 2],
    corners: Vec<([f32; 3], u32)>,
}

fn parse_kx2(bytes: &[u8]) -> Result<Kx2> {
    ensure!(bytes.len() >= 24 && bytes[..4] == *b"KX2\0", "not a KX2 file");
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let float = |at: usize| f32::from_bits(word(at));
    let triangles = word(20) as usize;
    ensure!(bytes.len() >= 24 + triangles.saturating_mul(48), "truncated KX2 triangles");
    let mut corners = Vec::with_capacity(triangles * 3);
    for corner in 0..triangles * 3 {
        let at = 24 + corner * 16;
        corners.push(([float(at), float(at + 4), float(at + 8)], word(at + 12)));
    }
    Ok(Kx2 { minimum: [float(4), float(8)], maximum: [float(12), float(16)], corners })
}

fn identity() -> [f32; 16] {
    let mut matrix = [0.; 16];
    for i in 0..4 {
        matrix[i * 5] = 1.;
    }
    matrix
}

/// 5DFA20: the half kept by the Plane and the thickness of each slice, along a side of
/// `length` pixels.
fn slice(length: f32) -> (f32, f32) {
    let pixels = length as i32;
    if pixels & 1 != 0 { ((pixels / 2) as f32, 1.) } else { ((pixels / 2 - 1) as f32, 2.) }
}

/// The records each mode draws: 5E29C0 keyed ones, 5E2C70 and 5E22A0 all, 5E1BB0 and 5E1F30
/// stop short of the last, which has no following record.
fn indices(trail: &Trail) -> Vec<u32> {
    let total = trail.count * trail.stride;
    match trail.mode {
        0 => (trail.stride..=total).step_by(trail.stride as usize).collect(),
        2 | 3 => (1..total).collect(),
        _ => (1..=total).collect(),
    }
}

/// 6782F0: row-vector matrix times a Z rotation.
fn rotate_z(matrix: &[f32; 16], angle: f32) -> [f32; 16] {
    let (sin, cos) = angle.sin_cos();
    let mut out = *matrix;
    for row in 0..4 {
        let (x, y) = (matrix[row * 4], matrix[row * 4 + 1]);
        out[row * 4] = x * cos - y * sin;
        out[row * 4 + 1] = x * sin + y * cos;
    }
    out
}

fn floats<const N: usize>(cx: &Ctx, address: u32) -> Result<[f32; N]> {
    let bytes = cx.machine.read_bytes(address, N * 4)?;
    let mut out = [0.; N];
    for (slot, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
        *slot = f32::from_le_bytes(chunk.try_into().unwrap());
    }
    Ok(out)
}

/// 67E360 / 67E3D0: `from`..`to` clamped over index..count.
fn lerp(from: f32, to: f32, index: u32, count: u32) -> f32 {
    if index >= count {
        return to;
    }
    from + (index as f32 / count as f32) * (to - from)
}

/// 5DFE50: every record takes the previous one, the base drifting by +296.
fn shift(trail: &mut Trail, newest: Record) {
    for at in (1..trail.records.len()).rev() {
        let mut record = trail.records[at - 1];
        for axis in 0..3 {
            record.base[axis] += trail.drift[axis];
        }
        trail.records[at] = record;
    }
    trail.records[0] = newest;
}

fn file_exists(cx: &mut Ctx, kind: &str, name: &str) -> Result<Option<String>> {
    let storage = cx.host.storage(cx.client)?;
    let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
    for path in catalog.candidates(storage.as_ref(), kind, name, &|| Ok(()))? {
        if let Some(found) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? {
            return Ok(Some(found.relative().to_owned()));
        }
    }
    Ok(None)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 Plane object native {} arguments", cx.call.id);
    let renderable = cx.tasks.plane(a[0]).is_ok_and(|plane| plane.render_type == 1);
    match cx.call.id {
        // 640730 → 69 → 67CE50: a `kx2` model. A missing file leaves the Plane empty.
        81 => {
            cx.machine.preflight_native(false, &[])?;
            let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 model name")?;
            if a[0] != 0 {
                if let Some(found) = file_exists(cx, "kx2", &name)? {
                    let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &found)?;
                    let bytes = cx.host.storage(cx.client)?.read(&path)?;
                    let model = parse_kx2(&bytes).with_context(|| format!("CatSystem2 kx2 model {name}"))?;
                    cx.state.plane_effects.models.insert(a[0], Kx2Run { model, asset: None });
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 640F40 → 57 → 67DED0: the object's world matrix (D3D rows, translation last).
        344 => {
            ensure!(a.len() == 3, "CatSystem2 Plane matrix arguments");
            cx.machine.preflight_native(false, &[])?;
            if a[0] != 0 && a[1] != 0 && renderable {
                let matrix: [f32; 16] = floats(cx, a[1])?;
                ensure!(matrix[3] == 0. && matrix[7] == 0. && matrix[11] == 0. && matrix[15] == 1., "CatSystem2 Plane matrix with a perspective row");
                let source_changed={
                    let plane=cx.tasks.plane_mut(a[0]).context("CatSystem2 Plane matrix target")?;
                    let source=a[2]!=0&&plane.source_pose.is_some();
                    let pose=if source {plane.source_pose.as_mut().unwrap()}else{&mut plane.pose};
                    // Even an explicit identity matrix replaces the Euler path.
                    pose.matrix=Some(matrix);
                    source
                };
                if source_changed {
                    #[cfg(feature="runtime")]
                    crate::native_plane::publish_source(cx.host,cx.client,cx.tasks,a[0])?;
                    #[cfg(not(feature="runtime"))]
                    anyhow::bail!("CatSystem2 source mesh matrix needs the runtime feature");
                    cx.machine.acknowledge(None)?;
                    return Ok(true);
                }
                if let Some(payload) = cx.tasks.plane(a[0])?.payload {
                    let mut layer: LayerSpec = serde_json::from_slice::<Option<LayerSpec>>(&cx.host.query_engine(cx.client, EngineQuery::Layer(payload))?)?.context("CatSystem2 Plane matrix layer")?;
                    layer.transform.local_matrix = Some(matrix);
                    layer.transform.rotate = 0.;
                    cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"set_transform","id":payload,"transform":layer.transform}]}))?)?;
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 640A10 → 79: a1 == 0 removes the trail (5DF980); otherwise 5E31D0 builds it.
        398 => {
            ensure!(a.len() == 12, "CatSystem2 Plane trail arguments");
            cx.machine.preflight_native(false, &[])?;
            if a[0] != 0 {
                remove_ghosts(cx.host, cx.client, &mut cx.state.plane_effects, a[0])?;
                if a[1] != 0 && cx.tasks.plane(a[0]).is_ok() {
                    ensure!(a[10] <= 5, "CatSystem2 Plane trail mode {}", a[10]);
                    // 5E31D0: mode 1 pairs its copies, modes 2..5 use every record.
                    let stride = match a[10] {
                        0 => a[2],
                        1 => 2,
                        _ => 1,
                    };
                    ensure!(stride != 0, "CatSystem2 Plane trail stride");
                    let pose = cx.tasks.plane(a[0])?.pose;
                    let newest = Record {
                        position: pose.position,
                        base: pose.base,
                        scale: pose.scale,
                        matrix: identity(),
                        argb: u32::from(pose.alpha) << 24 | 0xff_ffff,
                        visible: true,
                    };
                    let f = |i: usize| f32::from_bits(a[i]);
                    let drift = [f(3), f(4), f(5)];
                    let total = a[1].saturating_mul(stride);
                    ensure!(total <= 4096, "CatSystem2 Plane trail extent");
                    let mut records = vec![newest; total as usize + 1];
                    // 5E31D0: each record starts one drift behind the previous one.
                    for at in 1..records.len() {
                        for axis in 0..3 {
                            records[at].base[axis] = records[at - 1].base[axis] - drift[axis];
                        }
                    }
                    cx.state.plane_effects.trails.insert(a[0], Trail {
                        count: a[1],
                        stride,
                        mode: a[10],
                        end_alpha: a[6] as i32,
                        end_scale: [f(7), f(8), f(9)],
                        drift,
                        z_step: a[11] as i32,
                        records,
                        published: false,
                    });
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 6408B0 → 80: the script supplies the newest record.
        399 => {
            ensure!(a.len() == 7, "CatSystem2 Plane trail record arguments");
            cx.machine.preflight_native(false, &[])?;
            if cx.state.plane_effects.trails.contains_key(&a[0]) {
                let position: [f32; 3] = floats(cx, a[1])?;
                let base: [f32; 3] = floats(cx, a[2])?;
                let scale: [f32; 3] = floats(cx, a[3])?;
                let matrix: [f32; 16] = floats(cx, a[4])?;
                let record = Record { position, base, scale, matrix, argb: a[5], visible: a[6] != 0 };
                shift(cx.state.plane_effects.trails.get_mut(&a[0]).unwrap(), record);
            }
            cx.machine.acknowledge(None)?;
        }
        // 640030 → 46: a grid of "ptn plane" children, each cell a target quad.
        433 => {
            ensure!(a.len() == 5, "CatSystem2 Plane pattern arguments");
            cx.machine.preflight_native(false, &[])?;
            if a[0] != 0 && renderable {
                let (width, height) = (f32::from_bits(a[1]), f32::from_bits(a[2]));
                let (columns, rows) = (a[3] as i32, a[4] as i32);
                ensure!(columns.max(0).saturating_mul(rows.max(0)) <= 4096, "CatSystem2 Plane pattern extent");
                let parent_layer = cx.tasks.plane(a[0])?.layer;
                for row in 0..rows.max(0) {
                    for column in 0..columns.max(0) {
                        let child = cx.tasks.create_plane(a[0], b"ptn plane".to_vec())?;
                        let mut spec = child.layer_spec()?;
                        spec.parent = Some(parent_layer);
                        cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"put","layer":spec}]}))?)?;
                        let plane = cx.tasks.plane_mut(child.id).context("CatSystem2 pattern child")?;
                        plane.graph_parent = Some(parent_layer);
                        crate::native_plane::create_target(cx.host, cx.client, plane, width, height)?;
                        plane.pose.base = [column as f32 * width, row as f32 * height, 0.];
                        crate::native_plane::publish(cx.host, cx.client, plane)?;
                    }
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 6400C0 → 43 → 67A8C0 (`movie`) / 640170 → 44 → 67AD20 (`veff`): a video object on
        // the Plane. It stays stopped until the script plays the id 702/989 answer. A
        // missing file leaves the Plane empty.
        701 | 988 => {
            ensure!(a.len() == 2, "CatSystem2 Plane video arguments");
            cx.machine.preflight_native(false, &[])?;
            if a[0] != 0 && renderable {
                let name = crate::native_values::format_bytes(cx.machine, a[1])?;
                let name = String::from_utf8(crate::native_scene::stem(&name).to_vec()).context("CatSystem2 Plane video name")?;
                let kind = if cx.call.id == 701 { "movie" } else { "veff" };
                if let Some(path) = file_exists(cx, kind, &name)? {
                    #[cfg(feature = "runtime")]
                    {
                        let id = cx.movies.open(path, cx.canvas)?;
                        cx.state.plane_effects.movies.insert(a[0], id);
                    }
                    #[cfg(not(feature = "runtime"))]
                    bail!("CatSystem2 Plane video {path} needs the runtime feature");
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // 640530 / 640570 → 87 / 88: the Plane's video object, 0 without one.
        702 | 989 => {
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(cx.state.plane_effects.movies.get(&a[0]).copied().unwrap_or(0)))?;
        }
        id => bail!("CatSystem2 Plane object native {id} is not implemented"),
    }
    Ok(true)
}

fn ghost_id(plane: u32, index: u32) -> Result<u32> {
    ensure!(plane < 0x1_0000 && index < 0x100, "CatSystem2 Plane trail layer range");
    Ok(GHOST_LAYERS | plane << 8 | index)
}

fn remove_ghosts(host: &mut dyn HostServices, client: ClientId, state: &mut State, plane: u32) -> Result<()> {
    if let Some(trail) = state.trails.remove(&plane) {
        if trail.published {
            let mut commands = Vec::new();
            for index in indices(&trail) {
                commands.push(json!({"op":"remove","id":ghost_id(plane, index)?}));
            }
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":commands}))?)?;
        }
    }
    Ok(())
}

/// 5DFE50 → 5E29C0: position, base and matrix as recorded, scale and alpha eased to the
/// configured end over the history, each record `z_step` further back.
pub(crate) fn tick(host: &mut dyn HostServices, client: ClientId, tasks: &mut crate::task_policy::NativeTaskPolicy, state: &mut State) -> Result<()> {
    for (plane_id, run) in &mut state.models {
        let Ok(plane) = tasks.plane(*plane_id) else { continue };
        let Some(payload) = plane.payload else { continue };
        if run.asset.is_none() {
            let layer: Option<LayerSpec> = serde_json::from_slice(&host.query_engine(client, EngineQuery::Layer(payload))?)?;
            let Some(crysirror_scene::layer::Paint::Image { asset, .. }) = layer.map(|layer| layer.paint) else { continue };
            run.asset = Some(asset);
        }
        let model = &run.model;
        let span = [(model.maximum[0] - model.minimum[0]).max(1.), (model.maximum[1] - model.minimum[1]).max(1.)];
        let mut part = crysirror_puppet::FramePart { id: "kx2".into(), texture: 0, positions: Vec::new(), uvs: Vec::new(), indices: Vec::new(), colors: Vec::new(), blend: crysirror_puppet::PartBlend::Normal, masks: Vec::new(), invert_mask: false, visible: true };
        for (index, (position, argb)) in model.corners.iter().enumerate() {
            part.positions.push([position[0], position[1]]);
            part.uvs.push([(position[0] - model.minimum[0]) / span[0], (position[1] - model.minimum[1]) / span[1]]);
            let [b, g, r, a] = argb.to_le_bytes();
            part.colors.push([f32::from(r) / 255., f32::from(g) / 255., f32::from(b) / 255., f32::from(a) / 255.]);
            part.indices.push(index as u32);
        }
        let frame = crysirror_puppet::PuppetFrame { size: span, parts: vec![part], textures: vec![run.asset.clone().unwrap()] };
        host.present_puppet_frame(client, payload, &serde_json::to_vec(&frame)?)?;
    }
    for (plane_id, trail) in &mut state.trails {
        let Ok(plane) = tasks.plane(*plane_id) else { continue };
        let Some(payload) = plane.payload else { continue };
        if !plane.own_enabled {
            continue;
        }
        let pose = plane.pose;
        let matrix = trail.records[0].matrix;
        shift(trail, Record {
            position: pose.position,
            base: pose.base,
            scale: pose.scale,
            matrix,
            argb: u32::from(pose.alpha) << 24 | 0xff_ffff,
            visible: true,
        });
        let source: LayerSpec = serde_json::from_slice::<Option<LayerSpec>>(&host.query_engine(client, EngineQuery::Layer(payload))?)?.context("CatSystem2 Plane trail layer")?;
        let group: LayerSpec = serde_json::from_slice::<Option<LayerSpec>>(&host.query_engine(client, EngineQuery::Layer(plane.layer))?)?.context("CatSystem2 Plane trail group")?;
        let total = trail.count * trail.stride;
        let mut commands = Vec::new();
        for index in indices(trail) {
            let record = trail.records[index as usize];
            let id = ghost_id(*plane_id, index)?;
            let mut ghost = source.clone();
            ghost.id = id;
            ghost.parent = group.parent;
            ghost.order = group.order;
            ghost.visible = group.visible && record.visible;
            ghost.opacity = lerp((record.argb >> 24) as f32, trail.end_alpha as f32, index, total).clamp(0., 255.) as u8;
            ghost.rect = [record.base[0] - record.position[0], record.base[1] - record.position[1], pose.size[0].max(1.), pose.size[1].max(1.)];
            ghost.transform.z = record.position[2] + (index as i32).wrapping_mul(trail.z_step) as f32;
            let eased = |axis: usize| {
                let scale = lerp(record.scale[axis], record.scale[axis] * trail.end_scale[axis], index, total);
                if scale.abs() < 1e-4 { 1e-4f32.copysign(scale) } else { scale }
            };
            ghost.transform.scale_x = eased(0);
            ghost.transform.scale_y = eased(1);
            let mut matrix = record.matrix;
            let last = index == total;
            let at = index as usize;
            let chord = |next: [f32; 3], previous: [f32; 3]| {
                let (dx, dy, dz) = (next[0] - previous[0], next[1] - previous[1], next[2] - previous[2]);
                (dx * dx + dy * dy + dz * dz).sqrt()
            };
            let width = pose.size[0].max(1.);
            match trail.mode {
                // 5E2C70 turns and stretches only the unkeyed copies; 5E1BB0 halves the chord, 5E1F30 not.
                1 | 2 | 3 if trail.mode != 1 || index % trail.stride != 0 => {
                    let (next, previous) = (trail.records[at + 1].base, trail.records[at - 1].base);
                    matrix = rotate_z(&matrix, (next[1] - previous[1]).atan2(next[0] - previous[0]));
                    let half = if trail.mode == 2 { 0.5 } else { 1. };
                    ghost.transform.scale_x = (width + chord(next, previous) * half) / width;
                }
                // 5E22A0: copies hang off the path, the last without the stretch or the offset.
                4 | 5 => {
                    let previous = trail.records[at - 1].base;
                    matrix = rotate_z(&matrix, (record.base[1] - previous[1]).atan2(record.base[0] - previous[0]) - std::f32::consts::FRAC_PI_2);
                    let thickness = slice(pose.size[(trail.mode == 4) as usize]).1;
                    let offset = if last { 0. } else { thickness * 0.5 };
                    ghost.rect[1] = record.base[1] - offset;
                    if !last {
                        let next = trail.records[at + 1].base;
                        ghost.transform.scale_y = (thickness + chord(next, previous)) / thickness;
                    }
                }
                _ => {}
            }
            ghost.transform.local_matrix = (matrix != identity()).then_some(matrix);
            if trail.published {
                commands.push(json!({"op":"remove","id":id}));
            }
            commands.push(json!({"op":"put","layer":ghost}));
        }
        if trail.mode >= 4 {
            // 5E22A0 turns the Plane itself along its first step; 5DFA20 left it scaled to the slice.
            let (first, second) = (trail.records[0].base, trail.records[1].base);
            let angle = (second[1] - first[1]).atan2(second[0] - first[0]) - std::f32::consts::FRAC_PI_2;
            let turned = rotate_z(&trail.records[0].matrix, angle);
            let mut transform = source.transform.clone();
            transform.local_matrix = (turned != identity()).then_some(turned);
            let along = pose.size[(trail.mode == 4) as usize].max(1.);
            let (half, thickness) = slice(along);
            let scale = (along - (half - thickness)) / along;
            if trail.mode == 4 {
                transform.scale_y = pose.scale[1] * scale;
            } else {
                transform.scale_x = pose.scale[0] * scale;
            }
            commands.push(json!({"op":"set_transform","id":payload,"transform":transform}));
        }
        trail.published = true;
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":commands}))?)?;
    }
    Ok(())
}

pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    state.movies.remove(&id);
    remove_ghosts(host, client, state, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kx2_reads_bounds_and_three_corners_per_triangle() {
        let mut file = b"KX2\0".to_vec();
        for value in [0f32, 0., 100., 50.] {
            file.extend(value.to_le_bytes());
        }
        file.extend(1u32.to_le_bytes());
        for corner in [[0f32, 0., 0.], [100., 0., 0.], [0., 50., 0.]] {
            for value in corner {
                file.extend(value.to_le_bytes());
            }
            file.extend(u32::MAX.to_le_bytes());
        }
        let model = parse_kx2(&file).unwrap();
        assert_eq!(model.maximum, [100., 50.]);
        assert_eq!(model.corners.len(), 3);
        assert_eq!(model.corners[1].0, [100., 0., 0.]);
        assert!(parse_kx2(b"nope").is_err());
    }

    #[test]
    fn slices_follow_the_parity_of_the_side() {
        assert_eq!(slice(101.), (50., 1.));
        assert_eq!(slice(100.), (49., 2.));
    }

    #[test]
    fn history_shifts_with_drift_and_alpha_eases_to_end() {
        let record = Record { position: [0.; 3], base: [0.; 3], scale: [1.; 3], matrix: identity(), argb: 0xff00_0000, visible: true };
        let mut trail = Trail { count: 2, stride: 1, end_alpha: 0, end_scale: [1.; 3], drift: [1., 0., 0.], z_step: 0, mode: 0, records: vec![record; 3], published: false };
        shift(&mut trail, Record { base: [5., 0., 0.], ..record });
        assert_eq!(trail.records[0].base[0], 5.);
        assert_eq!(trail.records[1].base[0], 1.);
        shift(&mut trail, record);
        assert_eq!(trail.records[1].base[0], 6.);
        assert_eq!(lerp(255., 0., 1, 2), 127.5);
        assert_eq!(lerp(255., 0., 2, 2), 0.);
    }
}
