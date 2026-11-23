//! 4233A0 case 0x0E framebuffer transitions that run on shared effect
//! controls (the same actors DC4PH's framebuffer_ops drives).
//!
//! Selectors 15/16 (not replay, not 41D820 skipping): 41E7E0 loads
//! "system\\effbreak" into slot 35, 406720 selects the colour texture
//! this+1724 when present, and 406A30(this+1728 = slot 35, 5|4, pattern, 60)
//! starts the 33x19 tile field when Buf dword 9 (value 99/10) is nonzero.
//! 16 additionally runs 41A580 (clear the five live UI owners, 99/246 = 0)
//! and 4200F0(0). Both set v365, a one-frame yield (42A160 case 0xE).
//! 4200F0(0) tears down 0x27 weather state (Buf 23397..23403, slot 20,
//! 41FA80, 41A5D0(0,0,0)); DC3 0x27 is not connected, so that state is
//! always empty here and the teardown has nothing to release.
use super::*;
use crate::machine::effect_ops::EffectRequest;

impl Machine {
    ///41D820: !Buf62077 && (this[2840] || this[2841] || Buf62076).
    ///41D820 decided once per execution of an instruction: a retried
    ///instruction keeps the branch it started, even if Ctrl changes.
    pub(super) fn skip_for(&mut self, pc: u32) -> bool {
        let key = (pc, self.state.instructions);
        if let Some((owner, decision)) = self.state.skip_decision.filter(|(owner, _)| *owner == key) {
            let _ = owner;
            return decision;
        }
        let decision = self.native_skipping();
        self.state.skip_decision = Some((key, decision));
        decision
    }

    pub(super) fn native_skipping(&self) -> bool {
        self.state.runtime_values.get(&193).copied().unwrap_or(0) == 0
            && (self.state.skip_latches.iter().any(|value| *value != 0)
                || self.state.runtime_values.get(&195).copied().unwrap_or(0) != 0)
    }

    ///40D0F0: the text-speed adjustment (Buf dword 11 = value 99/12) of a
    ///native duration; speed 11 yields 0.
    pub(super) fn adjusted_ticks(&self, base: i32) -> Result<i32> {
        let speed = self.value(99, 12)?;
        if speed == 11 {
            return Ok(0);
        }
        Ok(if speed < 5 {
            (f64::from(base / 2) / 5. * f64::from(5 - speed) + f64::from(base)) as i32
        } else if speed > 5 {
            (f64::from(base) - f64::from(base / 2) / 6. * f64::from(speed - 5)) as i32
        } else {
            base
        })
    }

    ///Case 0x0E selectors 0..2 (the generic tail): 406720 keeps the colour
    ///texture this+1724 selected; when dispatching and either not skipping
    ///(41D820) or var(9,99) is set:
    ///  2: 4202F0 replaces 1724 with a black 43E100 texture and fades it in
    ///     (406690 mode 0, 40D0F0(4*(5n+5)) ticks), then 4200F0(1) and 41A580.
    ///  0/1: 406690(1, 40D0F0(4*(5n+5))) fades the selected colour out at
    ///     half rate.
    ///406690 does nothing unless Buf dword 9 (value 99/10) and a colour
    ///texture exist. Both set v365: a one-frame yield (42A160 case 0xE).
    fn flat_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome> {
        if self.skip_for(instruction.offset) && self.value(9, 99)? == 0 {
            return self.advance(instruction.next, instruction.opcode);
        }
        let out = a[0] == 2;
        let ticks = self.adjusted_ticks(4 * (5 * i32::from(a[1]) + 5))?;
        let color = if out { Some([0; 3]) } else { self.state.resources.transition_color };
        if self.state.effect_started != Some(instruction.offset) {
            if let Some(color) = color.filter(|_| self.value(99, 10).is_ok_and(|enabled| enabled != 0)) {
                ensure!(ticks > 0, "DC3WY 406690 zero-duration fade (1/0 step) is not connected");
                if !host.effect_control(&EffectRequest::FlatFade { out, ticks, color, protected: false })? {
                    return Ok(Outcome::Pending);
                }
            }
            self.state.effect_started = Some(instruction.offset);
        }
        if out {
            if !host.emit(&Event::ClearUi)? {
                return Ok(Outcome::Pending);
            }
            if !self.assign(host, 99, 246, 0)? {
                return Ok(Outcome::Pending);
            }
            self.state.resources.transition_color = Some([0; 3]);
        }
        self.state.effect_started = None;
        self.advance(instruction.next, instruction.opcode)?;
        Ok(Outcome::WaitUntil(now.saturating_add(16)))
    }

    ///Case 0x0E 17/20/21: 406770(slot-16 mask, mode, ticks) over the
    ///selected colour texture this+1724, then 41A580 and a one-frame yield.
    ///17 first loads "MSK\\MASK11A" (operand 1) or "MSK\\MASK04A" into slot
    ///16 (41E8B0) and runs mode 3 for 60 ticks; 20 is mode 2 and 21 mode 3,
    ///both for 12*operand ticks. 17/20 do nothing while skipping (41D820);
    ///21 also runs while skipping when var(9,99) or Buf62078 is set.
    ///406770 needs Buf dword 9 (value 99/10) or Buf62078 (99/192).
    fn mask_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome> {
        let skip = self.skip_for(instruction.offset)
            && (a[0] != 21 || (self.value(9, 99)? == 0 && self.value(99, 192)? == 0));
        if skip {
            return self.advance(instruction.next, instruction.opcode);
        }
        if self.state.effect_started != Some(instruction.offset) {
            if a[0] == 17 {
                let name = if a[1] == 1 { "MSK\\MASK11A" } else { "MSK\\MASK04A" };
                if !self.publish_texture(host, 16, name)? {
                    return Ok(Outcome::Pending);
                }
            }
            let enabled = self.value(99, 10)? != 0 || self.value(99, 192)? != 0;
            if let Some(color) = self.state.resources.transition_color.filter(|_| enabled) {
                let ticks = if a[0] == 17 { 60 } else { 12 * i32::from(a[1]) };
                ensure!(ticks > 0, "DC3WY 406770 zero-duration mask (1/0 step) is not connected");
                if !host.effect_control(&EffectRequest::MaskFade { reverse: a[0] != 20, ticks, color })? {
                    return Ok(Outcome::Pending);
                }
            }
            self.state.effect_started = Some(instruction.offset);
        }
        if !host.emit(&Event::ClearUi)? {
            return Ok(Outcome::Pending);
        }
        if !self.assign(host, 99, 246, 0)? {
            return Ok(Outcome::Pending);
        }
        self.state.effect_started = None;
        self.advance(instruction.next, instruction.opcode)?;
        Ok(Outcome::WaitUntil(now.saturating_add(16)))
    }

    pub(super) fn execute_effects(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode == 0x0e && matches!(a[0], 0..=2) {
            return self.flat_fade(instruction, a, host, now).map(Some);
        }
        //Case 0x0E 4/6: all objects 0..42 shake on x/y; 51/52: objects
        //10..42 selected by the operand mask (bit k selects 10(k+1)), 51 on
        //x with the native +8,0,+8,0 sequence, 52 on y. Nothing while
        //skipping (41D820).
        if instruction.opcode == 0x0e && matches!(a[0], 4 | 6 | 51 | 52) {
            if !self.skip_for(instruction.offset) {
                let objects: Vec<u16> = if matches!(a[0], 4 | 6) {
                    (0..43).collect()
                } else {
                    let mask = a[1];
                    (10..=42u16)
                        .filter(|object| {
                            [(1, 10), (2, 20), (4, 30), (8, 40)].iter().all(|(bit, only)| mask & bit == 0 || object == only)
                        })
                        .collect()
                };
                let axis = if matches!(a[0], 4 | 51) { 0 } else { 1 };
                let deltas = if a[0] == 51 { [8., 0., 8., 0.] } else { [8., 0., -8., 0.] };
                if !self.shake_objects(host, &objects, axis, deltas)? {
                    return Ok(Some(Outcome::Pending));
                }
            }
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        if instruction.opcode == 0x0e && matches!(a[0], 17 | 20 | 21) {
            return self.mask_fade(instruction, a, host, now).map(Some);
        }
        //0x0E 5: 41E360(0) renders the scene immediately; the shared
        //retained presentation already shows the current state.
        if instruction.opcode == 0x0e && a[0] == 5 {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        //0x0E 3: 406730(255,255,255) becomes this+1724, 406690(6, 5) and
        //this+13444 = operand (the shared flash with that repeat count),
        //then a one-frame yield.
        if instruction.opcode == 0x0e && a[0] == 3 {
            if self.state.effect_started != Some(instruction.offset) {
                if self.value(99, 10)? != 0
                    && !host.effect_control(&EffectRequest::Flash { repeats: a[1].max(0) as u16 })?
                {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.effect_started = Some(instruction.offset);
            }
            self.state.effect_started = None;
            self.state.resources.transition_color = Some([255; 3]);
            self.advance(instruction.next, instruction.opcode)?;
            return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
        }
        //Generic tail (8, 12, 18, 19, 22, 26..29, 33..50, 53..): 406720
        //keeps the colour texture; when dispatching and not skipping (or
        //var(9,99)) the dispatcher yields one frame.
        if instruction.opcode == 0x0e
            && !matches!(a[0], 0..=7 | 9..=11 | 13..=17 | 20 | 21 | 23..=25 | 30..=32 | 51 | 52)
        {
            self.advance(instruction.next, instruction.opcode)?;
            if self.native_skipping() && self.value(9, 99)? == 0 {
                return Ok(Some(Outcome::Advanced));
            }
            return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
        }
        if instruction.opcode != 0x0e || !matches!(a[0], 15 | 16) {
            return Ok(None);
        }
        if self.skip_for(instruction.offset) {
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        if self.state.effect_started != Some(instruction.offset) {
            let name = "SYSTEM\\EFFBREAK";
            if !host.emit(&Event::LoadImage { slot: 35, name: name.into() })? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.image_names.insert(35, name.into());
            if self.value(99, 10)? != 0
                && !host.effect_control(&EffectRequest::TileMask {
                    pattern: a[1],
                    color: self.state.resources.transition_color,
                })?
            {
                return Ok(Some(Outcome::Pending));
            }
            self.state.effect_started = Some(instruction.offset);
        }
        if a[0] == 16 {
            if !host.emit(&Event::ClearUi)? {
                return Ok(Some(Outcome::Pending));
            }
            if !self.stop_fog(host)? {
                return Ok(Some(Outcome::Pending));
            }
            if !self.assign(host, 99, 246, 0)? {
                return Ok(Some(Outcome::Pending));
            }
        }
        self.state.effect_started = None;
        self.advance(instruction.next, instruction.opcode)?;
        Ok(Some(Outcome::WaitUntil(now.saturating_add(16))))
    }
}

impl Machine {
    ///4200F0(0)'s fog part: with Buf23397 set, release slot 20 and clear it.
    pub(super) fn stop_fog(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if self.state.fog {
            if self.state.image_names.contains_key(&20) && !self.publish_texture(host, 20, "-")? {
                return Ok(false);
            }
            self.state.fog = false;
        }
        Ok(true)
    }

    ///Cases 0x9A/0x9B: 40D670 (418590: dwords 462..465 = x0, y0, x1, y1)
    ///and 40D6B0 (4185C0: dword 466 = chance) on all five weather pools; the
    ///4185D0 spawn window (shared FallingSpawnWindow, as DC4PH 0xA4/0xA5).
    pub(super) fn execute_spawn_window(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>> {
        let (bounds, chance) = match instruction.opcode {
            0x9a => ([a[0], a[1], a[2], a[3]].map(i32::from), self.state.spawn_window.1),
            0x9b => (self.state.spawn_window.0, i32::from(a[0])),
            _ => return Ok(None),
        };
        if !host.effect_control(&EffectRequest::SpawnWindow { bounds, chance })? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.spawn_window = (bounds, chance);
        self.advance(instruction.next, instruction.opcode).map(Some)
    }

    ///4233A0 case 0x27 (weather). Pending slot-22/23 records publish first
    ///(41E8B0). Selector 0 runs 4200F0(0): stop all five pools (41A5D0
    ///(0,0,0)) and clear Buf23398..23403. Other selectors store kind/density
    ///in Buf23398/23399 and call 41A5D0(kind, density, slot-22 texture),
    ///initializing the pools on objects 6/19/29/39/48 (419BC0).
    ///Selectors 2 (slot 23, Buf23400), 7, 13 (effyousei), 15 (efflens) and
    ///16 have their own owners and are not connected.
    ///41D510(0): bind and start the two repeating cloud scrolls.
    pub(super) fn start_clouds(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        self.start_cloud_scroll(host, [7200, 3600])
    }
    ///Slot 21 rows 0..576 on object 52 and 600..1176 on 53, each scrolled
    ///left by 2048 over its period (frames), repeating.
    pub(super) fn start_cloud_scroll(&mut self, host: &mut dyn NativeHost, periods: [u32; 2]) -> Result<bool> {
        use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, SpriteResourceBinding, SpriteResourceRequest};
        use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation, SequenceTrack};
        if host
            .sprite_resources(&SpriteResourceRequest {
                bindings: vec![
                    SpriteResourceBinding { slot: 21, object: 52, crop: Some([0, 0, 3072, 576]) },
                    SpriteResourceBinding { slot: 21, object: 53, crop: Some([0, 600, 3072, 1176]) },
                ],
            })?
            .is_none()
        {
            return Ok(false);
        }
        let commands = [(52u16, periods[0]), (53, periods[1])]
            .into_iter()
            .map(|(object, ticks)| SpriteCommand {
                seed: self.object_seed(object),
                operations: vec![
                    SequenceOperation::Finish { cancel: true, lanes: None },
                    SequenceOperation::Set { channels: vec![0, 1, 12, 13], values: vec![0., 0., 0., 0.], retained: true },
                    SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: SequenceTrack::Linear {
                            channels: vec![0, 1],
                            target: vec![-2048., 0.],
                            ticks,
                            velocity_divisor: None,
                            ieee_inputs: false,
                            references: vec![0, 1],
                            loop_origins: vec![0, 1],
                            delta: DeltaSource::Retained,
                            looping: true,
                            start: None,
                            complete_first: false,
                            complete_zero: true,
                            fast: true,
                            fast_channels: None,
                            skip_zero: true,
                            capture_zero: true,
                        },
                    },
                ],
            })
            .collect();
        if !host.sprite(&SpriteRequest { commands })? {
            return Ok(false);
        }
        for object in [52u16, 53] {
            self.state.sprites.entry(object).or_default().actor = true;
        }
        Ok(true)
    }

    pub(super) fn execute_weather(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x27 {
            return Ok(None);
        }
        //Selector 7: a nonzero operand sets the fog flag Buf23397; zero
        //releases slot 20 (41E8B0 "") and clears the flag.
        if a[0] == 7 {
            if a[1] != 0 {
                self.state.fog = true;
            } else {
                if self.state.image_names.contains_key(&20) && !self.publish_texture(host, 20, "-")? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.fog = false;
            }
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        //Selector 16 (Buf23402): 41D510(0) scrolls slot 21 on objects 52
        //(source 0,0..3072,576) and 53 (0,600..3072,1176) from x 0 to
        //-2048 over 7200/3600 ticks, repeating; zero releases slot 21
        //(41FA80), which unbinds both objects.
        if a[0] == 16 {
            if a[1] != 0 {
                if !self.start_clouds(host)? {
                    return Ok(Some(Outcome::Pending));
                }
            } else if self.state.image_names.contains_key(&21) && !self.publish_texture(host, 21, "-")? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.clouds = a[1] != 0;
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        ensure!(
            !matches!(a[0], 13 | 15),
            "DC3WY case 0x27 selector {} is not connected",
            a[0]
        );
        for slot in [22u8, 23] {
            if let Some(name) = self.state.resources.pending.get(&slot).cloned() {
                if !self.publish_texture(host, slot, &name)? {
                    return Ok(Some(Outcome::Pending));
                }
            }
        }
        if self.state.weather_started != Some(instruction.offset) {
            if !host.effect_control(&EffectRequest::FrameControl {
                fast: self.native_skipping(),
                suppress_fast_color: self.value(9, 99)? != 0,
                allow_skip: true,
                effects_enabled: self.value(99, 10)? != 0,
            })? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.weather_started = Some(instruction.offset);
        }
        if a[0] == 0 && !self.stop_fog(host)? {
            return Ok(Some(Outcome::Pending));
        }
        let (kind, density) = if a[0] == 0 { (0, 0) } else { (a[0], a[1]) };
        if !host.emit(&Event::FallingEffect { kind, parameter: density })? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.weather_started = None;
        //Kind 2 keeps its density in Buf23400; Buf23398/23399 stay.
        if a[0] == 2 {
            self.state.rain = (density != 0).then_some(density);
        } else {
            self.state.weather = if a[0] == 0 { None } else { Some([kind, density]) };
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}
