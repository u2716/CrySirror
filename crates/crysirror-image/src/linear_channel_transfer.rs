// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Linear channel-buffer transfer between immutable image shapes. Width and
//! height do not reinterpret the selected channel's contiguous pixel stream.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearChannelTransfer {
    pub source_channel: u8,
    pub destination_channel: u8,
    pub source_offset: u64,
    pub destination_offset: u64,
    pub length: u64,
}
impl LinearChannelTransfer {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.source_channel < 4
                && self.destination_channel < 4
                && self.source_offset <= MAX_EFFECT_PIXELS
                && self.destination_offset <= MAX_EFFECT_PIXELS
                && self.length <= MAX_EFFECT_PIXELS,
            "linear channel transfer bounds"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        destination.validate()?;
        source.validate()?;
        let source_len = u64::from(source.width) * u64::from(source.height);
        let destination_len = u64::from(destination.width) * u64::from(destination.height);
        ensure!(
            source_len <= MAX_EFFECT_PIXELS
                && destination_len <= MAX_EFFECT_PIXELS
                && self.source_offset + self.length <= source_len
                && self.destination_offset + self.length <= destination_len,
            "linear channel transfer image span"
        );
        check()?;
        let mut output = destination.clone();
        for i in 0..self.length as usize {
            if i % 4096 == 0 {
                check()?;
            }
            let from = (self.source_offset as usize + i) * 4 + usize::from(self.source_channel);
            let to =
                (self.destination_offset as usize + i) * 4 + usize::from(self.destination_channel);
            output.rgba[to] = source.rgba[from];
        }
        check()?;
        Ok(output)
    }
}
