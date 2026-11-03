// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! The script's own arithmetic natives: D3DX-layout 4x4 matrices in VM memory (207, 204,
//! 206), pow (397) and the bracket lists of 539. Nothing here touches the renderer.

pub(crate) type Matrix = [f32; 16];

pub(crate) const IDENTITY: Matrix = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

pub(crate) fn from_bytes(bytes: &[u8]) -> Matrix {
    let mut out = IDENTITY;
    for (slot, word) in out.iter_mut().zip(bytes.chunks_exact(4)) {
        *slot = f32::from_le_bytes(word.try_into().unwrap());
    }
    out
}
pub(crate) fn to_bytes(matrix: &Matrix) -> Vec<u8> {
    matrix.iter().flat_map(|value| value.to_le_bytes()).collect()
}
/// D3DXMatrixMultiply: row vectors, `a` applied first.
pub(crate) fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [0.; 16];
    for row in 0..4 {
        for column in 0..4 {
            out[row * 4 + column] = (0..4).map(|k| a[row * 4 + k] * b[k * 4 + column]).sum();
        }
    }
    out
}
/// 4D09DB, 4D0A7D, 4D0B20: D3DXMatrixRotationX/Y/Z.
pub(crate) fn rotation(axis: usize, radians: f32) -> Matrix {
    let (s, c) = radians.sin_cos();
    let mut m = IDENTITY;
    match axis {
        0 => { m[5] = c; m[6] = s; m[9] = -s; m[10] = c; }
        1 => { m[0] = c; m[2] = -s; m[8] = s; m[10] = c; }
        _ => { m[0] = c; m[1] = s; m[4] = -s; m[5] = c; }
    }
    m
}
/// 4D08C3: D3DXMatrixTranslation.
pub(crate) fn translation(x: f32, y: f32, z: f32) -> Matrix {
    let mut m = IDENTITY;
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m
}
/// D3DXVec4Transform on 16 bytes of VM memory.
pub(crate) fn transform(vector: &[u8], m: &Matrix) -> Vec<u8> {
    let v: Vec<f32> = vector.chunks_exact(4).map(|w| f32::from_le_bytes(w.try_into().unwrap())).collect();
    (0..4).flat_map(|c| (v[0] * m[c] + v[1] * m[4 + c] + v[2] * m[8 + c] + v[3] * m[12 + c]).to_le_bytes()).collect()
}
/// 646390: `[3-6]` and `[1,4,9]` expand to their values; anything else is one value.
pub(crate) fn bracket_words(text: &[u8], limit: usize, value: &mut dyn FnMut(&[u8]) -> u32) -> Vec<u32> {
    let mut out = Vec::new();
    if limit == 0 {
        return out;
    }
    let Some(body) = text.strip_prefix(b"[") else {
        out.push(value(text));
        return out;
    };
    let body = body.split(|b| *b == b']').next().unwrap_or_default();
    if !body.contains(&b',') {
        if let Some(dash) = body.iter().position(|b| *b == b'-') {
            let (low, high) = (value(&body[..dash]) as i32, value(&body[dash + 1..]) as i32);
            let mut current = low;
            while current <= high && out.len() < limit {
                out.push(current as u32);
                current += 1;
            }
            return out;
        }
    }
    for word in body.split(|b| *b == b',') {
        if out.len() == limit {
            break;
        }
        out.push(value(word));
    }
    out
}
/// 656130: where `needle` starts in `text`, walking whole characters; its length if absent.
pub(crate) fn find_characters(text: &[u8], needle: &[u8]) -> u32 {
    let mut at = 0;
    while at < text.len() {
        if !needle.is_empty() && text[at..].starts_with(needle) {
            return at as u32;
        }
        let lead = text[at];
        at += if lead < 0x80 { 1 } else if lead >= 0xf0 { 4 } else if lead >= 0xe0 { 3 } else { 2 };
    }
    text.len() as u32
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotations_and_translation_follow_d3dx_row_vectors() {
        let m = multiply(&rotation(2, std::f32::consts::FRAC_PI_2), &translation(3., 4., 5.));
        assert!((m[1] - 1.).abs() < 1e-6 && (m[4] + 1.).abs() < 1e-6);
        assert_eq!([m[12], m[13], m[14]], [3., 4., 5.]);
        assert_eq!(from_bytes(&to_bytes(&IDENTITY)), IDENTITY);
    }
    #[test]
    fn bracket_lists_and_character_search() {
        let mut number = |word: &[u8]| std::str::from_utf8(word).unwrap().parse::<u32>().unwrap();
        assert_eq!(bracket_words(b"[3-5]", 8, &mut number), [3, 4, 5]);
        assert_eq!(bracket_words(b"[1,4,9]", 2, &mut number), [1, 4]);
        assert_eq!(bracket_words(b"7", 8, &mut number), [7]);
        assert_eq!(find_characters(b"ab,c", b","), 2);
        assert_eq!(find_characters("あ,".as_bytes(), b","), 3);
        assert_eq!(find_characters(b"abc", b","), 3);
    }
}
