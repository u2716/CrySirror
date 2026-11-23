// SPDX-License-Identifier: MPL-2.0
//! CMMV timeline and scene-object ownership, separate from .char's ANI lists.
use super::{
    ClientId, HostServices, Published, Result, Sprite, asset, characters::Characters, json, put,
    remove, submit,
};
use crate::vm::AnimationCommand;
use anyhow::{Context, ensure};
use crysirror_core::service::ClientStorage;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::mpsc::Receiver,
};

struct Clip {
    images: Vec<String>,
    tracks: Vec<Track>,
    length: u32,
}
struct Track {
    kind: i32,
    resource: i32,
    first: u32,
    count: u32,
    order: i32,
    start: [i32; 2],
    end: [i32; 2],
    alpha: [u8; 2],
    width: [i32; 2],
    height: [i32; 2],
    color: [[u8; 4]; 2],
    scale: [[f64; 2]; 2],
}

fn take<'a>(data: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    let (v, rest) = data.split_at_checked(n).context("truncated MMV")?;
    *data = rest;
    Ok(v)
}
fn word(data: &mut &[u8]) -> Result<u32> {
    Ok(u32::from_le_bytes(take(data, 4)?.try_into()?))
}
impl Clip {
    fn parse(mut data: &[u8], dir: &str) -> Result<Self> {
        ensure!(take(&mut data, 4)? == b"MMV\0", "MMV signature");
        let stride = match word(&mut data)? {
            0x10000 => 76,
            0x20000 => 136,
            v => anyhow::bail!("MMV version {v:#x}"),
        };
        let entries = word(&mut data)? as usize;
        ensure!(
            entries <= data.len() && entries <= 65536,
            "MMV resource count"
        );
        let mut images = Vec::new();
        for _ in 0..entries {
            let n = data
                .iter()
                .position(|&v| v == 0)
                .context("MMV resource terminator")?;
            let raw = take(&mut data, n + 1)?;
            let (name, err) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..n]);
            ensure!(!err, "MMV resource encoding");
            if name.contains(".png") {
                images.push(asset("st", &format!("{dir}{name}"))?.relative().to_owned());
            }
        }
        let count = word(&mut data)? as usize;
        ensure!(
            count <= data.len() / stride && count <= 65536,
            "MMV motion count"
        );
        let mut tracks = Vec::with_capacity(count);
        let mut length = 0;
        for _ in 0..count {
            let bytes = take(&mut data, stride)?;
            let int = |i: usize| i32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            let scale = |i: usize| {
                if stride == 76 {
                    1.
                } else {
                    f64::from_le_bytes(bytes[i..i + 8].try_into().unwrap())
                }
            };
            let first = int(8) as u32;
            let count = int(12) as u32;
            length = length.max(
                first
                    .checked_add(count)
                    .context("MMV motion interval overflow")?,
            );
            let track = Track {
                kind: int(0),
                resource: int(4),
                first,
                count,
                order: int(16),
                start: [int(20), int(24)],
                end: [int(28), int(32)],
                alpha: [
                    (int(44) as u8).wrapping_sub(1),
                    (int(48) as u8).wrapping_sub(1),
                ],
                width: [int(52), int(56)],
                height: [int(60), int(64)],
                color: [bytes[68..72].try_into()?, bytes[72..76].try_into()?],
                scale: [[scale(76), scale(84)], [scale(92), scale(100)]],
            };
            ensure!(
                track.scale.iter().flatten().all(|v| v.is_finite()),
                "MMV non-finite scale"
            );
            tracks.push(track);
        }
        Ok(Self {
            images,
            tracks,
            length,
        })
    }
    fn read(storage: &dyn ClientStorage, name: &str) -> Result<Self> {
        let name = name.replace('\\', "/");
        let base = name.rsplit('/').next().unwrap_or(&name);
        let dir = format!("{name}/");
        let path = asset("st", &format!("{dir}{base}.mmv"))?;
        if !storage.exists(&path) {
            return Ok(Self {
                images: vec![],
                tracks: vec![],
                length: 0,
            });
        }
        Self::parse(
            &crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 16 << 20)?,
            &dir,
        )
    }
}

struct Object {
    clip: Clip,
    images: Vec<Option<Sprite>>,
    layers: Vec<u32>,
    visible: BTreeSet<u32>,
    pos: [i32; 2],
    parent: i32,
    loops: i32,
    start: Option<u64>,
    frame: Option<u32>,
    last: Option<(u32, [i32; 2])>,
}
#[derive(Default)]
pub(super) struct Animations {
    objects: BTreeMap<i32, Object>,
    next_layer: u32,
    loading: Option<(i32, Receiver<Result<Clip>>)>,
}
const FIRST_LAYER: u32 = 0x0100_0000;
impl Animations {
    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }
    pub fn playing(&self, id: i32) -> bool {
        if id < 0 {
            self.objects.values().any(|o| o.start.is_some())
        } else {
            self.objects.get(&id).is_some_and(|o| o.start.is_some())
        }
    }
    fn forget(&mut self, id: i32, ops: &mut Vec<serde_json::Value>) {
        if let Some(o) = self.objects.remove(&id) {
            ops.extend(o.visible.into_iter().map(remove));
        }
    }
    pub fn command(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        action: AnimationCommand,
        ops: &mut Vec<serde_json::Value>,
    ) -> Result<()> {
        use AnimationCommand::*;
        match action {
            Load(id, name) => {
                let storage = h.storage(c)?;
                self.loading = Some((
                    id,
                    crate::worker(move || Clip::read(storage.as_ref(), &name))?,
                ));
            }
            Clear(0) => {
                for o in std::mem::take(&mut self.objects).into_values() {
                    ops.extend(o.visible.into_iter().map(remove));
                }
            }
            Clear(id) => self.forget(id.saturating_abs(), ops),
            Play(id) => {
                if let Some(o) = self.objects.get_mut(&id)
                    && o.start.is_none()
                    && o.clip.length > 0
                {
                    o.start = Some(h.now_ms() / 10);
                    o.frame = Some(0);
                    o.last = None;
                }
            }
            Pause(id) => {
                if let Some(o) = self.objects.get_mut(&id) {
                    o.start = None;
                }
            }
            Position(id, pos) => {
                if let Some(o) = self.objects.get_mut(&id) {
                    o.pos = pos;
                }
            }
            Parent(id, parent) => {
                if let Some(o) = self.objects.get_mut(&id) {
                    o.parent = parent;
                }
            }
            Loop(id, n) => {
                if let Some(o) = self.objects.get_mut(&id) {
                    o.loops = n;
                }
            }
        }
        Ok(())
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        chars: &Characters,
        layers: &mut Published,
    ) -> Result<()> {
        let mut ops = Vec::new();
        if let Some((_, job)) = &self.loading {
            let Some(clip) = crate::poll(job)? else {
                return Ok(());
            };
            let (id, _) = self.loading.take().unwrap();
            let mut images = Vec::new();
            let storage = h.storage(c)?;
            for path in &clip.images {
                let art = if storage.exists(&crysirror_core::path::VfsPath::new(
                    crysirror_core::path::Mount::Asset,
                    path,
                )?) {
                    let [w, height] = h.image_size(c, path)?;
                    Some(Sprite {
                        asset: path.clone(),
                        mask_color: None,
                        rect: [0, 0, w.try_into()?, height.try_into()?],
                    })
                } else {
                    None
                };
                images.push(art);
            }
            let base = self.next_layer;
            self.next_layer = base
                .checked_add(clip.tracks.len().try_into()?)
                .context("MMV layer count")?;
            ensure!(self.next_layer < 0x0100_0000, "MMV layer ID range");
            self.forget(id, &mut ops);
            self.objects.insert(
                id,
                Object {
                    clip,
                    images,
                    layers: (base..self.next_layer).map(|i| FIRST_LAYER + i).collect(),
                    visible: BTreeSet::new(),
                    pos: [0; 2],
                    parent: -1,
                    loops: 1,
                    start: None,
                    frame: None,
                    last: None,
                },
            );
        }
        // Parent references are logical scene object IDs. Resolve ahead of mutable playback.
        let mut offsets = BTreeMap::new();
        for (&id, obj) in &self.objects {
            let mut parent = obj.parent;
            let mut pos = obj.pos;
            let mut seen = BTreeSet::from([id]);
            while parent >= 0 && seen.insert(parent) {
                let (p, next) = if let Some(o) = self.objects.get(&parent) {
                    (o.pos, o.parent)
                } else if let Some(p) = chars.position(parent) {
                    (p, -1)
                } else {
                    break;
                };
                pos = [pos[0].saturating_add(p[0]), pos[1].saturating_add(p[1])];
                parent = next;
            }
            offsets.insert(id, pos);
        }
        for (&id, o) in &mut self.objects {
            if let Some(start) = o.start {
                let frames = u128::from((h.now_ms() / 10).saturating_sub(start)) * 1000 / 1667;
                if o.loops > 0 && frames >= u128::from(o.clip.length) * o.loops as u128 {
                    o.frame = Some(o.clip.length - 1);
                    o.start = None;
                } else {
                    o.frame = Some((frames % u128::from(o.clip.length)) as u32);
                }
            }
            let Some(frame) = o.frame else { continue };
            let pos = offsets[&id];
            if o.last == Some((frame, pos)) {
                continue;
            }
            o.last = Some((frame, pos));
            let mut visible = BTreeSet::new();
            for (track, &layer) in o.clip.tracks.iter().zip(&o.layers) {
                if frame < track.first || frame - track.first >= track.count {
                    continue;
                }
                let t = f64::from(frame - track.first + 1) / f64::from(track.count);
                let lerp =
                    |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t) as i32;
                let alpha = lerp(i32::from(track.alpha[0]), i32::from(track.alpha[1])) as u8;
                let x = pos[0].saturating_add(lerp(track.start[0], track.end[0]));
                let y = pos[1].saturating_add(lerp(track.start[1], track.end[1]));
                let mut op = match track.kind {
                    0 => {
                        let Some(Some(art)) = o.images.get(track.resource as usize) else {
                            continue;
                        };
                        let mut art = art.clone();
                        for i in 0..2 {
                            let scale =
                                track.scale[0][i] + (track.scale[1][i] - track.scale[0][i]) * t;
                            art.rect[2 + i] = (scale * f64::from(art.rect[2 + i])).max(1.) as i32;
                        }
                        art.rect[0] = x;
                        art.rect[1] = y;
                        art.layer(layer, alpha)
                    }
                    2 => {
                        let color: [u8; 4] = std::array::from_fn(|i| {
                            lerp(i32::from(track.color[0][i]), i32::from(track.color[1][i])) as u8
                        });
                        let w = lerp(track.width[0], track.width[1]);
                        let height = lerp(track.height[0], track.height[1]);
                        if w <= 0 || height <= 0 {
                            continue;
                        }
                        put(
                            layer,
                            [x, y, w, height],
                            json!({"kind":"solid","color":[color[0],color[1],color[2],255]}),
                            255,
                        )
                    }
                    _ => continue,
                };
                let base = if id >= 2000 { 1_000_000 } else { 100 };
                op["layer"]["order"] =
                    json!([40, base + i64::from(id) * 4 + i64::from(track.order)]);
                ops.push(op);
                visible.insert(layer);
            }
            ops.extend(o.visible.difference(&visible).copied().map(remove));
            o.visible = visible;
        }
        submit(layers, h, c, ops)
    }
}
