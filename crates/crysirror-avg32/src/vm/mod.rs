// SPDX-License-Identifier: MPL-2.0
//! The original AVG32 interpreter and its native policies, retained from
//! siglus_rs `crates/avg32/src` at commit
//! 256c398e06fbcbae8e956eb046c831b3d6c8a593 (MPL-2.0; first vendored at
//! 5f358ef, whose native-engine build 256c398 keeps unchanged apart from the
//! `input`/`rng` split adopted here). See `SOURCE.json`.
//!
//! Raster buffers and transitions are the shared `crysirror_image::avg32`
//! device; AVI/Cinepak is `crysirror_video::avi`; files, saves, clock, text
//! glyphs and audio are engine services reached through the adapter (`fs`,
//! `sound::AudioCommand`, `text`).
#![allow(dead_code, clippy::all)]

pub mod animation;
pub mod archive;
pub mod ard;
pub mod cgmode;
pub mod cursor;
pub mod engine;
pub mod flags;
pub mod font;
pub mod fs;
pub mod game;
pub mod ini;
pub mod input;
pub mod menu;
pub mod meswin;
pub mod movie;
pub mod nls;
pub mod resource;
pub mod rng;
pub mod savedata;
pub mod scenario;
pub mod scene;
pub mod sound;
pub mod text;
pub mod system;
pub mod voicepatch;
