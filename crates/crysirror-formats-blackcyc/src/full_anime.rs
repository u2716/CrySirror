// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original FULLANIME's DWQ-wrapped linear8 BMP profile. This is separate from
//! ordinary DWQ display orientation, stride, masks and compressed-image decode.
use anyhow::{Context, Result, ensure};
use crysirror_image::indexed_plane::IndexedPlane;
use std::sync::Arc;

pub fn linear_indexed_dwq(data: &[u8], size: [u32; 2]) -> Result<IndexedPlane> {
    let required = crysirror_image::rgba_len(size[0], size[1])? / 4;
    ensure!(
        required <= crysirror_image::effects::MAX_EFFECT_PIXELS as usize,
        "FULLANIME plane budget"
    );
    ensure!(data.len() >= 118, "FULLANIME DWQ/BMP headers truncated");
    // Native fread always consumes a14-byte file header and40-byte DIB after
    // the64-byte DWQ wrapper. It ignores bfOffBits, bitmap height/orientation,
    // source width and row padding, reading exactly canvasWidth bytes per row.
    let bits = crate::u16at(data, 92)?;
    ensure!(
        bits == 8,
        "FULLANIME non-indexed BMP had an uninitialized native palette; unsupported source profile"
    );
    let declared = crate::u32at(data, 110)?;
    let entries = if declared == 0 {
        256
    } else {
        declared as usize
    };
    ensure!(
        entries <= 256,
        "FULLANIME palette would overflow native256 entries"
    );
    let start = 118usize
        .checked_add(
            entries
                .checked_mul(4)
                .context("FULLANIME palette size overflow")?,
        )
        .context("FULLANIME pixel offset overflow")?;
    let end = start
        .checked_add(required)
        .context("FULLANIME index size overflow")?;
    let palette_bytes = data
        .get(118..start)
        .context("FULLANIME palette truncated")?;
    let indices = data
        .get(start..end)
        .context("FULLANIME linear index rows truncated")?;
    // Original entries beyond biClrUsed were uninitialized stack words.
    // Define unused entries as black instead of reading nondeterministic data.
    let mut palette = [[0; 3]; 256];
    for (to, from) in palette.iter_mut().zip(palette_bytes.chunks_exact(4)) {
        *to = [from[2], from[1], from[0]];
    }
    let plane = IndexedPlane {
        size,
        palette,
        indices: Arc::from(indices),
    };
    plane.validate()?;
    Ok(plane)
}
