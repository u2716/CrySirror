//! Decode into the final shared allocation without a growing whole-track Vec.
use super::*;
use symphonia::core::{audio::SampleBuffer, errors::Error, io::MediaSourceStream};

pub(super) fn decode_static(bytes: &[u8]) -> anyhow::Result<StaticSoundData> {
    let encoded: Arc<[u8]> = Arc::from(bytes);
    let mut count = 0usize;
    let sample_rate = visit_frames(encoded.clone(), |frames| {
        count = count.checked_add(frames.len())
            .ok_or_else(|| anyhow::anyhow!("decoded audio length overflow"))?;
        Ok(())
    })?;
    anyhow::ensure!(count != 0, "audio decode produced no samples");
    count.checked_mul(std::mem::size_of::<Frame>())
        .ok_or_else(|| anyhow::anyhow!("decoded audio allocation overflow"))?;
    let mut output = Arc::<[Frame]>::new_uninit_slice(count);
    let storage = Arc::get_mut(&mut output).expect("new audio allocation is unique");
    let mut written = 0usize;
    let second_rate = visit_frames(encoded, |frames| {
        let end = written.checked_add(frames.len())
            .ok_or_else(|| anyhow::anyhow!("decoded audio length overflow"))?;
        anyhow::ensure!(end <= storage.len(), "audio length changed between decode passes");
        for (dst, frame) in storage[written..end].iter_mut().zip(frames) {
            dst.write(*frame);
        }
        written = end;
        Ok(())
    })?;
    anyhow::ensure!(written == count && sample_rate == second_rate,
                    "audio format changed between decode passes");
    // Both passes use the same immutable stream. Every Frame was initialized
    // above, and a failed or inconsistent decode drops only MaybeUninit values.
    let frames = unsafe { output.assume_init() };
    Ok(StaticSoundData { sample_rate, frames, settings: StaticSoundSettings::new(), slice: None })
}

pub(super) fn visit_frames(encoded: Arc<[u8]>, mut visit: impl FnMut(&[Frame]) -> anyhow::Result<()>)
    -> anyhow::Result<u32>
{
    let stream = MediaSourceStream::new(Box::new(Cursor::new(encoded)), Default::default());
    let mut format = symphonia::default::get_probe()
        .format(&Default::default(), stream, &Default::default(), &Default::default())?.format;
    let track = format.default_track().ok_or_else(|| anyhow::anyhow!("no default audio track"))?;
    let track_id = track.id;
    let rate = track.codec_params.sample_rate.ok_or_else(|| anyhow::anyhow!("unknown sample rate"))?;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;
    let mut frames = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(err.into()),
        };
        if packet.track_id() != track_id { continue; }
        let decoded = decoder.decode(&packet)?;
        let channels = decoded.spec().channels.count();
        anyhow::ensure!(matches!(channels, 1 | 2), "unsupported audio channel configuration");
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        frames.clear();
        frames.extend(samples.samples().chunks_exact(channels).map(|sample| {
            Frame::new(sample[0], if channels == 1 { sample[0] } else { sample[1] })
        }));
        visit(&frames)?;
    }
    Ok(rate)
}
