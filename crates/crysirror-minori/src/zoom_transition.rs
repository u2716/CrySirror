// SPDX-License-Identifier: MPL-2.0
use anyhow::{Result, ensure};
use crysirror_image::ImageSurface;
use std::sync::mpsc::Receiver;

pub(super) struct Zoom {
    pub target: super::Sprite,
    job: Receiver<Result<(ImageSurface, ImageSurface)>>,
    pixels: Option<(ImageSurface, ImageSurface, ImageSurface)>,
    speed: u32,
    start: u64,
    last: Option<u32>,
    phase: u32,
    pub published: bool,
}

pub(super) enum Draw<'a> {
    Hold,
    Pixels(&'a ImageSurface),
    Finished,
}

impl Zoom {
    pub fn begin(
        h: &mut dyn super::HostServices,
        c: super::ClientId,
        old: &super::Sprite,
        target: super::Sprite,
        speed: i32,
    ) -> Result<Self> {
        use crysirror_core::path::{Mount, VfsPath};
        let storage = h.storage(c)?;
        let assets = [old.asset.clone(), target.asset.clone()];
        let size = [target.rect[2] as u32, target.rect[3] as u32];
        let job = crate::worker(move || {
            let decode = |name: &str| -> Result<ImageSurface> {
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, name)?)?;
                let img = crysirror_image::decode_standard(&crysirror_resource::read_all(
                    reader.as_ref(),
                    128 << 20,
                )?)?;
                ensure!(
                    [img.width, img.height] == size
                        && img.rgba.chunks_exact(4).all(|p| p[3] == 255),
                    "unsupported minori stage transition 2 with nonopaque or resized background"
                );
                Ok(img)
            };
            Ok((decode(&assets[0])?, decode(&assets[1])?))
        })?;
        Ok(Self {
            target,
            job,
            pixels: None,
            speed: speed as u32,
            start: h.now_ms(),
            last: None,
            phase: 0,
            published: false,
        })
    }

    pub fn draw(&mut self, now: u64, interval: u32, skip: bool) -> Result<Draw<'_>> {
        if self.pixels.is_none() {
            let Some((old, new)) = crate::poll(&self.job)? else {
                return Ok(Draw::Hold);
            };
            self.pixels = Some((old.clone(), old, new));
        }
        if skip {
            return Ok(Draw::Finished);
        }
        let tick = (now.saturating_sub(self.start) / 10) as u32;
        if self.last.is_some_and(|t| tick.wrapping_sub(t) < interval) {
            return Ok(Draw::Hold);
        }
        self.last = Some(tick);
        let phase = tick.wrapping_mul(self.speed) / 10;
        if phase == self.phase {
            return Ok(Draw::Hold);
        }
        self.phase = phase;
        if phase >= 400 {
            return Ok(Draw::Finished);
        }
        let (history, old, new) = self.pixels.as_mut().unwrap();
        let (index, src) = if phase < 200 {
            (phase, old)
        } else {
            (399 - phase, new)
        };
        // All 200 original doubles yield these same integer crop extents for
        // every accepted canvas dimension (1..=8192), in all three profiles.
        let zoom = 1.0 + 99.0 * f64::from(index * index) / (199.0 * 199.0);
        let crop = [src.width, src.height].map(|d| (f64::from(d) / zoom) as u32);
        crysirror_image::overlap_resample::blend_centered_crop(history, src, crop)?;
        Ok(Draw::Pixels(history))
    }
}
