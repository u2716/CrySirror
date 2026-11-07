// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native EFFECT state hydration. No SetParam callback, Calcu or CountUp is
//! replayed: saved count/countMax and six rectangles are authoritative.
use super::*;
use crate::native_save::SavedEffect;
impl Bridge {
    pub(crate) fn load_saved_effect_state(
        &mut self,
        records: &[SavedEffect],
        layer_off: u32,
    ) -> Result<()> {
        ensure!(records.len() == 16, "NNN original sixteen effect records");
        ensure!(
            self.active_effects.is_empty()
                && self.patterns.is_empty()
                && self.fades.is_empty()
                && self.greys.is_empty()
                && self.negas.is_empty()
                && self.chara_trans.is_empty()
                && self.old_lamps.is_empty()
                && self.buras.is_empty()
                && self.moves.is_empty()
                && self.scrolls.is_empty()
                && self.fusumas.is_empty()
                && self.shakes.is_empty()
                && self.flashes.is_empty()
                && self.picture_effects.is_empty()
                && self.mosaics.is_empty()
                && self.fill_effects.is_empty(),
            "NNN saved effects require detached native state"
        );
        self.native_layer_off = layer_off;
        for (slot, record) in records.iter().enumerate() {
            let slot = slot as i32;
            let words = &record.words;
            // Disabled native records must not invoke a stale/unknown class,
            // but their real raw parameter/counter/previous-effect state still
            // belongs to this native slot and must survive another save.
            self.previous_effects.insert(slot, words[254]);
            self.rectangles.insert(slot, record.rectangles());
            self.not_trans.insert(slot, words[253] != 0);
            if !record.enabled() {
                self.inactive_effect_records.insert(slot, record.clone());
                continue;
            }
            let count = words[27];
            let frames = words[28];
            let original_p = &words[29..253];
            let mut parameters: [i32; 224] = original_p.try_into().unwrap();
            // Move need not initialize para0, and load copies countMax
            // independently. Typed validation must use the authoritative max.
            parameters[0] = frames;
            let p = &parameters;
            ensure!(
                (-864000..=1000000).contains(&count) && (-864000..=1000000).contains(&frames),
                "NNN saved effect finite count profile"
            );
            self.active_effects.insert(slot);
            self.previous_effects.insert(slot, words[254]);
            self.rectangles.insert(slot, record.rectangles());
            self.not_trans.insert(slot, words[253] != 0);
            self.layer_visible
                .insert(slot, layer_off & (1 << slot) == 0);
            if words[1] != -1 {
                ensure!(
                    self.images
                        .as_ref()
                        .is_some_and(|images| images.pictures.contains_key(&slot)),
                    "NNN saved effect slot{slot} requires its prepared or warm native picture"
                );
                // pic is a presence flag here; GetPicture(slot) is original.
                self.picture_assignments.insert(slot);
            }
            match words[2] {
                0 => {
                    self.passive_counts.insert(slot, (count, frames));
                }
                43 => {
                    let mut state = crate::native_effects_mosaic::MosaicState::parse(&original_p[..7], self.bindings.size)?;
                    state.count = count;
                    state.frames = frames;
                    self.mosaics.insert(slot, state);
                }
                8 | 90 | 99 | 124 => {
                    let length = if words[2] == 8 { 2 } else if words[2] == 90 { 6 } else if words[2] == 124 { 5 } else { 10 };
                    let mut state = crate::native_picture_effect::PictureEffectState::parse(words[2], &original_p[..length])?;
                    state.count = count;
                    state.frames = frames;
                    state.validate()?;
                    self.picture_effects.insert(slot, state);
                }
                9 => {
                    let mut state = crate::native_chara_trans::CharaTransState::parse(
                        &p[..14],
                        self.bindings.size,
                    )?;
                    state.frames = frames;
                    state.count = count;
                    self.chara_trans.insert(slot, state);
                }
                10 | 11 => {
                    let mut state = if words[2] == 11 {
                        crate::native_effects::FadeInState::parse_out(&p[..6])?
                    } else {
                        crate::native_effects::FadeInState::parse(&p[..6])?
                    };
                    state.frames = frames;
                    state.count = count;
                    state.validate_curve()?;
                    self.fades.insert(slot, state);
                }
                29 => {
                    // Actual legacy InitLoadGame->InitAllParameter invokes
                    // SUNA's InitParameter0x4a68d0: SetParam16 resets count0,
                    // countMax=para0 and reinitializes its class-global RNG
                    // pattern. Later public code preserves count instead.
                    let parameters = crate::native_effects::SunaParameters::parse(
                        &original_p[..10],
                        self.bindings.size,
                    )?;
                    ensure!(
                        (-864000..=864000).contains(&parameters.frames),
                        "NNN saved SUNA initialized frame profile"
                    );
                    self.saved_suna.insert(slot, parameters);
                }
                52 => {
                    let mut state =
                        crate::native_move::NativeMoveState::parse(&[frames], record.rectangles())?;
                    state.count = count;
                    self.moves.insert(slot, state);
                }
                54 => {
                    // Native fill reserves para0; the colour comes from1..3.
                    self.fill_effects
                        .insert(slot, [p[1] as u8, p[2] as u8, p[3] as u8]);
                    self.passive_counts.insert(slot, (count, frames));
                }
                17 => {
                    let mut state =
                        crate::native_effects_flash::FlashState::parse(&original_p[..6]);
                    state.frames = frames;
                    state.count = count;
                    self.flashes.insert(slot, state);
                }
                18 => {
                    let mut state =
                        crate::native_effects_shake::ShakeState::parse(&original_p[..3]);
                    state.frames = frames;
                    state.count = count;
                    self.shakes.insert(slot, state);
                }
                39 => {
                    let mut state = crate::native_effects_scroll::ScrollState::parse(
                        &original_p[..4],
                        self.bindings.size,
                    )?;
                    state.count = count;
                    state.frames = frames;
                    self.scrolls.insert(slot, state);
                }
                84 => {
                    let mut state = crate::native_effects_fusuma::FusumaState::parse(
                        &original_p[..7],
                        self.bindings.size,
                    )?;
                    state.count = count;
                    state.frames = frames;
                    self.fusumas.insert(slot, state);
                }
                74 => {
                    let mut state =
                        crate::native_bura::BuraState::parse(&p[..13], self.bindings.size)?;
                    state.frames = frames;
                    state.count = count;
                    state.supplied = original_p[..13].to_vec();
                    state.validate(self.bindings.size)?;
                    self.buras.insert(slot, state);
                }
                109 => {
                    let mut state = crate::native_old_lamp::OldLampState::parse(&p[..16])?;
                    state.count = count;
                    state.mode = p[16];
                    state.remaining = p[17];
                    ensure!((0..=2).contains(&state.mode), "NNN saved lamp native mode");
                    self.old_lamps.insert(slot, state);
                }
                31 => {
                    let mut state = crate::native_effects::LayerGreyState::parse(&p[..5])?;
                    state.frames = frames;
                    state.count = count;
                    state.validate_curve()?;
                    self.negas.insert(slot, state);
                }
                129 => {
                    let mut state = crate::native_effects::LayerGreyState::parse(&p[..5])?;
                    state.frames = frames;
                    state.count = count;
                    state.validate_curve()?;
                    self.greys.insert(slot, state);
                }
                command => bail!(
                    "NNN saved effect slot{slot} native command{command} has no shared consumer"
                ),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bridge() -> Bridge {
        Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [8, 8],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap()
    }
    fn records() -> Vec<SavedEffect> {
        (0..16)
            .map(|_| {
                let mut words = [0; 256];
                words[1] = -1;
                SavedEffect {
                    words,
                    filename: vec![],
                }
            })
            .collect()
    }
    fn effect(
        records: &mut [SavedEffect],
        slot: usize,
        command: i32,
        count: i32,
        max: i32,
        p: &[i32],
    ) {
        let w = &mut records[slot].words;
        w[0] = 1;
        w[2] = command;
        w[27] = count;
        w[28] = max;
        w[29..29 + p.len()].copy_from_slice(p);
    }
    #[test]
    fn complete_native_effect_state_uses_saved_counts_rectangles_work_and_previous_commands() {
        let mut records = records();
        effect(&mut records, 0, 10, 3, 20, &[i32::MIN, 10, 20, 30, 0, 0]);
        effect(&mut records, 1, 11, 5, 40, &[i32::MIN, 20, 30, 40, 0, 0]);
        effect(
            &mut records,
            2,
            9,
            7,
            60,
            &[i32::MIN, 50, 100, 0, 0, 1, 1, 2, 6, 6, 0, 1, 7, 7],
        );
        effect(&mut records, 3, 52, 9, 30, &[i32::MIN]);
        effect(&mut records, 4, 54, 2, 99, &[12345, 0x123, 0x234, 0x345]);
        effect(
            &mut records,
            5,
            74,
            4,
            20,
            &[i32::MIN, 4, 4, 110, 100, 20, 10, 0, 0, 0, 0, 4, 4],
        );
        effect(
            &mut records,
            6,
            109,
            33,
            999999,
            &[999999, 20, 0, 40, 1, 1, 0],
        );
        records[6].words[29 + 16] = 1;
        records[6].words[29 + 17] = 17;
        effect(&mut records, 7, 129, 5, 30, &[i32::MIN, 0, 0, 100, 0]);
        effect(&mut records, 8, 0, 99, 99999, &[]);
        for (slot, record) in records.iter_mut().enumerate() {
            record.words[254] = 1000 + slot as i32;
            record.words[200] = 0x12345678;
            for (index, value) in record.words[3..27].iter_mut().enumerate() {
                *value = index as i32 + slot as i32;
            }
        }
        records[15].words[2] = i32::MAX; // Disabled opaque stale command.
        let before = records
            .iter()
            .map(|record| record.words)
            .collect::<Vec<_>>();
        let mut bridge = bridge();
        bridge.load_saved_effect_state(&records, 1 << 3).unwrap();
        assert_eq!(bridge.fades[&0].frames, 20);
        assert_eq!(bridge.fades[&0].count, 3);
        assert_eq!(bridge.fades[&1].color, [20, 30, 40]);
        assert!(bridge.fades[&1].fade_out);
        assert_eq!(bridge.chara_trans[&2].frames, 60);
        assert_eq!(bridge.chara_trans[&2].count, 7);
        assert_eq!(bridge.moves[&3].frames, 30);
        assert_eq!(bridge.moves[&3].count, 9);
        assert_eq!(bridge.moves[&3].rectangles, records[3].rectangles());
        assert_eq!(bridge.fill_effects[&4], [0x23, 0x34, 0x45]);
        assert_eq!(bridge.buras[&5].count, 4);
        assert_eq!(bridge.buras[&5].supplied[0], i32::MIN);
        assert_eq!(
            (
                bridge.old_lamps[&6].count,
                bridge.old_lamps[&6].mode,
                bridge.old_lamps[&6].remaining
            ),
            (33, 1, 17)
        );
        assert_eq!(bridge.greys[&7].frames, 30);
        assert_eq!(bridge.greys[&7].count, 5);
        assert_eq!(bridge.passive_counts[&8], (99, 99999));
        assert_eq!(bridge.previous_effects[&8], 1008);
        assert_eq!(bridge.layer_visible[&3], false);
        assert!(!bridge.active_effects.contains(&15));
        assert_eq!(
            records
                .iter()
                .map(|record| record.words)
                .collect::<Vec<_>>(),
            before
        );
    }
    #[test]
    fn active_unknown_picture_and_uninitialized_class_global_pattern_never_fake_success() {
        let mut records = records();
        effect(&mut records, 0, 987654, 0, 0, &[]);
        assert!(
            bridge()
                .load_saved_effect_state(&records, 0)
                .unwrap_err()
                .to_string()
                .contains("987654")
        );
        records[0].words[2] = 0;
        records[0].words[1] = 15;
        assert!(
            bridge()
                .load_saved_effect_state(&records, 0)
                .unwrap_err()
                .to_string()
                .contains("warm native picture")
        );
        records[0].words[1] = -1;
        records[0].words[2] = 29;
        records[0].words[29..39].copy_from_slice(&[0, 0, 100, 0, 0, 0, 0, 0, 8, 8]);
        let mut native = bridge();
        native.load_saved_effect_state(&records, 0).unwrap();
        assert_eq!(native.saved_suna.len(), 1);
        assert!(
            native.patterns.is_empty(),
            "pending shared class initialization must not invent a pattern handle"
        );
    }
}
