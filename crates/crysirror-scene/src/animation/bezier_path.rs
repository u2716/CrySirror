// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Immutable cubic Bezier positions for2D/3D animation. Caller owns clocks and
//! final coordinate quantisation; evaluation keeps explicit f32 operation order.
use anyhow::{Result, ensure};
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CubicBezierPath {
    pub control: [[f32; 3]; 4],
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuadraticBezierPath {
    pub control: [[f32; 3]; 3],
}
impl QuadraticBezierPath {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.control
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1.0e12),
            "quadratic Bezier control extent"
        );
        Ok(())
    }
    pub fn sample(&self, t: f32) -> Result<[f32; 3]> {
        self.validate()?;
        ensure!(
            t.is_finite() && t.abs() <= 4096.,
            "quadratic Bezier sample phase"
        );
        let t2 = t * t;
        let t1 = t * (1. - t);
        let t0 = (1. - t) * (1. - t);
        let value = std::array::from_fn(|axis| {
            t0 * self.control[0][axis]
                + 2. * t1 * self.control[1][axis]
                + t2 * self.control[2][axis]
        });
        ensure!(
            value.iter().all(|v| v.is_finite()),
            "quadratic Bezier result extent"
        );
        Ok(value)
    }
}
impl CubicBezierPath {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.control
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1.0e12),
            "Bezier control extent"
        );
        Ok(())
    }
    pub fn sample(&self, t: f32) -> Result<[f32; 3]> {
        self.validate()?;
        ensure!(t.is_finite() && t.abs() <= 4096., "Bezier sample phase");
        let t3 = t * t * t;
        let t2 = t * t * (1. - t);
        let t1 = t * (1. - t) * (1. - t);
        let t0 = (1. - t) * (1. - t) * (1. - t);
        let value = std::array::from_fn(|axis| {
            t0 * self.control[0][axis]
                + 3. * t1 * self.control[1][axis]
                + 3. * t2 * self.control[2][axis]
                + t3 * self.control[3][axis]
        });
        ensure!(value.iter().all(|v| v.is_finite()), "Bezier result extent");
        Ok(value)
    }
}
