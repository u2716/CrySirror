// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Integer stereo PCM filter chain ([`PcmFilter`]).
//!
//! The arithmetic reproduces the QLIE OggData TODFilter family (Echo 57FC48,
//! Reverb 57EB70, Chorus 5802B4, Volume 57F6B0, Distortion 57F9AC, WhiteNoise
//! 57F368, 3BandEQ 57EF6C/57EE40, Mono 580158, PitchShift 5806F0/57BD44, AGC
//! 57C87C): 16-bit-scaled i32 frames, `/256` divisions truncating toward zero,
//! f32 filter state and the native 44100 Hz constants. PitchShift is the
//! public smbPitchShift phase vocoder (2048-point frames, 4x overlap).
use anyhow::{Result, ensure};
use crysirror_core::service::{PcmFilter, PcmFilterMix, PcmNoiseBlend};
use crysirror_tasks::Cancellation;

const RING: usize = 0x10000;
const RATE: f64 = 44100.;

/// Signed division by 256 truncating toward zero (`test/jns/add 255/sar 8`).
fn q8(v: i32) -> i32 {
    v.wrapping_add(if v < 0 { 255 } else { 0 }) >> 8
}
fn percent256(v: i32) -> i32 {
    (v << 8) / 100
}
/// 57E6F0: (left wet, right wet, dry) gains in 1/256 units.
fn routing(mix: PcmFilterMix, wet: i32, dry: i32) -> [i32; 3] {
    let wet = percent256(wet);
    let dry = percent256(dry);
    let (left, right) = match mix {
        PcmFilterMix::Normal => (wet, wet),
        PcmFilterMix::Stereo => (wet, -wet),
        PcmFilterMix::Left => (wet, 0),
        PcmFilterMix::Right => (0, wet),
    };
    [left, right, dry]
}
/// 57FEEC delay in frames.
fn delay_frames(ms: i32) -> usize {
    ((ms * 44100) as f32 / 1000. + 0.5) as usize
}
struct Delay {
    buffer: Vec<[i32; 2]>,
    read: usize,
    write: usize,
}
impl Delay {
    fn new(ms: i32) -> Self {
        Self {
            buffer: vec![[0; 2]; RING],
            read: 0,
            write: delay_frames(ms) % RING,
        }
    }
    fn take(&mut self) -> [i32; 2] {
        let v = self.buffer[self.read];
        self.read = (self.read + 1) % RING;
        v
    }
    fn put(&mut self, v: [i32; 2]) {
        self.buffer[self.write] = v;
        self.write = (self.write + 1) % RING;
    }
}
fn echo(frames: &mut [[i32; 2]], decay: i32, delay_ms: i32, wet: i32, dry: i32, mix: PcmFilterMix, room: Option<i32>) {
    let [wl, wr, d] = routing(mix, wet, dry);
    let feedback = ((100 - decay) << 8) / 100;
    // 57EDF4: one-pole low-pass coefficient of the reverb's delayed signal.
    let coef = room.map(|hz| ((hz as f64 / RATE * std::f64::consts::PI).sin() * 2.) as f32);
    let mut line = Delay::new(delay_ms);
    let mut lp = [0f32; 2];
    for frame in frames {
        let [l, r] = *frame;
        let mut delayed = line.take();
        if let Some(c) = coef {
            for ch in 0..2 {
                lp[ch] = (delayed[ch] as f32 - lp[ch]) * c + lp[ch];
                delayed[ch] = lp[ch] as i32;
            }
        }
        line.put([l + q8(delayed[0].wrapping_mul(feedback)), r + q8(delayed[1].wrapping_mul(feedback))]);
        frame[0] = q8(d.wrapping_mul(l).wrapping_add(wl.wrapping_mul(delayed[0])));
        frame[1] = q8(d.wrapping_mul(r).wrapping_add(wr.wrapping_mul(delayed[1])));
    }
}
fn sine_table() -> Vec<f32> {
    (0..4096)
        .map(|i| (i as f64 * 6.2831853 * 0.000_244_140_62).sin() as f32)
        .collect()
}
#[allow(clippy::too_many_arguments)]
fn chorus(frames: &mut [[i32; 2]], decay: i32, delay_ms: i32, wet: i32, dry: i32, mix: PcmFilterMix, rate: i32, depth: i32) {
    let [wl, wr, d] = routing(mix, wet, dry);
    let feedback = ((100 - decay) << 8) / 100;
    let table = sine_table();
    let step = (rate as f64 * 0.01) as f32;
    let mut depth = ((depth as f64 * 44.1) as f32) * 0.5;
    let delay = delay_frames(delay_ms) as f32;
    if delay < depth {
        depth = delay;
    }
    let mut line = Delay::new(delay_ms);
    let mut phase = 0f32;
    for frame in frames {
        let [l, r] = *frame;
        let index = ((4096. * phase as f64 * 2.267_573_696_145_124_8e-5 + 0.5) as i64 & 0xfff) as usize;
        let mut offset = depth * table[index];
        phase += step;
        if phase > 44100. {
            phase -= 44100.;
        }
        let at = (line.read as f32 + offset) as i64;
        let a = (at & 0xffff) as usize;
        let b = ((at + 1) & 0xffff) as usize;
        offset += 65536.;
        let fraction = offset - (offset as i64) as f32;
        let w = (fraction * 256.) as i32;
        let left = q8(line.buffer[a][0] * (256 - w) + line.buffer[b][0] * w);
        let right = q8(line.buffer[a][1] * (256 - w) + line.buffer[b][1] * w);
        line.read = (line.read + 1) % RING;
        // 5802B4 feeds the left modulated tap back into both channels.
        let back = q8(left.wrapping_mul(feedback)) + l;
        line.put([back, back]);
        frame[0] = q8(l.wrapping_mul(d).wrapping_add(left.wrapping_mul(wl)));
        frame[1] = q8(r.wrapping_mul(d).wrapping_add(right.wrapping_mul(wr)));
    }
}
fn volume(frames: &mut [[i32; 2]], volume: i32, range: i32, pan: i32) {
    let right = (pan * 100 / 50).min(100);
    let left = ((100 - pan) * 100 / 50).min(100);
    let right = percent256(volume * right / 100);
    let left = percent256(volume * left / 100);
    let limit = (range << 15) / 100;
    for frame in frames {
        frame[0] = q8(frame[0].wrapping_mul(left)).clamp(-limit, limit);
        frame[1] = q8(frame[1].wrapping_mul(right)).clamp(-limit, limit);
    }
}
fn distortion(frames: &mut [[i32; 2]], volume: i32, range: i32, foldback: bool) {
    let limit = (range << 15) / 100;
    let gain = percent256(volume);
    // 57F968: hard clip, or fold the excess back by half of it.
    let fold = |x: i32, t: i32| {
        if !foldback {
            return t;
        }
        let d = (x - t) / 2;
        if t > 0 { t - d.min(t) } else { t - d.max(t) }
    };
    for frame in frames {
        for s in frame.iter_mut() {
            if *s > limit {
                *s = fold(*s, limit);
            }
            if *s < -limit {
                *s = fold(*s, -limit);
            }
            *s = q8(s.wrapping_mul(gain));
        }
    }
}
fn noise(frames: &mut [[i32; 2]], amount: i32, volume: i32, blend: PcmNoiseBlend) {
    let wet = percent256(amount);
    let dry = 256 - wet;
    let level = percent256(volume);
    let mut seed: u32 = 0x34AB_CD0E;
    for frame in frames {
        seed = seed.wrapping_mul(0x41C6_4E6D).wrapping_add(0x3039);
        let n = q8((((seed & 0xffff) as i32) - 0x8000).wrapping_mul(level));
        let [l, r] = *frame;
        let (a, b) = match blend {
            PcmNoiseBlend::Replace => (n, n),
            PcmNoiseBlend::Add => (n + l, n + r),
            PcmNoiseBlend::Multiply => {
                let k = n / 16;
                ((k * l) / 2048, (k * r) / 2048)
            }
        };
        frame[0] = q8(l.wrapping_mul(dry).wrapping_add(a.wrapping_mul(wet)));
        frame[1] = q8(r.wrapping_mul(dry).wrapping_add(b.wrapping_mul(wet)));
    }
}
#[derive(Default)]
struct Eq3 {
    lf: f32,
    f1: [f32; 4],
    hf: f32,
    f2: [f32; 4],
    sdm: [f32; 3],
}
fn three_band(frames: &mut [[i32; 2]], gains: [i32; 3], low_hz: i32, high_hz: i32) {
    let coef = |hz: i32| ((hz as f64 / RATE * 3.141_592_653_5).sin() * 2.) as f32;
    let new = || Eq3 { lf: coef(low_hz), hf: coef(high_hz), ..Default::default() };
    let mut state = [new(), new()];
    let g = gains.map(|v| v as f64 * 0.01);
    for frame in frames {
        for (ch, s) in frame.iter_mut().enumerate() {
            let e = &mut state[ch];
            let x = *s as f32;
            e.f1[0] += (x - e.f1[0]) * e.lf;
            for i in 1..4 {
                e.f1[i] += (e.f1[i - 1] - e.f1[i]) * e.lf;
            }
            let low = e.f1[3];
            e.f2[0] += (x - e.f2[0]) * e.hf;
            for i in 1..4 {
                e.f2[i] += (e.f2[i - 1] - e.f2[i]) * e.hf;
            }
            let high = e.sdm[2] - e.f2[3];
            let mid = e.sdm[2] - (high + low);
            let l = (g[0] * low as f64) as f32;
            let m = (g[1] * mid as f64) as f32;
            let h = (g[2] * high as f64) as f32;
            e.sdm = [x, e.sdm[0], e.sdm[1]];
            *s = (l + m + h) as i32;
        }
    }
}
fn mono(frames: &mut [[i32; 2]], amount: i32) {
    let a = percent256(amount);
    let keep = 256 - a;
    for frame in frames {
        let [l, r] = *frame;
        let mid = (l + r) / 2;
        frame[0] = q8(l.wrapping_mul(keep).wrapping_add(mid.wrapping_mul(a)));
        frame[1] = q8(r.wrapping_mul(keep).wrapping_add(mid.wrapping_mul(a)));
    }
}
/// smbFft: in-place complex FFT over interleaved re/im pairs.
pub(crate) fn fft(buffer: &mut [f32], size: usize, sign: f32) {
    let mut i = 2;
    while i < 2 * size - 2 {
        let mut j = 0;
        let mut bit = 2;
        while bit < 2 * size {
            if i & bit != 0 {
                j += 1;
            }
            j <<= 1;
            bit <<= 1;
        }
        if i < j {
            buffer.swap(i, j);
            buffer.swap(i + 1, j + 1);
        }
        i += 2;
    }
    let stages = ((size as f64).ln() / 2f64.ln() + 0.5) as usize;
    let mut le = 2;
    for _ in 0..stages {
        le <<= 1;
        let le2 = le >> 1;
        let (mut ur, mut ui) = (1f32, 0f32);
        let arg = std::f32::consts::PI / (le2 >> 1) as f32;
        let (wr, wi) = (arg.cos(), sign * arg.sin());
        let mut j = 0;
        while j < le2 {
            let mut p = j;
            while p < 2 * size {
                let q = p + le2;
                let tr = buffer[q] * ur - buffer[q + 1] * ui;
                let ti = buffer[q] * ui + buffer[q + 1] * ur;
                buffer[q] = buffer[p] - tr;
                buffer[q + 1] = buffer[p + 1] - ti;
                buffer[p] += tr;
                buffer[p + 1] += ti;
                p += le;
            }
            let t = ur * wr - ui * wi;
            ui = ur * wi + ui * wr;
            ur = t;
            j += 2;
        }
    }
}
fn pitch_shift(input: &[f32], shift: f32, cancel: &Cancellation) -> Result<Vec<f32>> {
    const SIZE: usize = 2048;
    const OSAMP: usize = 4;
    let half = SIZE / 2;
    let step = SIZE / OSAMP;
    let per_bin = RATE / SIZE as f64;
    let expected = 2. * std::f64::consts::PI * step as f64 / SIZE as f64;
    let latency = SIZE - step;
    let window: Vec<f32> = (0..SIZE)
        .map(|k| (-0.5 * (2. * std::f64::consts::PI * k as f64 / SIZE as f64).cos() + 0.5) as f32)
        .collect();
    let mut in_fifo = vec![0f32; SIZE];
    let mut out_fifo = vec![0f32; SIZE];
    let mut work = vec![0f32; 2 * SIZE];
    let mut last_phase = vec![0f32; half + 1];
    let mut sum_phase = vec![0f32; half + 1];
    let mut accum = vec![0f32; 2 * SIZE];
    let (mut ana_freq, mut ana_magn) = (vec![0f32; SIZE], vec![0f32; SIZE]);
    let (mut syn_freq, mut syn_magn) = (vec![0f32; SIZE], vec![0f32; SIZE]);
    let mut rover = latency;
    let mut out = Vec::with_capacity(input.len());
    for &sample in input {
        in_fifo[rover] = sample;
        out.push(out_fifo[rover - latency]);
        rover += 1;
        if rover < SIZE {
            continue;
        }
        cancel.check()?;
        rover = latency;
        for k in 0..SIZE {
            work[2 * k] = in_fifo[k] * window[k];
            work[2 * k + 1] = 0.;
        }
        fft(&mut work, SIZE, -1.);
        for k in 0..=half {
            let (re, im) = (work[2 * k] as f64, work[2 * k + 1] as f64);
            let magn = 2. * (re * re + im * im).sqrt();
            let phase = im.atan2(re);
            let mut tmp = phase - last_phase[k] as f64;
            last_phase[k] = phase as f32;
            tmp -= k as f64 * expected;
            let mut qpd = (tmp / std::f64::consts::PI) as i64;
            qpd += if qpd >= 0 { qpd & 1 } else { -(qpd & 1) };
            tmp -= std::f64::consts::PI * qpd as f64;
            tmp = OSAMP as f64 * tmp / (2. * std::f64::consts::PI);
            ana_magn[k] = magn as f32;
            ana_freq[k] = (k as f64 * per_bin + tmp * per_bin) as f32;
        }
        syn_magn.iter_mut().for_each(|v| *v = 0.);
        syn_freq.iter_mut().for_each(|v| *v = 0.);
        for k in 0..=half {
            let index = (k as f32 * shift) as usize;
            if index <= half {
                syn_magn[index] += ana_magn[k];
                syn_freq[index] = ana_freq[k] * shift;
            }
        }
        for k in 0..=half {
            let mut tmp = syn_freq[k] as f64 - k as f64 * per_bin;
            tmp /= per_bin;
            tmp = 2. * std::f64::consts::PI * tmp / OSAMP as f64;
            tmp += k as f64 * expected;
            sum_phase[k] = (sum_phase[k] as f64 + tmp) as f32;
            let phase = sum_phase[k] as f64;
            work[2 * k] = (syn_magn[k] as f64 * phase.cos()) as f32;
            work[2 * k + 1] = (syn_magn[k] as f64 * phase.sin()) as f32;
        }
        work[SIZE + 2..2 * SIZE].iter_mut().for_each(|v| *v = 0.);
        fft(&mut work, SIZE, 1.);
        for k in 0..SIZE {
            accum[k] += (2. * window[k] as f64 * work[2 * k] as f64 / (half * OSAMP) as f64) as f32;
        }
        out_fifo[..step].copy_from_slice(&accum[..step]);
        accum.copy_within(step..step + SIZE, 0);
        in_fifo.copy_within(step..step + latency, 0);
    }
    Ok(out)
}
fn agc(frames: &mut [[i32; 2]], gain: i32, rate: i32, quiet: i32, target: i32) {
    if target <= quiet {
        return;
    }
    let rate = 1. / rate as f32;
    let mut g = gain as f32 / 100.;
    let window = 12288;
    let (mut count, mut peak, mut silent) = (0i32, 0i32, 0i32);
    for s in frames.iter_mut().flat_map(|f| f.iter_mut()) {
        let out = (*s as f64 * g as f64) as i32;
        let a = out.wrapping_abs();
        if s.wrapping_abs() >= quiet {
            silent = 0;
        } else {
            silent += 1;
        }
        if a >= target {
            g -= (a as f64 / target as f64 * rate as f64) as f32;
            peak = 0;
            count = 0;
        } else {
            peak = peak.max(a);
            count += 1;
            if window < count {
                if window >= silent {
                    g += (target as f64 / peak as f64 * rate as f64) as f32;
                    peak = 0;
                    count = 0;
                } else {
                    g += 10. * rate * (1. - g);
                }
            }
        }
        *s = out;
    }
}
/// Apply `chain` in order to interleaved PCM16 (mono is widened to stereo,
/// as the native stream does). Returns stereo PCM16, saturated at output.
pub fn apply(chain: &[PcmFilter], channels: u16, pcm: &[i16], cancel: &Cancellation) -> Result<Vec<i16>> {
    ensure!((1..=2).contains(&channels), "PCM filter channel layout");
    for filter in chain {
        filter.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    let mut frames: Vec<[i32; 2]> = if channels == 1 {
        pcm.iter().map(|&s| [i32::from(s); 2]).collect()
    } else {
        pcm.chunks_exact(2).map(|p| [i32::from(p[0]), i32::from(p[1])]).collect()
    };
    for filter in chain {
        cancel.check()?;
        match *filter {
            PcmFilter::Echo { decay, delay_ms, wet, dry, mix } => echo(&mut frames, decay, delay_ms, wet, dry, mix, None),
            PcmFilter::Reverb { decay, delay_ms, wet, dry, mix, room_hz } => {
                echo(&mut frames, decay, delay_ms, wet, dry, mix, Some(room_hz))
            }
            PcmFilter::Chorus { decay, delay_ms, wet, dry, mix, rate, depth } => {
                chorus(&mut frames, decay, delay_ms, wet, dry, mix, rate, depth)
            }
            PcmFilter::Volume { volume: v, range, pan } => volume(&mut frames, v, range, pan),
            PcmFilter::Distortion { volume, range, foldback } => distortion(&mut frames, volume, range, foldback),
            PcmFilter::Noise { amount, volume, blend } => noise(&mut frames, amount, volume, blend),
            PcmFilter::ThreeBandEq { low, mid, high, low_hz, high_hz } => {
                three_band(&mut frames, [low, mid, high], low_hz, high_hz)
            }
            PcmFilter::Mono { amount } => mono(&mut frames, amount),
            PcmFilter::PitchShift { pitch, wet, dry, mix } => {
                let [wl, wr, d] = routing(mix, wet, dry);
                // 5806F0: mono mid * 1/32768 into the vocoder, *32768 + 0.5 out.
                let input: Vec<f32> = frames
                    .iter()
                    .map(|f| ((f[0] + f[1]) / 2) as f32 * 3.051_757_8e-5)
                    .collect();
                let shifted = pitch_shift(&input, (pitch as f64 * 0.01) as f32, cancel)?;
                for (frame, s) in frames.iter_mut().zip(shifted) {
                    let v = (s * 32768. + 0.5) as i32;
                    frame[0] = q8(frame[0].wrapping_mul(d).wrapping_add(v.wrapping_mul(wl)));
                    frame[1] = q8(frame[1].wrapping_mul(d).wrapping_add(v.wrapping_mul(wr)));
                }
            }
            PcmFilter::Agc { gain, rate, quiet, target } => agc(&mut frames, gain, rate, quiet, target),
        }
    }
    Ok(frames
        .into_iter()
        .flat_map(|f| f.map(|s| s.clamp(-32768, 32767) as i16))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn echo_repeats_after_its_delay_and_volume_limits() {
        let mut pcm = vec![0i16; 2 * 9000];
        pcm[0] = 10000;
        pcm[1] = 10000;
        let chain = [PcmFilter::Echo { decay: 50, delay_ms: 100, wet: 50, dry: 100, mix: PcmFilterMix::Normal }];
        let out = apply(&chain, 2, &pcm, &Cancellation::default()).unwrap();
        // Frame 0: dry 256*10000/256; delayed copy at 4410 frames scaled by wet 128.
        assert_eq!(out[0], 10000);
        assert_eq!(out[2 * 4410], 5000);
        let loud = vec![30000i16; 4];
        let out = apply(&[PcmFilter::Volume { volume: 200, range: 50, pan: 50 }], 2, &loud, &Cancellation::default()).unwrap();
        assert_eq!(out[0], 16384);
    }
    #[test]
    fn pitch_shift_scales_frequency() {
        let n = 2048 * 8;
        let pcm: Vec<i16> = (0..n).map(|i| ((i as f32 * 0.05).sin() * 8000.) as i16).collect();
        let crossings = |pitch: i32| {
            let chain = [PcmFilter::PitchShift { pitch, wet: 100, dry: 0, mix: PcmFilterMix::Normal }];
            let out = apply(&chain, 1, &pcm, &Cancellation::default()).unwrap();
            (8192..14336).filter(|i| (out[2 * i] >= 0) != (out[2 * i + 2] >= 0)).count()
        };
        let input = (8192..14336).filter(|i| (pcm[*i] >= 0) != (pcm[i + 1] >= 0)).count() as f64;
        assert!((crossings(100) as f64 / input - 1.).abs() < 0.05);
        assert!((crossings(200) as f64 / input - 2.).abs() < 0.1);
    }
}
