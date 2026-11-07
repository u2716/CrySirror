// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original 437F10/438630/439110 single-instance sound policy. The shared
//! engine owns decoding, playback, pause and retirement, never a native device.
use super::*;
use crysirror_core::service::AudioRequest;

// Native globals571550/571548/57154C are distinct music/voice/sound instances.
// These logical channels are client scoped; they are never original pointers.
pub(super) const MUSIC: u32 = 0;
pub(super) const VOICE: u32 = 1;
pub(super) const SOUND: u32 = 2;
pub(super) struct NativeAudio {
    pub instance_gain: [i32; 3],
    pub music_names: [Vec<u8>; 2],
}
impl Default for NativeAudio {
    fn default() -> Self {
        Self {
            instance_gain: [1000; 3],
            music_names: Default::default(),
        }
    }
}
impl MajiroVm {
    pub(super) fn audio_control(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(arguments.is_empty(), "Majiro audio control ABI");
        let request = match hash {
            0x0c070535 => AudioRequest::Stop {
                channel: MUSIC,
                fade_ms: 0.0,
            },
            0x05d6ed69 => AudioRequest::Stop {
                channel: VOICE,
                fade_ms: 0.0,
            },
            0x83a53ffa => AudioRequest::Stop {
                channel: SOUND,
                fade_ms: 0.0,
            },
            //44D640 is idempotent pause, not a toggle. 44D6D0 requests resume.
            0x2626fe02 => AudioRequest::Pause {
                channel: MUSIC,
                paused: true,
                fade_ms: 0.0,
            },
            0xd011ff32 => AudioRequest::Pause {
                channel: MUSIC,
                paused: false,
                fade_ms: 0.0,
            },
            //438150/44D520 returns -1 during decoder preparation,1 during
            //source playback and0 when absent/stopped. Preparation is tracked
            //separately when the native play syscall is admitted.
            0xf8d8925b => {
                let playing = host
                    .audio_info(self.client, MUSIC)
                    .map_err(core_error)?
                    .is_some_and(|info| info.source_playing);
                return Ok(Some(if returns {
                    vec![Value::Integer(i32::from(playing))]
                } else {
                    vec![]
                }));
            }
            _ => bail!("Majiro unreviewed audio control"),
        };
        host.submit_audio(self.client, request)
            .map_err(core_error)?;
        self.api_calls += 1;
        let stopped = match hash {
            0x0c070535 => Some(MUSIC),
            0x05d6ed69 => Some(VOICE),
            0x83a53ffa => Some(SOUND),
            _ => None,
        };
        if let Some(channel) = stopped {
            self.audio.instance_gain[channel as usize] = 1000;
            if channel == MUSIC {
                self.audio.music_names.iter_mut().for_each(Vec::clear);
            }
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
}
