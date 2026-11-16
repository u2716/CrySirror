//!4233A0/41E8B0 native declarations. Image decoding, texture leases,
//! crop/bind, retained UI and fade sampling remain shared engine services.
use super::*;
use crate::machine::{
    effect_ops::EffectRequest,
    resource_ops::{ResourceReply, ResourceRequest},
    sprite_ops::{SpriteResourceBinding, SpriteResourceRequest},
};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Resources {
    pub pending: BTreeMap<u8, String>,
    pub hold_background: bool,
    pub background_mode: i32,
    pub render_flags: u8,
    //422F40 zeros its36 texture slots1588..1732;slot34 at1724 is
    //therefore genuinely absent before a native color/capture declaration.
    pub transition_color: Option<[u8; 3]>,
    ///0x36: v403[133], the pending slot-1 record's centre flag (+66).
    #[serde(default)]
    pub center_background: bool,
}
impl Machine {
    pub(super) fn execute_resources(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        match i.opcode {
            0x3a if a[0] == 1 => {
                ensure!(
                    !text.starts_with('*') && !text.starts_with("--"),
                    "DC3WY indirect/rollback background requires its original name receipt"
                );
                let raw = program
                    .literal_bytes(i.offset)
                    .context("DC3WY background filename")?;
                ensure!(raw.len() < 48, "DC3WY native48-byte background filename");
                //4377F0 on objects 10..40 tests the zoom fields (dword 55
                // != 100, dword 6 != 1, flag dword 56) that only the 0x93
                //camera (41A670/41AFE0) changes; 0x81's 4371F0 scale
                //(dwords 26/27) does not count.
                ensure!(
                    self.state.sprites.iter().filter(|(object, _)| **object != 0).all(|(_, sprite)| !sprite.queued),
                    "DC3WY background declaration with queued portrait motion requires its native fence"
                );
                if text.contains("BG\\BLACK") || text.contains("BG\\WHITE") {
                    self.state.resources.hold_background = false;
                }
                if !self.state.resources.hold_background
                    && [10u16, 20, 30, 40].iter().any(|object| self.state.sprites.get(object).and_then(|sprite| sprite.zoom).is_some_and(|(scale, _)| scale != 100.))
                {
                    //41AFE0(0, 100, 0, 0) + 41A670(0, 255, 0, 0, .., 100).
                    if !self.camera_zoom(host, 100, 0, 0)? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                if !self.state.resources.hold_background {
                    if !self.reset_background_actor(host)? {
                        return Ok(Some(Outcome::Pending));
                    }
                    let actor = self.state.sprites.get(&0).is_some_and(|sprite| sprite.actor);
                    //436F60 leaves the 437850 zoom fields untouched.
                    let zoom = self.state.sprites.get(&0).and_then(|sprite| sprite.zoom);
                    self.state.sprites.insert(0, Sprite { actor, zoom, ..Sprite::default() });
                }
                self.state.resources.pending.clear();
                self.state.resources.center_background = false; //memset(v403)
                self.state.resources.pending.insert(1, text.to_owned());
            }
            //4233A0 case 0x3A, slots 14..18: '?' becomes '1'+var(5,8)
            //(values above 2 use 0), then 41E7E0 loads immediately: release
            //the slot (41A4B0), load the upper-cased name, and record it in
            //the Buf+17872 name table. The ',' `_B` rewrite (word_47A5D0,
            //"_B,", '*'/'@' variants, local v374) is not connected.
            0x3a if (14..=18).contains(&a[0]) => {
                let digit = match self.value(5, 8)? {
                    value @ 0..=2 => b'1' + value as u8,
                    _ => b'1',
                };
                let name = text
                    .chars()
                    .map(|c| if c == '?' { char::from(digit) } else { c })
                    .collect::<String>();
                let name = if self.state.b_suppressed { name } else { b_rewrite(&name) }.to_ascii_uppercase();
                let slot = a[0] as u8;
                if !host.emit(&Event::LoadImage { slot, name: name.clone() })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.image_names.insert(slot, name);
            }
            //Case 0x3A, slots outside 1/14..18/20/21 (and 0): v403 records
            //published by the dispatcher tail through 41E8B0 ('-' releases).
            //Slots 22/23 are also published by 0x27. Slots bound to objects
            //(419EA0: 2..9/19/25) need portrait records and fail visibly.
            0x3a if !matches!(a[0], 0 | 1 | 14..=18 | 20 | 21) => {
                ensure!((0..36).contains(&a[0]), "DC3WY texture slot");
                let slot = a[0] as u8;
                ensure!(
                    crate::dc3::slot::resource_object(slot).is_none() || slot == 25 || (2..=5).contains(&slot),
                    "DC3WY 0x3A object slot {slot} (record copy) is not connected"
                );
                //Slots 2..5 and 25 replace '?' with '1'+var(5,8) like
                //14..18. The ',' `_B` rewrite targets 14..18 only.
                //4233A0's local v374: a 2..5 container name without a `_B`
                //variant suppresses the 14..18 rewrite for this dispatch.
                if (2..=5).contains(&slot) && text.contains(',') {
                    self.state.b_suppressed = !(text.contains("_B,") || text.contains("_B*,") || text.contains("_B@,"));
                }
                let name = if slot == 25 || (2..=5).contains(&slot) {
                    let digit = match self.value(5, 8)? {
                        value @ 0..=2 => b'1' + value as u8,
                        _ => b'1',
                    };
                    text.bytes().map(|byte| if byte == b'?' { char::from(digit) } else { char::from(byte) }).collect()
                } else {
                    text.to_owned()
                };
                self.state.resources.pending.insert(slot, name);
            }
            //0x5D: "-" marks the face record (slot 25) for release at the
            //tail (v403[1675] = "-", flag 1740); other operands do nothing.
            //Case 0x3A 20/21: 41E8B0 immediately (slot 20 binds object 54).
            0x3a if matches!(a[0], 20 | 21) => {
                if !self.publish_texture(host, a[0] as u8, text)? {
                    return Ok(Some(Outcome::Pending));
                }
            }
            0x5d => {
                if text == "-" {
                    self.state.resources.pending.insert(25, "-".into());
                }
            }
            0x43 => {
                if text.starts_with(":n") || text.starts_with(":r") {
                    let hold = text.as_bytes().get(1) == Some(&b'n');
                    self.state.resources.hold_background = hold;
                    self.state.resources.background_mode = i32::from(hold);
                    for label in &mut self.state.handlers.labels {
                        label.clear();
                    }
                    return self.advance(i.next, i.opcode).map(Some);
                }
                ensure!(
                    !text.starts_with("--"),
                    "DC3WY background rollback requires its retained original resource snapshot"
                );
                //420180: 43AE10 on objects 10..50, then 41A4B0 releases
                //texture slots 2..9, 19 and 25 (unbinding their objects);
                //object positions stay. Objects 3..5 (4388C0) and the
                //Buf23402 owner are never populated by connected producers.
                ensure!(
                    self.state.sprites.iter().filter(|(object, _)| **object != 0).all(|(_, sprite)| !sprite.queued),
                    "DC3WY background actor cleanup requires its native queue finish projection"
                );
                for slot in (2u8..=9).chain([19, 25]) {
                    if self.state.image_names.contains_key(&slot) && !self.publish_texture(host, slot, "-")? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                let cg = match host.resource(&ResourceRequest::CgIndex {
                    name: text.to_owned(),
                })? {
                    None => return Ok(Some(Outcome::Pending)),
                    Some(ResourceReply::Index(index)) => index,
                    Some(_) => bail!("DC3WY CG lookup returned a foreign receipt"),
                };
                if !host.emit(&Event::ClearUi)? {
                    return Ok(Some(Outcome::Pending));
                }
                if !host.effect_control(&EffectRequest::ResetColor)? {
                    return Ok(Some(Outcome::Pending));
                }
                let mut data = self.state.system_data.clone();
                if let Some(index) = cg {
                    ensure!(index < 1200, "DC3WY CG index");
                    ensure!(
                        data.set_byte(97, index as i16, 1)?,
                        "DC3WY original CG-seen bank"
                    );
                }
                self.state.system_data = data;
                self.state.builder = Builder::default();
                for label in &mut self.state.handlers.labels {
                    label.clear();
                }
                if !self.state.resources.hold_background {
                    self.state.camera = false; //Buf23574
                }
                //437170 takes the current background position as reference.
                let background = self.state.sprites.entry(0).or_default();
                background.retained_position = background.position;
                self.state.resources.transition_color = None;
            }
            0x36 => self.state.resources.center_background = true,
            0x14 => {
                let mut flags = a[0] as u8;
                let mut next = i.next;
                while let Ok(following) = program.instruction(next) {
                    if following.opcode != 0x14 {
                        break;
                    }
                    let Arguments::Bytes([flag, _]) = following.arguments else {
                        bail!("DC3WY frame flag width")
                    };
                    flags |= flag;
                    next = following.next;
                }
                self.state.resources.render_flags = flags;
                self.advance(next, i.opcode)?;
                return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
            }
            _ => return Ok(None),
        }
        self.advance(i.next, i.opcode).map(Some)
    }
    pub(crate) fn publish_resources(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        while let Some((&slot, name)) = self.state.resources.pending.first_key_value() {
            let name = name.clone();
            if slot != 1 {
                if !self.publish_texture(host, slot, &name)? {
                    return Ok(false);
                }
                continue;
            }
            match host.resource(&ResourceRequest::PrepareLoad {
                slot,
                name: name.clone(),
            })? {
                None => return Ok(false),
                Some(ResourceReply::Done) => {}
                Some(_) => bail!("DC3WY resource preparation returned a foreign receipt"),
            }
            if !host.emit(&Event::LoadImage {
                slot,
                name: name.clone(),
            })? {
                return Ok(false);
            }
            let Some(reply) = host.sprite_resources(&SpriteResourceRequest {
                bindings: vec![SpriteResourceBinding {
                    slot,
                    object: 0,
                    crop: None,
                }],
            })?
            else {
                return Ok(false);
            };
            let Some((_, logical)) = reply.dimensions.get(&0).copied().flatten() else {
                bail!("DC3WY loaded background lacks an acknowledged image extent")
            };
            //LABEL_972 +66: an untransformed object (4377F0) is centred on
            //the 1024x576 canvas with C truncating division, and 437170
            //retains that position.
            if self.state.resources.center_background {
                let position = [
                    (i32::try_from(logical[0])? - 1024) / -2,
                    (i32::try_from(logical[1])? - 576) / -2,
                ];
                if !self.place_background(host, position)? {
                    return Ok(false);
                }
                self.state.resources.center_background = false;
            }
            self.state
                .image_names
                .insert(slot, name.to_ascii_uppercase());
            self.state.sprites.entry(0).or_default().resource = Some(name);
            self.state.resources.pending.remove(&slot);
        }
        Ok(true)
    }
    ///41E8B0 for a slot without an object: '-' releases it (41A4B0),
    ///otherwise 41E7E0 loads the upper-cased name.
    pub(super) fn publish_texture(&mut self, host: &mut dyn NativeHost, slot: u8, name: &str) -> Result<bool> {
        if let Some(object) = crate::dc3::slot::resource_object(slot) {
            return self.publish_object_with(host, slot, object, name, true);
        }
        if name.starts_with('-') || name.is_empty() {
            if self.state.image_names.contains_key(&slot) {
                match host.resource(&ResourceRequest::Unload { slot })? {
                    None => return Ok(false),
                    Some(ResourceReply::Done) => {}
                    Some(_) => bail!("DC3WY texture release returned a foreign receipt"),
                }
            }
            self.state.image_names.remove(&slot);
        } else {
            let name = name.to_ascii_uppercase();
            if !host.emit(&Event::LoadImage { slot, name: name.clone() })? {
                return Ok(false);
            }
            self.state.image_names.insert(slot, name);
        }
        self.state.resources.pending.remove(&slot);
        Ok(true)
    }
    ///41E8B0 for an object slot: release ('-') or load, then 436F60 binds
    ///the texture to the object, keeping its position.
    ///`settle`: run the dispatcher tail's portrait offset/centring (not for
    ///a record restored by a "--" rollback).
    pub(super) fn publish_object_with(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, name: &str, settle: bool) -> Result<bool> {
        if name.starts_with('-') || name.is_empty() {
            if self.state.image_names.contains_key(&slot) {
                match host.resource(&ResourceRequest::Unload { slot })? {
                    None => return Ok(false),
                    Some(ResourceReply::Done) => {}
                    Some(_) => bail!("DC3WY object texture release returned a foreign receipt"),
                }
            }
            self.state.image_names.remove(&slot);
            self.state.sprites.entry(object).or_default().resource = None;
            self.state.resources.pending.remove(&slot);
            return Ok(true);
        }
        let name = if (2..=9).contains(&slot) { portrait::tint_rewrite(name, self.state.tint) } else { name.to_owned() };
        let name = name.to_ascii_uppercase();
        if self.state.image_names.get(&slot) != Some(&name) {
            if !host.emit(&Event::LoadImage { slot, name: name.clone() })? {
                return Ok(false);
            }
            self.state.image_names.insert(slot, name.clone());
        }
        let Some((_, logical)) = host.image_dimensions(slot)? else {
            return Ok(false);
        };
        //41E8B0: a "bst\" image takes 4371B0(-1, 0, w-2, h-1).
        let crop = name
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("BST\\"))
            .then(|| -> Result<[i32; 4]> {
                Ok([-1, 0, i32::try_from(logical[0])? - 2, i32::try_from(logical[1])? - 1])
            })
            .transpose()?;
        let Some(reply) = host.sprite_resources(&SpriteResourceRequest {
            bindings: vec![SpriteResourceBinding { slot, object, crop }],
        })?
        else {
            return Ok(false);
        };
        ensure!(
            reply.dimensions.get(&object).copied().flatten().is_some(),
            "DC3WY bound object texture lacks an acknowledged extent"
        );
        let placed = if settle && (2..=5).contains(&slot) {
            self.settle_portrait(host, slot, object, &name, logical)?
        } else {
            let position = self.state.sprites.get(&object).map_or([0, 0], |sprite| sprite.position);
            self.place_object(host, object, position.map(i32::from))?
        };
        if !placed {
            return Ok(false);
        }
        self.state.sprites.entry(object).or_default().resource = Some(name);
        self.state.resources.pending.remove(&slot);
        Ok(true)
    }
    ///@w/@wb: bind `object` to the image named in another slot and record
    ///that name for `slot`; the source rect and offset stay.
    pub(super) fn swap_object_texture(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, name: &str) -> Result<bool> {
        let loaded = host.emit(&Event::LoadImage { slot, name: name.to_owned() })?;
        if crate::trace_enabled() {
            eprintln!("DC3 swap slot={slot} object={object} name={name} loaded={loaded} dims={:?}", host.image_dimensions(slot)?);
        }
        if !loaded {
            return Ok(false);
        }
        self.state.image_names.insert(slot, name.to_owned());
        let Some((_, logical)) = host.image_dimensions(slot)? else { return Ok(false) };
        let crop = name
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("BST\\"))
            .then(|| -> Result<[i32; 4]> { Ok([-1, 0, i32::try_from(logical[0])? - 2, i32::try_from(logical[1])? - 1]) })
            .transpose()?;
        let Some(_) = host.sprite_resources(&SpriteResourceRequest {
            bindings: vec![SpriteResourceBinding { slot, object, crop }],
        })?
        else {
            return Ok(false);
        };
        self.state.sprites.entry(object).or_default().resource = Some(name.to_owned());
        Ok(true)
    }
    pub(super) fn release_scene_resources(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        //Staged v403 records are dispatcher locals: they survive 48/49 and
        //publish at the next yield, after this release.
        let slots = self
            .state
            .image_names
            .keys()
            .copied()
            .filter(|slot| !(28..=34).contains(slot))
            .collect::<Vec<_>>();
        //41EB70 releases every slot outside 28..34 through 41A4B0 (its
        //2->6, 3->7, 4->8, 5->9 chains only reach slots also released here).
        //420260(0) first runs 43AE10 on all 120 objects: the background
        //actor's queued motion completes before its image is released.
        if !self.finish_background_actor(host)? {
            return Ok(false);
        }
        //41EB70: 4377B0(0) on object 0 and 4377B0(50) on the others.
        let raised: Vec<u16> = self.state.sprites.iter().filter(|(_, sprite)| sprite.priority.is_some()).map(|(object, _)| *object).collect();
        for object in raised {
            let priority = if object == 0 { 0 } else { 50 };
            if !host.visual(&crate::machine::visual_ops::VisualRequest::Priority { object, priority })? {
                return Ok(false);
            }
            self.state.sprites.get_mut(&object).unwrap().priority = None;
        }
        //420260(0) ends with 4200F0(0): fog release and 41A5D0(0,0,0).
        self.state.fog = false;
        if self.state.weather.is_some() {
            if !host.emit(&Event::FallingEffect { kind: 0, parameter: 0 })? {
                return Ok(false);
            }
            self.state.weather = None;
        }
        ensure!(
            self.state.sprites.values().all(|sprite| !sprite.queued),
            "DC3WY scene release requires queued primary completion"
        );
        for slot in slots {
            match host.resource(&ResourceRequest::Unload { slot })? {
                None => return Ok(false),
                Some(ResourceReply::Done) => {}
                Some(_) => bail!("DC3WY scene release returned a foreign receipt"),
            }
        }
        Ok(true)
    }
}

///4233A0 case 0x3A for slots 14..18: a container name (`variant,crm`)
///whose variant lacks `_B` gets `_B` before the first '*' (else '@', else
///the ','), keeping that modifier, e.g. `A_1,C` -> `A_1_B,C`, `A*,C` ->
///`A_B*,C`.
fn b_rewrite(name: &str) -> String {
    let Some(comma) = name.find(',') else { return name.to_owned() };
    if name.contains("_B,") {
        return name.to_owned();
    }
    //The variant is cut at the modifier (or ','), "_B" and the modifier
    //character are appended, then the ',' tail.
    let modifier = name.find('*').or_else(|| name.find('@')).filter(|at| *at < comma);
    let cut = modifier.unwrap_or(comma);
    let marker = modifier.map_or("", |at| &name[at..at + 1]);
    format!("{}_B{}{}", &name[..cut], marker, &name[comma..])
}
