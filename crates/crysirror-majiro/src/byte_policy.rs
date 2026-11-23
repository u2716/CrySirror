// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original CP932/CRT profiles over shared byte-string operations.
use anyhow::{Context, Result, ensure};
use crysirror_text_legacy::{
    ByteNumberSymbols, ByteStringContext, ByteStringLimits, ByteStringProfile, NumericCompaction,
};
fn check() -> Result<()> {
    Ok(())
}
pub(crate) fn context() -> Result<ByteStringContext<'static>> {
    ByteStringContext::new(
        ByteStringProfile::SHIFT_JIS_CRT,
        ByteStringLimits::default(),
        &check,
    )
}
fn casing_domain(bytes: &[u8], upper:bool) -> Result<()> {
    let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    let text = encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .context("Majiro CRT string is not CP932")?;
    //The ASCII/Japanese domain is independent of a host Windows locale.
    //Extended cased DBCS letters require a reviewed native NLS mapping rather
    //than silently substituting the host Rust Unicode version.
    ensure!(
        text.chars()
            .filter(|c| !c.is_ascii())
            .all(|c| if upper{c.to_uppercase().eq(std::iter::once(c))}else{c.to_lowercase().eq(std::iter::once(c))}),
        "Majiro non-ASCII CRT casing profile is not connected"
    );
    Ok(())
}
pub(crate) fn upper(bytes: &[u8]) -> Result<Vec<u8>> {
    casing_domain(bytes,true)?;
    context()?.map_ascii_case(bytes, true)
}
pub(crate) fn lower(bytes:&[u8])->Result<Vec<u8>>{
    casing_domain(bytes,false)?;
    context()?.map_ascii_case(bytes,false)
}
pub(crate) fn compare(a: &[u8], b: &[u8]) -> Result<std::cmp::Ordering> {
    // Static CRT review: Scarlett437F26, Akatsuki450606 and YUKIIRO464929
    // all use the same reached-unit SBCS-lower/DBCS-upper NLS comparison.
    // Do not validate a full dictionary line before the first differing unit.
    context()?.compare_windows932_crt_case(a, b)
}
pub(crate) fn integer(bytes: &[u8]) -> Result<i32> {
    context()?.decimal_integer(
        bytes,
        ByteNumberSymbols::Ascii,
        NumericCompaction::Terminate,
    )
}
