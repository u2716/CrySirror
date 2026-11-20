// SPDX-License-Identifier: MPL-2.0
//! CSV2 and CSP1 gameplay slot envelopes. Each ABI selects its own checkpoint
//! transform, expansion limit and saved main-script format.

use crate::script::{MAX_PS2A_IMAGE, PS2A_LZSS, Script, ScriptFormat};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::AssetReader;
use crysirror_resource::{
    lzss::{lzss_compress_bytewise, lzss_decompress},
    read_exact_at,
};
use std::ops::Range;

pub(crate) mod csp1;

pub(crate) const HEADER_BYTES: usize = 600;
const CHECKPOINT_LIMIT: usize = 32 * 1024 * 1024;
const THUMBNAIL_LIMIT: usize = 64 * 1024 * 1024;
const CHECKPOINT_KEY: &[u8] = b"DAME_SAVE_IMAGE";

pub(crate) struct Slot {
    pub(crate) header: [u8; HEADER_BYTES],
    pub(crate) checkpoint: Vec<u8>,
    pub(crate) thumbnail: Option<Vec<u8>>,
    pub(crate) main: Script,
}

/// Native slot prefix: signature, local time and comment, with the thumbnail flag.
#[cfg(feature = "runtime")]
pub(crate) fn slot_header(stamp: &str, title: &[u8], thumbnail: bool) -> [u8; HEADER_BYTES] {
    let mut header = [0; HEADER_BYTES];
    header[..4].copy_from_slice(b"CSV2");
    put_word(&mut header, 4, HEADER_BYTES as u32);
    let label = stamp.as_bytes().iter().chain(title).take(255).copied();
    for (at, byte) in label.enumerate() {
        header[16 + at] = byte;
    }
    header[548] = u8::from(thumbnail);
    header
}

fn word(header: &[u8; HEADER_BYTES], at: usize) -> u32 {
    u32::from_le_bytes(header[at..at + 4].try_into().unwrap())
}

fn put_word(header: &mut [u8; HEADER_BYTES], at: usize, value: u32) {
    header[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy)]
pub(crate) enum SlotFormat { Csv2, Csp1 }

impl SlotFormat {
    fn signature(self) -> &'static [u8; 4] {
        match self { Self::Csv2 => b"CSV2", Self::Csp1 => b"CSP1" }
    }
    fn key(self) -> &'static [u8] {
        match self { Self::Csv2 => CHECKPOINT_KEY, Self::Csp1 => b"SAVE_IMAGE_ARC" }
    }
    fn script(self) -> ScriptFormat {
        match self { Self::Csv2 => ScriptFormat::Ps2a, Self::Csp1 => ScriptFormat::Pss0 }
    }
    fn limit(self) -> usize {
        match self { Self::Csv2 => CHECKPOINT_LIMIT, Self::Csp1 => 0x300000 }
    }
    fn check(self, header: &[u8; HEADER_BYTES]) -> Result<()> {
        ensure!(&header[..4] == self.signature() && word(header, 4) == HEADER_BYTES as u32,
            "unsupported CMVS whole-slot header");
        Ok(())
    }
}

pub(crate) fn pack(
    header: [u8; HEADER_BYTES], checkpoint: &[u8], thumbnail: Option<&[u8]>,
    main_image: &[u8], check_cancel: impl Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    pack_native(SlotFormat::Csv2, header, checkpoint, thumbnail, main_image, check_cancel)
}

pub(crate) fn pack_native(
    format: SlotFormat, mut header: [u8; HEADER_BYTES], checkpoint: &[u8],
    thumbnail: Option<&[u8]>, main_image: &[u8], check_cancel: impl Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    format.check(&header)?;
    ensure!(
        (1..=format.limit()).contains(&checkpoint.len()),
        "CMVS checkpoint size"
    );
    ensure!(
        (48..=MAX_PS2A_IMAGE).contains(&main_image.len()),
        "CMVS main script image size"
    );
    if matches!(format, SlotFormat::Csp1) { csp1::validate(checkpoint)?; }
    let thumbnail = thumbnail.unwrap_or_default();
    ensure!(thumbnail.len() <= THUMBNAIL_LIMIT, "CMVS thumbnail size");
    let has_thumbnail = header[548..550] != [0, 0];
    let mut file = Vec::new();
    file.try_reserve_exact(HEADER_BYTES)?;
    file.resize(HEADER_BYTES, 0);
    let mut checkpoint_end = 0;
    for (index, bytes) in [checkpoint, thumbnail, main_image].into_iter().enumerate() {
        let start = file.len();
        if index != 1 || has_thumbnail {
            lzss_compress_bytewise(bytes, &PS2A_LZSS, &mut file, &check_cancel)?;
        }
        if index == 0 {
            checkpoint_end = file.len();
        }
        put_word(&mut header, 560 + index * 4, (file.len() - start) as u32);
        put_word(&mut header, 580 + index * 4, bytes.len() as u32);
    }
    let mut sum = 0u16;
    for (byte, &key) in file[HEADER_BYTES..checkpoint_end]
        .iter_mut()
        .zip(format.key().iter().cycle())
    {
        *byte = byte.wrapping_add(0x80) ^ key;
        sum = sum.wrapping_add(u16::from(*byte));
    }
    check_cancel()?;
    put_word(&mut header, 540, file.len() as u32);
    header[546..548].copy_from_slice(&1u16.to_le_bytes());
    file[..HEADER_BYTES].copy_from_slice(&header);
    file.try_reserve_exact(2)?;
    file.extend_from_slice(&sum.to_le_bytes());
    Ok(file)
}

struct Section {
    packed: Range<u64>,
    expanded: usize,
}

fn section(
    header: &[u8; HEADER_BYTES],
    index: usize,
    start: u64,
    limit: usize,
    optional: bool,
) -> Result<Section> {
    let packed = word(header, 560 + 4 * index);
    let expanded = word(header, 580 + 4 * index) as usize;
    if optional && packed == 0 {
        return Ok(Section {
            packed: start..start,
            expanded: 0,
        });
    }
    ensure!((1..=limit).contains(&expanded), "CMVS slot expanded range");
    ensure!(
        packed != 0 && packed as usize <= expanded + expanded.div_ceil(8),
        "CMVS slot packed range"
    );
    Ok(Section {
        packed: start..start + u64::from(packed),
        expanded,
    })
}

impl Section {
    fn expand(
        self,
        source: &dyn AssetReader,
        format: SlotFormat,
        checksum: Option<u16>,
        check_cancel: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<u8>> {
        let length = (self.packed.end - self.packed.start) as usize;
        let mut data = Vec::new();
        data.try_reserve_exact(length)?;
        data.resize(length, 0);
        const CHUNK: usize = 64 * 1024;
        for (index, chunk) in data.chunks_mut(CHUNK).enumerate() {
            check_cancel()?;
            read_exact_at(source, self.packed.start + (index * CHUNK) as u64, chunk)?;
        }
        if let Some(expected) = checksum {
            let mut actual = 0u16;
            for (byte, &key) in data.iter_mut().zip(format.key().iter().cycle()) {
                actual = actual.wrapping_add(u16::from(*byte));
                *byte = (*byte ^ key).wrapping_add(0x80);
            }
            ensure!(expected == actual, "CMVS slot checkpoint checksum");
        }
        check_cancel()?;
        let expanded = lzss_decompress(&data, self.expanded, &PS2A_LZSS)?;
        check_cancel()?;
        Ok(expanded)
    }
}

pub(crate) fn unpack(source: &dyn AssetReader, check_cancel: impl Fn() -> Result<()>) -> Result<Slot> {
    unpack_native(SlotFormat::Csv2, source, check_cancel)
}

pub(crate) fn unpack_native(format: SlotFormat, source: &dyn AssetReader, check_cancel: impl Fn() -> Result<()>) -> Result<Slot> {
    check_cancel()?;
    let len = source.len()?;
    let mut header = [0; HEADER_BYTES];
    read_exact_at(source, 0, &mut header)?;
    format.check(&header)?;
    let checkpoint = section(&header, 0, HEADER_BYTES as u64, format.limit(), false)?;
    let thumbnail = section(&header, 1, checkpoint.packed.end, THUMBNAIL_LIMIT, true)?;
    let main = section(&header, 2, thumbnail.packed.end, MAX_PS2A_IMAGE, false)?;
    let trailer = u64::from(word(&header, 540));
    let checksum = if trailer + 2 == len {
        ensure!(trailer == main.packed.end, "CMVS slot checksum offset");
        let mut bytes = [0; 2];
        read_exact_at(source, trailer, &mut bytes)?;
        Some(u16::from_le_bytes(bytes))
    } else {
        // The original checks for a trailer by comparing offset + 2 to EOF.
        // The older untransformed form ends at the last packed section.
        ensure!(len == main.packed.end, "CMVS slot packed ranges disagree");
        None
    };
    let checkpoint = checkpoint
        .expand(source, format, checksum, &check_cancel)
        .context("expand CMVS slot checkpoint")?;
    if matches!(format, SlotFormat::Csp1) { csp1::validate(&checkpoint)?; }
    let thumbnail = if thumbnail.packed.is_empty() {
        None
    } else {
        Some(
            thumbnail
                .expand(source, format, None, &check_cancel)
                .context("expand CMVS slot thumbnail")?,
        )
    };
    let main = Script::decode_saved(main.expand(source, format, None, &check_cancel)?, format.script())
        .context("expand CMVS saved main script")?;
    Ok(Slot {
        header,
        checkpoint,
        thumbnail,
        main,
    })
}

#[cfg(test)]
mod tests;
