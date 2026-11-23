// SPDX-License-Identifier: MPL-2.0
//! Pre-gain mean absolute PCM16 levels at the decoded source's playback cursor.
use crate::{decoder::AudioSource, levels::to_pcm16};
use anyhow::{Result, ensure};

/// Compute complete windows of interleaved source samples. Each observation is
/// delivered immediately. Decoder read-ahead alone does not advance playback.
pub fn read_windows(
    source: &mut dyn AudioSource,
    window: u32,
    mut publish: impl FnMut(u32, u16) -> Result<()>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<()> {
    ensure!(window > 0, "empty PCM analysis window");
    let channels = u32::from(source.channel_count().unwrap_or(2));
    ensure!(
        (1..=2).contains(&channels),
        "PCM level analysis requires mono or stereo"
    );
    let mut frames = [[0.; 2]; 2048];
    let (mut sum, mut count) = (0u64, 0u32);
    while !cancelled() {
        let n = source.read(&mut frames)?;
        if n == 0 {
            break;
        }
        for frame in &frames[..n] {
            for &sample in &frame[..channels as usize] {
                sum += u64::from(to_pcm16(sample).unsigned_abs());
                count += 1;
                if count == window {
                    publish(channels, (sum / u64::from(window)) as u16)?;
                    sum = 0;
                    count = 0;
                }
            }
        }
    }
    Ok(())
}
