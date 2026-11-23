// SPDX-License-Identifier: MPL-2.0
//! Unity 4.x serialized files (`level*`, `mainData`, `*.assets`; format 9)
//! read as engine-owned data. Player builds of this generation carry no type
//! trees, so each supported class has a fixed reader for the recorded Unity
//! version; unknown versions and classes fail visibly. Nothing here executes
//! game code or depends on a window, GPU or audio device.
//!
//! Layout of format 9 (big-endian header, then the declared endianness):
//! `u32 metadata size, u32 file size, u32 format (9), u32 data offset,
//! u8 endianness, 3 reserved`; metadata: NUL-terminated Unity version,
//! i32 target platform, i32 type-tree class count (0 in player builds),
//! i32 reserved, i32 object count, objects `{i32 path id, u32 data offset,
//! u32 size, i32 type id, i16 class id, i16 destroyed}`, i32 external count,
//! externals `{NUL-terminated asset path, 16-byte GUID, i32 type,
//! NUL-terminated path name}`.
#![forbid(unsafe_code)]
pub mod animator;
pub mod bundle;
pub mod events;
pub mod classes;
pub mod gltf;
pub mod managed;
pub mod memorypack;
pub mod msgpack;
pub mod managed_gen;
pub mod particles;
pub mod reader;
pub mod scene6;
pub mod texture;
pub mod timeline;
pub mod typetree;
pub mod ugui;
pub mod unity6;
pub mod navmesh;
pub mod world6;

use anyhow::{bail, ensure, Context, Result};
use crysirror_core::service::AssetReader;
use std::{collections::HashMap, sync::Arc};

pub use reader::Reader;

/// Upper bound of objects in one file (the largest CM3D2 file holds ~19k).
pub const MAX_OBJECTS: usize = 1 << 20;
/// Upper bound of one object's serialized bytes.
pub const MAX_OBJECT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_METADATA_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectInfo {
    pub path_id: i64,
    /// Absolute offset of the object's bytes in the file.
    pub offset: u64,
    pub size: u32,
    pub type_id: i32,
    pub class_id: i32,
    /// Index into [`SerializedFile::types`] (format ≥ 16).
    pub type_index: Option<usize>,
}

/// A serialized type entry (format ≥ 13): class, script and type tree.
#[derive(Clone, Debug, PartialEq)]
pub struct SerializedType {
    pub class_id: i32,
    pub script_type_index: i16,
    pub script_id: Option<[u8; 16]>,
    pub tree: Option<std::sync::Arc<typetree::TypeTree>>,
}

/// A reference to an object: file 0 is this file, `n` is external `n − 1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PPtr {
    pub file_id: i32,
    pub path_id: i64,
}
impl PPtr {
    pub fn is_null(&self) -> bool {
        self.path_id == 0
    }
}

pub struct SerializedFile {
    pub unity_version: String,
    pub platform: i32,
    /// Serialized file format (9 for Unity 4, 17 for Unity 5.6, 22 for Unity 2020+).
    pub format: u32,
    pub types: Vec<SerializedType>,
    /// Format ≥ 20 reference types (`[SerializeReference]` classes).
    pub ref_types: Vec<typetree::RefType>,
    pub objects: Vec<ObjectInfo>,
    /// External file path names, in reference order (`file_id − 1`).
    pub externals: Vec<String>,
    by_path: HashMap<i64, usize>,
    source: Arc<dyn AssetReader>,
}

fn read_exact(source: &dyn AssetReader, at: u64, out: &mut [u8]) -> Result<()> {
    let mut done = 0;
    while done < out.len() {
        let n = source
            .read_at(at + done as u64, &mut out[done..])
            .map_err(|e| anyhow::anyhow!("Unity file read: {e}"))?;
        ensure!(n > 0 && n <= out.len() - done, "truncated Unity serialized file");
        done += n;
    }
    Ok(())
}

impl SerializedFile {
    pub fn open(source: Arc<dyn AssetReader>) -> Result<Self> {
        let length = source.len().map_err(|e| anyhow::anyhow!("Unity file length: {e}"))?;
        let mut header = [0u8; 48];
        read_exact(source.as_ref(), 0, &mut header[..20])?;
        let be = |h: &[u8], at: usize| u32::from_be_bytes(h[at..at + 4].try_into().unwrap());
        let format = be(&header, 8);
        match format {
            9 => Self::open_9(source, length, &header),
            17 => {
                let (size, offset) = (be(&header, 0) as u64, be(&header, 12) as u64);
                ensure!(header[16] == 0 && be(&header, 4) as u64 == length, "invalid Unity 17 header");
                ensure!(size <= MAX_METADATA_BYTES && 20 + size <= offset && offset <= length, "Unity 17 metadata extent");
                let mut meta = vec![0; size as usize];
                read_exact(source.as_ref(), 20, &mut meta)?;
                Self::metadata(source, length, offset, &meta, format)
            }
            22 => {
                read_exact(source.as_ref(), 0, &mut header)?;
                let be64 = |at: usize| u64::from_be_bytes(header[at..at + 8].try_into().unwrap());
                let (metadata_size, file_size, data_offset) = (be(&header, 20), be64(24), be64(32));
                ensure!(header[16] == 0, "big-endian Unity data is unsupported");
                ensure!(file_size == length, "Unity file size mismatch");
                ensure!(u64::from(metadata_size) <= MAX_METADATA_BYTES && 48 + u64::from(metadata_size) <= length, "Unity metadata size");
                ensure!(data_offset <= length, "Unity data offset");
                let mut meta = vec![0u8; metadata_size as usize];
                read_exact(source.as_ref(), 48, &mut meta)?;
                Self::metadata(source, length, data_offset, &meta, format)
            }
            other => bail!("unsupported Unity serialized format {other}"),
        }
    }

    /// Format 22 metadata (Unity 2020.1+): type tree types, 64-bit object
    /// offsets, script types, externals, reference types, user information.
    fn metadata(source: Arc<dyn AssetReader>, length: u64, data_offset: u64, meta: &[u8], format: u32) -> Result<Self> {
        let mut r = Reader::new(meta);
        let unity_version = r.cstring()?;
        let platform = r.i32()?;
        let type_trees = r.bool()?;
        let type_count = r.count(23)?;
        let mut types = Vec::with_capacity(type_count);
        for _ in 0..type_count {
            types.push(read_type(&mut r, type_trees, format)?);
        }
        let count = r.count(20)?;
        ensure!(count <= MAX_OBJECTS, "Unity object limit");
        let mut objects = Vec::with_capacity(count);
        let mut by_path = HashMap::with_capacity(count);
        for i in 0..count {
            // Object entries align to 4 bytes from the metadata start
            // (the metadata itself starts 4-aligned at 48).
            r.align()?;
            let path_id = r.i64()?;
            let relative = if format == 17 { u64::from(r.u32()?) } else { u64::try_from(r.i64()?).context("negative Unity object offset")? };
            let offset = data_offset.checked_add(relative).context("Unity object offset overflow")?;
            let size = r.u32()?;
            let type_index = r.i32()?;
            let ti = usize::try_from(type_index).ok().filter(|t| *t < types.len()).with_context(|| format!("Unity object {path_id} type index {type_index}"))?;
            ensure!(u64::from(size) <= MAX_OBJECT_BYTES && offset <= length && u64::from(size) <= length - offset, "Unity object {path_id} outside the file");
            ensure!(by_path.insert(path_id, i).is_none(), "duplicate Unity path id {path_id}");
            objects.push(ObjectInfo { path_id, offset, size, type_id: type_index, class_id: types[ti].class_id, type_index: Some(ti) });
        }
        let scripts = r.count(12)?;
        for _ in 0..scripts {
            let _file = r.i32()?;
            r.align()?;
            let _id = r.i64()?;
        }
        let externals_count = r.count(22)?;
        let mut externals = Vec::with_capacity(externals_count);
        for _ in 0..externals_count {
            let _asset_path = r.cstring()?;
            r.take(16)?;
            let _kind = r.i32()?;
            externals.push(r.cstring()?);
        }
        let ref_count = if format == 17 { 0 } else { r.count(23)? };
        let mut ref_types = Vec::with_capacity(ref_count);
        for _ in 0..ref_count {
            let (t, names) = read_ref_type(&mut r, type_trees)?;
            if let (Some(tree), Some((class, namespace, assembly))) = (t.tree, names) {
                ref_types.push(typetree::RefType { class, namespace, assembly, tree });
            }
        }
        Ok(Self { unity_version, platform, format, types, ref_types, objects, externals, by_path, source })
    }

    fn open_9(source: Arc<dyn AssetReader>, length: u64, header: &[u8]) -> Result<Self> {
        let be = |at: usize| u32::from_be_bytes(header[at..at + 4].try_into().unwrap());
        let (metadata_size, file_size, format, data_offset) = (be(0), be(4), be(8), be(12));
        ensure!(format == 9, "unsupported Unity serialized format {format}");
        ensure!(u64::from(file_size) == length, "Unity file size mismatch");
        ensure!(u64::from(metadata_size) <= MAX_METADATA_BYTES, "Unity metadata size");
        ensure!(header[16] == 0, "big-endian Unity data is unsupported");
        ensure!(u64::from(data_offset) <= length, "Unity data offset");
        // Player files of this generation place the metadata after the header.
        ensure!(20 + u64::from(metadata_size) <= length, "Unity metadata outside the file");
        let mut meta = vec![0u8; metadata_size as usize];
        read_exact(source.as_ref(), 20, &mut meta)?;
        let mut r = Reader::new(&meta);
        let unity_version = r.cstring()?;
        ensure!(unity_version.starts_with("4."), "unsupported Unity version {unity_version}");
        let platform = r.i32()?;
        let types = r.i32()?;
        ensure!(types == 0, "Unity type trees are not supported by this reader");
        let _reserved = r.i32()?;
        let count = r.count(20)?;
        ensure!(count <= MAX_OBJECTS, "Unity object limit");
        let mut objects = Vec::with_capacity(count);
        let mut by_path = HashMap::with_capacity(count);
        for i in 0..count {
            let path_id = i64::from(r.i32()?);
            let offset = u64::from(r.u32()?) + u64::from(data_offset);
            let size = r.u32()?;
            let type_id = r.i32()?;
            let class_id = i32::from(r.i16()?);
            let _destroyed = r.i16()?;
            ensure!(
                u64::from(size) <= MAX_OBJECT_BYTES && offset <= length && u64::from(size) <= length - offset,
                "Unity object {path_id} outside the file"
            );
            ensure!(by_path.insert(path_id, i).is_none(), "duplicate Unity path id {path_id}");
            objects.push(ObjectInfo { path_id, offset, size, type_id, class_id, type_index: None });
        }
        let externals_count = r.count(22)?;
        let mut externals = Vec::with_capacity(externals_count);
        for _ in 0..externals_count {
            let _asset_path = r.cstring()?;
            r.take(16)?;
            let _kind = r.i32()?;
            externals.push(r.cstring()?);
        }
        Ok(Self { unity_version, platform, format: 9, types: Vec::new(), ref_types: Vec::new(), objects, externals, by_path, source })
    }

    /// The type tree of an object (format ≥ 13 files with type trees).
    pub fn tree(&self, info: &ObjectInfo) -> Option<&typetree::TypeTree> {
        info.type_index.and_then(|i| self.types.get(i)).and_then(|t| t.tree.as_deref())
    }

    /// Decode an object under its type tree.
    pub fn decode(&self, info: &ObjectInfo) -> Result<managed::Fields> {
        let tree = self.tree(info).with_context(|| format!("Unity object {} has no type tree", info.path_id))?;
        tree.decode_with(&self.bytes(info)?, &self.ref_types)
    }

    /// Decode an object under its own type tree, else the registry's tree
    /// for its class or script (player files are built without trees).
    pub fn decode_using(&self, info: &ObjectInfo, registry: &TreeRegistry) -> Result<managed::Fields> {
        if let Some(tree) = self.tree(info) {
            return tree.decode_with(&self.bytes(info)?, &self.ref_types);
        }
        let ty = info.type_index.and_then(|i| self.types.get(i));
        let tree = registry
            .lookup(info.class_id, ty.and_then(|t| t.script_id))
            .with_context(|| format!("Unity object {} (class {}) has no type tree in the file or the registry", info.path_id, info.class_id))?;
        tree.decode_with(&self.bytes(info)?, &self.ref_types)
    }

    pub fn object(&self, path_id: i64) -> Option<&ObjectInfo> {
        self.by_path.get(&path_id).map(|i| &self.objects[*i])
    }

    /// The object's serialized bytes.
    pub fn bytes(&self, info: &ObjectInfo) -> Result<Vec<u8>> {
        let mut out = vec![0u8; info.size as usize];
        read_exact(self.source.as_ref(), info.offset, &mut out)?;
        Ok(out)
    }

    pub fn bytes_of(&self, path_id: i64) -> Result<Vec<u8>> {
        let info = *self.object(path_id).with_context(|| format!("Unity object {path_id} missing"))?;
        self.bytes(&info)
    }

    /// Name of the external file a non-local pointer refers to.
    pub fn external(&self, file_id: i32) -> Result<&str> {
        if file_id <= 0 {
            bail!("Unity pointer {file_id} is not external");
        }
        self.externals
            .get(file_id as usize - 1)
            .map(String::as_str)
            .with_context(|| format!("Unity external file {file_id} missing"))
    }
}

/// Type trees gathered from files that carry them (asset bundles of the
/// same player), for files built without: engine classes by class id,
/// MonoBehaviours by the 16-byte script id of their serialized type.
#[derive(Default)]
pub struct TreeRegistry {
    by_class: HashMap<i32, Arc<typetree::TypeTree>>,
    by_script: HashMap<[u8; 16], Arc<typetree::TypeTree>>,
}

impl TreeRegistry {
    /// Record the trees of a file (the first tree of a key wins).
    pub fn add(&mut self, file: &SerializedFile) {
        for t in &file.types {
            let Some(tree) = &t.tree else { continue };
            match (t.class_id, t.script_id) {
                (class_id::MONO_BEHAVIOUR, Some(id)) => {
                    self.by_script.entry(id).or_insert_with(|| tree.clone());
                }
                (class_id::MONO_BEHAVIOUR, None) => {}
                (c, _) => {
                    self.by_class.entry(c).or_insert_with(|| tree.clone());
                }
            }
        }
    }

    pub fn lookup(&self, class: i32, script: Option<[u8; 16]>) -> Option<&typetree::TypeTree> {
        if class == class_id::MONO_BEHAVIOUR {
            return script.and_then(|s| self.by_script.get(&s)).map(|t| t.as_ref());
        }
        self.by_class.get(&class).map(|t| t.as_ref())
    }

    /// Every recorded tree.
    pub fn trees(&self) -> impl Iterator<Item = &typetree::TypeTree> {
        self.by_class.values().chain(self.by_script.values()).map(|t| t.as_ref())
    }

    /// Record a tree for a MonoBehaviour script id.
    pub fn insert_script(&mut self, script: [u8; 16], tree: typetree::TypeTree) {
        self.by_script.insert(script, Arc::new(tree));
    }

    pub fn len(&self) -> (usize, usize) {
        (self.by_class.len(), self.by_script.len())
    }
}

/// A format-22 `SerializedType` (reference types carry class names).
fn read_type(r: &mut Reader, type_trees: bool, format: u32) -> Result<SerializedType> {
    let class_id = r.i32()?;
    let _stripped = r.u8()?;
    let script_type_index = r.i16()?;
    let script_id = if class_id == class_id::MONO_BEHAVIOUR { Some(r.take(16)?.try_into()?) } else { None };
    let _old_type_hash = r.take(16)?;
    let mut tree = None;
    if type_trees {
        tree = Some(std::sync::Arc::new(typetree::TypeTree::read_blob(r, format)?));
        if format >= 21 {
            let deps = r.count(4)?;
            r.take(deps * 4)?;
        }
    }
    Ok(SerializedType { class_id, script_type_index, script_id, tree })
}

/// A format-22 reference `SerializedType` with its class, namespace and
/// assembly names.
fn read_ref_type(r: &mut Reader, type_trees: bool) -> Result<(SerializedType, Option<(String, String, String)>)> {
    let class_id = r.i32()?;
    let _stripped = r.u8()?;
    let script_type_index = r.i16()?;
    let script_id = if script_type_index >= 0 { Some(r.take(16)?.try_into()?) } else { None };
    let _old_type_hash = r.take(16)?;
    if !type_trees {
        return Ok((SerializedType { class_id, script_type_index, script_id, tree: None }, None));
    }
    let tree = std::sync::Arc::new(typetree::TypeTree::read_blob(r, 22)?);
    let names = (r.cstring()?, r.cstring()?, r.cstring()?);
    Ok((SerializedType { class_id, script_type_index, script_id, tree: Some(tree) }, Some(names)))
}

/// Unity class ids used by the readers.
pub mod class_id {
    pub const GAME_OBJECT: i32 = 1;
    pub const TRANSFORM: i32 = 4;
    pub const CAMERA: i32 = 20;
    pub const MATERIAL: i32 = 21;
    pub const MESH_RENDERER: i32 = 23;
    pub const TEXTURE_2D: i32 = 28;
    pub const MESH_FILTER: i32 = 33;
    pub const MESH: i32 = 43;
    pub const SHADER: i32 = 48;
    pub const AUDIO_CLIP: i32 = 83;
    pub const RENDER_SETTINGS: i32 = 104;
    pub const LIGHT: i32 = 108;
    pub const ANIMATION: i32 = 111;
    pub const MONO_BEHAVIOUR: i32 = 114;
    pub const MONO_SCRIPT: i32 = 115;
    pub const FONT: i32 = 128;
    pub const SKINNED_MESH_RENDERER: i32 = 137;
    pub const ASSET_BUNDLE: i32 = 142;
    pub const BUILD_SETTINGS: i32 = 141;
    pub const RESOURCE_MANAGER: i32 = 147;
    pub const TEXT_ASSET: i32 = 49;
    pub const SPRITE: i32 = 213;
    pub const RECT_TRANSFORM: i32 = 224;
}
