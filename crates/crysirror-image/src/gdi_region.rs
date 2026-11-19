// SPDX-License-Identifier: MPL-2.0
//! Point membership of GDI regions (`CreateRectRgn`, `CreateEllipticRgn`,
//! `CreateRoundRectRgn` and `CreatePolygonRgn`) for scripts that hit-test the
//! pointer with `PtInRegion`. Regions cover whole pixels: the right and bottom
//! edges are excluded, and curved or slanted edges include a pixel when its
//! centre lies inside the outline.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolygonFill {
    /// `ALTERNATE` (1): even-odd crossing parity.
    Alternate,
    /// `WINDING` (2): non-zero winding number.
    Winding,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GdiRegion {
    Rect { rect: [i32; 4] },
    Ellipse { rect: [i32; 4] },
    /// Corner ellipses have the given full width and height.
    RoundRect { rect: [i32; 4], corner: [i32; 2] },
    Polygon { points: Vec<[i32; 2]>, fill: PolygonFill },
}

/// GDI normalizes reversed coordinates before building a region.
fn ordered(rect: [i32; 4]) -> [i64; 4] {
    let [left, top, right, bottom] = rect.map(i64::from);
    [left.min(right), top.min(bottom), left.max(right), bottom.max(top)]
}

fn in_ellipse(x: f64, y: f64, [left, top, right, bottom]: [f64; 4]) -> bool {
    let (rx, ry) = ((right - left) / 2., (bottom - top) / 2.);
    if rx <= 0. || ry <= 0. {
        return false;
    }
    let (dx, dy) = ((x - (left + rx)) / rx, (y - (top + ry)) / ry);
    dx * dx + dy * dy <= 1.
}

impl GdiRegion {
    /// `PtInRegion` for a pixel coordinate.
    pub fn contains(&self, [x, y]: [i32; 2]) -> bool {
        let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        match self {
            Self::Rect { rect } => {
                let [left, top, right, bottom] = ordered(*rect);
                (left..right).contains(&i64::from(x)) && (top..bottom).contains(&i64::from(y))
            }
            Self::Ellipse { rect } => in_ellipse(px, py, ordered(*rect).map(|v| v as f64)),
            Self::RoundRect { rect, corner } => {
                let [left, top, right, bottom] = ordered(*rect).map(|v| v as f64);
                if !(left..right).contains(&px) || !(top..bottom).contains(&py) {
                    return false;
                }
                let rx = (f64::from(corner[0].unsigned_abs() as i32) / 2.).min((right - left) / 2.);
                let ry = (f64::from(corner[1].unsigned_abs() as i32) / 2.).min((bottom - top) / 2.);
                // Only the four corner boxes follow the corner ellipses.
                let cx = if px < left + rx {
                    left + rx
                } else if px > right - rx {
                    right - rx
                } else {
                    return true;
                };
                let cy = if py < top + ry {
                    top + ry
                } else if py > bottom - ry {
                    bottom - ry
                } else {
                    return true;
                };
                in_ellipse(px, py, [cx - rx, cy - ry, cx + rx, cy + ry])
            }
            Self::Polygon { points, fill } => {
                if points.len() < 3 {
                    return false;
                }
                let mut winding = 0i32;
                let mut crossings = 0u32;
                for (index, start) in points.iter().enumerate() {
                    let end = points[(index + 1) % points.len()];
                    let [x0, y0] = start.map(f64::from);
                    let [x1, y1] = end.map(f64::from);
                    if (y0 <= py) == (y1 <= py) {
                        continue;
                    }
                    let at = x0 + (py - y0) * (x1 - x0) / (y1 - y0);
                    if at > px {
                        crossings += 1;
                        winding += if y1 > y0 { 1 } else { -1 };
                    }
                }
                match fill {
                    PolygonFill::Alternate => crossings % 2 == 1,
                    PolygonFill::Winding => winding != 0,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_exclude_right_and_bottom() {
        let region = GdiRegion::Rect { rect: [10, 20, 30, 40] };
        assert!(region.contains([10, 20]));
        assert!(region.contains([29, 39]));
        assert!(!region.contains([30, 39]));
        assert!(!region.contains([29, 40]));
        assert!(GdiRegion::Rect { rect: [30, 40, 10, 20] }.contains([10, 20]));
    }

    #[test]
    fn curved_regions_sample_pixel_centres() {
        let ellipse = GdiRegion::Ellipse { rect: [0, 0, 100, 50] };
        assert!(ellipse.contains([50, 25]));
        assert!(!ellipse.contains([0, 0]));
        assert!(!ellipse.contains([99, 49]));
        let round = GdiRegion::RoundRect { rect: [0, 0, 100, 100], corner: [40, 40] };
        assert!(round.contains([50, 0]));
        assert!(!round.contains([0, 0]));
        assert!(round.contains([20, 20]));
    }

    #[test]
    fn polygon_fill_modes_differ_on_overlap() {
        // A pentagram: its centre has winding number 2.
        let star = vec![[50, 0], [80, 100], [0, 35], [100, 35], [20, 100]];
        let alternate = GdiRegion::Polygon { points: star.clone(), fill: PolygonFill::Alternate };
        let winding = GdiRegion::Polygon { points: star, fill: PolygonFill::Winding };
        assert!(!alternate.contains([50, 50]));
        assert!(winding.contains([50, 50]));
        assert!(alternate.contains([50, 20]));
    }
}
