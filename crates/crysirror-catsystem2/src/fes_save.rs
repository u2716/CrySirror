// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FES save commands (77B270, 77EB10, 77ED00, 77EBE0, 77EF20, 77B520, 77F530, 77F6B0,
//! 785340): slot files on a worker, so the frame only waits for their completion.
use crate::native_save::Container;
use anyhow::Result;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
    Error,
};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;

pub(crate) fn slot_name(slot: u32) -> String {
    format!("save{slot:04}.dat")
}
/// What a finished job writes back into the VM.
#[derive(Debug, Default)]
pub(crate) struct Answer {
    pub flags: Vec<(u32, u32)>,
    pub string: Option<(u32, Vec<u8>)>,
}
pub(crate) struct Job {
    pub owner: u32,
    pub pc: u32,
    task: Task<Answer>,
}
impl Job {
    pub fn begin(pool: &TaskPool, owner: u32, pc: u32, work: impl FnOnce(Cancellation) -> Result<Answer> + Send + 'static) -> Result<Option<Self>> {
        let permit = match pool.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(Self { owner, pc, task: permit.spawn(work)? }))
    }
    pub fn poll(&mut self) -> Result<Option<Answer>> {
        self.task.try_take()
    }
}
fn path(name: &str) -> Result<VfsPath> {
    Ok(VfsPath::new(Mount::Save, name)?)
}
/// 78BB10 + 78BF10: 1 for a usable file, -1 outside the accepted version range, 0 when absent.
pub(crate) fn exists(storage: &dyn ClientStorage, client: ClientId, name: &str, range: Option<[u32; 2]>, cancel: Cancellation) -> Result<u32> {
    let Some(saved) = crate::native_save::load(storage, client, name, cancel)? else { return Ok(0) };
    Ok(match range {
        Some([low, high]) if saved.tag < low || saved.tag > high => u32::MAX,
        _ => 1,
    })
}
pub(crate) fn delete(storage: &dyn ClientStorage, name: &str) -> Result<()> {
    match storage.remove(&path(name)?) {
        Err(Error::NotFound) => Ok(()),
        other => Ok(other?),
    }
}
fn read(storage: &dyn ClientStorage, name: &str) -> Result<Option<Vec<u8>>> {
    let reader = match storage.open_read(&path(name)?) {
        Ok(reader) => reader,
        Err(Error::NotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(crysirror_resource::read_all(reader.as_ref(), 16 * 1024 * 1024)?))
}
/// 78C260: the destination becomes a copy; a missing source leaves it alone.
pub(crate) fn copy(storage: &dyn ClientStorage, from: &str, to: &str) -> Result<()> {
    if let Some(bytes) = read(storage, from)? {
        storage.write_atomic(&path(to)?, &bytes)?;
    }
    Ok(())
}
/// 78BFE0: the two slots trade contents.
pub(crate) fn exchange(storage: &dyn ClientStorage, a: &str, b: &str) -> Result<()> {
    let (first, second) = (read(storage, a)?, read(storage, b)?);
    match second {
        Some(bytes) => storage.write_atomic(&path(a)?, &bytes)?,
        None => delete(storage, a)?,
    }
    match first {
        Some(bytes) => storage.write_atomic(&path(b)?, &bytes)?,
        None => delete(storage, b)?,
    }
    Ok(())
}
/// 77EF20: the most recently written slot in the range, or -1.
pub(crate) fn newest(storage: &dyn ClientStorage, range: [u32; 2]) -> Result<u32> {
    let mut best: Option<(i64, u32)> = None;
    for slot in range[0]..=range[1] {
        let Some(metadata) = storage.metadata(Mount::Save, &slot_name(slot))? else { continue };
        let modified = metadata.modified_ms.unwrap_or(0);
        if best.is_none_or(|(time, _)| modified > time) {
            best = Some((modified, slot));
        }
    }
    Ok(best.map_or(u32::MAX, |(_, slot)| slot))
}
/// 78BD20: the slot's write time as year, month, day, hour, minute, second (UTC civil).
pub(crate) fn date(storage: &dyn ClientStorage, slot: u32) -> Result<Option<[u32; 6]>> {
    let Some(metadata) = storage.metadata(Mount::Save, &slot_name(slot))? else { return Ok(None) };
    let Some(modified) = metadata.modified_ms else { return Ok(None) };
    let [year, month, _week, day, hour, minute, second] = crate::native_misc::civil(modified.max(0) as u64 / 1000);
    Ok(Some([year, month, day, hour, minute, second]))
}
/// 77F530: the record (with its terminator, padded by the original to +4) replaces any earlier one.
pub(crate) fn append_string(storage: &dyn ClientStorage, client: ClientId, name: &str, key: u32, text: &[u8], cancel: Cancellation) -> Result<()> {
    let Some(mut saved) = crate::native_save::load(storage, client, name, cancel)? else { return Ok(()) };
    saved.remove(key);
    let mut data = text.to_vec();
    data.extend_from_slice(&[0; 4]);
    saved.append(key, &data)?;
    storage.write_atomic(&path(name)?, &saved.file_image()?)?;
    Ok(())
}
/// 77F6B0.
pub(crate) fn get_string(storage: &dyn ClientStorage, client: ClientId, name: &str, key: u32, cancel: Cancellation) -> Result<Option<Vec<u8>>> {
    let Some(saved) = crate::native_save::load(storage, client, name, cancel)? else { return Ok(None) };
    Ok(saved.first(key).map(|data| data.split(|b| *b == 0).next().unwrap_or_default().to_vec()))
}
/// 77B520: the container handle becomes the slot's file.
pub(crate) fn write(storage: &dyn ClientStorage, name: &str, saved: &Container) -> Result<()> {
    storage.write_atomic(&path(name)?, &saved.file_image()?)?;
    Ok(())
}
pub(crate) type SharedStorage = Arc<dyn ClientStorage>;
