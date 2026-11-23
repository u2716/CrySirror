// SPDX-License-Identifier: MPL-2.0
//! Native minori scenario execution over CrySirror's shared services.
#![forbid(unsafe_code)]
mod camera;
mod command;
mod controls;
mod firefly;
mod globals;
mod message;
mod message_action;
mod parameters;
mod presentation;
mod script;
mod shake;
mod rain;
mod snow;
mod snow2;
mod strips;
mod tile_transition;
mod vm;

use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{AudioPreparePhase, ClientStorage, HostServices, InputFrame, buttons},
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use presentation::Presentation;
use script::{Language, Program};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
};
use vm::{Vm, VmState};

/// SC dialect with language-tagged lines and NEXT script/label transfers.
pub const SCENARIO_ABI: &str = "sc-next";

type Registry = Option<Arc<BTreeSet<String>>>;

struct Factory;
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("minori", Arc::new(Factory))
}

impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "minori requires a native scenario");
        ensure!(
            matches!(
                spec.config["abi"].as_str(),
                Some(SCENARIO_ABI | "ef-first-aa")
            ),
            "unsupported minori scenario ABI"
        );
        let entry = script_name(spec.config["script"].as_str().unwrap_or("test.sc"))?;
        let lang = Language::from_code(spec.config["language"].as_str().unwrap_or("j"))
            .context("unknown minori script language")?;
        let canvas = match spec.config.get("canvas") {
            Some(value) => {
                serde_json::from_value(value.clone()).context("minori stage dimensions")?
            }
            None => [800, 600],
        };
        ensure!(
            canvas.iter().all(|v| (1..=8192).contains(v)),
            "minori stage dimensions"
        );
        let commands: Option<BTreeSet<String>> =
            serde_json::from_value(spec.config["commands"].clone())
                .context("invalid minori command registry")?;
        let screen_abi = match commands.as_ref() {
            Some(names) if names.contains("scrollXF") => shake::ScreenAbi::PersistentZoom,
            Some(names) if names.contains("char") => shake::ScreenAbi::SignedDuration,
            _ => shake::ScreenAbi::Early,
        };
        let mut view = Presentation::new(canvas, screen_abi);
        view.panel_layout = serde_json::from_value(spec.config["message_panel_layout"].clone())
            .context("unknown minori message panel layout")?;
        view.tile_grid = serde_json::from_value(spec.config["tile_transition_grid"].clone())
            .context("unknown minori transition grid")?;
        if let Some(programs)=spec.config.get("eye_sequences") {view.set_eye_sequences(serde_json::from_value(programs.clone())?)?;}
        if let Some(actors) = spec.config.get("voice_actors") {
            view.voice_actors = serde_json::from_value(actors.clone())?;
        }
        if let Some(marks)=spec.config.get("voice_marks") {
            view.voice_marks=serde_json::from_value(marks.clone())?;
            ensure!(view.voice_marks.is_empty() || view.voice_marks.len()==view.voice_actors.len(),"minori voice checkbox table");
        }
        Ok(Box::new(Adapter {
            episodes: spec.config.get("episodes")
                .cloned()
                .map(serde_json::from_value)
                .transpose()?
                .unwrap_or_default(),
            routes: serde_json::from_value(
                spec.config
                    .get("routes")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            )?,
            title: commands
                .as_ref()
                .map(|_| presentation::title::Title::default()),
            registry: commands.map(Arc::new),
            client,
            entry,
            lang,
            loading: None,
            session: None,
            view,
            scenario_timer: vm::ScenarioTimer::default(),
            bgm_timer: vm::ScenarioTimer::default(),
            control_down: false,
            control_latched: false,
            last: None,
            next: None,
            writing: None,
            parameter_write: None,
            closing: false,
        }))
    }
}

fn script_name(name: &str) -> Result<String> {
    let name = name.to_ascii_lowercase();
    ensure!(
        !name.contains(['/', '\\'])
            && (name.ends_with(".sc") || name.ends_with(".mis") || name.ends_with(".inc"))
            && name.len() <= 256,
        "invalid minori script name"
    );
    VfsPath::new(Mount::Asset, &name)?;
    Ok(name)
}

struct Session {
    effect_settings: parameters::EffectSettings,
    sound: parameters::SoundSettings,
    ctrl_skip: bool,
    panel_color: u32,
    speeds: parameters::Speeds,
    vm: Vm,
    storage: Arc<dyn ClientStorage>,
    persisted: HashMap<String, i32>,
}

fn read_script(
    storage: &dyn ClientStorage,
    name: &str,
    lang: Language,
    registry: &Registry,
) -> Result<Program> {
    let path = VfsPath::new(Mount::Asset, &format!("scr/{}", script_name(name)?))?;
    let reader = storage.open_asset(&path)?;
    let program = parse_script(
        &crysirror_resource::read_all(reader.as_ref(), 4 << 20)?,
        name,
        lang,
        registry.as_deref().filter(|r| r.contains("scrollXF")),
    )?;
    let program = if registry.as_ref().is_some_and(|r| r.contains("scrollXF")) {
        controls::expand(program)?
    } else {
        program
    };
    if let Some(commands) = registry {
        for statement in &program.statements {
            if let script::Statement::Command { kind, .. } = statement {
                ensure!(
                    commands.contains(kind.name()),
                    "minori {name}: .{} is absent from the native registry",
                    kind.name()
                );
            }
        }
    }
    Ok(program)
}

fn worker<T: Send + 'static>(
    task: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<Receiver<Result<T>>> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("minori-resource".into())
        .spawn(move || {
            let _ = tx.send(task());
        })?;
    Ok(rx)
}
fn poll<T>(rx: &Receiver<Result<T>>) -> Result<Option<T>> {
    match rx.try_recv() {
        Ok(result) => Ok(Some(result?)),
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => anyhow::bail!("minori resource worker stopped"),
    }
}

pub fn validate_entry(bytes: &[u8]) -> Result<()> {
    let program = parse_script(bytes, "test.sc", Language::Japanese, None)?;
    ensure!(
        program.statements.iter().any(|s| matches!(
            s,
            script::Statement::Command { .. }
                | script::Statement::Include(_)
                | script::Statement::Control { .. }
        )),
        "no minori scenario instructions"
    );
    Ok(())
}

fn parse_script(bytes: &[u8], name: &str, lang: Language, commands: Option<&BTreeSet<String>>) -> Result<Program> {
    let source = [encoding_rs::SHIFT_JIS, encoding_rs::GBK]
        .into_iter()
        .find_map(|codec| codec.decode_without_bom_handling_and_without_replacement(bytes))
        .with_context(|| format!("invalid scenario encoding in {name}"))?;
    Program::parse_registered(&source, lang, commands)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("parse {name}"))
}

fn load(
    storage: Arc<dyn ClientStorage>,
    entry: String,
    lang: Language,
    registry: Registry,
) -> Result<Session> {
    let first = read_script(storage.as_ref(), &entry, lang, &registry)?;
    let persisted = globals::read(storage.as_ref())?;
    let mut vm = Vm::new(entry, first);
    if let Some(names) = &registry {
        vm.audio_defaults = if names.contains("scrollXF") {(200,200)} else if names.len()==43 {(1,2)} else {(2,2)};
    }
    vm.replace_globals(persisted.clone());
    let (speeds, panel_color, ctrl_skip, sound, effect_settings) = parameters::Speeds::read(storage.as_ref())?;
    Ok(Session {
        effect_settings,
        sound,
        ctrl_skip,
        speeds,
        panel_color,
        vm,
        storage,
        persisted,
    })
}

struct Adapter {
    episodes: Vec<String>,
    routes: Vec<(String, [i32; 4])>,
    title: Option<presentation::title::Title>,
    registry: Registry,
    client: ClientId,
    entry: String,
    lang: Language,
    loading: Option<Receiver<Result<Session>>>,
    session: Option<Session>,
    view: Presentation,
    scenario_timer: vm::ScenarioTimer,
    bgm_timer: vm::ScenarioTimer,
    control_down: bool,
    control_latched: bool,
    last: Option<u64>,
    next: Option<(String, Receiver<Result<Program>>)>,
    writing: Option<Receiver<Result<HashMap<String, i32>>>>,
    parameter_write: Option<Receiver<Result<()>>>,
    closing: bool,
}

impl Adapter {
    fn finish(
        session: &Session,
        writing: &mut Option<Receiver<Result<HashMap<String, i32>>>>,
    ) -> Result<VmYield> {
        let values = session.vm.globals_snapshot();
        if values == session.persisted {
            return Ok(VmYield::Halted);
        }
        let storage = session.storage.clone();
        *writing = Some(worker(move || {
            globals::write(storage.as_ref(), &values)?;
            Ok(values)
        })?);
        Ok(VmYield::BudgetExhausted)
    }
    fn run(&mut self, h: &mut dyn HostServices, input: InputFrame, budget: u32) -> Result<VmYield> {
        // The window procedure writes on Ctrl key messages. Holding the key
        // across a scenario tick does not undo a .debug clear.
        let down = input.buttons_down & buttons::CTRL != 0;
        if down != self.control_down || input.buttons_pressed & buttons::CTRL != 0 {
            if h.window_focused(self.client)? != Some(false) {
                self.control_latched = down;
            }
            self.control_down = down;
        }
        let now = h.now_ms();
        let mut counter = self.scenario_timer.ticks(now);
        let mut bgm_ticks = self.bgm_timer.ticks(now);
        let Some(session) = self.session.as_mut() else {
            if let Some(rx) = &self.loading {
                if let Some(session) = poll(rx)? {
                    self.view.effect_settings = session.effect_settings;
                    self.view.speeds = session.speeds;
                    self.view.sound_settings = session.sound.clone();
                    self.view.panel_color = session.panel_color;
                    self.session = Some(session);
                    self.loading = None;
                    self.last = Some(now);
                }
                return Ok(VmYield::BudgetExhausted);
            }
            let storage = h.storage(self.client)?;
            let entry = self.entry.clone();
            let lang = self.lang;
            let registry = self.registry.clone();
            self.loading = Some(worker(move || load(storage, entry, lang, registry))?);
            return Ok(VmYield::BudgetExhausted);
        };
        if let Some(write) = &self.parameter_write {
            if poll(write)?.is_none() {
                return Ok(VmYield::BudgetExhausted);
            }
            self.parameter_write = None;
        }
        if let Some(write) = &self.writing {
            let Some(saved) = poll(write)? else {
                return Ok(VmYield::BudgetExhausted);
            };
            self.writing = None;
            session.persisted = saved;
        }
        let mut supplied = None;
        if let Some((name, rx)) = &self.next {
            let Some(program) = poll(rx)? else {
                return Ok(VmYield::BudgetExhausted);
            };
            supplied = Some((name.clone(), program));
            self.next = None;
        }
        if self.closing {
            return Self::finish(session, &mut self.writing);
        }
        if let Some(title) = &mut self.title {
            use presentation::title::Outcome;
            match title.tick(
                h,
                self.client,
                &mut self.view,
                &session.vm,
                input,
                &self.routes,
                &self.episodes,
            )? {
                Outcome::Pending => return Ok(VmYield::BudgetExhausted),
                Outcome::Waiting => return Ok(VmYield::WaitForInput),
                Outcome::SettingsClosed => {
                    let speeds = self.view.speeds;
                    let sound = self.view.sound_settings.clone();
                    if speeds != session.speeds || sound != session.sound {
                        let previous = session.speeds;
                        let old_sound=session.sound.clone();
                        let changes=sound.clone();
                        let storage = session.storage.clone();
                        self.parameter_write = Some(worker(move || speeds.write_all(storage.as_ref(),previous,&changes,&old_sound))?);
                        session.speeds = speeds;
                        session.sound = sound;
                        return Ok(VmYield::BudgetExhausted);
                    }
                    return Ok(VmYield::WaitForInput);
                }
                Outcome::Close => {
                    self.closing = true;
                    return Self::finish(session, &mut self.writing);
                }
                Outcome::Story { script, episode } => {
                    let storage = session.storage.clone();
                    let lang = self.lang;
                    let registry = self.registry.clone();
                    self.loading = Some(worker(move || {
                        let mut scene = load(storage, script, lang, registry)?;
                        if episode {
                            scene.vm.begin_episode_replay();
                        }
                        Ok(scene)
                    })?);
                    self.session = None;
                    self.title = None;
                    self.last = Some(now);
                    return Ok(VmYield::BudgetExhausted);
                }
                Outcome::Start => {
                    self.title = None;
                    self.last = Some(now);
                    return Ok(VmYield::BudgetExhausted);
                }
            }
        }
        if let Some(ticket) = self.view.audio_ticket {
            let prep = h.audio_preparation(self.client, ticket)?;
            if prep.phase == AudioPreparePhase::Pending {
                return Ok(VmYield::BudgetExhausted);
            }
            h.release_audio(self.client, ticket)?;
            self.view.audio_ticket = None;
            ensure!(
                prep.phase == AudioPreparePhase::Ready,
                "minori audio: {}",
                prep.error.unwrap_or_default()
            );
        }
        let mut delta = now
            .saturating_sub(self.last.replace(now).unwrap_or(now))
            .min(u64::from(u32::MAX)) as u32;
        let vm = &mut session.vm;
        self.view.poll_fades(h,self.client)?;
        if !self.view.poll_stage(h, self.client)? {
            return Ok(VmYield::BudgetExhausted);
        }
        if !self.view.poll_cutin_catalogue(h, self.client)? {
            return Ok(VmYield::BudgetExhausted);
        }
        self.view.poll_foreground(h,self.client)?;
        if self.view.foreground_pending(){return Ok(VmYield::BudgetExhausted);}
        self.view.poll_animations(h,self.client)?;
        if self.view.animation_loading(){return Ok(VmYield::BudgetExhausted);}
        self.view.poll_sequences(h,self.client)?;
        if self.view.sequence_pending() { return Ok(VmYield::BudgetExhausted); }
        self.view.poll_cutin(h, self.client)?;
        self.view.poll_walking_scroll(h, self.client, vm)?;
        self.view.poll_snow(h, self.client)?;
        if self.view.snow_loading() {
            return Ok(VmYield::BudgetExhausted);
        }
        let activate =
            input.buttons_pressed & (buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT) != 0;
        let has_control_lock = self.registry.as_ref().is_none_or(|r| r.contains("movie2"));
        let ctrl_skip =
            input.buttons_down & buttons::CTRL != 0 && (!has_control_lock || session.ctrl_skip);
        self.view.poll_scroll(
            h,
            self.client,
            ctrl_skip && vm.permits_key_skip(has_control_lock),
            activate,
        )?;
        let skip = ctrl_skip && vm.permits_key_skip(has_control_lock);
        for fg in [false, true] { self.view.poll_rotation(h, self.client, fg, skip)?; }
        if self.view.rotation_pending() { return Ok(VmYield::BudgetExhausted); }
        self.view.poll_characters(h, self.client, activate)?;
        self.view.poll_message_actions(h, self.client, (activate || skip) && matches!(vm.state(), VmState::WaitingInput))?;
        self.view.poll_shake(
            h,
            self.client,
            input.buttons_down & buttons::SHIFT != 0,
            activate,
        )?;
        if self.view.shake_blocks() {
            return Ok(VmYield::WaitUntil(now.saturating_add(10)));
        }
        match vm.state() {
            VmState::WaitingInput if self.view.revealing() => {
                if !self.view.poll_message(h, self.client, activate)? {
                    return Ok(VmYield::WaitUntil(now.saturating_add(10)));
                }
                return Ok(VmYield::WaitForInput);
            }
            VmState::WaitingInput if activate || self.view.auto_advance_message => {
                self.view.auto_advance_message = false;
                vm.advance_input();
            }
            VmState::WaitingTransition if self.view.transition_done(h, self.client,
                skip || input.buttons_pressed & (buttons::ENTER | buttons::MOUSE_LEFT) != 0)? => {
                self.view.finish_stage(h, self.client)?;
                vm.complete_transition()
            }
            VmState::WaitingSelect { .. } => {
                if let Some(index) = self.view.select(h, self.client, input)? {
                    vm.complete_select(index).map_err(anyhow::Error::msg)?
                }
            }
            VmState::WaitingCharMove if !self.view.character_moving() => vm.complete_char_move(),
            VmState::WaitingAnimation{id,..} if skip || !self.view.animation_playing(*id) => vm.finish_animation_wait(),
            VmState::WaitingRotation(fg) if skip || !self.view.rotation_active(*fg) => vm.finish_rotation_wait(),
            VmState::WaitingScroll if !self.view.scrolling() => vm.complete_scroll(),
            VmState::WaitingPanel if self.view.poll_panel(h, self.client)? => vm.complete_panel(),
            VmState::WaitingMovie if self.view.poll_movie(h, self.client, activate)? => {
                vm.complete_movie()
            }
            _ => {}
        }
        // The first registry predates the control-lock and cls preference.
        for _ in 0..budget {
            if matches!(
                vm.state(),
                VmState::WaitingScenarioTimer { .. } | VmState::WaitingBgmTimer { .. }
            ) {
                let skip = ctrl_skip && vm.permits_key_skip(has_control_lock);
                vm.poll_scenario_timer(counter, skip);
                vm.poll_bgm_timer(bgm_ticks, skip);
                if matches!(
                    vm.state(),
                    VmState::WaitingScenarioTimer { .. } | VmState::WaitingBgmTimer { .. }
                ) {
                    return Ok(VmYield::WaitUntil(self.scenario_timer.next_tick_ms(now)));
                }
            }
            match vm.state() {
                VmState::WaitingInput if self.view.revealing() => {
                    return Ok(VmYield::WaitUntil(now.saturating_add(10)));
                }
                VmState::WaitingInput | VmState::WaitingSelect { .. } => {
                    return Ok(VmYield::WaitForInput);
                }
                VmState::WaitingAnimation{..}
                | VmState::WaitingRotation(_)
                | VmState::WaitingScroll
                | VmState::WaitingCharMove
                | VmState::WaitingTransition
                | VmState::WaitingPanel
                | VmState::WaitingMovie => {
                    return Ok(VmYield::WaitUntil(now.saturating_add(1)));
                }
                VmState::Finished => {
                    let end = Self::finish(session, &mut self.writing)?;
                    if matches!(end, VmYield::Halted) && self.registry.is_some() {
                        self.view.leave_story(h, self.client)?;
                        self.title = Some(presentation::title::Title::default());
                        self.session = None;
                        self.last = None;
                        return Ok(VmYield::BudgetExhausted);
                    }
                    return Ok(end);
                }
                _ => {}
            }
            let mut missing = None;
            let result = vm.tick(delta, |name| {
                if supplied
                    .as_ref()
                    .is_some_and(|(n, _)| n.eq_ignore_ascii_case(name))
                {
                    return Ok(supplied.take().unwrap().1);
                }
                missing = Some(name.to_owned());
                Err("script preparation pending".into())
            });
            if let Some(name) = missing {
                let normalized = script_name(&name)?;
                let storage = session.storage.clone();
                let lang = self.lang;
                let registry = self.registry.clone();
                self.next = Some((
                    name,
                    worker(move || read_script(storage.as_ref(), &normalized, lang, &registry))?,
                ));
                return Ok(VmYield::BudgetExhausted);
            }
            let events = result
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("minori {}", vm.current_script().unwrap_or("<end>")))?;
            delta = 0;
            for event in events {
                if event == vm::VmEvent::ClearControlKeyLatch {
                    self.control_latched = false;
                    continue;
                }
                if let vm::VmEvent::ExecScene { name, arguments } = &event {
                    // Stage workers drop their source bitmaps before scenario execution
                    // resumes. Published planes have a separate native lifetime.
                    if self
                        .registry
                        .as_ref()
                        .is_some_and(|r| r.contains("exec") && !r.contains("cutin"))
                    {
                        // OutputDebugStringA has no effect on VM progress if there is no sink.
                        let _ = h.append_log(
                            self.client,
                            &format!("Scene::Execute( {name}, {arguments} )is called."),
                        );
                    }
                    continue;
                }
                if matches!(event, vm::VmEvent::PlayBgm(_)) {
                    self.bgm_timer.set(now, 0);
                    bgm_ticks = 0;
                }
                if let vm::VmEvent::SetScenarioTimer(value) = event {
                    self.scenario_timer.set(now, value);
                    counter = value;
                    continue;
                }
                if event == vm::VmEvent::PersistGlobals {
                    let values = vm.globals_snapshot();
                    if values != session.persisted {
                        let storage = session.storage.clone();
                        self.writing = Some(worker(move || {
                            globals::write(storage.as_ref(), &values)?;
                            Ok(values)
                        })?);
                    }
                    continue;
                }
                self.view
                    .apply(h, self.client, vm, event)
                    .with_context(|| {
                        format!("minori {}", vm.current_script().unwrap_or("<end>"))
                    })?;
            }
            if self.view.snow_loading() {
                return Ok(VmYield::BudgetExhausted);
            }
            if self.view.shake_blocks() {
                return Ok(VmYield::WaitUntil(now.saturating_add(10)));
            }
            if self.writing.is_some()
                || self.view.audio_ticket.is_some()
                || self.view.foreground_pending() || self.view.animation_loading() || self.view.sequence_pending() || self.view.loading_cutin() || self.view.rotation_pending()
            {
                return Ok(VmYield::BudgetExhausted);
            }
            if let VmState::WaitingTimer { remaining_ms } = vm.state() {
                return Ok(VmYield::WaitUntil(
                    now.saturating_add(u64::from(*remaining_ms)),
                ));
            }
        }
        Ok(VmYield::BudgetExhausted)
    }
}
impl VmAdapter for Adapter {
    fn request_close(&mut self) -> bool {
        self.closing = true;
        self.session.is_some()
    }
    fn engine_id(&self) -> &'static str {
        "minori"
    }
    fn preparation_pending(&self) -> bool {
        self.session.is_none()
    }
    fn diagnostics(&self) -> Vec<u8> {
        let Some(session) = &self.session else {
            return br#"{"state":"loading"}"#.to_vec();
        };
        if let Some(title) = &self.title {
            return serde_json::to_vec(&serde_json::json!({"state":title.name()}))
                .unwrap_or_default();
        }
        serde_json::to_vec(&serde_json::json!({
            "script":session.vm.current_script(), "state":session.vm.state().label(),
            "control_latched":self.control_latched,
        })).unwrap_or_default()
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        let result = self.run(h, input, budget).map(|yielded| {
            if self.view.sound_fading() || self.view.auto_advance_message || self.view.animation_playing(-1) || self.view.sequences_active() || self.view.rotation_active(false) || self.view.rotation_active(true) || self.view.scrolling()
                || self.view.shaking()
                || self.view.snow_active()
                || self.view.character_active()
            {
                let next = h.now_ms().saturating_add(10);
                match yielded {
                    VmYield::WaitForInput => VmYield::WaitUntil(next),
                    VmYield::WaitUntil(at) => VmYield::WaitUntil(at.min(next)),
                    other => other,
                }
            } else {
                yielded
            }
        });
        result.map_err(|e| Error::Io(format!("{e:#}")))
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("minori snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("minori snapshot"))
    }
}
