// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 CrySirror contributors.
//! minori PAZ banks backed by the requesting client's archive and executable readers.
// SATD-minori-paz-verification: retail reference reads are tracked by the adapter.
mod native;
mod profiles;
use crate::common::{Cursor, bytes, length};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
};
use crysirror_resource::{
    Archive, ArchiveEntry, ArchiveFormat, DecodeContext, Probe, blowfish_le_decrypt, concat,
    rc4_crypt, read_all,
};
use native::{MemberCipher, Parameters};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Copy)]
struct Fingerprint {
    length: usize,
    tag: u16,
    digest: &'static str,
}
impl Fingerprint {
    const fn new(length: usize, tag: u16, digest: &'static str) -> Self {
        Self {
            length,
            tag,
            digest,
        }
    }
    fn find(self, exe: &[u8], ctx: &DecodeContext) -> Result<Vec<u8>> {
        self.locate(exe, ctx)?
            .context("missing minori PAZ key parameter")
    }
    fn locate(self, exe: &[u8], ctx: &DecodeContext) -> Result<Option<Vec<u8>>> {
        for (at, window) in exe.windows(self.length).enumerate() {
            if at % 65536 == 0 {
                ctx.check_cancelled()?;
            }
            let tag = (u32::from(window[0]) * 251 * 251
                + u32::from(window[1]) * 251
                + u32::from(window[2])) as u16;
            if tag == self.tag && format!("{:x}", Sha256::digest(window)) == self.digest {
                return Ok(Some(window.to_vec()));
            }
        }
        Ok(None)
    }
}
struct Member {
    offset: u64,
    stored: u64,
    aligned: u64,
    compressed: bool,
    native_name: String,
}
struct Paz {
    source: Arc<dyn AssetReader>,
    entries: Vec<ArchiveEntry>,
    members: Vec<Member>,
    data_key: Option<Vec<u8>>,
    salt: Vec<u8>,
    xor: u8,
    cipher: MemberCipher,
    movie: Option<Vec<u8>>,
    member_salts: Option<BTreeMap<&'static str, Vec<u8>>>,
}
pub struct MinoriPaz;
impl ArchiveFormat for MinoriPaz {
    fn id(&self) -> &'static str {
        "minori/paz"
    }
    fn probe(&self, _: &dyn AssetReader) -> Result<Probe> {
        // No universal file signature. Installed-game detection supplies an explicit profile.
        Ok(Probe { confidence: 0 })
    }
    fn open(&self, source: Arc<dyn AssetReader>, ctx: &DecodeContext) -> Result<Box<dyn Archive>> {
        let opt = ctx
            .parameters
            .get(self.id())
            .context("minori PAZ requires installed profile")?;
        // Previously exported mount plans used a game label in this field.
        let legacy_member_mode = match opt.get("abi") {
            None => None,
            Some(v) if v == "ef-first-aa" => Some(false),
            Some(v) if v == "eden-retail" => Some(true),
            Some(_) => anyhow::bail!("unknown legacy minori PAZ profile"),
        };
        let name = ctx
            .source_name
            .as_deref()
            .context("missing PAZ bank name")?
            .rsplit('/')
            .next()
            .unwrap();
        let lower = name.to_ascii_lowercase();
        let stem = lower.strip_suffix(".paz").context("expected .paz source")?;
        let exe = read_all(
            ctx.open_sibling(
                opt["executable"]
                    .as_str()
                    .context("missing original executable")?,
            )?
            .as_ref(),
            64 << 20,
        )?;
        ensure!(exe.starts_with(b"MZ"), "minori PE header");
        let prefix = bytes(source.as_ref(), 0, 36, 36)?;
        // Headerless banks begin directly with a block-aligned index length.
        // The legacy native table and decrypted directory independently confirm this layout.
        let headerless_len = u32::from_le_bytes(prefix[..4].try_into()?);
        let headerless = headerless_len >= 8
            && headerless_len.is_multiple_of(8)
            && u64::from(headerless_len) <= ctx.limits.max_index_size
            && prefix[3] == 0;
        let header_size = if headerless { 4 } else { 36 };
        let size = &prefix[header_size - 4..header_size];
        let xor = size[3];
        let n = u32::from_le_bytes(std::array::from_fn(|i| size[i] ^ xor)) as u64;
        ensure!(n >= 8 && n.is_multiple_of(8), "minori PAZ index size");
        let mut raw = bytes(
            source.as_ref(),
            header_size as u64,
            n,
            ctx.limits.max_index_size,
        )?;
        for b in &mut raw {
            *b ^= xor;
        }
        let native = native::recover(&exe, stem, &raw[..8], raw.len(), ctx)?
            .filter(|p| legacy_member_mode.is_none_or(|mode| mode == p.members.is_some()));
        let Parameters {
            index: index_key,
            data: data_key,
            salt,
            members: member_salts,
            cipher,
        } = match native {
            Some(native) => native,
            None => (|| -> Result<Parameters> {
                let recover_key = |id: Fingerprint| -> Result<Vec<u8>> {
                    Ok(id
                        .find(&exe, ctx)?
                        .into_iter()
                        .map(u8::wrapping_neg)
                        .collect())
                };
                let mut matched = None;
                for table in profiles::TABLES {
                    if legacy_member_mode.is_some_and(|mode| mode != table.member_salts.is_some()) {
                        continue;
                    }
                    for &(_, index, data, salt) in table.banks.iter().filter(|row| row.0 == stem) {
                        if let Some(raw) = index.locate(&exe, ctx)? {
                            ensure!(
                                matched.is_none(),
                                "ambiguous minori PAZ key tables for bank {stem}"
                            );
                            let key: Vec<_> = raw.into_iter().map(u8::wrapping_neg).collect();
                            matched = Some((key, data, salt, table.member_salts));
                        }
                    }
                }
                let (index_key, data_id, salt_id, salt_parameters) = matched
                    .with_context(|| format!("no known PAZ index parameters for bank {stem}"))?;
                let data_key = data_id.map(recover_key).transpose()?;
                let salt = salt_id.find(&exe, ctx)?;
                let members = salt_parameters
                    .map(|p| {
                        let mut resolved = BTreeMap::new();
                        for (name, fp) in
                            [("script", p.script), ("audio", p.audio), ("image", p.image)]
                        {
                            if let Some(value) = fp.locate(&exe, ctx)? {
                                resolved.insert(name, value);
                            }
                        }
                        Ok::<_, anyhow::Error>(resolved)
                    })
                    .transpose()?;
                Ok(Parameters {
                    index: index_key,
                    data: data_key,
                    salt,
                    members,
                    cipher: MemberCipher::Rc4,
                })
            })()?,
        };
        ensure!(
            headerless == (cipher == MemberCipher::SinglePermutation),
            "minori PAZ header and native table disagree"
        );
        let index = blowfish_le_decrypt(&raw, &index_key, raw.len())?;
        let mut c = Cursor::new(&index);
        let count = c.le32()? as usize;
        ensure!(
            count > 0 && count <= ctx.limits.max_entries && count <= c.left() / 25,
            "minori PAZ entry count"
        );
        let movie = if stem == "mov" {
            if matches!(
                cipher,
                MemberCipher::BlowfishOnly | MemberCipher::SinglePermutation
            ) {
                let mut rows = c.take(if headerless { 256 } else { 65536 })?.to_vec();
                for row in rows.chunks_exact_mut(256) {
                    let mut inverse = [0u8; 256];
                    let mut seen = [false; 256];
                    for (i, &b) in row.iter().enumerate() {
                        ensure!(!seen[b as usize], "invalid PAZ movie permutation");
                        seen[b as usize] = true;
                        inverse[b as usize] = i as u8;
                    }
                    row.copy_from_slice(&inverse);
                }
                Some(rows)
            } else {
                Some(c.take(256)?.to_vec())
            }
        } else {
            None
        };
        let mut sources = vec![source];
        if let Some(parts) = opt.get("parts").and_then(|parts| parts.get(stem)) {
            let parts = parts.as_array().context("PAZ split volumes")?;
            ensure!(parts.len() <= 26, "PAZ split volume limit");
            for (i, part) in parts.iter().enumerate() {
                let path = part.as_str().context("PAZ split volume name")?;
                ensure!(
                    path.eq_ignore_ascii_case(&format!("{name}{}", (b'A' + i as u8) as char)),
                    "PAZ split volume sequence"
                );
                sources.push(ctx.open_sibling(path)?);
            }
        }
        let source = concat(sources)?;
        let total = length(source.as_ref())?;
        let mut entries = Vec::with_capacity(count);
        let mut members = Vec::with_capacity(count);
        let mut paths = BTreeSet::new();
        for id in 0..count {
            ctx.check_cancelled()?;
            let encoded = c.zero()?;
            ensure!(encoded.len() <= 4096, "minori filename length");
            let native_name = encoding_rs::SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(encoded)
                .context("invalid PAZ filename")?
                .into_owned();
            let path = native_name.replace('\\', "/").to_ascii_lowercase();
            ensure!(
                VfsPath::new(Mount::Asset, &path)?.relative() == path && paths.insert(path.clone()),
                "invalid or duplicate PAZ entry"
            );
            let offset = c.le64()?;
            let size = u64::from(c.le32()?);
            let stored = u64::from(c.le32()?);
            let aligned = u64::from(c.le32()?);
            let flags = c.le32()?;
            let trailing = member_salts.is_some() && movie.is_none() && aligned < 8;
            let span = if movie.is_some() {
                size
            } else if trailing {
                total.checked_sub(offset).context("PAZ tail offset")?
            } else {
                aligned
            };
            ensure!(
                flags <= 1
                    && stored <= span
                    && offset >= header_size as u64 + n
                    && offset.checked_add(span).is_some_and(|v| v <= total),
                "invalid PAZ member extent"
            );
            if movie.is_none() && !trailing {
                ensure!(span.is_multiple_of(8), "unaligned PAZ encrypted member");
            }
            entries.push(ArchiveEntry {
                id: id as u32,
                path,
                size,
                stored_size: span,
                metadata: serde_json::Value::Null,
            });
            members.push(Member {
                offset,
                stored,
                aligned: span,
                compressed: flags == 1,
                native_name,
            });
        }
        ensure!(c.left() < 8, "unexpected trailing PAZ index records");
        Ok(Box::new(Paz {
            source,
            entries,
            members,
            data_key,
            salt,
            xor,
            movie,
            member_salts,
            cipher,
        }))
    }
}
fn member_type(name: &str) -> Option<&'static str> {
    if name.ends_with(".sc") {
        Some("script")
    } else if name.ends_with(".png") {
        Some("image")
    } else if name.ends_with(".ogg") || !name.contains('.') {
        Some("audio")
    } else {
        None
    }
}
impl Archive for Paz {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn open(&self, id: u32, ctx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        let entry = self
            .entries
            .get(id as usize)
            .context("unknown PAZ entry id")?;
        let m = &self.members[id as usize];
        let make_key = |salt: &[u8]| -> Result<Vec<u8>> {
            let text = format!(
                "{} {:08X} {}",
                m.native_name.to_ascii_lowercase(),
                entry.size,
                std::str::from_utf8(salt)?
            );
            let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&text);
            ensure!(!errors, "PAZ key text encoding");
            Ok(encoded.into_owned())
        };
        if let Some(table) = &self.movie {
            ctx.check_cancelled()?;
            ensure!(!m.compressed, "unsupported compressed minori movie");
            let source = crysirror_resource::subrange(self.source.clone(), m.offset, entry.size)?;
            if matches!(
                self.cipher,
                MemberCipher::BlowfishOnly | MemberCipher::SinglePermutation
            ) {
                return Ok(Arc::new(PermutationMovie {
                    source,
                    rows: table.clone(),
                    xor: self.xor,
                }));
            }
            let key = make_key(&self.salt)?;
            let key: Vec<_> = table
                .iter()
                .enumerate()
                .map(|(i, v)| v ^ key[i % key.len()])
                .collect();
            let mut stream = rc4_crypt(&vec![0; 65536], &key)?;
            for byte in &mut stream {
                *byte ^= self.xor
            }
            return Ok(Arc::new(MovieReader { source, stream }));
        }
        ensure!(
            m.aligned <= ctx.limits.max_entry_size.saturating_add(7),
            "PAZ member read budget"
        );
        let working = m
            .aligned
            .checked_mul(2)
            .context("PAZ working byte size")?
            .max(entry.size);
        ctx.with_temporary_bytes(working, || {
            ctx.decode_with(entry.size, || {
                ctx.check_cancelled()?;
                let mut raw = bytes(self.source.as_ref(), m.offset, m.aligned, m.aligned)?;
                for b in &mut raw {
                    *b ^= self.xor;
                }
                let key = self.data_key.as_deref().context("missing PAZ data key")?;
                let mut clear = blowfish_le_decrypt(&raw, key, raw.len())?;
                drop(raw);
                if m.compressed {
                    clear.truncate(m.stored as usize);
                    let mut z = flate2::Decompress::new(true);
                    let mut out = vec![
                        0;
                        usize::try_from(entry.size)?
                            .checked_add(1)
                            .context("PAZ inflated size")?
                    ];
                    ensure!(
                        z.decompress(&clear, &mut out, flate2::FlushDecompress::Finish)?
                            == flate2::Status::StreamEnd
                            && z.total_out() == entry.size,
                        "PAZ compressed size mismatch"
                    );
                    out.truncate(entry.size as usize);
                    clear = out;
                } else {
                    ensure!(clear.len() as u64 >= entry.size, "short PAZ member");
                    clear.truncate(entry.size as usize);
                }
                if let Some(salts) = &self.member_salts {
                    if let Some(kind) = member_type(&entry.path) {
                        let salt = salts
                            .get(kind)
                            .with_context(|| format!("missing PAZ {kind} parameters"))?;
                        let key = make_key(salt)?;
                        if self.cipher == MemberCipher::Rc4Drop {
                            let mut crc = flate2::Crc::default();
                            crc.update(&key);
                            let skip = ((crc.sum() >> 12) & 255) as usize;
                            clear.splice(0..0, std::iter::repeat_n(0, skip));
                            clear = rc4_crypt(&clear, &key)?;
                            clear.drain(..skip);
                        } else {
                            clear = rc4_crypt(&clear, &key)?;
                        }
                    }
                } else if !m.compressed
                    && !matches!(
                        self.cipher,
                        MemberCipher::BlowfishOnly | MemberCipher::SinglePermutation
                    )
                {
                    clear = rc4_crypt(&clear, &make_key(&self.salt)?)?;
                }
                Ok(clear)
            })
        })
    }
}

struct PermutationMovie {
    source: Arc<dyn AssetReader>,
    rows: Vec<u8>,
    xor: u8,
}
impl AssetReader for PermutationMovie {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.source.len()
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let n = self.source.read_at(offset, out)?;
        for (i, b) in out[..n].iter_mut().enumerate() {
            let row = ((offset + i as u64) >> 16) as u8;
            *b = self.rows[(usize::from(row) * 256) % self.rows.len() + usize::from(*b ^ self.xor)];
        }
        Ok(n)
    }
}

/// Each 64-KiB region of a native movie uses the same mask, so seeking never
/// needs to decode the prefix or keep the encoded movie resident.
struct MovieReader {
    source: Arc<dyn AssetReader>,
    stream: Vec<u8>,
}
impl AssetReader for MovieReader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.source.len()
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let n = self.source.read_at(offset, out)?;
        let start = (offset & 0xffff) as usize;
        for (i, byte) in out[..n].iter_mut().enumerate() {
            *byte ^= self.stream[(i + start) & 0xffff]
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;

    #[test]
    fn permutation_movie_changes_rows_at_member_relative_64k_boundaries() {
        let xor = 77;
        let rows: Vec<u8> = (0..65536)
            .map(|i| (i as u8).wrapping_sub((i >> 8) as u8))
            .collect();
        let plain: Vec<u8> = (0..131080).map(|i| (i % 251) as u8).collect();
        let encoded: Vec<u8> = plain
            .iter()
            .enumerate()
            .map(|(i, b)| b.wrapping_add((i >> 16) as u8) ^ xor)
            .collect();
        let reader = PermutationMovie {
            source: Arc::new(MemoryAssetReader(encoded.into())),
            rows,
            xor,
        };
        for at in [65530usize, 131065] {
            let mut buf = [0; 12];
            crysirror_resource::read_exact_at(&reader, at as u64, &mut buf).unwrap();
            assert_eq!(buf.as_slice(), &plain[at..at + 12]);
        }
    }

    #[test]
    fn movie_random_access_keeps_phase_across_volume_and_cipher_boundaries() {
        let backing =
            |v: Vec<u8>| -> Arc<dyn AssetReader> { Arc::new(MemoryAssetReader(v.into())) };
        let mask: Vec<_> = (0..65536u32)
            .map(|n| (n.wrapping_mul(79) >> 5) as u8)
            .collect();
        let plain: Vec<_> = (0..66000u32).map(|n| (n / 17) as u8).collect();
        let mut encoded: Vec<_> = plain
            .iter()
            .enumerate()
            .map(|(i, v)| v ^ mask[i & 0xffff])
            .collect();
        let part = encoded.split_off(65535);
        let source = concat(vec![backing(encoded), backing(part)]).unwrap();
        let reader = MovieReader {
            source,
            stream: mask,
        };
        let mut sample = [0; 17];
        crysirror_resource::read_exact_at(&reader, 65530, &mut sample).unwrap();
        assert_eq!(&sample, &plain[65530..65547]);
        assert_eq!(reader.read_at(66000, &mut sample).unwrap(), 0);
    }
}
