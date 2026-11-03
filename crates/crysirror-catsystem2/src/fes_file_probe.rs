// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!77C070 original filename policy and result admission. File observation is
//!performed through scoped shared storage on a bounded engine worker.
use anyhow::{Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
pub(crate) struct Pending {
    pub owner: u32,
    pub pc: u32,
    pub operation: crate::fes_program::PendingOperation,
    pub flag: u32,
    task: Task<bool>,
    outcome: Option<std::result::Result<bool, String>>,
}
impl Pending {
    pub fn begin(
        pool: &TaskPool,
        storage: Arc<dyn ClientStorage>,
        owner: u32,
        pc: u32,
        operation: crate::fes_program::PendingOperation,
        flag: u32,
    ) -> Result<Option<Self>> {
        let bytes = &operation.arguments[0];
        ensure!(
            bytes.is_ascii() && bytes.len() < 260,
            "CatSystem2 FES fileexist filename encoding/extent"
        );
        let name = String::from_utf8(bytes.clone())?.replace('\\', "/");
        ensure!(
            VfsPath::new(Mount::Asset, &name)?.relative() == name,
            "CatSystem2 FES fileexist capability path"
        );
        let permit = match pool.reserve() {
            Ok(p) => p,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
            Err(e) => return Err(e),
        };
        let task = permit.spawn(move |cancel| {
            cancel.check()?;
            let startup =
                crate::native_settings::optional_asset(storage.as_ref(), "config/startup.xml")?
                    .ok_or_else(|| anyhow::anyhow!("CatSystem2 fileexist startup unavailable"))?;
            let xml = crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?;
            if let Some((archive, member)) = name.split_once('/') {
                if let Some(table) = xml.unique_child_path("INT")? {
                    for row in xml.children(table)? {
                        cancel.check()?;
                        let fields = xml.children(row)?.collect::<Vec<_>>();
                        if fields
                            .iter()
                            .find(|n| n.name == "int")
                            .is_some_and(|n| n.text.eq_ignore_ascii_case(archive))
                        {
                            if let Some(dir) = fields
                                .iter()
                                .find(|n| n.name == "dir" && !n.text.is_empty())
                            {
                                let candidate =
                                    format!("{}/{}", dir.text.replace('\\', "/"), member);
                                VfsPath::new(Mount::Asset, &candidate)?;
                                if storage.metadata(Mount::Asset, &candidate)?.is_some() {
                                    cancel.check()?;
                                    return Ok(true);
                                }
                            }
                            break;
                        }
                    }
                }
            }
            let present = storage.metadata(Mount::Asset, &name)?.is_some();
            cancel.check()?;
            Ok(present)
        })?;
        Ok(Some(Self {
            owner,
            pc,
            operation,
            flag,
            task,
            outcome: None,
        }))
    }
    pub fn poll(&mut self) -> Result<Option<bool>> {
        if self.outcome.is_none() {
            match self.task.try_take() {
                Ok(Some(value)) => self.outcome = Some(Ok(value)),
                Ok(None) => return Ok(None),
                Err(error) => self.outcome = Some(Err(format!("{error:#}"))),
            }
        }
        match self.outcome.as_ref().unwrap() {
            Ok(value) => Ok(Some(*value)),
            Err(error) => anyhow::bail!("{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::StorageMetadata;
    use std::{
        collections::BTreeMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct Storage {
        owner: std::thread::ThreadId,
        files: BTreeMap<String, Vec<u8>>,
        reads: AtomicUsize,
        probes: Mutex<Vec<String>>,
        fail: bool,
    }
    impl ClientStorage for Storage {
        fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            assert_ne!(self.owner, std::thread::current().id());
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.files
                .get(path.relative())
                .cloned()
                .ok_or(crysirror_core::Error::NotFound)
        }
        fn metadata(
            &self,
            mount: Mount,
            name: &str,
        ) -> crysirror_core::Result<Option<StorageMetadata>> {
            assert_ne!(self.owner, std::thread::current().id());
            assert_eq!(mount, Mount::Asset);
            self.probes.lock().unwrap().push(name.into());
            if self.fail {
                return Err(crysirror_core::Error::ReadOnly);
            }
            Ok(self.files.get(name).map(|b| StorageMetadata {
                is_directory: false,
                size: Some(b.len() as u64),
                created_ms: None,
                modified_ms: None,
                accessed_ms: None,
            }))
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
            Err(crysirror_core::Error::Unsupported("authored probe list"))
        }
    }
    fn operation(path: &[u8]) -> crate::fes_program::PendingOperation {
        crate::fes_program::PendingOperation {
            raw:Vec::new(),
            kind: crate::fes_program::PendingKind::Native,
            section: b"START".to_vec(),
            ordinal: 2,
            line: 3,
            byte_offset: 44,
            name: b"fileexist".to_vec(),
            arguments: vec![path.to_vec(), b"3".to_vec()],
        }
    }
    fn new_storage(fail: bool) -> Arc<Storage> {
        Arc::new(Storage {owner:std::thread::current().id(),files:BTreeMap::from([
            ("config/startup.xml".into(),b"<document><INT><int00><int>scene.int</int><dir>loose</dir></int00></INT></document>".to_vec()),
            ("loose/H01.cst".into(),vec![1]),
        ]),reads:AtomicUsize::new(0),probes:Mutex::new(vec![]),fail})
    }
    fn settle(p: &mut Pending) -> Result<bool> {
        let start = std::time::Instant::now();
        loop {
            if let Some(v) = p.poll()? {
                return Ok(v);
            }
            ensure!(
                start.elapsed() < std::time::Duration::from_secs(2),
                "authored probe timeout"
            );
            std::thread::yield_now();
        }
    }
    #[test]
    fn native_resource_probe_uses_worker_metadata_loose_priority_and_retained_result() {
        let pool = TaskPool::new("cat-probe-test", 1, 1).unwrap();
        let storage = new_storage(false);
        let mut pending = Pending::begin(
            &pool,
            storage.clone(),
            8,
            0x1943,
            operation(b"scene.int/H01.cst"),
            3,
        )
        .unwrap()
        .unwrap();
        assert!(settle(&mut pending).unwrap());
        assert_eq!(pending.poll().unwrap(), Some(true));
        assert_eq!(storage.reads.load(Ordering::SeqCst), 1);
        assert_eq!(*storage.probes.lock().unwrap(), ["loose/H01.cst"]);
        let before = storage.reads.load(Ordering::SeqCst);
        assert!(
            Pending::begin(
                &pool,
                storage.clone(),
                8,
                0x1943,
                operation(b"../escape"),
                3
            )
            .is_err()
        );
        assert_eq!(storage.reads.load(Ordering::SeqCst), before);
        let storage = new_storage(true);
        let mut pending = Pending::begin(
            &pool,
            storage.clone(),
            8,
            0x1943,
            operation(b"scene.int/H01.cst"),
            3,
        )
        .unwrap()
        .unwrap();
        let first = format!("{:#}", settle(&mut pending).unwrap_err());
        assert_eq!(format!("{:#}", pending.poll().unwrap_err()), first);
        assert_eq!(storage.probes.lock().unwrap().len(), 1);
    }
}
