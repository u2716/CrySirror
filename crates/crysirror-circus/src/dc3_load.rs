//! DC3WY load: 4233A0 case 0x16 selector 0 (outside replay).
//!
//! With slot var(99,254) readable (40B8D0): 420260(0) finishes every object
//! queue, empties the weather pools and fog, releases every slot outside
//! 28..34 and clears the UI; 40BA30 reads the slot into the live body; the
//! body is kept as the 0x1A snapshot (Buf+94568); 420370 loads its scene;
//! 41FCC0(0,0) rebuilds every named slot (41E7E0, skipping 10..13) and, for
//! object slots (419EA0), restores the 468-byte object record (43B870) and
//! rebinds the texture (436F60); then weather (Buf23398/23399), the fog flag,
//! music (Buf+19720, looping, or a music stop), the last voice (Buf+93636),
//! and the SFX channels (one-shots first, then channel 1 when looping).
//! The decoder resumes at Buf4980 with last opcode 0. The speaker and the
//! first body line return to this+11368/12392 until the resumed 0x62.
//! Not connected (fail visibly): the replay-mode branch (Buf23572), named
//! slots 6..9 (record copies), and nonzero 0x27 2/13/15, 0x94, 23571,
//! 23574..23576 fields.
use super::*;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest};
use crate::machine::text_ops::TextRequest;
use crysirror_scene::layer::queued::SequenceOperation;

const BASE: usize = 17704;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadProgress {
    pub owner: u32,
    pub bytes: Option<Vec<u8>>,
    pub phase: u8,
    pub cursor: usize,
    ///0: slot load (0x16 0); 1: 0x1B return with operand 0 (420260(0)
    ///release, weather rebuilt, fade from the capture); 2: 0x1B with a
    ///nonzero operand (nothing released, no weather rebuild).
    #[serde(default)]
    pub mode: u8,
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn dword(&self, offset: usize) -> u32 {
        let at = offset - BASE;
        u32::from_le_bytes(self.0[at..at + 4].try_into().expect("bounded DC3WY body dword"))
    }
    fn word(&self, offset: usize) -> u16 {
        let at = offset - BASE;
        u16::from_le_bytes(self.0[at..at + 2].try_into().expect("bounded DC3WY body word"))
    }
    fn float(&self, offset: usize) -> f32 {
        f32::from_bits(self.dword(offset))
    }
    fn text(&self, offset: usize, capacity: usize) -> Result<String> {
        let at = offset - BASE;
        let field = &self.0[at..at + capacity];
        let end = field.iter().position(|byte| *byte == 0).context("DC3WY body string is unterminated")?;
        let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&field[..end]);
        ensure!(!errors, "DC3WY body string is not CP932");
        Ok(text.into_owned())
    }
}

impl Machine {
    pub(super) fn execute_load(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        let mut progress = match self.state.load.clone() {
            Some(progress) => {
                ensure!(progress.owner == instruction.offset, "DC3WY load continuation owner");
                progress
            }
            None => LoadProgress { owner: instruction.offset, bytes: None, phase: 0, cursor: 0, mode: 0 },
        };
        if progress.phase == 0 {
            let slot = u16::try_from(self.value(99, 254)?).context("DC3WY load slot")?;
            let reply = host.resource(&ResourceRequest::ReadProfileSlot { profile: Profile::Dc3WithYou, slot })?;
            let bytes = match reply {
                None => return Ok(Some(Outcome::Pending)),
                Some(ResourceReply::Bytes(bytes)) => bytes,
                Some(_) => bail!("DC3WY slot read returned a foreign receipt"),
            };
            //40B8D0 only opens the file; 40BA30 then needs the full body.
            let Some(bytes) = bytes.filter(|bytes| bytes.len() >= body::BYTES) else {
                self.state.load = None;
                return self.advance(instruction.next, instruction.opcode).map(Some);
            };
            progress.bytes = Some(bytes[..body::BYTES].to_vec());
            progress.phase = 1;
            self.state.load = Some(progress.clone());
        }
        let bytes = progress.bytes.clone().context("DC3WY load body")?;
        let reader = Reader(&bytes);
        if progress.phase == 1 {
            if progress.mode < 2 && !self.load_teardown(host)? {
                return Ok(Some(Outcome::Pending));
            }
            progress.phase = 2;
            self.state.load = Some(progress.clone());
        }
        if progress.phase == 2 {
            let scene = reader.text(17712, 160)?;
            if host.scene(&scene)?.is_none() {
                return Ok(Some(Outcome::Pending));
            }
            self.restore_body(&reader, &scene)?;
            self.state.snapshot = Some(bytes.clone());
            progress.phase = 3;
            progress.cursor = 0;
            self.state.load = Some(progress.clone());
        }
        if progress.phase == 3 {
            let slots: Vec<(u8, String)> = self.state.image_names.iter().map(|(slot, name)| (*slot, name.clone())).collect();
            while progress.cursor < slots.len() {
                let (slot, name) = &slots[progress.cursor];
                if !self.restore_slot(host, &reader, *slot, name)? {
                    self.state.load = Some(progress);
                    return Ok(Some(Outcome::Pending));
                }
                progress.cursor += 1;
                self.state.load = Some(progress.clone());
            }
            progress.phase = 4;
            self.state.load = Some(progress.clone());
        }
        if progress.phase == 4 {
            if let Some([kind, density]) = self.state.rain.map(|density| [2, density]).or(self.state.weather).filter(|_| progress.mode < 2) {
                if !host.emit(&Event::FallingEffect { kind, parameter: density })? {
                    return Ok(Some(Outcome::Pending));
                }
            }
            progress.phase = 7;
            self.state.load = Some(progress.clone());
        }
        //Buf23402: the cloud scroll restarts over the restored slot 21.
        if progress.phase == 7 {
            if self.state.clouds && progress.mode < 2 && !self.start_clouds(host)? {
                return Ok(Some(Outcome::Pending));
            }
            progress.phase = 5;
            self.state.load = Some(progress.clone());
        }
        if progress.phase == 5 {
            if progress.mode == 0 && self.state.audio.music_name.is_empty() && !host.emit(&Event::StopAudio { channel: 0 })? {
                return Ok(Some(Outcome::Pending));
            }
            //41FCC0's a2 (returns pass 1): music, the last voice and the
            //one-shot SFX names are replayed only by the slot load.
            let replay = progress.mode == 0;
            let audio = &mut self.state.audio;
            if !replay {
                for (index, name) in audio.channel_names.clone().iter().enumerate() {
                    if index == 0 && audio.channel_one_loop && !name.is_empty() {
                        audio.pending.insert(1, audio::PendingSound { name: name.clone(), voice: false, looping: true });
                    }
                }
            } else if !audio.music_name.is_empty() {
                audio.pending.insert(0, audio::PendingSound { name: audio.music_name.clone(), voice: false, looping: true });
            }
            if replay && !audio.last_voice.is_empty() {
                audio.pending.insert(7, audio::PendingSound { name: audio.last_voice.clone(), voice: true, looping: false });
            }
            for (index, name) in audio.channel_names.clone().iter().enumerate().filter(|_| replay) {
                if !name.is_empty() {
                    let looping = index == 0 && audio.channel_one_loop;
                    audio.pending.insert(index as u8 + 1, audio::PendingSound { name: name.clone(), voice: false, looping });
                }
            }
        }
        let resume = reader.dword(19920);
        if progress.mode == 1 {
            //0x1B with operand 0: 406690(1, 40D0F0(20)) from the capture.
            let ticks = self.adjusted_ticks(20)?;
            if ticks > 0
                && !host.effect_control(&crate::machine::effect_ops::EffectRequest::SlotFade { slot: 34, ticks })?
            {
                self.state.load = Some(progress);
                return Ok(Some(Outcome::Pending));
            }
            self.state.skip_latches = [0, 0];
        }
        self.state.load = None;
        self.state.pc = resume;
        self.state.last_offset = instruction.offset;
        self.state.observations = None;
        if progress.mode == 0 {
            self.state.last_opcode = Some(0);
            return Ok(Some(Outcome::WaitUntil(0)));
        }
        self.state.last_opcode = Some(instruction.opcode);
        self.state.resources.pending.clear();
        for label in &mut self.state.handlers.labels {
            label.clear();
        }
        Ok(Some(Outcome::Advanced))
    }

    ///420260(0) for the load.
    fn load_teardown(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let actors: Vec<u16> = self.state.sprites.iter().filter(|(_, sprite)| sprite.actor).map(|(object, _)| *object).collect();
        if !actors.is_empty() {
            let commands = actors
                .iter()
                .map(|object| SpriteCommand {
                    seed: self.object_seed(*object),
                    operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }],
                })
                .collect();
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(false);
            }
        }
        self.state.resources.pending.clear();
        if !self.release_scene_resources(host)? {
            return Ok(false);
        }
        if !host.emit(&Event::ClearUi)? {
            return Ok(false);
        }
        if host.text(&TextRequest::ClearDialogue)?.is_none() {
            return Ok(false);
        }
        //The loaded records carry their own 437850 zoom; drop the live one.
        for object in crate::dc3::motion::CAMERA {
            if self.state.sprites.get(&object).is_some_and(|sprite| sprite.zoom.is_some()) {
                if !host.visual(&crate::machine::visual_ops::VisualRequest::Zoom { object, scale: 100., center: [512., 288.] })? {
                    return Ok(false);
                }
                self.state.sprites.entry(object).or_default().zoom = None;
            }
        }
        self.state.image_names.retain(|slot, _| (28..=34).contains(slot));
        for sprite in self.state.sprites.values_mut() {
            sprite.resource = None;
        }
        Ok(true)
    }

    fn restore_body(&mut self, reader: &Reader<'_>, scene: &str) -> Result<()> {
        for (offset, what) in [(93604, "0x27 13"), (93612, "0x27 15"), (94284, "23571"), (94300, "23575"), (94304, "23576")] {
            ensure!(reader.dword(offset) == 0, "DC3WY slot field {what} is not connected");
        }
        self.state.scene = scene.to_owned();
        self.state.image_names.clear();
        self.state.nameless.clear();
        for slot in 0..36u8 {
            let name = reader.text(17872 + usize::from(slot) * 48, 48)?;
            if !name.is_empty() {
                ensure!(!(6..=9).contains(&slot), "DC3WY slot record copy {slot} is not connected");
                self.state.image_names.insert(slot, name);
            }
        }
        let audio = &mut self.state.audio;
        audio.music_name = reader.text(19720, 32)?;
        audio.music_index = reader.dword(19752) as i32;
        for index in 0..4 {
            audio.channel_names[index] = reader.text(19728 + 32 * (index + 1), 32)?;
        }
        audio.channel_one_loop = reader.dword(19888) != 0;
        audio.last_voice = reader.text(93636, 160)?;
        audio.pending.clear();
        let label = reader.dword(19916) as u16;
        self.state.savepoint = Some((label, u32::from(label), reader.dword(19912) as u16));
        let history = &mut self.state.history;
        history.current = reader.dword(23932) as usize;
        ensure!(history.current < 400, "DC3WY slot history index");
        for (index, entry) in history.entries.iter_mut().enumerate() {
            let at = 23936 + index * 12;
            entry.scene = reader.word(at) as i16;
            entry.diff_start = reader.word(at + 2);
            entry.first = reader.word(at + 4);
            entry.last = reader.word(at + 6);
            entry.native_28756 = reader.word(at + 8);
            entry.native_28752 = reader.word(at + 10);
        }
        history.label = reader.dword(28740) as u16;
        history.native_28748 = reader.dword(28748);
        history.native_28752 = reader.dword(28752);
        history.native_28756 = reader.dword(28756);
        let count = reader.dword(31968) as usize;
        ensure!(count <= 1000, "DC3WY slot difference count");
        history.differences = (0..count)
            .map(|index| history::Difference { offset: reader.word(31972 + index * 2), value: reader.0[33972 + index - BASE] })
            .collect();
        history.mirror = reader.0[36076 - BASE..37176 - BASE].to_vec();
        self.state.script_bytes = reader.0[34976 - BASE..36076 - BASE].to_vec();
        if reader.dword(31964) != 0 {
            self.state.runtime_values.insert(233, 1);
        } else {
            self.state.runtime_values.remove(&233);
        }
        self.state.protagonist_names = [reader.text(93512, 20)?, reader.text(93532, 20)?];
        self.state.fog = reader.dword(93588) != 0;
        self.state.tint = reader.dword(94276) as i32;
        let kind = reader.dword(93592) as i16;
        self.state.weather = (kind != 0).then(|| [kind, reader.dword(93596) as i16]);
        self.state.resources.hold_background = reader.dword(94292) != 0;
        self.state.camera = reader.dword(94296) != 0;
        self.state.clouds = reader.dword(93608) != 0;
        self.state.rain = Some(reader.dword(93600) as i16).filter(|density| *density != 0);
        self.state.resources.pending.clear();
        let version_at = 94092 - BASE;
        let end = reader.0[version_at..version_at + 184].iter().position(|byte| *byte == 0).context("DC3WY version string")?;
        self.state.version = reader.0[version_at..version_at + end].to_vec();
        self.state.text = text::TextState {
            speaker: reader.text(94056, 36)?,
            body: reader.text(93796, 260)?,
            saved_speaker: reader.text(94056, 36)?,
            saved_first_line: reader.text(93796, 260)?,
            saved_body: reader.text(94308, 260)?,
            ..Default::default()
        };
        self.state.sprites.clear();
        for object in 0..120u16 {
            let at = 37332 + usize::from(object) * 468;
            let sprite = self.state.sprites.entry(object).or_default();
            let offset = [reader.float(at + 12), reader.float(at + 16)];
            sprite.offset = [offset[0] as i16, offset[1] as i16];
            sprite.position = [reader.float(at) as i16, reader.float(at + 4) as i16];
            sprite.retained_position = sprite.position;
            let scale = reader.float(at + 104);
            sprite.scale = (scale != 100.).then_some(scale);
            sprite.pivot = [reader.float(at + 116), reader.float(at + 120)];
            //437850 zoom (dwords 6..10).
            sprite.zoom = (reader.dword(at + 40) != 0)
                .then(|| (reader.float(at + 24) * 100., [reader.float(at + 32), reader.float(at + 36)]))
                .filter(|(scale, _)| *scale != 100.);
            let priority = reader.dword(at + 460) as i16;
            sprite.priority = (priority != if object == 0 { 0 } else { 50 }).then_some(priority);
        }
        self.state.sprites.retain(|object, sprite| *object == 0 || sprite.position != [0, 0] || sprite.scale.is_some() || sprite.pivot != [0., 0.] || sprite.offset != [0, 0] || sprite.priority.is_some() || sprite.zoom.is_some());
        self.state.text_snapshot = None;
        self.state.menu_wait = None;
        self.state.layer_fence = None;
        self.state.wait_until = None;
        Ok(())
    }

    ///41FCC0 per slot: 41E7E0 (10..13 excluded) and, for an object slot,
    ///43B870 + 436F60 through the shared actor.
    fn restore_slot(&mut self, host: &mut dyn NativeHost, _reader: &Reader<'_>, slot: u8, name: &str) -> Result<bool> {
        if (10..=13).contains(&slot) {
            return Ok(true);
        }
        let Some(object) = slot::resource_object(slot) else {
            return host.emit(&Event::LoadImage { slot, name: name.to_owned() });
        };
        if !host.emit(&Event::LoadImage { slot, name: name.to_owned() })? {
            return Ok(false);
        }
        let Some((_, logical)) = host.image_dimensions(slot)? else { return Ok(false) };
        //The record keeps 41E8B0's "bst\\" source rect.
        let crop = name
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("BST\\"))
            .then(|| -> Result<[i32; 4]> { Ok([-1, 0, i32::try_from(logical[0])? - 2, i32::try_from(logical[1])? - 1]) })
            .transpose()?;
        if host
            .sprite_resources(&crate::machine::sprite_ops::SpriteResourceRequest {
                bindings: vec![crate::machine::sprite_ops::SpriteResourceBinding { slot, object, crop }],
            })?
            .is_none()
        {
            return Ok(false);
        }
        let sprite = self.state.sprites.get(&object).cloned().unwrap_or_default();
        if let Some((scale, center)) = sprite.zoom {
            if !host.visual(&crate::machine::visual_ops::VisualRequest::Zoom { object, scale, center })? {
                return Ok(false);
            }
        }
        if let Some(priority) = sprite.priority {
            if !host.visual(&crate::machine::visual_ops::VisualRequest::Priority { object, priority })? {
                return Ok(false);
            }
        }
        if !self.place_object(host, object, sprite.position.map(i32::from))? {
            return Ok(false);
        }
        self.state.sprites.entry(object).or_default().resource = Some(name.to_owned());
        let scale = sprite.scale.unwrap_or(100.);
        let operations = vec![
            SequenceOperation::Set { channels: vec![3, 4], values: vec![scale, scale], retained: true },
            SequenceOperation::Set { channels: vec![12, 13], values: sprite.pivot.to_vec(), retained: true },
        ];
        host.sprite(&SpriteRequest { commands: vec![SpriteCommand { seed: self.object_seed(object), operations }] })
    }
}
