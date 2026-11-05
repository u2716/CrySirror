//! Native helper contracts whose visible names hide additional state effects.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct VoiceInfo {
    pub name: String,
    pub volume: i32,
    pub enabled: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_voice_registry_obeys_gate_order_case_empty_prefix_and_fallback() {
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        assert_eq!(vm.native_voice_channel_percent("vo01", 0), None);
        assert_eq!(vm.native_voice_channel_percent("vo01", 0x40), Some(100));
        vm.text_state.voice_info_enabled = Some(true);
        assert_eq!(vm.native_voice_channel_percent("vo01", 0), Some(0));
        vm.text_state.voice_info.insert(
            -1,
            VoiceInfo {
                name: "ignored".into(),
                volume: 27,
                enabled: 1,
            },
        );
        assert_eq!(vm.native_voice_channel_percent("vo01", 0), Some(27));
        vm.text_state.voice_info.insert(
            3,
            VoiceInfo {
                name: "vo01".into(),
                volume: 81,
                enabled: 1,
            },
        );
        vm.text_state.voice_info.insert(
            1,
            VoiceInfo {
                name: "vo".into(),
                volume: 42,
                enabled: 1,
            },
        );
        assert_eq!(vm.native_voice_channel_percent("vo01_line", 0), Some(42));
        assert_eq!(vm.native_voice_channel_percent("VO01_line", 0), Some(27));
        vm.text_state.voice_info.get_mut(&1).unwrap().enabled = 0;
        assert_eq!(vm.native_voice_channel_percent("vo01_line", 0), Some(0));
        vm.text_state.voice_info.insert(
            0,
            VoiceInfo {
                name: String::new(),
                volume: 63,
                enabled: -1,
            },
        );
        assert_eq!(vm.native_voice_channel_percent("anything", 0), Some(63));
        vm.text_state.voice_info_enabled = Some(false);
        assert_eq!(vm.native_voice_channel_percent("anything", 0), Some(100));
        vm.text_state.voice_info_enabled = None;
        assert_eq!(vm.native_voice_channel_percent("anything", 0), Some(100));
    }
}

impl ScriptRuntime {
    pub(super) fn ext_set_window_text(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        sprites: Option<&SpriteSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 {
            return ExtCallOutcome::Block;
        }
        // Game.exe 0x411360 changes the host title only in native debug mode.
        // Its savepoint side effect also executes in ordinary release builds.
        if debug_vm_enabled() {
            log::debug!(
                "[trace-system] set_window_text text={} mode={}",
                args[0],
                args[1]
            );
        }
        if !self.save_state.locked && self.dispatch_skip_state() & 0x10 == 0 {
            self.capture_save_checkpoint(assets, nls, sprites);
        }
        self.capture_history_checkpoint(sprites);
        ExtCallOutcome::Value(1)
    }

    pub(super) fn ext_set_voice_info(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        let args = self.pop_ext_args(4);
        if args.len() != 4 {
            return ExtCallOutcome::Block;
        }
        let slot = args[0];
        if !(-1..32).contains(&slot) {
            return ExtCallOutcome::Value(0);
        }
        let name = (args[1] != -1).then(|| {
            self.resolve_resource_string(args[1], assets, nls)
                .unwrap_or_default()
        });
        let record = self.text_state.voice_info.entry(slot).or_default();
        if let Some(name) = name {
            record.name = name;
        }
        record.volume = args[2];
        record.enabled = args[3];
        ExtCallOutcome::Value(1)
    }

    /// Native 0x4425F0 applies one channel gain, independently of global voice
    /// group gain. None keeps the CSV compatibility path for games that have
    /// never configured the native registry or its enable gate.
    pub(super) fn native_voice_channel_percent(&self, name: &str, flags: i32) -> Option<i32> {
        if flags & 0x40 != 0 {
            return Some(100);
        }
        let state = &self.text_state;
        if state.voice_info_enabled.is_none() && state.voice_info.is_empty() {
            return None;
        }
        if !state.voice_info_enabled.unwrap_or(false) {
            return Some(100);
        }
        let name = name.split('\0').next().unwrap_or_default();
        let record = state
            .voice_info
            .range(0..32)
            .find(|(_, record)| {
                name.starts_with(record.name.split('\0').next().unwrap_or_default())
            })
            .map(|(_, record)| record)
            .or_else(|| state.voice_info.get(&-1));
        Some(
            record
                .filter(|record| record.enabled != 0)
                .map_or(0, |record| record.volume.clamp(0, 100)),
        )
    }
}
