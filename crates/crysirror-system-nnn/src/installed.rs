// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Dormant installed-game factory. Only original VM policy is prepared here;
//! image decoding, composition, font resources, audio and clocks are services.
use crate::{
    Program,
    adapter::{NativeAdapter, ScriptLoader},
    bridge::{Bridge, CallbackProfile, SceneBindings},
    native_audio::AudioBindings,
    native_bootstrap::{BootEntry, PreparedBoot},
    native_resources::ResourceNames,
    native_services_bootstrap::{
        NativeWindowSource, PreparedNativeServices, resolved_window_resource,
    },
    native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp, SystemWrite},
    native_text::{MessageBindings, MessageWindow, NativeTextBridge, TextFace},
    native_ui::SelectionProfile,
    resource_bridge::{NativeResources, SourceOpener},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    path::{Mount, VfsPath},
    service::{CivilTime, CivilTimeQuery, CivilTimeState, ClientStorage, HostServices, InputFrame},
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
const SIZE: [u32; 2] = [800, 600];
mod native_load;
mod native_save;
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    #[default]
    Controller,
    NewGame,
}
/// Paths refer to the client capability namespace, including mounted installer
/// directories. The reviewed callback ABI is authenticated by PreparedBoot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub root: String,
    pub module: String,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default)]
    pub entry: Entry,
}
fn default_font() -> String {
    "default".into()
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let names = ResourceNames::new(&self.root)?;
        names.path(&self.module)?;
        ensure!(
            !self.font.is_empty() && self.font.len() <= 256,
            "System-NNN font alias"
        );
        Ok(())
    }
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "System-NNN executes original SPT; no data VM program"
        );
        Ok(Box::new(InstalledAdapter::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("system-nnn", Arc::new(Factory))
}
struct WindowInput {
    source: NativeWindowSource,
    input: Option<ImageInput>,
    effects: Arc<[u8]>,
    job: Option<ImageJob>,
    resolved: Option<MessageWindow>,
}
struct Prepared {
    boot: PreparedBoot,
    system_file: NativeSystemFile,
    result_data: Option<crate::native_result::NativeResultData>,
    native_load: Option<native_load::Policy>,
    native_save: Option<native_save::Policy>,
    loader: ScriptLoader,
    services: PreparedNativeServices,
    resources: NativeResources,
    windows: Vec<WindowInput>,
    startup: Option<crate::native_shell::NativeStartupProfile>,
    back_program: Option<Arc<Program>>,
}
/// No source read, task thread, engine mutation or decoder admission at creation.
pub struct InstalledAdapter {
    client: ClientId,
    config: Config,
    pool: Option<TaskPool>,
    task: Option<Task<Prepared>>,
    civil: Option<Box<dyn CivilTimeQuery>>,
    clock: Option<u32>,
    civil_sample: Option<CivilTime>,
    boot_admitted: bool,
    prepared: Option<Prepared>,
    vm: Option<NativeAdapter>,
    failure: Option<String>,
    native_load: Option<native_load::Controller>,
    native_save: Option<native_save::Controller>,
    published: Vec<String>,
    started: bool,
    shell: Option<crate::native_shell::NativeStartupShell>,
    outer: Option<crate::adapter::NativeOuterState>,
    movie: Option<crate::native_movie::NativeMoviePlayer>,
    movie_preferences: Option<crate::native_services_bootstrap::NativePreferences>,
    title: Option<crate::native_title::NativeTitlePlayer>,
    title_resources: Option<NativeResources>,
    title_program_task: Option<Task<Arc<Program>>>,
    title_program: Option<Arc<Program>>,
    title_exit_job: Option<ImageJob>,
    title_exit: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    native_exit_available: bool,
    title_newgame_requested: bool,
    title_restart_requested: Option<crate::native_shell::NativeMode>,
    opening_return_requested: bool,
    closing: bool,
    closed: bool,
    close_civil: Option<Box<dyn CivilTimeQuery>>,
    close_write: Option<SystemWrite>,
    close_writes_done: u8,
    close_audio_stopped: bool,
}
struct InstalledRestoreJob {
    inner: crate::native_game_restore::NativeRestoreJob,
    client: ClientId,
    config: Config,
    pool: Option<TaskPool>,
    clock: Option<u32>,
    civil_sample: Option<CivilTime>,
    published: Vec<String>,
    movie_preferences: Option<crate::native_services_bootstrap::NativePreferences>,
    title_resources: Option<NativeResources>,
    native_exit_available: bool,
    native_load: Option<native_load::Policy>,
    native_save: Option<native_save::Controller>,
    load_request: Option<crysirror_core::native_restore::OriginalRestoreTicket>,
}
impl crysirror_core::native_restore::OriginalRestorePreparation for InstalledRestoreJob {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> crysirror_core::Result<crysirror_core::native_restore::OriginalRestoreProgress> {
        crysirror_core::native_restore::OriginalRestorePreparation::poll(&mut self.inner, host)
    }
    fn finish(
        self: Box<Self>,
    ) -> crysirror_core::Result<crysirror_core::native_restore::OriginalRestoreCommit> {
        let this = *self;
        let (vm, engine_batches, audio, release_images) = this
            .inner
            .finish_native()
            .map_err(|error| Error::Io(format!("NNN installed original restore: {error:#}")))?;
        let mut installed = InstalledAdapter::new(this.client, this.config)
            .map_err(|error| Error::Io(error.to_string()))?;
        installed.pool = this.pool;
        installed.clock = this.clock;
        installed.civil_sample = this.civil_sample;
        installed.boot_admitted = true;
        installed.started = true;
        installed.vm = Some(vm);
        installed.published = this.published;
        installed.movie_preferences = this.movie_preferences;
        installed.title_resources = this.title_resources;
        installed.native_exit_available = this.native_exit_available;
        installed.native_save = this.native_save;
        installed.native_load = this
            .native_load
            .map(|policy| native_load::Controller::after_restore(policy, this.load_request));
        Ok(crysirror_core::native_restore::OriginalRestoreCommit {
            vm: Box::new(installed),
            commands: vec![],
            engine_batches,
            audio: vec![],
            prepared_audio: Some(audio),
            release_images,
        })
    }
}
fn read(
    storage: &dyn ClientStorage,
    path: &str,
    limit: u64,
    cancel: &Cancellation,
) -> Result<Option<Vec<u8>>> {
    cancel.check()?;
    let path = VfsPath::new(Mount::Asset, path)?;
    let source = match storage.open_asset(&path) {
        Ok(source) => source,
        Err(Error::NotFound) => return Ok(None),
        Err(reason) => return Err(reason.into()),
    };
    let bytes = crysirror_resource::read_all(source.as_ref(), limit)?;
    cancel.check()?;
    Ok(Some(bytes))
}
impl InstalledAdapter {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        ensure!(client.0 != 0, "System-NNN installed client identity");
        Ok(Self {
            client,
            config,
            pool: None,
            task: None,
            civil: None,
            clock: None,
            civil_sample: None,
            boot_admitted: false,
            prepared: None,
            vm: None,
            failure: None,
            native_load: None,
            native_save: None,
            published: vec![],
            started: false,
            shell: None,
            outer: None,
            movie: None,
            movie_preferences: None,
            title: None,
            title_resources: None,
            title_program_task: None,
            title_program: None,
            title_exit_job: None,
            title_exit: None,
            native_exit_available: false,
            title_newgame_requested: false,
            title_restart_requested: None,
            opening_return_requested: false,
            closing: false,
            closed: false,
            close_civil: None,
            close_write: None,
            close_writes_done: 0,
            close_audio_stopped: false,
        })
    }
    #[cfg(test)]
    pub(crate) fn original_restore_fixture(vm: NativeAdapter) -> Result<Self> {
        let mut adapter = Self::new(
            ClientId(1),
            Config {
                root: String::new(),
                module: "authored-layout.exe".into(),
                font: "fixture".into(),
                entry: Entry::Controller,
            },
        )?;
        adapter.started = true;
        adapter.boot_admitted = true;
        adapter.vm = Some(vm);
        adapter.pool = Some(TaskPool::new("nnn-installed-restore-fixture", 1, 1)?);
        Ok(adapter)
    }
    /// Only work admitted by a prior ordinary step is a dependency. This does
    /// not turn native timer or input waits into offline busy work.
    pub fn dependency_pending(&self) -> bool {
        if self.failure.is_some() || !self.started {
            return false;
        }
        (self.closing && !self.closed)
            || self.native_load.as_ref().is_some_and(|load| load.pending())
            || self.native_save.as_ref().is_some_and(|save| save.pending())
            || self.title_newgame_requested
            || self.title_restart_requested.is_some()
            || self.opening_return_requested
            || self.task.is_some()
            || self.civil.is_some()
            || self.prepared.is_some()
            || self.shell.as_ref().is_some_and(|s| s.dependency_pending())
            || self.vm.as_ref().is_some_and(|vm| vm.dependency_pending())
            || self
                .movie
                .as_ref()
                .is_some_and(|movie| movie.dependency_pending())
            || self.title_program_task.is_some()
            || self.title_exit_job.is_some()
            || self
                .title
                .as_ref()
                .is_some_and(|title| title.dependency_pending())
    }
    /// Independent owner completion pump. No fresh VM batch, clock advancement
    /// or input sampling occurs here; the caller remains free to yield.
    pub fn poll_dependency(&mut self, host: &mut dyn HostServices) -> crysirror_core::Result<()> {
        if let Some(reason) = &self.failure {
            return Err(Error::Io(reason.clone()));
        }
        if !self.dependency_pending() {
            return Ok(());
        }
        let result: Result<()> = if self.native_save.as_ref().is_some_and(|save| save.pending()) {
            self.poll_slot_save(host, InputFrame::default()).map(|_| ())
        } else if self.native_load.as_ref().is_some_and(|load| load.pending()) && !self.closing {
            self.poll_slot_load(host, InputFrame::default()).map(|_| ())
        } else if self.closing {
            self.poll_close(host).map(|_| ())
        } else if self.title_newgame_requested || self.title_restart_requested.is_some() {
            self.poll_title_newgame(host).map(|_| ())
        } else if self.opening_return_requested {
            self.poll_opening_return(host).map(|_| ())
        } else if self.title.is_some() {
            self.prepare_title_context(host).and_then(|_| {
                if let Some(vm) = self.vm.as_mut() {
                    vm.poll_dependency(host)?;
                }
                self.title
                    .as_mut()
                    .unwrap()
                    .poll_existing_dependencies(host, self.vm.as_mut())
            })
        } else if self
            .movie
            .as_ref()
            .is_some_and(|movie| movie.dependency_pending())
        {
            self.movie
                .as_mut()
                .unwrap()
                .poll(host, InputFrame::default())
                .map(|_| ())
        } else if self.vm.is_none() {
            self.advance(host, InputFrame::default(), 1).map(|_| ())
        } else if self.shell.as_ref().is_some_and(|s| s.dependency_pending()) {
            self.shell
                .as_mut()
                .unwrap()
                .poll(host, InputFrame::default())
                .map(|_| ())
        } else {
            self.vm
                .as_mut()
                .unwrap()
                .poll_dependency(host)
                .map_err(Into::into)
        };
        if let Err(reason) = result {
            let reason = format!("System-NNN installed bootstrap/VM: {reason:#}");
            if self.vm.is_none() {
                for path in self.published.drain(..) {
                    let _ = host.release_image_resource(self.client, &path);
                }
            }
            self.failure = Some(reason.clone());
            return Err(Error::Io(reason));
        }
        Ok(())
    }
    /// Original WM_CLOSE -> EndGame captures native variables; MainControl
    /// Game and End each call SystemFile.Save. Preserve both retained writes,
    /// including their separate real civil samples/saveCount increments.
    fn poll_close(&mut self, host: &mut dyn HostServices) -> Result<VmYield> {
        if self.closed {
            return Ok(VmYield::Halted);
        }
        if let Some(vm) = self.vm.as_mut() {
            if !vm.poll_shutdown_ready(host)? {
                return Ok(VmYield::BudgetExhausted);
            }
            ensure!(self.outer.is_none(), "NNN shutdown has two original owners");
            self.outer = Some(self.vm.take().unwrap().into_cancelled_outer_state()?);
        }
        let outer = self
            .outer
            .as_mut()
            .context("NNN shutdown original state absent")?;
        if !self.close_audio_stopped {
            outer.bridge.shutdown_audio(host)?;
            self.close_audio_stopped = true;
        }
        if let Some(write) = self.close_write.as_mut() {
            if !write.poll(outer.bridge.system_file_mut()?)? {
                return Ok(VmYield::BudgetExhausted);
            }
            if !outer.bridge.poll_result_save(host)? {
                return Ok(VmYield::BudgetExhausted);
            }
            self.close_write = None;
            self.close_writes_done += 1;
            if self.close_writes_done == 2 {
                self.closed = true;
                return Ok(VmYield::Halted);
            }
        }
        if self.close_civil.is_none() {
            self.close_civil = Some(host.begin_civil_time(self.client)?);
        }
        let sample = match self.close_civil.as_mut().unwrap().poll()? {
            CivilTimeState::Pending => return Ok(VmYield::BudgetExhausted),
            CivilTimeState::Failed(reason) => anyhow::bail!("NNN shutdown civil time: {reason}"),
            CivilTimeState::Ready(sample) => sample,
        };
        let stamp = SystemStamp::from_civil(sample, self.client)?;
        let variables = outer
            .machine
            .variables()
            .context("NNN shutdown original variable bank absent")?;
        let storage = host.storage(self.client)?;
        self.close_write = Some(outer.bridge.system_file_mut()?.begin_save(
            storage.as_ref(),
            variables,
            outer.configuration.profile,
            stamp,
            false,
        )?);
        self.close_civil = None;
        Ok(VmYield::BudgetExhausted)
    }
    fn start(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let pool = TaskPool::new("nnn-installed-bootstrap", 1, 1)?;
        // Sample the real local civil date before admitting the worker that
        // creates native defaults. No system-file parsing occurs on the owner.
        self.civil = Some(host.begin_civil_time(self.client)?);
        self.pool = Some(pool);
        self.started = true;
        Ok(())
    }
    fn start_resources(&mut self, host: &mut dyn HostServices, sample: CivilTime) -> Result<()> {
        ensure!(!self.boot_admitted, "NNN bootstrap worker already admitted");
        let stamp = SystemStamp::from_civil(sample, self.client)?;
        let permit = self
            .pool
            .as_ref()
            .context("NNN bootstrap pool absent")?
            .reserve()?;
        let storage = host.storage(self.client)?;
        let config = self.config.clone();
        let client = self.client;
        self.task = Some(permit.spawn(move |cancel| {
            let mut boot = PreparedBoot::mugen2(
                |path, limit| read(storage.as_ref(), path, limit, &cancel),
                &config.root,
                &config.module,
                match config.entry {
                    Entry::Controller => BootEntry::Controller,
                    Entry::NewGame => BootEntry::NewGame,
                },
                &cancel,
            )?;
            let module = read(
                storage.as_ref(),
                &boot.resources.path(&config.module)?,
                32 << 20,
                &cancel,
            )?
            .context("NNN callback module missing after boot")?;
            let services = PreparedNativeServices::from_configuration(
                &boot.configuration(),
                &module,
                storage.as_ref(),
                AudioBindings {
                    music: 0,
                    sound_base: 16,
                    voice_base: 32,
                },
                false,
                &cancel,
            )?;
            let defaults = NativeSystemFile::defaults(
                SystemLimits::prepare(&boot.configuration(), storage.as_ref(), &cancel)?,
                stamp,
                &SystemNames::from_module(&module)?,
            )?;
            let mut system_file = if let Some(original) = &services.preferences.original {
                NativeSystemFile::load(original, &defaults)?
            } else {
                defaults
            };
            system_file.initialize_player_names(&boot.setup)?;
            let native_load =
                native_load::Policy::prepare(&boot.configuration(), storage.as_ref(), &cancel)?;
            let native_save =
                native_save::Policy::prepare(&boot.configuration(), storage.as_ref(), &cancel)?;
            let result_data = crate::native_result::NativeResultData::prepare(
                &boot.setup,
                &boot.variable_names,
                storage.as_ref(),
                &cancel,
            )?;
            let mut variables = boot
                .session
                .machine()
                .variables()
                .context("NNN bootstrap original variable bank absent")?
                .clone();
            system_file.restore_variables(&mut variables, boot.profile)?;
            boot.session.machine_mut().replace_variables(variables)?;
            let io = storage.clone();
            let opener: SourceOpener = Arc::new(move |path, cancel| {
                cancel.check()?;
                match io.open_asset(&VfsPath::new(Mount::Asset, path)?) {
                    Ok(source) => Ok(Some(source)),
                    Err(Error::NotFound) => Ok(None),
                    Err(reason) => Err(reason.into()),
                }
            });
            let resources = NativeResources::blackcyc(client, opener)?;
            let startup = if matches!(config.entry, Entry::Controller) {
                Some(crate::native_shell::NativeStartupProfile::read(
                    &boot.configuration(),
                    storage.as_ref(),
                    &cancel,
                )?)
            } else {
                None
            };
            let back_program = if let Some(startup) = &startup {
                Some(boot.load_script(
                    &startup.opening_script,
                    |path, limit| read(storage.as_ref(), path, limit, &cancel),
                    &cancel,
                )?)
            } else {
                None
            };
            let levels = u32::try_from(boot.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
            let mut windows = vec![];
            if let Some(native) = services.windows(0, levels)? {
                for source in std::iter::once(native.body).chain(native.name) {
                    let decoded = resources.image_source(&source.picture, &cancel)?;
                    let effects = if decoded.has_mask {
                        Vec::new()
                    } else {
                        vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                            spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey {
                                right: false,
                            },
                        }]
                    };
                    windows.push(WindowInput {
                        source,
                        input: Some(decoded.input),
                        effects: serde_json::to_vec(&effects)?.into(),
                        job: None,
                        resolved: None,
                    });
                }
            }
            let names = boot.resources.clone();
            let scripts = boot.scripts;
            let loader: ScriptLoader = Arc::new(move |name, cancel| {
                let path = names.script(name)?;
                let bytes = read(storage.as_ref(), &path, 16 << 20, cancel)?
                    .with_context(|| format!("NNN script absent: {path}"))?;
                Ok(Arc::new(Program::parse(&bytes, scripts)?))
            });
            cancel.check()?;
            Ok(Prepared {
                boot,
                system_file,
                result_data,
                native_load,
                native_save,
                loader,
                services,
                resources,
                windows,
                startup,
                back_program,
            })
        })?);
        self.boot_admitted = true;
        Ok(())
    }
    fn begin_title(
        &mut self,
        host: &mut dyn HostServices,
        inherited_exit: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    ) -> Result<()> {
        let outer = self
            .outer
            .as_ref()
            .context("NNN Title original context absent")?;
        let profile = &self
            .shell
            .as_ref()
            .context("NNN Title original setup absent")?
            .profile;
        let spec =
            crate::native_title::NativeTitleSpec::from_profile(profile, &outer.configuration)?;
        let loader = outer.loader.clone();
        let name = profile.title_script.clone();
        self.title_program_task = Some(
            self.pool
                .as_ref()
                .context("NNN Title resource worker absent")?
                .spawn(move |cancel| loader(&name, &cancel))?,
        );
        let fill = if inherited_exit.is_some() {
            None
        } else {
            self.movie
                .as_ref()
                .context("NNN Title original mode handoff absent")?
                .spec
                .exit_fill
        };
        if let Some(image) = inherited_exit {
            ensure!(
                image.client() == self.client && image.size() == [800, 600],
                "NNN Title inherited exit scope"
            );
            self.title_exit = Some(image);
        } else if let Some(color) = fill {
            self.native_exit_available = true;
            self.title_exit_job = Some(host.image_metadata(self.client)?.begin_process(
                ImageInput::Solid {
                    size: [800, 600],
                    color: [color[0], color[1], color[2], 255],
                },
                b"[]".as_slice().into(),
            )?);
        } else {
            self.title_exit = Some(
                outer
                    .bridge
                    .presented_frame()
                    .context("NNN Title original unfilled exit frame unavailable")?,
            );
        }
        self.title = Some(crate::native_title::NativeTitlePlayer::new(
            self.client,
            9004,
            9100,
            [800, 600],
            u64::try_from(
                outer
                    .configuration
                    .setup
                    .integer(b"frameTime")?
                    .unwrap_or(50),
            )?,
            spec,
            self.title_resources
                .as_ref()
                .context("NNN Title scoped resources absent")?
                .clone(),
        )?);
        self.movie = None;
        self.shell.as_mut().unwrap().enter_title()?;
        Ok(())
    }
    fn prepare_title_context(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(task) = &mut self.title_program_task {
            if let Some(program) = task.try_take()? {
                self.title_program = Some(program);
                self.title_program_task = None;
            }
        }
        if let Some(job) = &mut self.title_exit_job {
            match job.poll()? {
                ImageProcessState::Pending => (),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN Title shared exit buffer: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == [800, 600],
                        "NNN Title exit resource scope"
                    );
                    self.title_exit = Some(image);
                    self.title_exit_job = None;
                }
            }
        }
        if !self
            .title
            .as_mut()
            .context("NNN Title actor absent")?
            .prepare(host)?
            || self.title_exit.is_none()
        {
            return Ok(false);
        }
        if self.vm.is_none() {
            let Some(program) = self.title_program.take() else {
                return Ok(false);
            };
            let mut outer = self
                .outer
                .take()
                .context("NNN Title continuation owner absent")?;
            let shell = self.shell.as_mut().unwrap();
            let count = outer.bridge.initialize_outer_title(
                host,
                u64::try_from(
                    outer
                        .configuration
                        .setup
                        .integer(b"frameTime")?
                        .unwrap_or(50),
                )?,
            )?;
            shell.update_launch_count(count);
            let profile = &shell.profile;
            self.vm = Some(NativeAdapter::from_outer_background(
                outer,
                profile.title_script.clone(),
                program,
                crate::native_shell::NativeMode::Title,
                profile.title_loop,
            )?);
        }
        Ok(true)
    }
    fn begin_title_newgame(&mut self) -> Result<()> {
        let profile = &self
            .shell
            .as_ref()
            .context("NNN Title native profile absent")?
            .profile;
        let returning = crate::native_general::ImmediateReturn::read(&profile.title, None)?;
        if returning.capture_exit {
            self.title_exit = Some(
                self.title
                    .as_ref()
                    .context("NNN Title selected actor absent")?
                    .completed_frame()?,
            );
            self.native_exit_available = true;
        } else {
            ensure!(
                self.native_exit_available,
                "NNN Title return requires an initialized original exit-screen buffer"
            );
        }
        let loader = self
            .vm
            .as_ref()
            .context("NNN Title background context absent")?
            .scoped_loader()?;
        self.title_program_task = Some(
            self.pool
                .as_ref()
                .context("NNN Title native loader worker absent")?
                .spawn(move |cancel| loader(b"sys", &cancel))?,
        );
        self.title_newgame_requested = true;
        Ok(())
    }
    fn begin_title_restart(&mut self, mode: crate::native_shell::NativeMode) -> Result<()> {
        use crate::native_shell::NativeMode;
        ensure!(
            matches!(
                mode,
                NativeMode::Nothing | NativeMode::Logo | NativeMode::Opening
            ),
            "NNN Title configured restart mode {mode:?} requires its original return consumer"
        );
        let profile = &self
            .shell
            .as_ref()
            .context("NNN Title restart setup absent")?
            .profile;
        let returning = crate::native_general::ImmediateReturn::read(&profile.title, Some(mode))?;
        if returning.capture_exit {
            self.title_exit = Some(
                self.title
                    .as_ref()
                    .context("NNN Title restart actor absent")?
                    .completed_frame()?,
            );
            self.native_exit_available = true;
        } else {
            ensure!(
                self.native_exit_available,
                "NNN Title restart requires an initialized original exit-screen buffer"
            );
        }
        let name = profile.opening_script.clone();
        let loader = self
            .vm
            .as_ref()
            .context("NNN Title restart native context absent")?
            .scoped_loader()?;
        self.title_program_task = Some(
            self.pool
                .as_ref()
                .context("NNN Title restart worker absent")?
                .spawn(move |cancel| loader(&name, &cancel))?,
        );
        self.title_restart_requested = Some(mode);
        Ok(())
    }
    fn poll_title_newgame(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        ensure!(
            self.title_newgame_requested || self.title_restart_requested.is_some(),
            "NNN Title return was not selected"
        );
        if let Some(task) = self.title_program_task.as_mut() {
            if let Some(program) = task.try_take()? {
                self.title_program = Some(program);
                self.title_program_task = None;
            }
        }
        if let Some(vm) = self.vm.as_mut() {
            if !vm.poll_outer_exit(host)? {
                return Ok(false);
            }
            self.outer = Some(self.vm.take().unwrap().into_cancelled_outer_state()?);
        }
        let Some(controller) = self.title_program.take() else {
            return Ok(false);
        };
        let mut outer = self
            .outer
            .take()
            .context("NNN Title completed background transfer absent")?;
        let exit = self
            .title_exit
            .as_ref()
            .context("NNN NewGame saved exit frame absent")?
            .clone();
        let vm = if let Some(mode) = self.title_restart_requested {
            let shell = self
                .shell
                .as_mut()
                .context("NNN Title restart shell absent")?;
            outer.bridge.initialize_outer_opening(
                host,
                u64::try_from(
                    outer
                        .configuration
                        .setup
                        .integer(b"frameTime")?
                        .unwrap_or(50),
                )?,
            )?;
            outer.bridge.set_outer_exit_frame(exit.clone())?;
            let vm = NativeAdapter::from_outer_background(
                outer,
                shell.profile.opening_script.clone(),
                controller,
                crate::native_shell::NativeMode::Opening,
                shell.profile.opening_loop,
            )?;
            shell.restart(host, mode, exit)?;
            vm
        } else {
            NativeAdapter::from_outer_newgame(outer, controller, host, exit)?
        };
        self.title
            .as_mut()
            .context("NNN Title selected actor absent")?
            .retire(host)?;
        self.vm = Some(vm);
        self.title = None;
        if self.title_restart_requested.is_none() {
            self.shell = None;
        }
        self.title_exit = None;
        self.title_newgame_requested = false;
        self.title_restart_requested = None;
        Ok(true)
    }
    fn poll_opening_return(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        use crate::native_shell::NativeMode;
        ensure!(
            self.opening_return_requested,
            "NNN Opening return was not requested"
        );
        let shell = self
            .shell
            .as_mut()
            .context("NNN Opening return setup absent")?;
        let next = shell.profile.opening_next;
        ensure!(
            matches!(next, NativeMode::Movie | NativeMode::Title),
            "NNN Opening configured next mode {next:?} requires its native return consumer"
        );
        if let Some(vm) = self.vm.as_mut() {
            if !vm.poll_outer_exit(host)? {
                return Ok(false);
            }
            self.outer = Some(self.vm.take().unwrap().into_cancelled_outer_state()?);
        }
        if next == NativeMode::Title {
            // Opening returns its next mode directly, unlike Title's explicit
            // ReturnFadeOut. Do not interpret ignored fade/configuration tokens
            // as a capture/fade callback. This image carries only the visible
            // completed frame until the Title background has produced one.
            let exit = self
                .outer
                .as_ref()
                .context("NNN Opening original owner absent")?
                .bridge
                .presented_frame()
                .or_else(|| shell.exit_frame())
                .context("NNN Opening completed visible handoff image absent")?;
            self.begin_title(host, Some(exit))?;
        } else {
            let outer = self
                .outer
                .as_mut()
                .context("NNN Opening original owner absent")?;
            let mut variables = outer
                .machine
                .variables()
                .context("NNN movie original variable bank absent")?
                .clone();
            ensure!(
                variables.bind_calculated_value(b"mode", NativeMode::Movie as i32)?,
                "NNN movie calculated mode binding absent"
            );
            let preferences = self
                .movie_preferences
                .as_ref()
                .context("NNN movie native preferences absent")?;
            let spec = crate::native_movie::NativeMovieSpec::from_profile(
                &shell.profile,
                &outer.configuration,
                &variables,
                preferences,
            )?;
            outer.machine.replace_variables(variables)?;
            outer.bridge.stop_for_outer_movie(host)?;
            self.movie = Some(crate::native_movie::NativeMoviePlayer::new(
                self.client,
                9002,
                9003,
                [800, 600],
                u64::try_from(
                    outer
                        .configuration
                        .setup
                        .integer(b"frameTime")?
                        .unwrap_or(50),
                )?,
                spec,
            )?);
            shell.enter_movie()?;
        }
        self.opening_return_requested = false;
        Ok(true)
    }
    fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> Result<VmYield> {
        if self.closing {
            if self.native_save.as_ref().is_some_and(|save| save.pending()) {
                self.poll_slot_save(host, InputFrame::default())?;
                return Ok(VmYield::BudgetExhausted);
            }
            return self.poll_close(host);
        }
        if self.shell.is_none()
            && self.outer.is_none()
            && self.title.is_none()
            && self.movie.is_none()
            && self.poll_slot_save(host, input)?
        {
            return Ok(VmYield::BudgetExhausted);
        }
        if self.shell.is_none()
            && self.outer.is_none()
            && self.title.is_none()
            && self.movie.is_none()
            && self.poll_slot_load(host, input)?
        {
            return Ok(VmYield::BudgetExhausted);
        }
        if self.title_newgame_requested || self.title_restart_requested.is_some() {
            let _ = self.poll_title_newgame(host)?;
            return Ok(VmYield::BudgetExhausted);
        }
        if self.opening_return_requested {
            let _ = self.poll_opening_return(host)?;
            return Ok(VmYield::BudgetExhausted);
        }
        if self.title.is_some() {
            if !self.prepare_title_context(host)? {
                return Ok(VmYield::BudgetExhausted);
            }
            let title = self.title.as_mut().unwrap();
            let vm = self.vm.as_mut().unwrap();
            if title.music_ready() {
                let yielded = vm.step(host, input, budget)?;
                if yielded == VmYield::Halted {
                    title.script_ended();
                }
            }
            let base = vm
                .presented_frame()
                .or_else(|| self.title_exit.clone())
                .context("NNN Title engine background absent")?;
            if let Some(action) = title.poll(host, input, base, vm)? {
                match action {
                    0 => self.begin_title_newgame()?,
                    // Actual Game vtable+240 -> 534b00 posts WM_CLOSE, same
                    // as the complete public PostCloseMessage implementation.
                    // Enter the existing scoped host-close/save protocol.
                    4 => ensure!(self.request_close(), "NNN Title native close was refused"),
                    _ => anyhow::bail!(
                        "NNN Title native action {action} requires its original Load/Config/Omake mode"
                    ),
                }
            } else if let Some(mode) = title.take_restart() {
                self.begin_title_restart(mode)?;
            }
            return Ok(VmYield::BudgetExhausted);
        }
        if let Some(movie) = &mut self.movie {
            if let Some(next) = movie.poll(host, input)? {
                ensure!(
                    next == crate::native_shell::NativeMode::Title,
                    "NNN unreviewed original Movie next mode {next:?}"
                );
                self.begin_title(host, None)?;
            }
            return Ok(VmYield::BudgetExhausted);
        }
        if let Some(vm) = &mut self.vm {
            if let Some(shell) = &mut self.shell {
                let entered = shell.mode() != crate::native_shell::NativeMode::Opening;
                if !shell.poll(host, input)? {
                    return Ok(VmYield::BudgetExhausted);
                }
                // GeneralInit resets native mouse state. The click that
                // finishes Logo must not also skip the newly entered Opening.
                let mouse = crysirror_core::service::buttons::MOUSE_LEFT
                    | crysirror_core::service::buttons::MOUSE_RIGHT
                    | crysirror_core::service::buttons::MOUSE_MIDDLE;
                let input = if entered {
                    InputFrame {
                        buttons_pressed: input.buttons_pressed & !mouse,
                        buttons_down: input.buttons_down & !mouse,
                        ..input
                    }
                } else {
                    input
                };
                let yielded = vm.step(host, input, budget)?;
                if vm.presented_frame().is_some() {
                    shell.release_cover(host)?;
                }
                if yielded == VmYield::Halted
                    || input.buttons_pressed
                        & (crysirror_core::service::buttons::MOUSE_LEFT
                            | crysirror_core::service::buttons::MOUSE_RIGHT)
                        != 0
                {
                    // CCommonOpening::Calcu uses either mouse click or the
                    // real background END. Drain admitted work, never forge
                    // ACK/NEXT to turn a click into a halted script.
                    self.opening_return_requested = true;
                    let _ = self.poll_opening_return(host)?;
                    return Ok(VmYield::BudgetExhausted);
                }
                return Ok(yielded);
            }
            return Ok(vm.step(host, input, budget)?);
        }
        if budget == 0 {
            return Ok(VmYield::BudgetExhausted);
        }
        if !self.started {
            self.start(host)?;
            return Ok(VmYield::BudgetExhausted);
        }
        if let Some(clock) = &mut self.civil {
            match clock.poll()? {
                CivilTimeState::Pending => (),
                CivilTimeState::Failed(reason) => anyhow::bail!("NNN host clock: {reason}"),
                CivilTimeState::Ready(time) => {
                    ensure!(time.client == self.client, "NNN clock scope");
                    self.clock = Some(u32::try_from(time.unix_ms.div_euclid(1000))?);
                    self.civil_sample = Some(time);
                    self.civil = None;
                }
            }
        }
        if !self.boot_admitted {
            let Some(sample) = self.civil_sample else {
                return Ok(VmYield::BudgetExhausted);
            };
            self.start_resources(host, sample)?;
        }
        if let Some(task) = &mut self.task {
            if let Some(prepared) = task.try_take()? {
                self.prepared = Some(prepared);
                self.task = None;
            }
        }
        let Some(prepared) = &mut self.prepared else {
            return Ok(VmYield::BudgetExhausted);
        };
        if self.clock.is_none() {
            return Ok(VmYield::BudgetExhausted);
        }
        let images = host.image_metadata(self.client)?;
        let mut pending = false;
        for (index, window) in prepared.windows.iter_mut().enumerate() {
            if window.resolved.is_some() {
                continue;
            }
            if let Some(source) = window.input.take() {
                window.job = Some(images.begin_process(source, window.effects.clone())?);
            }
            match window
                .job
                .as_mut()
                .context("NNN window image job missing")?
                .poll()?
            {
                ImageProcessState::Pending => pending = true,
                ImageProcessState::Failed(reason) => {
                    anyhow::bail!("NNN native message window: {reason}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(image.client() == self.client, "NNN window image scope");
                    let published = host.publish_image_resource(
                        self.client,
                        0x66000000 + index as u32,
                        image.clone(),
                    )?;
                    let path = published.path.relative().to_owned();
                    self.published.push(path.clone());
                    window.resolved = Some(resolved_window_resource(
                        5002 + index as u32,
                        &window.source,
                        path,
                        image,
                    )?);
                    window.job = None;
                }
            }
        }
        if pending {
            return Ok(VmYield::BudgetExhausted);
        }
        // All fallible resource work has completed before publishing the scene.
        let mut prepared = self.prepared.take().unwrap();
        prepared.boot.initialize_random_state(self.clock, None)?;
        let body = prepared
            .windows
            .first()
            .and_then(|window| window.resolved.clone());
        let name = prepared
            .windows
            .get(1)
            .and_then(|window| window.resolved.clone());
        let profile = prepared.services.message_profile_with_windows(
            MessageBindings {
                client: self.client,
                root: 1,
                body_layer: 5000,
                name_layer: 5001,
                order: 1000,
                canvas: SIZE,
                window: body,
            },
            &self.config.font,
            crysirror_core::service::buttons::MOUSE_LEFT | crysirror_core::service::buttons::ENTER,
            name,
        )?;
        host.text_measurement(self.client)?.measure_simple(
            &self.config.font,
            profile.body.size as f32,
            "",
        )?;
        let mut selection = prepared.services.selection_bridge(SelectionProfile {
            client: self.client,
            root: 1,
            widget_base: 6000,
            layer_base: 6000,
            order: 1100,
            canvas: SIZE,
            face: TextFace {
                font: self.config.font.clone(),
                size: profile.body.size,
                spacing: profile.body.spacing,
                color: profile.body.color,
            },
            rows: vec![],
            message_origins: vec![],
            single_byte_map: None,
        })?;
        selection.configure_native_background(&prepared.services.selection_setup)?;
        let mut bridge = Bridge::new(
            self.client,
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: SIZE,
                layers: 16,
                music_channel: 0,
            },
        )?;
        let frame_time = prepared.boot.setup.integer(b"frameTime")?.unwrap_or(50);
        bridge.set_frame_time(u64::try_from(frame_time)?)?;
        if matches!(self.config.entry, Entry::NewGame) {
            // Direct startup has no Title transition to invoke53a710. Apply
            // its52f710(0) preference reset before admitting sys.spt as well.
            prepared.system_file.set_auto_mode(false)?;
        }
        bridge.configure_system_file(prepared.system_file, &prepared.boot.setup)?;
        bridge.configure_result_data(prepared.result_data)?;
        self.native_load = prepared.native_load.map(native_load::Controller::new);
        self.native_save = prepared.native_save.map(native_save::Controller::new);
        let mut text = NativeTextBridge::new(profile)?;
        text.configure_native_timing(&prepared.services.print_setup)?;
        bridge.configure_text(text)?;
        bridge.configure_window_binding(
            prepared.services.clone(),
            prepared.resources.clone(),
            u32::try_from(prepared.boot.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?,
            self.pool
                .as_ref()
                .context("NNN native window task pool absent")?
                .clone(),
        )?;
        bridge.configure_selection(selection)?;
        bridge.configure_resources(
            prepared.resources.clone(),
            prepared.boot.resources.clone(),
            u32::try_from(prepared.boot.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?,
        )?;
        let resources = prepared.resources.clone();
        let names = prepared.services.resources.clone();
        bridge.configure_audio(crate::audio_bridge::AudioBridge::new(
            self.client,
            prepared.services.audio,
            prepared.services.music_names,
            prepared.services.sound_names,
            Arc::new(move |source, cancel| resources.audio(source, &names, cancel)),
        )?)?;
        if let Some(startup) = &prepared.startup {
            bridge.configure_system_sounds(
                startup.system_sounds.clone(),
                startup.system_sound_disabled,
                startup.system_sound_multi,
                64,
            )?;
        }
        let vm = if let Some(startup) = prepared.startup.take() {
            let frame_ms = u64::try_from(frame_time)?;
            self.movie_preferences = Some(prepared.services.preferences.clone());
            self.title_resources = Some(prepared.resources.clone());
            let shell = crate::native_shell::NativeStartupShell::new(
                self.client,
                1,
                SIZE,
                frame_ms,
                u32::try_from(prepared.services.preferences.launch_count)?,
                startup.clone(),
                prepared.resources,
            )?;
            let vm = NativeAdapter::from_background(
                prepared.boot,
                prepared.loader,
                bridge,
                startup.opening_script.clone(),
                prepared
                    .back_program
                    .take()
                    .context("NNN opening background SPT not prepared")?,
                crate::native_shell::NativeMode::Opening,
                startup.opening_loop,
            )?;
            self.shell = Some(shell);
            vm
        } else {
            NativeAdapter::from_bootstrap(prepared.boot, prepared.loader, bridge)?
        };
        host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,800,600],"paint":{"kind":"solid","color":[0,0,0,255]}}}]}))?)?;
        self.vm = Some(vm);
        Ok(VmYield::BudgetExhausted)
    }
}
impl VmAdapter for InstalledAdapter {
    fn presentation_pending(&self) -> bool {
        self.native_save.as_ref().is_some_and(|save| save.pending())
            || self.vm.as_ref().is_some_and(|vm| vm.presentation_pending())
    }
    fn preparation_pending(&self) -> bool {
        self.failure.is_none()
            && (self.native_save.as_ref().is_some_and(|save| save.pending())
                || self.native_load.as_ref().is_some_and(|load| load.pending())
                || (self.vm.is_none() && self.outer.is_none())
                || self.shell.as_ref().is_some_and(|s| s.preparation_pending()))
    }
    fn request_close(&mut self) -> bool {
        if self.failure.is_some() || (self.vm.is_none() && self.outer.is_none()) {
            return false;
        }
        self.closing = true;
        true
    }
    fn engine_id(&self) -> &'static str {
        "system-nnn"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        if let Some(reason) = &self.failure {
            return Err(Error::Io(reason.clone()));
        }
        match self.advance(host, input, budget) {
            Ok(result) => Ok(result),
            Err(reason) => {
                let reason = format!("System-NNN installed bootstrap/VM: {reason:#}");
                if self.vm.is_none() {
                    for path in self.published.drain(..) {
                        let _ = host.release_image_resource(self.client, &path);
                    }
                }
                self.failure = Some(reason.clone());
                Err(Error::Io(reason))
            }
        }
    }
    fn begin_original_restore(
        &self,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>>
    {
        if !self.started
            || self.failure.is_some()
            || self.closing
            || self.closed
            || self.shell.is_some()
            || self.outer.is_some()
            || self.movie.is_some()
            || self.title.is_some()
            || self.task.is_some()
            || self.prepared.is_some()
            || self.title_program_task.is_some()
            || self.title_exit_job.is_some()
        {
            return Err(Error::Unsupported(
                "NNN original restore requires stable native gameplay; active outer modes need their own saved actor",
            ));
        }
        if self.native_save.as_ref().is_some_and(|save| save.pending()) {
            return Err(Error::Unsupported(
                "NNN original restore during native slot save",
            ));
        }
        let vm = self
            .vm
            .as_ref()
            .ok_or(Error::Unsupported("NNN original gameplay VM absent"))?;
        let inner = vm.original_restore_job(bytes).map_err(|error| {
            Error::Io(format!(
                "NNN installed original restore admission: {error:#}"
            ))
        })?;
        Ok(Box::new(InstalledRestoreJob {
            inner,
            client: self.client,
            config: self.config.clone(),
            pool: self.pool.clone(),
            clock: self.clock,
            civil_sample: self.civil_sample,
            published: self.published.clone(),
            movie_preferences: self.movie_preferences.clone(),
            title_resources: self.title_resources.clone(),
            native_exit_available: self.native_exit_available,
            native_load: self.native_load.as_ref().map(|load| load.policy.clone()),
            native_save: self.native_save.as_ref().map(|save| save.restore_copy()),
            load_request: self.native_load.as_ref().and_then(|load| load.request()),
        }))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "System-NNN original resource/save reconstruction",
        ))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "System-NNN original resource/save reconstruction",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let mut value=self.vm.as_ref().and_then(|vm|serde_json::from_slice::<serde_json::Value>(&vm.diagnostics()).ok()).unwrap_or_else(||serde_json::json!({"phase":if self.started{"preparing"}else{"dormant"},"owns_framebuffer":false}));
        value["closing"] = serde_json::json!(self.closing);
        value["closed"] = serde_json::json!(self.closed);
        value["native_system_writes_completed"] = serde_json::json!(self.close_writes_done);
        if let Some(title) = &self.title {
            value["native_title"] = title.diagnostics();
            value["phase"] = "outer_title".into();
        }
        if let Some(movie) = &self.movie {
            value["native_movie"] = movie.diagnostics();
            value["phase"] = "outer_movie".into();
        }
        if let Some(outer) = &self.outer {
            value["pc"] = outer.machine.pc().into();
            value["instructions"] = outer.executed.into();
        }
        if let Some(shell) = &self.shell {
            value["native_shell"] = shell.diagnostics();
            value["dependency_pending"] = serde_json::json!(
                shell.dependency_pending()
                    || self.vm.as_ref().is_some_and(|vm| vm.dependency_pending())
                    || self
                        .movie
                        .as_ref()
                        .is_some_and(|movie| movie.dependency_pending())
                    || self.title_program_task.is_some()
                    || self.title_exit_job.is_some()
                    || self
                        .title
                        .as_ref()
                        .is_some_and(|title| title.dependency_pending())
            );
        }
        value["title_newgame_requested"] = self.title_newgame_requested.into();
        value["title_restart_requested"] =
            serde_json::json!(self.title_restart_requested.map(|m| format!("{m:?}")));
        value["opening_return_requested"] = self.opening_return_requested.into();
        value["factory_registered"] = true.into();
        value["bootstrap_pending"] = (self.task.is_some() || self.prepared.is_some()).into();
        value["error"] = serde_json::json!(self.failure);
        serde_json::to_vec(&value).unwrap_or_default()
    }
}

#[cfg(test)]
mod system_close_tests {
    use super::*;
    use crate::{
        XorProfile,
        config::Names,
        expression::{VariableProfile, Variables},
        machine::Machine,
        native_bootstrap::NativeConfiguration,
    };
    #[test]
    fn original_host_close_waits_for_both_scoped_system_writes_without_vm_clock_or_scene_work() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project: crysirror_runtime::Project =
            serde_json::from_value(serde_json::json!({"schema":2,
            "name":"native host close","content_revision":"1","assets":"game","canvas":[16,16],
            "clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]}))
            .unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let client = ClientId(1);
        let mut adapter = InstalledAdapter::new(
            client,
            Config {
                root: "".into(),
                module: "authored.exe".into(),
                font: "default".into(),
                entry: Entry::Controller,
            },
        )
        .unwrap();
        assert!(
            !adapter.request_close(),
            "dormant creation has no accepted native state to save"
        );
        let empty = Names::parse(b"", false).unwrap();
        let configuration = Arc::new(NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: empty.clone(),
            main_control: None,
            variable_names: empty.clone(),
            project: empty.clone(),
            commands: empty.clone(),
            functions: empty.clone(),
            initial: None,
            module_sha256: "authored-reviewed-layout".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        });
        let defaults = NativeSystemFile::defaults(
            SystemLimits::new(64, 20, 20),
            SystemStamp([2026, 11, 1, 0, 0, 0, 0, 0]),
            &SystemNames {
                font: b"fixture".to_vec(),
                surname: vec![],
                given_name: vec![],
            },
        )
        .unwrap();
        let mut bridge = Bridge::new(
            client,
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [16, 16],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge
            .configure_system_file(defaults.clone(), &empty)
            .unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        for (index, value) in [(199, 100), (200, 77), (299, -31), (300, 123)] {
            variables.write_stored(index, value);
        }
        adapter.outer = Some(crate::adapter::NativeOuterState {
            machine: Machine::with_variables(variables),
            bridge,
            configuration,
            loader: Arc::new(|_, _| anyhow::bail!("shutdown must not load another script")),
            executed: 0,
        });
        adapter.started = true;
        adapter.boot_admitted = true;
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        assert!(adapter.request_close());
        assert!(adapter.dependency_pending());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !adapter.closed {
            adapter.poll_dependency(&mut runtime.host).unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!adapter.dependency_pending());
        assert_eq!(adapter.close_writes_done, 2);
        assert_eq!(
            adapter
                .step(&mut runtime.host, InputFrame::default(), 1)
                .unwrap(),
            VmYield::Halted
        );
        let storage = runtime.host.storage(client).unwrap();
        let saved = storage
            .open_read(&VfsPath::new(Mount::Save, "sysfile.sav").unwrap())
            .unwrap();
        let bytes = crysirror_resource::read_all(saved.as_ref(), 1 << 20).unwrap();
        let loaded = NativeSystemFile::load(&bytes, &defaults).unwrap();
        assert_eq!(
            loaded.system_word(92).unwrap(),
            2,
            "original Game-loop and End each save once"
        );
        let mut restored = Variables::fresh(VariableProfile::Legacy1000);
        loaded
            .restore_variables(&mut restored, VariableProfile::Legacy1000)
            .unwrap();
        assert_eq!(restored.read_stored(200), 77);
        assert_eq!(restored.read_stored(299), -31);
        assert_eq!(restored.read_stored(199), 0);
        assert_eq!(restored.read_stored(300), 0);
        assert_eq!(runtime.host.now_ms(), 0);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
        assert_eq!(adapter.outer.as_ref().unwrap().machine.executed(), 0);
    }
}

#[cfg(test)]
mod native_return_tests {
    use super::*;
    use crate::{
        XorProfile,
        config::Names,
        expression::{VariableProfile, Variables},
        machine::Machine,
        native_bootstrap::NativeConfiguration,
        native_shell::{NativeMode, NativeStartupProfile, NativeStartupShell},
    };
    use std::time::{Duration, Instant};
    fn names(text: &str) -> Names {
        Names::parse(text.as_bytes(), false).unwrap()
    }
    fn fixture() -> (
        tempfile::TempDir,
        crysirror_runtime::Runtime,
        InstalledAdapter,
    ) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native mode return","content_revision":"1","assets":"game","canvas":[800,600],"clients":[{"name":"one","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        crysirror_formats_blackcyc::register_images(&mut runtime.assets.decoders).unwrap();
        runtime.tick(0, Default::default()).unwrap();
        let configuration = Arc::new(NativeConfiguration {
            main_control: None,
            resources: ResourceNames::new("").unwrap(),
            setup: names("frameTime 20 dwqPackLevel 0"),
            variable_names: names(""),
            project: names("back"),
            commands: names(""),
            functions: names(""),
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        });
        let profile=NativeStartupProfile::from_tables(&configuration,&names("nextMode logo"),&names("logoNumber 1 nextMode opening"),&names("backScriptFlag 1 backScriptName back nextMode title"),names("backScriptFlag 1 backScriptName back bgmNumber -1 basicButtonNumber 1 menu1Button1FileName button menu1Button1SizeX 2 menu1Button1SizeY 1 autoRestartTime 1"),names("movieFileName a.mpg")).unwrap();
        let image = crysirror_image::ImageSurface::filled(8, 4, [80, 90, 100, 255]).unwrap();
        let mut bytes = vec![b' '; 64];
        bytes[..4].copy_from_slice(b"PNG ");
        bytes[36..40].copy_from_slice(&8u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&4u32.to_le_bytes());
        bytes[48..64].copy_from_slice(b"PACKTYPE=8      ");
        bytes.extend(crysirror_image::encode_png(&image).unwrap());
        let bytes: Arc<[u8]> = bytes.into();
        let resources = NativeResources::blackcyc(
            ClientId(1),
            Arc::new(move |_, cancel| {
                cancel.check()?;
                Ok(Some(Arc::new(crysirror_core::service::MemoryAssetReader(
                    bytes.clone(),
                ))))
            }),
        )
        .unwrap();
        let mut shell = NativeStartupShell::new(
            ClientId(1),
            1,
            [800, 600],
            20,
            0,
            profile,
            resources.clone(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while shell.mode() != NativeMode::Opening {
            shell
                .poll(
                    &mut runtime.host,
                    InputFrame {
                        buttons_pressed: crysirror_core::service::buttons::MOUSE_LEFT,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([3, 0x66660002, 0x55560006]);
        words[18] = words.len() as u32;
        let program = Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|word| (!word).to_le_bytes())
                    .collect::<Vec<_>>(),
                XorProfile::default(),
            )
            .unwrap(),
        );
        let loader: ScriptLoader = Arc::new(move |name, cancel| {
            cancel.check()?;
            ensure!(name == b"back", "authored original project member");
            Ok(program.clone())
        });
        let preferences = crate::native_services_bootstrap::NativePreferences::original_defaults();
        let mut bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [800, 600],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge
            .configure_system_file(
                NativeSystemFile::defaults(
                    SystemLimits::new(64, 20, 20),
                    SystemStamp([2026, 11, 1, 0, 0, 0, 0, 0]),
                    &SystemNames {
                        font: b"authored".to_vec(),
                        surname: vec![],
                        given_name: vec![],
                    },
                )
                .unwrap(),
                &names(""),
            )
            .unwrap();
        bridge
            .configure_audio(
                crate::audio_bridge::AudioBridge::new(
                    ClientId(1),
                    crate::native_audio::NativeAudioPolicy::new(
                        preferences.audio,
                        crate::native_audio::AudioBindings {
                            music: 0,
                            sound_base: 16,
                            voice_base: 32,
                        },
                    )
                    .unwrap(),
                    names(""),
                    names(""),
                    Arc::new(|_, _| anyhow::bail!("no authored audio source expected")),
                )
                .unwrap(),
            )
            .unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.write_stored(0, 17);
        variables.write_stored(200, 55);
        variables.mark_calculated(30, b"mode".to_vec());
        let mut adapter = InstalledAdapter::new(
            ClientId(1),
            Config {
                root: "".into(),
                module: "authored.exe".into(),
                font: "default".into(),
                entry: Entry::Controller,
            },
        )
        .unwrap();
        adapter.started = true;
        adapter.pool = Some(TaskPool::new("nnn-return-fixture", 1, 1).unwrap());
        adapter.title_resources = Some(resources);
        adapter.shell = Some(shell);
        adapter.outer = Some(crate::adapter::NativeOuterState {
            machine: Machine::with_variables(variables),
            bridge,
            configuration,
            loader,
            executed: 0,
        });
        (dir, runtime, adapter)
    }
    #[test]
    fn installed_title_auto_restart_reenters_nothing_without_varinit_or_clock_and_retires_ui() {
        let (_dir, mut runtime, mut adapter) = fixture();
        let exit = adapter.shell.as_ref().unwrap().exit_frame().unwrap();
        adapter.begin_title(&mut runtime.host, Some(exit)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while adapter.title.is_some() {
            adapter
                .advance(&mut runtime.host, Default::default(), 32)
                .unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(adapter.shell.as_ref().unwrap().mode(), NativeMode::Nothing);
        assert!(!adapter.title_newgame_requested);
        assert!(adapter.title_restart_requested.is_none());
        assert!(runtime.host.engine.layers.get(ClientId(1), 9004).is_none());
        assert!(runtime.host.engine.layers.get(ClientId(1), 9001).is_some());
        assert_eq!(runtime.host.now_ms(), 0);
        let vm = adapter.vm.as_mut().unwrap();
        assert!(vm.poll_outer_exit(&mut runtime.host).unwrap());
        let state = adapter
            .vm
            .take()
            .unwrap()
            .into_cancelled_outer_state()
            .unwrap();
        let variables = state.machine.variables().unwrap();
        assert_eq!(variables.read_stored(0), 17);
        assert_eq!(variables.read_stored(200), 55);
        let mut bridge = state.bridge;
        assert_eq!(
            bridge.system_file_mut().unwrap().system_word(208).unwrap(),
            1
        );
        assert_eq!(
            bridge
                .initialize_outer_title(&mut runtime.host, 20)
                .unwrap(),
            1,
            "a second native Title Init keeps the process launch guard"
        );
    }
    #[test]
    fn installed_opening_click_returns_to_original_title_after_drain_without_fake_next() {
        let (_dir, mut runtime, mut adapter) = fixture();
        let state = adapter.outer.take().unwrap();
        let program = (state.loader)(b"back", &Cancellation::default()).unwrap();
        adapter.vm = Some(
            NativeAdapter::from_outer_background(
                state,
                b"back".to_vec(),
                program,
                NativeMode::Opening,
                false,
            )
            .unwrap(),
        );
        adapter
            .advance(
                &mut runtime.host,
                InputFrame {
                    buttons_pressed: crysirror_core::service::buttons::MOUSE_RIGHT,
                    ..Default::default()
                },
                1,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while adapter.title.is_none() {
            adapter.poll_dependency(&mut runtime.host).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(adapter.shell.as_ref().unwrap().mode(), NativeMode::Title);
        assert!(!adapter.opening_return_requested);
        assert_eq!(runtime.host.now_ms(), 0);
        assert!(adapter.outer.as_ref().unwrap().machine.executed() <= 1);
    }
}
