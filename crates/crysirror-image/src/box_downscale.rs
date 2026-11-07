//! Integer box reduction: each destination pixel averages an integer
//! `floor(src/dst)` block sampled at a 10-bit fixed-point source step, with a
//! truncated 12-bit reciprocal of the block area (as used by QLIE 560720 for
//! reductions to at most half size). Channels are averaged independently.
use anyhow::{ensure, Result};

/// Reduce RGBA8 `rgba` of `src` size into `dst` size. Each axis must shrink to
/// at most half; larger ratios use a different resampler and are rejected.
pub fn box_downscale_rgba(src: [u32; 2], rgba: &[u8], dst: [u32; 2]) -> Result<Vec<u8>> {
    ensure!(
        src.iter().chain(dst.iter()).all(|n| (1..=16384).contains(n)),
        "box downscale extent"
    );
    ensure!(
        rgba.len() as u64 == u64::from(src[0]) * u64::from(src[1]) * 4,
        "box downscale source bytes"
    );
    ensure!(
        u64::from(dst[0]) * 2 <= u64::from(src[0]) && u64::from(dst[1]) * 2 <= u64::from(src[1]),
        "box downscale requires a reduction to at most half size"
    );
    let [sw, sh] = src.map(|n| n as i64);
    let [dw, dh] = dst.map(|n| n as i64);
    let (bw, bh) = (sw / dw, sh / dh);
    let (step_x, step_y) = ((sw << 10) / dw, (sh << 10) / dh);
    let scale = (4096.0 / (bw * bh) as f64).trunc() as u64;
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = (step_y * y) >> 10;
        for x in 0..dw {
            let sx = (step_x * x) >> 10;
            let mut sum = [0u64; 4];
            for by in 0..bh {
                let row = ((sy + by).min(sh - 1) * sw) as usize;
                for bx in 0..bw {
                    let at = (row + (sx + bx).min(sw - 1) as usize) * 4;
                    for c in 0..4 {
                        sum[c] += u64::from(rgba[at + c]);
                    }
                }
            }
            let at = ((y * dw + x) * 4) as usize;
            for c in 0..4 {
                out[at + c] = ((scale * sum[c]) >> 12) as u8;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn averages_blocks_with_truncated_reciprocal() {
        // 4x2 -> 2x1: blocks of 2x2; 4096/4 = 1024 exactly.
        let mut rgba = vec![0u8; 4 * 2 * 4];
        for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
            px.copy_from_slice(&[(i * 10) as u8, 255, 0, 255]);
        }
        let out = box_downscale_rgba([4, 2], &rgba, [2, 1]).unwrap();
        assert_eq!(&out[..4], &[25, 255, 0, 255]);
        assert_eq!(&out[4..], &[45, 255, 0, 255]);
        // 3x3 block: trunc(4096/9)=455, so a full 255 block yields 254.
        let full = vec![255u8; 6 * 6 * 4];
        assert_eq!(box_downscale_rgba([6, 6], &full, [2, 2]).unwrap()[0], 254);
        assert!(box_downscale_rgba([4, 4], &full[..64], [3, 3]).is_err());
    }
}
