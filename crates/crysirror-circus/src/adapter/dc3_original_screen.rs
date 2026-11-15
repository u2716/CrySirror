//! D.C.III (0x8a71) process-state-14 frames over shared services.
//!
//! The presented screen (original 4E89D0) is frozen as an engine Canvas copy
//! of the scene root at the start of the frame whose script changes it; the
//! live scene is the next screen (486DE0). Crossfades, solid fades and shakes
//! present that Canvas under an isolated overlay group. Lookup, rule and tile
//! frames read the Canvas back once and run on the shared image workers
//! (linear_alpha_lookup, rule-table transitions, stored-byte composites).
//! No VM-private pixel loop exists here; the VM owns every parameter.
use super::*;
use crate::machine::effect_ops::OriginalScreen;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};

const CANVAS: u32 = 0x3c03_0000;
const PRESENTED: u32 = 0x1c03_0000;
const ROOT: u32 = 0x3b02_0000;
const SOLID: u32 = 0x3b02_0001;
const GROUP: u32 = 0x3b02_0002;
const IMAGE: u32 = 0x3b02_0003;
const PUBLICATION: u32 = 0x3c03_0010;

#[derive(Default)]
pub(super) struct Screen {
    ///Frames presented by the DC3 session owner.
    pub(super) presented_epoch: u64,
    ///Epoch whose start the frozen Canvas copy was taken at.
    frozen_epoch: Option<u64>,
    ///Epoch whose start already ran the lookahead.
    pub(super) lookahead_epoch: Option<u64>,
    created: bool,
    overlay: bool,
    capture: Option<ImageJob>,
    frozen: Option<Arc<dyn ImageResource>>,
    ///407A80's snapshot after the cracks drawn so far.
    cracked: Option<Arc<dyn ImageResource>>,
    ///Request in flight and its job; tile frames have two stages.
    job: Option<(OriginalScreen, u8, ImageJob)>,
    shown: Option<UploadedImage>,
    toggle: bool,
    ///Slot-16 mask source path, its crop job and the cropped image.
    mask: Option<(String, Option<ImageJob>, Option<Arc<dyn ImageResource>>)>,
}

impl Bridge<'_> {
    pub(super) fn original_screen(&mut self, request: &OriginalScreen) -> Result<bool> {
        let [width, height] = self.native_canvas();
        match request {
            OriginalScreen::Freeze => {
                self.freeze_original_screen()?;
                Ok(true)
            }
            OriginalScreen::Hold => {
                let screen = &self.services.dc3_original;
                if screen.frozen_epoch != Some(screen.presented_epoch) {
                    self.freeze_original_screen()?;
                }
                Ok(true)
            }
            OriginalScreen::Clear => {
                self.clear_original_screen()?;
                Ok(true)
            }
            OriginalScreen::Frozen { opacity } => {
                self.original_overlay(None, Some((*opacity, [0, 0])), false)?;
                Ok(true)
            }
            OriginalScreen::Solid { color, opacity } => {
                self.original_overlay(Some((*color, *opacity)), None, false)?;
                Ok(true)
            }
            OriginalScreen::Shifted { offset, background } => {
                self.original_overlay(Some((*background, 255)), Some((255, *offset)), false)?;
                Ok(true)
            }
            OriginalScreen::Lookup { .. } | OriginalScreen::Rule { .. } | OriginalScreen::Tiles { .. } => {
                let Some(frozen) = self.original_frozen_image()? else { return Ok(false) };
                if let OriginalScreen::Rule { slot, .. } = request {
                    if !self.original_rule_mask(*slot)? {
                        return Ok(false);
                    }
                }
                ensure!(frozen.size() == [width, height], "D.C.III frozen screen extent");
                let Some(image) = self.original_job(request, frozen)? else { return Ok(false) };
                let slot = PUBLICATION + u32::from(self.services.dc3_original.toggle);
                self.services.dc3_original.toggle = !self.services.dc3_original.toggle;
                let upload = self.host.publish_image_resource(self.client, slot, image)?;
                let path = upload.path.relative().to_owned();
                self.original_overlay(None, None, true)?;
                self.submit(
                    vec![json!({"op":"put","layer":{"id":IMAGE,"parent":ROOT,"rect":[0,0,width,height],"paint":{"kind":"image","asset":path},"filter":{"encoded_sampling":true},"order":[0,2]}})],
                    vec![],
                )?;
                if let Some(old) = self.services.dc3_original.shown.replace(upload) {
                    self.host.release_image_resource(self.client, old.path.relative())?;
                }
                Ok(true)
            }
        }
    }

    ///Clear the Canvas and copy the scene root into it, as presented now.
    fn freeze_original_screen(&mut self) -> Result<()> {
        self.root()?;
        let [width, height] = self.native_canvas();
        let mut canvases = Vec::new();
        if !self.services.dc3_original.created {
            canvases.push(json!({"op":"create","id":CANVAS,"width":width,"height":height}));
        }
        canvases.extend([
            json!({"op":"clear","id":CANVAS,"color":[0,0,0,255]}),
            json!({"op":"copy_subtree","source":SCENE_ROOT,"destination":CANVAS,"rect":[0,0,width,height],"position":[0,0]}),
        ]);
        self.host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"canvases":canvases}))?)?;
        let screen = &mut self.services.dc3_original;
        screen.created = true;
        screen.frozen_epoch = Some(screen.presented_epoch);
        screen.capture = None;
        screen.frozen = None;
        screen.cracked = None;
        Ok(())
    }

    ///Overlay group: optional solid and frozen Canvas presentation at an
    ///opacity/offset. Non-image frames drop the previous image frame.
    fn original_overlay(
        &mut self,
        solid: Option<([u8; 3], u8)>,
        frozen: Option<(u8, [i32; 2])>,
        image: bool,
    ) -> Result<()> {
        let [width, height] = self.native_canvas();
        ensure!(self.services.dc3_original.created, "D.C.III effect frame without a frozen screen");
        let (color, opacity) = solid.unwrap_or(([0; 3], 0));
        let (group_opacity, offset) = frozen.unwrap_or((0, [0, 0]));
        let mut layers = vec![
            json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,width,height],"paint":{"kind":"group"},"order":[3,1]}}),
            json!({"op":"put","layer":{"id":SOLID,"parent":ROOT,"rect":[0,0,width,height],"visible":solid.is_some(),"paint":{"kind":"solid","color":[color[0],color[1],color[2],255]},"opacity":opacity,"order":[0,0]}}),
            json!({"op":"put","layer":{"id":GROUP,"parent":ROOT,"rect":[0,0,width,height],"visible":frozen.is_some(),"paint":{"kind":"group"},"opacity":group_opacity,"order":[0,1]}}),
        ];
        let released = if image { None } else { self.services.dc3_original.shown.take() };
        if released.is_some() {
            layers.push(json!({"op":"remove","id":IMAGE}));
        }
        self.submit(layers, vec![])?;
        if let Some(old) = released {
            self.host.release_image_resource(self.client, old.path.relative())?;
        }
        self.services.dc3_original.overlay = true;
        if frozen.is_some() {
            self.host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"canvases":[{"op":"present_isolated","id":CANVAS,"layer":PRESENTED,"order":0,"parent":GROUP,"position":[offset[0],offset[1]]}]}))?,
            )?;
        }
        Ok(())
    }

    fn clear_original_screen(&mut self) -> Result<()> {
        if self.services.dc3_original.overlay {
            self.submit(vec![json!({"op":"remove","id":ROOT})], vec![])?;
        }
        let screen = &mut self.services.dc3_original;
        screen.overlay = false;
        screen.frozen_epoch = None;
        screen.capture = None;
        screen.frozen = None;
        screen.cracked = None;
        screen.job = None;
        screen.mask = None;
        if let Some(old) = screen.shown.take() {
            self.host.release_image_resource(self.client, old.path.relative())?;
        }
        Ok(())
    }

    ///One read-back of the frozen Canvas on the shared Canvas image worker.
    fn original_frozen_image(&mut self) -> Result<Option<Arc<dyn ImageResource>>> {
        if let Some(image) = &self.services.dc3_original.frozen {
            return Ok(Some(image.clone()));
        }
        ensure!(self.services.dc3_original.created, "D.C.III image frame without a frozen screen");
        if self.services.dc3_original.capture.is_none() {
            self.services.dc3_original.capture = Some(self.host.begin_canvas_image(self.client, CANVAS)?);
        }
        match self.services.dc3_original.capture.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => bail!("D.C.III frozen screen read-back failed: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(image.client() == self.client, "D.C.III foreign frozen screen");
                self.services.dc3_original.capture = None;
                self.services.dc3_original.frozen = Some(image.clone());
                Ok(Some(image))
            }
        }
    }

    ///407F20 clamps the mask to the 1024x576 screen and reads its rows with
    ///the mask's own stride: the top-left screen-sized region of a larger
    ///MSK image (1024x1024) is used. A smaller mask leaves black outside it
    ///in the original and fails visibly here.
    fn original_rule_mask(&mut self, slot: u8) -> Result<bool> {
        let [width, height] = self.native_canvas();
        let path = self.services.images.get(&slot).with_context(|| format!("D.C.III rule mask slot {slot}"))?.path.relative().to_owned();
        if self.services.dc3_original.mask.as_ref().is_some_and(|m| m.0 == path && m.2.is_some()) {
            return Ok(true);
        }
        if !self.services.dc3_original.mask.as_ref().is_some_and(|m| m.0 == path) {
            let mask = self.original_slot_image(slot)?;
            let size = mask.size();
            ensure!(size[0] >= width && size[1] >= height, "D.C.III rule mask extent {size:?} is smaller than the screen");
            let input = ImageInput::Composite {
                base: Box::new(ImageInput::Solid { size: [width, height], color: [0, 0, 0, 255] }),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(mask)),
                    source: [0, 0, width as i32, height as i32],
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Copy,
                }],
            };
            let job = self.host.image_metadata(self.client)?.begin_process(input, Arc::from(&b"[]"[..]))?;
            self.services.dc3_original.mask = Some((path, Some(job), None));
        }
        let mask = self.services.dc3_original.mask.as_mut().unwrap();
        match mask.1.as_mut().context("D.C.III rule mask job")?.poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => bail!("D.C.III rule mask crop failed: {error}"),
            ImageProcessState::Ready(image) => {
                mask.1 = None;
                mask.2 = Some(image);
                Ok(true)
            }
        }
    }

    fn original_slot_image(&mut self, slot: u8) -> Result<Arc<dyn ImageResource>> {
        let upload = self.services.images.get(&slot).with_context(|| format!("D.C.III effect image slot {slot}"))?;
        self.host
            .published_image_resource(self.client, upload.path.relative())?
            .with_context(|| format!("D.C.III effect image slot {slot} lease"))
    }

    ///Start or poll the worker job of an image frame; Some when complete.
    fn original_job(
        &mut self,
        request: &OriginalScreen,
        frozen: Arc<dyn ImageResource>,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if let Some((owner, _, _)) = &self.services.dc3_original.job {
            ensure!(owner == request, "D.C.III effect frame job owner");
        } else {
            let (stage, job) = self.begin_original_job(request, frozen.clone(), 0)?;
            self.services.dc3_original.job = Some((request.clone(), stage, job));
        }
        loop {
            let (_, stage, job) = self.services.dc3_original.job.as_mut().unwrap();
            let image = match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(error) => bail!("D.C.III effect frame failed: {error}"),
                ImageProcessState::Ready(image) => image,
            };
            let stage = *stage;
            if matches!(request, OriginalScreen::Tiles { .. }) && stage == 0 {
                //407A80's cracks persist in the snapshot.
                self.services.dc3_original.cracked = Some(image);
                let (next, job) = self.begin_original_job(request, frozen.clone(), 1)?;
                self.services.dc3_original.job = Some((request.clone(), next, job));
                continue;
            }
            self.services.dc3_original.job = None;
            return Ok(Some(image));
        }
    }

    fn begin_original_job(
        &mut self,
        request: &OriginalScreen,
        frozen: Arc<dyn ImageResource>,
        stage: u8,
    ) -> Result<(u8, ImageJob)> {
        let [width, height] = self.native_canvas();
        let (input, effects) = match request {
            OriginalScreen::Lookup { table, offset, step } => (
                ImageInput::Resource(frozen),
                json!([{"kind":"linear_alpha_lookup","spec":{"table":table,"offset":offset,"step":step,"outside":0,"complement":true}}]),
            ),
            OriginalScreen::Rule { weights, .. } => {
                let mask = self.services.dc3_original.mask.as_ref().and_then(|m| m.2.clone()).context("D.C.III rule mask")?;
                (
                    ImageInput::Transition { from: frozen.clone(), to: frozen, mask: Some(mask) },
                    //407F20 reads the first stored byte (blue) of the mask.
                    json!([{"kind":"transition","spec":{"progress":0.,"softness":0.,"blur":0.,"rule_table":{"weights":weights,"channel":2,"operation":"alpha_scale"}}}]),
                )
            }
            OriginalScreen::Tiles { overlay, cracks, tiles, fill } => {
                use crysirror_core::image_blend::{
                    RgbaBlendArithmetic as A, RgbaBlendComponent, RgbaBlendFactor as F, RgbaBlendState,
                    RgbaByteRounding as R,
                };
                let source = self.services.dc3_original.cracked.clone().unwrap_or(frozen);
                if stage == 0 {
                    let strip = self.original_slot_image(*overlay)?;
                    let strip_width = i32::try_from(strip.size()[0])?;
                    //4167C0: copy only the FF00FF strip texels.
                    let math = RgbaBlendState {
                        arithmetic: A::Byte255 { source: R::Floor, destination: R::Floor },
                        write_mask: [true, true, true, false],
                        source_color_key: Some([255, 0, 255]),
                        source_color_key_invert: true,
                        ..Default::default()
                    };
                    let layers = cracks
                        .iter()
                        .filter(|(_, frame)| *frame < strip_width)
                        .map(|(origin, frame)| ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(strip.clone())),
                            source: [*frame, 0, 32.min(strip_width - *frame), 32],
                            destination: *origin,
                            clip: Some([origin[0], origin[1], 32, 32]),
                            opacity: 255,
                            math: ImageCompositeMath::Normalized(math.clone()),
                        })
                        .collect();
                    (ImageInput::Composite { base: Box::new(ImageInput::Resource(source)), layers }, json!([]))
                } else {
                    //4125F0: (a*s + (16-a)*d) >> 4, FF00FF texels skipped.
                    let layers = tiles
                        .iter()
                        .filter(|(_, _, alpha)| *alpha > 0)
                        .map(|(origin, destination, alpha)| {
                            let k = f32::from(*alpha) / 16.;
                            ImageCompositeLayer {
                                input: Box::new(ImageInput::Resource(source.clone())),
                                source: [origin[0], origin[1], 32, 32],
                                destination: *destination,
                                clip: Some([0, 0, width as i32, height as i32]),
                                opacity: 255,
                                math: ImageCompositeMath::Normalized(RgbaBlendState {
                                    arithmetic: A::ByteShiftSum { shift: 4 },
                                    color: RgbaBlendComponent {
                                        source: F::ConstantAlpha,
                                        destination: F::OneMinusConstantAlpha,
                                        ..Default::default()
                                    },
                                    alpha: RgbaBlendComponent { source: F::Zero, destination: F::One, ..Default::default() },
                                    constant: [k; 4],
                                    write_mask: [true, true, true, false],
                                    source_color_key: Some([255, 0, 255]),
                                    ..Default::default()
                                }),
                            }
                        })
                        .collect();
                    (
                        ImageInput::Composite {
                            base: Box::new(ImageInput::Solid { size: [width, height], color: [fill[0], fill[1], fill[2], 255] }),
                            layers,
                        },
                        json!([]),
                    )
                }
            }
            _ => bail!("D.C.III effect frame has no image job"),
        };
        let job = self
            .host
            .image_metadata(self.client)?
            .begin_process(input, Arc::from(serde_json::to_vec(&effects)?))?;
        Ok((stage, job))
    }
}
