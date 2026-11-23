// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Owner coordinator for detached native-save jobs. No legacy renderer, VM
//! decoder or engine-specific save interpretation belongs here.
use super::{Runtime, SharedHost};
use crate::published_images::PreparedImages;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    image_processing::ImageResource,
    native_restore::{
        OriginalRestoreCommit, OriginalRestorePreparation, OriginalRestoreProgress,
        OriginalRestoreStatus, OriginalRestoreTicket,
    },
    path::{Mount, VfsPath},
    service::{
        AssetReader, ClientStorage, FileSystem, HostServices, ImageMetadata, StorageMetadata,
        TextMeasurement, UploadedImage,
    },
    state::{ClientId, Command, Limits},
    vm::VmYield,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
const MAX_RECEIPTS: usize = 64;
const MAX_INPUT: usize = 16 * 1024 * 1024;
const MAX_PENDING_BYTES: usize = 64 * 1024 * 1024;
#[path = "native_restore/requests.rs"]
mod requests;
pub(super) use requests::Requests;
struct Entry {
    ticket: OriginalRestoreTicket,
    status: OriginalRestoreStatus,
    job: Option<Box<dyn OriginalRestorePreparation>>,
    images: Option<PreparedImages>,
    plan: Option<OriginalRestoreCommit>,
    batches: Option<Vec<crysirror_scene::ServiceBatch>>,
    validation: Option<crysirror_tasks::Task<Vec<crysirror_scene::ServiceBatch>>>,
    engine: &'static str,
    schema: u32,
    clock_ms: u64,
    bytes: usize,
}
pub(super) struct Coordinator {
    owner: u64,
    epoch: u64,
    next: u64,
    bytes: usize,
    validation_pool: crysirror_tasks::TaskPool,
    entries: BTreeMap<u64, Entry>,
    active: BTreeMap<ClientId, u64>,
}
impl Coordinator {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            owner: NEXT_OWNER
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| Error::Limit("restore runtime identities"))?,
            epoch: 0,
            next: 0,
            bytes: 0,
            validation_pool: crysirror_tasks::TaskPool::new("native-restore-validate", 1, 64)?,
            entries: BTreeMap::new(),
            active: BTreeMap::new(),
        })
    }
    pub(super) fn active(&self, client: ClientId) -> bool {
        self.active.contains_key(&client)
    }
    pub(super) fn any_active(&self) -> bool {
        !self.active.is_empty()
    }
    fn entry(&self, client: ClientId, ticket: OriginalRestoreTicket) -> Result<&Entry> {
        ensure!(
            ticket.owner == self.owner && ticket.client == client,
            "foreign original restore ticket"
        );
        let entry = self
            .entries
            .get(&ticket.serial)
            .context("original restore ticket released/missing")?;
        ensure!(entry.ticket == ticket, "obsolete original restore ticket");
        Ok(entry)
    }
    fn entry_mut(&mut self, client: ClientId, ticket: OriginalRestoreTicket) -> Result<&mut Entry> {
        self.entry(client, ticket)?;
        Ok(self.entries.get_mut(&ticket.serial).unwrap())
    }
    pub(super) fn cancel_all(&mut self) {
        self.invalidate(self.epoch.saturating_add(1));
    }
    // Called only after the issued ticket and every fallible plan operation
    // have been validated under this exclusive owner borrow.
    fn committed(&mut self, ticket: OriginalRestoreTicket) {
        let entry = self
            .entries
            .get_mut(&ticket.serial)
            .expect("validated restore receipt");
        self.bytes -= entry.bytes;
        entry.bytes = 0;
        entry.status = OriginalRestoreStatus::Committed;
        entry.job = None;
        entry.images = None;
        self.active.remove(&ticket.client);
    }
    pub(super) fn inspect(&self) -> serde_json::Value {
        let entries:Vec<_>=self.entries.values().map(|entry| {
            let (state,error)=match &entry.status {
                OriginalRestoreStatus::Preparing=>("preparing",None),OriginalRestoreStatus::Ready=>("ready",None),
                OriginalRestoreStatus::Committed=>("committed",None),OriginalRestoreStatus::Cancelled=>("cancelled",None),
                OriginalRestoreStatus::Failed(error)=>("failed",Some(error.to_string().chars().take(1024).collect::<String>())),
            };
            let (images,image_bytes)=entry.images.as_ref().map_or((0,0),PreparedImages::stats);
            serde_json::json!({"client":entry.ticket.client.0,"serial":entry.ticket.serial.to_string(),"epoch":entry.ticket.epoch.to_string(),"state":state,"error":error,"dependency":if entry.validation.is_some(){"scene_validation"}else if entry.job.is_some(){"native_resources"}else{"none"},"retained_bytes":entry.bytes,"reserved_images":images,"reserved_image_bytes":image_bytes})
        }).collect();
        serde_json::json!({"epoch":self.epoch.to_string(),"pending_bytes":self.bytes,"validation_tasks":self.validation_pool.stats(),"entries":entries})
    }
    pub(super) fn next_epoch(&self) -> Result<u64> {
        self.epoch
            .checked_add(1)
            .context("original restore epoch exhausted")
    }
    pub(super) fn invalidate(&mut self, epoch: u64) {
        for entry in self.entries.values_mut() {
            if matches!(
                entry.status,
                OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready
            ) {
                entry.status = OriginalRestoreStatus::Cancelled;
                entry.job = None;
                entry.plan = None;
                entry.batches = None;
                entry.validation = None;
                entry.images = None;
                entry.bytes = 0;
            }
        }
        self.bytes = 0;
        self.active.clear();
        self.epoch = epoch;
    }
    fn end(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
        status: OriginalRestoreStatus,
    ) -> Result<()> {
        let bytes = {
            let entry = self.entry_mut(client, ticket)?;
            entry.status = status;
            entry.job = None;
            entry.plan = None;
            entry.batches = None;
            entry.validation = None;
            entry.images = None;
            std::mem::take(&mut entry.bytes)
        };
        self.bytes -= bytes;
        self.active.remove(&client);
        Ok(())
    }
}
fn plan_extent(plan: &OriginalRestoreCommit) -> Result<usize> {
    if !plan.audio.is_empty()
        || plan.commands.iter().any(|command| {
            matches!(
                command,
                Command::PlayAudio { .. }
                    | Command::StopAudio { .. }
                    | Command::SetAudioGain { .. }
            )
        })
    {
        return Err(Error::Unsupported(
            "atomic native audio restore requires prepared replacement channels",
        )
        .into());
    }
    ensure!(
        plan.commands.len() <= 4096
            && plan.engine_batches.len() <= 128
            && plan.release_images.len() <= 4096,
        "original restore plan limit"
    );
    let mut bytes = plan
        .commands
        .len()
        .checked_mul(std::mem::size_of::<Command>())
        .context("restore command extent")?;
    for command in &plan.commands {
        let extra = match command {
            Command::PutVisual { node, .. } => match &node.visual {
                crysirror_core::state::Visual::Text(text) => text
                    .text
                    .len()
                    .saturating_add(text.font.len())
                    .saturating_add(text.layout_profile.len()),
                crysirror_core::state::Visual::Sprite(sprite) => sprite.asset.relative().len(),
            },
            _ => 0,
        };
        bytes = bytes.checked_add(extra).context("restore command bytes")?;
    }
    for batch in &plan.engine_batches {
        bytes = bytes
            .checked_add(batch.bytes.len())
            .context("restore scene bytes")?;
    }
    for path in &plan.release_images {
        bytes = bytes
            .checked_add(path.len())
            .context("restore release bytes")?;
    }
    ensure!(bytes <= MAX_INPUT, "original restore plan byte limit");
    Ok(bytes)
}
/// A storage lease has no write/remove/lease authority while preparing a load.
struct ReadOnlyStorage(Arc<dyn ClientStorage>);
impl ClientStorage for ReadOnlyStorage {
    fn metadata(&self, m: Mount, p: &str) -> crysirror_core::Result<Option<StorageMetadata>> {
        self.0.metadata(m, p)
    }
    fn read(&self, p: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        self.0.read(p)
    }
    fn open_asset(&self, p: &VfsPath) -> crysirror_core::Result<Arc<dyn AssetReader>> {
        self.0.open_asset(p)
    }
    fn open_read(&self, p: &VfsPath) -> crysirror_core::Result<Arc<dyn AssetReader>> {
        self.0.open_read(p)
    }
    fn container_mount(&self, p: &str) -> crysirror_core::Result<Option<String>> {
        self.0.container_mount(p)
    }
    fn served_by_container(&self, p: &VfsPath) -> bool {
        self.0.served_by_container(p)
    }
    fn serving_container(&self, p: &VfsPath) -> Option<(String, String)> {
        self.0.serving_container(p)
    }
    fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::ReadOnly)
    }
    fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
        Err(Error::ReadOnly)
    }
    fn list(&self, m: Mount, p: &str) -> crysirror_core::Result<Vec<String>> {
        self.0.list(m, p)
    }
    fn exists(&self, p: &VfsPath) -> bool {
        self.0.exists(p)
    }
}
struct ResourceHost<'a> {
    source: &'a mut SharedHost,
    client: ClientId,
    images: &'a mut PreparedImages,
    clock_ms: u64,
}
impl ResourceHost<'_> {
    fn scope(&self, client: ClientId) -> crysirror_core::Result<()> {
        if client == self.client {
            Ok(())
        } else {
            Err(Error::Invalid("foreign native restore resource client"))
        }
    }
}
impl FileSystem for ResourceHost<'_> {
    fn read(&mut self, c: ClientId, p: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        self.storage(c)?.read(p)
    }
    fn write_atomic(&mut self, _: ClientId, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::ReadOnly)
    }
}
impl HostServices for ResourceHost<'_> {
    fn now_ms(&self) -> u64 {
        self.clock_ms
    }
    fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "native restore preparation scene mutation",
        ))
    }
    fn image_metadata(&mut self, c: ClientId) -> crysirror_core::Result<Arc<dyn ImageMetadata>> {
        self.scope(c)?;
        self.source.image_metadata(c)
    }
    fn text_measurement(
        &mut self,
        c: ClientId,
    ) -> crysirror_core::Result<Arc<dyn TextMeasurement>> {
        self.scope(c)?;
        self.source.text_measurement(c)
    }
    fn storage(&mut self, c: ClientId) -> crysirror_core::Result<Arc<dyn ClientStorage>> {
        self.scope(c)?;
        Ok(Arc::new(ReadOnlyStorage(self.source.storage(c)?)))
    }
    fn publish_image_resource(
        &mut self,
        c: ClientId,
        id: u32,
        image: Arc<dyn ImageResource>,
    ) -> crysirror_core::Result<UploadedImage> {
        self.scope(c)?;
        self.images.reserve(id, image)
    }
    fn release_image_resource(&mut self, c: ClientId, path: &str) -> crysirror_core::Result<()> {
        self.scope(c)?;
        self.images.release(path)
    }
    fn published_image_resource(&mut self, c: ClientId, path: &str)
        -> crysirror_core::Result<Option<Arc<dyn ImageResource>>> {
        self.scope(c)?;
        Ok(self.images.resource(path))
    }
    fn begin_pattern(
        &mut self,
        c: ClientId,
        request: crysirror_core::service::PatternRequest,
    ) -> crysirror_core::Result<crysirror_core::service::PatternJob> {
        self.scope(c)?;
        self.source.begin_pattern(c, request)
    }
    fn begin_media_probe(
        &mut self,
        c: ClientId,
        path: &VfsPath,
    ) -> crysirror_core::Result<crysirror_core::service::MediaProbeJob> {
        self.scope(c)?;
        self.source.begin_media_probe(c, path)
    }
    fn audio_codec_supported(&self, c: ClientId, codec: &str) -> crysirror_core::Result<bool> {
        self.scope(c)?;
        self.source.audio_codec_supported(c, codec)
    }
    fn begin_audio_replacement(
        &mut self,
        c: ClientId,
        requests: Vec<crysirror_core::service::AudioRequest>,
    ) -> crysirror_core::Result<crysirror_core::service::AudioReplacementJob> {
        self.scope(c)?;
        self.source.begin_audio_replacement(c, requests)
    }
}
impl Runtime {
    /// Begin at an adapter-defined native safepoint. Only this client's VM is
    /// held; preparation never steps it or changes engine time/input/playback.
    pub fn begin_original_save_restore(
        &mut self,
        client: ClientId,
        bytes: Arc<[u8]>,
    ) -> Result<OriginalRestoreTicket> {
        ensure!(bytes.len() <= MAX_INPUT, "original restore input limit");
        ensure!(
            self.native_restores.entries.len() < MAX_RECEIPTS
                && !self.native_restores.active(client),
            "original restore receipt/client busy limit"
        );
        let total = self
            .native_restores
            .bytes
            .checked_add(bytes.len())
            .context("restore input extent")?;
        ensure!(
            total <= MAX_PENDING_BYTES,
            "original restore pending byte limit"
        );
        let session = self
            .sessions
            .iter()
            .find(|session| session.id == client)
            .context("native client not registered")?;
        let serial = self
            .native_restores
            .next
            .checked_add(1)
            .context("original restore serial exhausted")?;
        let len = bytes.len();
        let job = session.vm.begin_original_restore(bytes)?;
        ensure!(
            job.client() == client,
            "original restore preparation returned foreign client"
        );
        let ticket = OriginalRestoreTicket {
            owner: self.native_restores.owner,
            client,
            epoch: self.native_restores.epoch,
            serial,
        };
        self.native_restores.next = serial;
        self.native_restores.bytes = total;
        self.native_restores.entries.insert(
            serial,
            Entry {
                ticket,
                status: OriginalRestoreStatus::Preparing,
                job: Some(job),
                plan: None,
                batches: None,
                validation: None,
                engine: session.vm.engine_id(),
                schema: session.vm.snapshot_schema(),
                images: Some(PreparedImages::new(
                    self.host.published_images.clone(),
                    client,
                )),
                clock_ms: self.host.clock_ms,
                bytes: len,
            },
        );
        self.native_restores.active.insert(client, serial);
        Ok(ticket)
    }
    /// Poll only existing native/resource/validation dependencies. The generic
    /// JSON decode runs on a bounded CPU worker, never on the owner commit.
    pub fn poll_original_save_restore(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> Result<OriginalRestoreStatus> {
        let outcome = (|| -> Result<()> {
            let entry = self.native_restores.entry_mut(client, ticket)?;
            if entry.status != OriginalRestoreStatus::Preparing {
                return Ok(());
            }
            if let Some(task) = &mut entry.validation {
                if let Some(batches) = task.try_take()? {
                    entry.batches = Some(batches);
                    entry.validation = None;
                    entry.status = OriginalRestoreStatus::Ready;
                }
                return Ok(());
            }
            let mut resources = ResourceHost {
                source: &mut self.host,
                client,
                images: entry.images.as_mut().unwrap(),
                clock_ms: entry.clock_ms,
            };
            if entry.job.as_mut().unwrap().poll(&mut resources)? == OriginalRestoreProgress::Pending
            {
                return Ok(());
            }
            let mut plan = entry.job.take().unwrap().finish()?;
            ensure!(
                plan.vm.engine_id() == entry.engine && plan.vm.snapshot_schema() == entry.schema,
                "original restore VM identity/schema changed"
            );
            if let Some(audio) = plan.prepared_audio.as_ref() {
                crate::audio_replacement::validate(
                    &self.host.audio_replacement_owner,
                    client,
                    audio,
                )?;
            }
            let extra = plan_extent(&plan)?;
            let total = self
                .native_restores
                .bytes
                .checked_add(extra)
                .context("restore plan extent")?;
            ensure!(
                total <= MAX_PENDING_BYTES,
                "original restore input/plan byte limit"
            );
            let permit = self.native_restores.validation_pool.reserve()?;
            let encoded = std::mem::take(&mut plan.engine_batches);
            let validation = permit.spawn(move |cancel| {
                let mut batches = Vec::with_capacity(encoded.len());
                for encoded in encoded {
                    cancel.check()?;
                    ensure!(encoded.schema == 1, "original restore scene schema");
                    let batch: crysirror_scene::ServiceBatch =
                        serde_json::from_slice(&encoded.bytes)?;
                    ensure!(
                        batch.world.is_empty(),
                        "native restore World requires prepared simulation ownership"
                    );
                    batches.push(batch);
                }
                cancel.check()?;
                Ok(batches)
            })?;
            self.native_restores.bytes = total;
            let entry = self.native_restores.entry_mut(client, ticket)?;
            entry.bytes += extra;
            entry.plan = Some(plan);
            entry.validation = Some(validation);
            Ok(())
        })();
        if let Err(error) = outcome {
            let failure = error
                .downcast_ref::<Error>()
                .cloned()
                .unwrap_or_else(|| Error::Io(format!("{error:#}")));
            self.native_restores
                .end(client, ticket, OriginalRestoreStatus::Failed(failure))?;
        }
        self.original_save_restore_status(client, ticket)
    }
    pub fn original_save_restore_status(
        &self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> Result<OriginalRestoreStatus> {
        Ok(self.native_restores.entry(client, ticket)?.status.clone())
    }
    pub fn cancel_original_save_restore(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> Result<()> {
        let status = &self.native_restores.entry(client, ticket)?.status;
        ensure!(
            matches!(
                status,
                OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready
            ),
            "original restore is already terminal"
        );
        self.native_restores
            .end(client, ticket, OriginalRestoreStatus::Cancelled)
    }
    pub fn release_original_save_restore(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> Result<()> {
        if matches!(
            self.native_restores.entry(client, ticket)?.status,
            OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready
        ) {
            self.cancel_original_save_restore(client, ticket)?;
        }
        self.native_restores.entries.remove(&ticket.serial);
        Ok(())
    }
    /// Atomically install a ready native VM, validated 2D scene and an optional
    /// detached channel replacement. Sequential audio, Media/World producers
    /// and classic scene audio are not silently approximated.
    pub fn commit_original_save_restore(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> Result<()> {
        ensure!(
            self.audio_frame.is_none() && !self.host.channel_audio.mixer.is_rendering(),
            "audio frame pending before original restore commit"
        );
        let entry = self.native_restores.entry(client, ticket)?;
        ensure!(
            entry.ticket.epoch == self.native_restores.epoch
                && entry.status == OriginalRestoreStatus::Ready,
            "original restore not ready/current"
        );
        let plan = self
            .native_restores
            .entry_mut(client, ticket)?
            .plan
            .take()
            .unwrap();
        let batches = self
            .native_restores
            .entry_mut(client, ticket)?
            .batches
            .take()
            .unwrap();
        let outcome = (|| -> Result<()> {
            let index = self
                .sessions
                .iter()
                .position(|session| session.id == client)
                .context("native client not registered")?;
            ensure!(
                plan.vm.engine_id() == self.sessions[index].vm.engine_id()
                    && plan.vm.snapshot_schema() == self.sessions[index].vm.snapshot_schema(),
                "original restore VM identity/schema changed"
            );
            if self.host.media.has_client_resources(client) {
                return Err(Error::Unsupported("native restore Media producer ownership").into());
            }
            if !self.host.engine.world.entities(client).is_empty() {
                return Err(Error::Unsupported("native restore World producer ownership").into());
            }
            if self
                .host
                .scene
                .voices()
                .keys()
                .any(|id| id.client == client)
            {
                return Err(
                    Error::Unsupported("native restore classic scene audio ownership").into(),
                );
            }
            if !plan.audio.is_empty() {
                return Err(Error::Unsupported(
                    "atomic native audio restore requires prepared replacement channels",
                )
                .into());
            }
            ensure!(
                plan.commands.len() <= 4096
                    && plan.engine_batches.len() <= 128
                    && plan.release_images.len() <= 4096,
                "original restore plan limit"
            );
            ensure!(
                plan.commands.iter().all(|command| !matches!(
                    command,
                    Command::PlayAudio { .. }
                        | Command::StopAudio { .. }
                        | Command::SetAudioGain { .. }
                )),
                "atomic native audio restore requires prepared replacement channels"
            );
            let mut engine = self.host.engine.clone();
            let mut scene = self.host.scene.clone();
            scene.apply_batch(client, &plan.commands, Limits::default())?;
            for batch in &batches {
                engine.submit(client, batch)?;
            }
            engine.ui.reset_client_input_after_restore(client);
            engine.input_routes.reset_client_input_after_restore(client);
            if engine.layers.has_external_motion_producers(client) {
                return Err(
                    Error::Unsupported("native restore external producer ownership").into(),
                );
            }
            let mut fonts = BTreeSet::new();
            engine.visit_font_dependencies(&mut |owner, font| {
                if owner == client {
                    fonts.insert(font.to_owned());
                }
            });
            for (id, node) in scene.visuals() {
                if id.client == client {
                    if let crysirror_core::state::Visual::Text(text) = &node.visual {
                        fonts.insert(text.font.clone());
                    }
                }
            }
            for font in fonts {
                // A dynamic catalog can be released by a capability on another
                // thread. is_loaded() alone is not a transactional font lease.
                // This profile accepts only the host-owned immutable static
                // aliases, which cannot change under this exclusive Runtime.
                if self.host.text.static_font_alias(&font).as_deref() != Some(font.as_str()) {
                    return Err(Error::Unsupported(
                        "native restore needs an installed static font or prepared font ownership",
                    )
                    .into());
                }
            }
            let epoch = self.native_restores.next_epoch()?;
            let restored = self.restored.checked_add(1).context("restore counter")?;
            let mut old_references = BTreeSet::new();
            self.host.engine.visit_image_dependencies(&mut |c, path| {
                old_references.insert((c, path.to_owned()));
            });
            for (id, node) in self.host.scene.visuals() {
                if let crysirror_core::state::Visual::Sprite(sprite) = &node.visual {
                    old_references.insert((id.client, sprite.asset.relative().to_owned()));
                }
            }
            let mut referenced = BTreeSet::new();
            let complete = engine.visit_image_dependencies(&mut |c, path| {
                referenced.insert((c, path.to_owned()));
            });
            referenced.extend(self.host.vfx_image_dependencies.iter().cloned());
            for (id, node) in scene.visuals() {
                if let crysirror_core::state::Visual::Sprite(sprite) = &node.visual {
                    referenced.insert((id.client, sprite.asset.relative().to_owned()));
                }
            }
            for (owner, path) in referenced.difference(&old_references) {
                if *owner == client {
                    let staged = self
                        .native_restores
                        .entry(client, ticket)?
                        .images
                        .as_ref()
                        .unwrap()
                        .contains(path);
                    ensure!(
                        path.starts_with("__image_resource/")
                            && (staged || self.host.published_images.get(client, path).is_some()),
                        "new restore image reference is not a ready immutable resource"
                    );
                }
            }

            let images = self
                .native_restores
                .entry_mut(client, ticket)?
                .images
                .take()
                .unwrap();
            // The exclusive token runs every fallible audio check before
            // image publication. Its install cannot fail or copy peer DSP.
            let audio = plan
                .prepared_audio
                .as_ref()
                .map(|source| {
                    crate::audio_replacement::take(
                        &self.host.audio_replacement_owner,
                        client,
                        source,
                    )
                })
                .transpose()?;
            let audio_commit = audio
                .map(|prepared| {
                    self.host
                        .channel_audio
                        .stage_client_replacement(client.0, prepared)
                })
                .transpose()?;
            images.commit(&plan.release_images, &referenced, !complete)?;
            if let Some(commit) = audio_commit {
                commit.install();
            }
            self.restored = restored;
            self.host.engine = engine;
            self.host.scene_checkpoint_epochs.remove(&client);
            self.host.text_input.retire_client(client);
            self.host.scene = scene;
            self.sessions[index].vm = plan.vm;
            self.sessions[index].yielded = VmYield::BudgetExhausted;
            self.host.native_restore_requests.invalidate_except(Some(ticket));
            self.native_restores.committed(ticket);
            self.native_restores.invalidate(epoch);
            Ok(())
        })();
        if let Err(error) = outcome {
            self.native_restores.end(
                client,
                ticket,
                OriginalRestoreStatus::Failed(
                    error
                        .downcast_ref::<Error>()
                        .cloned()
                        .unwrap_or_else(|| Error::Io(format!("{error:#}"))),
                ),
            )?;
            return Err(error);
        }
        Ok(())
    }
}
