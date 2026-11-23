//! Scalar packed-lane resource arithmetic shared by QLIE archives and saves.
//! Native 4E563C/4E56BC/4E6310 use independent MMX lanes, not a u64 carry.
//! Complete eight-byte blocks are transformed; a final partial block is intact.

fn add_lanes(a: u64, b: u64, bits: u32) -> u64 {
    let mask = (1u64 << bits) - 1;
    (0..64 / bits).fold(0, |out, lane| {
        let shift = lane * bits;
        out | ((((a >> shift) & mask).wrapping_add((b >> shift) & mask) & mask) << shift)
    })
}

/// Legacy packed-word checksum. Incomplete trailing bytes do not participate.
/// This is a resource integrity check, not a cryptographic authentication tag.
pub fn checksum(bytes: &[u8]) -> u32 {
    let (mut state, mut sum) = (0, 0);
    for block in bytes.chunks_exact(8) {
        state = add_lanes(state, 0xa35793a7a35793a7, 16);
        let added = add_lanes(
            sum,
            u64::from_le_bytes(block.try_into().unwrap()) ^ state,
            16,
        );
        sum = u64::from((added as u32).rotate_left(3))
            | (u64::from(((added >> 32) as u32).rotate_left(3)) << 32);
    }
    let lane = |n| ((sum >> (16 * n)) as u16 as i16) as i32;
    lane(0)
        .wrapping_mul(lane(2))
        .wrapping_add(lane(1).wrapping_mul(lane(3))) as u32
}

fn crypt(bytes: &mut [u8], seed: u32, decrypt: bool) {
    let initial = seed.wrapping_add(bytes.len() as u32) ^ 0xfec9753e;
    let mut previous = u64::from(initial) | (u64::from(initial) << 32);
    let mut key = 0xa73c5f9da73c5f9d;
    for block in bytes.chunks_exact_mut(8) {
        key = add_lanes(key, 0xce24f523ce24f523, 32) ^ previous;
        let input = u64::from_le_bytes(block.try_into().unwrap());
        let output = input ^ key;
        previous = if decrypt { output } else { input };
        block.copy_from_slice(&output.to_le_bytes());
    }
}

/// Decode a legacy feedback resource using its caller-provided format seed.
pub fn decrypt(bytes: &mut [u8], seed: u32) {
    crypt(bytes, seed, true);
}

/// Encode the same feedback profile. The recurrence retains plaintext blocks.
pub fn encrypt(bytes: &mut [u8], seed: u32) {
    crypt(bytes, seed, false);
}
