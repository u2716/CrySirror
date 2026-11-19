// SPDX-License-Identifier: MPL-2.0
//! Disk script loading and profile-specific structural decoding for CMVS.
#![forbid(unsafe_code)]

#[cfg(feature = "runtime")]
pub mod adapter;
mod script;
pub mod vm;

pub use script::Script;
