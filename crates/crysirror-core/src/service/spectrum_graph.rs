// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained source-audio analysis and bitmap visualization, without a device.
use crate::{Error, Result, image_processing::ImageResource, state::ClientId};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
#[derive(Clone, Debug)]
pub enum SpectrumGraphStyle {
    Fire,
    Bars {
        divisions: u32,
        thickness: u32,
        on: [u8; 4],
        off: [u8; 4],
        background: [u8; 4],
        peak: [u8; 4],
    },
}
#[derive(Clone, Debug)]
pub struct SpectrumGraphSpec {
    pub rect: [u32; 4],
    pub style: SpectrumGraphStyle,
}
impl SpectrumGraphSpec {
    pub fn validate(&self, size: [u32; 2]) -> Result<()> {
        let [x, y, w, h] = self.rect;
        if w == 0
            || h == 0
            || w > 8192
            || h > 8192
            || x.checked_add(w).is_none_or(|n| n > size[0])
            || y.checked_add(h).is_none_or(|n| n > size[1])
        {
            return Err(Error::Invalid("spectrum graph rectangle"));
        }
        if let SpectrumGraphStyle::Bars {
            divisions,
            thickness,
            ..
        } = self.style
        {
            if divisions == 0 || divisions > w || thickness == 0 || thickness > h / 2 {
                return Err(Error::Invalid("spectrum bar geometry"));
            }
        }
        Ok(())
    }
}
/// Explicit analysis history, committed only with a completed image. The
/// catalogue/voice/decoder remains host-owned; this contains no device handles.
#[derive(Clone, Debug, Default)]
pub struct SpectrumGraphState {
    pub cut: f32,
    pub levels: Vec<i32>,
    pub peaks: Vec<i32>,
    pub peak_age: Vec<i32>,
}
#[derive(Clone)]
pub struct SpectrumGraphReceipt {
    pub client: ClientId,
    pub image: Arc<dyn ImageResource>,
    pub state: SpectrumGraphState,
}
pub enum SpectrumGraphStatus {
    Pending,
    Ready(SpectrumGraphReceipt),
}
pub trait SpectrumGraphQuery: Send {
    fn poll(&mut self) -> Result<SpectrumGraphStatus>;
}
pub type SpectrumGraphJob = Box<dyn SpectrumGraphQuery>;
