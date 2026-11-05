// SPDX-License-Identifier: MPL-2.0
//! Particle task, class 16 (constructor 5F46A0, handler 5F5A00, tick 5F51F0,
//! draw 5F4DD0). Each frame a KCS script from `ptcl.int` restarts once per
//! particle index (877270) and edits that particle through the native table
//! A406C0/A40778. The adapter keeps those original records and hands the
//! finished quads to the engine as one puppet frame; no pixels are drawn here.
//! `ptclpi` plugins are Windows DLLs: `particle_plugins` restates the ones this title uses;
//! any other takes the original's load-failure path.
use super::Ctx;
use crate::kcs::{Event, Machine, NativeCall, Program};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_puppet::{FramePart, PartBlend, PuppetFrame};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[271, 272, 273, 274, 275, 426, 427, 693];

const VERTEX_BYTES: usize = 28;
const QUAD_BYTES: usize = VERTEX_BYTES * 4;
/// Instructions one particle's script may spend in one frame.
const SCRIPT_BUDGET: u32 = 1 << 20;

fn layer(id: u32) -> Result<u32> {
    0x1600_0000u32.checked_add(id).filter(|v| *v < 0x1800_0000).context("CatSystem2 particle layer range")
}
fn texture_slot(id: u32) -> Result<u32> {
    0x5300_0000u32.checked_add(id).filter(|v| *v < 0x5400_0000).context("CatSystem2 particle texture range")
}

struct Script {
    program: Arc<Program>,
    machine: Machine,
}
struct Particles {
    script: Option<Script>,
    plugin: Option<super::particle_plugins::Plugin>,
    /// +0x31C, +0x320, +0x324: per-particle pivot, rotation and scale (plugins only; the
    /// script natives leave them 0, 0 and 1).
    pivots: Vec<[f32; 3]>,
    rotations: Vec<f32>,
    scales: Vec<[f32; 3]>,
    /// +0x184 count, +0x188 z, +0x18C four 32-byte script parameters.
    count: usize,
    z: f32,
    parameters: Vec<u8>,
    fps: u32,
    texture: String,
    /// +0x330/+0x334: image extent, 256 without an image.
    size: [f32; 2],
    /// +0x310 and its +0x314 copy: 112-byte quads {x, y, z, rhw, colour, u, v}.
    quads: Vec<u8>,
    base: Vec<u8>,
    /// +0x318 position per particle; +0x328 visibility.
    positions: Vec<[f32; 3]>,
    visible: Vec<u32>,
    /// +0x340 running, +0x344 elapsed, +0x348 last, +0x34C first tick, +0x368 last now.
    running: bool,
    elapsed: u32,
    last: u32,
    started: u32,
    previous: u64,
    /// +0x350 current index.
    index: usize,
    /// +0x32C attached task (-1 none), +0x354 alpha 0..256, +0x35C blend preset.
    attached: u32,
    alpha: u32,
    blend: u32,
    applied_blend: Option<u32>,
    shown: bool,
}

struct Load {
    pc: u32,
    id: u32,
    count: usize,
    z: f32,
    parameters: Vec<u8>,
    program: Option<crysirror_tasks::Task<Option<Arc<Program>>>>,
    script: Option<Option<Arc<Program>>>,
    plugin: Option<super::particle_plugins::Kind>,
    /// `cmnptcl`: 2AB0 asks the host for `ptcl<word0><word1>.plist`.
    plist: Option<crysirror_tasks::Task<Option<Vec<u8>>>>,
    plist_bytes: Option<Vec<u8>>,
    image: Option<crate::native_image_resource::Pending>,
}

#[derive(Default)]
pub(crate) struct State {
    tasks: BTreeMap<u32, Particles>,
    load: Option<Load>,
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn float(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(word(bytes, at))
}
fn put(bytes: &mut [u8], at: usize, value: f32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

impl Particles {
    /// 5F5D90: unit-less quads over the image, opaque white, every particle visible.
    fn reset(&mut self) {
        let [w, h] = self.size;
        let quad: Vec<u8> = [[0., 0., 0., 0.], [w, 0., 1., 0.], [0., h, 0., 1.], [w, h, 1., 1.]]
            .iter()
            .flat_map(|[x, y, u, v]: &[f32; 4]| {
                let mut vertex = Vec::with_capacity(VERTEX_BYTES);
                for value in [*x, *y, 0., 1.] {
                    vertex.extend(value.to_le_bytes());
                }
                vertex.extend(u32::MAX.to_le_bytes());
                vertex.extend(u.to_le_bytes());
                vertex.extend(v.to_le_bytes());
                vertex
            })
            .collect();
        self.quads = quad.repeat(self.count);
        self.base = self.quads.clone();
        self.positions = vec![[0.; 3]; self.count];
        self.visible = vec![1; self.count];
        self.pivots = vec![[0.; 3]; self.count];
        self.rotations = vec![0.; self.count];
        self.scales = vec![[1.; 3]; self.count];
        if let Some(plugin) = &mut self.plugin {
            plugin.restart();
        }
    }
    /// 5F61A0: the plugin's records into the task's arrays.
    fn apply_plugin(&mut self) {
        let Some(plugin) = &self.plugin else { return };
        for (index, record) in plugin.records().iter().enumerate().take(self.count) {
            let base = index * QUAD_BYTES;
            for vertex in 0..4 {
                let at = base + vertex * VERTEX_BYTES;
                if let Some(corners) = record.corners {
                    put(&mut self.quads, at, corners[vertex][0]);
                    put(&mut self.quads, at + 4, corners[vertex][1]);
                    put(&mut self.quads, at + 8, 0.);
                }
                if let Some(uvs) = record.uvs {
                    put(&mut self.quads, at + 0x14, uvs[vertex][0]);
                    put(&mut self.quads, at + 0x18, uvs[vertex][1]);
                }
                if let Some(depths) = record.depths {
                    put(&mut self.quads, at + 8, depths[vertex]);
                }
                if let Some(color) = record.color {
                    self.quads[at + 0x10..at + 0x13].copy_from_slice(&color);
                }
                let alpha = record.alphas.map_or(record.alpha, |alphas| alphas[vertex]);
                self.quads[at + 0x13] = if record.visible { alpha } else { 0 };
            }
            self.positions[index] = record.position;
            self.pivots[index] = record.pivot;
            self.rotations[index] = record.rotation;
            self.scales[index] = record.scale;
        }
    }
    fn quad(&self) -> std::ops::Range<usize> {
        self.index * QUAD_BYTES..(self.index + 1) * QUAD_BYTES
    }
    /// One script native. Answers true when the script ended itself (32 returns 4).
    fn native(&mut self, host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, canvas: [u32; 2], random: &mut crate::motion::RandomStreams, machine: &mut Machine, call: &NativeCall) -> Result<bool> {
        let a = &call.arguments;
        let f = |i: usize| f32::from_bits(a[i]);
        let vertices = |machine: &Machine, at: u32| machine.read_bytes(at, QUAD_BYTES);
        let value = match call.id {
            0 => {
                let text = crate::native_values::format_bytes(machine, a[0])?;
                host.append_log(client, &crysirror_resource::text_stream::decode_auto(&text, 500)?)?;
                None
            }
            1 => Some(self.elapsed),
            2 => Some(self.last),
            // 65A7A0: milliseconds to frames: 0 stays, negative stays, else at least 1.
            3 => {
                let ms = a[0] as i32;
                Some(if ms <= 0 { a[0] } else { ((self.fps as f32 / 60. * ms as u32 as f32) as u32).max(1) })
            }
            4 => Some(self.fps),
            5 => {
                machine.commit_native(None, &[(a[0], canvas[0].to_le_bytes().to_vec()), (a[1], canvas[1].to_le_bytes().to_vec())])?;
                return Ok(false);
            }
            6..=21 => Some(crate::motion::scalar(call.id - 5, a)?),
            // Rotations, translation and scale of a VM quad (67E910, 67E9A0, 67EA30, 67E490).
            22..=26 => {
                let mut quad = vertices(machine, a[0])?;
                for vertex in 0..4 {
                    let at = vertex * VERTEX_BYTES;
                    let (x, y, z) = (float(&quad, at), float(&quad, at + 4), float(&quad, at + 8));
                    let (s, c) = (f64::from(a.get(1).map_or(0., |w| f32::from_bits(*w))).sin() as f32, f64::from(a.get(1).map_or(0., |w| f32::from_bits(*w))).cos() as f32);
                    let moved = match call.id {
                        22 => [x, z * s + y * c, z * c - y * s],
                        23 => [x * c - z * s, y, z * c + x * s],
                        24 => [x * c + y * s, y * c - x * s, z],
                        25 => [x + f(1), f(2) + y, f(3) + z],
                        _ => [x * f(1), y * f(2), z * f(3)],
                    };
                    for (i, value) in moved.iter().enumerate() {
                        put(&mut quad, at + i * 4, *value);
                    }
                }
                machine.commit_native(None, &[(a[0], quad)])?;
                return Ok(false);
            }
            27 => {
                self.positions[self.index] = [f(0), f(1), f(2)];
                None
            }
            28 => {
                let p = self.positions[self.index];
                machine.commit_native(None, &[(a[0], p[0].to_le_bytes().to_vec()), (a[1], p[1].to_le_bytes().to_vec()), (a[2], p[2].to_le_bytes().to_vec())])?;
                return Ok(false);
            }
            // 65AE50 reads only words 0 and 2, half a texel in, for both axes.
            29 => {
                let (low, high) = (f(0) - 0.5, f(2) - 0.5);
                let [w, h] = self.size;
                let range = self.quad();
                let quad = &mut self.quads[range];
                for (vertex, (u, v)) in [(low, low), (high, low), (low, high), (high, high)].iter().enumerate() {
                    put(quad, vertex * VERTEX_BYTES + 0x14, u / w);
                    put(quad, vertex * VERTEX_BYTES + 0x18, v / h);
                }
                None
            }
            30 => {
                let quad = self.quads[self.quad()].to_vec();
                machine.commit_native(None, &[(a[0], quad)])?;
                return Ok(false);
            }
            31 => {
                let quad = vertices(machine, a[0])?;
                let range = self.quad();
                self.quads[range].copy_from_slice(&quad);
                None
            }
            // 65B030: running off; handler result 4 ends this script run.
            32 => {
                self.running = false;
                machine.acknowledge(None)?;
                return Ok(true);
            }
            33 => Some(random.integer_word(client)?),
            34 => Some(random.float_value(client)?.to_bits()),
            35 => {
                machine.commit_native(None, &[(a[0], self.parameters.clone())])?;
                return Ok(false);
            }
            36 => Some(self.index as u32),
            37 => {
                let (x, y, z) = (f(0) - f(3), f(1) - f(4), f(2) - f(5));
                Some((f64::from(y * y + x * x + z * z).sqrt() as f32).to_bits())
            }
            // 65AB40: the rectangle the whole screen covers at depth a0.
            38 => {
                let center = [(canvas[0] >> 1) as f32, (canvas[1] >> 1) as f32];
                let scale = perspective(self.z, f(0));
                let half = if scale != 0. { [center[0] / scale, center[1] / scale] } else { center };
                let words = [center[0] - half[0], center[1] - half[1], half[0] + center[0], half[1] + center[1]];
                let writes: Vec<(u32, Vec<u8>)> = words.iter().enumerate().map(|(i, v)| (a[i + 1], v.to_le_bytes().to_vec())).collect();
                machine.commit_native(None, &writes)?;
                return Ok(false);
            }
            39 => {
                if (a[0] as usize) < self.count {
                    self.visible[a[0] as usize] = a[1];
                }
                None
            }
            40 => Some(self.count as u32 - 1),
            41 => Some(crate::motion::scalar(41, a)?),
            42 => Some(crate::motion::scalar(17, a)?),
            43 => Some(crate::motion::scalar(42, a)?),
            // 65B7C0 restores the quads from the copy 65B600 took (version 0x20000 scripts).
            44 => {
                self.quads.copy_from_slice(&self.base);
                None
            }
            45 => {
                self.base.copy_from_slice(&self.quads);
                None
            }
            46 => Some(self.attached),
            // 65B650 → 675BF0 and 65B6D0 → 676040: the attached object's position (x, y)
            // and base (x, y, z); no object or another class leaves zeros.
            47 | 49 => {
                let pose = tasks.plane(self.attached).ok().map(|plane| plane.pose);
                let values: Vec<f32> = if call.id == 47 { pose.map_or(vec![0.; 2], |p| p.position[..2].to_vec()) } else { pose.map_or(vec![0.; 3], |p| p.base.to_vec()) };
                ensure!(a.len() == values.len(), "CatSystem2 particle attached pose arguments");
                let writes: Vec<(u32, Vec<u8>)> = a.iter().zip(&values).map(|(at, v)| (*at, v.to_le_bytes().to_vec())).collect();
                machine.commit_native(None, &writes)?;
                return Ok(false);
            }
            // 65B770 → 676310 → 67C740 answers object word +4, which no Plane field models yet.
            48 => bail!("CatSystem2 particle native 48 (attached object word +4) is not implemented"),
            // 65B7F0: 1..5 select a blend, anything else leaves it; 0 is normal.
            50 => {
                if a[0] <= 5 {
                    self.blend = a[0];
                }
                None
            }
            id => bail!("CatSystem2 particle script native {id} at PC {:#x} is outside A40778", call.pc),
        };
        match value {
            Some(value) => machine.commit_native(Some(value), &[])?,
            None => machine.acknowledge(None)?,
        }
        Ok(false)
    }
}

/// 678170 at the depth 65000 - (z + task z), near 65000 - task z - 1000.
fn perspective(task_z: f32, z: f32) -> f32 {
    let near = (65000.0 - task_z) - 1000.0;
    let depth = 65000.0 - (z + task_z);
    let span = if depth != near { depth - near } else { 1. };
    (1000. / span).max(0.)
}

fn state_of<'a>(cx: &'a mut Ctx, id: u32) -> Option<&'a mut Particles> {
    cx.state.particles.tasks.get_mut(&id)
}

fn begin_load(cx: &mut Ctx, a: &[u32], plugin: bool) -> Result<bool> {
    let name = crate::native_values::format_bytes(cx.machine, a[1])?;
    let image = crate::native_values::format_bytes(cx.machine, a[2])?;
    ensure!(a[5] != 0, "CatSystem2 particle parameters pointer");
    let parameters = cx.machine.read_bytes(a[5], 128)?;
    ensure!(a[3] as usize <= 200_000, "CatSystem2 particle count");
    let pool = cx.pool.context("CatSystem2 worker pool missing")?;
    let image = String::from_utf8(crate::native_scene::stem(&image).to_vec()).context("CatSystem2 particle image name")?;
    let pending = if image.is_empty() {
        None
    } else {
        let Some(pending) = crate::native_image_resource::Pending::begin_named(pool, cx.host, cx.client, cx.tasks.current(), cx.call.pc, &image, 0)? else { return Ok(false) };
        Some(pending)
    };
    let kind = if plugin { super::particle_plugins::Plugin::kind_of(crate::native_scene::stem(&name)) } else { None };
    let (program, script) = if plugin {
        if kind.is_none() {
        cx.host.append_log(cx.client, &format!("CatSystem2 particle plugin {} is a Windows DLL; it takes the original's load-failure path", String::from_utf8_lossy(&name)))?;
        }
        (None, Some(None))
    } else {
        let permit = match pool.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
            Err(e) => return Err(e),
        };
        let storage = cx.host.storage(cx.client)?;
        let text = String::from_utf8(crate::native_scene::stem(&name).to_vec()).context("CatSystem2 particle script name")?;
        let task = permit.spawn(move |cancel| {
            let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
            for path in catalog.candidates(storage.as_ref(), "ptcl", &text, &|| cancel.check())? {
                cancel.check()?;
                if let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? {
                    let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), crate::kcs::MAX_SCRIPT_BYTES as u64)?;
                    return Ok(Some(Arc::new(Program::parse(&bytes)?)));
                }
            }
            Ok(None)
        })?;
        (Some(task), None)
    };
    let plist = if kind == Some(super::particle_plugins::Kind::Common) {
        let word = |at: usize| f32::from_le_bytes(parameters[at * 4..at * 4 + 4].try_into().unwrap()) as i32;
        let file = format!("ptcl.int/ptcl{}{:03}.plist", word(0), word(1));
        let storage = cx.host.storage(cx.client)?;
        let Ok(permit) = pool.reserve() else { return Ok(false) };
        Some(permit.spawn(move |cancel| {
            cancel.check()?;
            let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, &file)?;
            Ok(match crate::native_resource_paths::resolve(storage.as_ref(), &path)? {
                Some(path) => Some(storage.read(&path)?),
                None => None,
            })
        })?)
    } else {
        None
    };
    cx.state.particles.load = Some(Load { plist, plist_bytes: None, pc: cx.call.pc, id: a[0], count: a[3] as usize, z: f32::from_bits(a[4]), parameters, program, script, plugin: kind, image: pending });
    Ok(true)
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 particle native {} arguments", cx.call.id);
    match cx.call.id {
        // 656DA0 / 656FA0 → 314: script or plugin, image, count, z, parameters. A task
        // of another class ignores the message; the native still answers 1.
        271 | 693 => {
            ensure!(a.len() == 6, "CatSystem2 particle load arguments");
            cx.machine.preflight_native(true, &[])?;
            if cx.tasks.opaque_class(a[0]) != Some(16) {
                cx.machine.acknowledge(Some(1))?;
                return Ok(true);
            }
            if cx.state.particles.load.as_ref().is_none_or(|load| load.pc != cx.call.pc) {
                begin_load(cx, &a, cx.call.id == 693)?;
                return Ok(false);
            }
            let load = cx.state.particles.load.as_mut().unwrap();
            if let Some(task) = load.plist.as_mut() {
                let Some(bytes) = task.try_take()? else { return Ok(false) };
                load.plist_bytes = bytes;
                load.plist = None;
            }
            if let Some(task) = load.program.as_mut() {
                let Some(program) = task.try_take()? else { return Ok(false) };
                load.script = Some(program);
                load.program = None;
            }
            let mut decoded = None;
            if let Some(pending) = load.image.as_mut() {
                let Some(outcome) = pending.poll()? else { return Ok(false) };
                if let crate::native_image_resource::Outcome::Ready(frame) = outcome {
                    decoded = Some(frame.image);
                }
                load.image = None;
            }
            let load = cx.state.particles.load.take().unwrap();
            let slot = texture_slot(load.id)?;
            let (texture, size) = match decoded {
                Some(image) => {
                    let upload = cx.host.publish_image_resource(cx.client, slot, image)?;
                    (upload.path.relative().to_owned(), [upload.size[0] as f32, upload.size[1] as f32])
                }
                // 5F5440: no image leaves a null texture and an extent of 256.
                None => (cx.host.upload_rgba(cx.client, slot, 1, 1, &[255; 4])?.relative().to_owned(), [256.; 2]),
            };
            let script = match load.script.flatten() {
                Some(program) => Some(Script { machine: Machine::new(&program, 64 * 1024)?, program }),
                None => {
                    if cx.call.id == 271 {
                        cx.host.append_log(cx.client, "CatSystem2 particle script not found")?;
                    }
                    None
                }
            };
            let fps = cx.settings.as_ref().map_or(60, |settings| settings.fps);
            let mut plugin = load.plugin.map(super::particle_plugins::Plugin::new);
            if let Some(plugin) = &mut plugin {
                plugin.set_plist(load.plist_bytes);
            }
            let mut particles = Particles { script, plugin, pivots: Vec::new(), rotations: Vec::new(), scales: Vec::new(), count: load.count, z: load.z, parameters: load.parameters, fps, texture, size, quads: Vec::new(), base: Vec::new(), positions: Vec::new(), visible: Vec::new(), running: false, elapsed: 0, last: 0, started: 0, previous: 0, index: 0, attached: u32::MAX, alpha: 256, blend: 0, applied_blend: None, shown: false };
            particles.reset();
            cx.state.particles.tasks.insert(load.id, particles);
            cx.machine.acknowledge(Some(1))?;
        }
        // 657280 → 315: reset, run; a plugin that did not load stops at once.
        272 => {
            cx.machine.preflight_native(false, &[])?;
            if let Some(particles) = state_of(cx, a[0]) {
                particles.reset();
                particles.running = particles.script.is_some() || particles.plugin.is_some();
                particles.started = 0;
            }
            cx.machine.acknowledge(None)?;
        }
        // 6572B0 → 316: stop and take the draw callback away (67E0C0).
        273 => {
            cx.machine.preflight_native(false, &[])?;
            let hide = state_of(cx, a[0]).map(|particles| {
                particles.running = false;
                core::mem::take(&mut particles.shown)
            });
            if hide == Some(true) {
                let id = layer(a[0])?;
                cx.host.submit_engine(cx.client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"set_visible","id":id,"visible":false}]}))?)?;
            }
            cx.machine.acknowledge(None)?;
        }
        // 6571A0 → 318.
        274 => {
            cx.machine.preflight_native(true, &[])?;
            let running = state_of(cx, a[0]).is_some_and(|particles| particles.running);
            cx.machine.acknowledge(Some(u32::from(running)))?;
        }
        // 657240 → 319: new script parameters.
        275 => {
            ensure!(a.len() == 2, "CatSystem2 particle parameter arguments");
            cx.machine.preflight_native(false, &[])?;
            if cx.state.particles.tasks.contains_key(&a[0]) {
                let bytes = cx.machine.read_bytes(a[1], 128)?;
                state_of(cx, a[0]).unwrap().parameters = bytes;
            }
            cx.machine.acknowledge(None)?;
        }
        // 6571E0 → 320: alpha, 255 and above meaning 256.
        426 => {
            ensure!(a.len() == 2, "CatSystem2 particle alpha arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(particles) = state_of(cx, a[0]) {
                particles.alpha = if a[1] as i32 >= 255 { 256 } else { a[1] };
            }
            cx.machine.acknowledge(None)?;
        }
        // 657210 → 321: attached object, 0 meaning none.
        427 => {
            ensure!(a.len() == 2, "CatSystem2 particle attach arguments");
            cx.machine.preflight_native(false, &[])?;
            if let Some(particles) = state_of(cx, a[0]) {
                particles.attached = if a[1] == 0 { u32::MAX } else { a[1] };
            }
            cx.machine.acknowledge(None)?;
        }
        id => bail!("CatSystem2 native {id} has no handler in the particle family"),
    }
    Ok(true)
}

/// 5F51F0 + 5F4DD0: run every particle's script, then publish the sorted quads.
pub(crate) fn tick(host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, canvas: [u32; 2], state: &mut State, random: Option<&mut crate::motion::RandomStreams>, now_ms: u64) -> Result<()> {
    let mut random = random;
    for (id, particles) in &mut state.tasks {
        if !particles.running {
            continue;
        }
        if particles.script.is_none() && particles.plugin.is_none() {
            continue;
        }
        if particles.started == 0 {
            particles.started = 1;
            particles.last = u32::MAX;
            particles.elapsed = 0;
        } else {
            particles.elapsed = particles.elapsed.wrapping_add(now_ms.saturating_sub(particles.previous) as u32);
        }
        particles.previous = now_ms;
        let random = random.as_deref_mut().context("CatSystem2 particle script has no session random stream")?;
        if let Some(mut plugin) = particles.plugin.take() {
            // 5F60B0: the attached object's base; 5F5329 passes last and elapsed.
            let base = tasks.plane(particles.attached).ok().map_or([0.; 2], |plane| [plane.pose.base[0], plane.pose.base[1]]);
            let dt = particles.elapsed.wrapping_sub(particles.last) as i32;
            plugin.update(particles.count, canvas, particles.size, &particles.parameters, base, dt, random, client)?;
            particles.plugin = Some(plugin);
            particles.apply_plugin();
            particles.last = particles.elapsed;
            present(host, client, canvas, *id, particles)?;
            continue;
        }
        let Some(mut script) = particles.script.take() else { continue };
        for index in 0..particles.count {
            particles.index = index;
            let machine = &mut script.machine;
            machine.pc = 0;
            machine.stack = script.program.globals_bytes;
            machine.frame_base = script.program.globals_bytes;
            machine.frame_end = script.program.globals_bytes;
            machine.halted = false;
            let mut budget = SCRIPT_BUDGET;
            loop {
                let before = machine.instructions;
                let event = machine.step(&script.program, budget)?;
                budget = budget.saturating_sub(u32::try_from(machine.instructions - before).unwrap_or(u32::MAX));
                match event {
                    Event::Halted => break,
                    Event::BudgetExhausted => bail!("CatSystem2 particle script exceeded its frame budget"),
                    Event::Native(call) => {
                        if particles.native(host, client, tasks, canvas, random, machine, &call).with_context(|| format!("CatSystem2 particle native {} at PC {:#x}", call.id, call.pc))? {
                            break;
                        }
                    }
                }
            }
        }
        particles.script = Some(script);
        particles.last = particles.elapsed;
        present(host, client, canvas, *id, particles)?;
    }
    Ok(())
}

fn present(host: &mut dyn HostServices, client: ClientId, canvas: [u32; 2], id: u32, particles: &mut Particles) -> Result<()> {
    if particles.alpha == 0 {
        return Ok(());
    }
    // 5F51F0's key and 7C7C10 + 5F6810: ascending position z + first vertex z.
    let mut order: Vec<(f32, usize)> = (0..particles.count).map(|i| (particles.positions[i][2] + float(&particles.quads, i * QUAD_BYTES + 8), i)).collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let center = [(canvas[0] >> 1) as f32, (canvas[1] >> 1) as f32];
    let mut part = FramePart { id: "particles".into(), texture: 0, positions: Vec::new(), uvs: Vec::new(), indices: Vec::new(), colors: Vec::new(), blend: if particles.blend == 1 { PartBlend::Add } else { PartBlend::Normal }, masks: Vec::new(), invert_mask: false, visible: true };
    for (_, index) in order {
        if particles.visible[index] == 0 {
            continue;
        }
        let quad = &particles.quads[index * QUAD_BYTES..(index + 1) * QUAD_BYTES];
        let base = part.positions.len() as u32;
        let p = particles.positions[index];
        for vertex in 0..4 {
            let at = vertex * VERTEX_BYTES;
            // 5F4DD0: 67EC40 takes the pivot off, then scale, Z rotation (6782F0) and, in
            // 67EC90's translation row, the position scaled by the vertex's w.
            let w = float(quad, at + 12);
            let pivot = particles.pivots[index];
            let scale3 = particles.scales[index];
            let (sin, cos) = particles.rotations[index].sin_cos();
            let (lx, ly, lz) = ((float(quad, at) - pivot[0]) * scale3[0], (float(quad, at + 4) - pivot[1]) * scale3[1], (float(quad, at + 8) - pivot[2]) * scale3[2]);
            let (rx, ry) = (lx * cos - ly * sin, lx * sin + ly * cos);
            let (x, y, z) = (rx + p[0] * w, ry + p[1] * w, lz + p[2] * w);
            let scale = perspective(particles.z, z);
            part.positions.push([(x - center[0]) * scale + center[0], (y - center[1]) * scale + center[1]]);
            part.uvs.push([float(quad, at + 0x14), float(quad, at + 0x18)]);
            let [b, g, r, alpha] = word(quad, at + 0x10).to_le_bytes();
            // Only the alpha byte takes +0x354: (A * alpha) >> 8.
            let alpha = (u32::from(alpha) * particles.alpha) >> 8;
            part.colors.push([f32::from(r) / 255., f32::from(g) / 255., f32::from(b) / 255., alpha as f32 / 255.]);
        }
        part.indices.extend([base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    let size = [canvas[0] as f32, canvas[1] as f32];
    let frame = PuppetFrame { size, parts: vec![part], textures: vec![particles.texture.clone()] };
    let layer_id = layer(id)?;
    let mut commands = Vec::new();
    if !particles.shown {
        particles.shown = true;
        particles.applied_blend = None;
        commands.push(json!({"op":"put","layer":{"id":layer_id,"rect":[0.,0.,size[0],size[1]],"paint":{"kind":"solid","color":[0,0,0,0]}}}));
        commands.push(json!({"op":"set_depth","id":layer_id,"depth":particles.z}));
    }
    // The frame renders into an isolated target, so the fixed-function pair of
    // 6718F0 applies when that target meets the scene, as for a Plane (native 402).
    if particles.applied_blend != Some(particles.blend) {
        particles.applied_blend = Some(particles.blend);
        let blend = match particles.blend { 1 => "add", 2 => "source_over", 3 => "modulate", 4 => "inverse_destination", 5 => "modulate_add", _ => "normal" };
        commands.push(json!({"op":"set_blend","id":layer_id,"blend":blend}));
    }
    if !commands.is_empty() {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({ "layers": commands }))?)?;
    }
    host.present_puppet_frame(client, layer_id, &serde_json::to_vec(&frame)?)?;
    Ok(())
}

/// Native 67 destroyed task `id`.
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    if let Some(particles) = state.tasks.remove(&id) {
        if particles.shown {
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":layer(id)?}]}))?)?;
        }
    }
    Ok(())
}
