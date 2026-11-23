/* camellia.c ver1.0

Copyright (c) 2006
 NTT (Nippon Telegraph and Telephone Corporation) . All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer as
   the first lines of this file unmodified.
2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY NTT ``AS IS'' AND ANY EXPRESS OR
IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
IN NO EVENT SHALL NTT BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/
// Rust adaptation of morkt's GARbro Camellia.cs, revision
// b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0. No title keys are included.
use anyhow::{Result, ensure};
use zeroize::Zeroizing;

pub(super) fn key(bytes: &[u8]) -> Result<Zeroizing<[u32; 52]>> {
    ensure!(
        matches!(bytes.len(), 16 | 208 | 224),
        "Malie Camellia key needs 16 raw bytes or 52/56 little-endian expanded words"
    );
    let mut key = Zeroizing::new([0; 52]);
    if bytes.len() != 16 {
        for (word, raw) in key.iter_mut().zip(bytes.chunks_exact(4)) {
            *word = u32::from_le_bytes(raw.try_into().unwrap());
        }
        return Ok(key);
    }
    let kl = u128::from_be_bytes(bytes.try_into().unwrap());
    let mut state = Zeroizing::new(words(kl));
    for (left, right, sigma) in [(0, 2, 0xa09e667f3bcc908bu64), (2, 0, 0xb67ae8584caa73b2)] {
        let (a, b) = f(
            state[left],
            state[left + 1],
            (sigma >> 32) as u32,
            sigma as u32,
        );
        state[right] ^= a;
        state[right + 1] ^= b;
    }
    for (i, word) in words(kl).into_iter().enumerate() {
        state[i] ^= word;
    }
    for (left, right, sigma) in [(0, 2, 0xc6ef372fe94f82beu64), (2, 0, 0x54ff53a5f1d36f1c)] {
        let (a, b) = f(
            state[left],
            state[left + 1],
            (sigma >> 32) as u32,
            sigma as u32,
        );
        state[right] ^= a;
        state[right + 1] ^= b;
    }
    let ka = state.iter().fold(0u128, |v, &w| (v << 32) | u128::from(w));
    for (at, value, rotation) in [
        (0, ka, 111),
        (4, kl, 111),
        (8, ka, 94),
        (12, kl, 94),
        (16, kl, 77),
        (20, ka, 60),
        (28, kl, 45),
        (32, ka, 30),
        (36, ka, 15),
        (40, kl, 15),
        (44, ka, 0),
        (48, kl, 0),
    ] {
        key[at..at + 4].copy_from_slice(&words(value.rotate_left(rotation)));
    }
    key[24..26].copy_from_slice(&words(ka.rotate_left(45))[..2]);
    key[26..28].copy_from_slice(&words(kl.rotate_left(60))[2..]);
    Ok(key)
}
fn words(v: u128) -> [u32; 4] {
    [
        (v >> 96) as u32,
        (v >> 64) as u32,
        (v >> 32) as u32,
        v as u32,
    ]
}
fn s1(i: u32) -> u32 {
    u32::from(SBOX[i as usize & 255]) * 0x01010100
}
fn s2(i: u32) -> u32 {
    u32::from(SBOX[i as usize & 255].rotate_left(1)) * 0x00010101
}
fn s3(i: u32) -> u32 {
    u32::from(SBOX[i as usize & 255].rotate_right(1)) * 0x01000101
}
fn s4(i: u32) -> u32 {
    u32::from(SBOX[(i as u8).rotate_left(1) as usize]) * 0x01010001
}
fn f(a: u32, b: u32, k0: u32, k1: u32) -> (u32, u32) {
    let x = a ^ k0;
    let y = b ^ k1;
    let u = s3(x >> 8) ^ s4(x) ^ s2(x >> 16) ^ s1(x >> 24);
    let d = s4(y >> 8) ^ s1(y) ^ s3(y >> 16) ^ s2(y >> 24);
    (u ^ d, u ^ d ^ u.rotate_right(8))
}
pub(super) fn decrypt(key: &[u32; 52], offset: u64, block: &mut [u8]) {
    let roll = ((offset >> 4) & 15) as u32 + 16;
    let mut v = Zeroizing::new([0u32; 4]);
    for (i, p) in block.chunks_exact(4).enumerate() {
        let w = u32::from_le_bytes(p.try_into().unwrap());
        v[i] = if i % 2 == 0 {
            w.rotate_left(roll)
        } else {
            w.rotate_right(roll)
        };
        v[i] = v[i].swap_bytes() ^ key[i];
    }
    let mut k = 4;
    for round in 0..3 {
        for _ in 0..3 {
            let (a, b) = f(v[0], v[1], key[k + 2], key[k + 3]);
            v[2] ^= a;
            v[3] ^= b;
            let (a, b) = f(v[2], v[3], key[k], key[k + 1]);
            v[0] ^= a;
            v[1] ^= b;
            k += 4;
        }
        if round < 2 {
            v[1] ^= (v[0] & key[k + 2]).rotate_left(1);
            v[0] ^= v[1] | key[k + 3];
            v[2] ^= v[3] | key[k + 1];
            v[3] ^= (v[2] & key[k]).rotate_left(1);
            k += 4;
        }
    }
    v.swap(0, 2);
    v.swap(1, 3);
    for (i, out) in block.chunks_exact_mut(4).enumerate() {
        out.copy_from_slice(&(v[i] ^ key[k + i]).to_be_bytes());
    }
}
const SBOX: [u8; 256] = [
    0x70, 0x82, 0x2c, 0xec, 0xb3, 0x27, 0xc0, 0xe5, 0xe4, 0x85, 0x57, 0x35, 0xea, 0x0c, 0xae, 0x41,
    0x23, 0xef, 0x6b, 0x93, 0x45, 0x19, 0xa5, 0x21, 0xed, 0x0e, 0x4f, 0x4e, 0x1d, 0x65, 0x92, 0xbd,
    0x86, 0xb8, 0xaf, 0x8f, 0x7c, 0xeb, 0x1f, 0xce, 0x3e, 0x30, 0xdc, 0x5f, 0x5e, 0xc5, 0x0b, 0x1a,
    0xa6, 0xe1, 0x39, 0xca, 0xd5, 0x47, 0x5d, 0x3d, 0xd9, 0x01, 0x5a, 0xd6, 0x51, 0x56, 0x6c, 0x4d,
    0x8b, 0x0d, 0x9a, 0x66, 0xfb, 0xcc, 0xb0, 0x2d, 0x74, 0x12, 0x2b, 0x20, 0xf0, 0xb1, 0x84, 0x99,
    0xdf, 0x4c, 0xcb, 0xc2, 0x34, 0x7e, 0x76, 0x05, 0x6d, 0xb7, 0xa9, 0x31, 0xd1, 0x17, 0x04, 0xd7,
    0x14, 0x58, 0x3a, 0x61, 0xde, 0x1b, 0x11, 0x1c, 0x32, 0x0f, 0x9c, 0x16, 0x53, 0x18, 0xf2, 0x22,
    0xfe, 0x44, 0xcf, 0xb2, 0xc3, 0xb5, 0x7a, 0x91, 0x24, 0x08, 0xe8, 0xa8, 0x60, 0xfc, 0x69, 0x50,
    0xaa, 0xd0, 0xa0, 0x7d, 0xa1, 0x89, 0x62, 0x97, 0x54, 0x5b, 0x1e, 0x95, 0xe0, 0xff, 0x64, 0xd2,
    0x10, 0xc4, 0x00, 0x48, 0xa3, 0xf7, 0x75, 0xdb, 0x8a, 0x03, 0xe6, 0xda, 0x09, 0x3f, 0xdd, 0x94,
    0x87, 0x5c, 0x83, 0x02, 0xcd, 0x4a, 0x90, 0x33, 0x73, 0x67, 0xf6, 0xf3, 0x9d, 0x7f, 0xbf, 0xe2,
    0x52, 0x9b, 0xd8, 0x26, 0xc8, 0x37, 0xc6, 0x3b, 0x81, 0x96, 0x6f, 0x4b, 0x13, 0xbe, 0x63, 0x2e,
    0xe9, 0x79, 0xa7, 0x8c, 0x9f, 0x6e, 0xbc, 0x8e, 0x29, 0xf5, 0xf9, 0xb6, 0x2f, 0xfd, 0xb4, 0x59,
    0x78, 0x98, 0x06, 0x6a, 0xe7, 0x46, 0x71, 0xba, 0xd4, 0x25, 0xab, 0x42, 0x88, 0xa2, 0x8d, 0xfa,
    0x72, 0x07, 0xb9, 0x55, 0xf8, 0xee, 0xac, 0x0a, 0x36, 0x49, 0x2a, 0x68, 0x3c, 0x38, 0xf1, 0xa4,
    0x40, 0x28, 0xd3, 0x7b, 0xbb, 0xc9, 0x43, 0xc1, 0x15, 0xe3, 0xad, 0xf4, 0x77, 0xc7, 0x80, 0x9e,
];
