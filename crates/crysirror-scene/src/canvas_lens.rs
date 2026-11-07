//! Canvas-centred perspective with an explicit reference plane. This bounded
//! software-raster profile is useful for 2D scenes containing real3D poses:
//! ordinary canvas coordinates remain unchanged on `plane` and both UV
//! interpolation and near-plane admission are declared independently.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasLens {
    pub plane: f32,
    pub near: f32,
    pub far: f32,
    #[serde(default)]
    pub near_clamp: bool,
    #[serde(default)]
    pub affine_uv: bool,
}
impl CanvasLens {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            [self.plane, self.near, self.far]
                .iter()
                .all(|v| v.is_finite())
                && self.plane > 0.
                && self.near > 0.
                && self.far > self.near
                && self.far <= 1_000_000.,
            "canvas perspective lens bounds"
        );
        Ok(())
    }
    pub(super) fn point(
        &self,
        point: [f32; 3],
        canvas: [f32; 2],
    ) -> Option<super::projection::ProjectedPoint> {
        let mut z = if self.near_clamp {
            point[2].max(self.near)
        } else {
            point[2]
        };
        if z.abs() < 1e-20 {
            z = 1e-20f32.copysign(z)
        }
        let centre = canvas.map(|v| v * 0.5);
        let x = centre[0] + (point[0] - centre[0]) * self.plane / z;
        let y = centre[1] + (point[1] - centre[1]) * self.plane / z;
        let depth =
            self.far / (self.far - self.near) - self.near * self.far / ((self.far - self.near) * z);
        let depth = if self.near_clamp {
            depth.clamp(0., 1.)
        } else {
            depth
        };
        [x, y, depth, z].iter().all(|v| v.is_finite()).then_some(
            super::projection::ProjectedPoint {
                x,
                y,
                depth,
                clip_w: if self.affine_uv { 1. } else { z },
            },
        )
    }
    pub fn admits(&self, points: impl IntoIterator<Item = [f32; 3]>) -> bool {
        points.into_iter().any(|v| v[2] >= self.near)
    }
}
