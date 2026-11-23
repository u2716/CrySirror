// SPDX-License-Identifier: MPL-2.0
//! ExHibit loose .v voice pages. The native mixer consumes 2048 stereo
//! frames per callback, including the zero-filled suffix of the final page.
use crate::{DecodeContext, DecodedFile, FileFormat, FileKind, Probe, WaveSpec, read_exact_at};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::service::AssetReader;
use std::sync::Arc;

pub struct ExhibitPcm;

struct Header {
    blocks: u16,
    tail: u16,
}
impl Header {
    fn read(reader: &dyn AssetReader) -> Result<Self> {
        let mut header = [0u8; 8];
        read_exact_at(reader, 0, &mut header)?;
        let blocks = u16::from_le_bytes([header[4], header[5]]);
        let tail = u16::from_le_bytes([header[6], header[7]]);
        ensure!(
            header[..4] == [0; 4] && blocks > 0 && tail <= 2048,
            "unsupported ExHibit voice header"
        );
        Ok(Self { blocks, tail })
    }
    fn pages(&self) -> usize {
        usize::from(self.blocks) - usize::from(self.tail == 0)
    }
}

// Retouch's 4-bit IMA stream carries its predictor across pages. Nibbles
// arrive high first. Only interpolated pages update the previous output.
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const ADJUST: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
#[derive(Default)]
struct Predictor {
    sample: i32,
    index: i32,
    previous: i32,
}
impl Predictor {
    fn expand(&mut self, packed: &[u8], interpolated: bool, out: &mut [u8]) -> Result<()> {
        let mut frames = out.chunks_exact_mut(4);
        for code in packed.iter().flat_map(|b| [b >> 4, b & 15]) {
            let step = STEPS[self.index as usize];
            let mut delta = step >> 3;
            if code & 4 != 0 {
                delta += step
            }
            if code & 2 != 0 {
                delta += step >> 1
            }
            if code & 1 != 0 {
                delta += step >> 2
            }
            self.index = (self.index + ADJUST[code as usize]).clamp(0, 88);
            self.sample =
                (self.sample + if code & 8 != 0 { -delta } else { delta }).clamp(-32768, 32767);
            let mut write = |value: i32| -> Result<()> {
                let f = frames.next().context("voice IMA page overflow")?;
                let [a, b] = (value as i16).to_le_bytes();
                f.copy_from_slice(&[a, b, a, b]);
                Ok(())
            };
            if interpolated {
                write((self.previous + self.sample) >> 1)?;
                self.previous = self.sample;
            }
            write(self.sample)?;
        }
        Ok(())
    }
}

fn fill_page(
    predictor: &mut Predictor,
    tag: u8,
    input: &[u8],
    last: bool,
    out: &mut [u8],
) -> Result<()> {
    let mut frames = out.chunks_exact_mut(4);
    let mut previous = None;
    match tag {
        0 => {
            out.fill(0);
            *predictor = Predictor::default();
        }
        1 | 7 => predictor.expand(input, tag == 1, out)?,
        2 => out
            .get_mut(..input.len())
            .context("voice stereo page overflow")?
            .copy_from_slice(input),
        4 | 6 => {
            for bytes in input.chunks_exact(2) {
                let sample = i16::from_le_bytes([bytes[0], bytes[1]]);
                let mut stereo = |s: i16| -> Result<()> {
                    let f = frames.next().context("voice mono page overflow")?;
                    let s = s.to_le_bytes();
                    f.copy_from_slice(&[s[0], s[1], s[0], s[1]]);
                    Ok(())
                };
                if tag == 4 {
                    // Full pages round the midpoint towards the current sample.
                    // The native tail decoder simply duplicates each sample.
                    let current = i32::from(sample);
                    let mid = if last {
                        current
                    } else {
                        current - (current - previous.unwrap_or(current)) / 2
                    };
                    stereo(mid as i16)?;
                    previous = Some(current);
                }
                stereo(sample)?;
            }
        }
        _ => bail!("unsupported native voice page {tag}"),
    }
    Ok(())
}

impl FileFormat for ExhibitPcm {
    fn id(&self) -> &'static str {
        "exhibit/pcm-voice"
    }
    fn kind(&self) -> FileKind {
        FileKind::Audio
    }
    fn probe(&self, path: &str, source: &dyn AssetReader) -> Result<Probe> {
        if !path
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("v"))
            || source.len()? < 8
        {
            return Ok(Probe::NO);
        }
        Ok(if Header::read(source).is_ok() {
            Probe::YES
        } else {
            Probe::NO
        })
    }
    fn decode(
        &self,
        path: &str,
        source: Arc<dyn AssetReader>,
        cx: &DecodeContext,
    ) -> Result<DecodedFile> {
        ensure!(
            self.probe(path, &*source)?.confidence != 0,
            "invalid ExHibit voice file"
        );
        let h = Header::read(&*source)?;
        let size = h.pages() * 8192;
        let wave = WaveSpec {
            channels: 2,
            ..WaveSpec::default()
        };
        let bytes = wave.encoded_len(size)?;
        let source_len = source.len()?;
        let reader = cx.decode_with(bytes as u64, || {
            cx.with_temporary_bytes(size as u64 + 8192, || {
                let mut samples = vec![0; size];
                let mut at = 8u64;
                let mut predictor = Predictor::default();
                let mut packed = [0u8; 8192];
                for (i, page) in samples.chunks_exact_mut(8192).enumerate() {
                    cx.check_cancelled()?;
                    let mut t = [0; 1];
                    read_exact_at(&*source, at, &mut t)?;
                    at += 1;
                    let tail = i + 1 == usize::from(h.blocks);
                    let n = if tail {
                        usize::from(h.tail)
                    } else {
                        match t[0] {
                            1 | 4 => 1024,
                            _ => 2048,
                        }
                    };
                    let count = match t[0] {
                        0 => 0,
                        1 if n <= 1024 => n / 2,
                        7 if !tail => 1024,
                        2 => n * 4,
                        4 if n <= 1024 => n * 2,
                        6 => n * 2,
                        tag => bail!("unsupported ExHibit voice page {tag} at {i}"),
                    };
                    read_exact_at(&*source, at, &mut packed[..count])?;
                    at += count as u64;
                    fill_page(&mut predictor, t[0], &packed[..count], tail, page)?;
                }
                ensure!(at == source_len, "voice data after the declared pages");
                wave.encode(&samples)
            })
        })?;
        Ok(DecodedFile {
            reader,
            extension: "wav".into(),
            metadata: serde_json::json!({"sample_rate":44100,"channels":2,"frames":size/4}),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::state::ClientId;
    #[test]
    fn adpcm_keeps_the_predictor_across_pages_and_silence_resets_it() -> Result<()> {
        let mut predictor = Predictor::default();
        let mut frames = [0; 16];
        fill_page(&mut predictor, 1, &[0x71], false, &mut frames)?;
        let words: Vec<i16> = frames
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect();
        assert_eq!(words, [5, 5, 11, 11, 14, 14, 17, 17]);
        fill_page(&mut predictor, 7, &[0x00], false, &mut frames)?;
        assert_ne!(predictor.sample, 0);
        fill_page(&mut predictor, 0, &[], false, &mut frames)?;
        assert_eq!(
            (predictor.sample, predictor.index, predictor.previous),
            (0, 0, 0)
        );
        Ok(())
    }

    #[test]
    fn interpolates_full_pages_and_zero_pads_native_tail() -> Result<()> {
        let cx = DecodeContext::new(ClientId(1));
        let mut bytes = vec![0, 0, 0, 0, 2, 0, 2, 0, 4];
        for sample in [9i16, -4, 3]
            .into_iter()
            .chain(std::iter::repeat_n(0, 1021))
        {
            bytes.extend(sample.to_le_bytes());
        }
        bytes.push(4);
        bytes.extend(7i16.to_le_bytes());
        bytes.extend((-2i16).to_le_bytes());
        let data = ExhibitPcm.decode("v/1.v", cx.memory(bytes.clone())?, &cx)?;
        let wave = crate::read_all(&*data.reader, 20000)?;
        let at = wave.windows(4).position(|w| w == b"data").unwrap() + 8;
        let samples: Vec<i16> = wave[at..]
            .chunks_exact(2)
            .map(|w| i16::from_le_bytes([w[0], w[1]]))
            .collect();
        assert_eq!(&samples[..12], &[9, 9, 9, 9, 2, 2, -4, -4, 0, 0, 3, 3]);
        assert_eq!(&samples[4096..4104], &[7, 7, 7, 7, -2, -2, -2, -2]);
        assert!(samples[4104..].iter().all(|v| *v == 0));
        bytes.pop();
        assert!(ExhibitPcm.decode("v/1.v", cx.memory(bytes)?, &cx).is_err());
        Ok(())
    }
}
