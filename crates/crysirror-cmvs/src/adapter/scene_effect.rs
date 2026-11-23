// SPDX-License-Identifier: MPL-2.0
//! Scripted effects in the four native special-scene slots.

mod actors;
mod door;
mod particles;

use super::{effect::Rand, light::Light, visual::DecodedImage};
use actors::Actors;
pub(super) use actors::{SAMPLES as ACTOR_SAMPLES, TEXTURES as ACTOR_TEXTURES};
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{HostServices, UploadedImage},
    state::ClientId,
};
use crysirror_scene::{
    layer::Blend,
    projected_mesh::{MeshFrame, Part, Vertex},
};
use door::Door;
pub(super) use door::{OVERLAY as DOOR_OVERLAY, PANELS as DOOR_PANELS};
use glam::{Mat4, Vec3};
use particles::Particles;
pub(super) use particles::{SAMPLE as PARTICLE_SAMPLE, TEXTURES as PARTICLE_TEXTURES};
use serde_json::json;

pub(super) fn words(opcode: u16) -> u16 {
    match opcode {
        0x2290 => 3,
        0x2296 => 2,
        _ => 1,
    }
}

pub(super) enum Scene {
    Walk(Walk),
    Mesh(MeshScene),
}

impl Scene {
    pub fn status(&self) -> u32 {
        match self {
            Self::Walk(walk) => walk.status(),
            Self::Mesh(mesh) => match &mesh.mode {
                MeshMode::Tower(tower) => tower.status(),
                MeshMode::Door(door) => door.status(),
                MeshMode::Particles(particles) => particles.status(),
                MeshMode::Actors(actors) => actors.status(),
                MeshMode::Orbit(_) => 0,
                _ => u32::MAX,
            },
        }
    }

    pub fn trigger(&mut self, key: u32) {
        match self {
            Self::Walk(walk) => walk.trigger(key),
            Self::Mesh(mesh) => match &mut mesh.mode {
                MeshMode::Tower(tower) => tower.trigger(key),
                MeshMode::Door(door) => door.trigger(key),
                MeshMode::Particles(particles) => particles.trigger(key),
                MeshMode::Actors(actors) => actors.trigger(key),
                _ => {}
            },
        }
    }

    pub fn sample_slots(&self) -> std::ops::Range<usize> {
        match self {
            Self::Walk(_) => 16..21,
            Self::Mesh(mesh) if matches!(mesh.mode, MeshMode::Tower(_) | MeshMode::Actors(_)) => {
                16..18
            }
            Self::Mesh(mesh) if matches!(mesh.mode, MeshMode::Particles(_)) => 16..17,
            Self::Mesh(_) => 0..0,
        }
    }

    pub fn advance(&mut self, camera: &mut Light, delta: u32) -> SceneSound {
        match self {
            Self::Walk(walk) => walk.advance(camera, delta),
            Self::Mesh(mesh) => mesh.advance(camera.view(), delta),
        }
    }
}

/// The scene owns its scoped textures while the shared mesh compositor
/// owns clipping, sampling and rasterization.
pub(super) struct MeshScene {
    textures: Vec<Option<UploadedImage>>,
    mode: MeshMode,
    published: bool,
}

enum MeshMode {
    Track(Track),
    Tower(Tower),
    Door(Door),
    Particles(Particles),
    Actors(Actors),
    Spin(Spin),
    Orbit(Orbit),
}

/// Type 0x50 retains three floor segments and a translucent plane.
struct Track {
    depths: [f32; 3],
    first: usize,
    offset: f32,
    water: [f32; 2],
}

impl MeshScene {
    fn layer(index: usize) -> u32 {
        401_000 + index as u32
    }

    fn sprite_layers(index: usize) -> [u32; 3] {
        let base = Self::layer(index);
        [base, base + 4, base + 8]
    }

    pub fn new(
        host: &mut dyn HostServices,
        client: ClientId,
        opcode: u16,
        index: usize,
        mut images: Vec<Option<DecodedImage>>,
        rand: &mut Rand,
        door_fallback: [bool; 2],
    ) -> Result<Self> {
        if opcode == 0x2293 {
            images.resize_with(5, || None);
        }
        let textures = Self::publish_images(host, client, index, 0, images)?;
        Ok(Self {
            textures,
            mode: match opcode {
                0x2290 => MeshMode::Track(Track {
                    depths: [0., 10., 20.],
                    first: 0,
                    offset: 0.,
                    water: [0.; 2],
                }),
                0x2292 => MeshMode::Tower(Tower::default()),
                0x2293 => MeshMode::Door(Door::new(door_fallback)),
                0x2294 => MeshMode::Particles(Particles::new(rand)),
                0x2295 => MeshMode::Actors(Actors::default()),
                0x2296 => MeshMode::Spin(Spin::default()),
                0x2297 => MeshMode::Orbit(Orbit::new(rand)),
                _ => unreachable!("special scene constructor"),
            },
            published: false,
        })
    }

    pub fn requested_images(&self, key: u32) -> Option<&'static [&'static str]> {
        match (&self.mode, key) {
            (MeshMode::Orbit(_), 10) => Some(&ORBIT_REVEAL_TEXTURES),
            (MeshMode::Particles(_), 50) => Some(particles::RELOAD),
            _ => None,
        }
    }

    pub fn door_rule_request(&mut self) -> Option<[&'static str; 2]> {
        match &mut self.mode {
            MeshMode::Door(door) => door.rule_request(),
            _ => None,
        }
    }

    pub fn load_door_rules(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        images: [Option<DecodedImage>; 2],
    ) -> Result<()> {
        let MeshMode::Door(door) = &mut self.mode else {
            anyhow::bail!("CMVS rule textures arrived without their scene");
        };
        let staged = Self::publish_images(host, client, index, 3, images)?;
        self.textures[3..5]
            .iter_mut()
            .zip(staged)
            .for_each(|(slot, image)| *slot = image);
        door.rules_loaded();
        Ok(())
    }

    pub fn replace_images(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        images: Vec<Option<DecodedImage>>,
    ) -> Result<()> {
        let start = match &self.mode {
            MeshMode::Orbit(_) => 2,
            MeshMode::Particles(_) => 5,
            _ => anyhow::bail!("CMVS texture replacement without its scene"),
        };
        let staged = Self::publish_images(host, client, index, start, images)?;
        for (offset, image) in staged.into_iter().enumerate() {
            let old = std::mem::replace(&mut self.textures[start as usize + offset], image);
            if let Some(image) = old {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        if let MeshMode::Orbit(orbit) = &mut self.mode {
            orbit.revealed = true;
        }
        Ok(())
    }

    fn publish_images(
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        start: u32,
        images: impl IntoIterator<Item = Option<DecodedImage>>,
    ) -> Result<Vec<Option<UploadedImage>>> {
        let mut published: Vec<Option<UploadedImage>> = Vec::new();
        for (offset, image) in images.into_iter().enumerate() {
            let id = Self::layer(index) * 16 + start + offset as u32;
            match image
                .map(|image| image.publish(host, client, id))
                .transpose()
            {
                Ok(image) => published.push(image),
                Err(error) => {
                    for image in published.iter().flatten() {
                        host.release_image_resource(client, image.path.relative())?;
                    }
                    return Err(error);
                }
            }
        }
        Ok(published)
    }

    pub fn release(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
    ) -> Result<()> {
        if self.published {
            let ids = Self::sprite_layers(index);
            let count = if matches!(self.mode, MeshMode::Door(_) | MeshMode::Actors(_)) {
                3
            } else {
                1
            };
            let layers: Vec<_> = ids[..count]
                .iter()
                .map(|id| json!({"op":"remove","id":id}))
                .collect();
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
            self.published = false;
        }
        for texture in &mut self.textures {
            if let Some(image) = texture {
                host.release_image_resource(client, image.path.relative())?;
                *texture = None;
            }
        }
        Ok(())
    }

    fn advance(&mut self, camera: [f32; 7], delta: u32) -> SceneSound {
        match &mut self.mode {
            MeshMode::Track(track) => track.advance(camera),
            MeshMode::Tower(tower) => return tower.advance(delta),
            MeshMode::Door(door) => door.advance(delta),
            MeshMode::Particles(particles) => return particles.advance(delta),
            MeshMode::Actors(actors) => return actors.advance(delta),
            MeshMode::Spin(spin) => spin.advance(delta),
            MeshMode::Orbit(orbit) => orbit.advance(delta),
        }
        SceneSound::None
    }

    pub fn draw(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        canvas: [u32; 2],
        camera: [f32; 7],
        sprite_camera: &[u32; 40],
        delta: u32,
    ) -> Result<()> {
        let size = canvas.map(|v| v as f32);
        if let MeshMode::Door(door) = &mut self.mode {
            let rule_ended = door.sample_rule(delta);
            let layers = door.layers(
                Self::sprite_layers(index),
                index,
                &self.textures,
                size,
                camera,
            )?;
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
            self.published = true;
            if rule_ended {
                for texture in &mut self.textures[3..5] {
                    if let Some(lease) = texture {
                        host.release_image_resource(client, lease.path.relative())?;
                        *texture = None;
                    }
                }
            }
            return Ok(());
        }
        if let MeshMode::Actors(actors) = &mut self.mode {
            let layers = actors.layers(
                Self::sprite_layers(index),
                index,
                &self.textures,
                sprite_camera,
                canvas,
                delta,
            )?;
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers": layers}))?)?;
            self.published = true;
            return Ok(());
        }
        if self.textures.iter().all(Option::is_none) {
            return Ok(());
        }
        let mut mesh = MeshFrame {
            size,
            parts: Vec::new(),
        };
        match &self.mode {
            MeshMode::Track(track) => track.geometry(&mut mesh.parts, &self.textures),
            MeshMode::Tower(tower) => tower.geometry(&mut mesh.parts, &self.textures, camera),
            MeshMode::Spin(spin) => spin.geometry(&mut mesh.parts, &self.textures, camera),
            MeshMode::Orbit(orbit) => orbit.geometry(&mut mesh.parts, &self.textures, camera),
            MeshMode::Particles(particles) => {
                particles.geometry(&mut mesh.parts, &self.textures, camera)?
            }
            MeshMode::Door(_) | MeshMode::Actors(_) => {
                unreachable!("sprite layers were published above")
            }
        }
        let frame = mesh.project_homogeneous(view_projection(camera, size)?, false)?;
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({
                "layers":[{"op":"put","layer":{
                    "id":Self::layer(index),"rect":[0.,0.,size[0],size[1]],
                    "order":[-1,index],"paint":{"kind":"projected_mesh","frame":frame},
                }}]
            }))?,
        )?;
        self.published = true;
        Ok(())
    }
}

impl Track {
    fn advance(&mut self, camera: [f32; 7]) {
        self.water = [camera[0], camera[2]];
        // The original recycles at most one segment per admitted frame.
        if camera[2] > self.offset + 10. {
            self.depths[self.first] = self.offset + 30.;
            self.offset += 10.;
            self.first = (self.first + 1) % self.depths.len();
        }
    }

    fn geometry(&self, parts: &mut Vec<Part>, textures: &[Option<UploadedImage>]) {
        if let Some(ground) = &textures[0] {
            for &depth in &self.depths {
                parts.push(plane(
                    ground,
                    [-5., 5.],
                    [-5., 5.],
                    Mat4::from_translation(Vec3::new(0., -0.1, depth)),
                    255,
                    opaque_blend(ground),
                ));
            }
        }
        if let Some(water) = &textures[1] {
            parts.push(plane(
                water,
                [-15., 15.],
                [-15., 15.],
                Mat4::from_translation(Vec3::new(self.water[0], 0., self.water[1])),
                255,
                Blend::Normal,
            ));
        }
    }
}

pub(super) const SPIN_TEXTURES: [&str; 2] = ["po_bg510c.pb3", "it008d2.pb3"];
pub(super) const ORBIT_TEXTURES: [&str; 9] = [
    "__ev-xx004a.pb3",
    "__ev-xx004g.pb3",
    "__ev-xx004f.pb3",
    "__ev-xx004b1.pb3",
    "it011d1.pb3",
    "it011d2.pb3",
    "it011d3.pb3",
    "it011d4.pb3",
    "it011c.pb3",
];
pub(super) const ORBIT_REVEAL_TEXTURES: [&str; 2] = ["__ev-xx004e.pb3", "__ev-xx004b2.pb3"];

/// Type 0x57 uses six bands of 32 atlas quads and four revolving planes.
struct Orbit {
    cells: [[u8; 32]; 6],
    heights: [f32; 6],
    time: u32,
    advanced: bool,
    revealed: bool,
}

impl Orbit {
    fn new(rand: &mut Rand) -> Self {
        let mut height = -0.6;
        let heights = std::array::from_fn(|_| {
            let current = height;
            height += 0.3;
            current
        });
        let cells = std::array::from_fn(|_| std::array::from_fn(|_| (rand.next() % 8) as u8));
        Self {
            cells,
            heights,
            time: 0,
            advanced: false,
            revealed: false,
        }
    }

    fn advance(&mut self, delta: u32) {
        let delta = delta.min(200);
        self.time = self.time.wrapping_add(delta);
        let descent = (2000 * delta / 3000) as f32 / 1000.;
        for height in &mut self.heights {
            *height -= descent;
            if *height <= -0.9 {
                *height += 1.8;
            }
        }
        self.advanced = true;
    }

    fn angles(&self, row: usize, column: usize) -> (f32, f32) {
        if !self.advanced {
            let angle = column as u32 * 11;
            return (angle as f32, ((450 - angle) % 360) as f32);
        }
        let rotation = self.time.wrapping_mul(36_000) / 4000 % 36_000;
        let mut angle = (column as u32 * 1125 + rotation) % 36_000;
        if row % 2 == 0 {
            angle = (36_000 - angle) % 36_000;
        }
        (
            angle as f32 / 100.,
            ((45_000 - angle) % 36_000 / 100) as f32,
        )
    }

    fn geometry(
        &self,
        parts: &mut Vec<Part>,
        textures: &[Option<UploadedImage>],
        camera: [f32; 7],
    ) {
        if let Some(image) = &textures[0] {
            parts.push(wall(
                image,
                [-15., 15.],
                [-9., 9.],
                Vec3::new(0., 0., 32.),
                0.,
                opaque_blend(image),
            ));
        }
        if self.revealed {
            if let Some(image) = &textures[1] {
                parts.push(wall(
                    image,
                    [-15., 15.],
                    [-9., 9.],
                    Vec3::new(0., 0., 31.9),
                    0.,
                    Blend::Normal,
                ));
            }
        }
        let mut quads = Vec::with_capacity(198);
        for (index, width, height, position) in [
            (2, [-0.5, 0.5], [-0.5, 0.5], Vec3::new(-0.17, 0.17, 1.2)),
            (3, [-2.3, 1.7], [-0.2, 3.8], Vec3::new(2.6, -2.5, 10.)),
        ] {
            if let Some(image) = &textures[index] {
                quads.push((
                    position,
                    wall(image, width, height, position, 0., Blend::Normal),
                ));
            }
        }
        if self.revealed {
            let rotation = (self.time.wrapping_mul(3600) / 30_000 % 3600) as f32 / 10.;
            let position = Vec3::new(2.6, -2.5, 10.1);
            for (index, image) in textures[4..8].iter().enumerate() {
                if let Some(image) = image {
                    let yaw = if index % 2 == 0 {
                        rotation
                    } else {
                        360. - rotation
                    };
                    let world =
                        Mat4::from_translation(position) * Mat4::from_rotation_y(yaw.to_radians());
                    quads.push((
                        position,
                        plane(image, [-12., 12.], [-12., 12.], world, 255, Blend::Normal),
                    ));
                }
            }
        }
        if let Some(image) = &textures[8] {
            for (row, &height) in self.heights.iter().enumerate() {
                for (column, &cell) in self.cells[row].iter().enumerate() {
                    let (angle, yaw) = self.angles(row, column);
                    let radians = (angle / 180. * std::f32::consts::PI) as f64;
                    let (sine, cosine) = radians.sin_cos();
                    let position = Vec3::new(
                        hundredths((cosine * 1.5) as f32) - 0.17,
                        height,
                        hundredths((sine * 1.5) as f32) + 1.2,
                    );
                    let mut quad = wall(
                        image,
                        [-0.1, 0.1],
                        [-0.1, 0.1],
                        position,
                        yaw,
                        Blend::Normal,
                    );
                    let u = f32::from(cell & 3);
                    let v = f32::from(cell >> 2);
                    for vertex in &mut quad.vertices {
                        vertex.uv = [(vertex.uv[0] + u) * 0.25, (vertex.uv[1] + v) * 0.25];
                    }
                    quads.push((position, quad));
                }
            }
        }
        append_transparent(parts, quads, camera);
    }
}

pub(super) const TOWER_TEXTURES: [&str; 9] = [
    "try002a1.pb3",
    "try002a2.pb3",
    "try002c1.pb3",
    "try002c2.pb3",
    "try002c3.pb3",
    "try002c4.pb3",
    "try002c5.pb3",
    "try002c6.pb3",
    "try002d.pb3",
];
pub(super) const TOWER_SAMPLES: &[(usize, &str)] = &[(16, "SE006.ogg"), (17, "SE006c.wav")];

/// Type 0x52 queues a reveal (1) or a lift and fade (2) and
/// rotates four sets of six crossed quads independently of the phase.
struct Tower {
    rotation_ms: u32,
    phase_ms: u32,
    phase: u32,
    queued: Option<u32>,
    cue_started: bool,
    opacity: [u8; 12],
    lift: f32,
}

impl Default for Tower {
    fn default() -> Self {
        Self {
            rotation_ms: 0,
            phase_ms: 0,
            phase: 0,
            queued: None,
            cue_started: false,
            opacity: [255; 12],
            lift: 0.,
        }
    }
}

impl Tower {
    fn trigger(&mut self, key: u32) {
        if matches!(key, 1 | 2) {
            self.queued = Some(key);
        }
    }

    fn status(&self) -> u32 {
        if self.queued.is_some() {
            u32::MAX
        } else {
            self.phase
        }
    }

    fn advance(&mut self, delta: u32) -> SceneSound {
        self.rotation_ms = self.rotation_ms.wrapping_add(delta);
        if self.phase == 0 {
            if let Some(phase) = self.queued.take() {
                self.phase = phase;
                self.phase_ms = 0;
                self.cue_started = false;
            }
            return SceneSound::None;
        }

        let cue = if std::mem::replace(&mut self.cue_started, true) {
            SceneSound::None
        } else {
            SceneSound::Play {
                slot: 15 + self.phase as usize,
                stop_stream: false,
            }
        };
        self.phase_ms += delta.min(200);
        let elapsed = self.phase_ms;
        let reveal = self.phase == 1;
        let duration = if reveal { 6500 } else { 7000 };
        if elapsed >= duration {
            self.phase = 0;
            self.phase_ms = 0;
            self.opacity.fill(if reveal { 255 } else { 0 });
            if !reveal {
                self.lift = 0.;
                return SceneSound::StopResident(17);
            }
        } else {
            if !reveal {
                self.lift = elapsed.min(1000) as f32 * 32. / 1000.;
            }
            for (group, alpha) in self.opacity.iter_mut().enumerate() {
                let delay = group as u32 * 300 + if reveal { 0 } else { 1500 };
                if let Some(age) = elapsed.checked_sub(delay) {
                    *alpha = if reveal {
                        (255 * age.min(3000) / 3000) as u8
                    } else {
                        255 - (255 * age.min(2000) / 2000) as u8
                    };
                }
            }
        }
        cue
    }

    fn geometry(
        &self,
        parts: &mut Vec<Part>,
        textures: &[Option<UploadedImage>],
        camera: [f32; 7],
    ) {
        if let Some(ground) = &textures[0] {
            parts.push(plane(
                ground,
                [-30., 30.],
                [0., 36.],
                Mat4::from_translation(Vec3::new(0., -12., 39.)),
                255,
                opaque_blend(ground),
            ));
        }
        let yaw = (self.rotation_ms.wrapping_mul(360) / 5000 % 360) as f32;
        let mut quads = Vec::with_capacity(26);
        let mut add = |texture, position, left_right, near_far, yaw: f32, alpha| {
            if let Some(image) = &textures[texture] {
                let world =
                    Mat4::from_translation(position) * Mat4::from_rotation_y(yaw.to_radians());
                quads.push((
                    position,
                    plane(image, left_right, near_far, world, alpha, Blend::Normal),
                ));
            }
        };
        add(1, Vec3::new(0., 2., 7.15), [-5., 5.], [0., 6.], 0., 255);
        for row in 0..4 {
            for (quad, group) in [0, 1, 1, 2, 2, 3].into_iter().enumerate() {
                let height = if row == 0 {
                    1.25 + group as f32 * 0.25
                } else {
                    2.5 + (row - 1) as f32 * 3. + group as f32 * 0.75
                };
                let angle = if matches!(quad, 0 | 1 | 4) {
                    yaw
                } else {
                    360. - yaw
                };
                let alpha = if row == 0 {
                    255
                } else {
                    self.opacity[(row - 1) * 4 + group]
                };
                add(
                    quad + 2,
                    Vec3::new(2., height, 13.),
                    [-3., 3.],
                    [-3., 3.],
                    angle,
                    alpha,
                );
            }
        }
        add(
            8,
            Vec3::new(2., self.lift - 20., 12.),
            [-6., 6.],
            [0., 20.],
            0.,
            255,
        );
        append_transparent(parts, quads, camera);
    }
}

/// Type 0x56 covers a fixed backdrop and seven independently phased quads.
#[derive(Default)]
struct Spin {
    elapsed: u32,
    yaw: [f32; 7],
}

impl Spin {
    const POSITIONS: [[f32; 3]; 7] = [
        [-0.86, 0.6, 3.],
        [0.8, 1.4, 4.],
        [-0.6, 2.2, 5.],
        [0.6, 3., 6.],
        [-0.4, 3.8, 7.],
        [0.4, 4.6, 8.],
        [-0.2, 5.4, 9.],
    ];
    const PITCH: [f32; 7] = [5., -15., -15., -18., -18., -18., -18.];
    const ROLL: [f32; 7] = [-10., 10., -10., 10., -10., 10., -10.];
    const PHASES: [u32; 7] = [720, 0, 1720, 480, 1500, 640, 1280];

    fn advance(&mut self, delta: u32) {
        self.elapsed = self.elapsed.wrapping_add(delta.min(200));
        let rotation = self.elapsed.wrapping_mul(3600) / 120_000 % 3600;
        self.yaw = Self::PHASES.map(|phase| ((rotation + phase) % 3600) as f32 / 10.);
    }

    fn alpha(&self) -> u8 {
        let phase = self.elapsed % 2000;
        if phase < 1000 {
            255 - (63 * phase / 1000) as u8
        } else {
            192 + (63 * (phase - 1000) / 1000) as u8
        }
    }

    fn geometry(
        &self,
        parts: &mut Vec<Part>,
        textures: &[Option<UploadedImage>],
        camera: [f32; 7],
    ) {
        if let Some(background) = &textures[0] {
            let world = Mat4::from_translation(Vec3::new(0., -3., 8.7));
            parts.push(plane(
                background,
                [-7.5, 7.5],
                [0., 9.],
                world,
                255,
                opaque_blend(background),
            ));
        }
        if let Some(sprite) = &textures[1] {
            let mut quads = Vec::with_capacity(7);
            for (index, position) in Self::POSITIONS
                .map(Vec3::from_array)
                .into_iter()
                .enumerate()
            {
                let world = Mat4::from_translation(position)
                    * Mat4::from_rotation_z(Self::ROLL[index].to_radians())
                    * Mat4::from_rotation_x(Self::PITCH[index].to_radians())
                    * Mat4::from_rotation_y(self.yaw[index].to_radians());
                quads.push((
                    position,
                    plane(
                        sprite,
                        [-1., 1.],
                        [-1., 1.],
                        world,
                        self.alpha(),
                        Blend::Normal,
                    ),
                ));
            }
            append_transparent(parts, quads, camera);
        }
    }
}

fn append_transparent(parts: &mut Vec<Part>, mut quads: Vec<(Vec3, Part)>, camera: [f32; 7]) {
    let eye = Vec3::new(camera[0], camera[1], camera[2]);
    quads.reverse();
    quads.sort_by(|(a, _), (b, _)| b.distance_squared(eye).total_cmp(&a.distance_squared(eye)));
    parts.extend(quads.into_iter().map(|(_, part)| part));
}

fn opaque_blend(image: &UploadedImage) -> Blend {
    if image.opaque {
        Blend::Normal
    } else {
        Blend::StoredByte {
            mode: crysirror_image::rgb_lut::ConstantByteBlend::OpaqueScalar256,
            force_source_opaque: true,
            force_output_opaque: true,
        }
    }
}

fn plane(
    image: &UploadedImage,
    [left, right]: [f32; 2],
    [near, far]: [f32; 2],
    world: Mat4,
    alpha: u8,
    blend: Blend,
) -> Part {
    quad(
        image,
        [
            Vec3::new(left, 0., far),
            Vec3::new(right, 0., far),
            Vec3::new(left, 0., near),
            Vec3::new(right, 0., near),
        ],
        world,
        alpha,
        blend,
    )
}

fn wall(
    image: &UploadedImage,
    [left, right]: [f32; 2],
    [bottom, top]: [f32; 2],
    position: Vec3,
    yaw: f32,
    blend: Blend,
) -> Part {
    let world = Mat4::from_translation(position) * Mat4::from_rotation_y(yaw.to_radians());
    quad(
        image,
        [
            Vec3::new(left, top, 0.),
            Vec3::new(right, top, 0.),
            Vec3::new(left, bottom, 0.),
            Vec3::new(right, bottom, 0.),
        ],
        world,
        255,
        blend,
    )
}

fn quad(image: &UploadedImage, corners: [Vec3; 4], world: Mat4, alpha: u8, blend: Blend) -> Part {
    let uvs = [[0., 0.], [1., 0.], [0., 1.], [1., 1.]];
    let vertices = corners
        .into_iter()
        .zip(uvs)
        .map(|(position, uv)| Vertex {
            position: world.transform_point3(position).extend(1.).to_array(),
            uv,
            color: [1., 1., 1., f32::from(alpha) / 255.],
        })
        .collect();
    Part {
        vertices,
        indices: vec![0, 1, 2, 2, 1, 3],
        texture: Some(image.path.relative().to_string()),
        blend: Some(blend),
        add: [0.; 3],
        linear: true,
    }
}

/// The world camera at 0x140050380 / 0x140062b00 uses left-handed look-at
/// and a 0.1–100 projection. The native Euler helper truncates each vector
/// component to a thousandth before the look-at basis is normalized.
fn view_projection(camera: [f32; 7], size: [f32; 2]) -> Result<Mat4> {
    let fov = camera[6];
    ensure!(
        camera.iter().all(|v| v.is_finite()) && 0. < fov && fov < 180.,
        "CMVS world camera"
    );
    Ok(Mat4::perspective_lh(fov.to_radians(), size[0] / size[1], 0.1, 100.) * camera_view(camera)?)
}

fn camera_view(camera: [f32; 7]) -> Result<Mat4> {
    ensure!(camera.iter().all(|v| v.is_finite()), "CMVS world camera");
    let [x, y, z, pitch, yaw, roll, _] = camera;
    let eye = Vec3::new(x, y, z);
    let [pitch, yaw, roll] =
        [pitch, yaw, roll].map(|degrees| (degrees / 180. * std::f32::consts::PI) as f64);
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let quantized = |v: f64| ((v * 1000.) as i32) as f32 / 1000.;
    let forward = Vec3::new(quantized(sy * cp), quantized(-sp), quantized(cy * cp));
    let up = Vec3::new(quantized(-sr), quantized(cr), 0.);
    Ok(Mat4::look_at_lh(eye, eye + forward, up))
}

pub(super) const WALK_SAMPLES: &[(usize, &str)] =
    &[(16, "se001d.wav"), (17, "se001e.wav"), (20, "se003c.wav")];

/// The native walking sequence stops streamed SE device channel 10.
pub(super) const WALK_STREAM: usize = 1;

/// Type 0x51 drives the shared world camera and resident sound slots.
/// Its per-frame step is 0x14004aa30 / 0x14005ce80.
#[derive(Default)]
pub(super) struct Walk {
    phase: u32,
    queued: Option<u32>,
    elapsed: u32,
    played: bool,
    y: f32,
    z: f32,
}

impl Walk {
    pub fn start(camera: &mut Light) -> Self {
        camera.set_view([0., 1.5, 0.], Some(35.), Some(0.));
        Self {
            y: 1.5,
            ..Self::default()
        }
    }

    pub fn status(&self) -> u32 {
        if self.phase == 0 {
            u32::MAX
        } else {
            self.phase
        }
    }

    pub fn trigger(&mut self, phase: u32) {
        if matches!(phase, 1..=3) {
            self.queued = Some(phase);
        }
    }

    fn transition(&mut self, phase: u32) {
        self.phase = phase;
        self.elapsed = 0;
        self.played = false;
    }

    pub fn advance(&mut self, camera: &mut Light, delta: u32) -> SceneSound {
        match self.phase {
            0 => {
                if let Some(phase) = self.queued.take() {
                    self.transition(phase);
                }
                SceneSound::None
            }
            90 => {
                self.elapsed = self.elapsed.wrapping_add(delta);
                if let Some(phase) = self.queued.take() {
                    self.transition(phase);
                } else if self.elapsed > 600 {
                    self.transition(1);
                }
                SceneSound::None
            }
            1 => self.step(camera, delta, 1000, 250, 0.2, 1., 16, 90),
            2 => self.step(camera, delta, 800, 50, 0.4, 0.8, 17, 2),
            3 => self.land(camera, delta),
            _ => unreachable!("native walking phase"),
        }
    }

    // Stepping has an initial half sine, a short dip and a final quarter sine.
    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        camera: &mut Light,
        delta: u32,
        duration: u32,
        rebound: u32,
        distance: f32,
        height: f32,
        sound: usize,
        next: u32,
    ) -> SceneSound {
        self.elapsed = self.elapsed.wrapping_add(delta.min(200));
        if self.elapsed >= duration + rebound * 2 {
            self.z += distance;
            camera.set_view([0., self.y, self.z], None, None);
            let phase = self.queued.take().unwrap_or(next);
            self.transition(phase);
            return SceneSound::None;
        }
        let elapsed = self.elapsed as f32;
        let duration = duration as f32;
        let rebound = rebound as f32;
        let offset = elapsed - duration;
        let height = if offset < 0. {
            hundredths(sine(elapsed * 180. / duration) * height) / 10.
        } else if offset < rebound {
            -(offset * 0.02 / rebound)
        } else {
            hundredths(sine((offset - rebound) * 90. / rebound) * 0.2) / 10. - 0.02
        };
        let z = if offset < 0. {
            elapsed * distance / duration
        } else {
            distance
        };
        camera.set_view([0., self.y + height, self.z + z], None, None);
        self.cue(offset >= 0., sound, false)
    }

    fn land(&mut self, camera: &mut Light, delta: u32) -> SceneSound {
        self.elapsed = self.elapsed.wrapping_add(delta.min(200));
        if self.elapsed >= 2000 {
            self.y = -0.5;
            self.z += 1.;
            camera.set_view([0., self.y, self.z], Some(80.), None);
            self.transition(0);
            return SceneSound::StopStream;
        }
        let elapsed = self.elapsed as f32;
        let (height, z, pitch) = if elapsed < 1000. {
            let height = hundredths(sine(elapsed * 180. / 1000.)) / 10.;
            (height, elapsed * 0.5 / 1000., None)
        } else {
            let offset = elapsed - 1000.;
            let degrees = 90. - offset * 90. / 1000.;
            let height = hundredths(sine(degrees) * 2.) - 2.;
            (
                height,
                offset * 0.5 / 1000. + 0.5,
                Some(offset * 25. / 1000. + 35.),
            )
        };
        camera.set_view([0., self.y + height, self.z + z], pitch, None);
        self.cue(self.elapsed >= 1800, 20, true)
    }

    fn cue(&mut self, reached: bool, slot: usize, stop_stream: bool) -> SceneSound {
        if reached && !std::mem::replace(&mut self.played, true) {
            SceneSound::Play { slot, stop_stream }
        } else {
            SceneSound::None
        }
    }
}

fn sine(degrees: f32) -> f32 {
    ((degrees / 180. * std::f32::consts::PI) as f64).sin() as f32
}

fn hundredths(value: f32) -> f32 {
    (value * 100.) as i32 as f32 / 100.
}

pub(super) enum SceneSound {
    None,
    Play { slot: usize, stop_stream: bool },
    StopStream,
    StopResident(usize),
    Duplicate(usize, Option<usize>),
}

fn hidden_layer(id: u32, index: usize) -> serde_json::Value {
    json!({"op":"put","layer":{
        "id":id,"rect":[0.,0.,1.,1.],"order":[-1,index],
        "visible":false,"paint":{"kind":"group"}
    }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_uses_native_atlas_draws_and_alternating_integer_angle_clocks() {
        let mut rand = Rand::seeded(1);
        let mut reference = rand.clone();
        let mut scene = Orbit::new(&mut rand);
        for &cell in scene.cells.iter().flatten() {
            assert_eq!(i32::from(cell), reference.next() % 8);
        }
        assert_eq!(rand.next(), reference.next());
        assert_eq!(scene.angles(0, 31), (341., 109.));
        scene.advance(300);
        assert_eq!(scene.time, 200);
        assert_eq!(scene.angles(0, 0), (342., 108.));
        assert_eq!(scene.angles(1, 0), (18., 72.));
        assert!((scene.heights[0] + 0.733).abs() < 0.0001);
        scene.heights[0] = -0.8;
        scene.advance(200);
        assert!((scene.heights[0] - 0.867).abs() < 0.0001);
    }

    #[test]
    fn tower_staggers_groups_and_lift_during_queued_phases() {
        let mut scene = Tower::default();
        assert_eq!(scene.status(), 0);
        scene.trigger(1);
        assert_eq!(scene.status(), u32::MAX);
        assert!(matches!(scene.advance(0), SceneSound::None));
        assert_eq!(scene.status(), 1);
        assert!(matches!(
            scene.advance(250),
            SceneSound::Play { slot: 16, .. }
        ));
        assert_eq!(scene.rotation_ms, 250);
        assert_eq!(scene.phase_ms, 200);
        assert_eq!(&scene.opacity[..3], &[17, 255, 255]);
        scene.trigger(2);
        assert_eq!(scene.status(), u32::MAX);
        for _ in 0..32 {
            assert!(matches!(scene.advance(200), SceneSound::None));
        }
        assert_eq!(scene.opacity, [255; 12]);
        assert!(matches!(scene.advance(10), SceneSound::None));
        assert_eq!(scene.status(), 2);
        assert!(matches!(
            scene.advance(200),
            SceneSound::Play { slot: 17, .. }
        ));
        for _ in 0..9 {
            assert!(matches!(scene.advance(200), SceneSound::None));
        }
        assert_eq!(scene.lift, 32.);
        assert_eq!(&scene.opacity[..3], &[192, 230, 255]);
        for _ in 0..24 {
            assert!(matches!(scene.advance(200), SceneSound::None));
        }
        assert!(matches!(scene.advance(200), SceneSound::StopResident(17)));
        assert_eq!(scene.opacity, [0; 12]);
        assert_eq!(scene.lift, 0.);
        assert_eq!(scene.status(), 0);
    }

    #[test]
    fn rotating_quads_follow_native_phase_offsets_and_alpha_period() {
        let mut scene = Spin::default();
        assert_eq!(scene.yaw, [0.; 7]);
        assert_eq!(scene.alpha(), 255);

        scene.advance(250);
        assert_eq!(scene.elapsed, 200);
        assert_eq!(scene.yaw, [72.6, 0.6, 172.6, 48.6, 150.6, 64.6, 128.6]);
        assert_eq!(scene.alpha(), 243);
        for _ in 0..4 {
            scene.advance(200);
        }
        assert_eq!(scene.alpha(), 192);
        for _ in 0..5 {
            scene.advance(200);
        }
        assert_eq!(scene.alpha(), 255);
    }

    #[test]
    fn walking_cues_and_queued_phases_follow_the_native_clock() {
        let mut camera = Light::default();
        let mut scene = Walk::start(&mut camera);
        assert_eq!(scene.status(), u32::MAX);
        assert_eq!(camera.view()[..5], [0., 1.5, 0., 35., 0.]);

        scene.trigger(1);
        assert!(matches!(scene.advance(&mut camera, 100), SceneSound::None));
        assert_eq!(scene.status(), 1);
        for _ in 0..9 {
            assert!(matches!(scene.advance(&mut camera, 100), SceneSound::None));
        }
        assert!(matches!(
            scene.advance(&mut camera, 100),
            SceneSound::Play {
                slot: 16,
                stop_stream: false
            }
        ));
        for _ in 0..5 {
            assert!(matches!(scene.advance(&mut camera, 100), SceneSound::None));
        }
        assert_eq!(scene.status(), 90);
        assert_eq!(camera.view()[2], 0.2);

        scene.trigger(2);
        scene.advance(&mut camera, 100);
        assert_eq!(scene.status(), 2);
        for _ in 0..7 {
            assert!(matches!(scene.advance(&mut camera, 100), SceneSound::None));
        }
        assert!(matches!(
            scene.advance(&mut camera, 100),
            SceneSound::Play {
                slot: 17,
                stop_stream: false
            }
        ));
        scene.trigger(3);
        scene.advance(&mut camera, 100);
        assert_eq!(scene.status(), 3);
        for _ in 0..17 {
            assert!(matches!(scene.advance(&mut camera, 100), SceneSound::None));
        }
        assert!(matches!(
            scene.advance(&mut camera, 100),
            SceneSound::Play {
                slot: 20,
                stop_stream: true
            }
        ));
        assert!(matches!(
            scene.advance(&mut camera, 200),
            SceneSound::StopStream
        ));
        assert_eq!(scene.status(), u32::MAX);
        assert_eq!(camera.view()[1], -0.5);
        assert_eq!(camera.view()[3], 80.);
    }
}
