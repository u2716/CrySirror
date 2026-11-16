// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! N-System (FJSYS/MSD/MGD) native script policy. Shared engine services own
//! archives, MGD decoding, composition, transitions, text and audio.
#![forbid(unsafe_code)]
mod adapter;
pub mod message;
pub mod save;
pub mod script;
pub mod system;
pub use adapter::{Config, NSystemVm, opcodes};
#[cfg(feature = "runtime")]
pub use adapter::register;
