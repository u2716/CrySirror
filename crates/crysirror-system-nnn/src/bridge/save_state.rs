// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native EFFECT/EFFECTFILENAME production from retained semantic state.
//! No device handles, shared image leases or renderer state enter native files.
use super::*;
use crate::native_save::SavedEffect;
impl Bridge {
    pub(crate) fn configure_chart(&mut self, configuration: &crate::native_bootstrap::NativeConfiguration) -> Result<()> {
        self.auxiliary.configure_chart(configuration)
    }
    pub(crate) fn write_original_auxiliary(
        &self,
        file: &mut crate::native_save::NativeSaveFile,
        profile: &crate::native_save::LegacySaveProfile,
    ) -> Result<()> {
        let mut auxiliary=self.auxiliary.clone();
        if let Some(text)=&self.text {
            auxiliary.cutin_rows.fill([0;2]);
            for (to,from) in auxiliary.cutin_rows.iter_mut().zip(&text.state.cutin_rows) {*to=*from;}
        }
        auxiliary.write(file, profile)
    }
    pub(crate) fn commit_saved_remaining(&mut self, value: i32) {
        self.saved_remaining = value;
    }
    pub(crate) fn original_wait_state(
        &self,
        call: Option<&NativeCall>,
    ) -> Result<crate::native_save_wait::SavedWait> {
        use crate::native_save_wait::SavedWait;
        if matches!(self.restored_wait, Some(SavedWaitKind::Message))
            || call.is_some_and(|call| matches!(call.payload, Payload::Message { .. }))
        {
            let text = self
                .text
                .as_ref()
                .context("NNN save message actor absent")?;
            ensure!(
                text.published_message(),
                "NNN save unpublished native message"
            );
            return Ok(SavedWait::Message {
                rows: text.state.rows.clone(),
                mode: text.state.mode,
                serial: text.state.serial,
                window_number: self.script_window_number,
                window_visible: self.message_window_visible,
                cannot_click: self.cannot_click.unwrap_or(text.profile.cannot_click),
            });
        }
        let selection = self
            .selection
            .as_ref()
            .context("NNN save selection actor absent")?;
        let (rows, choices, messages) = selection.original_rows()?;
        let (time_limit, auto_select, special) = match call.map(|call| &call.payload) {
            Some(Payload::Selection {
                time_limit,
                auto_select,
                special,
                ..
            }) => {
                let _ = (time_limit, auto_select, special);
                selection.original_controls()
            }
            None if matches!(self.restored_wait, Some(SavedWaitKind::Selection)) => {
                selection.original_controls()
            }
            _ => bail!("NNN save outside original message/selection actor"),
        };
        Ok(SavedWait::Selection {
            rows,
            total: choices + messages,
            supplemental: messages,
            time_limit,
            auto_select,
            special,
        })
    }
    pub(crate) fn write_original_actor_state(
        &self,
        file: &mut crate::native_save::NativeSaveFile,
        call: Option<&NativeCall>,
    ) -> Result<()> {
        if self.chart_mode_active() {
            let text=self.text.as_ref().context("NNN chart save latent message actor absent")?;
            file.replace_live_wait(&crate::native_save_wait::SavedWait::Message {
                rows:text.state.rows.clone(),mode:text.state.mode,serial:text.state.serial,
                window_number:self.script_window_number,window_visible:self.message_window_visible,
                cannot_click:self.cannot_click.unwrap_or(text.profile.cannot_click),
            })?;
            return self.write_original_common_status(file);
        }
        file.replace_live_wait(&self.original_wait_state(call)?)?;
        self.write_original_common_status(file)
    }
    pub(crate) fn write_original_common_status(&self,file:&mut crate::native_save::NativeSaveFile)->Result<()> {
        let audio = self
            .audio
            .as_ref()
            .context("NNN original save audio policy absent")?
            .original_audio_work()?;
        file.replace_audio_work(&audio)?;
        if let Some(system) = &self.system_file {
            let names = system.player_names()?;
            file.replace_status_name(448, &names[0])?;
            file.replace_status_name(452, &names[1])?;
        }
        file.replace_status_word(45, i32::try_from(self.frame_time_ms)?)?;
        file.replace_status_word(44, i32::from(self.message_window_visible))?;
        file.replace_status_word(485, i32::from(self.cannot_click.unwrap_or_else(||self.text.as_ref().is_some_and(|text|text.profile.cannot_click))))?;
        if let Some(text)=self.text.as_ref() {
            file.replace_status_word(29, i32::try_from(text.state.rows.len())?)?;
            file.replace_status_word(30, text.state.mode as i32)?;
        }
        file.replace_status_word(180, self.script_window_number)?;
        file.replace_status_word(181, self.script_cursor_number)?;
        file.replace_status_word(182, self.script_mouse_number)?;
        file.replace_status_word(48, self.native_layer_off as i32)?;
        file.replace_status_word(484, self.auto_message)?;
        //Legacy53ad20 stores configMask at476. It has no persisted tagged
        //message serial; that separate CCommonPrintMessage state is retained.
        file.replace_status_word(476, self.configuration_mask)?;
        file.replace_status_word(486, self.cannot_skip)?;
        file.replace_status_word(32, self.last_select_id.unwrap_or(-1))?;
        file.replace_status_word(47, self.last_select_hero)?;
        file.replace_status_word(46, self.saved_remaining)?;
        file.replace_status_word(477, i32::from(self.skip_film_enabled))?;
        // These original last-ID members were not always initialized before
        // a game's first SELECT. Use the explicit no-ID state instead of RAM.
        file.replace_status_word(28, self.last_message_id.unwrap_or(-1))?;
        Ok(())
    }
    pub(crate) fn original_effect_records(&self) -> Result<Vec<SavedEffect>> {
        ensure!(
            !self.dependency_pending(),
            "NNN effect save during native resource preparation"
        );
        let effects = self.frame_effects();
        let mut out = Vec::with_capacity(16);
        for slot in 0..16 {
            if !self.active_effects.contains(&slot)
                && !effects.contains_key(&slot)
                && !self.picture_assignments.contains(&slot)
            {
                if let Some(record) = self.inactive_effect_records.get(&slot) {
                    let mut record = record.clone();
                    if let Some(r) = self.rectangles.get(&slot) {
                        for (at, rect) in [
                            (3, r.source),
                            (7, r.destination),
                            (11, r.source_from),
                            (15, r.destination_from),
                            (19, r.source_to),
                            (23, r.destination_to),
                        ] {
                            record.words[at..at + 4].copy_from_slice(&rect);
                        }
                    }
                    if let Some(value) = self.previous_effects.get(&slot) {
                        record.words[254] = *value;
                    }
                    if let Some(value) = self.not_trans.get(&slot) {
                        if *value != (record.words[253] != 0) {
                            record.words[253] = i32::from(*value);
                        }
                    }
                    if let Some(picture) = self
                        .images
                        .as_ref()
                        .and_then(|images| images.pictures.get(&slot))
                    {
                        record.filename = picture.name.clone();
                    }
                    out.push(record);
                    continue;
                }
            }
            let mut words = [0; 256];
            words[0] = i32::from(
                self.active_effects.contains(&slot)
                    || effects.contains_key(&slot)
                    || self.passive_counts.contains_key(&slot),
            );
            words[1] = if self.picture_assignments.contains(&slot) {
                self.picture_numbers.get(&slot).copied().unwrap_or(slot)
            } else {
                -1
            };
            let r = self.rectangles.get(&slot).cloned().unwrap_or_else(|| {
                crate::native_geometry::NativeRects::natural(self.bindings.size)
            });
            for (at, rect) in [
                (3, r.source),
                (7, r.destination),
                (11, r.source_from),
                (15, r.destination_from),
                (19, r.source_to),
                (23, r.destination_to),
            ] {
                words[at..at + 4].copy_from_slice(&rect);
            }
            let (count, max) = self.effect_counter(slot).unwrap_or((0, 0));
            words[27] = count;
            words[28] = max;
            words[253] = i32::from(
                self.not_trans
                    .get(&slot)
                    .copied()
                    .unwrap_or(slot < 4 || slot == 14),
            );
            words[254] = self.previous_effects.get(&slot).copied().unwrap_or(-1);
            let mut p = Vec::new();
            words[2] = match effects.get(&slot) {
                Some(NativeFrameEffect::MessageBackground | NativeFrameEffect::Paint(_)) => {
                    bail!("NNN save found a transient paint DTO")
                }
                Some(NativeFrameEffect::Move(frame)) => {
                    p.push(frame.state.frames);
                    frame.command()
                }
                Some(NativeFrameEffect::Scroll(frame)) => {
                    p.extend(frame.frame.state.parameters());
                    if frame.ended { 0 } else { 39 }
                }
                Some(NativeFrameEffect::Flash(frame)) => {
                    if frame.state.ended {
                        let color = frame.state.terminal_color();
                        p.extend([
                            frame.state.frames,
                            i32::from(color[0]),
                            i32::from(color[1]),
                            i32::from(color[2]),
                        ]);
                        54
                    } else {
                        p.extend(frame.state.parameters());
                        17
                    }
                }
                Some(NativeFrameEffect::Shake(frame)) => {
                    p.extend(frame.state.parameters());
                    frame.state.command()
                }
                Some(NativeFrameEffect::Fusuma(frame)) => {
                    p.extend(frame.state.parameters());
                    84
                }
                Some(NativeFrameEffect::Suna(state)) => {
                    let s = &state.parameters;
                    p.extend([
                        s.frames,
                        i32::from(s.transition),
                        s.end_percent,
                        s.start_percent,
                        s.easing,
                        s.subframe,
                        s.rect[0] as i32,
                        s.rect[1] as i32,
                        s.rect[2] as i32,
                        s.rect[3] as i32,
                    ]);
                    29
                }
                Some(NativeFrameEffect::Fade(state)) => {
                    p.extend([
                        state.frames,
                        state.color[0],
                        state.color[1],
                        state.color[2],
                        state.easing,
                        state.subframe,
                    ]);
                    state.command()
                }
                Some(NativeFrameEffect::Nega(state)) => {
                    p.extend([
                        state.frames,
                        state.easing,
                        state.subframe,
                        state.end_percent,
                        state.start_percent,
                    ]);
                    31
                }
                Some(NativeFrameEffect::Grey(state)) => {
                    p.extend(state.parameters());
                    state.command()
                }
                Some(NativeFrameEffect::Mosaic(state)) => {
                    p.extend(state.parameters());
                    43
                }
                Some(NativeFrameEffect::Picture(frame)) => {
                    p.extend(frame.state.parameters());
                    frame.state.command()
                }
                Some(NativeFrameEffect::Chara(frame)) => {
                    let s = &frame.state;
                    p.extend([
                        s.frames,
                        s.end_percent,
                        s.start_percent,
                        s.easing,
                        s.subframe,
                        i32::from(s.clip),
                    ]);
                    p.extend(s.clip_to);
                    p.extend(s.clip_from);
                    9
                }
                Some(NativeFrameEffect::Lamp(frame)) => {
                    let s = &frame.state;
                    p.resize(18, 0);
                    let n = s.supplied.len().min(16);
                    p[..n].copy_from_slice(&s.supplied[..n]);
                    p[1..4].copy_from_slice(&s.durations);
                    p[4..6].copy_from_slice(&s.flicker);
                    p[6] = s.random_flip;
                    p[16] = s.mode;
                    p[17] = s.remaining;
                    109
                }
                Some(NativeFrameEffect::Bura(state)) => {
                    p.extend([
                        state.frames,
                        state.center_to[0],
                        state.center_to[1],
                        state.scale_to,
                        state.scale_from,
                        state.blend_to,
                        state.blend_from,
                        state.scale_curve,
                        state.scale_subframe,
                        state.blend_curve,
                        state.blend_subframe,
                        state.center_from[0],
                        state.center_from[1],
                    ]);
                    74
                }
                None => {
                    if let Some(color) = self.fill_effects.get(&slot) {
                        p.extend([
                            0,
                            i32::from(color[0]),
                            i32::from(color[1]),
                            i32::from(color[2]),
                        ]);
                        54
                    } else {
                        0
                    }
                }
            };
            // Initialized canonical parameter storage replaces irrelevant native
            // uninitialized padding; every known SetParam/InitLoad consumer's
            // fields and live phase are produced explicitly above.
            words[29..29 + p.len()].copy_from_slice(&p);
            let filename = self
                .images
                .as_ref()
                .and_then(|images| images.pictures.get(&slot))
                .map(|picture| picture.name.clone())
                .unwrap_or_default();
            ensure!(
                filename.len() <= 254 && !filename.contains(&0),
                "NNN save picture filename ABI"
            );
            out.push(SavedEffect { words, filename });
        }
        Ok(out)
    }
}
