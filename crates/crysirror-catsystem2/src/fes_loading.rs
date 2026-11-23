// SPDX-License-Identifier: MPL-2.0
//! Native FES naming/load policy. Shared Storage/XML/Resource transforms own
//! capability I/O, XML parsing and zlib; this module has no decompressor.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, MemoryAssetReader},
    state::ClientId,
};
use crysirror_resource::{xml::XmlDocument, DecodeContext, TransformSpec};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Prepared {
    pub path: String,
    pub program: Arc<crate::fes_program::Program>,
}
pub(crate) fn load(
    storage: &dyn ClientStorage,
    client: ClientId,
    name: &str,
    cancel: crysirror_tasks::Cancellation,
) -> Result<Prepared> {
    let check = || cancel.check();
    check()?;
    ensure!(
        name.is_ascii() && name.len() < 260,
        "CatSystem2 unsupported FES filename profile"
    );
    let name = name.replace('\\', "/");
    VfsPath::new(Mount::Asset, &name)?;
    let startup = crate::native_settings::optional_asset(storage, "config/startup.xml")?
        .context("CatSystem2 FES startup missing")?;
    let xml = XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?;
    let table = xml
        .unique_child_path("INT")?
        .context("CatSystem2 FES INT table missing")?;
    let mut row = None;
    for node in xml.children(table)? {
        if let Some(n) = xml.children(node)?.find(|n| n.name == "name") {
            if n.text.eq_ignore_ascii_case("fes") {
                ensure!(row.is_none(), "CatSystem2 duplicate FES INT declaration");
                row = Some(node);
            }
        }
    }
    let row = row.context("CatSystem2 FES resource type missing")?;
    let field = |key: &str| -> Result<Option<String>> {
        let mut found = xml.children(row)?.filter(|n| n.name == key);
        let v = found.next();
        ensure!(found.next().is_none(), "CatSystem2 duplicate FES INT field");
        Ok(v.map(|n| n.text.clone()))
    };
    let archive = field("int")?.context("CatSystem2 FES archive missing")?;
    let ext = field("ext")?.context("CatSystem2 FES extension missing")?;
    ensure!(
        !ext.is_empty() && !ext.contains(['.', '/', '\\', ':']),
        "CatSystem2 FES extension profile"
    );
    let path = if name.contains('/') {
        name
    } else {
        let file = if name.contains('.') {
            name
        } else {
            format!("{name}.{ext}")
        };
        format!("{archive}/{file}")
    };
    VfsPath::new(Mount::Asset, &path)?;
    check()?;
    // 5D5330 opens members without regard to ASCII case (`hist_b` is `hist_B.fes`).
    let path = VfsPath::new(Mount::Asset, &path)
        .ok()
        .and_then(|vfs| crate::native_resource_paths::resolve(storage, &vfs).ok().flatten())
        .map_or(path, |found| found.relative().to_owned());
    let bytes = crate::native_settings::optional_native_asset(storage, &xml, &path)?
        .context("CatSystem2 unsupported FES missing-load result profile")?;
    ensure!(
        bytes.len() >= 16 && bytes.len() <= 16 * 1024 * 1024 && &bytes[..4] == b"FES\0",
        "CatSystem2 unsupported FES container"
    );
    let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    let packed = word(4);
    let size = word(8);
    ensure!(
        packed > 0
            && packed.checked_add(16) == Some(bytes.len())
            && size > 0
            && size <= 16 * 1024 * 1024
            && word(12) == 0,
        "CatSystem2 unsupported plain FES/preprocess or container extent"
    );
    let context = DecodeContext::new(client).with_cancellation(cancel.clone());
    let mut spec = TransformSpec::new("zlib");
    spec.expected_size = Some(size as u64);
    let reader = context.transforms.apply(
        &spec,
        Arc::new(MemoryAssetReader(Arc::from(bytes[16..].to_vec()))),
        &serde_json::Value::Null,
        &context,
    )?;
    let text = crysirror_resource::read_all(reader.as_ref(), 16 * 1024 * 1024)?;
    check()?;
    let program = Arc::new(crate::fes_program::Program::parse(&text, &check)?);
    Ok(Prepared { path, program })
}

#[cfg(all(test, feature = "runtime"))]
mod owner_tests {
    use super::*;
    use crysirror_resource::DependencyResolver;
    struct Root(std::path::PathBuf);
    impl DependencyResolver for Root {
        fn open(&self, path: &str) -> Result<Arc<dyn crysirror_core::service::AssetReader>> {
            let scoped = VfsPath::new(Mount::Asset, path)?;
            ensure!(scoped.relative() == path, "owner FES resolver escaped root");
            Ok(Arc::new(MemoryAssetReader(Arc::from(std::fs::read(
                self.0.join(path),
            )?))))
        }
    }
    #[test]
    #[ignore = "requires explicitly authorized CRYSIRROR_CATSYSTEM2_GAME; no game bytes distributed"]
    fn owner_actual_fes_source_executes_original_expressions_then_retains_platform_operation(
    ) -> Result<()> {
        use crysirror_core::service::HostServices;
        let root = std::path::PathBuf::from(
            std::env::var_os("CRYSIRROR_CATSYSTEM2_GAME").context("authorized game data root")?,
        );
        let resolver = Arc::new(Root(root.clone()));
        let mut context = DecodeContext::new(ClientId(1));
        context.resolver = Some(resolver.clone());
        context.source_name = Some("fes.int".into());
        context.parameters.insert(
            "cat-system/int".into(),
            serde_json::json!({"executable":"cs2.exe"}),
        );
        let format = crysirror_formats_arc::legacy_archive("cat-system/int").unwrap();
        let archive = format.open(resolver.open("fes.int")?, &context)?;
        let provider = Arc::new(crysirror_resource::ArchiveMount::new(archive, context)?);
        let project: crysirror_runtime::Project = serde_json::from_value(
            serde_json::json!({"schema":2,"name":"owner FES CPU","content_revision":"1","assets":"game","canvas":[1280,720],"clients":[{"name":"source","adapter":"crysirror-script-v1","game":"source"}]}),
        )?;
        let state = tempfile::tempdir()?;
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            &root,
            state.path(),
            Default::default(),
        )?;
        rt.mount_provider(ClientId(1), "fes.int", provider)?;
        let storage = rt.host.storage(ClientId(1))?;
        let pool = crysirror_tasks::TaskPool::new("owner-fes-source", 1, 1)?;
        let mut task =
            pool.spawn(move |cancel| load(storage.as_ref(), ClientId(1), "flow", cancel))?;
        let start = std::time::Instant::now();
        let prepared = loop {
            if let Some(ready) = task.try_take()? {
                break ready;
            }
            ensure!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "owner FES worker timeout"
            );
            std::thread::yield_now();
        };
        let mut interpreter = crate::native_interpreter::Interpreter::new(2048)?;
        for flag in [540, 542, 1001, 545] {
            interpreter.set_flag(flag, u32::MAX);
        }
        let mut vm = crate::fes_program::Machine::new(prepared.program, 8, &mut interpreter)?;
        let op = vm
            .step(&mut interpreter, 1024)?
            .context("original flow did not reach its first platform call")?;
        ensure!(
            vm.executed == 4
                && op.name == b"steam_isenable"
                && op.arguments == vec![b"1002".to_vec()],
            "original FES expression/native frontier changed"
        );
        for flag in [540, 542, 1001, 545] {
            ensure!(
                interpreter.flag(flag) == 0,
                "original FES expression was not executed"
            );
        }
        ensure!(
            vm.step(&mut interpreter, 1024)? == Some(op.clone()) && vm.executed == 4,
            "original FES pending operation replayed commands"
        );
        println!("actual original FES path={} section={} row={} physical_line={} byte_offset={:#x} executed={} pending={} args={:?}",prepared.path,String::from_utf8_lossy(&op.section),op.ordinal,op.line+1,op.byte_offset,vm.executed,String::from_utf8_lossy(&op.name),op.arguments);
        println!("{}", vm.diagnostics());
        Ok(())
    }
}

/// Completed worker output is cached until the owner atomically admits the
/// player. A failed admission/retry must never consume a task result twice.
pub(crate) struct Pending {
    pub task_id: u32,
    pub native_pc: u32,
    worker: Option<crysirror_tasks::Task<Prepared>>,
    ready: Option<Prepared>,
    failed: Option<String>,
}
impl Pending {
    pub fn new(task_id: u32, native_pc: u32, worker: crysirror_tasks::Task<Prepared>) -> Self {
        Self {
            task_id,
            native_pc,
            worker: Some(worker),
            ready: None,
            failed: None,
        }
    }
    pub fn poll(&mut self) -> Result<Option<Prepared>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("{error}");
        }
        if self.ready.is_none() {
            match self
                .worker
                .as_mut()
                .context("CatSystem2 FES worker missing")?
                .try_take()
            {
                Ok(Some(ready)) => {
                    self.ready = Some(ready);
                    self.worker = None;
                }
                Ok(None) => return Ok(None),
                Err(error) => {
                    let error = format!("{error:#}");
                    self.failed = Some(error.clone());
                    self.worker = None;
                    anyhow::bail!("{error}");
                }
            }
        }
        Ok(self.ready.clone())
    }
}

#[cfg(test)]
mod pending_tests {
    use super::*;
    #[test]
    fn completed_worker_stays_owned_across_repeated_admission_attempts() -> Result<()> {
        let pool = crysirror_tasks::TaskPool::new("fes-result-retry", 1, 1)?;
        let worker = pool.spawn(|_| {
            Ok(Prepared {
                path: "fes.int/test.fes".into(),
                program: Arc::new(crate::fes_program::Program::parse(
                    b"#START\n$0=1",
                    &|| Ok(()),
                )?),
            })
        })?;
        let mut pending = Pending::new(8, 0x1926, worker);
        let start = std::time::Instant::now();
        let first = loop {
            if let Some(v) = pending.poll()? {
                break v;
            }
            ensure!(
                start.elapsed() < std::time::Duration::from_secs(5),
                "FES worker timeout"
            );
            std::thread::yield_now();
        };
        let second = pending.poll()?.context("completed FES lost")?;
        assert!(Arc::ptr_eq(&first.program, &second.program));
        assert_eq!((pending.task_id, pending.native_pc), (8, 0x1926));
        let worker = pool.spawn(|_| anyhow::bail!("authored load failure"))?;
        let mut failed = Pending::new(8, 0x1926, worker);
        let error = loop {
            match failed.poll() {
                Err(e) => break e.to_string(),
                Ok(None) => std::thread::yield_now(),
                Ok(Some(_)) => anyhow::bail!("unexpected worker result"),
            }
            ensure!(
                start.elapsed() < std::time::Duration::from_secs(5),
                "FES failure timeout"
            );
        };
        assert_eq!(
            failed
                .poll()
                .err()
                .context("FES failed result lost")?
                .to_string(),
            error
        );
        assert!(error.contains("authored load failure"));
        Ok(())
    }
}
