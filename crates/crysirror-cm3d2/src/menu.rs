// SPDX-License-Identifier: MPL-2.0
//! `.menu` item scripts (`Menu.ProcScriptBin`): BinaryReader header
//! (`CM3D2_MENU`, version, source path, name, category, description, i32
//! body size) followed by commands, each a byte argument count and that many
//! strings, terminated by a zero count.
use anyhow::{ensure, Result};
use crysirror_asset::interchange::cm3d2::Reader;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuFile {
    pub path: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub commands: Vec<Vec<String>>,
}

/// Upper bound on commands of one menu (the largest 1.00 menu has ~300).
const MAX_COMMANDS: usize = 65536;

impl MenuFile {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let version = r.header("CM3D2_MENU")?;
        ensure!(version >= 1000, "unsupported menu version {version}");
        let path = r.string()?;
        let name = r.string()?;
        let category = r.string()?;
        let description = r.string()?;
        let _size = r.i32()?;
        let mut commands = Vec::new();
        loop {
            let n = r.u8()?;
            if n == 0 {
                break;
            }
            let mut args = Vec::with_capacity(n as usize);
            for _ in 0..n {
                args.push(r.string()?);
            }
            commands.push(args);
            ensure!(commands.len() <= MAX_COMMANDS, "menu command limit");
        }
        Ok(Self { path, name, category, description, commands })
    }

    /// Value of the first `key` command (`icon`, `name`, ...).
    pub fn first(&self, key: &str) -> Option<&str> {
        self.commands.iter().find(|c| c.first().is_some_and(|k| k == key)).and_then(|c| c.get(1)).map(String::as_str)
    }
}
