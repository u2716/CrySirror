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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_xmodem_check_vector_and_empty_input() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31c3);
        assert_eq!(crc16_xmodem(b""), 0);
        assert_eq!(crc16_xmodem(&[0]), 0);
        assert_eq!(crc16_xmodem(&[1]), 0x1021);
    }
}
