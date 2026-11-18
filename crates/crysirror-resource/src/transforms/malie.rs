// CFI adaptation of MalieEncryption.cs, Copyright (C) 2017 by morkt.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.
//
//! Position-dependent Malie blocks, GARbro revision
//! b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0, Malie/MalieEncryption.cs.
//! Immutable read-at views decrypt bounded windows, including the blocks on
//! both sides of an unaligned request. No archive-sized allocation or shared
//! seek cursor. Keys remain host supplied, client scoped and zeroized.
use super::*;
use zeroize::Zeroizing;

#[path = "malie_camellia.rs"]
mod camellia;

pub(super) fn apply(
    spec: &TransformSpec,
    input: Arc<dyn AssetReader>,
    context: &DecodeContext,
) -> Result<Arc<dyn AssetReader>> {
    #[derive(Deserialize, Default)]
    #[serde(default, deny_unknown_fields)]
    struct Params {
        offset: u64,
    }
    let params: Params = if spec.parameters.is_null() {
        Params::default()
    } else {
        serde_json::from_value(spec.parameters.clone())?
    };
    let key = context.keys.key(
        context.client,
        spec.key_id
            .as_deref()
            .context("Malie cipher requires key_id")?,
    )?;
    let cipher = key.with_bytes(|bytes| {
        if spec.algorithm == "malie-camellia" {
            Ok(Cipher::Camellia(camellia::key(bytes)?))
        } else {
            ensure!(
                bytes.len() == 48,
                "Malie CFI key needs 32 bytes and four little-endian rotation words"
            );
            Ok(Cipher::Cfi(key.clone()))
        }
    })?;
    let len = input.len().map_err(|e| anyhow::anyhow!("{e}"))?;
    ensure!(
        params.offset % 16 == 0 && len % 16 == 0,
        "Malie cipher source must contain complete aligned 16-byte blocks"
    );
    ensure!(
        params.offset.checked_add(len).is_some(),
        "Malie cipher position overflow"
    );
    Ok(Arc::new(Reader {
        input,
        cipher,
        len,
        offset: params.offset,
    }))
}
enum Cipher {
    Camellia(Zeroizing<[u32; 52]>),
    Cfi(KeyMaterial),
}
impl Cipher {
    fn decrypt(&self, offset: u64, block: &mut [u8]) {
        match self {
            Self::Camellia(key) => camellia::decrypt(key, offset, block),
            Self::Cfi(key) => key.with_bytes(|key| {
                // The input block starts at a physical 16-byte boundary.
                let first = block[0];
                for byte in &mut block[1..] {
                    *byte ^= first;
                }
                let at = ((offset >> 4) & 31) as usize;
                for (i, word) in block.chunks_exact_mut(4).enumerate() {
                    let shift = |n: usize| u32::from(key[(at + n) & 31] ^ 0xa5);
                    let rotation =
                        u32::from_le_bytes(key[32 + i * 4..36 + i * 4].try_into().unwrap());
                    let value = u32::from_le_bytes(word.try_into().unwrap());
                    let value = if i % 2 == 0 {
                        (value ^ rotation.rotate_right(shift(i * 3)))
                            .rotate_right(shift(12 + i * 3))
                    } else {
                        (value ^ rotation.rotate_left(shift(i * 3))).rotate_left(shift(12 + i * 3))
                    };
                    word.copy_from_slice(&value.to_le_bytes());
                }
            }),
        }
    }
}
struct Reader {
    input: Arc<dyn AssetReader>,
    cipher: Cipher,
    len: u64,
    offset: u64,
}
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.len)
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        if out.is_empty() || offset >= self.len {
            return Ok(0);
        }
        let count = (self.len - offset).min(out.len() as u64) as usize;
        let mut copied = 0;
        // Stack scratch only: each underlying request is at most 4 KiB. Reads
        // are independent across concurrent consumers and tolerate short I/O.
        let mut scratch = Zeroizing::new([0u8; 4096]);
        while copied < count {
            let at = offset + copied as u64;
            let base = at & !15;
            let skip = (at - base) as usize;
            let take = (count - copied).min(4096 - skip);
            let n = (skip + take + 15) & !15;
            crate::read_exact_at(self.input.as_ref(), base, &mut scratch[..n])
                .map_err(|e| crysirror_core::Error::Io(format!("Malie encrypted read: {e:#}")))?;
            for (i, block) in scratch[..n].chunks_exact_mut(16).enumerate() {
                self.cipher
                    .decrypt(self.offset + base + i as u64 * 16, block);
            }
            out[copied..copied + take].copy_from_slice(&scratch[skip..skip + take]);
            copied += take;
        }
        Ok(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::state::ClientId;

    #[test]
    fn camellia_rfc3713_vector_after_position_whitening() {
        // RFC 3713 Appendix A, 128-bit example. Undo only Malie's input word
        // rotations; the expected plaintext is independent of this port.
        let raw = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let encrypted = [
            0x67, 0x67, 0x31, 0x38, 0x54, 0x96, 0x69, 0x73, 0x08, 0x57, 0x06, 0x56, 0x48, 0xea,
            0xbe, 0x43,
        ];
        let key = camellia::key(&raw).unwrap();
        for offset in [0, 16, 240, 256, 0x1_0000_0050] {
            let mut stored = encrypted;
            let roll = ((offset >> 4) & 15) as u32 + 16;
            for word in stored.chunks_exact_mut(4).enumerate() {
                let (i, bytes) = word;
                let v = u32::from_be_bytes(bytes.try_into().unwrap()).swap_bytes();
                let v = if i % 2 == 0 {
                    v.rotate_right(roll)
                } else {
                    v.rotate_left(roll)
                };
                bytes.copy_from_slice(&v.to_le_bytes());
            }
            camellia::decrypt(&key, offset, &mut stored);
            assert_eq!(stored, raw, "offset {offset}");
        }
        let expanded: Vec<u8> = key.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(*camellia::key(&expanded).unwrap(), *key);
    }

    #[test]
    fn cfi_position_dependent_reference_blocks() {
        let mut key: Vec<u8> = (0..32).collect();
        key.extend(
            [0x01234567u32, 0x89abcdef, 0xfedcba98, 0x76543210]
                .into_iter()
                .flat_map(u32::to_le_bytes),
        );
        let cipher = Cipher::Cfi(KeyMaterial::new(key).unwrap());
        // Fixed vectors from the referenced CFI operation, including a physical
        // offset above 4 GiB; no native program or title key is used.
        for (offset, expected) in [
            (
                0,
                [
                    0x8d, 0x85, 0x9d, 0xb5, 0xb7, 0x99, 0xfb, 0xd5, 0x29, 0xb6, 0x3c, 0xa3, 0x2f,
                    0x4c, 0x69, 0x0a,
                ],
            ),
            (
                16,
                [
                    0x35, 0x10, 0x73, 0x16, 0x33, 0x8a, 0x62, 0xdb, 0xd2, 0xb8, 0x9e, 0xf4, 0x31,
                    0xb0, 0x30, 0xb1,
                ],
            ),
            (
                0x1_0000_0050,
                [
                    0x22, 0x70, 0x42, 0x14, 0x77, 0xfd, 0x62, 0xe8, 0x5a, 0xb8, 0x9e, 0x7c, 0x5f,
                    0x47, 0x4f, 0x57,
                ],
            ),
        ] {
            let mut block: Vec<u8> = (0x40..0x50).collect();
            cipher.decrypt(offset, &mut block);
            assert_eq!(block, expected);
        }
    }

    #[test]
    fn cfi_unaligned_reads_are_bounded_and_client_scoped() {
        let context = DecodeContext::new(ClientId(3));
        let keys = crate::MemoryKeyProvider::default();
        let mut material = vec![0xa5; 32];
        material.extend([0; 16]); // zero rotations: a hand-calculable XOR block
        keys.insert(context.client, "fixture", material).unwrap();
        let mut context = context;
        context.keys = Arc::new(keys);
        let stored = vec![0x55; 8192];
        let source: Arc<dyn AssetReader> = Arc::new(MemoryAssetReader(Arc::from(stored)));
        let mut spec = TransformSpec::new("malie-cfi");
        spec.key_id = Some("fixture".into());
        let reader = context
            .apply(&spec, source.clone(), &serde_json::Value::Null)
            .unwrap();
        let mut bytes = vec![255; 4100];
        assert_eq!(reader.read_at(15, &mut bytes).unwrap(), bytes.len());
        for (i, &v) in bytes.iter().enumerate() {
            assert_eq!(v, if (i + 15) % 16 == 0 { 0x55 } else { 0 });
        }
        assert_eq!(reader.read_at(8192, &mut bytes).unwrap(), 0);
        let mut other = DecodeContext::new(ClientId(4));
        other.keys = context.keys.clone();
        assert!(
            other
                .apply(&spec, source, &serde_json::Value::Null)
                .is_err()
        );
        assert_eq!(context.resident_bytes(), 0);
    }
}
