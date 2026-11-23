// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native filename/default policy; shared resource format owns decoding.
use super::*;
impl SiglusVm {
    pub(super) fn begin_catalog(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let tables = &mut self.state.vm.as_mut().context("catalogue VM")?.ctx.tables;
        let count = tables
            .gameexe
            .as_ref()
            .and_then(|g| g.get_usize("CGTABLE_FLAG_CNT"))
            .unwrap_or(0);
        ensure!(count <= 1_048_576, "native CG flag count budget");
        tables.cg_flags = Arc::new(vec![0; count.max(32)]);
        let Some(name) = tables
            .gameexe
            .as_ref()
            .and_then(|g| g.get_unquoted("CGTABLE_FILE"))
            .map(str::to_owned)
        else {
            return Ok(());
        };
        ensure!(
            !name.is_empty() && name.len() <= 4096,
            "native CG catalogue name"
        );
        let mut file = PathBuf::from(name.replace('\\', "/"));
        if file.extension().is_none() {
            file.set_extension("cgm");
        }
        let file = file.to_str().context("CG catalogue path UTF-8")?.to_owned();
        let candidates = if file.contains('/') {
            vec![file]
        } else {
            vec![format!("dat/{file}"), file]
        };
        let paths = candidates
            .into_iter()
            .map(|p| VfsPath::new(Mount::Asset, &p).map_err(|e| anyhow::anyhow!("{e}")))
            .collect::<Result<Vec<_>>>()?;
        let storage = host.storage(ClientId(self.state.client)).map_err(err)?;
        let owner = self.state.client;
        let pool = self
            .catalog_pool
            .get_or_insert(crysirror_tasks::TaskPool::new("siglus-catalog", 1, 1)?);
        self.catalog_job = Some(pool.spawn(move |cancel| {
            use crysirror_resource::FileFormat;
            let mut source = None;
            for path in paths {
                cancel.check()?;
                match storage.open_asset(&path) {
                    Ok(reader) => {
                        source = Some(reader);
                        break;
                    }
                    Err(Error::NotFound) => (),
                    Err(error) => return Err(anyhow::anyhow!("{error}")),
                }
            }
            let source = source.context("native configured CG catalogue missing")?;
            let context = crysirror_resource::DecodeContext::new(ClientId(owner))
                .with_cancellation(cancel.clone());
            cancel.check()?;
            let decoded = crysirror_resource::cg_catalog::CgCatalogFormat.decode(
                "catalog.cgm",
                source,
                &context,
            )?;
            let json = crysirror_resource::read_all(decoded.reader.as_ref(), 32 * 1024 * 1024)?;
            let catalog:crysirror_resource::cg_catalog::CgCatalog = serde_json::from_slice(&json)?;
            ensure!(catalog.entries.iter().all(|entry|entry.flag_no<1_048_576),"native CG catalogue flag index budget");
            cancel.check()?;
            Ok(Arc::new(catalog))
        })?);
        Ok(())
    }
    pub(super) fn poll_catalog(&mut self) -> Result<bool> {
        let Some(job) = self.catalog_job.as_mut() else {
            return Ok(true);
        };
        let Some(catalog) = job.try_take()? else {
            return Ok(false);
        };
        self.state
            .vm
            .as_mut()
            .context("catalogue completion VM")?
            .ctx
            .tables
            .cg_catalog = Some(catalog);
        self.catalog_job = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::{ClientStorage, FileSystem};
    #[test]
    #[ignore = "reads the owner's installed Rewrite CG table; no game data included"]
    fn real_catalogue_prepares_on_a_scoped_worker_before_original_instructions() {
        struct Storage {
            owner: std::thread::ThreadId,
        }
        impl ClientStorage for Storage {
            fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
                assert_ne!(
                    self.owner,
                    std::thread::current().id(),
                    "catalogue I/O must run on worker"
                );
                if path.mount() != Mount::Asset || path.relative() != "dat/mode.cgm" {
                    return Err(Error::NotFound);
                }
                std::fs::read("/Users/xmoe/Downloads/gamedata_rewrite_hf/dat/mode.cgm").map_err(err)
            }
            fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
                Err(Error::ReadOnly)
            }
            fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
                Err(Error::ReadOnly)
            }
            fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
                Err(Error::NotFound)
            }
        }
        struct Host(Arc<dyn ClientStorage>);
        impl FileSystem for Host {
            fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
                panic!("owner file read")
            }
            fn write_atomic(
                &mut self,
                _: ClientId,
                _: &VfsPath,
                _: &[u8],
            ) -> crysirror_core::Result<()> {
                Err(Error::ReadOnly)
            }
        }
        impl HostServices for Host {
            fn submit(
                &mut self,
                _: ClientId,
                _: &[crysirror_core::state::Command],
            ) -> crysirror_core::Result<()> {
                Err(Error::Unsupported("no drawing in catalogue preparation"))
            }
            fn now_ms(&self) -> u64 {
                0
            }
            fn storage(
                &mut self,
                client: ClientId,
            ) -> crysirror_core::Result<Arc<dyn ClientStorage>> {
                assert_eq!(client, ClientId(1));
                Ok(self.0.clone())
            }
        }
        let mut vm = SiglusVm::new(
            ClientId(1),
            serde_json::from_value(serde_json::json!({"scene":"Scene.scn"})).unwrap(),
        )
        .unwrap();
        let mut ctx = CommandContext::new("asset".into());
        ctx.tables.gameexe = Some(Arc::new(crate::gameexe::GameexeConfig::from_text(
            "#CGTABLE_FILE=\"mode\"\n#CGTABLE_FLAG_CNT=10000\n",
        )));
        let mut scene = crate::fixture::SceneBuilder::default();
        scene.code.push(crate::runtime::constants::cd::EOF);
        let stream = SceneStream::new_owned_with_string_codec(
            Arc::from(scene.finish()),
            SceneStringCodec::Plain,
        )
        .unwrap();
        vm.state.vm = Some(SceneVm::new(stream, ctx));
        let mut host = Host(Arc::new(Storage {
            owner: std::thread::current().id(),
        }));
        vm.begin_catalog(&mut host).unwrap();
        assert!(vm.catalog_job.is_some());
        assert!(vm.snapshot().is_err());
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !vm.poll_catalog().unwrap() {
            assert!(std::time::Instant::now() < end);
            std::thread::yield_now();
        }
        let ctx = &vm.state.vm.as_ref().unwrap().ctx;
        assert!(
            ctx.tables
                .cg_catalog
                .as_ref()
                .is_some_and(|c| !c.entries.is_empty())
        );
        assert_eq!(ctx.tables.cg_flags.len(), 10000);
        assert!(vm.snapshot().is_ok());
        assert_eq!(vm.diagnostics()["pc"], 0);
    }
}
