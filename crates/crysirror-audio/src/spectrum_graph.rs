// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Source-window sine transform and retained spectrum envelopes. Numerical
//! profile follows fftgraph's window/DST convention; FFT uses engine f32 math.
use anyhow::{Result, ensure};
use crysirror_core::service::{SpectrumGraphSpec, SpectrumGraphState, SpectrumGraphStyle};
const N: usize = 2048;
pub fn analyze(
    samples: &[[f32; 2]],
    spec: &SpectrumGraphSpec,
    mut state: SpectrumGraphState,
    check: &dyn Fn() -> Result<()>,
) -> Result<SpectrumGraphState> {
    ensure!(samples.len() <= N, "spectrum input sample budget");
    ensure!(
        samples.iter().flatten().all(|x| x.is_finite()),
        "spectrum finite samples"
    );
    let mut work = vec![0f32; 4 * N];
    // The native getVisBuffer zeroes the complete window on a short read.
    if samples.len() == N {
        let mul = 4.0 / (32768.0 * N as f64);
        for i in 0..N {
            check()?;
            let value = (((samples[i][0] + samples[i][1]) * 0.5).clamp(-1., 1.) * 32768.)
                .round()
                .clamp(-32768., 32767.) as i16;
            let j = if i < N / 2 { i } else { N - i - 1 };
            let window = ((std::f64::consts::PI * (j as f64 + 0.5) / N as f64).sin() * mul) as f32;
            let v = f32::from(value) * window;
            work[2 * i] = v;
            work[2 * (2 * N - 1 - i)] = -v;
        }
    }
    crate::pcm_filters::fft(&mut work, 2 * N, 1.);
    check()?;
    let mut spectrum = vec![0f32; N];
    // Odd extension turns the positive complex FFT into unnormalized DST-II.
    for k in 2..N {
        let theta = std::f64::consts::PI * k as f64 / (2 * N) as f64;
        spectrum[k] = ((f64::from(work[2 * k]) * theta.sin()
            + f64::from(work[2 * k + 1]) * theta.cos())
            * 0.5) as f32;
    }
    let [_, _, w, h] = spec.rect;
    let (count, cut, maxval, fall, hold, peakfall) = match spec.style {
        SpectrumGraphStyle::Fire => (
            w as usize,
            3.7f32,
            h as i32 - 1,
            (h + 15) as i32 / 16,
            30,
            (h + 31) as i32 / 32,
        ),
        SpectrumGraphStyle::Bars {
            divisions,
            thickness,
            ..
        } => {
            let step = 1000 / (h / thickness) as i32;
            (divisions as usize, 8., 1000 - step * 2, 40, 40, 10)
        }
    };
    ensure!(
        count > 0
            && count <= 8192
            && state.levels.len() <= 8192
            && state.peaks.len() <= 8192
            && state.peak_age.len() <= 8192,
        "spectrum state budget"
    );
    if state.levels.len() != count
        || state.cut != cut
        || state.peaks.len() != count
        || state.peak_age.len() != count
    {
        state = SpectrumGraphState {
            cut,
            levels: vec![0; count],
            peaks: vec![0; count],
            peak_age: vec![0; count],
        };
    }
    for i in 0..count {
        check()?;
        let a = i as f32 * ((cut - 1.) / cut) / count as f32 + 1. / cut;
        let b = (i + 1) as f32 * ((cut - 1.) / cut) / count as f32 + 1. / cut;
        let start = (N as f64).powf(f64::from(a)) as usize;
        let mut end = (N as f64).powf(f64::from(b)) as usize;
        if end == start {
            end += 1;
        }
        end = end.min(N);
        let max = spectrum
            .get(start.min(N)..end.max(start.min(N)))
            .unwrap_or_default()
            .iter()
            .fold(0f32, |m, x| m.max(x.abs()));
        let db = if max == 0. {
            -1_000_000.
        } else {
            (max * max).log10() * 10.
        }
        .clamp(-70., 0.);
        let level = (maxval as f32 - (maxval as f32 / -70.) * db) as i32;
        state.levels[i] = state.levels[i]
            .saturating_sub(fall)
            .max(0)
            .max(level)
            .min(maxval);
        if state.peak_age[i] == hold {
            state.peaks[i] = state.peaks[i].saturating_sub(peakfall).max(0);
        } else {
            state.peak_age[i] = state.peak_age[i].saturating_add(1);
        }
        if state.peaks[i] < level {
            state.peaks[i] = level;
            state.peak_age[i] = 0;
        }
    }
    Ok(state)
}
