// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! WATER24's source32 shared-class Print clock and reflected-row geometry.
//! Mask-free screen row copies execute in the common image scanline worker.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedSampleSpan, FixedScanlines, ScanlineMath},
};

pub(crate) fn parameters(values: &[i32], height: u32) -> [i32; 4] {
    let mut p = std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => height as i32 / 2,
            2 => 100,
            _ => 0,
        })
    });
    p[1] = p[1].clamp(0, height.saturating_sub(1) as i32);
    p
}
#[derive(Clone, Debug, Default)]
pub(crate) struct WaterClass {
    pub print_count: i32,
}
impl WaterClass {
    pub(crate) fn image_input(
        &mut self,
        p: [i32; 4],
        canvas: [u32; 2],
        base: ImageInput,
    ) -> Result<ImageInput> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN WATER canvas extent"
        );
        ensure!((0..canvas[1] as i32).contains(&p[1]), "NNN WATER start row");
        // This member belongs to CEffectWater, not to one EFFECT's parameter
        // storage. Multiple slots increment it in ascending painter order;
        // SetParam and raw-save InitParameter do not reset this class clock.
        self.print_count = self.print_count.wrapping_add(1);
        if self.print_count > 1_000_000 {
            self.print_count = 0;
        }
        let dv = canvas[1] as i32 - p[1];
        let mut spans = Vec::with_capacity(dv as usize);
        for k in 0..dv {
            let distance = dv - k;
            // Parentheses in the native double initialization evaluate these
            // products/divisions as int BEFORE conversion to double.
            let phase = self
                .print_count
                .wrapping_neg()
                .wrapping_mul(3)
                .wrapping_add(distance.wrapping_mul(distance) / dv * 4);
            let theta = f64::from(phase) * (3.14159 * 2.0) / 100.0;
            let displacement =
                (theta.cos() * 8.0 * f64::from(k + dv) / f64::from(dv * 2) + 0.5) as i32;
            let source = displacement
                .wrapping_add(p[1].wrapping_sub(k).wrapping_sub(10))
                .min(p[1] - 1)
                .max(0);
            spans.push(FixedSampleSpan {
                destination: [0, p[1] + k, canvas[0] as i32],
                source_q16: [0, i64::from(source) * 65536],
                source_step_q16: [65536, 0],
                math: ScanlineMath::Stored {
                    math: ByteRasterMath::CopyRgb,
                },
            });
        }
        let spec = FixedScanlines {
            spans,
            corner_rgb_key: false,
            source_color: None,
        };
        spec.validate()?;
        // Every source row is above the written water section (or row0 when
        // yStart0). An immutable source preserves the original in-place order;
        // earlier destination writes cannot affect a later source sample.
        Ok(ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base.clone()),
                source: Box::new(base),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::FixedScanlines { spec },
            }])?
            .into(),
        })
    }
}
// Source countMax99999; CountIsMax returnsTRUE without changing flag/command.
// CCommonEffect::CheckNeedPicPrint is inheritedTRUE. InitParameter copies the
//4 saved words into SetParam: clamp Y, count0/max99999, retaining para0 dummy.
