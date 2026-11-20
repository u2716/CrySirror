// MadoCrypt archive evidence follows xp3-brute PR #4 (docs/third-party/xp3-brute.txt).
use super::*;

const HEADERS: &[&[u8]] = &[
    b"TLG0.0\0sds\x1a",
    b"TLG5.0\0raw\x1a",
    b"TLG6.0\0raw\x1a",
    b"KBAD100\0",
];
const PROBE_BYTES: u64 = 256 * 1024 * 1024;

fn from_header(raw: &[u8], hash: u32, magic: &[u8]) -> Option<u32> {
    if raw.len() < magic.len() {
        return None;
    }
    let state = u32::from_le_bytes(std::array::from_fn(|i| raw[i] ^ magic[i]));
    if state >> 31 != state & 1 {
        return None;
    }
    let seed = (state ^ hash) & 0x7fff_ffff;
    let stream = key(seed ^ hash);
    magic
        .iter()
        .enumerate()
        .all(|(i, &b)| raw[i] ^ stream[i] == b)
        .then_some(seed)
}

fn infer(base: &ParsedArchive, ids: &BTreeSet<u32>, ctx: &DecodeContext) -> Result<Option<u32>> {
    let mut votes = BTreeMap::new();
    let mut budget = PROBE_BYTES;
    for e in base
        .entries
        .iter()
        .filter(|e| {
            ids.contains(&e.id)
                && e.path
                    .rsplit_once('.')
                    .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("tlg"))
                && e.size >= 8
        })
        .take(32)
    {
        ctx.check_cancelled()?;
        if e.size > budget {
            continue;
        }
        budget -= e.size;
        let Ok(raw) = base.open(e.id, ctx) else {
            continue;
        };
        let mut head = [0; 11];
        let n = e.size.min(head.len() as u64) as usize;
        if crysirror_resource::read_exact_at(raw.as_ref(), 0, &mut head[..n]).is_err() {
            continue;
        }
        let hash = base.xp3(e.id).context("missing XP3 Adler-32")?.adler32;
        for header in HEADERS {
            if let Some(seed) = from_header(&head[..n], hash, header) {
                let Ok((plain, decoded)) = checksums(raw.as_ref(), e.size, &key(seed ^ hash), ctx)
                else {
                    break;
                };
                if plain != hash && decoded == hash {
                    *votes.entry(seed).or_insert(0usize) += 1;
                    break;
                }
            }
        }
    }
    let mut passing = votes.into_iter().filter(|(_, count)| *count >= 2);
    Ok(passing
        .next()
        .map(|(seed, _)| seed)
        .filter(|_| passing.next().is_none()))
}

fn validates(
    base: &ParsedArchive,
    ids: &BTreeSet<u32>,
    seed: u32,
    ctx: &DecodeContext,
) -> Result<bool> {
    let mut sorted: Vec<_> = base
        .entries
        .iter()
        .filter(|e| ids.contains(&e.id) && e.size > 0)
        .collect();
    sorted.sort_unstable_by_key(|e| (e.size, e.id));
    let mut budget = PROBE_BYTES;
    let mut matches = 0;
    for e in sorted {
        if e.size > budget || matches == 3 {
            break;
        }
        budget -= e.size;
        let hash = base.xp3(e.id).context("missing XP3 Adler-32")?.adler32;
        let raw = base.open(e.id, ctx)?;
        let (plain, decoded) = checksums(raw.as_ref(), e.size, &key(seed ^ hash), ctx)?;
        if plain == hash {
            continue;
        }
        if decoded != hash {
            return Ok(false);
        }
        matches += 1;
    }
    Ok(matches > 0)
}

fn recover(
    base: &ParsedArchive,
    protected: &BTreeSet<u32>,
    ctx: &DecodeContext,
) -> Result<Option<u32>> {
    if let Some(seed) = infer(base, protected, ctx)? {
        if validates(base, protected, seed, ctx)? {
            return Ok(Some(seed));
        }
    }
    for sibling in ["fgimage.xp3", "arc/fgimage.xp3"] {
        ctx.check_cancelled()?;
        if ctx.source_name.as_deref().is_some_and(|p| {
            p.rsplit('/')
                .next()
                .is_some_and(|n| n.eq_ignore_ascii_case(sibling))
        }) {
            continue;
        }
        let Ok(source) = ctx.open_sibling(sibling) else {
            continue;
        };
        let Ok(mut index) = parse_index(source, ctx) else {
            continue;
        };
        if index.hxv4.is_some() || !index.specials.is_empty() {
            continue;
        }
        let ids = take_protected(&mut index.archive);
        if let Some(seed) = infer(&index.archive, &ids, ctx)? {
            if validates(&base, protected, seed, ctx)? {
                return Ok(Some(seed));
            }
        }
    }
    Ok(None)
}

pub(in crate::xp3) fn open_auto(
    mut base: ParsedArchive,
    ctx: &DecodeContext,
) -> Result<Box<dyn Archive>> {
    for (entry, plan) in base.entries.iter().zip(&base.plans) {
        if matches!(
            plan,
            EntryPlan::Segments {
                encrypted: true,
                ..
            }
        ) && ctx
            .pipeline(base.format, entry)
            .map_or(true, |pipeline| !pipeline.is_empty())
        {
            return Ok(Box::new(base));
        }
    }
    let protected = take_protected(&mut base);
    if !protected.is_empty() {
        // An inconclusive optional probe leaves the existing encrypted-entry
        // reader in charge, including its error on an unconfigured filter.
        let candidate = recover(&base, &protected, ctx);
        ctx.check_cancelled()?;
        if let Ok(Some(seed)) = candidate {
            return Ok(Box::new(AdlerArchive {
                base,
                protected,
                mado_seed: Some(seed),
                report: serde_json::json!({"family":"MadoCrypt","period":31}),
            }));
        }
        for id in protected {
            if let EntryPlan::Segments { encrypted, .. } = &mut base.plans[id as usize] {
                *encrypted = true;
            }
        }
    }
    Ok(Box::new(base))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xp3::cxdec::tests::xp3_archive;
    use crysirror_core::{service::MemoryAssetReader, state::ClientId};

    fn memory(bytes: Vec<u8>) -> Arc<dyn AssetReader> {
        Arc::new(MemoryAssetReader(bytes.into()))
    }
    fn file(
        name: &'static str,
        body: &[u8],
        seed: Option<u32>,
    ) -> (&'static str, Vec<u8>, u32, bool) {
        let mut bytes = body.to_vec();
        let mut adler = Adler::default();
        adler.update(body);
        let hash = adler.finish();
        if let Some(seed) = seed {
            apply(&key(seed ^ hash), 0, &mut bytes);
        }
        (name, bytes, hash, true)
    }

    #[test]
    fn companion_requires_independent_checksum_and_preserves_flagged_plaintext() {
        let seed = 0x1234567;
        let a = file("a.tlg", b"TLG6.0\0raw\x1a0123456789", Some(seed));
        let b = file(
            "b.tlg",
            b"KBAD100\0synthetic foreground definition",
            Some(seed),
        );
        let foreground = xp3_archive(&[a.clone(), b]);
        let plain = (0..2048).map(|n| (n * 17) as u8).collect::<Vec<_>>();
        let mut ctx = DecodeContext::new(ClientId(1));
        ctx.source_name = Some("data.xp3".into());
        ctx.resolver = Some(Arc::new(
            move |name: &str| -> Result<Arc<dyn AssetReader>> {
                ensure!(name == "arc/fgimage.xp3", "unexpected dependency");
                Ok(memory(foreground.clone()))
            },
        ));
        for (target_seed, accepted) in [(seed, true), (seed ^ 8, false)] {
            let target = xp3_archive(&[
                file("startup.tjs", &plain, Some(target_seed)),
                file("clear.bin", b"clear", None),
            ]);
            let archive = Xp3.open(memory(target), &ctx).unwrap();
            let reader = archive.open(0, &ctx);
            assert_eq!(reader.is_ok(), accepted);
            if let Ok(reader) = reader {
                let mut part = [0; 91];
                crysirror_resource::read_exact_at(reader.as_ref(), 127, &mut part).unwrap();
                assert_eq!(part.as_slice(), &plain[127..218]);
                assert_eq!(
                    read_all(archive.open(1, &ctx).unwrap().as_ref(), 5).unwrap(),
                    b"clear"
                );
            }
        }
        ctx.resolver = None;
        let single = Xp3.open(memory(xp3_archive(&[a])), &ctx).unwrap();
        assert!(single.open(0, &ctx).is_err());
    }
}
