//! Worker-ready transformed coverage grids. Geometry and sampling remain usable
//! by authored UI without a native adapter, graphics device or frame capture.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct AffineCoverage {
    pub source_size: [u32; 2],
    /// Destination position of `pivot` in the untransformed source image.
    pub position: [f32; 2],
    pub pivot: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
}
impl AffineCoverage {
    pub fn bounds(&self) -> Result<Option<[f32; 4]>> {
        ensure!(
            self.source_size.iter().all(|v| *v > 0 && *v <= 8192),
            "coverage source dimensions"
        );
        ensure!(
            self.position
                .iter()
                .chain(&self.pivot)
                .chain(&self.scale)
                .chain(std::iter::once(&self.rotation))
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "affine coverage bounds"
        );
        if self.scale.contains(&0.) {
            return Ok(None);
        }
        let (s, c) = self.rotation.sin_cos();
        let mut lo = [f32::INFINITY; 2];
        let mut hi = [f32::NEG_INFINITY; 2];
        for point in [
            [0., 0.],
            [self.source_size[0] as f32, 0.],
            [self.source_size[0] as f32, self.source_size[1] as f32],
            [0., self.source_size[1] as f32],
        ] {
            let p = [
                (point[0] - self.pivot[0]) * self.scale[0],
                (point[1] - self.pivot[1]) * self.scale[1],
            ];
            let p = [
                self.position[0] + p[0] * c - p[1] * s,
                self.position[1] + p[0] * s + p[1] * c,
            ];
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        let origin = lo.map(f32::floor);
        let size = std::array::from_fn::<_, 2, _>(|i| hi[i].ceil() - origin[i]);
        ensure!(
            size.iter().all(|v| *v > 0. && *v <= 8192.),
            "transformed coverage dimensions"
        );
        ensure!(
            f64::from(size[0]) * f64::from(size[1]) <= 16. * 1024. * 1024.,
            "transformed coverage grid budget"
        );
        Ok(Some([origin[0], origin[1], size[0], size[1]]))
    }
    /// Bind an immutable untransformed source alpha plane to current geometry.
    /// This is constant-size owner work; only source extraction needs a worker.
    pub fn source_mask(&self, source: &WidgetHitMask) -> Result<Option<([f32; 4], WidgetHitMask)>> {
        ensure!(
            source.size == self.source_size
                && source.source_from_local.is_none()
                && source.regions.is_empty(),
            "affine coverage requires an original source plane"
        );
        source.validate([
            0.,
            0.,
            self.source_size[0] as f32,
            self.source_size[1] as f32,
        ])?;
        let Some(bounds) = self.bounds()? else {
            return Ok(None);
        };
        let (s, c) = self.rotation.sin_cos();
        let [sx, sy] = self.scale;
        let x = bounds[0] - self.position[0];
        let y = bounds[1] - self.position[1];
        let mut mask = source.clone();
        mask.source_from_local = Some([
            c / sx,
            s / sx,
            (x * c + y * s) / sx + self.pivot[0],
            -s / sy,
            c / sy,
            (-x * s + y * c) / sy + self.pivot[1],
            0.,
            0.,
            1.,
        ]);
        mask.validate(bounds)?;
        Ok(Some((bounds, mask)))
    }
    pub fn from_rgba(
        &self,
        rgba: &[u8],
        threshold: u8,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Option<WidgetHitMask>> {
        let Some(bounds) = self.bounds()? else {
            return Ok(None);
        };
        ensure!(
            u64::from(self.source_size[0]) * u64::from(self.source_size[1]) * 4
                == rgba.len() as u64,
            "coverage RGBA length"
        );
        let size = [bounds[2] as u32, bounds[3] as u32];
        let (s, c) = self.rotation.sin_cos();
        let mut coverage = vec![0u8; size[0] as usize * size[1] as usize];
        for y in 0..size[1] {
            check()?;
            for x in 0..size[0] {
                let p = [
                    bounds[0] + x as f32 - self.position[0],
                    bounds[1] + y as f32 - self.position[1],
                ];
                let source = [
                    (p[0] * c + p[1] * s) / self.scale[0] + self.pivot[0],
                    (-p[0] * s + p[1] * c) / self.scale[1] + self.pivot[1],
                ];
                let source = source.map(f32::floor);
                if source[0] >= 0.
                    && source[1] >= 0.
                    && source[0] < self.source_size[0] as f32
                    && source[1] < self.source_size[1] as f32
                {
                    let at = ((source[1] as u32 * self.source_size[0] + source[0] as u32) * 4 + 3)
                        as usize;
                    coverage[(y * size[0] + x) as usize] = rgba[at];
                }
            }
        }
        check()?;
        Ok(Some(WidgetHitMask {
            size,
            coverage: coverage.into(),
            threshold,
            strict_bounds: false,
            source_from_local: None,
            regions: Vec::new(),
        }))
    }
}

impl WidgetHitMask {
    /// Bind an immutable source plane to projected geometry. Coverage stays in
    /// source space; clipping only bounds the widget's logical input rectangle.
    pub fn projected_source(
        &self,
        quad: [[f32; 2]; 4],
        clip: [f32; 4],
    ) -> Result<Option<([f32; 4], Self)>> {
        ensure!(
            self.source_from_local.is_none() && self.regions.is_empty(),
            "projected coverage requires original source plane"
        );
        self.validate([0., 0., self.size[0] as f32, self.size[1] as f32])?;
        ensure!(
            clip.iter().all(|v| v.is_finite()) && clip[2] > 0. && clip[3] > 0.,
            "projected hit clip"
        );
        let lo = [0, 1].map(|axis| {
            quad.iter()
                .map(|p| p[axis])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(clip[axis])
        });
        let hi = [0, 1].map(|axis| {
            quad.iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .min(clip[axis] + clip[axis + 2])
        });
        if hi[0] <= lo[0] || hi[1] <= lo[1] {
            return Ok(None);
        }
        let bounds = [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]];
        ensure!(
            bounds[2] <= 8192. && bounds[3] <= 8192.,
            "projected hit bounds"
        );
        let mut winding = None;
        for index in 0..4 {
            let a = quad[index];
            let b = quad[(index + 1) % 4];
            let c = quad[(index + 2) % 4];
            let cross = f64::from(b[0] - a[0]) * f64::from(c[1] - b[1])
                - f64::from(b[1] - a[1]) * f64::from(c[0] - b[0]);
            if cross.abs() <= 1.0e-9 {
                return Ok(None);
            }
            if winding.is_some_and(|positive| positive != (cross > 0.)) {
                return Ok(None);
            }
            winding = Some(cross > 0.);
        }
        let local = quad.map(|point| [point[0] - lo[0], point[1] - lo[1]]);
        let mut inverse = crysirror_image::warp::inverse_quad_transform(local)?;
        for value in &mut inverse[..3] {
            *value *= f64::from(self.size[0]);
        }
        for value in &mut inverse[3..6] {
            *value *= f64::from(self.size[1]);
        }
        let mut mask = self.clone();
        mask.source_from_local = Some(inverse.map(|value| value as f32));
        mask.validate(bounds)?;
        Ok(Some((bounds, mask)))
    }
}
