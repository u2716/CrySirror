// SPDX-License-Identifier: MPL-2.0
//! Complete Siglus runtime with CrySirror-owned host and output devices.
mod native_assets;
mod native_xfile;
mod native_vm;
mod host_scope;
mod adapter;
pub use adapter::register;
use native_assets::{keys,key_recovery};
pub mod decode_material;
pub mod gameexe { pub use crate::native_assets::gameexe::*; }
pub mod key_toml { pub use crate::native_assets::key_toml::*; }
