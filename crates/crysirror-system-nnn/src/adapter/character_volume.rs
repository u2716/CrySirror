// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Thin configuration-policy call. The menu owns UI delivery and audio retry.
use super::*;
impl NativeAdapter {
    pub(crate) fn apply_character_setting(
        &mut self,
        id: i32,
        value: i32,
        captured: bool,
    ) -> Result<Option<crate::bridge::ButtonAudioEvent>> {
        self.bridge.apply_character_setting(id, value, captured)
    }
}
