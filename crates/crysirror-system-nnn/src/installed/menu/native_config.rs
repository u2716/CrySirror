// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One CommonConfig object, retaining its source allocation aliases, Init
//! candidate and asynchronous reload chain. Shared image jobs own all pixels.
use super::*;
use crate::{
    bridge::ConfigUiContext,
    native_config_layout::{
        self as layout,
        allocation::{Allocation, Slot},
        pictures::{Key, Picture, Preparation},
        session::Session,
    },
    native_source_parameters::Policy,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
mod input;
mod print;
pub(in crate::installed) use input::{Effect, Frame};
pub(in crate::installed) use print::ORDER;

struct Load {
    key: Key,
    name: Option<Vec<u8>>,
}
pub(in crate::installed) struct ConfigMenu {
    pub session: Session,
    pub policy: Policy,
    pub allocation: Allocation,
    table: crate::config::Names,
    variables: crate::expression::Variables,
    context: ConfigUiContext,
    back_script: bool,
    original_effects: BTreeMap<Key, Picture>,
    loads: VecDeque<Load>,
    load: Option<Preparation>,
    generation: u64,
    pub ready: bool,
    pub frame: Option<crate::native_config_frame::FrameWork>,
    frame_base: Option<ImageJob>,
    base_capture: Option<ImageJob>,
    draw_commands: Vec<crate::native_config_frame::Draw>,
    late_commands: Vec<crate::native_config_frame::Draw>,
    window_job: Option<ImageJob>,
    window: Option<layout::window::WindowPreview>,
    pub completed: Option<Arc<dyn ImageResource>>,
    pub effects: VecDeque<Effect>,
    updown_clicking: Option<u8>,
    updown_status: Option<usize>,
    character_captured: Vec<bool>,
    pub counter: u64,
    pub widgets_dirty: bool,
    pub published_page: Option<usize>,
}
impl ConfigMenu {
    pub(in crate::installed) fn begin(
        table: &crate::config::Names,
        config: &crate::native_bootstrap::NativeConfiguration,
        variables: &crate::expression::Variables,
        context: ConfigUiContext,
        previous_page: usize,
        back_script: bool,
    ) -> Result<Self> {
        let voices = usize::try_from(config.setup.integer(b"useSystemVoice")?.unwrap_or(0))?;
        let layout = Arc::new(layout::Layout::read(table, config, voices)?);
        ensure!(
            layout.page_mode != 0 || layout.groups.iter().any(|g| matches!(g.action, layout::Action::Back)),
            "NNN Config back control absent"
        );
        let policy = Policy::read(&config.setup)?;
        let session = Session::init(
            layout.clone(),
            table,
            config,
            &policy,
            &context.file,
            &context.voices,
            variables,
            previous_page,
            context.mask,
        )?;
        if let Some(sample) = &session.sample {
            // PrintSampleText needs CGameCallBack's message-speed tables,
            // which this executable profile does not expose yet.
            anyhow::bail!(
                "NNN Config sample text (page {}) requires the native message-speed table",
                sample.page
            );
        }
        let allocation = Allocation::read(&layout, back_script)?;
        let original_effects = context
            .pictures
            .iter()
            .filter(|(key, _)| matches!(key, Key::Allocated(Slot::Effect(_))))
            .map(|(key, picture)| (key.clone(), picture.clone()))
            .collect();
        let window = layout::window::WindowPreview::read(table, context.window_variants)?;
        let mut owner = Self {
            character_captured: vec![false; layout.characters.len()],
            session,
            policy,
            allocation,
            table: table.clone(),
            variables: variables.clone(),
            context,
            back_script,
            original_effects,
            loads: VecDeque::new(),
            load: None,
            generation: 0,
            ready: false,
            frame: None,
            frame_base: None,
            base_capture: None,
            draw_commands: vec![],
            late_commands: vec![],
            window_job: None,
            window,
            completed: None,
            effects: VecDeque::new(),
            updown_clicking: None,
            updown_status: None,
            counter: 0,
            widgets_dirty: true,
            published_page: None,
        };
        owner.set_updown_appearance();
        // CMySlider's CSystemPicture objects are constructor-loaded. They are
        // independent of the current page and not aliases of the button bank.
        let bars = owner
            .session
            .layout
            .sliders
            .sliders
            .iter()
            .filter(|slider| slider.exists)
            .map(|slider| slider.bar.clone())
            .collect::<Vec<_>>();
        for name in bars {
            owner.system_load(Some(&name))?;
        }
        let character_bars = owner
            .session
            .layout
            .characters
            .iter()
            .filter_map(|character| character.slider.as_ref().map(|slider| slider.file.clone()))
            .collect::<Vec<_>>();
        for name in character_bars {
            owner.system_load(name.as_deref())?;
        }
        let variables = owner.variables.clone();
        owner.reload_page(&variables, back_script)?;
        // Init: SetPageNumber for a back-script profile (one-based page).
        if back_script {
            if let Some(variable) = owner.session.layout.page_variable {
                owner
                    .effects
                    .push_back(Effect::Variable(variable, owner.session.page as i32 + 1));
            }
        }
        Ok(owner)
    }
    fn system_load(&mut self, name: Option<&[u8]>) -> Result<()> {
        let (key, logical, _) = layout::pictures::system_request(
            name,
            &self.context.reload.names,
            self.context.reload.levels,
        )?;
        // The source CSystemPicture cache is case-sensitive and returns an
        // existing object without another LoadDWQ, even after a failed load.
        if self.context.pictures.contains_key(&key) || self.loads.iter().any(|load| load.key == key) {
            return Ok(());
        }
        self.context
            .pictures
            .insert(key.clone(), Picture::fresh(self.context.reload.resources.client(), true)?);
        self.loads.push_back(Load {
            key,
            name: Some(logical),
        });
        Ok(())
    }
    fn control_load(
        &mut self,
        group: usize,
        member: usize,
        variables: &crate::expression::Variables,
    ) -> Result<()> {
        let slot = *self
            .allocation
            .controls
            .get(&(group, member))
            .context("NNN Config control picture slot absent")?;
        let key = Key::Allocated(slot);
        if !self.context.pictures.contains_key(&key) {
            self.context
                .pictures
                .insert(key.clone(), Picture::fresh(self.context.reload.resources.client(), true)?);
        }
        let control = &self.session.controls[group];
        let (spec, _) = control.active(member)?;
        let name = if let layout::Action::Character { slot, .. } = control.action {
            let character = &self.session.layout.characters[slot];
            let variant = character
                .filename_variable
                .map(|id| variables.read_stored(id))
                .unwrap_or(0);
            character.native_name(control.value(), variant)?
        } else {
            spec.button
                .filename
                .as_ref()
                .map(|name| [b"sys\\".as_slice(), name].concat())
        };
        self.loads.push_back(Load { key, name });
        Ok(())
    }
    pub(in crate::installed) fn reload_control(
        &mut self,
        group: usize,
        variables: &crate::expression::Variables,
    ) -> Result<()> {
        ensure!(self.load.is_none(), "NNN Config reload overlaps a retained native read");
        for member in 0..self.session.controls[group].len() {
            self.control_load(group, member, variables)?;
        }
        self.ready = false;
        self.widgets_dirty = true;
        Ok(())
    }
    pub(in crate::installed) fn reload_page(
        &mut self,
        variables: &crate::expression::Variables,
        back_script: bool,
    ) -> Result<()> {
        ensure!(self.load.is_none(), "NNN Config page switch during native read");
        if !back_script {
            let key = Key::CommonBackground;
            if !self.context.pictures.contains_key(&key) {
                self.context
                    .pictures
                    .insert(key.clone(), Picture::fresh(self.context.reload.resources.client(), true)?);
            }
            self.loads.push_back(Load {
                key,
                name: Some([b"sys\\".as_slice(), &self.session.layout.pages[self.session.page]].concat()),
            });
        }
        // Init loads Back, then ReloadAllRadioButtonPic (voice/volume/modes/
        // checks/gore/systemVoice/extra radios/checks/clicks), then UpDown/Tab.
        // Different order from constructor allocation is observable when the
        // source deliberately aliases slots across pages.
        let mut order = (0..self.session.controls.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| match self.session.controls[*index].action {
            layout::Action::Back => 0,
            layout::Action::Character { .. } => 1,
            layout::Action::Toggle(200..=209) => 2,
            layout::Action::ScreenMode | layout::Action::Parameter { id: 2 | 6, .. } => 3,
            layout::Action::Toggle(210..=214) => 4,
            layout::Action::Gore => 5,
            layout::Action::SystemVoice(_) => 6,
            layout::Action::Parameter { .. } => 7,
            layout::Action::Toggle(_) => 8,
            layout::Action::Click(_) => 9,
            layout::Action::PageStep(_) | layout::Action::Page(_) => 10,
        });
        for group in order {
            if !self.session.controls[group].visible(self.session.page) {
                continue;
            }
            for member in 0..self.session.controls[group].len() {
                self.control_load(group, member, variables)?;
            }
        }
        self.ready = false;
        self.widgets_dirty = true;
        Ok(())
    }
    pub(in crate::installed) fn poll_resources(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        loop {
            if let Some(work) = self.load.as_mut() {
                let Some((key, picture)) = work.poll(host)? else {
                    return Ok(false);
                };
                self.context.pictures.insert(key, picture);
                self.load = None;
            }
            let Some(load) = self.loads.pop_front() else {
                self.ready = true;
                return Ok(true);
            };
            self.generation = self
                .generation
                .checked_add(1)
                .context("NNN Config load generation exhausted")?;
            let previous = self
                .context
                .pictures
                .get(&load.key)
                .context("NNN Config allocated picture disappeared")?
                .clone();
            self.load = Some(Preparation::begin(
                load.key,
                previous,
                load.name.as_deref(),
                &self.context.reload,
                self.generation,
            )?);
        }
    }
    /// ChangePage after Print: commit the reserved page, reload its pictures and
    /// request the page variable/background restart through the owner.
    pub(in crate::installed) fn commit_page(&mut self) -> Result<bool> {
        if self.session.page_pending.is_none() {
            return Ok(false);
        }
        self.session.commit_page(self.back_script)?;
        self.updown_clicking = None;
        self.updown_status = None;
        self.set_updown_appearance();
        let variables = self.variables.clone();
        self.reload_page(&variables, self.back_script)?;
        for (variable, value) in std::mem::take(&mut self.session.variables) {
            self.effects.push_back(Effect::Variable(variable, value));
        }
        if std::mem::take(&mut self.session.background_restarts) {
            self.effects.push_back(Effect::RestartBackground);
        }
        Ok(true)
    }
    pub(in crate::installed) fn picture(&self, group: usize, member: usize) -> Result<&Picture> {
        let slot = *self
            .allocation
            .controls
            .get(&(group, member))
            .context("NNN Config picture identity absent")?;
        self.context
            .pictures
            .get(&Key::Allocated(slot))
            .context("NNN Config native picture object absent")
    }
    pub(in crate::installed) fn system_picture(&self, name: Option<&[u8]>) -> Option<&Picture> {
        self.context.pictures.get(&Key::System(name.unwrap_or(b"").to_vec()))
    }
    pub(in crate::installed) fn background(&self) -> Option<&Picture> {
        self.context.pictures.get(&Key::CommonBackground)
    }
    /// Pictures to hand back to the Bridge at FinalExitRoutine. Effect aliases
    /// are restored to their original (FuqueAllEffect) objects; a back-script
    /// profile never allocated Effect slots, so it returns none of them.
    pub(in crate::installed) fn restore_pictures(&self) -> BTreeMap<Key, Picture> {
        let mut restored = self.context.pictures.clone();
        restored.retain(|key, _| !matches!(key, Key::Allocated(Slot::Effect(_))));
        if !self.back_script {
            restored.extend(self.original_effects.clone());
        }
        restored
    }
    pub(in crate::installed) fn update_file(&mut self, file: NativeSystemFile) {
        self.context.file = file;
    }
    pub(in crate::installed) fn update_variables(&mut self, variables: crate::expression::Variables) {
        self.variables = variables;
    }
    pub(in crate::installed) fn file(&self) -> &NativeSystemFile {
        &self.context.file
    }
    pub(in crate::installed) fn print_in_flight(&self) -> bool {
        self.frame.is_some() || self.frame_base.is_some() || self.base_capture.is_some() || self.window_job.is_some()
    }
    pub(in crate::installed) fn window_preview(&self) -> Option<&layout::window::WindowPreview> {
        self.window
            .as_ref()
            .filter(|w| w.page as i64 == self.session.page as i64 + 1 && !self.session.entry.sample(self.session.layout.entry_wait, self.session.layout.appearance_frames).0)
    }
    pub(in crate::installed) fn context_faces(&self) -> Option<&(crate::native_text::TextFace, crate::native_text::TextFace)> {
        self.context.faces.as_ref()
    }
    pub(in crate::installed) fn back_script(&self) -> bool {
        self.back_script
    }
    pub(in crate::installed) fn out_variable(&self) -> Option<(u16, i32)> {
        self.session.layout.out_variable
    }
    pub(in crate::installed) fn in_variable(layout_table: &crate::config::Names, config: &crate::native_bootstrap::NativeConfiguration) -> Result<Option<(u16, i32)>> {
        Ok(layout_table
            .parameter(b"inSetVarName")
            .and_then(|name| config.variable_names.index(name))
            .map(|id| -> Result<(u16, i32)> { Ok((id, layout_table.integer(b"inSetVarData")?.unwrap_or(0))) })
            .transpose()?)
    }
}
