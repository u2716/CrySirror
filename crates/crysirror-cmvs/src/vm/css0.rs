// SPDX-License-Identifier: MPL-2.0
//! CVNS system data, written by 0x411670 and read by 0x412170.
//! The script chooses the persistent suffix of each bank before the call.
use super::evaluate::State;
use super::system_save::SYSTEM_LZSS;
use anyhow::{Context, Result, ensure};
use crysirror_resource::{lzss::lzss_compress_bytewise, lzss_decompress};

const HEADER: usize = 600;
const LIMIT: usize = 1 << 20;
const KEY: &[u8] = b"PURPLE YUKO KAWAI ARC CHECK";

#[derive(Clone, Copy)]
pub(crate) struct Suffix {
    bit_byte: usize,
    integer: u32,
    string: u32,
}

impl Suffix {
    pub(super) fn new([bit, integer, string]: [u32; 3]) -> Result<Self> {
        ensure!(
            bit <= 262144 && integer <= 4096 && string <= 256,
            "CVNS persistent bank bounds"
        );
        Ok(Self {
            bit_byte: bit as usize / 8,
            integer,
            string,
        })
    }

    fn bit_bytes(self) -> usize {
        32768 - self.bit_byte
    }
    fn integer_bytes(self) -> usize {
        (4096 - self.integer) as usize * 4
    }
}

/// A detached bank suffix and its two resource-name registries.
pub(crate) struct SystemData {
    suffix: Suffix,
    banks: Vec<u8>,
    strings: Vec<Vec<u8>>,
    names: [Vec<Vec<u8>>; 2],
}

impl SystemData {
    pub(crate) fn decode(mut file: Vec<u8>, suffix: Suffix) -> Result<Self> {
        ensure!(
            file.len() >= HEADER && &file[..4] == b"CSS0",
            "invalid CVNS system save signature"
        );
        let word = |at| u32::from_le_bytes(file[at..at + 4].try_into().unwrap()) as usize;
        ensure!(
            word(4) == HEADER && file[544..546] == [1, 0],
            "unsupported CSS0 header or encoding"
        );
        let end = HEADER
            .checked_add(word(560))
            .context("CSS0 packed size overflow")?;
        let counts = [word(552), word(556)];
        let size = word(580);
        ensure!(
            word(540) == end && (end == file.len() || end.checked_add(2) == Some(file.len())),
            "CSS0 packed extent"
        );
        if file.len() != end {
            let sum = file[HEADER..end]
                .iter()
                .fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
            ensure!(
                file[end..] == sum.to_le_bytes(),
                "CVNS save000.dat checksum fault 0x10000000"
            );
        }
        let banks_end = suffix.bit_bytes() + suffix.integer_bytes();
        let string_count = (256 - suffix.string) as usize;
        ensure!(
            size <= LIMIT
                && banks_end + string_count <= size
                && counts
                    .iter()
                    .try_fold(0usize, |sum, n| sum.checked_add(*n))
                    .is_some_and(|n| n <= size - banks_end - string_count),
            "CSS0 expanded extents"
        );
        let (xor, add) = (file[546], file[548]);
        let packed = &mut file[HEADER..end];
        for (i, byte) in packed.iter_mut().enumerate() {
            *byte =
                (byte.wrapping_sub(add) ^ xor.wrapping_add(KEY[i % KEY.len()])).wrapping_sub(64);
        }
        let mut banks = lzss_decompress(packed, size, &SYSTEM_LZSS)?;
        let mut tail = &banks[banks_end..];
        let mut strings = Vec::new();
        for _ in 0..string_count {
            strings.push(super::system_save::read_name(&mut tail, 1023)?);
        }
        let mut names = [Vec::new(), Vec::new()];
        for (list, count) in names.iter_mut().zip(counts) {
            for _ in 0..count {
                list.push(super::system_save::read_name(&mut tail, 63)?);
            }
        }
        ensure!(tail.is_empty(), "trailing CSS0 bank data");
        banks.truncate(banks_end);
        Ok(Self {
            suffix,
            banks,
            strings,
            names,
        })
    }

    pub(super) fn capture(state: &State, suffix: Suffix, names: [Vec<Vec<u8>>; 2]) -> Result<Self> {
        let mut banks = state.packed_bit_suffix(suffix.bit_byte);
        for slot in suffix.integer..4096 {
            banks.extend_from_slice(
                &state
                    .integer(slot)
                    .context("missing CVNS integer")?
                    .to_le_bytes(),
            );
        }
        let strings = (suffix.string..256)
            .map(|i| {
                state
                    .string(i)
                    .map(<[u8]>::to_vec)
                    .context("missing CVNS string cell")
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            suffix,
            banks,
            strings,
            names,
        })
    }

    pub(crate) fn encode(self, salts: [u8; 2], cancel: impl Fn() -> Result<()>) -> Result<Vec<u8>> {
        let mut payload = self.banks;
        let mut file = vec![0; HEADER];
        file[..4].copy_from_slice(b"CSS0");
        let label = encoding_rs::SHIFT_JIS
            .encode("書き換えちゃ…や～ですの！！なんてね♪")
            .0;
        file[16..16 + label.len()].copy_from_slice(&label);
        file[4..8].copy_from_slice(&(HEADER as u32).to_le_bytes());
        file[544] = 1;
        file[546] = salts[0];
        file[548] = salts[1];
        for (i, list) in self.names.iter().enumerate() {
            let at = 552 + i * 4;
            file[at..at + 4].copy_from_slice(&u32::try_from(list.len())?.to_le_bytes());
            ensure!(
                list.iter().all(|n| n.len() < 64),
                "CSS0 name exceeds native record"
            );
        }
        for name in self.strings.iter().chain(self.names.iter().flatten()) {
            ensure!(
                payload.len() + name.len() < LIMIT,
                "CSS0 expanded size limit"
            );
            payload.extend_from_slice(name);
            payload.push(0);
        }
        file[580..584].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        lzss_compress_bytewise(&payload, &SYSTEM_LZSS, &mut file, cancel)?;
        let end = file.len();
        file[540..544].copy_from_slice(&(end as u32).to_le_bytes());
        file[560..564].copy_from_slice(&((end - HEADER) as u32).to_le_bytes());
        let mut sum = 0u16;
        for (i, b) in file[HEADER..].iter_mut().enumerate() {
            *b = (b.wrapping_add(64) ^ salts[0].wrapping_add(KEY[i % KEY.len()]))
                .wrapping_add(salts[1]);
            sum = sum.wrapping_add(u16::from(*b));
        }
        file.extend_from_slice(&sum.to_le_bytes());
        Ok(file)
    }

    pub(super) fn apply(self, state: &mut State) -> Result<[Vec<Vec<u8>>; 2]> {
        let suffix = self.suffix;
        let (bits, integers) = self.banks.split_at(suffix.bit_bytes());
        state
            .restore_bits((suffix.bit_byte * 8) as u32, bits)
            .map_err(anyhow::Error::msg)?;
        for (slot, value) in (suffix.integer..4096).zip(integers.as_chunks::<4>().0) {
            state
                .fill_integers(slot, 1, i32::from_le_bytes(*value))
                .map_err(anyhow::Error::msg)?;
        }
        for (slot, value) in (suffix.string..256).zip(self.strings) {
            state
                .fill_strings(slot, 1, &value)
                .map_err(anyhow::Error::msg)?;
        }
        Ok(self.names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_only_configured_suffixes_and_checks_the_packed_bytes() -> Result<()> {
        let mut state = State::new().with_string_cells(256);
        let suffix = Suffix::new([262137, 4095, 255])?;
        state.fill_bits(262136, 2, true).unwrap();
        state.fill_integers(4095, 1, -3).unwrap();
        state.fill_strings(255, 1, b"last").unwrap();
        let lists = [vec![b"a.pb2".to_vec()], vec![b"b.pss".to_vec()]];
        let file =
            SystemData::capture(&state, suffix, lists.clone())?.encode([234, 179], || Ok(()))?;
        let mut next = State::new().with_string_cells(256);
        next.fill_bits(0, 1, true).unwrap();
        next.fill_integers(4094, 1, 7).unwrap();
        next.fill_strings(254, 1, b"keep").unwrap();
        for bytes in [&file[..], &file[..file.len() - 2]] {
            assert_eq!(
                SystemData::decode(bytes.to_vec(), suffix)?.apply(&mut next)?,
                lists
            );
            assert_eq!(next.bit(0), Some(true));
            assert_eq!(next.bit(262136), Some(true));
            assert_eq!(next.integer(4094), Some(7));
            assert_eq!(next.integer(4095), Some(-3));
            assert_eq!(next.string(254), Some(&b"keep"[..]));
            assert_eq!(next.string(255), Some(&b"last"[..]));
        }
        let mut damaged = file;
        damaged[HEADER] ^= 1;
        assert!(SystemData::decode(damaged, suffix).is_err());
        Ok(())
    }
}
