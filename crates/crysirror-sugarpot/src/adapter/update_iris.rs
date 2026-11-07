// SPDX-License-Identifier: MPL-2.0
//!UD25/26 ->435AB0 native23/24 ->410B10: a midpoint circle about (512,300)
//!whose row spans copy the inside source; the rest of each drawn row copies
//!the outside source. UD25 closes over the old stage (radius
//!trunc((255-p)*2.325490236282349), 408DB0 restoring the target under the
//!previous bounding square); UD26 opens the target over the old stage
//!(radius trunc(p*2.3254902362823486), p clamped to 1..255). The union of
//!spans is an immutable shared RectMap mask; no VM pixel loop exists.
use super::*;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};

const WIDTH: i32 = 1024;
const HEIGHT: i32 = 600;

/// Radius of a sample.
pub(super) fn radius(kind: i32, phase: u32) -> i32 {
    let p = phase.clamp(1, 255) as i32;
    if kind == 25 {
        (f64::from(255 - p) * 2.325490236282349) as i32
    } else {
        (f64::from(p) * 2.3254902362823486) as i32
    }
}

/// 410B10's row spans, merged to the widest span per row: (y, left, right).
pub(super) fn spans(radius: i32) -> Vec<(i32, i32, i32)> {
    let mut rows = std::collections::BTreeMap::<i32, (i32, i32)>::new();
    if radius < 0 {
        return Vec::new();
    }
    let mut put = |y: i32, left: i32, right: i32| {
        if !(0..HEIGHT).contains(&y) {
            return;
        }
        let (left, right) = (left.max(0), right.min(WIDTH));
        if left >= right {
            return;
        }
        let entry = rows.entry(y).or_insert((left, right));
        entry.0 = entry.0.min(left);
        entry.1 = entry.1.max(right);
    };
    let mut error = radius;
    let mut step = 2 * radius;
    let mut odd = 0;
    let (mut bottom, mut top) = (radius + 300, 300 - radius);
    let (mut down, mut up) = (300, 300);
    let (mut extent_right, mut extent_left) = (radius + 300, 300 - radius);
    loop {
        put(down, extent_left + 212, extent_right + 212);
        put(up, top + 212, bottom + 212);
        put(bottom, up + 212, down + 212);
        put(top, up + 212, down + 212);
        error += -1 - odd;
        if error < 0 {
            error += step - 2;
            bottom -= 1;
            top += 1;
            step -= 2;
        }
        up -= 1;
        extent_right = bottom;
        odd += 2;
        let k = down - 299;
        down += 1;
        if bottom - 300 < k {
            break;
        }
        extent_left = top;
    }
    rows.into_iter().map(|(y, (l, r))| (y, l, r)).collect()
}

/// The inside-source mask (white inside, black elsewhere).
pub(super) fn mask(radius: i32) -> (ImageInput, crysirror_image::effects::ImageEffect) {
    (
        ImageInput::Solid { size: [1024, 600], color: [255; 4] },
        crysirror_image::effects::ImageEffect::RectMap {
            spec: RectMap {
                output_size: None,
                regions: spans(radius)
                    .into_iter()
                    .map(|(y, left, right)| RectMapRegion {
                        source: [0, 0, 1, 1],
                        destination: [left, y, right - left, 1],
                    })
                    .collect(),
                clear: [0, 0, 0, 255],
                sample: RectMapSample::Floor,
                copy_alpha: true,
            },
        },
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn spans_are_a_centred_disc() {
        let s = super::spans(100);
        assert_eq!(s.len(), 201);
        let middle = s.iter().find(|r| r.0 == 300).unwrap();
        assert_eq!((middle.1, middle.2), (412, 612));
        let top = s.first().unwrap();
        assert_eq!(top.0, 200);
        assert!(top.2 - top.1 < 40);
        assert!(super::spans(700).iter().all(|r| r.1 == 0 && r.2 == 1024));
    }
}
