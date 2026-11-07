// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned vm_reset/vm_execute_ain/library lifecycle and original
// Sys42VM100248F0 reset callback. Reset is a nonreturning native control request,
// never an alias for Exit or an input-file signature accepted as a save.
use super::*;
use crysirror_core::{
    native_restore::{
        OriginalRestoreBatch, OriginalRestoreCommit, OriginalRestorePreparation,
        OriginalRestoreProgress, OriginalRestoreStatus, OriginalRestoreTicket,
    },
    service::{AudioReplacementJob, AudioReplacementResource, AudioReplacementState, EngineQuery},
};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
static NEXT_RESET: AtomicU64 = AtomicU64::new(1);
const TOKEN: &[u8] = b"CrySirror.System4.NativeReset.1\0";
pub(super) struct ResetState {
    call: crate::vm::ExternalCall,
    nonce: u64,
    task: Option<Task<Prepared>>,
    prepared: Arc<Mutex<Option<Prepared>>>,
    todo: Vec<u32>,
    next: usize,
    existing: Vec<u32>,
    images: Vec<String>,
    canvas: bool,
    ticket: Option<OriginalRestoreTicket>,
    irreversible: bool,
}
struct Snapshot {
    owner: ClientId,
    config: Config,
    program: Arc<Program>,
    assets: Option<Arc<crate::assets::Assets>>,
    bootstrap: Option<crate::bootstrap::Bootstrap>,
    debug_log: Option<AbsentDebugLog>,
    factory_created: bool,
}
struct Prepared {
    adapter: Box<NativeAdapter>,
}
impl ResetState {
    fn token(&self) -> Arc<[u8]> {
        let mut bytes = TOKEN.to_vec();
        bytes.extend(self.nonce.to_le_bytes());
        bytes.extend(self.call.pc.to_le_bytes());
        bytes.into()
    }
    fn accepts(&self, bytes: &[u8]) -> bool {
        self.ticket.is_some() && self.token().as_ref() == bytes
    }
}
struct ResetJob {
    owner: ClientId,
    slot: Arc<Mutex<Option<Prepared>>>,
    prepared: Option<Prepared>,
    scope: Option<Vec<OriginalRestoreBatch>>,
    scope_task: Option<Task<Vec<OriginalRestoreBatch>>>,
    scope_ids: Option<Vec<u32>>,
    scope_canvas: bool,
    pool: TaskPool,
    images: Vec<String>,
    audio: Option<AudioReplacementJob>,
    audio_ready: Option<Arc<dyn AudioReplacementResource>>,
}
impl OriginalRestorePreparation for ResetJob {
    fn client(&self) -> ClientId {
        self.owner
    }
    fn poll(
        &mut self,
        resources: &mut dyn HostServices,
    ) -> crysirror_core::Result<OriginalRestoreProgress> {
        if self.prepared.is_none() {
            self.prepared = self
                .slot
                .lock()
                .map_err(|_| Error::Invalid("System4 reset preparation lock"))?
                .take();
            if self.prepared.is_none() {
                return Err(Error::Invalid("System4 reset preparation consumed"));
            }
        }
        if self.scope.is_none() {
            if self.scope_task.is_none() {
                let ids = self
                    .scope_ids
                    .take()
                    .ok_or(Error::Invalid("System4 reset scope consumed"))?;
                let canvas = self.scope_canvas;
                self.scope_task = Some(
                    self.pool
                        .spawn(move |cancel| scope_batches(ids, canvas, &cancel))
                        .map_err(core_error)?,
                );
            }
            if let Some(scope) = self
                .scope_task
                .as_mut()
                .unwrap()
                .try_take()
                .map_err(core_error)?
            {
                self.scope_task = None;
                self.scope = Some(scope);
            }
        }
        if self.audio_ready.is_none() {
            if self.audio.is_none() {
                self.audio = Some(resources.begin_audio_replacement(self.owner, Vec::new())?);
            }
            let job = self.audio.as_mut().unwrap();
            if job.client() != self.owner {
                return Err(Error::Invalid("System4 foreign reset audio preparation"));
            }
            match job.poll()? {
                AudioReplacementState::Pending => return Ok(OriginalRestoreProgress::Pending),
                AudioReplacementState::Failed(error) => return Err(error),
                AudioReplacementState::Ready(resource) => {
                    if resource.client() != self.owner || resource.info().channels != 0 {
                        return Err(Error::Invalid("System4 reset audio replacement receipt"));
                    }
                    self.audio_ready = Some(resource);
                    self.audio = None;
                }
            }
        }
        Ok(if self.scope.is_some() {
            OriginalRestoreProgress::Ready
        } else {
            OriginalRestoreProgress::Pending
        })
    }
    fn finish(mut self: Box<Self>) -> crysirror_core::Result<OriginalRestoreCommit> {
        let prepared = self
            .prepared
            .take()
            .ok_or(Error::Invalid("System4 reset VM not ready"))?;
        let audio = self
            .audio_ready
            .take()
            .ok_or(Error::Invalid("System4 reset audio not ready"))?;
        Ok(OriginalRestoreCommit {
            vm: prepared.adapter,
            commands: Vec::new(),
            engine_batches: self
                .scope
                .take()
                .ok_or(Error::Invalid("System4 reset scope not ready"))?,
            audio: Vec::new(),
            prepared_audio: Some(audio),
            release_images: std::mem::take(&mut self.images),
        })
    }
}
impl NativeAdapter {
    pub(super) fn reset_job(
        &self,
        bytes: Arc<[u8]>,
    ) -> Result<Box<dyn OriginalRestorePreparation>> {
        let state = self
            .restart
            .as_ref()
            .context("System4 reset control request absent")?;
        ensure!(
            state.accepts(&bytes) && state.irreversible && state.next == state.todo.len(),
            "System4 unknown/stale reset control token"
        );
        let machine = self
            .machine
            .as_ref()
            .context("System4 reset lost original VM")?;
        machine.validate_system(&state.call)?;
        ensure!(
            matches!(state.call.target, crate::vm::ExternalTarget::System(5)),
            "System4 reset is not current original SYS5"
        );
        ensure!(
            state
                .prepared
                .lock()
                .map_err(|_| anyhow::anyhow!("System4 reset preparation lock"))?
                .is_some(),
            "System4 reset preparation already consumed"
        );
        ensure!(
            state.existing.len() <= 65536 && state.images.len() <= 4096,
            "System4 reset scope budget"
        );
        Ok(Box::new(ResetJob {
            owner: self.client,
            slot: state.prepared.clone(),
            prepared: None,
            scope: None,
            scope_task: None,
            scope_ids: Some(state.existing.clone()),
            scope_canvas: state.canvas,
            pool: self.pool.clone(),
            images: state.images.clone(),
            audio: None,
            audio_ready: None,
        }))
    }
    pub(super) fn advance_reset(
        &mut self,
        call: &crate::vm::ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<VmYield> {
        self.machine
            .as_ref()
            .context("System4 reset original VM absent")?
            .validate_system(call)?;
        ensure!(
            matches!(call.target, crate::vm::ExternalTarget::System(5)),
            "System4 reset call target"
        );
        if self.restart.is_none() {
            let permit = self.pool.reserve()?;
            let m = self.machine.as_ref().unwrap();
            let snapshot = Snapshot {
                owner: self.client,
                config: self.config.clone(),
                program: m.program.clone(),
                assets: self.bridge.assets.clone(),
                bootstrap: self.bootstrap.clone(),
                debug_log: self.bridge.debug_log.clone(),
                factory_created: self.factory_created,
            };
            let storage = host.storage(self.client)?;
            let nonce = NEXT_RESET
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| anyhow::anyhow!("System4 reset nonce exhausted"))?;
            let mut todo = BTreeSet::from([1, 2, 16,crate::sact::BACK_SCENE]);
            if let Some(sprites) = self.bridge.sact.as_ref().and_then(|s| s.sprites.as_ref()) {
                todo.extend(sprites.owned_layer_ids()?);
            }
            ensure!(todo.len() <= 65536, "System4 reset owned layer budget");
            let mut images = self.native_cg.published_paths();
            if let Some(path) = self.native_scene.published_path() {
                images.push(path.to_owned());
            }
            images.extend(self.native_text.published_paths());
            images.sort();
            images.dedup();
            ensure!(
                images.len() <= 4096,
                "System4 reset image retirement budget"
            );
            let task = permit.spawn(move |cancel| prepare(snapshot, storage, &cancel))?;
            self.restart = Some(ResetState {
                call: call.clone(),
                nonce,
                task: Some(task),
                prepared: Arc::new(Mutex::new(None)),
                todo: todo.into_iter().collect(),
                next: 0,
                existing: Vec::new(),
                images,
                canvas: false,
                ticket: None,
                irreversible: false,
            });
        }
        let state = self.restart.as_mut().unwrap();
        ensure!(state.call == *call, "System4 changed reset PC");
        if let Some(ticket) = state.ticket {
            match host.original_save_restore_request_status(self.client,ticket)? {
                OriginalRestoreStatus::Preparing|OriginalRestoreStatus::Ready=>return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1))),
                OriginalRestoreStatus::Failed(error)=>anyhow::bail!("System4 native reset failed after library-exit prelude; old VM cannot resume: {error}"),
                OriginalRestoreStatus::Cancelled=>anyhow::bail!("System4 native reset cancelled after library-exit prelude; old VM cannot resume"),
                OriginalRestoreStatus::Committed=>anyhow::bail!("System4 old reset VM survived atomic replacement"),
            }
        }
        if let Some(task) = &mut state.task {
            let Some(prepared) = task.try_take()? else {
                return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
            };
            state.task = None;
            *state
                .prepared
                .lock()
                .map_err(|_| anyhow::anyhow!("System4 reset preparation lock"))? = Some(prepared);
        }
        // A fully constructed replacement exists BEFORE old library teardown.
        // File/read-history flush and Media/audio/IME exit are real irreversible
        // native prelude effects. A later fault never returns to the old script.
        state.irreversible = true;
        match self.native_file.close(host)? {
            Dispatch::WaitUntil(time) => return Ok(VmYield::WaitUntil(time)),
            Dispatch::Completed => {}
            Dispatch::Presented => anyhow::bail!("System4 reset File flush presentation"),
        }
        match self.native_msgskip.flush(self.client, host)? {
            Dispatch::WaitUntil(time) => return Ok(VmYield::WaitUntil(time)),
            Dispatch::Completed => {}
            Dispatch::Presented => anyhow::bail!("System4 reset history flush presentation"),
        }
        self.native_input_string.close(host)?;
        self.native_dialog.close(host)?;
        self.native_movie.close(host)?;
        self.native_logo.close(host)?;
        self.native_text.close(host)?;
        self.native_present.cancel();
        self.native_wp.cancel();
        self.native_effect.cancel();
        self.native_save_cg.cancel();
        self.native_csv.cancel();
        self.native_audio.close(host)?;
        let end = (state.next + 128).min(state.todo.len());
        for id in &state.todo[state.next..end] {
            let bytes = host.query_engine(self.client, EngineQuery::Layer(*id))?;
            ensure!(
                bytes.len() <= 1024 * 1024,
                "System4 reset layer observation budget"
            );
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            if !value.is_null() {
                state.existing.push(*id);
            }
        }
        state.next = end;
        if state.next < state.todo.len() {
            return Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)));
        }
        if state.existing.contains(&crate::sact::BACK_SCENE){
            // Shared removal recursively retires descendants. Submitting both
            // the hidden back root and its sprite/glyph children would double
            // remove them and violate the transactional Layer contract.
            state.existing.retain(|id|*id==crate::sact::BACK_SCENE||*id==16);
        }
        let canvas = host.query_engine(self.client, EngineQuery::Canvas(1))?;
        ensure!(
            canvas.len() <= 1024 * 1024,
            "System4 reset canvas observation budget"
        );
        state.canvas = !serde_json::from_slice::<serde_json::Value>(&canvas)?.is_null();
        // Images and layers remain live until the coordinator validates every
        // new VM/scene/audio dependency and installs one replacement for this client.
        let ticket = host.request_original_save_restore(self.client, state.token())?;
        ensure!(
            ticket.client == self.client,
            "System4 foreign reset request ticket"
        );
        state.ticket = Some(ticket);
        Ok(VmYield::WaitUntil(host.now_ms().saturating_add(1)))
    }
    pub(super) fn cancel_reset(&mut self) {
        self.restart = None;
    }
}
fn prepare(
    snapshot: Snapshot,
    storage: Arc<dyn crysirror_core::service::ClientStorage>,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Prepared> {
    cancel.check()?;
    let mut adapter = NativeAdapter::new(snapshot.owner, snapshot.config.clone())?;
    let mut machine = Machine::new(snapshot.program)?;
    machine.reset_once = true;
    adapter.machine = Some(machine);
    adapter.started = true;
    adapter.boot_ready = true;
    adapter.stop = Some(Step::Advanced);
    adapter.factory_created = snapshot.factory_created;
    adapter.bridge.assets = snapshot.assets;
    adapter.bridge.debug_log = snapshot.debug_log;
    if let Some(bootstrap) = snapshot.bootstrap {
        let ini = snapshot
            .config
            .ini
            .as_ref()
            .context("System4 reset bootstrap INI absent")?;
        let reader = storage.open_asset(&VfsPath::new(Mount::Asset, ini)?)?;
        let bytes = crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?;
        cancel.check()?;
        let document = crate::bootstrap::Document::parse(&bytes)?;
        ensure!(
            crate::bootstrap::Bootstrap::from_document(&document)? == bootstrap,
            "System4 bootstrap changed during native reset"
        );
        let volume = document
            .entries
            .iter()
            .filter(|e| e.name == "DefaultVolumeRate")
            .try_fold(100, |_, e| {
                if let crate::bootstrap::Value::Integer(value) = e.value {
                    Ok(value)
                } else {
                    anyhow::bail!("System4 reset default volume type")
                }
            })?;
        let mut metadata = crate::native_audio::AudioMetadata::load(
            snapshot.owner,
            storage.as_ref(),
            cancel.clone(),
        )?;
        metadata.reviewed_music =
            crate::native_audio_profile::load(&document, storage.as_ref(), cancel)?;
        adapter
            .native_audio
            .configure(&bootstrap, volume, metadata)?;
        adapter.native_files.configure(&bootstrap)?;
        adapter.native_system.configure(&bootstrap)?;
        let draw_profile=crate::native_draw_profile::load(&document,storage.as_ref(),cancel)?;
        adapter.native_draw.configure_original_profile(draw_profile);
        adapter.native_effect.configure_original_profile(draw_profile);
        adapter
            .native_draw
            .configure_text_profile(crate::native_draw_profile::load_text(
                &document,
                storage.as_ref(),
                cancel,
            )?);
        adapter
            .native_movie
            .configure(crate::native_draw_profile::load_movie(
                &document,
                storage.as_ref(),
                cancel,
            )?);
        adapter.bootstrap = Some(bootstrap);
    }
    cancel.check()?;
    Ok(Prepared {
        adapter: Box::new(adapter),
    })
}

fn scope_batches(
    ids: Vec<u32>,
    canvas: bool,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Vec<OriginalRestoreBatch>> {
    let mut batches = Vec::new();
    for (index, ids) in ids.chunks(512).enumerate() {
        cancel.check()?;
        let layers = ids
            .iter()
            .map(|id| serde_json::json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut batch = serde_json::json!({"layers":layers});
        if index == 0 && canvas {
            batch["canvases"] = serde_json::json!([{"op":"retire","id":1}]);
        }
        batches.push(OriginalRestoreBatch {
            schema: 1,
            bytes: serde_json::to_vec(&batch)?.into(),
        });
    }
    if batches.is_empty() && canvas {
        batches.push(OriginalRestoreBatch {
            schema: 1,
            bytes: serde_json::to_vec(&serde_json::json!({"canvases":[{"op":"retire","id":1}]}))?
                .into(),
        });
    }
    ensure!(batches.len() <= 128, "System4 reset engine batch budget");
    cancel.check()?;
    Ok(batches)
}
