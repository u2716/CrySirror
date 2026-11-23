// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Mono carrier generators ([`Oscillator`]) and the channel vocoder.
//!
//! The generators follow the STK classes (SineWave, Blit, BlitSaw,
//! BlitSquare, Modulate; notice in `stk.LICENSE`) at the stream rate, and a
//! xoshiro128+ noise source. The vocoder is a port of Philip Bennefall's
//! voclib single-channel path (notice in `voclib.LICENSE`): per band a
//! cascade of identical band-pass biquads on modulator and carrier, a
//! four-pole envelope follower on the modulator band, summed products.
use anyhow::Result;
use crysirror_core::service::Oscillator;
use crysirror_tasks::Cancellation;
use std::f64::consts::PI;

const TABLE: f64 = 2048.;
fn table_sine(phase: f64) -> f64 {
    let i = phase as u32 as f64;
    let a = (2. * PI * i / TABLE).sin();
    let b = (2. * PI * (i + 1.) / TABLE).sin();
    a + (b - a) * (phase - i)
}
fn wrap_table(mut phase: f64) -> f64 {
    while phase < 0. {
        phase += TABLE;
    }
    while phase >= TABLE {
        phase -= TABLE;
    }
    phase
}
/// MSVC `rand()` as STK Noise uses it in Modulate.
struct MsRand(u32);
impl MsRand {
    fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        ((self.0 >> 16) & 0x7fff) as i32
    }
}

/// `count` carrier samples at `rate`.
pub fn generate(osc: &Oscillator, rate: u32, count: usize, cancel: &Cancellation) -> Result<Vec<f32>> {
    cancel.check()?;
    let sr = rate as f64;
    let mut out = vec![0f32; count];
    match osc {
        Oscillator::Const { value } => out.fill(*value),
        Oscillator::Sine { gain, freq } => {
            let step = *freq as f64 * TABLE / sr;
            let mut phase = 0f64;
            for o in &mut out {
                phase = wrap_table(phase);
                *o = table_sine(phase) as f32 * gain;
                phase += step;
            }
        }
        Oscillator::Noise { gain, seed } => {
            let seed = seed.unwrap_or(0x9E37_79B9_7F4A_7C15);
            let (hi, lo) = ((seed >> 32) as u32, seed as u32);
            let mut s = [hi, hi.reverse_bits(), lo, lo.reverse_bits()];
            for o in &mut out {
                let v = ((s[0].wrapping_add(s[3])) >> 8) as f32 * 5.960_464_5e-8;
                let t = s[1] << 9;
                s[2] ^= s[0];
                s[3] ^= s[1];
                s[1] ^= s[2];
                s[0] ^= s[3];
                s[2] ^= t;
                s[3] = s[3].rotate_left(11);
                *o = ((v + v) - 1.) * gain;
            }
        }
        Oscillator::Blit { gain, freq, harmonics } => {
            let p = sr / *freq as f64;
            let step = PI / p;
            let m = (2 * if *harmonics == 0 { (0.5 * p).floor() as u32 } else { *harmonics } + 1) as f64;
            let mut phase = 0f64;
            for o in &mut out {
                let d = phase.sin();
                let v = if d.abs() <= f64::EPSILON { 1. } else { (m * phase).sin() / (m * d) };
                phase += step;
                if phase >= PI {
                    phase -= PI;
                }
                *o = v as f32 * gain;
            }
        }
        Oscillator::BlitSaw { gain, freq, harmonics } => {
            let p = sr / *freq as f64;
            let c2 = 1. / p;
            let step = PI * c2;
            let m = (2 * if *harmonics == 0 { (0.5 * p).floor() as u32 } else { *harmonics } + 1) as f64;
            let a = m / p;
            let (mut phase, mut state) = (0f64, 0f64);
            for o in &mut out {
                let d = phase.sin();
                let mut v = if d.abs() <= f64::EPSILON { a } else { (m * phase).sin() / (p * d) };
                v += state - c2;
                state = v * 0.995;
                phase += step;
                if phase >= PI {
                    phase -= PI;
                }
                *o = v as f32 * gain;
            }
        }
        Oscillator::BlitSquare { gain, freq, harmonics } => {
            let p = 0.5 * sr / *freq as f64;
            let step = PI / p;
            let m = (2 * (if *harmonics == 0 { (0.5 * p).floor() as u32 } else { *harmonics } + 1)) as f64;
            let a = m / p;
            let (mut phase, mut blit, mut dcb, mut last) = (0f64, 0f64, 0f64, 0f64);
            for o in &mut out {
                let temp = blit;
                let d = phase.sin();
                blit = if d.abs() < f64::EPSILON {
                    if phase < 0.1 || phase > 2. * PI - 0.1 { a } else { -a }
                } else {
                    (m * phase).sin() / (p * d)
                };
                blit += temp;
                last = blit - dcb + 0.999 * last;
                dcb = blit;
                phase += step;
                if phase >= 2. * PI {
                    phase -= 2. * PI;
                }
                *o = last as f32 * gain;
            }
        }
        Oscillator::Modulate { gain, vibrato_rate, vibrato_gain, random_gain } => {
            let step = *vibrato_rate as f64 * TABLE / sr;
            let noise_rate = (sr * 330. / 22050.) as u32;
            let (mut phase, mut counter, mut noise, mut y1) = (0f64, noise_rate, 0f64, 0f64);
            let mut rng = MsRand(1);
            for o in &mut out {
                phase = wrap_table(phase);
                let mut v = table_sine(phase) * vibrato_gain;
                phase += step;
                let c = counter;
                counter += 1;
                if c >= noise_rate {
                    let r = rng.next() as f64;
                    noise = (r + r) * 0.000030517578125 - 1.;
                    counter = 0;
                }
                // OnePole(pole 0.999) with gain = random gain.
                y1 = noise * random_gain * (1. - 0.999) + 0.999 * y1;
                v += y1;
                *o = v as f32 * gain;
            }
        }
        Oscillator::Mix { sources } => {
            for s in sources {
                for (o, v) in out.iter_mut().zip(generate(s, rate, count, cancel)?) {
                    *o += v;
                }
            }
        }
        Oscillator::Filter { source, filters } => {
            let dry = generate(source, rate, count, cancel)?;
            let (ch, mut wet) = crate::wave_filters::apply(filters, rate, 1, dry, cancel)?;
            // A stage that widens to stereo keeps its left channel.
            if ch == 2 {
                wet = wet.chunks_exact(2).map(|f| f[0]).collect();
            }
            wet.resize(count, 0.);
            out = wet;
        }
        Oscillator::Loop { samples, pitch } => {
            let n = samples.len() as f64;
            let mut at = 0f64;
            for o in &mut out {
                *o = samples[at as usize % samples.len()];
                at += pitch;
                if at >= n {
                    at -= n;
                }
            }
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    c: [f32; 5],
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}
impl Biquad {
    fn band_pass(freq: f32, srate: f32, bandwidth: f32) -> [f32; 5] {
        let omega = (2. * PI * freq as f64 / srate as f64) as f32;
        let (sn, cs) = ((omega as f64).sin() as f32, (omega as f64).cos() as f32);
        let alpha = sn * ((std::f64::consts::LN_2 / 2. * bandwidth as f64 * omega as f64 / sn as f64).sinh() as f32);
        let a0 = 1. + alpha;
        [alpha / a0, 0. / a0, -alpha / a0, -2. * cs / a0, (1. - alpha) / a0]
    }
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.c[0] * x + self.c[1] * self.x1 + self.c[2] * self.x2 - self.c[3] * self.y1 - self.c[4] * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}
/// voclib with one carrier channel.
pub struct Vocoder {
    analysis: Vec<Vec<Biquad>>,
    synthesis: Vec<Vec<Biquad>>,
    envelopes: Vec<[f32; 4]>,
    coef: f32,
}
impl Vocoder {
    pub fn new(bands: u8, filters_per_band: u8, rate: u32, formant_shift: f32, reaction_time: f32) -> Self {
        let (minfreq, maxfreq) = (80f64, (rate as f64).min(12000.));
        let step = (maxfreq / minfreq).powf(1. / bands as f64);
        let mut lastfreq = 0f64;
        let (mut analysis, mut synthesis) = (vec![], vec![]);
        for _ in 0..bands {
            let prior = lastfreq;
            lastfreq = if lastfreq > 0. { lastfreq * step } else { minfreq };
            let bandwidth = ((lastfreq * step - prior) / lastfreq) as f32;
            let a = Biquad::band_pass(lastfreq as f32, rate as f32, bandwidth);
            let s = if formant_shift != 1. {
                Biquad::band_pass((lastfreq * formant_shift as f64) as f32, rate as f32, bandwidth)
            } else {
                a
            };
            analysis.push(vec![Biquad { c: a, ..Default::default() }; filters_per_band as usize]);
            synthesis.push(vec![Biquad { c: s, ..Default::default() }; filters_per_band as usize]);
        }
        let coef = 0.01f64.powf(1. / (reaction_time as f64 * rate as f64)) as f32;
        Self { analysis, synthesis, envelopes: vec![[0.; 4]; bands as usize], coef }
    }
    pub fn tick(&mut self, carrier: f32, modulator: f32) -> f32 {
        let mut out = 0f32;
        let coef = self.coef;
        for ((a, s), h) in self.analysis.iter_mut().zip(&mut self.synthesis).zip(&mut self.envelopes) {
            let (mut ab, mut sb) = (modulator, carrier);
            for (fa, fs) in a.iter_mut().zip(s.iter_mut()) {
                ab = fa.tick(ab);
                sb = fs.tick(sb);
            }
            h[0] = (1. - coef) * ab.abs() + coef * h[0];
            h[1] = (1. - coef) * h[0] + coef * h[1];
            h[2] = (1. - coef) * h[1] + coef * h[2];
            h[3] = (1. - coef) * h[2] + coef * h[3];
            out += sb * h[3];
        }
        out
    }
}
