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
    /// YUKIIRO59EA68 / classic56E9F8: one-shot pan for the next ordinary
    /// voice playback attempt. Active-instance pan changes do not replace it.
    pub pending_voice_pan: i32,
}
impl Default for NativeAudio {
    fn default() -> Self {
        Self {
            instance_gain: [1000; 3],
            music_names: Default::default(),
            pending_voice_pan: 0,
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
            // Shared source-clock observation; a completed first pass is
            // distinct from both decoder preparation and the loop setting.
            0xf8d8925b => {
                let state = self.native_audio_state(host, MUSIC)?;
                return Ok(Some(if returns {
                    vec![Value::Integer(state)]
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
            self.audio_names.remove(&channel);
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
