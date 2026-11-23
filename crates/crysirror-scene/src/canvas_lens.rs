//! Canvas-centred perspective with an explicit reference plane. This bounded
//! software-raster profile is useful for 2D scenes containing real3D poses:
//! ordinary canvas coordinates remain unchanged on `plane` and both UV
//! interpolation and near-plane admission are declared independently.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasLens {
    /// Additive [X,Y,Z] Euler angles in radians before each sprite's model
    /// rotation. This differs from rotating the completed scene/view matrix.
    /// Zero preserves existing geometry; native camera-angle subtraction can
    /// supply the negated camera channels without changing sprite motion.
    #[serde(default)]
    pub model_rotation_offset: [f32; 3],
    pub plane: f32,
    pub near: f32,
    pub far: f32,
    #[serde(default)]
    pub near_clamp: bool,
    #[serde(default)]
    pub affine_uv: bool,
    /// Legacy canvas rasterizers truncate projected coordinates into signed
    /// 16-bit vertices. Zero/negative reference distances are meaningful for
    /// this profile (collapsed/reflected geometry), without changing the
    /// ordinary positive-distance perspective contract.
    #[serde(default)]
    pub signed_16_xy: bool,
}
impl CanvasLens {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.model_rotation_offset.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.), "canvas model rotation offset");
        anyhow::ensure!(
            [self.plane, self.near, self.far]
                .iter()
                .all(|v| v.is_finite())
                && (self.plane > 0. || self.signed_16_xy)
                && (!self.signed_16_xy || self.plane.abs() <= 1_000_000.)
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
        let mut x = centre[0] + (point[0] - centre[0]) * self.plane / z;
        let mut y = centre[1] + (point[1] - centre[1]) * self.plane / z;
        if !x.is_finite() || !y.is_finite() { return None; }
        if self.signed_16_xy {
            x = (f64::from(x).trunc() as i64 as i16) as f32;
            y = (f64::from(y).trunc() as i64 as i16) as f32;
        }
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
