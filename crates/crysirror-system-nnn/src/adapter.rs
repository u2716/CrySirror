// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Core VM-port boundary: workers own original control/expressions; only the
//! engine owner submits scoped presentation/audio calls. Bootstrap resources
//! are prepared before construction; this is not a registered/playable factory.
use crate::{
    bridge::Bridge,
    machine::{Machine, NativeCall, Step},
    native_bootstrap::{NativeConfiguration, PreparedBoot},
    native_control::ScriptSession,
    Program,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, InputFrame},
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
mod menu_context;
mod scene;
mod user_gaiji;
use menu_context::MenuContext;

pub type ScriptLoader =
    Arc<dyn Fn(&[u8], &crysirror_tasks::Cancellation) -> Result<Arc<Program>> + Send + Sync>;
pub(crate) struct NativeOuterState {
    pub machine: Machine,
    pub bridge: Bridge,
    pub configuration: Arc<NativeConfiguration>,
    pub loader: ScriptLoader,
    pub executed: u64,
}
pub struct NativeAdapter {
    session: Option<ScriptSession>,
    loader: Option<ScriptLoader>,
    configuration: Option<Arc<NativeConfiguration>>,
    pending: Option<NativeCall>,
    worker: Option<Task<(ScriptSession, Result<Step>)>>,
    pool: TaskPool,
    bridge: Bridge,
    pc: u32,
    executed: u64,
    halted: bool,
    menu_context: Option<Box<MenuContext>>,
    scene_context: Option<Box<scene::SceneContext>>,
    user_gaiji: Option<user_gaiji::UserGaiji>,
    error: Option<String>,
    observed_script: Vec<u8>,
    observed_depth: usize,
    background_loop: Option<bool>,
    input_snapshot: InputFrame,
    instruction_base: u64,
    previous_mode_frame: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
    outer_exit_variables: Option<crate::expression::Variables>,
    restored_original: Option<(
        Arc<crate::native_save::NativeSaveFile>,
        crate::native_save::LegacySaveProfile,
    )>,
}
impl NativeAdapter {
    pub(crate) fn gallery_progress(&self) -> Result<crate::native_system_file::NativeSystemFile> {
        self.bridge.gallery_progress()
    }
    pub(crate) fn apply_system_message_action(&mut self,
        action:crate::native_message_control::SystemMessageAction,host:&mut dyn HostServices)->Result<()> {
        ensure!(self.worker.is_none()&&self.menu_context.is_none(),
            "NNN SystemMenu action before foreground restoration");
        self.bridge.apply_system_message_action(action,host)
    }
    pub(crate) fn resume_after_menu(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.bridge.resume_after_menu(host)
    }
    pub(crate) fn menu_thumbnail(
        &self,
        host: &mut dyn HostServices,
        image: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<crate::native_thumbnail::ThumbnailJob> {
        let config = self
            .configuration
            .as_ref()
            .context("NNN menu save configuration absent")?;
        crate::native_thumbnail::ThumbnailPolicy::prepare(&config.setup, self.bridge.canvas_size())?
            .from_image(self.bridge.client(), self.pool.clone(), host, image)
    }
    pub(crate) fn menu_settings(&self) -> Result<Vec<(i32, i32)>> {
        self.bridge.menu_settings()
    }
    pub(crate) fn apply_menu_setting(
        &mut self,
        id: i32,
        value: i32,
        host: &mut dyn HostServices,
    ) -> Result<Option<[i32; 4]>> {
        if let Some(config) = &self.configuration {
            let ignored = match id {
                103 | 203 => config.setup.integer(b"movieVolumeNotUse")?.unwrap_or(0) != 0,
                104 | 204 => {
                    config
                        .setup
                        .integer(b"soundVoiceVolumeNotUse")?
                        .unwrap_or(0)
                        != 0
                }
                _ => false,
            };
            if ignored {
                return Ok(None);
            }
        }
        self.bridge.apply_menu_setting(id, value, host)
    }
    pub(crate) fn begin_menu_settings_save(
        &mut self,
        host: &mut dyn HostServices,
        stamp: crate::native_system_file::SystemStamp,
    ) -> Result<crate::native_system_file::SystemWrite> {
        anyhow::ensure!(
            !self.dependency_pending(),
            "NNN settings save during a native dependency"
        );
        let session = self
            .session
            .as_ref()
            .context("NNN settings save session absent")?;
        let variables = match self.pending.as_ref() {
            Some(call) => session.machine().native_variables_candidate(call)?,
            None => session
                .machine()
                .variables()
                .context("NNN settings save variables absent")?
                .clone(),
        };
        let config = self
            .configuration
            .as_ref()
            .context("NNN settings save configuration absent")?;
        let trial = config.setup.integer(b"taikenFlag")?.unwrap_or(0) != 0;
        let storage = host.storage(self.bridge.client())?;
        self.bridge.system_file_mut()?.begin_save(
            storage.as_ref(),
            &variables,
            config.profile,
            stamp,
            trial,
        )
    }
    pub(crate) fn poll_menu_settings_save(
        &mut self,
        write: &mut crate::native_system_file::SystemWrite,
    ) -> Result<bool> {
        write.poll(self.bridge.system_file_mut()?)
    }
    /// Resource loading/decryption/initial values must be prepared on the
    /// caller's scoped resource workers. No filesystem access occurs here.
    pub fn from_prepared(
        program: Arc<Program>,
        machine: Machine,
        mut bridge: Bridge,
    ) -> Result<Self> {
        bridge.enter_foreground_context();
        Ok(Self {
            pc: machine.pc(),
            executed: machine.executed(),
            session: Some(ScriptSession::new(program, machine)),
            loader: None,
            configuration: None,
            pending: None,
            worker: None,
            pool: TaskPool::new("nnn-native-vm", 1, 1)?,
            bridge,
            halted: false,
            menu_context: None,
            scene_context: None,
            user_gaiji: None,
            error: None,
            observed_script: b"sys".to_vec(),
            observed_depth: 0,
            background_loop: None,
            input_snapshot: InputFrame::default(),
            instruction_base: 0,
            previous_mode_frame: None,
            outer_exit_variables: None,
            restored_original: None,
        })
    }
    /// Boot metadata and resource loading are prepared outside the owner loop.
    /// The loader is capability-scoped and called only by the admitted worker.
    pub fn from_bootstrap(
        boot: PreparedBoot,
        loader: ScriptLoader,
        mut bridge: Bridge,
    ) -> Result<Self> {
        bridge.enter_foreground_context();
        let pc = boot.session.machine().pc();
        let executed = boot.session.executed();
        bridge.set_frame_time(u64::try_from(
            boot.setup.integer(b"frameTime")?.unwrap_or(50),
        )?)?;
        bridge.configure_functions(boot.functions.clone())?;
        bridge.configure_chart(&boot.configuration())?;
        let configuration = Some(Arc::new(boot.configuration()));
        let observed_script = boot.session.script_name().to_vec();
        let observed_depth = boot.session.depth();
        Ok(Self {
            session: Some(boot.session),
            loader: Some(loader),
            configuration,
            pending: None,
            worker: None,
            pool: TaskPool::new("nnn-native-vm", 1, 1)?,
            bridge,
            pc,
            executed,
            halted: false,
            menu_context: None,
            scene_context: None,
            user_gaiji: None,
            error: None,
            observed_script,
            observed_depth,
            background_loop: None,
            input_snapshot: InputFrame::default(),
            instruction_base: 0,
            previous_mode_frame: None,
            outer_exit_variables: None,
            restored_original: None,
        })
    }
    pub(crate) fn from_original_restore(
        session: ScriptSession,
        mut bridge: Bridge,
        configuration: Arc<NativeConfiguration>,
        loader: ScriptLoader,
        pool: TaskPool,
        original: Arc<crate::native_save::NativeSaveFile>,
        profile: crate::native_save::LegacySaveProfile,
    ) -> Result<Self> {
        bridge.enter_foreground_context();
        if original.status_word(20)?==34 {
            let execution=crate::native_save::LiveExecution::capture(&session,&configuration,None)?;
            bridge.restore_chart_mode(crate::bridge::ChartContext {call:None,configuration:configuration.clone(),execution:Arc::new(execution),pool:pool.clone(),loader:loader.clone()})?;
        }
        Ok(Self {
            pc: session.machine().pc(),
            executed: session.executed(),
            observed_script: session.script_name().to_vec(),
            observed_depth: session.depth(),
            session: Some(session),
            loader: Some(loader),
            configuration: Some(configuration),
            pending: None,
            worker: None,
            pool,
            bridge,
            halted: false,
            menu_context: None,
            scene_context: None,
            user_gaiji: None,
            error: None,
            background_loop: None,
            input_snapshot: InputFrame::default(),
            instruction_base: 0,
            previous_mode_frame: None,
            outer_exit_variables: None,
            restored_original: Some((original, profile)),
        })
    }
    /// Pure logical producer for the pending native save boundary. Storage,
    /// thumbnail composition and the remaining original actors are prepared
    /// separately; capturing these fields never acknowledges PRINT/SELECT.
    pub(crate) fn original_save_execution(&self) -> Result<crate::native_save::LiveExecution> {
        anyhow::ensure!(
            self.original_input_mode().is_some(),
            "NNN original save requires a drained native PRINT/SELECT actor"
        );
        let session=self.session.as_ref().context("NNN original save session absent")?;
        let config=self.configuration.as_ref().context("NNN original save configuration absent")?;
        if self.bridge.chart_mode_active() {
            if let Some(call)=self.pending.as_ref() {return crate::native_save::LiveExecution::capture_general(session,config,call);}
        }
        crate::native_save::LiveExecution::capture(session,config,self.pending.as_ref())
    }
    pub(crate) fn write_original_execution_effects(
        &self,
        file: &mut crate::native_save::NativeSaveFile,
    ) -> Result<()> {
        let state = self.original_save_execution()?;
        let effects = self.bridge.original_effect_records()?;
        let mode = self
            .original_input_mode()
            .context("NNN original save actor absent")?;
        // Work on an original candidate: malformed sections cannot partly
        // overwrite the caller's native slot when a later producer fails.
        let mut candidate = file.clone();
        candidate.replace_live_execution(&state, mode)?;
        candidate.replace_effect_records(&effects)?;
        self.bridge
            .write_original_actor_state(&mut candidate, self.pending.as_ref())?;
        let profile = crate::native_save::LegacySaveProfile::for_configuration(
            self.configuration
                .as_ref()
                .context("NNN original save configuration absent")?,
        )?;
        self.bridge
            .write_original_auxiliary(&mut candidate, &profile)?;
        *file = candidate;
        Ok(())
    }
    pub(crate) fn begin_native_thumbnail(
        &self,
        host: &mut dyn HostServices,
    ) -> Result<crate::native_thumbnail::ThumbnailJob> {
        anyhow::ensure!(
            self.original_input_mode().is_some(),
            "NNN native save outside stable PRINT/SELECT"
        );
        let configuration = self
            .configuration
            .as_ref()
            .context("NNN native save configuration absent")?;
        crate::native_save::LegacySaveProfile::for_configuration(configuration)?
            .validate_live_actors()?;
        let policy = crate::native_thumbnail::ThumbnailPolicy::prepare(
            &configuration.setup,
            self.bridge.canvas_size(),
        )?;
        policy.begin(self.bridge.client(), self.pool.clone(), host)
    }
    pub(crate) fn produce_native_save(
        &mut self,
        thumbnail: crate::native_thumbnail::NativeThumbnail,
        stamp: crate::native_system_file::SystemStamp,
    ) -> Result<crate::native_save::NativeSaveFile> {
        let configuration = self
            .configuration
            .clone()
            .context("NNN native save configuration absent")?;
        let profile = crate::native_save::LegacySaveProfile::for_configuration(&configuration)?;
        let mut file = crate::native_save::NativeSaveFile::for_live(&profile, stamp, thumbnail)?;
        self.write_original_execution_effects(&mut file)?;
        let state = self
            .session
            .as_ref()
            .context("NNN native save session absent")?;
        let variables = if let Some(call) = self.pending.as_ref() {
            state.machine().native_variables_candidate(call)?
        } else {
            state
                .machine()
                .variables()
                .context("NNN native save variable bank absent")?
                .clone()
        };
        let date = variables.read_calculated(
            configuration
                .variable_names
                .index(b"date")
                .unwrap_or(u16::MAX),
        )?;
        let year = if let Some(id) = configuration.variable_names.index(b"year") {
            variables.read_calculated(id)?
        } else {
            2000
        };
        file.replace_status_word(16, date / 100)?;
        file.replace_status_word(17, date % 100)?;
        file.replace_status_word(
            19,
            crysirror_core::service::integer_weekday_wrapping(year, date / 100, date % 100),
        )?;
        let mut saved_remaining = None;
        if configuration
            .setup
            .integer(b"specialNumberForSaveSetFlag")?
            .unwrap_or(0)
            != 0
        {
            let fallback = encoding_rs::SHIFT_JIS.encode("残り回数").0;
            let name = configuration
                .setup
                .parameter(b"specialNumberForSaveVarName")
                .unwrap_or(fallback.as_ref());
            if let Some(id) = configuration.variable_names.index(name) {
                let value = variables.read_calculated(id)?;
                file.replace_status_word(46, value)?;
                saved_remaining = Some(value);
            }
        }
        file.validate(&profile)?;
        //53ad20's GetVarData calls are live native reads (the configured
        // special save number may even be rnd). Retain their shared RNG
        // outcome without acknowledging or advancing the suspended wait.
        let machine = self
            .session
            .as_mut()
            .context("NNN native save session absent")?
            .machine_mut();
        if let Some(call) = self.pending.as_ref() {
            machine.update_native_variables(call, variables)?;
        } else {
            machine.replace_variables(variables)?;
        }
        if let Some(value) = saved_remaining {
            self.bridge.commit_saved_remaining(value);
        }
        Ok(file)
    }
    pub(crate) fn original_restore_job(
        &self,
        bytes: Arc<[u8]>,
    ) -> Result<crate::native_game_restore::NativeRestoreJob> {
        anyhow::ensure!(
            self.worker.is_none()
                && self.error.is_none()
                && self.scene_context.is_none()
                && !self.user_gaiji_pending(),
            "NNN original restore requires an idle healthy VM outside scene replay"
        );
        anyhow::ensure!(
            self.pending.as_ref().is_none_or(|call| matches!(
                call.payload,
                crate::machine::Payload::Message { .. } | crate::machine::Payload::Selection { .. }
            )),
            "NNN original restore cannot cancel an in-flight native opcode"
        );
        let current = self
            .session
            .as_ref()
            .context("NNN original session unavailable")?
            .machine()
            .variables()
            .context("NNN original variable bank unavailable")?
            .clone();
        let configuration = self
            .configuration
            .clone()
            .context("NNN original source configuration unavailable")?;
        let loader = self
            .loader
            .clone()
            .context("NNN original scoped script loader unavailable")?;
        let mut seed = if self.background_loop.is_some() {
            anyhow::ensure!(
                self.pending.is_none() && self.native_background_mode()? == 4,
                "NNN original load requires an idle Title or foreground actor"
            );
            self.bridge.idle_title_restore_seed()?
        } else {
            self.bridge.original_restore_seed()?
        };
        seed.release_images.extend(self.inactive_menu_images());
        Ok(crate::native_game_restore::NativeRestoreJob::new(
            self.bridge.client(),
            bytes,
            seed,
            current,
            configuration,
            loader,
            self.pool.clone(),
        ))
    }
    /// Original outer Opening/Title modes run CBackExecClass on a separate
    /// script PC with the shared native variable bank and the same engine API
    /// bridge. The program is prepared by the capability loader, not copied.
    pub fn from_background(
        mut boot: PreparedBoot,
        loader: ScriptLoader,
        bridge: Bridge,
        name: Vec<u8>,
        program: Arc<Program>,
        mode: crate::native_shell::NativeMode,
        looping: bool,
    ) -> Result<Self> {
        anyhow::ensure!(
            matches!(
                mode,
                crate::native_shell::NativeMode::Opening | crate::native_shell::NativeMode::Title
            ),
            "NNN background native outer mode profile"
        );
        boot.resources.script(&name)?;
        let allow_read_marks = boot.setup.integer(b"backScriptSetCGOkFlag")?.unwrap_or(0) != 0;
        let mut variables = boot
            .session
            .machine()
            .variables()
            .context("NNN background variable bank absent")?
            .clone();
        anyhow::ensure!(
            variables.bind_calculated_value(b"mode", mode as i32)?,
            "NNN reviewed background mode calculated binding absent"
        );
        boot.session =
            ScriptSession::background(name, program, Machine::with_variables(variables))?;
        let mut vm = Self::from_bootstrap(boot, loader, bridge)?;
        vm.bridge
            .configure_background_read_marks(allow_read_marks)?;
        vm.background_loop = Some(looping);
        Ok(vm)
    }
    pub(crate) fn from_outer_background(
        mut state: NativeOuterState,
        name: Vec<u8>,
        program: Arc<Program>,
        mode: crate::native_shell::NativeMode,
        looping: bool,
    ) -> Result<Self> {
        anyhow::ensure!(
            matches!(
                mode,
                crate::native_shell::NativeMode::Opening | crate::native_shell::NativeMode::Title
            ),
            "NNN outer background original mode"
        );
        state.configuration.resources.script(&name)?;
        let mut variables = state
            .machine
            .variables()
            .context("NNN outer native variable bank absent")?
            .clone();
        anyhow::ensure!(
            variables.bind_calculated_value(b"mode", mode as i32)?,
            "NNN outer calculated mode binding absent"
        );
        state.bridge.enter_foreground_context();
        state.bridge.configure_background_read_marks(
            state
                .configuration
                .setup
                .integer(b"backScriptSetCGOkFlag")?
                .unwrap_or(0)
                != 0,
        )?;
        let session =
            ScriptSession::background(name.clone(), program, Machine::with_variables(variables))?;
        let previous_mode_frame = state.bridge.presented_frame();
        Ok(Self {
            session: Some(session),
            loader: Some(state.loader),
            configuration: Some(state.configuration),
            pending: None,
            worker: None,
            pool: TaskPool::new("nnn-native-vm", 1, 1)?,
            bridge: state.bridge,
            pc: 0,
            executed: state.executed,
            halted: false,
            menu_context: None,
            scene_context: None,
            user_gaiji: None,
            error: None,
            observed_script: name,
            observed_depth: 0,
            background_loop: Some(looping),
            input_snapshot: InputFrame::default(),
            instruction_base: state.executed,
            previous_mode_frame,
            outer_exit_variables: None,
            restored_original: None,
        })
    }
    /// Disable the original background context at an external mode boundary.
    /// Existing worker output and one admitted frame may finish; no next batch
    /// or frame is admitted and a cancelled receipt is never acknowledged.
    pub(crate) fn poll_outer_exit(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        anyhow::ensure!(
            self.background_loop.is_some(),
            "NNN outer exit requires background script"
        );
        self.poll_native_exit(host, false)
    }
    /// Window-close stops future native batches, while already-admitted work
    /// reaches the same real resource/frame boundary. No unknown receipt is ACKed.
    pub(crate) fn poll_shutdown_ready(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        self.cancel_user_gaiji(host)?;
        if self.scene_active()
            && !self.poll_scene_replay(host, InputFrame::default(), false, true)?
        {
            return Ok(false);
        }
        if !self.end_menu_background(host)? {
            return Ok(false);
        }
        self.poll_native_exit(host, true)
    }
    fn poll_native_exit(&mut self, host: &mut dyn HostServices, shutdown: bool) -> Result<bool> {
        if self.outer_exit_variables.is_some() {
            return Ok(true);
        }
        if let Some(error) = &self.error {
            anyhow::bail!("NNN cannot cancel failed original context: {error}");
        }
        if self.bridge.idle_background_pending() {
            self.bridge.poll_background_idle(
                self.session
                    .as_mut()
                    .context("NNN retained background session absent")?
                    .machine_mut(),
                host,
                false,
            )?;
            if self.bridge.idle_background_pending() {
                return Ok(false);
            }
        }
        if let Some(mut task) = self.worker.take() {
            let Some((session, step)) = task.try_take()? else {
                self.worker = Some(task);
                return Ok(false);
            };
            self.pc = session.machine().pc();
            self.executed = self.instruction_base.saturating_add(session.executed());
            self.observed_script = session.script_name().to_vec();
            self.observed_depth = session.depth();
            self.session = Some(session);
            match step? {
                Step::Native(call) => self.pending = Some(call),
                Step::Halted => self.halted = true,
                Step::Budget => (),
            }
        }
        if let Some(call) = self.pending.as_ref() {
            let session = self
                .session
                .as_mut()
                .context("NNN exiting background state absent")?;
            let program = session.program().clone();
            let candidate = if shutdown {
                self.bridge
                    .poll_shutdown(session.machine_mut(), call, &program, host)?
            } else {
                self.bridge
                    .poll_outer_exit(session.machine_mut(), call, &program, host)?
            };
            let Some(variables) = candidate else {
                return Ok(false);
            };
            // Resource callbacks that genuinely completed are counted; a
            // cancelled DRAW/OVERRAP keeps its original pending PC/count.
            self.pc = session.machine().pc();
            self.executed = self.instruction_base.saturating_add(session.executed());
            // The old pending PC is never advanced by a forged success.
            self.outer_exit_variables = Some(variables);
        } else {
            let mut variables = self
                .session
                .as_ref()
                .context("NNN exiting background state absent")?
                .machine()
                .variables()
                .context("NNN outer variable bank absent")?
                .clone();
            variables.finish_native_instruction();
            self.outer_exit_variables = Some(variables);
        }
        Ok(true)
    }
    pub(crate) fn into_cancelled_outer_state(mut self) -> Result<NativeOuterState> {
        anyhow::ensure!(
            self.worker.is_none(),
            "NNN background worker still owns context"
        );
        let variables = self
            .outer_exit_variables
            .take()
            .context("NNN background exit has not reached a completed frame")?;
        Ok(NativeOuterState {
            machine: Machine::with_variables(variables),
            bridge: self.bridge,
            configuration: self
                .configuration
                .context("NNN outer configuration absent")?,
            loader: self.loader.context("NNN outer scoped loader absent")?,
            executed: self.executed,
        })
    }
    pub(crate) fn from_outer_newgame(
        state: NativeOuterState,
        controller: Arc<Program>,
        host: &mut dyn HostServices,
        exit: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<Self> {
        Self::from_outer_newgame_return(state, controller, host, exit, None)
    }
    pub(crate) fn from_outer_newgame_return(
        mut state: NativeOuterState,
        controller: Arc<Program>,
        host: &mut dyn HostServices,
        exit: Arc<dyn crysirror_core::image_processing::ImageResource>,
        script_return: Option<(Option<u16>, i32)>,
    ) -> Result<Self> {
        let mut variables = state
            .machine
            .variables()
            .context("NNN NewGame variable bank absent")?
            .clone();
        crate::config::initialize_new_game(
            &mut variables,
            state.configuration.profile,
            state.configuration.initial.as_ref(),
        )?;
        if let Some((Some(variable), value)) = script_return {
            variables.write_stored(variable, value);
        }
        state.bridge.initialize_outer_newgame(host, exit)?;
        let mut vm =
            Self::from_prepared(controller, Machine::with_variables(variables), state.bridge)?;
        vm.loader = Some(state.loader);
        vm.configuration = Some(state.configuration);
        vm.instruction_base = state.executed;
        vm.executed = state.executed;
        Ok(vm)
    }
    pub(crate) fn take_result_notices(&mut self) -> std::collections::VecDeque<i32> {
        self.bridge.take_result_notices()
    }
    pub(crate) fn poll_system_voice(&mut self,host:&mut dyn HostServices,name:&[u8])->Result<bool> {
        self.bridge.poll_system_voice(host,name)
    }
    pub(crate) fn poll_system_sound(
        &mut self,
        host: &mut dyn HostServices,
        number: i32,
    ) -> Result<bool> {
        self.bridge.poll_system_sound(host, number)
    }
    pub(crate) fn poll_outer_music(
        &mut self,
        host: &mut dyn HostServices,
        words: [i32; 4],
        restart: bool,
    ) -> Result<bool> {
        self.bridge.poll_outer_music(host, words, restart)
    }
    pub(crate) fn poll_listen_music(
        &mut self,
        host: &mut dyn HostServices,
        music: Option<crate::native_audio::ListenMusic>,
    ) -> Result<bool> {
        self.bridge.poll_listen_music(host, music)
    }
    pub(crate) fn listen_music_playing(&self, host: &dyn HostServices) -> Result<bool> {
        self.bridge.listen_music_playing(host)
    }
    pub(crate) fn poll_gallery_audio(&mut self, host: &mut dyn HostServices, event: &crate::native_cg::CgAudio) -> Result<bool> {
        self.bridge.poll_gallery_audio(host,event)
    }
    pub(crate) fn presented_frame(
        &self,
    ) -> Option<Arc<dyn crysirror_core::image_processing::ImageResource>> {
        let current = self.bridge.presented_frame()?;
        if self
            .previous_mode_frame
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &current))
        {
            None
        } else {
            Some(current)
        }
    }
    fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
        schedule_next: bool,
    ) -> Result<VmYield> {
        if self.user_gaiji_pending() && !self.poll_user_gaiji(host)? {
            return Ok(VmYield::BudgetExhausted);
        }
        self.bridge.set_frame_admission(schedule_next);
        if schedule_next {
            self.input_snapshot = input;
            self.bridge.observe_message_keys(input);
        }
        if let Some(error) = &self.error {
            anyhow::bail!("System-NNN terminal error: {error}");
        }
        if self.halted {
            if self.background_loop.is_some() {
                self.bridge.poll_background_idle(
                    self.session
                        .as_mut()
                        .context("NNN ended background reader absent")?
                        .machine_mut(),
                    host,
                    schedule_next,
                )?;
            }
            return Ok(VmYield::Halted);
        }
        if budget == 0 {
            return Ok(VmYield::BudgetExhausted);
        }
        if self.bridge.chart_mode_active() && !self.menu_background_active() {
            return Ok(VmYield::WaitForInput);
        }
        if self.bridge.saved_wait_pending() {
            // Dependency pumping never consumes a post-load input edge. These
            // waits precede the next original instruction; no fake callback is
            // acknowledged and no worker batch may run while they are active.
            if !schedule_next {
                let session = self.session.as_mut().context("NNN saved session absent")?;
                self.bridge
                    .poll_saved_frame_dependency(session.machine_mut(), host)?;
                return Ok(VmYield::WaitForInput);
            }
            anyhow::ensure!(
                self.worker.is_none() && self.pending.is_none(),
                "NNN saved wait overlaps a native instruction"
            );
            let session = self.session.as_mut().context("NNN saved session absent")?;
            let program = session.program().clone();
            if !self
                .bridge
                .poll_saved_wait(session.machine_mut(), &program, host, input)?
            {
                return Ok(VmYield::WaitForInput);
            }
            self.pc = session.machine().pc();
        }
        if let Some(mut task) = self.worker.take() {
            match task.try_take()? {
                None => {
                    self.worker = Some(task);
                    return Ok(VmYield::BudgetExhausted);
                }
                Some((session, step)) => {
                    self.pc = session.machine().pc();
                    self.executed = self.instruction_base.saturating_add(session.executed());
                    self.observed_script = session.script_name().to_vec();
                    self.observed_depth = session.depth();
                    self.session = Some(session);
                    match step? {
                        Step::Halted => {
                            self.halted = true;
                            if self.background_loop.is_some() {
                                // NEXT finishes this admitted background batch,
                                // including its final painter pass. Later pumps
                                // can complete it without executing another SPT.
                                self.bridge.request_background_repaint();
                                self.bridge.poll_background_idle(
                                    self.session.as_mut().unwrap().machine_mut(),
                                    host,
                                    true,
                                )?;
                            }
                            return Ok(VmYield::Halted);
                        }
                        Step::Budget => {}
                        Step::Native(call) => {
                            self.pending = Some(call);
                        }
                    }
                }
            }
        }
        if self.try_scene_boundary(host)? {
            return Ok(VmYield::BudgetExhausted);
        }
        if let Some(call) = self.pending.as_ref() {
            anyhow::ensure!(
                !self.menu_background_active()
                    || !matches!(
                        call.payload,
                        crate::machine::Payload::Message { .. }
                            | crate::machine::Payload::Selection { .. }
                    ),
                "NNN menu background requested a foreground General-mode transfer"
            );
            self.bridge
                .set_system_script(self.configuration.as_ref().map(|configuration| {
                    configuration
                        .project
                        .index(&self.observed_script)
                        .unwrap_or(0)
                }));
            let program = self.session.as_ref().unwrap().program().clone();
            if self.bridge.chart_context_needed(call)? {
                let configuration=self.configuration.as_ref().context("NNN ChartMenu configuration absent")?.clone();
                let execution=crate::native_save::LiveExecution::capture_general(self.session.as_ref().unwrap(),&configuration,call)?;
                self.bridge.bind_chart_context(crate::bridge::ChartContext {call:Some(call.clone()),configuration,execution:Arc::new(execution),pool:self.pool.clone(),loader:self.loader.clone().context("NNN ChartMenu native script loader absent")?})?;
            }
            if !self.bridge.try_commit_with_context(
                self.session.as_mut().unwrap().machine_mut(),
                call,
                &program,
                input,
                host,
            )? {
                return Ok(VmYield::BudgetExhausted);
            }
            self.pending = None;
            let session = self.session.as_ref().unwrap();
            self.pc = session.machine().pc();
            self.executed = self.instruction_base.saturating_add(session.executed());
        }
        if !schedule_next {
            return Ok(VmYield::BudgetExhausted);
        }
        // Reserve capacity before transferring VM state. The worker owns no
        // HostServices/window/GPU/audio pointer, and never joins its own pool.
        let permit = self.pool.reserve()?;
        let mut session = self
            .session
            .take()
            .context("System-NNN VM ownership absent")?;
        // GetCalcuVarData mouse/button fields are sampled engine input. Freeze
        // them at VM admission; workers do not query or mutate the host input.
        if let Some(current) = session.machine().variables() {
            let mut variables = current.clone();
            self.bridge.bind_preference_variables(&mut variables)?;
            let input = self.input_snapshot;
            for (name, value) in [
                (b"mousex".as_slice(), input.pointer_x),
                (b"mousey", input.pointer_y),
                (
                    b"trig1",
                    i32::from(
                        input.buttons_down & crysirror_core::service::buttons::MOUSE_LEFT != 0,
                    ),
                ),
                (
                    b"trig2",
                    i32::from(
                        input.buttons_down & crysirror_core::service::buttons::MOUSE_RIGHT != 0,
                    ),
                ),
                (
                    b"trig3",
                    i32::from(
                        input.buttons_down & crysirror_core::service::buttons::MOUSE_MIDDLE != 0,
                    ),
                ),
                (
                    b"click1",
                    i32::from(
                        input.buttons_pressed & crysirror_core::service::buttons::MOUSE_LEFT != 0,
                    ),
                ),
                (
                    b"click2",
                    i32::from(
                        input.buttons_pressed & crysirror_core::service::buttons::MOUSE_RIGHT != 0,
                    ),
                ),
                (
                    b"click3",
                    i32::from(
                        input.buttons_pressed & crysirror_core::service::buttons::MOUSE_MIDDLE != 0,
                    ),
                ),
            ] {
                variables.bind_calculated_value(name, value)?;
            }
            session.machine_mut().replace_variables(variables)?;
        }
        let loader = self.loader.clone();
        let background_loop = self.background_loop;
        self.worker = Some(permit.spawn(move |cancel| {
            cancel.check()?;
            let result = (|| -> Result<Step> {
                // Each original instruction/control transfer spends admission
                // budget; even a cyclic controller yields to the owner.
                for _ in 0..budget.min(10000) {
                    cancel.check()?;
                    match session.poll(1)? {
                        Step::Native(call) => {
                            if let Some(looping)=background_loop {
                                if session.depth()==0&&matches!(call.payload,crate::machine::Payload::Control{code:0x55560006,..}) {
                                    session.acknowledge(&call)?;
                                    if looping {session.restart_background()?;continue;}
                                    return Ok(Step::Halted);
                                }
                            }
                            if let Some(load) = &loader {
                                if session.dispatch_control(&call, |name| load(name, &cancel))? {
                                    continue;
                                }
                            }
                            return Ok(Step::Native(call));
                        }
                        Step::Halted => {
                            anyhow::ensure!(background_loop.is_none(),"NNN background script reached END instead of original NEXT completion");
                            if loader.is_some() {
                                session.restart_controller_after_end()?;
                            } else {
                                return Ok(Step::Halted);
                            }
                        }
                        Step::Budget => {}
                    }
                }
                Ok(Step::Budget)
            })();
            cancel.check()?;
            Ok((session, result))
        })?);
        Ok(VmYield::BudgetExhausted)
    }
    /// Non-mutating readiness query for an offline host dependency fence.
    pub fn dependency_pending(&self) -> bool {
        if self.user_gaiji_pending() {
            return true;
        }
        self.worker.is_some() || self.bridge.dependency_pending()
    }
    /// Poll only already-admitted work and commit its owner receipt. Never
    /// samples input, advances a clock, or schedules the next VM batch.
    pub fn poll_dependency(&mut self, host: &mut dyn HostServices) -> crysirror_core::Result<()> {
        if !self.dependency_pending() {
            return Ok(());
        }
        match self.advance(host, InputFrame::default(), 1024, false) {
            Ok(_) => Ok(()),
            Err(error) => {
                let text = format!(
                    "System-NNN original pc={:#x} instructions={}: {error:#}",
                    self.pc, self.executed
                );
                self.error = Some(text.clone());
                Err(Error::Io(text))
            }
        }
    }
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }
    /// Owner transition after original background NEXT. Does not serialize or
    /// reconstruct a native save and refuses a task/callback still in flight.
    pub fn into_idle_state(self) -> Result<(Machine, Bridge)> {
        anyhow::ensure!(
            self.halted && self.worker.is_none() && self.pending.is_none(),
            "NNN outer transition requires completed original background NEXT"
        );
        let session = self
            .session
            .as_ref()
            .context("NNN completed background state absent")?;
        anyhow::ensure!(
            session.depth() == 0,
            "NNN completed background has a subcontext"
        );
        Ok((session.machine().clone(), self.bridge))
    }
    pub(crate) fn into_outer_state(self) -> Result<NativeOuterState> {
        let configuration = self
            .configuration
            .clone()
            .context("NNN outer configuration absent")?;
        let loader = self
            .loader
            .clone()
            .context("NNN outer scoped script loader absent")?;
        let executed = self.executed;
        let (machine, bridge) = self.into_idle_state()?;
        Ok(NativeOuterState {
            machine,
            bridge,
            configuration,
            loader,
            executed,
        })
    }
    pub(crate) fn scoped_loader(&self) -> Result<ScriptLoader> {
        self.loader
            .clone()
            .context("NNN scoped script loader absent")
    }
    pub fn configuration(&self) -> Option<&Arc<NativeConfiguration>> {
        self.configuration.as_ref()
    }
    pub(crate) fn frame_period_ms(&self) -> u64 {
        self.bridge.frame_period_ms()
    }
    pub(crate) fn retained_mode_exit_frame(&self)->Option<Arc<dyn crysirror_core::image_processing::ImageResource>> {self.bridge.retained_mode_exit_frame()}
    /// General's exit-screen buffer becomes the opening script's immutable
    /// overlap source before any callback/worker is admitted. It is inherited
    /// state, not a frame produced by that script, so keep the cover visible.
    pub(crate) fn inherit_outer_exit_frame(
        &mut self,
        image: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<()> {
        anyhow::ensure!(
            self.background_loop.is_some()
                && self.worker.is_none()
                && self.pending.is_none()
                && self
                    .session
                    .as_ref()
                    .is_some_and(|session| session.executed() == 0),
            "NNN outer exit inheritance needs an unstarted background context"
        );
        self.bridge.set_outer_exit_frame(image.clone())?;
        self.previous_mode_frame = Some(image);
        Ok(())
    }
    /// Original Load may interrupt a stable PRINT/SELECT actor, never a VM
    /// worker or a half-published resource/native callback.
    pub(crate) fn original_input_mode(&self) -> Option<i32> {
        if self.worker.is_some()
            || self.error.is_some()
            || self.background_loop.is_some()
            || self.session.is_none()
            || self.bridge.scene_replay_active()
            || self.bridge.dependency_pending()
        {
            return None;
        }
        match self.pending.as_ref().map(|call| &call.payload) {
            Some(crate::machine::Payload::UserCall{command:true,..}) if self.bridge.chart_mode_active()=>Some(34),
            Some(crate::machine::Payload::Message { .. }) => Some(10),
            Some(crate::machine::Payload::Selection { .. }) => Some(9),
            None if self.bridge.chart_mode_active()=>Some(34),
            None => match self.bridge.wait_kind(None) {
                "saved_message_input" => Some(10),
                "saved_selection_input" => Some(9),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn take_chart_mode_request(&mut self)->bool {self.bridge.take_chart_mode_request()}
    pub(crate) fn chart_view(&self)->Result<(&crate::native_chart::ChartRuntime,&crate::native_chart::ChartState)> {self.bridge.chart_view()}
    pub(crate) fn poll_chart_return(&mut self,slot:i32,host:&mut dyn HostServices)->Result<bool> {
        anyhow::ensure!(self.worker.is_none()&&self.menu_context.is_none(),"NNN chart return requires its foreground reader");
        let call=self.pending.as_ref();
        match self.bridge.poll_chart_return(self.session.as_mut().unwrap().machine_mut(),call,slot,host)? {
            crate::bridge::ChartReturn::Pending=>return Ok(false),
            crate::bridge::ChartReturn::Continue=>{},
            crate::bridge::ChartReturn::Restored(session)=>{
                self.instruction_base=self.executed;
                self.session=Some(session);
                self.halted=false;
                self.background_loop=None;
            }
        }
        self.pending=None;
        let session=self.session.as_ref().unwrap();
        self.pc=session.machine().pc();
        self.observed_script=session.script_name().to_vec();
        self.observed_depth=session.depth();
        self.executed=self.instruction_base.saturating_add(session.executed());
        Ok(true)
    }
    pub(crate) fn publish_general_variables(&mut self,variables:crate::expression::Variables)->Result<()> {
        ensure!(self.worker.is_none(),"NNN General variable publication while VM worker owns its reader");
        let machine=self.session.as_mut().context("NNN General script reader absent")?.machine_mut();
        if let Some(call)=self.pending.as_ref() {machine.update_native_variables(call,variables)?;} else {machine.replace_variables(variables)?;}
        Ok(())
    }
}
impl VmAdapter for NativeAdapter {
    fn presentation_pending(&self) -> bool {
        self.bridge.presentation_pending()
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
        match self.advance(host, input, budget, true) {
            Ok(result) => Ok(result),
            Err(error) => {
                let text = format!(
                    "System-NNN original pc={:#x} instructions={}: {error:#}",
                    self.pc, self.executed
                );
                self.error = Some(text.clone());
                Err(Error::Io(text))
            }
        }
    }
    fn begin_original_restore(
        &self,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>>
    {
        self.original_restore_job(bytes)
            .map(|job| {
                Box::new(job) as Box<dyn crysirror_core::native_restore::OriginalRestorePreparation>
            })
            .map_err(|error| Error::Io(format!("NNN original restore admission: {error:#}")))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "System-NNN original resource/save reconstruction",
        ))
    }
    fn restore(&mut self, _data: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "System-NNN original resource/save reconstruction",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
        "vm":"original-system-nnn-spt","background_loop":self.background_loop,"native_location":format!("pc={:#x}",self.pc),
        "wait_kind":if self.worker.is_some(){"vm_worker"}else{self.bridge.wait_kind(self.pending.as_ref())},"pending_call":self.pending.as_ref().map(|call|format!("identify={:#x} code={:#x} {:?}",call.identify,call.code,call.payload).chars().take(512).collect::<String>()),"dependency_pending":self.dependency_pending(),"pending_dependencies":self.bridge.pending_dependencies(),
        "input_capabilities":{"left_right_mouse":true,"middle_mouse":true},"instructions":self.executed,"worker_pending":self.worker.is_some(),"resource_pending":self.pending.is_some(),"script":String::from_utf8_lossy(&self.observed_script),"script_depth":self.observed_depth,"error":self.error,
        "engine_api_calls":self.bridge.api_calls,"overlap_captures":self.bridge.overlap_captures,
        "story_name_bytes":self.bridge.metadata.story_name.len(),"film_name_bytes":self.bridge.metadata.film_name.len(),
        "effect_counters":self.bridge.passive_effect_counts(),"original_save_restored":self.restored_original.is_some(),"factory_registered":false,"owns_framebuffer":false,"halted":self.halted
    })).unwrap_or_default()
    }
}

#[cfg(test)]
mod outer_newgame_tests {
    use super::*;
    use crate::{
        bridge::{CallbackProfile, SceneBindings},
        config::Names,
        expression::{VariableProfile, Variables},
        native_bootstrap::NativeConfiguration,
        native_resources::ResourceNames,
        XorProfile,
    };
    use crysirror_core::image_processing::{ImageInput, ImageProcessState};
    use crysirror_core::state::ClientId;
    use std::time::{Duration, Instant};
    #[test]
    fn newgame_resets_original_variable_ranges_and_controller_without_owner_io_or_synthetic_mode() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"newgame","content_revision":"1","assets":"assets","canvas":[2,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        let mut job = rt
            .host
            .image_metadata(ClientId(1))
            .unwrap()
            .begin_process(
                ImageInput::Solid {
                    size: [2, 1],
                    color: [1, 2, 3, 255],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        let exit = loop {
            match job.poll().unwrap() {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(e) => panic!("{e}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < end);
                    std::thread::yield_now();
                }
            }
        };
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([3, 0x66660002, 0x55560005]);
        words[18] = 35;
        let controller = Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|w| (!w).to_le_bytes())
                    .collect::<Vec<_>>(),
                XorProfile::default(),
            )
            .unwrap(),
        );
        let names = Names::parse(b"", false).unwrap();
        let config = Arc::new(NativeConfiguration {
            main_control: None,
            chart_setup: None,
            resources: ResourceNames::new("").unwrap(),
            setup: names.clone(),
            variable_names: names.clone(),
            project: names.clone(),
            commands: names.clone(),
            functions: names,
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        });
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.write_stored(100, 11);
        variables.write_stored(200, 22);
        variables.write_stored(299, 33);
        variables.write_stored(300, 44);
        variables.mark_calculated(6, b"mode".to_vec());
        variables.bind_calculated_value(b"mode", 4).unwrap();
        let random = crysirror_image::temporal_pattern::PatternRng::msvc(7);
        variables.replace_random_state(random.clone()).unwrap();
        let bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [2, 1],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        let outer = NativeOuterState {
            machine: Machine::with_variables(variables),
            bridge,
            configuration: config,
            loader: Arc::new(|_, _| anyhow::bail!("owner must not read source")),
            executed: 261,
        };
        let vm = NativeAdapter::from_outer_newgame(outer, controller, &mut rt.host, exit.clone())
            .unwrap();
        let session = vm.session.as_ref().unwrap();
        let variables = session.machine().variables().unwrap();
        assert_eq!(session.script_name(), b"sys");
        assert_eq!(session.depth(), 0);
        assert_eq!(session.machine().pc(), 0);
        assert!(session.machine().return_frames().is_empty());
        assert_eq!(variables.read_stored(100), 0);
        assert_eq!(variables.read_stored(300), 0);
        assert_eq!(variables.read_stored(200), 22);
        assert_eq!(variables.read_stored(299), 33);
        assert_eq!(
            variables.read_calculated(6).unwrap(),
            4,
            "NewGame does not assign an invented mode0"
        );
        assert_eq!(
            serde_json::to_vec(&variables.random_state().unwrap()).unwrap(),
            serde_json::to_vec(&random).unwrap()
        );
        assert!(Arc::ptr_eq(&vm.bridge.presented_frame().unwrap(), &exit));
        assert_eq!(vm.executed, 261);
        assert_eq!(rt.host.now_ms(), 0);
    }
    #[test]
    fn outer_exit_does_not_swallow_an_unknown_pending_native_call() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"unknown exit","content_revision":"1","assets":"assets","canvas":[2,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([6, 0x66660007, 0x33330013, 2, 0, 255]);
        words[18] = 38;
        let program = Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|w| (!w).to_le_bytes())
                    .collect::<Vec<_>>(),
                XorProfile::default(),
            )
            .unwrap(),
        );
        let mut bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [2, 1],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge.configure_background_read_marks(false).unwrap();
        // from_prepared correctly clears background policy; the authored fixture
        // explicitly binds the background context as the installed driver does.
        let mut vm = NativeAdapter::from_prepared(
            program,
            Machine::with_variables(Variables::fresh(VariableProfile::Legacy1000)),
            bridge,
        )
        .unwrap();
        vm.bridge.configure_background_read_marks(false).unwrap();
        vm.background_loop = Some(false);
        vm.advance(&mut rt.host, InputFrame::default(), 100, true)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let error = loop {
            match vm.poll_outer_exit(&mut rt.host) {
                Err(error) => break error,
                Ok(false) => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                Ok(true) => panic!("unknown native was discarded as a success"),
            }
        };
        assert!(error.to_string().contains("unsupported"));
        let pending = vm.pending.as_ref().unwrap();
        assert_eq!(pending.pc, 32);
        assert_eq!(vm.session.as_ref().unwrap().machine().pc(), 32);
        vm.session
            .as_ref()
            .unwrap()
            .machine()
            .validate_receipt(pending)
            .unwrap();
        assert!(vm.outer_exit_variables.is_none());
        vm.background_loop = None;
        vm.bridge.enter_foreground_context();
        assert!(vm
            .poll_shutdown_ready(&mut rt.host)
            .unwrap_err()
            .to_string()
            .contains("unsupported"));
        assert_eq!(vm.session.as_ref().unwrap().machine().pc(), 32);
        assert!(
            vm.outer_exit_variables.is_none(),
            "OS close is not an unknown-native acknowledgement"
        );
        assert_eq!(rt.host.now_ms(), 0);
    }
}
