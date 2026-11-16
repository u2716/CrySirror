// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! General's optional mini-game menu overlay. Area data/variables stay native;
//! shared UI owns pointer admission, and ordinary shared sprite jobs paint the
//! button-group profile. A configured area alone never starts a mini-game.
use super::*;
#[derive(Clone, Debug)]
pub(super) struct Setup {
    pub paths: [String; 2],
    pub configuration: Arc<NativeConfiguration>,
}
#[derive(Clone, Debug)]
pub(super) struct Area {
    pub mode: i32,
    pub variable: Option<u16>,
    pub rect: [i32; 4],
    pub game_type: Option<i32>,
}
pub(super) struct Overlay {
    pub button_mode: bool,
    pub wait_ready: bool,
    pub buttons_initialized: bool,
    pub areas: Vec<Area>,
    table: Names,
    configuration: Arc<NativeConfiguration>,
    /// Appended menu-button index -> original mini-game catalogue index.
    pub buttons: std::collections::BTreeMap<usize, usize>,
    pub area_widgets: std::collections::BTreeSet<u32>,
}
impl Setup {
    pub(super) fn read(
        &self,
        open: &crate::resource_bridge::SourceOpener,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<Overlay> {
        let mut table = Names { tokens: vec![] };
        for (i, path) in self.paths.iter().enumerate() {
            cancel.check()?;
            if let Some(source) = open(path, cancel)? {
                table = Names::parse(
                    &crysirror_resource::read_all(source.as_ref(), 1 << 20)?,
                    i == 0,
                )?;
                break;
            }
        }
        // Explicit zero overrides addMiniGame; only an absent key inherits.
        let count = table
            .integer(b"miniGameCheckNumber")?
            .unwrap_or(table.integer(b"addMiniGame")?.unwrap_or(0));
        ensure!((0..=64).contains(&count), "NNN mini-game menu area budget");
        let add = table.integer(b"addMiniGame")?.unwrap_or(0);
        ensure!(
            (0..=64).contains(&add),
            "NNN mini-game actor catalogue budget"
        );
        let mut game_types = Vec::new();
        for row in 1..=add {
            if let Some(kind) = table
                .integer(format!("miniGame{row}").as_bytes())?
                .filter(|kind| *kind > 0)
            {
                game_types.push(kind);
            }
        }
        let mut mode = 15;
        let mut rect = [0, 0, 16, 16];
        let mut areas = vec![];
        for i in 1..=count {
            if let Some(name) = table.parameter(format!("miniGameCheckMode{i}").as_bytes()) {
                // GetModeNumberBySetup changes the inherited output only for
                // a positive lookup (including decimal-prefix mode names).
                let next = crate::native_mode::number(name)?;
                if next > 0 {
                    mode = next;
                }
            }
            for (axis, key) in ["X", "Y", "SizeX", "SizeY"].iter().enumerate() {
                rect[axis] = table
                    .integer(format!("miniGameCheck{key}{i}").as_bytes())?
                    .unwrap_or(rect[axis]);
            }
            ensure!(
                rect.iter().all(|n| n.unsigned_abs() <= 1_000_000),
                "NNN mini-game area geometry"
            );
            let variable = table
                .parameter(format!("checkVarName{i}").as_bytes())
                .filter(|name| !name.is_empty())
                .and_then(|name| self.configuration.variable_names.index(name));
            areas.push(Area {
                mode,
                variable,
                rect,
                game_type: game_types.get((i - 1) as usize).copied(),
            });
        }
        Ok(Overlay {
            button_mode: table.integer(b"areaButtonMode")?.unwrap_or(0) != 0,
            wait_ready: false,
            buttons_initialized: false,
            areas,
            table,
            configuration: self.configuration.clone(),
            buttons: Default::default(),
            area_widgets: Default::default(),
        })
    }
}
impl Overlay {
    fn active(&self, index: usize, mode: i32, variables: &crate::expression::Variables) -> bool {
        let area = &self.areas[index];
        area.mode == mode && area.variable.is_none_or(|v| variables.read_stored(v) != 0)
    }
    /// Button mode samples its existence flags at InitMiniGameButton; area
    /// mode instead samples variables on every GetOnMiniGame call below.
    pub(super) fn initialize_buttons(
        &mut self,
        base: usize,
        mode: i32,
        variables: &crate::expression::Variables,
    ) -> Result<Vec<NativeButtonSpec>> {
        self.buttons.clear();
        self.buttons_initialized = true;
        if !self.button_mode {
            return Ok(vec![]);
        }
        let mut result = vec![];
        let mut previous = None;
        for i in 0..self.areas.len() {
            let spec = read_menu_button(
                &self.table,
                &format!("minigameButton{}", i + 1),
                &self.configuration,
                previous.as_ref(),
                [0, 0],
            )?;
            previous = Some(spec.clone());
            if self.active(i, mode, variables) {
                let mut spec = spec;
                spec.exists = true;
                spec.enabled = true;
                self.buttons.insert(base + result.len(), i);
                result.push(spec);
            }
        }
        Ok(result)
    }
    pub(super) fn area_commands(
        &mut self,
        base: u32,
        parent: Option<u32>,
        mode: i32,
        variables: &crate::expression::Variables,
        enabled: bool,
    ) -> Vec<serde_json::Value> {
        if self.button_mode {
            return vec![];
        }
        let mut commands = vec![];
        for (i, area) in self.areas.iter().enumerate() {
            if area.mode != mode || area.rect[2] <= 0 || area.rect[3] <= 0 {
                continue;
            }
            let id = base + 128 + i as u32;
            self.area_widgets.insert(id);
            let active = enabled && self.active(i, mode, variables);
            let mut widget = serde_json::json!({"id":id,"rect":area.rect,"kind":"button","interaction_only":true,"keyboard_focusable":false,"activate_on_press":true,"z":128+i,"visible":active,"enabled":active});
            if let Some(parent) = parent {
                widget["parent"] = serde_json::json!(parent);
            }
            commands.push(serde_json::json!({"op":"put","widget":widget}));
        }
        commands
    }
    pub(super) fn check_area_click(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        base: u32,
        mode: i32,
        variables: &crate::expression::Variables,
        left_click: bool,
    ) -> Result<()> {
        if self.button_mode || !left_click {
            return Ok(());
        }
        // Original GetOnMiniGame takes the first eligible half-open area.
        // Shared UI owns geometry/modal pointer admission; no VM hit loop.
        for i in 0..self.areas.len() {
            if !self.active(i, mode, variables) {
                continue;
            }
            let id = base + 128 + i as u32;
            if !self.area_widgets.contains(&id) {
                continue;
            }
            let state: serde_json::Value = serde_json::from_slice(
                &host.query_engine(client, EngineQuery::UiInteraction(id))?,
            )?;
            ensure!(
                state["hovered"] != true,
                "NNN mini-game row {} source type {:?} (native type {:?}) selected from mode {mode} is not implemented",
                i + 1,
                self.areas[i].game_type,
                self.areas[i].game_type.map(|kind| kind - 1)
            );
        }
        Ok(())
    }
}
impl NativeTitlePlayer {
    pub(super) fn prepare_mini_game(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<bool> {
        if let Some(task) = self.mini_game_task.as_mut() {
            let Some(overlay) = task.try_take()? else {
                return Ok(false);
            };
            self.mini_game_task = None;
            self.mini_game = Some(overlay);
        }
        if self.mini_game_initialized {
            return Ok(true);
        }
        let Some(overlay) = self.mini_game.as_mut() else {
            self.mini_game_initialized = true;
            return Ok(true);
        };
        if overlay.button_mode
            && !overlay.wait_ready
            && self.spec.menu_wait.saturating_add(self.spec.menu_fade) > 0
        {
            // GeneralInit disables the mini group. Its existence variables
            // are sampled at EndStartWaitMode, not while loading normal art.
            self.mini_game_initialized = true;
            return Ok(true);
        }
        // Re-entering Title/Omake calls InitMiniGameButton again. Retire only
        // overlay controls actually created by this owner, not ordinary ones.
        if self.widgets {
            let ids = (self.base_buttons..self.spec.buttons.len())
                .map(|i| self.widget_base + i as u32)
                .chain(overlay.area_widgets.iter().copied());
            let commands = ids
                .map(|id| serde_json::json!({"op":"remove","id":id}))
                .collect::<Vec<_>>();
            if !commands.is_empty() {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&serde_json::json!({"ui":commands}))?,
                )?;
            }
        }
        overlay.area_widgets.clear();
        self.widgets = false;
        self.spec.buttons.truncate(self.base_buttons);
        self.images.truncate(self.base_buttons);
        self.states.truncate(self.base_buttons);
        let variables = vm.gallery_variables()?;
        let buttons =
            overlay.initialize_buttons(self.base_buttons, self.general_mode, &variables)?;
        for spec in buttons {
            let lookup = spec.source.clone().context("NNN mini-game button without filename requires its original picture-slot owner")?;
            let resources = self.resources.clone();
            let task = self
                .pool
                .spawn(move |cancel| resources.image_source(&lookup, &cancel))?;
            self.states.push(NativeButtonState::new(&spec));
            self.spec.buttons.push(spec);
            self.images.push(TitleImage {
                source_task: Some(task),
                job: None,
                image: None,
                mask: None,
            });
        }
        self.mini_game_initialized = true;
        self.prepared = false;
        // No input collected for the previous mode may become a mini-game
        // activation after asynchronous source preparation.
        self.pressed_buttons = 0;
        self.activated.clear();
        Ok(true)
    }
    pub(super) fn admit_mini_game_wait_end(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
        counter: u32,
    ) -> Result<()> {
        let end = self.spec.menu_wait.saturating_add(self.spec.menu_fade);
        if let Some(overlay) = self.mini_game.as_mut() {
            if overlay.button_mode && !overlay.buttons_initialized && counter >= end {
                overlay.wait_ready = true;
                self.mini_game_initialized = false;
                self.prepare_mini_game(host, vm)?;
            }
        }
        Ok(())
    }
    /// This extends an already admitted native Print. Asset completion cannot
    /// Calcu the group again or skip the first overlay animation cell.
    pub(super) fn schedule_mini_game_print(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Some(mut frame) = self.inflight.take() else {
            return Ok(());
        };
        if frame.mini_scheduled {
            self.inflight = Some(frame);
            return Ok(());
        }
        if self.capture_only_for_entry()
            || frame.counter < self.spec.menu_wait.saturating_add(self.spec.menu_fade)
        {
            frame.mini_scheduled = true;
            self.inflight = Some(frame);
            return Ok(());
        }
        if let Some(overlay) = self.mini_game.as_ref() {
            for &index in overlay.buttons.keys() {
                while frame.states.len() <= index {
                    frame.states.push(self.states[frame.states.len()].clone());
                }
                let state = &mut frame.states[index];
                let input =
                    self.button_input(index, frame.background.clone(), state, frame.counter, None)?;
                let effects = if matches!(input, ImageInput::Ternary { .. }) {
                    let (frames, percent) = state.frame(&self.spec.buttons[index]);
                    let image = self.images[index]
                        .image
                        .as_ref()
                        .context("NNN mini-game sprite not prepared")?;
                    vec![ImageEffect::MaskedPairBlend {
                        spec: MaskedPairBlend {
                            sources: [
                                self.spec.buttons[index].source_rect(
                                    state.mode,
                                    frames[0],
                                    image.size(),
                                )?,
                                self.spec.buttons[index].source_rect(
                                    state.mode,
                                    frames[1],
                                    image.size(),
                                )?,
                            ],
                            destination: [0, 0],
                            weights256: [percent * 256 / 100, (100 - percent) * 256 / 100],
                            clip: None,
                        },
                    }]
                } else {
                    vec![]
                };
                frame.jobs.push((
                    index,
                    host.image_metadata(self.client)?
                        .begin_process(input, serde_json::to_vec(&effects)?.into())?,
                ));
                state.printed(&self.spec.buttons[index]);
            }
        }
        frame.mini_scheduled = true;
        self.inflight = Some(frame);
        Ok(())
    }
    pub(super) fn mini_game_widget_ids(&self) -> Vec<u32> {
        self.mini_game
            .as_ref()
            .map(|m| m.area_widgets.iter().copied().collect())
            .unwrap_or_default()
    }
    pub(super) fn update_mini_game_areas(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
        enabled: bool,
    ) -> Result<()> {
        if let Some(overlay) = self.mini_game.as_mut() {
            let variables = vm.gallery_variables()?;
            let commands = overlay.area_commands(
                self.widget_base,
                self.widget_parent,
                self.general_mode,
                &variables,
                enabled,
            );
            if !commands.is_empty() {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&serde_json::json!({"ui":commands}))?,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn calcu_mini_game(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
        states: &mut [NativeButtonState],
        enabled: bool,
    ) -> Result<Option<crate::bridge::ButtonAudioEvent>> {
        let Some(overlay) = self.mini_game.as_ref() else {
            return Ok(None);
        };
        if !enabled {
            return Ok(None);
        }
        let variables = vm.gallery_variables()?;
        overlay.check_area_click(
            host,
            self.client,
            self.widget_base,
            self.general_mode,
            &variables,
            self.pressed_buttons & crysirror_core::service::buttons::MOUSE_LEFT != 0,
        )?;
        let mut sound = None;
        let mut locked = None;
        for (&index, &original) in &overlay.buttons {
            if locked.is_some_and(|selected| selected != index) {
                states[index].calcu_without_input(&self.spec.buttons[index]);
                continue;
            }
            let interaction: serde_json::Value = serde_json::from_slice(&host.query_engine(
                self.client,
                EngineQuery::UiInteraction(self.widget_base + index as u32),
            )?)?;
            let before = states[index].mode;
            let signal = states[index].calcu(
                &self.spec.buttons[index],
                interaction["hovered"] == true,
                self.activated.remove(&index),
            );
            ensure!(
                signal != NativeButtonSignal::Selected,
                "NNN mini-game row {} source type {:?} (native type {:?}) selected from mode {} is not implemented",
                original + 1,
                overlay.areas[original].game_type,
                overlay.areas[original].game_type.map(|kind| kind - 1),
                self.general_mode
            );
            let part = match signal {
                NativeButtonSignal::ClickStarted => {
                    locked = Some(index);
                    Some(0)
                }
                NativeButtonSignal::Entered if self.spec.buttons[index].sound_waits[0] == 0 => {
                    Some(2)
                }
                NativeButtonSignal::Exited if self.spec.buttons[index].sound_waits[1] == 0 => {
                    Some(3)
                }
                NativeButtonSignal::Sound(_) => Some(match states[index].mode {
                    3 => 1,
                    1 => 2,
                    _ => 3,
                }),
                _ => None,
            };
            if let Some(part) = part {
                let spec = &self.spec.buttons[index];
                let n = spec.sounds[part];
                sound = Some(crate::bridge::ButtonAudioEvent::new(
                    if n > 0 { n & 127 } else { 0 },
                    0,
                    0,
                ));
            }
            if before == 2 {
                locked = Some(index);
            }
        }
        Ok(sound)
    }
}
