//! sdat methods (6DBA38): SystemData files in the save capability.
//!
//! `s[,name]` writes (6D5ABC), `l[,name]` loads (6D5904), `d,name` deletes
//! (509758), `g`/`?g,name` test existence (50940C); the name defaults to
//! SystemData.dat and each success pushes ResultInt 1 (a failure pushes
//! nothing). `i` frees the svar calculator bank (6C0E74).
use super::*;

pub(super) enum SdatIo {
    Wrote,
    Read(Option<Vec<u8>>),
    Exists(bool),
    Removed(bool),
}
pub(super) struct SdatPending {
    owner: (String, usize, String),
    task: Task<SdatIo>,
}

impl QlieVm {
    pub(super) fn sdat_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        if op == "i" {
            m.clear_shared_bank();
            return Ok(true);
        }
        ensure!(
            matches!(op, "s" | "l" | "d" | "g" | "?g"),
            "unsupported QLIE sdat method \\{op}"
        );
        ensure!(a.len() <= 1, "QLIE system data ABI");
        let mut name = a.first().map(Value::text).unwrap_or_default();
        if name.is_empty() {
            if op == "d" {
                return Ok(true);
            }
            name = "SystemData.dat".into();
        }
        let owner = (m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.sdat_io.is_none() {
            let path = VfsPath::new(Mount::Save, &crate::program::canonical(&name.replace('\\', "/"))?)?;
            let storage = h.storage(self.client)?;
            let task = match op {
                "s" => {
                    let bytes = self.system_data_bytes(m)?;
                    self.pool.spawn(move |cancel| {
                        cancel.check()?;
                        storage.write_atomic(&path, &bytes)?;
                        Ok(SdatIo::Wrote)
                    })?
                }
                "l" => self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let found = storage
                        .metadata(path.mount(), path.relative())?
                        .is_some_and(|i| !i.is_directory);
                    Ok(SdatIo::Read(if found { Some(storage.read(&path)?) } else { None }))
                })?,
                "d" => self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let found = storage.metadata(path.mount(), path.relative())?.is_some();
                    if found {
                        storage.remove(&path)?;
                    }
                    Ok(SdatIo::Removed(found))
                })?,
                _ => self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    Ok(SdatIo::Exists(
                        storage
                            .metadata(path.mount(), path.relative())?
                            .is_some_and(|i| !i.is_directory),
                    ))
                })?,
            };
            self.sdat_io = Some(SdatPending { owner: owner.clone(), task });
        }
        let pending = self.sdat_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE system data completion owner");
        let Some(result) = pending.task.try_take()? else {
            return Ok(false);
        };
        self.sdat_io = None;
        let ok = match result {
            SdatIo::Wrote => true,
            SdatIo::Read(Some(bytes)) => self.apply_system_data(h, m, &bytes)?,
            SdatIo::Read(None) => false,
            SdatIo::Exists(found) | SdatIo::Removed(found) => found,
        };
        if ok {
            m.result(0, Value::Int(1));
        }
        Ok(true)
    }
}
