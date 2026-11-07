// SPDX-License-Identifier: MPL-2.0
//! Native43F580 draw ordering expressed as reusable engine image
//! jobs. Recipes retain immutable scoped resources, never GPU pointers or a
//! VM-owned raster surface. Batches bound graph nodes independently of sprites.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use std::{collections::VecDeque, ops::Range};
type Resource = Arc<dyn ImageResource>;
enum Part {
    Images(Vec<ImageCompositeLayer>),
    Effects(Vec<crysirror_image::effects::ImageEffect>),
    Checkpoint,
}
pub(super) struct StagePlan {
    input: Option<ImageInput>,
    parts: VecDeque<Part>,
    job: Option<ImageJob>,
    output: Option<Resource>,
    checkpoints: Vec<Resource>,
}
impl StagePlan {
    pub(super) fn new(input: ImageInput) -> Self {
        Self {
            input: Some(input),
            parts: VecDeque::new(),
            job: None,
            output: None,
            checkpoints: vec![],
        }
    }
    pub(super) fn ready_resource(&self) -> Option<Resource> {
        self.output.clone()
    }
    pub(super) fn checkpoint(&mut self) {
        self.parts.push_back(Part::Checkpoint);
    }
    pub(super) fn checkpoint_resource(&self, index: usize) -> Option<Resource> {
        self.checkpoints.get(index).cloned()
    }
    pub(super) fn poll(&mut self, h: &mut dyn HostServices, client: ClientId) -> Result<bool> {
        if let Some(job) = &mut self.job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("SugarPot native stage worker: {error}"),
                ImageProcessState::Ready(resource) => {
                    self.input = Some(ImageInput::Resource(resource.clone()));
                    self.output = Some(resource);
                    self.job = None;
                }
            }
        }
        if let Some(part) = self.parts.pop_front() {
            if matches!(part, Part::Checkpoint) {
                if self.output.is_none() {
                    self.parts.push_front(Part::Checkpoint);
                    self.job = Some(
                        h.image_metadata(client)?.begin_process(
                            self.input
                                .take()
                                .context("SugarPot checkpoint base missing")?,
                            serde_json::to_vec(&vec![
                                crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                            ])?
                            .into(),
                        )?,
                    );
                    return Ok(false);
                }
                let resource = self
                    .output
                    .clone()
                    .context("SugarPot stage checkpoint before image completion")?;
                self.checkpoints.push(resource);
                return Ok(false);
            }
            let input = self
                .input
                .take()
                .context("SugarPot stage dependency consumed")?;
            self.output = None;
            self.job = Some(match part {
                Part::Images(layers) => h.image_metadata(client)?.begin_process(
                    ImageInput::Composite {
                        base: Box::new(input),
                        layers,
                    },
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
                Part::Effects(effects) => h
                    .image_metadata(client)?
                    .begin_process(input, serde_json::to_vec(&effects)?.into())?,
                Part::Checkpoint => unreachable!("handled above"),
            });
            return Ok(false);
        }
        if self.output.is_none() {
            self.job = Some(
                h.image_metadata(client)?.begin_process(
                    self.input.take().context("SugarPot empty stage base")?,
                    serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                    ])?
                    .into(),
                )?,
            );
            return Ok(false);
        }
        Ok(true)
    }
    pub(super) fn effects(&mut self, effects: Vec<crysirror_image::effects::ImageEffect>) {
        self.parts.push_back(Part::Effects(effects));
    }
    pub(super) fn images(&mut self, layers: Vec<ImageCompositeLayer>) {
        let mut batch = Vec::new();
        for layer in layers {
            batch.push(layer);
            //A silhouette adds a nested Process node;64 records fit the
            //engine's256-node graph bound even when every input is transformed.
            if batch.len() == 64 {
                self.parts
                    .push_back(Part::Images(std::mem::take(&mut batch)));
            }
        }
        if !batch.is_empty() {
            self.parts.push_back(Part::Images(batch));
        }
    }
}
impl SugarPotVm {
    fn stage_sprite(
        &self,
        h: &mut dyn HostServices,
        sprite: &Sprite,
        background: bool,
    ) -> Result<Option<ImageCompositeLayer>> {
        if !sprite.visible {
            return Ok(None);
        }
        let Some(image) = &sprite.image else {
            return Ok(None);
        };
        ensure!(
            matches!(sprite.native_depth, 24 | 32),
            "SugarPot native stage source bitmap depth"
        );
        ensure!(
            sprite.divisions > 0
                && image.size[1] % sprite.divisions == 0
                && sprite.cell < sprite.divisions,
            "SugarPot native stage source cells"
        );
        let resource = h
            .published_image_resource(self.client, image.path.relative())?
            .context("SugarPot stage source outside prepared scope")?;
        let mut input = ImageInput::Resource(resource);
        let attribute = if sprite.native_depth == 24 && !background {
            0
        } else {
            sprite.attribute as u8
        };
        let mut opacity = if sprite.system == 2 {
            255 * self.settings.values[10].clamp(0, 100) / 100
        } else {
            255
        };
        opacity = Self::sprite_opacity(sprite, background, opacity);
        let math = if attribute == 1 {
            ImageCompositeMath::IntegerStored {
                destination_format: 1,
                source_format: 1,
                mode: 2,
                parameter: 256,
            }
        } else if sprite.native_depth == 32 {
            if matches!(attribute, 2 | 3) {
                input = ImageInput::Process {
                    input: Box::new(input),
                    effects: serde_json::to_vec(&vec![
                        crysirror_image::effects::ImageEffect::IntegerColorMatrix {
                            spec: crysirror_image::integer_color::ColorMatrixFixed {
                                matrix: [[0; 3]; 3],
                                offset: [if attribute == 3 { 255 } else { 0 }; 3],
                                divisor: 1,
                            },
                        },
                    ])?
                    .into(),
                };
            }
            ImageCompositeMath::MaskHoldAlphaScalar256
        } else if opacity == 255 {
            ImageCompositeMath::CopyRgb
        } else {
            ImageCompositeMath::OpaqueScalar256
        };
        let height = image.size[1] / sprite.divisions;
        Ok(Some(ImageCompositeLayer {
            input: Box::new(input),
            source: [
                0,
                (sprite.cell * height) as i32,
                image.size[0] as i32,
                height as i32,
            ],
            destination: sprite.position,
            clip: None,
            opacity: opacity as u8,
            math,
        }))
    }
    pub(super) fn plan_stage_sprites(
        &self,
        h: &mut dyn HostServices,
        base: ImageInput,
        range: Range<usize>,
        system: bool,
    ) -> Result<StagePlan> {
        ensure!(
            range.start <= range.end && range.end <= self.sprites.len(),
            "SugarPot native stage sprite range"
        );
        let mut plan = StagePlan::new(base);
        let mut layers = Vec::new();
        for index in range {
            let sprite = &self.sprites[index];
            if index != 0 && !system && sprite.system != 0 {
                continue;
            }
            if let Some(layer) = self.stage_sprite(h, sprite, index == 0)? {
                layers.push(layer);
            }
        }
        plan.images(layers);
        Ok(plan)
    }
    ///4344C0 composes native index order, not the ordinary SYS presentation
    ///z-order. A whole-stage warp stops at the first active exact SYS1/2;
    ///subsequent sprites are composed after the map, including SYS3.
    pub(super) fn plan_stage_warp(
        &self,
        h: &mut dyn HostServices,
        target: i32,
        offset: [i32; 2],
    ) -> Result<StagePlan> {
        use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
        let mut plan = StagePlan::new(ImageInput::Solid {
            size: [1024, 600],
            color: [0, 0, 0, 255],
        });
        let boundary = if target == -1 {
            self.sprites
                .iter()
                .position(|sprite| sprite.visible && matches!(sprite.system, 1 | 2))
                .unwrap_or(self.sprites.len())
        } else {
            self.sprites.len()
        };
        let mut layers = Vec::new();
        for (index, sprite) in self.sprites[..boundary].iter().enumerate() {
            let mut sprite = sprite.clone();
            if target == index as i32 {
                sprite.position =
                    std::array::from_fn(|axis| sprite.position[axis].wrapping_add(offset[axis]));
            }
            if let Some(layer) = self.stage_sprite(h, &sprite, index == 0)? {
                layers.push(layer);
            }
        }
        plan.images(layers);
        if target == -1 {
            //40FCE0 builds its original nearest table after clipping the source
            //by twice the signed displacement. Native negative extents would
            //read outside CBuffer; define that undefined case as an empty map.
            let inset = offset.map(|value| i64::from(value).abs() * 2);
            let source = [
                if offset[0] < 0 { 0 } else { inset[0] },
                if offset[1] < 0 { 0 } else { inset[1] },
                1024 - inset[0],
                600 - inset[1],
            ];
            let regions = if source[2] > 0 && source[3] > 0 {
                vec![RectMapRegion {
                    source: source.map(|v| v as i32),
                    destination: [0, 0, 1024, 600],
                }]
            } else {
                vec![]
            };
            plan.effects(vec![crysirror_image::effects::ImageEffect::RectMap {
                spec: RectMap {
                    output_size: None,
                    regions,
                    clear: [0, 0, 0, 255],
                    sample: RectMapSample::NearestRoundLinear,
                    copy_alpha: false,
                },
            }]);
            let mut layers = Vec::new();
            for (index, sprite) in self.sprites.iter().enumerate().skip(boundary) {
                if let Some(layer) = self.stage_sprite(h, sprite, index == 0)? {
                    layers.push(layer);
                }
            }
            plan.images(layers);
        }
        Ok(plan)
    }
}
