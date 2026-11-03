// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original encoded image properties, distinct from normalized RGBA opacity.
use crate::{path::VfsPath, state::ClientId, Result};
use alloc::string::String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageEncodingInfo {
    pub format: String,
    pub size: [u32; 2],
    pub source_origin: Option<[i32; 2]>,
    /// Original logical color-pixel bits, not the decoded RGBA storage depth.
    pub color_bits: Option<u16>,
    /// None means unavailable. Plane presence is independent of whether every
    /// decoded alpha byte is255. An absent color plane can decode to black.
    pub has_color_plane: Option<bool>,
    pub has_alpha_plane: Option<bool>,
    pub color_row_bytes: Option<u32>,
    /// Original bytes per alpha sample; this is not a row-pitch estimate.
    pub alpha_sample_bytes: Option<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageEncodingInfoReceipt {
    pub client: ClientId,
    pub asset: VfsPath,
    pub fingerprint: [u8; 32],
    pub encoding: ImageEncodingInfo,
}
/// Ready confirms the encoded header and declared ranges. It does not confirm
/// compressed payload checksums or pixel decoding; image preparation is separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageEncodingInfoState {
    Pending,
    Ready(ImageEncodingInfoReceipt),
    /// Retain the actual engine error class, including NotFound. Adapters must
    /// not parse a formatted message to apply native missing-resource policy.
    Failed(crate::Error),
}
/// Nonblocking client capability. Results/errors are retained; dropping the
/// query cancels unfinished resource work. No bytes/device pointers enter saves.
pub trait ImageEncodingInfoQuery: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<ImageEncodingInfoState>;
}
