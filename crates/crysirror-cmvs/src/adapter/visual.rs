// SPDX-License-Identifier: MPL-2.0
//! CMVS object-slot state and mapping to shared retained layers.

use super::{
    mask,
    visual_opacity::Opacity,
    raster,
    visual_track::{Channel, Track},
};
use crate::vm::{CmvsVm, DrawProperty, ScriptSources, VmProfile};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageResource, RgbaImage},
    service::{HostServices, UploadedImage},
    state::ClientId,
    Error,
};
use crysirror_image::DecoderRegistry;
use crysirror_resource::{read_all, DecodeContext};
use serde_json::json;
use std::sync::Arc;

const IMAGE_LIMIT: u64 = 64 * 1024 * 1024;
const LAYER_BASE: u32 = 256;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Draw {
    crop: [i16; 4],
    position: [i32; 2],
    origin: [i16; 2],
    order: i32,
    opacity: i32,
    scale: [f32; 2],
    #[serde(skip)]
    opacity_track: Option<Opacity>,
    /// Record `+0`: 0 and 1 are screen sprites, 2 and 3 are projected through a camera.
    kind: u32,
    /// Record `+4`. Every native device state composes these sprites as source-over.
    blend: u32,
    /// Record `+80`, in hundredths of a degree.
    rotation: i32,
    /// Record `+44..+52`.
    world: [f32; 3],
    /// Record `+60` and `+64`.
    depth: [f32; 2],
    /// Record `+108`.
    extent: f32,
    /// Solid colour from `0x2043`, drawn without a source image.
    fill: Option<u32>,
    /// Record `+88` (`0x2321`): 4 masks the sprite with the object's second texture
    /// stretched over the screen (0x140032b10 builds that UV from device position).
    #[serde(default)]
    mode: u32,
    /// Record `+92` (`0x2320`): UV flips, 0x10 horizontal and 0x20 vertical (0x1400369a0).
    #[serde(default)]
    flip: u32,
    /// Record `+104` (`0x2322`): the pixel shader pair that outputs `1 - rgb`.
    #[serde(default)]
    invert: bool,
    #[serde(skip)]
    projected: Option<Projected>,
    #[serde(skip)]
    tracks: Vec<Track>,
}

impl Draw {
    fn channel_values(&self, channel: Channel) -> [f32; 6] {
        let mut out = [0.0; 6];
        match channel {
            Channel::Scale => out[..2].copy_from_slice(&self.scale),
            Channel::Rotation => out[0] = self.rotation as f32,
            Channel::Position => {
                out[..2].copy_from_slice(&self.position.map(|value| value as f32));
            }
            Channel::World => out[..3].copy_from_slice(&self.world),
            Channel::Frame => {}
        }
        out
    }

    fn apply_channel(&mut self, channel: Channel, values: [f32; 6]) {
        match channel {
            Channel::Scale => self.scale = [values[0], values[1]],
            Channel::Rotation => self.rotation = values[0] as i32,
            Channel::Position => self.position = [values[0] as i32, values[1] as i32],
            Channel::World => self.world = [values[0], values[1], values[2]],
            Channel::Frame => {
                self.crop = std::array::from_fn(|axis| values[axis] as i16);
                for axis in 0..2 {
                    if values[4 + axis] >= 0.0 {
                        self.origin[axis] = values[4 + axis] as i16;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Projected {
    position: [f32; 2],
    scale: [f32; 2],
    opacity: i32,
    roll: i32,
}

/// 0x140061130: places a world-space sprite through one of the three camera modes.
fn project(camera: &[u32; 40], draw: &Draw) -> Option<Projected> {
    let f = |index: usize| f32::from_bits(camera[index]);
    let [x, y, z] = draw.world;
    let [lift, focus] = draw.depth;
    let distance = z - f(3);
    let roll = (f(14) * 100.0) as i32 % 36000;
    match camera[0] {
        1 => {
            if distance <= 0.0 {
                return None;
            }
            let scale = if distance == focus {
                1.0
            } else {
                let step = (focus - distance) * 0.5;
                if step < 0.0 {
                    1.0 / (step.abs() + 1.0)
                } else {
                    step + 1.0
                }
            };
            let fade = if distance >= 0.5 {
                255
            } else {
                (distance * 512.0) as i32
            };
            let half = f(6) * 0.5;
            let view = 1.0 / ((half - (f(6) - distance) * 0.5) / half);
            let sx = (x - f(1)) * view * (f(9) / f(7)) + f(9) * 0.5;
            let sy = ((f(2) - y) * view + lift) * (f(10) / f(8))
                + f(10) * 0.5
                + (focus - distance) * f(11);
            Some(Projected {
                position: [sx, sy],
                scale: [scale; 2],
                opacity: fade * draw.opacity / 255,
                roll,
            })
        }
        mode @ (2 | 3) => {
            if distance <= 9.0 {
                return None;
            }
            let yaw = f(13);
            let shift = if yaw != 0.0 {
                f(7) * 0.5 * (yaw / 45.0) * (distance / f(6))
            } else {
                0.0
            };
            let view = focus / distance;
            let fade = if distance >= 10.0 {
                255
            } else {
                ((distance - 9.0) * 256.0) as i32
            };
            let aspect = distance / f(6);
            let (zoom_x, zoom_y, lead) = if mode == 2 {
                (1.0, 1.0, 0.30000001)
            } else {
                (f(17), f(18), 0.0)
            };
            let sx = f(9) / (aspect * f(7)) / zoom_x * ((x - (shift + f(1))) * view) + f(15);
            let vertical = if mode == 2 { f(10) / f(8) } else { f(10) / (aspect * f(8)) / zoom_y };
            let sy = ((f(2) - y) * view + lift) * vertical
                + f(16)
                + (focus - distance + lead) * f(11);
            Some(Projected {
                position: [sx, sy],
                scale: [view * view; 2],
                opacity: fade * draw.opacity / 255,
                roll,
            })
        }
        _ => None,
    }
}

/// Special scenes use the same native projected draw record and atlas crop.
pub(super) fn projected_atlas_cell(
    id: u32,
    scene: usize,
    image: &UploadedImage,
    cell: i16,
    world: [f32; 3],
    rotation: i32,
    camera: &[u32; 40],
    canvas: [u32; 2],
) -> Result<Option<serde_json::Value>> {
    let mut draw = Draw {
        kind: 2,
        world,
        depth: [0., 5.],
        crop: [256 * (cell / 4), 192 * (cell % 4), 256, 192],
        origin: [128, 96],
        ..Draw::default()
    };
    draw.projected = project(camera, &draw).map(|mut projection| {
        projection.roll = (projection.roll + rotation) % 36000;
        projection
    });
    let logical_size = [camera[9], camera[10]].map(|word| f32::from_bits(word) as u32);
    put(id, scene, u64::from(id), &draw, Some(image), None, logical_size, canvas, [0; 2], [0.; 2])
}

/// `0x2056` (0x1400617a0): screen position of a world point through a mode 3 camera.
/// `alternate` selects the camera's second eye (+92..+100) when it is enabled (+84).
/// Other camera modes return the point unchanged.
pub(super) fn project_point(camera: &[u32; 40], alternate: bool, point: [f32; 3], lift: f32, focus: f32) -> [f32; 3] {
    let f = |index: usize| f32::from_bits(camera[index]);
    if camera[0] != 3 {
        return point;
    }
    let eye = if alternate && camera[21] != 0 { [f(23), f(24), f(25)] } else { [f(1), f(2), f(3)] };
    let [x, y, z] = point;
    let distance = z - eye[2];
    let view = focus / distance;
    let shift = if f(13) != 0.0 { f(7) * 0.5 * (f(13) / 45.0) * (distance / f(6)) } else { 0.0 };
    let aspect = distance / f(6);
    let sy = f(10) / (aspect * f(8)) / f(18) * ((eye[1] - y) * view + lift) + f(16) + (focus - distance) * f(11);
    let sx = f(9) / (aspect * f(7)) / f(17) * ((x - (shift + eye[0])) * view) + f(15);
    [sx, sy, 0.0]
}

/// One child sprite posed by an effect for the current frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pose {
    pub crop: [i16; 4],
    pub origin: [i16; 2],
    pub position: [i32; 2],
    /// Record kind: 0 screen, 2 and 3 projected through camera 0 or 1 at `world`.
    pub kind: u32,
    pub world: [f32; 3],
    /// Record +60/+64, used by the camera projection.
    pub depth: [f32; 2],
    pub scale: [f32; 2],
    /// Hundredths of a degree.
    pub rotation: i32,
    pub opacity: i32,
    pub order: i32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            crop: [0; 4],
            origin: [0; 2],
            position: [0; 2],
            kind: 0,
            world: [0.0; 3],
            depth: [0.0; 2],
            scale: [1.0; 2],
            rotation: 0,
            opacity: 255,
            order: 0,
        }
    }
}

impl Default for Draw {
    fn default() -> Self {
        Self {
            crop: [0; 4],
            position: [0; 2],
            origin: [0; 2],
            order: 0,
            opacity: 255,
            scale: [1.0; 2],
            opacity_track: None,
            kind: 0,
            blend: 1,
            rotation: 0,
            world: [0.0; 3],
            depth: [0.0; 2],
            extent: 0.0,
            fill: None,
            mode: 0,
            flip: 0,
            invert: false,
            projected: None,
            tracks: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
struct Object {
    image: Option<UploadedImage>,
    native_alpha: Option<bool>,
    draw: Option<Draw>,
    children: std::collections::BTreeMap<usize, Child>,
    published: bool,
    flags: Flags,
    mask: Option<mask::Sample>,
    /// Scene movie picture owned by the media service rather than this object.
    movie: Option<UploadedImage>,
    /// Previous picture shown beneath a `0x22c4` crossfade.
    crossfade: Option<Crossfade>,
    /// Native image name, kept so a save can reload it.
    source: Option<String>,
    /// Scene movie slot and file name.
    movie_source: Option<(usize, String)>,
    /// `0x2035` second texture: its name and the screen-sized matte of its green channel.
    second: Option<(String, UploadedImage)>,
    /// Effect objects (`0x22a1..=0x22a9`) draw only their posed children.
    children_only: bool,
    /// `0x2328` raster: the object draws these rows instead of itself.
    raster: Option<raster::Raster>,
    raster_published: Vec<bool>,
    /// `0x2330..=0x2332` collision points in object space.
    points: Option<Vec<[f32; 3]>>,
}

impl Object {
    fn owned_images(&self) -> impl Iterator<Item = &UploadedImage> {
        self.image.iter().chain(self.second.iter().map(|(_, matte)| matte)).chain(
            self.children
                .values()
                .filter_map(|child| child.image.as_ref()),
        )
    }
}

#[derive(Clone)]
struct Crossfade {
    previous: UploadedImage,
    duration: u32,
    elapsed: u32,
    published: bool,
}

#[derive(Clone)]
struct Child {
    source: Option<String>,
    image: Option<UploadedImage>,
    draw: Option<Draw>,
    inherits: bool,
    flags: Flags,
    published: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Flags {
    shown: bool,
    follows_quake: bool,
    size: [u32; 2],
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            shown: true,
            follows_quake: true,
            size: [0; 2],
        }
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(super) struct ChildImage {
    pub child: u32,
    pub crop: [i16; 4],
    pub origin: [i16; 2],
}

impl ChildImage {
    pub const UNSET: u32 = u32::MAX;
}

impl Default for ChildImage {
    fn default() -> Self {
        Self {
            child: Self::UNSET,
            crop: [0; 4],
            origin: [0; 2],
        }
    }
}

#[derive(Clone)]
pub(super) struct Objects {
    slots: std::collections::BTreeMap<usize, Object>,
    canvas: [u32; 2],
    quake: [[i32; 2]; 2],
    cameras: [[u32; 40]; 2],
}

impl Objects {
    fn reproject(draw: &mut Draw, cameras: &[[u32; 40]; 2]) {
        draw.projected = (draw.kind >= 2)
            .then(|| project(&cameras[usize::from(draw.kind == 3)], draw))
            .flatten();
    }

    /// Republishes projected sprites after the VM changes either camera.
    pub fn set_cameras(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        cameras: [[u32; 40]; 2],
    ) -> Result<()> {
        if cameras == self.cameras {
            return Ok(());
        }
        self.cameras = cameras;
        let mut commands = Vec::new();
        for (&slot, object) in &mut self.slots {
            let mut touched = false;
            for draw in object
                .draw
                .iter_mut()
                .chain(object.children.values_mut().filter_map(|child| child.draw.as_mut()))
                .filter(|draw| draw.kind >= 2)
            {
                Self::reproject(draw, &cameras);
                touched = true;
            }
            if touched {
                commands.extend(plan(slot, object, self.canvas, self.quake)?);
            }
        }
        submit(host, client, commands)
    }

    /// Keyframe channel commands `0x21e0..=0x21fd`, excluding opacity.
    pub fn track_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<Option<u32>> {
        let (channel, command) = Channel::of(opcode).context("CMVS draw track opcode")?;
        let words = channel.words(command);
        let [slot, child] = vm.owner_arguments(opcode)?;
        let (slot, child) = (slot as i32, child as i32);
        if !(0..256).contains(&slot) || !self.slots.contains_key(&(slot as usize)) {
            vm.complete_owner_call(words, None)?;
            return Ok(Some(0x10));
        }
        if child >= 1024 {
            vm.complete_owner_call(words, None)?;
            return Ok(Some(0x20));
        }
        let slot = slot as usize;
        let mut changed = self.slots[&slot].clone();
        let record = match usize::try_from(child).ok() {
            Some(index) => changed.children.get_mut(&index).context("CMVS draw child missing")?.draw.as_mut(),
            None => changed.draw.as_mut(),
        }
        .context("CMVS object has no draw record")?;
        let position = record.tracks.iter().position(|track| track.channel() == channel);
        let mut result = None;
        match command {
            0 => {
                let mut base = record.channel_values(channel);
                match channel {
                    Channel::Scale => {
                        let [_, _, x, y] = vm.owner_arguments(opcode)?;
                        base[..2].copy_from_slice(&[f32::from_bits(x), f32::from_bits(y)]);
                    }
                    Channel::Rotation => {
                        let [_, _, raw] = vm.owner_arguments(opcode)?;
                        base[0] = ((raw as i32) % 360 * 100) as f32;
                    }
                    _ => {}
                }
                record.tracks.retain(|track| track.channel() != channel);
                record.tracks.push(Track::new(channel, base));
            }
            1 => {
                if let Some(index) = position {
                    let mut raw = [0; 9];
                    for (offset, word) in raw.iter_mut().take(usize::from(words) - 2).enumerate() {
                        *word = vm.owner_argument(opcode, 12 + offset * 4)?;
                    }
                    record.tracks[index].set_key(&raw)?;
                }
            }
            2 => {
                let [_, _, key] = vm.owner_arguments(opcode)?;
                if let Some(index) = position {
                    let base = record.channel_values(channel);
                    if let Some(values) = record.tracks[index].start(key as usize, base)? {
                        record.apply_channel(channel, values);
                    }
                }
            }
            3 => {
                if let Some(index) = position {
                    if let Some(values) = record.tracks[index].finish() {
                        record.apply_channel(channel, values);
                    }
                    record.tracks.remove(index);
                }
            }
            _ => {
                result = Some(position.map_or(u32::MAX, |index| record.tracks[index].state()));
            }
        }
        Self::reproject(record, &self.cameras);
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        vm.complete_owner_call(words, result)?;
        Ok(None)
    }

    /// Draw record commands `0x2025..=0x2078`. `Some(bit)` is the native error bit to latch.
    pub fn property_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<Option<u32>> {
        let words: u16 = match opcode {
            0x2041 => 1,
            0x2025 | 0x202c | 0x202d | 0x2039 | 0x2042 | 0x204f | 0x2051 | 0x2078 => 2,
            0x2049 | 0x2054 => 4,
            0x2043 => 5,
            _ => 3,
        };
        let [slot] = vm.owner_arguments(opcode)?;
        let child = if opcode == 0x2041 {
            -1
        } else {
            vm.owner_argument(opcode, 8)? as i32
        };
        let slot = slot as i32;
        if !(0..256).contains(&slot) || !self.slots.contains_key(&(slot as usize)) {
            vm.complete_owner_call(words, None)?;
            return Ok(Some(0x10));
        }
        let child_limit = match (opcode, vm.profile()) {
            (0x2321, VmProfile::Pc20160201X64) => 768,
            _ => 1024,
        };
        if child >= child_limit || (matches!(opcode, 0x202c | 0x2039 | 0x2051) && child < 0) {
            vm.complete_owner_call(words, None)?;
            return Ok(Some(if child < 0 { 0x10 } else { 0x20 }));
        }
        let slot = slot as usize;
        let argument = |distance| vm.owner_argument(opcode, distance);
        let float = |distance| vm.owner_argument(opcode, distance).map(f32::from_bits);
        let mut changed = self.slots[&slot].clone();
        let index = usize::try_from(child).ok();
        if let Some(index) = index {
            ensure!(changed.children.contains_key(&index), "CMVS draw child missing");
        }
        match opcode {
            0x2025 | 0x202d => {}
            0x202c => {
                changed.children.get_mut(&index.context("CMVS child")?).context("CMVS child")?.inherits = false;
            }
            0x2026 => {
                let shown = argument(12)? != 0;
                match index {
                    Some(index) => changed.children.get_mut(&index).context("CMVS child")?.flags.shown = shown,
                    None => changed.flags.shown = shown,
                }
            }
            0x2039 => {
                let child = changed.children.get_mut(&index.context("CMVS child")?).context("CMVS child")?;
                if let Some(image) = child.image.take() {
                    update(host, client, slot, &mut changed, self.canvas, self.quake)?;
                    host.release_image_resource(client, image.path.relative())?;
                    self.slots.insert(slot, changed);
                    vm.complete_owner_call(words, None)?;
                    return Ok(None);
                }
            }
            0x2041 | 0x2051 => match index {
                Some(index) => changed.children.get_mut(&index).context("CMVS child")?.draw = None,
                None => changed.draw = None,
            },
            _ => {
                let draw = match index {
                    Some(index) => changed.children.get_mut(&index).context("CMVS child")?.draw.as_mut(),
                    None => changed.draw.as_mut(),
                }
                .context("CMVS object has no draw record")?;
                match opcode {
                    0x2042 => draw.kind = 2,
                    0x204c => draw.kind = 2 + u32::from(argument(12)? != 0),
                    0x204e => draw.kind = u32::from(argument(12)? != 0),
                    0x2055 => {
                        let kind = argument(12)?;
                        if kind <= 3 {
                            draw.kind = kind;
                        }
                    }
                    0x2043 => {
                        let [width, height, color] = [argument(12)?, argument(16)?, argument(20)?];
                        draw.crop = [0, 0, width as i16, height as i16];
                        draw.fill = Some(color | 0xff00_0000);
                    }
                    0x2049 => draw.scale = [float(12)?, float(16)?],
                    0x204a => draw.rotation = (argument(12)? as i32).wrapping_abs() % 360 * 100,
                    0x204b => {
                        let blend = argument(12)?;
                        if blend <= 3 {
                            draw.blend = blend;
                        }
                    }
                    0x2054 => draw.world = [float(12)?, float(16)?, 0.0],
                    0x2070 => draw.depth[0] = float(12)?,
                    0x2071 => draw.depth[1] = float(12)?,
                    0x2072 => draw.world[0] = float(12)?,
                    0x2073 => draw.world[1] = float(12)?,
                    0x2074 => draw.world[2] = float(12)?,
                    0x2075 => draw.extent = float(12)?,
                    0x2320 => draw.flip = argument(12)?,
                    0x2321 => draw.mode = argument(12)?,
                    0x2322 => draw.invert = argument(12)? != 0,
                    0x204f => {
                        let (scale, values) = (draw.scale, [draw.position[0] as u32, draw.position[1] as u32, draw.order as u32]);
                        let opacity = draw.opacity as u32;
                        vm.complete_owner_typed_results(
                            words,
                            &[(0, opacity), (1, values[0]), (2, values[1]), (3, values[2])],
                            &[(0, scale[0].to_bits()), (1, scale[1].to_bits())],
                        )?;
                        return Ok(None);
                    }
                    0x2078 => {
                        let world = draw.world.map(f32::to_bits);
                        vm.complete_owner_typed_results(
                            words,
                            &[],
                            &[(0, world[0]), (1, world[1]), (2, world[2])],
                        )?;
                        return Ok(None);
                    }
                    _ => anyhow::bail!("unhandled CMVS draw record command 0x{opcode:04x}"),
                }
                Self::reproject(draw, &self.cameras);
            }
        }
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        vm.complete_owner_call(words, None)?;
        Ok(None)
    }

    pub fn window_offset(&self) -> [i32; 2] {
        self.quake[1]
    }
    pub fn opacity_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<()> {
        let [slot, child] = vm.owner_arguments(opcode)?;
        ensure!(
            slot < 256 && (child as i32) < 768,
            "CMVS opacity target outside object slots"
        );
        let slot = slot as usize;
        let current = self
            .slots
            .get_mut(&slot)
            .context("CMVS opacity object missing")?;
        let mut candidate = current.clone();
        let draw = if (child as i32) < 0 {
            candidate.draw.as_mut()
        } else {
            candidate
                .children
                .get_mut(&(child as usize))
                .and_then(|child| child.draw.as_mut())
        }
        .context("CMVS opacity target has no draw state")?;
        let old = draw.opacity;
        let (words, result) =
            Opacity::command(opcode, vm, &mut draw.opacity_track, &mut draw.opacity)?;
        if draw.opacity != old {
            update(host, client, slot, &mut candidate, self.canvas, self.quake)?;
        }
        vm.complete_owner_call(words, result)?;
        *current = candidate;
        Ok(())
    }

    pub fn set_quake(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        offset: [[i32; 2]; 2],
    ) -> Result<()> {
        if self.quake == offset {
            return Ok(());
        }
        let mut commands = Vec::new();
        for (&slot, object) in &self.slots {
            let group = usize::from(slot >= 256);
            if self.quake[group] != offset[group] && object.flags.follows_quake {
                commands.extend(plan(slot, &mut object.clone(), self.canvas, offset)?);
            }
        }
        submit(host, client, commands)?;
        self.quake = offset;
        Ok(())
    }

    pub fn advance_opacity(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        delta: u32,
    ) -> Result<()> {
        fn active(draw: &Option<Draw>) -> bool {
            draw.as_ref().is_some_and(|draw| {
                draw.opacity_track.as_ref().is_some_and(Opacity::playing)
                    || draw.tracks.iter().any(Track::playing)
            })
        }
        let cameras = self.cameras;
        let sample = |draw: &mut Option<Draw>, delta: u32| -> bool {
            if let Some(draw) = draw {
                let mut tracks = std::mem::take(&mut draw.tracks);
                let mut expired = false;
                tracks.retain_mut(|track| {
                    let (values, end) = track.advance(delta);
                    if let Some(values) = values {
                        draw.apply_channel(track.channel(), values);
                    }
                    expired |= end == Some(-2);
                    end.is_none()
                });
                draw.tracks = tracks;
                Self::reproject(draw, &cameras);
                if expired {
                    return true;
                }
                if let Some(end) = draw
                    .opacity_track
                    .as_mut()
                    .and_then(|track| track.advance(delta, &mut draw.opacity))
                {
                    draw.opacity_track = None;
                    return end == -2;
                }
            }
            false
        };
        let mut expired = Vec::new();
        for (&slot, current) in self.slots.range_mut(..256) {
            if !active(&current.draw) && !current.children.values().any(|child| active(&child.draw))
            {
                continue;
            }
            let mut candidate = current.clone();
            if sample(&mut candidate.draw, delta) {
                expired.push(slot);
                continue;
            }
            let mut removed = Vec::new();
            let mut retired = Vec::new();
            candidate.children.retain(|&index, child| {
                let expired = sample(&mut child.draw, delta);
                if expired {
                    if child.published {
                        removed.push(remove(child_id(slot, index)));
                    }
                    retired.extend(child.image.take());
                }
                !expired
            });
            removed.extend(plan(slot, &mut candidate, self.canvas, self.quake)?);
            submit(host, client, removed)?;
            *current = candidate;
            for image in retired {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        for slot in expired {
            self.reset(host, client, slot, None, false)?;
        }
        Ok(())
    }

    pub fn new(canvas: [u32; 2]) -> Self {
        Self {
            slots: Default::default(),
            canvas,
            quake: [[0; 2]; 2],
            cameras: [[0; 40]; 2],
        }
    }

    pub fn has_child(&self, slot: usize, child: usize) -> bool {
        self.slots
            .get(&slot)
            .is_some_and(|object| object.children.contains_key(&child))
    }

    pub fn contains(&self, slot: usize) -> bool {
        self.slots.contains_key(&slot)
    }

    pub fn move_to(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        target: usize,
        source: usize,
    ) -> Result<()> {
        ensure!(target != source, "CMVS cannot move an object onto itself");
        let mut commands = Vec::new();
        let destination = self.slots.get(&target);
        let origin = self.slots.get(&source);
        commands.extend(
            destination
                .into_iter()
                .flat_map(|object| visible_ids(target, object))
                .map(remove),
        );
        commands.extend(
            origin
                .into_iter()
                .flat_map(|object| visible_ids(source, object))
                .map(remove),
        );
        let moved = origin
            .cloned()
            .map(|mut object| {
                object.published = false;
                for child in object.children.values_mut() {
                    child.published = false;
                }
                commands.extend(plan(target, &mut object, self.canvas, self.quake)?);
                Ok::<_, anyhow::Error>(object)
            })
            .transpose()?;
        submit(host, client, commands)?;
        self.slots.remove(&source);
        let replaced = self.slots.remove(&target);
        if let Some(object) = moved {
            self.slots.insert(target, object);
        }
        if let Some(object) = replaced {
            for image in object.owned_images() {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let layers = self
            .slots
            .range(..256)
            .flat_map(|(&slot, object)| visible_ids(slot, object));
        submit(host, client, layers.map(remove).collect())?;
        let windows = self.slots.split_off(&256);
        for object in std::mem::replace(&mut self.slots, windows).into_values() {
            for image in object.owned_images() {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        Ok(())
    }

    pub fn reset_all(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        self.clear(host, client)?;
        let window_slots: Vec<_> = self.slots.keys().copied().collect();
        for slot in window_slots {
            self.reset(host, client, slot, None, false)?;
        }
        self.quake = [[0; 2]; 2];
        Ok(())
    }

    pub fn capture(&self, scope: &mut crysirror_core::scene_checkpoint::SceneCheckpointScope) {
        for (&slot, object) in &self.slots {
            scope.layer_roots.extend(visible_ids(slot, object));
            scope.generated_images.extend(
                object
                    .owned_images()
                    .map(|image| image.path.relative().to_owned()),
            );
            if let Some(mask) = &object.mask {
                scope.generated_images.push(mask.asset().to_owned());
            }
        }
    }

    pub fn reset(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        create: bool,
    ) -> Result<()> {
        self.replace(host, client, slot, child, create, None)
    }

    pub fn replace_preview(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        image: Option<DecodedImage>,
    ) -> Result<()> {
        self.replace(host, client, slot, child, true, image)
    }

    fn replace(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        create: bool,
        image: Option<DecodedImage>,
    ) -> Result<()> {
        if let Some(index) = child {
            let current = self
                .slots
                .get_mut(&slot)
                .context("CMVS child parent missing")?;
            let id = child_id(slot, index);
            let commands = current
                .children
                .get(&index)
                .filter(|child| child.published)
                .map(|_| remove(id))
                .into_iter()
                .collect();
            let image = replace_resource(host, client, id, image, commands)?;
            let previous = if create {
                current.children.insert(
                    index,
                    Child {
                        source: None,
                        image: image.map(|(image, _)| image),
                        draw: None,
                        inherits: current.draw.is_some(),
                        flags: Flags::default(),
                        published: false,
                    },
                )
            } else {
                current.children.remove(&index)
            };
            if let Some(image) = previous.and_then(|child| child.image) {
                host.release_image_resource(client, image.path.relative())?;
            }
            return Ok(());
        }

        let commands = self
            .slots
            .get(&slot)
            .into_iter()
            .flat_map(|object| visible_ids(slot, object))
            .map(remove)
            .collect();
        let image = replace_resource(host, client, layer_id(slot), image, commands)?;
        let previous = if create {
            let mut replacement = Object::default();
            if let Some((image, alpha)) = image {
                replacement.image = Some(image);
                replacement.native_alpha = alpha;
            }
            self.slots.insert(slot, replacement)
        } else {
            self.slots.remove(&slot)
        };
        if let Some(object) = previous {
            for image in object.owned_images() {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        Ok(())
    }

    pub fn image(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        image: DecodedImage,
    ) -> Result<()> {
        let current = self
            .slots
            .get_mut(&slot)
            .context("CMVS image object missing")?;
        let published = host.publish_image_resource(client, layer_id(slot), image.image)?;
        let mut changed = current.clone();
        changed.image = Some(published.clone());
        changed.native_alpha = image.native_alpha;
        if let Err(error) = update(host, client, slot, &mut changed, self.canvas, self.quake) {
            host.release_image_resource(client, published.path.relative())?;
            return Err(error);
        }
        let previous = std::mem::replace(current, changed);
        if let Some(previous) = previous.image {
            host.release_image_resource(client, previous.path.relative())?;
        }
        Ok(())
    }

    /// Creates an effect-owned object at `slot` that shows only its posed children.
    pub fn create_effect_object(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        image: Option<DecodedImage>,
    ) -> Result<()> {
        self.remove_object(host, client, slot)?;
        self.replace(host, client, slot, None, true, image)?;
        let object = self.slots.get_mut(&slot).context("CMVS effect object")?;
        object.children_only = true;
        object.draw = Some(Draw::default());
        Ok(())
    }

    /// `0x22a8`: moves a script object into an effect slot; the script slot becomes empty.
    pub fn adopt(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        from: usize,
        to: usize,
    ) -> Result<bool> {
        let Some(mut object) = self.slots.remove(&from) else {
            return Ok(false);
        };
        let removed: Vec<_> = visible_ids(from, &object).map(remove).collect();
        submit(host, client, removed)?;
        object.published = false;
        for child in object.children.values_mut() {
            child.published = false;
        }
        object.children_only = true;
        self.remove_object(host, client, to)?;
        self.slots.insert(to, object);
        let mut changed = self.slots[&to].clone();
        update(host, client, to, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(to, changed);
        Ok(true)
    }

    /// Next unused child index of an object (0x14004ea30 numbers children in order).
    pub fn next_child(&self, slot: usize) -> usize {
        self.slots.get(&slot).and_then(|object| object.children.keys().next_back()).map_or(0, |last| last + 1)
    }

    /// Published picture of an object or one of its children, and whether it has alpha.
    pub fn picture(&self, slot: usize, child: Option<usize>) -> Option<(String, bool)> {
        let object = self.slots.get(&slot)?;
        let image = match child {
            Some(index) => object.children.get(&index)?.image.as_ref().or(object.image.as_ref())?,
            None => object.image.as_ref()?,
        };
        Some((image.path.relative().to_owned(), object.native_alpha.unwrap_or(true)))
    }

    /// Where list items are measured from (0x14001bb00): world x/y for record kind 5,
    /// else the screen position.
    pub fn anchor(&self, slot: usize) -> [i32; 2] {
        self.slots.get(&slot).and_then(|object| object.draw.as_ref()).map_or([0; 2], |draw| {
            if draw.kind == 5 {
                [draw.world[0] as i32, draw.world[1] as i32]
            } else {
                draw.position
            }
        })
    }

    /// Logical screen size, which screen-space effects wrap within.
    pub fn canvas(&self) -> [u32; 2] {
        self.canvas
    }

    /// Image size of an object, for effects that cut it into pieces.
    pub fn object_image_size(&self, slot: usize) -> Option<[u32; 2]> {
        self.slots.get(&slot).and_then(|object| object.image.as_ref()).map(|image| image.size)
    }

    /// Removes an object and releases its images.
    pub fn remove_object(&mut self, host: &mut dyn HostServices, client: ClientId, slot: usize) -> Result<()> {
        let Some(object) = self.slots.remove(&slot) else {
            return Ok(());
        };
        submit(host, client, visible_ids(slot, &object).map(remove).collect())?;
        for image in object.owned_images() {
            host.release_image_resource(client, image.path.relative())?;
        }
        Ok(())
    }

    /// Sets the given children of an effect object for this frame in one layer batch.
    pub fn pose_children(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        poses: &[(usize, Pose)],
    ) -> Result<()> {
        let mut changed = self.slots.get(&slot).context("CMVS effect object")?.clone();
        for &(index, pose) in poses {
            let child = changed.children.entry(index).or_insert_with(|| Child {
                source: None,
                image: None,
                draw: None,
                inherits: true,
                flags: Flags::default(),
                published: false,
            });
            let draw = child.draw.get_or_insert_with(Draw::default);
            draw.crop = pose.crop;
            draw.origin = pose.origin;
            draw.position = pose.position;
            draw.kind = pose.kind;
            draw.world = pose.world;
            draw.depth = pose.depth;
            draw.scale = pose.scale;
            draw.rotation = pose.rotation;
            draw.opacity = pose.opacity;
            draw.order = pose.order;
            Self::reproject(draw, &self.cameras);
        }
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        Ok(())
    }

    /// Drops children an effect has retired, removing their published layers.
    pub fn remove_children(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        children: &[usize],
    ) -> Result<()> {
        let Some(object) = self.slots.get_mut(&slot) else {
            return Ok(());
        };
        let removed = children
            .iter()
            .filter_map(|index| object.children.remove(index).filter(|child| child.published).map(|_| remove(child_id(slot, *index))))
            .collect();
        submit(host, client, removed)
    }

    /// `0x2035`: replaces the object's second texture with a screen-sized matte.
    /// A missing image leaves the object without one, as the failed CTexture has no texels.
    pub fn set_second(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        name: String,
        matte: Option<DecodedImage>,
    ) -> Result<()> {
        let current = self.slots.get_mut(&slot).context("CMVS image object missing")?;
        let published = matte
            .map(|matte| host.publish_image_resource(client, 700_000 + slot as u32, matte.image))
            .transpose()?;
        let mut changed = current.clone();
        changed.second = published.clone().map(|matte| (name, matte));
        if let Err(error) = update(host, client, slot, &mut changed, self.canvas, self.quake) {
            if let Some(matte) = published {
                host.release_image_resource(client, matte.path.relative())?;
            }
            return Err(error);
        }
        let previous = std::mem::replace(current, changed);
        if let Some((_, matte)) = previous.second {
            host.release_image_resource(client, matte.path.relative())?;
        }
        Ok(())
    }

    /// `0x2034` (0x140043bc0): the stored texel at (x, y) as 0xAARRGGBB, or 0 outside
    /// the texture or without one.
    pub fn pixel(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        point: [i32; 2],
    ) -> Result<u32> {
        let Some(object) = self.slots.get(&slot) else {
            return Ok(0);
        };
        let image = match child {
            Some(child) => object.children.get(&child).and_then(|child| child.image.as_ref()),
            None => object.image.as_ref(),
        };
        let Some(image) = image else {
            return Ok(0);
        };
        let [width, height] = image.size;
        let [x, y] = point;
        if x < 0 || y < 0 || x as u32 >= width || y as u32 >= height {
            return Ok(0);
        }
        let Some(resource) = host.published_image_resource(client, image.path.relative())? else {
            return Ok(0);
        };
        let at = (y as usize * width as usize + x as usize) * 4;
        let [r, g, b, a] = resource.rgba()[at..at + 4] else {
            return Ok(0);
        };
        Ok(u32::from_be_bytes([a, r, g, b]))
    }

    /// `0x22c4`: shows a new image over the current one and fades it in.
    pub fn crossfade_image(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        image: DecodedImage,
        duration: u32,
    ) -> Result<()> {
        let current = self.slots.get_mut(&slot).context("CMVS image object missing")?;
        if duration == 0 || current.image.is_none() {
            return self.image(host, client, slot, image);
        }
        self.finish_crossfade(host, client, slot)?;
        let current = self.slots.get_mut(&slot).context("CMVS image object missing")?;
        let published = host.publish_image_resource(client, crossfade_id(slot) + 10_000, image.image)?;
        let previous = current.image.replace(published).context("CMVS crossfade source")?;
        current.native_alpha = image.native_alpha;
        current.crossfade = Some(Crossfade { previous, duration, elapsed: 0, published: false });
        update(host, client, slot, current, self.canvas, self.quake)
    }

    pub fn crossfade_active(&self, slot: usize) -> bool {
        self.slots.get(&slot).is_some_and(|object| object.crossfade.is_some())
    }

    /// `0x22c5`: ends a crossfade on the new picture.
    pub fn finish_crossfade(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
    ) -> Result<()> {
        let Some(object) = self.slots.get_mut(&slot) else {
            return Ok(());
        };
        let Some(fade) = object.crossfade.take() else {
            return Ok(());
        };
        let mut commands = Vec::new();
        if fade.published {
            commands.push(remove(crossfade_id(slot)));
        }
        commands.extend(plan(slot, object, self.canvas, self.quake)?);
        submit(host, client, commands)?;
        host.release_image_resource(client, fade.previous.path.relative())?;
        Ok(())
    }

    pub fn advance_crossfades(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        delta: u32,
    ) -> Result<()> {
        let active: Vec<_> = self
            .slots
            .iter()
            .filter(|(_, object)| object.crossfade.is_some())
            .map(|(slot, _)| *slot)
            .collect();
        for slot in active {
            let object = self.slots.get_mut(&slot).context("CMVS crossfade object")?;
            let fade = object.crossfade.as_mut().context("CMVS crossfade state")?;
            fade.elapsed = fade.elapsed.saturating_add(delta.min(200));
            if fade.elapsed >= fade.duration {
                self.finish_crossfade(host, client, slot)?;
            } else {
                update(host, client, slot, object, self.canvas, self.quake)?;
            }
        }
        Ok(())
    }

    /// Records the native name of an image just loaded into a slot or child.
    /// `0x2328` (0x14004f5d0): starts a raster over the object's current crop. False
    /// for a missing object, which latches fault 0x10.
    pub fn start_raster(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        mode: u32,
        words: [u32; 8],
    ) -> Result<bool> {
        let Some(object) = self.slots.get(&slot) else {
            return Ok(false);
        };
        let mut changed = object.clone();
        let size = changed.draw.as_ref().map_or([0; 2], |draw| [i32::from(draw.crop[2]), i32::from(draw.crop[3])]);
        changed.raster = Some(raster::Raster::new(mode, words, size));
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        Ok(true)
    }

    /// `0x2329` (0x14004fb40): stops the raster and drops its rows.
    pub fn stop_raster(&mut self, host: &mut dyn HostServices, client: ClientId, slot: usize) -> Result<bool> {
        let Some(object) = self.slots.get(&slot) else {
            return Ok(false);
        };
        let mut changed = object.clone();
        changed.raster = None;
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        Ok(true)
    }

    /// 0x14004fb60 for every object: mode 1 rows swing with the frame delta.
    pub fn advance_rasters(&mut self, host: &mut dyn HostServices, client: ClientId, delta: u32) -> Result<()> {
        let mut commands = Vec::new();
        for (&slot, object) in &mut self.slots {
            if object.raster.as_mut().is_some_and(|raster| raster.advance(delta)) {
                commands.extend(plan(slot, object, self.canvas, self.quake)?);
            }
        }
        submit(host, client, commands)
    }

    /// `0x2330..=0x2332`: creates, frees or extends an object's point list. False for a
    /// missing object.
    pub fn point_command(&mut self, slot: usize, command: u16, point: [f32; 3]) -> bool {
        let Some(object) = self.slots.get_mut(&slot) else {
            return false;
        };
        match command {
            0x2330 => object.points = Some(Vec::new()),
            0x2331 => object.points = None,
            _ => {
                if let Some(points) = object.points.as_mut().filter(|points| points.len() < raster::POINT_LIMIT) {
                    points.push(point);
                }
            }
        }
        true
    }

    /// 0x14005fbd0: points of objects 20..=27 moved to their world positions, at most 256.
    pub fn collision_points(&self) -> Vec<[f32; 3]> {
        let mut points = Vec::new();
        for slot in 20..28 {
            let Some(object) = self.slots.get(&slot) else { continue };
            let (Some(list), Some(draw)) = (&object.points, &object.draw) else { continue };
            for point in list.iter().take(256 - points.len()) {
                points.push([point[0] + draw.world[0], point[1] + draw.world[1], point[2] + draw.world[2]]);
            }
        }
        points
    }

    /// Step-list world moves (0x14002a680, subs 1 and 2): a one-key world track to
    /// `target` over `duration` ms, or with `instant` the track's removal and the
    /// target applied at once.
    #[allow(clippy::too_many_arguments)]
    pub fn sequence_world(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        mode: u32,
        duration: u32,
        target: [f32; 3],
        instant: bool,
    ) -> Result<()> {
        let Some(object) = self.slots.get(&slot) else {
            return Ok(());
        };
        let mut changed = object.clone();
        let Some(record) = changed.draw.as_mut() else {
            return Ok(());
        };
        record.tracks.retain(|track| track.channel() != Channel::World);
        if instant {
            record.world = target;
        } else {
            let base = record.channel_values(Channel::World);
            let mut track = Track::new(Channel::World, base);
            let [x, y, z] = target.map(f32::to_bits);
            track.set_key(&[0, u32::MAX, mode, duration, x, y, z])?;
            track.start(0, base)?;
            record.tracks.push(track);
        }
        Self::reproject(record, &self.cameras);
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        self.slots.insert(slot, changed);
        Ok(())
    }

    pub fn set_source(&mut self, slot: usize, child: Option<usize>, name: String) {
        if let Some(object) = self.slots.get_mut(&slot) {
            match child {
                Some(index) => {
                    if let Some(child) = object.children.get_mut(&index) {
                        child.source = Some(name);
                    }
                }
                None => object.source = Some(name),
            }
        }
    }

    pub fn set_movie_source(&mut self, slot: usize, index: usize, name: String) {
        if let Some(object) = self.slots.get_mut(&slot) {
            object.movie_source = Some((index, name));
        }
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let slots: Vec<_> = self
            .slots
            .iter()
            .map(|(slot, object)| {
                let children: Vec<_> = object
                    .children
                    .iter()
                    .map(|(index, child)| {
                        json!({"index": index, "source": child.source, "draw": child.draw,
                            "inherits": child.inherits, "flags": child.flags})
                    })
                    .collect();
                json!({"slot": slot, "source": object.source, "draw": object.draw,
                    "flags": object.flags, "alpha": object.native_alpha,
                    "movie": object.movie_source, "size": object.movie.as_ref().map(|m| m.size),
                    "children": children, "raster": object.raster, "points": object.points,
                    "second": object.second.as_ref().map(|(name, _)| name)})
            })
            .collect();
        json!({"slots": slots, "quake": self.quake})
    }

    /// Native image names a saved scene needs, as (slot, child, name).
    pub fn saved_images(value: &serde_json::Value) -> Vec<(usize, Option<usize>, String)> {
        let mut names = Vec::new();
        for object in value["slots"].as_array().into_iter().flatten() {
            let Some(slot) = object["slot"].as_u64().map(|slot| slot as usize) else {
                continue;
            };
            if let Some(name) = object["source"].as_str() {
                names.push((slot, None, name.to_owned()));
            }
            for child in object["children"].as_array().into_iter().flatten() {
                if let (Some(index), Some(name)) = (child["index"].as_u64(), child["source"].as_str()) {
                    names.push((slot, Some(index as usize), name.to_owned()));
                }
            }
        }
        names
    }

    /// Second textures (`0x2035`) a saved scene needs, as (slot, name).
    pub fn saved_seconds(value: &serde_json::Value) -> Vec<(usize, String)> {
        value["slots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|object| Some((object["slot"].as_u64()? as usize, object["second"].as_str()?.to_owned())))
            .collect()
    }

    /// Scene movies a saved scene needs, as (object slot, movie slot, name, size, alpha).
    pub fn saved_movies(value: &serde_json::Value) -> Vec<(usize, usize, String, [u32; 2], bool)> {
        let mut movies = Vec::new();
        for object in value["slots"].as_array().into_iter().flatten() {
            let (Some(slot), Some(index), Some(name)) = (
                object["slot"].as_u64(),
                object["movie"][0].as_u64(),
                object["movie"][1].as_str(),
            ) else {
                continue;
            };
            let size = [
                object["size"][0].as_u64().unwrap_or(0) as u32,
                object["size"][1].as_u64().unwrap_or(0) as u32,
            ];
            let alpha = object["alpha"].as_bool().unwrap_or(false);
            movies.push((slot as usize, index as usize, name.to_owned(), size, alpha));
        }
        movies
    }

    /// Rebuilds every object from a save. Images were decoded by the load worker.
    pub fn restore(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        value: &serde_json::Value,
        images: Vec<(usize, Option<usize>, String, DecodedImage)>,
        cameras: [[u32; 40]; 2],
    ) -> Result<()> {
        self.reset_all(host, client)?;
        self.cameras = cameras;
        for saved in value["slots"].as_array().context("CMVS saved objects")? {
            let slot = saved["slot"].as_u64().context("CMVS saved object slot")? as usize;
            self.reset(host, client, slot, None, true)?;
            for child in saved["children"].as_array().into_iter().flatten() {
                let index = child["index"].as_u64().context("CMVS saved child")? as usize;
                self.reset(host, client, slot, Some(index), true)?;
            }
            let object = self.slots.get_mut(&slot).context("CMVS restored object")?;
            object.flags = serde_json::from_value(saved["flags"].clone())?;
            object.draw = serde_json::from_value(saved["draw"].clone())?;
            object.native_alpha = saved["alpha"].as_bool();
            // 0x14004f1c0 restarts a saved raster from phase 0.
            object.raster = serde_json::from_value::<Option<raster::Raster>>(saved["raster"].clone())
                .unwrap_or_default()
                .map(|mut raster| {
                    raster.phase = 0;
                    raster
                });
            object.points = serde_json::from_value(saved["points"].clone()).unwrap_or_default();
            if let Some(draw) = &mut object.draw {
                Self::reproject(draw, &cameras);
            }
            for child in saved["children"].as_array().into_iter().flatten() {
                let index = child["index"].as_u64().context("CMVS saved child")? as usize;
                let entry = object.children.get_mut(&index).context("CMVS restored child")?;
                entry.flags = serde_json::from_value(child["flags"].clone())?;
                entry.draw = serde_json::from_value(child["draw"].clone())?;
                entry.inherits = child["inherits"].as_bool().unwrap_or(false);
                if let Some(draw) = &mut entry.draw {
                    Self::reproject(draw, &cameras);
                }
            }
        }
        for (slot, child, name, image) in images {
            match child {
                Some(index) => self.child_image(host, client, slot, index, image)?,
                None => self.image(host, client, slot, image)?,
            }
            self.set_source(slot, child, name);
        }
        let slots: Vec<_> = self.slots.keys().copied().collect();
        for slot in slots {
            let object = self.slots.get_mut(&slot).context("CMVS restored object")?;
            update(host, client, slot, object, self.canvas, self.quake)?;
        }
        Ok(())
    }

    /// Reattaches a restored object to its running scene movie.
    pub fn resume_movie(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        index: usize,
        name: String,
        path: crysirror_core::path::VfsPath,
        size: [u32; 2],
        alpha: bool,
    ) -> Result<()> {
        let object = self.slots.get_mut(&slot).context("CMVS restored movie object")?;
        object.movie = Some(UploadedImage { path, size, opaque: !alpha });
        object.movie_source = Some((index, name));
        update(host, client, slot, object, self.canvas, self.quake)
    }

    /// `0x2308`: replaces the slot with an object showing a scene movie.
    pub fn movie(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        path: crysirror_core::path::VfsPath,
        size: [u32; 2],
        alpha: bool,
    ) -> Result<()> {
        self.reset(host, client, slot, None, true)?;
        let object = self.slots.get_mut(&slot).context("CMVS movie object missing")?;
        object.movie = Some(UploadedImage { path, size, opaque: !alpha });
        object.native_alpha = Some(alpha);
        object.draw = Some(Draw {
            crop: [0, 0, size[0] as i16, size[1] as i16],
            ..Draw::default()
        });
        update(host, client, slot, object, self.canvas, self.quake)
    }

    /// `0x2038`: replaces one child's image and keeps its draw record.
    pub fn child_image(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        index: usize,
        image: DecodedImage,
    ) -> Result<()> {
        let current = self.slots.get_mut(&slot).context("CMVS image object missing")?;
        ensure!(current.children.contains_key(&index), "CMVS image child missing");
        let published = host.publish_image_resource(client, child_id(slot, index), image.image)?;
        let mut changed = current.clone();
        let child = changed.children.get_mut(&index).context("CMVS image child missing")?;
        let previous = child.image.replace(published.clone());
        if let Err(error) = update(host, client, slot, &mut changed, self.canvas, self.quake) {
            host.release_image_resource(client, published.path.relative())?;
            return Err(error);
        }
        *current = changed;
        if let Some(previous) = previous {
            host.release_image_resource(client, previous.path.relative())?;
        }
        Ok(())
    }

    pub fn image_info(&self, slot: usize) -> Result<Option<([u32; 2], bool)>> {
        let object = self
            .slots
            .get(&slot)
            .context("CMVS image query object missing")?;
        object
            .image
            .as_ref()
            .or(object.movie.as_ref())
            .map(|image| {
                let alpha = object
                    .native_alpha
                    .context("CMVS native image depth unavailable")?;
                Ok((image.size, alpha))
            })
            .transpose()
    }

    pub fn image_size(&self, slot: usize) -> Option<[u32; 2]> {
        self.slots
            .get(&slot)?
            .image
            .as_ref()
            .map(|image| image.size)
    }

    pub fn mask(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        mask: Option<mask::Sample>,
    ) -> Result<()> {
        if let Some(object) = self.slots.get_mut(&slot) {
            if object.published {
                let filter = mask.as_ref().map(mask::Sample::filter);
                submit(
                    host,
                    client,
                    vec![json!({"op":"set_filter","id":layer_id(slot),"filter":filter})],
                )?;
            }
            object.mask = mask;
        }
        Ok(())
    }

    pub fn release_image(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
    ) -> Result<()> {
        let object = self
            .slots
            .get_mut(&slot)
            .context("CMVS draw object missing")?;
        let Some(image) = object.image.as_ref() else {
            return Ok(());
        };
        let mut candidate = object.clone();
        candidate.image = None;
        candidate.native_alpha = None;
        update(host, client, slot, &mut candidate, self.canvas, self.quake)?;
        host.release_image_resource(client, image.path.relative())?;
        *object = candidate;
        Ok(())
    }

    pub fn draw_info(&self, slot: usize, child: i32) -> Result<([u32; 4], [f32; 2])> {
        let object = self.slots.get(&slot).context("CMVS draw object missing")?;
        let draw = if child < 0 {
            object.draw.as_ref()
        } else {
            object
                .children
                .get(&(child as usize))
                .and_then(|child| child.draw.as_ref())
        }
        .context("CMVS object has no draw state")?;
        let [x, y] = draw.position;
        Ok((
            [draw.opacity as u32, x as u32, y as u32, draw.order as u32],
            draw.scale,
        ))
    }

    pub fn draw(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        property: DrawProperty,
    ) -> Result<()> {
        let current = self
            .slots
            .get_mut(&slot)
            .context("CMVS draw object missing")?;
        let mut changed = current.clone();
        let (draw, flags) = match child {
            None => (&mut changed.draw, &mut changed.flags),
            Some(index) => {
                let child = changed
                    .children
                    .get_mut(&index)
                    .context("CMVS draw child missing")?;
                (&mut child.draw, &mut child.flags)
            }
        };
        match property {
            DrawProperty::Reset => *draw = Some(Draw::default()),
            DrawProperty::Shown(visible) => flags.shown = visible,
            DrawProperty::LogicalSize(size) => flags.size = size,
            DrawProperty::ClearQuakeFollow => flags.follows_quake = false,
            property => {
                let draw = draw.as_mut().context("CMVS object has no draw state")?;
                match property {
                    DrawProperty::Crop(rect) => draw.crop = rect,
                    DrawProperty::Position(point) => draw.position = point,
                    DrawProperty::Origin(point) => draw.origin = point,
                    DrawProperty::Priority(priority) => draw.order = priority,
                    DrawProperty::Opacity(opacity) => draw.opacity = opacity.min(255),
                    DrawProperty::Scale(scale) => draw.scale = [scale; 2],
                    DrawProperty::Frame { crop, origin } => {
                        draw.crop = crop;
                        for (axis, value) in draw.origin.iter_mut().zip(origin) {
                            if value >= 0 {
                                *axis = value;
                            }
                        }
                    }
                    _ => unreachable!("handled draw lifecycle"),
                }
            }
        }
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        *current = changed;
        Ok(())
    }

    pub fn button_images(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
        images: &[ChildImage],
        show: bool,
    ) -> Result<()> {
        if images.is_empty() {
            return Ok(());
        }
        let current = self
            .slots
            .get_mut(&slot)
            .context("CMVS button group object missing")?;
        let mut changed = current.clone();
        for image in images {
            // A state that `0x2214` never registered has no child image to apply.
            if image.child == ChildImage::UNSET {
                continue;
            }
            let child = changed
                .children
                .get_mut(&(image.child as usize))
                .context("CMVS button child missing")?;
            let draw = child
                .draw
                .as_mut()
                .context("CMVS button child has no draw state")?;
            draw.crop = image.crop;
            draw.origin = image.origin;
            child.flags.shown |= show;
        }
        update(host, client, slot, &mut changed, self.canvas, self.quake)?;
        *current = changed;
        Ok(())
    }
}

fn visible_ids(slot: usize, object: &Object) -> impl Iterator<Item = u32> + '_ {
    object
        .published
        .then_some(layer_id(slot))
        .into_iter()
        .chain(
            object
                .children
                .iter()
                .filter_map(move |(&i, c)| c.published.then_some(child_id(slot, i))),
        )
        .chain(
            object
                .raster_published
                .iter()
                .enumerate()
                .filter_map(move |(row, &published)| published.then_some(raster_id(slot, row))),
        )
}

fn replace_resource(
    host: &mut dyn HostServices,
    client: ClientId,
    id: u32,
    image: Option<DecodedImage>,
    commands: Vec<serde_json::Value>,
) -> Result<Option<(UploadedImage, Option<bool>)>> {
    let published = image
        .map(|image| {
            let alpha = image.native_alpha;
            image.publish(host, client, id).map(|image| (image, alpha))
        })
        .transpose()?;
    if let Err(error) = submit(host, client, commands) {
        if let Some((image, _)) = published {
            host.release_image_resource(client, image.path.relative())?;
        }
        return Err(error);
    }
    Ok(published)
}

fn update(
    host: &mut dyn HostServices,
    client: ClientId,
    slot: usize,
    object: &mut Object,
    canvas: [u32; 2],
    quake: [[i32; 2]; 2],
) -> Result<()> {
    submit(host, client, plan(slot, object, canvas, quake)?)
}

fn plan(
    slot: usize,
    object: &mut Object,
    canvas: [u32; 2],
    quake: [[i32; 2]; 2],
) -> Result<Vec<serde_json::Value>> {
    let offset = if object.flags.follows_quake {
        quake[usize::from(slot >= 256)]
    } else {
        [0; 2]
    };
    let mut commands = Vec::new();
    let parent = object.draw.as_ref().filter(|_| object.flags.shown);
    let image = object.image.as_ref().or(object.movie.as_ref());
    let second = object.second.as_ref().map(|(_, matte)| matte);
    let faded = parent.map(|draw| {
        let mut draw = draw.clone();
        if let Some(fade) = &object.crossfade {
            draw.opacity = (i64::from(draw.opacity) * i64::from(fade.elapsed)
                / i64::from(fade.duration.max(1))) as i32;
        }
        draw
    });
    if let Some(mut fade) = object.crossfade.take() {
        let ghost = match parent {
            Some(draw) => put(
                crossfade_id(slot),
                slot,
                0,
                draw,
                Some(&fade.previous),
                None,
                object.flags.size,
                canvas,
                offset,
                draw.position.map(|value| value as f32),
            )?,
            None => None,
        };
        reconcile(&mut commands, &mut fade.published, crossfade_id(slot), ghost);
        object.crossfade = Some(fade);
    }
    let mut own = match (faded.as_ref(), image) {
        _ if object.children_only || object.raster.is_some() => None,
        (Some(draw), image) if image.is_some() || draw.fill.is_some() => put(
            layer_id(slot),
            slot,
            1,
            draw,
            image,
            second,
            object.flags.size,
            canvas,
            offset,
            draw.position.map(|value| value as f32),
        )?,
        _ => None,
    };
    if let (Some(layer), Some(mask)) = (&mut own, &object.mask) {
        let mut filter = mask.filter();
        if let (Some(own), Some(merged)) =
            (layer["layer"]["filter"].as_object().cloned(), filter.as_object_mut())
        {
            merged.extend(own);
        }
        layer["layer"]["filter"] = filter;
    }
    reconcile(&mut commands, &mut object.published, layer_id(slot), own);
    let rows = match (&object.raster, faded.as_ref()) {
        (Some(raster), Some(draw)) if image.is_some() => raster.rows(raster::Parent {
            origin: draw.origin,
            position: draw.position,
            opacity: draw.opacity,
            order: draw.order,
        }),
        _ => Vec::new(),
    };
    let published = std::mem::take(&mut object.raster_published);
    let mut states = vec![false; rows.len().max(published.len())];
    states[..published.len()].copy_from_slice(&published);
    for (index, state) in states.iter_mut().enumerate() {
        let new = match (rows.get(index), faded.as_ref()) {
            (Some(row), Some(draw)) => {
                let mut draw = draw.clone();
                draw.crop = row.crop;
                draw.origin = row.origin;
                draw.position = row.position;
                draw.scale = row.scale;
                draw.opacity = row.opacity;
                draw.order = row.order;
                let tie = 1 + (u64::from((!draw.order) as u32) << 10) + index as u64;
                let position = draw.position.map(|value| value as f32);
                put(raster_id(slot, index), slot, tie, &draw, image, second, object.flags.size, canvas, offset, position)?
            }
            _ => None,
        };
        reconcile(&mut commands, state, raster_id(slot, index), new);
    }
    while states.last() == Some(&false) {
        states.pop();
    }
    object.raster_published = states;
    for (&index, child) in &mut object.children {
        let id = child_id(slot, index);
        let child_image = child.image.as_ref().or_else(|| parent.and(image));
        let new = match (child_image, child.draw.as_ref()) {
            (image, Some(draw))
                if (image.is_some() || draw.fill.is_some())
                    && object.flags.shown
                    && child.flags.shown
                    && draw.order >= 0 =>
            {
                let mut draw = draw.clone();
                let mut position = draw.position.map(|value| value as f32);
                let tie = 1 + (u64::from((!draw.order) as u32) << 10) + index as u64;
                let mut inherited_matte = None;
                if let Some(parent) = parent.filter(|_| child.inherits) {
                    for (axis, value) in position.iter_mut().enumerate() {
                        *value = *value * parent.scale[0] + parent.position[axis] as f32;
                        draw.scale[axis] *= parent.scale[axis];
                    }
                    draw.opacity = (draw.opacity.max(0) * parent.opacity.max(0)) / 255;
                    draw.order = parent.order;
                    // Native child drawing inherits the parent's second texture
                    // only with the parent's mode 4.
                    if parent.mode == 4 {
                        draw.mode = parent.mode;
                        inherited_matte = second;
                    }
                }
                let size = if child.flags.size == [0; 2] {
                    object.flags.size
                } else {
                    child.flags.size
                };
                put(id, slot, tie, &draw, image, inherited_matte, size, canvas, offset, position)?
            }
            _ => None,
        };
        reconcile(&mut commands, &mut child.published, id, new);
    }
    Ok(commands)
}

#[allow(clippy::too_many_arguments)]
fn put(
    id: u32,
    slot: usize,
    tie: u64,
    draw: &Draw,
    image: Option<&UploadedImage>,
    second: Option<&UploadedImage>,
    logical_size: [u32; 2],
    canvas: [u32; 2],
    offset: [i32; 2],
    position: [f32; 2],
) -> Result<Option<serde_json::Value>> {
    let [x, y, w, h] = draw.crop.map(i32::from);
    if draw.kind < 2 && second.is_some() && draw.mode != 4 {
        return Ok(None);
    }
    let (position, scale_xy, opacity, rotation) = match (draw.kind >= 2, draw.projected) {
        (true, None) => return Ok(None),
        (true, Some(view)) => (view.position, view.scale, view.opacity, view.roll),
        (false, _) => (position, draw.scale, draw.opacity, draw.rotation),
    };
    if w <= 0 || h <= 0 || opacity <= 0 || scale_xy.contains(&0.0) {
        return Ok(None);
    }
    let [px, py] = position;
    let [ox, oy] = draw.origin.map(f32::from);
    let [sx, sy] = scale_xy;
    let scale = |value: f32, axis: usize| {
        f64::from(value)
            * if logical_size[axis] == 0 {
                1.
            } else {
                f64::from(canvas[axis]) / f64::from(logical_size[axis])
            }
    };
    let mut rect = [
        scale(px - sx * ox, 0) + f64::from(offset[0]),
        scale(py - sy * oy, 1) + f64::from(offset[1]),
        scale(w as f32, 0),
        scale(h as f32, 1),
    ];
    let radians = (f64::from(rotation) / 100.).to_radians();
    let pivot = [scale(ox, 0), scale(oy, 1)];
    // A UV flip mirrors the quad about its centre: negate the scale about the pivot and
    // move by R·S·(size − 2·pivot) on the flipped axis.
    let mut signed = [f64::from(sx), f64::from(sy)];
    for (axis, bit) in [(0, 0x10), (1, 0x20)] {
        if draw.flip & bit != 0 {
            let along = signed[axis] * (rect[2 + axis] - 2. * pivot[axis]);
            let (sin, cos) = radians.sin_cos();
            let local = if axis == 0 { [along, 0.] } else { [0., along] };
            rect[0] += cos * local[0] - sin * local[1];
            rect[1] += sin * local[0] + cos * local[1];
            signed[axis] = -signed[axis];
        }
    }
    let transform = json!({"scale_x":signed[0],"scale_y":signed[1],"rotate":radians,
        "pivot_x":pivot[0],"pivot_y":pivot[1]});
    let mut filter = serde_json::Map::new();
    if draw.invert && (draw.kind >= 2 || second.is_none()) {
        filter.insert("invert".into(), json!(1.0));
    }
    if let Some(matte) = second {
        filter.insert(
            "stencil".into(),
            json!({"asset":matte.path.relative(),"canvas":matte.size,"origin":[0.0, 0.0]}),
        );
    }
    let Some(image) = image else {
        let color = draw.fill.context("CMVS draw record has no image or fill")?;
        let mut layer = json!({"op":"put","layer":{
            "id":id,"rect":rect,"transform":transform,
            "order":[!draw.order,slot as i32],"tie_order":tie,"opacity":opacity,
            "paint":{"kind":"solid","color":[(color >> 16) as u8,(color >> 8) as u8,color as u8,255]}
        }});
        if !filter.is_empty() {
            layer["layer"]["filter"] = serde_json::Value::Object(filter);
        }
        return Ok(Some(layer));
    };
    let [width, height] = image.size.map(|axis| axis as i32);
    ensure!(
        x >= 0 && y >= 0 && x + w <= width && y + h <= height,
        "CMVS image crop outside source"
    );
    let uv = [
        x as f64 / f64::from(width),
        y as f64 / f64::from(height),
        (x + w) as f64 / f64::from(width),
        (y + h) as f64 / f64::from(height),
    ];
    let mut layer = json!({"op":"put","layer":{
        "id":id,"rect":rect,"transform":transform,
        "order":[!draw.order,slot as i32],"tie_order":tie,"uv":uv,"opacity":opacity,
        "paint":{"kind":"image","asset":image.path.relative()}
    }});
    if !filter.is_empty() {
        layer["layer"]["filter"] = serde_json::Value::Object(filter);
    }
    Ok(Some(layer))
}

fn reconcile(
    commands: &mut Vec<serde_json::Value>,
    published: &mut bool,
    id: u32,
    command: Option<serde_json::Value>,
) {
    let was_published = std::mem::replace(published, command.is_some());
    if let Some(command) = command {
        commands.push(command);
    } else if was_published {
        commands.push(remove(id));
    }
}

fn remove(id: u32) -> serde_json::Value {
    json!({"op":"remove","id":id})
}

fn submit(
    host: &mut dyn HostServices,
    client: ClientId,
    commands: Vec<serde_json::Value>,
) -> Result<()> {
    if !commands.is_empty() {
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":commands}))?)?;
    }
    Ok(())
}

fn layer_id(slot: usize) -> u32 {
    if slot < 256 {
        LAYER_BASE + slot as u32
    } else {
        300_000 + (slot - 256) as u32
    }
}

fn crossfade_id(slot: usize) -> u32 {
    390_000 + slot as u32
}

fn raster_id(slot: usize, row: usize) -> u32 {
    40_000_000 + (slot * 16_384 + row.min(16_383)) as u32
}

fn child_id(slot: usize, child: usize) -> u32 {
    if slot < 256 {
        LAYER_BASE + 256 + (slot * 768 + child) as u32
    } else {
        // Upper-bank and effect objects can own thousands of children (2048 type 4 streaks).
        2_000_000 + ((slot - 256) * 65_536 + child.min(65_535)) as u32
    }
}

#[derive(Clone)]
pub(super) struct DecodedImage {
    image: Arc<dyn ImageResource>,
    native_alpha: Option<bool>,
}

impl DecodedImage {
    /// Wraps a captured frame as an opaque object image.
    pub fn from_frame(image: Arc<dyn ImageResource>) -> Self {
        Self {
            image,
            native_alpha: Some(false),
        }
    }

    pub fn from_surface(
        client: ClientId,
        image: crysirror_image::ImageSurface,
        native_alpha: Option<bool>,
    ) -> Result<Self> {
        Ok(Self {
            image: Arc::new(RgbaImage::new(
                client,
                [image.width, image.height],
                image.rgba.into(),
            )?),
            native_alpha,
        })
    }

    pub fn rgba(&self) -> Arc<[u8]> {
        self.image.rgba()
    }

    /// Whether the decoded picture keeps an alpha channel of its own.
    pub fn has_alpha(&self) -> bool {
        self.native_alpha.unwrap_or(true)
    }

    pub fn size(&self) -> [u32; 2] {
        self.image.size()
    }

    pub fn publish(
        self,
        host: &mut dyn HostServices,
        client: ClientId,
        id: u32,
    ) -> Result<UploadedImage> {
        Ok(host.publish_image_resource(client, id, self.image)?)
    }

    /// The green channel stretched over `size`: the second-texture upload copies
    /// green into alpha, and draw mode 4 samples it across the whole screen.
    pub fn screen_matte(&self, size: [u32; 2]) -> Result<Self> {
        let [width, height] = self.image.size();
        let source = crysirror_image::ImageSurface {
            width,
            height,
            rgba: self.image.rgba().to_vec(),
        };
        let matte = crysirror_image::channel_matte::stretched_channel(&source, 1, size)?;
        Self::from_surface(self.image.client(), matte, Some(false))
    }

    pub fn invert_rgb(mut self) -> Result<Self> {
        let [width, height] = self.image.size();
        let image = crysirror_image::ImageSurface {
            width,
            height,
            rgba: self.image.rgba().to_vec(),
        };
        let image = image.apply_effect(&crysirror_image::effects::ImageEffect::Invert)?;
        self.image = Arc::new(RgbaImage::new(
            self.image.client(),
            [width, height],
            image.rgba.into(),
        )?);
        Ok(self)
    }
}

pub(super) fn prepare_image(
    sources: &ScriptSources,
    name: &str,
    context: &DecodeContext,
) -> Result<DecodedImage> {
    prepare_image_with(sources, name, None, context)
}

/// Decodes `name`, using bytes `0x2338` already read when `0x203f` armed the cache.
pub(super) fn prepare_image_with(
    sources: &ScriptSources,
    name: &str,
    prefetched: Option<Arc<[u8]>>,
    context: &DecodeContext,
) -> Result<DecodedImage> {
    let bytes = match prefetched {
        Some(bytes) => bytes.to_vec(),
        None => {
            let input = sources
                .open_entry(name)?
                .with_context(|| format!("CMVS image missing: {name}"))?;
            read_all(input.as_ref(), IMAGE_LIMIT)?
        }
    };
    let native_alpha = bytes
        .starts_with(b"PB3B")
        .then(|| crysirror_formats_arc::images_a::pb3_bit_depth(&bytes).map(|bits| bits == 32))
        .transpose()?;
    let mut decoders = DecoderRegistry::default();
    crysirror_formats_arc::register_images(&mut decoders)?;
    let dependency = |path: &str| {
        let result = sources
            .open_entry(path)
            .and_then(|input| {
                input
                    .map(|input| read_all(input.as_ref(), IMAGE_LIMIT))
                    .transpose()
            })
            .map_err(|error| Error::Io(format!("{error:#}")))?;
        result.ok_or(Error::NotFound)
    };
    let image = decoders.decode_with(name, &bytes, Some(&dependency), Some(context), None)?;
    DecodedImage::from_surface(context.client, image, native_alpha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{
        path::{Mount, VfsPath},
        service::FileSystem,
        state::Command,
    };
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Default)]
    struct TestHost {
        generation: u32,
        images: BTreeSet<String>,
        layers: BTreeMap<u32, String>,
        reject_batch: bool,
    }

    impl FileSystem for TestHost {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::NotFound)
        }

        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
    }

    impl HostServices for TestHost {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            unreachable!()
        }

        fn now_ms(&self) -> u64 {
            0
        }

        fn submit_engine(
            &mut self,
            _: ClientId,
            schema: u32,
            bytes: &[u8],
        ) -> crysirror_core::Result<()> {
            assert_eq!(schema, 1);
            if self.reject_batch {
                return Err(Error::Invalid("test rejects layer batch"));
            }
            let batch: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            for command in batch["layers"].as_array().unwrap() {
                if command["op"] == "remove" {
                    self.layers
                        .remove(&(command["id"].as_u64().unwrap() as u32));
                } else {
                    let layer = &command["layer"];
                    let asset = layer["paint"]["asset"].as_str().unwrap();
                    assert!(self.images.contains(asset));
                    self.layers
                        .insert(layer["id"].as_u64().unwrap() as u32, asset.to_owned());
                }
            }
            Ok(())
        }

        fn publish_image_resource(
            &mut self,
            client: ClientId,
            id: u32,
            image: Arc<dyn ImageResource>,
        ) -> crysirror_core::Result<UploadedImage> {
            assert_eq!(image.client(), client);
            self.generation += 1;
            let name = format!("generated/{id}-{}", self.generation);
            assert!(self.images.insert(name.clone()));
            Ok(UploadedImage {
                path: VfsPath::new(Mount::Asset, &name).unwrap(),
                size: image.size(),
                opaque: true,
            })
        }

        fn release_image_resource(
            &mut self,
            _: ClientId,
            name: &str,
        ) -> crysirror_core::Result<()> {
            assert!(self.layers.values().all(|asset| asset != name));
            assert!(self.images.remove(name));
            Ok(())
        }
    }

    fn image(client: ClientId, width: u32) -> DecodedImage {
        let bytes = [0, 0, 0, 255].repeat(width as usize);
        DecodedImage {
            image: Arc::new(RgbaImage::new(client, [width, 1], bytes.into()).unwrap()),
            native_alpha: Some(false),
        }
    }

    fn draw(
        objects: &mut Objects,
        host: &mut TestHost,
        client: ClientId,
        slot: usize,
        child: Option<usize>,
        width: i16,
    ) {
        objects
            .draw(host, client, slot, child, DrawProperty::Reset)
            .unwrap();
        objects
            .draw(
                host,
                client,
                slot,
                child,
                DrawProperty::Crop([0, 0, width, 1]),
            )
            .unwrap();
    }

    #[test]
    fn second_texture_is_used_by_mode_four_and_inheriting_children() {
        let client = ClientId(9);
        let mut host = TestHost::default();
        let mut objects = Objects::new([16, 16]);
        objects
            .replace_preview(&mut host, client, 11, None, Some(image(client, 1)))
            .unwrap();
        draw(&mut objects, &mut host, client, 11, None, 1);
        objects.reset(&mut host, client, 11, Some(4), true).unwrap();
        draw(&mut objects, &mut host, client, 11, Some(4), 1);
        objects
            .set_second(&mut host, client, 11, "matte".into(), Some(image(client, 1)))
            .unwrap();
        assert!(!host.layers.contains_key(&layer_id(11)));
        assert!(host.layers.contains_key(&child_id(11, 4)));

        let object = objects.slots.get_mut(&11).unwrap();
        object.draw.as_mut().unwrap().mode = 4;
        let batch = plan(11, object, [16, 16], [[0; 2]; 2]).unwrap();
        let matte = object.second.as_ref().unwrap().1.path.relative();
        assert_eq!(batch.len(), 2);
        for command in batch {
            assert_eq!(command["layer"]["filter"]["stencil"]["asset"], matte);
        }
    }

    #[test]
    fn independent_child_image_survives_parent_image_release_and_move() {
        let client = ClientId(9);
        let mut host = TestHost::default();
        let mut objects = Objects::new([16, 16]);
        objects
            .replace_preview(&mut host, client, 1, None, Some(image(client, 1)))
            .unwrap();
        draw(&mut objects, &mut host, client, 1, None, 1);
        objects
            .replace_preview(&mut host, client, 1, Some(7), Some(image(client, 2)))
            .unwrap();
        draw(&mut objects, &mut host, client, 1, Some(7), 2);
        let root = host.layers[&layer_id(1)].clone();
        let child = host.layers[&child_id(1, 7)].clone();
        assert_ne!(root, child);

        let mut scope = Default::default();
        objects.capture(&mut scope);
        assert_eq!(scope.generated_images, [root, child.clone()]);
        objects.release_image(&mut host, client, 1).unwrap();
        assert_eq!(
            host.layers,
            BTreeMap::from([(child_id(1, 7), child.clone())])
        );
        assert_eq!(host.images, BTreeSet::from([child.clone()]));

        objects
            .replace_preview(&mut host, client, 3, None, Some(image(client, 1)))
            .unwrap();
        objects
            .replace_preview(&mut host, client, 3, Some(0), Some(image(client, 1)))
            .unwrap();
        objects.move_to(&mut host, client, 3, 1).unwrap();
        assert_eq!(
            host.layers,
            BTreeMap::from([(child_id(3, 7), child.clone())])
        );
        assert_eq!(host.images, BTreeSet::from([child]));
        objects.clear(&mut host, client).unwrap();
        assert!(host.images.is_empty() && host.layers.is_empty());
    }

    #[test]
    fn preview_replacement_retires_child_only_after_layer_batch_succeeds() {
        let client = ClientId(9);
        let mut host = TestHost::default();
        let mut objects = Objects::new([16, 16]);
        objects
            .replace_preview(&mut host, client, 1, None, Some(image(client, 1)))
            .unwrap();
        objects
            .replace_preview(&mut host, client, 1, Some(7), Some(image(client, 2)))
            .unwrap();
        draw(&mut objects, &mut host, client, 1, Some(7), 2);
        let layer_id = child_id(1, 7);
        let asset = host.layers[&layer_id].clone();
        host.reject_batch = true;
        assert!(objects
            .replace_preview(&mut host, client, 1, Some(7), Some(image(client, 3)))
            .is_err());
        assert_eq!(host.layers[&layer_id], asset);
        assert_eq!(host.images.len(), 2);
        host.reject_batch = false;
        objects
            .replace_preview(&mut host, client, 1, Some(7), None)
            .unwrap();
        assert!(!host.layers.contains_key(&layer_id) && !host.images.contains(&asset));
        assert!(objects.has_child(1, 7));
        assert!(objects.draw_info(1, 7).is_err());
        assert_eq!(objects.image_size(1), Some([1, 1]));
        objects
            .replace_preview(&mut host, client, 1, None, None)
            .unwrap();
        assert!(!objects.has_child(1, 7));
        assert!(host.images.is_empty() && host.layers.is_empty());
    }
}
