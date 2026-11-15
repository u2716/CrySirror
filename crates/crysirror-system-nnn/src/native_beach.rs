// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BEACH83's class-wide phase, two ordered spans per row and packed colour
//! coefficients. Full548-line class/header read; scalar wave/rectangle policy
//! stays here, reusable byte-word affine raster belongs to shared image jobs.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;
const WAVES: [[(f64, f64, i32); 3]; 2] = [
    [(30.0, 33.2, 8130), (21.0, -18.0, 121), (10.0, 16.7, 71)],
    [(30.0, 22.6, 105), (23.0, -17.1, 40), (10.0, 35.7, 8102)],
];
#[derive(Clone)]
pub(crate) struct BeachClass {
    phase: [[i32; 3]; 2],
    movement: [i32; 2],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 15] {
    let defaults = [
        20,
        20,
        128,
        192,
        255,
        0,
        canvas[1] as i32,
        30,
        43,
        300,
        250,
        100,
        100,
        700,
        100,
    ];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[6] = p[6].min(canvas[1] as i32);
    p[7] = p[7].max(1);
    p[8] = p[8].max(1);
    p[14] = p[14].max(1);
    p
}
pub(crate) fn validate(p: &[i32; 15]) -> Result<()> {
    ensure!(
        p[7] > 0 && p[8] > 0 && p[14] > 0 && p[6] <= 16384,
        "NNN beach scalar/row domain"
    );
    Ok(())
}
#[derive(Clone, Copy)]
pub(crate) struct Coefficients {
    pub multiplier: u16,
    pub add_rgb: [u16; 3],
}
#[derive(Clone, Copy)]
pub(crate) struct Span {
    pub destination: [i32; 3],
    pub profile: usize,
}
pub(crate) struct Paint {
    pub spans: Vec<Span>,
    pub coefficients: [Coefficients; 2],
}
impl BeachClass {
    pub(crate) fn image_input(
        &self,
        p: &[i32; 15],
        canvas: [u32; 2],
        base: crysirror_core::image_processing::ImageInput,
    ) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
        use crysirror_image::{
            effects::ImageEffect,
            stored_word_spans::{StoredWordMath, StoredWordSpan, StoredWordSpans},
        };
        let paint = self.plan(p, canvas)?;
        if paint.spans.is_empty() {
            return Ok(None);
        }
        let spec = StoredWordSpans {
            spans: paint
                .spans
                .into_iter()
                .map(|span| {
                    let coefficients = paint.coefficients[span.profile];
                    StoredWordSpan {
                        destination: span.destination,
                        math: StoredWordMath::WrappingAffine {
                            multiplier: [coefficients.multiplier; 3],
                            addend: coefficients.add_rgb,
                            shift: 7,
                        },
                    }
                })
                .collect(),
        };
        spec.validate()?;
        Ok(Some(
            crysirror_core::image_processing::ImageInput::Process {
                input: Box::new(base),
                effects: serde_json::to_vec(&[ImageEffect::StoredWordSpans { spec }])?.into(),
            },
        ))
    }
    pub(crate) fn initialize(rng: &mut PatternRng) -> Result<Self> {
        let mut phase = [[0; 3]; 2];
        for row in &mut phase {
            for cell in row {
                *cell = (rng.next_draw()? & 0x1fff) as i32;
            }
        }
        let mut movement = [0; 2];
        for v in &mut movement {
            *v = (rng.next_draw()? & 0x7ff) as i32;
        }
        Ok(Self { phase, movement })
    }
    pub(crate) fn calcu(&mut self, p: &[i32; 15]) {
        for j in 0..2 {
            for i in 0..3 {
                self.phase[j][i] = (self.phase[j][i] + WAVES[j][i].2) & 0x1fff;
            }
        }
        for i in 0..2 {
            self.movement[i] = self.movement[i].wrapping_add(p[7 + i]) & 0x7ff;
        }
    }
    pub(crate) fn plan(&self, p: &[i32; 15], canvas: [u32; 2]) -> Result<Paint> {
        validate(p)?;
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN beach canvas"
        );
        let start = p[5].clamp(0, canvas[1] as i32 - 1);
        let rows = p[6].min(canvas[1] as i32 - start).max(0);
        let phases: [[f64; 3]; 2] = std::array::from_fn(|j| {
            std::array::from_fn(|i| {
                let mut value = f64::from(self.phase[j][i]);
                value /= 8192.0;
                value *= 3.1415926 * 2.0;
                value
            })
        });
        let scales = [f64::from(p[11]) * 0.01, f64::from(p[12]) * 0.01];
        let moves = std::array::from_fn::<_, 2, _>(|i| {
            let mut th = f64::from(self.movement[i]);
            th /= 2048.0;
            th *= 3.14159 * 2.0;
            (1.0 + th.cos()) * f64::from(p[9 + i]) * 0.5
        });
        let mut spans = Vec::with_capacity(rows as usize * 2);
        for k in 0..rows {
            // Perspective uses ORIGINAL para5+k, not the clamped destination
            // start. Its wave phase, conversely, always starts at local k0.
            let ml = f64::from(p[5].wrapping_add(k)) * (1.0 / f64::from(canvas[1]));
            let mut perspective = f64::from(p[14]);
            perspective *= 0.01;
            perspective += (1.0 - perspective) * ml;
            let mut right = f64::from(p[13].wrapping_sub(canvas[0] as i32 / 2));
            right *= perspective;
            right += f64::from(canvas[0] as i32 / 2);
            let wave = std::array::from_fn::<_, 2, _>(|j| {
                let mut sum = 0.0;
                for i in 0..3 {
                    let mut th = f64::from(k);
                    th *= 1.0 / f64::from(canvas[1]);
                    th *= WAVES[j][i].1;
                    sum += WAVES[j][i].0 * (th + phases[j][i]).cos();
                }
                sum * scales[j]
            });
            let mut a = ((right - moves[0] * perspective - wave[0]) as i32).max(0);
            let mut b = ((right - moves[1] * perspective - wave[1]) as i32).max(0);
            if a >= b {
                std::mem::swap(&mut a, &mut b);
            }
            let end = (right as i32).clamp(1, canvas[0] as i32);
            for (profile, x, length) in [(0, a, b.wrapping_sub(a)), (1, b, end.wrapping_sub(b))] {
                let n = length.min(canvas[0] as i32 - x).max(0);
                if n > 0 && x < canvas[0] as i32 {
                    spans.push(Span {
                        destination: [x, start + k, n],
                        profile,
                    });
                }
            }
        }
        let p1 = p[1];
        let p2 = p1.wrapping_mul(2).wrapping_sub(p1.wrapping_mul(p1) / 100);
        let coefficient = |percent: i32| {
            let color =
                [p[2], p[3], p[4]].map(|v| (v.wrapping_mul(percent) / 100).wrapping_shl(7) as u32);
            // MMX constructs the RGB offset using three movd+word shifts and
            // paddw. Out-of-byte parameters can spill a high16 word into the
            // adjacent channel; preserve that parameter packing, not a clamp.
            let [r, g, b] = color;
            Coefficients {
                multiplier: (100i32.wrapping_sub(percent).wrapping_mul(128) / 100) as u16,
                add_rgb: [
                    (r as u16).wrapping_add((g >> 16) as u16),
                    (g as u16).wrapping_add((b >> 16) as u16),
                    b as u16,
                ],
            }
        };
        Ok(Paint {
            spans,
            coefficients: [coefficient(p1), coefficient(p2)],
        })
    }
}
// Shared raster formula is min255(((destinationRGB*multiplier + addRGB)
// mod65536)>>7), with alpha retained. Both profiles are sequential disjoint
// spans per row; no source bitmap, CharaClip or mask participates.
// Constructor draws6 phases then2 motions. Fresh SetParam stores15 words but
// does NOT reset the class phase. Calcu executes per active BEACH slot before
// all Prints observe the final class state. Raw InitParameter is inherited
// empty; CountIsMaxTRUE, NeedPicPrintTRUE, eternal/picture/only-one metadata1.
// GetOnlyOneType has no caller in the native source: do not invent a runtime
// rule which silently removes other BEACH effects.
