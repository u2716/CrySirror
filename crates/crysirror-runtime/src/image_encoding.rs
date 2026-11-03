// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Shared scoped encoded-image metadata queries. No VM or RGBA inference.
use crysirror_core::{
    image_encoding::*,
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
    Error, Result,
};
use crysirror_image::DecoderRegistry;
use crysirror_tasks::{Task, TaskPool};
use std::sync::{atomic::AtomicUsize, Arc};

struct Query {
    owner: ClientId,
    task: Option<Task<ImageEncodingInfoReceipt>>,
    state: ImageEncodingInfoState,
}
impl ImageEncodingInfoQuery for Query {
    fn client(&self) -> ClientId {
        self.owner
    }
    fn poll(&mut self) -> Result<ImageEncodingInfoState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(receipt)) => {
                    self.task = None;
                    self.state = ImageEncodingInfoState::Ready(receipt)
                }
                Err(error) => {
                    self.task = None;
                    self.state = ImageEncodingInfoState::Failed(
                        error
                            .downcast_ref::<Error>()
                            .cloned()
                            .unwrap_or_else(|| Error::Io(error.to_string())),
                    )
                }
            }
        }
        Ok(self.state.clone())
    }
}
pub(crate) fn begin(
    owner: ClientId,
    asset: &str,
    storage: Arc<dyn ClientStorage>,
    decoders: DecoderRegistry,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<Box<dyn ImageEncodingInfoQuery>> {
    let path = VfsPath::new(Mount::Asset, asset)?;
    let permit = pool
        .reserve()
        .map_err(|error| Error::Io(error.to_string()))?;
    let task = permit
        .spawn(move |cancel| {
            cancel.check()?;
            let reader = storage.open_asset(&path)?;
            let length = reader.len()?;
            anyhow::ensure!(
                length <= crysirror_image::MAX_DECODED_BYTES as u64,
                "encoded metadata byte limit"
            );
            let _charge = crate::image_processing::Charge::new(memory, length as usize)?;
            // Read only the length whose memory was charged. A mutable provider
            // cannot increase the allocation by reporting a second larger len.
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(length as usize)?;
            bytes.resize(length as usize, 0);
            for (index, chunk) in bytes.chunks_mut(64 * 1024).enumerate() {
                cancel.check()?;
                crysirror_resource::read_exact_at(
                    reader.as_ref(),
                    (index * 64 * 1024) as u64,
                    chunk,
                )?;
            }
            anyhow::ensure!(
                reader.len()? == length,
                "image metadata source length changed during query"
            );
            cancel.check()?;
            let info = decoders
                .encoding_info_with(path.relative(), &bytes, None, None, None)?
                .ok_or(Error::Unsupported(
                    "selected codec original encoded image metadata",
                ))?;
            let fingerprint = *blake3::hash(&bytes).as_bytes();
            cancel.check()?;
            Ok(ImageEncodingInfoReceipt {
                client: owner,
                asset: path,
                fingerprint,
                encoding: info,
            })
        })
        .map_err(|error| Error::Io(error.to_string()))?;
    Ok(Box::new(Query {
        owner,
        task: Some(task),
        state: ImageEncodingInfoState::Pending,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::AssetReader;
    use std::{
        sync::atomic::Ordering,
        time::{Duration, Instant},
    };
    struct ChangingReader {
        calls: AtomicUsize,
        maximum: AtomicUsize,
    }
    impl AssetReader for ChangingReader {
        fn len(&self) -> Result<u64> {
            Ok(if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
                4
            } else {
                8
            })
        }
        fn read_at(&self, offset: u64, output: &mut [u8]) -> Result<usize> {
            self.maximum.fetch_max(output.len(), Ordering::Relaxed);
            let bytes = b"QNT\0xxxx";
            let count = output
                .len()
                .min(bytes.len().saturating_sub(offset as usize));
            output[..count].copy_from_slice(&bytes[offset as usize..offset as usize + count]);
            Ok(count)
        }
    }
    struct Storage(Arc<ChangingReader>);
    impl ClientStorage for Storage {
        fn read(&self, _: &VfsPath) -> Result<Vec<u8>> {
            Err(Error::Unsupported("authored full read"))
        }
        fn open_asset(&self, _: &VfsPath) -> Result<Arc<dyn AssetReader>> {
            Ok(self.0.clone())
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> Result<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> Result<()> {
            Err(Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> Result<Vec<String>> {
            Ok(vec![])
        }
    }
    #[test]
    fn mutable_provider_cannot_grow_allocation_past_its_reserved_length() {
        let reader = Arc::new(ChangingReader {
            calls: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
        });
        let memory = Arc::new(AtomicUsize::new(0));
        let mut query = begin(
            ClientId(1),
            "x",
            Arc::new(Storage(reader.clone())),
            DecoderRegistry::default(),
            TaskPool::new("header budget", 1, 1).unwrap(),
            memory.clone(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match query.poll().unwrap() {
                ImageEncodingInfoState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now()
                }
                ImageEncodingInfoState::Failed(error) => {
                    assert!(error.to_string().contains("length changed"));
                    break;
                }
                ImageEncodingInfoState::Ready(_) => panic!("mutable provider changed length"),
            }
        }
        assert_eq!(reader.maximum.load(Ordering::Relaxed), 4);
        assert_eq!(memory.load(Ordering::Acquire), 0);
    }
}
