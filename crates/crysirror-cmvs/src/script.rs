// SPDX-License-Identifier: MPL-2.0
//! PS2A and PSS0 disk containers. The version and text bytes remain opaque to
//! the VM profile selector. See `NATIVE_ABI.md` for the file layout.

use anyhow::{Context, Result, ensure};
use crysirror_core::service::AssetReader;
use crysirror_resource::{LzssParams, lzss_decompress, read_all};

const HEADER_SIZE: usize = 48;
const MAX_PAYLOAD: usize = 64 * 1024 * 1024;
pub(crate) const MAX_PS2A_IMAGE: usize = HEADER_SIZE + MAX_PAYLOAD;
pub(crate) const PS2A_LZSS: LzssParams = LzssParams {
    mode: crysirror_resource::LzssMode::Bytewise,
    initial: 0x7df,
    position_bits: 11,
    size_bits: 5,
    min_match: 2,
    invert_length: false,
};

#[derive(Debug)]
pub struct Script {
    header: Header,
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptFormat {
    Ps2a,
    Pss0,
}

#[derive(Debug)]
struct Header {
    raw: [u8; HEADER_SIZE],
    format: ScriptFormat,
    code_start: usize,
    data_start: usize,
    text_start: usize,
    payload_len: usize,
}

impl Header {
    fn word(&self, index: usize) -> u32 {
        u32::from_le_bytes(self.raw.as_chunks::<4>().0[index])
    }

    fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= MAX_PS2A_IMAGE,
            "PS2A file exceeds byte limit"
        );
        let raw = *bytes
            .first_chunk::<HEADER_SIZE>()
            .context("truncated PS2A header")?;
        let (words, _) = raw.as_chunks::<4>();
        let format = match &words[0] {
            b"PS2A" => ScriptFormat::Ps2a,
            b"PSS0" => ScriptFormat::Pss0,
            _ => anyhow::bail!("expected PS2A or PSS0 signature"),
        };
        let word = |index: usize| u32::from_le_bytes(words[index]);
        ensure!(
            word(1) == HEADER_SIZE as u32,
            "unsupported PS2A header length"
        );
        let label_bytes = u64::from(word(4)) * 4;
        let code_bytes = word(5);
        let data_bytes = word(6);
        let text_bytes = word(7);
        let entry_pc = word(8);
        let total =
            label_bytes + u64::from(code_bytes) + u64::from(data_bytes) + u64::from(text_bytes);
        ensure!(
            total <= MAX_PAYLOAD as u64,
            "PS2A segments exceed byte limit"
        );
        ensure!(
            entry_pc.checked_add(2).is_some_and(|end| end <= code_bytes),
            "PS2A entry has no complete opcode in the code segment"
        );
        let code_start = label_bytes as usize;
        let data_start = code_start + code_bytes as usize;
        Ok(Self {
            raw,
            format,
            code_start,
            data_start,
            text_start: data_start + data_bytes as usize,
            payload_len: total as usize,
        })
    }
}

impl Script {
    /// Read and decode a script from its scoped storage provider. The caller
    /// owns the I/O task and only receives a script after validation succeeds.
    pub fn read(source: &dyn AssetReader) -> Result<Self> {
        Self::decode(read_all(source, MAX_PS2A_IMAGE as u64).context("read CMVS script")?)
    }

    pub fn decode(mut bytes: Vec<u8>) -> Result<Self> {
        let header = Header::parse(&bytes)?;
        let key = header.word(3);
        let packed_bytes = header.word(9) as usize;
        if packed_bytes == 0 {
            ensure!(
                bytes.len() - HEADER_SIZE == header.payload_len,
                "PS2A segment lengths disagree"
            );
        } else {
            ensure!(
                header.word(10) as usize == header.payload_len,
                "PS2A decoded size disagrees with segments"
            );
            ensure!(
                packed_bytes <= bytes.len() - HEADER_SIZE,
                "truncated PS2A packed payload"
            );
            bytes.truncate(HEADER_SIZE + packed_bytes);
        }
        bytes.drain(..HEADER_SIZE);
        if header.format == ScriptFormat::Ps2a {
            let xor = ((key >> 3) + (key >> 24)) as u8;
            let shift = (key >> 20) % 5 + 1;
            for value in &mut bytes {
                *value = (value.wrapping_sub(0x7c) ^ xor).rotate_right(shift);
            }
        }
        let payload = if packed_bytes == 0 {
            bytes
        } else {
            lzss_decompress(&bytes, header.payload_len, &PS2A_LZSS)
                .context("expand PS2A payload")?
        };
        Self::from_payload(header, payload)
    }

    // SATD-cmvs-x64-dispatch. The slot codec is isolated until the native
    // checkpoint restore can bind this image and all its resources atomically.
    #[cfg(any(test, feature = "runtime"))]
    pub(crate) fn decode_expanded(mut bytes: Vec<u8>) -> Result<Self> {
        let header = Header::parse(&bytes)?;
        ensure!(
            header.format == ScriptFormat::Ps2a,
            "expected PS2A save image"
        );
        bytes.drain(..HEADER_SIZE);
        Self::from_payload(header, bytes)
    }

    #[cfg(any(test, feature = "runtime"))]
    pub(crate) fn expanded_image(&self, data: &[u8], text: &[u8]) -> Result<Vec<u8>> {
        ensure!(
            data.len() == self.initial_data().len(),
            "CMVS saved main script data size"
        );
        ensure!(
            text.len() == self.text_bytes().len(),
            "CMVS saved main script text size"
        );
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(HEADER_SIZE + self.payload.len())?;
        bytes.extend_from_slice(&self.header.raw);
        bytes.extend_from_slice(&self.payload[..self.header.data_start]);
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(text);
        Ok(bytes)
    }

    fn from_payload(header: Header, payload: Vec<u8>) -> Result<Self> {
        ensure!(
            payload.len() == header.payload_len,
            "PS2A segment lengths disagree"
        );
        let code_bytes = header.word(5);
        let script = Self { header, payload };
        ensure!(
            script
                .labels()
                .all(|pc| pc.checked_add(2).is_some_and(|end| end <= code_bytes)),
            "PS2A label has no complete opcode in the code segment"
        );
        Ok(script)
    }

    pub(crate) fn format(&self) -> ScriptFormat {
        self.header.format
    }

    /// On-disk version word. VM profile selection belongs to the caller.
    pub fn version_raw(&self) -> u32 {
        self.header.word(2)
    }

    /// Byte offset from the start of `code()`.
    pub fn entry_pc(&self) -> u32 {
        self.header.word(8)
    }

    pub fn labels(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.payload[..self.header.code_start]
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(u32::from_le_bytes)
    }

    pub fn code(&self) -> &[u8] {
        &self.payload[self.header.code_start..self.header.data_start]
    }

    pub fn initial_data(&self) -> &[u8] {
        &self.payload[self.header.data_start..self.header.text_start]
    }

    pub fn text_bytes(&self) -> &[u8] {
        &self.payload[self.header.text_start..]
    }
}
