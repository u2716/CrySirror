// SPDX-License-Identifier: MPL-2.0
//! SEBreath stores at most 32 mouth-centered masks, in emission order.
use super::{ClientId, HostServices, Result, Sprite, Value, asset};
use crysirror_image::{ImageSurface, temporal_pattern::PatternRng};
use std::{
    collections::VecDeque,
    sync::{Arc, mpsc::Receiver},
};

pub(super) const BASE: u32 = 552;
#[derive(Clone, Copy)]
struct Puff {
    center: [i32; 2],
    born: u64,
    level: i32,
    width: i32,
}

pub(super) struct Breath {
    mask: Option<Arc<ImageSurface>>,
    loading: Option<Receiver<Result<ImageSurface>>>,
    drawing: Option<Receiver<Result<ImageSurface>>>,
    puffs: VecDeque<Puff>,
    last: Option<u64>,
    elapsed: i32,
    gain: i32,
    had_pixels: bool,
    pub published: bool,
    pub retiring: bool,
    pub fading: bool,
}

impl Breath {
    pub fn load(h: &mut dyn HostServices, c: ClientId) -> Result<Self> {
        let storage = h.storage(c)?;
        let path = asset("sys", "breath.png")?;
        let loading = crate::worker(move || {
            if storage.exists(&path) {
                let bytes =
                    crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 64 << 20)?;
                if let Ok(mask) = crysirror_image::decode_standard(&bytes) {
                    return Ok(mask);
                }
            }
            ImageSurface::filled(256, 256, [255, 0, 0, 255])
        })?;
        Ok(Self {
            mask: None,
            loading: Some(loading),
            drawing: None,
            puffs: VecDeque::new(),
            last: None,
            elapsed: 0,
            gain: 0,
            had_pixels: false,
            published: false,
            retiring: false,
            fading: false,
        })
    }
    pub fn emit(
        &mut self,
        center: [i32; 2],
        amplitude: u32,
        now: u64,
        rng: &mut PatternRng,
    ) -> Result<()> {
        let level = (rng.next_draw()? % 32 + 40) as i32;
        // The native initializer overwrites its provisional audio-derived
        // width. Its conditional random draw still advances the shared RNG.
        if (amplitude >> 10) + rng.next_draw()? % 32 <= 100 {
            rng.next_draw()?;
        }
        let width = (rng.next_draw()? % 64 + 160) as i32;
        if self.puffs.len() == 32 {
            self.puffs.pop_front();
        }
        self.puffs.push_back(Puff {
            center,
            born: now,
            level,
            width,
        });
        Ok(())
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        slot: usize,
        canvas: [i32; 2],
        interval: u32,
        ops: &mut Vec<Value>,
    ) -> Result<()> {
        let id = BASE + slot as u32;
        if let Some(job) = &self.loading {
            let Some(mask) = crate::poll(job)? else {
                return Ok(());
            };
            self.mask = Some(Arc::new(mask));
            self.loading = None;
        }
        if let Some(job) = &self.drawing {
            let Some(image) = crate::poll(job)? else {
                return Ok(());
            };
            self.drawing = None;
            let path = h.present_surface(c, id, image.width, image.height, &image.rgba)?;
            let art = Sprite {
                asset: path.relative().to_owned(),
                mask_color: None,
                rect: [0, 0, canvas[0], canvas[1]],
            };
            let mut op = art.layer(id, 255);
            op["layer"]["order"] = serde_json::json!([49, slot]);
            ops.push(op);
            self.published = true;
        }
        let now = h.now_ms() / 10;
        let delta = match self.last {
            None => 1,
            Some(last) => {
                let d = now.abs_diff(last);
                if d == 0 || d < u64::from(interval) {
                    return Ok(());
                }
                if d > 20 { interval as i32 } else { d as i32 }
            }
        };
        self.last = Some(now);
        self.elapsed = self.elapsed.wrapping_add(delta);
        let step = self.elapsed >> 4;
        self.gain = if self.fading {
            self.gain.saturating_sub(step).max(0)
        } else {
            self.gain.saturating_add(step).min(256)
        };
        let mask = self.mask.as_ref().expect("loaded breath mask").clone();
        let mut stamps = Vec::new();
        self.puffs.retain(|p| {
            let age = now.abs_diff(p.born).min(i32::MAX as u64) as i32;
            let level = p.level - age / 5;
            let strength = level.max(0) * self.gain >> 8;
            if level > 0 && strength > 0 {
                let size = (p.width + age / 2).min(mask.width as i32).max(0) as u32;
                stamps.push((
                    p.center.map(|n| n.saturating_sub(size as i32 / 2)),
                    size,
                    strength as u32,
                ));
            }
            level > 0
        });
        if stamps.is_empty() && !self.had_pixels {
            return Ok(());
        }
        self.had_pixels = !stamps.is_empty();
        self.drawing = Some(crate::worker(move || {
            let mut frame = ImageSurface::filled(canvas[0] as u32, canvas[1] as u32, [0; 4])?;
            for (origin, size, strength) in stamps {
                crysirror_image::packed_stamps::add_white_mask(
                    &mut frame, &mask, origin, [size; 2], strength, 512, 127,
                )?;
            }
            Ok(frame)
        })?);
        Ok(())
    }
}
