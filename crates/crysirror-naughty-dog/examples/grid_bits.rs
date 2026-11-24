// SPDX-License-Identifier: MPL-2.0
//! Histogram of the position grid bits of each package's meshes.
use anyhow::Result;
use crysirror_core::service::MemoryAssetReader;
use crysirror_naughty_dog::{mesh::Loaded, package::Directory, scene};
use std::collections::BTreeMap;

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let source = MemoryAssetReader(std::fs::read(&path)?.into());
        let directory = Directory::read(&source)?;
        let loaded = Loaded::read(&source, 256 << 20)?;
        let mut h: BTreeMap<u32, usize> = BTreeMap::new();
        for m in scene::meshes_of(&loaded, &directory)? {
            *h.entry(m.grid_bits).or_default() += 1;
        }
        println!("{}: {h:?}", path.rsplit('/').next().unwrap_or(&path));
    }
    Ok(())
}
