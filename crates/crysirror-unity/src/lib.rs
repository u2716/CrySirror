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
pub mod classes;
pub mod gltf;
pub mod managed;
pub mod reader;
pub mod texture;

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
        let mut header = [0u8; 20];
        read_exact(source.as_ref(), 0, &mut header)?;
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
                u64::from(size) <= MAX_OBJECT_BYTES && offset + u64::from(size) <= length,
                "Unity object {path_id} outside the file"
            );
            ensure!(by_path.insert(path_id, i).is_none(), "duplicate Unity path id {path_id}");
            objects.push(ObjectInfo { path_id, offset, size, type_id, class_id });
        }
        let externals_count = r.count(22)?;
        let mut externals = Vec::with_capacity(externals_count);
        for _ in 0..externals_count {
            let _asset_path = r.cstring()?;
            r.take(16)?;
            let _kind = r.i32()?;
            externals.push(r.cstring()?);
        }
        Ok(Self { unity_version, platform, objects, externals, by_path, source })
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
    pub const BUILD_SETTINGS: i32 = 141;
    pub const RESOURCE_MANAGER: i32 = 147;
    pub const TEXT_ASSET: i32 = 49;
    pub const SPRITE: i32 = 213;
    pub const RECT_TRANSFORM: i32 = 224;
}
