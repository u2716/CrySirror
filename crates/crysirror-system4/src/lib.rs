// Copyright (c) 2026 CrySirror contributors.
// AIN format research: libsys4 (C) 2019 Nunuhara Cabbage, GPL-2.0-or-later.
//! Original AIN program and bounded native VM. No embedded System4 engine or compositor.
//! Shared engine operation mappings remain a separate thin adapter.
#![forbid(unsafe_code)]
pub mod ain;

pub mod instruction;
pub mod opcodes;

pub mod adapter;
pub mod vm;
pub mod worker_adapter;

pub mod sact;

pub mod assets;
pub mod bootstrap;
#[cfg(feature = "runtime")]
pub mod factory;
pub mod sprites;

pub mod input;

mod buffered_input;
mod math;
mod math_random;
mod native_async;
mod native_cg;
mod native_csv;
mod native_confirm;
mod native_dialog;
mod native_draw;
mod native_effect;
mod native_file;
mod native_files;
mod native_input_string;
mod native_logo;
mod native_logs;
mod native_movie;
mod native_msgskip;
mod native_sact_intrinsics;
mod native_sact_text;
mod native_save;
mod native_save_cg;
mod native_scene;
mod native_scene_present;
mod native_system;
mod native_wallpaper_get;

// The whole SystemService mixer family is routed. SACT stream lifetime
// operations remain unregistered until their complete source policy is ready.
mod native_audio;

mod native_pixel;

mod native_audio_profile;
mod native_draw_profile;
