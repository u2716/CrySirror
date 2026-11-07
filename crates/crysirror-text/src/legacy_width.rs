// SPDX-License-Identifier: MPL-2.0
//! Legacy Japanese half-width conversion and width counting as performed by
//! Windows-era visual-novel engines: a fixed full-width table (Latin letters,
//! ideographic space, katakana, digits and ASCII symbols) mapped one to one
//! onto ASCII and JIS X 0201 katakana, with voiced katakana expanded to a
//! base character plus a sound mark.

const FULL: &str = "\u{3000}ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲンァィゥェォッャュョ\
１２３４５６７８９０－＾￥！”＃＄％＆（）＝～｜＠［‘｛；：］＋＊｝、．／＜＞？＿’";
const HALF: &str = " abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\
ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜｦﾝｧｨｩｪｫｯｬｭｮ\
1234567890-^\\!\"#$%&()=~|@[`{;:]+*},./<>?_'";
const VOICED: &str = "ガギグゲゴザジズゼゾダヂヅデドバビブベボパピプペポ";
const VOICED_HALF: &str = "ｶﾞｷﾞｸﾞｹﾞｺﾞｻﾞｼﾞｽﾞｾﾞｿﾞﾀﾞﾁﾞﾂﾞﾃﾞﾄﾞﾊﾞﾋﾞﾌﾞﾍﾞﾎﾞﾊﾟﾋﾟﾌﾟﾍﾟﾎﾟ";

/// Converts table characters to their half-width forms; others are kept.
pub fn to_halfwidth(text: &str) -> String {
    let voiced_half: Vec<char> = VOICED_HALF.chars().collect();
    let half: Vec<char> = HALF.chars().collect();
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if let Some(i) = VOICED.chars().position(|v| v == c) {
            out.push(voiced_half[i * 2]);
            out.push(voiced_half[i * 2 + 1]);
        } else if let Some(i) = FULL.chars().position(|f| f == c) {
            out.push(half[i]);
        } else {
            out.push(c);
        }
    }
    out
}

/// True for characters counted as one column: ASCII, U+00A5, U+203E and
/// half-width katakana U+FF61..U+FF9F.
pub fn is_single_column(c: char) -> bool {
    (c as u32) < 0x80 || c == '\u{a5}' || c == '\u{203e}' || ('\u{ff61}'..='\u{ff9f}').contains(&c)
}

/// Column count: one per single-column character, two otherwise (a
/// surrogate pair counts two).
pub fn columns(text: &str) -> usize {
    text.chars().map(|c| if is_single_column(c) { 1 } else { 2 }).sum()
}

#[cfg(test)]
mod tests {
    #[test]
    fn tables_are_paired() {
        assert_eq!(super::FULL.chars().count(), super::HALF.chars().count());
        assert_eq!(super::VOICED.chars().count() * 2, super::VOICED_HALF.chars().count());
    }
    #[test]
    fn converts_and_counts() {
        assert_eq!(super::to_halfwidth("ＡＢ１ガ漢"), "AB1ｶﾞ漢");
        assert_eq!(super::columns("Aｱ漢"), 4);
    }
}
