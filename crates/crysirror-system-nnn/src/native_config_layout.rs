// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig's original control catalogue. This is native setup/state data,
//! not a widget implementation: capture, hit-testing, images and audio belong
//! to shared services. Original constructor/Init/Print determine this ordering.
use crate::{
    config::Names,
    native_bootstrap::NativeConfiguration,
    native_radio_group::{RadioButtonSpec, RadioSpec},
};
use anyhow::{Result, ensure};
pub(crate) mod allocation;
pub(crate) mod appearance;
pub(crate) mod characters;
pub(crate) mod controls;
pub(crate) mod pictures;
pub(crate) mod reset;
pub(crate) mod sample;
pub(crate) mod session;
pub(crate) mod window;
#[derive(Clone, Copy, Debug)]
pub(crate) enum Action {
    Back,
    ScreenMode,
    Gore,
    PageStep(i32),
    Page(usize),
    Parameter { id: i32, value: i32 },
    Toggle(i32),
    Character { slot: usize, toggle: bool },
    SystemVoice(usize),
    Click(usize),
}
#[derive(Clone, Debug)]
pub(crate) enum Buttons {
    Single(RadioButtonSpec),
    Check([RadioButtonSpec; 2]),
    Radio(RadioSpec),
}
#[derive(Clone, Debug)]
pub(crate) struct Group {
    /// Source page numbers are one-based. Values outside 1..pageMax deliberately
    /// remain non-visible, rather than being reassigned to the first page.
    pub page: Option<i32>,
    pub action: Action,
    pub buttons: Buttons,
}
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub pages: Vec<Vec<u8>>,
    pub page_mode: i32,
    pub force_first_page: bool,
    pub page_variable: Option<u16>,
    pub restart_script_on_page: bool,
    pub sliders: crate::native_config_volume::Profile,
    pub groups: Vec<Group>,
    pub characters: Vec<characters::Character>,
    pub entry_wait: i32,
    pub appearance_frames: i32,
    pub appearance_type: i32,
    pub in_variable: Option<(u16, i32)>,
    pub out_variable: Option<(u16, i32)>,
}
fn n(table: &Names, key: &str, default: i32) -> Result<i32> {
    Ok(table.integer(key.as_bytes())?.unwrap_or(default))
}
fn variable(table: &Names, config: &NativeConfiguration, key: &str) -> Option<u16> {
    table
        .parameter(key.as_bytes())
        .and_then(|name| config.variable_names.index(name))
}
fn single(
    table: &Names,
    prefix: &str,
    config: &NativeConfiguration,
    old: Option<&RadioButtonSpec>,
    delta: [i32; 2],
) -> Result<RadioButtonSpec> {
    // CommonButton carries these input flags independently of its atlas. The
    // Config owner consumes their exact roles; the Title-only parser must not
    // reject them or accidentally substitute Title actions.
    let filtered = Names {
        tokens: table
            .tokens
            .chunks_exact(2)
            .filter(|pair| {
                ![
                    "IsCancelButton",
                    "IsUpButton",
                    "IsDownButton",
                    "UpDownReverse",
                ]
                .iter()
                .any(|suffix| pair[0].eq_ignore_ascii_case(format!("{prefix}{suffix}").as_bytes()))
            })
            .flat_map(|pair| pair.iter().cloned())
            .collect(),
    };
    let button = crate::native_title::read_menu_button(
        &filtered,
        prefix,
        config,
        old.map(|old| &old.button),
        delta,
    )?;
    let get = |key: &str, default: i32| n(table, &format!("{prefix}{key}"), default);
    let mut raw_sounds = old.map(|old| old.raw_sounds).unwrap_or([0; 4]);
    let mut voices = old.map(|old| old.voices).unwrap_or([0; 4]);
    for (i, suffix) in ["ClickSound", "BadSound", "EnterSound", "ExitSound"]
        .iter()
        .enumerate()
    {
        raw_sounds[i] = get(suffix, raw_sounds[i])?;
    }
    for (i, suffix) in ["ClickVoice", "BadVoice", "EnterVoice", "ExitVoice"]
        .iter()
        .enumerate()
    {
        voices[i] = get(suffix, voices[i])?;
    }
    Ok(RadioButtonSpec {
        button,
        raw_sounds,
        voices,
        volume_type: get("VolumeType", old.map(|old| old.volume_type).unwrap_or(0))?,
        cancel: get("IsCancelButton", 0)? != 0,
        wheel: [get("IsUpButton", 0)? != 0, get("IsDownButton", 0)? != 0],
        wheel_reverse: get("UpDownReverse", 0)? != 0,
    })
}
fn check(
    table: &Names,
    prefix: &str,
    config: &NativeConfiguration,
    old: Option<&RadioButtonSpec>,
) -> Result<[RadioButtonSpec; 2]> {
    let first = single(
        table,
        &format!("{prefix}1"),
        config,
        old,
        old.map(|old| old.button.size).unwrap_or([0, 0]),
    )?;
    // CCommonMultiStateButton resets an inherited position to orgPoint when
    // the second state has no own PrintX/Y. Atlas/source inheritance remains.
    let second = single(table, &format!("{prefix}2"), config, Some(&first), [0, 0])?;
    Ok([first, second])
}
impl Group {
    pub(crate) fn paint_order(&self) -> u8 {
        match self.action {
            Action::Character { .. } => 0,
            Action::ScreenMode | Action::Parameter { id: 2 | 6, .. } => 1,
            Action::Toggle(210..=214) | Action::Gore => 2,
            Action::Click(_) => 3,
            Action::SystemVoice(_) => 4,
            Action::Parameter { id: 300..=315, .. } => 5,
            Action::Toggle(400..=415) => 6,
            Action::Toggle(_) => 7,
            Action::Back | Action::PageStep(_) | Action::Page(_) => 8,
            Action::Parameter { .. } => 7,
        }
    }
    pub(crate) fn calcu_order(&self) -> u8 {
        match self.action {
            Action::Back | Action::PageStep(_) | Action::Page(_) => 0,
            Action::Character { .. } => 1,
            Action::ScreenMode | Action::Parameter { id: 2 | 6, .. } => 2,
            Action::Toggle(210..=214) | Action::Gore => 3,
            Action::Parameter { id: 300..=315, .. } | Action::Toggle(400..=415) => 4,
            Action::SystemVoice(_) => 5,
            Action::Click(_) => 6,
            _ => 7,
        }
    }
}
impl Layout {
    pub(crate) fn read(
        table: &Names,
        config: &NativeConfiguration,
        system_voices: usize,
    ) -> Result<Self> {
        let count = n(table, "modePageMax", 1)?;
        ensure!((1..=100).contains(&count), "NNN Config native page budget");
        let page_mode = n(table, "pageMode", i32::from(count > 1))?;
        ensure!(
            (0..=2).contains(&page_mode),
            "NNN Config native page selector"
        );
        let mut file = table
            .parameter(b"filenameBG")
            .unwrap_or(b"bg_config")
            .to_vec();
        let mut pages = Vec::with_capacity(count as usize);
        for page in 1..=count {
            if let Some(name) = table.parameter(format!("filenameBG{page}").as_bytes()) {
                file = name.to_vec();
            }
            ensure!(
                file.len() <= 254 && !file.contains(&0),
                "NNN Config background name bounds"
            );
            pages.push(file.clone());
        }
        // This is the reviewed DirectSound/native profile. CPAL being the
        // output device does not imply the source's XAudio2 setup branch.
        let sliders = crate::native_config_volume::Profile::read(
            table,
            false,
            config.module_sha256 == crate::native_bootstrap::MUGEN2_MODULE_SHA256,
        )?;
        let mut groups = Vec::new();
        let characters = characters::read(table, config)?;
        for (index, character) in characters.iter().enumerate() {
            groups.push(Group {
                page: Some(character.page),
                action: Action::Character {
                    slot: index,
                    toggle: true,
                },
                buttons: Buttons::Check(character.buttons.clone()),
            });
        }
        let back_type = n(table, "backButtonType", 0)?;
        ensure!(
            (0..=1).contains(&back_type),
            "NNN Config back-button ownership profile"
        );
        let common_back = back_type == 0 && n(table, "useCommonBackSetup", 0)? != 0;
        let back_setup = if common_back || back_type == 1 {
            &config.setup
        } else {
            table
        };
        let mut back = single(back_setup, "back", config, None, [0, 0])?;
        back.cancel = true;
        back.button.origin = [
            n(
                table,
                "backPrintX",
                if common_back {
                    n(back_setup, "backPrintX", 0)?
                } else {
                    0
                },
            )?,
            n(
                table,
                "backPrintY",
                if common_back {
                    n(back_setup, "backPrintY", 0)?
                } else {
                    0
                },
            )?,
        ];
        groups.push(Group {
            page: None,
            action: Action::Back,
            buttons: Buttons::Single(back),
        });
        if page_mode == 1 {
            let updown_type = n(table, "updownButtonType", 0)?;
            // Preserve the source's unusual else-if(backButtonType==1),
            // rather than inferring that every nonzero updown type is global.
            ensure!(
                updown_type == 0 || back_type == 1,
                "NNN Config original up/down owner is undefined"
            );
            let common_updown = updown_type == 0 && n(table, "useCommonUpDownSetup", 0)? != 0;
            let common = if common_updown || updown_type != 0 {
                &config.setup
            } else {
                table
            };
            let mut up = single(common, "up", config, None, [0, 0])?;
            let mut down = single(common, "down", config, Some(&up), up.button.size)?;
            up.wheel[0] = true;
            up.button.origin = [
                n(
                    table,
                    "upPrintX",
                    if common_updown {
                        n(common, "upPrintX", 0)?
                    } else {
                        0
                    },
                )?,
                n(
                    table,
                    "upPrintY",
                    if common_updown {
                        n(common, "upPrintY", 0)?
                    } else {
                        0
                    },
                )?,
            ];
            down.wheel[1] = true;
            down.button.origin = [
                n(
                    table,
                    "downPrintX",
                    if common_updown {
                        n(common, "downPrintX", 0)?
                    } else {
                        0
                    },
                )?,
                n(
                    table,
                    "downPrintY",
                    if common_updown {
                        n(common, "downPrintY", 0)?
                    } else {
                        0
                    },
                )?,
            ];
            groups.push(Group {
                page: None,
                action: Action::PageStep(-1),
                buttons: Buttons::Single(up),
            });
            groups.push(Group {
                page: None,
                action: Action::PageStep(1),
                buttons: Buttons::Single(down),
            });
        } else if page_mode == 2 {
            groups.push(Group {
                page: None,
                action: Action::Page(0),
                buttons: Buttons::Radio(RadioSpec::read(table, "tab", count as usize, config)?),
            });
        }
        for (tag, id) in [("screen", 4), ("skip", 2), ("auto", 6)] {
            if n(table, &format!("{tag}ModeButtonExistFlag"), 1)? != 0 {
                groups.push(Group {
                    page: Some(n(table, &format!("{tag}ModeButtonPrintPage"), 1)?),
                    action: if id == 4 {
                        Action::ScreenMode
                    } else {
                        Action::Parameter { id, value: 0 }
                    },
                    buttons: Buttons::Radio(RadioSpec::read(table, tag, 2, config)?),
                });
            }
        }
        for (index, tag) in [
            "autoContinue",
            "autoSkip",
            "changeReadColor",
            "changeSelectColor",
            "continueVoice",
        ]
        .iter()
        .enumerate()
        {
            if n(table, &format!("{tag}CheckButtonExistFlag"), 0)? != 0 {
                groups.push(Group {
                    page: Some(n(table, &format!("{tag}CheckButtonPrintPage"), 1)?),
                    action: Action::Toggle(210 + index as i32),
                    buttons: Buttons::Check(check(table, &format!("{tag}Check"), config, None)?),
                });
            }
        }
        if n(&config.setup, "useGoreFlag", 0)? != 0 {
            groups.push(Group {
                page: Some(n(table, "goreButtonPrintPage", -1)?),
                action: Action::Gore,
                buttons: Buttons::Check(check(table, "goreButton", config, None)?),
            });
        }
        let mut inherited_volume = None;
        for (index, tag) in [
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
        ]
        .iter()
        .enumerate()
        .take(sliders.sliders.len())
        {
            if !sliders.sliders[index].exists || !((1..=5).contains(&index) || index >= 8) {
                continue;
            }
            let mut pair = check(
                table,
                &format!("{tag}Button"),
                config,
                if index > 1 {
                    inherited_volume.as_ref()
                } else {
                    None
                },
            )?;
            let position = [
                n(
                    table,
                    &format!("{tag}ButtonPrintX"),
                    sliders.sliders[index].origin[0] - pair[0].button.size[0],
                )?,
                n(
                    table,
                    &format!("{tag}ButtonPrintY"),
                    sliders.sliders[index].origin[1],
                )?,
            ];
            for button in &mut pair {
                button.button.origin = position;
            }
            inherited_volume = Some(pair[1].clone());
            groups.push(Group {
                page: Some(sliders.sliders[index].page),
                action: Action::Toggle(crate::native_config_volume::SWITCHES[index]),
                buttons: Buttons::Check(pair),
            });
        }
        for (key, default, prefix, id, radio) in [
            ("expRadioNumber", 0, "Radio", 300, true),
            ("expCheckNumber", 0, "Check", 400, false),
        ] {
            let number = n(table, key, default)?;
            ensure!(
                (0..=16).contains(&number),
                "NNN Config extension record bounds"
            );
            for index in 0..number {
                let tag = format!("exp{}{prefix}", index + 1);
                let buttons = if radio {
                    let count = n(table, &tag, 2)?;
                    ensure!(
                        (1..=128).contains(&count),
                        "NNN Config extra radio button budget"
                    );
                    Buttons::Radio(RadioSpec::read(table, &tag, count as usize, config)?)
                } else {
                    Buttons::Check(check(table, &tag, config, None)?)
                };
                groups.push(Group {
                    page: Some(n(table, &format!("{tag}PrintPage"), 1)?),
                    action: if radio {
                        Action::Parameter {
                            id: id + index,
                            value: 0,
                        }
                    } else {
                        Action::Toggle(id + index)
                    },
                    buttons,
                });
            }
        }
        let clicks = n(table, "clickNumber", 0)?;
        ensure!(
            (0..=128).contains(&clicks),
            "NNN Config click button budget"
        );
        for index in 0..clicks {
            let tag = format!("click{}", index + 1);
            groups.push(Group {
                page: Some(n(table, &format!("{tag}PrintPage"), 1)?),
                action: Action::Click(index as usize),
                buttons: Buttons::Single(single(table, &tag, config, None, [0, 0])?),
            });
        }
        ensure!(
            system_voices <= 64,
            "NNN Config system voice catalogue budget"
        );
        if system_voices > 0 {
            groups.push(Group {
                page: Some(n(table, "systemVoiceButtonPrintPage", 1)?),
                action: Action::SystemVoice(0),
                buttons: Buttons::Radio(RadioSpec::read(
                    table,
                    "systemVoice",
                    system_voices + 1,
                    config,
                )?),
            });
        }
        let assignment = |name: &str, value: &str| -> Result<Option<(u16, i32)>> {
            variable(table, config, name)
                .map(|id| Ok((id, n(table, value, 0)?)))
                .transpose()
        };
        Ok(Self {
            pages,
            page_mode,
            force_first_page: n(table, "mustStartPage1Flag", 0)? != 0,
            page_variable: if n(table, "pageNumberSetFlag", 0)? != 0 {
                config.variable_names.index(
                    table
                        .parameter(b"pageNumberVarName")
                        .unwrap_or(b"\x83R\x83\x93\x83t\x83B\x83O\x83y\x81[\x83W"),
                )
            } else {
                None
            },
            restart_script_on_page: n(table, "changePageRestartScriptFlag", 0)? != 0,
            sliders,
            groups,
            characters,
            entry_wait: n(table, "menuStartWaitTime", 0)?,
            appearance_frames: n(table, "menuStartEffectTime", 0)?,
            appearance_type: n(table, "menuStartEffectType", 0)?,
            in_variable: assignment("inSetVarName", "inSetVarData")?,
            out_variable: assignment("outSetVarName", "outSetVarData")?,
        })
    }
    pub(crate) fn page(&self, page: usize) -> Result<impl Iterator<Item = &Group>> {
        ensure!(page < self.pages.len(), "NNN Config current page bounds");
        Ok(self.groups.iter().filter(move |group| {
            group
                .page
                .is_none_or(|number| i64::from(number) == page as i64 + 1)
        }))
    }
}
