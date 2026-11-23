//! The recovered DC2 copy/feedback functions operate on nativeRGB24 work
//! bitmaps. Reject a different source layout instead of accidentally applying
//! the DC4 transparent/planar profile to an old background.
use super::*;
/// Returns whether the source is the 32-bit (inverted-alpha) CRX layout that
/// native 40B840 delegates to 40B5F0's alpha blit instead of the keyed copy.
pub(super) fn validate(storage: &dyn ClientStorage, name: &str) -> Result<bool> {
    //402FE0 strips one '@'/'*' before the extension (VM rejects '?').
    let (name, _) = super::images::native_image_variant(name)?;
    let native = name.replace('\\', "/");
    let stem = native
        .rsplit_once('.')
        .filter(|(_, extension)| extension.eq_ignore_ascii_case("crx"))
        .map_or(native.as_str(), |(stem, _)| stem);
    let reader = storage.open_read(&image_path(stem)?)?;
    let mut header = [0; 20];
    crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut header)?;
    let word = |at| u16::from_le_bytes([header[at], header[at + 1]]);
    ensure!(
        &header[..4] == b"CRXG" && matches!(word(12), 1 | 2) && word(14) <= 1 && word(16) <= 1,
        "DC2 native display bitmap requires the reviewed RGB24/ARGB32 CRXG profile"
    );
    Ok(word(16) == 1)
}
