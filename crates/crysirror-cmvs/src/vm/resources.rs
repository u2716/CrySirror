// SPDX-License-Identifier: MPL-2.0
//! Native path policy over the shared CrySirror resource capability.

use super::{PreparedTransfer, ScriptRequest, VmProfile};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AssetReader, ClientStorage},
};
use crysirror_resource::{
    ArchiveMount, DecodeContext, FormatRegistry, KeyMaterial, KeyProvider,
    capability_lookup::resolve_ascii_case_insensitive, pe::PeData, read_all,
};
use std::{collections::BTreeMap, sync::Arc};

const CPZ6_FORMAT: &str = "purple-software/cpz6";
const CPZ7_FORMAT: &str = "purple-software/cpz7";
const CPZ1_FORMAT: &str = "purple-software/cpz1";
const CPZ6_KEY: &str = "cmvs-mode1-table";
const CPZ6_TABLE_BYTES: usize = 96;
const EXECUTABLE_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceSource {
    Directory(VfsPath),
    Archive(VfsPath),
    /// Registered through `0x2268..=0x226e` with the script's four V7B words.
    KeyedArchive(VfsPath, [u32; 4]),
}

#[derive(Clone, Debug, Default)]
pub struct ResourceBindings {
    slots: BTreeMap<(u16, u32), ResourceSource>,
    movie_root: Option<VfsPath>,
    save_suffix: Option<VfsPath>,
    #[cfg(feature = "runtime")]
    save_suffix_is_directory: bool,
}

impl ResourceBindings {
    pub fn sources(&self, opcode: u16) -> impl Iterator<Item = &ResourceSource> {
        self.slots
            .range((opcode, 0)..=(opcode, u32::MAX))
            .map(|(_, source)| source)
    }

    pub fn script_sources(&self) -> Vec<ResourceSource> {
        self.sources(0x2010).cloned().collect()
    }

    pub fn movie_root(&self) -> Option<&VfsPath> {
        self.movie_root.as_ref()
    }

    pub fn save_suffix(&self) -> Option<&VfsPath> {
        self.save_suffix.as_ref()
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn save_file_base(&self) -> Option<(&VfsPath, bool)> {
        self.save_suffix()
            .map(|path| (path, self.save_suffix_is_directory))
    }

    pub(super) fn set_source(&mut self, opcode: u16, slot: u32, path: &[u8]) -> Result<()> {
        let path_name = path_string(path)?;
        let directory = path_name.ends_with(['/', '\\']);
        let path = VfsPath::new(Mount::Asset, &path_name.replace('\\', "/"))?;
        let source = if directory {
            ResourceSource::Directory(path)
        } else {
            ResourceSource::Archive(path)
        };
        self.slots.insert((opcode, slot), source);
        Ok(())
    }

    pub(super) fn set_keyed_source(
        &mut self,
        opcode: u16,
        slot: u32,
        path: &[u8],
        words: [u32; 4],
    ) -> Result<()> {
        let path = VfsPath::new(Mount::Asset, &path_string(path)?.replace('\\', "/"))?;
        self.slots
            .insert((opcode, slot), ResourceSource::KeyedArchive(path, words));
        Ok(())
    }

    pub(super) fn set_root(&mut self, opcode: u16, path: &[u8]) -> Result<()> {
        let path = path_string(path)?.replace('\\', "/");
        if opcode == 0x2015 {
            self.movie_root = Some(VfsPath::new(Mount::Asset, &path)?);
        } else {
            self.save_suffix = Some(VfsPath::new(Mount::Save, &path)?);
            #[cfg(feature = "runtime")]
            {
                self.save_suffix_is_directory = path.ends_with('/');
            }
        }
        Ok(())
    }
}

pub(super) fn path_string(bytes: &[u8]) -> Result<String> {
    ensure!(
        !bytes.is_empty() && bytes.len() < 2048,
        "CMVS resource path length"
    );
    let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!errors, "CMVS resource path is not valid CP932");
    Ok(text.into_owned())
}

struct ClientTable {
    client: crysirror_core::state::ClientId,
    id: &'static str,
    table: KeyMaterial,
    fallback: Arc<dyn KeyProvider>,
}

impl KeyProvider for ClientTable {
    fn key(&self, client: crysirror_core::state::ClientId, key_id: &str) -> Result<KeyMaterial> {
        if key_id == self.id {
            ensure!(
                client == self.client,
                "CMVS family archive table belongs to a different client"
            );
            Ok(self.table.clone())
        } else {
            self.fallback.key(client, key_id)
        }
    }
}

/// Scoped configuration for the shared Purple Software archive readers.
/// Blocking executable and archive reads belong to the storage preparation worker.
pub struct Cpz6Resources {
    context: DecodeContext,
    format: &'static str,
}

impl Cpz6Resources {
    pub fn prepare(
        profile: VmProfile,
        context: DecodeContext,
        executable: &dyn AssetReader,
    ) -> Result<Self> {
        let bytes = read_all(executable, EXECUTABLE_LIMIT)?;
        let pe = PeData::parse(&bytes)?;
        cpz6_from_pe(profile, context, &pe)
    }

    pub fn open(
        &self,
        storage: &dyn ClientStorage,
        registry: &FormatRegistry,
        path: &VfsPath,
    ) -> Result<ArchiveMount> {
        ensure!(
            path.mount() == Mount::Asset,
            "CMVS archive must be in the asset root"
        );
        let path =
            resolve_ascii_case_insensitive(storage, path)?.context("CMVS archive missing")?;
        let source = storage.open_asset(&path)?;
        let mut context = self.context.clone();
        context.source_name = Some(path.relative().into());
        let (_, archive) = registry.open(source, Some(self.format), &context)?;
        ArchiveMount::new(archive, context)
    }

    pub fn open_keyed(
        &self,
        storage: &dyn ClientStorage,
        registry: &FormatRegistry,
        path: &VfsPath,
        words: [u32; 4],
    ) -> Result<ArchiveMount> {
        ensure!(
            self.format == CPZ6_FORMAT,
            "CPZ7 parameters require a CMVS profile"
        );
        ensure!(
            path.mount() == Mount::Asset,
            "CMVS archive must be in the asset root"
        );
        let path =
            resolve_ascii_case_insensitive(storage, path)?.context("CMVS archive missing")?;
        let source = storage.open_asset(&path)?;
        let mut context = self.context.clone();
        context.source_name = Some(path.relative().into());
        context.parameters.insert(
            CPZ7_FORMAT.into(),
            serde_json::json!({
                "layout": "cmvs_v7b",
                "key_id": CPZ6_KEY,
                "words": words,
            }),
        );
        let (_, archive) = registry.open(source, Some(CPZ7_FORMAT), &context)?;
        ArchiveMount::new(archive, context)
    }
}

pub(crate) fn cpz6_from_pe(
    profile: VmProfile,
    mut context: DecodeContext,
    pe: &PeData<'_>,
) -> Result<Cpz6Resources> {
    let section = pe
        .sections()
        .iter()
        .find(|section| &section.name == b".rdata\0\0")
        .context("CMVS executable has no .rdata section")?;
    let is_cvns = profile == VmProfile::Cvns20040515X86;
    let table_len = if is_cvns { 64 } else { CPZ6_TABLE_BYTES };
    let table_rva = profile.archive_table_rva();
    ensure!(
        table_rva >= section.rva
            && u64::from(table_rva) + table_len as u64
                <= u64::from(section.rva) + u64::from(section.file_size),
        "native archive table is not in the supported file-backed section"
    );
    let mut bytes = pe.rva_slice(table_rva, table_len)?.to_vec();
    let (format, id, parameters) = if is_cvns {
        for byte in &mut bytes {
            *byte = byte.wrapping_add(10);
        }
        (
            CPZ1_FORMAT,
            CPZ1_FORMAT,
            serde_json::json!({"key_id": CPZ1_FORMAT}),
        )
    } else {
        (
            CPZ6_FORMAT,
            CPZ6_KEY,
            serde_json::json!({"layout": "cmvs_mode1", "key_id": CPZ6_KEY}),
        )
    };
    context.keys = Arc::new(ClientTable {
        client: context.client,
        id,
        table: KeyMaterial::new(bytes)?,
        fallback: context.keys.clone(),
    });
    context.parameters.insert(format.into(), parameters);
    Ok(Cpz6Resources { context, format })
}

enum ScriptSource {
    Directory(VfsPath),
    Archive(ArchiveMount),
}

/// Prepared script sources hold shared archive readers, with no extracted copies.
pub struct ScriptSources {
    storage: Arc<dyn ClientStorage>,
    sources: Vec<ScriptSource>,
}

impl ScriptSources {
    pub fn prepare(
        storage: Arc<dyn ClientStorage>,
        registry: &FormatRegistry,
        cpz6: &Cpz6Resources,
        sources: Vec<ResourceSource>,
    ) -> Result<Self> {
        let sources = sources
            .into_iter()
            .map(|source| {
                Ok(match source {
                    ResourceSource::Directory(path) => ScriptSource::Directory(path),
                    ResourceSource::Archive(path) => {
                        ScriptSource::Archive(cpz6.open(storage.as_ref(), registry, &path)?)
                    }
                    ResourceSource::KeyedArchive(path, words) => ScriptSource::Archive(
                        cpz6.open_keyed(storage.as_ref(), registry, &path, words)?,
                    ),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { storage, sources })
    }

    pub fn prepare_transfer(&self, request: ScriptRequest) -> Result<PreparedTransfer> {
        let native = path_string(request.name())?;
        let input = self
            .open_entry(&native)?
            .with_context(|| format!("CMVS script missing from registered sources: {native}"))?;
        request.prepare(input.as_ref())
    }

    pub(crate) fn open_entry(&self, name: &str) -> Result<Option<Arc<dyn AssetReader>>> {
        let relative = VfsPath::new(Mount::Asset, name)?;
        ensure!(
            relative.relative() == name && !name.contains('/'),
            "CMVS expected asset basename"
        );
        for source in &self.sources {
            match source {
                ScriptSource::Directory(directory) => {
                    let path =
                        VfsPath::new(Mount::Asset, &format!("{}/{name}", directory.relative()))?;
                    if let Some(path) =
                        resolve_ascii_case_insensitive(self.storage.as_ref(), &path)?
                    {
                        return Ok(Some(self.storage.open_asset(&path)?));
                    }
                }
                ScriptSource::Archive(archive) => {
                    if let Some(entry) = archive.entries().iter().find(|entry| {
                        entry
                            .path
                            .rsplit('/')
                            .next()
                            .is_some_and(|entry_name| entry_name.eq_ignore_ascii_case(name))
                    }) {
                        return Ok(Some(archive.open_entry(&entry.path)?));
                    }
                }
            }
        }
        Ok(None)
    }
}
