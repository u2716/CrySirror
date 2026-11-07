// SPDX-License-Identifier: MPL-2.0
//! Native mono policy tables. Shared image workers own all pixel traversal.
use crysirror_image::{
    effects::ImageEffect,
    luma_lookup::{LookupIndex, LumaLookup},
    rgb_lut::{RgbLookup, RgbLookupProfile},
};
pub(crate) fn effects(mode: i32) -> Vec<ImageEffect> {
    if matches!(mode, 1 | 2) || !(3..=8).contains(&mode) {
        let table = (0..256_i32)
            .map(|v| {
                if mode == 2 {
                    [
                        (v + ((48 * (255 - v)) >> 8)) as u8,
                        (v - (v >> 5)) as u8,
                        (v - ((56 * v) >> 8)) as u8,
                        255,
                    ]
                } else {
                    [v as u8, v as u8, v as u8, 255]
                }
            })
            .collect();
        return vec![ImageEffect::LumaLookup {
            spec: LumaLookup {
                table,
                index: LookupIndex::WeightedRgb {
                    coefficients: [28, 77, 151],
                    fractional_bits: 8,
                },
                preserve_alpha: true,
            },
        }];
    }
    let table = |channel| {
        (0..256_i32)
            .map(|v| {
                (match (mode, channel) {
                    (3, 1 | 2) => v - ((51 * v) >> 8),
                    (4, 0 | 1) => v - (v >> 2),
                    (4, 2) => v + ((255 - v) >> 3),
                    (5, _) => 255 - v,
                    (6, 0) => v + ((96 * (255 - v)) >> 8),
                    (6, 1) => v - ((80 * v) >> 8),
                    (7, 0 | 2) => v - ((96 * v) >> 8),
                    (7, 1) => v + ((255 - v) >> 3),
                    (8, 0) => v + ((255 - v) >> 3),
                    (8, 1 | 2) => v - ((192 * v) >> 8),
                    _ => v,
                }) as u8
            })
            .collect()
    };
    vec![ImageEffect::RgbLookup {
        spec: RgbLookup {
            red: table(0),
            green: table(1),
            blue: table(2),
            profile: RgbLookupProfile::StraightRgba,
        },
    }]
}
