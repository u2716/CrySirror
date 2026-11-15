//! DC3WY message text: 4233A0 cases 0x61/0x62/0x06, the dispatcher tail and
//! process 42A160 case 6.
//!
//! 0x61 copies the speaker into this+11368 and 0x62 the body into
//! this+12392 (and Buf+94308); both pass through 41B410. 0x62 also clears
//! Buf+93796 and, without a speaker, the last voice name Buf+93636, copies
//! the speaker to Buf+94056 and the body up to its first '\n' to Buf+93796,
//! and sets v397: the dispatcher tail snapshots the body (419E10) with the
//! 0x62 offset as resume pc. 0x06 records last opcode 6 and yields unless
//! Buf23576 is set (cleared instead). At the yield the tail clears and sets
//! the speaker text of template this+10252 (Rust template 1) and the body of
//! template this+10248 (template 0) through 433BE0/435C00; the shared text
//! window owns layout, glyphs and reveal. Process case 6 then waits for
//! input; its advance clears this+11368..13416 (both buffers).
//!
//! 41B410 here: `$f`/`$n` take the protagonist names (Buf+93532/93512).
//! Its inline effect table (@w/@wb/@bt/@by/@bi/@bo/@mu/@se with @t timing and
//! @b1..4/@bc targets) and the `｛base／ruby｝` rewrite are not connected and
//! fail visibly. Process case 6's auto mode, skip mode and history paths are
//! not connected either; this waits for an explicit advance.
use super::*;
use crate::machine::text_ops::{TextReply, TextRequest};
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextState {
    pub speaker: String,
    pub body: String,
    ///Buf+94056/93796/94308 copies kept for the save body.
    pub saved_speaker: String,
    pub saved_first_line: String,
    pub saved_body: String,
    ///The 0x06 whose rendered line awaits its advance.
    pub wait: Option<u32>,
    pub rendered: bool,
    ///this+21772.. 41B410 event table (at most 10) and its start time.
    #[serde(default)]
    pub events: Vec<TextEvent>,
    #[serde(default)]
    pub events_start: u64,
    ///The advance's 41F1D0(1) flush is still running.
    #[serde(default)]
    pub flushing: bool,
    ///this+1880: when the line was dispatched (auto-mode timer).
    #[serde(default)]
    pub shown_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextEvent {
    ///@wb 1..5, @w 6..10, @mu 11, @se 12, @bt 13..16, @by 17..20, @bi
    ///21..24, @bo 25..28, @hy 29, @ht 30.
    pub kind: u8,
    pub at_ms: i64,
    ///@b1..4 -> slots 2..5, @bc -> 19; default 2.
    pub target: u8,
    ///@mu/@se operand (40BC90/40BCE0 index).
    pub number: i32,
}

///41B410's scan. Commands are removed from the text; each event's time is
///100*@t (the last @t, sticky) or, without one, (int)(22*(bytes/2) /
///(2*speed+16)) * 16.66 over the CP932 text before it. `$f`/`$n` insert the
///protagonist names (Buf+93532/93512). At most 10 events; afterwards an
///event equal in time and kind to its predecessor is removed by shifting
///the later kinds/times (not their targets) down, as the native loop does.
pub(crate) fn parse_markup(text: &str, names: &[String; 2], speed: i32) -> Result<(String, Vec<TextEvent>)> {
    let mut out = String::new();
    let mut events: Vec<TextEvent> = Vec::new();
    let mut rest = text;
    let mut timing = 0i64;
    let mut target = 2u8;
    let bytes = |value: &str| -> Result<usize> {
        let (encoded, _, unmappable) = encoding_rs::SHIFT_JIS.encode(value);
        ensure!(!unmappable, "DC3WY text is not CP932");
        Ok(encoded.len())
    };
    let digits = |value: &str| -> (i64, usize) {
        let length = value.bytes().take_while(u8::is_ascii_digit).count();
        (value[..length].parse().unwrap_or(0), length)
    };
    while !rest.is_empty() {
        if events.len() >= 10 {
            out.push_str(rest);
            break;
        }
        if let Some(tail) = rest.strip_prefix("$f") {
            out.push_str(&names[1]);
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("$n") {
            out.push_str(&names[0]);
            rest = tail;
            continue;
        }
        let at = |out: &str| -> Result<i64> {
            Ok(if timing != 0 {
                100 * timing
            } else {
                let width = 22 * (bytes(out)? as i64 >> 1) / (2 * i64::from(speed) + 16);
                (width as f64 * f64::from(16.66f32)) as i64
            })
        };
        let mut kind = None;
        for (prefix, base, count) in [("@wb", 1u8, 5u8), ("@bt", 13, 4), ("@by", 17, 4), ("@bi", 21, 4), ("@bo", 25, 4), ("@w", 6, 5)] {
            if let Some(tail) = rest.strip_prefix(prefix) {
                if let Some(digit) = tail.bytes().next().filter(|d| (b'1'..b'1' + count).contains(d)) {
                    kind = Some((base + digit - b'1', prefix.len() + 1, 0));
                    break;
                }
            }
        }
        if kind.is_none() {
            if rest.starts_with("@hy") {
                kind = Some((29, 3, 0));
            } else if rest.starts_with("@ht") {
                kind = Some((30, 3, 0));
            } else if let Some(tail) = rest.strip_prefix("@mu") {
                let (number, length) = digits(tail);
                kind = Some((11, 3 + length, number as i32));
            } else if let Some(tail) = rest.strip_prefix("@se") {
                let (number, length) = digits(tail);
                kind = Some((12, 3 + length, number as i32));
            }
        }
        if let Some((kind, length, number)) = kind {
            events.push(TextEvent { kind, at_ms: at(&out)?, target, number });
            rest = &rest[length..];
            continue;
        }
        if let Some(tail) = rest.strip_prefix("@t") {
            let (value, length) = digits(tail);
            timing = value;
            rest = &tail[length..];
            continue;
        }
        if let Some(digit) = rest.strip_prefix("@b").and_then(|tail| tail.bytes().next()).filter(|d| (b'1'..=b'4').contains(d)) {
            target = digit - b'1' + 2;
            rest = &rest[3..];
            continue;
        }
        if let Some(tail) = rest.strip_prefix("@bc") {
            target = 19;
            rest = tail;
            continue;
        }
        let character = rest.chars().next().unwrap();
        out.push(character);
        rest = &rest[character.len_utf8()..];
    }
    let mut count = events.len();
    let mut index = 1;
    while index < count {
        if events[index - 1].at_ms == events[index].at_ms && events[index - 1].kind == events[index].kind {
            for later in index..count - 1 {
                events[later].kind = events[later + 1].kind;
                events[later].at_ms = events[later + 1].at_ms;
            }
            count -= 1;
        }
        index += 1;
    }
    events.truncate(count);
    Ok((out, events))
}

///41B410 tail: for the first `｛A／B｝` (each marker located by strstr over the
///whole string), when A and B differ in CP932 byte length, A becomes `・`
///repeated until it is at least as long as B. 434D40 then draws A as 14 px
///ruby over B; the shared text window owns that layout.
fn ruby_rewrite(text: &str) -> Result<String> {
    let (Some(open), Some(separator), Some(close)) = (text.find('｛'), text.find('／'), text.find('｝')) else {
        return Ok(text.to_owned());
    };
    let start = open + '｛'.len_utf8();
    ensure!(start <= separator && separator < close, "DC3WY ruby marker order");
    let bytes = |value: &str| -> Result<usize> {
        let (encoded, _, unmappable) = encoding_rs::SHIFT_JIS.encode(value);
        ensure!(!unmappable, "DC3WY ruby text is not CP932");
        Ok(encoded.len())
    };
    let reading = &text[start..separator];
    let base = &text[separator + '／'.len_utf8()..close];
    let (reading_bytes, base_bytes) = (bytes(reading)?, bytes(base)?);
    if reading_bytes == base_bytes {
        return Ok(text.to_owned());
    }
    let mut padding = String::new();
    while padding.len() / '・'.len_utf8() * 2 < base_bytes {
        padding.push('・');
    }
    Ok(format!("{}{}{}", &text[..start], padding, &text[separator..]))
}

impl Machine {
    ///41B410: markup, inline events (replacing the table) and ruby.
    fn dc3_markup(&mut self, text: &str, now: u64) -> Result<String> {
        let speed = self.value(99, 26)?;
        let (text, events) = parse_markup(text, &self.state.protagonist_names, speed)?;
        self.state.text.events = events;
        self.state.text.events_start = now;
        ruby_rewrite(&text)
    }

    pub(super) fn execute_text(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x61 => {
                self.state.text.speaker = self.dc3_markup(text, now)?;
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            0x62 => {
                let body = self.dc3_markup(text, now)?;
                if self.state.text.speaker.is_empty() {
                    self.state.audio.last_voice.clear();
                }
                self.state.text.saved_body = text.to_owned();
                self.state.text.saved_speaker = self.state.text.speaker.clone();
                self.state.text.saved_first_line = body.split('\n').next().unwrap_or("").to_owned();
                self.state.text.body = body;
                self.state.text_snapshot = Some(instruction.offset);
                self.advance(instruction.next, instruction.opcode).map(Some)
            }
            0x06 => {
                let _ = a;
                let _ = host;
                self.state.text.wait = Some(instruction.offset);
                self.state.text.rendered = false;
                self.state.text.shown_at = Some(now);
                self.advance(instruction.next, instruction.opcode)?;
                Ok(Some(Outcome::WaitForInput))
            }
            _ => Ok(None),
        }
    }

    ///The dispatcher tail for last opcode 6: publish speaker and body into
    ///templates 1 and 0 (433BE0 + 435C00).
    pub(super) fn render_text(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if self.state.text.rendered || self.state.text.wait.is_none() {
            return Ok(true);
        }
        let split = |slot: u8| -> Option<(Group, Option<crate::machine::TextPart>)> {
            self.state.templates.get(&slot).map(|builder| {
                let mut group = builder.group.clone();
                let template = group.texts.pop();
                (group, template)
            })
        };
        let (body_group, body_template) = split(0).context("DC3WY message template this+10248 is absent")?;
        if crate::trace_enabled() {
            eprintln!("DC3 text template0 images={:?} texts={} slot28={:?}", body_group.images.iter().map(|i| (i.slot, i.position, i.source, i.resource_binding.clone())).collect::<Vec<_>>(), body_group.texts.len(), host.image_identity(28)?);
        }
        let name = split(1);
        let request = TextRequest::Dialogue {
            speaker: self.state.text.speaker.clone(),
            body: self.state.text.body.clone(),
            body_template,
            name_template: name.as_ref().and_then(|(_, template)| template.clone()),
            body_group: Some(body_group),
            name_group: name.map(|(group, _)| group),
            speed: self.value(99, 26)?,
        };
        if host.text(&request)?.is_none() {
            return Ok(false);
        }
        self.state.text.rendered = true;
        Ok(true)
    }

    ///41F1D0(force): run every event whose time has elapsed since 41B410
    ///(all of them when forced). Executed events are removed.
    fn run_text_events(&mut self, host: &mut dyn NativeHost, now: u64, force: bool) -> Result<bool> {
        while let Some(index) = self.state.text.events.iter().position(|event| {
            force || now.saturating_sub(self.state.text.events_start) as i64 >= event.at_ms
        }) {
            let event = self.state.text.events[index].clone();
            if crate::trace_enabled() {
                eprintln!("DC3 text event {event:?} images={:?}", self.state.image_names);
            }
            if !self.run_text_event(host, &event)? {
                return Ok(false);
            }
            self.state.text.events.remove(index);
        }
        Ok(true)
    }

    ///43AE10 then four 10-tick 43B110 moves to position+delta on `axis`
    ///(0 x, 1 y) for each object with a shared actor. Objects without one
    ///have no presentation and return to their position afterwards.
    pub(super) fn shake_objects(&mut self, host: &mut dyn NativeHost, objects: &[u16], axis: usize, deltas: [f32; 4]) -> Result<bool> {
        use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, linear};
        use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation};
        let mut commands = Vec::new();
        for &object in objects {
            let Some(sprite) = self.state.sprites.get(&object).filter(|sprite| sprite.actor).cloned() else { continue };
            let base = [
                f32::from(sprite.position[0]) + f32::from(sprite.offset[0]),
                f32::from(sprite.position[1]) + f32::from(sprite.offset[1]),
            ];
            let mut operations = vec![SequenceOperation::Finish { cancel: false, lanes: None }];
            for delta in deltas {
                let mut target = base;
                target[axis] += delta;
                operations.push(SequenceOperation::Enqueue {
                    lane: 0,
                    bank: None,
                    track: linear(vec![0, 1], target.to_vec(), 10, vec![0, 1], vec![0, 1], DeltaSource::Retained, false, true),
                });
            }
            commands.push(SpriteCommand { seed: self.object_seed(object), operations });
        }
        if commands.is_empty() {
            return Ok(true);
        }
        host.sprite(&SpriteRequest { commands })
    }

    fn run_text_event(&mut self, host: &mut dyn NativeHost, event: &TextEvent) -> Result<bool> {
        match event.kind {
            //432F20(name, 1, 0): one-shot sound, category 1.
            12 => {
                let Some(name) = host.sound_name(u16::try_from(event.number)?)? else { return Ok(false) };
                let declaration = crate::machine::media_ops::AudioDeclaration { name, voice: false, looping: false, voice_volume: None };
                Ok(host.publish_audio(&declaration)?.is_some())
            }
            //@bt / @by: 43AE10 then four 10-tick 43B110 moves of +-8 px.
            13..=20 => {
                if self.native_skipping() {
                    return Ok(true);
                }
                let object = 10 * (u16::from((event.kind - 13) % 4) + 1);
                let axis = if event.kind <= 16 { 1 } else { 0 };
                self.shake_objects(host, &[object], axis, [8., 0., -8., 0.])
            }
            //@w: object 419EA0(target) takes the slot 14..18 texture, whose
            //name the target slot then records (436F60 + name copy).
            6..=10 => {
                let source = 14 + (event.kind - 6);
                let Some(object) = crate::dc3::slot::resource_object(event.target) else { return Ok(true) };
                let name = self.state.image_names.get(&source).cloned().context("DC3WY @w source texture is absent")?;
                self.swap_object_texture(host, event.target, object, &name)
            }
            //@wb: object 0 takes the slot 14..18 texture as the background
            //(slot 1 handle and name), then 40D250 marks that CG seen.
            1..=5 => {
                let source = 14 + (event.kind - 1);
                let name = self.state.image_names.get(&source).cloned().context("DC3WY @wb source texture is absent")?;
                let cg = match host.resource(&ResourceRequest::CgIndex { name: name.clone() })? {
                    None => return Ok(false),
                    Some(ResourceReply::Index(index)) => index,
                    Some(_) => bail!("DC3WY CG lookup returned a foreign receipt"),
                };
                if !self.swap_object_texture(host, 1, 0, &name)? {
                    return Ok(false);
                }
                if let Some(index) = cg {
                    ensure!(index < 1200, "DC3WY CG index");
                    ensure!(self.state.system_data.set_byte(97, index as i16, 1)?, "DC3WY original CG-seen bank");
                }
                Ok(true)
            }
            //@hy/@ht: every object 0..42 (43AE10 + 43B110 x4).
            29 | 30 => {
                if self.native_skipping() {
                    return Ok(true);
                }
                let objects: Vec<u16> = (0..43).collect();
                self.shake_objects(host, &objects, if event.kind == 30 { 1 } else { 0 }, [8., 0., -8., 0.])
            }
            kind => bail!("DC3WY 41F1D0 inline event kind {kind} is not connected"),
        }
    }

    ///Process case 6: the rendered line waits for its advance.
    pub(super) fn poll_text_wait(&mut self, host: &mut dyn NativeHost, pressed: bool, now: u64) -> Result<Option<Outcome>> {
        if self.state.text.wait.is_none() {
            return Ok(None);
        }
        if !self.render_text(host)? {
            return Ok(Some(Outcome::Pending));
        }
        //41F1D0 runs once per process frame. An event whose shared work is
        //in flight retries on the next step (the VM owns that worker job);
        //events not yet due keep the frame clock running.
        if !self.run_text_events(host, now, false)? {
            return Ok(Some(Outcome::Pending));
        }
        if self.state.text.flushing {
            if !self.run_text_events(host, now, true)? {
                return Ok(Some(Outcome::Pending));
            }
            if !self.clear_dialogue_controls(host)? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.text.flushing = false;
            self.state.text.wait = None;
            self.state.text.speaker.clear();
            self.state.text.body.clear();
            return Ok(None);
        }
        //Skipping (41D820): process case 6 advances without input.
        let skipping = self.native_skipping();
        let mut pressed = pressed;
        if !skipping {
            if let Some(outcome) = self.poll_dialogue_controls(host, pressed, now)? {
                return Ok(Some(outcome));
            }
            pressed |= std::mem::take(&mut self.state.controls.advance);
        }
        let auto = !skipping && !pressed && self.auto_advance_due(host, now)?;
        if !pressed && !skipping && !auto {
            return Ok(Some(if self.state.text.events.is_empty() && self.value(99, 13)? == 0 {
                Outcome::WaitForInput
            } else {
                Outcome::WaitUntil(now.saturating_add(16))
            }));
        }
        match if skipping { Some(TextReply::Done) } else { host.text(&TextRequest::Input)? } {
            None => return Ok(Some(Outcome::Pending)),
            Some(TextReply::Input { advance: false }) => {
                //434210 completes the reveal; 41B390 keeps the latest event
                //and schedules it immediately.
                if let Some(latest) = self.state.text.events.iter().max_by_key(|event| event.at_ms).cloned() {
                    self.state.text.events = vec![TextEvent { at_ms: 0, ..latest }];
                }
                return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
            }
            Some(TextReply::Input { advance: true } | TextReply::Done) => {}
            Some(_) => bail!("DC3WY text wait received a foreign reply"),
        }
        if !self.run_text_events(host, now, true)? {
            self.state.text.flushing = true;
            return Ok(Some(Outcome::Pending));
        }
        if !self.clear_dialogue_controls(host)? {
            self.state.text.flushing = true;
            return Ok(Some(Outcome::Pending));
        }
        self.state.text.flushing = false;
        self.state.text.wait = None;
        self.state.text.speaker.clear();
        self.state.text.body.clear();
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn markup_events_follow_41b410_targets_and_times() {
        let names = ["N".to_string(), "F".to_string()];
        let (text, events) = super::parse_markup("「@b2@t23@w1比喩だよ」$f", &names, 5).unwrap();
        assert_eq!(text, "「比喩だよ」F");
        assert_eq!(events, vec![super::TextEvent { kind: 6, at_ms: 2300, target: 3, number: 0 }]);
        //Without @t: (int)(22*(bytes/2)/(2*5+16)) * 16.66 over the prefix.
        let (_, events) = super::parse_markup("あいうえお@bt1", &names, 5).unwrap();
        assert_eq!(events[0].at_ms, (((22 * 5) / 26) as f64 * f64::from(16.66f32)) as i64);
        assert_eq!(events[0].kind, 13);
    }
    #[test]
    fn ruby_rewrite_pads_unequal_readings_like_41b410() {
        assert_eq!(super::ruby_rewrite("もうすぐ｛サマーターム／夏学期｝が").unwrap(), "もうすぐ｛・・・／夏学期｝が");
        assert_eq!(super::ruby_rewrite("｛あい／愛子｝").unwrap(), "｛あい／愛子｝");
    }
}
