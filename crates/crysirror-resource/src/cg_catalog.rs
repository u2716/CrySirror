// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Shared bounded CGTABLE/CGTABLE2 catalogue decoding. Layout reference:
//! xmoezzz/siglus_rs 5f358ef1360b0439e92aed2b749e8f25d8721954,
//! crates/siglus_assets/src/cgm.rs. No VM, filesystem or progress flags.
use crate::{DecodeContext, DecodedFile, FileFormat, FileKind, Probe};
use anyhow::{ensure, Result};
use crysirror_core::service::AssetReader;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub const MAX_CATALOG_ENTRIES: usize = 65_536;
const MAX_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CgCatalogEntry {
    pub name: String,
    pub flag_no: i32,
    pub codes: [i32; 5],
    pub code_count: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CgCatalog {
    pub version: u8,
    pub auto_flag: i32,
    pub entries: Vec<CgCatalogEntry>,
}
impl CgCatalog {
    pub fn by_name(&self, name: &str) -> Option<&CgCatalogEntry> {
        self.entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }
    pub fn by_flag(&self, flag: i32) -> Option<&CgCatalogEntry> {
        self.entries.iter().find(|e| e.flag_no == flag)
    }
}
pub fn decode(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<CgCatalog> {
    check()?;
    ensure!((40..=MAX_BYTES).contains(&bytes.len()), "CGM encoded size");
    let ident = &bytes[..16];
    let end = ident.iter().position(|b| *b == 0).unwrap_or(16);
    let version = match &ident[..end] {
        b"CGTABLE" => 1,
        b"CGTABLE2" => 2,
        _ => anyhow::bail!("CGM signature"),
    };
    let count = i32::from_le_bytes(bytes[16..20].try_into()?);
    ensure!(
        count > 0 && count as usize <= MAX_CATALOG_ENTRIES,
        "CGM entry count"
    );
    let count = count as usize;
    let stride = if version == 1 { 36 } else { 60 };
    let mut packed = bytes[32..].to_vec();
    for (i, b) in packed.iter_mut().enumerate() {
        if i % 4096 == 0 {
            check()?;
        }
        *b ^= KEY[i & 255];
    }
    let size = crate::backref_lzss::lzss_decompressed_size(&packed)?;
    ensure!(
        size >= count * stride && size <= MAX_BYTES,
        "CGM expanded layout"
    );
    let expanded = crate::backref_lzss::lzss_unpack_lenient(&packed)?;
    check()?;
    let mut entries = Vec::with_capacity(count);
    for row in expanded.chunks_exact(stride).take(count) {
        check()?;
        let end = row[..32].iter().position(|b| *b == 0).unwrap_or(32);
        let (name, _, bad) = encoding_rs::SHIFT_JIS.decode(&row[..end]);
        ensure!(!bad, "CGM invalid Shift-JIS name");
        let flag_no = i32::from_le_bytes(row[32..36].try_into()?);
        let mut codes = [0; 5];
        let mut code_count = 0;
        if version == 2 {
            for (i, code) in codes.iter_mut().enumerate() {
                *code = i32::from_le_bytes(row[36 + i * 4..40 + i * 4].try_into()?);
            }
            code_count = i32::from_le_bytes(row[56..60].try_into()?);
            ensure!((0..=5).contains(&code_count), "CGM group code count");
        }
        entries.push(CgCatalogEntry {
            name: name.to_ascii_uppercase(),
            flag_no,
            codes,
            code_count,
        });
    }
    ensure!(entries.len() == count, "CGM truncated rows");
    Ok(CgCatalog {
        version,
        auto_flag: i32::from_le_bytes(bytes[20..24].try_into()?),
        entries,
    })
}
/// Each entry's list number and group indices from the original CG group
/// tree: entries sorted by their five codes (then list number) and grouped
/// level by level; an entry's group at a level counts the distinct codes
/// before it under the same prefix. Adopted from siglus_rs 256c398
/// crates/siglus_assets/src/cgm.rs `create_sort_list`/`create_group_tree`
/// (MPL-2.0). Shared by every VM that exposes CG-mode group queries.
pub fn group_tree(catalog: &CgCatalog) -> Vec<(i32, [i32; 5])> {
    const CODES: usize = 5;
    let entries = &catalog.entries;
    let mut groups = vec![[0i32; CODES]; entries.len()];
    let mut sort_list: Vec<usize> = (0..entries.len()).collect();
    sort_list.sort_by(|&a, &b| entries[a].codes.cmp(&entries[b].codes).then(a.cmp(&b)));
    fn rec(
        entries: &[CgCatalogEntry],
        groups: &mut [[i32; 5]],
        sort_list: &[usize],
        start: usize,
        code: &mut [i32; 5],
        code_index: usize,
    ) {
        let mut end = start;
        while end < sort_list.len()
            && (0..code_index).all(|i| entries[sort_list[end]].codes[i] == code[i])
        {
            end += 1;
        }
        if start >= end {
            return;
        }
        let mut now_code = -1i32;
        let mut group_no = 0i32;
        let mut idx = start;
        while idx < end {
            let sub = sort_list[idx];
            let sub_code = entries[sub].codes[code_index];
            if sub_code != now_code || code_index == CODES - 1 {
                if now_code != -1 {
                    group_no += 1;
                }
                now_code = sub_code;
                if code_index + 1 < CODES {
                    code[code_index] = now_code;
                    rec(entries, groups, sort_list, idx, code, code_index + 1);
                }
            }
            groups[sub][code_index] = group_no;
            idx += 1;
        }
    }
    if !sort_list.is_empty() {
        let mut code = [-1i32; CODES];
        rec(entries, &mut groups, &sort_list, 0, &mut code, 0);
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(i, g)| (i as i32, g))
        .collect()
}
/// Public file decoder exposes the same catalogue as bounded JSON metadata.
pub struct CgCatalogFormat;
impl FileFormat for CgCatalogFormat {
    fn id(&self) -> &'static str {
        "siglus/cgm"
    }
    fn kind(&self) -> FileKind {
        FileKind::File
    }
    fn probe(&self, _path: &str, source: &dyn AssetReader) -> Result<Probe> {
        if source.len()? < 40 {
            return Ok(Probe::NO);
        }
        let mut head = [0; 16];
        crate::read_exact_at(source, 0, &mut head)?;
        Ok(
            if head.starts_with(b"CGTABLE\0") || head.starts_with(b"CGTABLE2\0") {
                Probe::YES
            } else {
                Probe::NO
            },
        )
    }
    fn decode(
        &self,
        _path: &str,
        source: Arc<dyn AssetReader>,
        context: &DecodeContext,
    ) -> Result<DecodedFile> {
        let reader = context.with_temporary_bytes(48 * 1024 * 1024, || {
            let source = crate::read_all(source.as_ref(), MAX_BYTES as u64)?;
            context.decode_bounded(32 * 1024 * 1024, || {
                let catalog = decode(&source, &|| context.check_cancelled())?;
                let output = serde_json::to_vec(&catalog)?;
                ensure!(output.len() <= 32 * 1024 * 1024, "CGM JSON output budget");
                Ok(output)
            })
        })?;
        Ok(DecodedFile {
            reader,
            extension: "json".into(),
            metadata: serde_json::json!({"kind":"cg_catalog"}),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encoded(version: u8) -> Vec<u8> {
        let stride = if version == 1 { 36 } else { 60 };
        let mut raw = vec![0; stride * 2];
        for (i, name) in [b"first".as_slice(), b"SECOND".as_slice()]
            .iter()
            .enumerate()
        {
            let row = &mut raw[i * stride..(i + 1) * stride];
            row[..name.len()].copy_from_slice(name);
            row[32..36].copy_from_slice(&(i as i32 + 7).to_le_bytes());
            if version == 2 {
                row[36..40].copy_from_slice(&13i32.to_le_bytes());
                row[56..60].copy_from_slice(&1i32.to_le_bytes());
            }
        }
        let mut packed = vec![0; 8];
        packed[4..8].copy_from_slice(&(raw.len() as u32).to_le_bytes());
        for part in raw.chunks(8) {
            packed.push(255);
            packed.extend(part);
        }
        let size = packed.len() as u32;
        packed[..4].copy_from_slice(&size.to_le_bytes());
        let mut out = vec![0; 32];
        let magic = if version == 1 {
            b"CGTABLE".as_slice()
        } else {
            b"CGTABLE2".as_slice()
        };
        out[..magic.len()].copy_from_slice(magic);
        out[16..20].copy_from_slice(&2i32.to_le_bytes());
        out.extend(
            packed
                .into_iter()
                .enumerate()
                .map(|(i, b)| b ^ KEY[i & 255]),
        );
        out
    }
    #[test]
    fn group_tree_counts_distinct_codes_per_prefix() {
        let e = |codes: [i32; 5]| CgCatalogEntry { name: String::new(), flag_no: 0, codes, code_count: 2 };
        let catalog = CgCatalog { version: 2, auto_flag: 0, entries: vec![e([1, 0, 0, 0, 0]), e([0, 5, 0, 0, 0]), e([0, 7, 0, 0, 0]), e([1, 3, 0, 0, 0])] };
        let tree = group_tree(&catalog);
        assert_eq!(tree.iter().map(|t| t.0).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        assert_eq!(tree[1].1[..2], [0, 0]);
        assert_eq!(tree[2].1[..2], [0, 1]);
        assert_eq!(tree[0].1[..2], [1, 0]);
        assert_eq!(tree[3].1[..2], [1, 1]);
    }
    #[test]
    fn both_versions_are_reusable_file_resources_with_exact_metadata() {
        for version in [1, 2] {
            let bytes = encoded(version);
            let catalog = decode(&bytes, &|| Ok(())).unwrap();
            assert_eq!(catalog.version, version);
            assert_eq!(catalog.by_name("FiRsT").unwrap().flag_no, 7);
            assert_eq!(catalog.by_flag(8).unwrap().name, "SECOND");
            assert!(catalog.by_flag(-1).is_none());
            let context = DecodeContext::new(crysirror_core::state::ClientId(1));
            let decoded = CgCatalogFormat
                .decode("catalog.cgm", crate::io::memory(bytes), &context)
                .unwrap();
            let json = crate::read_all(decoded.reader.as_ref(), 32 * 1024 * 1024).unwrap();
            assert_eq!(serde_json::from_slice::<CgCatalog>(&json).unwrap(), catalog);
        }
    }
    #[test]
    fn invalid_sizes_and_cancellation_fail_before_results_escape() {
        let mut bad = encoded(2);
        bad[16..20].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(decode(&bad, &|| Ok(())).is_err());
        assert!(decode(&encoded(1), &|| anyhow::bail!("cancelled")).is_err());
        let context = DecodeContext::new(crysirror_core::state::ClientId(1));
        assert!(CgCatalogFormat
            .decode("bad", crate::io::memory(bad), &context)
            .is_err());
        assert!(CgCatalogFormat
            .decode("good", crate::io::memory(encoded(1)), &context)
            .is_ok());
    }
    #[test]
    #[ignore = "reads owner-installed Rewrite catalogues; no game data included"]
    fn owner_rewrite_catalogues_decode_with_the_shared_format() {
        for root in ["gamedata_rewrite_hf", "GameData_rewrite_plus"] {
            let path = std::path::Path::new("/Users/xmoe/Downloads")
                .join(root)
                .join("dat/mode.cgm");
            let catalog = decode(&std::fs::read(path).unwrap(), &|| Ok(())).unwrap();
            assert!(!catalog.entries.is_empty());
            assert!(catalog.entries.iter().all(|e| e.flag_no < 1_048_576));
            eprintln!("{root}: {} CG entries", catalog.entries.len());
        }
    }
}

const KEY: [u8; 256] = [
    0x8b, 0xe5, 0x5d, 0xc3, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x85, 0xc0, 0x74, 0x09, 0x5f, 0x5e, 0x33,
    0xc0, 0x5b, 0x8b, 0xe5, 0x5d, 0xc3, 0x8b, 0x45, 0x0c, 0x85, 0xc0, 0x75, 0x14, 0x8b, 0x55, 0xec,
    0x83, 0xc2, 0x20, 0x52, 0x6a, 0x00, 0xe8, 0xf5, 0x28, 0x01, 0x00, 0x83, 0xc4, 0x08, 0x89, 0x45,
    0x0c, 0x8b, 0x45, 0xe4, 0x6a, 0x00, 0x6a, 0x00, 0x50, 0x53, 0xff, 0x15, 0x34, 0xb1, 0x43, 0x00,
    0x8b, 0x45, 0x10, 0x85, 0xc0, 0x74, 0x05, 0x8b, 0x4d, 0xec, 0x89, 0x08, 0x8a, 0x45, 0xf0, 0x84,
    0xc0, 0x75, 0x78, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x8b, 0x7d, 0xe8, 0x8b, 0x75, 0x0c, 0x85, 0xc0,
    0x75, 0x44, 0x8b, 0x1d, 0xd0, 0xb0, 0x43, 0x00, 0x85, 0xff, 0x76, 0x37, 0x81, 0xff, 0x00, 0x00,
    0x04, 0x00, 0x6a, 0x00, 0x76, 0x43, 0x8b, 0x45, 0xf8, 0x8d, 0x55, 0xfc, 0x52, 0x68, 0x00, 0x00,
    0x04, 0x00, 0x56, 0x50, 0xff, 0x15, 0x2c, 0xb1, 0x43, 0x00, 0x6a, 0x05, 0xff, 0xd3, 0xa1, 0xe0,
    0x30, 0x44, 0x00, 0x81, 0xef, 0x00, 0x00, 0x04, 0x00, 0x81, 0xc6, 0x00, 0x00, 0x04, 0x00, 0x85,
    0xc0, 0x74, 0xc5, 0x8b, 0x5d, 0xf8, 0x53, 0xe8, 0xf4, 0xfb, 0xff, 0xff, 0x8b, 0x45, 0x0c, 0x83,
    0xc4, 0x04, 0x5f, 0x5e, 0x5b, 0x8b, 0xe5, 0x5d, 0xc3, 0x8b, 0x55, 0xf8, 0x8d, 0x4d, 0xfc, 0x51,
    0x57, 0x56, 0x52, 0xff, 0x15, 0x2c, 0xb1, 0x43, 0x00, 0xeb, 0xd8, 0x8b, 0x45, 0xe8, 0x83, 0xc0,
    0x20, 0x50, 0x6a, 0x00, 0xe8, 0x47, 0x28, 0x01, 0x00, 0x8b, 0x7d, 0xe8, 0x89, 0x45, 0xf4, 0x8b,
    0xf0, 0xa1, 0xe0, 0x30, 0x44, 0x00, 0x83, 0xc4, 0x08, 0x85, 0xc0, 0x75, 0x56, 0x8b, 0x1d, 0xd0,
    0xb0, 0x43, 0x00, 0x85, 0xff, 0x76, 0x49, 0x81, 0xff, 0x00, 0x00, 0x04, 0x00, 0x6a, 0x00, 0x76,
];
