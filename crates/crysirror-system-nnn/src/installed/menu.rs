// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original load/save catalogue and settings actions through shared modal UI.
//! Native dataN slot identities/layout and SYSTEM DATA settings stay native;
//! widget hit testing, focus, text, images and input ownership stay in the engine.
use super::*;
use crate::native_save::{LegacySaveProfile, NativeSaveFile};
use crysirror_core::{
    image_processing::ImageResource,
    service::{buttons, EngineQuery},
};
use serde_json::{json, Value};
pub(super) const ROOT: u32 = 0x66040000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Load,
    Save,
    Config,
    Omake,
    CgCharacters,
    Cg(usize),
    SceneCharacters,
    Scenes(usize),
    Music,
    Movies,
    System,
}
impl Kind {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Save => "save",
            Self::Config => "config",
            Self::Omake => "omake",
            Self::CgCharacters => "selectCGChara",
            Self::Cg(_) => "selectCG",
            Self::SceneCharacters => "selectSceneChara",
            Self::Scenes(_) => "selectScene",
            Self::Music => "listenBGM",
            Self::Movies => "selectMovie",
            Self::System => "SystemMenu",
        }
    }
    pub(super) fn native_mode(self) -> i32 {
        match self {
            Self::Load => 5,
            Self::Save => 6,
            Self::Config => 12,
            Self::Omake => 15,
            Self::CgCharacters => 16,
            Self::Cg(_) => 17,
            Self::SceneCharacters => 20,
            Self::Scenes(_) => 18,
            Self::Music => 8,
            Self::Movies => 26,
            Self::System => 13,
        }
    }
    pub(super) fn gallery(self) -> bool {
        !matches!(self, Self::Load | Self::Save | Self::Config)
    }
    pub(super) fn from_native(mode: i32) -> Result<Self> {
        Ok(match mode {
            5 => Self::Load,
            6 => Self::Save,
            8 => Self::Music,
            12 => Self::Config,
            15 => Self::Omake,
            16 => Self::CgCharacters,
            17 => Self::Cg(0),
            18 => Self::Scenes(0),
            20 => Self::SceneCharacters,
            26 => Self::Movies,
            13 => Self::System,
            _ => anyhow::bail!("NNN native menu special mode {mode} has no mode actor"),
        })
    }
}
#[derive(Clone)]
pub(super) struct Background {
    pub name: Vec<u8>,
    pub program: Arc<Program>,
    pub looping: bool,
}
#[derive(Clone)]
pub(super) enum Action {
    Close,
    ResumeMessage(crate::native_message_control::SystemMessageAction),
    Back,
    StartGame {
        variable: Option<u16>,
        value: i32,
    },
    Load(i32),
    Save(u32),
    Setting(i32, i32),
    Navigate(Kind),
    Picture {
        group: usize,
        pictures: Arc<[crate::native_cg::CgChoice]>,
        start: usize,
    },
    Scene {
        group: usize,
        index: usize,
        script: Vec<u8>,
        films: Vec<i32>,
    },
    PlayMusic { music: Option<crate::native_audio::ListenMusic>, number: Option<i32> },
    PlayMovie(String),
    GalleryAudio(crate::native_cg::CgAudio),
    System(usize),
    SystemCommand(i32),
    ReturnTitle,
}
pub(super) struct Slot {
    pub(super) number: usize,
    pub(super) text: String,
    pub(super) valid: bool,
    pub(super) occupied: bool,
    pub(super) thumbnail: Option<crysirror_image::ImageSurface>,
    pub(super) action: Option<Action>,
}
pub(super) struct Page {
    pub(super) background: Option<Background>,
    pub(super) slots: Vec<Slot>,
    pub(super) pages: usize,
    pub(super) columns: usize,
    pub(super) origin: [i32; 2],
    pub(super) step: [i32; 2],
    pub(super) sound: i32,
    pub(super) error_sound: i32,
    pub(super) skin: Option<crate::native_title::NativeTitleSpec>,
    pub(super) image: Option<ImageInput>,
    pub(super) table: Option<crate::config::Names>,
}
pub(super) struct Menu {
    pub return_mode:i32,
    system_initialized:bool,
    pub listen_number: i32,
    pub listen_initialized: bool,
    pub listen_playing: bool,
    pub owner_paused: bool,
    pub kind: Kind,
    history: Vec<Kind>,
    progress: Option<crate::native_system_file::NativeSystemFile>,
    resources: Option<NativeResources>,
    variables: Option<crate::expression::Variables>,
    skin: Option<crate::native_title::NativeTitlePlayer>,
    skin_base: Option<Arc<dyn ImageResource>>,
    skin_job: Option<ImageJob>,
    viewer: Option<super::gallery_viewer::PictureViewer>,
    movie: Option<crate::native_movie::NativeMoviePlayer>,
    config: Arc<crate::native_bootstrap::NativeConfiguration>,
    page: usize,
    task: Option<Task<Page>>,
    ready: Option<Page>,
    published: bool,
    paths: Vec<String>,
    nodes: Vec<u32>,
    settings: Vec<(i32, i32)>,
    pub load_sound: i32,
    pub error_sound: i32,
    confirm: Option<Action>,
    pub capture: Option<ImageJob>,
    pub background: Option<Arc<dyn ImageResource>>,
    pub back_script: Option<Background>,
    pub back_script_started: bool,
}
impl Menu {
    pub(super) fn initialize_system_menu(&mut self,variables:&mut crate::expression::Variables)->Result<bool> {
        if self.kind!=Kind::System || self.system_initialized {return Ok(false)}
        let mut random=variables.random_state().context("NNN SystemMenu original random state absent")?;
        let table=self.native_table()?.clone();
        self.skin.as_mut().context("NNN SystemMenu buttons absent")?.configure_system_intro(&table,&mut random)?;
        variables.replace_random_state(random)?;self.system_initialized=true;Ok(true)
    }
    pub(super) fn native_table(&self)->Result<&crate::config::Names> {self.ready.as_ref().and_then(|p|p.table.as_ref()).context("NNN mode setup not prepared")}
    pub(super) fn confirm_title(&mut self)->Result<bool> {
        let enabled=self.ready.as_ref().and_then(|p|p.table.as_ref()).map(|t|t.integer(b"warningPrintFlag")).transpose()?.flatten().unwrap_or(1)!=0;
        if enabled {self.confirm=Some(Action::ReturnTitle);self.published=false;}
        Ok(enabled)
    }
    pub(super) fn hide_controls(
        &self,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        let layers = self
            .nodes
            .iter()
            .map(|id| json!({"op":"set_visible","id":id,"visible":false}))
            .collect::<Vec<_>>();
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(
                &json!({"layers":layers,"ui":[{"op":"set_visible","id":ROOT,"visible":false}]}),
            )?,
        )?;
        Ok(())
    }
    pub fn new(
        kind: Kind,
        config: Arc<crate::native_bootstrap::NativeConfiguration>,
        settings: Vec<(i32, i32)>,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        let capture = if matches!(kind,Kind::Save|Kind::System) {
            Some(host.begin_layer_prefix_image_over(
                client,
                SIZE,
                [i32::MAX, i32::MAX],
                [0, 0, 0, 255],
            )?)
        } else {
            None
        };
        Ok(Self {
            return_mode:10,
            system_initialized:false,
            listen_number: 1,
            listen_initialized: false,
            listen_playing: false,
            owner_paused: false,
            kind,
            history: vec![],
            progress: None,
            resources: None,
            variables: None,
            skin: None,
            skin_base: None,
            skin_job: None,
            viewer: None,
            movie: None,
            config,
            page: 0,
            task: None,
            ready: None,
            published: false,
            paths: vec![],
            nodes: vec![],
            settings,
            load_sound: 1,
            error_sound: -1,
            confirm: None,
            capture,
            background: None,
            back_script: None,
            back_script_started: false,
        })
    }
    pub(super) fn bind_progress(
        &mut self,
        progress: crate::native_system_file::NativeSystemFile,
        variables: crate::expression::Variables,
        resources: NativeResources,
    ) {
        self.progress = Some(progress);
        self.variables = Some(variables);
        self.resources = Some(resources);
    }
    pub(super) fn set_settings(&mut self, settings: Vec<(i32, i32)>) {
        self.settings = settings;
    }
    pub(super) fn newgame_frame(
        &self,
    ) -> Result<(Arc<dyn ImageResource>, crate::native_general::ModeReturn)> {
        let frame = self
            .skin
            .as_ref()
            .context("NNN native menu script action lacks its completed picture")?
            .completed_frame()?;
        let table = self
            .ready
            .as_ref()
            .and_then(|page| page.table.as_ref())
            .context("NNN native menu return setup absent")?;
        Ok((frame, crate::native_general::ModeReturn::read(table, None)?))
    }
    pub(super) fn update_variables(&mut self, variables: crate::expression::Variables) {
        self.variables = Some(variables);
    }
    pub(super) fn navigate(
        &mut self,
        kind: Kind,
        progress: crate::native_system_file::NativeSystemFile,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(
            self.history.len() < 64,
            "NNN modal mode return depth budget"
        );
        if self.kind!=Kind::System {self.history.push(self.kind);}
        self.reset_page(kind, progress, client, host)
    }
    pub(super) fn back_kind(&self) -> Option<Kind> {
        self.history.last().copied()
    }
    pub(super) fn back(
        &mut self,
        progress: crate::native_system_file::NativeSystemFile,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        let kind = self
            .history
            .pop()
            .context("NNN native menu return stack empty")?;
        self.reset_page(kind, progress, client, host)
    }
    fn reset_page(
        &mut self,
        kind: Kind,
        progress: crate::native_system_file::NativeSystemFile,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        self.remove(client, host)?;
        self.kind = kind;
        self.listen_number = 1;
        self.listen_initialized = false;
        self.listen_playing = false;
        self.page = 0;
        self.ready = None;
        self.task = None;
        self.back_script = None;
        self.back_script_started = false;
        self.progress = Some(progress);
        self.confirm = None;
        self.system_initialized=false;
        Ok(())
    }
    pub(super) fn picture(
        &mut self,
        pictures: Arc<[crate::native_cg::CgChoice]>,
        start: usize,
        group: usize,
        resources: NativeResources,
        pool: TaskPool,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        let viewer = super::gallery_viewer::PictureViewer::new(
            pictures, start, group,
            self.config.clone(),
            resources,
            host.storage(client)?,
            pool,
            client,
        )?;
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"ui":[{"op":"set_visible","id":ROOT,"visible":false}]}))?,
        )?;
        self.viewer = Some(viewer);
        Ok(())
    }
    pub(super) fn poll_native_viewer(&mut self,host:&mut dyn HostServices,input:InputFrame,admit:bool,vm:&mut NativeAdapter)->Result<Option<Action>> {
        let viewer=self.viewer.as_mut().context("NNN native picture viewer absent")?;
        let finished=viewer.poll_native(host,input,admit,vm)?;
        if let Some(audio)=viewer.take_audio(){return Ok(Some(Action::GalleryAudio(audio)));}
        if finished{viewer.remove(host)?;self.viewer=None;self.published=false;}
        Ok(None)
    }
    pub(super) fn picture_viewer_active(&self)->bool{self.viewer.is_some()}
    pub(super) fn viewer_active(&self) -> bool {
        self.viewer.is_some() || self.movie.is_some()
    }
    pub(super) fn gallery_audio_complete(&mut self, now: u64) {
        if let Some(viewer) = self.viewer.as_mut() { viewer.resume_clock(now); }
    }
    pub(super) fn movie(
        &mut self,
        asset: String,
        client: ClientId,
        host: &mut dyn HostServices,
        gain: f32,
        frame_ms: u64,
    ) -> Result<()> {
        let spec = crate::native_movie::NativeMovieSpec {
            asset: Some(asset),
            next: crate::native_shell::NativeMode::Title,
            no_skip: false,
            last_wait: 5,
            exit_fill: None,
            gain,
        };
        let movie = crate::native_movie::NativeMoviePlayer::new(
            client, 0x66049000, 0x66049001, SIZE, frame_ms, spec,
        )?;
        let layers = self
            .nodes
            .iter()
            .map(|id| json!({"op":"set_visible","id":id,"visible":false}))
            .collect::<Vec<_>>();
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(
                &json!({"layers":layers,"ui":[{"op":"set_visible","id":ROOT,"visible":false}]}),
            )?,
        )?;
        self.movie = Some(movie);
        Ok(())
    }
    pub fn busy(&self) -> bool {
        self.skin_job.is_some()
            || self.skin.as_ref().is_some_and(|s| s.dependency_pending())
            || self.movie.as_ref().is_some_and(|m| m.dependency_pending())
            || self.viewer.as_ref().is_some_and(|v| v.pending())
            || self.task.is_some()
            || self.capture.is_some()
            || !self.published
    }
    pub fn refresh(&mut self) {
        self.ready = None;
        self.published = false;
    }
    pub fn cleanup(&self) -> Result<(Vec<Value>, Vec<Value>, Vec<String>)> {
        ensure!(
            !self.viewer_active(),
            "NNN original restore during gallery viewer"
        );
        let (extra_layers, extra_ui, extra_paths) = if let Some(skin) = &self.skin {
            skin.restore_cleanup()?
        } else {
            (vec![], vec![], vec![])
        };
        Ok((
            self.nodes
                .iter()
                .map(|id| json!({"op":"remove","id":id}))
                .chain(self.viewer.iter().flat_map(|v| v.cleanup()))
                .chain(extra_layers)
                .collect(),
            std::iter::once(json!({"op":"remove","id":ROOT}))
                .chain(extra_ui)
                .collect(),
            self.paths
                .iter()
                .cloned()
                .chain(self.viewer.iter().flat_map(|v| v.paths()))
                .chain(extra_paths)
                .collect(),
        ))
    }
    pub fn remove(&mut self, client: ClientId, host: &mut dyn HostServices) -> Result<()> {
        if let Some(mut skin) = self.skin.take() {
            skin.retire(host)?;
        }
        self.skin_job = None;
        self.skin_base = None;
        if let Some(mut movie) = self.movie.take() {
            movie.cancel(host)?;
        }
        if let Some(mut viewer) = self.viewer.take() {
            viewer.remove(host)?;
        }
        let (layers, ui, paths) = self.cleanup()?;
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        for path in paths {
            host.release_image_resource(client, &path)?;
        }
        self.paths.clear();
        self.nodes.clear();
        self.published = false;
        Ok(())
    }
    pub(super) fn prepare_page(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if let Some(capture) = self.capture.as_mut() {
            match capture.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN save menu entry capture: {error}")
                }
                ImageProcessState::Ready(image) => {
                    if self.kind==Kind::System {self.skin_base=Some(image.clone());}
                    self.background = Some(image);
                    self.capture = None;
                }
            }
        }
        if self.ready.is_some() {
            return self.prepare_skin();
        }
        if self.task.is_none() {
            let config = self.config.clone();
            let page = self.page;
            let kind = self.kind;
            let prepare_script = !self.back_script_started;
            let storage = host.storage(client)?;
            let progress = self.progress.clone();
            let resources = self.resources.clone();
            let variables = self.variables.clone();
            let return_mode=self.return_mode;
            let captured=self.background.clone();
            self.task = Some(pool.spawn(move |cancel| {
                let table = native_load::table(&config, storage.as_ref(), kind.name(), &cancel)?;
                let n = |name: &[u8], default| -> Result<i32> {
                    Ok(table.integer(name)?.unwrap_or(default))
                };
                // Title/Omake/SystemMenu own the exact shared entry lifecycle.
                // Other mode actors retain their explicit unsupported gate.
                if !matches!(kind,Kind::Omake|Kind::System) {
                    crate::native_general::ModeReturn::validate_entry(&table)?;
                }
                let background = if prepare_script && n(b"backScriptFlag", 0)? != 0 {
                    let name = table
                        .parameter(b"backScriptName")
                        .context("NNN menu background script name absent")?
                        .to_vec();
                    let path = config.resources.script(&name)?;
                    let bytes = read(storage.as_ref(), &path, 16 << 20, &cancel)?
                        .with_context(|| format!("NNN menu background script missing: {path}"))?;
                    Some(Background {
                        name,
                        program: Arc::new(Program::parse(&bytes, config.scripts)?),
                        looping: n(b"backScriptLoopFlag", 1)? != 0,
                    })
                } else {
                    None
                };
                if kind.gallery() {
                    let mut page = super::gallery::prepare(
                        kind,
                        page,
                        &table,
                        &config,
                        storage.as_ref(),
                        progress.as_ref().context("NNN gallery progress absent")?,
                        variables.as_ref().context("NNN gallery variables absent")?,
                        &cancel,
                    )?;
                    page.background = background;
                    if let Some(resources) = resources.as_ref() {
                        if page.background.is_none() {
                            let filename =
                                table.parameter(b"filenameBG").or(if kind == Kind::Omake {
                                    Some(b"title_bg2".as_slice())
                                } else {
                                    None
                                });
                            if let Some(filename) = filename {
                                let lookup = config.resources.picture(
                                    &[b"sys/".as_slice(), filename].concat(),
                                    u32::try_from(
                                        config.setup.integer(b"dwqPackLevel")?.unwrap_or(0),
                                    )?,
                                )?;
                                page.image = Some(resources.image(&lookup, &cancel)?);
                            }
                        }
                    }
                    if matches!(kind,Kind::Omake|Kind::System) {
                        if kind==Kind::System && page.background.is_none() {
                            page.image=Some(super::system_menu::backdrop(&table,&config,resources.as_ref().context("NNN SystemMenu resource capability absent")?,captured.context("NNN SystemMenu captured frame absent")?,&cancel)?);
                        }
                        ensure!(
                            kind==Kind::System || n(b"allOnButtonUseFlag", 0)? == 0,
                            "NNN Omake all-on debug dialog requires its native game-version policy"
                        );
                        let mut native = table.clone();
                        let mask_load=kind==Kind::System && n(b"NoSaveDataToMaskMenu",0)?!=0;
                        if kind==Kind::System {
                            native.tokens=native.tokens.chunks_exact(2).filter(|pair|![b"NoSaveDataToMaskMenu".as_slice(),b"basicButtonNumber",b"extModeNumber",b"extButtonNumber",b"miniGameFlag",b"htmlButtonEnable",b"lastLoadButtonEnable"].iter().any(|key|pair[0].eq_ignore_ascii_case(key))).flat_map(|pair|pair.iter().cloned()).collect();
                        }
                        if native.parameter(b"basicButtonNumber").is_none() {
                            native
                                .tokens
                                .extend([b"basicButtonNumber".to_vec(), if kind==Kind::System {n(b"optionNumber",9)?.to_string().into_bytes()} else {b"4".to_vec()}]);
                        }
                        let variant = crate::native_title::NativeTitleSpec::selected_variant(
                            &native,
                            &config,
                            variables.as_ref().context("NNN Omake variables absent")?,
                        )?;
                        let mut spec = crate::native_title::NativeTitleSpec::from_table_variant(
                            &native, &config, variant,
                        )?;
                        if mask_load && !super::system_menu::has_save(&config,storage.as_ref(),&cancel)? {
                            if let Some(button)=spec.buttons.get_mut(1) {button.enabled=false;}
                        }
                        if kind == Kind::System {
                            for (index, button) in spec.buttons.iter_mut().enumerate().skip(9) {
                                let mode = n(format!("expOptionMode{}",index+1).as_bytes(),0)?;
                                if mode > 0 && !super::system_menu::mode_enabled(&config,variables.as_ref().context("NNN system mode variable bank absent")?,mode)? {button.enabled=false;}
                            }
                        }
                        if kind == Kind::System {
                            if let Some(quick) = native_load::Policy::prepare(&config,storage.as_ref(),&cancel)? {
                                let mut quick_button=None;
                                for index in 9..spec.buttons.len() {
                                    if n(format!("expOptionCommand{}",index+1).as_bytes(),0)?==5 {quick_button=Some(index);break;}
                                }
                                if let Some(index)=quick_button {
                                    // The source enables the first quick-load control
                                    // after applying mode gates; preserve that order.
                                    spec.buttons[index].enabled=quick.data_exists(&config,storage.as_ref())?;
                                }
                            }
                        }
                        if kind==Kind::System && !matches!(return_mode,9|10) {
                            for index in [3,4,5,6] {if let Some(button)=spec.buttons.get_mut(index) {button.enabled=false;}}
                        }
                        if kind==Kind::System {spec.music_number=0;spec.music_restart=false;spec.menu_wait=0;spec.menu_fade=0;spec.menu_effect=0;spec.menu_delta=[0;2];spec.auto_restart=0;spec.script_end_exits=false;}
                        for (i, flag) in if kind==Kind::System {vec![]} else {vec![
                            (0, b"cgModeExistFlag".as_slice()),
                            (1, b"sceneModeExistFlag"),
                            (2, b"bgmModeExistFlag"),
                            (4, b"movieModeExistFlag"),
                        ]} {
                            if let Some(button) = spec.buttons.get_mut(i) {
                                button.exists &= n(flag, i32::from(i != 4))? != 0;
                            }
                        }
                        page.skin = Some(spec);
                        page.table = Some(table);
                    }
                    return Ok(page);
                }
                ensure!(
                    n(b"extModeNumber", 0)? == 0,
                    "NNN {} menu extension requires its native mode actor",
                    kind.name()
                );
                if kind == Kind::Config {
                    return Ok(Page {
                        background,
                        slots: vec![],
                        pages: 1,
                        columns: 1,
                        origin: [0; 2],
                        step: [0; 2],
                        sound: 0,
                        error_sound: -1,
                        skin: None,
                        image: None,
                        table: None,
                    });
                }
                let pages = usize::try_from(n(b"page", 10)?)?;
                let columns = usize::try_from(n(b"blockNumberX", 4)?)?;
                let rows = usize::try_from(n(b"blockNumberY", 3)?)?;
                ensure!(
                    (1..=100).contains(&pages)
                        && (1..=16).contains(&columns)
                        && (1..=16).contains(&rows)
                        && columns * rows <= 64
                        && pages * columns * rows <= 10000,
                    "NNN load/save menu geometry budget"
                );
                ensure!(page < pages, "NNN load/save page bounds");
                let origin = [n(b"printX", 85)?, n(b"printY", 113)?];
                let step = [n(b"nextX", 166)?, n(b"nextY", 124)?];
                ensure!(
                    origin
                        .iter()
                        .chain(step.iter())
                        .all(|v| (-32767..=32767).contains(v)),
                    "NNN slot menu coordinates"
                );
                let profile = LegacySaveProfile::for_configuration(&config)?;
                let mut slots = Vec::new();
                let mut pixels = 0usize;
                for i in 0..columns * rows {
                    cancel.check()?;
                    let number = page * columns * rows + i;
                    let source = match storage
                        .open_read(&crate::native_save::slot_path(&config,number)?)
                    {
                        Ok(source) => source,
                        Err(Error::NotFound) => {
                            slots.push(Slot {
                                action: None,
                                number,
                                text: format!("{:03}  Empty", number + 1),
                                valid: false,
                                occupied: false,
                                thumbnail: None,
                            });
                            continue;
                        }
                        Err(error) => return Err(error.into()),
                    };
                    let bytes = crysirror_resource::read_all(source.as_ref(), 64 << 20)?;
                    match NativeSaveFile::parse(&bytes, &profile) {
                        Ok(file) => {
                            let w = u32::from_le_bytes(
                                file.blocks[3].payload[0..4].try_into().unwrap(),
                            );
                            let h = u32::from_le_bytes(
                                file.blocks[3].payload[4..8].try_into().unwrap(),
                            );
                            pixels = pixels
                                .checked_add(file.blocks[3].payload.len() - 16)
                                .context("NNN slot thumbnail budget overflow")?;
                            ensure!(pixels <= 32 << 20, "NNN slot thumbnail page budget");
                            let thumbnail = crysirror_image::packed_pixels::decode(
                                [w, h],
                                crysirror_core::image_processing::PackedPixelLayout::Bgrx8,
                                &file.blocks[3].payload[16..],
                                &|| cancel.check(),
                            )?;
                            let date = format!(
                                "{:04}/{:02}/{:02}",
                                file.status_word(8)?,
                                file.status_word(9)?,
                                file.status_word(11)?
                            );
                            slots.push(Slot {
                                action: None,
                                number,
                                text: format!("{:03}  {date}", number + 1),
                                valid: true,
                                occupied: true,
                                thumbnail: Some(thumbnail),
                            });
                        }
                        Err(_) => slots.push(Slot {
                            action: None,
                            number,
                            text: format!("{:03}  Invalid save", number + 1),
                            valid: false,
                            occupied: true,
                            thumbnail: None,
                        }),
                    }
                }
                Ok(Page {
                    background,
                    slots,
                    pages,
                    columns,
                    origin,
                    step,
                    sound: n(
                        if kind == Kind::Load {
                            b"loadSound"
                        } else {
                            b"saveSound"
                        },
                        1,
                    )?,
                    error_sound: n(b"cannotLoadSound", -1)?,
                    skin: None,
                    image: None,
                    table: None,
                })
            })?);
        }
        let Some(page) = self.task.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        self.task = None;
        self.load_sound = page.sound;
        self.error_sound = page.error_sound;
        if !self.back_script_started {
            self.back_script = page.background.clone();
        }
        let image = page.image.clone();
        let skin = page.skin.clone();
        self.ready = Some(page);
        if let Some(image) = image {
            self.skin_job = Some(
                host.image_metadata(client)?
                    .begin_process(image, b"[]".as_slice().into())?,
            );
        }
        if let Some(spec) = skin {
            let mut player = crate::native_title::NativeTitlePlayer::new(
                client,
                ROOT + 5000,
                ROOT + 6000,
                SIZE,
                u64::try_from(self.config.setup.integer(b"frameTime")?.unwrap_or(50))?,
                spec,
                self.resources
                    .clone()
                    .context("NNN Omake image capability absent")?,
            )?;
            player.set_widget_parent(ROOT)?;
            player.set_cancel_button(if self.kind==Kind::System{7}else{3});
            self.skin = Some(player);
        }
        self.prepare_skin()
    }
    fn build(&mut self, client: ClientId, font: &str, host: &mut dyn HostServices) -> Result<()> {
        let mut ui = vec![
            json!({"op":"put","widget":{"id":ROOT,"kind":"panel","rect":[0,0,SIZE[0],SIZE[1]],"modal":true,"z":1000,"color":[0,0,0,0],"font":font}}),
        ];
        if self.skin.is_some() && self.confirm.is_some() {
            // Keep the native painter's completed frame. Replacing it with the
            // generic catalogue background would erase the original buttons.
            ui.push(json!({"op":"put","widget":{"id":ROOT+8,"parent":ROOT,"kind":"panel","rect":[210,230,390,140],"modal":true,"z":1001,"color":[20,24,32,255],"font":font}}));
            for (id,kind,rect,text) in [(ROOT+5,"label",[15,10,360,40],"タイトルに戻りますか？"),(ROOT+6,"button",[30,65,140,36],"はい"),(ROOT+7,"button",[200,65,140,36],"いいえ")] {
                ui.push(json!({"op":"put","widget":{"id":id,"parent":ROOT+8,"kind":kind,"rect":rect,"text":text,"font":font,"text_size":18,"color":if kind=="label"{[0,0,0,0]}else{[48,62,80,255]}}}));
            }
            let mut commands=vec![json!({"op":"remove","id":ROOT})];commands.extend(ui);
            host.submit_engine(client,1,&serde_json::to_vec(&json!({"ui":commands}))?)?;
            self.published=true;return Ok(())
        }
        if self.skin.is_some() && self.confirm.is_none() {
            let layers=std::mem::take(&mut self.nodes).into_iter().map(|id|json!({"op":"remove","id":id})).collect::<Vec<_>>();
            host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":layers}))?)?;
            for path in std::mem::take(&mut self.paths) {host.release_image_resource(client,&path)?;}
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"ui":ui}))?)?;
            self.published = true;
            return Ok(());
        }
        let widget = |id, kind: &str, rect: [i32; 4], text: String, enabled: bool| json!({"op":"put","widget":{"id":id,"parent":ROOT,"kind":kind,"rect":rect,"text":text,"font":font,"text_size":16,"enabled":enabled,"keyboard_focusable":kind!="label","color":if kind=="label" {[0,0,0,0]} else {[48,62,80,255]}}});
        ui.push(widget(
            ROOT + 1,
            "button",
            [666, 16, 115, 36],
            "Back".into(),
            true,
        ));
        ui.push(widget(
            ROOT + 2,
            "label",
            [24, 16, 420, 36],
            self.kind.name().to_uppercase(),
            true,
        ));
        let mut layers = vec![
            json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,SIZE[0],SIZE[1]],"order":[1999999999,0],"paint":{"kind":"solid","color":[16,20,28,if self.back_script.is_some(){0}else{250}]}}}),
        ];
        let mut paths = Vec::new();
        if let Some(image) = self.skin_base.as_ref() {
            let size = image.size();
            let path = host
                .publish_image_resource(client, ROOT, image.clone())?
                .path
                .relative()
                .to_owned();
            layers[0] = json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,size[0],size[1]],"order":[1999999999,0],"paint":{"kind":"image","asset":path}}});
            paths.push(path);
        }
        let mut nodes = vec![ROOT];
        if self.kind == Kind::Config {
            for (i, (id, value)) in self.settings.iter().enumerate() {
                let s = crate::native_settings::setting(*id)?;
                let x = 24 + (i / 8) as i32 * 388;
                let y = 75 + (i % 8) as i32 * 61;
                ui.push(widget(
                    ROOT + 100 + i as u32 * 2,
                    "label",
                    [x, y, 360, 22],
                    s.name.into(),
                    true,
                ));
                let mut control = widget(
                    ROOT + 101 + i as u32 * 2,
                    if s.maximum == 1 { "toggle" } else { "slider" },
                    [x, y + 25, 345, 24],
                    String::new(),
                    true,
                );
                control["widget"]["value"] = json!(*value as f32 / s.maximum as f32);
                ui.push(control);
            }
        } else {
            let page = self
                .ready
                .as_ref()
                .context("NNN slot menu page unprepared")?;
            ui.push(widget(
                ROOT + 3,
                "button",
                [470, 16, 80, 36],
                "<".into(),
                self.page > 0,
            ));
            ui.push(widget(
                ROOT + 4,
                "button",
                [565, 16, 80, 36],
                ">".into(),
                self.page + 1 < page.pages,
            ));
            for (i, slot) in page.slots.iter().enumerate() {
                let point = [
                    page.origin[0] + (i % page.columns) as i32 * page.step[0],
                    page.origin[1] + (i / page.columns) as i32 * page.step[1],
                ];
                ui.push(widget(
                    ROOT + 100 + i as u32,
                    "button",
                    [point[0], point[1] - 23, 150, 23],
                    slot.text.clone(),
                    self.kind == Kind::Save || slot.valid,
                ));
                if let Some(image) = &slot.thumbnail {
                    let id = ROOT + 1000 + i as u32;
                    let path =
                        host.upload_rgba(client, id, image.width, image.height, &image.rgba)?;
                    paths.push(path.relative().to_owned());
                    nodes.push(id);
                    layers.push(json!({"op":"put","layer":{"id":id,"rect":[point[0],point[1],image.width,image.height],"order":[2000000000,i],"paint":{"kind":"image","asset":path.relative()}}}));
                    ui.push(json!({"op":"put","widget":{"id":ROOT+200+i as u32,"parent":ROOT,"kind":"button","rect":[point[0],point[1],image.width,image.height],"interaction_only":true,"enabled":self.kind==Kind::Save||slot.valid,"keyboard_focusable":false,"font":font}}));
                }
            }
        }
        if self.confirm.is_some() {
            ui.push(json!({"op":"put","widget":{"id":ROOT+8,"parent":ROOT,"kind":"panel","rect":[210,230,390,140],"modal":true,"z":1001,"color":[20,24,32,255],"font":font}}));
            for mut command in [
                widget(
                    ROOT + 5,
                    "label",
                    [15, 10, 360, 40],
                    match self.confirm.as_ref() {Some(Action::ReturnTitle)=>"タイトルに戻りますか？",Some(Action::Load(_))=>"このデータをロードしますか？",_=>"このデータに上書きしますか？"}.into(),
                    true,
                ),
                widget(ROOT + 6, "button", [30, 65, 140, 36], "はい".into(), true),
                widget(
                    ROOT + 7,
                    "button",
                    [200, 65, 140, 36],
                    "Cancel".into(),
                    true,
                ),
            ] {
                command["widget"]["parent"] = json!(ROOT + 8);
                ui.push(command);
            }
        }
        let old_nodes = std::mem::replace(&mut self.nodes, nodes);
        let old_paths = std::mem::replace(&mut self.paths, paths);
        let mut remove = old_nodes
            .into_iter()
            .filter(|id| !self.nodes.contains(id))
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        remove.extend(layers);
        // Recreate this modal subtree as one batch. Unrelated native wait
        // widgets and their retained activation receipts remain untouched.
        let mut commands = vec![json!({"op":"remove","id":ROOT})];
        commands.extend(ui);
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"layers":remove,"ui":commands}))?,
        )?;
        for path in old_paths {
            host.release_image_resource(client, &path)?;
        }
        self.published = true;
        Ok(())
    }
    fn prepare_skin(&mut self) -> Result<bool> {
        if self.kind==Kind::System && self.skin_base.is_none() {self.skin_base=self.background.clone();}
        if let Some(job) = self.skin_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN gallery background: {e}"),
                ImageProcessState::Ready(image) => {
                    self.skin_base = Some(image);
                    self.skin_job = None;
                }
            }
        }
        Ok(true)
    }
    pub(super) fn entry_blocks_update(&self,now_ms:u64)->bool {
        self.skin.as_ref().is_some_and(|skin|skin.entry_blocks_update(now_ms))
    }
    pub(super) fn has_skin(&self) -> bool {
        self.skin.is_some()
    }
    pub(super) fn poll_skin(
        &mut self,
        client: ClientId,
        font: &str,
        pool: &TaskPool,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        vm: &mut NativeAdapter,
    ) -> Result<Option<Action>> {
        if self.confirm.is_some() {return self.poll(client,font,pool,host,input,admit);}
        if !self.published {
            self.build(client, font, host)?;
        }
        let skin = self
            .skin
            .as_mut()
            .context("NNN native Omake buttons absent")?;
        if !admit {
            skin.poll_existing_dependencies(host, Some(vm))?;
            return Ok(None);
        }
        let base = self
            .skin_base
            .clone()
            .or_else(|| vm.presented_frame())
            .context("NNN Omake native background absent")?;
        let Some(index) = skin.poll(host, input, base, vm)? else {
            return Ok(None);
        };
        let table = self
            .ready
            .as_ref()
            .and_then(|p| p.table.as_ref())
            .context("NNN Omake native setup absent")?;
        if self.kind==Kind::System {return Ok(Some(Action::System(index)));}
        Ok(Some(match index {
            0 => Action::Navigate(if table.integer(b"autoCGCharaFlag")?.unwrap_or(0) != 0 {
                Kind::Cg(0)
            } else {
                Kind::CgCharacters
            }),
            1 => Action::Navigate(if table.integer(b"autoSceneCharaFlag")?.unwrap_or(0) != 0 {
                Kind::Scenes(0)
            } else {
                Kind::SceneCharacters
            }),
            2 => Action::Navigate(Kind::Music),
            3 => Action::Close,
            4 if table.integer(b"basicButtonNumber")?.unwrap_or(4) >= 5 => {
                Action::Navigate(Kind::Movies)
            }
            _ => match crate::native_title::extension_request(table, &self.config, index, false)? {
                crate::native_title::MenuRequest::Mode(mode) => {
                    Action::Navigate(Kind::from_native(mode)?)
                }
                crate::native_title::MenuRequest::Script { variable, value } => {
                    Action::StartGame { variable, value }
                }
            },
        }))
    }
    pub fn poll(
        &mut self,
        client: ClientId,
        font: &str,
        pool: &TaskPool,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
    ) -> Result<Option<Action>> {
        if let Some(movie) = self.movie.as_mut() {
            let input = if admit { input } else { InputFrame::default() };
            if movie.poll(host, input)?.is_some() {
                self.movie = None;
                self.published = false;
            }
            return Ok(None);
        }
        if let Some(viewer) = self.viewer.as_mut() {
            let finished = viewer.poll(host, input, admit)?;
            if let Some(audio) = viewer.take_audio() { return Ok(Some(Action::GalleryAudio(audio))); }
            if finished {
                viewer.remove(host)?;
                self.viewer = None;
                self.published = false;
            }
            return Ok(None);
        }
        if !self.prepare_page(client, pool, host)? {
            return Ok(None);
        }
        if !self.published {
            self.build(client, font, host)?;
            return Ok(None);
        }
        if !admit {
            return Ok(None);
        }
        if input.buttons_pressed & buttons::ESCAPE != 0 {
            return Ok(Some(Action::Close));
        }
        let events: Vec<Value> = serde_json::from_slice(
            &host.query_engine(client, EngineQuery::UiSubtreeEvents(ROOT))?,
        )?;
        for event in events {
            let Some(id) = event["widget"]["id"]
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
            else {
                continue;
            };
            if id < ROOT {
                continue;
            }
            if self.kind == Kind::Config {
                if matches!(event["kind"].as_str(), Some("change" | "activate")) && id >= ROOT + 101
                {
                    let index = (id - ROOT - 101) / 2;
                    if (id - ROOT - 101) % 2 == 0 {
                        if let Some((parameter, _)) = self.settings.get(index as usize) {
                            let setting = crate::native_settings::setting(*parameter)?;
                            let state: Value = serde_json::from_slice(
                                &host.query_engine(client, EngineQuery::UiWidget(id))?,
                            )?;
                            let v = state["value"]
                                .as_f64()
                                .or_else(|| state["spec"]["value"].as_f64())
                                .context("NNN settings control value absent")?;
                            ensure!(v.is_finite(), "NNN settings slider is not finite");
                            let value =
                                (v.clamp(0., 1.) * f64::from(setting.maximum)).round() as i32;
                            return Ok(Some(Action::Setting(*parameter, value)));
                        }
                    }
                }
            }
            if event["kind"] != "activate" {
                continue;
            }
            match id - ROOT {
                1 => return Ok(Some(Action::Close)),
                6 if self.confirm.is_some() => return Ok(self.confirm.take()),
                7 if self.confirm.is_some() => {
                    if matches!(self.confirm,Some(Action::ReturnTitle)) {self.confirm=None;return Ok(Some(Action::Close));}
                    self.confirm = None;
                    self.published = false;
                    if self.skin.is_some() {
                        host.submit_engine(client,1,&serde_json::to_vec(&json!({"ui":[{"op":"remove","id":ROOT}]}))?)?;
                        self.build(client,font,host)?;
                        self.skin.as_mut().unwrap().resume_after_menu(host)?;
                    }
                    return Ok(None);
                }
                3 if self.kind != Kind::Config && self.page > 0 => {
                    self.page -= 1;
                    self.refresh();
                    return Ok(None);
                }
                4 if self.kind != Kind::Config
                    && self.ready.as_ref().is_some_and(|p| self.page + 1 < p.pages) =>
                {
                    self.page += 1;
                    self.refresh();
                    return Ok(None);
                }
                i if self.kind != Kind::Config
                    && self.confirm.is_none()
                    && ((100..164).contains(&i) || (200..264).contains(&i)) =>
                {
                    let index = if i >= 200 { i - 200 } else { i - 100 };
                    if let Some(slot) = self
                        .ready
                        .as_ref()
                        .and_then(|p| p.slots.get(index as usize))
                    {
                        if self.kind.gallery() && slot.valid {
                            return Ok(slot.action.clone());
                        }
                        if self.kind == Kind::Load && slot.valid {
                            return Ok(Some(Action::Load(i32::try_from(slot.number)?)));
                        }
                        if self.kind == Kind::Save {
                            let action = Action::Save(u32::try_from(slot.number)?);
                            if slot.occupied {
                                self.confirm = Some(action);
                                self.published = false;
                            } else {
                                return Ok(Some(action));
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(None)
    }
    pub fn update_setting(&mut self, id: i32, value: i32) {
        if let Some(pair) = self.settings.iter_mut().find(|v| v.0 == id) {
            pair.1 = value;
        }
    }
}
