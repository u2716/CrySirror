// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Persistent CommonConfig object state, distinct from its disposable UI/image
//! publications. Init performs no hit tests, drawing or ambient filesystem I/O.
use super::{Action, Layout, controls::Control, sample::Sample};
use crate::{
    config::Names, expression::Variables, native_config_slider::SliderOwner,
    native_source_parameters::Policy, native_system_file::NativeSystemFile,
    native_voice_policy::VoicePolicy,
};
use anyhow::{Result, ensure};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Session {
    pub layout: Arc<Layout>,
    pub page: usize,
    pub controls: Vec<Control>,
    pub sliders: Vec<SliderOwner>,
    pub sample: Option<Sample>,
    pub entry: super::appearance::EntryClock,
    pub page_pending: Option<usize>,
    pub background_restarts: bool,
    pub variables: Vec<(u16, i32)>,
    // Only Back/UpDown/Tab start-clicks set the source m_mode lock. Ordinary
    // radio/check starts do not (those assignments are commented out).
    pub locked_navigation: Option<usize>,
}
impl Session {
    pub(crate) fn init(
        layout: Arc<Layout>,
        table: &Names,
        configuration: &crate::native_bootstrap::NativeConfiguration,
        policy: &Policy,
        file: &NativeSystemFile,
        voices: &VoicePolicy,
        variables: &Variables,
        previous_page: usize,
        mask: i32,
    ) -> Result<Self> {
        let page = if layout.force_first_page {
            0
        } else {
            previous_page
        };
        ensure!(page < layout.pages.len(), "NNN Config retained page bounds");
        let system_voice = if file.source_version2() {
            file.flag2_word(6)?
        } else {
            0
        };
        let mut controls = Vec::new();
        for group in &layout.groups {
            let value = match group.action {
                Action::Parameter { id, .. } | Action::Toggle(id) => policy.get(file, id)?,
                Action::ScreenMode => policy.get(file, 4)?,
                Action::Gore => {
                    if file.source_version2() {
                        file.flag2_word(9)?
                    } else {
                        0
                    }
                }
                Action::Character { slot, .. } => {
                    i32::from(crate::native_character_setting::voice_enabled(file, slot)?)
                }
                Action::SystemVoice(_) => system_voice,
                Action::Page(_) => page as i32,
                _ => 0,
            };
            let mut control = Control::new(group, value)?;
            match group.action {
                Action::Parameter { id: 300..=307, .. } => {
                    let Action::Parameter { id, .. } = group.action else {
                        unreachable!()
                    };
                    control.set_enabled(mask & (1 << (id - 300)) == 0)?;
                }
                Action::Toggle(id @ 400..=407) => {
                    control.set_enabled(mask & (1 << (id - 400 + 8)) == 0)?
                }
                Action::SystemVoice(_) => {
                    for index in 1..control.len() {
                        let character = super::n(table, &format!("systemVoice{index}On"), 0)?;
                        // Source CheckPlayerVoice is CheckVoice's USED flag
                        // in VERSION2 and constant TRUE without VERSION2.
                        let mut allowed = character == 0
                            || character > 0
                                && (!file.source_version2() || file.voice(character)?);
                        if let Some(id) = super::variable(
                            table,
                            configuration,
                            &format!("systemVoice{index}OnVarName"),
                        ) {
                            allowed |= variables.read_stored(id) != 0;
                        }
                        control.set_radio_flags(index, allowed, true)?;
                    }
                    control.select(usize::try_from(system_voice)?)?;
                }
                Action::Character { slot, .. } => {
                    let character = &layout.characters[slot];
                    control
                        .set_exists(!character.only_used || voices.button_voice_used(file, slot)?);
                    let image_variant = character
                        .filename_variable
                        .map(|id| variables.read_stored(id))
                        .unwrap_or(0);
                    let alternate = character
                        .sound_variable
                        .is_some_and(|id| variables.read_stored(id) > 0);
                    for state in 0..2 {
                        control.set_check_assets(
                            state,
                            character.image(state, image_variant)?,
                            character.sounds(state, alternate)?,
                        )?;
                    }
                }
                _ => (),
            }
            controls.push(control);
        }
        let sliders = layout
            .sliders
            .sliders
            .iter()
            .enumerate()
            .map(|(index, _)| {
                SliderOwner::new(
                    &layout.sliders,
                    index,
                    policy.get(file, crate::native_config_volume::PARAMETERS[index])?,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        // Before calling Init, the owner must complete layout.in_variable's
        // native SetVarData (including its configured write side effects).
        // Conditions above use that resulting snapshot; no deferred duplicate
        // assignment may be replayed after resources finish preparing.
        let changes = Vec::new();
        // SetPageNumber is called only by a back-script profile. The owner
        // appends that assignment when it actually installs the source reader.
        Ok(Self {
            layout,
            page,
            controls,
            sliders,
            sample: Sample::read(table)?,
            entry: Default::default(),
            page_pending: None,
            background_restarts: false,
            variables: changes,
            locked_navigation: None,
        })
    }
    pub(crate) fn request_page(&mut self, page: usize) -> Result<()> {
        ensure!(
            page < self.layout.pages.len(),
            "NNN Config page request bounds"
        );
        self.page_pending = Some(page);
        Ok(())
    }
    /// Source Print applies m_pageYoyaku AFTER finishing the old page. Its
    /// owner reloads new images before invoking this commit, without running
    /// another Calcu/Print merely to complete resource work.
    pub(crate) fn commit_page(&mut self, back_script: bool) -> Result<()> {
        let Some(page) = self.page_pending.take() else {
            return Ok(());
        };
        self.page = page;
        for control in &mut self.controls {
            match control.action {
                Action::Page(_) => control.select(page)?,
                Action::Back | Action::PageStep(_) => control.select(0)?,
                Action::Character { .. } => {
                    let selected = control.value();
                    control.select(selected)?;
                }
                _ => (),
            }
        }
        // ChangePage does not Init any CMySlider: retain its source drag
        // state and raw gaze when leaving and returning to a page.
        self.locked_navigation = None;
        if back_script {
            if let Some(variable) = self.layout.page_variable {
                // SetPageNumber writes the one-based page.
                self.variables.push((variable, page as i32 + 1));
            }
            self.background_restarts = self.layout.restart_script_on_page;
        }
        Ok(())
    }
    pub(crate) fn exit_variables(&self) -> Vec<(u16, i32)> {
        self.layout.out_variable.into_iter().collect()
    }
}
