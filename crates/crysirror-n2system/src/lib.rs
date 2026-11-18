// SPDX-License-Identifier: MPL-2.0
//! N2System NsScript semantics over shared CrySirror services (adopt).
//! Reference: CommitteeOfZero/nitrosharp e3869842bb5bdd5ed95344027297620cd54454a6.
//! NSS is the native source language. Any lowered instructions here are an
//! internal execution representation, not a claim about native binary opcodes.
#![forbid(unsafe_code)]
mod adapter;
pub mod archive;
mod arguments;
mod array;
pub mod catalog;
mod entity;
mod native_save;
pub mod program;
mod resource;
pub mod source;
mod text;
pub mod value;
pub mod vm;
#[cfg(feature = "runtime")]
pub use adapter::register;
pub use adapter::{Config, N2SystemVm};
