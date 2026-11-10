// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original Title/button configuration and frame policy. Pixels, clipping,
//! interpolation, hit masks and retained input belong to shared engine APIs.
use crate::{
    config::Names, native_bootstrap::NativeConfiguration, native_resources::ImageLookup,
    native_shell::NativeStartupProfile,
};
use anyhow::{ensure, Context, Result};
mod system_intro;
mod general_entry;
#[derive(Clone, Debug)]
pub struct NativeButtonSpec {
    pub source: ImageLookup,
    pub size: [i32; 2],
    pub origin: [i32; 2],
    pub states: [[i32; 2]; 4],
    pub check_origin: [i32; 2],
    pub check_delta: [i32; 2],
    pub check_size: [i32; 2],
    pub patterns: [u32; 4],
    pub speeds: [u32; 4],
    pub types: [u8; 4],
    pub vertical_first: bool,
    pub print_mode: u8,
    pub sounds: [i32; 4], // click, cannot-click, enter, exit
    pub sound_waits: [u32; 2],
    pub click_frames: u32,
    pub accelerators: [u8; 3],
    pub enabled: bool,
    pub exists: bool,
}
#[derive(Clone, Debug)]
pub struct NativeTitleSpec {
    pub(crate) entry:Option<crate::native_general_entry::ModeEntry>,
    pub buttons: Vec<NativeButtonSpec>,
    pub menu_wait: u32,
    pub menu_anime_adjust: bool,
    pub menu_fade: u32,
    pub menu_effect: u8,
    pub menu_delta: [i32; 2],
    pub auto_restart: u32,
    pub restart_next: crate::native_shell::NativeMode,
    pub script_end_exits: bool,
    pub music_number: i32,
    pub music_loop: i32,
    pub music_volume: i32,
    pub music_fade_in: i32,
    pub music_restart: bool,
}
pub(crate) enum MenuRequest {
    Mode(i32),
    Script { variable: Option<u16>, value: i32 },
}
pub(crate) fn extension_request(
    table: &Names,
    configuration: &NativeConfiguration,
    index: usize,
    title: bool,
) -> Result<MenuRequest> {
    let ordinary = if title { 5 } else { 4 };
    let basic = usize::try_from(table.integer(b"basicButtonNumber")?.unwrap_or(ordinary))?;
    let (key, value) = if index < basic {
        (
            format!("basicButtonSpecialMode{}", index + 1),
            -i32::try_from(
                index
                    .checked_sub(ordinary as usize)
                    .context("NNN ordinary menu action passed as extension")?
                    + 1,
            )?,
        )
    } else {
        // Legacy Title counts from five, Omake from basicButtonNumber.
        let offset = index
            .checked_sub(if title { 5 } else { basic })
            .context("NNN native menu extension index")?;
        (
            format!("extButtonSpecialMode{}", offset + 1),
            i32::try_from(offset + 1)?,
        )
    };
    let mode = table.integer(key.as_bytes())?.unwrap_or(-1);
    if mode > 0 {
        return Ok(MenuRequest::Mode(mode));
    }
    let variable = table
        .parameter(b"extModeReturnVarName")
        .and_then(|name| configuration.variable_names.index(name));
    Ok(MenuRequest::Script { variable, value })
}
fn integer(table: &Names, prefix: &str, key: &str, default: i32) -> Result<i32> {
    Ok(table
        .integer(format!("{prefix}{key}").as_bytes())?
        .unwrap_or(default))
}
fn nonnegative(value: i32, what: &str) -> Result<u32> {
    ensure!((0..=864000).contains(&value), "NNN Title {what} range");
    Ok(value as u32)
}
impl NativeTitleSpec {
    pub(crate) fn selected_variant(
        table: &Names,
        configuration: &NativeConfiguration,
        variables: &crate::expression::Variables,
    ) -> Result<usize> {
        let count = table.integer(b"extModeNumber")?.unwrap_or(0);
        ensure!((0..=64).contains(&count), "NNN menu variant budget");
        let value = table
            .parameter(b"extModeCheckVarName")
            .and_then(|name| configuration.variable_names.index(name))
            // CCommonTitle/Omake::Init call GetVarData, not GetCalcuVarData.
            // Sampling a menu variant must not draw from the shared RNG.
            .map(|index| variables.read_stored(index))
            .unwrap_or(0);
        Ok(value.clamp(0, count) as usize)
    }
    pub fn from_profile(
        profile: &NativeStartupProfile,
        configuration: &NativeConfiguration,
    ) -> Result<Self> {
        Self::from_table(&profile.title, configuration)
    }
    pub(crate) fn from_table(table: &Names, configuration: &NativeConfiguration) -> Result<Self> {
        Self::from_table_variant(table, configuration, 0)
    }
    pub(crate) fn from_table_variant(
        table: &Names,
        configuration: &NativeConfiguration,
        variant: usize,
    ) -> Result<Self> {
        crate::native_general::ModeReturn::validate_skinned_entry(table)?;
        let get = |key: &str, default: i32| integer(table, "", key, default);
        let variants = usize::try_from(get("extModeNumber", 0)?)?;
        ensure!(variants <= 64, "NNN menu variant budget");
        let variant = variant.min(variants);
        ensure!(
            get("miniGameFlag", 0)? == 0
                && get("htmlButtonEnable", 0)? == 0
                && get("lastLoadButtonEnable", 0)? == 0,
            "NNN Title additional native controls require their original action policy"
        );
        ensure!(
            get("NoSaveDataToMaskMenu", 0)? == 0,
            "NNN Title save-presence menu masking requires original save catalogue"
        );
        let basic = get("basicButtonNumber", 5)?;
        let extra = if variants > 0 {
            get("extButtonNumber", 1)?
        } else {
            0
        };
        let count = basic
            .checked_add(extra)
            .context("NNN menu button count overflow")?;
        ensure!(
            basic > 0 && extra >= 0 && (1..=64).contains(&count),
            "NNN menu button count"
        );
        let effect = get("menuStartEffectType", 0)?;
        ensure!(
            (0..=1).contains(&effect),
            "NNN Title non-opacity menu appearance requires original directional policy"
        );
        let next = [
            get("menuNextX", 0)?,
            get("menuNextY", get("menu1Button1SizeY", 16)?)?,
        ];
        ensure!(
            next.iter().all(|v| (-32767..=32767).contains(v)),
            "NNN Title inherited position delta"
        );
        let mut previous: Vec<NativeButtonSpec> = Vec::new();
        for menu in 0..=variant {
            let mut buttons: Vec<NativeButtonSpec> = Vec::new();
            for index in 0..count as usize {
                let prefix = format!("menu{}Button{}", menu + 1, index + 1);
                let old = if menu == 0 {
                    buttons.last()
                } else {
                    previous.get(index)
                };
                let delta = if menu == 0 { next } else { [0, 0] };
                let mut button = read_menu_button(table, &prefix, configuration, old, delta)?;
                let default_exists = if menu == 0 {
                    index < basic as usize
                } else if menu == 1 && index >= basic as usize {
                    true
                } else {
                    previous[index].exists
                };
                button.exists =
                    integer(table, &prefix, "ExistFlag", i32::from(default_exists))? != 0;
                let default_enabled = if menu == 0 {
                    button.exists
                } else if button.exists && index < basic as usize {
                    previous[index].enabled
                } else {
                    button.exists
                };
                button.enabled =
                    integer(table, &prefix, "EnableFlag", i32::from(default_enabled))? != 0;
                buttons.push(button);
            }
            previous = buttons;
        }
        let buttons = previous;
        Ok(Self {
            entry:crate::native_general_entry::ModeEntry::read(table)?,
            buttons,
            menu_wait: nonnegative(get("menuStartWaitTime", 0)?, "menu wait")?,
            menu_anime_adjust: get("menuAnimeAdjustFlag", 0)? != 0,
            menu_fade: nonnegative(get("menuStartEffectTime", 0)?, "menu appearance time")?,
            menu_effect: effect as u8,
            menu_delta: [get("menuStartDeltaX", 0)?, get("menuStartDeltaY", 0)?],
            auto_restart: nonnegative(get("autoRestartTime", 0)?, "auto restart")?,
            restart_next: table
                .parameter(b"restartNextMode")
                .map(crate::native_shell::NativeMode::named)
                .transpose()?
                .unwrap_or(crate::native_shell::NativeMode::Nothing),
            script_end_exits: get("scriptEndIsModeEnd", 0)? != 0,
            music_number: get("bgmNumber", 1)?,
            music_loop: get("bgmLoop", 0)?,
            music_volume: get("bgmVolume", 0)?,
            music_fade_in: get("bgmFadeInTime", 0)?,
            music_restart: get("bgmRestartFlag", 1)? != 0,
        })
    }
}
// CMenuButtonSetup copies row zero from the preceding button with menuNext;
// subsequent variants copy the same column of the preceding variant, without
// positional deltas. CSuperButtonSetup deliberately does not inherit accel keys
// and copies EnterWait into both waits; retain those source details here.
fn read_menu_button(
    table: &Names,
    prefix: &str,
    configuration: &NativeConfiguration,
    old: Option<&NativeButtonSpec>,
    delta: [i32; 2],
) -> Result<NativeButtonSpec> {
    let levels = u32::try_from(configuration.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
    let get = |key: &str, default: i32| integer(table, &prefix, key, default);
    let mut size = old.map(|v| v.size).unwrap_or([16, 16]);
    let mut origin = old
        .map(|v| [v.origin[0] + delta[0], v.origin[1] + delta[1]])
        .unwrap_or([0, 0]);
    for axis in 0..2 {
        size[axis] = get(["SizeX", "SizeY"][axis], size[axis])?;
        origin[axis] = get(["PrintX", "PrintY"][axis], origin[axis])?;
    }
    ensure!(
        size.iter().all(|n| (1..=8192).contains(n))
            && origin.iter().all(|n| (-32767..=32767).contains(n)),
        "NNN Title button geometry"
    );
    let inherited = old.map(|v| v.source.clone());
    let source = if let Some(file) = table.parameter(format!("{prefix}FileName").as_bytes()) {
        configuration
            .resources
            .picture(&[b"sys\\".as_slice(), file].concat(), levels)?
    } else {
        inherited.context("NNN Title original button filename absent")?
    };
    let mut states = old.map(|v| v.states).unwrap_or([[0; 2]; 4]);
    let mut patterns = old.map(|v| v.patterns).unwrap_or([1; 4]);
    let mut speeds = old.map(|v| v.speeds).unwrap_or([1; 4]);
    let mut types = old.map(|v| v.types).unwrap_or([1; 4]);
    for mode in 0..4 {
        if old.is_none() {
            states[mode] = [size[0] * mode as i32, 0];
            if mode > 0 {
                patterns[mode] = patterns[mode - 1];
                speeds[mode] = speeds[mode - 1];
                types[mode] = types[mode - 1];
            }
        }
        for axis in 0..2 {
            states[mode][axis] = get(
                &format!("PicSrc{}{}", ["X", "Y"][axis], mode + 1),
                states[mode][axis],
            )?;
        }
        let pattern = get(&format!("AnimePattern{}", mode + 1), patterns[mode] as i32)?;
        ensure!(
            (1..=4096).contains(&pattern),
            "NNN Title animation pattern bounds"
        );
        patterns[mode] = pattern as u32;
        speeds[mode] = get(&format!("AnimeSpeed{}", mode + 1), speeds[mode] as i32)?.max(1) as u32;
        ensure!(speeds[mode] <= 864000, "NNN Title animation speed");
        let kind = get(&format!("AnimeType{}", mode + 1), i32::from(types[mode]))?;
        ensure!((1..=8).contains(&kind), "NNN Title animation type");
        types[mode] = kind as u8;
        ensure!(
            !matches!(kind, 3 | 4 | 7 | 8) || patterns[mode] >= 2,
            "NNN Title degenerate reverse animation needs signed native index policy"
        );
        let span = if matches!(kind, 3 | 4 | 7 | 8) {
            (patterns[mode] - 1).saturating_mul(2).max(1)
        } else {
            patterns[mode]
        };
        ensure!(
            span.checked_mul(speeds[mode])
                .is_some_and(|n| n <= i32::MAX as u32),
            "NNN Title animation cycle overflow"
        );
    }
    let mut check_origin = old.map(|v| v.check_origin).unwrap_or([0, 0]);
    let mut check_delta = old.map(|v| v.check_delta).unwrap_or([0, 0]);
    let mut check_size = size;
    for axis in 0..2 {
        check_origin[axis] = get(["CheckPicX", "CheckPicY"][axis], check_origin[axis])?;
        check_delta[axis] = get(["CheckDeltaX", "CheckDeltaY"][axis], check_delta[axis])?;
        check_size[axis] = get(["CheckSizeX", "CheckSizeY"][axis], check_size[axis])?;
    }
    ensure!(
        check_size.iter().all(|n| (1..=8192).contains(n)),
        "NNN Title check geometry"
    );
    ensure!(
        check_origin.iter().all(|v| (0..=8192).contains(v))
            && check_delta.iter().all(|v| (-32767..=32767).contains(v)),
        "NNN Title hit source/delta bounds"
    );
    let mut sounds = old.map(|v| v.sounds).unwrap_or([0; 4]);
    for i in 0..4 {
        sounds[i] = get(
            ["ClickSound", "BadSound", "EnterSound", "ExitSound"][i],
            sounds[i],
        )?;
        // MakeButtonStatus encodes only strictly positive sound IDs. Negative
        // configuration sentinels have no audio request, just like zero.
        if sounds[i] < 0 { sounds[i]=0; }
    }
    for key in [
        "ClickVoice",
        "BadVoice",
        "EnterVoice",
        "ExitVoice",
        "VolumeType",
        "IsCancelButton",
        "IsUpButton",
        "IsDownButton",
        "UpDownReverse",
    ] {
        ensure!(
            get(key, 0)? == 0,
            "NNN Title optional button {key} requires original semantics"
        );
    }
    let old_waits = old
        .map(|v| [v.sound_waits[0], v.sound_waits[0]])
        .unwrap_or([0; 2]);
    let sound_waits = [
        nonnegative(get("EnterWait", old_waits[0] as i32)?, "enter sound delay")?,
        nonnegative(get("ExitWait", old_waits[1] as i32)?, "exit sound delay")?,
    ];
    let mut accelerators = [0; 3];
    for (i, key) in accelerators.iter_mut().enumerate() {
        if let Some(raw) = table.parameter(format!("{prefix}AccelKey{}", i + 1).as_bytes()) {
            let number = std::str::from_utf8(raw)
                .ok()
                .and_then(|v| v.parse::<i32>().ok())
                .filter(|n| *n != 0 || raw == b"0")
                .unwrap_or_else(|| i32::from(raw.first().copied().unwrap_or(0)));
            *key = u8::try_from(number)?;
        }
    }
    let print_mode = get(
        "PrintMode",
        old.map(|v| i32::from(v.print_mode)).unwrap_or(2),
    )?;
    ensure!((0..=2).contains(&print_mode), "NNN Title print mode");
    Ok(NativeButtonSpec {
        source,
        size,
        origin,
        states,
        check_origin,
        check_delta,
        check_size,
        patterns,
        speeds,
        types,
        vertical_first: get(
            "TateYoko",
            old.map(|v| if v.vertical_first { 0 } else { 1 })
                .unwrap_or(0),
        )? == 0,
        print_mode: print_mode as u8,
        sounds,
        sound_waits,
        click_frames: nonnegative(
            get("ClickTime", old.map(|v| v.click_frames as i32).unwrap_or(8))?,
            "click time",
        )?,
        accelerators,
        exists: true,
        enabled: true,
    })
}
#[derive(Clone, Debug)]
pub struct NativeButtonState {
    pub mode: usize,
    pub count: u32,
    pub animation: [u32; 4],
    entered: bool,
    exited: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeButtonSignal {
    None,
    ClickStarted,
    Selected,
    Entered,
    Exited,
    Sound(i32),
}
impl NativeButtonState {
    pub fn new(spec: &NativeButtonSpec) -> Self {
        Self {
            mode: if spec.enabled { 0 } else { 3 },
            count: 0,
            animation: [0; 4],
            entered: false,
            exited: false,
        }
    }
    /// CCommonButton::Calcu(NULL) suspends its timer and resets only the
    /// mode/other animation cells. It is not an ordinary unhover event.
    fn calcu_without_input(&mut self, spec: &NativeButtonSpec) {
        if !spec.exists { return; }
        self.mode = if spec.enabled {0} else {3};
        for (mode,count) in self.animation.iter_mut().enumerate() {
            if mode != self.mode { *count=0; }
        }
    }
    /// Pointer ownership/hit coverage is supplied by the retained engine widget.
    pub fn calcu(
        &mut self,
        spec: &NativeButtonSpec,
        hovered: bool,
        pressed: bool,
    ) -> NativeButtonSignal {
        if !spec.exists {
            return NativeButtonSignal::None;
        }
        if !spec.enabled {
            self.mode = 3;
            self.animation.iter_mut().take(3).for_each(|n| *n = 0);
            return if pressed && spec.sounds[1] > 0 {NativeButtonSignal::Sound(spec.sounds[1])} else {NativeButtonSignal::None};
        }
        let signal = if self.mode == 2 {
            self.count = self.count.saturating_add(1);
            if self.count >= spec.click_frames {
                NativeButtonSignal::Selected
            } else {
                NativeButtonSignal::None
            }
        } else if pressed {
            self.mode = 2;
            self.count = 0;
            if spec.click_frames == 0 {
                NativeButtonSignal::Selected
            } else {
                NativeButtonSignal::ClickStarted
            }
        } else {
            self.count = self.count.saturating_add(1);
            match (self.mode, hovered) {
                (0, true) => {
                    self.mode = 1;
                    self.count = 0;
                    self.entered = true;
                    NativeButtonSignal::Entered
                }
                (1, false) => {
                    self.mode = 0;
                    self.count = 0;
                    self.exited = true;
                    NativeButtonSignal::Exited
                }
                (1, true) if self.entered && self.count == spec.sound_waits[0] && spec.sounds[2] > 0 => NativeButtonSignal::Sound(spec.sounds[2]),
                (0, false) if self.exited && self.count == spec.sound_waits[1] && spec.sounds[3] > 0 => NativeButtonSignal::Sound(spec.sounds[3]),
                _ => NativeButtonSignal::None,
            }
        };
        for (mode, count) in self.animation.iter_mut().enumerate() {
            if mode != self.mode {
                *count = 0;
            }
        }
        signal
    }
    pub fn frame(&self, spec: &NativeButtonSpec) -> ([u32; 2], u16) {
        let mode = self.mode;
        let kind = spec.types[mode];
        let speed = spec.speeds[mode].max(1);
        let pattern = spec.patterns[mode];
        let reverse = matches!(kind, 3 | 4 | 7 | 8);
        let hold = matches!(kind, 2 | 4 | 6 | 8);
        let span = if reverse {
            (pattern - 1).saturating_mul(2).max(1)
        } else {
            pattern
        };
        let mut a = self.animation[mode] / speed;
        let mut b = a + 1;
        if hold {
            let last = if reverse { span } else { span - 1 };
            if a >= span {
                a = last
            }
            if b >= span {
                b = last
            }
        }
        b %= span;
        if reverse {
            if a >= pattern {
                a = (pattern - 1) * 2 - a
            }
            if b >= pattern {
                b = (pattern - 1) * 2 - b
            }
        }
        let percent = if kind >= 5 {
            100 * (speed - self.animation[mode] % speed) / speed
        } else {
            100
        };
        ([a, b], percent as u16)
    }
    pub fn printed(&mut self, spec: &NativeButtonSpec) {
        let kind = spec.types[self.mode];
        let pattern = spec.patterns[self.mode];
        let reverse = matches!(kind, 3 | 4 | 7 | 8);
        let frames = if reverse {
            (pattern - 1).saturating_mul(2).max(1)
        } else {
            pattern
        } * spec.speeds[self.mode].max(1);
        let next = self.animation[self.mode].saturating_add(1);
        self.animation[self.mode] = if matches!(kind, 2 | 4 | 6 | 8) {
            next.min(frames)
        } else {
            next % frames
        };
    }
}
impl NativeButtonSpec {
    pub(crate) fn from_named_setup(
        table: &Names,
        prefix: &str,
        configuration: &NativeConfiguration,
    ) -> Result<Self> {
        let mut translated = Names {
            tokens: vec![b"basicButtonNumber".to_vec(), b"1".to_vec()],
        };
        for pair in table.tokens.chunks_exact(2) {
            if pair[0]
                .get(..prefix.len())
                .is_some_and(|key| key.eq_ignore_ascii_case(prefix.as_bytes()))
            {
                translated
                    .tokens
                    .push([b"menu1Button1".as_slice(), &pair[0][prefix.len()..]].concat());
                translated.tokens.push(pair[1].clone());
            }
        }
        let mut spec = NativeTitleSpec::from_table(&translated, configuration)?
            .buttons
            .remove(0);
        if !spec.vertical_first {
            for state in 0..4 {
                for axis in 0..2 {
                    if translated
                        .parameter(
                            format!("menu1Button1PicSrc{}{}", ["X", "Y"][axis], state + 1)
                                .as_bytes(),
                        )
                        .is_none()
                    {
                        spec.states[state][axis] = if axis == 0 {
                            0
                        } else {
                            spec.size[1] * state as i32
                        };
                    }
                }
            }
        }
        Ok(spec)
    }
    pub fn source_rect(&self, mode: usize, pattern: u32, image: [u32; 2]) -> Result<[i32; 4]> {
        ensure!(mode < 4, "NNN Title button state");
        let [w, h] = self.size.map(i64::from);
        let [mut x, mut y] = self.states[mode].map(i64::from);
        let [iw, ih] = image.map(i64::from);
        if self.vertical_first {
            y += i64::from(pattern) * h;
            if y + h > ih {
                let left = (y + h - ih) / h - 1;
                x += w * (1 + left / (ih / h).max(1));
                y = h * (left % (ih / h).max(1));
            }
        } else {
            x += i64::from(pattern) * w;
            if x + w > iw {
                let left = (x + w - iw) / w - 1;
                y += h * (1 + left / (iw / w).max(1));
                x = w * (left % (iw / w).max(1));
            }
        }
        ensure!(
            x >= 0 && y >= 0 && x + w <= iw && y + h <= ih,
            "NNN Title source frame outside original sprite"
        );
        Ok([
            i32::try_from(x)?,
            i32::try_from(y)?,
            self.size[0],
            self.size[1],
        ])
    }
}

use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    service::{EngineQuery, HostServices, InputFrame},
    state::ClientId,
};
use crysirror_image::{effects::ImageEffect, masked_pair::MaskedPairBlend};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
struct TitleImage {
    source_task: Option<Task<crate::resource_bridge::NativeImageSource>>,
    job: Option<ImageJob>,
    image: Option<Arc<dyn ImageResource>>,
    mask: Option<Arc<[u8]>>,
}
struct TitleFrame {
    background: Arc<dyn ImageResource>,
    counter: u32,
    states: Vec<NativeButtonState>,
    jobs: Vec<(usize, ImageJob)>,
    ready: Vec<(usize, Arc<dyn ImageResource>)>,
    final_job: Option<ImageJob>,
    sounds: std::collections::VecDeque<i32>,
    system_intro:Option<system_intro::SystemIntro>,
}
/// A native title retains configuration/counters and resource handles only.
/// Shared workers produce each cropped button and the immutable complete frame;
/// shared widgets own pointer coverage and event delivery.
pub struct NativeTitlePlayer {
    general_entry:Option<general_entry::GeneralEntry>,
    client: ClientId,
    layer: u32,
    widget_base: u32,
    widget_parent: Option<u32>,
    cancel_button: Option<usize>,
    canvas: [u32; 2],
    frame_ms: u64,
    pub spec: NativeTitleSpec,
    resources: crate::resource_bridge::NativeResources,
    pool: TaskPool,
    images: Vec<TitleImage>,
    states: Vec<NativeButtonState>,
    prepared: bool,
    widgets: bool,
    music_ready: bool,
    clock_origin: Option<u64>,
    counter: u32,
    inflight: Option<TitleFrame>,
    display_path: Option<String>,
    last_presented: Option<Arc<dyn ImageResource>>,
    pressed_buttons: u64,
    held_keys: u64,
    activated: std::collections::BTreeSet<usize>,
    pub api_calls: u64,
    pending_action: Option<usize>,
    pending_restart: Option<crate::native_shell::NativeMode>,
    restart_ready: Option<crate::native_shell::NativeMode>,
    script_ended: bool,
    system_intro:Option<system_intro::SystemIntro>,
}
impl NativeTitlePlayer {
    pub(crate) fn configure_system_intro(&mut self,table:&Names,random:&mut crysirror_image::temporal_pattern::PatternRng)->Result<()> {
        ensure!(!self.widgets&&self.inflight.is_none(),"NNN system intro changed after button publication");
        self.system_intro=Some(system_intro::SystemIntro::new(table,self.spec.buttons.len(),random)?);Ok(())
    }
    /// Attach native buttons to an existing engine modal panel before publication.
    /// Event delivery is scoped to that panel.
    pub fn set_widget_parent(&mut self, parent: u32) -> Result<()> {
        ensure!(
            parent != 0 && !self.widgets,
            "NNN menu widget parent changed after publication"
        );
        self.widget_parent = Some(parent);
        Ok(())
    }
    pub(crate) fn set_cancel_button(&mut self, index: usize) {
        self.cancel_button = (index < self.spec.buttons.len()).then_some(index);
    }
    pub(crate) fn hide_for_menu(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let layers: Vec<_> = self
            .display_path
            .iter()
            .map(|_| serde_json::json!({"op":"set_visible","id":self.layer,"visible":false}))
            .collect();
        let ui: Vec<_> = if self.widgets {
            (0..self.spec.buttons.len()).map(|index|serde_json::json!({"op":"set_visible","id":self.widget_base+index as u32,"visible":false})).collect()
        } else {
            vec![]
        };
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":layers,"ui":ui}))?,
        )?;
        self.api_calls += 1;
        Ok(())
    }
    pub(crate) fn resume_after_menu(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.states = self
            .spec
            .buttons
            .iter()
            .map(NativeButtonState::new)
            .collect();
        self.pending_action = None;
        self.activated.clear();
        self.pressed_buttons = 0;
        self.held_keys = 0;
        self.clock_origin = None;
        self.counter = 0;
        self.restart_entry();
        self.script_ended = false;
        self.music_ready = false;
        if self.display_path.is_some() {
            host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"set_visible","id":self.layer,"visible":true}]}))?)?;
            self.api_calls += 1;
        }
        if self.widgets {
            self.publish_widgets(
                host,
                self.spec.menu_wait.saturating_add(self.spec.menu_fade) == 0,
            )?;
        }
        Ok(())
    }
    pub fn new(
        client: ClientId,
        layer: u32,
        widget_base: u32,
        canvas: [u32; 2],
        frame_ms: u64,
        spec: NativeTitleSpec,
        resources: crate::resource_bridge::NativeResources,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0
                && layer != 0
                && widget_base != 0
                && (1..=10000).contains(&frame_ms)
                && canvas.iter().all(|n| (1..=8192).contains(n)),
            "NNN Title identity/bounds"
        );
        ensure!(
            !spec.buttons.is_empty() && spec.buttons.len() <= 64,
            "NNN Title button resource budget"
        );
        widget_base
            .checked_add(spec.buttons.len() as u32)
            .context("NNN Title widget identity overflow")?;
        // The independent crop graph uses a common background for each button.
        // Overlapping native matte restoration requires ordered destination jobs.
        for (i, a) in spec.buttons.iter().enumerate() {
            for b in spec.buttons.iter().skip(i + 1) {
                let intersect = a.origin[0] < b.origin[0] + b.size[0]
                    && b.origin[0] < a.origin[0] + a.size[0]
                    && a.origin[1] < b.origin[1] + b.size[1]
                    && b.origin[1] < a.origin[1] + a.size[1];
                ensure!(
                    !a.exists || !b.exists || !intersect,
                    "NNN overlapping native Title buttons require ordered matte policy"
                );
            }
        }
        for button in &spec.buttons {
            for key in button.accelerators.into_iter().filter(|key| *key != 0) {
                ensure!(
                    crysirror_core::service::buttons::windows_virtual_key(u32::from(key)).is_some(),
                    "NNN Title accelerator VK {key} needs shared key mapping"
                );
            }
        }
        let states = spec.buttons.iter().map(NativeButtonState::new).collect();
        let general_entry=spec.entry.map(general_entry::GeneralEntry::new);
        let pool = TaskPool::new("nnn-title-source", 2, 64)?;
        let mut images = Vec::with_capacity(spec.buttons.len());
        for button in &spec.buttons {
            let lookup = button.source.clone();
            let resources = resources.clone();
            let task = pool.spawn(move |cancel| resources.image_source(&lookup, &cancel))?;
            images.push(TitleImage {
                source_task: Some(task),
                job: None,
                image: None,
                mask: None,
            });
        }
        Ok(Self {
            general_entry,
            client,
            layer,
            widget_base,
            widget_parent: None,
            cancel_button: None,
            canvas,
            frame_ms,
            spec,
            resources,
            pool,
            images,
            states,
            prepared: false,
            widgets: false,
            music_ready: false,
            clock_origin: None,
            counter: 0,
            inflight: None,
            display_path: None,
            last_presented: None,
            pressed_buttons: 0,
            held_keys: 0,
            activated: Default::default(),
            api_calls: 0,
            pending_action: None,
            pending_restart: None,
            restart_ready: None,
            script_ended: false,
            system_intro:None,
        })
    }
    pub fn prepared(&self) -> bool {
        self.prepared
    }
    pub(crate) fn music_ready(&self) -> bool {
        self.music_ready
    }
    /// Record a genuine background END; this does not stop/advance the VM.
    pub fn script_ended(&mut self) {
        self.script_ended = true;
    }
    /// Consume the original restart request only after its shared frame is ready.
    pub fn take_restart(&mut self) -> Option<crate::native_shell::NativeMode> {
        self.restart_ready.take()
    }
    pub(crate) fn poll_existing_dependencies(
        &mut self,
        host: &mut dyn HostServices,
        vm: Option<&mut crate::adapter::NativeAdapter>,
    ) -> Result<()> {
        self.prepare(host)?;
        if let Some(vm) = vm {
            if self.prepared {
                self.poll_music(host, vm)?;
            }
            self.poll_frame_sounds(host, vm)?;
        }
        self.complete_frame(host)?;
        self.poll_entry_dependency(host)?;
        Ok(())
    }
    fn poll_music(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<bool> {
        if self.music_ready {
            return Ok(true);
        }
        if !vm.poll_outer_music(
            host,
            [
                self.spec.music_number,
                self.spec.music_loop,
                self.spec.music_volume,
                self.spec.music_fade_in,
            ],
            self.spec.music_restart,
        )? {
            return Ok(false);
        }
        self.music_ready = true;
        Ok(true)
    }
    fn poll_frame_sounds(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<bool> {
        let Some(frame) = &mut self.inflight else {
            return Ok(true);
        };
        while let Some(number) = frame.sounds.front().copied() {
            if !vm.poll_system_sound(host, number)? {
                return Ok(false);
            }
            frame.sounds.pop_front();
        }
        Ok(true)
    }
    fn draw_origin(&self, index: usize, counter: u32) -> Result<[i32; 2]> {
        let mut destination = self.spec.buttons[index].origin;
        let end = self.spec.menu_wait + self.spec.menu_fade;
        if counter < end {
            for axis in 0..2 {
                destination[axis] = destination[axis]
                    .checked_add(i32::try_from(
                        i64::from(self.spec.menu_delta[axis]) * i64::from(end - counter)
                            / i64::from(self.spec.menu_fade.max(1)),
                    )?)
                    .context("NNN Title appearance position overflow")?;
            }
        }
        Ok(destination)
    }
    pub fn dependency_pending(&self) -> bool {
        if self.entry_dependency_pending(){return true;}
        self.images
            .iter()
            .any(|v| v.source_task.is_some() || v.job.is_some())
            || self.inflight.is_some()
    }
    /// Resource admission is owner-driven; bytes are obtained only by scoped
    /// source workers. This does not execute another VM batch or advance time.
    pub fn prepare(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.prepared {
            return Ok(true);
        }
        for (index, slot) in self.images.iter_mut().enumerate() {
            let spec = &self.spec.buttons[index];
            if let Some(task) = &mut slot.source_task {
                if let Some(source) = task.try_take()? {
                    let effects = if source.has_mask {
                        vec![]
                    } else {
                        vec![ImageEffect::SpriteAlpha {
                            spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey {
                                right: false,
                            },
                        }]
                    };
                    slot.job = Some(
                        host.image_metadata(self.client)?
                            .begin_process(source.input, serde_json::to_vec(&effects)?.into())?,
                    );
                    self.api_calls += 1;
                    slot.source_task = None;
                }
            }
            if let Some(job) = &mut slot.job {
                match job.poll()? {
                    ImageProcessState::Pending => (),
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("NNN Title shared sprite preparation: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client,
                            "NNN Title foreign sprite resource"
                        );
                        for mode in 0..4 {
                            let _ = spec.source_rect(mode, 0, image.size())?;
                        }
                        if spec.print_mode != 0 {
                            slot.mask = Some(image.alpha_region([
                                u32::try_from(spec.check_origin[0])?,
                                u32::try_from(spec.check_origin[1])?,
                                spec.check_size[0] as u32,
                                spec.check_size[1] as u32,
                            ])?);
                        }
                        slot.image = Some(image);
                        slot.job = None;
                    }
                }
            }
            if slot.image.is_none() && slot.job.is_none() && slot.source_task.is_none() {
                let lookup = spec.source.clone();
                let resources = self.resources.clone();
                slot.source_task = Some(
                    self.pool
                        .spawn(move |cancel| resources.image_source(&lookup, &cancel))?,
                );
            }
        }
        self.prepared = self.images.iter().all(|slot| slot.image.is_some());
        Ok(self.prepared)
    }
    fn publish_widgets(&mut self, host: &mut dyn HostServices, enabled: bool) -> Result<()> {
        let mut commands = Vec::new();
        for (index, spec) in self.spec.buttons.iter().enumerate() {
            let mut widget = serde_json::json!({"id":self.widget_base+index as u32,"kind":"button","rect":[spec.origin[0]+spec.check_delta[0],spec.origin[1]+spec.check_delta[1],spec.check_size[0],spec.check_size[1]],"interaction_only":true,"keyboard_focusable":false,"activate_on_press":true,"visible":spec.exists,"enabled":enabled&&spec.enabled,"text":"","color":[0,0,0,0],"z":index});
            if let Some(parent) = self.widget_parent {
                widget["parent"] = serde_json::json!(parent);
            }
            if let Some(mask) = &self.images[index].mask {
                widget["hit_mask"] = serde_json::json!({"size":spec.check_size,"coverage":mask,"threshold":1,"strict_bounds":false});
            }
            commands.push(serde_json::json!({"op":"put","widget":widget}));
        }
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"ui":commands}))?,
        )?;
        self.api_calls += 1;
        self.widgets = true;
        Ok(())
    }
    fn poll_input(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<()> {
        self.held_keys = input.buttons_down;
        self.pressed_buttons |= input.buttons_pressed
            & (crysirror_core::service::buttons::MOUSE_LEFT
                | crysirror_core::service::buttons::MOUSE_RIGHT
                | if self.cancel_button.is_some() {
                    crysirror_core::service::buttons::ESCAPE
                } else {
                    0
                });
        if !self.widgets {
            return Ok(());
        }
        let query = self
            .widget_parent
            .map(EngineQuery::UiSubtreeEvents)
            .unwrap_or(EngineQuery::UiEvents);
        let events: Vec<serde_json::Value> =
            serde_json::from_slice(&host.query_engine(self.client, query)?)?;
        for event in events {
            if event["kind"] == "activate" {
                if let Some(id) = event["widget"]["id"].as_u64() {
                    let first = u64::from(self.widget_base);
                    if (first..first + self.spec.buttons.len() as u64).contains(&id) {
                        self.activated.insert((id - first) as usize);
                    }
                }
            }
        }
        Ok(())
    }
    fn button_input(
        &self,
        index: usize,
        background: Arc<dyn ImageResource>,
        state: &NativeButtonState,
        count: u32,
        system_intro:Option<&system_intro::SystemIntro>,
    ) -> Result<ImageInput> {
        let spec = &self.spec.buttons[index];
        let image = self.images[index]
            .image
            .as_ref()
            .context("NNN Title sprite absent")?
            .clone();
        let [w, h] = spec.size;
        let origin = self.draw_origin(index, count)?;
        let base = ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: [w as u32, h as u32],
                color: [0, 0, 0, 255],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(background)),
                source: [origin[0], origin[1], w, h],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        };
        let intro_end = self.spec.menu_wait.saturating_add(self.spec.menu_fade);
        if let Some(intro)=system_intro.filter(|intro|!intro.finished) {
            let Some((source,destination,extent))=intro.view(index,[w,h]) else {return Ok(base)};
            let frame=spec.source_rect(state.mode,0,image.size())?;
            return Ok(ImageInput::Composite{base:Box::new(base),layers:vec![ImageCompositeLayer {input:Box::new(ImageInput::Resource(image)),source:[frame[0]+source[0],frame[1]+source[1],extent[0],extent[1]],destination,clip:None,opacity:255,math:ImageCompositeMath::MaskSeparate256}]});
        }
        if count < intro_end {
            let percent = if self.spec.menu_effect & 1 != 0 {
                100 * count.saturating_sub(self.spec.menu_wait) / self.spec.menu_fade.max(1)
            } else {
                100
            };
            let source = spec.source_rect(state.mode, 0, image.size())?;
            return Ok(ImageInput::Composite {
                base: Box::new(base),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(image)),
                    source,
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: if percent >= 100 {
                        ImageCompositeMath::MaskSeparate256
                    } else {
                        ImageCompositeMath::MaskPiecewise256 {
                            weight: (percent * 256 / 100) as u16,
                        }
                    },
                }],
            });
        }
        let (frames, percent) = state.frame(spec);
        let a = spec.source_rect(state.mode, frames[0], image.size())?;
        let b = spec.source_rect(state.mode, frames[1], image.size())?;
        if spec.print_mode == 2 && percent > 0 && percent < 100 {
            // Native percentage conversion floors the two weights separately.
            return Ok(ImageInput::Ternary {
                destination: Box::new(base),
                first: Box::new(ImageInput::Resource(image.clone())),
                second: Box::new(ImageInput::Resource(image)),
            });
        }
        Ok(ImageInput::Composite {
            base: Box::new(base),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(image)),
                source: if percent == 0 { b } else { a },
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: if spec.print_mode == 0 {
                    ImageCompositeMath::CopyRgb
                } else {
                    ImageCompositeMath::MaskSeparate256
                },
            }],
        })
    }
    fn complete_frame(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let capture_only=self.capture_only_for_entry();
        let Some(frame) = &mut self.inflight else {
            return Ok(true);
        };
        if !frame.sounds.is_empty() {
            return Ok(false);
        }
        let mut pending = Vec::new();
        for (index, mut job) in frame.jobs.drain(..) {
            match job.poll()? {
                ImageProcessState::Pending => pending.push((index, job)),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN Title shared button composition: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client
                            && image.size() == self.spec.buttons[index].size.map(|n| n as u32),
                        "NNN Title button output scope/dimensions"
                    );
                    frame.ready.push((index, image));
                }
            }
        }
        frame.jobs = pending;
        if !frame.jobs.is_empty() {
            return Ok(false);
        }
        if frame.final_job.is_none() {
            frame.ready.sort_by_key(|(index, _)| *index);
            let mut layers = Vec::new();
            let end = self.spec.menu_wait.saturating_add(self.spec.menu_fade);
            for (index, image) in &frame.ready {
                let spec = &self.spec.buttons[*index];
                let [w, h] = spec.size;
                let mut destination = spec.origin;
                if frame.counter < end {
                    let remainder = i64::from(end - frame.counter);
                    let divisor = i64::from(self.spec.menu_fade.max(1));
                    for axis in 0..2 {
                        destination[axis] = destination[axis]
                            .checked_add(i32::try_from(
                                i64::from(self.spec.menu_delta[axis]) * remainder / divisor,
                            )?)
                            .context("NNN Title appearance position overflow")?;
                    }
                }
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(image.clone())),
                    source: [0, 0, w, h],
                    destination,
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                });
            }
            frame.final_job = Some(host.image_metadata(self.client)?.begin_process(
                ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(frame.background.clone())),
                    layers,
                },
                b"[]".as_slice().into(),
            )?);
            self.api_calls += 1;
        }
        match frame.final_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN Title complete shared frame composition: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.canvas,
                    "NNN Title complete frame scope"
                );
                if !capture_only {
                let publication =
                    host.publish_image_resource(self.client, self.layer, image.clone())?;
                self.api_calls += 1;
                let path = publication.path.relative().to_owned();
                let [w, h] = self.canvas;
                if let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[4000,0],"paint":{"kind":"image","asset":path}}}]}))?){let _=host.release_image_resource(self.client,&path);return Err(error.into())}
                self.api_calls += 1;
                if let Some(old) = self.display_path.replace(path) {
                    host.release_image_resource(self.client, &old)?;
                }
                }
                self.last_presented = Some(image);
                let frame = self.inflight.take().unwrap();
                let intro_finished=frame.system_intro.as_ref().is_some_and(|intro|intro.finished) && self.system_intro.as_ref().is_some_and(|intro|!intro.finished);
                self.counter = frame.counter;
                self.states = frame.states;
                self.system_intro=frame.system_intro;
                if !capture_only && (intro_finished || self.counter == self.spec.menu_wait.saturating_add(self.spec.menu_fade)) {
                    self.publish_widgets(host, true)?;
                }
                Ok(true)
            }
        }
    }
    /// Returns a native button index only after its configured click count and
    /// the corresponding complete shared frame have been published.
    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        background: Arc<dyn ImageResource>,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<Option<usize>> {
        ensure!(
            background.client() == self.client && background.size() == self.canvas,
            "NNN Title background scope"
        );
        self.poll_input(host,input)?;
        if self.entry_blocks_update(host.now_ms()){self.pressed_buttons=0;self.held_keys=0;self.activated.clear();}
        if !self.prepare(host)? {
            return Ok(None);
        }
        if !self.poll_music(host, vm)? {
            return Ok(None);
        }
        if !self.widgets {
            let _ = host.query_engine(self.client, EngineQuery::UiEvents)?;
            self.publish_widgets(host, !self.entry_active() && self.spec.menu_wait + self.spec.menu_fade == 0 && self.system_intro.as_ref().is_none_or(|intro|intro.finished))?;
        }
        if !self.poll_frame_sounds(host, vm)? {
            return Ok(None);
        }
        if !self.complete_frame(host)? {
            return Ok(None);
        }
        if self.entry_active() && !self.poll_entry(host,input,background.clone(),vm)? {
            return Ok(None);
        }
        if let Some(action) = self.pending_action.take() {
            return Ok(Some(action));
        }
        if let Some(mode) = self.pending_restart.take() {
            self.restart_ready = Some(mode);
            return Ok(None);
        }
        if self.restart_ready.is_some() {
            return Ok(None);
        }
        let now = host.now_ms();
        let origin = *self.clock_origin.get_or_insert(now);
        ensure!(now >= origin, "NNN Title clock moved backwards");
        if now - origin < u64::from(self.counter) * self.frame_ms {
            return Ok(None);
        }
        let end = self.spec.menu_wait + self.spec.menu_fade;
        let mut counter = self.counter.saturating_add(1);
        let mut states = self.states.clone();
        let mut system_intro=self.system_intro.clone();
        let mut selected = None;
        let mut sounds = std::collections::VecDeque::new();
        if let Some(intro)=system_intro.as_mut().filter(|intro|!intro.finished) {
            intro.advance(self.pressed_buttons & (crysirror_core::service::buttons::MOUSE_LEFT|crysirror_core::service::buttons::MOUSE_RIGHT)!=0);
            for (state,spec) in states.iter_mut().zip(&self.spec.buttons){state.calcu_without_input(spec);}
            self.activated.clear();
        }else if self.counter < end {
            if self.pressed_buttons != 0 {
                counter = end;
            }
            self.activated.clear();
        } else {
            // Legacy MenuButtonGroup locks the first click and passes NULL
            // input to every other button. It returns one last non-NOTHING
            // event, not a separate sound/action for every hovered button.
            let mut locked = states.iter().position(|state| state.mode == 2);
            let mut group_sound = None;
            let mut reset_animation = false;
            for (index, state) in states.iter_mut().enumerate() {
                let accepts_input = locked.is_none_or(|active| active == index);
                let interaction: serde_json::Value = serde_json::from_slice(&host.query_engine(
                    self.client,
                    EngineQuery::UiInteraction(self.widget_base + index as u32),
                )?)?;
                let previous_mode = state.mode;
                if !accepts_input { state.calcu_without_input(&self.spec.buttons[index]); continue; }
                let signal = state.calcu(
                    &self.spec.buttons[index],
                    accepts_input && interaction["hovered"] == true,
                    accepts_input
                        && (self.activated.remove(&index)
                            || (self.cancel_button == Some(index)
                                && self.pressed_buttons
                                    & (crysirror_core::service::buttons::MOUSE_RIGHT
                                        | crysirror_core::service::buttons::ESCAPE)
                                    != 0)
                            || self.spec.buttons[index]
                                .accelerators
                                .iter()
                                .filter_map(|key| {
                                    crysirror_core::service::buttons::windows_virtual_key(
                                        u32::from(*key),
                                    )
                                })
                                .any(|bit| self.held_keys & bit != 0)),
                );
                self.activated.remove(&index);
                let sound = match signal {
                    NativeButtonSignal::ClickStarted => self.spec.buttons[index].sounds[0],
                    NativeButtonSignal::Selected if previous_mode != 2 => {
                        self.spec.buttons[index].sounds[0]
                    }
                    NativeButtonSignal::Selected => 0,
                    NativeButtonSignal::Entered if self.spec.buttons[index].sound_waits[0] == 0 => self.spec.buttons[index].sounds[2],
                    NativeButtonSignal::Exited if self.spec.buttons[index].sound_waits[1] == 0 => self.spec.buttons[index].sounds[3],
                    NativeButtonSignal::Sound(sound) => sound,
                    _ => 0,
                };
                if accepts_input && signal != NativeButtonSignal::None {
                    group_sound = Some(sound);
                    if matches!(
                        signal,
                        NativeButtonSignal::ClickStarted | NativeButtonSignal::Selected
                    ) {
                        locked = Some(index);
                    }
                    if signal == NativeButtonSignal::Selected {
                        selected = Some(index);
                    }
                    if matches!(
                        signal,
                        NativeButtonSignal::Entered | NativeButtonSignal::Exited
                    ) && self.spec.menu_anime_adjust
                    {
                        reset_animation = true;
                    }
                }
            }
            if reset_animation {
                for state in &mut states {
                    state.animation = [0; 4];
                }
            }
            // Native MakeButtonStatus carries a seven-bit one-based sound
            // number. Keep zero silent after masking (e.g. source ID 128).
            if let Some(sound) = group_sound.filter(|sound| *sound > 0).map(|sound|sound & 0x7f).filter(|sound|*sound>0) {
                sounds.push_back(sound);
            }
        }
        self.pressed_buttons = 0;
        let mut frame = TitleFrame {
            background: background.clone(),
            counter,
            states: states.clone(),
            jobs: Vec::new(),
            ready: Vec::new(),
            final_job: None,
            sounds,
            system_intro,
        };
        if counter >= self.spec.menu_wait {
            for (index, state) in states.iter_mut().enumerate() {
                if !self.spec.buttons[index].exists {
                    continue;
                }
                let input = self.button_input(index, background.clone(), state, counter,frame.system_intro.as_ref())?;
                let effects = if matches!(input, ImageInput::Ternary { .. }) {
                    let (frames, percent) = state.frame(&self.spec.buttons[index]);
                    let image = self.images[index].image.as_ref().unwrap();
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
                self.api_calls += 1;
                if counter >= end && frame.system_intro.as_ref().is_none_or(|intro|intro.finished) {
                    state.printed(&self.spec.buttons[index]);
                } else {
                    state.animation = [0; 4];
                }
            }
        }
        frame.states = states;
        self.inflight = Some(frame);
        // Actions are retained until frame completion; resource latency never
        // turns a queued native button into an early NewGame transition.
        self.pending_action = selected;
        // Native menu selection returns before the automatic/script-end tests.
        // Retain a return until its exact shared frame has completed, just like
        // a button; a dependency poll never starts a new outer mode.
        if selected.is_none()
            && ((self.spec.auto_restart > 0 && counter >= self.spec.auto_restart)
                || (self.spec.script_end_exits && self.script_ended))
        {
            self.pending_restart = Some(self.spec.restart_next);
        }
        Ok(None)
    }
    pub(crate) fn completed_frame(&self) -> Result<Arc<dyn ImageResource>> {
        ensure!(
            self.inflight.is_none()
                && self.pending_action.is_none()
                && self.pending_restart.is_none(),
            "NNN Title exit requires its completed selection frame"
        );
        self.last_presented
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NNN Title completed frame absent"))
    }
    /// GeneralPrint replaces the same retained title plane. A replacement is
    /// published before releasing its predecessor, including worker retries.
    pub(crate) fn present_return_frame(
        &mut self,
        host: &mut dyn HostServices,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let _ = self.completed_frame()?;
        ensure!(
            image.client() == self.client && image.size() == self.canvas,
            "NNN Title return frame scope"
        );
        let publication = host.publish_image_resource(self.client, self.layer, image.clone())?;
        let path = publication.path.relative().to_owned();
        let [w, h] = self.canvas;
        if let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[4000,0],"paint":{"kind":"image","asset":path}}}]}))?) {
            let _ = host.release_image_resource(self.client, &path);
            return Err(error.into());
        }
        self.api_calls += 2;
        if let Some(old) = self.display_path.replace(path) {
            host.release_image_resource(self.client, &old)?;
        }
        self.last_presented = Some(image);
        Ok(())
    }
    /// Declarative retirement for an atomic original-save replacement. Preparing
    /// a load must leave the currently displayed title and its input intact.
    pub(crate) fn restore_cleanup(
        &self,
    ) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>, Vec<String>)> {
        ensure!(
            !self.dependency_pending(),
            "NNN title restore while a frame is pending"
        );
        let widgets = if self.widgets {
            (0..self.spec.buttons.len())
                .map(|n| serde_json::json!({"op":"remove","id":self.widget_base+n as u32}))
                .collect()
        } else {
            vec![]
        };
        let layers = self
            .display_path
            .as_ref()
            .map(|_| serde_json::json!({"op":"remove","id":self.layer}))
            .into_iter()
            .collect();
        Ok((layers, widgets, self.display_path.iter().cloned().collect()))
    }
    pub(crate) fn retire(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let _ = self.completed_frame()?;
        let widgets: Vec<_> = if self.widgets {
            (0..self.spec.buttons.len())
                .map(
                    |index| serde_json::json!({"op":"remove","id":self.widget_base + index as u32}),
                )
                .collect()
        } else {
            vec![]
        };
        let layers: Vec<_> = self
            .display_path
            .as_ref()
            .map(|_| serde_json::json!({"op":"remove","id":self.layer}))
            .into_iter()
            .collect();
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":layers,"ui":widgets}))?,
        )?;
        self.api_calls += 1;
        if let Some(path) = self.display_path.take() {
            host.release_image_resource(self.client, &path)?;
        }
        self.widgets = false;
        Ok(())
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"mode":"Title","prepared":self.prepared,"music_ready":self.music_ready,"counter":self.counter,"widgets":self.widgets,"frame_pending":self.inflight.is_some(),"engine_api_calls":self.api_calls,"shared_accelerators_mapped":true,"background_ended":self.script_ended,"pending_action":self.pending_action,"pending_restart":self.pending_restart.map(|mode|format!("{mode:?}")),"restart_ready":self.restart_ready.map(|mode|format!("{mode:?}"))})
    }
}
#[cfg(test)]
mod variant_tests {
    use super::*;
    #[test]
    fn native_menu_variants_inherit_columns_without_position_delta_or_accelerators() {
        let empty = Names::parse(b"", false).unwrap();
        let config = NativeConfiguration {
            resources: crate::native_resources::ResourceNames::new("").unwrap(),
            setup: empty.clone(),
            main_control: None,
            chart_setup: None,
            variable_names: Names::parse(b"unused variant", false).unwrap(),
            project: empty.clone(),
            commands: empty.clone(),
            functions: empty,
            initial: None,
            module_sha256: "authored".into(),
            profile: crate::expression::VariableProfile::Legacy1000,
            scripts: Default::default(),
        };
        let table=Names::parse(b"basicButtonNumber 2 extModeNumber 2 extButtonNumber 1 extModeCheckVarName variant menuNextX 2 menuNextY 3 menu1Button1FileName atlas menu1Button1SizeX 8 menu1Button1SizeY 8 menu1Button1PrintX 10 menu1Button1PrintY 20 menu1Button1EnterWait 5 menu1Button1ExitWait 9 menu1Button1AccelKey1 65 menu2Button2PrintX 100 menu3Button3EnableFlag 0",false).unwrap();
        let base = NativeTitleSpec::from_table_variant(&table, &config, 0).unwrap();
        assert_eq!(base.buttons.len(), 3);
        assert!(!base.buttons[2].exists);
        assert_eq!(base.buttons[2].origin, [14, 26]);
        assert_eq!(base.buttons[0].accelerators, [65, 0, 0]);
        assert_eq!(base.buttons[1].accelerators, [0; 3]);
        // Calcu(NULL) retains the hover delay count; the shared frame pump
        // must not manufacture hover exits or delayed sound requests.
        let mut button=base.buttons[0].clone();
        button.sound_waits=[2,3];button.sounds=[1,2,3,4];
        let mut state=NativeButtonState::new(&button);
        assert_eq!(state.calcu(&button,true,false),NativeButtonSignal::Entered);
        assert_eq!(state.calcu(&button,true,false),NativeButtonSignal::None);
        assert_eq!(state.calcu(&button,true,false),NativeButtonSignal::Sound(3));
        assert_eq!(state.calcu(&button,true,false),NativeButtonSignal::None);
        assert_eq!(state.calcu(&button,false,false),NativeButtonSignal::Exited);
        assert_eq!(state.calcu(&button,false,false),NativeButtonSignal::None);
        let count=state.count;state.calcu_without_input(&button);assert_eq!(state.count,count);
        assert_eq!(state.calcu(&button,false,false),NativeButtonSignal::None);
        assert_eq!(state.calcu(&button,false,false),NativeButtonSignal::Sound(4));
        let mut variables = crate::expression::Variables::fresh(config.profile);
        variables.write_stored(1, 999);
        variables.mark_calculated(1, b"rnd".to_vec());
        let variant = NativeTitleSpec::selected_variant(&table, &config, &variables).unwrap();
        assert_eq!(variant, 2);
        let selected = NativeTitleSpec::from_table_variant(&table, &config, variant).unwrap();
        assert_eq!(selected.buttons[0].origin, [10, 20]);
        assert_eq!(selected.buttons[1].origin, [100, 23]);
        assert_eq!(selected.buttons[2].origin, [14, 26]);
        assert!(selected.buttons[2].exists);
        assert!(!selected.buttons[2].enabled);
        assert_eq!(selected.buttons[0].accelerators, [0; 3]);
        assert_eq!(selected.buttons[0].sound_waits, [5, 5]);
        let extra = Names::parse(
            b"basicButtonNumber 7 extModeReturnVarName variant extButtonSpecialMode3 12",
            false,
        )
        .unwrap();
        assert!(matches!(
            extension_request(&extra, &config, 5, true).unwrap(),
            MenuRequest::Script {
                variable: Some(1),
                value: -1
            }
        ));
        assert!(matches!(
            extension_request(&extra, &config, 7, true).unwrap(),
            MenuRequest::Mode(12)
        ));
        let omake =
            Names::parse(b"basicButtonNumber 4 extModeReturnVarName variant", false).unwrap();
        assert!(matches!(
            extension_request(&omake, &config, 4, false).unwrap(),
            MenuRequest::Script {
                variable: Some(1),
                value: 1
            }
        ));
    }
}
