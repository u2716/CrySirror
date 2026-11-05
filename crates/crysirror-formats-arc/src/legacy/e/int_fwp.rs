// Copyright (c) 2026 CrySirror contributors.
//! Frontwing installation-media transform, reviewed in the owner's install.exe:
//! 0x487380 -> 0x482EE0 -> 0x482DE0. The inclusive middle-three-fifths
//! region is XORed before ordinary INT decoding. No executable is run.
use super::*;

pub(in crate::legacy) fn prepare(
    source: Arc<dyn AssetReader>,
    cx: &DecodeContext,
) -> Result<Arc<dyn AssetReader>> {
    if !cx
        .source_name
        .as_deref()
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".int.fwp"))
    {
        return Ok(source);
    }
    cx.check_cancelled()?;
    let len = length(source.as_ref())?;
    ensure!(len >= 5, "FWP file is too short");
    let start = len / 5;
    let end = len - start + 1; // Installer comparison includes the upper endpoint.
    let n = le32(&bytes(source.as_ref(), 4, 4, 4)?, 0)? as usize;
    limits(cx, n)?;
    let table_size = (n as u64)
        .checked_mul(72)
        .context("FWP index extent overflow")?;
    ensure!(
        8 + table_size <= start,
        "unsupported FWP encrypted index layout"
    );
    let raw = index(source.as_ref(), cx, 8, table_size)?;
    let expected = if let Some(record) = raw
        .chunks_exact(72)
        .find(|p| zstr(&p[..64]) == b"__key__.dat")
    {
        let key = mt_classic_first(le32(record, 68)?).to_le_bytes();
        crysirror_resource::blowfish_le_encrypt(b"CatScene", &key, 8)?
    } else {
        b"CatScene".to_vec()
    };
    // Only the unprotected directory is parsed here. Alias comparisons must
    // happen on the decoded read-at view, since XOR phase varies by offset.
    let (rows, known_decode) = int_rows(source.as_ref(), cx)?;
    ensure!(
        known_decode,
        "FWP requires the native INT executable/key profile"
    );
    let rows = checked_rows(source.as_ref(), cx, rows)?;
    let mut key = [None; 40];
    let mut witnesses = [0usize; 40];
    let mut seen = std::collections::BTreeSet::new();
    for row in &rows {
        cx.check_cancelled()?;
        if !has_ext(&row.path, "cst")
            || row.stored < 8
            || row.offset < start
            || row.offset + 8 > end
            || !seen.insert(row.offset)
        {
            continue;
        }
        let header = bytes(source.as_ref(), row.offset, 8, 8)?;
        for (i, (&encoded, &plain)) in header.iter().zip(&expected).enumerate() {
            let slot = ((row.offset + i as u64) % 40) as usize;
            let value = encoded ^ plain;
            ensure!(
                key[slot].is_none_or(|prior| prior == value),
                "FWP inconsistent scene-header evidence in {}",
                row.path
            );
            key[slot] = Some(value);
            witnesses[slot] += 1;
        }
    }
    ensure!(
        witnesses.iter().all(|&n| n >= 2),
        "unsupported FWP: insufficient independent scene-header evidence for the 40-byte key"
    );
    let key: Vec<u8> = key
        .into_iter()
        .map(|v| v.expect("checked FWP evidence"))
        .collect();
    cx.check_cancelled()?;
    // The wrapper retains the source and inferred bytes, not this index job's
    // cancellation token. Existing resource workers own subsequent reads.
    crysirror_resource::io::xor_region(source, start..end, key.into())
}
