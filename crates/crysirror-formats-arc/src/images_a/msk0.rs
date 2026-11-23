// SPDX-License-Identifier: GPL-3.0-or-later
//! CMVS mask images. The original memory decoder at `0x140036240` reads
//! a tightly packed grayscale plane from the offset stored in the header.

use super::common::{gray, C};
use anyhow::{ensure, Result};
use crysirror_image::{rgba_len, ImageSurface};

pub(crate) fn decode(bytes: &[u8]) -> Result<ImageSurface> {
    let mut header = C::new(bytes);
    ensure!(header.take(4)? == b"MSK0", "invalid MSK0 signature");
    let offset = header.u32()? as usize;
    let width = header.u32()?;
    let height = header.u32()?;
    let pixels = rgba_len(width, height)? / 4;
    let plane = C::at(bytes, offset)?.take(pixels)?;
    gray(width, height, plane, false)
}
