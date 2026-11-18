// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Per-title configuration compiled into a codeX RScript executable (the
//! `APINI` block; `RsInit.cfg` beside the executable replaces it, WinMain
//! 0x433730 in the Shapeshifter trial build).
//!
//! A build is selected by FileVersion plus a block layout verified against
//! its executable, never by the title text. Unverified layouts are rejected.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

/// Field offsets inside one build's APINI block.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub id: &'static str,
    pub version: &'static str,
    pub size: usize,
    pub title_len: usize,
    pub save_prefix: usize,
    pub save_dir: usize,
    /// Scripts, system, system alternate, sprites, faces, events, movies,
    /// sounds, music, voices (21 bytes each).
    pub dirs: usize,
    pub font: usize,
    pub width: usize,
    pub height: usize,
    pub tick: usize,
    pub skip_top_menu: usize,
    pub start_script: usize,
    pub default_layer_kind: usize,
    pub keep_backdrop: usize,
    pub save_pages: usize,
    pub palette: usize,
    pub question_color: usize,
    pub answer_color: usize,
    pub save_text_color: usize,
    pub question_size: usize,
    pub answer_size: usize,
    pub date_size: usize,
    pub save_text_size: usize,
    pub text_shadow: usize,
    pub backlog_color: usize,
    pub thumb_by_image: usize,
    pub thumb_image_limit: usize,
    pub test_music: usize,
    pub test_sound: usize,
    pub test_voice: usize,
    pub test_bank_voices: usize,
}

/// RScript 1.11.0.3 with the 0x240-byte block (Shapeshifter trial build,
/// SHA-256 f9e32994…f191d). Offsets verified from its WinMain/reader xrefs:
/// width +428, height +432, tick +436 (sub_452680), skip +438 and start
/// +440 (sub_432BC0), layer kind +450, backdrop +452, pages +454, palette
/// +456, choice colours +496/+500, sizes +508/+510, date +512, backlog
/// colour flag/value +518/+520; save screen page text colour +504 and size
/// +514 (sub_40C040), thumbnails by backdrop image flag +524 and limit +526
/// (sub_4304C0, sub_40CB60). The two extra name slots after the voice
/// directory (+318 `RsComp.dll`, +339 `FlowDll.dll`): the second is the extension
/// DLL answered by `crate::plugins`.
pub const WIDE_1_11: Layout = Layout {
    id: "rscript-1.11.0.3-wide",
    version: "1.11.0.3",
    size: 0x240,
    title_len: 51,
    save_prefix: 57,
    save_dir: 87,
    dirs: 108,
    font: 360,
    width: 428,
    height: 432,
    tick: 436,
    skip_top_menu: 438,
    start_script: 440,
    default_layer_kind: 450,
    keep_backdrop: 452,
    save_pages: 454,
    palette: 456,
    question_color: 496,
    answer_color: 500,
    save_text_color: 504,
    question_size: 508,
    answer_size: 510,
    date_size: 512,
    save_text_size: 514,
    text_shadow: 516,
    backlog_color: 518,
    thumb_by_image: 524,
    thumb_image_limit: 526,
    test_music: 528,
    test_sound: 530,
    test_voice: 532,
    test_bank_voices: 536,
};

pub const LAYOUTS: &[Layout] = &[WIDE_1_11];

/// Parsed APINI values the scene uses. Paths keep their native spelling.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Apini {
    pub layout: String,
    pub title: String,
    pub save_prefix: String,
    pub save_dir: String,
    /// scripts, system, system_alt, sprites, faces, events, movies, sounds,
    /// music, voices.
    pub dirs: [String; 10],
    /// Shift-JIS bytes of the default font, sprite and event directories.
    pub font_cp932: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub tick_ms: u16,
    pub skip_top_menu: bool,
    pub start_script: u16,
    pub default_layer_kind: u16,
    pub keep_backdrop: bool,
    pub save_pages: u16,
    /// `^C0`..`^C9` as 0xRRGGBB.
    pub palette: [u32; 10],
    pub question_color: u32,
    pub answer_color: u32,
    /// Save screen page text colour and size.
    pub save_text_color: u32,
    pub save_text_size: u16,
    pub question_size: u16,
    pub answer_size: u16,
    pub date_size: u16,
    pub text_shadow: bool,
    pub backlog_color: Option<u32>,
    /// Slot thumbnails come from `thmb\\thmbNNNN` by the backdrop image
    /// number when it is non-zero and below the limit.
    pub thumb_by_image: bool,
    pub thumb_image_limit: u16,
    /// Configuration test buttons: music track, sound, voice and the voice
    /// of each voice bank (sub_419DE0, sub_41B000, sub_41BD40, sub_41AD80).
    pub test_music: u16,
    pub test_sound: u16,
    pub test_voice: u32,
    pub test_bank_voices: [u32; 10],
    /// Extension DLL name (+339; empty when none).
    pub extension_dll: String,
    /// Configuration status-page text from the executable (verified build).
    pub status_text: Option<crate::status::StatusText>,
    /// CG and scene view tables from the executable (verified build).
    pub gallery: Option<crate::status::GalleryTables>,
}

fn field(b: &[u8], at: usize, len: usize) -> &[u8] {
    let f = &b[at..at + len];
    &f[..f.iter().position(|&c| c == 0).unwrap_or(len)]
}
fn sjis(b: &[u8], at: usize, len: usize) -> String {
    encoding_rs::SHIFT_JIS.decode(field(b, at, len)).0.into_owned()
}
fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

impl Apini {
    pub fn parse(block: &[u8], layout: &Layout) -> Result<Self> {
        ensure!(block.len() >= layout.size, "truncated RScript APINI block");
        let b = &block[..layout.size];
        ensure!(field(b, 0, 6) == b"APINI", "not an RScript APINI block");
        let (width, height) = (u32_at(b, layout.width), u32_at(b, layout.height));
        ensure!((1..=8192).contains(&width) && (1..=8192).contains(&height), "invalid RScript screen size {width}x{height}");
        let dirs = std::array::from_fn(|i| sjis(b, layout.dirs + 21 * i, 21));
        // The sprite and event directories must be plain relative paths.
        for d in &dirs {
            ensure!(!d.contains(".."), "RScript APINI directory {d} leaves the game");
        }
        Ok(Self {
            layout: layout.id.into(),
            title: sjis(b, 6, layout.title_len),
            save_prefix: sjis(b, layout.save_prefix, 30),
            save_dir: sjis(b, layout.save_dir, 21),
            dirs,
            font_cp932: field(b, layout.font, 32).iter().take(31).copied().collect(),
            width,
            height,
            tick_ms: u16_at(b, layout.tick).max(1),
            skip_top_menu: u16_at(b, layout.skip_top_menu) != 0,
            start_script: u16_at(b, layout.start_script),
            default_layer_kind: u16_at(b, layout.default_layer_kind),
            keep_backdrop: u16_at(b, layout.keep_backdrop) != 0,
            save_pages: u16_at(b, layout.save_pages),
            palette: std::array::from_fn(|i| u32_at(b, layout.palette + 4 * i)),
            question_color: u32_at(b, layout.question_color),
            answer_color: u32_at(b, layout.answer_color),
            save_text_color: u32_at(b, layout.save_text_color),
            save_text_size: u16_at(b, layout.save_text_size),
            question_size: u16_at(b, layout.question_size),
            answer_size: u16_at(b, layout.answer_size),
            date_size: u16_at(b, layout.date_size),
            text_shadow: u16_at(b, layout.text_shadow) != 0,
            backlog_color: (u16_at(b, layout.backlog_color) != 0).then(|| u32_at(b, layout.backlog_color + 2)),
            thumb_by_image: u16_at(b, layout.thumb_by_image) != 0,
            thumb_image_limit: u16_at(b, layout.thumb_image_limit),
            test_music: u16_at(b, layout.test_music),
            test_sound: u16_at(b, layout.test_sound),
            test_voice: u32_at(b, layout.test_voice),
            test_bank_voices: std::array::from_fn(|i| u32_at(b, layout.test_bank_voices + 4 * i)),
            status_text: None,
            gallery: None,
            extension_dll: encoding_rs::SHIFT_JIS.decode(&field(b, 339, 21).split(|&c| c == 0).next().unwrap_or_default().to_vec()).0.into_owned(),
        })
    }
    pub fn dir(&self, which: Dir) -> &str {
        &self.dirs[which as usize]
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Dir {
    Scripts = 0,
    System = 1,
    SystemAlt = 2,
    Sprites = 3,
    Faces = 4,
    Events = 5,
    Movies = 6,
    Sounds = 7,
    Music = 8,
    Voices = 9,
}

/// The executable's VS_FIXEDFILEINFO file version, dotted (`1.11.0.3`).
pub fn file_version(exe: &[u8]) -> Option<String> {
    let sig = [0xBD, 0x04, 0xEF, 0xFE];
    let at = exe.windows(4).position(|w| w == sig)?;
    let ms = u32::from_le_bytes(exe.get(at + 8..at + 12)?.try_into().ok()?);
    let ls = u32::from_le_bytes(exe.get(at + 12..at + 16)?.try_into().ok()?);
    Some(format!("{}.{}.{}.{}", ms >> 16, ms & 0xFFFF, ls >> 16, ls & 0xFFFF))
}

/// Finds the executable's APINI block and the verified layout it matches.
/// An `RsInit.cfg` block, when given, replaces the embedded one natively.
pub fn from_executable(exe: &[u8], rsinit: Option<&[u8]>) -> Result<Apini> {
    let version = file_version(exe).context("RScript executable has no version resource")?;
    let candidates: Vec<&Layout> = LAYOUTS.iter().filter(|l| l.version == version).collect();
    if candidates.is_empty() {
        bail!("unsupported codeX RScript revision {version}; its native layouts are not verified");
    }
    let tag = b"APINI\0";
    let mut errors = Vec::new();
    let mut at = 0;
    while let Some(found) = exe[at..].windows(tag.len()).position(|w| w == tag) {
        let start = at + found;
        for layout in &candidates {
            let block = rsinit.unwrap_or(&exe[start..]);
            match Apini::parse(block, layout) {
                Ok(mut a) => {
                    a.status_text = crate::status::StatusText::from_exe(exe);
                    a.gallery = crate::status::GalleryTables::from_exe(exe);
                    return Ok(a);
                }
                Err(e) => errors.push(format!("{}: {e}", layout.id)),
            }
        }
        at = start + 1;
    }
    bail!("RScript {version}: no verified APINI layout matches ({})", errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn block(layout: &Layout, w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0u8; 0x260];
        b[..6].copy_from_slice(b"APINI\0");
        b[layout.width..layout.width + 4].copy_from_slice(&w.to_le_bytes());
        b[layout.height..layout.height + 4].copy_from_slice(&h.to_le_bytes());
        b[layout.tick..layout.tick + 2].copy_from_slice(&42u16.to_le_bytes());
        b
    }
    #[test]
    fn wide_block_fields_and_screen_size_check() {
        let wide = block(&WIDE_1_11, 1920, 1080);
        let a = Apini::parse(&wide, &WIDE_1_11).unwrap();
        assert_eq!((a.width, a.height, a.tick_ms), (1920, 1080, 42));
        assert!(Apini::parse(&block(&WIDE_1_11, 0, 1080), &WIDE_1_11).is_err());
    }
}
