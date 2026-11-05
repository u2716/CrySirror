//! Newer SE wrappers: EXE 0x461970 and 0x461F90..0x462ED0.
//! See reverse/evidence/totsulover-se-playback.json and se-prefix-volume.json.
use super::*;

const SE_GROUPS: [PalSoundGroup; 3] = [
    PalSoundGroup::GROUP5,
    PalSoundGroup::GROUP4,
    PalSoundGroup::GROUP6,
];
const PRELOADED: i32 = 0x2000_0000;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SeChannelControl {
    volume: Option<i32>,
    frequency: Option<i32>,
}

impl SeChannelControl {
    pub(super) fn load_gain(self) -> f32 {
        self.volume
            .map(|volume| volume as u32 as f32 / 100.0)
            .unwrap_or(0.0)
    }
}

impl ScriptRuntime {
    fn apply_registered_se_channel(&self, slot: i32, handle: AudioHandle, audio: &mut AudioSystem) {
        let control = self.registered_se_controls[slot as usize];
        if let Some(volume) = control.volume {
            // This path clamps as unsigned, unlike the prefix consumer.
            let raw = (volume.wrapping_mul(100) as u32).min(10_000) as i32;
            let _ = audio.set_channel_volume(handle, PalVolume::from_raw(raw));
        }
        if let Some(frequency) = control.frequency {
            let _ = audio.set_channel_frequency(handle, frequency);
        }
    }

    fn consume_registered_se_pan(
        &mut self,
        flags: i32,
        handle: AudioHandle,
        audio: &mut AudioSystem,
    ) {
        if flags & 0x20 != 0 {
            let _ = audio.set_channel_pan(handle, self.registered_se_pan);
            self.registered_se_pan = 0;
        }
    }
    pub(super) fn registered_se_prefix_volume(&self, name: &str) -> Option<i32> {
        // Native strncmp is case-sensitive and every match overwrites the
        // previous match. The enable word is not consulted by this consumer.
        self.se_prefix_info.iter().rev().find_map(|record| {
            (record.initialized && name.starts_with(&record.prefix))
                .then(|| record.volume.wrapping_mul(100).min(10_000))
        })
    }

    pub(crate) fn apply_registered_se_levels(&self, audio: &mut AudioSystem) {
        for (lane, group) in SE_GROUPS.into_iter().enumerate() {
            let lane = lane as i32;
            let percent = self.se_volume_percent.get(&lane).copied().unwrap_or(100);
            let enabled = self.se_enabled.get(&lane).copied().unwrap_or(true);
            let raw = if enabled {
                percent.wrapping_mul(100)
            } else {
                0
            };
            let _ = audio.set_group_volume(group, PalVolume::from_raw(raw));
        }
    }

    pub(super) fn dispatch_registered_se(
        &mut self,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
        resources: Option<&mut ResourceManager>,
        mut audio: Option<&mut AudioSystem>,
        input: Option<&PalInputState>,
    ) -> ExtCallOutcome {
        if matches!(index, 1 | 2 | 7 | 17) && self.pending_registered_se_wait.is_some() {
            return self.poll_registered_se_wait(audio, input);
        }
        let count = match index {
            0 | 1 | 15 | 18 => 3,
            2 => 5,
            3 | 4 | 14 => 2,
            5..=7 | 13 | 16 => 1,
            17 => 7,
            _ => return ExtCallOutcome::Skip,
        };
        let args = self.pop_ext_args(count);
        if args.len() != count {
            return ExtCallOutcome::Block;
        }
        match index {
            0 | 2 | 17 => {
                let (slot, name_value, flags, fade, lane) = if index == 0 {
                    (args[1], args[0], PRELOADED, 0, args[2])
                } else {
                    let slot = if args[0] == -1 {
                        self.next_free_audio_slot(5, 16)
                    } else {
                        args[0]
                    };
                    (slot, args[1], args[2], args[3], args[4])
                };
                if !(0..16).contains(&slot) {
                    return ExtCallOutcome::Continue;
                }
                let Some(&group) = SE_GROUPS.get(lane as u32 as usize) else {
                    return ExtCallOutcome::Continue;
                };
                if let (Some(handle), Some(audio)) =
                    (self.game_audio.remove(&(5, slot)), audio.as_deref_mut())
                {
                    let _ = audio.release(handle);
                }
                self.registered_se_flags[slot as usize] = flags;
                let skip = self.dispatch_skip_state();
                if index != 0 && flags & 9 == 0 && skip & 5 != 0 {
                    return ExtCallOutcome::Value(slot);
                }
                if index == 17 {
                    self.registered_se_controls[slot as usize] = SeChannelControl {
                        volume: Some(args[5]),
                        frequency: Some(args[6]),
                    };
                }
                if let Some(name) = self.resolve_resource_string(name_value, assets, nls) {
                    let volume = self.registered_se_prefix_volume(&name);
                    let loaded = self.load_named_audio(
                        5,
                        slot,
                        group,
                        &name,
                        flags,
                        100,
                        false,
                        None,
                        resources,
                        audio.as_deref_mut(),
                    );
                    if loaded {
                        let audio = audio.as_deref_mut().unwrap();
                        let handle = self.game_audio[&(5, slot)];
                        if let Some(raw) = volume {
                            let _ = audio.set_channel_volume(handle, PalVolume::from_raw(raw));
                        } else {
                            self.apply_registered_se_channel(slot, handle, audio);
                        }
                        if index != 0 {
                            self.consume_registered_se_pan(flags, handle, audio);
                            let _ = audio.play_fade(
                                handle,
                                flags & 1 != 0,
                                if flags & 4 != 0 { fade } else { 0 },
                            );
                        }
                    }
                }
                if index != 0 {
                    if flags & 2 != 0 && skip & 0x30 == 0 {
                        return self.begin_registered_se_wait(slot, slot);
                    }
                    ExtCallOutcome::Value(slot)
                } else {
                    ExtCallOutcome::Continue
                }
            }
            1 => {
                let (slot, flags, fade) = (args[0], args[1], args[2]);
                if !(0..16).contains(&slot) {
                    return ExtCallOutcome::Continue;
                }
                self.registered_se_flags[slot as usize] =
                    flags | (self.registered_se_flags[slot as usize] & !0xffff);
                if flags & 9 == 0 && self.dispatch_skip_state() & 5 != 0 {
                    return ExtCallOutcome::Continue;
                }
                if let (Some(audio), Some(&handle)) = (audio, self.game_audio.get(&(5, slot))) {
                    // A preloaded play passes NULL to the prefix matcher:
                    // only explicit channel overrides may change its gain.
                    self.apply_registered_se_channel(slot, handle, audio);
                    self.consume_registered_se_pan(flags, handle, audio);
                    let _ = audio.play_fade(
                        handle,
                        flags & 1 != 0,
                        if flags & 4 != 0 { fade } else { 0 },
                    );
                }
                if flags & 2 != 0 && self.dispatch_skip_state() & 0x30 == 0 {
                    return self.begin_registered_se_wait(slot, 1);
                }
                ExtCallOutcome::Continue
            }
            3 | 6 => {
                let slot = args[0];
                if slot != -1 && !(0..16).contains(&slot) {
                    return ExtCallOutcome::Continue;
                }
                let fade = if index == 3 && self.dispatch_skip_state() == 0 {
                    args[1].max(0) as u32
                } else {
                    0
                };
                for lane in 0..16 {
                    if slot != -1 && lane != slot {
                        continue;
                    }
                    self.registered_se_flags[lane as usize] &= !1;
                    if index == 3 {
                        self.registered_se_controls[lane as usize] = SeChannelControl::default();
                    }
                    let Some(&handle) = self.game_audio.get(&(5, lane)) else {
                        continue;
                    };
                    if index == 3 && fade != 0 {
                        if let Some(audio) = audio.as_deref_mut() {
                            let _ = audio.stop_fade(handle, fade);
                        }
                    } else if index == 3 && self.registered_se_flags[lane as usize] & PRELOADED != 0
                    {
                        if let Some(audio) = audio.as_deref_mut() {
                            let _ = audio.stop(handle);
                        }
                    } else {
                        self.game_audio.remove(&(5, lane));
                        self.registered_se_flags[lane as usize] = 0;
                        if let Some(audio) = audio.as_deref_mut() {
                            let _ = audio.release(handle);
                        }
                    }
                }
                ExtCallOutcome::Continue
            }
            4 => {
                let lane = args[1];
                if !(0..3).contains(&lane) {
                    return ExtCallOutcome::Continue;
                }
                self.se_volume_percent.insert(lane, args[0]);
                if let Some(audio) = audio {
                    self.apply_registered_se_levels(audio);
                }
                ExtCallOutcome::Continue
            }
            5 => {
                if !(0..3).contains(&args[0]) {
                    return ExtCallOutcome::Continue;
                }
                ExtCallOutcome::Value(self.se_volume_percent.get(&args[0]).copied().unwrap_or(100))
            }
            7 => {
                if !(0..16).contains(&args[0]) || self.dispatch_skip_state() & 0x30 != 0 {
                    return ExtCallOutcome::Continue;
                }
                self.begin_registered_se_wait(args[0], 1)
            }
            13 => {
                // Shares the limit configured by voice_autopan_initialize.
                let width = self.logical_size().0 as i32;
                let step = width / 20;
                let x = if args[0] < 0 {
                    -step
                } else if args[0] > 20 {
                    width.saturating_add(step)
                } else {
                    args[0] * step
                };
                let limit = self.text_state.voice_autopan_limit;
                self.registered_se_pan = if x < 0 {
                    limit.wrapping_neg()
                } else if x > width {
                    limit
                } else {
                    ((x as f32 / width.max(1) as f32) * limit.wrapping_mul(2) as f32 - limit as f32)
                        as i32
                };
                ExtCallOutcome::Continue
            }
            14 => {
                if (0..3).contains(&args[1]) {
                    self.se_enabled.insert(args[1], args[0] == 0);
                    if let Some(audio) = audio {
                        self.apply_registered_se_levels(audio);
                    }
                }
                ExtCallOutcome::Continue
            }
            15 | 18 => {
                let slot = args[0];
                if !(0..16).contains(&slot) {
                    return ExtCallOutcome::Continue;
                }
                let control = &mut self.registered_se_controls[slot as usize];
                if index == 15 {
                    control.frequency = (args[1] > 0).then_some(args[1]);
                } else {
                    control.volume = (args[1] >= 0).then_some(args[1]);
                }
                if args[2] != 0 {
                    if let Some(audio) = audio {
                        if index == 15 {
                            if let Some(&handle) = self.game_audio.get(&(5, slot)) {
                                let _ = audio.set_channel_frequency(handle, args[1]);
                            }
                        } else {
                            // Native 0x46318B passes the logical slot where
                            // 0x461900 expects an encoded PAL handle. Preserve
                            // that lookup instead of applying to this slot.
                            if let Some((&(_, found), &handle)) =
                                self.game_audio.iter().find(|((category, _), handle)| {
                                    *category == 5 && handle.raw() == slot as u32
                                })
                            {
                                self.apply_registered_se_channel(found, handle, audio);
                            }
                        }
                    }
                }
                ExtCallOutcome::Continue
            }
            16 => ExtCallOutcome::Value(
                audio
                    .and_then(|audio| {
                        self.game_audio
                            .get(&(5, args[0]))
                            .and_then(|&handle| audio.channel_frequency(handle).ok())
                    })
                    .unwrap_or(-1),
            ),
            _ => unreachable!(),
        }
    }

    fn begin_registered_se_wait(&mut self, slot: i32, value: i32) -> ExtCallOutcome {
        self.pending_registered_se_wait = Some(slot);
        self.pc = self.pc.saturating_sub(self.extcall_instruction_size);
        ExtCallOutcome::Wait {
            value,
            request: WaitRequest::Frame(1),
        }
    }

    fn poll_registered_se_wait(
        &mut self,
        audio: Option<&mut AudioSystem>,
        input: Option<&PalInputState>,
    ) -> ExtCallOutcome {
        let slot = self.pending_registered_se_wait.unwrap();
        let handle = self.game_audio.get(&(5, slot)).copied();
        if let (Some(audio), Some(handle)) = (audio, handle) {
            if audio.is_playing(handle).unwrap_or(false) {
                let keys = input
                    .map(|input| {
                        if self.button_release_mode {
                            input.raw_key_pull()
                        } else {
                            input.raw_key_push()
                        }
                    })
                    .unwrap_or(0);
                if self.dispatch_skip_state() == 0 && keys & 0x800006 == 0 {
                    return self.begin_registered_se_wait(slot, slot);
                }
                let _ = audio.stop_fade(handle, 1000);
            } else if self.registered_se_flags[slot as usize] & PRELOADED == 0 {
                let _ = audio.release(handle);
                self.game_audio.remove(&(5, slot));
            }
        }
        self.pending_registered_se_wait = None;
        ExtCallOutcome::Continue
    }
}
