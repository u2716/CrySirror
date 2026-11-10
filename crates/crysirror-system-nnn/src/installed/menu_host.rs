// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One modal mode owner. Native waits are retained while workers prepare slots
//! or audio; resource completion never consumes another UI activation.
use super::menu::{Action, Kind, Menu};
use super::*;
pub(super) enum Operation {
    ReturnTitle(super::menu_return::TitleReturn),
    SceneSetup {
        task: Option<Task<Arc<Program>>>,
        ready: Option<Arc<Program>>,
        name: Vec<u8>,
        films: Vec<i32>,
    },
    Activate(Action),
    Close,
    CloseMessage(crate::native_message_control::SystemMessageAction),
    Save { previous: Option<native_save::Controller>, close: bool },
    Load { previous: Option<native_load::Policy>, close: bool },
    Music([i32; 4]),
    GalleryAudio(crate::native_cg::CgAudio),
    ListenMusic { music: Option<crate::native_audio::ListenMusic>, number: i32 },
    CloseConfig {
        civil: Option<Box<dyn CivilTimeQuery>>,
        write: Option<SystemWrite>,
    },
}
impl Operation {
    pub(super) fn dependency_pending(&self)->bool {match self {Self::ReturnTitle(work)=>work.dependency_pending(),_=>true}}
}
impl InstalledAdapter {
    pub(super) fn poll_menu_shutdown(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.vm.as_ref().is_some_and(|vm| vm.scene_active())
            && !self.vm.as_mut().unwrap().poll_scene_replay(
                host,
                InputFrame::default(),
                false,
                true,
            )?
        {
            return Ok(());
        }
        if self.menu_operation.is_some() {
            self.poll_menu(host, InputFrame::default(), false)?;
        }
        if self.menu.is_some() && self.menu_operation.is_none() {
            if !self.finish_menu(host)? {
                self.menu_operation = Some(Operation::Close);
            }
        }
        Ok(())
    }
    pub(super) fn open_menu(&mut self, kind: Kind, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            self.menu.is_none() && self.menu_operation.is_none(),
            "NNN modal mode already active"
        );
        let vm = self.vm.as_ref().context("NNN modal mode VM absent")?;
        if kind == Kind::Save {
            ensure!(
                vm.original_input_mode().is_some(),
                "NNN save menu requires a stable native wait"
            );
        }
        let config = vm
            .configuration()
            .cloned()
            .context("NNN modal mode configuration absent")?;
        let settings = if kind == Kind::Config {
            vm.menu_settings()?
        } else {
            vec![]
        };
        self.menu = Some(Menu::new(kind, config, settings, self.client, host)?);
        self.menu.as_mut().unwrap().return_mode=vm.original_input_mode().unwrap_or(4);
        if kind.gallery() {
            if let Some(title) = self.title.as_mut() {
                title.hide_for_menu(host)?;
            }
            self.menu.as_mut().unwrap().bind_progress(
                vm.gallery_progress()?,
                vm.gallery_variables()?,
                self.title_resources
                    .clone()
                    .context("NNN gallery resources absent")?,
            );
        }
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(
                &serde_json::json!({"layers":[{"op":"set_clock_paused","id":1,"paused":true}]}),
            )?,
        )?;
        self.vm.as_mut().unwrap().set_modal_mode(kind.native_mode())?;
        Ok(())
    }
    fn finish_menu(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self
            .vm
            .as_mut()
            .context("NNN menu return VM absent")?
            .end_menu_background(host)?
        {
            return Ok(false);
        }
        if let Some(mut menu) = self.menu.take() {
            menu.remove(self.client, host)?;
        }
        self.vm
            .as_mut()
            .context("NNN menu return VM absent")?
            .resume_after_menu(host)?;
        if let Some(mode)=self.vm.as_ref().unwrap().original_input_mode(){self.vm.as_mut().unwrap().set_modal_mode(mode)?;}
        if let Some(chart)=self.chart.as_mut() {chart.resume_clock(host.now_ms());}
        if let Some(title) = self.title.as_mut() {
            title.resume_after_menu(host)?;
        }
        Ok(true)
    }
    pub(super) fn poll_menu(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
    ) -> Result<bool> {
        if self.menu.is_none() {
            return Ok(false);
        }
        if !self.vm.as_ref().unwrap().menu_background_active() {self.vm.as_mut().unwrap().set_modal_mode(self.menu.as_ref().unwrap().kind.native_mode())?;}
        if let Some(operation) = self.menu_operation.take() {
            // The existing modal remains eligible, but clicks made during its
            // native I/O action do not queue another save/load/config action.
            let _ = host.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::UiSubtreeEvents(menu::ROOT),
            )?;
            match operation {
                Operation::ReturnTitle(mut work)=> {
                    if self.closing {work.cancel(host)?;self.menu_operation=Some(Operation::Close);return Ok(true)}
                    if !work.poll(host,admit,self.vm.as_mut().unwrap())? || !self.vm.as_mut().unwrap().poll_shutdown_ready(host)? {self.menu_operation=Some(Operation::ReturnTitle(work));return Ok(true)}
                    let (profile,frame)=work.finish(host)?;
                    if let Some(mut menu)=self.menu.take(){menu.remove(self.client,host)?;}
                    if let Some(mut chart)=self.chart.take(){chart.remove(host)?;}
                    let vm=self.vm.take().unwrap();let frame_ms=vm.frame_period_ms();
                    self.outer=Some(vm.into_cancelled_outer_state()?);
                    self.shell=Some(crate::native_shell::NativeStartupShell::title_return(self.client,1,SIZE,frame_ms,0,profile,self.title_resources.clone().context("NNN title return resources absent")?)?);
                    self.native_exit_available=true;
                    self.begin_title(host,Some(frame))?;
                }
                Operation::SceneSetup {
                    mut task,
                    mut ready,
                    name,
                    films,
                } => {
                    if let Some(job) = task.as_mut() {
                        if let Some(program) = job.try_take()? {
                            ready = Some(program);
                            task = None;
                        }
                    }
                    if let Some(program) = ready.as_ref() {
                        if self.vm.as_mut().unwrap().start_scene_replay(
                            name.clone(),
                            program.clone(),
                            films.clone(),
                            host,
                        )? {
                            self.menu.as_mut().unwrap().back_script_started = false;
                            self.menu
                                .as_ref()
                                .unwrap()
                                .hide_controls(self.client, host)?;
                            return Ok(true);
                        }
                    }
                    self.menu_operation = Some(Operation::SceneSetup {
                        task,
                        ready,
                        name,
                        films,
                    });
                }
                Operation::Activate(action) => {
                    if !self
                        .vm
                        .as_mut()
                        .unwrap()
                        .switch_menu_background(false, host)?
                    {
                        self.menu_operation = Some(Operation::Activate(action));
                    } else {
                        self.activate_menu_action(action, host)?;
                    }
                }
                Operation::Close => {
                    if !self.finish_menu(host)? {
                        self.menu_operation = Some(Operation::Close);
                    }
                }
                Operation::CloseMessage(action)=>{
                    if !self.finish_menu(host)? {
                        self.menu_operation=Some(Operation::CloseMessage(action));
                    }else{
                        self.vm.as_mut().context("NNN SystemMenu foreground absent")?
                            .apply_system_message_action(action,host)?;
                    }
                }
                Operation::Save { previous, close } => {
                    self.poll_slot_save(host, InputFrame::default())?;
                    if self.native_save.as_ref().is_some_and(|save| save.pending()) {
                        self.menu_operation = Some(Operation::Save { previous, close });
                    } else {
                        let completed = self.native_save.take();
                        self.native_save = previous;
                        if let (Some(previous), Some(completed)) =
                            (&mut self.native_save, completed)
                        {
                            previous.last_file = completed.last_file;
                        }
                        if close { self.menu_operation = Some(Operation::Close); }
                        else { self.menu.as_mut().unwrap().refresh(); }
                    }
                }
                Operation::Load { previous, close } => {
                    self.poll_slot_load(host, InputFrame::default())?;
                    if self.native_load.as_ref().is_some_and(|load| load.pending()) {
                        self.menu_operation = Some(Operation::Load { previous, close });
                    } else {
                        self.native_load = previous.map(native_load::Controller::new);
                        if close { self.menu_operation = Some(Operation::Close); }
                    }
                }
                Operation::GalleryAudio(event) => {
                    if !self.vm.as_mut().unwrap().poll_gallery_audio(host,&event)? {
                        self.menu_operation = Some(Operation::GalleryAudio(event));
                    } else {
                        self.menu.as_mut().unwrap().gallery_audio_complete(host.now_ms());
                    }
                }
                Operation::Music(words) => {
                    if !self
                        .vm
                        .as_mut()
                        .unwrap()
                        .poll_outer_music(host, words, true)?
                    {
                        self.menu_operation = Some(Operation::Music(words));
                    }
                }
                Operation::ListenMusic { music, number } => {
                    if !self.vm.as_mut().unwrap().poll_listen_music(host, music.clone())? {
                        self.menu_operation = Some(Operation::ListenMusic { music, number });
                    } else {
                        self.vm.as_mut().unwrap().set_listen_state(i32::from(music.is_some()), number)?;
                        let menu = self.menu.as_mut().unwrap();
                        menu.listen_number = number;
                        menu.listen_playing = music.is_some();
                    }
                }
                Operation::CloseConfig {
                    mut civil,
                    mut write,
                } => {
                    if write.is_none() {
                        if civil.is_none() {
                            civil = Some(host.begin_civil_time(self.client)?);
                        }
                        match civil.as_mut().unwrap().poll()? {
                            CivilTimeState::Pending => (),
                            CivilTimeState::Failed(error) => {
                                anyhow::bail!("NNN settings save date: {error}")
                            }
                            CivilTimeState::Ready(time) => {
                                write = Some(self.vm.as_mut().unwrap().begin_menu_settings_save(
                                    host,
                                    crate::native_system_file::SystemStamp::from_civil(
                                        time,
                                        self.client,
                                    )?,
                                )?);
                                civil = None;
                            }
                        }
                    }
                    if let Some(job) = write.as_mut() {
                        if self.vm.as_mut().unwrap().poll_menu_settings_save(job)? {
                            if self.menu.as_ref().unwrap().back_kind().is_some() {
                                self.menu_operation = Some(Operation::Activate(Action::Back));
                                return Ok(true);
                            }
                            ensure!(
                                self.finish_menu(host)?,
                                "NNN settings close resumed an inactive background unexpectedly"
                            );
                            return Ok(true);
                        }
                    }
                    self.menu_operation = Some(Operation::CloseConfig { civil, write });
                }
            }
            return Ok(true);
        }
        let vm = self.vm.as_mut().context("NNN modal mode VM absent")?;
        if vm.scene_active() {
            let cancel =
                admit && input.buttons_pressed & crysirror_core::service::buttons::ESCAPE != 0;
            if vm.poll_scene_replay(host, input, admit, cancel)? {
                self.menu.as_mut().unwrap().refresh();
                self.menu.as_mut().unwrap().back_script_started = false;
            }
            return Ok(true);
        }
        if vm.dependency_pending() {
            vm.poll_dependency(host)?;
            if vm.dependency_pending() {
                return Ok(true);
            }
        }
        if let Some(title) = self.title.as_mut() {
            if title.dependency_pending() {
                title.poll_existing_dependencies(host, Some(vm))?;
                if title.dependency_pending() {
                    return Ok(true);
                }
            }
        }
        let pool = self
            .pool
            .as_ref()
            .context("NNN modal mode worker pool absent")?;
        if self.title.is_some() && !self.menu.as_ref().unwrap().owner_paused {
            if !vm.pause_title_for_menu(host)? {
                return Ok(true);
            }
            self.menu.as_mut().unwrap().owner_paused = true;
        }
        if self.menu.as_ref().unwrap().viewer_active() {
            if vm.dependency_pending(){vm.poll_dependency(host)?;if vm.dependency_pending(){return Ok(true);}}
            let action=if self.menu.as_ref().unwrap().picture_viewer_active() {
                self.menu.as_mut().unwrap().poll_native_viewer(host,input,admit,vm)?
            }else{self.menu.as_mut().unwrap().poll(self.client,&self.config.font,pool,host,input,admit)?};
            if let Some(action) = action { self.menu_operation = Some(Operation::Activate(action)); }
            return Ok(true);
        }
        if !self
            .menu
            .as_mut()
            .unwrap()
            .prepare_page(self.client, pool, host)?
        {
            return Ok(true);
        }
        if self.menu.as_ref().unwrap().kind == Kind::Music {
            let menu = self.menu.as_mut().unwrap();
            if !menu.listen_initialized {
                menu.listen_initialized = true;
                self.menu_operation = Some(Operation::ListenMusic { music: None, number: 1 });
                return Ok(true);
            }
            if menu.listen_playing && !vm.listen_music_playing(host)? {
                self.menu_operation = Some(Operation::Activate(Action::PlayMusic { music: None, number: None }));
                return Ok(true);
            }
        }
        if self.menu.as_ref().unwrap().kind==Kind::System {
            let mut variables=vm.gallery_variables()?;
            if self.menu.as_mut().unwrap().initialize_system_menu(&mut variables)? {vm.publish_general_variables(variables)?;}
        }
        if !self.menu.as_ref().unwrap().back_script_started {
            if let Some(script) = self.menu.as_ref().unwrap().back_script.clone() {
                let mode = self.menu.as_ref().unwrap().kind.native_mode();
                if !vm.start_menu_background(
                    script.name,
                    script.program,
                    script.looping,
                    mode,
                    host,
                )? {
                    return Ok(true);
                }
                if let Some(title) = self.title.as_mut() {
                    title.hide_for_menu(host)?;
                }
            }
            self.menu.as_mut().unwrap().back_script_started = true;
        }
        if vm.has_menu_background() {
            if !vm.switch_menu_background(true, host)? {
                return Ok(true);
            }
            if admit && !self.menu.as_ref().unwrap().entry_blocks_update(host.now_ms()) {
                vm.tick_menu_background(host, input)?;
            }
        }
        let action = if self.menu.as_ref().unwrap().has_skin() {
            self.menu.as_mut().unwrap().poll_skin(
                self.client,
                &self.config.font,
                pool,
                host,
                input,
                admit,
                vm,
            )?
        } else {
            self.menu.as_mut().unwrap().poll(
                self.client,
                &self.config.font,
                pool,
                host,
                input,
                admit,
            )?
        };
        if let Some(action) = action {
            self.menu_operation = Some(Operation::Activate(action));
        }
        Ok(true)
    }
    fn activate_menu_action(&mut self, action: Action, host: &mut dyn HostServices) -> Result<()> {
        if self.menu.as_ref().is_some_and(|menu| menu.kind == Kind::Music)
            && matches!(&action, Action::Back | Action::Close | Action::Navigate(_) | Action::StartGame { .. })
        {
            if !self.vm.as_mut().unwrap().poll_listen_music(host, None)? {
                self.menu_operation = Some(Operation::Activate(action));
                return Ok(());
            }
            let number = self.menu.as_ref().unwrap().listen_number;
            self.vm.as_mut().unwrap().set_listen_state(0, number)?;
        }
        match action {
            Action::System(index)=> {
                let next=match index {
                    0=>Action::Navigate(Kind::Save),1=>Action::Navigate(Kind::Load),2=>Action::Navigate(Kind::Config),
                    3=>Action::ResumeMessage(crate::native_message_control::SystemMessageAction::HideWindow),
                    4=>Action::ResumeMessage(crate::native_message_control::SystemMessageAction::Skip),
                    6=>Action::ResumeMessage(crate::native_message_control::SystemMessageAction::Automatic),
                    7=>Action::Close,
                    8=>{if self.menu.as_mut().unwrap().confirm_title()? {return Ok(())}else{Action::ReturnTitle}},
                    9.. => {
                        let table = self.menu.as_ref().unwrap().native_table()?;
                        let mode = table.integer(format!("expOptionMode{}", index + 1).as_bytes())?.unwrap_or(0);
                        if mode > 0 { Action::Navigate(Kind::from_native(mode)?) }
                        else {
                            let command = table.integer(format!("expOptionCommand{}", index + 1).as_bytes())?.unwrap_or(0);
                            if command <= 0 { return Ok(()); } // source has no branch
                            Action::SystemCommand(command)
                        }
                    }
                    _=>anyhow::bail!("NNN SystemMenu option {index} requires its native action consumer"),
                };
                self.activate_menu_action(next,host)?;
            }
            Action::ResumeMessage(action)=>{
                ensure!(self.menu.as_ref().is_some_and(|menu|matches!(menu.return_mode,9|10)),
                    "NNN SystemMenu message option outside PRINT/SELECT mode");
                self.menu_operation=Some(Operation::CloseMessage(action));
            }
            Action::SystemCommand(command) => {
                match command {
                    6 => {
                        let Some(policy) = self.native_save.as_ref().map(|save|save.policy.clone()) else {
                            self.menu_operation = Some(Operation::Close); return Ok(());
                        };
                        let image = self.menu.as_ref().unwrap().background.clone().context("NNN system quick-save entry picture absent")?;
                        let thumbnail = self.vm.as_ref().unwrap().menu_thumbnail(host,image)?;
                        let previous = self.native_save.take();
                        self.native_save = Some(native_save::Controller::selected(policy,thumbnail,previous.as_ref()));
                        self.menu_operation = Some(Operation::Save { previous, close: true });
                    }
                    5 => {
                        let Some(policy) = self.native_load.as_ref().map(|load|load.policy.clone()) else {
                            self.menu_operation = Some(Operation::Close); return Ok(());
                        };
                        let load = native_load::Controller::admit(policy,host.now_ms(),self.client,host)?;
                        let previous = self.native_load.take().map(|load|load.policy);
                        self.native_load = Some(load);
                        self.menu_operation = Some(Operation::Load { previous, close: true });
                    }
                    _ => {
                        // The public base UserSystemMenu returns -1, but this
                        // legacy game can override that virtual. An unreviewed
                        // positive command must not be treated as the base stub.
                        anyhow::bail!("NNN SystemMenu custom command {command} requires its native override");
                    }
                }
            }
            Action::ReturnTitle=> {
                let table=self.menu.as_ref().unwrap().native_table()?;
                self.menu_operation=Some(Operation::ReturnTitle(super::menu_return::TitleReturn::begin(self.vm.as_ref().unwrap(),table,self.pool.as_ref().context("NNN title return worker absent")?,host,self.client)?));
            }
            Action::GalleryAudio(event) => {
                self.menu_operation = Some(Operation::GalleryAudio(event));
            }
            Action::Back => {
                if !self.vm.as_mut().unwrap().end_menu_background(host)? {
                    self.menu_operation = Some(Operation::Activate(Action::Back));
                } else {
                    let progress = self.vm.as_ref().unwrap().gallery_progress()?;
                    if self.menu.as_ref().unwrap().back_kind() == Some(Kind::Config) {
                        self.menu
                            .as_mut()
                            .unwrap()
                            .set_settings(self.vm.as_ref().unwrap().menu_settings()?);
                    }
                    self.menu
                        .as_mut()
                        .unwrap()
                        .update_variables(self.vm.as_ref().unwrap().gallery_variables()?);
                    self.menu
                        .as_mut()
                        .unwrap()
                        .back(progress, self.client, host)?;
                }
            }
            Action::StartGame { variable, value } => {
                ensure!(
                    self.vm.as_mut().unwrap().end_menu_background(host)?,
                    "NNN menu new-game reader boundary"
                );
                let (image, returning) = self.menu.as_ref().unwrap().newgame_frame()?;
                let period = self.vm.as_ref().unwrap().frame_period_ms();
                self.title_return = returning
                    .fade
                    .map(|spec| {
                        crate::native_general::ExitFadePlayer::new_with_enter(
                            host,
                            self.client,
                            image.clone(),
                            self.vm.as_ref().unwrap().retained_general_enter_frame(),
                            spec,
                            period,
                        )
                    })
                    .transpose()?;
                self.title_exit = Some(image);
                self.native_exit_available = true;
                let loader = self.vm.as_ref().unwrap().scoped_loader()?;
                self.title_program_task = Some(
                    self.pool
                        .as_ref()
                        .context("NNN menu script loader worker absent")?
                        .spawn(move |cancel| loader(b"sys", &cancel))?,
                );
                self.menu.as_mut().unwrap().remove(self.client, host)?;
                self.menu = None;
                self.title_script_return = Some((variable, value));
                self.title_newgame_requested = true;
            }
            Action::Navigate(kind) => {
                if !self.vm.as_mut().unwrap().end_menu_background(host)? {
                    self.menu_operation = Some(Operation::Activate(Action::Navigate(kind)));
                } else {
                    let progress = self.vm.as_ref().unwrap().gallery_progress()?;
                    if kind == Kind::Config {
                        self.menu
                            .as_mut()
                            .unwrap()
                            .set_settings(self.vm.as_ref().unwrap().menu_settings()?);
                    }
                    self.menu
                        .as_mut()
                        .unwrap()
                        .update_variables(self.vm.as_ref().unwrap().gallery_variables()?);
                    self.menu
                        .as_mut()
                        .unwrap()
                        .navigate(kind, progress, self.client, host)?;
                }
            }
            Action::PlayMusic { music, number } => {
                let number = number.unwrap_or(self.menu.as_ref().unwrap().listen_number);
                self.menu_operation = Some(Operation::ListenMusic { music, number });
            }
            Action::Picture { pictures, start, group } => {
                ensure!(
                    self.vm.as_mut().unwrap().end_menu_background(host)?,
                    "NNN gallery viewer did not drain its modal reader"
                );
                self.menu.as_mut().unwrap().back_script_started = false;
                let resources = self
                    .title_resources
                    .clone()
                    .context("NNN gallery resource capability absent")?;
                let pool = self
                    .pool
                    .clone()
                    .context("NNN gallery worker pool absent")?;
                self.menu.as_mut().unwrap().picture(
                    pictures, start, group,
                    resources,
                    pool,
                    self.client,
                    host,
                )?;
            }
            Action::PlayMovie(asset) => {
                ensure!(
                    self.vm.as_mut().unwrap().end_menu_background(host)?,
                    "NNN gallery movie did not drain its modal reader"
                );
                let frame_ms = self.vm.as_ref().unwrap().frame_period_ms();
                let prefs = self
                    .movie_preferences
                    .as_ref()
                    .context("NNN movie gallery native settings absent")?;
                let gain = crate::native_audio::legacy_gain(if prefs.movie_enabled {
                    prefs.movie_volume
                } else {
                    0
                })?;
                self.menu.as_mut().unwrap().back_script_started = false;
                self.menu
                    .as_mut()
                    .unwrap()
                    .movie(asset, self.client, host, gain, frame_ms)?;
            }
            Action::Scene { script, films, .. } => {
                ensure!(
                    self.vm.as_mut().unwrap().end_menu_background(host)?,
                    "NNN scene setup did not drain modal reader"
                );
                let loader = self.vm.as_ref().unwrap().scoped_loader()?;
                let name = script.clone();
                let task = self
                    .pool
                    .as_ref()
                    .context("NNN scene resource worker absent")?
                    .spawn(move |cancel| loader(&script, &cancel))?;
                self.menu_operation = Some(Operation::SceneSetup {
                    task: Some(task),
                    ready: None,
                    name,
                    films,
                });
            }
            Action::Close => {
                if self.menu.as_ref().unwrap().kind != Kind::Config
                    && self.menu.as_ref().unwrap().back_kind().is_some()
                {
                    self.menu_operation = Some(Operation::Activate(Action::Back));
                    return Ok(());
                }
                if self.menu.as_ref().unwrap().kind == Kind::Config {
                    self.menu_operation = Some(Operation::CloseConfig {
                        civil: None,
                        write: None,
                    });
                } else {
                    self.menu_operation = Some(Operation::Close);
                }
            }
            Action::Load(slot) => {
                let menu = self.menu.as_ref().unwrap();
                let policy =
                    native_load::Policy::selected(slot, menu.load_sound, menu.error_sound)?;
                let previous = self.native_load.take().map(|load| load.policy);
                self.native_load = Some(native_load::Controller::selected(policy, host.now_ms()));
                self.menu_operation = Some(Operation::Load { previous, close: false });
            }
            Action::Save(slot) => {
                let menu = self.menu.as_ref().unwrap();
                let image = menu
                    .background
                    .as_ref()
                    .context("NNN save entry background absent")?
                    .clone();
                let thumbnail = self.vm.as_ref().unwrap().menu_thumbnail(host, image)?;
                let policy = native_save::Policy::selected(slot, menu.load_sound)?;
                let previous = self.native_save.take();
                self.native_save = Some(native_save::Controller::selected(
                    policy,
                    thumbnail,
                    previous.as_ref(),
                ));
                self.menu_operation = Some(Operation::Save { previous, close: false });
            }
            Action::Setting(id, value) => {
                let restart = self
                    .vm
                    .as_mut()
                    .unwrap()
                    .apply_menu_setting(id, value, host)?;
                self.menu.as_mut().unwrap().update_setting(id, value);
                if let Some(preferences) = self.movie_preferences.as_mut() {
                    match id {
                        103 => preferences.movie_volume = value,
                        203 => preferences.movie_enabled = value != 0,
                        _ => (),
                    }
                }
                if let Some(words) = restart {
                    self.menu_operation = Some(Operation::Music(words));
                }
            }
        }
        Ok(())
    }
    pub(super) fn restored_quick_load_policy(&self) -> Option<native_load::Policy> {
        match &self.menu_operation {
            Some(Operation::Load { previous, .. }) => previous
                .clone()
                .or_else(|| self.native_load.as_ref().map(|l| l.policy.clone())),
            _ => self.native_load.as_ref().map(|l| l.policy.clone()),
        }
    }
}
