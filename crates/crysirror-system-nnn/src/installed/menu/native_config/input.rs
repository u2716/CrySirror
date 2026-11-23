// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonConfig::Calcu over scoped engine-UI results. Hit testing, focus and
//! pointer capture stay in shared UI; this owner keeps the original control
//! order, m_mode lock, ProcessButtonGroup audio and SetSystemParam effects.
use super::*;
use crate::{
    native_config_layout::controls::Signal,
    native_config_volume::{PARAMETERS, SWITCHES},
    native_radio_group::ButtonInput,
};
use std::collections::BTreeSet;
/// Native requests produced by one admitted Calcu. The owner applies them in
/// order before the next Print, so Print reads the committed SystemFile.
#[derive(Clone, Debug)]
pub(in crate::installed) enum Effect {
    /// SetSystemParam(id, value), with the original side effects.
    Parameter(i32, i32),
    /// SetVoiceFlag(character, flag).
    Voice { slot: usize, flag: bool },
    /// SetUseSystemVoice + ReLoadSystemSound.
    SystemVoice(i32),
    /// SetGoreFlag + SetGoreLayer.
    Gore(i32),
    /// PlaySystemSound/PlayButtonVoice from ProcessButtonGroup.
    Audio(crate::bridge::ButtonAudioEvent),
    /// PlayExtSe2 for a released slider.
    Preview(crate::native_config_volume::Preview),
    /// SetVarData requested by SetPageNumber/inSetVar.
    Variable(u16, i32),
    /// RestartBackScript after ChangePage.
    RestartBackground,
    /// MyReturnRoutine.
    Return,
}
pub(in crate::installed) struct Frame<'a> {
    pub input: InputFrame,
    pub activated: &'a BTreeSet<u32>,
    pub ticket: u64,
}
impl ConfigMenu {
    fn inputs(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        group: usize,
        frame: &Frame,
    ) -> Result<Vec<ButtonInput>> {
        let control = &self.session.controls[group];
        let mut out = Vec::with_capacity(control.len());
        let pressed = frame.input.buttons_pressed;
        for member in 0..control.len() {
            let (spec, _) = control.active(member)?;
            let id = Self::button_id(group, member)?;
            let interaction: Value =
                serde_json::from_slice(&host.query_engine(client, EngineQuery::UiInteraction(id))?)?;
            let up = pressed & buttons::WHEEL_UP != 0;
            let down = pressed & buttons::WHEEL_DOWN != 0;
            let wheel = if spec.wheel_reverse { [down, up] } else { [up, down] };
            out.push(ButtonInput {
                hovered: interaction["hovered"] == true,
                pressed: frame.activated.contains(&id)
                    || spec.cancel && pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0
                    || spec.wheel.iter().zip(wheel).any(|(flag, trig)| *flag && trig),
            });
        }
        Ok(out)
    }
    fn calcu_group(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        group: usize,
        frame: &Frame,
        admitted: bool,
    ) -> Result<Option<Signal>> {
        let inputs = if admitted {
            Some(self.inputs(host, client, group, frame)?)
        } else {
            None
        };
        self.session.controls[group].calcu(inputs.as_deref())
    }
    /// CCommonGeneral::ProcessButtonGroup/ProcessCommonButton/ProcessUpDownBack.
    fn process(&mut self, signal: Option<Signal>) -> Option<usize> {
        let signal = signal?;
        if signal.sound > 0 || signal.voice > 0 {
            self.effects.push_back(Effect::Audio(crate::bridge::ButtonAudioEvent::new(
                signal.sound,
                signal.voice,
                signal.volume,
            )));
        }
        signal.completes_click().then_some(signal.selected).flatten()
    }
    fn groups(&self, wanted: impl Fn(layout::Action) -> bool) -> Vec<usize> {
        (0..self.session.controls.len())
            .filter(|group| wanted(self.session.controls[*group].action))
            .collect()
    }
    fn visible(&self, group: usize) -> bool {
        self.session.controls[group].visible(self.session.page)
    }
    pub(super) fn set_updown_appearance(&mut self) {
        let pages = self.session.layout.pages.len();
        let page = self.session.page;
        for control in &mut self.session.controls {
            if let layout::Action::PageStep(step) = control.action {
                // CCommonUpDownButtonGroup::CalcuAppear.
                control.set_exists(if step < 0 {
                    page > 0 && pages > 1
                } else {
                    page + 1 < pages && pages > 1
                });
            }
        }
    }
    fn step_groups(&self) -> [Option<usize>; 2] {
        [
            self.groups(|a| matches!(a, layout::Action::PageStep(s) if s < 0)).first().copied(),
            self.groups(|a| matches!(a, layout::Action::PageStep(s) if s > 0)).first().copied(),
        ]
    }
    /// CCommonUpDownButtonGroup::ChangePage + AdjustPage + button Init.
    fn updown_change_page(&mut self, button: usize) -> Result<usize> {
        let pages = self.session.layout.pages.len();
        let page = if button == 0 {
            self.session.page.saturating_sub(1)
        } else {
            (self.session.page + 1).min(pages - 1)
        };
        self.updown_status = None;
        for group in self.step_groups().into_iter().flatten() {
            self.session.controls[group].select(0)?;
        }
        Ok(page)
    }
    /// CCommonUpDownButtonGroup::CalcuSuper. Some(page) only for NUMBER.
    fn updown_group(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        frame: &Frame,
        admitted: bool,
    ) -> Result<Option<(Signal, Option<usize>)>> {
        let steps = self.step_groups();
        if let Some(active) = self.updown_status {
            let mut result = None;
            for (index, group) in steps.iter().enumerate() {
                let Some(group) = *group else { continue };
                if index != active {
                    self.calcu_group(host, client, group, frame, false)?;
                    continue;
                }
                if let Some(signal) = self.calcu_group(host, client, group, frame, admitted)? {
                    let page = if signal.completes_click() {
                        Some(self.updown_change_page(index)?)
                    } else {
                        None
                    };
                    result = Some((signal, page));
                }
            }
            return Ok(result);
        }
        let mut result = None;
        for (index, group) in steps.iter().enumerate() {
            let Some(group) = *group else { continue };
            // CalcuAppear: a non-appearing button is not calculated at all.
            if !self.visible(group) {
                continue;
            }
            if self.updown_status.is_some() {
                self.calcu_group(host, client, group, frame, false)?;
                continue;
            }
            let Some(signal) = self.calcu_group(host, client, group, frame, admitted)? else {
                continue;
            };
            if signal.starts_click() {
                self.updown_status = Some(index);
                result = Some((signal, None));
            } else if signal.completes_click() {
                let page = self.updown_change_page(index)?;
                result = Some((signal, Some(page)));
            } else {
                result = Some((signal, None));
            }
        }
        Ok(result)
    }
    /// CCommonUpdownBack::Calcu, returning ProcessUpDownBack's number:
    /// 0 for Back, otherwise new page + 1.
    fn updown_back(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        frame: &Frame,
    ) -> Result<(Option<Signal>, Option<usize>)> {
        let back = self.groups(|a| matches!(a, layout::Action::Back))[0];
        let back_input = matches!(self.updown_clicking, None | Some(0));
        let mut rt = self.calcu_group(host, client, back, frame, back_input)?;
        if let Some(signal) = rt {
            if signal.completes_click() {
                return Ok((Some(signal), Some(0)));
            }
            if signal.starts_click() {
                self.updown_clicking = Some(0);
            }
        }
        match self.updown_clicking {
            None => {
                if let Some((signal, page)) = self.updown_group(host, client, frame, true)? {
                    if let Some(page) = page {
                        self.updown_clicking = None;
                        return Ok((Some(signal), Some(page + 1)));
                    }
                    if signal.starts_click() {
                        rt = Some(signal);
                        self.updown_clicking = self.updown_status.map(|index| index as u8 + 1);
                    }
                }
            }
            Some(1 | 2) => {
                let result = self.updown_group(host, client, frame, true)?;
                if let Some((signal, Some(page))) = result {
                    self.updown_clicking = None;
                    return Ok((Some(signal), Some(page + 1)));
                }
                rt = result.map(|(signal, _)| signal);
            }
            _ => {
                self.updown_group(host, client, frame, false)?;
            }
        }
        Ok((rt, None))
    }
    /// One admitted CCommonConfig::Calcu.
    pub(in crate::installed) fn calcu(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        frame: &Frame,
    ) -> Result<()> {
        let layout = self.session.layout.clone();
        let skip = frame.input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT) != 0;
        if self
            .session
            .entry
            .advance(layout.entry_wait, layout.appearance_frames, skip)
        {
            return Ok(());
        }
        let locked = self.session.locked_navigation.is_some();
        match layout.page_mode {
            0 | 2 => {
                let back = self.groups(|a| matches!(a, layout::Action::Back))[0];
                let tab = self.groups(|a| matches!(a, layout::Action::Page(_))).first().copied();
                let tab_locked = self.session.locked_navigation.is_some_and(|g| Some(g) == tab);
                let back_signal = self.calcu_group(host, client, back, frame, !tab_locked)?;
                let started = back_signal.is_some_and(|s| s.starts_click());
                if self.process(back_signal).is_some() {
                    self.effects.push_back(Effect::Return);
                    return Ok(());
                }
                if !locked && started {
                    self.session.locked_navigation = Some(back);
                    return Ok(());
                }
                if let Some(tab) = tab {
                    let back_locked = self.session.locked_navigation == Some(back);
                    let signal = self.calcu_group(host, client, tab, frame, !back_locked)?;
                    if let Some(page) = self.process(signal) {
                        self.session.request_page(page)?;
                        return Ok(());
                    }
                }
            }
            1 => {
                let (signal, number) = self.updown_back(host, client, frame)?;
                let started = signal.is_some_and(|s| s.starts_click());
                self.process(signal);
                match number {
                    Some(0) => {
                        self.effects.push_back(Effect::Return);
                        return Ok(());
                    }
                    Some(page) => {
                        self.session.request_page(page - 1)?;
                        return Ok(());
                    }
                    None => (),
                }
                if !locked && started {
                    let back = self.groups(|a| matches!(a, layout::Action::Back))[0];
                    self.session.locked_navigation = Some(back);
                    return Ok(());
                }
            }
            _ => anyhow::bail!("NNN Config page selector"),
        }
        let locked = self.session.locked_navigation.is_some();
        // Per-character voice buttons and optional character sliders.
        for group in self.groups(|a| matches!(a, layout::Action::Character { .. })) {
            if !self.visible(group) {
                continue;
            }
            let layout::Action::Character { slot, .. } = self.session.controls[group].action else {
                unreachable!()
            };
            if let Some(slider) = layout.characters[slot].slider.clone() {
                let off = self.session.controls[group].value() == 0;
                if !(locked || off) {
                    self.character_slider(host, client, group, slot, &slider, frame)?;
                } else {
                    self.character_captured[slot] = false;
                }
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                self.effects.push_back(Effect::Voice { slot, flag: number == 0 });
                self.session.controls[group].select(1 - number.min(1))?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        // Screen/skip/auto radios, then autoContinue..continueVoice checks.
        for group in self.groups(|a| {
            matches!(a, layout::Action::ScreenMode | layout::Action::Parameter { id: 2 | 6, .. })
        }) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                let id = match self.session.controls[group].action {
                    layout::Action::ScreenMode => 4,
                    layout::Action::Parameter { id, .. } => id,
                    _ => unreachable!(),
                };
                self.effects.push_back(Effect::Parameter(id, number as i32));
                self.session.controls[group].select(number)?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        let toggle = |range: std::ops::RangeInclusive<i32>| {
            move |a: layout::Action| matches!(a, layout::Action::Toggle(id) if range.contains(&id))
        };
        for group in self.groups(toggle(210..=214)) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                let layout::Action::Toggle(id) = self.session.controls[group].action else {
                    unreachable!()
                };
                self.effects.push_back(Effect::Parameter(id, 1 - number as i32));
                self.session.controls[group].select(1 - number.min(1))?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        for group in self.groups(|a| matches!(a, layout::Action::Gore)) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                self.effects.push_back(Effect::Gore(1 - number as i32));
                self.session.controls[group].select(1 - number.min(1))?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        for group in self.groups(|a| matches!(a, layout::Action::Parameter { id: 300..=315, .. })) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                let layout::Action::Parameter { id, .. } = self.session.controls[group].action else {
                    unreachable!()
                };
                self.effects.push_back(Effect::Parameter(id, number as i32));
                self.session.controls[group].select(number)?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        for group in self.groups(toggle(400..=415)) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if let Some(number) = self.process(signal) {
                let layout::Action::Toggle(id) = self.session.controls[group].action else {
                    unreachable!()
                };
                self.effects.push_back(Effect::Parameter(id, 1 - number as i32));
                self.session.controls[group].select(1 - number.min(1))?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        for group in self.groups(|a| matches!(a, layout::Action::SystemVoice(_))) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            // ProcessButtonGroupGetNumberOnly precedes the audio request.
            if let Some(number) = signal.filter(|s| s.completes_click()).and_then(|s| s.selected) {
                self.effects.push_back(Effect::SystemVoice(number as i32));
            }
            if let Some(number) = self.process(signal) {
                self.session.controls[group].select(number)?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        for group in self.groups(|a| matches!(a, layout::Action::Click(_))) {
            if !self.visible(group) {
                continue;
            }
            let signal = self.calcu_group(host, client, group, frame, !locked)?;
            if self.process(signal).is_some() {
                if matches!(self.session.controls[group].action, layout::Action::Click(0)) {
                    self.init_all_config()?;
                }
                self.session.controls[group].select(0)?;
                self.reload_control(group, &self.variables.clone())?;
            }
        }
        // Sliders use the mouse directly and are not gated by m_mode.
        let movie = self.policy.get(&self.context.file, 103)?;
        let sound = self.policy.get(&self.context.file, 101)?;
        for (index, slider) in layout.sliders.sliders.iter().enumerate() {
            if !slider.exists || slider.page as i64 != self.session.page as i64 + 1 {
                continue;
            }
            let switch = SWITCHES[index];
            if switch != 0 && self.policy.get(&self.context.file, switch)? == 0 {
                continue;
            }
            let interaction: Value = serde_json::from_slice(&host.query_engine(
                client,
                EngineQuery::UiInteraction(ROOT + 2000 + index as u32),
            )?)?;
            let captured = interaction["pressed"] == true;
            let Some(change) = self.session.sliders[index].calcu(
                &layout.sliders,
                index,
                frame.ticket,
                captured,
                frame.input.pointer_x,
                true,
                movie,
                sound,
            )?
            else {
                continue;
            };
            debug_assert_eq!(change.id, PARAMETERS[index]);
            self.effects.push_back(Effect::Parameter(change.id, change.value));
            if index == 0 {
                if let Some(sample) = self.session.sample.as_mut() {
                    sample.reset();
                }
            }
            if let Some(preview) = change.preview {
                self.effects.push_back(Effect::Preview(preview));
            }
        }
        // Volume switches (1..5, 8, 9) also ignore m_mode.
        for (index, switch) in SWITCHES.iter().copied().enumerate() {
            if switch == 0 || index >= layout.sliders.sliders.len() {
                continue;
            }
            for group in self.groups(|a| matches!(a, layout::Action::Toggle(id) if id == switch)) {
                if !self.visible(group) {
                    continue;
                }
                let signal = self.calcu_group(host, client, group, frame, true)?;
                let Some(number) = self.process(signal) else { continue };
                self.effects.push_back(Effect::Parameter(switch, 1 - number as i32));
                self.session.controls[group].select(1 - number.min(1))?;
                self.reload_control(group, &self.variables.clone())?;
                if index == 2 && number == 0 {
                    // Replay the ON button's click sound after enabling SE.
                    if let layout::Buttons::Check(pair) = &layout.groups[group].buttons {
                        let sound = pair[0].raw_sounds[0];
                        if sound > 0 {
                            self.effects.push_back(Effect::Audio(
                                crate::bridge::ButtonAudioEvent::new(sound, 0, pair[0].volume_type),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn character_slider(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        group: usize,
        slot: usize,
        slider: &crate::native_config_layout::characters::Slider,
        frame: &Frame,
    ) -> Result<()> {
        let interaction: Value = serde_json::from_slice(&host.query_engine(
            client,
            EngineQuery::UiInteraction(ROOT + 3000 + slot as u32),
        )?)?;
        let captured = interaction["pressed"] == true;
        let sample = self.character_captured[slot] || captured;
        self.character_captured[slot] = captured;
        if !sample {
            return Ok(());
        }
        let width = i64::from(slider.size[0]);
        ensure!(width > 0, "NNN Config character slider width");
        let divide = i64::from((slider.maximum - slider.minimum).max(1));
        let x = (i64::from(frame.input.pointer_x) - i64::from(slider.origin[0])).clamp(0, width);
        let gaze = ((x * divide + width / 2) / width).clamp(0, divide) as i32;
        self.effects
            .push_back(Effect::Parameter(500 + slot as i32, gaze + slider.minimum));
        if !captured {
            let state = self.session.controls[group].value();
            let layout = self.session.layout.clone();
            let buttons = &layout.characters[slot].buttons;
            let mut sound = buttons[1 - state.min(1)].raw_sounds[0];
            if slider.preview_sound != 0 {
                sound = slider.preview_sound;
            }
            if sound > 0 {
                self.effects.push_back(Effect::Audio(crate::bridge::ButtonAudioEvent::new(
                    sound,
                    0,
                    buttons[state.min(1)].volume_type,
                )));
            }
        }
        Ok(())
    }
    /// CCommonConfig::InitAllConfig through the reviewed reset plan.
    fn init_all_config(&mut self) -> Result<()> {
        use crate::native_config_layout::reset::Command;
        let file = self.context.file.clone();
        let policy = self.policy.clone();
        let plan = crate::native_config_layout::reset::plan(&self.table, &self.session.layout, |id| {
            policy.get(&file, id)
        })?;
        for command in plan {
            match command {
                Command::Parameter(id, value) => self.effects.push_back(Effect::Parameter(id, value)),
                Command::Slider(index, gaze) => self.session.sliders[index].gaze = gaze,
                Command::Window(_) => anyhow::bail!(
                    "NNN Config initWindowNumber requires the message-window variant owner"
                ),
                Command::VolumeSwitch { index, state } => {
                    for group in self.groups(|a| matches!(a, layout::Action::Toggle(id) if id == SWITCHES[index])) {
                        self.session.controls[group].select(usize::try_from(state)?.min(1))?;
                        self.reload_control(group, &self.variables.clone())?;
                    }
                }
                Command::SkipRadio(value) => {
                    for group in self.groups(|a| matches!(a, layout::Action::Parameter { id: 2, .. })) {
                        self.session.controls[group].select(usize::try_from(value)?)?;
                        self.reload_control(group, &self.variables.clone())?;
                    }
                }
                Command::CharacterSlider { index, gaze } => {
                    let _ = (index, gaze); // value written by the following Parameter(500+i)
                }
                Command::CharacterEnabled { index, enabled } => {
                    self.effects.push_back(Effect::Voice { slot: index, flag: enabled });
                    for group in self.groups(|a| matches!(a, layout::Action::Character { slot, .. } if slot == index)) {
                        self.session.controls[group].select(usize::from(enabled))?;
                        self.reload_control(group, &self.variables.clone())?;
                    }
                }
                Command::AnimeSwitch(value) => {
                    for group in self.groups(|a| matches!(a, layout::Action::Parameter { id: 300, .. })) {
                        self.session.controls[group].select(usize::try_from(value)?)?;
                        self.reload_control(group, &self.variables.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}
