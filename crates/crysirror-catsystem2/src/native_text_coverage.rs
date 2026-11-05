// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native glyph edge policy mapped to the engine's reusable coverage kernel.
//! Glyphs are evaluated separately on transparent scratch with a clear one-pixel
//! border (690CB0/691A70). The native font constructor fixes shadow offsets at 1,1.

/// Mask 1 selects the offset shadow and mask 2 selects the saturated 3×3 edge.
/// Both are applied before the foreground glyph. Repeated offsets intentionally
/// retain repeated coverage contributions; deduplicating changes antialiasing.
pub(crate) fn kernel(flags: u32, font_height: u32, color: [u8; 3]) -> Option<crysirror_image::coverage_sum::CoverageSum> {
    let outline = flags & 2 != 0;
    let shadow = flags & 1 != 0;
    if !outline && !shadow { return None; }
    let mut displacement = [1i32; 2];
    if font_height < 20 {
        for value in &mut displacement { *value = (*value + 1) >> 1; }
    }
    let mut offsets = Vec::with_capacity(if outline && shadow { 19 } else if outline { 9 } else { 1 });
    if outline {
        for y in -1..=1 { for x in -1..=1 { offsets.push([x, y]); } }
    }
    if shadow {
        if outline {
            // 6A40D0 reads both the edge destination and original glyph:
            // D(p) = sat(O(p) + O(p-d) + G(p-d)). Rows are visited backwards,
            // so O(p-d) still holds the unmodified edge at the default d=(1,1).
            for y in -1..=1 { for x in -1..=1 { offsets.push([x - displacement[0], y - displacement[1]]); } }
        }
        offsets.push([-displacement[0], -displacement[1]]);
    }
    Some(crysirror_image::coverage_sum::CoverageSum { offsets, color })
}
