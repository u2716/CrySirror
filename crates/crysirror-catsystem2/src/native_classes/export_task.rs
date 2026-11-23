// SPDX-License-Identifier: MPL-2.0
//! Export task, class 45 (633B60, worker 633EC0): unpacks a `.zt` bundle of bonus files (icons,
//! wallpapers). The original asks for a folder and fills it; here the tree goes to `export/`
//! under the game's save folder, so no dialog or path outside it is involved.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use crysirror_core::path::{Mount, VfsPath};
use std::collections::BTreeMap;
use std::io::Read;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[690, 691, 692];

#[derive(Default)]
pub(crate) struct State {
    /// this[158]: a worker per task.
    workers: BTreeMap<u32, crysirror_tasks::Task<()>>,
}

/// 7269B0's node: { next sibling, first child, data size, data }, offsets from the node.
/// The data is { 1 for a folder, name at +4 }, and for a file its deflated bytes at +272.
fn walk(bytes: &[u8], mut at: usize, path: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<()> {
    let word = |at: usize| -> Result<usize> {
        Ok(u32::from_le_bytes(bytes.get(at..at + 4).context("CatSystem2 export node")?.try_into().unwrap()) as usize)
    };
    loop {
        let (next, child, size, kind) = (word(at)?, word(at + 4)?, word(at + 8)?, word(at + 12)?);
        let name = bytes.get(at + 16..at + 16 + 256).context("CatSystem2 export name")?;
        let name = name.iter().position(|b| *b == 0).map_or(name, |end| &name[..end]);
        let name = String::from_utf8_lossy(name);
        ensure!(!name.is_empty() && !name.contains(['/', '\\', ':']) && name != ".." && name != ".", "CatSystem2 export name {name}");
        let full = if path.is_empty() { name.into_owned() } else { format!("{path}/{name}") };
        if kind == 0 {
            let packed = bytes.get(at + 12 + 272..at + 12 + size).context("CatSystem2 export file")?;
            let mut plain = Vec::new();
            flate2::read::ZlibDecoder::new(packed).read_to_end(&mut plain)?;
            out.push((full.clone(), plain));
        }
        if child != 0 {
            walk(bytes, at + child, &full, out)?;
        }
        if next == 0 {
            return Ok(());
        }
        at += next;
    }
}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 export native arguments");
    match cx.call.id {
        // 650D10 / 650BD0 → 507: the bundle (692 passes the second name) in the export folder.
        690 | 692 => {
            cx.machine.preflight_native(false, &[])?;
            let source = if cx.call.id == 690 { a[1] } else { a[2] };
            let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, source)?).context("CatSystem2 export name")?;
            if !cx.state.export_task.workers.contains_key(&a[0]) {
                let storage = cx.host.storage(cx.client)?;
                let permit = match cx.pool.context("CatSystem2 worker pool missing")?.reserve() {
                    Ok(permit) => permit,
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                };
                let task = permit.spawn(move |cancel| {
                    let catalog = crate::native_resource_paths::Catalog::load(storage.as_ref())?;
                    for path in catalog.candidates(storage.as_ref(), "export", &name, &|| cancel.check())? {
                        cancel.check()?;
                        let Some(path) = crate::native_resource_paths::resolve(storage.as_ref(), &path)? else { continue };
                        let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 1 << 30)?;
                        let mut files = Vec::new();
                        walk(&bytes, 0, "", &mut files)?;
                        for (file, data) in files {
                            cancel.check()?;
                            storage.write_atomic(&VfsPath::new(Mount::Save, &format!("export/{file}"))?, &data)?;
                        }
                        break;
                    }
                    Ok(())
                })?;
                cx.state.export_task.workers.insert(a[0], task);
            }
            cx.machine.acknowledge(None)?;
        }
        // 650CD0 → 508: 1 once the worker ended; its failure surfaces here.
        691 => {
            cx.machine.preflight_native(true, &[])?;
            let finished = match cx.state.export_task.workers.get_mut(&a[0]) {
                Some(task) => task.try_take()?.is_some(),
                None => true,
            };
            if finished {
                cx.state.export_task.workers.remove(&a[0]);
            }
            cx.machine.acknowledge(Some(u32::from(finished)))?;
        }
        id => anyhow::bail!("CatSystem2 export native {id} is not implemented"),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn node(next: u32, child: u32, kind: u32, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut data = kind.to_le_bytes().to_vec();
        data.extend(name.as_bytes());
        data.resize(272, 0);
        data.extend(payload);
        let mut out = next.to_le_bytes().to_vec();
        out.extend(child.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    #[test]
    fn folders_and_deflated_files_unpack() {
        let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        zlib.write_all(b"hello").unwrap();
        let file = node(0, 0, 0, "a.jpg", &zlib.finish().unwrap());
        let mut bytes = node(0, 284, 1, "dir", &[]);
        bytes.extend(file);
        let mut out = Vec::new();
        walk(&bytes, 0, "", &mut out).unwrap();
        assert_eq!(out, vec![("dir/a.jpg".to_owned(), b"hello".to_vec())]);
    }
}
