//! DC2 screen-buffer lifecycle over client-scoped engine Canvas/Image jobs.
//! No raster math or legacy software framebuffer lives in the adapter.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
#[path = "dc2_composite_records.rs"]
mod records;
#[path = "dc2_darken.rs"]
mod darken;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};
///424CA0 per-pixel weights as stored alpha, built for one 40EAB0 step.
#[derive(Debug)]
struct WipeMask {
    client: ClientId,
    rgba: Arc<[u8]>,
}
impl ImageResource for WipeMask {
    fn client(&self) -> ClientId {
        self.client
    }
    fn size(&self) -> [u32; 2] {
        crate::dc2::CANVAS
    }
    fn rgba(&self) -> Arc<[u8]> {
        self.rgba.clone()
    }
    fn opaque(&self) -> bool {
        false
    }
    fn pixel_at(&self, point: [u32; 2]) -> crysirror_core::Result<Option<[u8; 4]>> {
        crysirror_core::image_processing::rgba_pixel_at(crate::dc2::CANVAS, &self.rgba, point)
    }
}
const CANVAS_ID: u32 = 0x3c02_0000;
const SCREEN_ID: u32 = 0x3c02_0001;
const SCREEN_LAYER: u32 = 0x3c02_0002;
#[derive(Default)]
pub(super) struct Bitmap {
    owner: Option<Owner>,
    created: bool,
    bound: Option<UploadedImage>,
    job: Option<ImageJob>,
    pending: Option<Request>,
    current: Option<Arc<dyn ImageResource>>,
    //403CB0 retains the original screen throughout a case2 dimmer; feedback
    //results must never replace this native generated-bitmap source.
    original: Option<Arc<dyn ImageResource>>,
    published: Option<UploadedImage>,
    target: Option<Arc<dyn ImageResource>>,
    target_source: Option<String>,
    ///4188A0 "scrbrk": the captured screen with accumulated crack punches.
    shatter_source: Option<Arc<dyn ImageResource>>,
    ///True while the crack job of the pending screen-break frame runs.
    shatter_cracking: bool,
    /// Slot -> (published source key, native ARGB32 alpha-blit layout).
    pub(super) validated: BTreeMap<u8, (String, bool)>,
    validation: Option<(
        Request,
        Vec<(u8, String, String)>,
        Receiver<std::result::Result<Vec<bool>, String>>,
    )>,
}
impl Bridge<'_> {
    pub(in crate::adapter) fn discard_dc2_scene_bitmap(&mut self) -> Result<()> {
        let state = &mut self.services.dc2_bitmap;
        ensure!(
            state.owner.is_none() && state.job.is_none() && state.pending.is_none(),
            "DC2 scene release while a bitmap receipt is pending"
        );
        //The VM has already removed the transient sprite binding. Keep the
        //reusable Canvas, while retiring the separately published screen lease.
        if let Some(image) = state.bound.take() {
            self.host
                .release_image_resource(self.client, image.path.relative())?;
        }
        ensure!(
            state.published.is_none(),
            "DC2 scene release has an uncommitted screen"
        );
        state.current = None;
        state.original = None;
        state.target = None;
        state.target_source = None;
        Ok(())
    }
    pub(in crate::adapter) fn dc2_bitmap(&mut self, request: &Request) -> Result<bool> {
        let mut bitmap = std::mem::take(&mut self.services.dc2_bitmap);
        let result = self.tick_dc2_bitmap(&mut bitmap, request);
        self.services.dc2_bitmap = bitmap;
        result
    }
    fn tick_dc2_bitmap(&mut self, state: &mut Bitmap, request: &Request) -> Result<bool> {
        ensure!(
            self.services.native_profile == Some(crate::Profile::Dc2PlusCommunication),
            "DC2 bitmap profile"
        );
        if matches!(request, Request::ClearPrepared) {
            state.target = None;
            state.target_source = None;
            state.original = None;
            return Ok(true);
        }
        let owner = match request {
            Request::Capture(owner)
            | Request::Bake { owner, .. }
            | Request::Darken { owner, .. }
            | Request::Pan { owner, .. }
            | Request::Mix { owner, .. }
            | Request::Wipe { owner, .. }
            | Request::Crossfade { owner, .. }
            | Request::Flash { owner, .. }
            | Request::Shake { owner, .. }
            | Request::FadeIn { owner, .. }
            | Request::Commit { owner, .. }
            | Request::Prepare { owner, .. }
            | Request::Present { owner, .. }
            | Request::Shatter { owner, .. } => owner,
            Request::ClearPrepared => unreachable!(),
        };
        ensure!(
            self.services.native_profile == Some(crate::Profile::Dc2PlusCommunication),
            "DC2 bitmap profile"
        );
        ensure!(
            state.owner.as_ref().is_none_or(|old| old == owner),
            "DC2 bitmap completion owner {:?} for {:?} (current={})",
            state.owner, request, state.current.is_some()
        );
        if let Some(pending) = &state.pending {
            ensure!(pending == request, "DC2 bitmap worker request ownership");
            let image = match state
                .job
                .as_mut()
                .context("DC2 bitmap worker missing")?
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("DC2 shared bitmap job failed: {error}"),
                ImageProcessState::Ready(image) => image,
            };
            if let Request::Shatter { .. } = request {
                if state.shatter_cracking {
                    state.shatter_cracking = false;
                    state.shatter_source = Some(image);
                    let input = self.shatter_frame(state, request)?;
                    state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
                    return Ok(false);
                }
            }
            let expected_extent = if let Request::Bake { slot, .. } = request {
                self.services.image_extents.get(slot).copied().context("DC2 native bake extent")?
            } else { crate::dc2::CANVAS };
            ensure!(
                image.client() == self.client && image.size() == expected_extent,
                "DC2 shared bitmap image owner/extent"
            );
            if let Request::Prepare { slot, .. } | Request::Present { slot, .. } | Request::Bake { slot, .. } = request {
                //A UI paint is a presentation of native0, not a new native0.
                if !matches!(request, Request::Present { .. }) {
                    state.target = Some(image.clone());
                    state.target_source = Some(
                        self.services
                            .images
                            .get(slot)
                            .context("DC2 prepared background identity")?
                            .path
                            .relative()
                            .into(),
                    );
                }
                state.job = None;
                state.pending = None;
                if matches!(request, Request::Present { .. }) {
                    ensure!(
                        state.current.is_none() && state.published.is_none(),
                        "DC2 UI paint during a feedback capture"
                    );
                    let published =
                        self.host
                            .publish_image_resource(self.client, SCREEN_ID, image)?;
                    self.bind_transient_sprite_image(0, &published, [800, 600], [0, 0, 800, 600])?;
                    if let Some(old) = state.bound.replace(published.clone()) {
                        if old.path != published.path {
                            self.host
                                .release_image_resource(self.client, old.path.relative())?;
                        }
                    }
                }
                if state.current.is_none() {
                    state.owner = None;
                }
                return Ok(true);
            }
            //Replacing the published image occurs only on the session owner, after
            //the complete immutable result exists; old leases are then released.
            let published =
                self.host
                    .publish_image_resource(self.client, SCREEN_ID, image.clone())?;
            let old = state.published.replace(published.clone());
            self.submit(vec![json!({"op":"put","layer":{"id":SCREEN_LAYER,"rect":[0,0,800,600],"paint":{"kind":"image","asset":published.path.relative()},"filter":{"encoded_sampling":true},"order":[3,0]}})],vec![])?;
            if let Some(old) = old {
                if old.path != published.path {
                    self.host
                        .release_image_resource(self.client, old.path.relative())?;
                }
            }
            if matches!(request, Request::Capture(_)) {
                state.original = Some(image.clone());
            }
            state.current = Some(image);
            state.job = None;
            state.pending = None;
            if matches!(request, Request::Commit { .. } | Request::Darken { source_weight: 0, .. } | Request::Pan { finalize: true, .. } | Request::Shatter { finalize: true, .. }) {
                if matches!(request, Request::Shatter { .. }) {
                    state.shatter_source = None;
                    state.original = None;
                }
                //Bind the engine-composed opaque result below ordinary retained
                //UI. The encoded native slot remains independently available.
                self.bind_transient_sprite_image(0, &published, [800, 600], [0, 0, 800, 600])?;
                if let Some(old) = state.bound.replace(published.clone()) {
                    if old.path != published.path {
                        self.host
                            .release_image_resource(self.client, old.path.relative())?;
                    }
                }
                self.submit(vec![json!({"op":"remove","id":SCREEN_LAYER})], vec![])?;
                state.published = None; //its lease is now owned by bound above.
                state.current = None;
                state.owner = None;
            }
            return Ok(true);
        }
        if let Request::Flash { finalize: true, .. } = request {
            self.submit(vec![json!({"op":"remove","id":SCREEN_LAYER})], vec![])?;
            if let Some(old) = state.published.take() {
                self.host.release_image_resource(self.client, old.path.relative())?;
            }
            state.current = None;
            state.original = None;
            state.owner = None;
            return Ok(true);
        }
        match request {
            Request::ClearPrepared => unreachable!(),
            Request::Shake { offset, fill, .. } => {
                let original = state.original.as_ref().context("DC2 shake has no frozen screen")?.clone();
                let base = state.current.clone().unwrap_or_else(|| original.clone());
                let mut layers = Vec::new();
                if let Some(([x1, y1, x2, y2], c)) = fill {
                    let (x1, y1, x2, y2) = ((*x1).max(0), (*y1).max(0), (*x2).min(800), (*y2).min(600));
                    if x2 > x1 && y2 > y1 {
                        layers.push(ImageCompositeLayer {
                            input: Box::new(ImageInput::Solid { size: [u32::try_from(x2 - x1)?, u32::try_from(y2 - y1)?], color: [c[0], c[1], c[2], 255] }),
                            source: [0, 0, x2 - x1, y2 - y1],
                            destination: [x1, y1],
                            clip: Some([0, 0, 800, 600]),
                            opacity: 255,
                            math: ImageCompositeMath::CopyRgb,
                        });
                    }
                }
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(original)),
                    source: [0, 0, 800, 600],
                    destination: *offset,
                    clip: Some([0, 0, 800, 600]),
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                });
                let input = ImageInput::Composite { base: Box::new(ImageInput::Resource(base)), layers };
                state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
            }
            Request::Flash { fill, .. } => {
                let input = match fill {
                    Some(c) => ImageInput::Solid { size: crate::dc2::CANVAS, color: [c[0], c[1], c[2], 255] },
                    None => ImageInput::Resource(
                        state.original.as_ref().context("DC2 flash has no frozen screen")?.clone(),
                    ),
                };
                state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
            }
            Request::Prepare { slot, draw, .. } | Request::Present { slot, draw, .. } | Request::Bake { slot, draw, .. } => {
                if let Some((expected, _, receiver)) = &state.validation {
                    ensure!(expected == request, "DC2 display-format worker owner");
                    match receiver.try_recv() {
                        Err(TryRecvError::Empty) => return Ok(false),
                        Err(TryRecvError::Disconnected) => {
                            bail!("DC2 display-format worker disconnected")
                        }
                        Ok(result) => {
                            let alpha = result.map_err(anyhow::Error::msg)?;
                            let (_, sources, _) = state.validation.take().unwrap();
                            ensure!(alpha.len() == sources.len(), "DC2 display-format result count");
                            for ((slot, key, _), alpha) in sources.into_iter().zip(alpha) {
                                state.validated.insert(slot, (key, alpha));
                            }
                        }
                    }
                }
                let mut sources = BTreeMap::new();
                for record in draw {
                    if let crate::dc2_bitmap::Draw::Image { slot, .. } = record {
                        if let Some(image) = self.services.images.get(slot) {
                            let key = image.path.relative().to_owned();
                            if state.validated.get(slot).map(|(k, _)| k) != Some(&key) {
                                sources.insert(
                                    *slot,
                                    (
                                        key,
                                        self.services
                                            .image_names
                                            .get(slot)
                                            .context("DC2 display source filename")?
                                            .clone(),
                                    ),
                                );
                            }
                        }
                    }
                }
                if !sources.is_empty() {
                    let sources = sources
                        .into_iter()
                        .map(|(slot, (key, name))| (slot, key, name))
                        .collect::<Vec<_>>();
                    let names = sources
                        .iter()
                        .map(|(_, _, name)| name.clone())
                        .collect::<Vec<_>>();
                    let storage = self
                        .services
                        .storage
                        .as_ref()
                        .context("DC2 display-format storage")?
                        .clone();
                    let (sender, receiver) = mpsc::sync_channel(1);
                    std::thread::Builder::new()
                        .name("circus-dc2-display-format".into())
                        .spawn(move || {
                            let result = names
                                .iter()
                                .map(|name| dc2_background::validate(storage.as_ref(), name))
                                .collect::<Result<Vec<_>>>();
                            let _ = sender.send(result.map_err(|error| format!("{error:#}")));
                        })?;
                    state.validation = Some((request.clone(), sources, receiver));
                    return Ok(false);
                }
                let extent = if matches!(request, Request::Bake { .. }) {
                    self.services.image_extents.get(slot).copied().context("DC2 native bake extent")?
                } else { crate::dc2::CANVAS };
                //Process28 (40F930) repaints the whole screen without native0.
                if matches!(request, Request::Present { .. }) && !self.services.images.contains_key(slot) {
                    let input = ImageInput::Composite {
                        base: Box::new(ImageInput::Solid { size: extent, color: [0, 0, 0, 255] }),
                        layers: records::layers(self, draw, extent, &state.validated)?,
                    };
                    state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
                    state.owner = Some(owner.clone());
                    state.pending = Some(request.clone());
                    return Ok(false);
                }
                let source = self.services.images.get(slot).context("DC2 bake source identity")?.path.relative();
                let base = if matches!(request, Request::Bake { .. }) && state.target_source.as_deref() == Some(source) {
                    let previous = state.target.as_ref().context("DC2 native bake previous bitmap")?;
                    ensure!(previous.size() == extent, "DC2 native bake after viewport-only composition is not connected");
                    ImageInput::Resource(previous.clone())
                } else if matches!(request, Request::Present { .. }) && state.target_source.as_deref() == Some(source) {
                    //UI paint redraws over the composed native0 (baked records).
                    let previous = state.target.as_ref().context("DC2 composed native0 bitmap")?.clone();
                    let size = previous.size();
                    ImageInput::Composite {
                        base: Box::new(ImageInput::Solid { size: extent, color: [0, 0, 0, 255] }),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(previous)),
                            source: [0, 0, i32::try_from(size[0])?, i32::try_from(size[1])?],
                            destination: [0, 0],
                            clip: Some([0, 0, i32::try_from(extent[0])?, i32::try_from(extent[1])?]),
                            opacity: 255,
                            math: ImageCompositeMath::CopyRgb,
                        }],
                    }
                } else { self.dc2_background_input_extent(*slot, extent)? };
                let input = ImageInput::Composite {
                    base: Box::new(base),
                    layers: records::layers(self, draw, extent, &state.validated)?,
                };
                state.job = Some(
                    self.host
                        .image_metadata(self.client)?
                        .begin_process(input, Arc::from(b"[]".as_slice()))?,
                );
            }
            Request::Capture(_) => {
                if state.current.is_some() {
                    return Ok(true);
                }
                self.root()?;
                let mut canvases = Vec::new();
                if !state.created {
                    canvases.push(json!({"op":"create","id":CANVAS_ID,"width":800,"height":600}));
                }
                canvases.extend([json!({"op":"clear","id":CANVAS_ID,"color":[0,0,0,255]}),json!({"op":"copy_subtree","source":SCENE_ROOT,"destination":CANVAS_ID,"rect":[0,0,800,600],"position":[0,0]})]);
                self.host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&json!({"canvases":canvases}))?,
                )?;
                state.created = true;
                state.job = Some(self.host.begin_canvas_image(self.client, CANVAS_ID)?);
            }
            Request::Shatter { overlay_slot, cracks, .. } => {
                if state.shatter_source.is_none() {
                    state.shatter_source = Some(state.original.as_ref().or(state.current.as_ref())
                        .context("DC2 screen break has no captured scrbrk screen")?.clone());
                }
                let source = state.shatter_source.clone().unwrap();
                let input = if cracks.is_empty() {
                    self.shatter_frame(state, request)?
                } else {
                    let overlay = self.services.images.get(overlay_slot).context("DC2 screen-break overlay")?;
                    let overlay = self.host.published_image_resource(self.client, overlay.path.relative())?
                        .context("DC2 screen-break overlay lease")?;
                    let width = i32::try_from(overlay.size()[0])?;
                    use crysirror_core::image_blend::{RgbaBlendArithmetic as A, RgbaBlendState, RgbaByteRounding as R};
                    //40B9C0: copy only the FF00FF overlay texels (hole punches).
                    let math = RgbaBlendState {
                        arithmetic: A::Byte255 { source: R::Floor, destination: R::Floor },
                        write_mask: [true, true, true, false],
                        source_color_key: Some([255, 0, 255]),
                        source_color_key_invert: true,
                        ..Default::default()
                    };
                    let layers = cracks.iter().filter(|(_, frame)| *frame < width).map(|(origin, frame)| ImageCompositeLayer {
                        input: Box::new(ImageInput::Resource(overlay.clone())),
                        source: [*frame, 0, 32.min(width - *frame), 32],
                        destination: *origin,
                        clip: Some([origin[0], origin[1], 32, 32]),
                        opacity: 255,
                        math: ImageCompositeMath::Normalized(math),
                    }).collect();
                    state.shatter_cracking = true;
                    ImageInput::Composite { base: Box::new(ImageInput::Resource(source)), layers }
                };
                state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
            }
            Request::Darken { source_weight, .. } => {
                let input = darken::input(
                    self.client,
                    state.original.as_ref().context("DC2 dimmer has no frozen screen capture")?.clone(),
                    *source_weight,
                )?;
                state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
            }
            Request::Pan { slot, position, source_weight, .. } => {
                ensure!(*source_weight <= 16, "DC2 pan byte feedback weight");
                let source = self.services.images.get(slot).context("DC2 pan native0 identity")?;
                let extent = self.services.image_extents.get(slot).copied().context("DC2 pan native0 extent")?;
                let input = if state.target_source.as_deref() == Some(source.path.relative()) {
                    let target = state.target.as_ref().context("DC2 pan composed native0")?;
                    ensure!(target.size() == extent, "DC2 pan requires complete native0 rather than a viewport crop");
                    ImageInput::Resource(target.clone())
                } else { self.dc2_background_input_extent(*slot, extent)? };
                let input = ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(state.current.as_ref().context("DC2 pan previous framebuffer")?.clone())),
                    layers: vec![ImageCompositeLayer {
                        input: Box::new(input),
                        source: [0, 0, i32::try_from(extent[0])?, i32::try_from(extent[1])?],
                        destination: *position, clip: Some([0, 0, 800, 600]), opacity: 255,
                        math: ImageCompositeMath::IntegerStored { destination_format: 1, source_format: 1, mode: 0xf0, parameter: i32::from(16 - source_weight) * 16 },
                    }],
                };
                state.job = Some(self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?);
            }
            Request::Mix { slot, .. } | Request::Wipe { slot, .. } | Request::Commit { slot, .. } | Request::Crossfade { slot, .. } | Request::FadeIn { slot, .. } => {
                //Shared mode f0 computes s+floor((d-s)*(p/2)/128); p is the
                //destination share in /256 units (even for every native step).
                let (parameter, base) = match request {
                    Request::Mix { source_weight, .. } => {
                        ensure!(*source_weight <= 16, "DC2 native feedback source weight");
                        (i32::from(16 - source_weight) * 16, state.current.as_ref())
                    }
                    Request::Crossfade { weight, .. } => {
                        ensure!(*weight <= 256 && weight % 2 == 0, "DC2 native crossfade weight");
                        (256 - i32::from(*weight), state.original.as_ref())
                    }
                    Request::FadeIn { weight, .. } => {
                        ensure!(*weight <= 16, "DC2 native fade-in weight");
                        (i32::from(16 - weight) * 16, None)
                    }
                    //40EAB0 blends onto the presented screen, like 40B300.
                    Request::Wipe { .. } => (0, state.current.as_ref()),
                    _ => (0, state.current.as_ref()),
                };
                let base = match (request, base) {
                    (Request::FadeIn { .. }, _) => ImageInput::Solid { size: crate::dc2::CANVAS, color: [0, 0, 0, 255] },
                    (_, base) => ImageInput::Resource(base.context("DC2 feedback has no captured previous screen")?.clone()),
                };
                let source = self
                    .services
                    .images
                    .get(slot)
                    .context("DC2 bitmap source texture")?;
                let image = self
                    .host
                    .published_image_resource(self.client, source.path.relative())?
                    .context("DC2 bitmap source image lease")?;
                let size = self
                    .services
                    .image_extents
                    .get(slot)
                    .copied()
                    .context("DC2 bitmap logical source extent")?;
                let native0 =
                            if state.target_source.as_deref() == Some(source.path.relative()) {
                                ImageInput::Resource(
                                    state
                                        .target
                                        .as_ref()
                                        .context("DC2 composed target receipt missing")?
                                        .clone(),
                                )
                            } else {
                                ImageInput::Composite {
                                    base: Box::new(ImageInput::Solid {
                                        //402370 used HeapAlloc without clearing the native
                                        //work bitmap. Define uncovered small-image regions
                                        //as opaque black instead of retaining uninitialized
                                        //memory. A full800x600 background is unchanged.
                                        size: crate::dc2::CANVAS,
                                        color: [0, 0, 0, 255],
                                    }),
                                    layers: vec![ImageCompositeLayer {
                                        input: Box::new(ImageInput::Resource(image)),
                                        source: [
                                            0,
                                            0,
                                            i32::try_from(size[0])?,
                                            i32::try_from(size[1])?,
                                        ],
                                        destination: [0, 0],
                                        clip: Some([0, 0, 800, 600]),
                                        opacity: 255,
                                        math: ImageCompositeMath::CopyRgb,
                                    }],
                                }
                            };
                let (native0, math) = if let Request::Wipe { mode, offset, .. } = request {
                    //Mask a = 16w-1 (0 for w = 0): MaskIncrement256 then computes
                    //(s*16w + d*(256-16w)) >> 8 == (w*s + (16-w)*d) >> 4 exactly.
                    let mut rgba = vec![0u8; 800 * 600 * 4];
                    for y in 0..600 {
                        for x in 0..800 {
                            let weight = crate::dc2::wipe::field(*mode, x, y)
                                .context("DC2 424B40 field mode")?
                                .saturating_add(*offset)
                                .clamp(0, 16);
                            rgba[((y * 800 + x) * 4 + 3) as usize] =
                                if weight == 0 { 0 } else { (weight * 16 - 1) as u8 };
                        }
                    }
                    let mask: Arc<dyn ImageResource> = Arc::new(WipeMask { client: self.client, rgba: rgba.into() });
                    (
                        ImageInput::Composite {
                            base: Box::new(native0),
                            layers: vec![ImageCompositeLayer {
                                input: Box::new(ImageInput::Resource(mask)),
                                source: [0, 0, 800, 600],
                                destination: [0, 0],
                                clip: None,
                                opacity: 255,
                                math: ImageCompositeMath::CopyAlpha,
                            }],
                        },
                        ImageCompositeMath::MaskIncrement256,
                    )
                } else {
                    //40B300: floor((source*w + destination*(16-w))/16).
                    //Shared integer modef0 uses s+floor((d-s)*(p/2)/128),
                    //identical when p=(16-w)*16. Opaque inputs preserve alpha255.
                    (
                        native0,
                        ImageCompositeMath::IntegerStored {
                            destination_format: 1,
                            source_format: 1,
                            mode: 0xf0,
                            parameter,
                        },
                    )
                };
                let input = ImageInput::Composite {
                    base: Box::new(base),
                    layers: vec![ImageCompositeLayer {
                        input: Box::new(native0),
                        source: [0, 0, 800, 600],
                        destination: [0, 0],
                        clip: Some([0, 0, 800, 600]),
                        opacity: 255,
                        math,
                    }],
                };
                state.job = Some(
                    self.host
                        .image_metadata(self.client)?
                        .begin_process(input, Arc::from(b"[]".as_slice()))?,
                );
            }
        }
        state.owner = Some(owner.clone());
        if crate::trace_enabled() {
            let kind = format!("{request:?}");
            eprintln!("DC2 bitmap begin {}", &kind[..kind.len().min(120)]);
        }
        state.pending = Some(request.clone());
        Ok(false)
    }
    ///418570: palette fill (40E300) then keyed /16 tile blits (40C360).
    fn shatter_frame(&self, state: &Bitmap, request: &Request) -> Result<ImageInput> {
        let Request::Shatter { fill, tiles, .. } = request else { bail!("DC2 screen-break request") };
        let source = state.shatter_source.clone().context("DC2 screen-break source")?;
        use crysirror_core::image_blend::{RgbaBlendArithmetic as A, RgbaBlendComponent, RgbaBlendFactor as F, RgbaBlendState};
        let layers = tiles.iter().map(|(origin, destination, weight)| {
            let k = f32::from(*weight) / 16.;
            ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source.clone())),
                source: [origin[0], origin[1], 32, 32],
                destination: *destination,
                clip: Some([0, 0, 800, 600]),
                opacity: 255,
                math: ImageCompositeMath::Normalized(RgbaBlendState {
                    arithmetic: A::ByteShiftSum { shift: 4 },
                    color: RgbaBlendComponent { source: F::ConstantAlpha, destination: F::OneMinusConstantAlpha, ..Default::default() },
                    alpha: RgbaBlendComponent { source: F::Zero, destination: F::One, ..Default::default() },
                    constant: [k; 4],
                    write_mask: [true, true, true, false],
                    source_color_key: Some([255, 0, 255]),
                    ..Default::default()
                }),
            }
        }).collect();
        Ok(ImageInput::Composite {
            base: Box::new(ImageInput::Solid { size: crate::dc2::CANVAS, color: [fill[0], fill[1], fill[2], 255] }),
            layers,
        })
    }
    fn dc2_background_input_extent(&mut self, slot: u8, extent: [u32; 2]) -> Result<ImageInput> {
        let source = self
            .services
            .images
            .get(&slot)
            .context("DC2 compose background slot")?;
        let image = self
            .host
            .published_image_resource(self.client, source.path.relative())?
            .context("DC2 compose background lease")?;
        let size = self
            .services
            .image_extents
            .get(&slot)
            .copied()
            .context("DC2 compose background extent")?;
        Ok(ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: extent,
                color: [0, 0, 0, 255],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(image)),
                source: [0, 0, i32::try_from(size[0])?, i32::try_from(size[1])?],
                destination: [0, 0],
                clip: Some([0, 0, i32::try_from(extent[0])?, i32::try_from(extent[1])?]),
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        })
    }
}
