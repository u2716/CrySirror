// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native two-state radio groups over common button policy. UI supplies scoped
//! hover/activation; images, hit masks and painting remain shared services.
use crate::{
    config::Names,
    native_bootstrap::NativeConfiguration,
    native_title::{NativeButtonSignal, NativeButtonSpec, NativeButtonState},
};
use anyhow::{Result, ensure};

pub(crate) const NOTHING: i32 = -1;
const NUMBER: i32 = 0xf0000000u32 as i32;
const START_CLICK: i32 = 0xf8000000u32 as i32;
const CLICKING: i32 = 0xf9000000u32 as i32;
const ENTER: i32 = 0xfb000000u32 as i32;
const EXIT: i32 = 0xfc000000u32 as i32;
const REQUEST_SOUND: i32 = 0xfd000000u32 as i32;

#[derive(Clone, Debug)]
pub(crate) struct RadioButtonSpec {
    pub button: NativeButtonSpec,
    pub voices: [i32; 4],
    pub raw_sounds: [i32; 4],
    pub volume_type: i32,
    pub cancel: bool,
    pub wheel: [bool; 2],
    pub wheel_reverse: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct RadioSpec {
    pub buttons: Vec<[RadioButtonSpec; 2]>,
}
impl RadioSpec {
    pub(crate) fn read(
        table: &Names,
        tag: &str,
        count: usize,
        configuration: &NativeConfiguration,
    ) -> Result<Self> {
        ensure!(
            !tag.is_empty() && tag.len() <= 64 && count > 0 && count <= 256,
            "NNN radio group source identity/budget"
        );
        let reset_source = configuration
            .setup
            .integer(b"notSrcCopyFlagEnable")?
            .unwrap_or(0)
            != 0;
        let mut buttons: Vec<[RadioButtonSpec; 2]> = Vec::with_capacity(count);
        for index in 0..count {
            let mut pair = Vec::with_capacity(2);
            for state in 0..2 {
                let prefix = format!("{tag}{}Button{}", index + 1, state + 1);
                let previous = if state == 0 {
                    buttons.last().map(|p| &p[1])
                } else {
                    pair.last()
                };
                let mut inherited = previous.map(|p: &RadioButtonSpec| p.button.clone());
                if state == 0 && reset_source {
                    if let Some(old) = &mut inherited {
                        let width = table
                            .integer(format!("{prefix}SizeX").as_bytes())?
                            .unwrap_or(old.size[0]);
                        old.states =
                            std::array::from_fn(|mode| [width.wrapping_mul(mode as i32), 0]);
                    }
                }
                // The common skin parser is reused for geometry, hit metadata,
                // source atlas, sound waits and source-copy inheritance. These
                // native extra controls are retained here instead of rejected
                // or silently erased by the menu-only consumer's profile.
                let unsupported = [
                    "ClickVoice",
                    "BadVoice",
                    "EnterVoice",
                    "ExitVoice",
                    "VolumeType",
                    "IsCancelButton",
                    "IsUpButton",
                    "IsDownButton",
                    "UpDownReverse",
                ];
                let filtered = Names {
                    tokens: table
                        .tokens
                        .chunks_exact(2)
                        .filter(|pair| {
                            !unsupported.iter().any(|key| {
                                pair[0].eq_ignore_ascii_case(format!("{prefix}{key}").as_bytes())
                            })
                        })
                        .flat_map(|pair| pair.iter().cloned())
                        .collect(),
                };
                let delta = if state == 0 {
                    inherited.as_ref().map(|p| [p.size[0], 0]).unwrap_or([0, 0])
                } else {
                    [0, 0]
                };
                let button = crate::native_title::read_menu_button(
                    &filtered,
                    &prefix,
                    configuration,
                    inherited.as_ref(),
                    delta,
                )?;
                let get = |key: &str, default: i32| -> Result<i32> {
                    Ok(table
                        .integer(format!("{prefix}{key}").as_bytes())?
                        .unwrap_or(default))
                };
                let mut raw_sounds = previous.map(|p| p.raw_sounds).unwrap_or([0; 4]);
                for (word, key) in
                    raw_sounds
                        .iter_mut()
                        .zip(["ClickSound", "BadSound", "EnterSound", "ExitSound"])
                {
                    *word = get(key, *word)?;
                }
                let mut voices = previous.map(|p| p.voices).unwrap_or([0; 4]);
                for (word, key) in
                    voices
                        .iter_mut()
                        .zip(["ClickVoice", "BadVoice", "EnterVoice", "ExitVoice"])
                {
                    *word = get(key, *word)?;
                }
                pair.push(RadioButtonSpec {
                    button,
                    voices,
                    raw_sounds,
                    volume_type: get("VolumeType", previous.map(|p| p.volume_type).unwrap_or(0))?,
                    // Source CopyParam intentionally does not copy these flags.
                    cancel: get("IsCancelButton", 0)? != 0,
                    wheel: [get("IsUpButton", 0)? != 0, get("IsDownButton", 0)? != 0],
                    wheel_reverse: get("UpDownReverse", 0)? != 0,
                });
            }
            buttons.push(
                pair.try_into()
                    .map_err(|_| anyhow::anyhow!("NNN radio pair construction"))?,
            );
        }
        Ok(Self { buttons })
    }
}

/// CCommonButton::MakeButtonStatus, including request bits whose masked sound
/// or voice index becomes zero. The mode's audio consumer decides playback.
pub(crate) fn encode(status: i32, sound: i32, voice: i32, data: Option<usize>, volume: i32) -> i32 {
    let mut out = status & 0xff000000u32 as i32;
    if sound > 0 {
        out |= (sound.wrapping_shl(8) & 0x7f00) | 0x8000;
        if volume != -1 {
            out |= volume.wrapping_shl(22) & 0xc00000;
        }
    }
    if voice > 0 {
        out |= (voice.wrapping_shl(16) & 0x3f0000) | 0x8000;
    }
    if let Some(data) = data {
        out |= (data as i32 & 0x7f) | 0x80;
    }
    out
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ButtonInput {
    pub hovered: bool,
    pub pressed: bool,
}
#[derive(Clone, Debug)]
struct ButtonState {
    state: NativeButtonState,
    suppress_enter: bool,
    enter: bool,
    exit: bool,
    ignore: u32,
}
impl ButtonState {
    fn new(spec: &NativeButtonSpec) -> Self {
        Self {
            state: NativeButtonState::new(spec),
            suppress_enter: false,
            enter: false,
            exit: false,
            ignore: 0,
        }
    }
    fn calcu(&mut self, spec: &RadioButtonSpec, input: Option<ButtonInput>) -> i32 {
        if !spec.button.exists {
            return NOTHING;
        }
        let input = if self.ignore > 0 {
            self.ignore -= 1;
            None
        } else {
            input
        };
        let Some(input) = input else {
            self.state.calcu_without_input(&spec.button);
            return NOTHING;
        };
        let before = self.state.mode;
        let signal = self.state.calcu(&spec.button, input.hovered, input.pressed);
        let (status, audio, data) = if !spec.button.enabled {
            if input.pressed && (spec.raw_sounds[1] != 0 || spec.voices[1] > 0) {
                (REQUEST_SOUND, Some(1), None)
            } else {
                return NOTHING;
            }
        } else if before == 2 {
            if signal == NativeButtonSignal::Selected {
                (NUMBER, None, Some(0))
            } else {
                (CLICKING, None, None)
            }
        } else if matches!(
            signal,
            NativeButtonSignal::ClickStarted | NativeButtonSignal::Selected
        ) {
            (
                if signal == NativeButtonSignal::Selected {
                    NUMBER
                } else {
                    START_CLICK
                },
                Some(0),
                (signal == NativeButtonSignal::Selected).then_some(0),
            )
        } else if signal == NativeButtonSignal::Entered {
            self.enter = true;
            (
                ENTER,
                (spec.button.sound_waits[0] == 0 && !self.suppress_enter).then_some(2),
                None,
            )
        } else if signal == NativeButtonSignal::Exited {
            self.exit = true;
            self.suppress_enter = false;
            (EXIT, (spec.button.sound_waits[1] == 0).then_some(3), None)
        } else if self.state.mode == 0 {
            self.suppress_enter = false;
            if self.exit
                && self.state.count == spec.button.sound_waits[1]
                && (spec.raw_sounds[3] > 0 || spec.voices[3] > 0)
            {
                (REQUEST_SOUND, Some(3), None)
            } else {
                return NOTHING;
            }
        } else if self.state.mode == 1
            && self.enter
            && !self.suppress_enter
            && self.state.count == spec.button.sound_waits[0]
            && (spec.raw_sounds[2] > 0 || spec.voices[2] > 0)
        {
            (REQUEST_SOUND, Some(2), None)
        } else {
            return NOTHING;
        };
        let (sound, voice) = audio
            .map(|i| (spec.raw_sounds[i], spec.voices[i]))
            .unwrap_or((0, 0));
        encode(status, sound, voice, data, spec.volume_type)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct RadioGroup {
    pub spec: RadioSpec,
    states: Vec<[ButtonState; 2]>,
    active: Vec<usize>,
    exists: Vec<bool>,
    pub radio: usize,
    clicking: Option<usize>,
}
impl RadioGroup {
    pub(crate) fn new(spec: RadioSpec) -> Self {
        let states = spec
            .buttons
            .iter()
            .map(|p| std::array::from_fn(|i| ButtonState::new(&p[i].button)))
            .collect();
        let count = spec.buttons.len();
        Self {
            spec,
            states,
            active: vec![0; count],
            exists: vec![true; count],
            radio: 0,
            clicking: None,
        }
    }
    pub(crate) fn init(&mut self, radio: usize) -> Result<()> {
        ensure!(radio < self.states.len(), "NNN radio initial identity");
        self.radio = radio;
        self.clicking = None;
        for (index, pair) in self.states.iter_mut().enumerate() {
            let selected = usize::from(index == radio);
            self.active[index] = selected;
            pair[selected] = ButtonState::new(&self.spec.buttons[index][selected].button);
            // MultiState::Init calls SetEntered after Init to suppress the
            // first hover's sound/voice, while retaining the ENTER status.
            pair[selected].suppress_enter = true;
        }
        Ok(())
    }
    pub(crate) fn set_flags(&mut self, index: usize, exists: bool, enabled: bool) -> Result<()> {
        ensure!(index < self.states.len(), "NNN radio visibility identity");
        self.exists[index] = exists;
        for spec in &mut self.spec.buttons[index] {
            spec.button.enabled = enabled;
        }
        Ok(())
    }
    pub(crate) fn calcu(&mut self, input: Option<&[ButtonInput]>) -> Result<i32> {
        ensure!(
            input.is_none_or(|v| v.len() == self.states.len()),
            "NNN radio input extent"
        );
        let mut result = NOTHING;
        let mut level = 0;
        for index in 0..self.states.len() {
            if !self.exists[index] {
                continue;
            }
            let active = self.active[index];
            let eligible = index != self.radio && self.clicking.is_none_or(|n| n == index);
            let signal = self.states[index][active].calcu(
                &self.spec.buttons[index][active],
                if eligible {
                    input.map(|v| v[index])
                } else {
                    None
                },
            );
            if signal == NOTHING {
                continue;
            }
            let status = signal & 0xff000000u32 as i32;
            let priority = if status == NUMBER {
                5
            } else if status == START_CLICK {
                self.clicking = Some(index);
                3
            } else {
                1
            };
            if priority > level {
                level = priority;
                result = signal;
            }
            if status == NUMBER {
                // Source rewrites NUMBER data after priority selection, so a
                // later completed selection wins even when priorities tie.
                result = (signal & !0xff) | (index as i32 & 0x7f) | 0x80;
                self.radio = index;
            }
        }
        Ok(result)
    }
    pub(crate) fn exists(&self, index: usize) -> Result<bool> {
        ensure!(index < self.exists.len(), "NNN radio visible identity");
        Ok(self.exists[index])
    }
    pub(crate) fn active(&self, index: usize) -> Result<(&RadioButtonSpec, &NativeButtonState)> {
        ensure!(index < self.states.len(), "NNN radio active identity");
        let active = self.active[index];
        Ok((
            &self.spec.buttons[index][active],
            &self.states[index][active].state,
        ))
    }
    pub(crate) fn active_mut(&mut self, index: usize) -> Result<(&RadioButtonSpec, &mut NativeButtonState)> {
        ensure!(index < self.states.len(), "NNN radio mutable Print identity");
        let active = self.active[index];
        Ok((&self.spec.buttons[index][active], &mut self.states[index][active].state))
    }
    pub(crate) fn printed(&mut self, index: usize, native_picture_exists: bool) -> Result<()> {
        ensure!(index < self.states.len(), "NNN radio print identity");
        if self.exists[index] && native_picture_exists {
            let active = self.active[index];
            self.states[index][active]
                .state
                .printed(&self.spec.buttons[index][active].button);
        }
        Ok(())
    }
}
