// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Preserve original DWQ palette indices before any RGB expansion. Unlike
//! FULLANIME's linear reader, ordinary CPicture data uses BMP row padding or
//! the packed predictor stream while retaining encoded scanline order.
use super::*;
use crysirror_image::indexed_plane::IndexedPlane;
use std::sync::Arc;
impl Dwq {
    pub fn indexed_plane(data: &[u8], cx: &ImageDecodeContext<'_>) -> Result<IndexedPlane> {
        Ok(Self::indexed_plane_and_depth(data, cx)?.0)
    }
    /// Returns the original BMP sample depth separately from preserved byte
    /// indices; caller allocation policy must not infer it from expanded RGB.
    pub fn indexed_plane_and_depth(
        data: &[u8],
        cx: &ImageDecodeContext<'_>,
    ) -> Result<(IndexedPlane, u16)> {
        let header = header(data)?;
        let size = Self::dimensions(data)?;
        let body = data.get(64..).context("indexed DWQ body absent")?;
        let length = match header.pack {
            0 => body.len(),
            1 if data.starts_with(b"IF PACKTYPE==") => body.len(),
            2 | 3 => u32at(data, 32)? as usize,
            _ => bail!(
                "indexed DWQ codec profile {} requires preserved source indices",
                header.pack
            ),
        };
        let input = body
            .get(..length)
            .context("indexed DWQ color payload span")?;
        let bitmap = bitmap(input, size, matches!(header.pack, 1 | 3), cx)?;
        ensure!(
            bitmap.bpp == 8 || !matches!(header.pack, 1 | 3),
            "indexed DWQ packed non8-bit predictor profile"
        );
        // Native copies all256 stack entries even for a short biClrUsed.
        // For non8-bit plain BMP it still copies each row's first WIDTH bytes
        // into the indexed buffer, but never initializes rgbTable. Define that
        // palette (and a short8-bit palette tail) as black, not stack data.
        let mut palette = [[0; 3]; 256];
        for (to, from) in palette.iter_mut().zip(&bitmap.palette) {
            *to = *from;
        }
        let mut indices = Vec::with_capacity(size[0] as usize * size[1] as usize);
        for row in bitmap.bytes.chunks_exact(bitmap.stride) {
            cx.check_cancelled()?;
            indices.extend_from_slice(&row[..size[0] as usize]);
        }
        let plane = IndexedPlane {
            size,
            palette,
            indices: Arc::from(indices),
        };
        plane.validate()?;
        Ok((plane, bitmap.bpp))
    }
}
