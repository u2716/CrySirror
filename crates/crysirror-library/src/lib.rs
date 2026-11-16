// SPDX-License-Identifier: MPL-2.0
//! The CrySirror game library shared by every host (desktop, web, PS Vita,
//! Switch): cover art discovery ([`cover`]) and the launcher project
//! ([`library`]), an authored CrySirror project run by the shared runtime
//! and renderer. No window, GPU or platform API is used here.
pub mod cover;
pub mod library;
