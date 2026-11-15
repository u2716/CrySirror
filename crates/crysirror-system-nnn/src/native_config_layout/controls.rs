// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Config's CommonButton/MultiState/Radio semantic owners. Inputs are scoped
//! engine-UI results; no pointer hit-test, image algorithm or host event loop.
use super::{Action, Buttons, Group};
use crate::{
    native_radio_group::{ButtonInput, RadioButtonSpec, RadioGroup, encode},
    native_title::{NativeButtonSignal, NativeButtonState},
};
use anyhow::{Result, ensure};
const NOTHING: i32 = -1;
const NUMBER: i32 = 0xf0000000u32 as i32;
const START: i32 = 0xf8000000u32 as i32;
const CLICKING: i32 = 0xf9000000u32 as i32;
const ENTER: i32 = 0xfb000000u32 as i32;
const EXIT: i32 = 0xfc000000u32 as i32;
const SOUND: i32 = 0xfd000000u32 as i32;
#[derive(Clone, Debug)]
struct Button {
    spec: RadioButtonSpec,
    state: NativeButtonState,
    suppress_enter: bool,
    entered: bool,
    exited: bool,
}
impl Button {
    fn new(spec: RadioButtonSpec, multi: bool) -> Self {
        Self {
            state: NativeButtonState::new(&spec.button),
            spec,
            suppress_enter: multi,
            entered: false,
            exited: false,
        }
    }
    fn init(&mut self, multi: bool) {
        self.state = NativeButtonState::new(&self.spec.button);
        self.suppress_enter = multi;
        self.entered = false;
        self.exited = false;
    }
    fn calcu(&mut self, input: Option<ButtonInput>) -> i32 {
        if !self.spec.button.exists {
            return NOTHING;
        }
        let Some(input) = input else {
            self.state.calcu_without_input(&self.spec.button);
            return NOTHING;
        };
        let before = self.state.mode;
        let result = self
            .state
            .calcu(&self.spec.button, input.hovered, input.pressed);
        let (status, event, data) = if !self.spec.button.enabled {
            if input.pressed && (self.spec.raw_sounds[1] != 0 || self.spec.voices[1] > 0) {
                (SOUND, Some(1), None)
            } else {
                return NOTHING;
            }
        } else if before == 2 {
            if result == NativeButtonSignal::Selected {
                (NUMBER, None, Some(0))
            } else {
                (CLICKING, None, None)
            }
        } else if matches!(
            result,
            NativeButtonSignal::ClickStarted | NativeButtonSignal::Selected
        ) {
            (
                if result == NativeButtonSignal::Selected {
                    NUMBER
                } else {
                    START
                },
                Some(0),
                (result == NativeButtonSignal::Selected).then_some(0),
            )
        } else if result == NativeButtonSignal::Entered {
            self.entered = true;
            (
                ENTER,
                (self.spec.button.sound_waits[0] == 0 && !self.suppress_enter).then_some(2),
                None,
            )
        } else if result == NativeButtonSignal::Exited {
            self.exited = true;
            self.suppress_enter = false;
            (
                EXIT,
                (self.spec.button.sound_waits[1] == 0).then_some(3),
                None,
            )
        } else if self.state.mode == 0 {
            self.suppress_enter = false;
            if self.exited
                && self.state.count == self.spec.button.sound_waits[1]
                && (self.spec.raw_sounds[3] > 0 || self.spec.voices[3] > 0)
            {
                (SOUND, Some(3), None)
            } else {
                return NOTHING;
            }
        } else if self.state.mode == 1
            && self.entered
            && !self.suppress_enter
            && self.state.count == self.spec.button.sound_waits[0]
            && (self.spec.raw_sounds[2] > 0 || self.spec.voices[2] > 0)
        {
            (SOUND, Some(2), None)
        } else {
            return NOTHING;
        };
        let (sound, voice) = event
            .map(|i| (self.spec.raw_sounds[i], self.spec.voices[i]))
            .unwrap_or((0, 0));
        encode(status, sound, voice, data, self.spec.volume_type)
    }
}
#[derive(Clone, Debug)]
enum State {
    Single(Button),
    Check {
        buttons: [Button; 2],
        selected: usize,
    },
    Radio(RadioGroup),
}
#[derive(Clone, Debug)]
pub(crate) struct Control {
    pub action: Action,
    pub page: Option<i32>,
    state: State,
    exists: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Signal {
    pub status: i32,
    pub selected: Option<usize>,
    pub sound: i32,
    pub voice: i32,
    pub volume: i32,
}
impl Signal {
    fn read(value: i32) -> Option<Self> {
        if value == NOTHING {
            return None;
        }
        Some(Self {
            status: value & 0xff000000u32 as i32,
            selected: (value & 0x80 != 0).then_some((value & 0x7f) as usize),
            sound: if value & 0x8000 != 0 {
                (value >> 8) & 0x7f
            } else {
                0
            },
            voice: if value & 0x8000 != 0 {
                (value >> 16) & 0x3f
            } else {
                0
            },
            volume: (value >> 22) & 3,
        })
    }
    pub(crate) fn starts_click(self) -> bool {
        self.status == START
    }
    pub(crate) fn completes_click(self) -> bool {
        self.status == NUMBER
    }
}
impl Control {
    pub(crate) fn new(group: &Group, value: i32) -> Result<Self> {
        let state = match &group.buttons {
            Buttons::Single(spec) => State::Single(Button::new(spec.clone(), false)),
            Buttons::Check(spec) => {
                ensure!(
                    (0..=1).contains(&value),
                    "NNN Config original checkbox state"
                );
                State::Check {
                    buttons: spec.clone().map(|spec| Button::new(spec, true)),
                    selected: value as usize,
                }
            }
            Buttons::Radio(spec) => {
                let mut radio = RadioGroup::new(spec.clone());
                radio.init(usize::try_from(value)?)?;
                State::Radio(radio)
            }
        };
        Ok(Self {
            action: group.action,
            page: group.page,
            state,
            exists: true,
        })
    }
    pub(crate) fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        match &mut self.state {
            State::Single(button) => {
                button.spec.button.enabled = enabled;
                button.init(false);
            }
            State::Check { buttons, selected } => {
                for button in buttons.iter_mut() {
                    button.spec.button.enabled = enabled;
                }
                buttons[*selected].init(true);
            }
            State::Radio(group) => {
                for index in 0..group.spec.buttons.len() {
                    let exists = group.exists(index)?;
                    group.set_flags(index, exists, enabled)?;
                }
                group.init(group.radio)?;
            }
        }
        Ok(())
    }
    pub(crate) fn set_radio_flags(
        &mut self,
        index: usize,
        exists: bool,
        enabled: bool,
    ) -> Result<()> {
        let State::Radio(group) = &mut self.state else {
            anyhow::bail!("NNN Config radio flag applied to a non-radio control")
        };
        group.set_flags(index, exists, enabled)
    }
    pub(crate) fn set_check_assets(
        &mut self,
        index: usize,
        source: Option<crate::native_resources::ImageLookup>,
        sounds: [i32; 4],
    ) -> Result<()> {
        let State::Check { buttons, .. } = &mut self.state else {
            anyhow::bail!("NNN Config character controls require a two-state button")
        };
        ensure!(index < 2, "NNN Config character asset state");
        let button = &mut buttons[index];
        button.spec.button.source = source;
        button.spec.raw_sounds = sounds;
        button.spec.button.sounds = sounds.map(|sound| sound.max(0));
        Ok(())
    }
    pub(crate) fn set_exists(&mut self, exists: bool) {
        self.exists = exists;
    }
    pub(crate) fn visible(&self, page: usize) -> bool {
        self.exists
            && self
                .page
                .is_none_or(|number| i64::from(number) == page as i64 + 1)
    }
    pub(crate) fn len(&self) -> usize {
        match &self.state {
            State::Radio(group) => group.spec.buttons.len(),
            _ => 1,
        }
    }
    pub(crate) fn active(&self, index: usize) -> Result<(&RadioButtonSpec, &NativeButtonState)> {
        match &self.state {
            State::Radio(group) => group.active(index),
            State::Single(button) => {
                ensure!(index == 0, "NNN Config single control identity");
                Ok((&button.spec, &button.state))
            }
            State::Check { buttons, selected } => {
                ensure!(index == 0, "NNN Config check control identity");
                Ok((&buttons[*selected].spec, &buttons[*selected].state))
            }
        }
    }
    pub(crate) fn active_mut(&mut self, index: usize) -> Result<(&RadioButtonSpec, &mut NativeButtonState)> {
        match &mut self.state {
            State::Radio(group) => group.active_mut(index),
            State::Single(button) => {
                ensure!(index == 0, "NNN Config single mutable Print identity");
                Ok((&button.spec, &mut button.state))
            }
            State::Check { buttons, selected } => {
                ensure!(index == 0, "NNN Config check mutable Print identity");
                let button=&mut buttons[*selected];
                Ok((&button.spec, &mut button.state))
            }
        }
    }
    pub(crate) fn calcu(&mut self, input: Option<&[ButtonInput]>) -> Result<Option<Signal>> {
        if !self.exists {
            return Ok(None);
        }
        ensure!(
            input.is_none_or(|input| input.len() == self.len()),
            "NNN Config scoped control input extent"
        );
        let raw = match &mut self.state {
            State::Radio(group) => group.calcu(input)?,
            State::Single(button) => button.calcu(input.map(|input| input[0])),
            State::Check { buttons, selected } => {
                let raw = buttons[*selected].calcu(input.map(|input| input[0]));
                if raw != NOTHING && raw & 0x80 != 0 {
                    (raw & !0xff) | (*selected as i32 & 0x7f) | 0x80
                } else {
                    raw
                }
            }
        };
        Ok(Signal::read(raw))
    }
    /// Called after the service/file action and source image reload complete.
    /// A resource retry must not invoke this or another control Calcu twice.
    pub(crate) fn value(&self) -> usize {
        match &self.state {
            State::Single(_) => 0,
            State::Check { selected, .. } => *selected,
            State::Radio(group) => group.radio,
        }
    }
    pub(crate) fn select(&mut self, value: usize) -> Result<()> {
        match &mut self.state {
            State::Single(button) => {
                ensure!(value == 0, "NNN Config single Init identity");
                button.init(false);
            }
            State::Check { buttons, selected } => {
                ensure!(value < 2, "NNN Config check Init state");
                *selected = value;
                buttons[value].init(true);
            }
            State::Radio(group) => group.init(value)?,
        }
        Ok(())
    }
    pub(crate) fn printed(&mut self, index: usize, picture_exists: bool) -> Result<()> {
        match &mut self.state {
            State::Radio(group) => group.printed(index, picture_exists),
            State::Single(button) => {
                ensure!(index == 0, "NNN Config single Print identity");
                if picture_exists {
                    button.state.printed(&button.spec.button);
                }
                Ok(())
            }
            State::Check { buttons, selected } => {
                ensure!(index == 0, "NNN Config check Print identity");
                if picture_exists {
                    buttons[*selected]
                        .state
                        .printed(&buttons[*selected].spec.button);
                }
                Ok(())
            }
        }
    }
}
