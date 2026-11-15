// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig::InitAllConfig as an ordered native service/control plan.
//! The owner applies every command before committing its file/UI candidate.
use super::{Layout, n};
use crate::{
    config::Names,
    native_config_volume::{PARAMETERS, SWITCHES},
};
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub(crate) enum Command {
    Parameter(i32, i32),
    Slider(usize, i32),
    Window(i32),
    VolumeSwitch { index: usize, state: i32 },
    SkipRadio(i32),
    CharacterSlider { index: usize, gaze: i32 },
    CharacterEnabled { index: usize, enabled: bool },
    AnimeSwitch(i32),
}
fn slider(out: &mut Vec<Command>, layout: &Layout, index: usize, gaze: i32) {
    if layout
        .sliders
        .sliders
        .get(index)
        .is_some_and(|slider| slider.exists)
    {
        out.push(Command::Slider(index, gaze));
    }
}
pub(crate) fn plan(
    table: &Names,
    layout: &Layout,
    mut current: impl FnMut(i32) -> Result<i32>,
) -> Result<Vec<Command>> {
    let mut out = Vec::new();
    let v = n(table, "initWindowPercent", -1)?;
    if v >= 0 {
        out.push(Command::Parameter(106, 100i32.wrapping_sub(v)));
        slider(&mut out, layout, 6, v);
    }
    let v = n(table, "initWindowNumber", -1)?;
    if v >= 0 {
        out.push(Command::Window(v));
    }
    let v = n(table, "initTotalVolume", -1)?;
    if v >= 0 {
        out.push(Command::Parameter(108, v));
        slider(&mut out, layout, 8, v);
    }
    let tags = [
        "messageSpeed",
        "music",
        "sound",
        "voice",
        "movie",
        "soundvoice",
        "windowPercent",
        "autoSpeed",
        "total",
        "scriptse",
    ];
    // The source constructor loops <=volumeKosuu, which touches the array's
    // unallocated index10 at maximum size. Only the ten declared entries are
    // observable by InitAllConfig. Do not reproduce that out-of-bounds access.
    for index in 1..layout.sliders.sliders.len() {
        if index >= 6 && index < 9 {
            continue;
        }
        let v = n(table, &format!("init{}Volume", tags[index]), -1)?;
        if v < 0 {
            continue;
        }
        let slider_profile = &layout.sliders.sliders[index];
        let value = slider_profile
            .real_minimum
            .wrapping_add(
                slider_profile
                    .real_maximum
                    .wrapping_sub(slider_profile.real_minimum)
                    .wrapping_mul(v)
                    / 100,
            )
            .clamp(0, 100);
        out.push(Command::Parameter(PARAMETERS[index], value));
        slider(
            &mut out,
            layout,
            index,
            v.wrapping_mul(slider_profile.divide) / 100,
        );
    }
    for index in 1..layout.sliders.sliders.len() {
        if (6..8).contains(&index) {
            continue;
        }
        let v = n(table, &format!("init{}VolumeSwitch", tags[index]), -1)?;
        if v < 0 {
            continue;
        }
        // Music and total/script-SE avoid a redundant SetSystemParam call,
        // preventing a gratuitous music restart. The UI still reloads/Init's.
        if !(index == 1 || index >= 8) || current(SWITCHES[index])? != v {
            out.push(Command::Parameter(SWITCHES[index], v));
        }
        if layout.sliders.sliders[index].exists {
            out.push(Command::VolumeSwitch { index, state: v });
        }
    }
    for (index, key) in [(0, "initMessageSpeed"), (7, "initAutoSpeed")] {
        let v = n(table, key, -1)?;
        if v >= 0 {
            out.push(Command::Parameter(PARAMETERS[index], v));
            slider(&mut out, layout, index, v);
        }
    }
    let v = n(table, "initSkipMode", -1)?;
    if v >= 0 {
        out.push(Command::Parameter(2, v));
        if n(table, "skipModeButtonExistFlag", 1)? != 0 {
            out.push(Command::SkipRadio(v));
        }
    }
    let chara = n(table, "voiceCutButtonNumber", 0)?;
    ensure!(
        (0..=256).contains(&chara),
        "NNN Config reset character button budget"
    );
    let volume_enabled = n(table, "charaVoiceVolumeFlag", 0)? != 0;
    let min = n(table, "charaVoiceVolumeMin", -15)?.clamp(-15, 15);
    let v = n(table, "initCharaVoiceVolume", -1)?;
    if v >= 0 && volume_enabled {
        for index in 0..chara as usize {
            out.push(Command::CharacterSlider { index, gaze: v });
            out.push(Command::Parameter(500 + index as i32, v.wrapping_add(min)));
        }
    }
    let v = n(table, "initCharaVoiceOff", -1)?;
    if v >= 0 && volume_enabled {
        for index in 0..chara as usize {
            // Source tests ==0, then sets raw VoiceFlag TRUE and button state1.
            // Its nearby 'off' label contradicts CheckVoiceFlag/SetVoiceFlag;
            // retain actual flag behavior instead of reversing the Boolean.
            out.push(Command::CharacterEnabled {
                index,
                enabled: v == 0,
            });
        }
    }
    let v = n(table, "initAnimeSwitch", -1)?;
    if v >= 0 && n(table, "expRadioNumber", 0)? > 0 {
        out.push(Command::Parameter(300, v));
        out.push(Command::AnimeSwitch(v));
    }
    Ok(out)
}
