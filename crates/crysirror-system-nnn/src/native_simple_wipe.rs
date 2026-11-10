// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CAllSimpleWipe's selector table and integer time policy. Geometry and all
//! pixel copying live in shared Image; original numeric IDs remain native.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageInput, ImageResource};
use crysirror_image::{
    byte_raster::ByteRaster,
    effects::ImageEffect,
    fixed_mix::{FixedMixAlpha, FixedMixRounding, FixedMixSpec},
    integer_wipe::{
        IntegerWipe, IntegerWipeGeometry, scale::ScaleWipe, scanline::ScanlineWipe,
        shapes::ShapeWipe,
    },
    transition::ImageTransition,
};
use std::sync::Arc;
mod fragments;
#[derive(Clone, Default)]
pub(crate) struct Tables {
    pub rows: Option<Arc<[u16]>>,
}
pub(crate) fn needs_random(kind: i32) -> bool {
    matches!(kind, 24 | 25 | 38 | 39 | 48..=53)
}
pub(crate) fn plan_with_random(
    kind: i32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    count: u32,
    duration: u32,
    random: &mut crysirror_image::temporal_pattern::PatternRng,
    tables: &mut Tables,
) -> Result<(ImageInput, Vec<ImageEffect>)> {
    if !needs_random(kind) {
        return plan(kind, old, new, count, duration);
    }
    use crysirror_image::integer_wipe::random::{RandomWipe, RandomWipeGeometry};
    let duration = duration.max(1);
    let count = count.min(duration);
    let [_, height] = old.size();
    let draw = |random: &mut crysirror_image::temporal_pattern::PatternRng| -> Result<u16> {
        Ok((random.next_draw()? & 32767) as u16)
    };
    let (geometry, samples) = match kind {
        24 | 25 | 48..=53 => {
            if tables.rows.is_none() {
                tables.rows = Some(
                    (0..height * 2)
                        .map(|_| draw(random))
                        .collect::<Result<Vec<_>>>()?
                        .into(),
                );
            }
            let rows = tables.rows.as_ref().unwrap();
            ensure!(
                rows.len() == height as usize * 2,
                "NNN shared wipe random-table canvas changed"
            );
            (
                RandomWipeGeometry::RaggedEdge {
                    reverse: kind == 25 || kind >= 51,
                    gradient: if kind < 48 {
                        0
                    } else {
                        64 << ((kind - 48) % 3)
                    },
                },
                rows.to_vec(),
            )
        }
        38 => {
            let half = (duration / 2).max(1);
            let c = if count >= half {
                (duration - count).min(half)
            } else {
                count
            };
            let cell = ((u64::from(c) + 1) * 64 / u64::from(half)).clamp(1, 64);
            (
                RandomWipeGeometry::Mosaic,
                if cell == 1 {
                    vec![]
                } else {
                    vec![draw(random)?, draw(random)?]
                },
            )
        }
        39 => (
            RandomWipeGeometry::RowDissolve,
            (0..height)
                .map(|_| draw(random))
                .collect::<Result<Vec<_>>>()?,
        ),
        _ => unreachable!(),
    };
    Ok((
        ImageInput::Raster {
            destination: Box::new(ImageInput::Resource(old)),
            source: Box::new(ImageInput::Resource(new)),
        },
        vec![ImageEffect::ByteRaster {
            spec: ByteRaster::RandomWipe {
                spec: RandomWipe {
                    geometry,
                    count,
                    duration,
                    samples,
                },
            },
        }],
    ))
}
pub(crate) fn supported(kind: i32) -> bool {
    (0..96).contains(&kind)
}
pub(crate) fn plan(
    kind: i32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    count: u32,
    duration: u32,
) -> Result<(ImageInput, Vec<ImageEffect>)> {
    ensure!(
        supported(kind),
        "NNN native simple wipe {kind} requires its exact shared profile"
    );
    ensure!(
        !needs_random(kind),
        "NNN randomized wipe requires its native RNG owner"
    );
    let count = count.min(duration.max(1));
    if matches!(kind, 34 | 35) {
        return fragments::plan(old, new, count, duration.max(1), kind == 35);
    }

    if kind == 0 {
        let percent = u64::from(count) * 100 / u64::from(duration.max(1));
        return Ok((
            ImageInput::Transition {
                from: old,
                to: new,
                mask: None,
            },
            vec![ImageEffect::Transition {
                spec: ImageTransition {
                    progress: percent as f32 / 100.,
                    softness: 0.,
                    blur: 0.,
                    rule_table: None,
                    fixed_weighted: Some(FixedMixSpec {
                        left_weight: ((100 - percent) * 256 / 100) as u32,
                        right_weight: (percent * 256 / 100) as u32,
                        fractional_bits: 8,
                        rounding: FixedMixRounding::SumThenShift,
                        alpha: FixedMixAlpha::Opaque,
                    }),
                },
            }],
        ));
    }
    let geometry = match kind {
        80..=83 => IntegerWipeGeometry::Scale {
            spec: ScaleWipe::CentreZoom {
                level: (kind - 80) as u8,
            },
        },
        84..=87 => IntegerWipeGeometry::Scale {
            spec: ScaleWipe::ExpandingPanel {
                reverse: kind % 2 == 1,
                blend: kind >= 86,
            },
        },
        28 | 29 => IntegerWipeGeometry::Shape {
            spec: ShapeWipe::Star {
                reverse: kind == 29,
            },
        },
        30 | 31 => IntegerWipeGeometry::Shape {
            spec: ShapeWipe::Heart {
                reverse: kind == 31,
            },
        },
        32 | 33 | 78 | 79 => IntegerWipeGeometry::Shape {
            spec: ShapeWipe::RotatingStar {
                reverse: kind % 2 == 1,
                slow: kind >= 78,
            },
        },
        26 | 27 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::CosineWarp {
                black_edges: kind == 27,
            },
        },
        36 | 37 | 54..=59 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::WaveEdge {
                reverse: kind == 37 || kind >= 57,
                gradient: if kind < 54 {
                    0
                } else {
                    64 << ((kind - 54) % 3)
                },
            },
        },
        41 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::FoldRows,
        },
        42..=47 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::AcceleratingRows {
                origin: ((kind - 42) / 2) as u8,
                reverse: (kind - 42) % 2 != 0,
            },
        },
        60 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::MultiplyCross,
        },
        61..=64 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::PushPanels {
                direction: (kind - 61) as u8,
            },
        },
        65..=68 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::SlantEdge {
                orientation: (kind - 65) as u8,
                gradient: 64,
            },
        },
        88..=93 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::ChevronEdge {
                inverted: kind == 89 || kind >= 92,
                gradient: if kind < 90 {
                    0
                } else {
                    64 << (((kind - 90) % 2) * 2)
                },
            },
        },
        94 | 95 => IntegerWipeGeometry::Scanline {
            spec: ScanlineWipe::DoubleChevron {
                reverse: kind == 95,
            },
        },
        1 | 2 => IntegerWipeGeometry::StaggeredBars {
            rows: kind == 2,
            bar: 16,
            repeat: 2,
            rounded_groups: kind == 2,
        },
        3 => IntegerWipeGeometry::InterleavedRows,
        40 => IntegerWipeGeometry::AdditiveCross,
        69..=72 | 74..=77 => IntegerWipeGeometry::StripRoll {
            columns: kind < 73,
            strips: (if kind < 73 { kind - 69 } else { kind - 74 }) as u8 + 2,
            turns: [2, 3, 5, 4, 1],
        },
        22 => IntegerWipeGeometry::EdgeSweep,
        23 | 73 => IntegerWipeGeometry::BouncingPanel {
            trailing_restarts: kind == 73,
        },
        6 | 7 => IntegerWipeGeometry::Circular { reverse: kind == 7 },
        8 => IntegerWipeGeometry::PackedCircular,
        9 => IntegerWipeGeometry::Clock { pi: 3.14159 },
        4 | 5 => IntegerWipeGeometry::Diagonal {
            translate_old: kind == 5,
        },
        10..=17 => IntegerWipeGeometry::Panel {
            direction: ((kind - 10) % 4) as u8,
            reverse: kind >= 14,
        },
        18..=21 => IntegerWipeGeometry::SplitPanels {
            columns: (kind - 18) % 2 == 1,
            reverse: kind >= 20,
        },
        _ => unreachable!(),
    };
    Ok((
        ImageInput::Raster {
            destination: Box::new(ImageInput::Resource(old)),
            source: Box::new(ImageInput::Resource(new)),
        },
        vec![ImageEffect::ByteRaster {
            spec: ByteRaster::IntegerWipe {
                spec: IntegerWipe {
                    geometry,
                    count,
                    duration,
                },
            },
        }],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_image::{ImageSurface, temporal_pattern::PatternRng};
    #[derive(Debug)]
    struct Image(ImageSurface);
    impl ImageResource for Image {
        fn client(&self) -> crysirror_core::state::ClientId {
            crysirror_core::state::ClientId(1)
        }
        fn size(&self) -> [u32; 2] {
            [self.0.width, self.0.height]
        }
        fn rgba(&self) -> Arc<[u8]> {
            self.0.rgba.clone().into()
        }
        fn opaque(&self) -> bool {
            true
        }
    }
    fn surface(image: &Arc<dyn ImageResource>) -> ImageSurface {
        let [width, height] = image.size();
        ImageSurface {
            width,
            height,
            rgba: image.rgba().to_vec(),
        }
    }
    #[test]
    fn all_registered_wipes_run_shared_kernels_and_preserve_native_random_table_lifetime() {
        let image = |offset: u8| -> Arc<dyn ImageResource> {
            Arc::new(Image(ImageSurface {
                width: 64,
                height: 48,
                rgba: (0..64 * 48)
                    .flat_map(|i| [offset.wrapping_add(i as u8), (i / 64) as u8, offset, 255])
                    .collect(),
            }))
        };
        let old = image(0);
        let new = image(101);
        let mut random = PatternRng::msvc(7);
        let mut tables = Tables::default();
        for kind in 0..96 {
            for count in [0, 1, 5, 9, 10] {
                let (input, effects) = plan_with_random(
                    kind,
                    old.clone(),
                    new.clone(),
                    count,
                    10,
                    &mut random,
                    &mut tables,
                )
                .unwrap();
                let effect = serde_json::from_slice::<Vec<ImageEffect>>(
                    &serde_json::to_vec(&effects).unwrap(),
                )
                .unwrap()
                .remove(0);
                let output = match (input, effect) {
                    (
                        ImageInput::Raster {
                            destination,
                            source,
                        },
                        ImageEffect::ByteRaster { spec },
                    ) => {
                        let (ImageInput::Resource(d), ImageInput::Resource(s)) =
                            (*destination, *source)
                        else {
                            panic!("unexpected nested source")
                        };
                        spec.apply(&surface(&d), &surface(&s), &|| Ok(())).unwrap()
                    }
                    (
                        ImageInput::Transition {
                            from,
                            to,
                            mask: None,
                        },
                        ImageEffect::Transition { spec },
                    ) => surface(&from)
                        .transition_to(&surface(&to), None, &spec, &|| Ok(()))
                        .unwrap(),
                    _ => panic!("native selector {kind} escaped the shared two-image contract"),
                };
                assert_eq!([output.width, output.height], [64, 48], "selector {kind}");
                assert!(
                    output.rgba.chunks_exact(4).all(|p| p[3] == 255),
                    "opaque XRGB selector {kind}"
                );
            }
        }
        let mut rng = PatternRng::msvc(19);
        let mut expected = rng.clone();
        let mut tables = Tables::default();
        for _ in 0..96 {
            expected.next_draw().unwrap();
        }
        plan_with_random(24, old.clone(), new.clone(), 0, 10, &mut rng, &mut tables).unwrap();
        assert_eq!(rng, expected);
        plan_with_random(53, old.clone(), new.clone(), 5, 10, &mut rng, &mut tables).unwrap();
        assert_eq!(
            rng, expected,
            "lazy Y table reused across selectors and frames"
        );
        for _ in 0..48 {
            expected.next_draw().unwrap();
        }
        plan_with_random(39, old, new, 5, 10, &mut rng, &mut tables).unwrap();
        assert_eq!(rng, expected, "row dissolve consumes one new draw per row");
        assert!(!supported(96));
    }
}
