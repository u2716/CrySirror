// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native picture-slot loading through scoped resource workers and ImageJob.
use crate::{
    machine::{Machine, NativeCall, Payload},
    native_resources::ResourceNames,
    resource_bridge::NativeResources,
};
use anyhow::{Result, bail, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone)]
pub struct PreparedPicture {
    pub has_mask: bool,
    pub slot: i32,
    pub preload: bool,
    pub name: Vec<u8>,
    pub raw: Arc<dyn ImageResource>,
    pub opaque: Arc<dyn ImageResource>,
    pub display: Arc<dyn ImageResource>,
    /// Original masked/keyed blit view, even for a currently opaque BG slot.
    /// The mask-exists fact chooses the policy; opacity never substitutes it.
    pub keyed: Arc<dyn ImageResource>,
}
pub(crate) struct PicturePreparation {
    client: ClientId,
    slot: i32,
    preload: bool,
    name: Vec<u8>,
    task: Option<Task<crate::resource_bridge::NativeImageSource>>,
    job: Option<ImageJob>,
    raw: Option<Arc<dyn ImageResource>>,
    stage: u8,
    opaque: Option<Arc<dyn ImageResource>>,
    has_mask: Option<bool>,
    ready: Option<PreparedPicture>,
    pub api_calls: u64,
}
struct Pending {
    call: NativeCall,
    preparation: PicturePreparation,
}
pub struct ImageBridge {
    client: ClientId,
    names: ResourceNames,
    levels: u32,
    resources: NativeResources,
    pool: TaskPool,
    pending: Option<Pending>,
    /// Original CPicture slots retain raw alpha even when their current
    /// background display ignores it. Engine resources own every pixel.
    pub pictures: BTreeMap<i32, PreparedPicture>,
    pub api_calls: u64,
}
impl ImageBridge {
    pub fn new(
        client: ClientId,
        names: ResourceNames,
        levels: u32,
        resources: NativeResources,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0 && levels <= 1024 && resources.client() == client,
            "NNN image capability/pack-level bounds"
        );
        Ok(Self {
            client,
            names,
            levels,
            resources,
            pool: TaskPool::new("nnn-native-picture", 1, 1)?,
            pending: None,
            pictures: BTreeMap::new(),
            api_calls: 0,
        })
    }
    pub(crate) fn detached_restore_copy(&self) -> Result<Self> {
        ensure!(
            self.pending.is_none(),
            "NNN image restore requires drained native picture receipt"
        );
        Ok(Self {
            client: self.client,
            names: self.names.clone(),
            levels: self.levels,
            resources: self.resources.clone(),
            pool: self.pool.clone(),
            pending: None,
            pictures: self.pictures.clone(),
            api_calls: 0,
        })
    }
    pub(crate) fn resources(&self) -> NativeResources {
        self.resources.clone()
    }
    pub fn dependency_pending(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|p| p.preparation.ready.is_none())
    }
    pub fn client(&self) -> ClientId {
        self.client
    }
    pub fn prepare(
        &mut self,
        machine: &Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<PreparedPicture>> {
        machine.validate_receipt(call)?;
        let Payload::Image {
            preload,
            slot,
            name,
        } = &call.payload
        else {
            bail!("NNN image slot payload")
        };
        ensure!((0..16).contains(slot), "NNN legacy picture slot bounds");
        if self.pending.is_none() {
            self.pending = Some(Pending {
                call: call.clone(),
                preparation: PicturePreparation::begin(
                    self.client,
                    *slot,
                    name.clone(),
                    *preload,
                    &self.names,
                    self.levels,
                    self.resources.clone(),
                    &self.pool,
                )?,
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "NNN image slot receipt changed");
        let before = pending.preparation.api_calls;
        let result = pending.preparation.poll(host);
        self.api_calls += pending.preparation.api_calls - before;
        result
    }
    /// Owner presentation and exact native acknowledgment must succeed first.
    pub fn accept(&mut self, call: &NativeCall) -> Result<()> {
        let pending = self
            .pending
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NNN picture acceptance absent"))?;
        ensure!(
            pending.call == *call,
            "NNN picture acceptance receipt changed"
        );
        let prepared = pending
            .preparation
            .ready
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NNN picture is not ready"))?;
        self.pictures.insert(prepared.slot, prepared);
        self.pending = None;
        Ok(())
    }
}

impl PicturePreparation {
    /// Native slot/alpha policy over shared resource workers and ImageJobs.
    /// Original save restoration can prepare pictures without inventing a VM
    /// instruction/receipt; all pixels and format decoding remain engine-owned.
    pub(crate) fn begin(
        client: ClientId,
        slot: i32,
        name: Vec<u8>,
        preload: bool,
        names: &ResourceNames,
        levels: u32,
        resources: NativeResources,
        pool: &TaskPool,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0
                && (0..16).contains(&slot)
                && levels <= 1024
                && resources.client() == client,
            "NNN picture preparation scope/slot/profile"
        );
        let lookup = names.script_image(&name, levels)?;
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            let source = resources.image_source(&lookup, &cancel)?;
            cancel.check()?;
            Ok(source)
        })?;
        Ok(Self {
            client,
            slot,
            preload,
            name,
            task: Some(task),
            job: None,
            raw: None,
            stage: 0,
            opaque: None,
            has_mask: None,
            ready: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<PreparedPicture>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        if let Some(task) = self.task.as_mut() {
            let Some(source) = task.try_take()? else {
                return Ok(None);
            };
            self.has_mask = Some(source.has_mask);
            self.job = Some(
                host.image_metadata(self.client)?
                    .begin_process(source.input, b"[]".as_slice().into())?,
            );
            self.api_calls += 1;
            self.task = None;
        }
        let Some(job) = self.job.as_mut() else {
            bail!("NNN image dependency missing")
        };
        match job.poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => bail!("NNN shared image decoder: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(image.client() == self.client, "NNN foreign decoded image");
                if self.stage == 0 {
                    self.raw = Some(image.clone());
                    // Retain a separate engine-owned opaque view for native
                    // alpha-ignore operations, including CTRANS without mask.
                    // Do not destroy the original bytes or infer mask presence.
                    self.stage = 1;
                    if !image.opaque() {
                        self.job = Some(
                            host.image_metadata(self.client)?.begin_process(
                                ImageInput::Resource(image),
                                serde_json::to_vec(&[
                                    crysirror_image::effects::ImageEffect::OverrideAlpha {
                                        alpha: 255,
                                    },
                                ])?
                                .into(),
                            )?,
                        );
                        self.api_calls += 1;
                        return Ok(None);
                    }
                }
                if self.stage == 1 {
                    self.opaque = Some(image.clone());
                    self.stage = 2;
                    if !self.has_mask.unwrap() {
                        self.job = Some(
                            host.image_metadata(self.client)?.begin_process(
                                ImageInput::Resource(self.raw.clone().unwrap()),
                                serde_json::to_vec(&[
                                    crysirror_image::effects::ImageEffect::SpriteAlpha {
                                        spec:
                                            crysirror_image::sprite_alpha::SpriteAlpha::CornerKey {
                                                right: false,
                                            },
                                    },
                                ])?
                                .into(),
                            )?,
                        );
                        self.api_calls += 1;
                        return Ok(None);
                    }
                }
                let keyed = if self.has_mask.unwrap() {
                    self.raw.clone().unwrap()
                } else {
                    image
                };
                let display = if self.preload {
                    self.raw.clone().unwrap()
                } else if self.slot < 4 || self.slot == 14 {
                    self.opaque.clone().unwrap()
                } else {
                    keyed.clone()
                };
                let prepared = PreparedPicture {
                    has_mask: self.has_mask.unwrap(),
                    slot: self.slot,
                    preload: self.preload,
                    name: self.name.clone(),
                    raw: self.raw.clone().unwrap(),
                    opaque: self.opaque.clone().unwrap(),
                    display,
                    keyed,
                };
                self.ready = Some(prepared.clone());
                Ok(Some(prepared))
            }
        }
    }
}
