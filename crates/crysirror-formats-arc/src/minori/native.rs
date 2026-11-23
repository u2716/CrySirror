// SPDX-License-Identifier: MPL-2.0
//! Recover the PAZ reader's initialized data from a file-backed PE32 image.
use super::*;
use crysirror_resource::pe::PeData;

pub(super) struct Parameters {
    pub index: Vec<u8>,
    pub data: Option<Vec<u8>>,
    pub salt: Vec<u8>,
    pub members: Option<BTreeMap<&'static str, Vec<u8>>>,
    pub cipher: MemberCipher,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum MemberCipher {
    SinglePermutation,
    BlowfishOnly,
    Rc4,
    Rc4Drop,
}

const SHORT: &[&str] = &[
    "scr", "bg", "bg", "st", "sys", "bgm", "voice", "se", "se", "mov",
];
const LONG: &[&str] = &[
    "scr", "bg", "bg", "st", "sys", "bgm", "voice", "se", "se", "se", "se", "mov",
];

fn word(b: &[u8]) -> u32 {
    u32::from_le_bytes(b[..4].try_into().unwrap())
}

fn salt(bytes: &[u8]) -> Option<Vec<u8>> {
    let (nul, text) = bytes.split_last()?;
    (*nul == 0 && text.len() == 8 && text.iter().all(u8::is_ascii_graphic)).then(|| text.to_vec())
}

// The newer initializer assigns script, sound, image and movie strings to four
// consecutive globals. Resolve each immediate as a preferred VA, never as code.
fn literal_salts(pe: &PeData<'_>, exe: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut found = None;
    for s in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let start = s.file_offset as usize;
        let code = &exe[start..start + s.file_size as usize];
        for ops in code.windows(40).filter(|w| w.starts_with(&[0xc7, 5])) {
            let base = word(&ops[2..]);
            let mut salts = Vec::new();
            for (i, op) in ops.chunks_exact(10).enumerate() {
                if op[..2] != [0xc7, 5] || Some(word(&op[2..])) != base.checked_add(4 * i as u32) {
                    break;
                }
                let Some(text) = pe
                    .va_slice(u64::from(word(&op[6..])), 9)
                    .ok()
                    .and_then(salt)
                else {
                    break;
                };
                salts.push(text);
            }
            if salts.len() == 4 {
                ensure!(found.is_none(), "ambiguous minori PAZ member parameters");
                found = Some(salts);
            }
        }
    }
    found.context("missing native PAZ member parameter initializer")
}

// The compact tables keep the same bank order across two compiler ABIs.
// Their property strings are recovered from the original initializer.
fn decrypt_properties(
    pe: &PeData<'_>,
    seed: u32,
    records: u32,
    stride: u32,
) -> Option<Vec<Vec<u8>>> {
    let seed = pe.va_slice(u64::from(seed), 128).ok()?;
    let end = seed.iter().position(|&b| b == 0)?;
    if end == 0 {
        return None;
    }
    (0..4)
        .map(|i| {
            let value = pe
                .va_slice(u64::from(records) + i * u64::from(stride), 9)
                .ok()?;
            salt(&rc4_crypt(value, &seed[..end]).ok()?)
        })
        .collect()
}

fn inline_properties(pe: &PeData<'_>, exe: &[u8], stack: bool) -> Result<Option<Vec<Vec<u8>>>> {
    let mut found = None;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let start = section.file_offset as usize;
        let code = &exe[start..start + section.file_size as usize];
        for block in code.windows(27) {
            let candidate = if stack {
                // Store the seed VA on the stack, load the first two ciphertext words.
                if block[..2] != [0xc7, 0x85]
                    || block[10] != 0xa1
                    || block[15..17] != [0x89, 0x85]
                    || block[21..23] != [0x8b, 0x0d]
                    || word(&block[11..]).checked_add(4) != Some(word(&block[23..]))
                {
                    continue;
                }
                decrypt_properties(pe, word(&block[6..]), word(&block[11..]), 12)
            } else {
                // push record; mov edx, seed; call; add esp,4; push next record
                if block[0] != 0x68
                    || block[5] != 0xba
                    || block[10] != 0xe8
                    || block[15..19] != [0x83, 0xc4, 4, 0x68]
                    || word(&block[1..]).checked_add(9) != Some(word(&block[19..]))
                {
                    continue;
                }
                decrypt_properties(pe, word(&block[6..]), word(&block[1..]), 9)
            };
            if let Some(values) = candidate {
                ensure!(
                    found.as_ref().is_none_or(|old| old == &values),
                    "ambiguous native PAZ properties"
                );
                found = Some(values);
            }
        }
    }
    Ok(found)
}

// Both long-table literal/drop readers pass two 32-byte slices to the same
// initializer. Their index address agrees relative to the names, but the
// member table is independently placed. Resolve the second native operand.
fn paired_member_distance(
    pe: &PeData<'_>,
    exe: &[u8],
    names_va: u32,
    index_distance: usize,
) -> Result<usize> {
    let index = names_va
        .checked_sub(u32::try_from(index_distance)?)
        .context("PAZ index address")?;
    let mut distance = None;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let start = section.file_offset as usize;
        let code = &exe[start..start + section.file_size as usize];
        for pair in code
            .windows(47)
            .filter(|p| p.starts_with(&[0xc1, 0xe0, 5, 5]))
        {
            // shl eax,5 / add eax,index / push eax; ... push 32;
            // mov ecx,[ebp+slot] / shl ecx,5 / add ecx,members / push ecx.
            if word(&pair[4..]) != index
                || pair[8] != 0x50
                || pair[24] != 0xe8
                || pair[29..33] != [0x6a, 0x20, 0x8b, 0x8d]
                || pair[37..42] != [0xc1, 0xe1, 5, 0x81, 0xc1]
                || pair[46] != 0x51
            {
                continue;
            }
            let d = names_va
                .checked_sub(word(&pair[42..]))
                .context("invalid PAZ member table address")?;
            ensure!(
                d as usize >= LONG.len() * 32 && (d as usize) <= index_distance - LONG.len() * 32,
                "overlapping PAZ index, member or name tables"
            );
            ensure!(
                distance.is_none_or(|old| old == d),
                "ambiguous PAZ member table address"
            );
            distance = Some(d);
        }
    }
    Ok(distance.context("missing paired PAZ table initializer")? as usize)
}

pub(super) fn recover(
    exe: &[u8],
    stem: &str,
    prefix: &[u8],
    size: usize,
    ctx: &DecodeContext,
) -> Result<Option<Parameters>> {
    let pe = PeData::parse(exe)?;
    let sections: Vec<_> = pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x40 != 0 && s.characteristics & 0x2000_0000 == 0)
        .collect();
    let mut markers = BTreeSet::new();
    for s in &sections {
        let start = s.file_offset as usize;
        let bytes = &exe[start..start + s.file_size as usize];
        for (i, _) in bytes.windows(4).enumerate().filter(|(_, w)| *w == b"scr\0") {
            if let Ok(addr) = u32::try_from(pe.image_base() + u64::from(s.rva) + i as u64) {
                markers.insert(addr);
                ensure!(markers.len() <= 1024, "too many PAZ bank marker candidates");
            }
        }
    }
    let mut found = None;
    for section in sections {
        ctx.check_cancelled()?;
        let start = section.file_offset as usize;
        let raw = &exe[start..start + section.file_size as usize];
        for (j, _) in raw
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, ptr)| markers.contains(&word(ptr)))
        {
            ctx.check_cancelled()?;
            let base = 4 * j;
            for &(names, index_distance, data_distance, cipher) in &[
                (
                    &["scr", "bg", "st", "sys", "bgm", "voice", "se", "se", "mov"][..],
                    0x240,
                    Some(0x120),
                    MemberCipher::SinglePermutation,
                ),
                (SHORT, 0x280, Some(0x140), MemberCipher::BlowfishOnly),
                (SHORT, 0x280, Some(0x140), MemberCipher::Rc4),
                (LONG, 0x300, Some(0x180), MemberCipher::Rc4),
                (LONG, 0x480, Some(0x240), MemberCipher::Rc4Drop),
                (SHORT, 0x500, Some(0x280), MemberCipher::Rc4),
                (LONG, 0x460, None, MemberCipher::Rc4Drop),
            ] {
                if base < index_distance
                    || !names.iter().enumerate().all(|(n, name)| {
                        raw.get(base + 4 * n..base + 4 * n + 4)
                            .and_then(|w| pe.va_slice(u64::from(word(w)), name.len() + 1).ok())
                            .is_some_and(|v| {
                                v.last() == Some(&0) && &v[..name.len()] == name.as_bytes()
                            })
                    })
                {
                    continue;
                }
                let Some(slot) = names.iter().position(|name| *name == stem) else {
                    continue;
                };
                let key = |distance| {
                    let a = base - distance + 32 * slot;
                    raw[a..a + 32]
                        .iter()
                        .enumerate()
                        .map(|(i, b)| match cipher {
                            MemberCipher::BlowfishOnly => b ^ b"minori"[i % 6],
                            _ => b.wrapping_neg(),
                        })
                        .collect::<Vec<_>>()
                };
                let index = key(index_distance);
                let first = blowfish_le_decrypt(prefix, &index, 8)?;
                let count = word(&first) as usize;
                if count == 0 || count > ctx.limits.max_entries || count > (size - 4) / 25 {
                    continue;
                }
                let mut found_compact_drop = false;
                let salts = match cipher {
                    MemberCipher::SinglePermutation | MemberCipher::BlowfishOnly => vec![],
                    MemberCipher::Rc4Drop if index_distance == 0x480 => {
                        inline_properties(&pe, exe, true)?
                            .context("missing stacked PAZ properties")?
                    }
                    MemberCipher::Rc4 if index_distance == 0x280 => {
                        let table = pe.image_base() + u64::from(section.rva) + base as u64;
                        let compact = [(0x390, 0x268), (0x398, 0x30)].into_iter().find_map(
                            |(seed_distance, records_offset)| {
                                let at = base.checked_sub(seed_distance)?;
                                let seed = word(raw.get(at..at + 4)?);
                                let records = u32::try_from(table + records_offset).ok()?;
                                decrypt_properties(&pe, seed, records, 9)
                            },
                        );
                        if let Some(values) = compact {
                            values
                        } else {
                            let Some(values) = inline_properties(&pe, exe, false)? else {
                                continue;
                            };
                            found_compact_drop = true;
                            values
                        }
                    }
                    MemberCipher::Rc4 if index_distance == 0x300 => literal_salts(&pe, exe)?,
                    MemberCipher::Rc4Drop => literal_salts(&pe, exe)?,
                    MemberCipher::Rc4 => {
                        let key_at = base
                            .checked_sub(0x618)
                            .context("PAZ parameter seed pointer")?;
                        let seed = pe.va_slice(u64::from(word(&raw[key_at..])), 256)?;
                        let end = seed
                            .iter()
                            .position(|v| *v == 0)
                            .context("unterminated PAZ parameter seed")?;
                        let values = raw
                            .get(base + 0x30..base + 0x30 + 36)
                            .context("truncated PAZ member parameters")?;
                        values
                            .chunks_exact(9)
                            .map(|v| {
                                salt(&rc4_crypt(v, &seed[..end])?)
                                    .context("invalid PAZ member parameter")
                            })
                            .collect::<Result<Vec<_>>>()?
                    }
                };
                let data_distance = if let Some(distance) = data_distance {
                    distance
                } else {
                    let names_va =
                        u32::try_from(pe.image_base() + u64::from(section.rva) + base as u64)?;
                    paired_member_distance(&pe, exe, names_va, index_distance)?
                };
                let members = (!salts.is_empty()).then(|| {
                    ["script", "audio", "image"]
                        .into_iter()
                        .zip(salts.iter().cloned())
                        .collect()
                });
                ensure!(found.is_none(), "ambiguous native minori PAZ table");
                found = Some(Parameters {
                    index,
                    data: (stem != "mov").then(|| key(data_distance)),
                    salt: salts.last().cloned().unwrap_or_default(),
                    members,
                    cipher: if found_compact_drop {
                        MemberCipher::Rc4Drop
                    } else {
                        cipher
                    },
                });
            }
        }
    }
    Ok(found)
}
