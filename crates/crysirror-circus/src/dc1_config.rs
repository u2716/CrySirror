//! D.C. (DC.EXE) system settings (418160 defaults) and the 407CC0 option
//! screen state. Values keep their original units: DirectSound hundredths of
//! a decibel, BA9D9C wipe steps and BA9E44 window alpha. Drawing belongs to
//! the adapter; this module only applies the original hit/value rules.
use crate::dc1_frontend::ConfigTables;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// BA9DB0: voices under VOICE\suginami and VOICE\sgaku7.
    pub voice_special: bool,
    /// BA9DB4: every other voice.
    pub voice_other: bool,
    /// BA9D4C (grade wipe) and BA9D50 (MMX present, so switchable).
    pub effect_mode: bool,
    pub effect_capable: bool,
    /// BA9D84 option row 3.
    pub option3: bool,
    /// BA9D80: opcodes 88/89 jump over already seen events.
    pub seen_skip: bool,
    /// BA9D28/BA9E24, BA9D2C/BA9E2C, BA9D30/BA9E28.
    pub music: (bool, i32),
    pub voice: (bool, i32),
    pub sound: (bool, i32),
    /// BA9E3C (0..3), BA9E4C and BA9F34 (auto advance interval).
    pub text_mode: i32,
    pub auto_advance: Option<u64>,
    /// BA9E40, BA9D9C and BA9D3C.
    pub speed: i32,
    pub wipe_step: i32,
    pub wipe_enabled: bool,
    /// BA9E44.
    pub window_alpha: i32,
}
impl Default for Settings {
    /// 418160 with 41A560 reporting MMX, which every supported host has.
    fn default() -> Self {
        Self {
            voice_special: true,
            voice_other: true,
            effect_mode: true,
            effect_capable: true,
            option3: true,
            seen_skip: true,
            music: (true, -900),
            voice: (true, -300),
            sound: (true, 0),
            text_mode: 0,
            auto_advance: None,
            speed: 1,
            wipe_step: 32,
            wipe_enabled: true,
            window_alpha: 128,
        }
    }
}
impl Settings {
    /// 4083E0 slider width for a DirectSound level (0 when disabled).
    pub fn level_width(enabled: bool, level: i32) -> i32 {
        if !enabled || level == -3000 {
            0
        } else if level == 0 {
            160
        } else {
            16 * (level / 300 + 10)
        }
    }
    pub fn speed_width(&self) -> i32 {
        40 * self.speed.clamp(0, 4)
    }
    pub fn alpha_width(&self) -> i32 {
        if self.window_alpha != 0 {
            10 * (self.window_alpha / 16)
        } else {
            0
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigAction {
    None,
    Redraw,
    /// 42A690 sample (aVoiceNemu0223a) after a voice level change.
    PreviewVoice,
    /// 42A5B0(19) after an SE level change.
    PreviewSound,
    MusicLevel,
    MusicOff,
    VoiceOff,
    /// Row 11: 41E180 player-name entry.
    NameEntry,
    /// Row 12 or BA9F70: 40CFB0(1), 422C60, return to the caller.
    Close,
}

#[derive(Clone, Debug, Default)]
pub struct ConfigScreen {
    /// 6B0DB8 (retained after the pointer leaves a row) and 6B0DC0.
    pub hover: i32,
    pub radio_hover: i32,
}

pub struct ConfigInput {
    pub x: i32,
    pub y: i32,
    /// BA9FA0: pointer activity this frame.
    pub pointer: bool,
    /// BA9F50 (left click), BA9F48 (Space), BA9F4C (Enter).
    pub select: bool,
    /// BA9F70.
    pub cancel: bool,
}

impl ConfigScreen {
    /// One 407CC0(0) call.
    pub fn input(
        &mut self,
        tables: &ConfigTables,
        settings: &mut Settings,
        input: &ConfigInput,
    ) -> ConfigAction {
        let (x, y) = (input.x, input.y);
        let mut inside = false;
        if input.pointer {
            for (index, row) in tables.rows.iter().enumerate() {
                if x >= row[0] && y >= row[1] && x < row[2] && y < row[3] {
                    self.hover = index as i32;
                    if index == 8 {
                        let base = tables.rows[8];
                        for (cell, radio) in tables.radio.iter().enumerate() {
                            let (left, top) = (base[0] + radio[0], base[1] + radio[1]);
                            if x >= left && y >= top && x < left + radio[2] && y < top + radio[3] {
                                self.radio_hover = cell as i32;
                            }
                        }
                    }
                    inside = true;
                }
            }
        }
        if !input.select {
            if input.cancel {
                return ConfigAction::Close;
            }
            return if inside { ConfigAction::Redraw } else { ConfigAction::None };
        }
        let row = |index: usize| tables.rows[index];
        let slider = |index: usize, divisor: i32| -> Option<i32> {
            let r = row(index);
            (x >= r[0] + 168 && y >= r[1] + 16 && x < r[0] + 328 && y < r[1] + 24)
                .then(|| (x - (r[0] + 168) + 1) / divisor)
        };
        //42AA70: -300 per step, clamped to -3000..0.
        let level = |steps: i32| (-300 * (10 - steps)).clamp(-3000, 0);
        let action = match self.hover {
            0 => {
                settings.voice_special = !settings.voice_special;
                ConfigAction::Redraw
            }
            1 => {
                settings.voice_other = !settings.voice_other;
                ConfigAction::Redraw
            }
            2 => {
                if settings.effect_capable {
                    settings.effect_mode = !settings.effect_mode;
                }
                ConfigAction::Redraw
            }
            3 => {
                settings.option3 = !settings.option3;
                ConfigAction::Redraw
            }
            4 => {
                settings.seen_skip = !settings.seen_skip;
                ConfigAction::Redraw
            }
            5 => match slider(5, 15) {
                Some(0) => {
                    settings.music.0 = false;
                    ConfigAction::MusicOff
                }
                Some(steps) => {
                    settings.music = (true, level(steps));
                    ConfigAction::MusicLevel
                }
                None => ConfigAction::Redraw,
            },
            6 => match slider(6, 15) {
                Some(0) => {
                    settings.voice.0 = false;
                    ConfigAction::VoiceOff
                }
                Some(steps) => {
                    settings.voice = (true, level(steps));
                    ConfigAction::PreviewVoice
                }
                None => ConfigAction::Redraw,
            },
            7 => match slider(7, 15) {
                Some(0) => {
                    settings.sound.0 = false;
                    //407CC0 stops the voice channel (42A8E0) here.
                    ConfigAction::VoiceOff
                }
                Some(steps) => {
                    settings.sound = (true, level(steps));
                    ConfigAction::PreviewSound
                }
                None => ConfigAction::Redraw,
            },
            8 => {
                let base = row(8);
                for (cell, radio) in tables.radio.iter().enumerate() {
                    let (left, top) = (base[0] + radio[0], base[1] + radio[1]);
                    if x >= left && y >= top && x < left + radio[2] && y < top + radio[3] {
                        settings.text_mode = cell as i32;
                    }
                }
                settings.auto_advance = match settings.text_mode {
                    2 => Some(5000),
                    3 => Some(2000),
                    _ => None,
                };
                ConfigAction::Redraw
            }
            9 => {
                if let Some(step) = slider(9, 33) {
                    settings.wipe_enabled = step < 4;
                    settings.speed = step.min(4);
                    settings.wipe_step = [16, 32, 64, 128, 256][step.clamp(0, 4) as usize];
                }
                ConfigAction::Redraw
            }
            10 => {
                let r = row(10);
                if x >= r[0] + 168 && y >= r[1] + 16 && x < r[0] + 328 && y < r[1] + 24 {
                    settings.window_alpha = 16 * ((x - (r[0] + 168)) / 10 + 1);
                }
                ConfigAction::Redraw
            }
            11 => ConfigAction::NameEntry,
            12 => ConfigAction::Close,
            _ => ConfigAction::Redraw,
        };
        //BA9F70 closes after any action except 41E180, which returns first.
        if input.cancel && action != ConfigAction::NameEntry {
            return ConfigAction::Close;
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_units_follow_4083e0_and_42aa70() {
        assert_eq!(Settings::level_width(true, -900), 112);
        assert_eq!(Settings::level_width(true, 0), 160);
        assert_eq!(Settings::level_width(true, -3000), 0);
        assert_eq!(Settings::level_width(false, -300), 0);
        let settings = Settings::default();
        assert_eq!(settings.speed_width(), 40);
        assert_eq!(settings.alpha_width(), 80);
    }
}
