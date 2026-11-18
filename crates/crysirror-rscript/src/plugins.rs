// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native plug-in interfaces (ADR 0001), never an OS DLL loader. A game's
//! DLL is answered by a built-in facade registered under its canonical
//! name, implemented in CrySirror over shared capabilities.
//!
//! The verified build calls DLLs in two places:
//! - opcode 0x19 (sub_43D500) stores a DLL name, sets variable 0 and ends
//!   the thread; the plug-in state (sub_42D680) loads the DLL and passes its
//!   `GetInfo` export a callback table (CAppFunc) and the screens it may
//!   open. A DLL that does not load returns to the game state.
//! - the APINI extension DLL (+339, `Init`/`Term`, panel `extd`
//!   sub_42EC20, F1 sub_432630). A DLL that does not load leaves the
//!   button without effect.
//!
//! A DLL the game does not ship takes those native fallbacks. A shipped
//! DLL without a facade fails visibly with its name.
use crate::vm::Vm;
use anyhow::{Result, bail};
use crysirror_core::{path::Mount, service::HostServices};

/// What a facade answers.
pub type SceneHook = fn(&mut Vm, &mut dyn HostServices, value: i16) -> Result<()>;
pub type ExtensionHook = fn(&mut Vm, &mut dyn HostServices) -> Result<()>;

pub struct Facade {
    /// Canonical file name (lower case, `.dll`).
    pub name: &'static str,
    /// Shared capability the facade maps onto.
    pub capability: &'static str,
    /// Opcode 0x19 plug-in scene.
    pub scene: Option<SceneHook>,
    /// APINI extension (`extd`, F1).
    pub extension: Option<ExtensionHook>,
}

/// Built-in facades. A game that needs a plug-in adds its facade here.
pub const FACADES: &[Facade] = &[];

pub fn canonical(name: &str) -> String {
    let base = name.replace('\\', "/").rsplit('/').next().unwrap_or_default().to_ascii_lowercase();
    if base.contains('.') { base } else { format!("{base}.dll") }
}

pub fn facade(name: &str) -> Option<&'static Facade> {
    let c = canonical(name);
    FACADES.iter().find(|f| f.name == c)
}

pub enum Resolved {
    Facade(&'static Facade),
    /// The game does not ship the DLL: the native load fails.
    Absent,
}

impl Vm {
    /// Finds the facade of a DLL; a shipped DLL without one is an error.
    pub(crate) fn resolve_plugin(&mut self, h: &mut dyn HostServices, name: &str, what: &str) -> Result<Resolved> {
        if let Some(f) = facade(name) {
            return Ok(Resolved::Facade(f));
        }
        let path = name.replace('\\', "/");
        let path = path.trim_start_matches("./");
        let shipped = h.storage(self.client)?.metadata(Mount::Asset, path).ok().flatten().is_some_and(|m| !m.is_directory);
        if shipped {
            bail!("RScript {what} needs the plug-in {name:?}, which has no CrySirror implementation; game DLLs are never loaded");
        }
        Ok(Resolved::Absent)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_are_canonical() {
        assert_eq!(super::canonical(".\\Plug\\FlowDll.DLL"), "flowdll.dll");
        assert_eq!(super::canonical("Mini"), "mini.dll");
    }
}
