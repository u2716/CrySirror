// SPDX-License-Identifier: MPL-2.0
//! Gradient skies as equirectangular HDR environments: a zenith–horizon
//! gradient above the horizon, a dim ground below and a glow around the
//! sun, for engines whose sky is a parameterised time-of-day model rather
//! than an image. Rows run from +Y (top) to −Y; column 0 is −X, matching
//! the renderer's `equirect_uv`.
use crate::pbr::HdrImage;
use anyhow::{ensure, Context, Result};
use glam::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct GradientSky {
    /// Direction toward the sun (world, normalised by `render`).
    pub sun: [f32; 3],
    /// Linear radiance at the zenith and at the horizon.
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
    /// Radiance of the lower hemisphere.
    pub ground: [f32; 3],
    /// Sun glow colour (radiance at the sun) and its cosine exponent.
    pub glow: [f32; 3],
    pub glow_power: f32,
}

impl GradientSky {
    /// Radiance seen along `dir`.
    pub fn radiance(&self, dir: Vec3) -> Vec3 {
        let d = dir.normalize_or_zero();
        let sun = Vec3::from_array(self.sun).normalize_or_zero();
        let up = d.y;
        let base = if up >= 0. {
            // Horizon colour dominates near the horizon (√ falloff).
            Vec3::from_array(self.horizon).lerp(Vec3::from_array(self.zenith), up.max(0.).sqrt())
        } else {
            Vec3::from_array(self.horizon).lerp(Vec3::from_array(self.ground), (-up * 4.).min(1.))
        };
        let c = d.dot(sun).max(0.);
        base + Vec3::from_array(self.glow) * c.powf(self.glow_power.max(1.)) * if up >= -0.05 { 1. } else { 0. }
    }

    /// The sky as a `w`×`h` equirectangular image (`w = 2h`).
    pub fn render(&self, w: u32, h: u32) -> Result<HdrImage> {
        ensure!(w == h * 2 && (2..=4096).contains(&w), "sky size");
        let mut rgb = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            let theta = (y as f32 + 0.5) / h as f32 * std::f32::consts::PI;
            for x in 0..w {
                let phi = ((x as f32 + 0.5) / w as f32 - 0.5) * std::f32::consts::TAU;
                let dir = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
                rgb.push(self.radiance(dir).max(Vec3::ZERO).to_array());
            }
        }
        Ok(HdrImage { width: w, height: h, rgb })
    }
}

/// Encode linear RGB pixels as a Radiance `.hdr` file.
pub fn encode_hdr(img: &HdrImage) -> Result<Vec<u8>> {
    ensure!(img.rgb.len() == (img.width as usize) * (img.height as usize), "HDR pixel count");
    let mut out = Vec::new();
    let px: Vec<image::Rgb<f32>> = img.rgb.iter().map(|p| image::Rgb(*p)).collect();
    image::codecs::hdr::HdrEncoder::new(&mut out).encode(&px, img.width as usize, img.height as usize).context("encode HDR")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_round_trips_through_hdr_and_glows_toward_the_sun() {
        let sky = GradientSky { sun: [0., 0.2, 1.], zenith: [0.1, 0.2, 0.6], horizon: [0.6, 0.6, 0.7], ground: [0.1, 0.1, 0.1], glow: [2., 1., 0.5], glow_power: 16. };
        let img = sky.render(64, 32).unwrap();
        let back = crate::pbr::load_hdr(&encode_hdr(&img).unwrap()).unwrap();
        assert_eq!((back.width, back.height), (64, 32));
        let toward = sky.radiance(Vec3::new(0., 0.2, 1.));
        let away = sky.radiance(Vec3::new(0., 0.2, -1.));
        assert!(toward.x > away.x + 1.);
        assert!(sky.radiance(Vec3::Y).z > sky.radiance(Vec3::Y).x);
        assert!(sky.render(10, 10).is_err());
    }
}
