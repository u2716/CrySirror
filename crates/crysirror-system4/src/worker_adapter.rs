// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Owner/worker boundary for original VM execution. All file reads, AIN parsing,
//! heap/array allocation and instruction batches run on the shared task pool.
//! The owner alone completes typed host calls. No factory is registered here.
use crate::{
    adapter::{AbsentDebugLog, Bridge, Dispatch},
    ain::Program,
    vm::{Machine, Step},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{HostServices, InputFrame},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
#[path = "worker_restart.rs"]
mod restart;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub program: String,
    #[serde(default)]
    pub ini: Option<String>,
    #[serde(default)]
    pub volumes: Vec<crate::assets::Volume>,
    /// Native logical view dimensions, read from original configuration.
    #[serde(default)]
    pub canvas: Option<[u32; 2]>,
    /// Explicit portable bindings for native Gothic/Mincho faces.
    #[serde(default = "font_aliases")]
    pub font_aliases: [String; 2],
    pub reviewed_output_log_dll: Option<String>,
}
fn font_aliases() -> [String; 2] {
    ["default".into(), "default".into()]
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if let Some(canvas) = self.canvas {
            crate::sact::Sact::new(canvas, self.font_aliases.clone())?;
        }

        ensure!(self.volumes.len() <= 104, "System4 archive volume budget");
        for volume in &self.volumes {
            volume.validate()?;
        }
        for s in std::iter::once(&self.program)
            .chain(self.reviewed_output_log_dll.iter())
            .chain(self.ini.iter())
        {
            ensure!(
                VfsPath::new(Mount::Asset, s)?.relative() == s,
                "System4 canonical asset path"
            );
        }
        ensure!(
            self.program.to_ascii_lowercase().ends_with(".ain"),
            "System4 original AIN path"
        );
        Ok(())
    }
}
struct Completed {
    machine: Machine,
    stop: std::result::Result<Step, String>,
    debug_log: Option<AbsentDebugLog>,
    assets: Option<Arc<crate::assets::Assets>>,
    bootstrap: Option<crate::bootstrap::Bootstrap>,
    audio_config: Option<(i32, crate::native_audio::AudioMetadata)>,
    draw_profile: Option<bool>,
    text_profile: Option<bool>,
    movie_profile: Option<bool>,
}
pub struct NativeAdapter {
    client: ClientId,
    config: Config,
    pool: TaskPool,
    job: Option<Task<Completed>>,
    global_save: Option<Task<crate::native_save::Completed>>,
    restart: Option<restart::ResetState>,
    machine: Option<Machine>,
    bridge: Bridge,
    native_async: crate::native_async::NativeAsync,
    native_cg: crate::native_cg::NativeCg,
    native_draw: crate::native_draw::NativeDraw,
    native_scene: crate::native_scene::NativeScene,
    native_wp:crate::native_wallpaper_get::NativeWallpaperGet,
    native_audio: crate::native_audio::NativeAudio,
    native_files: crate::native_files::NativeFiles,
    native_dialog: crate::native_dialog::NativeDialog,
    native_movie: crate::native_movie::NativeMovie,
    native_logo: crate::native_logo::NativeLogo,
    native_text: crate::native_sact_text::NativeText,
    native_present:crate::native_scene_present::NativePresent,
    native_effect:crate::native_effect::NativeEffect,
    native_save_cg:crate::native_save_cg::NativeSaveCg,
    native_csv:crate::native_csv::NativeCsv,
    native_system: crate::native_system::NativeSystem,
    native_logs: crate::native_logs::NativeLogs,
    native_input_string: crate::native_input_string::NativeInputString,
    native_file: crate::native_file::NativeFile,
    native_msgskip: crate::native_msgskip::NativeMsgSkip,
    close_requested: bool,
    deferred_error: Option<String>,
    input_events: crate::buffered_input::Events,
    stop: Option<Step>,
    started: bool,
    boot_ready: bool,
    bootstrap: Option<crate::bootstrap::Bootstrap>,
    factory_created: bool,
    last_location: String,
    last_error: Option<String>,
}
fn core_error(e: anyhow::Error) -> Error {
    Error::Io(format!("{e:#}"))
}
impl NativeAdapter {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let sact = config
            .canvas
            .map(|canvas| crate::sact::Sact::new(canvas, config.font_aliases.clone()))
            .transpose()?;
        let mut bridge = Bridge::default();
        bridge.sact = sact;
        bridge.buffered_input = true;
        let native_files = crate::native_files::NativeFiles::new(client)?;
        let native_msgskip =
            crate::native_msgskip::NativeMsgSkip::new(client, native_files.io_pool());
        let native_logo=crate::native_logo::NativeLogo::new(client,native_files.io_pool(),config.ini.clone());
        Ok(Self {
            client,
            config,
            pool: TaskPool::new("system4-native", 1, 2)?,
            job: None,
            global_save: None,
            restart: None,
            machine: None,
            bridge,
            native_async: crate::native_async::NativeAsync::default(),
            native_cg: crate::native_cg::NativeCg::default(),
            native_draw: crate::native_draw::NativeDraw::new(client),
            native_scene: crate::native_scene::NativeScene::new(client),
            native_wp:crate::native_wallpaper_get::NativeWallpaperGet::new(client),
            native_audio: crate::native_audio::NativeAudio::new(client, native_files.io_pool()),
            native_movie: crate::native_movie::NativeMovie::new(client, native_files.io_pool()),
            native_logo,
            native_text:crate::native_sact_text::NativeText::new(client,native_files.io_pool()),
            native_present:crate::native_scene_present::NativePresent::new(client),
            native_effect:crate::native_effect::NativeEffect::new(client),
            native_save_cg:crate::native_save_cg::NativeSaveCg::new(client,native_files.io_pool()),
            native_csv:crate::native_csv::NativeCsv::new(client,native_files.io_pool())?,
            native_system: crate::native_system::NativeSystem::new(client, native_files.io_pool()),
            native_logs: crate::native_logs::NativeLogs::new(client, native_files.io_pool())?,
            native_input_string: crate::native_input_string::NativeInputString::new(client),
            native_file: crate::native_file::NativeFile::new(client, native_files.io_pool()),
            native_files,
            native_dialog: crate::native_dialog::NativeDialog::new(client),
            native_msgskip,
            close_requested: false,
            deferred_error: None,
            input_events: crate::buffered_input::Events::new(client)?,
            stop: None,
            started: false,
            boot_ready: false,
            bootstrap: None,
            factory_created: false,
            last_location: String::new(),
            last_error: None,
        })
    }
    #[cfg(feature = "runtime")]
    pub(crate) fn from_factory(client: ClientId, config: Config) -> Result<Self> {
        let mut adapter = Self::new(client, config)?;
        adapter.factory_created = true;
        Ok(adapter)
    }
    fn advance(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> Result<VmYield> {
        if let Some(error) = self.last_error.as_ref().filter(|_| !self.close_requested) {
            anyhow::bail!("{error}")
        }
        if !self.close_requested && !self.native_dialog.active() {
            let revision = match h.input_observation(self.client) {
                Ok(observation) => {
                    crate::input::Input::validate_scope(observation, self.client)?;
                    Some(observation.revision)
                }
                // Runtime has no backend observation before the first input
                // publication (including CPU/headless startup). The VmAdapter
                // InputFrame remains the actual provided input source; this does
                // not manufacture pointer position, focus or key representation.
                Err(Error::Unsupported(_) | Error::NotFound) => None,
                Err(error) => return Err(error.into()),
            };
            self.input_events
                .offer(self.client, h.now_ms(), input, revision)?;
        }
        if self.native_file.has_worker() {
            if !self.native_file.poll(&mut self.machine)? {
                return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
            }
            self.last_location = self.machine.as_ref().unwrap().location();
            self.stop = None;
            return Ok(VmYield::BudgetExhausted);
        }
        if self.native_logs.has_pending() {
            if !self.native_logs.poll(&mut self.machine)? {
                return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
            }
            self.last_location = self.machine.as_ref().unwrap().location();
            self.stop = None;
            return Ok(VmYield::BudgetExhausted);
        }
        if self.native_csv.has_pending(){
            if !self.native_csv.poll(&mut self.machine)?{return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)))}
            self.last_location=self.machine.as_ref().unwrap().location();self.stop=None;return Ok(VmYield::BudgetExhausted)
        }
        if let Some(task) = &mut self.global_save {
            let Some(done) = task.try_take()? else {
                return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
            };
            self.global_save = None;
            self.last_location = done.machine.location();
            self.machine = Some(done.machine);
            if let Some(error) = done.error {
                anyhow::bail!("{error}");
            }
            self.stop = None;
            return Ok(VmYield::BudgetExhausted);
        }
        if !self.started {
            let storage = h.storage(self.client)?;
            let path = VfsPath::new(Mount::Asset, &self.config.program)?;
            let dll = self.config.reviewed_output_log_dll.clone();
            let ini = self.config.ini.clone();
            let expected = self.config.clone();
            let client = self.client;
            self.job = Some(self.pool.spawn(move |cancel| {
                cancel.check()?;
                let mut audio_config = None;
                let mut draw_profile = None;
                let mut text_profile = None;
                let mut movie_profile = None;
                let bootstrap = if let Some(ini) = ini {
                    let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &ini)?)?;
                    let bytes = crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?;
                    let document = crate::bootstrap::Document::parse(&bytes)?;
                    let bootstrap = crate::bootstrap::Bootstrap::from_document(&document)?;
                    draw_profile = Some(crate::native_draw_profile::load(
                        &document,
                        storage.as_ref(),
                        &cancel,
                    )?);
                    text_profile = Some(crate::native_draw_profile::load_text(
                        &document,
                        storage.as_ref(),
                        &cancel,
                    )?);
                    movie_profile = Some(crate::native_draw_profile::load_movie(
                        &document,
                        storage.as_ref(),
                        &cancel,
                    )?);
                    let default_volume = document
                        .entries
                        .iter()
                        .filter(|e| e.name == "DefaultVolumeRate")
                        .try_fold(100, |_, entry| {
                            if let crate::bootstrap::Value::Integer(value) = entry.value {
                                Ok(value)
                            } else {
                                anyhow::bail!("System4 original default mixer volume type")
                            }
                        })?;
                    let mut metadata = crate::native_audio::AudioMetadata::load(
                        client,
                        storage.as_ref(),
                        cancel.clone(),
                    )?;
                    metadata.reviewed_music =
                        crate::native_audio_profile::load(&document, storage.as_ref(), &cancel)?;
                    audio_config = Some((default_volume, metadata));
                    ensure!(
                        bootstrap.program == expected.program
                            && expected.canvas == Some(bootstrap.canvas),
                        "System4 native INI changed after detection"
                    );
                    Some(bootstrap)
                } else {
                    None
                };
                let assets = Arc::new(crate::assets::Assets::load(
                    client,
                    storage.as_ref(),
                    &expected.volumes,
                    cancel.clone(),
                )?);
                let reader = storage.open_asset(&path)?;
                let bytes = crysirror_resource::read_all(
                    reader.as_ref(),
                    crate::ain::MAX_AIN_BYTES as u64,
                )?;
                cancel.check()?;
                let p = Program::parse(&bytes)?;
                p.validate_code_layout()?;
                let profile = if let Some(dll) = dll {
                    let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &dll)?)?;
                    let bytes = crysirror_resource::read_all(reader.as_ref(), 64 * 1024)?;
                    Some(AbsentDebugLog::verify_native_library(&bytes)?)
                } else {
                    None
                };
                cancel.check()?;
                let machine = Machine::new(Arc::new(p))?;
                Ok(Completed {
                    machine,
                    stop: Ok(Step::Advanced),
                    debug_log: profile,
                    assets: Some(assets),
                    bootstrap,
                    audio_config,
                    draw_profile,
                    text_profile,
                    movie_profile,
                })
            })?);
            self.started = true;
        }
        if let Some(job) = &mut self.job {
            let Some(done) = job.try_take()? else {
                return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
            };
            self.job = None;
            self.last_location = done.machine.location();
            self.machine = Some(done.machine);
            if let Some(assets) = done.assets {
                self.bridge.assets = Some(assets);
            }
            if let Some(bootstrap) = done.bootstrap {
                self.native_files.configure(&bootstrap)?;
                self.native_system.configure(&bootstrap)?;
                let (volume, metadata) = done
                    .audio_config
                    .context("System4 audio bootstrap configuration lost")?;
                self.native_audio.configure(&bootstrap, volume, metadata)?;
                self.bootstrap = Some(bootstrap);
            }
            if let Some(profile) = done.movie_profile {
                self.native_movie.configure(profile);
            }
            if let Some(profile) = done.text_profile {
                self.native_draw.configure_text_profile(profile);
            }
            if let Some(profile) = done.draw_profile {
                self.native_draw.configure_original_profile(profile);
                self.native_effect.configure_original_profile(profile);
            }
            if done.debug_log.is_some() {
                self.bridge.debug_log = done.debug_log;
            }
            let bootstrap_complete = !self.boot_ready;
            self.stop = Some(done.stop.map_err(anyhow::Error::msg)?);
            self.boot_ready = true;
            if bootstrap_complete {
                return Ok(VmYield::BudgetExhausted);
            }
        }
        if self.close_requested {
            self.cancel_reset();
            // Finish admitted initialization before saving its resulting flag
            // state, then run no further script instructions.
            if self.native_msgskip.pending_init() {
                let Some(Step::External(call)) = self.stop.clone() else {
                    anyhow::bail!("System4 closing MsgSkip lost original call")
                };
                let machine = self
                    .machine
                    .as_mut()
                    .context("System4 closing MsgSkip lost VM")?;
                match self.native_msgskip.dispatch(
                    self.client,
                    machine,
                    &call,
                    &self.native_files,
                    h,
                )? {
                    Some(Dispatch::WaitUntil(time)) => return Ok(VmYield::WaitUntil(time)),
                    Some(Dispatch::Completed) => self.stop = None,
                    _ => anyhow::bail!("System4 closing MsgSkip original operation changed"),
                }
            }
            match self.native_file.close(h)? {
                Dispatch::WaitUntil(time) => return Ok(VmYield::WaitUntil(time)),
                Dispatch::Completed => {}
                Dispatch::Presented => anyhow::bail!("System4 invalid File close presentation"),
            }
            self.native_input_string.close(h)?;
            self.native_dialog.close(h)?;
            self.native_movie.close(h)?;
            self.native_logo.close(h)?;
            self.native_text.close(h)?;
            self.native_present.cancel();
            self.native_wp.cancel();
            self.native_effect.cancel();
            self.native_save_cg.cancel();
            self.native_csv.cancel();
            self.native_system.cancel();
            self.native_logs.cancel();
            self.native_audio.close(h)?;
            self.native_scene.cancel_pending_for_shutdown();
            self.native_draw.cancel_pending_for_shutdown();
            self.native_files.cancel_pending_for_shutdown();
            self.native_cg.cancel_pending_for_shutdown();
            self.native_async.cancel_pending_for_shutdown();
            return match self.native_msgskip.flush(self.client, h)? {
                Dispatch::WaitUntil(time) => Ok(VmYield::WaitUntil(time)),
                Dispatch::Completed => Ok(VmYield::Halted),
                Dispatch::Presented => {
                    anyhow::bail!("System4 invalid read-history close presentation")
                }
            };
        }
        if let Some(machine) = &self.machine {
            self.input_events.pump(
                self.client,
                machine.peek_locked(),
                false,
                &mut self.bridge.input,
            )?;
        }
        match self.stop.clone() {
            Some(Step::Halted) => {
                match self.native_file.close(h)? {
                    Dispatch::WaitUntil(time) => return Ok(VmYield::WaitUntil(time)),
                    Dispatch::Completed => {}
                    Dispatch::Presented => anyhow::bail!("System4 invalid File halt presentation"),
                }
                self.native_input_string.close(h)?;
                self.native_dialog.close(h)?;
                self.native_movie.close(h)?;
                self.native_logo.close(h)?;
                self.native_text.close(h)?;
                self.native_present.cancel();
            self.native_wp.cancel();
            self.native_effect.cancel();
                self.native_save_cg.cancel();
                self.native_csv.cancel();
                self.native_system.cancel();
                self.native_logs.cancel();
                self.native_audio.close(h)?;
                return match self.native_msgskip.flush(self.client, h)? {
                    Dispatch::WaitUntil(time) => Ok(VmYield::WaitUntil(time)),
                    Dispatch::Completed => Ok(VmYield::Halted),
                    Dispatch::Presented => {
                        anyhow::bail!("System4 invalid read-history halt presentation")
                    }
                };
            }
            Some(Step::External(call)) => {
                if matches!(call.target, crate::vm::ExternalTarget::System(5)) {
                    return self.advance_reset(&call, h);
                }
                if crate::native_csv::NativeCsv::handles(self.machine.as_ref().context("native CSV VM absent")?,&call){
                    let dispatch=self.native_csv.dispatch(self.client,&mut self.machine,&call,&self.native_files,h)?.context("native CSV target lost")?;
                    return match dispatch{Dispatch::WaitUntil(time)=>Ok(VmYield::WaitUntil(time)),_=>anyhow::bail!("native CSV worker ownership receipt absent")}
                }

                if crate::native_file::NativeFile::handles(
                    self.machine.as_ref().context("System4 File VM absent")?,
                    &call,
                ) {
                    let dispatch = self
                        .native_file
                        .dispatch(self.client, &mut self.machine, &call, &self.native_files, h)?
                        .context("System4 File dispatch lost native target")?;
                    return match dispatch {
                        Dispatch::WaitUntil(time) => Ok(VmYield::WaitUntil(time)),
                        Dispatch::Completed => {
                            self.stop = None;
                            Ok(VmYield::BudgetExhausted)
                        }
                        Dispatch::Presented => {
                            anyhow::bail!("System4 invalid File worker presentation")
                        }
                    };
                }

                if crate::native_logs::NativeLogs::handles(
                    self.machine.as_ref().context("System4 message VM absent")?,
                    &call,
                ) {
                    let dispatch = self
                        .native_logs
                        .dispatch(self.client, &mut self.machine, &call, &self.native_files, h)?
                        .context("System4 message dispatch lost native target")?;
                    return match dispatch {
                        Dispatch::WaitUntil(time) => Ok(VmYield::WaitUntil(time)),
                        _ => anyhow::bail!(
                            "System4 message worker completed without ownership receipt"
                        ),
                    };
                }

                if matches!(
                    call.target,
                    crate::vm::ExternalTarget::System(8 | 9 | 22 | 23)
                ) {
                    let args = self
                        .machine
                        .as_ref()
                        .context("System4 resume lost VM")?
                        .resume_arguments(&call)?;
                    let paths = self.native_files.save_path(&args.filename)?;
                    let storage = h.storage(self.client)?;
                    let permit = self.native_files.io_pool().reserve()?;
                    let machine = self.machine.take().unwrap();
                    self.global_save = Some(permit.spawn(move |cancel| {
                        Ok(crate::native_save::run_resume(
                            machine, call, args, paths, storage, cancel,
                        ))
                    })?);
                    return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
                }

                if matches!(
                    call.target,
                    crate::vm::ExternalTarget::System(1 | 2 | 24 | 25)
                ) {
                    let machine = self
                        .machine
                        .as_ref()
                        .context("System4 global save lost VM")?;
                    let args = machine.global_save_arguments(&call)?;
                    let paths = self.native_files.save_path(&args.filename)?;
                    let storage = h.storage(self.client)?;
                    let permit = self.native_files.io_pool().reserve()?;
                    let machine = self.machine.take().unwrap();
                    self.global_save = Some(permit.spawn(move |cancel| {
                        Ok(crate::native_save::run(
                            machine, call, args, paths, storage, cancel,
                        ))
                    })?);
                    return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
                }
                let machine = self.machine.as_mut().unwrap();
                let explicit_peek = matches!(call.target, crate::vm::ExternalTarget::System(0x14));
                self.input_events.pump(
                    self.client,
                    machine.peek_locked(),
                    explicit_peek,
                    &mut self.bridge.input,
                )?;
                let modal_input =
                    if matches!(call.target, crate::vm::ExternalTarget::System(7 | 18 | 15)) {
                        let held = match h.input_observation(self.client) {
                            Ok(observation) => {
                                crate::input::Input::validate_scope(observation, self.client)?;
                                observation.buttons_down
                            }
                            Err(Error::Unsupported(_) | Error::NotFound) => input.buttons_down,
                            Err(error) => return Err(error.into()),
                        };
                        Some((crate::buffered_input::Events::new(self.client)?, held))
                    } else {
                        None
                    };
                if self.bridge.sact.as_ref().is_some_and(|s|!s.initialized){
                    let path=self.native_cg.current_window_path(self.client)?;
                    self.bridge.sact.as_mut().unwrap().initial_window=path;
                }
                let replacement = crate::native_cg::NativeCg::surface_replacement(machine, &call)?;
                if let Some(number)=self.native_cg.sprite_clear_target(machine,&call)? {
                    self.native_text.clear_for_cg(number,&mut self.bridge,h)?;
                    self.native_cg.reconcile(self.client,number,&self.bridge,h)?;
                }
                let dispatch = if explicit_peek {
                    machine.complete_system(&call, 0, None)?;
                    Dispatch::Completed
                } else if let Some(dispatch) = crate::native_sact_intrinsics::dispatch(
                    self.client, machine, &call, &self.bridge, h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_logo.dispatch(
                    self.client,machine,&call,&mut self.bridge,&mut self.native_cg,h)? {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_input_string
                        .dispatch(self.client, machine, &call, h)?
                {
                    dispatch
                } else if let Some(dispatch) = self.native_msgskip.dispatch(
                    self.client,
                    machine,
                    &call,
                    &self.native_files,
                    h,
                )? {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_dialog
                        .dispatch(self.client, machine, &call, &self.bridge, h)?
                {
                    if dispatch == Dispatch::Completed {
                        // Modal input belongs to shared UI, never the game.
                        let (events, held) =
                            modal_input.context("System4 modal input completion lost")?;
                        self.input_events = events;
                        self.bridge.input.suppress_modal_buttons(held);
                    }
                    dispatch
                } else if let Some(dispatch) =
                    self.native_files.dispatch(self.client, machine, &call, h)?
                {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_movie
                        .dispatch(self.client, machine, &call, &self.bridge, h)?
                {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_system
                        .dispatch(self.client, machine, &call, &self.bridge, h)?
                {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_async.dispatch(self.client, machine, &call, h)?
                {
                    dispatch
                } else if let Some(dispatch) = self.native_audio.dispatch_reviewed_unavailable(
                    self.client,
                    machine,
                    &call,
                    &self.bridge,
                    h,
                )? {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_audio
                        .dispatch_mixer(self.client, machine, &call, h)?
                {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_audio
                        .dispatch(self.client, machine, &call, &self.bridge, h)?
                {
                    dispatch
                }else if let Some(dispatch)=self.native_wp.dispatch(
                    self.client,machine,&call,&mut self.bridge,&mut self.native_cg,&self.native_scene,h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_scene.dispatch(
                    self.client,
                    machine,
                    &call,
                    &self.bridge,
                    &self.native_cg,
                    h,
                )? {
                    dispatch
                } else if let Some(dispatch) = self.native_save_cg.dispatch(
                    self.client,machine,&call,&self.bridge,&self.native_cg,&self.native_files,h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_effect.dispatch(
                    self.client,machine,&call,&mut self.bridge,&mut self.native_cg,h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_present.dispatch(
                    self.client,machine,&call,&mut self.bridge,&mut self.native_cg,h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_text.dispatch(
                    self.client,machine,&call,&mut self.bridge,h)? {
                    dispatch
                } else if let Some(dispatch) = self.native_draw.dispatch(
                    self.client,
                    machine,
                    &call,
                    &mut self.bridge,
                    &mut self.native_cg,
                    h,
                )? {
                    dispatch
                } else if let Some(dispatch) =
                    self.native_cg
                        .dispatch_with_files(self.client, machine, &call, &mut self.bridge, &self.native_files, h)?
                {
                    dispatch
                } else {
                    self.bridge.dispatch_for(self.client, machine, &call, h)?
                };
                match dispatch {
                    Dispatch::WaitUntil(t) => return Ok(VmYield::WaitUntil(t)),
                    Dispatch::Completed => {
                        if let Some(number) = replacement {
                            self.native_cg
                                .reconcile(self.client, number, &self.bridge, h)?;
                            if self.bridge.sact.as_ref().and_then(|s|s.sprites.as_ref()).and_then(|s|s.get(number)).is_none(){
                                self.native_text.forget_deleted(number,h)?;
                            }
                        }
                        self.stop = None;
                    }
                    Dispatch::Presented => {
                        self.input_events.pump(
                            self.client,
                            machine.peek_locked(),
                            true,
                            &mut self.bridge.input,
                        )?;
                        self.stop = None;
                        return Ok(VmYield::WaitUntil(h.now_ms().saturating_add(1)));
                    }
                }
            }
            _ => {}
        }
        if self.job.is_none() {
            // Reserve before transferring the only mutable native state to the worker.
            let permit = self.pool.reserve()?;
            let mut m = self
                .machine
                .take()
                .ok_or_else(|| anyhow::anyhow!("System4 missing worker state"))?;
            let count = budget.clamp(1, 100_000);
            self.job = Some(permit.spawn(move |cancel| {
                let mut stop = Ok(Step::Advanced);
                for n in 0..count {
                    if n % 256 == 0 {
                        cancel.check()?;
                    }
                    match m.step_with_check(&|| {
                        cancel.check()?;
                        Ok(())
                    }) {
                        Ok(Step::Advanced) => {}
                        Ok(s) => {
                            stop = Ok(s);
                            break;
                        }
                        Err(e) => {
                            stop = Err(format!("{e:#}"));
                            break;
                        }
                    }
                }
                Ok(Completed {
                    machine: m,
                    stop,
                    debug_log: None,
                    assets: None,
                    bootstrap: None,
                    audio_config: None,
                    draw_profile: None,
                    text_profile: None,
                    movie_profile: None,
                })
            })?);
        }
        Ok(VmYield::BudgetExhausted)
    }
}
impl NativeAdapter {
    fn flush_deferred_error(&mut self, host: &mut dyn HostServices) -> Result<VmYield> {
        self.cancel_reset();
        self.native_input_string.close(host)?;
        self.native_file.cancel();
        self.native_dialog.close(host)?;
        self.native_movie.close(host)?;
        self.native_logo.close(host)?;
        self.native_text.close(host)?;
        self.native_present.cancel();
            self.native_wp.cancel();
            self.native_effect.cancel();
        self.native_save_cg.cancel();
        self.native_csv.cancel();
        self.native_system.cancel();
        self.native_logs.cancel();
        let error = self
            .deferred_error
            .clone()
            .context("System4 deferred error missing")?;
        if let Err(cleanup) = self.native_audio.close(host) {
            anyhow::bail!("{error}; audio cleanup failed: {cleanup:#}");
        }
        match self.native_msgskip.flush(self.client, host) {
            Ok(Dispatch::WaitUntil(time)) => Ok(VmYield::WaitUntil(time)),
            Ok(Dispatch::Completed) => {
                self.deferred_error = None;
                anyhow::bail!("{error}")
            }
            Ok(Dispatch::Presented) => {
                anyhow::bail!("System4 invalid deferred read-history presentation")
            }
            Err(save) => anyhow::bail!("{error}; read-history flush failed: {save:#}"),
        }
    }
}
impl VmAdapter for NativeAdapter {
    fn request_close(&mut self) -> bool {
        if !self.native_msgskip.has_state()
            && !self.native_audio.has_state()
            && !self.native_dialog.active()
            && !self.native_movie.has_state()
            && !self.native_logo.has_state()
            && !self.native_text.has_state()
            && !self.native_present.has_pending()
            && !self.native_wp.has_pending()
            && !self.native_effect.has_pending()
            && !self.native_save_cg.has_pending()
            && !self.native_csv.has_pending()
            && !self.native_system.has_pending()
            && !self.native_logs.has_pending()
            && !self.native_input_string.has_state()
            && !self.native_file.needs_close()
            && self.global_save.is_none()
            && self.restart.is_none()
        {
            return false;
        }
        self.close_requested = true;
        true
    }
    fn preparation_pending(&self) -> bool {
        !self.boot_ready
    }
    fn engine_id(&self) -> &'static str {
        "system4"
    }
    fn snapshot_schema(&self) -> u32 {
        0
    }
    fn step(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        let mut result = if self.deferred_error.is_some() {
            self.flush_deferred_error(h)
        } else {
            self.advance(h, input, budget)
        };
        if let Err(error) = &result {
            if (self.native_msgskip.needs_flush()
                || self.native_audio.has_state()
                || self.native_dialog.active()
                || self.native_movie.has_state()
                || self.native_logo.has_state()
                || self.native_text.has_state()
                || self.native_present.has_pending()
                || self.native_wp.has_pending()
                || self.native_effect.has_pending()
                || self.native_save_cg.has_pending()
                || self.native_csv.has_pending()
                || self.native_system.has_pending()
                || self.native_logs.has_pending()
                || self.native_input_string.has_state()
                || self.native_file.needs_close()
                || self.restart.is_some())
                && self.deferred_error.is_none()
            {
                // Native module destruction saves read flags even when an
                // unsupported script operation stops execution. Release retained
                // shared audio authorities on that same owner boundary. Preserve the
                // original error and PC until the actual scoped write settles.
                self.native_msgskip.abandon_failed_init();
                self.native_scene.cancel_pending_for_shutdown();
                self.native_draw.cancel_pending_for_shutdown();
                self.native_cg.cancel_pending_for_shutdown();
                self.native_async.cancel_pending_for_shutdown();
                self.native_files.cancel_pending_for_shutdown();
                self.deferred_error = Some(format!("{error:#}"));
                result = self.flush_deferred_error(h);
            }
        }
        if let Err(e) = &result {
            self.last_error = Some(format!("{e:#}"));
        }
        result.map_err(core_error)
    }
    fn begin_original_restore(
        &self,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>>
    {
        self.reset_job(bytes).map_err(core_error)
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("System4 serialized native snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("System4 serialized native snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let pending = self.machine.as_ref().and_then(|machine| {
            let Step::External(call) = self.stop.as_ref()? else { return None; };
            let crate::vm::ExternalTarget::Hll { library, function } = call.target else { return Some(serde_json::json!({"pc":call.pc,"target":format!("{:?}",call.target)})); };
            let lib = &machine.program.libraries[library];
            let function = &lib.functions[function];
            Some(serde_json::json!({"pc":call.pc,"library":String::from_utf8_lossy(&lib.name),"function":String::from_utf8_lossy(&function.name),"return_kind":function.return_kind,"arguments":function.arguments.iter().map(|(_,kind)|*kind).collect::<Vec<_>>(),"stack_tail":machine.stack.iter().rev().take(16).copied().collect::<Vec<_>>()}))
        });
        serde_json::to_vec(&serde_json::json!({"engine":"system4","scene":self.native_scene.diagnostics(),"audio":self.native_audio.diagnostics(),"movie":self.native_movie.diagnostics(),"logo":self.native_logo.diagnostics(),"native_text":self.native_text.diagnostics(),"text_input":self.native_input_string.diagnostics(),"draw_plugin":self.native_draw.diagnostics(),"message_skip":self.native_msgskip.diagnostics(),"native_location":self.last_location,"exit_status":self.machine.as_ref().and_then(|m|m.exit_status),"modal_active":self.native_dialog.active(),"native_pending":pending,"error":self.last_error,"worker_pending":self.job.is_some(),"native_save_pending":self.global_save.is_some(),"debug_log_interface":if self.bridge.debug_log.is_some(){"reviewed-absent"}else{"unreviewed"},"debug_log_calls":self.bridge.absent_debug_calls,"clock_calls":self.bridge.clock_calls,"civil_time_samples":self.native_async.civil_samples,"log_calls":self.bridge.log_calls,"sact":self.bridge.sact.as_ref().map(|s|serde_json::json!({"initialized":s.initialized,"canvas":s.canvas,"font_ready":s.font_ready,"audio_ready":s.audio_ready,"calls":s.calls})),"factory_registered":self.factory_created,"bootstrap":self.bootstrap.as_ref().map(|b|serde_json::json!({"program":b.program,"game_name":b.game_name,"canvas":b.canvas,"native_vm":b.native_vm,"joypad":b.joypad,"mixers":b.mixers.len()}))})).unwrap_or_default()
    }
}

#[cfg(all(test, feature = "runtime"))]
#[path = "worker_audio_tests.rs"]
mod audio_actor_tests;
