// SPDX-License-Identifier: MPL-2.0
//!EF30's433DA0/410530 scalar7-bit inverse sampling and original hide commit.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::fixed_affine::{FixedAffineMath, FixedAffineNearest};
type Resource = Arc<dyn ImageResource>;
pub(super) struct Rotation {
    target: i32,
    pivot: [i32; 2],
    direction: i32,
    start: u64,
    duration: u64,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    source: Option<Resource>,
    spec: Option<FixedAffineNearest>,
    image: Option<UploadedImage>,
    finishing: bool,
}
impl Rotation {
    pub(super) fn preparing(&self) -> bool {
        self.capture.is_some() || self.process.is_some()
    }
    pub(super) fn owns_sprite(&self, id: usize) -> bool {
        self.target == id as i32
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.image
            .as_ref()
            .map(|image| image.path.relative().to_owned())
    }
}
impl SugarPotVm {
    pub(super) fn clear_stage_underlay(&mut self) {
        self.clear_underlay = true;
    }
    pub(super) fn begin_rotation(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
    ) -> Result<bool> {
        let target = self.number(args, 0, 0)?;
        ensure!(
            target == -1 || (target >= 0 && (target as usize) < self.sprites.len()),
            "SugarPot EF30 sprite ID"
        );
        let mut pivot = [self.number(args, 2, 0)?, self.number(args, 3, 0)?];
        if let Some(sprite) = usize::try_from(target)
            .ok()
            .and_then(|id| self.sprites.get(id))
        {
            let width = sprite
                .image
                .as_ref()
                .map_or(0, |image| image.size[0] as i32);
            //448C30 adds the origin displacement here; movement selectors
            //subtract it instead. The pivot is a source-local coordinate.
            pivot[0] = pivot[0].wrapping_add(match sprite.origin {
                1 => width / 2,
                2 => width,
                _ => 0,
            });
        }
        let duration = self.number(args, 5, 1000)?;
        //448530 computes a1000ms default but this branch accidentally passes
        //the raw zero into IDIV. Repair that zero using its initialized default.
        let duration = if duration == 0 { 1000 } else { duration.max(0) } as u64;
        let direction = self.number(args, 4, 0)?;
        for window in &mut self.windows {
            window.visibility = 2;
        }
        self.publish(h, None)?;
        self.rotation = Some(Rotation {
            target,
            pivot,
            direction,
            start: h.now_ms(),
            duration,
            capture: None,
            process: None,
            source: None,
            spec: None,
            image: None,
            finishing: false,
        });
        self.wait = Some(Wait::Rotation);
        Ok(true)
    }
    pub(super) fn rotation_frame(&mut self, h: &mut dyn HostServices, skip: bool) -> Result<bool> {
        let mut work = self
            .rotation
            .take()
            .context("SugarPot EF30 wait without controller")?;
        let result = self.poll_rotation(h, &mut work, skip);
        if !matches!(result, Ok(true)) {
            self.rotation = Some(work);
        }
        result
    }
    fn poll_rotation(
        &mut self,
        h: &mut dyn HostServices,
        work: &mut Rotation,
        skip: bool,
    ) -> Result<bool> {
        let elapsed = h.now_ms().saturating_sub(work.start);
        let finished =
            work.finishing || skip || self.settings.header[4] != 0 || elapsed >= work.duration;
        if finished {
            if !work.finishing {
                if h.now_ms() < self.native_effect_next {
                    return Ok(false);
                }
                self.native_effect_next = h.now_ms().saturating_add(33);
                self.native_effect_index = 0;
            }
            //When target0 disappears,433B60 has no base draw and retains the
            //last initialized native temporary image. Keep that engine-owned
            //immutable image as an unsaved stage underlay, not a black fill.
            if work.target == 0 && work.image.is_none() {
                if !work.finishing {
                    work.process = None;
                    work.capture = Some(h.begin_layer_prefix_image_over(
                        self.client,
                        [1024, 600],
                        [1999, 0],
                        self.clear_color,
                    )?);
                    work.finishing = true;
                    return Ok(false);
                }
                match work
                    .capture
                    .as_mut()
                    .context("SugarPot EF30 final source")?
                    .poll()?
                {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(error) => {
                        bail!("SugarPot EF30 final source: {error}")
                    }
                    ImageProcessState::Ready(resource) => {
                        work.image = Some(h.publish_image_resource(
                            self.client,
                            self.next_image,
                            resource,
                        )?);
                        self.next_image = self
                            .next_image
                            .checked_add(1)
                            .context("SugarPot EF30 image ID")?;
                        work.capture = None;
                    }
                }
            }
            if work.target == -1 {
                //44AC30's ALL branch starts at sprite1 and excludes SYS.
                for sprite in self.sprites.iter_mut().skip(1) {
                    if sprite.system == 0 {
                        sprite.visible = false;
                    }
                }
            } else {
                self.sprites[work.target as usize].visible = false;
            }
            let previous = if work.target == 0 {
                let previous = self.stage_underlay.take();
                self.stage_underlay = work.image.take();
                self.clear_underlay = false;
                previous
            } else {
                None
            };
            self.publish(h, None)?;
            if let Some(previous) = previous {
                h.release_image_resource(self.client, previous.path.relative())?;
            }
            if let Some(image) = work.image.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(true);
        }
        if let Some(job) = &mut work.process {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("SugarPot EF30 fixed affine: {error}"),
                ImageProcessState::Ready(resource) => {
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot EF30 image ID")?;
                    let order = if work.target == 0 {
                        0
                    } else {
                        100 + work.target
                    };
                    self.submit(h,vec![json!({"op":"put","layer":{"id":work.target as u32+1,"rect":[0,0,1024,600],"order":[order,0],
                        "blend":Self::native_image_blend(24,0),"sampling":{"linear":false},"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
                    if let Some(previous) = work.image.replace(image) {
                        h.release_image_resource(self.client, previous.path.relative())?;
                    }
                    work.process = None;
                    return Ok(false);
                }
            }
        }
        if let Some(job) = &mut work.capture {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("SugarPot EF30 lower stage: {error}"),
                ImageProcessState::Ready(base) => {
                    work.capture = None;
                    let spec = work.spec.take().context("SugarPot EF30 captured affine")?;
                    let source = work
                        .source
                        .take()
                        .context("SugarPot EF30 captured source")?;
                    work.process = Some(
                        h.image_metadata(self.client)?.begin_process(
                            ImageInput::Raster {
                                destination: Box::new(ImageInput::Resource(base)),
                                source: Box::new(ImageInput::Resource(source)),
                            },
                            serde_json::to_vec(&vec![
                                crysirror_image::effects::ImageEffect::FixedAffineNearest { spec },
                            ])?
                            .into(),
                        )?,
                    );
                    return Ok(false);
                }
            }
        }
        if h.now_ms() < self.native_effect_next {
            return Ok(false);
        }
        self.native_effect_next = h.now_ms().saturating_add(33);
        let raw_phase = (255u32.wrapping_mul(elapsed as u32) as i32) / work.duration as i32;
        self.native_effect_index = raw_phase;
        let Some(sprite) = usize::try_from(work.target)
            .ok()
            .and_then(|id| self.sprites.get(id))
            .filter(|sprite| sprite.visible && sprite.system == 0)
        else {
            return Ok(false);
        };
        let image = sprite
            .image
            .as_ref()
            .context("SugarPot EF30 active sprite image")?;
        ensure!(
            matches!(sprite.native_depth, 24 | 32),
            "SugarPot EF30 original bitmap depth"
        );
        let phase = raw_phase.clamp(1, 255);
        //Unknown direction values read an uninitialized native stack float.
        //Portable policy initializes them to the native direction0 profile.
        let angle = (f64::from(phase)
            * if work.direction == 1 {
                0.7058823704719543
            } else {
                -0.7058823704719543
            }) as f32;
        let radians = f64::from(angle) * 6.283185307 / 360.;
        let sine = (radians.sin() * 128.) as i32;
        let cosine = (radians.cos() * 128.) as i32;
        let [px, py] = work.pivot.map(i64::from);
        let [sin, cos] = [sine, cosine].map(i64::from);
        let spec = FixedAffineNearest {
            //410530's source extent caps against the destination surface;
            //bounded clipping repairs its negative/out-of-row raw pointer writes.
            rectangle: [
                sprite.position[0],
                sprite.position[1],
                image.size[0].min(1024) as i32,
                image.size[1].min(600) as i32,
            ],
            origin: sprite.position,
            linear: [[cosine, sine], [-sine, cosine]],
            translation: [
                px * 128 - py * sin - px * cos,
                py * 128 - py * cos + px * sin,
            ],
            fractional_bits: 7,
            math: if sprite.native_depth == 24 {
                FixedAffineMath::CopyRgb
            } else {
                FixedAffineMath::MaskHoldAlphaScalar256
            },
        };
        spec.validate()?;
        work.source = Some(
            h.published_image_resource(self.client, image.path.relative())?
                .context("SugarPot EF30 source not resident")?,
        );
        work.spec = Some(spec);
        let order = if work.target == 0 {
            0
        } else {
            100 + work.target
        };
        work.capture = Some(h.begin_layer_prefix_image_over(
            self.client,
            [1024, 600],
            [order - 1, i32::MAX],
            self.clear_color,
        )?);
        Ok(false)
    }
}
