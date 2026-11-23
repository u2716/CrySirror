// SPDX-License-Identifier: MPL-2.0
use super::{
    ClientId, HostServices, Published, Result, Sprite, asset, json, remove, sprite, submit,
};
use crate::vm::EffectRequest;
use anyhow::bail;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
};
use crysirror_image::effects::ImageEffect;

pub(super) struct Crossfade {
    sources: Vec<Sprite>,
    toned: Vec<Option<ImageInput>>,
    preparing: Vec<(usize, std::sync::mpsc::Receiver<Result<crysirror_image::ImageSurface>>)>,
    indices: [usize; 2],
    forwards: [bool; 2],
    alpha: i32,
    increment: i32,
    interval: i32,
    tick: Option<u64>,
    job: Option<ImageJob>,
    published: Option<String>,
    stage: bool,
    pub retiring: bool,
}
impl Crossfade {
    pub fn load(h: &mut dyn HostServices, c: ClientId, r: &EffectRequest) -> Result<Self> {
        let mut sources = Vec::new();
        let mut preparing = Vec::new();
        for name in r.resource.split(':').filter(|s| !s.is_empty()) {
            if h.storage(c)?.exists(&asset("bg", super::tone::split(name).1)?) {
                let art = sprite(h, c, &["bg"], name)?;
                if let Some((tone,raw)) = super::tone::path(&art.asset) {
                    preparing.push((sources.len(), super::tone::prepare(h,c,tone,raw)?));
                }
                sources.push(art);
            }
        }
        Ok(Self {
            toned: (0..sources.len()).map(|_| None).collect(),
            preparing,
            sources,
            indices: [0, 1],
            forwards: [true, true],
            alpha: 0,
            increment: r.first,
            interval: r.second,
            tick: None,
            job: None,
            published: None,
            stage: r.kind == "CrossFade2",
            retiring: false,
        })
    }
    fn layer(&self, source: &Sprite, slot: usize, camera: [i32; 2]) -> serde_json::Value {
        let mut art = source.clone();
        if self.stage { art.rect[..2].copy_from_slice(&camera.map(i32::saturating_neg)); }
        let mut op = art.layer(560 + slot as u32, 255);
        op["layer"]["order"] = if self.stage { json!([1, 1]) } else { json!([49, 1 + slot * 2]) };
        op
    }
    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut Published,
        slot: usize,
        camera: [i32; 2],
    ) -> Result<()> {
        if self.sources.len() < 2 {
            return Ok(());
        }
        while let Some((index, job)) = self.preparing.last() {
            let Some(image) = crate::poll(job)? else { return Ok(()); };
            let resource = crysirror_core::image_processing::RgbaImage::new(c,[image.width,image.height],image.rgba.into())?;
            self.toned[*index] = Some(ImageInput::Resource(std::sync::Arc::new(resource)));
            self.preparing.pop();
        }
        let layer = 560 + slot as u32;
        if let Some(job) = &mut self.job {
            let image = match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("minori CrossFade: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            self.job = None;
            let size = image.size();
            let path = h
                .publish_image_resource(c, layer, image)?
                .path
                .relative()
                .to_owned();
            let art = Sprite {
                asset: path.clone(),
                rect: [0, 0, size[0].try_into()?, size[1].try_into()?],
                mask_color: None,
            };
            let op = self.layer(&art, slot, camera);
            if let Err(e) = submit(layers, h, c, vec![op]) {
                h.release_image_resource(c, &path)?;
                return Err(e);
            }
            if let Some(old) = self.published.replace(path) {
                h.release_image_resource(c, &old)?;
            }
        }
        let tick = h.now_ms() / 10;
        if self.tick.is_some_and(|last| {
            tick == last || tick.saturating_sub(last) < self.interval.max(0) as u64
        }) {
            return Ok(());
        }
        self.tick = Some(tick);
        if self.alpha >= 255 {
            let op = self.layer(&self.sources[self.indices[1]], slot, camera);
            submit(layers, h, c, vec![op])?;
            for (i, forward) in self.indices.iter_mut().zip(&mut self.forwards) {
                if *forward && *i + 1 == self.sources.len() {
                    *forward = false;
                } else if !*forward && *i == 0 {
                    *forward = true;
                }
                if *forward { *i += 1 } else { *i -= 1 }
            }
            self.alpha = 0;
        } else {
            let [first, second] = self.indices.map(|i| &self.sources[i]);
            let input = |index:usize, s: &Sprite| self.toned[index].clone().unwrap_or_else(|| ImageInput::Asset {
                path: s.asset.clone(),
                size: [s.rect[2] as u32, s.rect[3] as u32],
                fingerprint: None,
            });
            let source = [
                0,
                0,
                first.rect[2].min(second.rect[2]),
                first.rect[3].min(second.rect[3]),
            ];
            let composite = ImageInput::Composite {
                base: Box::new(input(self.indices[0],first)),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(input(self.indices[1],second)),
                    source,
                    destination: [0, 0],
                    clip: None,
                    opacity: self.alpha as u8,
                    math: ImageCompositeMath::OpaqueScalar256,
                }],
            };
            self.job = Some(h.image_metadata(c)?.begin_process(
                composite,
                serde_json::to_vec(&[ImageEffect::OverrideAlpha { alpha: 255 }])?.into(),
            )?);
        }
        self.alpha = self.alpha.wrapping_add(self.increment);
        Ok(())
    }
    pub fn release(
        self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut Published,
        slot: usize,
    ) -> Result<()> {
        if let Some(name) = self.published {
            submit(layers, h, c, vec![remove(560 + slot as u32)])?;
            h.release_image_resource(c, &name)?;
        }
        Ok(())
    }
}
