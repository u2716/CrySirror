// SPDX-License-Identifier: MPL-2.0
//! SQZ1 independently deflated, top-down BGRA frames.
use crate::{ImageSurface, rgba_len, swap_red_blue};
use anyhow::{Result, ensure};
use crysirror_core::service::AssetReader;

#[derive(Clone, Copy)]
pub struct Sequence {
    pub count: u32,
    pub size: [u32; 2],
    pub rate: i32,
}

fn u32le(b: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(b[offset..offset + 4].try_into().unwrap())
}

fn read(reader: &dyn AssetReader, mut at: u64, mut into: &mut [u8]) -> Result<()> {
    while !into.is_empty() {
        let got = reader.read_at(at, into)?;
        ensure!(got > 0 && got <= into.len(), "short SQZ read");
        into = &mut into[got..];
        at += got as u64;
    }
    Ok(())
}

impl Sequence {
    pub fn header(b: &[u8], length: u64) -> Result<Self> {
        ensure!(b.len() >= 20 && &b[..4] == b"SQZ1", "SQZ1 header");
        let count = u32le(b, 4);
        ensure!(
            count > 0 && 20 + 8 * u64::from(count) <= length,
            "SQZ frame table"
        );
        let size = [u32le(b, 8), u32le(b, 12)];
        rgba_len(size[0], size[1])?;
        Ok(Self {
            count,
            size,
            rate: u32le(b, 16) as i32,
        })
    }
    pub fn open(reader: &dyn AssetReader) -> Result<Self> {
        let mut b = [0; 20];
        read(reader, 0, &mut b)?;
        Self::header(&b, reader.len()?)
    }
    fn range(self, b: &[u8], length: u64) -> Result<(usize, usize)> {
        let start = u32le(b, 0) as usize;
        let size = u32le(b, 4) as usize;
        ensure!(
            start as u64 >= 20 + 8 * u64::from(self.count)
                && size > 0
                && size <= crate::MAX_DECODED_BYTES
                && start as u64 + size as u64 <= length,
            "SQZ frame extent"
        );
        Ok((start, size))
    }
    fn inflate(self, compressed: &[u8]) -> Result<ImageSurface> {
        let [width, height] = self.size;
        let expected = rgba_len(width, height)?;
        let mut rgba =
            miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(compressed, expected)
                .map_err(|_| anyhow::anyhow!("invalid SQZ frame stream"))?;
        ensure!(rgba.len() == expected, "SQZ decoded size");
        swap_red_blue(&mut rgba)?;
        Ok(ImageSurface {
            width,
            height,
            rgba,
        })
    }
    pub fn frame(self, reader: &dyn AssetReader, index: u32) -> Result<ImageSurface> {
        ensure!(index < self.count, "SQZ frame ordinal");
        let mut entry = [0; 8];
        read(reader, 20 + u64::from(index) * 8, &mut entry)?;
        let (at, size) = self.range(&entry, reader.len()?)?;
        let mut compressed = vec![0; size];
        read(reader, at as u64, &mut compressed)?;
        self.inflate(&compressed)
    }
    pub fn first(bytes: &[u8]) -> Result<ImageSurface> {
        let seq = Self::header(bytes, bytes.len() as u64)?;
        let (start, size) = seq.range(&bytes[20..28], bytes.len() as u64)?;
        seq.inflate(&bytes[start..start + size])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;
    use miniz_oxide::deflate::compress_to_vec_zlib;

    fn fixture() -> Vec<u8> {
        let frames = [[3, 2, 1, 4], [7, 6, 5, 8]].map(|pixel| compress_to_vec_zlib(&pixel, 1));
        let mut bytes = b"SQZ1".to_vec();
        for field in [2_u32, 1, 1, u32::MAX] {
            bytes.extend(field.to_le_bytes());
        }
        let mut offset = 36;
        for frame in &frames {
            bytes.extend(u32::to_le_bytes(offset));
            bytes.extend((frame.len() as u32).to_le_bytes());
            offset += frame.len() as u32;
        }
        for frame in frames {
            bytes.extend(frame);
        }
        bytes
    }

    #[test]
    fn manual_frames_use_absolute_offsets_and_preserve_alpha() {
        let data = fixture();
        assert_eq!(Sequence::first(&data).unwrap().rgba, [1, 2, 3, 4]);
        let reader = MemoryAssetReader(data.into());
        let sequence = Sequence::open(&reader).unwrap();
        assert_eq!(sequence.rate, -1);
        assert_eq!(sequence.frame(&reader, 1).unwrap().rgba, [5, 6, 7, 8]);
        assert!(sequence.frame(&reader, 2).is_err());
    }

    #[test]
    fn rejects_payload_overlapping_the_index_and_short_reads() {
        let mut bytes = fixture();
        assert!(Sequence::first(&bytes[..27]).is_err());
        bytes[20..24].copy_from_slice(&32_u32.to_le_bytes());
        assert!(Sequence::first(&bytes).is_err());
        let mut bytes = fixture();
        bytes.truncate(bytes.len() - 1);
        let reader = MemoryAssetReader(bytes.into());
        assert!(Sequence::open(&reader).unwrap().frame(&reader, 1).is_err());
    }
}
