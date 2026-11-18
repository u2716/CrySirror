// SPDX-License-Identifier: MPL-2.0
//! Native Malie EXEC semantics over scoped shared CrySirror capabilities.
#![forbid(unsafe_code)]
pub mod material;
pub mod compiler;
pub mod program;
pub mod vm;

pub mod adapter;
pub mod adv;
pub mod message;
pub mod picture;
pub mod screen;
pub mod settings;
#[cfg(feature = "runtime")]
pub use adapter::register;
