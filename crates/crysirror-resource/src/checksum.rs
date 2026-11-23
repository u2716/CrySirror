//! Bounded, allocation-free checksums for native resource and identifier ABIs.

/// MD5 digest for legacy file/protocol compatibility. The result is raw16-byte
/// digest data; callers choose their native textual encoding separately.
pub fn md5_legacy(bytes: &[u8]) -> [u8; 16] {
    use md5::{Digest, Md5};
    let digest = Md5::digest(bytes);
    let mut result = [0; 16];
    result.copy_from_slice(&digest);
    result
}

/// SHA-256 over a bounded capability reader. Callers admit blocking reads on
/// an asset worker and retain responsibility for cancellation between jobs.
pub fn sha256_reader(
    source: &dyn crysirror_core::service::AssetReader,
    limit: u64,
) -> anyhow::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let length = source.len()?;
    anyhow::ensure!(length <= limit, "SHA256 input byte budget");
    let mut digest = Sha256::new();
    let mut at = 0u64;
    let mut buffer = [0u8; 65536];
    while at < length {
        let take = (length - at).min(buffer.len() as u64) as usize;
        let n = source.read_at(at, &mut buffer[..take])?;
        anyhow::ensure!(n > 0 && n <= take, "truncated SHA256 input");
        digest.update(&buffer[..n]);
        at += n as u64;
    }
    Ok(digest.finalize().into())
}

/// CRC-16/CCITT with polynomial0x1021, initial0, no reflection and no final XOR
/// (also known as CRC-16/XMODEM). This is an integrity checksum, not a MAC.
pub fn crc16_xmodem(bytes: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                crc.wrapping_shl(1) ^ 0x1021
            } else {
                crc.wrapping_shl(1)
            };
        }
    }
    crc
}
/// SHA-256 of an in-memory byte slice.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}

/// Bit-serial CRC-16 over x^16+x^12+x^5+1 with a zero register, data bits
/// taken least-significant first, and the result read with register bit 0 as
/// the most significant output bit (QLIE 4E6384).
pub fn crc16_ccitt_serial(bytes: &[u8]) -> u16 {
    const TAPS: [bool; 16] = {
        let mut t = [false; 16];
        t[0] = true;
        t[5] = true;
        t[12] = true;
        t
    };
    let mut r = [false; 16];
    for byte in bytes {
        let mut b = *byte;
        for _ in 0..8 {
            let mut carry = b & 1 != 0;
            b >>= 1;
            let feedback = r[15];
            for i in 0..16 {
                let old = r[i];
                r[i] = (feedback && TAPS[i]) ^ carry;
                carry = old;
            }
        }
    }
    r.iter().fold(0u16, |acc, bit| (acc << 1) | u16::from(*bit))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serial_crc_vectors() {
        assert_eq!(crc16_ccitt_serial(b"123456789"), 0x507f);
        assert_eq!(crc16_ccitt_serial(&[1]), 0x100);
        assert_eq!(crc16_ccitt_serial(b""), 0);
    }
    #[test]
    fn standard_xmodem_check_vector_and_empty_input() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31c3);
        assert_eq!(crc16_xmodem(b""), 0);
        assert_eq!(crc16_xmodem(&[0]), 0);
        assert_eq!(crc16_xmodem(&[1]), 0x1021);
    }
}
