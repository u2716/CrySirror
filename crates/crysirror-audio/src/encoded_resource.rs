//! Immutable encoded-resource preparation, independent of filesystem assets.
use crate::channels::ChannelAudio;
use anyhow::Result;
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{AssetReader, AudioAsset, ClientStorage, EncodedAudio, MemoryAssetReader},
};
use std::sync::Arc;
struct Storage {
    path: VfsPath,
    bytes: Arc<[u8]>,
}
impl ClientStorage for Storage {
    fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        if path == &self.path {
            Ok(self.bytes.to_vec())
        } else {
            Err(Error::NotFound)
        }
    }
    fn open_asset(&self, path: &VfsPath) -> crysirror_core::Result<Arc<dyn AssetReader>> {
        if path == &self.path {
            Ok(Arc::new(MemoryAssetReader(self.bytes.clone())))
        } else {
            Err(Error::NotFound)
        }
    }
    fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::ReadOnly)
    }
    fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
        Err(Error::ReadOnly)
    }
    fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
        Err(Error::Unsupported("encoded-resource directory"))
    }
}
impl ChannelAudio {
    pub fn begin_encoded_resource_preparation(
        &mut self,
        client: u32,
        name: &str,
        encoded: EncodedAudio,
        measure_frames: bool,
    ) -> Result<u64> {
        let path = VfsPath::new(Mount::Asset, name)?;
        let asset = AudioAsset {
            path: path.clone(),
            frame_limit: encoded.frame_limit,
        };
        let storage = Arc::new(Storage {
            path,
            bytes: encoded.bytes,
        });
        self.begin_resource_preparation(client, asset, measure_frames, storage)
    }
}
