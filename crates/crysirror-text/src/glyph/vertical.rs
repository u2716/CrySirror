// SPDX-License-Identifier: MPL-2.0
//! Local CJK glyph orientation on a clockwise-rotated line. Policy follows
//! NitroSharp VerticalGlyphOrientation.cs, 77a93db4 (MIT); this does not change
//! line flow or select a second atlas/font.
pub fn is_upright(c:char)->bool {
    let c=c as u32;
    if matches!(c,0x3008..=0x3011|0x3014..=0x301B|0x30FC|0x2014|0x2015|0x2026|0x2025|0xFF08|0xFF09|0xFF3B|0xFF3D|0xFF5B|0xFF5D) {return false;}
    matches!(c,0x2E80..=0xA4CF|0xAC00..=0xD7AF|0xF900..=0xFAFF|0xFE10..=0xFE1F|0xFE30..=0xFE4F|0xFF01..=0xFF60|0xFFE0..=0xFFE6|0x20000..=0x323AF)
}
pub fn clockwise_line_corners(c:char,bitmap:[f32;4],em:f32,corners:[[f32;2];4])->[[f32;2];4] {
    if !is_upright(c) {return corners;}
    let center=[bitmap[0]+bitmap[2]*0.5,bitmap[1]+bitmap[3]*0.5];
    let shift=if matches!(c,'、'|'。'|'，'|'．') {-em*0.2}else{0.};
    corners.map(|p|[center[0]+p[1]-center[1]+shift,center[1]-p[0]+center[0]+shift])
}
