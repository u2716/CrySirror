//! Actual531170/531820/5321A0 skip policy, separate from decoder/mixer work.
//! Later public callback IDs differ: legacy08 is voice and09 is sound.
use super::*;
use crate::native_skip::NativeSkipPolicy;
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioCallbackContext {
    pub auto_message: bool,
    /// Raw Ctrl/Shift latch before current-message read filtering.
    pub message_skipping: bool,
    /// Actual531820 read-policy result for Ctrl voice suppression.
    pub suppress_voice: bool,
    pub must_set_music: bool,
}
pub(crate) struct Admission {
    pub plan: NativeAudioPlan,
    pub stop_before: Option<u32>,
    pub music_mark: Option<i32>,
}
impl NativeAudioPolicy {
    pub(crate) fn admit_with_skip(
        &self,
        call: &NativeCall,
        music: &Names,
        sound: &Names,
        playing: bool,
        skip: &NativeSkipPolicy,
        context: AudioCallbackContext,
    ) -> Result<Admission> {
        ensure!(call.identify == 0x66660007, "NNN native audio identify");
        let mut seed = self.clone();
        let mut stop_before = None;
        let mut music_mark = None;
        match (&call.payload, call.code) {
            (Payload::Words { count, words }, 0x33330007) => {
                ensure!(
                    *count as usize == words.len() && (1..=256).contains(&words.len()),
                    "NNN music argument count"
                );
                let number = words[0];
                let suppressed = skip.next_command && skip.effect_command;
                if number > 0 && (!suppressed || context.must_set_music) {
                    music_mark = Some(number - 1);
                }
                if suppressed {
                    if number > 0 {
                        seed.last_music_number = Some(number - 1);
                        seed.music_repeat_count = words.get(1).copied().unwrap_or(0);
                    }
                    return Ok(Admission {
                        plan: NativeAudioPlan {
                            operation: AudioOperation::None,
                            source: None,
                            next: seed,
                        },
                        stop_before: None,
                        music_mark,
                    });
                }
            }
            (Payload::Words { count, words }, 0x33330009) => {
                ensure!(
                    *count as usize == words.len() && (2..=256).contains(&words.len()),
                    "NNN sound argument count"
                );
                let channel = usize::try_from(words[1]).context("NNN sound channel")?;
                ensure!(channel < 8, "NNN sound channel bounds");
                if words[0] != -1
                    && (skip.next_command || context.message_skipping)
                    && !context.auto_message
                {
                    let operation = if seed.sound_work[channel][0] != 0 {
                        seed.sound_work[channel][0] = 0;
                        AudioOperation::Stop {
                            channel: self.bindings.sound_base + channel as u32,
                            fade: None,
                        }
                    } else {
                        AudioOperation::None
                    };
                    return Ok(Admission {
                        plan: NativeAudioPlan {
                            operation,
                            source: None,
                            next: seed,
                        },
                        stop_before: None,
                        music_mark: None,
                    });
                }
            }
            (Payload::Voice { parameters, .. }, 0x33330008) => {
                ensure!(
                    (2..=256).contains(&parameters.len()),
                    "NNN voice argument count"
                );
                let channel = usize::try_from(parameters[1]).context("NNN voice channel")?;
                ensure!(channel < 8, "NNN voice channel bounds");
                if (skip.next_command || context.message_skipping)
                    && !context.auto_message
                    && seed.voice_work[channel][0] != 0
                {
                    seed.voice_work[channel][0] = 0;
                    stop_before = Some(self.bindings.voice_base + channel as u32);
                }
                if skip.next_command
                    || (context.message_skipping && !context.auto_message && context.suppress_voice)
                {
                    //No new name/work/duration writes: only a stopped old loop.
                    return Ok(Admission {
                        plan: NativeAudioPlan {
                            operation: AudioOperation::None,
                            source: None,
                            next: seed,
                        },
                        stop_before,
                        music_mark: None,
                    });
                }
            }
            _ => {}
        }
        Ok(Admission {
            plan: seed.plan(call, music, sound, playing)?,
            stop_before,
            music_mark,
        })
    }
}
