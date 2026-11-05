//! Movie soundtracks share the video's transport, outside the ordinary PAL lanes.
use crate::native_vm::audio::{decode_movie_audio, AudioSystem};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::Tween;
use std::time::Duration;

pub(super) struct MovieAudio {
    data: Option<StaticSoundData>,
    handle: Option<StaticSoundHandle>,
    gain_raw: i32,
    generation: u64,
    submitted: bool,
}

fn immediate() -> Tween {
    Tween { duration: Duration::ZERO, ..Tween::default() }
}

/// PAL truncates DirectSound's hundredths-of-a-decibel value before submission.
pub(super) fn native_decibels(raw: i32) -> f32 {
    if raw <= 0 { -100.0 } else { (2000.0 * (raw as f32 / 10000.0).log10()).trunc() / 100.0 }
}

impl MovieAudio {
    pub(super) fn decode(bytes: &[u8], gain_raw: i32) -> Self {
        let data = match decode_movie_audio(bytes) {
            Ok(data) => data,
            Err(error) => {
                log::warn!("movie soundtrack could not be decoded: {error}");
                None
            }
        };
        Self::new(data, gain_raw)
    }

    pub(super) fn new(data: Option<StaticSoundData>, gain_raw: i32) -> Self {
        Self { data, handle: None, gain_raw, generation: 0, submitted: false }
    }

    pub(super) fn set_gain(&mut self, gain_raw: i32) -> bool {
        self.gain_raw = gain_raw;
        if let Some(handle) = &mut self.handle {
            handle.set_volume(native_decibels(gain_raw), immediate());
        }
        self.data.is_some()
    }

    #[cfg(test)]
    pub(super) fn gain(&self) -> i32 { self.gain_raw }

    pub(super) fn stop(&mut self) {
        if let Some(mut handle) = self.handle.take() { handle.stop(immediate()); }
        self.submitted = false;
    }

    pub(super) fn sync(&mut self, audio: &mut AudioSystem, playing: bool, position_ms: u32, generation: u64) {
        if !playing { self.stop(); return; }
        if self.data.is_none() { return; }
        if !audio.is_enabled() { return; }
        // Never put a loop region on the initial sound. Native starts at zero
        // and only applies stored loop points when the first graph reaches EOF.
        if generation != self.generation {
            self.stop();
        }
        self.generation = generation;
        if self.submitted { return; }
        let data = self.data.as_ref().unwrap();
        if f64::from(position_ms) / 1000.0 >= data.duration().as_secs_f64() { return; }
        match audio.play_movie_sound(data.start_position(f64::from(position_ms) / 1000.0)
            .volume(native_decibels(self.gain_raw))) {
            Ok(handle) => { self.handle = Some(handle); self.submitted = true; }
            Err(error) => log::warn!("movie soundtrack could not start: {error}"),
        }
    }

}

impl Drop for MovieAudio {
    fn drop(&mut self) { self.stop(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_uses_native_integer_directsound_decibels() {
        assert_eq!(native_decibels(0), -100.0);
        assert_eq!(native_decibels(10000), 0.0);
        assert_eq!(native_decibels(5000), -6.02);
        assert_eq!(native_decibels(2500), -12.04);
    }

    #[cfg(feature = "host-audio")]
    #[test]
    fn transport_starts_at_video_position_restarts_and_stops_on_drop() {
        use crate::native_vm::audio::AudioConfig;
        use kira::{Frame, sound::static_sound::StaticSoundSettings};
        let data = StaticSoundData { sample_rate: 48000,
            frames: (0..48000).map(|i| Frame::from_mono(if i < 24000 { 0.25 } else { -0.25 })).collect::<Vec<_>>().into(),
            settings: StaticSoundSettings::default(), slice: None };
        let mut audio = AudioSystem::new(AudioConfig { enabled: true }).unwrap();
        let sink = audio.host_sink();
        let mut track = MovieAudio::new(Some(data), 10000);
        let mut output = [0; 1024];
        track.sync(&mut audio, true, 600, 0);
        sink.render_i16(&mut output);
        assert!(output.iter().any(|&x| x < -1000));
        track.sync(&mut audio, true, 0, 1);
        sink.render_i16(&mut output);
        assert!(output.iter().any(|&x| x > 1000));
        track.sync(&mut audio, false, 20, 1);
        sink.render_i16(&mut output);
        assert!(output.iter().all(|&x| x == 0));
        track.sync(&mut audio, true, 20, 1);
        sink.render_i16(&mut output);
        assert!(output.iter().any(|&x| x > 1000));
        drop(track);
        sink.render_i16(&mut output);
        assert!(output.iter().all(|&x| x == 0));
    }
}
