// SPDX-License-Identifier: MPL-2.0
//! CVNS 2004 software scene over the shared layer service.
//!
//! The original keeps 128 bitmaps (+11232), 1,024 sprite records (+7136), two
//! ADV windows (+11768) over eight message windows (+11776) and a screen fade
//! (+18536 colour, +18540 level). `0x40fa20` buckets the drawable records by
//! layer and the band compositor `0x410e10` draws bucket 127 first.

mod adv;
mod checkpoint;
pub(super) mod bitmap;
mod fx;
mod input;
pub(super) mod mask_fonts;
pub(super) mod movie;
mod sprite;
mod primary;
pub(super) mod restore;
pub(super) mod slot;
mod shake;
mod settings;
pub(super) mod registry;
pub(super) mod rule;
mod text;

use super::visual::DecodedImage;
use crate::vm::CmvsVm;
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, InputFrame, UploadedImage},
    state::ClientId,
};
use crysirror_resource::private_profile::PrivateProfile;
use serde_json::json;
use sprite::{Frame, Kind, Sprite, SpriteId};
use std::collections::{BTreeMap, BTreeSet};

const BITMAPS: usize = 128;
const MESSAGES: usize = 8;
const ADVS: usize = 2;
const FADE_LAYER: u32 = 0x4356_8001;
const MESSAGE_LAYER: u32 = 0x4356_8100;
const BITMAP_RESOURCE: u32 = 0x4356_9000;
const ADV_RESOURCE: u32 = 0x4356_9100;

/// Where a decoded bitmap goes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BitmapSlot {
    Global(usize),
    Adv(usize, usize),
}

/// What the adapter must do after a scene selector.
pub(super) enum Step {
    Done,
    /// Consume operands now and report the native error flags at frame end.
    Fault { words: u16, flags: u32 },
    /// Decode an image from the image sources, then finish the call.
    Bitmap(bitmap::Job),
    Rule(rule::Request),
    MaskSheet(String, u32),
    Movie(usize, crysirror_core::path::VfsPath, Vec<u8>),
    Registry(registry::Request),
    Pending,
    /// The selector ended the frame.
    Frame,
}

pub(super) fn rgb(color: u32) -> [u8; 3] {
    // 0x413a80 stores colour words directly as 32-bit DIB pixels.
    [(color >> 16) as u8, (color >> 8) as u8, color as u8]
}

struct Transition {
    /// `0x4229e0` word 0: nonzero raises the level from 0 to 256.
    reveal: u32,
    color: u32,
    duration: u32,
    elapsed: u32,
}

pub(super) struct Scene {
    bitmaps: Vec<Option<UploadedImage>>,
    pub(super) movies: movie::Slots,
    pictures: Vec<Option<bitmap::Picture>>,
    capture: Option<crysirror_core::image_processing::ImageJob>,
    clipboard: Option<Box<dyn crysirror_core::service::DesktopRequestJob>>,
    copy_next_frame: bool,
    copied_screen: Option<DecodedImage>,
    sprites: BTreeMap<SpriteId, Sprite>,
    tint_curves: BTreeMap<(u32, i16), UploadedImage>,
    messages: Vec<Option<text::Window>>,
    advs: Vec<Option<adv::Adv>>,
    pub(super) masks: mask_fonts::Fonts,
    input: input::Input,
    /// Settings block `S[k]` at engine `this[360 + k]`.
    settings: [u32; 40],
    /// `0x420ee0` auto-advance clock `dword_46BA78`.
    auto_clock: u32,
    pub(super) audio_lead_ms: u32,
    /// Native frame time `0x439370` for selectors that accumulate it.
    delta: u32,
    /// +2744/+2752 wheel counters.
    wheel: [u32; 2],
    font_name: Vec<u8>,
    right_mouse_mode: bool,
    registry_base: String,
    pub(super) priority: u32,
    pub(super) settings_dirty: bool,
    fixed_clock: bool,
    sampled_at: u32,
    exclude_vm_time: bool,
    accumulated: u32,
    /// CRT generator seeded from the boot clock by 0x410f50.
    random: u32,
    level: u32,
    color: u32,
    transition: Option<Transition>,
    shake: Option<shake::Shake>,
    rule: Option<rule::Active>,
    rule_image: Option<UploadedImage>,
    /// `0x2008`: `0x410e10` fills each dirty band with the fade colour first.
    background: bool,
    primary: primary::Primary,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            bitmaps: vec![None; BITMAPS],
            movies: Default::default(),
            pictures: vec![None; BITMAPS],
            capture: None,
            clipboard: None,
            copy_next_frame: false,
            copied_screen: None,
            sprites: BTreeMap::new(),
            tint_curves: BTreeMap::new(),
            messages: (0..MESSAGES).map(|_| None).collect(),
            advs: (0..ADVS).map(|_| None).collect(),
            masks: Default::default(),
            input: input::Input::new(),
            settings: [0; 40],
            auto_clock: 0,
            audio_lead_ms: 92,
            delta: 0,
            random: 1,
            font_name: Vec::new(),
            right_mouse_mode: false,
            registry_base: "KAWAIKO".into(),
            priority: 1,
            settings_dirty: false,
            fixed_clock: false,
            sampled_at: 0,
            exclude_vm_time: true,
            accumulated: 0,
            wheel: [0; 2],
            level: 0,
            color: 0,
            transition: None,
            shake: None,
            rule: None,
            rule_image: None,
            background: false,
            primary: Default::default(),
        }
    }
}

/// `0x8000000`, `0x400000` and `0x200000` error bits end the native game.
fn sprite_mut(sprites: &mut BTreeMap<SpriteId, Sprite>, id: SpriteId) -> Result<&mut Sprite> {
    sprites.get_mut(&id).with_context(|| format!("CVNS sprite {id:?} is absent"))
}

fn bitmap_index(index: u32) -> Result<usize> {
    ensure!(index < BITMAPS as u32, "CVNS bitmap {index} outside 0..127");
    Ok(index as usize)
}

fn component_index(index: u32) -> Result<usize> {
    ensure!(index < 8, "CVNS bitmap component {index} outside 0..7");
    Ok(index as usize)
}

fn global(index: u32) -> Result<SpriteId> {
    ensure!(index < u32::from(sprite::SPRITES), "CVNS sprite {index} outside 0..1023");
    Ok(SpriteId::Global(index as u16))
}

fn part(window: u32, part: u32) -> Result<SpriteId> {
    ensure!((window as usize) < ADVS && part < u32::from(sprite::PARTS), "CVNS ADV part {window}/{part}");
    Ok(SpriteId::Part(window as u8, part as u8))
}

/// The MSVC CRT generator behind `rand()`.
fn msvc_rand(state: &mut u32) -> i32 {
    *state = state.wrapping_mul(214_013).wrapping_add(2_531_011);
    ((*state >> 16) & 0x7fff) as i32
}

impl Scene {
    /// `0x401480` reads `MESSAGE_INFO` into the settings block.
    pub fn configure(&mut self, config: &[u8]) {
        let (text, _, _) = encoding_rs::SHIFT_JIS.decode(config);
        let profile = PrivateProfile::parse(&text);
        let value = |key: &str, default: i32| profile.integer("MESSAGE_INFO", key, default) as u32;
        let system = |key: &str, default: i32| profile.integer("SYSTEM_INFO", key, default) as u32;
        let s = &mut self.settings;
        self.right_mouse_mode = system("MOUSE_RIGHT_MODE", 0) != 0;
        let face = profile.string("MESSAGE_INFO", "FONT").filter(|s| !s.is_empty()).unwrap_or("ＭＳ ゴシック");
        self.font_name = encoding_rs::SHIFT_JIS.encode(face).0.into_owned();
        self.font_name.truncate(127);
        s[0] = u32::from(system("ACTIVE_DRAW", 0) != 0);
        s[1] = u32::from(system("DLG_LOAD", 0) != 0);
        s[2] = u32::from(system("DLG_SAVE", 0) != 0);
        s[3] = system("EFFECT_USE", 1);
        s[4] = system("WAIT_TIMING", 0);
        self.fixed_clock = s[4] != 0;
        s[6] = u32::from(value("READ_SKIP", 1) != 0);
        s[7] = u32::from(value("AUTO_SKIP", 0) != 0);
        s[8] = u32::from(value("TEXT_SADOW", 0) != 0);
        s[9] = value("EFFECT_TYPE", 1);
        s[10] = value("EFFECT_TIMER", 2);
        s[11] = value("TEXT_WAIT", 2);
        s[12] = value("VOICE_CUT_MODE", 0);
        for (index, key) in ["TEXT_WAIT_EX_1", "TEXT_WAIT_EX_2", "TEXT_WAIT_EX_3", "TEXT_WAIT_EX_4"].iter().enumerate() {
            s[13 + index] = value(key, 0);
        }
        let sound = |key: &str, default: i32| profile.integer("SOUND_INFO", key, default) as u32;
        // 0x40ae70 / 0x40c7d0: two native stereo PCM16 buffers in milliseconds.
        // High quality doubles both the output rate and buffer bytes.
        let bytes = if sound("SOUND_BUFFER", 0) == 0 { 4096 } else { 8192 };
        self.audio_lead_ms = 2000 * bytes / (4 * 22050);
        let mouse = |key: &str, default: i32| profile.integer("MOUSE_INFO", key, default) as u32;
        for (index, key) in ["MASTER_VOLUME", "MUSIC_VOLUME", "SE_VOLUME", "VOICE_VOLUME", "VIDEO_VOLUME"].iter().enumerate() {
            s[27 + index] = sound(key, 100);
        }
        s[32] = u32::from(sound("MUSIC_USE", 1) != 0);
        s[33] = u32::from(sound("VOICE_USE", 1) != 0);
        s[34] = u32::from(sound("SE_USE", 1) != 0);
        s[35] = mouse("AUTO_CLEAR", 1);
        s[36] = mouse("AUTO_MOVE", 0);
        s[37] = mouse("WHEEL_MODE", 0);
        s[38] = mouse("WHEEL_USE", 0);
        s[39] = profile.integer("VIDEO_INFO", "FRAME_CTRL", 0) as u32;
        s[18] = value("AUTO_MODE_1", 10);
        s[19] = value("AUTO_MODE_2", 0);
        s[20] = value("AUTO_MODE_3", 0);
        s[21] = value("AUTO_VOICE_SYNC", 1);
    }

    pub fn default_font(&self, host: &mut dyn HostServices, client: ClientId) -> Result<std::sync::Arc<str>> {
        Ok(super::message::resolve_face(host, client, &self.font_name)?.unwrap_or_else(|| "default".into()))
    }

    pub fn select_font(&mut self, host: &mut dyn HostServices, client: ClientId, name: Vec<u8>) -> Result<()> {
        self.font_name = name;
        let font = self.default_font(host, client)?;
        for window in self.messages.iter_mut().flatten().filter(|w| w.default_face) {
            window.font = font.clone();
        }
        self.settings_dirty = true;
        Ok(())
    }

    pub fn seed(&mut self, clock: u32) {
        self.random = clock;
        self.reset_clock(self.fixed_clock, clock);
    }

    pub fn save_salts(&mut self) -> [u8; 2] {
        [msvc_rand(&mut self.random) as u8, 128 | (msvc_rand(&mut self.random) as u8 & 127)]
    }

    pub fn effect_levels(&self) -> [u32; 2] {
        [self.settings[27], self.settings[29]]
    }

    pub fn music_gain(&self) -> f32 {
        super::sound::cvns_gain(self.settings[27], self.settings[28], 0)
    }

    pub fn movie_gain(&self) -> f32 {
        super::sound::cvns_gain(self.settings[27], self.settings[31], 100)
    }

    pub fn consume_movie_skip(&mut self) -> bool {
        self.input.consume_movie_skip()
    }

    pub fn voice_settings(&self) -> (bool, u32, u32) {
        (self.settings[33] != 0, self.settings[27], self.settings[30])
    }

    pub fn admit_input(&mut self, frame: InputFrame) {
        use crysirror_core::service::buttons::{WHEEL_DOWN, WHEEL_UP};
        // 0x402450 converts positive WM_MOUSEWHEEL deltas into negative notches.
        self.wheel[0] = self.wheel[0]
            .wrapping_add(u32::from(frame.buttons_pressed & WHEEL_DOWN != 0))
            .wrapping_sub(u32::from(frame.buttons_pressed & WHEEL_UP != 0));
        self.input.admit(frame);
    }

    /// `0x412460` before the VM runs a native frame.
    pub fn reset_clock(&mut self, fixed: bool, now: u32) {
        self.fixed_clock = fixed;
        self.sampled_at = now;
        self.accumulated = 0;
        self.delta = 0;
    }

    pub fn begin_frame(&mut self, now: u32, period: u32) {
        self.delta = if self.fixed_clock { period } else { now.wrapping_sub(self.sampled_at) };
        self.sampled_at = now;
        self.accumulated = self.accumulated.wrapping_add(period);
        self.input.begin_frame();
    }

    fn style(&self) -> text::Style {
        let s = &self.settings;
        text::Style {
            speed: s[11],
            effect: s[9],
            timer: s[10],
            shadow: s[8] != 0,
            instant: s[5] != 0,
            pauses: [s[13], s[14], s[15], s[16]],
        }
    }

    fn message(&mut self, index: usize) -> Result<&mut text::Window> {
        self.messages
            .get_mut(index)
            .and_then(Option::as_mut)
            .with_context(|| format!("CVNS message window {index} is absent"))
    }

    fn adv(&mut self, index: u32) -> Result<&mut adv::Adv> {
        self.advs
            .get_mut(index as usize)
            .and_then(Option::as_mut)
            .with_context(|| format!("CVNS ADV window {index} is absent"))
    }

    fn adv_message(&mut self, index: u32) -> Result<&mut text::Window> {
        let message = self.adv(index)?.message;
        self.message(message)
    }

    fn bitmap_size(&self, kind: Kind, bitmap: u32) -> Option<[u32; 2]> {
        match kind {
            Kind::Movie => self.movies.image(bitmap as usize),
            Kind::Part(window) => self.advs.get(usize::from(window))?.as_ref()?.bitmaps.get(bitmap as usize)?.as_ref(),
            _ => self.bitmaps.get(bitmap as usize)?.as_ref(),
        }
        .map(|image| image.size)
    }

    /// `0x42db10`: show or hide the background part, the buttons and the icon.
    fn show_parts(&mut self, window: u32, shown: bool) -> Result<()> {
        let adv = self.adv(window)?;
        let [x, y, ..] = adv.rect;
        let buttons = adv.buttons;
        let background = SpriteId::Part(window as u8, 1 - 1);
        if let Some(sprite) = self.sprites.get_mut(&background) {
            if shown {
                sprite.position = [x, y];
            }
            sprite.enabled = shown;
        } else {
            return Ok(());
        }
        for button in buttons.iter().filter(|button| button.enabled) {
            let id = part(window, u32::from(button.part))?;
            let state = button.states[usize::from(button.state)];
            let sprite = sprite_mut(&mut self.sprites, id)?;
            if shown {
                sprite.set_source(state.bitmap, state.crop);
                sprite.position = [x + i32::from(state.offset[0]), y + i32::from(state.offset[1])];
            }
            sprite.enabled = shown;
        }
        if let Some(icon) = self.sprites.get_mut(&SpriteId::Part(window as u8, 1)) {
            if (1..=2).contains(&icon.animation()) {
                icon.enabled = shown;
            }
        }
        Ok(())
    }

    /// Stops and hides the click-wait icon when the wait ends.
    fn stop_icon(&mut self, window: u32) -> Result<()> {
        if let Some(icon) = self.sprites.get_mut(&SpriteId::Part(window as u8, 1)) {
            if icon.enabled || (1..=2).contains(&icon.animation()) {
                icon.stop()?;
                icon.enabled = false;
            }
        }
        Ok(())
    }

    fn read_state(&mut self, vm: &CmvsVm, bit: u32) -> Result<bool> {
        // 0x42e330: unread text stops skipping; READ_SKIP with AUTO_SKIP resumes it.
        let read = if (bit as i32) >= 0 && self.settings[6] != 0 {
            let read = vm.bit(bit).context("CVNS read flag outside the bit bank")?;
            if self.settings[5] != 0 && !read {
                self.settings[5] = 0;
            }
            read
        } else {
            true
        };
        Ok(read)
    }

    pub fn command(
        &mut self,
        selector: u16,
        canvas: [u32; 2],
        muted: bool,
        vm: &mut CmvsVm,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Option<Step>> {
        let opcode = selector + 0x800;
        macro_rules! args {
            ($n:literal) => {
                vm.owner_arguments::<$n>(opcode)?
            };
        }
        macro_rules! done {
            ($words:expr) => {{
                vm.complete_owner_call($words, None)?;
                return Ok(Some(Step::Done));
            }};
            ($words:expr, $result:expr) => {{
                vm.complete_owner_call($words, Some($result))?;
                return Ok(Some(Step::Done));
            }};
        }
        let text = |vm: &CmvsVm, raw: u32| -> Result<Vec<u8>> {
            Ok(vm.text_reference(raw).context("CVNS text operand")?.to_vec())
        };
        let generation = self.masks.generation();
        let font = |host: &mut dyn HostServices, face: &[u8]| -> Result<_> {
            Ok(super::message::resolve_face(host, client, face)?
                .unwrap_or_else(|| std::sync::Arc::from("default")))
        };
        match selector {
            0x2158..=0x215a => {
                use crysirror_core::service::{DesktopRequest, DesktopRequestState, DesktopResponse};
                if self.clipboard.is_none() {
                    let request = match selector {
                        0x2158 => {
                            let frame = match self.capture_frame(host, client) {
                                Ok(Some(frame)) => frame,
                                Ok(None) => return Ok(Some(Step::Pending)),
                                Err(_) => {
                                    self.capture = None;
                                    done!(0, 0)
                                }
                            };
                            let size = frame.size();
                            let [w,h] = size.map(|n| n as usize);
                            let mut rgba = frame.rgba().to_vec();
                            // 0x43bcc0 BitBlts at most 640x480 into a compatible bitmap.
                            for (i, pixel) in rgba.chunks_exact_mut(4).enumerate() {
                                if (w > 640 && i % w >= 640) || (h > 480 && i / w >= 480) {
                                    pixel[..3].fill(0);
                                }
                                pixel[3] = 255;
                            }
                            DesktopRequest::ClipboardImage { size, rgba: rgba.into() }
                        }
                        0x2159 => DesktopRequest::ClipboardClear,
                        _ => {
                            let [reference] = args!(1);
                            let bytes = text(vm, reference)?;
                            DesktopRequest::ClipboardWrite { text: encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned() }
                        }
                    };
                    // OpenClipboard/SetClipboardData failures are ignored by all three wrappers.
                    self.clipboard = host.begin_desktop_request(client, request).ok();
                }
                if let Some(job) = &mut self.clipboard {
                    match job.poll() {
                        Ok(DesktopRequestState::Pending) => return Ok(Some(Step::Pending)),
                        Ok(DesktopRequestState::Ready(DesktopResponse::ClipboardWritten)) | Err(_) => {}
                        Ok(_) => bail!("unexpected CVNS clipboard receipt"),
                    }
                    self.clipboard = None;
                }
                if selector == 0x2158 { done!(0, 1) }
                done!(u16::from(selector == 0x215a))
            }
            0x2048 => {
                let [company] = args!(1);
                self.registry_base = registry::name(&text(vm, company)?)?;
                done!(1)
            }
            0x2049..=0x204c => {
                let key = registry::name(&text(vm, vm.owner_argument(opcode, 4)?)?)?;
                let value = registry::name(&text(vm, vm.owner_argument(opcode, 8)?)?)?;
                let action = match selector {
                    0x2049 => registry::Action::ReadText(vm.owner_argument(opcode, 12)?),
                    0x204a => registry::Action::ReadWord,
                    0x204b => registry::Action::Write(vm.owner_argument(opcode, 12)?.to_le_bytes().to_vec()),
                    _ => registry::Action::Write(text(vm, vm.owner_argument(opcode, 12)?)?),
                };
                return Ok(Some(Step::Registry(registry::Request::new(&self.registry_base, &key, &value, action)?)));
            }
            0x2200 => {
                let [name, size] = args!(2);
                let bytes = text(vm, name)?;
                let (name, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes);
                ensure!(!invalid, "invalid CVNS mask-font name");
                return Ok(Some(Step::MaskSheet(name.into_owned(), size)));
            }
            0x2201 => {
                let [cell, code] = args!(2);
                self.masks.register(cell, code as u16)?;
                done!(2)
            }
            0x20e4 | 0x20e5 => {
                let (base, limit, words) = if selector == 0x20e4 {
                    let [limit] = args!(1);
                    (0, limit, 1)
                } else {
                    let [base, end] = args!(2);
                    (base, end.wrapping_sub(base), 2)
                };
                let sample = msvc_rand(&mut self.random);
                anyhow::ensure!(limit != 0, "CVNS random divisor is zero");
                let offset = sample % (limit as i32);
                done!(words, base.wrapping_add(offset as u32))
            }
            0x2008 => {
                let [enabled] = args!(1);
                self.background = enabled != 0;
                done!(1)
            }
            0x2009 => {
                let [level, color] = args!(2);
                self.primary.invalidate_all(canvas);
                self.level = level.min(256);
                self.color = color;
                done!(2)
            }
            0x200a => {
                let [color] = args!(1);
                self.color = color;
                done!(1)
            }
            0x200b => {
                let [color] = args!(1);
                self.primary.fill(host, client, canvas, color)?;
                done!(1)
            }
            0x200c => {
                let [x,y,w,h] = args!(4).map(|v| v as i32);
                self.primary.invalidate(canvas,[x,y,x.wrapping_add(w),y.wrapping_add(h)]);
                done!(4)
            }
            0x200d => {
                self.primary.cancel();
                done!(0)
            }
            0x2030 => {
                let [x, y] = self.input.pointer();
                vm.complete_owner_results(0, &[(1, x as u32), (2, y as u32)])?;
                return Ok(Some(Step::Done));
            }
            0x2031 => {
                let [x, y, w, h] = args!(4).map(|word| word as i32);
                let [px, py] = self.input.pointer();
                let inside = px >= x && px <= x + w && py >= y && py <= y + h;
                done!(4, u32::from(inside))
            }
            // The window procedure writes +2744 only. +2752 has a separate reset.
            0x2038 | 0x203a => done!(0, self.wheel[usize::from(selector == 0x203a)]),
            0x2039 | 0x203b => {
                self.wheel[usize::from(selector == 0x203b)] = 0;
                done!(0)
            }
            0x203d => done!(0, u32::from(self.input.words[24] != 0)),
            0x2042 => {
                // 0x437480 reads one input word by number.
                let [query] = args!(1);
                let index = match query {
                    1 => Some(18),
                    2 => Some(19),
                    3 => Some(24),
                    16..=21 => Some(2 * (query as usize - 16)),
                    22 => Some(12),
                    23 => Some(14),
                    256 => Some(33),
                    257 => Some(34),
                    258 => Some(38),
                    259 => Some(29),
                    260 => Some(45),
                    261 => Some(50),
                    262 => Some(51),
                    _ => None,
                };
                done!(1, index.map_or(0, |index| self.input.words[index]))
            }
            0x2047 => {
                let skipping = self.settings[5] != 0 || self.input.words[33] as i32 > 0 || self.input.words[12] as i32 > 0;
                done!(0, u32::from(skipping))
            }
            0x206c | 0x206d => {
                // +1676/+1680: synthetic left/right edges in this frame.
                self.input.words[if selector == 0x206c { 18 } else { 19 }] = 1;
                done!(0)
            }
            0x2100 | 0x2101 => {
                let [index, value] = args!(2);
                let window = self.message(index as usize)?;
                if selector == 0x2100 {
                    window.gradient = value;
                } else {
                    window.layer = value;
                }
                done!(2)
            }
            0x2040 => {
                let [peek] = args!(1);
                let mask = self.input.mask(peek == 0);
                done!(1, mask)
            }
            0x2051..=0x2053 | 0x2130 => {
                let (slot, layer, position, name, flag, frame, words) = match selector {
                    0x2051 => {
                        let [index, name, flag] = args!(3);
                        (BitmapSlot::Global(bitmap_index(index)?), 0, [0; 2], name, flag, 0, 3)
                    }
                    0x2052 => {
                        let [index, name, frame, flag] = args!(4);
                        (BitmapSlot::Global(bitmap_index(index)?), 0, [0; 2], name, flag, frame, 4)
                    }
                    0x2053 => {
                        let [index, layer, x, y, name, flag] = args!(6);
                        (BitmapSlot::Global(bitmap_index(index)?), component_index(layer)?, [x as i32, y as i32], name, flag, 0, 6)
                    }
                    _ => {
                        let [window, index, name, flag] = args!(4);
                        ensure!((index as usize) < adv::BITMAPS, "CVNS ADV bitmap {index} outside 0..4");
                        self.adv(window)?;
                        (BitmapSlot::Adv(window as usize, index as usize), 0, [0; 2], name, flag, 0, 4)
                    }
                };
                let bytes = text(vm, name)?;
                let (name, _, invalid) = encoding_rs::SHIFT_JIS.decode(&bytes);
                ensure!(!invalid && !name.is_empty(), "invalid CVNS bitmap name");
                let picture = if let (0x2053, BitmapSlot::Global(index)) = (selector, slot) {
                    self.picture(index)?.clone()
                } else { bitmap::Picture::default() };
                if matches!(slot, BitmapSlot::Global(_)) { vm.record_seen_image(&name); }
                let edit = bitmap::Edit::Load { layer, position, flag, name: bytes, frame };
                return Ok(Some(Step::Bitmap(bitmap::Job {slot, words, picture, edit})));
            }
            0x2050 | 0x2057 | 0x2058 | 0x205c..=0x205f | 0x2062 | 0x2063 | 0x2066 | 0x2067 => {
                use bitmap::Edit;
                let [dest] = args!(1);
                let dest = bitmap_index(dest)?;
                if selector == 0x2062 {
                    let [_, layer] = args!(2);
                    match self.picture(dest)?.size(component_index(layer)?) {
                        None => done!(2, 0),
                        Some([w,h]) => vm.complete_owner_results(2, &[(0,1), (1,w), (2,h)])?,
                    }
                    return Ok(Some(Step::Done));
                }
                let mut picture = if matches!(selector, 0x2050 | 0x205c | 0x205d) { bitmap::Picture::default() } else { self.picture(dest)?.clone() };
                let (words, edit) = match selector {
                    0x2050 => { let [_,w,h] = args!(3); (3, Edit::Create([w,h])) }
                    0x2057 => {
                        let [_,layer] = args!(2);
                        ensure!(layer != 0, "CVNS cannot release component zero");
                        (2, Edit::Release(component_index(layer)?))
                    }
                    0x2058 => (1, Edit::Bake),
                    0x205c | 0x205d => {
                        let [_, from] = args!(2);
                        let from = bitmap_index(from)?;
                        self.picture(from)?;
                        if selector == 0x205c {
                            self.release(host, client, BitmapSlot::Global(dest))?;
                            if from != dest {
                                self.bitmaps[dest] = self.bitmaps[from].take();
                                self.pictures[dest] = self.pictures[from].take();
                            }
                            done!(2)
                        }
                        if from != dest { picture = self.picture(from)?.clone(); }
                        (2, Edit::Copy)
                    }
                    0x205e => { let [_,layer,color] = args!(3); (3, Edit::Tint(layer as usize, color)) }
                    0x205f => { let [_,layer,mode,param] = args!(4); (4, Edit::Filter(component_index(layer)?,mode,param)) }
                    0x2063 => { let [_,layer,x,y] = args!(4); (4, Edit::Position(component_index(layer)?,[x as i32,y as i32])) }
                    0x2066 => { let [_,color] = args!(2); (2, Edit::Fill(color)) }
                    _ => {
                        let [_,to,source,from,x,y] = args!(6);
                        let source = bitmap_index(source)?;
                        (6, Edit::CopyLayer { source: self.picture(source)?.clone(), from: component_index(from)?,
                            to: component_index(to)?, position: [x as i32,y as i32], same: dest == source && to == from })
                    }
                };
                return Ok(Some(Step::Bitmap(bitmap::Job {slot:BitmapSlot::Global(dest),words,picture,edit})));
            }
            0x2055 => {
                let [index] = args!(1);
                ensure!((index as usize) < BITMAPS, "CVNS bitmap {index} outside 0..127");
                self.release(host, client, BitmapSlot::Global(index as usize))?;
                done!(1)
            }
            0x2056 => {
                for index in 0..BITMAPS {
                    self.release(host, client, BitmapSlot::Global(index))?;
                }
                done!(0)
            }
            0x2059 => {
                let [index] = args!(1);
                ensure!((index as usize) < BITMAPS, "CVNS bitmap {index} outside 0..127");
                if let Some(picture) = self.pictures[index as usize].as_ref() {
                    if let Some([w,h]) = picture.size(0) {
                        vm.complete_owner_results(1, &[(0, 1), (1, w), (2, h)])?;
                    } else {
                        // Native size outputs are undefined for a componentless object.
                        vm.complete_owner_call(1, Some(1))?;
                    }
                } else {
                    vm.complete_owner_call(1, Some(0))?;
                }
                return Ok(Some(Step::Done));
            }
            0x2160 => {
                self.copy_next_frame = true;
                vm.finish_owner_call(0, None, true)?;
                return Ok(Some(Step::Frame));
            }
            0x2163 => {
                let [index] = args!(1);
                if index >= BITMAPS as u32 { done!(1); }
                let edit = match &self.copied_screen {
                    Some(frame) => bitmap::Edit::Capture(frame.clone()),
                    None => bitmap::Edit::Create(canvas),
                };
                return Ok(Some(Step::Bitmap(bitmap::Job {
                    slot: BitmapSlot::Global(index as usize), words: 1, picture: Default::default(), edit,
                })));
            }
            0x205a => {
                let [index] = args!(1);
                let slot = BitmapSlot::Global(bitmap_index(index)?);
                let Some(image) = self.capture_frame(host, client)? else {
                    return Ok(Some(Step::Pending));
                };
                let edit = bitmap::Edit::Capture(image);
                return Ok(Some(Step::Bitmap(bitmap::Job { slot, words: 1, picture: Default::default(), edit })));
            }
            0x205b => {
                let [index] = args!(1);
                ensure!((index as usize) < BITMAPS, "CVNS bitmap {index} outside 0..127");
                done!(1, u32::from(self.pictures[index as usize].is_some()))
            }
            0x20c8..=0x20cb => {
                let slot = vm.owner_argument(opcode, 4)? as usize;
                if slot >= movie::COUNT {
                    return Ok(Some(Step::Fault { words: if selector == 0x20c8 { 2 } else { 1 }, flags: 0x800000 }));
                }
                match selector {
                    0x20c8 => {
                        let reference = vm.owner_argument(opcode, 8)?;
                        let path = vm.movie_path_for(reference)?;
                        let name = text(vm, reference)?;
                        self.movies.remove(host, client, slot)?;
                        return Ok(Some(Step::Movie(slot, path, name)));
                    }
                    0x20c9 => {
                        self.movies.start(host, client, slot, if muted { 0. } else { self.movie_gain() })?;
                        done!(1, 1)
                    }
                    0x20ca => done!(1, self.movies.position(host, client, slot, self.audio_lead_ms)?),
                    _ => {
                        self.movies.remove(host, client, slot)?;
                        done!(1)
                    }
                }
            }
            0x20cc => {
                for slot in 0..movie::COUNT { self.movies.remove(host, client, slot)?; }
                done!(0)
            }
            0x2070..=0x2072 | 0x2074 => {
                let id = global(vm.owner_argument(opcode, 4)?)?;
                match selector {
                    0x2070 | 0x2071 => {
                        let kind = if selector == 0x2071 { Kind::Movie } else { Kind::Bitmap };
                        self.sprites.insert(id, Sprite::new(kind));
                        done!(1)
                    }
                    0x2072 => {
                        let [_, x, y, w, h, color] = args!(6);
                        let mut fill = Sprite::new(Kind::Fill);
                        fill.position = [x as i32, y as i32];
                        fill.crop = [0, 0, w as i16, h as i16];
                        fill.size = [w as i32, h as i32];
                        fill.fill = color;
                        self.sprites.insert(id, fill);
                        done!(6)
                    }
                    _ => {
                        self.sprites.remove(&id);
                        done!(1)
                    }
                }
            }
            0x2075 => {
                self.sprites.retain(|id, _| !matches!(id, SpriteId::Global(_)));
                done!(0)
            }
            0x2076 | 0x213a => {
                let (id, kind, [bitmap, x, y, w, h], words) = if selector == 0x2076 {
                    let [index, bitmap, x, y, w, h] = args!(6);
                    {
                        let id = global(index)?;
                        let kind = sprite_mut(&mut self.sprites, id)?.kind;
                        (id, kind, [bitmap, x, y, w, h], 6)
                    }
                } else {
                    let [window, index, bitmap, x, y, w, h] = args!(7);
                    (part(window, index)?, Kind::Part(window as u8), [bitmap, x, y, w, h], 7)
                };
                let Some(size) = self.bitmap_size(kind, bitmap) else {
                    return Ok(Some(Step::Fault { words, flags: if selector == 0x2076 { 0x800000 } else { 0x400000 } }));
                };
                let sprite = sprite_mut(&mut self.sprites, id)?;
                // 0x427c10 clamps negative origins to 0 and negative sizes to the bitmap.
                let x = (x as i32).max(0) as i16;
                let y = (y as i32).max(0) as i16;
                let w = if (w as i32) < 0 { size[0] as i16 } else { w as i16 };
                let h = if (h as i32) < 0 { size[1] as i16 } else { h as i16 };
                sprite.set_source(bitmap as i16, [x, y, w, h]);
                done!(words)
            }
            0x2077 | 0x2148 => {
                let (id, [bitmap, x, y, w, h], words) = if selector == 0x2077 {
                    let [index, bitmap, x, y, w, h] = args!(6);
                    (global(index)?, [bitmap, x, y, w, h], 6)
                } else {
                    let [window, index, bitmap, x, y, w, h] = args!(7);
                    (part(window, index)?, [bitmap, x, y, w, h], 7)
                };
                sprite_mut(&mut self.sprites, id)?.set_source(bitmap as i16, [x, y, w, h].map(|word| word as i16));
                done!(words)
            }
            0x2131 => {
                let [window, index] = args!(2);
                if index >= adv::BITMAPS as u32 || self.adv(window).is_err() {
                    return Ok(Some(Step::Fault { words: 2, flags: 0x800000 }));
                }
                self.release(host, client, BitmapSlot::Adv(window as usize, index as usize))?;
                done!(2)
            }
            0x213e | 0x213f | 0x2141..=0x2143 | 0x214f => {
                let [window, index] = args!(2);
                let words = match selector {
                    0x214f => 2,
                    0x213f | 0x2142 => 4,
                    _ => 3,
                };
                let fault = Step::Fault { words, flags: 0x0800_0000 };
                let Ok(id) = part(window, index) else { return Ok(Some(fault)); };
                if self.adv(window).is_err() { return Ok(Some(fault)); }
                let part = self.sprites.get_mut(&id);
                if selector == 0x214f { done!(2, u32::from(part.is_some())); }
                let Some(part) = part else { return Ok(Some(fault)); };
                let value = vm.owner_argument(opcode, 12)?;
                match selector {
                    0x213e => part.alpha = value.min(256) as u16,
                    0x213f => part.tint = (value & 0xff_ffff, vm.owner_argument(opcode, 16)? as i16),
                    0x2141 => part.rotation = (value as i32).wrapping_abs() % 360,
                    0x2142 => part.scale = [value as i32, vm.owner_argument(opcode, 16)? as i32],
                    _ => part.enabled = value != 0,
                }
                done!(words)
            }
            0x2078 => {
                let [index, bitmap] = args!(2);
                sprite_mut(&mut self.sprites, global(index)?)?.source = bitmap as i16;
                done!(2)
            }
            0x2079 | 0x213b => {
                let (id, x, y, words) = if selector == 0x2079 {
                    let [index, x, y, _, _] = args!(5);
                    (global(index), x, y, 5)
                } else {
                    let [window, index, x, y, _, _] = args!(6);
                    (part(window, index), x, y, 6)
                };
                let Some(sprite) = id.ok().and_then(|id| self.sprites.get_mut(&id)) else {
                    return Ok(Some(Step::Fault { words, flags: 0x8000000 }));
                };
                // 0x41b050 consumes four words but derives size from the crop.
                sprite.position = [x as i32, y as i32];
                sprite.measure_crop();
                done!(words)
            }
            0x207a | 0x207f | 0x2081 | 0x213c | 0x2140 => {
                let (id, [a, b], words) = if selector < 0x2100 {
                    let [index, a, b] = args!(3);
                    (global(index)?, [a, b], 3)
                } else {
                    let [window, index, a, b] = args!(4);
                    (part(window, index)?, [a, b], 4)
                };
                let sprite = sprite_mut(&mut self.sprites, id)?;
                let pair = [a as i32, b as i32];
                match selector {
                    0x207a | 0x213c => sprite.position = pair,
                    0x207f | 0x2140 => sprite.origin = pair,
                    _ => sprite.scale = pair,
                }
                done!(words)
            }
            0x207b..=0x207d | 0x2080 | 0x2082 | 0x213d => {
                let (id, value, words) = if selector == 0x213d {
                    let [window, index, value] = args!(3);
                    (part(window, index)?, value, 3)
                } else {
                    let [index, value] = args!(2);
                    (global(index)?, value, 2)
                };
                let sprite = sprite_mut(&mut self.sprites, id)?;
                match selector {
                    0x207b | 0x213d if value < 128 => sprite.layer = value,
                    0x207b | 0x213d => {}
                    0x207c => sprite.alpha = value.min(256) as u16,
                    0x207d => sprite.hidden = value as i16,
                    0x2080 => sprite.rotation = (value as i32).unsigned_abs() as i32 % 360,
                    _ => sprite.enabled = value != 0,
                }
                done!(words)
            }
            0x207e => {
                let [index, color, weight] = args!(3);
                sprite_mut(&mut self.sprites, global(index)?)?.tint = (color & 0xff_ffff, weight as i16);
                done!(3)
            }
            0x2083 => {
                let [index] = args!(1);
                let exists = self.sprites.contains_key(&global(index)?);
                done!(1, u32::from(exists))
            }
            0x2084 => {
                let [index] = args!(1);
                let [x, y] = sprite_mut(&mut self.sprites, global(index)?)?.position;
                vm.complete_owner_results(1, &[(1, x as u32), (2, y as u32)])?;
                return Ok(Some(Step::Done));
            }
            0x2086 => {
                let [index] = args!(1);
                let layer = sprite_mut(&mut self.sprites, global(index)?)?.layer;
                done!(1, layer)
            }
            0x2085 => {
                let [index] = args!(1);
                let Some(sprite) = global(index).ok().and_then(|id| self.sprites.get(&id)) else {
                    return Ok(Some(Step::Fault { words: 1, flags: 0x8000000 }));
                };
                let [width, height] = sprite.size;
                vm.complete_owner_results(1, &[(1, width as u32), (2, height as u32)])?;
                return Ok(Some(Step::Done));
            }
            0x2088 => {
                // 0x428770 moves record SP-8 to SP-4 when the source exists.
                let [target, source] = args!(2);
                let (target, source) = (global(target)?, global(source)?);
                let moved = self.sprites.remove(&source).context("CVNS moved sprite is absent")?;
                self.sprites.insert(target, moved);
                done!(2)
            }
            0x2090 | 0x2091 | 0x2144 | 0x2145 => {
                let adv = selector >= 0x2144;
                let words: u16 = match selector {
                    0x2090 => 12,
                    0x2091 => 11,
                    0x2144 => 13,
                    _ => 12,
                };
                let mut values = [0u32; 13];
                for (index, value) in values.iter_mut().take(usize::from(words)).enumerate() {
                    *value = vm.owner_argument(opcode, 4 * (index + 1))?;
                }
                let (id, rest) = if adv {
                    (part(values[0], values[1])?, &values[2..])
                } else {
                    (global(values[0])?, &values[1..])
                };
                // 0x41a690: frame, next, duration, bitmap, crop, origin and slot.
                let word = |at: usize| rest[at] as i16;
                let frame = Frame {
                    bitmap: word(3),
                    crop: [word(4), word(5), word(6), word(7)],
                    origin: [word(8), word(9)],
                    duration: word(2),
                    next: word(1),
                };
                let slot = matches!(selector, 0x2090 | 0x2144).then(|| rest[10]).filter(|slot| (*slot as i32) > -1);
                sprite_mut(&mut self.sprites, id)?.set_frame(rest[0], frame, slot)?;
                done!(words)
            }
            0x2092 | 0x2146 => {
                let (id, slot, words) = if selector == 0x2092 {
                    let [index, slot] = args!(2);
                    (global(index)?, slot, 2)
                } else {
                    let [window, index, slot] = args!(3);
                    (part(window, index)?, slot, 3)
                };
                sprite_mut(&mut self.sprites, id)?.start(slot)?;
                done!(words)
            }
            0x2093 | 0x2147 => {
                let (id, words) = if selector == 0x2093 {
                    let [index] = args!(1);
                    (global(index)?, 1)
                } else {
                    let [window, index] = args!(2);
                    (part(window, index)?, 2)
                };
                sprite_mut(&mut self.sprites, id)?.stop()?;
                done!(words)
            }
            0x2094 => {
                let [index] = args!(1);
                let state = sprite_mut(&mut self.sprites, global(index)?)?.animation();
                done!(1, state as u32)
            }
            // Message windows 0x20d0..0x20df.
            0x20d9..=0x20db | 0x2102 => {
                let [slot] = args!(1);
                let words = if matches!(selector, 0x20d9 | 0x20db) { 2 } else { 1 };
                let argument = if words == 2 { vm.owner_argument(opcode, 8)? } else { 0 };
                let content = if selector == 0x20db { text(vm, argument)? } else { Vec::new() };
                let Some(window) = self.messages.get_mut(slot as usize).and_then(Option::as_mut) else {
                    return Ok(Some(Step::Fault { words, flags: 0x400000 }));
                };
                match selector {
                    0x20d9 => window.kinsoku = argument != 0,
                    0x20da => done!(1, window.revealing() as u32),
                    0x20db => window.print(&content, None, true, generation)?,
                    _ => window.complete(),
                }
                done!(words)
            }
            0x20d0 | 0x20d1 => {
                let [index] = args!(1);
                ensure!((index as usize) < MESSAGES, "CVNS message window {index} outside 0..7");
                self.messages[index as usize] = if selector == 0x20d0 { Some(text::Window::new(font(host, &self.font_name)?)) } else { None };
                done!(1)
            }
            0x20d2..=0x20d8 | 0x20dc..=0x20df => {
                let index = vm.owner_argument(opcode, 4)? as usize;
                ensure!(index < MESSAGES, "CVNS message window {index} outside 0..7");
                let style = None;
                let words = match selector {
                    0x20d5 => 1,
                    0x20dd => 2,
                    0x20d7 | 0x20df => 2,
                    0x20d6 => 4,
                    0x20d4 => 5,
                    _ => 3,
                };
                let mut values = [0u32; 5];
                for (at, value) in values.iter_mut().take(words).enumerate() {
                    *value = vm.owner_argument(opcode, 4 * (at + 1))?;
                }
                let bytes = matches!(selector, 0x20d7 | 0x20df).then(|| text(vm, values[1])).transpose()?;
                let face = if selector == 0x20d7 {
                    let name = bytes.as_deref().filter(|s| !s.is_empty()).unwrap_or(&self.font_name);
                    Some(font(host, name)?)
                } else { None };
                let window = self.message(index)?;
                let [_, a, b, c, d] = values;
                match selector {
                    0x20d2 => window.position = [a as i32, b as i32],
                    0x20d3 => window.buffer = [a as i32, b as i32],
                    0x20d4 => window.set_area([a, b, c, d].map(|word| word as i32)),
                    0x20d5 => window.clear(),
                    0x20d6 => {
                        if (a as i32) > 0 {
                            window.size = a as i32;
                        }
                        if (b as i32) >= 0 {
                            window.spacing = b as i32;
                        }
                        if (c as i32) >= 0 {
                            window.line_spacing = c as i32;
                        }
                    }
                    0x20d7 => {
                        window.default_face = bytes.as_deref().is_none_or(<[u8]>::is_empty);
                        window.native_face = bytes.clone().unwrap_or_default();
                        window.font = face.context("CVNS font face")?;
                    }
                    0x20d8 => {
                        if a & 0xff00_0000 == 0 {
                            window.colors[0] = a;
                        }
                        if b & 0xff00_0000 == 0 {
                            window.colors[1] = b;
                        }
                    }
                    0x20dc => window.reveal = [a, b],
                    0x20dd => window.delay = a,
                    0x20de => window.cursor = [a as i32, b as i32],
                    _ => window.print(&bytes.unwrap_or_default(), style, false, generation)?,
                }
                done!(words as u16)
            }
            // ADV windows 0x2110..0x2156.
            0x2110 => {
                let [window, message, x, y, w, h] = args!(6);
                ensure!((window as usize) < ADVS, "CVNS ADV window {window} outside 0..1");
                ensure!((message as usize) < MESSAGES, "CVNS message window {message} outside 0..7");
                self.destroy_adv(host, client, window as usize)?;
                // 0x420600 resizes the new window and resets its text defaults.
                let mut text = text::Window::new(font(host, &self.font_name)?);
                text.buffer = [w as i32, h as i32];
                text.position = [x as i32, y as i32];
                text.set_area([0, 0, w as i32, h as i32]);
                text.size = 24;
                self.messages[message as usize] = Some(text);
                self.advs[window as usize] = Some(adv::Adv::new(message as usize, [x, y, w, h].map(|word| word as i32)));
                done!(6)
            }
            0x2111 => {
                let [window, shown] = args!(2);
                let shown = shown != 0;
                let adv = self.adv(window)?;
                if adv.shown != shown {
                    adv.user_hidden = false;
                    adv.shown = shown;
                    let [x, y, ..] = adv.rect;
                    let message = self.adv_message(window)?;
                    message.shown = shown;
                    if shown {
                        message.position = [x, y];
                    }
                    self.show_parts(window, shown)?;
                }
                self.adv(window)?.user_hidden = false;
                done!(2)
            }
            0x2112 => {
                let [window, bit, raw] = args!(3);
                let read = self.read_state(vm, bit)?;
                if (bit as i32) >= 0 && self.settings[6] != 0 && self.settings[7] != 0 && self.settings[5] == 0 && read {
                    self.settings[5] = 1;
                }
                let first = !vm.repeats_system();
                let style = self.style();
                if first {
                    let bytes = text(vm, raw)?;
                    self.adv_message(window)?.print(&bytes, Some(style), true, generation)?;
                }
                if self.print_done(window, read)? {
                    vm.finish_owner_call(3, None, true)?;
                    return Ok(Some(Step::Frame));
                }
                vm.wait_owner_call(opcode, None)?;
                return Ok(Some(Step::Frame));
            }
            0x2113 => {
                let [window] = args!(1);
                self.adv(window)?;
                self.destroy_adv(host, client, window as usize)?;
                done!(1)
            }
            0x2114 | 0x2119 | 0x2121 | 0x2124 | 0x2125 => {
                let [window, value] = if selector == 0x2121 { [args!(1)[0], 0] } else { args!(2) };
                let message = self.adv_message(window)?;
                match selector {
                    0x2114 if (value as i32) > 0 => message.size = value as i32,
                    0x2114 => {}
                    0x2119 => message.delay = value,
                    0x2121 => message.clear(),
                    0x2124 => message.gradient = value,
                    _ => message.layer = value,
                }
                done!(if selector == 0x2121 { 1 } else { 2 })
            }
            0x2117 => {
                let [window, raw] = args!(2);
                let bytes = text(vm, raw)?;
                let style = self.style();
                self.adv_message(window)?.print(&bytes, Some(style), false, generation)?;
                done!(2)
            }
            0x2118 | 0x211e | 0x2122 | 0x2123 => {
                let [window, a, b] = args!(3);
                let message = self.adv_message(window)?;
                match selector {
                    0x2118 => {
                        if (a as i32) >= 0 {
                            message.spacing = a as i32;
                        }
                        if (b as i32) >= 0 {
                            message.line_spacing = b as i32;
                        }
                    }
                    0x211e => {
                        if a & 0xff00_0000 == 0 {
                            message.colors[0] = a;
                        }
                        if b & 0xff00_0000 == 0 {
                            message.colors[1] = b;
                        }
                    }
                    0x2122 => message.cursor = [a as i32, b as i32],
                    _ => message.reveal = [a, b],
                }
                done!(3)
            }
            0x2126 => {
                let [window, mode, x, y] = args!(4);
                let Some(adv) = self.advs.get_mut(window as usize).and_then(Option::as_mut) else {
                    return Ok(Some(Step::Fault { words: 4, flags: 0x200000 }));
                };
                let previous = [adv.rect[0], adv.rect[1]];
                let point = [x as i32, y as i32];
                let delta = if mode == 0 {
                    [point[0].wrapping_sub(previous[0]), point[1].wrapping_sub(previous[1])]
                } else { point };
                let position = if mode == 1 {
                    [point[0].wrapping_add(previous[0]), point[1].wrapping_add(previous[1])]
                } else { point };
                adv.rect[..2].copy_from_slice(&position);
                for button in adv.buttons.iter_mut().filter(|b| b.enabled) {
                    button.rect[0] = button.rect[0].wrapping_add(delta[0] as i16);
                    button.rect[1] = button.rect[1].wrapping_add(delta[1] as i16);
                }
                for (id, sprite) in &mut self.sprites {
                    if matches!(id, SpriteId::Part(owner, _) if u32::from(*owner) == window) {
                        sprite.position[0] = sprite.position[0].wrapping_add(delta[0]);
                        sprite.position[1] = sprite.position[1].wrapping_add(delta[1]);
                    }
                }
                self.adv_message(window)?.position = position;
                done!(4)
            }
            0x2139 => {
                let [window, index] = args!(2);
                let valid = self.advs.get(window as usize).is_some_and(Option::is_some)
                    && index < u32::from(sprite::PARTS)
                    && self.sprites.remove(&SpriteId::Part(window as u8, index as u8)).is_some();
                if !valid { return Ok(Some(Step::Fault { words: 2, flags: 0x8000000 })); }
                done!(2)
            }
            0x211a => {
                let [window, mode, x, y] = args!(4);
                let adv = self.adv(window)?;
                adv.icon_mode = mode as i16;
                match mode {
                    0 => adv.icon_offset = [0, 0],
                    1 => adv.icon_offset = [x as i16, y as i16],
                    _ => {}
                }
                done!(4)
            }
            0x211b => {
                let [window, hide] = args!(2);
                let adv = self.adv(window)?;
                if adv.shown && adv.hide_allowed {
                    adv.user_hidden = hide != 0;
                    self.adv_message(window)?.shown = hide == 0;
                    self.show_parts(window, hide == 0)?;
                }
                done!(2)
            }
            0x211d => {
                let [window, x, y, w, h] = args!(5);
                self.adv_message(window)?.set_area([x, y, w, h].map(|word| word as i32));
                done!(5)
            }
            0x211f => {
                let [window] = args!(1);
                let hidden = self.adv(window)?.user_hidden;
                done!(1, u32::from(hidden))
            }
            0x2120 => {
                let [window, bit, voice] = args!(3);
                // 0x42e250: with READ_SKIP the read flag also gates skipping.
                let (skip_ok, read) = if (bit as i32) >= 0 && self.settings[6] != 0 {
                    let read = self.read_state(vm, bit)?;
                    (read, read)
                } else {
                    (true, (bit as i32) < 0 || vm.bit(bit).context("CVNS read flag outside the bit bank")?)
                };
                if self.wait_done(window, skip_ok, voice as i32, read, host, client)? {
                    done!(3)
                }
                vm.wait_owner_call(opcode, None)?;
                return Ok(Some(Step::Frame));
            }
            0x2138 => {
                let [window, index] = args!(2);
                let id = part(window, index)?;
                self.adv(window)?;
                let mut sprite = Sprite::new(Kind::Part(window as u8));
                sprite.enabled = false;
                self.sprites.insert(id, sprite);
                done!(2)
            }
            0x2150 => {
                let [window, button, state, part_index, bitmap, x, y, w, h, dx, dy] = args!(11);
                let entry = self.button(window, button)?;
                *entry.states.get_mut(state as usize).context("CVNS button state outside 0..5")? = adv::ButtonState {
                    bitmap: bitmap as i16,
                    crop: [x, y, w, h].map(|word| word as i16),
                    offset: [dx as i16, dy as i16],
                };
                entry.part = part_index as u16;
                done!(11)
            }
            0x2151 => {
                let [window, button, x, y, w, h] = args!(6);
                let [left, top, ..] = self.adv(window)?.rect;
                self.button(window, button)?.rect =
                    [(x as i32 + left) as i16, (y as i32 + top) as i16, w as i16, h as i16];
                done!(6)
            }
            0x2153 => {
                let [window] = args!(1);
                let point = self.input.pointer();
                let clicked = self.input.words[18] != 0;
                let hit = self.adv(window)?.hit(point);
                let results = [(4, hit.map_or(u32::MAX, |index| index as u32)), (5, u32::from(hit.is_some() && clicked))];
                vm.complete_owner_results(1, &results)?;
                return Ok(Some(Step::Done));
            }
            0x2154 => {
                let [window, button] = args!(2);
                let enabled = self.button(window, button)?.enabled;
                done!(2, u32::from(enabled))
            }
            0x2152 => {
                let [window, button] = args!(2);
                if self.advs.get(window as usize).is_none_or(Option::is_none) {
                    return Ok(Some(Step::Fault { words: 2, flags: 0x100000 }));
                }
                let entry = self.button(window, button)?;
                done!(2, u32::from(entry.state))
            }
            0x2155 => {
                let [window, button, value] = args!(3);
                let entry = self.button(window, button)?;
                entry.enabled = true;
                entry.value = value as u16;
                done!(3)
            }
            0x2156 => {
                // 0x42d790 changes only the part source, and only for a new state.
                let [window, button, state] = args!(3);
                let entry = self.button(window, button)?;
                entry.state = state as u16;
                let changed = entry.applied != entry.state;
                entry.applied = entry.state;
                let image = *entry.states.get(state as usize).context("CVNS button state outside 0..5")?;
                let part_index = entry.part;
                if changed {
                    sprite_mut(&mut self.sprites, part(window, u32::from(part_index))?)?
                        .set_source(image.bitmap, image.crop);
                }
                done!(3)
            }
            0x2188 | 0x2189 => {
                let (name, target, duration, edge, reveal, screen) = if selector == 0x2189 {
                    let [name, duration, edge] = args!(3);
                    let Some(image) = self.capture_frame(host, client)? else { return Ok(Some(Step::Pending)); };
                    (name, 0, duration, edge, false, Some(image))
                } else {
                    let [name, target, duration, edge, reveal] = args!(5);
                    let target = if target == u32::MAX { 0 } else { target };
                    if let Some(sprite) = self.sprites.get_mut(&global(target)?) {
                        sprite.rule_blit = true;
                    }
                    (name, target as u16, duration, edge, reveal != 0, None)
                };
                let bytes = text(vm, name)?;
                let (name, _, invalid) = encoding_rs::SHIFT_JIS.decode(&bytes);
                ensure!(!invalid, "invalid CVNS rule name");
                return Ok(Some(Step::Rule(rule::Request { name: name.into_owned(), native_name: bytes, target, duration, edge, reveal, screen })));
            }
            0x218a => done!(0, self.rule.as_ref().map_or(u32::MAX, rule::Active::remaining)),
            0x218b => {
                self.finish_rule(host, client)?;
                done!(0)
            }
            0x218c => {
                let [selection] = args!(1);
                if self.rule.is_none() { done!(1, 0) }
                let skipping = self.settings[5] != 0 || self.input.words[33] as i32 > 0 || self.input.words[12] as i32 > 0;
                if (selection != 0 && (self.settings[3] == 0 || skipping)) || self.input.take(selection, true) {
                    self.finish_rule(host, client)?;
                    vm.finish_owner_call(1, Some(1), true)?;
                } else {
                    vm.wait_owner_call(opcode, Some(0))?;
                }
                return Ok(Some(Step::Frame));
            }
            0x218d => {
                let [index, kind, duration, a6, a7, a8] = args!(6);
                let id = global(index)?;
                let random = &mut self.random;
                let sprite = sprite_mut(&mut self.sprites, id)?;
                let base = sprite.base();
                sprite.effect = Some(fx::Effect::new(kind, duration, a6 as i32, a7 as i32, a8 as i32, base, &mut || msvc_rand(random))?);
                done!(6)
            }
            0x218e => {
                // 0x42fb20 finishes when the selected input is used or a skip is active.
                let [index, selection, consume] = args!(3);
                let id = global(index)?;
                let skipping = self.settings[5] != 0 || self.input.words[33] as i32 > 0 || self.input.words[12] as i32 > 0;
                let effects = self.settings[3] != 0;
                let Some(sprite) = self.sprites.get_mut(&id).filter(|sprite| sprite.effect.is_some()) else {
                    done!(3, 0)
                };
                let _ = sprite;
                if (selection != 0 && (!effects || skipping)) || self.input.take(selection, consume != 0) {
                    sprite_mut(&mut self.sprites, id)?.finish_effect();
                    vm.finish_owner_call(3, Some(1), true)?;
                    return Ok(Some(Step::Frame));
                }
                vm.wait_owner_call(opcode, Some(0))?;
                return Ok(Some(Step::Frame));
            }
            0x218f => {
                let [reveal, color, duration] = args!(3);
                self.transition = Some(Transition {
                    reveal,
                    color,
                    duration,
                    elapsed: 0,
                });
                done!(3, 1)
            }
            0x2191 => {
                self.finish_transition();
                done!(0)
            }
            0x2190 => {
                let remaining = self.transition.as_ref().map_or(u32::MAX, |transition| {
                    transition.duration.saturating_sub(transition.elapsed)
                });
                done!(0, remaining)
            }
            0x2192 => {
                let [skippable] = args!(1);
                if self.transition.is_none() {
                    done!(1, 0)
                }
                // 0x42f9e0 finishes at once for a skippable wait unless effects are on
                // and no skip is held; otherwise one consumed selected input finishes it.
                let skipping = self.settings[5] != 0 || self.input.words[33] as i32 > 0 || self.input.words[12] as i32 > 0;
                let waits = skippable == 0 || (self.settings[3] != 0 && !skipping);
                if !waits || self.input.take(skippable, true) {
                    self.finish_transition();
                    vm.finish_owner_call(1, Some(1), true)?;
                    return Ok(Some(Step::Frame));
                }
                vm.wait_owner_call(opcode, Some(0))?;
                return Ok(Some(Step::Frame));
            }
            0x2193 => {
                self.shake = Some(shake::Shake::new(args!(4)));
                done!(4)
            }
            0x2194 => {
                let [wait] = args!(1);
                if self.shake.is_none() { done!(1, 1) }
                if wait != 0 {
                    vm.wait_owner_call(opcode, Some(0))?;
                } else {
                    self.shake = None;
                    vm.finish_owner_call(1, Some(1), true)?;
                }
                return Ok(Some(Step::Frame));
            }
            0x2195 => {
                let [pause] = args!(1);
                if let Some(shake) = &mut self.shake { shake.paused = pause != 0; }
                done!(1)
            }
            0x2196 => done!(0, self.shake.as_ref().map_or(u32::MAX, shake::Shake::remaining)),
            0x2004 => {
                let [mode] = args!(1);
                self.reset_clock(mode != 0, host.now_ms() as u32);
                done!(1)
            }
            0x201f => {
                let [enabled] = args!(1);
                self.exclude_vm_time = enabled != 0;
                done!(1)
            }
            // 0x413870 sets bit 3 for a 32-bit display mode. The shared
            // presentation surface for this canvas exposes RGBA8.
            0x2183 => done!(0, 8),
            0x2018 => done!(0, if self.fixed_clock { self.accumulated } else { self.sampled_at }),
            0x2019 => done!(0, self.delta),
            0x21ec => {
                let [index, destination] = args!(2);
                if index == 15 {
                    vm.write_cvns_text(opcode, destination, &self.font_name)?;
                }
                let Some(value) = self.setting(index) else { done!(2); };
                vm.complete_owner_call(2, Some(value))?;
                return Ok(Some(Step::Done));
            }
            _ => return Ok(None),
        }
    }

    fn button(&mut self, window: u32, button: u32) -> Result<&mut adv::Button> {
        self.adv(window)?
            .buttons
            .get_mut(button as usize)
            .with_context(|| format!("CVNS button {button} outside 0..11"))
    }

    /// `0x4206f0` after the text was placed.
    fn print_done(&mut self, window: u32, read: bool) -> Result<bool> {
        if self.adv(window)?.user_hidden {
            return Ok(false);
        }
        let message = self.adv_message(window)?;
        if message.revealing() == 0 {
            return Ok(true);
        }
        let skipping = self.settings[5] != 0 || self.input.words[33] > 0 || self.input.words[12] > 0;
        if read && skipping {
            self.adv_message(window)?.complete();
            return Ok(true);
        }
        if self.input.words[18] != 0 || self.input.words[29] == 1 || self.input.words[8] == 1 {
            self.input.words[18] = 0;
            self.input.words[29] = 2;
            self.input.words[8] = 2;
            self.adv_message(window)?.complete();
            return Ok(true);
        }
        Ok(false)
    }

    /// `0x420ee0`: the click, skip or auto-advance wait.
    fn wait_done(&mut self, window: u32, skip_ok: bool, voice: i32, read: bool, host: &dyn HostServices, client: ClientId) -> Result<bool> {
        let point = self.input.pointer();
        let adv = self.adv(window)?;
        if adv.user_hidden {
            return Ok(false);
        }
        let on_button = adv.hit(point).is_some();
        let skipping = self.settings[5] != 0 || self.input.words[33] > 0 || self.input.words[12] > 0;
        if skipping && skip_ok {
            self.stop_icon(window)?;
            self.auto_clock = 0;
            return Ok(true);
        }
        if !on_button && (self.input.words[18] != 0 || self.input.words[8] == 1 || self.input.words[29] == 1) {
            self.input.words[18] = 0;
            self.input.words[8] = 2;
            self.input.words[29] = 2;
            self.auto_clock = 0;
            self.stop_icon(window)?;
            return Ok(true);
        }
        if self.settings[17] != 0 {
            self.auto_clock = self.auto_clock.wrapping_add(self.delta);
            let wait = if voice >= 0 && self.settings[21] != 0 {
                let channel = 5 + voice as u32 % 4;
                if host.audio_info(client, channel)?.is_some_and(|info| {
                    info.source_playing && info.transport_active && !info.paused
                }) {
                    self.auto_clock = 0;
                }
                250
            } else {
                self.auto_text_delay(window)?
            };
            if wait <= self.auto_clock {
                self.auto_clock = 0;
                self.stop_icon(window)?;
                return Ok(true);
            }
        }
        let adv = self.adv(window)?;
        let [x, y, ..] = adv.rect;
        let offset = adv.icon_offset;
        if let Some(icon) = self.sprites.get_mut(&SpriteId::Part(window as u8, 1)) {
            if !(icon.enabled && (1..=2).contains(&icon.animation())) {
                icon.enabled = true;
                icon.position = [x + i32::from(offset[0]), y + i32::from(offset[1])];
                // Unread text uses slot 1 and read text slot 2.
                icon.start(if read { 2 } else { 1 })?;
            }
        }
        Ok(false)
    }

    fn auto_text_delay(&mut self, window: u32) -> Result<u32> {
        let count = self.adv_message(window)?.glyph_count() as u32;
        let [base, per_glyph, acceleration] = [self.settings[18], self.settings[19], self.settings[20]];
        let full = 5u32.wrapping_mul(count.wrapping_mul(per_glyph).wrapping_add(20 * base));
        let mut wait = full;
        if acceleration != 0 && per_glyph != 0 {
            // The x87 loop adds a step per glyph that grows by a quarter of AUTO_MODE_3.
            let (mut sum, mut step) = (0.0f64, 0.0f64);
            for _ in 0..count {
                sum += step;
                if step < f64::from(5 * per_glyph) {
                    step += f64::from(acceleration) * 0.25;
                }
            }
            wait = full.wrapping_sub(sum as u64 as u32);
            if wait > 0xffff {
                wait = full;
            }
            wait = wait.max(100 * base);
        }
        Ok(wait)
    }

    fn destroy_adv(&mut self, host: &mut dyn HostServices, client: ClientId, window: usize) -> Result<()> {
        if let Some(adv) = self.advs[window].take() {
            self.messages[adv.message] = None;
            for image in adv.bitmaps.into_iter().flatten() {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        self.sprites.retain(|id, _| !matches!(id, SpriteId::Part(owner, _) if usize::from(*owner) == window));
        Ok(())
    }

    pub fn finish_screen_copy(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<bool> {
        if self.copy_next_frame {
            let Some(image) = self.capture_frame(host, client)? else { return Ok(false); };
            self.copied_screen = Some(image);
            self.copy_next_frame = false;
        }
        Ok(true)
    }

    fn capture_frame(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<Option<DecodedImage>> {
        use crysirror_core::image_processing::ImageProcessState;
        if self.capture.is_none() { self.capture = Some(self.primary.capture(host, client)?); }
        match self.capture.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(reason) => bail!("CVNS screen capture: {reason}"),
            ImageProcessState::Ready(frame) => {
                self.capture = None;
                Ok(Some(DecodedImage::from_frame(frame)))
            }
        }
    }

    pub fn commit_rule(&mut self, host: &mut dyn HostServices, client: ClientId, vm: &mut CmvsVm,
        mask: Option<rule::Active>, captured: Option<bitmap::Prepared>) -> Result<()> {
        self.rule = mask;
        let words = if let Some(captured) = captured {
            if self.rule.is_some() {
                let [width, height] = captured.picture.size(0).context("empty CVNS capture")?;
                self.install_bitmap(host, client, captured)?;
                let mut sprite = Sprite::new(Kind::Bitmap);
                sprite.set_source(0, [0, 0, width as i16, height as i16]);
                sprite.layer = 0;
                sprite.rule_blit = true;
                self.sprites.insert(SpriteId::Global(0), sprite);
            } else {
                self.release(host, client, BitmapSlot::Global(0))?;
                self.sprites.remove(&SpriteId::Global(0));
            }
            3
        } else { 5 };
        vm.complete_owner_call(words, Some(u32::from(self.rule.is_some())))?;
        Ok(())
    }

    fn finish_rule(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        if let Some(rule) = self.rule.take() {
            if rule.reveal {
                if let Some(sprite) = self.sprites.get_mut(&rule.target) {
                    sprite.rule_blit = false;
                }
            } else {
                if let Some(sprite) = self.sprites.remove(&rule.target) {
                    if let Ok(index) = usize::try_from(sprite.source) {
                        self.release(host, client, BitmapSlot::Global(index))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn commit_bitmap(&mut self, host: &mut dyn HostServices, client: ClientId, vm: &mut CmvsVm, prepared: bitmap::Prepared) -> Result<()> {
        let (words, result) = (prepared.words, prepared.result);
        self.install_bitmap(host, client, prepared)?;
        vm.complete_owner_call(words, result)?;
        Ok(())
    }

    pub(super) fn install_bitmap(&mut self, host: &mut dyn HostServices, client: ClientId, prepared: bitmap::Prepared) -> Result<()> {
        let bitmap::Prepared {slot, picture, refresh, image, ..} = prepared;
        if let Some(image) = image {
            let resource = match slot {
                BitmapSlot::Global(index) => BITMAP_RESOURCE + index as u32,
                BitmapSlot::Adv(window, index) => ADV_RESOURCE + (window * adv::BITMAPS + index) as u32,
            };
            let uploaded = image.publish(host, client, resource)?;
            let old = match slot {
                BitmapSlot::Global(index) => self.bitmaps[index].replace(uploaded),
                BitmapSlot::Adv(window, index) => self.adv(window as u32)?.bitmaps[index].replace(uploaded),
            };
            if let Some(old) = old { host.release_image_resource(client, old.path.relative())?; }
        } else if refresh {
            self.release(host, client, slot)?;
        }
        match slot {
            BitmapSlot::Global(index) => self.pictures[index] = Some(picture),
            BitmapSlot::Adv(window, index) => self.adv(window as u32)?.bitmap_components[index] = Some(picture.components),
        }
        Ok(())
    }

    pub(super) fn checkpoint_records(&self, out: &mut Vec<u8>) -> Result<()> {
        for (tag,field) in [(0x200,self.color),(0x201,self.level),(0x202,u32::from(self.background)),(0x203,u32::from(self.exclude_vm_time))] {
            crate::vm::save_slot::csp1::append(out,tag,&[],&field.to_le_bytes())?;
        }
        for (index,message) in self.messages.iter().enumerate() {
            if let Some(message)=message { message.record(index,out)?; }
        }
        for (&id,sprite) in &self.sprites { sprite.record(id,out)?; }
        self.movies.records(out)?;
        if let Some(shake)=&self.shake { shake.record(out)?; }
        if let Some(fade)=&self.transition {
            let args=[fade.reveal,fade.color,fade.duration];
            let bytes:Vec<_>=args.into_iter().chain(args).chain([fade.elapsed]).flat_map(u32::to_le_bytes).collect();
            crate::vm::save_slot::csp1::append(out,0x610,&[],&bytes)?;
        }
        if let Some(rule)=&self.rule { rule.record(out)?; }
        for (index, picture) in self.pictures.iter().enumerate() {
            if let Some(picture) = picture {
                picture.components.record(0x480, &[index as u16], out)?;
            }
        }
        for (window, adv) in self.advs.iter().enumerate() {
            if let Some(adv) = adv {
                adv.record(window,out)?;
                for (index, components) in adv.bitmap_components.iter().enumerate() {
                    if let Some(components) = components {
                        components.record(0x481, &[window as u16, index as u16], out)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn picture(&self, index: usize) -> Result<&bitmap::Picture> {
        self.pictures[index].as_ref().context("CVNS bitmap object is absent")
    }

    fn release(&mut self, host: &mut dyn HostServices, client: ClientId, slot: BitmapSlot) -> Result<()> {
        let image = match slot {
            BitmapSlot::Global(index) => { self.pictures[index] = None; self.bitmaps[index].take() },
            BitmapSlot::Adv(window, index) => {
                let adv = self.adv(window as u32)?;
                adv.bitmap_components[index] = None;
                adv.bitmaps[index].take()
            }
        };
        if let Some(image) = image {
            host.release_image_resource(client, image.path.relative())?;
        }
        Ok(())
    }

    /// 0x431070 releases drawable records and active movies, keeps the mask
    /// font table, input bindings and user settings, then damages the whole canvas.
    pub fn reset_scene(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        self.sprites.clear();
        self.messages.fill_with(|| None);
        for index in 0..movie::COUNT { self.movies.remove(host, client, index)?; }
        for index in 0..self.bitmaps.len() { self.release(host, client, BitmapSlot::Global(index))?; }
        for adv in self.advs.iter_mut().filter_map(Option::take) {
            for image in adv.bitmaps.into_iter().flatten() {
                host.release_image_resource(client, image.path.relative())?;
            }
        }
        self.rule = None;
        self.transition = None;
        self.shake = None;
        self.level = 256;
        self.color = 0;
        self.background = true;
        self.primary.reset_damage();
        self.settings[5] = 0;
        self.settings[17] = 0;
        self.reset_clock(self.fixed_clock, host.now_ms() as u32);
        Ok(())
    }

    /// `0x422af0`: the final level and colour replace the screen state.
    fn finish_transition(&mut self) {
        if let Some(transition) = self.transition.take() {
            self.color = transition.color;
            self.level = if transition.reveal != 0 { 256 } else { 0 };
        }
    }

    /// The per-frame passes of `0x402770` after the VM, then the composed layers.
    pub fn frame(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<()> {
        // 0x439320 advances both real clock samples equally after VM execution.
        // The current delta survives; the following frame starts at this sample.
        if self.exclude_vm_time { self.sampled_at = host.now_ms() as u32; }
        let delta = self.delta;
        if let Some(rule) = &mut self.rule {
            rule.elapsed = rule.elapsed.wrapping_add(delta);
            if rule.remaining() == 0 || !self.sprites.contains_key(&rule.target) {
                self.finish_rule(host, client)?;
            }
        }
        let previous_rule_image = self.rule_image.take();
        if let Some(rule) = &self.rule {
            self.rule_image = Some(rule.sample(client)?.publish(host, client, 0x4356_9201)?);
        }
        if let Some(shake) = &mut self.shake {
            if shake.advance(delta, &mut self.random)? { self.shake = None; }
        }
        // 0x40ff70: 0x422a60 adds the frame time, then reports that level.
        if let Some(transition) = &mut self.transition {
            self.primary.invalidate_all(canvas);
            transition.elapsed = transition.elapsed.saturating_add(delta);
            self.color = transition.color;
            if transition.elapsed >= transition.duration {
                self.finish_transition();
            } else {
                let ramp = (transition.elapsed << 8) / transition.duration;
                self.level = if transition.reveal != 0 { ramp } else { 256 - ramp };
            }
        }
        // 0x40fa20 runs transitions and animations while bucketing, then
        // 0x40fb60 deletes ended records.
        let random = &mut self.random;
        for sprite in self.sprites.values_mut() {
            sprite.advance_effect(delta, &mut || msvc_rand(random))?;
            sprite.advance(delta)?;
        }
        self.sprites.retain(|_, sprite| !sprite.dead);
        for message in self.messages.iter_mut().flatten() {
            message.advance(delta);
        }
        let mut layers = Vec::new();
        let full = [0.0, 0.0, canvas[0] as f32, canvas[1] as f32];
        self.movies.freeze_frames(host, client)?;
        let mut used_curves = BTreeSet::new();
        for (&id, sprite) in &self.sprites {
            let bitmap = |kind: Kind, source: i16| match kind {
                Kind::Movie => self.movies.frozen(source as usize).cloned(),
                Kind::Part(window) => self
                    .advs
                    .get(usize::from(window))?
                    .as_ref()?
                    .bitmaps
                    .get(source as usize)?
                    .clone(),
                _ => self.bitmaps.get(source as usize)?.clone(),
            };
            let offset = self.shake.as_ref().map_or([0; 2], |s| s.offset(matches!(sprite.kind, Kind::Part(_))));
            if let Some(cells) = sprite.effect_layers(id, bitmap, offset)? {
                for cell in cells {
                    layers.push(cell);
                }
            } else if let Some(mut layer) = sprite.layer(id, bitmap, offset)? {
                if sprite.kind != Kind::Fill && sprite.tint.1 != 256 {
                    let key = sprite.tint;
                    let curve = match self.tint_curves.entry(key) {
                        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            // 0x415c40/0x415df0 truncate each product separately.
                            let weight = key.1 as u16;
                            let bias = rgb(key.0).map(|c| (256u16.wrapping_sub(weight).wrapping_mul(u16::from(c)) >> 8) as u8);
                            let lut = crysirror_image::rgb_lut::RgbLookup::low_word_affine([weight; 3], bias);
                            let rgba = (0..256).flat_map(|i| [lut.red[i], lut.green[i], lut.blue[i], 255]).collect();
                            let surface = crysirror_image::ImageSurface { width: 256, height: 1, rgba };
                            entry.insert(DecodedImage::from_surface(client, surface, Some(false))?.publish(host, client, 0x4356_9200)?)
                        }
                    };
                    layer["layer"]["filter"] = json!({"tone_curve":{"asset":curve.path.relative(),"row":0.5}});
                    used_curves.insert(key);
                }
                if sprite.rule_blit && sprite.kind == Kind::Bitmap
                    && sprite.scale == [1000; 2] && sprite.rotation == 0 {
                    // 0x4137c0 only blits a rule for full opacity and full image weight.
                    let mask = self.rule_image.as_ref().context("CVNS sprite requests a missing rule image")?;
                    if sprite.alpha != 256 || sprite.tint.1 != 256 || self.level != 256 { continue; }
                    layer["layer"]["filter"] = json!({"mask":{"asset":mask.path.relative(),"channel":"red","source_space":true}});
                }
                layers.push(layer);
            }
        }
        let mut used_masks = BTreeSet::new();
        for (index, message) in self.messages.iter().enumerate() {
            let Some(message) = message else {
                continue;
            };
            let id = MESSAGE_LAYER + index as u32;
            let offset = self.shake.as_ref().map_or([0; 2], |s| s.offset(true));
            if let Some(layer) = message.layer(id, sprite::order(message.layer, 1064 + index as i32), offset,
                |ink, generation| self.masks.image(host, client, ink, generation, &mut used_masks))? {
                layers.push(layer);
            }
        }
        if self.level < 256 {
            // 0x410200 mixes every default record toward the colour by the level,
            // which equals one colour layer over the composed records.
            let [r, g, b] = rgb(self.color);
            let opacity = ((256 - self.level) * 255 / 256) as u8;
            layers.push(json!({"op":"put","layer":{"id":FADE_LAYER,"rect":full,
                "order":[i32::MAX, 0],"opacity":opacity,"paint":{"kind":"solid","color":[r, g, b, 255]}}}));
        }
        self.primary.paint(host, client, canvas, layers, self.background.then_some(self.color))?;
        for sprite in self.sprites.values_mut() {
            sprite.last_bounds=Some(sprite.damage_bounds());
        }
        self.masks.retire(host, client, &used_masks)?;
        if let Some(previous) = previous_rule_image {
            host.release_image_resource(client, previous.path.relative())?;
        }
        let obsolete: Vec<_> = self.tint_curves.keys().filter(|key| !used_curves.contains(*key)).copied().collect();
        for key in obsolete {
            if let Some(curve) = self.tint_curves.remove(&key) {
                host.release_image_resource(client, curve.path.relative())?;
            }
        }
        Ok(())
    }
}
