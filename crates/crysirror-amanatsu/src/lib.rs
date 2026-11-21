// SPDX-License-Identifier: MPL-2.0
//! 甘夏ろけーしょん (ILLGAMES `AmanatsuLocation`, Unity 6000.0.65f1, IL2CPP)
//! native semantics over CrySirror's shared Unity data readers. Adopt mode:
//! game logic is ported from the original program; assets are read from
//! the user's installation.
#![forbid(unsafe_code)]
pub mod actionmap;
pub mod adapter;
pub mod adv;
pub mod level;
pub mod map;
pub mod scenes;
pub mod sound;
pub mod tables;
pub mod ui;
pub mod unity;

pub use adapter::{AmanatsuVm, Config, ENGINE_ID};
#[cfg(feature = "runtime")]
pub use adapter::register;
