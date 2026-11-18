// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! codeX RScript (Liar-soft) native GSC script policy over CrySirror
//! services. Shared crates own XFL/LWG archives, WCG decoding, composition,
//! transitions, text and audio.
#![forbid(unsafe_code)]
mod adapter;
mod anim;
pub mod apini;
mod config;
mod extras;
mod gallery;
pub mod gsc;
pub mod layouts;
pub mod memory;
mod owner;
mod panel;
pub mod plugins;
mod present;
pub mod res;
pub mod save;
mod scene;
mod screens;
mod states;
pub mod status;
pub mod sprite;
pub mod text;
mod vm;
mod zoom;
pub use adapter::{Config, RScriptVm};
#[cfg(feature = "runtime")]
pub use adapter::register;
