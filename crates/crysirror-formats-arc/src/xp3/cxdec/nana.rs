// Nana dls: profile and name cipher from xp3-brute PR #3 (docs/third-party/xp3-brute.txt).
use super::*;
use flate2::{Decompress, FlushDecompress, Status};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::xp3) struct Profile {
    name_state: [u32; 6],
    content: Content,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Content {
    mask: u32,
    offset: u32,
    prolog_order: [u8; 3],
    even_branch_order: [u8; 8],
    odd_branch_order: [u8; 6],
    random_seed: u32,
    control_block_file: String,
}

fn canonical(path: &str) -> Result<()> {
    use crysirror_core::path::{Mount, VfsPath};
    ensure!(
        VfsPath::new(Mount::Asset, path)?.relative() == path,
        "noncanonical Nana resource path"
    );
    Ok(())
}

pub(in crate::xp3) fn profile(
    specials: &[Special],
    ctx: &DecodeContext,
) -> Result<Option<(Profile, String)>> {
    let [descriptor] = specials else {
        return Ok(None);
    };
    if descriptor.tag != *b"dls:" {
        return Ok(None);
    }
    let option = ctx
        .parameters
        .get("kirikiri/xp3")
        .and_then(|p| p.get("nana_profile"));
    if option.is_none()
        && (super::requested(ctx) || adler_xor::requested(ctx) || hash_fold_xor::requested(ctx))
    {
        return Ok(None);
    }
    let relative = match option {
        Some(v) => v
            .as_str()
            .context("nana_profile must be a relative JSON filename")?,
        None => "nana-profile.json",
    };
    canonical(relative)?;
    let parent = ctx
        .source_name
        .as_deref()
        .and_then(|s| s.rsplit_once('/'))
        .map(|(p, _)| p)
        .unwrap_or("");
    let mut dirs = vec![parent];
    if option.is_none()
        && parent
            .rsplit('/')
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("arc"))
    {
        dirs.push(parent.rsplit_once('/').map_or("", |(p, _)| p));
    }
    for dir in dirs {
        let path = if dir.is_empty() {
            relative.into()
        } else {
            format!("{dir}/{relative}")
        };
        let source = match ctx.open_dependency(&path) {
            Ok(reader) => reader,
            Err(_) if option.is_none() => continue,
            Err(e) => return Err(e.context("open Nana profile")),
        };
        let profile: Profile = serde_json::from_slice(&read_all(source.as_ref(), 65536)?)?;
        canonical(&profile.content.control_block_file)?;
        let table = match path.rsplit_once('/') {
            Some((p, _)) => format!("{p}/{}", profile.content.control_block_file),
            None => profile.content.control_block_file.clone(),
        };
        return Ok(Some((profile, table)));
    }
    Ok(None)
}

fn decipher(words: [u32; 6], bytes: &mut [u8]) {
    let mut schedule = [words[0]; 27];
    let mut rotating = [words[1], words[2], words[3]];
    for i in 0..26 {
        let slot = &mut rotating[i % 3];
        *slot = slot.rotate_right(8).wrapping_add(schedule[i]) ^ i as u32;
        schedule[i + 1] = schedule[i].rotate_left(3) ^ *slot;
    }
    let seed = u64::from(words[4]) | (u64::from(words[5]) << 32);
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        let counter = seed ^ (i as u64 + 1);
        let (mut lo, mut hi) = (counter as u32, (counter >> 32) as u32);
        for &round in &schedule {
            hi = hi.rotate_right(8).wrapping_add(lo) ^ round;
            lo = lo.rotate_left(3) ^ hi;
        }
        let mask = u64::from(lo) | (u64::from(hi) << 32);
        for (b, k) in chunk.iter_mut().zip(mask.to_le_bytes()) {
            *b ^= k;
        }
    }
}

fn name_blob(base: &ParsedArchive, d: &Special, ctx: &DecodeContext) -> Result<Vec<u8>> {
    let limit = ctx
        .limits
        .max_index_size
        .min(special::MAX_NAMES_BYTES as u64);
    ensure!(
        u64::from(d.original) <= limit,
        "Nana decoded name index limit"
    );
    bytes(base.source.as_ref(), d.offset, u64::from(d.stored), limit)
}
fn authenticated_names(
    base: &ParsedArchive,
    original: usize,
    words: [u32; 6],
    mut data: Vec<u8>,
) -> Result<Vec<String>> {
    let n = data.len().min(256);
    decipher(words, &mut data[..n]);
    let mut out = vec![0; original + 1];
    let mut z = Decompress::new(true);
    ensure!(
        z.decompress(&data, &mut out, FlushDecompress::Finish)? == Status::StreamEnd
            && z.total_in() == data.len() as u64
            && z.total_out() == original as u64,
        "invalid Nana name index extent"
    );
    drop(data);
    let records = special::parse_records_limited(&out[..original], base.entries.len())?;
    drop(out);
    ensure!(
        records.len() == base.entries.len(),
        "Nana filename count mismatch"
    );
    let mut names = BTreeMap::new();
    for (hash, name) in records {
        let path = name.replace('\\', "/");
        canonical(&path)?;
        ensure!(
            names
                .insert(special::token("dls:", &path), (hash, path))
                .is_none(),
            "duplicate Nana name token"
        );
    }
    base.entries
        .iter()
        .map(|entry| {
            let (hash, path) = names
                .remove(&entry.path.to_ascii_lowercase())
                .context("unmapped Nana File token")?;
            ensure!(
                base.xp3(entry.id)
                    .context("invalid Nana File plan")?
                    .adler32
                    == hash,
                "Nana filename Adler-32 mismatch"
            );
            Ok(path)
        })
        .collect()
}

/// Only inspect initialized data addressed by an i386 PUSH/MOV immediate.
/// A reference supplies a candidate, never evidence that this game uses it.
fn referenced_states(
    raw: &[u8],
    stored: &[u8],
    states: &mut BTreeSet<[u32; 6]>,
    ctx: &DecodeContext,
) -> Result<()> {
    let Ok(image) = pe::PeImage::parse(raw) else {
        return Ok(());
    };
    if !image.i386() || stored.len() < 2 {
        return Ok(());
    }
    let mut seen = BTreeSet::new();
    for (a, b) in image.exec_ranges(raw.len()) {
        for (i, instruction) in raw[a..b].windows(5).enumerate() {
            if i % 65536 == 0 {
                ctx.check_cancelled()?;
            }
            if !matches!(instruction[0], 0x68 | 0xb8..=0xbf) {
                continue;
            }
            let va = u32::from_le_bytes(instruction[1..].try_into().unwrap());
            let Some(rva) = va.checked_sub(image.image_base) else {
                continue;
            };
            let Some(section) = image.section_for_rva(rva) else {
                continue;
            };
            if section.executable() {
                continue;
            }
            let delta = rva - section.virtual_address;
            let (lo, hi) = section.file_range(raw.len());
            let Some(at) = lo.checked_add(delta as usize) else {
                continue;
            };
            if at > hi || hi - at < 24 || !seen.insert(at) {
                continue;
            }
            ensure!(seen.len() <= 65536, "Nana module data reference limit");
            let words = std::array::from_fn(|j| {
                u32::from_le_bytes(raw[at + j * 4..at + j * 4 + 4].try_into().unwrap())
            });
            let mut header = [stored[0], stored[1]];
            decipher(words, &mut header);
            if header[0] & 15 != 8
                || header[0] >> 4 > 7
                || header[1] & 32 != 0
                || u16::from_be_bytes(header) % 31 != 0
            {
                continue;
            }
            states.insert(words);
            ensure!(states.len() <= 1024, "Nana candidate limit");
        }
    }
    Ok(())
}

pub(super) fn installed_names(
    base: &ParsedArchive,
    d: &Special,
    modules: &[&str],
    ctx: &DecodeContext,
) -> Result<Vec<String>> {
    let stored = name_blob(base, d, ctx)?;
    let mut states = BTreeSet::new();
    for &name in modules {
        let bytes = read_module(ctx, name)?;
        referenced_states(&bytes, &stored, &mut states, ctx)?;
        for (_, embedded) in pe::embedded_modules(&bytes) {
            referenced_states(&embedded, &stored, &mut states, ctx)?;
        }
    }
    recover_names(base, d, &stored, &states, ctx)
}

fn recover_names(
    base: &ParsedArchive,
    d: &Special,
    stored: &[u8],
    states: &BTreeSet<[u32; 6]>,
    ctx: &DecodeContext,
) -> Result<Vec<String>> {
    let mut recovered = None;
    let mut attempts = 0;
    for &words in states {
        ctx.check_cancelled()?;
        // Probe the first record before allocating the complete index.
        let mut prefix = stored[..stored.len().min(512)].to_vec();
        let n = prefix.len().min(256);
        decipher(words, &mut prefix[..n]);
        let mut first = [0; 18];
        let mut z = Decompress::new(true);
        if z.decompress(&prefix, &mut first, FlushDecompress::None)
            .is_err()
            || z.total_out() < 18
        {
            continue;
        }
        let units = i16::from_le_bytes(first[16..18].try_into().unwrap());
        let size = i64::from_le_bytes(first[4..12].try_into().unwrap());
        if units <= 0 || size != 8 + 2 * i64::from(units) || size + 12 > i64::from(d.original) {
            continue;
        }
        attempts += 1;
        ensure!(attempts <= 8, "Nana full index probe limit");
        if let Ok(names) = authenticated_names(base, d.original as usize, words, stored.to_vec()) {
            ensure!(
                recovered.as_ref().is_none_or(|old| old == &names),
                "ambiguous Nana name index"
            );
            recovered = Some(names);
        }
    }
    recovered.context(
        "installed modules have no referenced Nana state authenticating the complete dls: index",
    )
}

pub(in crate::xp3) fn open(
    mut base: ParsedArchive,
    d: &Special,
    (profile, table): (Profile, String),
    ctx: &DecodeContext,
) -> Result<Box<dyn Archive>> {
    let blob = name_blob(&base, d, ctx)?;
    let names = authenticated_names(&base, d.original as usize, profile.name_state, blob)?;
    for (e, path) in base.entries.iter_mut().zip(names) {
        e.path = path;
    }
    let protected = adler_xor::take_protected(&mut base);
    let c = profile.content;
    let control_block = read_all(
        ctx.open_dependency(&table)?.as_ref(),
        engine::CONTROL_BLOCK_SIZE as u64,
    )?;
    let engine = CxdecEngine::new(engine::CxdecProfile {
        mask: c.mask,
        offset: c.offset,
        prolog_order: c.prolog_order,
        even_branch_order: c.even_branch_order,
        odd_branch_order: c.odd_branch_order,
        control_block,
        generator: engine::GeneratorKind::Cabbage {
            random_seed: c.random_seed,
        },
        prefix8: None,
    })?;
    let archive = CxArchive {
        base,
        protected,
        engine: Some(Arc::new(engine)),
        embedded: BTreeMap::new(),
        report: serde_json::json!({"special":"dls:", "content":{"generator":"cabbage","source":"Nana profile"}}),
    };
    // Check the first genuinely encrypted sample before accepting the profile.
    // All subsequent reads independently verify the entire requested entry.
    let mut ordered: Vec<_> = archive
        .entries()
        .iter()
        .filter(|e| archive.protected.contains(&e.id) && e.size > 0)
        .collect();
    ordered.sort_unstable_by_key(|e| (e.size, e.id));
    let mut budget = 256 * 1024 * 1024u64;
    for e in ordered {
        if e.size > budget {
            break;
        }
        budget -= e.size;
        if archive.verified_reader(e.id, ctx)?.1 {
            break;
        }
    }
    Ok(Box::new(archive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xp3::cxdec::tests::{chunk, xp3_archive};
    use crysirror_core::{service::MemoryAssetReader, state::ClientId};
    use std::io::Write;

    const WORDS: [u32; 6] = [
        0x01234567, 0x89abcdef, 0x0badf00d, 0xc001d00d, 0x13579bdf, 0x2468ace0,
    ];
    #[test]
    fn published_name_cipher_vector() {
        let mut data = [0; 16];
        decipher(WORDS, &mut data);
        assert_eq!(
            data,
            [
                0x6c, 0x58, 0x53, 0x5f, 0x87, 0x80, 0xfd, 0xeb, 0x8a, 0xad, 0xc4, 0x86, 0xf7, 0x1e,
                0xef, 0xb1
            ]
        );
    }

    fn context(words: [u32; 6], table_path: &str) -> DecodeContext {
        let config = serde_json::to_vec(&serde_json::json!({
            "name_state":words, "content":{"mask":15,"offset":7,"random_seed":217,"prolog_order":[0,1,2],
            "even_branch_order":[0,1,2,3,4,5,6,7],"odd_branch_order":[0,1,2,3,4,5],"control_block_file":table_path}
        })).unwrap();
        let mut ctx = DecodeContext::new(ClientId(1));
        ctx.source_name = Some("game/arc/data.xp3".into());
        ctx.resolver = Some(Arc::new(
            move |path: &str| -> Result<Arc<dyn AssetReader>> {
                let data = match path {
                    "game/nana-profile.json" => config.clone(),
                    "game/control.bin" => vec![0; 4096],
                    _ => bail!("unknown fixture dependency"),
                };
                Ok(Arc::new(MemoryAssetReader(data.into())))
            },
        ));
        ctx
    }

    fn fixture(
        name: &str,
        bad_hash: bool,
        trailing: bool,
        bad_content: bool,
    ) -> Arc<dyn AssetReader> {
        let engine = CxdecEngine::new(engine::CxdecProfile {
            mask: 15,
            offset: 7,
            prolog_order: [0, 1, 2],
            even_branch_order: [0, 1, 2, 3, 4, 5, 6, 7],
            odd_branch_order: [0, 1, 2, 3, 4, 5],
            control_block: vec![0; 4096],
            generator: engine::GeneratorKind::Cabbage { random_seed: 217 },
            prefix8: None,
        })
        .unwrap();
        fixture_with_engine(name, bad_hash, trailing, bad_content, &engine)
    }
    fn fixture_with_engine(
        name: &str,
        bad_hash: bool,
        trailing: bool,
        bad_content: bool,
        engine: &CxdecEngine,
    ) -> Arc<dyn AssetReader> {
        let plain = (0..768).map(|i| (i * 27) as u8).collect::<Vec<_>>();
        let hash = adler32(&plain);
        let mut stored = plain;
        engine.apply(0, hash, &mut stored).unwrap();
        if bad_content {
            stored[300] ^= 4;
        }
        let token = special::token("dls:", name);
        let mut xp3 = xp3_archive(&[(&token, stored, hash, true)]);
        let index_at = u64::from_le_bytes(xp3[11..19].try_into().unwrap()) as usize;
        let mut index = xp3[index_at + 9..].to_vec();
        xp3.truncate(index_at);
        let mut record = (hash ^ u32::from(bad_hash)).to_le_bytes().to_vec();
        record.extend((name.encode_utf16().count() as u16).to_le_bytes());
        record.extend(name.encode_utf16().flat_map(u16::to_le_bytes));
        record.extend([0, 0]);
        let records = chunk(b"hnfn", &record);
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&records).unwrap();
        let mut names = encoder.finish().unwrap();
        if trailing {
            names.push(0);
        }
        let n = names.len().min(256);
        decipher(WORDS, &mut names[..n]);
        let mut d = (xp3.len() as u64).to_le_bytes().to_vec();
        d.extend((records.len() as u32).to_le_bytes());
        d.extend((names.len() as u32).to_le_bytes());
        index.extend(chunk(b"dls:", &d));
        xp3.extend(names);
        let at = (xp3.len() as u64).to_le_bytes();
        xp3[11..19].copy_from_slice(&at);
        xp3.push(0);
        xp3.extend((index.len() as u64).to_le_bytes());
        xp3.extend(index);
        Arc::new(MemoryAssetReader(xp3.into()))
    }

    #[test]
    fn installed_game_module_supplies_names_and_content_without_sidecars() {
        use crate::xp3::cxdec::tests::synthetic_module;
        let (mut module, control_block) = synthetic_module(0x1a3, 0x0b7);
        // Store the six words in .data and reference them from .text.
        module[0x3100..0x3118].copy_from_slice(
            &WORDS
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>(),
        );
        let mut code = vec![0x68];
        code.extend(0x1000_3d00u32.to_le_bytes());
        for (op, shift) in [
            (0xe0, 17),
            (0xe0, 18),
            (0xe8, 15),
            (0xe0, 13),
            (0xe8, 17),
            (0xe0, 5),
        ] {
            code.extend([0xc1, op, shift, 0x31, 0xc8]);
        }
        code.extend([0xf7, 0xd0]);
        let random_seed = 0x2317_9841u32;
        code.extend([0x89, 0x44, 0x24, 0x08, 0xc7, 0x44, 0x24, 0x10]);
        code.extend(random_seed.to_le_bytes());
        code.extend([
            0x89, 0x54, 0x24, 0x14, 0xc7, 0x44, 0x24, 0x18, 0, 0, 0, 0, 0xe8, 0, 0, 0, 0,
        ]);
        module[0x1140..0x1140 + code.len()].copy_from_slice(&code);
        let engine = CxdecEngine::new(engine::CxdecProfile {
            mask: 0x1a3,
            offset: 0x0b7,
            prolog_order: [2, 0, 1],
            even_branch_order: [0, 7, 5, 6, 3, 1, 4, 2],
            odd_branch_order: [4, 3, 2, 1, 5, 0],
            control_block,
            generator: engine::GeneratorKind::Cabbage { random_seed },
            prefix8: None,
        })
        .unwrap();
        let mut ctx = DecodeContext::new(ClientId(9));
        ctx.source_name = Some("game/arc/data.xp3".into());
        ctx.parameters.insert(
            "kirikiri/xp3".into(),
            serde_json::json!({"cxdec_modules":["game.exe"]}),
        );
        ctx.resolver = Some(Arc::new(
            move |path: &str| -> Result<Arc<dyn AssetReader>> {
                ensure!(
                    path == "game/game.exe",
                    "fixture contains only original module"
                );
                Ok(Arc::new(MemoryAssetReader(module.clone().into())))
            },
        ));
        let input = |bad_hash, bad_content| {
            fixture_with_engine("startup.tjs", bad_hash, false, bad_content, &engine)
        };
        let archive = Xp3.open(input(false, false), &ctx).unwrap();
        assert_eq!(archive.entries()[0].path, "startup.tjs");
        let mut slice = [0; 64];
        crysirror_resource::read_exact_at(archive.open(0, &ctx).unwrap().as_ref(), 517, &mut slice)
            .unwrap();
        assert_eq!(slice, std::array::from_fn(|i| ((i + 517) * 27) as u8));
        assert!(Xp3.open(input(true, false), &ctx).is_err());
        assert!(Xp3.open(input(false, true), &ctx).is_err());
    }

    #[test]
    fn mounted_original_name_and_content_require_valid_profile_and_index() {
        let name = "scene/日本.bin";
        let ctx = context(WORDS, "control.bin");
        let data = fixture(name, false, false, false);
        let a = Xp3.open(data.clone(), &ctx).unwrap();
        assert_eq!(a.entries()[0].path, name);
        let mut part = [0; 73];
        crysirror_resource::read_exact_at(a.open(0, &ctx).unwrap().as_ref(), 251, &mut part)
            .unwrap();
        assert_eq!(
            part.as_slice(),
            (251..324).map(|i| (i * 27) as u8).collect::<Vec<_>>()
        );
        for (hash, trailing, content) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            assert!(Xp3
                .open(fixture(name, hash, trailing, content), &ctx)
                .is_err());
        }
        assert!(Xp3
            .open(fixture("../x", false, false, false), &ctx)
            .is_err());
        assert!(Xp3
            .open(data.clone(), &context([0; 6], "control.bin"))
            .is_err());
        assert!(Xp3.open(data, &context(WORDS, "../control.bin")).is_err());
    }
}
