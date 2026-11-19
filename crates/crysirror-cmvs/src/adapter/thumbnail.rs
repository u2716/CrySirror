// SPDX-License-Identifier: MPL-2.0
//! Retained native save thumbnail. The presenter owns frame readback and
//! the CPU worker owns BMP encoding before the owner commits 0x22b8.

use crate::vm::CmvsVm;
use anyhow::{ensure, Result};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState, ImageResource},
    service::{AssetReader, HostServices},
    state::ClientId,
};
use crysirror_image::bmp_encode::{box_average_bmp_len, encode_box_average_bmp};
use crysirror_resource::DecodeContext;
use crysirror_tasks::{Task, TaskPermit, TaskPool};
use serde_json::{json, Value};
use std::sync::Arc;

struct Bitmap {
    size: [u32; 2],
    bytes: Arc<dyn AssetReader>,
    fingerprint: u64,
}

enum Capture {
    Frame {
        job: ImageJob,
        permit: TaskPermit,
        size: [u32; 2],
        length: u64,
    },
    Encode(Task<Bitmap>),
}

pub(super) struct Thumbnail {
    context: DecodeContext,
    pending: Option<Capture>,
    bitmap: Option<Bitmap>,
    error: Option<String>,
}

impl Thumbnail {
    pub fn new(client: ClientId) -> Self {
        Self {
            context: DecodeContext::new(client),
            pending: None,
            bitmap: None,
            error: None,
        }
    }

    /// The retained BMP written into the next save slot.
    pub fn saved_bitmap(&self) -> Option<Arc<dyn AssetReader>> {
        self.bitmap.as_ref().map(|bitmap| bitmap.bytes.clone())
    }

    pub fn clear(&mut self) {
        self.pending = None;
        self.bitmap = None;
        self.error = None;
    }

    pub fn begin(
        &mut self,
        host: &mut dyn HostServices,
        pool: &TaskPool,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        ensure!(self.pending.is_none(), "CMVS thumbnail already pending");
        let size = vm.settings().thumbnail_size;
        let request = (|| -> Result<Capture> {
            let length = box_average_bmp_len(size)? as u64;
            let permit = pool.reserve()?;
            let job = host.begin_frame_capture(self.context.client)?;
            Ok(Capture::Frame {
                job,
                permit,
                size,
                length,
            })
        })();
        match request {
            Ok(capture) => self.pending = Some(capture),
            Err(error) => self.finish(vm, Err(error))?,
        }
        Ok(self.pending.is_some())
    }

    pub fn poll(&mut self, vm: &mut CmvsVm) -> Result<bool> {
        let Some(capture) = self.pending.take() else {
            return Ok(false);
        };
        match capture {
            Capture::Frame {
                mut job,
                permit,
                size,
                length,
            } => match job.poll() {
                Ok(ImageProcessState::Pending) => {
                    self.pending = Some(Capture::Frame {
                        job,
                        permit,
                        size,
                        length,
                    })
                }
                Ok(ImageProcessState::Ready(frame)) => {
                    let context = self.context.clone();
                    match permit.spawn(move |cancel| {
                        let context = context.with_cancellation(cancel);
                        encode(&context, frame.as_ref(), size, length)
                    }) {
                        Ok(task) => self.pending = Some(Capture::Encode(task)),
                        Err(error) => self.finish(vm, Err(error))?,
                    }
                }
                Ok(ImageProcessState::Failed(error)) => {
                    self.finish(vm, Err(anyhow::anyhow!(error)))?
                }
                Err(error) => self.finish(vm, Err(error.into()))?,
            },
            Capture::Encode(mut task) => match task.poll_owner() {
                Ok(None) => self.pending = Some(Capture::Encode(task)),
                Ok(Some(bitmap)) => self.finish(vm, Ok(bitmap))?,
                Err(error) => self.finish(vm, Err(error))?,
            },
        }
        Ok(self.pending.is_some())
    }

    fn finish(&mut self, vm: &mut CmvsVm, result: Result<Bitmap>) -> Result<()> {
        vm.owner_arguments::<0>(0x22b8)?;
        vm.complete_owner_call(0, None)?;
        match result {
            Ok(bitmap) => {
                self.bitmap = Some(bitmap);
                self.error = None;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
        Ok(())
    }

    pub fn diagnostics(&self) -> Value {
        json!({
            "pending": self.pending.is_some(),
            "bitmap": self.bitmap.as_ref().map(|bitmap| json!({
                "size": bitmap.size, "bytes": bitmap.bytes.len().ok(),
                "fingerprint": format!("{:016x}", bitmap.fingerprint),
            })),
            "error": self.error,
        })
    }
}

fn encode(
    context: &DecodeContext,
    frame: &dyn ImageResource,
    size: [u32; 2],
    length: u64,
) -> Result<Bitmap> {
    ensure!(
        frame.client() == context.client,
        "CMVS foreign thumbnail frame"
    );
    let mut fingerprint = 0;
    // Account for the temporary encoder Vec alongside the new immutable
    // reader and the old resident BMP until the owner commits.
    let bytes = context.with_temporary_bytes(length, || {
        context.decode_with(length, || {
            let data = encode_box_average_bmp(frame.size(), &frame.rgba(), size, &|| {
                Ok(context.check_cancelled()?)
            })?;
            fingerprint = data.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, &byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
            });
            context.check_cancelled()?;
            Ok(data)
        })
    })?;
    Ok(Bitmap {
        size,
        bytes,
        fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{VmProfile, VmYield};
    use crysirror_core::{
        image_processing::{ImageProcess, RgbaImage},
        path::VfsPath,
        service::{FileSystem, MemoryAssetReader},
        state::Command,
        Error,
    };

    struct Host {
        responses: std::collections::VecDeque<crysirror_core::Result<ImageProcessState>>,
    }

    struct Presented(Option<ImageProcessState>);

    impl ImageProcess for Presented {
        fn poll(&mut self) -> crysirror_core::Result<ImageProcessState> {
            Ok(self.0.take().map_or_else(
                || ImageProcessState::Failed("already delivered".into()),
                |state| state,
            ))
        }
    }

    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::NotFound)
        }

        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            panic!("thumbnail must be retained in memory")
        }
    }

    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Ok(())
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn begin_frame_capture(&mut self, _: ClientId) -> crysirror_core::Result<ImageJob> {
            Ok(Box::new(Presented(Some(
                self.responses
                    .pop_front()
                    .expect("authored presenter result")?,
            ))))
        }
    }

    fn machine() -> CmvsVm {
        let mut code = Vec::new();
        for word in [99u32, 1, 2] {
            code.extend_from_slice(&[0x40, 0x04, 4, 0]);
            code.extend_from_slice(&word.to_le_bytes());
        }
        for opcode in [0x22b0u16, 0x22b8, 0x22b8, 0x22b8, 0x2000] {
            code.extend_from_slice(&opcode.to_le_bytes());
        }
        let mut bytes = b"PS2A".to_vec();
        for word in [
            48,
            1,
            0,
            0,
            code.len() as u32,
            0,
            0,
            0,
            0,
            code.len() as u32,
            0,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend(
            code.into_iter()
                .map(|byte| byte.rotate_left(1).wrapping_add(0x7c)),
        );
        CmvsVm::prepare(VmProfile::Pc20160201X64, &MemoryAssetReader(bytes.into())).unwrap()
    }

    #[test]
    fn capture_pauses_vm_and_failed_requests_preserve_the_committed_bitmap() {
        let client = ClientId(4);
        let frame: Arc<dyn ImageResource> = Arc::new(
            RgbaImage::new(client, [2, 1], [9, 19, 27, 2, 3, 5, 7, 0].as_slice().into()).unwrap(),
        );
        let mut host = Host {
            responses: [
                Ok(ImageProcessState::Ready(frame)),
                Err(Error::Unsupported("capture unavailable")),
                Ok(ImageProcessState::Failed("readback failed".into())),
            ]
            .into(),
        };
        let pool = TaskPool::new("thumbnail-test", 1, 2).unwrap();
        let mut vm = machine();
        let mut thumbnail = Thumbnail::new(client);
        let next = |vm: &mut CmvsVm, pc| {
            assert_eq!(
                vm.run_slice(&mut 64).unwrap(),
                VmYield::SystemCall { pc, opcode: 0x22b8 }
            );
            assert_eq!(vm.parameter(4), Some(99));
        };

        next(&mut vm, 26);
        assert!(thumbnail.begin(&mut host, &pool, &mut vm).unwrap());
        assert!(thumbnail.poll(&mut vm).unwrap());
        assert_eq!(vm.pc(), 26);
        assert!(thumbnail.bitmap.is_none());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while thumbnail.poll(&mut vm).unwrap() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        let before = thumbnail.bitmap.as_ref().unwrap().bytes.clone();
        let bytes = crysirror_resource::read_all(before.as_ref(), 62).unwrap();
        assert_eq!(&bytes[54..], [27, 19, 9, 7, 5, 3, 0, 0]);

        next(&mut vm, 28);
        assert!(!thumbnail.begin(&mut host, &pool, &mut vm).unwrap());
        assert!(thumbnail
            .error
            .as_ref()
            .unwrap()
            .contains("capture unavailable"));
        assert!(Arc::ptr_eq(
            &before,
            &thumbnail.bitmap.as_ref().unwrap().bytes
        ));
        next(&mut vm, 30);
        assert!(thumbnail.begin(&mut host, &pool, &mut vm).unwrap());
        assert!(!thumbnail.poll(&mut vm).unwrap());
        assert_eq!(thumbnail.error.as_deref(), Some("readback failed"));
        assert!(Arc::ptr_eq(
            &before,
            &thumbnail.bitmap.as_ref().unwrap().bytes
        ));
        thumbnail.clear();
        assert!(thumbnail.bitmap.is_none());
        assert!(!thumbnail.poll(&mut vm).unwrap());
    }
}
