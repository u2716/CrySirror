//! QLIE FilePack 3.1. Native packed-lane arithmetic is written explicitly so
//! decoding is identical on scalar, SIMD and non-x86 hosts, without DLLs.
//! Format research: https://github.com/rafi441/qlie-unpack and
//! https://github.com/Inori/FuckGalEngine/tree/master/QLIE .
use super::*;

fn checksum(bytes: &[u8]) -> u32 {
    crysirror_resource::lane_feedback::checksum(bytes)
}
fn filename(raw: &[u8], key: u32) -> Result<String> {
    let count = (raw.len() / 2) as u32;
    let salt = (count.wrapping_mul(count) ^ count ^ 0x3e13 ^ (key >> 16) ^ key) & 0xffff;
    let mut state = salt;
    let units = raw.chunks_exact(2).enumerate().map(|(i, pair)| {
        state = state.wrapping_mul(8).wrapping_add(salt).wrapping_add(i as u32);
        u16::from_le_bytes([pair[0], pair[1]]) ^ state as u16
    }).collect::<Vec<_>>();
    String::from_utf16(&units).context("QLIE 3.1 UTF16 filename")
}
fn decrypt(bytes: &[u8], name: &str, key: u32, method: u32, fkey: Option<&[u8]>) -> Result<Vec<u8>> {
    ensure!(method <= 2, "unsupported QLIE 3.1 cipher method {method}");
    if method == 0 { return Ok(bytes.to_vec()); }
    let second = method == 2;
    let (mut hash, mut seed) = if second {(0x86f7e2u32, 0x4437f1u32)} else {(0x85f532, 0x33f641)};
    for (i, unit) in name.encode_utf16().enumerate() {
        hash = hash.wrapping_add(u32::from(unit) << (i & 7));
        seed ^= hash;
    }
    let n = bytes.len() as u32;
    let factor: u32 = if second {13} else {9};
    let multiplier: u32 = if second {13} else {7};
    seed = seed.wrapping_add(key ^ multiplier.wrapping_mul(n & 0xffffff).wrapping_add(n)
        .wrapping_add(hash).wrapping_add(hash ^ n ^ if second {0x56e213} else {0x8f32dc}));
    seed = (seed & 0xffffff).wrapping_mul(factor);
    let multiplier = if second {0x8a77f473u64} else {0x8df21431u64};
    let table: [u32; 32] = std::array::from_fn(|_| {
        let wide = multiplier * (seed ^ multiplier as u32) as u64;
        seed = wide.wrapping_add(wide >> 32) as u32;
        seed
    });
    let mut extra = [0u8; 1024];
    if second {
        for (i, word) in extra.chunks_exact_mut(4).enumerate() {
            let i = i as i32;
            let value = (i + 7) * if i % 3 == 0 { i + 3 } else { -(i + 3) };
            word.copy_from_slice(&value.to_le_bytes());
        }
        if let Some(fkey) = fkey.filter(|k| k.len() >= 128) {
            let mut pos = fkey[49] as usize % 73 + 128;
            let step = fkey[79] as usize % 7 + 7;
            for byte in &mut extra {
                pos = (pos + step) % fkey.len();
                *byte ^= fkey[pos];
            }
        }
    }
    let pair = |i: usize| u64::from(table[i]) | (u64::from(table[i + 1]) << 32);
    let mut pos = if second {(8 * (table[8] & 13)) as usize & 127} else {2 * (table[13] as usize & 15)};
    let mut state = pair(6);
    let mut out = bytes.to_vec();
    for word in out.chunks_exact_mut(8) {
        let mask = if second {
            pair(2 * (pos & 15)) ^ u64::from_le_bytes(extra[8 * pos..8 * pos + 8].try_into().unwrap())
        } else { pair(pos) };
        state = padd(state ^ mask, mask);
        let plain = u64::from_le_bytes(word.try_into().unwrap()) ^ state;
        word.copy_from_slice(&plain.to_le_bytes());
        state = padb(state, plain) ^ plain;
        state = padw((state << 1) & 0xfffffffefffffffe, plain);
        pos = if second {(pos + 1) & 127} else {(pos + 2) & 31};
    }
    Ok(out)
}
#[derive(Clone)]
struct Entry {
    path: String, offset: u64, stored: u64, size: u64,
    compressed: bool, cipher: u32, checksum: u32,
}
fn decoded(e: &Entry, input: &[u8], seed: u32, fkey: Option<&[u8]>) -> Result<Vec<u8>> {
    ensure!(checksum(input) == e.checksum, "QLIE encrypted data checksum mismatch: {}", e.path);
    let bytes = decrypt(input, &e.path, seed, e.cipher, fkey)?;
    let bytes = if e.compressed || (bytes.len() as u64 != e.size && bytes.starts_with(b"1PC\xff")) {
        pack_decompress(&bytes, usize::try_from(e.size)?)?
    } else { bytes };
    ensure!(bytes.len() as u64 == e.size, "QLIE 3.1 decoded size mismatch: {}", e.path);
    Ok(bytes)
}
pub(super) fn open(source: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    let length = length(source)?;
    let footer = length.checked_sub(28).context("QLIE footer")?;
    let seed_at = length.checked_sub(0x41c).context("QLIE 3.1 key table")?;
    let seed = checksum(&bytes(source, seed_at, 256, 256)?) & 0x0fffffff;
    let count = u32_at(source, footer + 16)? as usize;
    limits(cx, count)?;
    let offset = u64::from_le_bytes(bytes(source, footer + 20, 8, 8)?.try_into().unwrap());
    let table = index(source, cx, offset, seed_at.checked_sub(offset).context("QLIE 3.1 index range")?)?;
    let mut cursor = Cursor::new(&table);
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let chars = cursor.le16()? as usize;
        ensure!(chars > 0 && chars <= 4096, "QLIE 3.1 name length limit");
        let path = filename(cursor.take(chars * 2)?, seed)?;
        let e = Entry { path, offset: cursor.le64()?, stored: cursor.le32()?.into(), size: cursor.le32()?.into(),
            compressed: cursor.le32()? != 0, cipher: cursor.le32()?, checksum: cursor.le32()? };
        ensure!(e.offset.checked_add(e.stored).is_some_and(|end| end <= offset), "QLIE entry overlaps index");
        ensure!(e.size <= cx.limits.max_entry_size, "QLIE entry size budget");
        ensure!(e.cipher <= 2, "unsupported QLIE 3.1 cipher method {}", e.cipher);
        entries.push(e);
    }
    let external = cx.parameters.get("qlie/pack").and_then(|p|p["key1_id"].as_str())
        .map(|id| cx.keys.key(cx.client, id)).transpose()?;
    let key_entry = entries.iter().find(|e|e.path.rsplit(['/', '\\']).next().unwrap_or("").starts_with("pack_keyfile"));
    let key = if let Some(e) = key_entry {
        ensure!(e.size <= 1024 * 1024 && e.stored <= cx.limits.max_index_size, "QLIE embedded key budget");
        let input = index(source, cx, e.offset, e.stored)?;
        let external = external.as_ref().map(|k|k.with_bytes(|b|b.to_vec()));
        Some(crysirror_resource::KeyMaterial::new(decoded(e, &input, seed, external.as_deref())?)?)
    } else { external.clone() };
    let key_path = key_entry.map(|k|k.path.clone());
    Ok(entries.into_iter().map(|e| {
        let own_key = key_path.as_ref().is_some_and(|k| *k == e.path);
        let key = if own_key { external.clone() } else { key.clone() };
        let captured = e.clone();
        Row::packed(e.path, e.offset, e.stored, e.size, decode(move |input, _| {
            let fkey = key.as_ref().map(|k|k.with_bytes(|b|b.to_vec()));
            decoded(&captured, input, seed, fkey.as_deref())
        }))
    }).collect())
}
