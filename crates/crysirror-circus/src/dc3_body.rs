//! DC3WY live save body Buf+17704..+94568 (76864 bytes), projected from VM
//! state when 419E10 (opcode 0x1A) snapshots it to Buf+94568 and 40B6A0
//! writes that snapshot as DC3WYnn.dat.
//!
//! Layout (Buf offsets, statically read from 4233A0/402E40/4024A0/41FCC0/
//! 432690/436F60/437C40):
//!   17712 scene name[160]   17872 image names[36][48]
//!   19720 music name[32]    19752 music index (4938)
//!   19760 SFX channel k=1..4 name[32] at 19728+32k   19888 channel-1 loop
//!   19908 decoder pc 4977   19912 label metadata 4978   19916 label 4979
//!   19920 resume pc 4980    23932 history index 5983
//!   23936 history runs[400] {scene i16, diff start|0x8000, first, last,
//!         word 28756, word 28752}
//!   28740 label 7185  28744 7186  28748/28752/28756 run origins 7187..7189
//!   31964 special-run flag 7991  31968 difference count 7992
//!   31972 difference offsets u16[1000]  33972 difference values u8[1000]
//!   34976 script banks[1100]  36076 their mirror[1100]
//!   37332 objects[120][468] (raw 437C40 object prefix, 437790 copy)
//!   93512 $n name[20]  93532 $f name[20]  93588..93635 0x27 weather dwords
//!   93636 last voice[160]  93796 message[260]  94056 speaker[36]
//!   94092 version string[184]  94276 0x94 value  94284 23571  94292 hold
//!   background 23573  94296 23574  94300 saved music volume 23575
//!   94304 23576  94308 message copy[260]
//! Bytes with no native writer in the reviewed code stay zero. State this
//! VM does not model fails visibly instead of being written as zero.
use super::*;
use crysirror_scene::layer::queued::SequenceStatus;

pub const BYTES: usize = crate::dc3_settings::SLOT_BYTES;
const BASE: usize = 17704;
const OBJECT_BYTES: usize = 468;

struct Body(Vec<u8>);
impl Body {
    fn at(offset: usize) -> usize {
        offset - BASE
    }
    fn dword(&mut self, offset: usize, value: u32) {
        let at = Self::at(offset);
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn word(&mut self, offset: usize, value: u16) {
        let at = Self::at(offset);
        self.0[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn text(&mut self, offset: usize, capacity: usize, value: &str) -> Result<()> {
        let (bytes, _, unmappable) = encoding_rs::SHIFT_JIS.encode(value);
        ensure!(!unmappable, "DC3WY body string is not CP932: {value}");
        ensure!(
            bytes.len() < capacity && !bytes.contains(&0),
            "DC3WY body string exceeds its native {capacity}-byte field"
        );
        let at = Self::at(offset);
        self.0[at..at + bytes.len()].copy_from_slice(&bytes);
        Ok(())
    }
}

///437C40 object prefix (memset 0x1D4: dwords 0..116; its writes to dwords
///117..123, including the 437170 retained position, lie outside the saved
///record): floats 6/7/25 = 1, 26/27/55 = 100, 31/32 = the presentation
///extent, dword 37 = 1, then 4371D0(0,0,1024,576) into dwords 17..20. Dwords 31/32 come from the renderer's back-buffer
///extent (dword_4A954C+56/57), which this projection takes as the 1024x576
///canvas of the windowed profile.
fn fresh_object() -> [u8; OBJECT_BYTES] {
    let mut record = [0u8; OBJECT_BYTES];
    let mut float = |index: usize, value: f32| record[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    for index in [6, 7, 25] {
        float(index, 1.);
    }
    for index in [26, 27, 55] {
        float(index, 100.);
    }
    float(31, CANVAS[0] as f32);
    float(32, CANVAS[1] as f32);
    let mut int = |index: usize, value: u32| record[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    int(37, 1);
    int(19, CANVAS[0]);
    int(20, CANVAS[1]);
    record
}

impl Machine {
    ///Object 0 after 436F60 bound its background image and the shared actor
    ///settled. Dword 38 holds a native texture-pool handle; 41FCC0 rebinds it
    ///with 436F60 on load, so the projection stores 0 rather than a host id.
    fn object_record(
        &self,
        object: u16,
        extents: Option<([u32; 2], [u32; 2])>,
        actor: Option<&SequenceStatus>,
    ) -> Result<[u8; OBJECT_BYTES]> {
        let mut record = fresh_object();
        let mut lane_flags = [false; 2];
        let sprite = self.state.sprites.get(&object).cloned().unwrap_or_default();
        let mut float = |index: usize, value: f32| record[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        let mut current = sprite.position.map(f32::from);
        if let Some(status) = actor {
            current = [
                status.values[0] - f32::from(sprite.offset[0]),
                status.values[1] - f32::from(sprite.offset[1]),
            ];
            //A queued 43B110 motion keeps one map per axis (x: dword 57,
            //y: dword 58); 43B870 re-seeds a neutral record for set flags,
            //so a load leaves the object at its current position.
            if !status.complete {
                let final_position = sprite.position.map(f32::from);
                let mut active = [current[0] != final_position[0], current[1] != final_position[1]];
                for lane in status.lanes.values() {
                    for record in lane.records.values() {
                        if let crysirror_scene::layer::queued::SequenceTrack::Linear { channels, target, .. } = &record.track {
                            for (channel, value) in channels.iter().zip(target) {
                                if let Some(axis) = [0u8, 1].iter().position(|c| c == channel) {
                                    let offset = f32::from(sprite.offset[axis]);
                                    active[axis] |= value - offset != final_position[axis];
                                }
                            }
                        }
                    }
                }
                lane_flags = active;
            }
        }
        float(3, f32::from(sprite.offset[0]));
        float(4, f32::from(sprite.offset[1]));
        if let Some(scale) = sprite.scale {
            float(26, scale);
            float(27, scale);
        }
        float(29, sprite.pivot[0]);
        float(30, sprite.pivot[1]);
        //437850 zoom: dwords 6/7 fraction, 8/9 center, flag 10; 437830
        //flag 56 when the percent is not 100.
        if let Some((scale, center)) = sprite.zoom {
            float(6, scale / 100.);
            float(7, scale / 100.);
            float(8, center[0]);
            float(9, center[1]);
            float(10, f32::from_bits(1));
            float(56, f32::from_bits(u32::from(scale != 100.)));
        }
        let priority = sprite.priority.unwrap_or(if object == 0 { 0 } else { 50 });
        float(0, current[0]);
        float(1, current[1]);
        float(11, f32::from(sprite.position[0]));
        float(12, f32::from(sprite.position[1]));
        if let Some((allocation, logical)) = extents {
            let [aw, ah] = allocation.map(|value| value as f32);
            let [lw, lh] = logical.map(|value| value as f32);
            float(33, aw);
            float(34, ah);
            float(15, lw);
            float(16, lh);
            float(23, lw);
            float(24, lh);
            float(13, aw);
            float(14, ah);
            float(35, aw);
            float(36, ah);
            record[37 * 4..37 * 4 + 4].copy_from_slice(&1u32.to_le_bytes());
        }
        for (axis, set) in lane_flags.into_iter().enumerate() {
            if set {
                record[(57 + axis) * 4..(57 + axis) * 4 + 4].copy_from_slice(&1u32.to_le_bytes());
            }
        }
        record[115 * 4..116 * 4].copy_from_slice(&(priority as u32).to_le_bytes());
        Ok(record)
    }

    ///419E10's body for the current state. `extents` is the shared binding
    ///of slot 1; `actor` the shared object-0 sequence, when it exists.
    ///Bound objects (419EA0 of every named slot) with their shared extents
    ///and actor status, read at the snapshot.
    pub(super) fn bound_objects(
        &self,
        host: &mut dyn NativeHost,
    ) -> Result<BTreeMap<u16, (Option<([u32; 2], [u32; 2])>, Option<SequenceStatus>)>> {
        let mut objects = BTreeMap::new();
        let states = host.sprite_states()?;
        for &slot in self.state.image_names.keys() {
            let Some(object) = slot::resource_object(slot) else { continue };
            ensure!(matches!(object, 0 | 10 | 20 | 30 | 40 | 54 | 65), "DC3WY snapshot of object {object} requires its native record");
            let actor = states.iter().find(|(candidate, _)| *candidate == object).map(|(_, status)| status.clone());
            objects.insert(object, (host.image_dimensions(slot)?, actor));
        }
        Ok(objects)
    }

    pub(super) fn project_body(
        &self,
        resume: u32,
        objects: &BTreeMap<u16, (Option<([u32; 2], [u32; 2])>, Option<SequenceStatus>)>,
    ) -> Result<Vec<u8>> {
        let mut body = Body(vec![0; BYTES]);
        body.text(17712, 160, &self.state.scene)?;
        for (&slot, name) in &self.state.image_names {
            ensure!(slot < 36, "DC3WY image slot");
            if self.state.nameless.get(&slot) == Some(name) {
                continue;
            }
            body.text(17872 + usize::from(slot) * 48, 48, name)?;
        }
        let audio = &self.state.audio;
        body.text(19720, 32, &audio.music_name)?;
        body.dword(19752, audio.music_index as u32);
        for (index, name) in audio.channel_names.iter().enumerate() {
            body.text(19728 + 32 * (index + 1), 32, name)?;
        }
        body.dword(19888, u32::from(audio.channel_one_loop));
        body.dword(19908, resume);
        let (label, _, metadata) = self.state.savepoint.unwrap_or((0, 0, 0));
        body.dword(19912, u32::from(metadata));
        body.dword(19916, u32::from(label));
        body.dword(19920, resume);
        let history = &self.state.history;
        ensure!(history.entries.len() == 400 && history.differences.len() <= 1000, "DC3WY history extent");
        body.dword(23932, history.current as u32);
        for (index, entry) in history.entries.iter().enumerate() {
            let at = 23936 + index * 12;
            body.word(at, entry.scene as u16);
            body.word(at + 2, entry.diff_start);
            body.word(at + 4, entry.first);
            body.word(at + 6, entry.last);
            body.word(at + 8, entry.native_28756);
            body.word(at + 10, entry.native_28752);
        }
        body.dword(28740, u32::from(history.label));
        body.dword(28748, history.native_28748);
        body.dword(28752, history.native_28752);
        body.dword(28756, history.native_28756);
        body.dword(31964, u32::from(self.state.runtime_values.get(&233).copied().unwrap_or(0) != 0));
        body.dword(31968, history.differences.len() as u32);
        for (index, difference) in history.differences.iter().enumerate() {
            body.word(31972 + index * 2, difference.offset);
            body.0[Body::at(33972 + index)] = difference.value;
        }
        ensure!(self.state.script_bytes.len() == 1100 && history.mirror.len() == 1100, "DC3WY script bank extent");
        body.0[Body::at(34976)..Body::at(36076)].copy_from_slice(&self.state.script_bytes);
        body.0[Body::at(36076)..Body::at(37176)].copy_from_slice(&history.mirror);
        for object in 0..120usize {
            let record = match objects.get(&(object as u16)) {
                Some((extents, actor)) => self.object_record(object as u16, *extents, actor.as_ref())?,
                //Object 0 keeps its position even without a bound image.
                None if object == 0 => self.object_record(0, None, None)?,
                None => fresh_object(),
            };
            let at = Body::at(37332 + object * OBJECT_BYTES);
            body.0[at..at + OBJECT_BYTES].copy_from_slice(&record);
        }
        body.text(93512, 20, &self.state.protagonist_names[0])?;
        body.text(93532, 20, &self.state.protagonist_names[1])?;
        body.text(93636, 160, &audio.last_voice)?;
        ensure!(self.state.version.len() < 184 && !self.state.version.contains(&0), "DC3WY version string extent");
        let at = Body::at(94092);
        body.0[at..at + self.state.version.len()].copy_from_slice(&self.state.version);
        body.dword(94292, u32::from(self.state.resources.hold_background));
        body.dword(94296, u32::from(self.state.camera));
        body.dword(93608, u32::from(self.state.clouds));
        body.dword(93600, self.state.rain.unwrap_or(0) as u32);
        body.dword(94276, self.state.tint as u32);
        body.dword(93588, u32::from(self.state.fog));
        if let Some([kind, density]) = self.state.weather {
            body.dword(93592, kind as u32);
            body.dword(93596, density as u32);
        }
        body.text(93796, 260, &self.state.text.saved_first_line)?;
        body.text(94056, 36, &self.state.text.saved_speaker)?;
        body.text(94308, 260, &self.state.text.saved_body)?;
        Ok(body.0)
    }
}

impl Machine {
    ///4233A0 case 0x1A: 419E10 (when not held) snapshots the body with
    ///Buf4980 = the following literal's offset, then LABEL_85 skips that
    ///literal. On load 41FCC0 resumes at it (normally a 0x4D label jump).
    ///43E100's screen capture into this+1720 (the save-menu thumbnail) is
    ///not kept: no admitted consumer reads it before the next capture.
    pub(super) fn execute_snapshot(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x1a {
            return Ok(None);
        }
        let literal = program
            .instruction(instruction.next)
            .context("DC3WY 0x1A resume literal")?;
        ensure!(
            matches!(literal.arguments, Arguments::String(_)),
            "DC3WY 0x1A is followed by its resume literal"
        );
        let objects = self.bound_objects(host)?;
        let body = self.project_body(literal.offset, &objects)?;
        self.state.snapshot = Some(body);
        self.advance(literal.next, instruction.opcode).map(Some)
    }

    ///Dispatcher tail after a yield (LABEL_972): render the pending line,
    ///then the v397 snapshot. Called by the adapter at its frame fence.
    pub(crate) fn dispatch_tail(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        self.state.b_suppressed = false;
        if !self.render_text(host)? {
            return Ok(false);
        }
        if let Some(resume) = self.state.text_snapshot {
            let objects = self.bound_objects(host)?;
            self.state.snapshot = Some(self.project_body(resume, &objects)?);
            self.state.text_snapshot = None;
        }
        Ok(true)
    }

    ///Case 0x16 selector 4: 40B6A0(slot) writes the 0x1A snapshot after
    ///40B630 stamps the local time into it (slots below 110).
    pub(super) fn write_snapshot(&mut self, host: &mut dyn NativeHost, slot: i32) -> Result<bool> {
        let slot = u16::try_from(slot).context("DC3WY slot index")?;
        if let Some((pending, bytes)) = self.state.slot_write.clone() {
            ensure!(pending == slot, "DC3WY slot write continuation owner");
            return match host.resource(&crate::machine::resource_ops::ResourceRequest::WriteProfileSlot {
                profile: Profile::Dc3WithYou,
                slot,
                bytes,
            })? {
                None => Ok(false),
                Some(crate::machine::resource_ops::ResourceReply::Done) => {
                    self.state.slot_write = None;
                    Ok(true)
                }
                Some(_) => bail!("DC3WY slot write returned a foreign receipt"),
            };
        }
        let bytes = self
            .state
            .snapshot
            .clone()
            .context("DC3WY slot write before any 0x1A snapshot")?;
        let mut body = slot::Slot::try_from(bytes)?;
        if slot < 110 {
            let Some(crate::machine::system_ops::SystemReply::LocalTime(time)) =
                host.system(crate::machine::system_ops::SystemRequest::LocalTime)?
            else {
                return Ok(false);
            };
            let [year, month, day, hour, minute, second] = time;
            body.patch_clock(slot::Clock {
                month: u8::try_from(month)?,
                day: u8::try_from(day)?,
                hour: u8::try_from(hour)?,
                minute: u8::try_from(minute)?,
                year_in_century: u8::try_from(year.rem_euclid(100))?,
                second: u8::try_from(second)?,
            })?;
        }
        //The clock stamp is taken once; retries resend the same bytes.
        self.state.slot_write = Some((slot, body.bytes().to_vec()));
        self.write_snapshot(host, i32::from(slot))
    }
}
