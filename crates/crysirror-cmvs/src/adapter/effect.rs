// SPDX-License-Identifier: MPL-2.0
//! Effect slots `0x2290..=0x22af` (engine +4032, four 0xa0-byte records). Object
//! effects own a scene object whose children they pose each frame; the shared Layer
//! service draws them like any other object.

use super::visual::{DecodedImage, Objects, Pose};
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};

/// Internal object slots of the four effect records.
const OBJECT_BASE: usize = 512;

pub(super) fn object_slot(index: usize) -> usize {
    OBJECT_BASE + index
}

/// The MSVC CRT generator behind the engine's `rand()`; startup seeds it from the clock.
#[derive(Clone)]
pub(super) struct Rand(u32);

impl Rand {
    pub fn seeded(seed: u32) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((self.0 >> 16) & 0x7fff) as i32
    }

    /// `(float)rand() / 32767.0`.
    fn unit(&mut self) -> f32 {
        self.next() as f32 / 32767.0
    }
}

/// Values truncated to hundredths, as `(float)(int)(v * 100) / 100` in the original.
fn centi(value: f32) -> f32 {
    (value * 100.0) as i32 as f32 / 100.0
}

/// Type 8 (`0x22a8`, 0x14001ab50): the adopted object breaks into falling tiles.
struct Shatter {
    tiles: Vec<Tile>,
    height: i32,
    start: u64,
    done: bool,
}

struct Tile {
    child: usize,
    delay: u32,
    center: [i32; 2],
    speed: f32,
    drift: f32,
    offset: f32,
    crop: [i16; 4],
    half: i16,
}

/// Type 5 (`0x22a5`, 0x14005d440): sprite-sheet particles drifting in world space.
struct Drift {
    particles: Vec<Particle>,
    /// Record +20: low nibble of byte 3 selects the respawn depth band, the top
    /// nibble the projected size.
    mode: u32,
    center: f32,
    depth: f32,
    previous: u64,
    start: u64,
}

struct Particle {
    child: usize,
    world: [f32; 3],
    angle: f32,
    speed: f32,
    column: i16,
    size: f32,
}

/// Type 6 (`0x22a6`, 0x14005f4e0): 32-pixel particles falling in world space onto
/// the point-list triangles of objects 20..=27 (`0x2330..=0x2332`).
struct Fall {
    particles: Vec<FallParticle>,
    landings: Vec<Landing>,
    mode: u32,
    depth: f32,
    size: f32,
    previous: u64,
    fade: Option<Fade>,
}

struct FallParticle {
    child: usize,
    world: [f32; 3],
    speed: f32,
}

/// Type 6 state per particle: record +40, the height it settled on, and its opacity.
#[derive(Clone, Copy)]
struct Landing {
    height: f32,
    opacity: i32,
}

impl Default for Landing {
    fn default() -> Self {
        Self { height: 0.0, opacity: 255 }
    }
}

/// Type 2 (`0x22a2`, 0x140060950): billboards spread over `area`, falling through
/// `height`, recentred on the camera when `follow` is set and faded by distance.
struct Field {
    particles: Vec<FieldParticle>,
    area: f32,
    height: f32,
    fade: f32,
    follow: bool,
    size: [u32; 2],
    previous: u64,
}

struct FieldParticle {
    child: usize,
    world: [f32; 3],
    speed: f32,
}

/// Types 1 (`0x22a1`, 0x14005e470) and 4 (`0x22a4`, 0x14005ed30): an emitter that
/// releases one 6×112 streak per `interval` ms (at most a second of backlog per
/// frame); each falls along `angle` until it drops below the floor.
struct Streaks {
    /// Type 4: 2048 streaks spread over wider bands, a quarter of the time base.
    large: bool,
    streaks: Vec<Option<Streak>>,
    mode: u32,
    angle: f32,
    speed: f32,
    interval: u32,
    previous: u64,
    backlog: u32,
    /// Children are numbered in creation order like 0x14004ea30's object ids.
    next_child: usize,
}

struct Streak {
    child: usize,
    world: [f32; 3],
    time: u64,
    rotation: i32,
}

/// Type 3 (`0x22a3`, 0x14005f360): 32-pixel particles looping through ten units of height.
struct Loop {
    particles: Vec<FallParticle>,
    lift: f32,
    previous: u64,
}

/// Type 7 (`0x1400604e0`) fades a type 6 field in or out: record byte 2 % 3 picks
/// none, in or out, byte 1 the delay and byte 0 the duration in 100 ms steps.
#[derive(Clone, Copy)]
struct Fade {
    mode: u32,
    delay: u64,
    duration: u64,
    start: u64,
    opacity: i32,
}

/// Type 9 (`0x22a9`, 0x14001b4e0): screen-space flakes in three sizes falling in
/// 10-bit fixed point and wrapping 16 pixels above the screen.
struct Flurry {
    flakes: Vec<Flake>,
    height: i32,
    previous: u64,
}

struct Flake {
    child: usize,
    position: [i32; 2],
    speed: u32,
    crop: [i16; 4],
    origin: i16,
    order: i32,
}

enum Kind {
    Shatter(Shatter),
    Drift(Drift),
    Fall(Fall),
    Field(Field),
    Streaks(Streaks),
    Loop(Loop),
    Flurry(Flurry),
}

/// What a save keeps of an effect record (0x1400525b0): its command, image and words.
/// Restoring starts it afresh (0x140052e80); type 8 records are not kept.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct Saved {
    pub opcode: u16,
    pub name: String,
    pub words: Vec<u32>,
}

#[derive(Default)]
pub(super) struct Effects {
    slots: [Option<Kind>; 4],
    saved: [Option<Saved>; 4],
}

impl Effects {
    /// `0x22a0` (0x140051350): releases a record and its object.
    pub fn release(&mut self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, index: usize) -> Result<()> {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = None;
            self.saved[index] = None;
            objects.remove_object(host, client, object_slot(index))?;
        }
        Ok(())
    }

    /// `0x22ae` (0x1400526c0): -1 once the effect has finished; types without an
    /// end report -1 at once.
    pub fn status(&self, index: usize) -> u32 {
        match self.slots.get(index).and_then(Option::as_ref) {
            Some(Kind::Shatter(shatter)) => if shatter.done { u32::MAX } else { 0 },
            _ => u32::MAX,
        }
    }

    /// `0x22a8`: tiles of `source`'s image, `size` pixels apart on a 20-pixel grid.
    #[allow(clippy::too_many_arguments)]
    pub fn shatter(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        source: usize,
        size: u32,
        now: u64,
    ) -> Result<bool> {
        let [width, height] = objects.object_image_size(source).unwrap_or([0; 2]);
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        if !objects.adopt(host, client, source, slot)? {
            return Ok(false);
        }
        let columns = width.div_ceil(20);
        let rows = height.div_ceil(20);
        let size = size as i32;
        let half = (size >> 1) as i16;
        let mut tiles = Vec::with_capacity((columns * rows) as usize);
        let mut child = objects.next_child(slot);
        let mut y = 0;
        for row in 0..rows {
            let mut x = 0;
            for _ in 0..columns {
                // 0x14001ab50 draws the angle but does not use it afterwards.
                let _angle = rand.next();
                let delay = (rand.next() % 60) as u32 + 70 * (rows - row);
                let speed = (rand.next() % 100) as f32 / 100.0 - 0.5 + 4.0;
                let drift = (rand.next() % 100) as f32 / 50.0 - 1.0;
                tiles.push(Tile {
                    child,
                    delay,
                    center: [x + i32::from(half), y + i32::from(half)],
                    speed,
                    drift,
                    offset: 0.0,
                    crop: [x as i16, y as i16, size as i16, size as i16],
                    half,
                });
                child += 1;
                x += size;
            }
            y += size;
        }
        let mut shatter = Shatter { tiles, height: height as i32, start: now, done: false };
        shatter.pose(host, client, objects, slot, now)?;
        self.slots[index] = Some(Kind::Shatter(shatter));
        Ok(true)
    }

    /// `0x22a5`: `count` particles from `image`, see [`Drift`].
    #[allow(clippy::too_many_arguments)]
    pub fn drift(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        image: Option<DecodedImage>,
        [mode, center, depth, direction, speed, count]: [u32; 6],
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        objects.create_effect_object(host, client, slot, image)?;
        let [center, depth, direction, speed] = [center, depth, direction, speed].map(f32::from_bits);
        let band = depth_band(mode);
        let size = projected_size(mode, depth);
        let mut particles = Vec::with_capacity(count as usize);
        for child in 0..count as usize {
            // The first pair is drawn and overwritten by 0x14005d440.
            let _ = (rand.unit(), rand.unit());
            let x = rand.unit() * 120.0 + center - 60.0;
            let z = rand.unit() * band + depth;
            let y = rand.unit() * 100.0;
            let mut angle = (rand.next() % 100) as f32 / 10.0 - 5.0 + direction + 270.0;
            if angle >= 360.0 {
                angle -= 360.0;
            }
            let spread = speed / 5.0;
            let speed = rand.unit() * spread - spread * 0.5 + speed;
            let column = (40 * (rand.next() % 3)) as i16;
            particles.push(Particle {
                child,
                world: [centi(x), centi(y), centi(z)],
                angle,
                speed,
                column,
                size,
            });
        }
        let drift = Drift { particles, mode, center, depth, previous: now, start: now };
        drift.pose(host, client, objects, slot, now)?;
        self.slots[index] = Some(Kind::Drift(drift));
        Ok(())
    }

    /// `0x22a6`, and `0x22a7` with `faded` (0x1400600c0): `count` particles of
    /// `image`, see [`Fall`] and [`Fade`].
    #[allow(clippy::too_many_arguments)]
    pub fn fall(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        image: Option<DecodedImage>,
        [mode, depth, speed, count]: [u32; 4],
        faded: bool,
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        objects.create_effect_object(host, client, slot, image)?;
        let [depth, speed] = [depth, speed].map(f32::from_bits);
        let band = depth_band(mode);
        let mut particles = Vec::with_capacity(count as usize);
        for child in 0..count as usize {
            let x = rand.unit() * 90.0 - 45.0;
            let z = rand.unit() * band + depth;
            let y = rand.unit() * 75.0;
            rand.next();
            let spread = speed * 0.25;
            let speed = rand.unit() * spread - spread * 0.5 + speed;
            particles.push(FallParticle { child, world: [centi(x), centi(y), centi(z)], speed });
        }
        let fade = faded.then(|| Fade::new(mode, now));
        let landings = vec![Landing::default(); particles.len()];
        let fall = Fall { particles, landings, mode, depth, size: projected_size(mode, depth), previous: now, fade };
        fall.pose(host, client, objects, slot)?;
        self.slots[index] = Some(Kind::Fall(fall));
        Ok(())
    }

    /// `0x22af` (0x1400515f0): key 1 of a type 6 field replaces its mode, which
    /// changes the depth band later respawns use.
    pub fn set(&mut self, index: usize, key: u32, value: u32) {
        if let (Some(Some(Kind::Fall(fall))), 1) = (self.slots.get_mut(index), key) {
            if fall.fade.is_none() {
                fall.mode = value;
                if let Some(saved) = self.saved[index].as_mut() {
                    saved.words[0] = value;
                }
            }
        }
    }

    pub fn remember(&mut self, index: usize, saved: Saved) {
        if let Some(slot) = self.saved.get_mut(index) {
            *slot = Some(saved);
        }
    }

    pub fn saved(&self) -> [Option<Saved>; 4] {
        self.saved.clone()
    }

    /// `0x22a1` and `0x22a4`: a streak emitter, see [`Streaks`]. The words after the
    /// mode are two unused floats, the angle, the speed and the interval.
    #[allow(clippy::too_many_arguments)]
    pub fn streaks(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        index: usize,
        image: Option<DecodedImage>,
        large: bool,
        [mode, _, _, angle, speed, interval]: [u32; 6],
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        objects.create_effect_object(host, client, object_slot(index), image)?;
        let capacity = if large { 2048 } else { 1024 };
        let mut streaks = Vec::with_capacity(capacity);
        streaks.resize_with(capacity, || None);
        self.slots[index] = Some(Kind::Streaks(Streaks {
            large,
            streaks,
            mode,
            angle: f32::from_bits(angle),
            speed: f32::from_bits(speed),
            // A zero interval never leaves the original's spawn loop; one millisecond
            // is the shortest period that still returns.
            interval: interval.max(1),
            previous: now,
            backlog: 0,
            next_child: 0,
        }));
        Ok(())
    }

    /// `0x22a3` (0x14005ef60): `count` particles around `depth`, see [`Loop`].
    #[allow(clippy::too_many_arguments)]
    pub fn looped(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        image: Option<DecodedImage>,
        [mode, depth, speed, count]: [u32; 4],
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        objects.create_effect_object(host, client, slot, image)?;
        let [depth, speed] = [depth, speed].map(f32::from_bits);
        let particles = (0..count as usize)
            .map(|child| {
                let x = rand.unit() * 10.0 - 5.0;
                let z = rand.unit() * 10.0 + depth;
                let y = rand.unit() * 10.0;
                let spread = speed / 5.0;
                let speed = rand.unit() * spread - spread * 0.5 + speed;
                FallParticle { child, world: [x, y, z], speed }
            })
            .collect();
        let looped = Loop { particles, lift: 3.0 + (mode & 3) as f32, previous: now };
        looped.pose(host, client, objects, slot)?;
        self.slots[index] = Some(Kind::Loop(looped));
        Ok(())
    }

    /// `0x22a9` (0x14001b1b0): `count` flakes over the screen, see [`Flurry`].
    #[allow(clippy::too_many_arguments)]
    pub fn flurry(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        image: Option<DecodedImage>,
        [_, speed, order, count]: [u32; 4],
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        objects.create_effect_object(host, client, slot, image)?;
        let [width, height] = objects.canvas().map(|axis| axis.max(1));
        let flakes = (0..count as usize)
            .map(|child| {
                let x = rand.next() as u32 % width;
                let y = rand.next() as u32 % height;
                let size = rand.next() % 3;
                // rand() % 0 faults in the original when speed / 5 is below 16.
                let spread = (speed / 5) >> 4;
                let jitter = rand.next() as u32;
                let mut speed = speed.wrapping_add(if spread == 0 { 0 } else { (jitter % spread) << 7 });
                let (crop, origin) = match size {
                    0 => {
                        speed = speed.wrapping_mul(3) >> 1;
                        ([0, 0, 24, 24], 12)
                    }
                    1 => ([24, 0, 16, 16], 8),
                    _ => {
                        speed = 2 * speed / 3;
                        ([48, 0, 8, 8], 4)
                    }
                };
                Flake {
                    child,
                    position: [(x << 10) as i32, (y << 10) as i32],
                    speed,
                    crop,
                    origin,
                    order: order.wrapping_add(2 * size as u32) as i32,
                }
            })
            .collect();
        let flurry = Flurry { flakes, height: height as i32, previous: now };
        flurry.pose(host, client, objects, slot)?;
        self.slots[index] = Some(Kind::Flurry(flurry));
        Ok(())
    }

    /// `0x22a2`: `count` billboards of `image`, see [`Field`].
    #[allow(clippy::too_many_arguments)]
    pub fn field(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        index: usize,
        image: Option<DecodedImage>,
        [area, height, speed, fade, count, follow]: [u32; 6],
        cameras: &[[u32; 40]; 2],
        now: u64,
    ) -> Result<()> {
        self.release(host, client, objects, index)?;
        let slot = object_slot(index);
        let size = image.as_ref().map_or([0; 2], DecodedImage::size);
        objects.create_effect_object(host, client, slot, image)?;
        let [area, height, speed, fade] = [area, height, speed, fade].map(f32::from_bits);
        let mut particles = Vec::with_capacity(count as usize);
        for child in 0..count as usize {
            let half = area * 0.5;
            let x = rand.unit() * area - half;
            let z = rand.unit() * area - half;
            let y = rand.unit() * height;
            let spread = speed / 5.0;
            let speed = rand.unit() * spread - spread * 0.5 + speed;
            particles.push(FieldParticle { child, world: [x, y, z], speed });
        }
        let field = Field { particles, area, height, fade, follow: follow != 0, size, previous: now };
        field.pose(host, client, objects, slot, cameras)?;
        self.slots[index] = Some(Kind::Field(field));
        Ok(())
    }

    /// One frame of every active effect (0x140052b10).
    pub fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        cameras: &[[u32; 40]; 2],
        ground: &[[f32; 3]],
        now: u64,
    ) -> Result<()> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let object = object_slot(index);
            match slot {
                Some(Kind::Shatter(shatter)) => shatter.pose(host, client, objects, object, now)?,
                Some(Kind::Drift(drift)) => {
                    drift.step(rand, now);
                    drift.pose(host, client, objects, object, now)?;
                }
                Some(Kind::Fall(fall)) => {
                    fall.step(rand, ground, now);
                    fall.pose(host, client, objects, object)?;
                }
                Some(Kind::Field(field)) => {
                    field.step(cameras, now);
                    field.pose(host, client, objects, object, cameras)?;
                }
                Some(Kind::Streaks(streaks)) => streaks.advance(host, client, objects, rand, object, now)?,
                Some(Kind::Loop(looped)) => {
                    looped.step(now);
                    looped.pose(host, client, objects, object)?;
                }
                Some(Kind::Flurry(flurry)) => {
                    flurry.step(now);
                    flurry.pose(host, client, objects, object)?;
                }
                None => {}
            }
        }
        Ok(())
    }

    pub fn clear(&mut self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects) -> Result<()> {
        for index in 0..self.slots.len() {
            self.release(host, client, objects, index)?;
        }
        Ok(())
    }
}

/// Record +60 of type 5 and 6 particles: the top nibble of the mode, else depth + 5.
fn projected_size(mode: u32, depth: f32) -> f32 {
    match mode >> 28 {
        4 => 60.0,
        5 => 50.0,
        6 => 40.0,
        7 => 30.0,
        8 => 20.0,
        _ => depth + 5.0,
    }
}

/// Depth band of type 5 respawns, from byte 3 of record +20.
fn depth_band(mode: u32) -> f32 {
    match (mode >> 24) & 0xf {
        1 => 15.0,
        2 => 10.0,
        3 => 5.0,
        4 => 2.5,
        5 => 1.0,
        _ => 20.0,
    }
}

impl Shatter {
    /// 0x14001afd0: after its delay a tile falls at its own speed, drifts sideways,
    /// turns 360° per 10 s and shrinks to nothing over 7.5 s.
    fn pose(&mut self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize, now: u64) -> Result<()> {
        let elapsed = now.saturating_sub(self.start) as u32;
        let mut active = false;
        let mut poses = Vec::with_capacity(self.tiles.len());
        for tile in &mut self.tiles {
            let mut pose = Pose {
                crop: tile.crop,
                origin: [tile.half; 2],
                position: tile.center,
                order: 16,
                ..Pose::default()
            };
            if elapsed < tile.delay {
                active = true;
            } else {
                let falling = elapsed - tile.delay;
                tile.offset += tile.drift;
                let y = tile.center[1] + (falling as f32 / tile.speed) as i32;
                pose.position = [tile.center[0] + tile.offset as i32, y];
                if y <= self.height + 32 {
                    active = true;
                }
                pose.rotation = 100 * ((falling * 360 / 10_000) % 360) as i32;
                let scale = 1.0 - (falling as f32 / 7500.0).clamp(0.0, 1.0);
                pose.scale = [scale; 2];
            }
            poses.push((tile.child, pose));
        }
        if !active {
            self.done = true;
        }
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Drift {
    /// 0x14005dd10: particles move along their angle at most 500 ms per step and
    /// respawn at the top of a new band once they fall below zero.
    fn step(&mut self, rand: &mut Rand, now: u64) {
        let band = depth_band(self.mode);
        let elapsed = now.saturating_sub(self.previous).min(500) as f32 * 10.0;
        for particle in &mut self.particles {
            let distance = elapsed / particle.speed;
            let radians = if particle.angle == 0.0 { 0.0 } else { particle.angle / 180.0 * std::f32::consts::PI };
            particle.world[0] += centi(radians.cos() * distance);
            particle.world[1] += centi(radians.sin() * distance);
            if particle.world[1] < 0.0 {
                particle.world[0] = centi(rand.unit() * 120.0 + self.center - 60.0);
                particle.world[2] = centi(rand.unit() * band + self.depth);
                particle.world[1] = 100.0;
            }
        }
        self.previous = now;
    }

    /// Children are projected sprites (record kind 2) cycling a ten-frame sheet at
    /// 100 ms per frame: six frames in column x, four in column x + 20.
    fn pose(&self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize, now: u64) -> Result<()> {
        let frame = (now.saturating_sub(self.start) / 100 % 10) as i16;
        let poses: Vec<_> = self
            .particles
            .iter()
            .map(|particle| {
                let (x, y) = if frame < 6 { (particle.column, frame * 20) } else { (particle.column + 20, (frame - 6) * 20) };
                let pose = Pose {
                    crop: [x, y, 20, 20],
                    origin: [10, 10],
                    kind: 2,
                    world: particle.world,
                    depth: [particle.size, 0.0],
                    ..Pose::default()
                };
                (particle.child, pose)
            })
            .collect();
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Fall {
    /// 0x14005fbd0 (type 6) and 0x140060600 (type 7): falling at most 120 ms per step;
    /// below zero a particle returns 75 units higher at a new x and depth. Type 6
    /// particles settle on the first `ground` triangle beneath them, fading out over the
    /// unit around it, and respawn once half a unit below it.
    fn step(&mut self, rand: &mut Rand, ground: &[[f32; 3]], now: u64) {
        if let Some(fade) = &mut self.fade {
            fade.step(now);
        }
        let collide = self.fade.is_none() && !ground.is_empty();
        let band = depth_band(self.mode);
        let depth = self.depth;
        let elapsed = now.saturating_sub(self.previous).min(120) as f32 * 10.0;
        let respawn = |rand: &mut Rand, particle: &mut FallParticle, landing: &mut Landing| {
            particle.world[0] = centi(rand.unit() * 90.0 - 45.0);
            let z = rand.unit() * band + depth;
            particle.world[1] += 75.0;
            landing.height = 0.0;
            particle.world[2] = centi(z);
            landing.opacity = 255;
        };
        let window = |offset: f32| (offset > -0.5 && offset <= 0.5).then(|| (((offset + 0.5) * 255.0) as i32).min(255));
        for (particle, landing) in self.particles.iter_mut().zip(&mut self.landings) {
            particle.world[1] -= elapsed / particle.speed;
            if collide {
                let y = particle.world[1];
                if landing.height < 1.0 {
                    if let Some(hit) = super::raster::ground(ground, particle.world) {
                        if let Some(opacity) = window(y - hit[1]) {
                            landing.opacity = opacity;
                            landing.height = hit[1];
                        }
                    }
                } else if y - landing.height > -0.5 {
                    if let Some(opacity) = window(y - landing.height) {
                        landing.opacity = opacity;
                    }
                } else {
                    respawn(rand, particle, landing);
                }
            }
            if particle.world[1] < 0.0 {
                respawn(rand, particle, landing);
            }
        }
        self.previous = now;
    }

    fn pose(&self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize) -> Result<()> {
        let poses: Vec<_> = self
            .particles
            .iter()
            .zip(&self.landings)
            .map(|(particle, landing)| {
                let pose = Pose {
                    crop: [0, 0, 32, 32],
                    origin: [16, 16],
                    kind: 2,
                    world: particle.world,
                    depth: [self.size, 0.0],
                    opacity: self.fade.map_or(landing.opacity, |fade| fade.opacity),
                    ..Pose::default()
                };
                (particle.child, pose)
            })
            .collect();
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Fade {
    fn new(mode: u32, now: u64) -> Self {
        let kind = ((mode >> 16) & 0xff) % 3;
        Self {
            mode: kind,
            delay: 100 * u64::from((mode >> 8) & 0xff),
            duration: 100 * u64::from(mode & 0xff),
            start: now,
            opacity: if kind == 1 { 0 } else { 255 },
        }
    }

    /// 0x140060600: before the delay a fade-in is hidden and a fade-out opaque.
    fn step(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.start);
        let progress = elapsed.checked_sub(self.delay).filter(|&at| at < self.duration);
        self.opacity = match (self.mode, progress) {
            (1, Some(at)) => (255 * at / self.duration) as i32,
            (1, None) if elapsed < self.delay => 0,
            (1, None) => 255,
            (2, Some(at)) => 255 - (255 * at / self.duration) as i32,
            (2, None) if elapsed < self.delay => 255,
            (2, None) => 0,
            _ => self.opacity,
        };
    }
}

impl Streaks {
    /// 0x14005e100 / 0x14005e670: a streak takes the first free record.
    fn spawn(&mut self, rand: &mut Rand, time: u64) {
        let Some(free) = self.streaks.iter().position(Option::is_none) else {
            return;
        };
        let world = if self.large {
            let unit = |rand: &mut Rand, scale: f32, offset: f32| rand.unit() * scale + offset;
            // Unlisted bands leave the original's float uninitialised; they read as 0.
            let x = match self.mode & 0xf0 {
                0x00 => unit(rand, 100.0, -50.0),
                0x10 => unit(rand, 100.0, 0.0),
                0x20 => unit(rand, 100.0, -100.0),
                0x30 => unit(rand, 100.0, 50.0),
                0x40 => unit(rand, 100.0, -150.0),
                0x50 => unit(rand, 100.0, -15.0),
                0x60 => unit(rand, 100.0, -85.0),
                0x70 => unit(rand, 100.0, -25.0),
                0x80 => unit(rand, 100.0, -75.0),
                0xa0 => unit(rand, 100.0, -78.0),
                _ => 0.0,
            };
            let z = match self.mode & 0xf00 {
                0x000 => unit(rand, 100.0, 0.0),
                0x100 => unit(rand, 50.0, 0.0),
                0x200 => unit(rand, 50.0, 10.0),
                0x300 => unit(rand, 50.0, 20.0),
                0x400 => unit(rand, 50.0, 30.0),
                0x500 => unit(rand, 50.0, 40.0),
                0x600 => unit(rand, 50.0, 50.0),
                _ => 0.0,
            };
            [x, 150.0, z]
        } else {
            let x = rand.unit() * 10.0 - 5.0;
            let z = rand.unit() * 10.0;
            [x, 6.0, z]
        };
        let degrees = (self.angle as i32).wrapping_add(360) % 360;
        let rotation = 100 * (degrees.abs() % 360);
        self.streaks[free] = Some(Streak { child: self.next_child, world, time, rotation });
        self.next_child += 1;
    }

    fn depth(&self) -> [f32; 2] {
        if self.large {
            [[20.5, 30.5, 40.5, 50.5][(self.mode & 3) as usize], 20.0]
        } else {
            [3.0 + (self.mode & 3) as f32, 3.0]
        }
    }

    fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        rand: &mut Rand,
        slot: usize,
        now: u64,
    ) -> Result<()> {
        self.backlog = (self.backlog + now.saturating_sub(self.previous) as u32).min(1000);
        self.previous = now;
        while self.backlog > self.interval {
            let time = now + u64::from(self.interval) - u64::from(self.backlog);
            self.spawn(rand, time);
            if self.large && self.mode & 0x1000 != 0 {
                self.spawn(rand, time);
            }
            self.backlog -= self.interval;
        }
        let (divisor, floor) = if self.large { (250.0, -50.0) } else { (1000.0, 0.0) };
        let depth = self.depth();
        let mut poses = Vec::new();
        let mut retired = Vec::new();
        for record in &mut self.streaks {
            let Some(streak) = record else { continue };
            let distance = now.saturating_sub(streak.time) as f32 * self.speed / divisor;
            if self.angle == 0.0 {
                streak.world[1] -= distance;
            } else {
                let radians = (self.angle + 90.0) / 180.0 * std::f32::consts::PI;
                streak.world[1] -= radians.sin() * distance;
                streak.world[0] += radians.cos() * distance;
            }
            if streak.world[1] <= floor {
                retired.push(streak.child);
                *record = None;
                continue;
            }
            streak.time = now;
            let pose = Pose {
                crop: [0, 0, 6, 112],
                origin: [3, 112],
                kind: 2,
                world: streak.world,
                depth,
                rotation: streak.rotation,
                ..Pose::default()
            };
            poses.push((streak.child, pose));
        }
        objects.remove_children(host, client, slot, &retired)?;
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Loop {
    fn step(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.previous).min(500) as f32 * 10.0;
        for particle in &mut self.particles {
            particle.world[1] -= elapsed / particle.speed;
            if particle.world[1] < 0.0 {
                particle.world[1] += 10.0;
            }
        }
        self.previous = now;
    }

    fn pose(&self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize) -> Result<()> {
        let poses: Vec<_> = self
            .particles
            .iter()
            .map(|particle| {
                let pose = Pose {
                    crop: [0, 0, 32, 32],
                    origin: [16, 16],
                    kind: 2,
                    world: particle.world,
                    depth: [self.lift, 3.0],
                    ..Pose::default()
                };
                (particle.child, pose)
            })
            .collect();
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Flurry {
    fn step(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.previous).min(500) as u32;
        for flake in &mut self.flakes {
            flake.position[1] = flake.position[1].wrapping_add((elapsed.wrapping_mul(flake.speed) >> 4) as i32);
            if flake.position[1] >> 10 >= self.height + 32 {
                flake.position[1] = -16_384;
            }
        }
        self.previous = now;
    }

    fn pose(&self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize) -> Result<()> {
        let poses: Vec<_> = self
            .flakes
            .iter()
            .map(|flake| {
                let pose = Pose {
                    crop: flake.crop,
                    origin: [flake.origin; 2],
                    position: flake.position.map(|axis| axis >> 10),
                    order: flake.order,
                    ..Pose::default()
                };
                (flake.child, pose)
            })
            .collect();
        objects.pose_children(host, client, slot, &poses)
    }
}

impl Field {
    /// 0x140060cd0: particles fall `area` units per `speed` ms (at most 500 ms per
    /// step), wrap through `height` and, when following, stay within half an area of
    /// the camera eye.
    fn step(&mut self, cameras: &[[u32; 40]; 2], now: u64) {
        let elapsed = now.saturating_sub(self.previous).min(500) as f32;
        let eye = [1, 2, 3].map(|index| f32::from_bits(cameras[0][index]));
        let half = self.area * 0.5;
        for particle in &mut self.particles {
            particle.world[1] -= elapsed * self.area / particle.speed;
            if particle.world[1] <= 0.0 {
                particle.world[1] += self.height;
            }
            if self.follow {
                for axis in [0, 2] {
                    let offset = particle.world[axis] - eye[axis];
                    if offset.abs() > half {
                        particle.world[axis] += if offset < 0.0 { self.area } else { -self.area };
                    }
                }
            }
        }
        self.previous = now;
    }

    /// Opacity: full within `fade` of the eye (lowered below height 0.5), then a
    /// linear fade over the next unit.
    fn pose(&self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize, cameras: &[[u32; 40]; 2]) -> Result<()> {
        let eye = [1, 2, 3].map(|index| f32::from_bits(cameras[0][index]));
        let [width, height] = self.size.map(|axis| axis.min(i16::MAX as u32) as i16);
        let poses: Vec<_> = self
            .particles
            .iter()
            .map(|particle| {
                let distance = particle.world.iter().zip(eye).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt();
                let opacity = if distance <= self.fade {
                    let y = particle.world[1];
                    if y < 0.5 && ((y * 510.0) as i32) < 255 { (y * 510.0) as i32 } else { 255 }
                } else if distance - self.fade <= 1.0 {
                    ((1.0 - (distance - self.fade)) * 255.0) as i32
                } else {
                    0
                };
                let pose = Pose {
                    crop: [0, 0, width, height],
                    origin: [width / 2, height / 2],
                    kind: 2,
                    world: particle.world,
                    opacity,
                    ..Pose::default()
                };
                (particle.child, pose)
            })
            .collect();
        objects.pose_children(host, client, slot, &poses)
    }
}
