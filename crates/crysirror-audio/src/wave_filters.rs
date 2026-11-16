// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Floating-point WaveFilter chain ([`WaveFilter`]) over decoded PCM.
//!
//! Each stage reproduces a KiriKiri wave filter plug-in as the shipped
//! binaries compute it (recovered with IDA from `wfBasicEffect.dll` /
//! `wfTypicalDSP.dll`; addresses in crysirror-krkr `wf_plugins`):
//!
//! * Delay: one line per channel of `floor(time·rate/1000)` samples;
//!   `d = line[i]`, `line[i] = (x + d)·feedback`, `y = x + d·mix`.
//! * FreeVerb: STK FreeVerb (notice in `stk.LICENSE`). The plug-in never sets
//!   `Stk::sampleRate`, so the comb/allpass lengths stay the 44.1 kHz tunings
//!   at every stream rate. Mono input is duplicated; output is stereo.
//! * GraphicEq: ten parallel sections at 31.5 Hz..16 kHz built from first-
//!   order (outer bands) and second-order (inner bands) allpasses, each
//!   adding `x + (G-1)/2·(…)`. The ten outputs are summed, so unity gains
//!   give ten times the input, as the plug-in does.
//! * Iir: a DSPFilters cascade ([`crate::iir_design`]).
//!
//! Stages with `extend_ms` keep running on silence for that long after their
//! input ends; the tail feeds the next stage.
use crate::iir_design;
use anyhow::{Context, Result, ensure};
use crysirror_core::service::WaveFilter;
use crysirror_tasks::Cancellation;

/// Total output samples a chain may produce (interleaved).
const MAX_SAMPLES: usize = 64 << 20;

fn tail_frames(extend_ms: u32, rate: u32) -> usize {
    (extend_ms as u64 * rate as u64 / 1000) as usize
}

fn delay(time_ms: u32, feedback: f32, mix: f32, extend_ms: u32, rate: u32, ch: usize, input: &[f32], cancel: &Cancellation) -> Result<Vec<f32>> {
    let len = (time_ms as u64 * rate as u64 / 1000) as usize;
    ensure!(len > 0, "Delay time {time_ms} ms is shorter than one sample at {rate} Hz");
    let frames = input.len() / ch + tail_frames(extend_ms, rate);
    let mut lines = vec![vec![0f32; len]; ch];
    let mut out = Vec::with_capacity(frames * ch);
    let mut at = 0;
    for n in 0..frames {
        if n % 65536 == 0 {
            cancel.check()?;
        }
        for (c, line) in lines.iter_mut().enumerate() {
            let x = input.get(n * ch + c).copied().unwrap_or(0.);
            let d = line[at];
            line[at] = (x + d) * feedback;
            out.push(x + d * mix);
        }
        at = if at + 1 == len { 0 } else { at + 1 };
    }
    Ok(out)
}

/// STK FreeVerb with its fixed 44.1 kHz tunings.
struct FreeVerb {
    comb: [[Vec<f64>; 2]; 8],
    comb_at: [[usize; 2]; 8],
    lp: [[f64; 2]; 8],
    all: [[Vec<f64>; 2]; 4],
    all_at: [[usize; 2]; 4],
    gain: f64,
    room: f64,
    damp_b0: f64,
    damp_a1: f64,
    dry: f64,
    wet1: f64,
    wet2: f64,
}
impl FreeVerb {
    const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
    const SPREAD: usize = 23;
    fn new(effect_mix: f64, room_size: f64, damping: f64, width: f64, frozen: bool) -> Self {
        let line = |n: usize| [vec![0.; n], vec![0.; n + Self::SPREAD]];
        // Effect::setEffectMix clamps to [0, 1].
        let mix = effect_mix.clamp(0., 1.);
        let mut wet = 3. * mix;
        let mut dry = 2. * (1. - mix);
        wet /= wet + dry;
        dry /= wet + dry;
        let (room, damp, gain) = if frozen { (1., 0., 0.) } else { (room_size * 0.28 + 0.7, damping * 0.4, 0.015) };
        // OnePole::setCoefficients(1 - damp, -damp) refuses |a1| >= 1 and
        // keeps the constructor's (1, 0) coefficients.
        let (damp_b0, damp_a1) = if damp.abs() >= 1. { (1., 0.) } else { (1. - damp, -damp) };
        Self {
            comb: Self::COMBS.map(line),
            comb_at: [[0; 2]; 8],
            lp: [[0.; 2]; 8],
            all: Self::ALLPASSES.map(line),
            all_at: [[0; 2]; 4],
            gain,
            room,
            damp_b0,
            damp_a1,
            dry,
            wet1: wet * (width / 2. + 0.5),
            wet2: wet * (1. - width) / 2.,
        }
    }
    fn tick(&mut self, l: f64, r: f64) -> (f64, f64) {
        let input = (l + r) * self.gain;
        let mut out = [0f64; 2];
        for i in 0..8 {
            for s in 0..2 {
                let line = &mut self.comb[i][s];
                let at = self.comb_at[i][s];
                let lp = self.damp_b0 * line[at] - self.damp_a1 * self.lp[i][s];
                self.lp[i][s] = lp;
                let yn = input + self.room * lp;
                line[at] = yn;
                self.comb_at[i][s] = if at + 1 == line.len() { 0 } else { at + 1 };
                out[s] += yn;
            }
        }
        for i in 0..4 {
            for s in 0..2 {
                let line = &mut self.all[i][s];
                let at = self.all_at[i][s];
                let vn_m = line[at];
                let vn = out[s] + 0.5 * vn_m;
                line[at] = vn;
                self.all_at[i][s] = if at + 1 == line.len() { 0 } else { at + 1 };
                out[s] = -vn + 1.5 * vn_m;
            }
        }
        (
            out[0] * self.wet1 + out[1] * self.wet2 + l * self.dry,
            out[1] * self.wet1 + out[0] * self.wet2 + r * self.dry,
        )
    }
}

/// GraphicEqualizer coefficients: per band `(k, c1, g)` as single floats.
fn eq_design(gains: &[f32; 10], rate: u32) -> [[f32; 3]; 10] {
    const BANDS: [f32; 10] = [31.5, 63., 125., 250., 500., 1000., 2000., 4000., 8000., 16000.];
    let rate = rate as f32 as f64;
    let mut out = [[0f32; 3]; 10];
    for i in 0..10 {
        let gain = gains[i] as f64;
        let freq = BANDS[i] as f64;
        let inner = (1..=8).contains(&i);
        let boost = gain > 1.;
        let f = if inner { (freq + freq) / 3. } else { freq };
        let mut t = ((f * std::f64::consts::PI / rate).tan() as f32) as f64;
        let mut c1 = 0f64;
        let k = if inner {
            let k = if boost { (t - 1.) / (t + 1.) } else { (t - gain) / (t + gain) };
            c1 = -((1. - k) * (freq * std::f64::consts::PI / rate).cos());
            k
        } else if i == 0 {
            if boost { (t - 1.) / (t + 1.) } else { (t - gain) / (t + gain) }
        } else {
            if !boost {
                t *= gain;
            }
            (t - 1.) / (t + 1.)
        };
        out[i] = [k as f32, c1 as f32, ((gain - 1.) * 0.5) as f32];
    }
    out
}
fn eq_tick(c: &[[f32; 3]; 10], s: &mut [f32; 41], x: f32) -> f32 {
    // Band 0: first-order section, state s[0].
    let v = x - c[0][0] * s[0];
    let mut acc = (c[0][0] * v + s[0] + x) * c[0][2] + x;
    s[0] = v;
    for b in 1..9 {
        let [k, c1, g] = c[b];
        let st = &mut s[1 + (b - 1) * 4..1 + b * 4];
        let y = -k * x + st[0] * c1 + st[1] - st[2] * c1 + k * st[3];
        let band = (x - y) * g + x;
        st[1] = st[0];
        st[0] = x;
        st[3] = st[2];
        st[2] = y;
        acc = band + acc;
    }
    let v = x - s[40] * c[9][0];
    let band = x + (c[9][0] * v + s[40] + x) * c[9][2];
    s[40] = v;
    band + acc
}

fn stage(f: &WaveFilter, rate: u32, ch: usize, input: &[f32], cancel: &Cancellation) -> Result<(usize, Vec<f32>)> {
    Ok(match f {
        WaveFilter::Delay { time_ms, feedback, mix, extend_ms } => {
            (ch, delay(*time_ms, *feedback, *mix, *extend_ms, rate, ch, input, cancel)?)
        }
        WaveFilter::FreeVerb { effect_mix, room_size, damping, width, frozen, extend_ms } => {
            let mut verb = FreeVerb::new(*effect_mix, *room_size, *damping, *width, *frozen);
            let frames = input.len() / ch + tail_frames(*extend_ms, rate);
            let mut out = Vec::with_capacity(frames * 2);
            for n in 0..frames {
                if n % 65536 == 0 {
                    cancel.check()?;
                }
                let l = input.get(n * ch).copied().unwrap_or(0.) as f64;
                let r = if ch == 2 { input.get(n * ch + 1).copied().unwrap_or(0.) as f64 } else { l };
                let (a, b) = verb.tick(l, r);
                out.push(a as f32);
                out.push(b as f32);
            }
            (2, out)
        }
        WaveFilter::GraphicEq { gains } => {
            let c = eq_design(gains, rate);
            let mut state = vec![[0f32; 41]; ch];
            let mut out = Vec::with_capacity(input.len());
            for (n, x) in input.iter().enumerate() {
                if n % 65536 == 0 {
                    cancel.check()?;
                }
                out.push(eq_tick(&c, &mut state[n % ch], *x));
            }
            (ch, out)
        }
        WaveFilter::GainLimit { gain, threshold, mode } => {
            let (gain, th) = (*gain, threshold.abs());
            // A sigmoid mode at threshold >= 1 degrades to the hard clip.
            let mode = if th >= 1. && *mode != 0 { -1 } else { *mode };
            let f: Option<fn(f32) -> f32> = match mode {
                1 => Some(|x| x / (x.abs() + 1.)),
                2 => Some(|x| ((x as f64 * 1.5707964).atan() / 1.5707964) as f32),
                3 => Some(|x| (x as f64 / (1. + x as f64 * x as f64).sqrt()) as f32),
                4 => Some(|x| (x as f64).tanh() as f32),
                _ => None,
            };
            let pure = th < 1.1920929e-7;
            let out = input
                .iter()
                .map(|x| {
                    let v = x * gain;
                    match f {
                        Some(f) if pure => f(v),
                        Some(f) => {
                            let over = v.abs() - th;
                            if over > 0. {
                                let w = f(over / (1. - th)) * (1. - th) + th;
                                if v < 0. { -w } else { w }
                            } else {
                                v
                            }
                        }
                        None if mode != 0 => if v <= th { (-th).max(v) } else { th },
                        None => v,
                    }
                })
                .collect();
            (ch, out)
        }
        WaveFilter::PhaseVocoder { pitch, time, window, overlap } => {
            (ch, crate::phase_vocoder::process(*pitch, *time, *window, *overlap, rate, ch, input, cancel)?)
        }
        WaveFilter::Abs => (ch, input.iter().map(|x| x.abs()).collect()),
        WaveFilter::Add { value } => (ch, input.iter().map(|x| x + value).collect()),
        WaveFilter::Mul { value } => (ch, input.iter().map(|x| x * value).collect()),
        WaveFilter::RingMod { carrier } => {
            let c = crate::carriers::generate(carrier, rate, input.len() / ch, cancel)?;
            (ch, input.iter().enumerate().map(|(n, x)| x * c[n / ch]).collect())
        }
        WaveFilter::Vocoder { carrier, formant_shift, reaction_time, bands, filters_per_band } => {
            let frames = input.len() / ch;
            let c = crate::carriers::generate(carrier, rate, frames, cancel)?;
            let mut v = crate::carriers::Vocoder::new(*bands, *filters_per_band, rate, *formant_shift, *reaction_time);
            let mut out = Vec::with_capacity(frames);
            for n in 0..frames {
                if n % 16384 == 0 {
                    cancel.check()?;
                }
                let m = if ch == 2 { (input[n * 2] + input[n * 2 + 1]) * 0.5 } else { input[n] };
                out.push(v.tick(c[n], m));
            }
            (1, out)
        }
        WaveFilter::Iir { kind, family, structure, params } => {
            let mut p = *params;
            p[0] = rate as f64;
            let stages = iir_design::design(*kind, *family, &p)
                .with_context(|| format!("WaveDSPFilter {kind:?}/{family:?} is not a DSPFilters design"))?;
            let mut state = vec![iir_design::State::new(*structure, stages.len()); ch];
            let mut out = Vec::with_capacity(input.len());
            for (n, x) in input.iter().enumerate() {
                if n % 65536 == 0 {
                    cancel.check()?;
                }
                out.push(state[n % ch].process(*x as f64, &stages) as f32);
            }
            (ch, out)
        }
    })
}

/// Runs `chain` over interleaved `input` (1 or 2 channels) at `rate`;
/// returns the output channel count and interleaved samples.
pub fn apply(chain: &[WaveFilter], rate: u32, channels: u16, input: Vec<f32>, cancel: &Cancellation) -> Result<(u16, Vec<f32>)> {
    ensure!((1..=2).contains(&channels), "wave filters need mono or stereo input");
    ensure!((8000..=192000).contains(&rate), "wave filter sample rate {rate}");
    let mut ch = channels as usize;
    let mut pcm = input;
    for f in chain {
        f.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
        let tail = match f {
            WaveFilter::Delay { extend_ms, .. } | WaveFilter::FreeVerb { extend_ms, .. } => tail_frames(*extend_ms, rate),
            _ => 0,
        };
        ensure!((pcm.len() / ch + tail) * 2 <= MAX_SAMPLES, "wave filter output budget");
        let (c, out) = stage(f, rate, ch, &pcm, cancel)?;
        ch = c;
        pcm = out;
    }
    Ok((ch as u16, pcm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::{IirFamily, IirKind, IirStructure};
    #[test]
    fn delay_feeds_back_and_extends() {
        // 1 ms at 8 kHz = 8 samples; impulse, feedback 0.5, mix 1, 2 ms tail.
        let f = WaveFilter::Delay { time_ms: 1, feedback: 0.5, mix: 1., extend_ms: 2 };
        let (ch, out) = apply(&[f], 8000, 1, vec![1.], &Cancellation::default()).unwrap();
        assert_eq!((ch, out.len()), (1, 17));
        let mut want = vec![0f32; 17];
        want[0] = 1.;
        want[8] = 0.5;
        want[16] = 0.25;
        assert_eq!(out, want);
    }
    #[test]
    fn freeverb_is_stereo_and_decays() {
        let f = WaveFilter::FreeVerb { effect_mix: 0.75, room_size: 0.75, damping: 0.25, width: 1., frozen: false, extend_ms: 3000 };
        let mut input = vec![0f32; 100];
        input[0] = 1.;
        let (ch, out) = apply(&[f], 44100, 1, input, &Cancellation::default()).unwrap();
        assert_eq!(ch, 2);
        assert_eq!(out.len(), (100 + 132300) * 2);
        // t=0: the combs pass 2·0.015 each, the four allpasses flip the sign
        // four times, plus the dry path.
        let wet = 2.25 / 2.75;
        let dry = 0.5 / (wet + 0.5);
        assert!((out[0] as f64 - (0.24 * wet + dry)).abs() < 1e-6, "{}", out[0]);
        assert!(out[2 * 1116..2 * 1700].iter().any(|v| v.abs() > 1e-4));
        let late: f32 = out[out.len() - 2000..].iter().map(|v| v.abs()).fold(0., f32::max);
        assert!(late < 1e-3, "{late}");
    }
    #[test]
    fn graphic_eq_sums_ten_unity_bands() {
        let f = WaveFilter::GraphicEq { gains: [1.; 10] };
        let (_, out) = apply(&[f], 44100, 2, vec![0.01, -0.02, 0.03, 0.], &Cancellation::default()).unwrap();
        for (o, x) in out.iter().zip([0.01f32, -0.02, 0.03, 0.]) {
            assert!((o - 10. * x).abs() < 1e-6);
        }
        // A cut band lowers its centre frequency.
        let mut gains = [1f32; 10];
        gains[5] = 0.1;
        let c = eq_design(&gains, 44100);
        let mut s = [0f32; 41];
        let n = 44100;
        let mut peak = 0f32;
        for i in 0..n {
            let x = (2. * std::f32::consts::PI * 1000. * i as f32 / 44100.).sin();
            let y = eq_tick(&c, &mut s, x);
            if i > n / 2 {
                peak = peak.max(y.abs());
            }
        }
        assert!(peak < 9.5, "{peak}");
    }
    #[test]
    fn iir_low_pass_attenuates_high_frequencies() {
        let f = WaveFilter::Iir {
            kind: IirKind::LowPass,
            family: IirFamily::Butterworth,
            structure: IirStructure::DirectFormII,
            params: [0., 4., 1000., 0., 0., 0., 0., 0.],
        };
        let rate = 44100;
        let tone = |hz: f32| (0..rate).map(|i| (2. * std::f32::consts::PI * hz * i as f32 / rate as f32).sin()).collect::<Vec<_>>();
        let level = |v: &[f32]| v[v.len() / 2..].iter().map(|x| x.abs()).fold(0., f32::max);
        let (_, low) = apply(&[f.clone()], rate as u32, 1, tone(100.), &Cancellation::default()).unwrap();
        let (_, high) = apply(&[f], rate as u32, 1, tone(8000.), &Cancellation::default()).unwrap();
        assert!(level(&low) > 0.99 && level(&high) < 0.001, "{} {}", level(&low), level(&high));
    }
}
