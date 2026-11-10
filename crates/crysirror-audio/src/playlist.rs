//! Shared gapless segmented decoder. Program traversal and audio decoding run
//! inside the normal BufferedSource worker; only one clip decoder stays open.
use crate::decoder::{AudioSource, StereoFrame};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::{AudioPlaylist, AudioPlaylistNode as Node};

fn extent(node: &Node, lengths: &[u64]) -> Result<Option<u64>> {
    match node {
        Node::Clip(i) => Ok(Some(lengths[usize::from(*i)])),
        Node::Sequence(v) | Node::Repeat { body: v, .. } => {
            let mut sum = 0u64;
            for child in v {
                let Some(n) = extent(child, lengths)? else {
                    return Ok(None);
                };
                sum = sum.checked_add(n).context("playlist duration overflow")?;
            }
            match node {
                Node::Repeat { count: None, .. } => Ok(None),
                Node::Repeat { count: Some(n), .. } => Ok(Some(
                    sum.checked_mul(u64::from(*n))
                        .context("playlist repeats overflow")?,
                )),
                _ => Ok(Some(sum)),
            }
        }
    }
}
fn locate(node: &Node, mut frame: u64, lengths: &[u64]) -> Result<Option<(usize, u64, u64)>> {
    match node {
        Node::Clip(i) => {
            let i = usize::from(*i);
            Ok((frame < lengths[i]).then_some((i, frame, lengths[i].saturating_sub(frame))))
        }
        Node::Sequence(v) | Node::Repeat { body: v, .. } => {
            if let Node::Repeat { count, .. } = node {
                let mut cycle = Some(0u64);
                for child in v {
                    cycle = match (cycle, extent(child, lengths)?) {
                        (Some(a), Some(b)) => {
                            Some(a.checked_add(b).context("playlist cycle overflow")?)
                        }
                        _ => None,
                    };
                }
                if let Some(cycle) = cycle {
                    ensure!(cycle > 0, "empty playlist repeat");
                    if count.is_some_and(|n| frame / cycle >= u64::from(n)) {
                        return Ok(None);
                    }
                    frame %= cycle;
                }
            }
            for child in v {
                match extent(child, lengths)? {
                    Some(n) if frame >= n => frame -= n,
                    _ => return locate(child, frame, lengths),
                }
            }
            Ok(None)
        }
    }
}
pub(crate) fn source(
    program: AudioPlaylist,
    gain: f32,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Box<dyn AudioSource>> {
    program.encoded_bytes()?;
    let mut lengths = Vec::new();
    let mut rate = 0;
    let mut replay_gain = false;
    for clip in &program.clips {
        cancel.check()?;
        let source = crate::channels::source(clip.clone(), gain)?;
        if rate == 0 {
            rate = source.sample_rate();
        }
        ensure!(
            source.sample_rate() == rate,
            "playlist clips must share a sample rate"
        );
        let n = source
            .frame_count()
            .context("playlist clip needs a measured frame count")?;
        ensure!(n > 0, "empty playlist clip");
        lengths.push(n);
        replay_gain |= source.has_replay_gain();
    }
    let frames = extent(&program.program, &lengths)?;
    Ok(Box::new(Playlist {
        program,
        lengths,
        rate,
        frames,
        gain,
        replay_gain,
        at: 0,
        current: None,
    }))
}
struct Playlist {
    program: AudioPlaylist,
    lengths: Vec<u64>,
    rate: u32,
    frames: Option<u64>,
    gain: f32,
    replay_gain: bool,
    at: u64,
    current: Option<(usize, u64, Box<dyn AudioSource>)>,
}
impl AudioSource for Playlist {
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn frame_count(&self) -> Option<u64> {
        self.frames
    }
    fn has_replay_gain(&self) -> bool {
        self.replay_gain
    }
    fn read_position(&self) -> Option<u64> {
        Some(self.at)
    }
    fn seek(&mut self, frame: u64) -> Result<()> {
        ensure!(
            self.frames.is_none_or(|end| frame <= end),
            "playlist seek out of bounds"
        );
        self.at = frame;
        self.current = None;
        Ok(())
    }
    fn read(&mut self, output: &mut [StereoFrame]) -> Result<usize> {
        let mut written = 0;
        while written < output.len() {
            let Some((index, offset, remaining)) =
                locate(&self.program.program, self.at, &self.lengths)?
            else {
                break;
            };
            if self
                .current
                .as_ref()
                .is_none_or(|(i, at, _)| *i != index || *at != offset)
            {
                let mut source =
                    crate::channels::source(self.program.clips[index].clone(), self.gain)?;
                if offset != 0 {
                    source.seek(offset)?;
                }
                self.current = Some((index, offset, source));
            }
            let (_, position, source) = self.current.as_mut().unwrap();
            let count =
                (output.len() - written).min(usize::try_from(remaining).unwrap_or(usize::MAX));
            let n = source.read(&mut output[written..written + count])?;
            ensure!(
                n > 0 && n <= count,
                "playlist clip ended before declared duration"
            );
            *position += n as u64;
            self.at = self
                .at
                .checked_add(n as u64)
                .context("playlist position overflow")?;
            written += n;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::EncodedAudio;
    fn clip(values: &[i16]) -> EncodedAudio {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + values.len() as u32 * 2).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(48000u32.to_le_bytes());
        bytes.extend(96000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((values.len() as u32 * 2).to_le_bytes());
        for n in values {
            bytes.extend(n.to_le_bytes());
        }
        EncodedAudio {
            bytes: bytes.into(),
            frame_limit: None,
        }
    }
    #[test]
    fn shared_segment_program_reads_gaplessly_and_seeks_across_nested_repeats() {
        let program = AudioPlaylist {
            clips: vec![clip(&[8192, 16384]), clip(&[-8192])],
            program: Node::Sequence(vec![
                Node::Clip(0),
                Node::Repeat {
                    count: Some(2),
                    body: vec![Node::Clip(1), Node::Clip(0)],
                },
            ]),
        };
        let mut decoder = source(program, 1., &crysirror_tasks::Cancellation::default()).unwrap();
        assert_eq!(decoder.frame_count(), Some(8));
        let mut samples = [[0.; 2]; 16];
        assert_eq!(decoder.read(&mut samples).unwrap(), 8);
        for (frame, expected) in samples
            .iter()
            .zip([0.25, 0.5, -0.25, 0.25, 0.5, -0.25, 0.25, 0.5])
        {
            assert!((frame[0] - expected).abs() < 0.00001);
            assert_eq!(frame[0], frame[1]);
        }
        decoder.seek(5).unwrap();
        assert_eq!(decoder.read(&mut samples).unwrap(), 3);
        assert!((samples[0][0] + 0.25).abs() < 0.00001);
        assert!(decoder.seek(9).is_err());
    }
    #[test]
    fn infinite_program_keeps_bounded_data_and_random_access() {
        let mut decoder = source(
            AudioPlaylist {
                clips: vec![clip(&[8192, -8192])],
                program: Node::Repeat {
                    count: None,
                    body: vec![Node::Clip(0)],
                },
            },
            1.,
            &crysirror_tasks::Cancellation::default(),
        )
        .unwrap();
        assert_eq!(decoder.frame_count(), None);
        decoder.seek(1_000_000_001).unwrap();
        let mut samples = [[0.; 2]; 3];
        assert_eq!(decoder.read(&mut samples).unwrap(), 3);
        assert_eq!(samples.map(|s| s[0]), [-0.25, 0.25, -0.25]);
    }
}
