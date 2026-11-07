// Copyright (c) 2026 CrySirror contributors. MIT.
//! Scoped original module discovery and worker preparation. The host mounts
//! archives; native policy selects a basename and executes its real MJO entry.
use super::*;
use crysirror_core::service::ClientStorage;
use crysirror_tasks::Cancellation;

#[derive(Default)]
pub(super) struct ModuleCatalog {
    paths: BTreeMap<String, String>,
    pub assets: Arc<BTreeMap<String, Vec<String>>>,
}
pub(super) struct PendingLibrary {
    event: Event,
    path: String,
    task: Task<Arc<Program>>,
}
impl ModuleCatalog {
    pub(super) fn prepare(
        storage: &dyn ClientStorage,
        script: &str,
        cancel: &Cancellation,
    ) -> Result<Self> {
        let mut roots = std::collections::BTreeSet::new();
        roots.insert(String::new());
        roots.insert(
            script
                .rsplit_once('/')
                .map(|(dir, _)| dir)
                .unwrap_or("")
                .to_owned(),
        );
        let names = storage.list(Mount::Asset, "")?;
        ensure!(names.len() <= 32768, "Majiro root directory index budget");
        for name in names {
            cancel.check()?;
            if name.to_ascii_lowercase().ends_with(".arc") {
                if let Some(point) = storage.container_mount(&name)? {
                    roots.insert(point);
                }
            }
        }
        ensure!(roots.len() <= 66, "Majiro module provider budget");
        let mut paths = BTreeMap::new();
        let mut assets: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut entries = 0usize;
        for root in roots {
            cancel.check()?;
            let names = storage.list(Mount::Asset, &root)?;
            entries = entries
                .checked_add(names.len())
                .context("Majiro module index overflow")?;
            ensure!(entries <= 262144, "Majiro module index entry budget");
            for name in names {
                cancel.check()?;
                ensure!(
                    !name.contains(['/', '\\']) && !name.chars().any(char::is_control),
                    "Majiro invalid indexed module name"
                );
                let path = if root.is_empty() {
                    name.clone()
                } else {
                    format!("{root}/{name}")
                };
                let path = VfsPath::new(Mount::Asset, &path)?.relative().to_owned();
                let key = name.to_ascii_lowercase();
                let providers = assets.entry(key.clone()).or_default();
                if !providers.contains(&path) {
                    providers.push(path.clone());
                }
                if !key.ends_with(".mjo") {
                    continue;
                }
                if let Some(previous) = paths.insert(key, path.clone()) {
                    ensure!(
                        previous == path,
                        "Majiro duplicate module providers require native archive precedence mapping"
                    );
                }
                ensure!(paths.len() <= 512, "Majiro module catalogue budget");
            }
        }
        Ok(Self {
            paths,
            assets: Arc::new(assets),
        })
    }
    fn resolve(&self, bytes: &[u8]) -> Result<&str> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= 4096 && !bytes.contains(&0),
            "Majiro module name extent"
        );
        let name = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .context("Majiro module name encoding")?;
        //40A790 uses _splitpath and appends .MJO; directories/drives never
        //grant access outside the scoped module catalogue.
        let base = name
            .rsplit(['/', '\\'])
            .next()
            .context("Majiro module basename")?;
        let stem = base.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(base);
        ensure!(
            !stem.is_empty() && !stem.contains(':') && !stem.chars().any(char::is_control),
            "Majiro module basename"
        );
        let logical = format!("{stem}.mjo");
        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&logical);
        ensure!(
            !errors && encoded.len() <= 127,
            "Majiro native module basename budget"
        );
        self.paths
            .get(&logical.to_ascii_lowercase())
            .map(String::as_str)
            .context("Majiro module is absent from scoped catalogue")
    }
}
impl MajiroVm {
    pub(super) fn try_library(
        &mut self,
        machine: &mut Machine,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        let Event::Native {
            hash: 0x10db0a43 | 0xcd00d280 | 0x9627ddc3 | 0x0edefb0b,
            arguments,
            ..
        } = event
        else {
            bail!("Majiro load_library event ABI");
        };
        ensure!(
            !arguments.is_empty(),
            "Majiro load_library filename argument absent"
        );
        let path = self
            .module_catalog
            .resolve(arguments[0].bytes()?)?
            .to_owned();
        if let Some(pending) = &self.library {
            ensure!(
                &pending.event == event && pending.path == path,
                "Majiro stale module dependency receipt"
            );
        } else if let Some(program) = machine.linked_module(&path) {
            machine.enter_library(path, program)?;
            return Ok(true);
        } else {
            let storage = host.storage(self.client).map_err(core_error)?;
            let source = VfsPath::new(Mount::Asset, &path)?;
            let permit = match self.pool.reserve() {
                Ok(permit) => permit,
                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(error) => return Err(error),
            };
            let task = permit.spawn(move |cancel| {
                cancel.check()?;
                let reader = storage.open_asset(&source)?;
                let len = usize::try_from(reader.len()?).context("Majiro library byte extent")?;
                ensure!(
                    (32..=MAX_MODULE_BYTES).contains(&len),
                    "Majiro library byte budget"
                );
                let mut bytes = vec![0; len];
                let mut at = 0;
                while at < len {
                    cancel.check()?;
                    let end = (at + 65536).min(len);
                    let n = reader.read_at(at as u64, &mut bytes[at..end])?;
                    ensure!(n > 0 && n <= end - at, "Majiro library short read");
                    at += n;
                }
                let program = Program::parse(&bytes)?;
                cancel.check()?;
                Ok(Arc::new(program))
            })?;
            self.library = Some(PendingLibrary {
                event: event.clone(),
                path,
                task,
            });
            return Ok(false);
        }
        let Some(program) = self.library.as_mut().unwrap().task.try_take()? else {
            return Ok(false);
        };
        //Module publication and the stack transition share one owner turn.
        //A failed read/parse leaves the original Native event/PC untouched.
        machine.enter_library(path, program)?;
        self.library = None;
        Ok(true)
    }
}
