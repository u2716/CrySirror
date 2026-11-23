// SPDX-License-Identifier: MPL-2.0
//! The v1 ANI resource binds named eye and mouth bitmaps to a standing image.
use super::{ClientId, HostServices, Result, Sprite, Value, json, remove};
use anyhow::{Context, ensure};
use crysirror_core::{
    image_processing::PackedPixelLayout,
    path::{Mount, VfsPath},
};
use crysirror_image::{ImageSurface, packed_pixels, temporal_pattern::PatternRng};
use std::{collections::BTreeMap, sync::mpsc::Receiver};

pub(in crate::presentation) type EyeSequences = std::sync::Arc<[Vec<(u8, u64)>]>;

fn matches_speaker(actor: &str, speaker: &str) -> bool {
    let mut name = actor;
    if name.len() > 1
        && name
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase() || *b == b'$')
    {
        name = &name[1..];
    }
    if name.ends_with('>')
        && let Some((base, _)) = name.split_once('<')
    {
        name = base;
    }
    speaker == "*" || speaker == name
}

struct Patch {
    at: [i32; 2],
    bitmap: ImageSurface,
}
struct Track {
    actor: String,
    eye: bool,
    images: BTreeMap<u8, Patch>,
    pose: Option<u8>,
    sprite: Option<Sprite>,
    published: bool,
    phase: usize,
    eye_preset: i32,
    next: u64,
    previous_level: u32,
    callback: Option<(u64, bool, u64)>,
}
pub(super) struct Face {
    tracks: Vec<Track>,
    loading: Option<Receiver<Result<Vec<Track>>>>,
    blink: bool,
    programs: EyeSequences,
}

fn decode(bytes: &[u8], default_eye: i32) -> Result<Vec<Track>> {
    ensure!(
        bytes.len() >= 8 && bytes[..2] == [0, 1] && bytes[4..8] == [0; 4],
        "unsupported minori ANI header"
    );
    let count = u16::from_le_bytes([bytes[2], bytes[3]]);
    let mut data = &bytes[8..];
    let mut tracks = BTreeMap::new();
    for _ in 0..count {
        let end = data
            .iter()
            .position(|&b| b == 0)
            .context("unterminated minori ANI label")?;
        let label = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&data[..end])
            .context("invalid minori ANI label")?;
        data = &data[end + 1..];
        let header = data.get(..10).context("truncated minori ANI patch")?;
        let word = |i| u16::from_le_bytes([header[i], header[i + 1]]);
        let size = [u32::from(word(0)), u32::from(word(2))];
        ensure!(
            word(4) == 32,
            "unsupported minori ANI pixel depth {}",
            word(4)
        );
        let at = [i32::from(word(6) as i16), i32::from(word(8) as i16)];
        let len = crysirror_image::rgba_len(size[0], size[1])?;
        let pixels = data
            .get(10..10 + len)
            .context("truncated minori ANI pixels")?;
        data = &data[10 + len..];
        let Some((actor, name)) = label.split_once(' ') else {
            continue;
        };
        let (name, override_id) = name.split_once(' ').map_or((name, None), |(pose, id)| {
            (
                pose,
                crate::vm::decimal_prefix(id).filter(|id| (0..6).contains(id)),
            )
        });
        let (eye, pose) = match name {
            "開き目" => (true, 0),
            "中目" => (true, 1),
            "閉じ目" => (true, 2),
            "閉じ口" => (false, 0),
            "中口" => (false, 1),
            "開き口" => (false, 2),
            _ => continue,
        };
        let key = (actor.to_owned(), eye);
        let track = tracks.entry(key).or_insert_with(|| Track {
            actor: actor.to_owned(),
            eye,
            images: BTreeMap::new(),
            pose: None,
            sprite: None,
            published: false,
            phase: 0,
            eye_preset: default_eye,
            next: 0,
            previous_level: 0,
            callback: None,
        });
        if let Some(id) = override_id {
            track.eye_preset = id;
        }
        track.images.insert(
            pose,
            Patch {
                at,
                bitmap: packed_pixels::decode(size, PackedPixelLayout::Bgra8, pixels, &|| Ok(()))?,
            },
        );
    }
    ensure!(tracks.len() <= 64, "minori ANI track limit");
    Ok(tracks.into_values().collect())
}

// Sequence IDs select native pose/duration pairs. The tick is 10 ms.
fn eye_sequence(id: i32) -> &'static [(u8, u64)] {
    const PRESETS: [&[(u8, u64)]; 6] = [
        &[
            (0, 40),
            (1, 40),
            (2, 80),
            (1, 40),
            (0, 400),
            (1, 40),
            (2, 80),
            (1, 40),
            (0, 3600),
            (1, 40),
            (2, 80),
            (1, 40),
            (0, 1200),
        ],
        &[
            (0, 10),
            (1, 20),
            (2, 40),
            (1, 20),
            (0, 200),
            (1, 20),
            (2, 40),
            (1, 20),
            (0, 1800),
            (1, 20),
            (2, 40),
            (1, 20),
            (0, 600),
        ],
        &[
            (0, 10),
            (1, 10),
            (2, 20),
            (1, 10),
            (0, 100),
            (1, 10),
            (2, 20),
            (1, 10),
            (0, 900),
            (1, 10),
            (2, 20),
            (1, 10),
            (0, 300),
        ],
        &[
            (0, 5),
            (1, 5),
            (2, 10),
            (1, 5),
            (0, 100),
            (1, 5),
            (2, 10),
            (1, 5),
            (0, 450),
            (1, 5),
            (2, 10),
            (1, 5),
            (0, 150),
        ],
        &[
            (0, 3),
            (1, 3),
            (2, 7),
            (1, 3),
            (0, 33),
            (1, 3),
            (2, 7),
            (1, 3),
            (0, 225),
            (1, 3),
            (2, 7),
            (1, 3),
            (0, 75),
        ],
        &[(0, 30), (1, 30), (2, 30), (1, 30)],
    ];
    PRESETS.get(id as usize).copied().unwrap_or(PRESETS[2])
}
fn companion(raw: &str) -> Result<VfsPath> {
    Ok(VfsPath::new(
        Mount::Asset,
        &format!("{}.ani", raw.rsplit_once('.').map_or(raw, |p| p.0)),
    )?)
}

pub(crate) fn apply_fixed_poses(
    storage: &dyn crysirror_core::service::ClientStorage,
    image_path: &str,
    poses: &[crate::vm::FixedFacePose],
    tone: Option<super::tone::Tone>,
    image: &mut ImageSurface,
) -> Result<()> {
    use crysirror_image::bitmap::{BitmapRect, BlitOptions, PixelMath};
    let path = companion(image_path)?;
    if !storage.exists(&path) { return Ok(()); }
    let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 64 << 20)?;
    for track in decode(&bytes, 0)? {
        let Some(patch) = poses.iter().rev()
            .find(|p| p.eye == track.eye && p.actor.as_ref().is_none_or(|a| matches_speaker(&track.actor, a)))
            .and_then(|p| track.images.get(&p.pose))
        else { continue };
        let mapped;
        let source = if let Some(tone) = tone {
            mapped = tone.map(&patch.bitmap)?;
            &mapped
        } else { &patch.bitmap };
        image.blit(source, BlitOptions {
            source_rect: BitmapRect::new(0, 0, source.width, source.height),
            destination: patch.at,
            opacity: 255,
            mode: PixelMath::MaskRgbaScalar256,
            color_key: None,
        })?;
    }
    Ok(())
}

pub(super) fn has_companion(h: &mut dyn HostServices, c: ClientId, source: &Sprite) -> Result<bool> {
    let raw = super::tone::path(&source.asset).map(|(_, raw)| raw);
    Ok(h.storage(c)?.exists(&companion(raw.as_deref().unwrap_or(&source.asset))?))
}

impl Face {
    pub fn open(
        h: &mut dyn HostServices,
        c: ClientId,
        source: &Sprite,
        blink: bool,
        eye_preset: i32,
        programs: EyeSequences,
    ) -> Result<Self> {
        let (tone, raw) = super::tone::path(&source.asset)
            .map_or((None, source.asset.clone()), |(t, p)| (Some(t), p));
        let path = companion(&raw)?;
        let storage = h.storage(c)?;
        let loading = if storage.exists(&path) {
            Some(crate::worker(move || {
                let mut tracks = decode(
                    &crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 64 << 20)?,
                    eye_preset,
                )?;
                if let Some(tone) = tone {
                    for patch in tracks.iter_mut().flat_map(|t| t.images.values_mut()) {
                        patch.bitmap = tone.map(&patch.bitmap)?;
                    }
                }
                Ok(tracks)
            })?)
        } else {
            None
        };
        Ok(Self {
            tracks: Vec::new(),
            loading,
            blink,
            programs,
        })
    }
    pub fn active(&self) -> bool {
        self.loading.is_some() || !self.tracks.is_empty()
    }
    fn id(base: u32, index: usize) -> Result<u32> {
        base.checked_mul(64)
            .and_then(|x| x.checked_add(0x1000_0000 + index as u32))
            .context("minori ANI layer exhaustion")
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        base: u32,
        speaker: &str,
        level: super::VoiceLevel,
        rng: &mut PatternRng,
    ) -> Result<bool> {
        if let Some(job) = &self.loading {
            let Some(tracks) = crate::poll(job)? else {
                return Ok(false);
            };
            self.loading = None;
            self.tracks = tracks;
        }
        let now = h.now_ms() / 10;
        let mut changed = false;
        for (index, track) in self.tracks.iter_mut().enumerate() {
            let previous = track.pose.unwrap_or(0);
            let pose = if track.eye {
                if self.blink && now >= track.next {
                    let sequence = self
                        .programs
                        .get(track.eye_preset as usize)
                        .map(Vec::as_slice)
                        .unwrap_or_else(|| eye_sequence(track.eye_preset));
                    let (pose, delay) = sequence[track.phase];
                    let jitter = if track.phase == 0 && track.eye_preset < 5 {
                        u64::from(rng.next_draw()? % 500 + 50)
                    } else {
                        0
                    };
                    track.next = now + delay + jitter;
                    track.phase = (track.phase + 1) % sequence.len();
                    pose
                } else {
                    previous
                }
            } else {
                let playing = level.playing && matches_speaker(&track.actor, speaker);
                let callback = (
                    level.generation,
                    playing,
                    if playing { level.serial } else { 0 },
                );
                if track.callback == Some(callback) {
                    continue;
                }
                track.callback = Some(callback);
                let level = if playing { level.amplitude } else { 0 };
                let pose = if level < 0x320 {
                    u8::from(previous == 2)
                } else if level > track.previous_level {
                    if previous == 0 { 1 } else { 2 }
                } else {
                    u8::from(previous != 1 || level >= track.previous_level / 2)
                };
                track.previous_level = level;
                pose
            };
            if track.pose == Some(pose) {
                continue;
            }
            track.pose = Some(pose);
            changed = true;
            track.sprite = if let Some(patch) = track.images.get(&pose) {
                let image = &patch.bitmap;
                let path = h.present_surface(
                    c,
                    Self::id(base, index)?,
                    image.width,
                    image.height,
                    &image.rgba,
                )?;
                track.published = true;
                Some(Sprite {
                    asset: path.relative().to_owned(),
                    mask_color: None,
                    rect: [
                        patch.at[0],
                        patch.at[1],
                        image.width as i32,
                        image.height as i32,
                    ],
                })
            } else {
                None
            };
        }
        Ok(changed)
    }

    pub fn mouth_centers(
        &self,
        speaker: &str,
        rect: [i32; 4],
        original: [i32; 2],
        out: &mut Vec<[i32; 2]>,
    ) {
        for track in &self.tracks {
            if track.eye || !matches_speaker(&track.actor, speaker) {
                continue;
            }
            if let Some(patch) = &track.sprite
                && patch.rect[2] > 1
                && patch.rect[3] > 1
            {
                out.push([0, 1].map(|i| {
                    rect[i].saturating_add(
                        ((i64::from(patch.rect[i]) + i64::from(patch.rect[i + 2] / 2))
                            * i64::from(rect[i + 2])
                            / i64::from(original[i])) as i32,
                    )
                }));
            }
        }
    }
    pub fn draw(
        &self,
        base: u32,
        rect: [i32; 4],
        original: [i32; 2],
        order: [i32; 2],
        opacity: u8,
        ops: &mut Vec<Value>,
    ) {
        for (i, track) in self.tracks.iter().enumerate() {
            let Ok(id) = Self::id(base, i) else { continue };
            if let Some(patch) = &track.sprite {
                let mut art = patch.clone();
                for axis in 0..2 {
                    let scale = |v| {
                        (i64::from(v) * i64::from(rect[axis + 2]) / i64::from(original[axis]))
                            as i32
                    };
                    art.rect[axis] = rect[axis].saturating_add(scale(art.rect[axis]));
                    art.rect[axis + 2] = scale(art.rect[axis + 2]).max(1);
                }
                let mut op = art.layer(id, opacity);
                op["layer"]["order"] = json!(order);
                ops.push(op);
            } else {
                ops.push(remove(id));
            }
        }
    }
    pub fn remove(&self, base: u32, ops: &mut Vec<Value>) {
        for i in 0..self.tracks.len() {
            if let Ok(id) = Self::id(base, i) {
                ops.push(remove(id));
            }
        }
    }
    pub fn release(&self, h: &mut dyn HostServices, c: ClientId, base: u32) -> Result<()> {
        for (i, track) in self.tracks.iter().enumerate() {
            if track.published {
                h.release_surface(c, Self::id(base, i)?)?;
            }
        }
        Ok(())
    }
}
