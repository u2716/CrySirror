// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Presentation, Result, Sprite, remove, submit};
use crate::vm::{Rotate, RotationSpec, Vm};
use anyhow::bail;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
};
use crysirror_image::{
    effects::ImageEffect,
    fixed_affine::{FixedAffineMath, FixedAffineNearest},
};

#[derive(Default)]
pub(super) struct Plane {
    playing: Option<(RotationSpec, u64)>,
    still: Option<RotationSpec>,
    blocking: bool,
    last_tick: Option<u32>,
    finish: bool,
    job: Option<ImageJob>,
    published: Option<String>,
}

impl Plane {
    fn sample(&mut self, now: u64, skip: bool) -> Option<(RotationSpec, f64)> {
        if let Some(s) = self.still.take() {
            return Some((s, f64::from(s.first).to_radians()));
        }
        let (s, start) = self.playing?;
        let tick = (now / 10).wrapping_sub(start) as u32;
        if self.finish || skip || tick >= s.duration as u32 {
            self.playing = None;
            self.finish = false;
            return Some((s, f64::from(s.last).to_radians()));
        }
        if self.last_tick == Some(tick) {
            return None;
        }
        self.last_tick = Some(tick);
        let delta = ((f64::from(s.last - s.first).to_radians()) / f64::from(s.duration)).abs();
        Some((
            s,
            f64::from(s.first).to_radians() + delta * f64::from(s.direction) * f64::from(tick),
        ))
    }
}

fn input(
    h: &mut dyn HostServices,
    c: ClientId,
    art: &Sprite,
    filtered: &std::collections::BTreeMap<String, super::StageImage>,
) -> Result<ImageInput> {
    let size = [art.rect[2].try_into()?, art.rect[3].try_into()?];
    if let Some(resource) = filtered.get(&art.asset) {
        return Ok(ImageInput::Resource(resource.clone()));
    }
    if let Some(resource) = h.published_image_resource(c, &art.asset)? {
        Ok(ImageInput::Resource(resource))
    } else {
        Ok(ImageInput::Asset {
            path: art.asset.clone(),
            size,
            fingerprint: None,
        })
    }
}

impl Presentation {
    pub fn rotation_active(&self, foreground: bool) -> bool {
        let plane = &self.rotations[usize::from(foreground)];
        plane.playing.is_some() || plane.job.is_some() || plane.still.is_some()
    }

    pub(super) fn rotate(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        vm: &mut Vm,
        foreground: bool,
        action: Rotate,
    ) -> Result<()> {
        let plane = &mut self.rotations[usize::from(foreground)];
        match action {
            Rotate::Set(spec) => {
                plane.still = Some(spec);
                plane.blocking = true;
            }
            Rotate::Start(spec) => {
                plane.playing = Some((spec, h.now_ms() / 10));
                plane.last_tick = None;
                plane.finish = false;
            }
            Rotate::End => plane.finish = plane.playing.is_some(),
            Rotate::Wait => {
                if self.rotation_active(foreground) {
                    vm.wait_rotation(foreground);
                }
                return Ok(());
            }
        }
        if matches!(action, Rotate::Set(_)) {
            self.poll_rotation(h, c, foreground, false)?;
        }
        Ok(())
    }

    pub fn rotation_pending(&self) -> bool {
        self.rotations.iter().any(|p| p.blocking)
    }

    pub(super) fn release_rotations(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<()> {
        for plane in &mut self.rotations {
            plane.job = None;
            plane.blocking = false;
            if let Some(path) = plane.published.take() {
                h.release_image_resource(c, &path)?;
            }
        }
        Ok(())
    }

    pub fn poll_rotation(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        fg: bool,
        skip: bool,
    ) -> Result<()> {
        let layer = if fg { self.world.len() as u32 + 1 } else { 1 };
        let index = usize::from(fg);
        let plane = &mut self.rotations[index];
        if let Some(job) = &mut plane.job {
            let image = match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("minori rotation: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            plane.job = None;
            let output = h.publish_image_resource(c, 530 + index as u32, image)?;
            let path = output.path.relative().to_owned();
            let art = Sprite {
                asset: path.clone(),
                rect: [0, 0, self.canvas[0], self.canvas[1]],
                mask_color: None,
            };
            let mut ops = vec![art.layer(layer, 255)];
            if !fg {
                ops.extend((2..=self.world.len() as u32).map(remove));
            }
            if let Err(e) = submit(&mut self.published, h, c, ops) {
                h.release_image_resource(c, &path)?;
                return Err(e);
            }
            if let Some(old) = plane.published.replace(path) {
                h.release_image_resource(c, &old)?;
            }
            plane.blocking &= plane.still.is_some();
            return Ok(());
        }
        if self.scroll.is_some() && plane.still.is_none() {
            return Ok(());
        }
        let Some((spec, angle)) = plane.sample(h.now_ms(), skip) else {
            return Ok(());
        };
        let art = if fg {
            self.foreground.as_ref()
        } else {
            self.world.first()
        };
        let Some(art) = art else {
            plane.blocking = false;
            return Ok(());
        };
        let mut source = input(h, c, art, &self.filtered_inputs)?;
        if !fg && self.world.len() > 1 {
            let mut layers = Vec::new();
            for art in &self.world[1..] {
                layers.push(ImageCompositeLayer {
                    input: Box::new(input(h, c, art, &self.filtered_inputs)?),
                    source: [0, 0, art.rect[2], art.rect[3]],
                    destination: [art.rect[0], art.rect[1]],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::AlphaOnAlphaScalar256,
                });
            }
            source = ImageInput::Composite {
                base: Box::new(source),
                layers,
            };
        }
        let canvas = self.canvas;
        let cos = (1024. * angle.cos()) as i32;
        let sin = (1024. * angle.sin()) as i32;
        let transform = FixedAffineNearest {
            rectangle: [0, 0, canvas[0], canvas[1]],
            origin: spec.destination,
            linear: [[cos, -sin], [sin, cos]],
            translation: spec.source.map(|p| i64::from(p) << 10),
            fractional_bits: 10,
            math: FixedAffineMath::Copy,
        };
        let destination = ImageInput::Solid {
            size: canvas.map(|v| v as u32),
            color: [0, 0, 0, if fg { 0 } else { 255 }],
        };
        // The image worker accepts a flat binary input. Prepare a multi-layer source first.
        let metadata = h.image_metadata(c)?;
        if matches!(source, ImageInput::Composite { .. }) {
            let capture = metadata.begin_process(source, std::sync::Arc::from(b"[]".as_slice()))?;
            plane.job = Some(Box::new(ComposedRotation {
                metadata,
                capture,
                render: None,
                destination: Some(destination),
                effects: serde_json::to_vec(&[ImageEffect::FixedAffineNearest {
                    spec: transform,
                }])?
                .into(),
            }));
        } else {
            plane.job = Some(metadata.begin_process(
                ImageInput::Raster {
                    destination: Box::new(destination),
                    source: Box::new(source),
                },
                serde_json::to_vec(&[ImageEffect::FixedAffineNearest { spec: transform }])?.into(),
            )?);
        }
        Ok(())
    }
}
struct ComposedRotation {
    metadata: std::sync::Arc<dyn crysirror_core::service::ImageMetadata>,
    capture: ImageJob,
    render: Option<ImageJob>,
    destination: Option<ImageInput>,
    effects: std::sync::Arc<[u8]>,
}
impl crysirror_core::image_processing::ImageProcess for ComposedRotation {
    fn poll(&mut self) -> crysirror_core::Result<ImageProcessState> {
        if let Some(job) = &mut self.render {
            return job.poll();
        }
        let image = match self.capture.poll()? {
            ImageProcessState::Ready(s) => s,
            state => return Ok(state),
        };
        let destination = self
            .destination
            .take()
            .ok_or(crysirror_core::Error::Invalid("rotation destination"))?;
        self.render = Some(self.metadata.begin_process(
            ImageInput::Raster {
                destination: Box::new(destination),
                source: Box::new(ImageInput::Resource(image)),
            },
            self.effects.clone(),
        )?);
        Ok(ImageProcessState::Pending)
    }
}
