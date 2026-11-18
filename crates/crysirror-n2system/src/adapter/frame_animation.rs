// SPDX-License-Identifier: MPL-2.0
//! NSS playlist policy from NitroSharp FrameSequence.cs, 77a93db4 (MIT).
//! Shared FrameSequenceClock owns stepping; retained layers own GPU drawing.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_scene::animation::FrameSequenceClock;

pub(super) struct SurfaceFrames {
    names: Vec<String>,
    final_source: String,
    references: BTreeMap<String, u32>,
    jobs: BTreeMap<u32, ImageJob>,
    ready: BTreeMap<String, Arc<dyn ImageResource>>,
}
impl SurfaceFrames {
    fn key(id: u32) -> String {
        // Private metadata identity, deliberately not a valid VFS path. Every
        // occurrence is republished into an owned immutable alias before Put.
        format!("\0n2-surface:{id}")
    }
    fn image(&self, name: &str) -> Option<Image> {
        let id = *self.references.get(name)?;
        let key = Self::key(id);
        let resource = self.ready.get(&key)?;
        Some(Image {
            paint: json!({"kind":"image","asset":key}),
            size: resource.size(),
        })
    }
    fn poll(&mut self, mut begin: impl FnMut(u32) -> Result<ImageJob>) -> Result<bool> {
        if self.jobs.is_empty() && self.ready.is_empty() {
            let ids: BTreeSet<_> = self.references.values().copied().collect();
            ensure!(
                ids.len() <= 16,
                "N2System simultaneous animation surface capture limit"
            );
            // Admit every distinct input before yielding so all GPU copies
            // freeze at one presenter fence, including the terminal picture.
            for id in ids {
                match begin(id) {
                    Ok(job) => {
                        self.jobs.insert(id, job);
                    }
                    Err(error)
                        if matches!(
                            error.downcast_ref::<Error>(),
                            Some(Error::Limit("Canvas capture jobs"))
                        ) =>
                    {
                        self.jobs.clear();
                        return Ok(false);
                    }
                    Err(error) => {
                        self.jobs.clear();
                        return Err(error);
                    }
                }
            }
        }
        let mut finished = Vec::new();
        for (id, job) in &mut self.jobs {
            match job.poll()? {
                ImageProcessState::Pending => {}
                ImageProcessState::Failed(error) => {
                    bail!("N2System animation surface capture: {error}")
                }
                ImageProcessState::Ready(resource) => {
                    self.ready.insert(Self::key(*id), resource);
                    finished.push(*id);
                }
            }
        }
        for id in finished {
            self.jobs.remove(&id);
        }
        Ok(self.jobs.is_empty())
    }
}

pub(super) struct Prepared {
    images: Vec<Image>,
    sources: Vec<String>,
    final_image: Option<Image>,
    final_source: Option<String>,
    leases: BTreeMap<String, Arc<dyn crysirror_core::image_processing::ImageResource>>,
}
pub(super) struct Preparing {
    prepared: Prepared,
    inputs: Vec<(String, [u32; 2])>,
    next: usize,
    job: Option<ImageJob>,
}
impl Preparing {
    fn new(prepared: Prepared) -> Result<Self> {
        let mut inputs = BTreeMap::new();
        let mut bytes = 0u64;
        for image in prepared.images.iter().chain(prepared.final_image.iter()) {
            if let Some(path) = image.paint.get("asset").and_then(Json::as_str) {
                if !inputs.contains_key(path) {
                    bytes = bytes
                        .checked_add(u64::from(image.size[0]) * u64::from(image.size[1]) * 4)
                        .context("N2System frame resource byte overflow")?;
                    inputs.insert(path.to_owned(), image.size);
                }
            }
        }
        // A playlist retains all its unique decoded inputs, like the native
        // runtime. Admit a fixed working set before decoding the first frame.
        let limit = if cfg!(target_os = "vita") {
            32 * 1024 * 1024
        } else {
            64 * 1024 * 1024
        };
        ensure!(
            bytes <= limit,
            "N2System animation decoded-image working set"
        );
        let inputs = inputs
            .into_iter()
            .filter(|(p, _)| !prepared.leases.contains_key(p))
            .collect();
        Ok(Self {
            prepared,
            inputs,
            next: 0,
            job: None,
        })
    }
    fn poll(&mut self, client: ClientId, images: &dyn ImageMetadata) -> Result<bool> {
        if let Some(job) = &mut self.job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!(
                    "N2System animation frame {}: {error}",
                    self.inputs[self.next].0
                ),
                ImageProcessState::Ready(image) => {
                    let (path, size) = &self.inputs[self.next];
                    ensure!(
                        image.client() == client && image.size() == *size,
                        "N2System decoded animation frame identity"
                    );
                    self.prepared.leases.insert(path.clone(), image);
                    self.next += 1;
                    self.job = None;
                }
            }
        }
        if self.next == self.inputs.len() {
            return Ok(true);
        }
        let (path, size) = &self.inputs[self.next];
        match images.begin_process(
            ImageInput::Asset {
                path: path.clone(),
                size: *size,
                fingerprint: None,
            },
            Arc::from(&b"[]"[..]),
        ) {
            Ok(job) => self.job = Some(job),
            Err(Error::Limit("image preparation task capacity")) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(false)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Animation {
    pub clock: FrameSequenceClock,
    images: Vec<Image>,
    sources: Vec<String>,
    final_image: Option<Image>,
    final_source: Option<String>,
    remove: bool,
    last_ms: u64,
    shown: Option<usize>,
    showed_final: bool,
}
impl N2SystemVm {
    pub(super) fn native_frame_animation(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let mut result = Value::Null;
        match call.function {
            Builtin::WaitFrameAnimation => {
                let ids = self
                    .query(current, &args.string(0)?)?
                    .into_iter()
                    .flat_map(|id| self.entities.subtree(id))
                    .filter(|id| self.frame_animations.contains_key(id))
                    .collect();
                state.wait = Wait::Frames(ids);
            }
            Builtin::FinishFrameAnimation => {
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(animation) = self.frame_animations.get(&id) {
                        result = Value::String(if animation.remove {
                            String::new()
                        } else {
                            animation.final_source.clone().unwrap_or_else(|| {
                                animation.sources[animation.clock.index as usize].clone()
                            })
                        });
                        self.control_frame_animation(host, id, true)?;
                        break;
                    }
                }
            }
            Builtin::CreateFrameAnimation => {
                if let Some(preparing) = &mut state.frame_preparation {
                    if !preparing.poll(self.client, self.images.as_ref().unwrap().as_ref())? {
                        return Ok(false);
                    }
                }
                let mut prepared = if let Some(preparing) = state.frame_preparation.take() {
                    preparing.prepared
                } else if let Some(task) = &mut state.frame_images {
                    let Some(prepared) = task.try_take()? else {
                        return Ok(false);
                    };
                    state.frame_images = None;
                    state.frame_preparation = Some(Preparing::new(prepared)?);
                    return Ok(false);
                } else {
                    if state.frame_surfaces.is_none() {
                        let Value::Array(id, _) = &call
                            .arguments
                            .get(4)
                            .context("N2System frame list argument")?
                            .value
                        else {
                            bail!("N2System frame list requires an array");
                        };
                        let items = &self
                            .machine
                            .arrays
                            .nodes
                            .get(id)
                            .context("N2System frame array identity")?
                            .items;
                        ensure!(
                            !items.is_empty() && items.len() <= 4096,
                            "N2System frame list bounds"
                        );
                        let names = items
                            .iter()
                            .map(|id| self.machine.arrays.value(*id)?.string())
                            .collect::<Result<Vec<_>>>()?;
                        let final_source = args.string(6)?;
                        let mut references = BTreeMap::new();
                        for name in names.iter().chain(std::iter::once(&final_source)) {
                            if let Some(id) = self.surface_source(current, name)? {
                                references.insert(name.clone(), id);
                            }
                        }
                        state.frame_surfaces = Some(SurfaceFrames {
                            names,
                            final_source,
                            references,
                            jobs: BTreeMap::new(),
                            ready: BTreeMap::new(),
                        });
                    }
                    let surfaces = state.frame_surfaces.as_mut().unwrap();
                    if surfaces.jobs.is_empty()
                        && surfaces.ready.is_empty()
                        && surfaces
                            .references
                            .values()
                            .any(|id| self.shades.get(id).is_some_and(shade::Shade::pending))
                    {
                        return Ok(false);
                    }
                    if !surfaces.poll(|id| self.begin_surface_image(host, id))? {
                        return Ok(false);
                    }
                    let names = surfaces.names.clone();
                    let final_source = surfaces.final_source.clone();
                    let mut images = Vec::with_capacity(names.len());
                    let mut sources = Vec::with_capacity(names.len());
                    for source in names {
                        let image = if source.is_empty() {
                            None
                        } else if let Some(image) = surfaces.image(&source) {
                            Some(image)
                        } else {
                            self.source_image(current, &source)?
                        };
                        if let Some(image) = image {
                            images.push(image);
                            sources.push(source);
                        } else {
                            images.push(
                                images
                                    .last()
                                    .context("N2System first animation frame is missing")?
                                    .clone(),
                            );
                            sources.push(sources.last().unwrap().clone());
                        }
                    }
                    let final_image = if final_source.is_empty() {
                        None
                    } else if let Some(image) = surfaces.image(&final_source) {
                        Some(image)
                    } else {
                        self.source_image(current, &final_source)?
                    };
                    let metadata = self.images.as_ref().unwrap().clone();
                    let mut leases = surfaces.ready.clone();
                    for image in images.iter().chain(final_image.iter()) {
                        if let Some(path) = image.paint.get("asset").and_then(Json::as_str) {
                            if !leases.contains_key(path) {
                                if let Some(resource) =
                                    host.published_image_resource(self.client, path)?
                                {
                                    leases.insert(path.to_owned(), resource);
                                }
                            }
                        }
                    }
                    match self.pool.as_ref().unwrap().spawn(move |cancel| {
                        let mut prepared = Prepared {
                            images,
                            sources,
                            final_source: final_image.as_ref().map(|_| final_source),
                            final_image,
                            leases,
                        };
                        for image in prepared
                            .images
                            .iter_mut()
                            .chain(prepared.final_image.iter_mut())
                        {
                            cancel.check()?;
                            if image.size == [0; 2] {
                                let asset = image.paint["asset"]
                                    .as_str()
                                    .context("N2System frame asset")?;
                                image.size = metadata.size(asset)?;
                            }
                            ensure!(
                                image.size.iter().all(|s| (1..=16384).contains(s)),
                                "N2System frame dimensions"
                            );
                        }
                        Ok(prepared)
                    }) {
                        Ok(task) => {
                            state.frame_images = Some(task);
                            state.frame_surfaces = None;
                        }
                        Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                        Err(e) => return Err(e),
                    }
                    return Ok(false);
                };
                let ticks = args.integer(5)?;
                ensure!(
                    ticks > 0 && ticks <= 2_618_181,
                    "N2System frame interval bounds"
                );
                let clock = FrameSequenceClock::new(
                    prepared.images.len() as u32,
                    ticks as u64 * 33,
                    args.truth(8)?,
                    true,
                )?;
                let image = &prepared.images[0];
                let first_path = image
                    .paint
                    .get("asset")
                    .and_then(Json::as_str)
                    .map(str::to_owned);
                let first_lease = first_path
                    .as_ref()
                    .and_then(|path| prepared.leases.get(path))
                    .cloned();
                let id = self.visual_owned(
                    host,
                    current,
                    args,
                    image.paint.clone(),
                    image.size,
                    None,
                    first_lease,
                )?;
                let mut paths = BTreeMap::new();
                let mut owned = Vec::new();
                if let Some(source) = first_path {
                    if let Some(path) = self.captured_images.get(&id) {
                        paths.insert(source, path.clone());
                    }
                }
                for (source, resource) in prepared.leases {
                    if paths.contains_key(&source) {
                        continue;
                    }
                    let uploaded = host.publish_image_resource(self.client, id, resource)?;
                    let path = uploaded.path.relative().to_owned();
                    owned.push(path.clone());
                    paths.insert(source, path);
                }
                for image in prepared
                    .images
                    .iter_mut()
                    .chain(prepared.final_image.iter_mut())
                {
                    if let Some(path) = image
                        .paint
                        .get("asset")
                        .and_then(Json::as_str)
                        .and_then(|p| paths.get(p))
                    {
                        image.paint["asset"] = json!(path);
                    }
                }
                self.animation_image_leases.insert(id, owned);
                self.frame_animations.insert(
                    id,
                    Animation {
                        clock,
                        images: prepared.images,
                        sources: prepared.sources,
                        final_image: prepared.final_image,
                        final_source: prepared.final_source,
                        remove: args.truth(7)?,
                        last_ms: host.now_ms(),
                        shown: Some(0),
                        showed_final: false,
                    },
                );
                if args.truth(9)? {
                    state.wait = Wait::Frames(vec![id]);
                }
            }
            _ => bail!("N2System frame animation dispatch mismatch"),
        }
        self.machine.complete(&mut state.thread, result)?;
        Ok(true)
    }
    pub(super) fn control_frame_animation(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        stop: bool,
    ) -> Result<()> {
        let animation = self
            .frame_animations
            .get_mut(&id)
            .context("N2System animation identity")?;
        if stop {
            animation.clock.stop();
        } else {
            animation.clock.resume();
        }
        animation.last_ms = host.now_ms();
        self.sample_frame_animations(host)
    }
    pub(super) fn interrupt_frame_waits(&mut self, input: &mut InputFrame) {
        if input.buttons_pressed
            & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::ENTER | buttons::SPACE)
            == 0
            && input.buttons_down & buttons::SHIFT == 0
        {
            return;
        }
        for state in self.threads.values() {
            if !state.enabled || state.thread.halted {
                continue;
            }
            if let Wait::Frames(ids) = &state.wait {
                for id in ids {
                    if let Some(animation) = self.frame_animations.get_mut(id) {
                        if !animation.clock.complete {
                            animation.clock.stop();
                            input.buttons_pressed &= !(buttons::MOUSE_LEFT
                                | buttons::MOUSE_RIGHT
                                | buttons::ENTER
                                | buttons::SPACE);
                        }
                    }
                }
            }
        }
    }
    pub(super) fn sample_frame_animations(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut remove = Vec::new();
        let mut updates = Vec::new();
        for (&id, animation) in &mut self.frame_animations {
            animation
                .clock
                .advance(now.saturating_sub(animation.last_ms))?;
            animation.last_ms = now;
            if animation.clock.complete && animation.remove {
                remove.push(id);
                continue;
            }
            let final_frame = animation.clock.complete && animation.final_image.is_some();
            let index = animation.clock.index as usize;
            if animation.shown == Some(index) && animation.showed_final == final_frame {
                continue;
            }
            let image = if final_frame {
                animation.final_image.as_ref().unwrap()
            } else {
                &animation.images[index]
            };
            // Changing paint preserves independent move/fade/rotate tracks.
            updates.push((id, image.clone()));
            animation.shown = Some(index);
            animation.showed_final = final_frame;
        }
        let mut changes = Vec::new();
        for (id, image) in updates {
            let layer = self.layer(host, id)?;
            changes.push(json!({"op":"set_paint","id":id,"paint":image.paint}));
            changes.push(json!({"op":"set_rect","id":id,"rect":[layer["rect"][0],layer["rect"][1],image.size[0],image.size[1]]}));
            let entity = self.entities.nodes.get_mut(&id).unwrap();
            if entity.pivot_auto {
                let mut transform = layer["transform"].clone();
                transform["pivot_x"] = json!(image.size[0] as f32 * 0.5);
                transform["pivot_y"] = json!(image.size[1] as f32 * 0.5);
                changes.push(json!({"op":"set_transform","id":id,"transform":transform}));
            }
            entity.size = image.size.map(|s| s as f32);
        }
        self.submit(host, changes, vec![])?;
        for id in remove {
            self.destroy(host, id)?;
        }
        Ok(())
    }
}
