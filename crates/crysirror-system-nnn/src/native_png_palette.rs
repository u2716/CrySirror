// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPngLoader's native palette history is session policy, separate from the
//! shared PNG decoder. Commit observe/load actions in original CPicture load
//! order; concurrent resource completion is not a license to reorder them.
use anyhow::Result;
use crysirror_image::{indexed_plane::IndexedPlane, png_header::PngHeader, png_rows::PngRows};

#[derive(Clone, Debug)]
pub(crate) struct PngPalette {
    colors: [[u8; 3]; 256],
}
impl Default for PngPalette {
    fn default() -> Self {
        Self {
            colors: std::array::from_fn(|i| [i as u8; 3]),
        }
    }
}
#[derive(Clone)]
pub(crate) enum IndexedCopy {
    Pixels(IndexedPlane),
    /// CPicture::LoadDWQ had already copied an uninitialized RGB stack table
    /// before this non8-bit GetPic8AndPalette returned without writing pixels.
    /// Keep existing index storage and replace that undefined palette by black.
    RetainedPixelsBlackPalette,
}
impl PngPalette {
    /// Every PNG LoadFile changes this static loader, even for an ordinary
    ///32-bit CPicture. PLTE only participates for PNG_COLOR_TYPE_PALETTE.
    pub(crate) fn observe(&mut self, rows: &PngRows) -> Result<()> {
        rows.validate()?;
        if rows.color_type == 3 {
            let palette = rows
                .palette
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("NNN indexed PNG palette absent"))?;
            self.colors = [[0; 3]; 256];
            self.colors[..palette.len()].copy_from_slice(palette);
        }
        Ok(())
    }
    /// Ordinary RGB CPicture loads affect CPngLoader's shared palette too.
    /// Only the header is inspected here; image success still belongs to the
    /// real shared decode job, so this does not acknowledge a failed picture.
    pub(crate) fn observe_header(&mut self, header: &PngHeader) -> Result<()> {
        if header.color_type == 3 {
            let colors = header
                .palette
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("NNN palette PNG header lacks PLTE"))?;
            anyhow::ensure!(
                !colors.is_empty() && colors.len() <= 256,
                "NNN palette PNG header span"
            );
            self.colors = [[0; 3]; 256];
            self.colors[..colors.len()].copy_from_slice(colors);
        }
        Ok(())
    }
    pub(crate) fn copy8(
        &self,
        rows: &PngRows,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<IndexedCopy> {
        rows.validate()?;
        if rows.bit_depth != 8 {
            return Ok(IndexedCopy::RetainedPixelsBlackPalette);
        }
        // GetPic8AndPalette asks for a byte prefix, not channel extraction.
        // The shared image worker performs that reusable row operation.
        let plane = rows.prefix_index_plane(self.colors, check)?;
        Ok(IndexedCopy::Pixels(plane))
    }
}
