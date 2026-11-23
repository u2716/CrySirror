// SPDX-License-Identifier: MPL-2.0
//! Read-only census: decode the background prototype meshes of every
//! package in the installed `pak22.psarc` and report failures by message.
use anyhow::{Context, Result};
use crysirror_capfs::{ambient_authority, fs::Dir};
use crysirror_core::{service::AssetReader, state::ClientId, vfs::VfsProvider};
use crysirror_naughty_dog::{mesh, package::Directory};
use crysirror_resource::{ArchiveMount, DecodeContext, FormatRegistry};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

struct File(Mutex<crysirror_capfs::fs::File>, u64);
impl AssetReader for File {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.1)
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let mut f = self.0.lock().map_err(|_| crysirror_core::Error::Io("file lock poisoned".into()))?;
        f.seek(SeekFrom::Start(offset)).and_then(|_| f.read(out)).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

fn main() -> Result<()> {
    let root = std::env::args_os().nth(1).context("usage: background_census GAME_ROOT")?;
    let dir = Dir::open_ambient_dir(root, ambient_authority())?.open_dir("PS3_GAME/USRDIR/build/main")?;
    let f = dir.open("pak22.psarc")?;
    let len = f.metadata()?.len();
    let mut registry = FormatRegistry::default();
    crysirror_formats_arc::register(&mut registry)?;
    let mut context = DecodeContext::new(ClientId(1));
    context.limits.max_entry_size = 256 * 1024 * 1024;
    let (_, archive) = registry.open(Arc::new(File(Mutex::new(f), len)), Some("sony/psarc"), &context)?;
    let names: Vec<String> = archive.entries().iter().map(|e| e.path.to_string()).filter(|p| p.ends_with(".pak")).collect();
    let mount = ArchiveMount::new(archive, context)?;
    let (mut packages, mut meshes, mut fragments, mut triangles) = (0, 0, 0, 0);
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in &names {
        let handle = mount.open(name)?;
        let directory = Directory::read(handle.as_ref())?;
        let Some(item) = directory.items.iter().find(|i| i.kind == "PM_COLLECTION_2") else { continue };
        packages += 1;
        let result = mesh::Loaded::read(handle.as_ref(), 256 << 20).and_then(|l| mesh::prototype_meshes(&l, item.payload_offset as usize));
        match result {
            Ok(m) => {
                meshes += m.len();
                fragments += m.iter().map(Vec::len).sum::<usize>();
                triangles += m.iter().flatten().map(|f| f.triangles.len()).sum::<usize>();
            }
            Err(e) => failures.entry(format!("{e:#}")).or_default().push(name.clone()),
        }
    }
    println!("{packages} packages with prototype meshes: {meshes} meshes, {fragments} fragments, {triangles} triangles decoded");
    for (message, names) in &failures {
        println!("{} failed: {message} (e.g. {})", names.len(), names.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
    }
    Ok(())
}
