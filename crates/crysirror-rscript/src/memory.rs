// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Engine memory shared by script code, saves and native screens. The
//! configuration (0x872 bytes, word_4ED2A8) and scene (0x996C bytes,
//! word_4EDB1C) blocks keep their native layout because save slots and
//! nested system calls copy them whole (sizes from WinMain 0x433730 of the
//! Shapeshifter build; the script stack, return stack and depth offsets are
//! confirmed by its dispatcher 0x4358A0 at scene +0x1E/+0x34/+0x5C).
use crate::apini::Apini;
use serde::{Deserialize, Serialize};

/// Script-visible 16-bit variables (Src, 0x4E20 bytes).
pub const VARIABLES: usize = 10_000;
/// Variables 0..6999 belong to a play-through and are saved with slots.
pub const GAME_VARIABLES: usize = 7_000;
/// Calls from level 9 are refused (nest overflow).
pub const MAX_DEPTH: u16 = 9;
/// Expression temporaries; register 0 carries conditions.
pub const REGISTERS: usize = 102;
pub const CONFIG_SIZE: usize = 0x872;
pub const SCENE_SIZE: usize = 0x996C;

/// Configuration offsets (relative to word_4ED2A8).
pub mod config {
    pub const SCREEN_MODE: usize = 0x00;
    pub const MESSAGE_SPEED: usize = 0x02;
    pub const MUSIC_ENABLED: usize = 0x04;
    pub const SOUND_ENABLED: usize = 0x06;
    pub const VOICE_ENABLED: usize = 0x08;
    pub const MUSIC_VOLUME: usize = 0x0A;
    pub const SOUND_VOLUME: usize = 0x0C;
    pub const VOICE_VOLUME: usize = 0x0E;
    pub const EFFECTS_ENABLED: usize = 0x12;
    pub const SKIP_UNREAD: usize = 0x14;
    /// Companion panel switch (`cmp`, word_4ED2BE).
    pub const COMPANION: usize = 0x16;
    pub const BACKGROUND_AUDIO: usize = 0x18;
    pub const WINDOW_ALPHA: usize = 0x1A;
    pub const AUTO_WAIT: usize = 0x1E;
    pub const VOICE_CONTINUES: usize = 0x20;
    pub const AUTO_HIDE: usize = 0x22;
    pub const VOICE_BANKS: usize = 0x24;
    pub const FONT_NAME: usize = 0x83E;
    /// Save screen page and last saved slot (dword_4ED2E0).
    pub const SAVE_PAGE: usize = 0x38;
    pub const SAVE_SLOT: usize = 0x3A;
    /// System-save words (0x8D/0x8E and 0x8C) and strings (0x9E/0x9F).
    pub const SYSTEM_WORDS: usize = 0x3C;
    pub const SYSTEM_WORDS_B: usize = 0x50;
    pub const SYSTEM_STRINGS: usize = 0x64;
}

/// Scene offsets (relative to word_4EDB1C).
pub mod scene {
    pub const MUSIC: usize = 0x0000;
    pub const SOUND_CHANNELS: usize = 0x0004;
    pub const SOUND_STRIDE: usize = 6;
    pub const RANDOM_SEED: usize = 0x001C;
    pub const SCRIPT_STACK: usize = 0x001E;
    pub const RETURN_STACK: usize = 0x0034;
    pub const CALL_DEPTH: usize = 0x005C;
    pub const EFFECT_IMAGE: usize = 0x005E;
    pub const EFFECT_BLEND: usize = 0x0060;
    pub const EFFECT_ORDER: usize = 0x0062;
    pub const OVERLAY_PERCENT: usize = 0x0064;
    pub const OVERLAY_BLEND: usize = 0x0066;
    pub const OVERLAY_ORDER: usize = 0x0068;
    pub const PANEL_SAVE_LOAD: usize = 0x006C;
    pub const MESSAGE_SNAPSHOTS: usize = 0x0070;
    pub const AUTO_REBUILD: usize = 0x0074;
    pub const LAYER_ANIMATION_RESET: usize = 0x0078;
    pub const MESSAGE_MODE: usize = 0x007C;
    pub const LAYERS: usize = 0x0080;
    pub const LAYER_STRIDE: usize = 200;
    pub const LAYER_COUNT: usize = 100;
    pub const LAYER_ORDERS: usize = 0x4EA0;
    pub const LAYER_GROUPS: usize = 0x4F68;
    pub const GAUGES: usize = 0x5030;
    pub const GAUGE_STRIDE: usize = 52;
    pub const GAUGE_COUNT: usize = 10;
    pub const SCALE_X: usize = 0x5238;
    pub const SCALE_Y: usize = 0x523A;
    pub const MESSAGE: usize = 0x523C;
    pub const STRING_REGISTERS: usize = 0x8C60;
    pub const STRING_STRIDE: usize = 201;
    pub const STRING_FLAGS: usize = 0x943A;
    /// Dynamic selection list (0xD2..0xD5): count, script, target, then
    /// 100 entries of (script, target, value, condition) and the cancel
    /// target (selection +0x4BC) — 0x4C4 bytes cleared by 0xD2.
    pub const SELECTION: usize = 0x9464;
    pub const SELECTION_SIZE: usize = 0x4C4;
    /// Play-through words saved with slots (0x73/0x74/0x75).
    pub const WORDS: usize = 0x9942;
    pub const WORDS_B: usize = 0x9956;
    /// Map screen: number, sub-map, then a hidden byte per button id
    /// (CMap record dword_4F7444, sub_417CD0).
    pub const MAP: usize = 0x9928;
}

/// 200-byte layer record (scene +0x80 + 200·index).
pub mod layer {
    pub const X: usize = 0;
    pub const Y: usize = 4;
    pub const KIND: usize = 8;
    pub const SOURCE: usize = 10;
    pub const PATTERN: usize = 12;
    pub const IMAGE: usize = 14;
    pub const COLOR: usize = 16;
    /// Load flags: the tone conversion of kind 0 images (sub_415810).
    pub const LOAD_FLAGS: usize = 20;
    pub const BLEND_MODE: usize = 22;
    pub const BLEND_VALUE: usize = 24;
    /// Kind 0 load words set by 0xC9: mirror bits, zoom percentage,
    /// rotation degrees and the picture drawn over the image.
    pub const MIRROR: usize = 26;
    pub const ZOOM: usize = 28;
    pub const ROTATION: usize = 30;
    pub const OVERLAY_IMAGE: usize = 32;
    pub const VISIBLE: usize = 36;
    pub const CENTER_X: usize = 40;
    pub const CENTER_Y: usize = 44;
    pub const KEEP_ON_SCREEN: usize = 48;
    /// Button state: enabled flag, hover style, mode, keep-hover, id.
    pub const BUTTON: usize = 52;
    pub const HOVER_STYLE: usize = 56;
    pub const BUTTON_MODE: usize = 58;
    pub const OVERLAYS: usize = 60;
    pub const KEEP_HOVER: usize = 66;
    pub const OVERLAY_POSITIONS: usize = 68;
    pub const PRIORITY: usize = 92;
    pub const BUTTON_ID: usize = 94;
    pub const NAME: usize = 96;
    pub const DIRECTORY: usize = 177;
}

/// Message state (scene +0x523C).
pub mod message {
    pub const PANEL_ENABLED: usize = 0;
    pub const BACKLOG_ENABLED: usize = 4;
    pub const MENU_ENABLED: usize = 8;
    pub const PANEL_BUTTONS: usize = 12;
    pub const TAB_ENABLED: usize = 16;
    pub const BOXES: usize = 20;
    pub const BOX_STRIDE: usize = 96;
    pub const SOURCES: usize = 404;
    pub const SOURCE_STRIDE: usize = 12;
    pub const PAGE: usize = 452;
    pub const PANEL_X: usize = 452;
    pub const PANEL_Y: usize = 456;
    pub const FACE_X: usize = 460;
    pub const FACE_Y: usize = 464;
    pub const FACE_ORDER: usize = 470;
    pub const CURRENT_BOX: usize = 472;
    pub const VOICE: usize = 476;
    pub const VOICE_PAN: usize = 480;
    pub const BACKLOG: usize = 484;
    pub const BACKLOG_STRIDE: usize = 144;
    pub const BACKLOG_COUNT: usize = 100;
    /// Backlog entry fields.
    pub const ENTRY_PAGE_START: usize = 0;
    pub const ENTRY_BOX: usize = 4;
    pub const ENTRY_SCRIPT: usize = 100;
    pub const ENTRY_NAME: usize = 104;
    pub const ENTRY_TEXT: usize = 108;
    pub const ENTRY_PAGE: usize = 112;
}

/// 96-byte text box record (message +20 + 96·box).
pub mod text_box {
    pub const X: usize = 0;
    pub const Y: usize = 4;
    pub const TEXT_X: usize = 8;
    pub const TEXT_Y: usize = 12;
    pub const NAME_X: usize = 16;
    pub const NAME_Y: usize = 20;
    pub const WAIT_X: usize = 24;
    pub const WAIT_Y: usize = 28;
    pub const TEXT_WIDTH: usize = 32;
    pub const TEXT_HEIGHT: usize = 36;
    pub const NAME_WIDTH: usize = 40;
    pub const NAME_HEIGHT: usize = 44;
    pub const FRAME: usize = 48;
    pub const WAIT_ICON: usize = 50;
    pub const FACE: usize = 54;
    pub const SIZE: usize = 56;
    pub const ALIGN: usize = 58;
    pub const RUBY_FACE: usize = 60;
    pub const RUBY_SIZE: usize = 62;
    pub const RUBY_RAISE: usize = 64;
    pub const LINE_SPACING: usize = 66;
    pub const CHAR_SPACING: usize = 68;
    pub const SPEED: usize = 70;
    pub const INDENT: usize = 72;
    pub const FIRST_INDENT: usize = 74;
    pub const COLOR: usize = 76;
    pub const GLOW: usize = 80;
    pub const NAME_VISIBLE: usize = 84;
    pub const WAIT_ENABLED: usize = 88;
    pub const VISIBLE: usize = 92;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Memory {
    pub vars: Vec<i16>,
    pub regs: Vec<i16>,
    /// Configuration block plus the two bytes the font name may reach.
    pub config: Vec<u8>,
    pub scene: Vec<u8>,
    /// State at the last message snapshot.
    pub message_scene: Vec<u8>,
    pub message_vars: Vec<i16>,
    /// Previous-choice snapshot.
    pub previous_scene: Vec<u8>,
    pub previous_vars: Vec<i16>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            vars: vec![0; VARIABLES],
            regs: vec![0; REGISTERS],
            config: vec![0; CONFIG_SIZE + 2],
            scene: vec![0; SCENE_SIZE],
            message_scene: vec![0; SCENE_SIZE],
            message_vars: vec![0; GAME_VARIABLES],
            previous_scene: vec![0; SCENE_SIZE],
            previous_vars: vec![0; GAME_VARIABLES],
        }
    }
}

impl Memory {
    pub fn var(&self, i: i32) -> i16 {
        usize::try_from(i).ok().and_then(|i| self.vars.get(i).copied()).unwrap_or(0)
    }
    /// Out-of-range writes are dropped like the native bounds (diagnosed by
    /// the caller).
    pub fn set_var(&mut self, i: i32, v: i16) -> bool {
        match usize::try_from(i).ok().and_then(|i| self.vars.get_mut(i)) {
            Some(slot) => {
                *slot = v;
                true
            }
            None => false,
        }
    }
    pub fn cfg(&self, at: usize) -> i16 {
        i16::from_le_bytes([self.config[at], self.config[at + 1]])
    }
    pub fn set_cfg(&mut self, at: usize, v: i32) {
        self.config[at..at + 2].copy_from_slice(&(v as i16).to_le_bytes());
    }
    pub fn w(&self, at: usize) -> i16 {
        i16::from_le_bytes([self.scene[at], self.scene[at + 1]])
    }
    pub fn uw(&self, at: usize) -> u16 {
        self.w(at) as u16
    }
    pub fn set_w(&mut self, at: usize, v: i32) {
        self.scene[at..at + 2].copy_from_slice(&(v as i16).to_le_bytes());
    }
    pub fn d(&self, at: usize) -> i32 {
        i32::from_le_bytes(self.scene[at..at + 4].try_into().unwrap())
    }
    pub fn set_d(&mut self, at: usize, v: i32) {
        self.scene[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn cfg_str(&self, at: usize, cap: usize) -> &[u8] {
        let f = &self.config[at..at + cap];
        &f[..f.iter().position(|&b| b == 0).unwrap_or(cap)]
    }
    pub fn set_cfg_str(&mut self, at: usize, cap: usize, v: &[u8]) {
        self.config[at..at + cap].fill(0);
        let n = v.len().min(cap - 1);
        self.config[at..at + n].copy_from_slice(&v[..n]);
    }
    pub fn str_at(&self, at: usize, cap: usize) -> &[u8] {
        let f = &self.scene[at..at + cap];
        &f[..f.iter().position(|&b| b == 0).unwrap_or(cap)]
    }
    pub fn set_str(&mut self, at: usize, cap: usize, v: &[u8]) {
        self.scene[at..at + cap].fill(0);
        let n = v.len().min(cap - 1);
        self.scene[at..at + n].copy_from_slice(&v[..n]);
    }
    pub fn capture_message_snapshot(&mut self) {
        self.message_scene.copy_from_slice(&self.scene);
        self.message_vars.copy_from_slice(&self.vars[..GAME_VARIABLES]);
    }
    pub fn promote_message_snapshot(&mut self) {
        self.previous_scene.copy_from_slice(&self.message_scene);
        self.previous_vars.copy_from_slice(&self.message_vars);
    }
    pub fn restore_previous_snapshot(&mut self) {
        self.scene.copy_from_slice(&self.previous_scene);
        self.vars[..GAME_VARIABLES].copy_from_slice(&self.previous_vars);
    }

    /// Defaults used without a system save.
    pub fn init_config(&mut self, apini: &Apini) {
        use config::*;
        self.config.fill(0);
        self.vars[GAME_VARIABLES..].fill(0);
        for (at, v) in [
            (MESSAGE_SPEED, 1),
            (MUSIC_ENABLED, 1),
            (SOUND_ENABLED, 1),
            (VOICE_ENABLED, 1),
            (MUSIC_VOLUME, 255),
            (SOUND_VOLUME, 255),
            (VOICE_VOLUME, 255),
            (EFFECTS_ENABLED, 1),
            (0x1C, 1),
            (WINDOW_ALPHA, 128),
            (AUTO_WAIT, 1500),
        ] {
            self.set_cfg(at, v);
        }
        for bank in 0..10 {
            self.set_cfg(VOICE_BANKS + 2 * bank, 1);
        }
        let n = apini.font_cp932.len().min(51);
        self.config[FONT_NAME..FONT_NAME + n].copy_from_slice(&apini.font_cp932[..n]);
    }

    /// New-game scene state.
    pub fn init_scene(&mut self, apini: &Apini, clear: bool, seed: u16) {
        use scene::*;
        if clear {
            self.scene.fill(0);
            self.vars[..GAME_VARIABLES].fill(0);
        }
        self.set_d(PANEL_SAVE_LOAD, 1);
        self.set_d(MESSAGE_SNAPSHOTS, 1);
        self.set_d(AUTO_REBUILD, 1);
        self.set_d(LAYER_ANIMATION_RESET, 1);
        self.set_w(RANDOM_SEED, i32::from(seed));
        for b in 0..4 {
            let r = MESSAGE + message::BOXES + b * message::BOX_STRIDE;
            use text_box as t;
            self.set_w(r + t::FRAME, 0);
            self.set_w(r + t::WAIT_ICON, 0);
            self.set_d(r + t::GLOW, 0xFFFF80);
            self.set_d(r + t::X, 50);
            self.set_d(r + t::Y, 340);
            self.set_d(r + t::WAIT_X, 200);
            self.set_d(r + t::WAIT_Y, 20);
            self.set_d(r + t::TEXT_X, 30);
            self.set_d(r + t::TEXT_Y, 20);
            self.set_d(r + t::TEXT_WIDTH, 504);
            self.set_d(r + t::TEXT_HEIGHT, 90);
            self.set_w(r + t::INDENT, 0);
            self.set_w(r + t::FIRST_INDENT, 0);
            self.set_w(r + t::SIZE, 24);
            self.set_d(r + t::COLOR, 0xFFFFFF);
            self.set_d(r + t::WAIT_ENABLED, 1);
            self.set_d(r + t::VISIBLE, 0);
            self.set_w(r + t::RUBY_SIZE, 6);
            self.set_w(r + t::RUBY_RAISE, 6);
            self.set_w(r + t::LINE_SPACING, 5);
            self.set_w(r + t::CHAR_SPACING, 0);
        }
        let m = |o: usize| MESSAGE + o;
        self.set_d(m(message::PANEL_X), 397);
        self.set_d(m(message::PANEL_Y), 343);
        self.set_d(m(message::PANEL_ENABLED), 0);
        self.set_d(m(message::MENU_ENABLED), 1);
        self.set_d(m(message::BACKLOG_ENABLED), 1);
        self.set_d(m(message::TAB_ENABLED), 1);
        self.set_w(m(message::FACE_ORDER), 10);
        let sprites = encoding_rs::SHIFT_JIS.encode(&apini.dirs[3]).0.into_owned();
        let events = encoding_rs::SHIFT_JIS.encode(&apini.dirs[5]).0.into_owned();
        for l in 0..LAYER_COUNT {
            let r = LAYERS + l * LAYER_STRIDE;
            self.set_w(r + layer::IMAGE, 0);
            self.set_d(r + layer::VISIBLE, 1);
            self.set_w(LAYER_ORDERS + 2 * l, l as i32);
            self.set_w(LAYER_GROUPS + 2 * l, 1);
            self.set_w(r + layer::BLEND_MODE, 0);
            self.set_d(r + layer::CENTER_X, 1);
            self.set_d(r + layer::CENTER_Y, 1);
            self.set_w(r + layer::PRIORITY, 30);
            self.set_w(r + layer::KIND, i32::from(apini.default_layer_kind));
            self.set_str(r + layer::DIRECTORY, 23, &sprites);
        }
        // Layer 0 is the background: top-left anchored, unblended, events.
        self.set_d(LAYERS + layer::CENTER_X, 0);
        self.set_d(LAYERS + layer::CENTER_Y, 0);
        self.set_w(LAYERS + layer::BLEND_MODE, 99);
        self.set_w(LAYERS + layer::KIND, 0);
        self.set_w(LAYER_GROUPS, 0);
        self.set_str(LAYERS + layer::DIRECTORY, 23, &events);
        self.set_w(SCALE_X, 1);
        self.set_w(SCALE_Y, 1);
        for r in 0..10 {
            self.scene[STRING_REGISTERS + r * STRING_STRIDE] = 0;
            self.scene[STRING_FLAGS + 4 * r] = 4;
        }
    }
}
