// Portions adapted from Kirikiri Z (krkrz/krkrz fd5c4baa6a2ef5978db1bd043634351f48667daf:
// visual/CharacterData.cpp tTVPCharacterData::Blur, visual/glgen/gengl.pl
// TVPChBlurCopy_c, TVPChBlurAddMulCopy_c, TVPChBlurMulCopy_c, fast_int_hypot).
// Copyright (c) W.Dee and contributors; Kirikiri Z Project Contributors.
// Kirikiri licence (docs/third-party/krkrz.txt). Modifications CrySirror
// contributors (MPL-2.0).
//! Blurred glyph coverage for text shadows/edges: a circular kernel weighted
//! `width - distance + 1`, normalised to 2^18 and scaled by `level/256`, with
//! each accumulation clamped to 255. Operates on 8-bit coverage, as the
//! native 256-level path does.

pub use crysirror_image::coverage_blur::blur_coverage;
#[cfg(test)]
use crysirror_image::coverage_blur::fast_int_hypot;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blur_spreads_and_level_amplifies() {
        let src = [255u8];
        let (out, w, h) = blur_coverage(&src, 1, 1, 2, 255);
        assert_eq!((w, h), (5, 5));
        // Centre carries the largest weight. The native approximate hypot
        // puts (2, 2) at distance 2, so even the corner is inside radius 2.
        assert!(out[12] > out[11] && out[11] > out[0] && out[0] > 0);
        assert_eq!(fast_int_hypot(2, 2), 2);
        let (strong, ..) = blur_coverage(&src, 1, 1, 2, 2048);
        assert!(strong[11] > out[11]);
        assert_eq!(blur_coverage(&src, 1, 1, 0, 128).0, vec![127]);
    }
}
