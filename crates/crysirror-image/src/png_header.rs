// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded PNG header/PLTE inspection without decoding IDAT scanlines. Native
//! resource owners can order palette observations independently of row workers.
use anyhow::{Result, ensure};
use std::io::{self, BufRead, Cursor, Read, Seek, SeekFrom};
#[derive(Clone, Debug)]
pub struct PngHeader {
    pub size: [u32; 2],
    pub bit_depth: u8,
    pub color_type: u8,
    pub palette: Option<Vec<[u8; 3]>>,
}
struct Checked<'a> {
    reader: Cursor<&'a [u8]>,
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
        self.reader.read(to)
    }
}
impl BufRead for Checked<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.check()?;
        self.reader.fill_buf()
    }
    fn consume(&mut self, n: usize) {
        self.reader.consume(n)
    }
}
impl Seek for Checked<'_> {
    fn seek(&mut self, at: SeekFrom) -> io::Result<u64> {
        self.check()?;
        self.reader.seek(at)
    }
}
impl PngHeader {
    /// The PNG codec validates format/CRC through read_info. Full row decode
    /// still owns IDAT/DEFLATE validation; this is not a successful-image receipt.
    pub fn read(data: &[u8], check: &dyn Fn() -> Result<()>) -> Result<Self> {
        check()?;
        ensure!(
            data.len() <= crate::MAX_DECODED_BYTES,
            "PNG header encoded budget"
        );
        let mut decoder = png::Decoder::new_with_limits(
            Checked {
                reader: Cursor::new(data),
                check,
            },
            png::Limits {
                bytes: crate::MAX_DECODED_BYTES,
            },
        );
        decoder.set_transformations(png::Transformations::IDENTITY);
        let reader = decoder.read_info()?;
        let info = reader.info();
        let size = [info.width, info.height];
        crate::rgba_len(size[0], size[1])?;
        let palette = info
            .palette
            .as_ref()
            .map(|bytes| -> Result<Vec<[u8; 3]>> {
                ensure!(
                    !bytes.is_empty() && bytes.len() <= 768 && bytes.len() % 3 == 0,
                    "PNG header palette span"
                );
                Ok(bytes
                    .chunks_exact(3)
                    .map(|p| p.try_into().unwrap())
                    .collect())
            })
            .transpose()?;
        let color_type = match info.color_type {
            png::ColorType::Grayscale => 0,
            png::ColorType::Rgb => 2,
            png::ColorType::Indexed => 3,
            png::ColorType::GrayscaleAlpha => 4,
            png::ColorType::Rgba => 6,
        };
        ensure!(
            color_type != 3 || palette.is_some(),
            "PNG header indexed palette absent"
        );
        check()?;
        Ok(Self {
            size,
            bit_depth: info.bit_depth as u8,
            color_type,
            palette,
        })
    }
}
