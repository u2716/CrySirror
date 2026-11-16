// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded PNG source rows without palette expansion, gamma conversion or tRNS
//! alpha insertion. Consumers keep explicit source-index/channel semantics;
//! the shared PNG codec owns filtering, DEFLATE and Adam7 reconstruction.
use anyhow::{Context, Result, ensure};
use std::{
    io::{self, BufRead, Cursor, Read, Seek, SeekFrom},
    sync::Arc,
};
#[derive(Clone, Debug)]
pub struct PngRows {
    pub size: [u32; 2],
    pub bit_depth: u8,
    /// PNG IHDR color type:0 grey,2 RGB,3 indexed,4 grey+alpha,6 RGBA.
    pub color_type: u8,
    pub channels: u8,
    pub stride: usize,
    pub palette: Option<Vec<[u8; 3]>>,
    pub bytes: Arc<[u8]>,
}
struct Checked<'a> {
    cursor: Cursor<&'a [u8]>,
    check: &'a dyn Fn() -> Result<()>,
}
impl Checked<'_> {
    fn check(&self) -> io::Result<()> {
        (self.check)().map_err(|e| io::Error::other(format!("{e:#}")))
    }
}
impl Read for Checked<'_> {
    fn read(&mut self, to: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        self.cursor.read(to)
    }
}
impl BufRead for Checked<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.check()?;
        self.cursor.fill_buf()
    }
    fn consume(&mut self, n: usize) {
        self.cursor.consume(n)
    }
}
impl Seek for Checked<'_> {
    fn seek(&mut self, at: SeekFrom) -> io::Result<u64> {
        self.check()?;
        self.cursor.seek(at)
    }
}
impl PngRows {
    pub fn validate(&self) -> Result<()> {
        let rgba = crate::rgba_len(self.size[0], self.size[1])?;
        ensure!(
            matches!(self.bit_depth, 1 | 2 | 4 | 8 | 16),
            "PNG row sample depth"
        );
        let channels = match self.color_type {
            0 | 3 => 1,
            2 => 3,
            4 => 2,
            6 => 4,
            _ => anyhow::bail!("PNG row color type"),
        };
        ensure!(self.channels == channels, "PNG row channel count");
        ensure!(
            match self.color_type {
                0 => true,
                3 => self.bit_depth <= 8,
                _ => matches!(self.bit_depth, 8 | 16),
            },
            "PNG row color/depth combination"
        );
        ensure!(
            self.color_type != 3 || self.palette.is_some(),
            "PNG indexed rows require PLTE"
        );
        let stride =
            (u64::from(self.size[0]) * u64::from(channels) * u64::from(self.bit_depth)).div_ceil(8);
        let length = stride
            .checked_mul(u64::from(self.size[1]))
            .context("PNG row size overflow")?;
        ensure!(
            stride == self.stride as u64
                && length == self.bytes.len() as u64
                && length <= crate::MAX_DECODED_BYTES as u64
                && rgba > 0,
            "PNG row extent/budget"
        );
        if let Some(palette) = &self.palette {
            ensure!(
                !palette.is_empty() && palette.len() <= 256,
                "PNG row palette count"
            );
        }
        Ok(())
    }
    /// Build an indexed plane from the first width bytes of each decoded row.
    /// This is an explicit byte-stream interpretation, not automatic RGB to
    /// palette quantisation; supplied colors define every resulting index.
    pub fn prefix_index_plane(
        &self,
        palette: [[u8; 3]; 256],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<crate::indexed_plane::IndexedPlane> {
        self.validate()?;
        check()?;
        ensure!(
            self.bit_depth == 8 && self.stride >= self.size[0] as usize,
            "PNG byte-prefix indexed plane profile"
        );
        let width = self.size[0] as usize;
        let mut indices = Vec::with_capacity(width * self.size[1] as usize);
        for row in self.bytes.chunks_exact(self.stride) {
            check()?;
            indices.extend_from_slice(&row[..width]);
        }
        let plane = crate::indexed_plane::IndexedPlane {
            size: self.size,
            palette,
            indices: indices.into(),
        };
        plane.validate()?;
        Ok(plane)
    }
    pub fn decode(data: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        check()?;
        ensure!(
            data.len() <= crate::MAX_DECODED_BYTES,
            "PNG encoded row budget"
        );
        let mut decoder = png::Decoder::new_with_limits(
            Checked {
                cursor: Cursor::new(data),
                check,
            },
            png::Limits {
                bytes: crate::MAX_DECODED_BYTES,
            },
        );
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        let size = [info.width, info.height];
        crate::rgba_len(size[0], size[1])?;
        let palette = info
            .palette
            .as_ref()
            .map(|bytes| -> Result<Vec<[u8; 3]>> {
                ensure!(
                    bytes.len().is_multiple_of(3) && !bytes.is_empty() && bytes.len() <= 768,
                    "PNG PLTE bytes"
                );
                Ok(bytes
                    .chunks_exact(3)
                    .map(|rgb| rgb.try_into().unwrap())
                    .collect())
            })
            .transpose()?;
        let capacity = reader
            .output_buffer_size()
            .context("PNG output size overflow")?;
        ensure!(
            capacity <= crate::MAX_DECODED_BYTES,
            "PNG source rows exceed memory budget"
        );
        let mut bytes = vec![0; capacity];
        let frame = reader.next_frame(&mut bytes)?;
        ensure!(
            [frame.width, frame.height] == size,
            "PNG first-frame subrectangle profile"
        );
        bytes.truncate(frame.buffer_size());
        let color_type = match frame.color_type {
            png::ColorType::Grayscale => 0,
            png::ColorType::Rgb => 2,
            png::ColorType::Indexed => 3,
            png::ColorType::GrayscaleAlpha => 4,
            png::ColorType::Rgba => 6,
        };
        let rows = Self {
            size,
            bit_depth: frame.bit_depth as u8,
            color_type,
            channels: frame.color_type.samples() as u8,
            stride: frame.line_size,
            palette,
            bytes: bytes.into(),
        };
        check()?;
        rows.validate()?;
        Ok(rows)
    }
}
