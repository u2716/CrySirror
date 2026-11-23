//! Stateless noise-volume instance generation. The caller owns the immutable
//! random ring and clock, so CPU workers, World/VFX and scene layers can share
//! one reproducible sampling contract without a VM or device.
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug)]
pub enum NoiseVolumeShape {
    Box,
    Sphere { pi: f64 },
}
#[derive(Clone, Debug)]
pub struct NoiseVolume {
    pub shape: NoiseVolumeShape,
    pub count: f32,
    pub seed: u16,
    pub cells: u32,
    pub phase: [f32; 3],
    pub extent: [f32; 3],
    pub soft_edge: f32,
    pub timeline: [i64; 2],
    pub motions: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct VolumeInstance {
    pub position: [f32; 3],
    pub opacity: u8,
    pub time: i64,
    pub motion: u32,
}
impl NoiseVolume {
    pub fn sample(&self, ring: &[u32; 1024]) -> Result<Vec<VolumeInstance>> {
        ensure!(
            self.count.is_finite() && (0.0..=1024.).contains(&self.count),
            "noise volume instance budget"
        );
        ensure!(
            (1..=4096).contains(&self.cells) && (1..=65536).contains(&self.motions),
            "noise volume cells/motions"
        );
        ensure!(
            self.phase
                .iter()
                .chain(&self.extent)
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                && self.soft_edge.is_finite()
                && (0.0..=1000.).contains(&self.soft_edge),
            "noise volume finite parameters"
        );
        if let NoiseVolumeShape::Sphere { pi } = self.shape {
            ensure!(
                pi.is_finite() && (3.0..=3.2).contains(&pi),
                "noise volume angle profile"
            );
        }
        let noise = |index: u32| f64::from(ring[index as usize & 1023] & 0xffffff) / 16777215.;
        let whole = self.count.floor() as u32;
        let fraction = self.count.fract();
        let count = whole + u32::from(fraction >= 0.0000001);
        let last = if fraction >= 0.0000001 {
            (fraction * 255.).round() as i32
        } else {
            255
        };
        let seed = u32::from(self.seed).wrapping_mul(777);
        let base = f64::from(100 - 100 % self.cells);
        let mut out = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut phase = [
                noise(index + seed + 234) + base + f64::from(self.phase[0]),
                noise(index + seed + 555) + base + f64::from(self.phase[1]),
                noise(index + seed + 777) + base + f64::from(self.phase[2]),
            ];
            let cell = if self.cells > 1 {
                phase.map(|v| (v.floor() as i64 % i64::from(self.cells)) as i32)
            } else {
                [0; 3]
            };
            if cell[0] > 0 {
                phase[1] += noise((cell[0] as u32).wrapping_add(index).wrapping_add(123));
                phase[2] += noise((cell[0] as u32).wrapping_add(index).wrapping_add(678));
            }
            if cell[1] > 0 {
                phase[0] += noise((cell[1] as u32).wrapping_add(index).wrapping_add(789));
                phase[2] += noise((cell[1] as u32).wrapping_add(index).wrapping_add(234));
            }
            if cell[2] > 0 {
                phase[0] += noise((cell[2] as u32).wrapping_add(index).wrapping_add(345));
                phase[1] += noise((cell[2] as u32).wrapping_add(index).wrapping_add(899));
            }
            for value in &mut phase {
                *value = value.fract();
            }
            let code = (cell[2].wrapping_mul(16) | cell[0] | cell[1].wrapping_mul(4)) as u16;
            let selector = u32::from(code) ^ index.wrapping_add(seed);
            let r = noise(selector);
            let time = ((self.timeline[1] as f64 - self.timeline[0] as f64) * r
                + self.timeline[0] as f64)
                .trunc() as i64;
            let motion = ((f64::from(self.motions)
                * noise(u32::from(code) ^ index.wrapping_add(seed + 999)))
            .round() as u32)
                .min(self.motions - 1);
            let mut alpha = 255i32;
            if self.soft_edge > 0.0000001 {
                let fade = f64::from(self.soft_edge);
                let axes: &[f64] = if matches!(self.shape, NoiseVolumeShape::Sphere { .. }) {
                    &phase[2..]
                } else {
                    &phase
                };
                for &value in axes {
                    let mut edge = 255;
                    if value < fade {
                        edge = (value / fade * 255.).trunc() as i32;
                    }
                    if 1. - fade < value {
                        edge = ((1. - value) / fade * 255.).trunc() as i32;
                    }
                    alpha = alpha.min(edge);
                }
            }
            if index == count - 1 {
                alpha = alpha.min(last);
            }
            out.push(VolumeInstance {
                position: {
                    let coordinate = match self.shape {
                        NoiseVolumeShape::Box => phase,
                        NoiseVolumeShape::Sphere { pi } => {
                            let x = phase[0] * 2. * pi;
                            let y = phase[1] * 2. * pi;
                            let radius = phase[2] * 0.5;
                            let (sx, cx) = x.sin_cos();
                            let (sy, cy) = y.sin_cos();
                            [
                                radius * cx * sy + 0.5,
                                -radius * sx + 0.5,
                                radius * cx * cy + 0.5,
                            ]
                        }
                    };
                    std::array::from_fn(|axis| {
                        (coordinate[axis] * f64::from(self.extent[axis])) as f32
                    })
                },
                opacity: alpha.clamp(0, 255) as u8,
                time,
                motion,
            });
        }
        Ok(out)
    }
}
