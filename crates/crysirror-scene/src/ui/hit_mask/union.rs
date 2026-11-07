//! Retained unions of immutable input regions; no composed coverage bitmap.
use super::*;
impl WidgetHitMask {
    pub fn union(regions: &[([f32; 4], Self)], clip: [f32; 4]) -> Result<Option<([f32; 4], Self)>> {
        ensure!(regions.len() <= 256, "UI hit union region budget");
        if regions.is_empty() {
            return Ok(None);
        }
        let mut lower = [f32::INFINITY; 2];
        let mut upper = [f32::NEG_INFINITY; 2];
        for (rect, mask) in regions {
            ensure!(crate::layer::rect_valid(*rect), "UI hit union rectangle");
            mask.validate(*rect)?;
            for axis in 0..2 {
                lower[axis] = lower[axis].min(rect[axis]);
                upper[axis] = upper[axis].max(rect[axis] + rect[axis + 2]);
            }
        }
        let Some(bounds) = crate::layer::intersect(
            [lower[0], lower[1], upper[0] - lower[0], upper[1] - lower[1]],
            clip,
        ) else {
            return Ok(None);
        };
        let mut parts = Vec::new();
        for (rect, mask) in regions {
            let mut leaf = mask.clone();
            let children = std::mem::take(&mut leaf.regions);
            parts.push(WidgetHitRegion {
                rect: [rect[0] - bounds[0], rect[1] - bounds[1], rect[2], rect[3]],
                mask: Box::new(leaf),
            });
            for child in children {
                parts.push(WidgetHitRegion {
                    rect: [
                        rect[0] + child.rect[0] - bounds[0],
                        rect[1] + child.rect[1] - bounds[1],
                        child.rect[2],
                        child.rect[3],
                    ],
                    mask: child.mask,
                });
            }
        }
        ensure!(parts.len() <= 256, "UI hit union leaf budget");
        let mask = Self {
            size: [1, 1],
            coverage: Arc::from([0u8]),
            threshold: 1,
            strict_bounds: false,
            source_from_local: Some([0., 0., 0., 0., 0., 0., 0., 0., 1.]),
            regions: parts,
        };
        mask.validate(bounds)?;
        Ok(Some((bounds, mask)))
    }
}
