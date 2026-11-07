// Copyright (c)2026 CrySirror contributors. GPL-2.0-or-later.
//! Original System4 MsgLogManager binary record layout. The authorized native
//! module was inspected as static data only: Save100018a0/record10001130 and
//! Load10001950/record100011a0. No file, process or engine policy is hosted.
use anyhow::{ensure, Context, Result};
use crysirror_resource::{record_sequence::ByteRecord, DecodeContext};
use std::{collections::VecDeque, sync::Arc};
const BYTES: usize = 64 * 1024 * 1024;
const RECORDS: usize = 1_000_000;
/// Parse the original uncompressed LE count/tag/length/payload records.
/// Extra trailing bytes remain permitted as in the native sequential reader.
pub fn decode(bytes: &[u8], cx: &DecodeContext) -> Result<VecDeque<ByteRecord>> {
    ensure!(
        bytes.len() <= BYTES && bytes.len() as u64 <= cx.limits.max_entry_size,
        "Alice message log byte budget"
    );
    let mut at = 0usize;
    let count = word(bytes, &mut at)? as usize;
    ensure!(
        count <= RECORDS && count <= cx.limits.max_entries,
        "Alice message log record budget"
    );
    let mut records = VecDeque::new();
    for _ in 0..count {
        cx.check_cancelled()?;
        let tag = word(bytes, &mut at)? as i32;
        let len = word(bytes, &mut at)? as usize;
        let end = at
            .checked_add(len)
            .filter(|n| *n <= bytes.len())
            .context("Alice message log payload extent")?;
        records.push_back(ByteRecord {
            tag,
            bytes: Arc::from(&bytes[at..end]),
        });
        at = end;
    }
    cx.check_cancelled()?;
    Ok(records)
}
/// Prepare all bytes before an engine storage capability publishes the file.
pub fn encode(records: &VecDeque<ByteRecord>, cx: &DecodeContext) -> Result<Vec<u8>> {
    ensure!(
        records.len() <= RECORDS && records.len() <= cx.limits.max_entries,
        "Alice message log record count"
    );
    let mut bytes = 4usize;
    for record in records {
        cx.check_cancelled()?;
        bytes = bytes
            .checked_add(8)
            .and_then(|n| n.checked_add(record.bytes.len()))
            .filter(|n| *n <= BYTES)
            .context("Alice message log output byte budget")?;
    }
    ensure!(
        bytes as u64 <= cx.limits.max_entry_size,
        "Alice message log configured byte budget"
    );
    let mut out = Vec::with_capacity(bytes);
    out.extend(u32::try_from(records.len())?.to_le_bytes());
    for record in records {
        cx.check_cancelled()?;
        out.extend(record.tag.to_le_bytes());
        out.extend(u32::try_from(record.bytes.len())?.to_le_bytes());
        out.extend(record.bytes.as_ref());
    }
    Ok(out)
}
fn word(bytes: &[u8], at: &mut usize) -> Result<u32> {
    let end = at
        .checked_add(4)
        .context("Alice message log word overflow")?;
    let word = bytes
        .get(*at..end)
        .context("Alice message log truncated word")?;
    *at = end;
    Ok(u32::from_le_bytes(word.try_into()?))
}
