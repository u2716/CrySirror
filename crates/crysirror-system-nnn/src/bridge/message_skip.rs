// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native WM_CONTROL hold / WM_SHIFT toggle and read-message policy. Device
//! events remain engine InputFrames, glyph reveal remains a shared operation.
use super::*;
impl Bridge {
    pub(crate) fn observe_message_keys(&mut self, input: InputFrame) {
        use crysirror_core::service::buttons;
        self.native_input = input;
        let control = input.buttons_down & buttons::CTRL != 0;
        // Native modifier key repeat does not repeatedly override a script's
        // AUTOMESSAGE/CANNOTSKIP kill-shift decision while Control stays down.
        if control && !self.control_was_down {
            self.message_skip = true;
        } else if !control && self.control_was_down {
            self.message_skip = false;
        }
        self.control_was_down = control;
        // Native WM_KEYDOWN dispatches these only while PRINTMESSAGE is the
        // foreground mode. Retain the edge while its admitted shared frame is
        // preparing; dependency pumps do not manufacture/repeat key events.
        let message = self.background_read_marks.is_none()
            && !self.scene_replay
            && self
                .text
                .as_ref()
                .is_some_and(|text| text.pending_message() || text.published_message())
            && !self.selection.as_ref().is_some_and(|selection| {
                selection.pending_selection() || selection.published_selection()
            });
        if message {
            if self.f4_enabled && input.buttons_pressed & buttons::F4 != 0 {
                self.native_skip_key = Some(false);
            }
            if self.f5_enabled && input.buttons_pressed & buttons::F5 != 0 {
                self.native_skip_key = Some(true);
            }
        }
        if input.buttons_pressed & buttons::SHIFT != 0 {
            let message_mode = self.background_read_marks.is_none()
                && !self.selection.as_ref().is_some_and(|selection| {
                    selection.pending_selection() || selection.published_selection()
                });
            if self.message_skip || !self.shift_message_only || message_mode {
                self.message_skip = !self.message_skip;
            }
        }
    }
    /// Actual WM_F4/F5: SetMessageRead(lastID), update five independent skip
    /// flags, optional ClearAllEffect, then SetYoyaku. The caller ends its real
    /// message receipt only after this admitted frame/resource boundary.
    pub(super) fn apply_native_skip_key(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(f5) = self.native_skip_key else {
            return Ok(false);
        };
        ensure!(
            self.actor_frame.is_none(),
            "NNN key action during incomplete native actor frame"
        );
        let mut file = self
            .system_file
            .clone()
            .context("NNN F4/F5 original SystemFile absent")?;
        if let Some(id) = self.last_message_id {
            file.mark_message(
                self.system_script
                    .context("NNN F4/F5 source reader absent")?,
                id,
            )?;
        }
        if f5 {
            self.clear_all_native_effects(host)?;
        }
        self.system_file = Some(file);
        self.skip.f4 = !f5;
        self.skip.f5 = f5;
        self.skip.next_command = true;
        self.skip.effect_command = true;
        self.skip.film_end = false;
        self.native_skip_key = None;
        Ok(true)
    }
    pub(super) fn clear_all_native_effects(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ops = self
            .active_effects
            .iter()
            .map(|slot| Ok(json!({"op":"remove","id":self.layer_id(*slot)?})))
            .collect::<Result<Vec<_>>>()?;
        self.submit(host, json!({"layers":ops}))?;
        self.obsolete_message_exp_wait = false;
        self.active_effects.clear();
        self.inactive_effect_records.clear();
        self.patterns.clear();
        self.fades.clear();
        self.greys.clear();
        self.negas.clear();
        self.chara_trans.clear();
        self.old_lamps.clear();
        self.buras.clear();
        self.moves.clear();
        self.scrolls.clear();
        self.fusumas.clear();
        self.shakes.clear();
        self.flashes.clear();
        self.picture_effects.clear();
        self.mosaics.clear();
        self.fill_effects.clear();
        self.passive_counts.clear();
        self.picture_assignments.clear();
        self.picture_numbers.clear();
        self.rectangles.clear();
        self.not_trans.clear();
        // Actual538760 ->4fe950 clears CutinWaitClick, not encoded rows or
        // retained GlyphRuns. APPEND after CLEARALL/F5 must still see them.
        self.actor_refresh_required = true;
        Ok(())
    }
    /// Actual531820 queries the raw modifier latch independently of
    /// CANNOTSKIP. Unlike SE, an unread voice survives read-only Control skip.
    pub(super) fn native_voice_suppressed(&self) -> Result<bool> {
        if !self.message_skip || self.auto_message != 0 {
            return Ok(false);
        }
        let file = self
            .system_file
            .as_ref()
            .context("NNN voice skip SystemFile absent")?;
        if file.system_word(140)? == 1 {
            return Ok(true);
        }
        let Some(id) = self.last_message_id else {
            return Ok(false);
        };
        file.message(
            self.system_script
                .context("NNN voice skip source reader absent")?,
            id,
        )
    }
    pub(super) fn native_message_skip_allowed(&self, serial: u32) -> Result<bool> {
        if !self.message_skip || self.auto_message > 0 || self.cannot_skip != 0 {
            return Ok(false);
        }
        let file = self
            .system_file
            .as_ref()
            .context("NNN skip requires original persistent read/preferences state")?;
        if file.system_word(140)? != 0 || serial == u32::MAX {
            return Ok(true);
        }
        let script = self
            .system_script
            .context("NNN read-message script index absent")?;
        file.message(script, (serial & 0xffff) as i32)
    }
    pub(super) fn mark_native_message_read(&mut self, serial: u32) -> Result<()> {
        if serial != u32::MAX {
            // SetMessageRead's default script argument is the current reader's
            // project index, not serial's optional high16 display tag.
            if let (Some(file), Some(script)) = (self.system_file.as_mut(), self.system_script) {
                file.mark_message(script, (serial & 0xffff) as i32)?;
            }
        }
        Ok(())
    }
}
