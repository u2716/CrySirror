// SPDX-License-Identifier: MPL-2.0
//! Capability-rooted file access with one API on every target.
//!
//! Desktop and mobile targets re-export `cap-std`. Horizon (Switch) and Vita
//! have no `rustix` backend, so this crate provides the same subset over
//! `std::fs`. A `Dir` there is an owned root path. Every relative path is
//! checked component by component: absolute paths, prefixes and `..` are
//! rejected with `PermissionDenied`, so an operation cannot leave its root.
//! Those console filesystems (FAT/exFAT on sdmc:, ux0:) have no symbolic links,
//! so the lexical check is the whole traversal rule. The console backend
//! cannot defend against a root that is renamed during an operation.
#![forbid(unsafe_code)]

#[cfg(not(any(target_os = "horizon", target_os = "vita")))]
pub use cap_std::{AmbientAuthority, ambient_authority, fs, time};

#[cfg(any(target_os = "horizon", target_os = "vita", test))]
mod console;
#[cfg(any(target_os = "horizon", target_os = "vita"))]
pub use console::{AmbientAuthority, ambient_authority, fs, time};
