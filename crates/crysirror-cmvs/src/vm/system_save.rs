// SPDX-License-Identifier: MPL-2.0
//! CMVS CSS1 mode 1. Prepare a detached read-only import.

use super::evaluate::State;
use anyhow::{ensure, Context, Result};
use crysirror_core::{path::VfsPath, service::ClientStorage, Error};
use crysirror_resource::{
    lzss::lzss_compress_bytewise, lzss_decompress, read_all, LzssMode, LzssParams,
};

const PAYLOAD_OFFSET: usize = 360;
const BITS_END: usize = 32512;
const INTEGERS_END: usize = 40704;
const FLOATS_END: usize = 44800;
const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_EXPANDED: usize = 1024 * 1024;
const CHECK_SEQUENCE: &[u8] = b"YUKO_KAWAI_PURPLE_SYSTEM_SAVE_CHECK";
const CSS1_LZSS: LzssParams = LzssParams {
    mode: LzssMode::Bytewise,
    initial: 976,
    position_bits: 10,
    size_bits: 6,
    min_match: 2,
    invert_length: false,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSaveRequest {
    pub(super) session: u64,
    pub(super) generation: u64,
    pub(super) pc: usize,
    pub(super) path: VfsPath,
    pub(super) game_id: Option<u32>,
}

pub struct PreparedSystemSave {
    pub(super) request: SystemSaveRequest,
    pub(super) contents: Option<CssState>,
}

impl SystemSaveRequest {
    /// Worker-side read and validation. Only an explicit NotFound permits the
    /// original fresh-session path.
    pub fn prepare(self, storage: &dyn ClientStorage) -> Result<PreparedSystemSave> {
        let contents = match storage.open_read(&self.path) {
            Err(Error::NotFound) => None,
            source => {
                let source = source.context("open CMVS system save")?;
                let bytes = read_all(source.as_ref(), MAX_FILE)?;
                Some(CssState::decode(bytes, self.game_id)?)
            }
        };
        Ok(PreparedSystemSave {
            request: self,
            contents,
        })
    }
}

pub(super) struct CssState {
    expanded: Vec<u8>,
    strings: Vec<Vec<u8>>,
    lists: [Vec<Vec<u8>>; 2],
}

impl CssState {
    fn decode(mut bytes: Vec<u8>, game_id: Option<u32>) -> Result<Self> {
        ensure!(bytes.len() >= PAYLOAD_OFFSET, "truncated CSS1 header");
        let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let dword = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        ensure!(
            &bytes[..4] == b"CSS1" && dword(4) == 104 && word(76) == 1,
            "unsupported CSS1 header or encoding mode"
        );
        let stored_game = dword(68);
        ensure!(
            stored_game == 0 || game_id == Some(stored_game),
            "CSS1 game identifier mismatch"
        );
        let id_counts = [dword(88) as usize, dword(84) as usize];
        ensure!(
            id_counts.iter().all(|&count| count <= 4096),
            "CSS1 name count limit"
        );
        let declared = dword(92) as usize;
        ensure!(
            (FLOATS_END + 64 + id_counts[0] + id_counts[1]..=MAX_EXPANDED).contains(&declared),
            "CSS1 expanded size outside the supported form"
        );
        let end = PAYLOAD_OFFSET
            .checked_add(dword(96) as usize)
            .context("CSS1 payload overflow")?;
        ensure!(
            end == dword(100) as usize && end.checked_add(2) == Some(bytes.len()),
            "CSS1 packed extents disagree"
        );
        let actual = bytes[PAYLOAD_OFFSET..end]
            .iter()
            .fold(0u16, |sum, &byte| sum.wrapping_add(u16::from(byte)));
        ensure!(actual == word(end), "CSS1 packed checksum mismatch");
        let shift = u32::from((word(82) >> ((word(78) >> 1) & 7)) & 3) + 2;
        let (subtract, xor_bias) = (bytes[78], bytes[80]);
        let packed = &mut bytes[PAYLOAD_OFFSET..end];
        for (index, byte) in packed.iter_mut().enumerate() {
            let key = xor_bias.wrapping_add(CHECK_SEQUENCE[index % CHECK_SEQUENCE.len()]);
            *byte = (byte.rotate_right(shift).wrapping_sub(subtract) ^ key).wrapping_sub(64);
        }
        let expanded = lzss_decompress(packed, declared, &CSS1_LZSS)?;
        let mut strings = Vec::with_capacity(64);
        let mut lists = [Vec::new(), Vec::new()];
        let mut tail = &expanded[FLOATS_END..];
        for _ in 0..64 {
            strings.push(read_name(&mut tail, 1023)?);
        }
        for (entries, count) in lists.iter_mut().zip(id_counts) {
            entries.reserve(count);
            for _ in 0..count {
                entries.push(read_name(&mut tail, MAX_EXPANDED)?);
            }
        }
        ensure!(tail.is_empty(), "CSS1 trailing expanded data");
        Ok(Self {
            expanded,
            strings,
            lists,
        })
    }

    pub(super) fn apply(
        self,
        state: &State,
    ) -> std::result::Result<(State, [Vec<Vec<u8>>; 2]), &'static str> {
        let mut candidate = state.clone();
        candidate.restore_bits(2048, &self.expanded[..BITS_END])?;
        for (index, bytes) in self.expanded[BITS_END..INTEGERS_END]
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
        {
            candidate.fill_integers(2048 + index as u32, 1, i32::from_le_bytes(*bytes))?;
        }
        for (index, bytes) in self.expanded[INTEGERS_END..FLOATS_END]
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
        {
            candidate.fill_floats(1024 + index as u32, 1, u32::from_le_bytes(*bytes))?;
        }
        for (index, bytes) in self.strings.iter().enumerate() {
            candidate.fill_strings(64 + index as u32, 1, bytes)?;
        }
        Ok((candidate, self.lists))
    }
}

/// Native `system.dat` image written by `0x2129` (0x14002e2a0 in the 2021 x64 build).
pub(super) fn encode(
    state: &State,
    lists: &[Vec<Vec<u8>>; 2],
    game_id: Option<u32>,
    system_flag: bool,
) -> Result<Vec<u8>> {
    let mut expanded = state.system_image();
    for name in lists.iter().flatten() {
        expanded.extend_from_slice(name);
        expanded.push(0);
    }
    ensure!(expanded.len() <= MAX_EXPANDED, "CSS1 expanded size limit");
    let mut packed = Vec::new();
    lzss_compress_bytewise(&expanded, &CSS1_LZSS, &mut packed, || Ok(()))?;
    let (subtract, xor_bias, selector, rotation) = (0x5a_u8, 0xa5_u8, 0x5a_u16, 0x1234_u16);
    let shift = u32::from((rotation >> ((selector >> 1) & 7)) & 3) + 2;
    let mut checksum = 0u16;
    for (index, byte) in packed.iter_mut().enumerate() {
        let key = xor_bias.wrapping_add(CHECK_SEQUENCE[index % CHECK_SEQUENCE.len()]);
        *byte = (byte.wrapping_add(64) ^ key)
            .wrapping_add(subtract)
            .rotate_left(shift);
        checksum = checksum.wrapping_add(u16::from(*byte));
    }
    let mut out = vec![0; PAYLOAD_OFFSET];
    out[..4].copy_from_slice(b"CSS1");
    let put = |out: &mut Vec<u8>, at: usize, value: u32| {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    };
    put(&mut out, 4, 104);
    put(&mut out, 68, game_id.unwrap_or(0));
    // Engine +11840, set by `0x212c`.
    put(&mut out, 72, u32::from(system_flag));
    out[76..78].copy_from_slice(&1u16.to_le_bytes());
    out[78..80].copy_from_slice(&(u16::from(subtract) | selector & 0xff00).to_le_bytes());
    out[80..82].copy_from_slice(&u16::from(xor_bias).to_le_bytes());
    out[82..84].copy_from_slice(&rotation.to_le_bytes());
    put(&mut out, 84, lists[1].len() as u32);
    put(&mut out, 88, lists[0].len() as u32);
    put(&mut out, 92, expanded.len() as u32);
    put(&mut out, 96, packed.len() as u32);
    put(&mut out, 100, (PAYLOAD_OFFSET + packed.len()) as u32);
    out.extend_from_slice(&packed);
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

fn read_name(tail: &mut &[u8], limit: usize) -> Result<Vec<u8>> {
    let remaining = *tail;
    let end = remaining
        .iter()
        .position(|&byte| byte == 0)
        .context("unterminated CSS1 string")?;
    ensure!(end <= limit, "CSS1 string exceeds its destination");
    let name = remaining[..end].to_vec();
    *tail = &remaining[end + 1..];
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_image_reads_back_through_the_native_decoder() {
        let mut state = State::new();
        state.fill_bits(2050, 3, true).unwrap();
        state.fill_integers(2100, 2, -7).unwrap();
        state.fill_floats(1030, 1, 1.5f32.to_bits()).unwrap();
        state.fill_strings(70, 1, b"name").unwrap();
        let lists = [vec![b"first".to_vec()], vec![b"second".to_vec(), b"third".to_vec()]];
        let image = encode(&state, &lists, Some(0x2020_a053), false).unwrap();
        let decoded = CssState::decode(image, Some(0x2020_a053)).unwrap();
        let (restored, names) = decoded.apply(&State::new()).unwrap();
        assert_eq!(names, lists);
        assert_eq!(restored.system_image(), state.system_image());
        assert_eq!(restored.integer(2101), Some(-7));
    }
}
