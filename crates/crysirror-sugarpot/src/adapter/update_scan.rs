// SPDX-License-Identifier: MPL-2.0
//!435AB0 cases9..16 retain scan counters and copy whole native lines/columns.
//!This module declares bounded logical stripe events; shared RectMap workers
//!construct the mask and shared RuleTable composition selects image bytes.
use super::*;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
pub(super) type Stripes = BTreeSet<[i32; 4]>;
pub(super) fn complete(kind: i32, index: i32, stride: i32) -> bool {
    if kind <= 13 {
        index >= stride
    } else {
        i64::from(index) * i64::from(stride) >= if kind <= 16 { 1024 } else { 600 }
    }
}
pub(super) fn add(kind: i32, index: i32, stride: i32, stripes: &mut Stripes) -> Result<()> {
    ensure!(
        (1..=65536).contains(&stride),
        "SugarPot native update scan stride budget"
    );
    if kind <= 13 {
        let vertical = kind <= 11;
        let extent = if vertical { 1024 } else { 600 };
        let mut at = if matches!(kind, 11 | 13) {
            stride - index
        } else {
            index
        };
        while at < extent {
            if at >= 0 {
                stripes.insert(if vertical {
                    [at, 0, 1, 600]
                } else {
                    [0, at, 1024, 1]
                });
            }
            at = at
                .checked_add(stride)
                .context("SugarPot update scan coordinate")?;
        }
    } else {
        let extent = if kind <= 16 { 1024 } else { 600 };
        let limit = stride.wrapping_add(index).max(0);
        for j in 0..limit {
            let at = if matches!(kind, 16 | 18) {
                i64::from(extent) - i64::from(index) - i64::from(stride) * i64::from(j)
            } else {
                i64::from(index) + i64::from(stride) * i64::from(j)
            };
            //After leaving the meaningful image interval, increasing j can
            //only move farther away. Discard native wrapped-address writes
            //rather than scanning an overflowing, attacker-sized loop.
            if (matches!(kind, 16 | 18) && at < 0)
                || (!matches!(kind, 16 | 18) && at >= i64::from(extent))
            {
                break;
            }
            if at >= 0 && at < i64::from(extent) {
                let at = at as i32;
                stripes.insert(if kind <= 16 {
                    [at, 0, 1, 600]
                } else {
                    [0, at, 1024, 1]
                });
            } else if kind == 15 && (-1024..0).contains(&at) {
                //Case13 checks only signed column<1024: a negative column
                //lands in the preceding row's tail. Retain every initialized
                //in-buffer write; drop only its first-row out-of-bounds write.
                stripes.insert([1024 + at as i32, 0, 1, 599]);
            }
        }
    }
    ensure!(
        stripes.len() <= 2048,
        "SugarPot native update stripe budget"
    );
    Ok(())
}
pub(super) fn mask(stripes: &Stripes) -> (ImageInput, crysirror_image::effects::ImageEffect) {
    (
        ImageInput::Solid {
            size: [1024, 600],
            color: [255; 4],
        },
        crysirror_image::effects::ImageEffect::RectMap {
            spec: RectMap {
                output_size: None,
                regions: stripes
                    .iter()
                    .map(|rect| RectMapRegion {
                        source: [0, 0, 1, 1],
                        destination: *rect,
                    })
                    .collect(),
                clear: [0, 0, 0, 255],
                sample: RectMapSample::Floor,
                copy_alpha: true,
            },
        },
    )
}
