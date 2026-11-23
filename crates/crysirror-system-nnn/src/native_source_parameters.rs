// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Published-source Get/SetSystemParam policy (gamecallback.cpp5335..5651,
//! commonSystemParamName.h and packed SYSTEMDATA/SYSTEMFLAG2 declarations).
//! This does not change MUGEN2's reviewed callback ABI. Native file writes are
//! candidates; the owner must finish every returned service action before
//! committing the candidate. No setter may silently discard those actions.
use crate::{config::Names, native_system_file::NativeSystemFile};
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub(crate) struct Policy {
    movie_alias: i32,
    sound_voice_alias: i32,
    total_enabled: bool,
    mouse_radio: i32,
    cursor_radio: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    ChangeMusicVolume(i32),
    /// FALSE stops music but preserves its native last-number; TRUE replays
    /// that number with the original repeat count if one actually exists.
    SwitchMusic(bool),
    /// Scoped mixer gain, not a change to the machine's output-device volume.
    ChangeTotalVolume(i32),
    MessageWindowPercent,
    MouseByConfig(i32),
    MessageCursorByConfig(i32),
}
pub(crate) struct Write {
    pub file: NativeSystemFile,
    pub actions: Vec<Action>,
}
fn data_offset(id: i32) -> Option<usize> {
    Some(match id {
        2 => 140,
        4 => 84,
        6 => 164,
        100 => 176,
        101 => 184,
        102 => 180,
        103 => 188,
        104 => 212,
        105 => 136,
        106 => 1932,
        107 => 220,
        108 => 1968,
        200 => 192,
        201 => 200,
        202 => 196,
        203 => 204,
        204 => 216,
        208 => 1972,
        300..=315 => 1792 + (id - 300) as usize * 4,
        400..=415 => 1856 + (id - 400) as usize * 4,
        _ => return None,
    })
}
fn flag2_index(id: i32) -> Option<usize> {
    Some(match id {
        109 => 4,
        209 => 5,
        210 => 2,
        211 => 8,
        212 => 3,
        213 => 7,
        214 => 1,
        _ => return None,
    })
}
impl Policy {
    pub(crate) fn read(game: &Names) -> Result<Self> {
        Ok(Self {
            movie_alias: game.integer(b"movieVolumeNotUse")?.unwrap_or(0),
            sound_voice_alias: game.integer(b"soundVoiceVolumeNotUse")?.unwrap_or(1),
            total_enabled: game.integer(b"totalVolumeUseFlag")?.unwrap_or(0) != 0,
            mouse_radio: game.integer(b"expModeButtonIsMouse")?.unwrap_or(0),
            cursor_radio: game.integer(b"expModeButtonIsMessageCursor")?.unwrap_or(0),
        })
    }
    pub(crate) fn get(&self, file: &NativeSystemFile, id: i32) -> Result<i32> {
        // Get aliases and setter suppression are separate native branches.
        let id = match id {
            103 => match self.movie_alias {
                1 => 100,
                2 => 101,
                3 => 102,
                _ => 103,
            },
            203 => match self.movie_alias {
                1 => 200,
                2 => 201,
                3 => 202,
                _ => 203,
            },
            104 => match self.sound_voice_alias {
                0 => 104,
                1 => 102,
                _ => 101,
            },
            204 => {
                if self.sound_voice_alias == 0 {
                    204
                } else {
                    202
                }
            }
            _ => id,
        };
        if let Some(at) = data_offset(id) {
            return file.system_word(at);
        }
        if (500..572).contains(&id) {
            return crate::native_voice_policy::character_slider(file, id - 500);
        }
        if file.source_version2() {
            if let Some(at) = flag2_index(id) {
                return file.flag2_word(at);
            }
        }
        // The complete base-source switch explicitly returns0 here, including
        // declared-but-unused BGMMODE/USERDEFINE IDs. It is not a guessed ACK
        // for an unknown executable/native dispatcher profile.
        Ok(0)
    }
    pub(crate) fn prepare(&self, file: &NativeSystemFile, id: i32, value: i32) -> Result<Write> {
        let mut next = file.clone();
        let mut actions = Vec::new();
        let suppressed = matches!(id, 103 | 203) && self.movie_alias != 0
            || matches!(id, 104 | 204) && self.sound_voice_alias != 0
            || matches!(id, 108 | 208) && !self.total_enabled;
        if suppressed {
            return Ok(Write {
                file: next,
                actions,
            });
        }
        if let Some(at) = data_offset(id) {
            // Source SetSystemParam does not range-clamp dat. UI sliders/radios
            // validate their own domain; script/callback values retain all bits.
            next.set_setting_word(at, value)?;
            match id {
                100 => actions.push(Action::ChangeMusicVolume(value)),
                200 | 208 => actions.push(Action::SwitchMusic(value != 0)),
                108 => actions.push(Action::ChangeTotalVolume(value)),
                106 => actions.push(Action::MessageWindowPercent),
                300..=315 => {
                    // Base AfterChangeSystemParamExpRadio returns FALSE.
                    // Subclasses need their own explicit override profile.
                    if id - 300 + 1 == self.mouse_radio {
                        actions.push(Action::MouseByConfig(value));
                    }
                    if id - 300 + 1 == self.cursor_radio {
                        actions.push(Action::MessageCursorByConfig(value));
                    }
                }
                // Base AfterChangeSystemParamExpCheck also returns FALSE;
                // there is no follow-up mouse/cursor call in that branch.
                _ => {}
            }
        } else if (500..572).contains(&id) {
            crate::native_voice_policy::set_character_slider(&mut next, id - 500, value)?;
        } else if next.source_version2() {
            if let Some(at) = flag2_index(id) {
                next.set_flag2_word(at, value)?;
            }
        }
        ensure!(actions.len() <= 2, "NNN source parameter action extent");
        Ok(Write {
            file: next,
            actions,
        })
    }
}
