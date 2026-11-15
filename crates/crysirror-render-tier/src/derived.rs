// SPDX-License-Identifier: MPL-2.0
//! CPU-derived render data for tiers whose native shader programs lack
//! integer bit operations or a compute stage. Each function reproduces the
//! named desktop WGSL computation exactly; results are derived resources,
//! never save state.
use core::f32::consts::PI;

/// fullscreen.wgsl `radical_inverse`.
pub fn radical_inverse(mut bits: u32) -> f32 {
    bits = bits.rotate_left(16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

/// The desktop sample points `(k / count, radical_inverse(k))`.
pub fn hammersley(count: u32) -> Vec<[f32; 2]> {
    (0..count)
        .map(|k| [k as f32 / count as f32, radical_inverse(k)])
        .collect()
}

fn importance_ggx(xi: [f32; 2], a: f32) -> [f32; 3] {
    // fullscreen.wgsl importance_ggx with n = +Z (tx = +X, ty = +Y).
    let phi = 2. * PI * xi[0];
    let cos_t = ((1. - xi[1]) / (1. + (a * a - 1.) * xi[1])).sqrt();
    let sin_t = (1. - cos_t * cos_t).max(0.).sqrt();
    let h = [phi.cos() * sin_t, phi.sin() * sin_t, cos_t];
    let l = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
    [h[0] / l, h[1] / l, h[2] / l]
}

/// brdf.wgsl `fs_brdf`: split-sum (scale, bias) for `size`² texel centres,
/// row-major with x = N·V and y = roughness, 512 samples per texel.
pub fn brdf_lut(size: u32) -> Vec<[f32; 2]> {
    let count = 512u32;
    let points = hammersley(count);
    let mut out = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let nv = ((x as f32 + 0.5) / size as f32).max(1e-3);
            let roughness = (y as f32 + 0.5) / size as f32;
            let a = roughness * roughness;
            let v = [(1. - nv * nv).sqrt(), 0., nv];
            let (mut sa, mut sb) = (0f32, 0f32);
            for xi in &points {
                let h = importance_ggx(*xi, a);
                let vdh = v[0] * h[0] + v[2] * h[2];
                let mut l = [2. * vdh * h[0] - v[0], 2. * vdh * h[1], 2. * vdh * h[2] - v[2]];
                let len = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
                l = [l[0] / len, l[1] / len, l[2] / len];
                let nl = l[2].max(0.);
                let nh = h[2].max(0.);
                let vh = vdh.max(0.);
                if nl > 0. {
                    let k = a / 2.;
                    let g = (nv / (nv * (1. - k) + k)) * (nl / (nl * (1. - k) + k));
                    let g_vis = g * vh / (nh * nv).max(1e-5);
                    let fc = (1. - vh).powi(5);
                    sa += (1. - fc) * g_vis;
                    sb += fc * g_vis;
                }
            }
            out.push([sa / count as f32, sb / count as f32]);
        }
    }
    out
}


/// L1 irradiance of one probe: rows r, g, b with `dot(row, (1, n))`, and the
/// gi_project.wgsl state (x usable, y back-face fraction, z refreshed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProbeSh {
    pub rows: [[f32; 4]; 3],
    pub state: [f32; 4],
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// gi_project.wgsl `cs_project` irradiance and state, on the CPU for tiers
/// without compute. `faces` holds six `n`×`n` captures (face-major, rows top
/// to bottom) with the signed hit distance in alpha; `bases` are the
/// (forward, up) pairs the capture used. Distance moments are not produced:
/// tiers using this path disable Chebyshev visibility.
pub fn project_probe(faces: &[[f32; 4]], n: usize, bases: &[([f32; 3], [f32; 3]); 6], back_limit: f32) -> Option<ProbeSh> {
    if n == 0 || faces.len() != 6 * n * n {
        return None;
    }
    let mut sum = [[0f32; 4]; 3];
    let (mut w, mut back) = (0f32, 0f32);
    for (k, (f, up)) in bases.iter().enumerate() {
        let right = cross(*f, *up);
        for v in 0..n {
            for u in 0..n {
                let x = (u as f32 + 0.5) / n as f32 * 2. - 1.;
                let y = 1. - (v as f32 + 0.5) / n as f32 * 2.;
                let dir = [0, 1, 2].map(|i| f[i] + right[i] * x + up[i] * y);
                let len2 = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
                let inv = 1. / len2.sqrt();
                let d = dir.map(|c| c * inv);
                let dw = 1. / (len2 * len2.sqrt());
                let s = faces[(k * n + v) * n + u];
                let basis = [0.282095, 0.488603 * d[0], 0.488603 * d[1], 0.488603 * d[2]];
                for c in 0..3 {
                    for i in 0..4 {
                        sum[c][i] += basis[i] * s[c] * dw;
                    }
                }
                w += dw;
                if s[3] < 0. {
                    back += dw;
                }
            }
        }
    }
    let norm = 4. * PI / w;
    let conv = [PI * 0.282095, 2. * PI / 3. * 0.488603, 2. * PI / 3. * 0.488603, 2. * PI / 3. * 0.488603];
    let rows = sum.map(|row| [0, 1, 2, 3].map(|i| row[i] * norm * conv[i]));
    let fraction = back / w;
    Some(ProbeSh {
        rows,
        state: [if fraction > back_limit { 0. } else { 1. }, fraction, 1., 0.],
    })
}

/// Per-draw irradiance-volume sample (the PSV reduction of pbr.wgsl
/// `gi_irradiance`): trilinear over valid probes at `p` without the
/// per-pixel backface/visibility weights. Returns the L1 rows and the fade
/// weight toward the SH9 fallback (0 when outside or no valid probe).
pub fn sample_volume(probes: &[ProbeSh], center: [f32; 3], extent: [f32; 3], counts: [u32; 3], p: [f32; 3]) -> ([[f32; 4]; 3], f32) {
    let zero = ([[0.; 4]; 3], 0.);
    if counts.iter().any(|c| *c < 2) || probes.len() != counts.iter().product::<u32>() as usize {
        return zero;
    }
    let cells = counts.map(|c| (c - 1) as f32);
    let mut t = [0f32; 3];
    let mut outside = 0f32;
    for i in 0..3 {
        let cell = 2. * extent[i] / cells[i];
        let rel = p[i] - center[i];
        outside = outside.max((rel.abs() - extent[i]).max(0.) / cell);
        t[i] = (rel.clamp(-extent[i], extent[i]) + extent[i]) / (2. * extent[i]) * cells[i];
    }
    if outside >= 1. {
        return zero;
    }
    let b = [0, 1, 2].map(|i| t[i].floor().clamp(0., cells[i] - 1.) as u32);
    let f = [0, 1, 2].map(|i| t[i] - b[i] as f32);
    let mut acc = [[0f32; 4]; 3];
    let mut wsum = 0f32;
    for c in 0..8u32 {
        let o = [c & 1, (c >> 1) & 1, (c >> 2) & 1];
        let idx = [0, 1, 2].map(|i| b[i] + o[i]);
        let probe = &probes[(idx[0] + counts[0] * (idx[1] + counts[1] * idx[2])) as usize];
        if probe.state[0] < 0.5 {
            continue;
        }
        let w: f32 = (0..3).map(|i| if o[i] == 1 { f[i] } else { 1. - f[i] }).product();
        for r in 0..3 {
            for i in 0..4 {
                acc[r][i] += probe.rows[r][i] * w;
            }
        }
        wsum += w;
    }
    if wsum <= 1e-7 {
        return zero;
    }
    (acc.map(|row| row.map(|v| v / wsum)), 1. - outside)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hammersley_matches_reference_bits() {
        assert_eq!(radical_inverse(0), 0.);
        assert_eq!(radical_inverse(1), 0.5);
        assert_eq!(radical_inverse(2), 0.25);
        assert_eq!(radical_inverse(3), 0.75);
        assert_eq!(hammersley(64).len(), 64);
    }
    #[test]
    fn brdf_lut_energy() {
        let lut = brdf_lut(16);
        assert!(lut.iter().all(|t| t.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))));
        // Smooth surface viewed head-on reflects (nearly) all energy: A + B ≈ 1.
        let head_on = lut[15];
        assert!((head_on[0] + head_on[1] - 1.).abs() < 0.05, "{head_on:?}");
        // Rough surfaces lose energy to the single-scattering model.
        let rough = lut[15 * 16 + 15];
        assert!(rough[0] + rough[1] < head_on[0] + head_on[1]);
    }
    const BASES: [([f32; 3], [f32; 3]); 6] = [
        ([1., 0., 0.], [0., 1., 0.]),
        ([-1., 0., 0.], [0., 1., 0.]),
        ([0., 1., 0.], [0., 0., -1.]),
        ([0., -1., 0.], [0., 0., 1.]),
        ([0., 0., 1.], [0., 1., 0.]),
        ([0., 0., -1.], [0., 1., 0.]),
    ];
    #[test]
    fn uniform_radiance_projects_to_pi_irradiance() {
        // Uniform radiance L gives irradiance πL in every direction.
        let n = 8;
        let faces = vec![[0.5, 1.0, 2.0, 3.0]; 6 * n * n];
        let sh = project_probe(&faces, n, &BASES, 0.5).unwrap();
        for (c, l) in [0.5f32, 1.0, 2.0].iter().enumerate() {
            assert!((sh.rows[c][0] - PI * l).abs() < 1e-3 * PI * l, "{:?}", sh.rows[c]);
            assert!(sh.rows[c][1..].iter().all(|v| v.abs() < 1e-3));
        }
        assert_eq!(sh.state, [1., 0., 1., 0.]);
        let inside = vec![[0., 0., 0., -1.]; 6 * n * n];
        assert_eq!(project_probe(&inside, n, &BASES, 0.5).unwrap().state[0], 0.);
    }
    #[test]
    fn volume_sample_interpolates_and_fades() {
        let probe = |v: f32| ProbeSh { rows: [[v, 0., 0., 0.]; 3], state: [1., 0., 1., 0.] };
        let probes: Vec<_> = (0..8).map(|i| probe(if i & 1 == 1 { 2. } else { 0. })).collect();
        let (rows, fade) = sample_volume(&probes, [0.; 3], [1.; 3], [2, 2, 2], [0., 0., 0.]);
        assert!((rows[0][0] - 1.).abs() < 1e-6 && fade == 1.);
        let (_, fade) = sample_volume(&probes, [0.; 3], [1.; 3], [2, 2, 2], [1.5, 0., 0.]);
        assert!((fade - 0.75).abs() < 1e-6);
        assert_eq!(sample_volume(&probes, [0.; 3], [1.; 3], [2, 2, 2], [5., 0., 0.]).1, 0.);
    }
}
