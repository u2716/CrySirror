// Copyright (c) 2026 CrySirror contributors. MIT; see LICENSE.
//! Resource initialization through the caller's scoped storage and task pool.
//! Archive order and startup identity are explicit, not inferred from a demo.
use crate::program::{Profile, Program, MAX_SCRIPT_BYTES};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_resource::{Archive, ArchiveEntry, DecodeContext};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveKind {
    Will,
    AdvHd,
}
#[derive(Clone, Debug)]
pub struct ArchiveProbe {
    pub kind: ArchiveKind,
    pub entries: usize,
    pub ws2_scripts: usize,
    pub wsc_scripts: usize,
}
/// Structural format recognition, not executable/VM version recognition.
/// No filename-only ARC guess, archive extraction or host code execution.
pub fn probe_archive(
    client: ClientId,
    source: Arc<dyn crysirror_core::service::AssetReader>,
    cancel: &Cancellation,
) -> Result<Option<ArchiveProbe>> {
    ensure!(client.0 != 0, "WillPlus archive probe client identity");
    let mut selected = None;
    for kind in [ArchiveKind::Will, ArchiveKind::AdvHd] {
        cancel.check()?;
        let format = crysirror_formats_arc::legacy_archive(kind.format())
            .context("WillPlus shared archive format unavailable")?;
        if format.probe(source.as_ref())?.confidence == 0 {
            continue;
        }
        let context = DecodeContext::new(client).with_cancellation(cancel.clone());
        let archive = format.open(source.clone(), &context)?;
        ensure!(
            selected.is_none(),
            "ambiguous WillPlus ARC structural profile"
        );
        let mut ws2_scripts = 0;
        let mut wsc_scripts = 0;
        for entry in archive.entries() {
            cancel.check()?;
            let path = VfsPath::new(Mount::Asset, &entry.path.replace('\\', "/"))?;
            let lower = path.relative().to_ascii_lowercase();
            ws2_scripts += usize::from(lower.ends_with(".ws2"));
            wsc_scripts += usize::from(lower.ends_with(".wsc"));
        }
        selected = Some(ArchiveProbe {
            kind,
            entries: archive.entries().len(),
            ws2_scripts,
            wsc_scripts,
        });
    }
    cancel.check()?;
    Ok(selected)
}
impl ArchiveKind {
    pub fn format(self) -> &'static str {
        match self {
            Self::Will => "will/arc-will",
            Self::AdvHd => "will/arc-pulltop",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSpec {
    /// Relative to the caller's Asset capability, never an ambient OS path.
    pub path: String,
    pub kind: ArchiveKind,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryPoint {
    pub bank: usize,
    pub path: String,
    pub profile: Profile,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub archives: Vec<ArchiveSpec>,
    pub entry: Option<EntryPoint>,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.archives.is_empty() && self.archives.len() <= 64,
            "WillPlus archive bank budget"
        );
        let mut paths = BTreeSet::new();
        for bank in &self.archives {
            let path = VfsPath::new(Mount::Asset, &bank.path)?;
            ensure!(
                paths.insert(path.relative().to_ascii_lowercase()),
                "duplicate WillPlus archive bank"
            );
        }
        if let Some(entry) = &self.entry {
            ensure!(
                entry.bank < self.archives.len(),
                "WillPlus entry bank outside catalogue"
            );
            let path = VfsPath::new(Mount::Asset, &entry.path.replace('\\', "/"))?;
            ensure!(
                path.relative().to_ascii_lowercase().ends_with(".ws2"),
                "entry requires an explicitly decoded WS2 profile; WSC execution is not mapped"
            );
        }
        Ok(())
    }
}
pub struct Bank {
    pub source: VfsPath,
    pub entries: Vec<ArchiveEntry>,
    archive: Arc<dyn Archive>,
}
pub struct Prepared {
    pub client: ClientId,
    pub banks: Vec<Bank>,
    pub program: Option<Program>,
}
impl Prepared {
    pub fn open(
        &self,
        bank: usize,
        path: &str,
        cancel: &Cancellation,
    ) -> Result<Arc<dyn crysirror_core::service::AssetReader>> {
        cancel.check()?;
        let path = VfsPath::new(Mount::Asset, &path.replace('\\', "/"))?;
        let bank = self
            .banks
            .get(bank)
            .context("WillPlus resource bank absent")?;
        let entry = bank
            .entries
            .iter()
            .find(|e| e.path.eq_ignore_ascii_case(path.relative()))
            .with_context(|| format!("WillPlus resource absent: {}", path.relative()))?;
        bank.archive.open(
            entry.id,
            &DecodeContext::new(self.client).with_cancellation(cancel.clone()),
        )
    }
}
/// The host owns the worker pool and publishes this result after polling the
/// retained Task. Dropping the task cancels it; no frame, PCM or VM ACK occurs.
pub fn begin(
    pool: &TaskPool,
    client: ClientId,
    storage: Arc<dyn ClientStorage>,
    config: Config,
) -> Result<Task<Prepared>> {
    config.validate()?;
    ensure!(client.0 != 0, "WillPlus client identity");
    pool.spawn(move |cancel| {
        let mut banks = Vec::new();
        let mut total = 0usize;
        for spec in config.archives {
            cancel.check()?;
            let path = VfsPath::new(Mount::Asset, &spec.path)?;
            let source = storage.open_read(&path)?;
            let format = crysirror_formats_arc::legacy_archive(spec.kind.format())
                .context("WillPlus shared archive format unavailable")?;
            let context = DecodeContext::new(client).with_cancellation(cancel.clone());
            let archive = format.open(source, &context)?;
            let mut paths = BTreeSet::new();
            let mut entries = archive.entries().to_vec();
            for entry in &mut entries {
                cancel.check()?;
                let path = VfsPath::new(Mount::Asset, &entry.path.replace('\\', "/"))?;
                ensure!(
                    paths.insert(path.relative().to_ascii_lowercase()),
                    "ambiguous WillPlus resource name in one bank"
                );
                total = total
                    .checked_add(1)
                    .context("WillPlus catalogue count overflow")?;
                ensure!(total <= 100_000, "WillPlus catalogue entry budget");
                entry.path = path.relative().to_owned();
            }
            banks.push(Bank {
                source: path,
                entries,
                archive: Arc::from(archive),
            });
        }
        let mut result = Prepared {
            client,
            banks,
            program: None,
        };
        if let Some(entry) = config.entry {
            let source = result.open(entry.bank, &entry.path, &cancel)?;
            let bytes = crysirror_resource::read_all(source.as_ref(), MAX_SCRIPT_BYTES as u64)?;
            cancel.check()?;
            result.program = Some(Program::parse(entry.profile, bytes.into(), || {
                cancel.check()
            })?);
        }
        cancel.check()?;
        Ok(result)
    })
}
