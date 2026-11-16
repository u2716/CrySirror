// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Immutable grow-or-retain byte storage for worker-owned packed image planes.
//! A larger allocation explicitly zeroes previously undefined contents; a
//! smaller replacement preserves capacity and the unwritten linear tail.
use anyhow::{Result, ensure};
use std::sync::Arc;
#[derive(Clone, Debug, Default)]
pub struct LinearByteStorage {
    bytes: Arc<[u8]>,
}
impl LinearByteStorage {
    pub fn capacity(&self) -> usize {
        self.bytes.len()
    }
    pub fn replace_prefix(
        &self,
        requested: usize,
        prefix: Option<&[u8]>,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        ensure!(
            requested <= crate::MAX_DECODED_BYTES && self.bytes.len() <= crate::MAX_DECODED_BYTES,
            "linear byte storage capacity budget"
        );
        let capacity = requested.max(self.bytes.len());
        ensure!(
            prefix.is_none_or(|p| p.len() <= capacity),
            "linear byte storage prefix capacity"
        );
        check()?;
        let mut bytes = vec![0; capacity];
        if requested <= self.bytes.len() {
            for (dst, src) in bytes.chunks_mut(65536).zip(self.bytes.chunks(65536)) {
                check()?;
                dst.copy_from_slice(src);
            }
        }
        if let Some(prefix) = prefix {
            for (dst, src) in bytes[..prefix.len()]
                .chunks_mut(65536)
                .zip(prefix.chunks(65536))
            {
                check()?;
                dst.copy_from_slice(src);
            }
        }
        check()?;
        Ok(Self {
            bytes: bytes.into(),
        })
    }
    pub fn prefix(&self, length: usize, check: &dyn Fn() -> Result<()>) -> Result<Arc<[u8]>> {
        ensure!(length <= self.bytes.len(), "linear byte storage read span");
        check()?;
        if length == self.bytes.len() {
            return Ok(self.bytes.clone());
        }
        let mut bytes = Vec::with_capacity(length);
        for chunk in self.bytes[..length].chunks(65536) {
            check()?;
            bytes.extend_from_slice(chunk);
        }
        check()?;
        Ok(bytes.into())
    }
}
