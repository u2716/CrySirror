// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KE27 shares native show/animation work with HAE108, but uses a bit-set
//! boundary selector, probabilistic reselection and an EMPTY save initializer.
use anyhow::Result;
use crysirror_image::temporal_pattern::PatternRng;
pub(crate) fn parameters(values: &[i32]) -> [i32; 144] {
    let mut p = super::native_picture_hae::parameters(values);
    if values.len() < 12 {
        p[11] = 100;
    }
    p
}
pub(crate) fn initialize(p: &mut [i32; 144], canvas: [u32; 2], rng: &mut PatternRng) -> Result<()> {
    super::native_picture_hae::initialize_mode(p, canvas, rng, true)
}
pub(crate) fn calcu(
    p: &mut [i32; 144],
    count: &mut i32,
    canvas: [u32; 2],
    rng: &mut PatternRng,
) -> Result<()> {
    super::native_picture_hae::calcu_mode(p, count, canvas, rng, true)
}
// Print, countMax999999, CountIsMaxFALSE and no underlay match HAE exactly;
// work6 retains the selected boundary through ordinary raw-save hydration.
