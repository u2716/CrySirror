// SPDX-License-Identifier: MPL-2.0
//! SugarPot's Tsukumo-2012 native script policy. Shared engine services own
//! decompression, images, glyphs, scene composition, input widgets and audio.
#![forbid(unsafe_code)]
mod adapter;
pub mod expression;
pub mod machine;
mod native_storage;
pub mod program;
mod resources;
mod text;
mod tone;
mod values;
#[cfg(feature = "runtime")]
pub use adapter::register;
pub use adapter::{Config, SugarPotVm};
