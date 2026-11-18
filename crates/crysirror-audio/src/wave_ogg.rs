//! WAVE files whose `data` chunk is an Ogg Vorbis stream (the Vorbis ACM
//! codec tags 0x674F..0x6771, e.g. codeX RScript music, effects and voices).
//! The Ogg pages are decoded by the shared Vorbis path; no ACM driver runs.

/// Vorbis ACM format tags (modes 1, 2, 3, 1+, 2+, 3+).
const VORBIS_ACM: [u16; 6] = [0x674F, 0x6750, 0x6751, 0x676F, 0x6770, 0x6771];

/// Byte range of the embedded Ogg stream, from a header prefix that reaches
/// the start of the `data` chunk.
pub(crate) fn ogg_range(prefix: &[u8], file_len: u64) -> Option<(u64, u64)> {
    if prefix.len() < 12 || &prefix[..4] != b"RIFF" || &prefix[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12usize;
    let mut vorbis = false;
    while at + 8 <= prefix.len() {
        let id = &prefix[at..at + 4];
        let size = u32::from_le_bytes(prefix[at + 4..at + 8].try_into().ok()?) as u64;
        let body = at + 8;
        if id == b"fmt " {
            let tag = u16::from_le_bytes(prefix.get(body..body + 2)?.try_into().ok()?);
            vorbis = VORBIS_ACM.contains(&tag);
            if !vorbis {
                return None;
            }
        } else if id == b"data" {
            if !vorbis || prefix.get(body..body + 4)? != b"OggS" {
                return None;
            }
            let start = body as u64;
            let len = size.min(file_len.saturating_sub(start));
            return (len > 0).then_some((start, len));
        }
        at = body.checked_add(usize::try_from(size).ok()?)?.checked_add((size & 1) as usize)?;
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_the_ogg_payload_of_vorbis_acm_waves_only() {
        let mut w = b"RIFF\0\0\0\0WAVEfmt \x12\0\0\0".to_vec();
        w.extend(0x6771u16.to_le_bytes());
        w.extend([0; 16]);
        w.extend(b"data\x08\0\0\0OggSxxxx");
        assert_eq!(super::ogg_range(&w, w.len() as u64), Some((46, 8)));
        w[20] = 1;
        w[21] = 0;
        assert_eq!(super::ogg_range(&w, w.len() as u64), None);
    }
}
