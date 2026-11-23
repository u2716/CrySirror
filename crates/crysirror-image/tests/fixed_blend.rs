//! Independent stored-byte oracles; no VM, renderer or GPU.
use crysirror_core::image_blend::{
    RgbaBlendComponent as C, RgbaBlendEquation as E, RgbaBlendFactor as F, RgbaBlendState as S,
};
use crysirror_image::{bitmap::BitmapRect, ImageSurface};
use std::cell::Cell;
fn component(source: F, destination: F, equation: E) -> C {
    C {
        source,
        destination,
        equation,
    }
}
fn pixel(source: [u8; 4], destination: [u8; 4], state: S) -> [u8; 4] {
    let source = ImageSurface::filled(1, 1, source).unwrap();
    let mut destination = ImageSurface::filled(1, 1, destination).unwrap();
    destination
        .blend_fixed_factors(&source, BitmapRect::new(0, 0, 1, 1), [0, 0], state, &|| {
            Ok(())
        })
        .unwrap();
    destination.rgba.try_into().unwrap()
}
#[test]
fn each_factor_uses_original_components_and_independent_alpha() {
    let cases = [
        (F::Zero, [0, 0, 0, 0]),
        (F::One, [64, 128, 192, 128]),
        (F::SourceColor, [16, 64, 145, 64]),
        (F::OneMinusSourceColor, [48, 64, 47, 64]),
        (F::DestinationColor, [32, 32, 24, 32]),
        (F::OneMinusDestinationColor, [32, 96, 168, 96]),
        (F::SourceAlpha, [32, 64, 96, 64]),
        (F::OneMinusSourceAlpha, [32, 64, 96, 64]),
        (F::DestinationAlpha, [16, 32, 48, 32]),
        (F::OneMinusDestinationAlpha, [48, 96, 144, 96]),
        (F::ConstantColor, [16, 64, 144, 64]),
        (F::OneMinusConstantColor, [48, 64, 48, 64]),
        (F::ConstantAlpha, [32, 64, 96, 64]),
        (F::OneMinusConstantAlpha, [32, 64, 96, 64]),
        (F::SourceAlphaSaturate, [32, 64, 96, 128]),
    ];
    for (factor, expected) in cases {
        let mode = component(factor, F::Zero, E::Add);
        assert_eq!(
            pixel(
                [64, 128, 192, 128],
                [128, 64, 32, 64],
                S {
                    color: mode,
                    alpha: mode,
                    constant: [0.25, 0.5, 0.75, 0.5],
                    ..Default::default()
                }
            ),
            expected,
            "{factor:?}"
        );
    }
    // Destination factors also read the complete pre-write source snapshot.
    assert_eq!(
        pixel(
            [64, 128, 192, 128],
            [128, 64, 32, 64],
            S {
                color: component(F::Zero, F::SourceColor, E::Add),
                alpha: component(F::Zero, F::SourceAlpha, E::Add),
                ..Default::default()
            }
        ),
        [32, 32, 24, 32]
    );
}
#[test]
fn source_alpha_interpolation_matches_integer_oracle_for_all_alpha_bytes() {
    let mut source = ImageSurface::filled(256, 7, [0; 4]).unwrap();
    let mut target = source.clone();
    for (row, color) in [0u8, 1, 2, 127, 128, 254, 255].into_iter().enumerate() {
        for alpha in 0..256usize {
            let at = (row * 256 + alpha) * 4;
            source.rgba[at..at + 4].copy_from_slice(&[color, 255 - color, 93, alpha as u8]);
            target.rgba[at..at + 4].copy_from_slice(&[255 - color, color, 177, 37]);
        }
    }
    target
        .blend_fixed_factors(
            &source,
            BitmapRect::new(0, 0, 256, 7),
            [0, 0],
            S {
                color: component(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add),
                write_mask: [true, true, true, false],
                ..Default::default()
            },
            &|| Ok(()),
        )
        .unwrap();
    for (row, color) in [0u8, 1, 2, 127, 128, 254, 255].into_iter().enumerate() {
        for alpha in 0..256u32 {
            let at = (row * 256 + alpha as usize) * 4;
            for (c, (s, d)) in [(color, 255 - color), (255 - color, color), (93, 177)]
                .into_iter()
                .enumerate()
            {
                let expected = (u32::from(s) * alpha + u32::from(d) * (255 - alpha) + 127) / 255;
                assert_eq!(
                    u32::from(target.rgba[at + c]),
                    expected,
                    "row{row}/a{alpha}/c{c}"
                );
            }
            assert_eq!(target.rgba[at + 3], 37);
        }
    }
}
#[test]
fn equations_clamp_separately_minmax_ignore_factors_and_mask_preserves_raw_bytes() {
    assert_eq!(
        pixel(
            [100, 200, 50, 70],
            [180, 100, 90, 40],
            S {
                color: component(F::Zero, F::Zero, E::Min),
                alpha: component(F::Zero, F::Zero, E::Max),
                ..Default::default()
            }
        ),
        [100, 100, 50, 70]
    );
    assert_eq!(
        pixel(
            [100, 200, 50, 70],
            [180, 100, 90, 40],
            S {
                color: component(F::One, F::One, E::ReverseSubtract),
                alpha: component(F::One, F::One, E::Subtract),
                ..Default::default()
            }
        ),
        [80, 0, 40, 30]
    );
    assert_eq!(
        pixel(
            [100, 200, 50, 70],
            [180, 100, 90, 40],
            S {
                color: component(F::One, F::One, E::Add),
                write_mask: [true, false, true, false],
                ..Default::default()
            }
        ),
        [255, 100, 140, 40]
    );
}
#[test]
fn fragment_keys_ranges_and_float_transform_have_no_intermediate_quantization() {
    let original = [60, 70, 80, 90];
    assert_eq!(
        pixel(
            [64, 128, 192, 128],
            original,
            S {
                source_color_key: Some([64, 128, 192]),
                ..Default::default()
            }
        ),
        original
    );
    assert_eq!(
        pixel(
            [64, 128, 192, 128],
            original,
            S {
                source_alpha_key: Some(128),
                ..Default::default()
            }
        ),
        original
    );
    assert_eq!(
        pixel(
            [64, 128, 192, 128],
            original,
            S {
                source_alpha_range: [0., 127. / 255.],
                ..Default::default()
            }
        ),
        original
    );
    assert_eq!(
        pixel(
            [64, 128, 192, 128],
            original,
            S {
                source_alpha_range: [128. / 255., 128. / 255.],
                ..Default::default()
            }
        ),
        [64, 128, 192, 128]
    );
    // One source byte scaled by1/2 is not rounded to1 before alpha blending:
    // ideal result is(0.5 * 128/255), which rounds to0, not1.
    assert_eq!(
        pixel(
            [1, 1, 1, 128],
            [0; 4],
            S {
                source_scale: [0.5, 0.5, 0.5, 1.],
                color: component(F::SourceAlpha, F::Zero, E::Add),
                ..Default::default()
            }
        ),
        [0, 0, 0, 128]
    );
    assert_eq!(
        pixel(
            [10, 30, 70, 90],
            original,
            S {
                source_scale: [-1., -1., -1., 1.],
                source_bias: [1., 1., 1., 0.],
                ..Default::default()
            }
        ),
        [245, 225, 185, 90]
    );
}
#[test]
fn clipping_cancellation_and_invalid_descriptors_use_the_common_checked_visitor() {
    let source = ImageSurface {
        width: 3,
        height: 2,
        rgba: (1..=6).flat_map(|v| [v, v, v, 255]).collect(),
    };
    let mut target = ImageSurface::filled(3, 2, [0; 4]).unwrap();
    target
        .blend_fixed_factors(
            &source,
            BitmapRect::new(-1, 0, 4, 2),
            [-1, 0],
            S::default(),
            &|| Ok(()),
        )
        .unwrap();
    assert_eq!(target, source);
    let before = target.clone();
    for state in [
        S {
            constant: [f32::NAN; 4],
            ..Default::default()
        },
        S {
            source_scale: [65.; 4],
            ..Default::default()
        },
        S {
            source_alpha_range: [1., 0.],
            ..Default::default()
        },
    ] {
        assert!(target
            .blend_fixed_factors(&source, BitmapRect::new(0, 0, 3, 2), [0, 0], state, &|| Ok(
                ()
            ))
            .is_err());
        assert_eq!(target, before);
    }
    assert!(target
        .blend_fixed_factors(
            &source,
            BitmapRect::new(0, 0, 3, 2),
            [0, 0],
            S::default(),
            &|| anyhow::bail!("cancelled before write")
        )
        .is_err());
    assert_eq!(target, before);
    let calls = Cell::new(0);
    assert!(target
        .blend_fixed_factors(
            &source,
            BitmapRect::new(0, 0, 3, 2),
            [0, 0],
            S::default(),
            &|| {
                calls.set(calls.get() + 1);
                if calls.get() == 3 {
                    anyhow::bail!("cancelled at second row");
                }
                Ok(())
            }
        )
        .is_err());
    assert_eq!(calls.get(), 3);
    let malformed = ImageSurface {
        width: 1,
        height: 1,
        rgba: vec![],
    };
    assert!(target
        .blend_fixed_factors(
            &malformed,
            BitmapRect::new(0, 0, 1, 1),
            [0, 0],
            S::default(),
            &|| Ok(())
        )
        .is_err());
}

#[test]
fn integer_product_profiles_match_complete_multiply_screen_and_separate_lerp_tables() {
    use crysirror_core::image_blend::{RgbaBlendArithmetic as A, RgbaByteRounding as Q};
    let mut source = ImageSurface::filled(256, 256, [0; 4]).unwrap();
    let mut destination = source.clone();
    for s in 0..256usize {
        for d in 0..256usize {
            let at = (s * 256 + d) * 4;
            source.rgba[at..at + 4].copy_from_slice(&[s as u8; 4]);
            destination.rgba[at..at + 4].copy_from_slice(&[d as u8; 4]);
        }
    }
    let original = destination.clone();
    let mut state = S {
        arithmetic: A::Byte255 {
            source: Q::Floor,
            destination: Q::Floor,
        },
        color: component(F::DestinationColor, F::Zero, E::Add),
        write_mask: [true, true, true, false],
        ..Default::default()
    };
    destination
        .blend_fixed_factors(
            &source,
            BitmapRect::new(0, 0, 256, 256),
            [0, 0],
            state,
            &|| Ok(()),
        )
        .unwrap();
    for s in 0..256usize {
        for d in 0..256usize {
            let at = (s * 256 + d) * 4;
            assert_eq!(&destination.rgba[at..at + 3], &[(s * d / 255) as u8; 3]);
            assert_eq!(destination.rgba[at + 3], d as u8);
        }
    }
    state.arithmetic = A::Byte255 {
        source: Q::Floor,
        destination: Q::Ceil,
    };
    state.color = component(F::One, F::OneMinusSourceColor, E::Add);
    let mut destination = original;
    destination
        .blend_fixed_factors(
            &source,
            BitmapRect::new(0, 0, 256, 256),
            [0, 0],
            state,
            &|| Ok(()),
        )
        .unwrap();
    for s in 0..256usize {
        for d in 0..256usize {
            let at = (s * 256 + d) * 4;
            assert_eq!(
                &destination.rgba[at..at + 3],
                &[(255 - (255 - s) * (255 - d) / 255) as u8; 3]
            );
        }
    }
    let state = S {
        arithmetic: A::Byte255 {
            source: Q::Floor,
            destination: Q::Floor,
        },
        source_scale: [64. / 255., 64. / 255., 64. / 255., 1.],
        constant: [0., 0., 0., 128. / 255.],
        color: component(F::ConstantAlpha, F::OneMinusConstantAlpha, E::Add),
        write_mask: [true, true, true, false],
        ..Default::default()
    };
    assert_eq!(
        pixel([111, 122, 133, 202], [10, 20, 30, 61], state),
        [17, 24, 30, 61]
    );
    for alpha in 0..=255u32 {
        let state = S {
            arithmetic: A::Byte255 {
                source: Q::Floor,
                destination: Q::Floor,
            },
            color: component(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add),
            write_mask: [true, true, true, false],
            ..Default::default()
        };
        let result = pixel([111, 122, 133, alpha as u8], [10, 20, 30, 61], state);
        let expected = [
            (111 * alpha / 255 + 10 * (255 - alpha) / 255) as u8,
            (122 * alpha / 255 + 20 * (255 - alpha) / 255) as u8,
            (133 * alpha / 255 + 30 * (255 - alpha) / 255) as u8,
            61,
        ];
        assert_eq!(result, expected);
    }
}
