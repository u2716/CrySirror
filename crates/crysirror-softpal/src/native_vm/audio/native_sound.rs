//! PAL fades amplitude before conversion to decibels. Kira's built-in fades
//! interpolate decibels, so ordinary PAL sounds need their own envelope.
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::{PlaybackState, Sound, SoundData};
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::{FromFileError, streaming::StreamingSoundHandle};
use kira::{Frame, Tween, info::Info};
use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU32, Ordering, fence},
};

const RUN: u8 = 0;
const FADE_IN: u8 = 1;
const PAUSING: u8 = 2;
const STOPPING: u8 = 3;
const PAUSED: u8 = 4;
const STOPPED: u8 = 5;
const WAITING_TO_RESUME: u8 = 6;

struct Shared {
    sequence: AtomicU32,
    command_kind: AtomicU8,
    duration_ms: AtomicU32,
    state: AtomicU8,
    worker_command: AtomicU32,
    natural_end: AtomicU8,
    clock_ms: Option<Arc<AtomicU32>>,
    command_start_ms: AtomicU32,
}

pub(super) struct NativeSoundData<D = StaticSoundData> {
    pub(super) data: D,
    pub(super) fade_ms: u32,
    pub(super) clock_ms: Option<Arc<AtomicU32>>,
}

pub(super) struct NativeSoundHandle {
    inner: InnerHandle,
    shared: Arc<Shared>,
}

pub(super) enum InnerHandle {
    Static(StaticSoundHandle),
    #[cfg(not(target_arch = "wasm32"))]
    Stream(StreamingSoundHandle<FromFileError>),
}
impl From<StaticSoundHandle> for InnerHandle {
    fn from(handle: StaticSoundHandle) -> Self { Self::Static(handle) }
}
#[cfg(not(target_arch = "wasm32"))]
impl From<StreamingSoundHandle<FromFileError>> for InnerHandle {
    fn from(handle: StreamingSoundHandle<FromFileError>) -> Self { Self::Stream(handle) }
}
macro_rules! forward_control {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match &$self.inner {
            InnerHandle::Static(handle) => handle.$method($($arg),*),
            #[cfg(not(target_arch = "wasm32"))]
            InnerHandle::Stream(handle) => handle.$method($($arg),*),
        }
    };
}
macro_rules! forward_control_mut {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match &mut $self.inner {
            InnerHandle::Static(handle) => handle.$method($($arg),*),
            #[cfg(not(target_arch = "wasm32"))]
            InnerHandle::Stream(handle) => handle.$method($($arg),*),
        }
    };
}

impl NativeSoundHandle {
    fn command(&self, kind: u8, duration_ms: u32) {
        // A single writer publishes a coherent command with 32-bit atomics,
        // including on Vita targets without lock-free 64-bit atomics.
        self.shared.sequence.fetch_add(1, Ordering::AcqRel);
        self.shared.command_kind.store(kind, Ordering::Relaxed);
        self.shared
            .duration_ms
            .store(duration_ms, Ordering::Relaxed);
        let start = self.shared.clock_ms.as_ref().map_or(0, |clock| clock.load(Ordering::Acquire));
        self.shared.command_start_ms.store(start, Ordering::Relaxed);
        let native_command = match kind {
            PAUSING => 9,
            FADE_IN => 10,
            STOPPING if duration_ms != 0 => 7,
            STOPPING => 3,
            _ => 5,
        };
        self.shared.worker_command.store(native_command, Ordering::Release);
        self.shared.sequence.fetch_add(1, Ordering::Release);
    }
    pub(super) fn report_errors(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let InnerHandle::Stream(handle) = &mut self.inner {
            while let Some(error) = handle.pop_error() {
                log::error!("streaming audio decode failed: {error}");
            }
        }
    }
    pub(super) fn state(&self) -> PlaybackState {
        if forward_control!(self, state) == PlaybackState::Stopped {
            return PlaybackState::Stopped;
        }
        match self.shared.state.load(Ordering::Acquire) {
            FADE_IN => PlaybackState::Resuming,
            PAUSING => PlaybackState::Pausing,
            STOPPING => PlaybackState::Stopping,
            PAUSED => PlaybackState::Paused,
            WAITING_TO_RESUME => PlaybackState::WaitingToResume,
            STOPPED => PlaybackState::Stopped,
            _ => PlaybackState::Playing,
        }
    }
    pub(super) fn fade_state(&self) -> i32 {
        match self.shared.state.load(Ordering::Acquire) {
            FADE_IN | WAITING_TO_RESUME => 1,
            PAUSING | STOPPING => 2,
            _ => 0,
        }
    }
    pub(super) fn fade_duration(&self) -> u32 {
        self.shared.duration_ms.load(Ordering::Acquire)
    }
    pub(super) fn worker_command(&self) -> i32 {
        self.shared.worker_command.load(Ordering::Acquire) as i32
    }
    pub(super) fn set_worker_command(&self, command: i32) {
        self.shared.worker_command.store(command as u32, Ordering::Release);
    }
    pub(super) fn ended_naturally(&self) -> bool {
        self.shared.natural_end.load(Ordering::Acquire) != 0
    }
    pub(super) fn position(&self) -> f64 {
        forward_control!(self, position)
    }
    pub(super) fn set_loop_region(&mut self, region: Option<kira::sound::Region>) {
        forward_control_mut!(self, set_loop_region, region);
    }
    pub(super) fn set_volume(&mut self, value: f32, tween: Tween) {
        forward_control_mut!(self, set_volume, value, tween);
    }
    pub(super) fn set_panning(&mut self, value: f32, tween: Tween) {
        forward_control_mut!(self, set_panning, value, tween);
    }
    pub(super) fn set_playback_rate(&mut self, value: f64, tween: Tween) {
        forward_control_mut!(self, set_playback_rate, value, tween);
    }
    pub(super) fn pause(&mut self, tween: Tween) {
        self.command(
            PAUSING,
            tween.duration.as_millis().min(u32::MAX as u128) as u32,
        );
    }
    pub(super) fn resume(&mut self, tween: Tween) {
        self.command(
            FADE_IN,
            tween.duration.as_millis().min(u32::MAX as u128) as u32,
        );
    }
    pub(super) fn stop(&mut self, tween: Tween) {
        let duration = tween.duration.as_millis().min(u32::MAX as u128) as u32;
        if duration != 0 && self.shared.state.load(Ordering::Acquire) == STOPPING {
            return;
        }
        self.command(STOPPING, duration);
    }
}

impl Drop for NativeSoundHandle {
    fn drop(&mut self) {
        self.command(STOPPING, 0);
    }
}

struct NativeSound {
    inner: Box<dyn Sound>,
    shared: Arc<Shared>,
    command: u32,
    state: u8,
    elapsed: f64,
    duration: f64,
    silent: bool,
    start_ms: u32,
}

impl<D: SoundData> SoundData for NativeSoundData<D>
where D::Handle: Into<InnerHandle> {
    type Error = D::Error;
    type Handle = NativeSoundHandle;
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (inner, handle) = self.data.into_sound()?;
        let state = if self.fade_ms == 0 { RUN } else { FADE_IN };
        let start_ms = self.clock_ms.as_ref().map_or(0, |clock| clock.load(Ordering::Acquire));
        let shared = Arc::new(Shared {
            sequence: AtomicU32::new(0),
            command_kind: AtomicU8::new(RUN),
            duration_ms: AtomicU32::new(self.fade_ms),
            state: AtomicU8::new(state),
            // Native PlayFade waits for command 6 to be acknowledged before
            // returning, so callers observe the stable command 5 here.
            worker_command: AtomicU32::new(5),
            natural_end: AtomicU8::new(0),
            clock_ms: self.clock_ms,
            command_start_ms: AtomicU32::new(start_ms),
        });
        let sound = NativeSound {
            inner,
            shared: shared.clone(),
            command: 0,
            state,
            elapsed: 0.0,
            duration: f64::from(self.fade_ms) / 1000.0,
            silent: false,
            start_ms,
        };
        Ok((
            Box::new(sound),
            NativeSoundHandle {
                inner: handle.into(),
                shared,
            },
        ))
    }
}

impl Sound for NativeSound {
    fn on_start_processing(&mut self) {
        self.inner.on_start_processing();
        let sequence = self.shared.sequence.load(Ordering::Acquire);
        if sequence != self.command && sequence & 1 == 0 && self.state != STOPPED {
            let kind = self.shared.command_kind.load(Ordering::Relaxed);
            let duration_ms = self.shared.duration_ms.load(Ordering::Relaxed);
            let start_ms = self.shared.command_start_ms.load(Ordering::Relaxed);
            fence(Ordering::Acquire);
            if sequence != self.shared.sequence.load(Ordering::Relaxed) {
                return;
            }
            self.command = sequence;
            self.silent = self.state == PAUSED || self.silent;
            self.state = if kind == FADE_IN && self.silent {
                WAITING_TO_RESUME
            } else {
                kind
            };
            self.duration = f64::from(duration_ms) / 1000.0;
            self.elapsed = 0.0;
            self.start_ms = start_ms;
            if kind == STOPPING {
                self.shared.worker_command.store(3, Ordering::Release);
            }
            if self.duration == 0.0 {
                self.complete();
            }
            self.shared.state.store(self.state, Ordering::Release);
        }
    }
    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        if let Some(clock) = &self.shared.clock_ms {
            let elapsed = clock.load(Ordering::Acquire).wrapping_sub(self.start_ms);
            self.process_clocked(out, dt, info, elapsed);
            return;
        }
        if self.state == PAUSED || self.state == STOPPED {
            out.fill(Frame::ZERO);
            return;
        }
        let fading = matches!(self.state, FADE_IN | PAUSING | STOPPING | WAITING_TO_RESUME);
        let count = if fading && self.state != FADE_IN {
            (((self.duration - self.elapsed) / dt).ceil().max(0.0) as usize).min(out.len())
        } else {
            out.len()
        };
        if self.silent {
            out[..count].fill(Frame::ZERO);
        } else {
            self.inner.process(&mut out[..count], dt, info);
        }
        out[count..].fill(Frame::ZERO);
        if fading {
            for frame in &mut out[..count] {
                self.elapsed += dt;
                let fraction = (self.elapsed / self.duration).clamp(0.0, 1.0) as f32;
                let gain = if self.state == FADE_IN {
                    fraction
                } else {
                    1.0 - fraction
                };
                *frame *= gain;
            }
            if self.elapsed + 1e-12 >= self.duration {
                self.complete();
                self.shared.state.store(self.state, Ordering::Release);
            }
        }
        if self.inner.finished() {
            self.shared.natural_end.store(1, Ordering::Release);
            self.shared.state.store(STOPPED, Ordering::Release);
        }
    }
    fn finished(&self) -> bool {
        self.state == STOPPED || self.inner.finished()
    }
}

impl NativeSound {
    fn process_clocked(&mut self, out: &mut [Frame], dt: f64, info: &Info, elapsed_ms: u32) {
        let fading = matches!(self.state, FADE_IN | PAUSING | STOPPING | WAITING_TO_RESUME);
        let duration_ms = (self.duration * 1000.0).round() as u32;
        if fading && elapsed_ms >= duration_ms {
            self.complete();
            self.shared.state.store(self.state, Ordering::Release);
        }
        if self.state == PAUSED || self.state == STOPPED {
            out.fill(Frame::ZERO);
            return;
        }
        if self.silent { out.fill(Frame::ZERO); }
        else { self.inner.process(out, dt, info); }
        if fading && elapsed_ms < duration_ms {
            // PAL evaluates one float32 amplitude for each worker tick from
            // its shared millisecond clock, independently of the audio cursor.
            let fraction = elapsed_ms as f32 / duration_ms as f32;
            let gain = if self.state == FADE_IN { fraction } else { 1.0 - fraction };
            for frame in out { *frame *= gain; }
        }
        if self.inner.finished() {
            self.shared.natural_end.store(1, Ordering::Release);
            self.shared.state.store(STOPPED, Ordering::Release);
        }
    }

    fn complete(&mut self) {
        // Completion acknowledges pause/resume/stop only if a newer producer
        // command has not replaced the one this audio block is processing.
        if self.shared.sequence.load(Ordering::Acquire) == self.command {
            self.shared.worker_command.store(5, Ordering::Release);
        }
        self.state = match self.state {
            PAUSING => PAUSED,
            STOPPING => STOPPED,
            _ => RUN,
        };
        self.silent = self.state == PAUSED || self.state == STOPPED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kira::sound::static_sound::StaticSoundSettings;
    fn sound(fade_ms: u32) -> NativeSoundData {
        NativeSoundData {
            data: StaticSoundData {
                sample_rate: 1000,
                frames: vec![Frame::new(1.0, 1.0); 5000].into(),
                settings: StaticSoundSettings::default(),
                slice: None,
            },
            fade_ms,
            clock_ms: None,
        }
    }
    #[test]
    fn fade_is_linear_amplitude_and_delayed_resume_retains_position() {
        let (mut sound, mut handle) = sound(1000).into_sound().unwrap();
        let info = kira::info::MockInfoBuilder::new();
        let info = info.build();
        sound.on_start_processing();
        let mut out = vec![Frame::ZERO; 500];
        sound.process(&mut out, 0.001, &info);
        assert!((out[499].left - 0.5).abs() < 0.002);
        assert_eq!(handle.fade_state(), 1);
        sound.process(&mut out, 0.001, &info);
        assert!((out[499].left - 1.0).abs() < 0.002);
        assert_eq!(handle.fade_state(), 0);
        let tween = Tween {
            duration: std::time::Duration::from_millis(1000),
            ..Tween::default()
        };
        handle.pause(tween);
        sound.on_start_processing();
        sound.process(&mut out, 0.001, &info);
        assert!((out[499].left - 0.5).abs() < 0.002);
        sound.process(&mut out, 0.001, &info);
        assert_eq!(handle.state(), PlaybackState::Paused);
        sound.on_start_processing();
        let position = handle.position();
        sound.process(&mut out, 0.001, &info);
        assert!(out.iter().all(|frame| frame.left == 0.0));
        assert_eq!(handle.position(), position);
        handle.resume(tween);
        sound.on_start_processing();
        assert_eq!(handle.state(), PlaybackState::WaitingToResume);
        sound.process(&mut out, 0.001, &info);
        assert!(out.iter().all(|frame| frame.left == 0.0));
        assert_eq!(handle.position(), position);
        sound.process(&mut out, 0.001, &info);
        sound.process(&mut out, 0.001, &info);
        assert!((out[499].left - 1.0).abs() < 0.002);
        handle.stop(Tween {
            duration: std::time::Duration::ZERO,
            ..tween
        });
        sound.on_start_processing();
        assert!(sound.finished());
    }
}
