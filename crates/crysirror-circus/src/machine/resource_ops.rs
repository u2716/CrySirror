//!4284B0 dispatcher-local resource declarations and447D20 rollback mapping.
use super::*;
use crysirror_scene::layer::queued::{SequenceOperation, SequenceStatus};
#[derive(Clone, Debug)]
pub enum ResourceRequest {
    ImageName { index: usize },
    CgIndex { name: String },
    BackgroundOverlay { name: String },
    PrepareLoad { slot: u8, name: String },
    RestoreAux { records: Vec<(u16, Vec<u8>)> },
    Alignment {seed:sprite_ops::SpriteSeed,maps:[crysirror_scene::layer::queued::ValueMap;2]},
    ///444750/447D20 copy geometry without replacing renderer188.
    PreserveTextureGeometry { object:u16, crop:[i32;4],allocation:[u32;2],reference:[f32;2] },
    AdvanceObjects { objects: Vec<u16>, fast: bool },
    Unload { slot: u8 },
    ResetStage,
    PruneShader { object: u16 },
    Offset { objects: Vec<u16>, offset: [f32; 2] },
    Visible { object: u16, visible: bool },
}
#[derive(Clone, Debug)]
pub enum ResourceReply {
    Done,
    Name(String),
    Index(Option<usize>),
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct ResourceState {
    pending: BTreeMap<u8, String>,
    #[serde(default)]
    prepared_names: BTreeSet<u8>,
    #[serde(default)]
    centered: BTreeSet<u8>,
    visibility: Option<i16>,
    publishing: bool,
    #[serde(default)]
    actor_fade_finished:Option<(String,u32,u8)>,
    #[serde(default)]
    current_controls: BTreeMap<u16, SequenceStatus>,
    #[serde(default)]
    rollback_controls: BTreeMap<u16, SequenceStatus>,
}
impl ResourceState {
    pub(super) fn clear_pending(&mut self, slot: u8) {
        self.pending.remove(&slot);
        self.prepared_names.remove(&slot);
        self.centered.remove(&slot);
    }
    pub(super) fn stage_unload(&mut self, slot: u8) {
        self.pending.insert(slot, "-".into());
        self.prepared_names.remove(&slot);
        self.centered.remove(&slot);
    }
    pub(super) fn remember_controls(&mut self, states: &[(u16, SequenceStatus)]) {
        self.current_controls
            .extend(states.iter().filter(|(n, _)| *n < 150).cloned());
    }
    pub(super) fn restore_dispatch(&mut self, states: &[(u16, SequenceStatus)]) {
        self.pending.clear();
        self.prepared_names.clear();
        self.centered.clear();
        self.visibility = None;
        self.actor_fade_finished=None;
        self.publishing = false;
        self.current_controls = states
            .iter()
            .filter(|(object, _)| *object < 150)
            .cloned()
            .collect();
    }
    pub(super) fn capture_rollback(&mut self) {
        self.rollback_controls = self.current_controls.clone();
    }
}
impl Machine {
    pub(crate) fn resource_publication_pending(&self) -> bool {
        self.state.resources.publishing
    }
    fn resource_done(
        &mut self,
        host: &mut dyn NativeHost,
        request: ResourceRequest,
    ) -> Result<bool> {
        let complete = host.resource(&request)?.is_some();
        if complete {
            if let ResourceRequest::Offset { objects, offset } = &request {
                self.state
                    .native_live
                    .resource_texture_offset(objects, *offset)?;
            }
        }
        Ok(complete)
    }
    pub(super) fn resource_load(
        &mut self,
        host: &mut dyn NativeHost,
        slot: u8,
        name: &str,
        bind: bool,
    ) -> Result<bool> {
        if name.starts_with('-') {
            if !self.resource_done(host, ResourceRequest::Unload { slot })? {
                return Ok(false);
            }
            self.state.native_live.unload_image(slot)?;
            self.state.image_names.remove(&slot);
            return Ok(true);
        }
        if bind && name.is_empty() {
            return Ok(true);
        }
        let replacing = host.image_dimensions(slot)?.is_some()
            && self
                .state
                .image_names
                .get(&slot)
                .is_none_or(|old| !old.eq_ignore_ascii_case(name));
        if !self.resource_done(
            host,
            ResourceRequest::PrepareLoad {
                slot,
                name: name.to_owned(),
            },
        )? {
            return Ok(false);
        }
        if !host.emit(&Event::LoadImage {
            slot,
            name: name.to_owned(),
        })? {
            return Ok(false);
        }
        self.state.native_live.load_image(slot, name)?;
        if replacing && (4..=8).contains(&slot) {
            self.state.image_names.remove(&(slot + 5));
        }
        self.state
            .image_names
            .insert(slot, name.to_ascii_uppercase());
        if bind {
            if let Some(object) = visual_ops::resource_object(slot) {
                let request = sprite_ops::SpriteResourceRequest {
                    bindings: vec![sprite_ops::SpriteResourceBinding {
                        slot,
                        object,
                        crop: None,
                    }],
                };
                let Some(reply) = host.sprite_resources(&request)? else {
                    return Ok(false);
                };
                self.state.native_live.bind_resource(
                    slot,
                    object,
                    None,
                    reply.dimensions.get(&object).copied().flatten(),
                )?;
            }
        }
        Ok(true)
    }
    ///41E7B0 literal filename policy. `_B` may intentionally be duplicated:
    ///the native noncompound previous-name path does not suppress it.
    pub(super) fn resource_variant(&self,slot:u8,name:&str)->String {
        let mut name=name.to_owned();
        let compound=name.contains(',');
        let mode=self.state.native_flags.get(&66508).copied().unwrap_or(0);
        if mode!=0&&((19..=23).contains(&slot)&&name.get(..4).is_some_and(|prefix|prefix.eq_ignore_ascii_case("bst\\"))||(4..=13).contains(&slot)) {
            let current=if name.contains('@'){2}else if name.contains('*'){1}else{0};
            if mode!=current {
                if let Some(at)=name.find('*'){name.replace_range(at..at+1,"@");}
                else if let Some(at)=name.find('@'){name.replace_range(at..at+1,"*");}
                else if matches!(mode,1|2) {if let Some(at)=name.find(','){name.insert(at,if mode==1{'*'}else{'@'});}}
            }
        }
        if !(9..=13).contains(&slot){return name}
        let previous=self.state.image_names.get(&(slot-5)).map(|name|name.to_ascii_uppercase()).unwrap_or_default();
        let (mut base,mut variant,mut changed)=(false,0,false);
        if previous.contains(',') {
            for (pattern,has_base,mark) in [("_B,",true,0),("_B*,",true,1),("_B@,",true,2),("*,",false,1),("@,",false,2)] {
                if previous.contains(pattern)&&!name.contains(pattern) {
                    changed=true;if has_base{base=true}if mark!=0{variant=mark}
                }
            }
            if !changed{return name}
        }else {
            if previous.ends_with('*'){variant=1;changed=true}
            if previous.ends_with('@'){variant=2;changed=true}
            base=variant==0&&previous.ends_with("_B")||variant!=0&&(previous.ends_with("_B*")||previous.ends_with("_B@"));
            if !base&&!changed{return name}
        }
        if variant!=0 {
            if let Some(at)=name.find(if variant==1{'*'}else{'@'}){name.remove(at);}
        }
        if compound {
            if let Some(at)=name.find(',') {
                let (prefix,suffix)=name.split_at(at);
                let mut result=prefix.to_owned();
                if base{result.push_str("_B")}
                if variant!=0{result.push(if variant==1{'*'}else{'@'})}
                result.push_str(suffix);name=result;
            }
        }
        name
    }
    ///431F20 types6..10 run41ED90 before moving the staged texture.
    ///The ASM deliberately reuses String scratch: in the non-composite path
    ///it is the uppercased old target (or the suffix after a removed marker),
    ///not an inferred "new filename plus inherited flags" implementation.
    ///40CA30 truncates the first extension separator and compares a complete
    ///NAMES image-table entry;40EA70 updates the shared system.dat CG byte.
    pub(super) fn native_cg_index(&mut self,host:&mut dyn NativeHost,name:&str)->Result<Option<Option<usize>>> {
        match host.resource(&ResourceRequest::CgIndex{name:name.to_owned()})? {
            Some(ResourceReply::Index(index))=>Ok(Some(index)),
            None=>Ok(None), _=>bail!("Circus CG lookup receipt"),
        }
    }
    pub(super) fn prepare_inline_transfer(&mut self,host:&mut dyn NativeHost,source:u8,target:u8)->Result<bool> {
        let old=self.state.image_names.get(&target).map(String::as_str).unwrap_or("");
        let staged=self.state.image_names.get(&source).map(String::as_str).unwrap_or("");
        let name=retarget_inline_name(old,staged)?;
        if name==staged {return Ok(true)}
        if name.is_empty() {
            if !self.resource_done(host,ResourceRequest::Unload{slot:source})? {return Ok(false)}
            self.state.native_live.unload_image(source)?;
            self.state.image_names.insert(source,String::new());
            return Ok(true)
        }
        self.resource_load(host,source,&name,false)
    }
    fn resource_rollback(
        &mut self,
        host: &mut dyn NativeHost,
        slot: u8,
        load_slot: u8,
    ) -> Result<bool> {
        let captured = self
            .state
            .saves
            .rollback
            .as_ref()
            .context("Circus resource rollback requires independent432AB0 mirror")?;
        let body = captured.observed_body();
        let name = body.native_string(19496 + usize::from(slot) * 48, 48)?;
        if name.is_empty() {
            return Ok(true);
        }
        let object =
            visual_ops::resource_object(slot).context("Circus rollback resource object")?;
        let record = body.sprite(usize::from(object))?.bytes().to_vec();
        let current = host
            .sprite_states()?
            .into_iter()
            .find(|(n, _)| *n == object)
            .map(|(_, s)| s)
            .or_else(|| self.state.resources.current_controls.get(&object).cloned());
        let mut candidate = self.state.native_live.clone();
        candidate.restore_object_record(object, &record)?;
        let seed = candidate.sprite_seed(object)?;
        let current = current.unwrap_or_else(|| SequenceStatus {
            generation: 0,
            revision: 0,
            values: seed.values.clone(),
            retained: seed.retained.clone(),
            lanes: BTreeMap::new(),
            capture: false,
            captured: BTreeMap::new(),
            repeats: 0,
            forever: false,
            complete: true,
        });
        let control_base = self
            .state
            .resources
            .rollback_controls
            .get(&object)
            .unwrap_or(&current);
        let mut control = crate::native_control::from_record(&record, control_base)?;
        let packed = name.get(..3).is_some_and(|s| s.eq_ignore_ascii_case("_p,"));
        if packed {
            if !host.packed(&packed_ops::PackedRequest::Load {
                slot: load_slot,
                name: name[3..].to_owned(),
            })? {
                return Ok(false);
            }
            let ticks = i32::from_le_bytes(record[916..920].try_into()?);
            let looping = current
                .lanes
                .get(&9)
                .and_then(|lane| lane.control.as_ref().or(lane.last_record.as_ref()))
                .is_some_and(|record| {
                    matches!(
                        record.track,
                        crysirror_scene::layer::queued::SequenceTrack::Frames { looping: true, .. }
                    )
                });
            if !host.packed(&packed_ops::PackedRequest::Play {
                slot: load_slot,
                seed: seed.clone(),
                position: [seed.values[0] as i16, seed.values[1] as i16],
                hold_ticks: i16::try_from(ticks).context("Circus rollback PCK frame interval")?,
                looping,
            })? {
                return Ok(false);
            }
            if let Some((_, prepared)) = host
                .sprite_states()?
                .into_iter()
                .find(|(n, _)| *n == object)
            {
                if let Some(lane) = prepared.lanes.get(&9) {
                    control.lanes.insert(9, lane.clone());
                }
            }
            self.state.image_names.insert(load_slot, name.clone());
            self.state.native_live.image_name(load_slot, &name)?;
        } else if !self.resource_load(host, load_slot, &name, false)? {
            return Ok(false);
        }
        if !host.sprite(&sprite_ops::SpriteRequest {
            commands: vec![sprite_ops::SpriteCommand {
                seed,
                operations: vec![SequenceOperation::RestoreControl {
                    state: control,
                    seed_missing: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 10],
                }],
            }],
        })? {
            return Ok(false);
        }
        if !self.resource_done(
            host,
            ResourceRequest::RestoreAux {
                records: vec![(object, record.clone())],
            },
        )? {
            return Ok(false);
        }
        self.state
            .native_live
            .restore_object_record(object, &record)?;
        if !packed {
            let Some(reply) = host.sprite_resources(&sprite_ops::SpriteResourceRequest {
                bindings: vec![sprite_ops::SpriteResourceBinding {
                    slot,
                    object,
                    crop: None,
                }],
            })?
            else {
                return Ok(false);
            };
            self.state.native_live.bind_resource(
                slot,
                object,
                None,
                reply.dimensions.get(&object).copied().flatten(),
            )?;
        }
        self.state.resources.pending.remove(&slot);
        self.sync_sprite_states(host)?;
        Ok(true)
    }
    pub(super) fn resource_copy_object(
        &mut self,
        host: &mut dyn NativeHost,
        source: u16,
        target: u16,
        slot: u8,
        advance: bool,
        preserve_texture: bool,
    ) -> Result<bool> {
        let original = self.state.native_live.object_record(target)?;
        if advance {
            if !self.resource_done(
                host,
                ResourceRequest::AdvanceObjects {
                    objects: vec![source],
                    fast: self.state.value(99, 214)? != 0 || self.state.value(99, 195)? != 0,
                },
            )? {
                return Ok(false);
            }
            self.sync_sprite_states(host)?;
            let samples = host.effect_states()?;
            self.sync_effect_frame(&samples)?;
        }
        let mut record = self.state.native_live.object_record(source)?;
        //444770 saves renderer[47] (texture pointer) before replacing geometry.
        if preserve_texture {
            record[180..184].copy_from_slice(&original[180..184]);
        }
        let mut candidate = self.state.native_live.clone();
        candidate.restore_object_record(target, &record)?;
        let seed = candidate.sprite_seed(target)?;
        let current = host
            .sprite_states()?
            .into_iter()
            .find(|(n, _)| *n == source)
            .map(|(_, state)| state)
            .or_else(|| self.state.resources.current_controls.get(&source).cloned())
            .unwrap_or_else(|| SequenceStatus {
                generation: 0,
                revision: 0,
                values: seed.values.clone(),
                retained: seed.retained.clone(),
                lanes: BTreeMap::new(),
                capture: false,
                captured: BTreeMap::new(),
                repeats: 0,
                forever: false,
                complete: true,
            });
        let state = crate::native_control::from_record(&record, &current)?;
        if !host.sprite(&sprite_ops::SpriteRequest {
            commands: vec![sprite_ops::SpriteCommand {
                seed,
                operations: vec![SequenceOperation::RestoreControl {
                    state,
                    seed_missing: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 10],
                }],
            }],
        })? {
            return Ok(false);
        }
        if !self.resource_done(
            host,
            ResourceRequest::RestoreAux {
                records: vec![(target, record.clone())],
            },
        )? {
            return Ok(false);
        }
        let f = |offset: usize| {
            f32::from_bits(u32::from_le_bytes(
                record[offset - 8..offset - 4].try_into().unwrap(),
            )) as i32
        };
        let crop = Some([f(96), f(100), f(104), f(108)]);
        if preserve_texture {
            if !self.resource_done(host,ResourceRequest::PreserveTextureGeometry{object:target,crop:crop.unwrap(),allocation:[f(176) as u32,f(180) as u32],reference:[f(72) as f32,f(76) as f32]})? {return Ok(false);}
        } else {
        let Some(reply) = host.sprite_resources(&sprite_ops::SpriteResourceRequest {
            bindings: vec![sprite_ops::SpriteResourceBinding {
                slot,
                object: target,
                crop,
            }],
        })?
        else {
            return Ok(false);
        };
        self.state.native_live.bind_resource(
            slot,
            target,
            crop,
            reply.dimensions.get(&target).copied().flatten(),
        )?;
        }
        self.state
            .native_live
            .restore_object_record(target, &record)?;
        Ok(true)
    }
    fn resource_actor_fade(
        &mut self,
        host: &mut dyn NativeHost,
        slot: u8,
        removing: bool,
    ) -> Result<bool> {
        use crysirror_scene::layer::queued::{DeltaSource, SequenceTrack};
        let object = visual_ops::resource_object(slot).unwrap();
        let previous = self
            .state
            .image_names
            .get(&slot)
            .cloned()
            .unwrap_or_default();
        let mut commands = Vec::new();
        let alpha = |to: f32| SequenceOperation::Enqueue {
            lane: 3,
            bank: None,
            track: SequenceTrack::Linear {
                channels: vec![2],
                target: vec![to],
                ticks: 15,
                velocity_divisor: None, ieee_inputs: false,
                references: vec![2],
                loop_origins: vec![2],
                delta: DeltaSource::CurrentAtZeroBank,
                looping: false,
                start: None,
                complete_first: false,
                complete_zero: true,
                fast: true,
                fast_channels: None,
                skip_zero: true,
                capture_zero: true,
            },
        };
        let (previous_bytes,_,encoding_error)=encoding_rs::SHIFT_JIS.encode(&previous);
        ensure!(!encoding_error,"Circus actor filename encoding");
        if removing || previous_bytes.len() >= 7 {
            let duplicate = object + 1;
            //The source is the frozen Buf mirror of the outgoing carrier,
            //not this dispatch's live primary object or current resource name.
            let record=self.state.native_live.snapshot_object_record(duplicate)?;
            let raw_name=&record[736..];
            let end=raw_name.iter().position(|byte|*byte==0).context("Circus outgoing snapshot filename terminator")?;
            let (name,error)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw_name[..end]);
            ensure!(!error,"Circus outgoing snapshot filename encoding");
            let copy=slot+42;
            if name.is_empty() {
                if !self.resource_done(host,ResourceRequest::Unload{slot:copy})? {return Ok(false)}
                self.state.native_live.unload_image(copy)?;
                self.state.image_names.remove(&copy);
            }else {
                if !self.resource_load(host,copy,&name,false)? {return Ok(false)}
                let owner=(self.state.scene.clone(),self.state.pc,slot);
                if self.state.resources.actor_fade_finished.as_ref()!=Some(&owner) {
                    if !host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{
                        seed:self.state.native_live.sprite_seed(duplicate)?,
                        operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}],
                    }]})? {return Ok(false)}
                    self.sync_sprite_states(host)?;
                    self.state.resources.actor_fade_finished=Some(owner);
                }
                let crop=std::array::from_fn(|axis|crate::machine::menu_ops::native_ftol(f64::from(f32::from_le_bytes(record[88+4*axis..92+4*axis].try_into().unwrap()))));
                let Some(reply)=host.sprite_resources(&sprite_ops::SpriteResourceRequest{bindings:vec![sprite_ops::SpriteResourceBinding{slot:copy,object:duplicate,crop:Some(crop)}]})? else{return Ok(false)};
                if reply.dimensions.get(&duplicate).copied().flatten().is_some() {
                    let mut candidate=self.state.native_live.clone();
                    candidate.restore_object_record(duplicate,&record)?;
                    let seed=candidate.sprite_seed(duplicate)?;
                    if !self.resource_done(host,ResourceRequest::RestoreAux{records:vec![(duplicate,record.clone())]})? {return Ok(false)}
                    self.state.native_live.restore_object_record(duplicate,&record)?;
                    //4284B0's3F call pushes-1 after the960-byte snapshot.
                    //447D20 passes it to447AC0;44E0F0(-1) returns-1 before
                    //any frame/queue publication. Keep the copied descriptor
                    //and static texture; do not invent a PCK restart here.
                    self.state.native_live.bind_resource(copy,duplicate,Some(crop),reply.dimensions.get(&duplicate).copied().flatten())?;
            let mut ops = Vec::new();
            let default_control=SequenceStatus{generation:0,revision:0,values:seed.values.clone(),retained:seed.retained.clone(),lanes:BTreeMap::new(),capture:false,captured:BTreeMap::new(),repeats:0,forever:false,complete:true};
            let current_states=host.sprite_states()?.into_iter().collect::<BTreeMap<_,_>>();
            let current=current_states.get(&duplicate).unwrap_or(&default_control);
            ops.push(SequenceOperation::RestoreControl {
                state:crate::native_control::from_record(&record,current)?,
                seed_missing:vec![0,1,2,3,4,5,6,7,8,10],
            });
            ops.push(SequenceOperation::Set {
                channels: vec![2],
                values: vec![1.],
                retained: false,
            });
            ops.push(alpha(0.));
            commands.push(sprite_ops::SpriteCommand {
                seed,
                operations: ops,
            });
                }
            }
        }
        if !removing {
            commands.push(sprite_ops::SpriteCommand {
                seed: self.state.native_live.sprite_seed(object)?,
                operations: vec![
                    SequenceOperation::Set {
                        channels: vec![2],
                        values: vec![0.],
                        retained: false,
                    },
                    alpha(1.),
                ],
            });
        }
        if !host.sprite(&sprite_ops::SpriteRequest { commands })? {
            return Ok(false);
        }
        self.sync_sprite_states(host)?;
        self.state.resources.actor_fade_finished=None;
        Ok(true)
    }
    fn resource_finish(&mut self, host: &mut dyn NativeHost, objects: Vec<u16>) -> Result<bool> {
        let commands = objects
            .into_iter()
            .map(|object| {
                Ok(sprite_ops::SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![SequenceOperation::Finish {
                        cancel: false,
                        lanes: None,
                    }],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !host.sprite(&sprite_ops::SpriteRequest { commands })? {
            return Ok(false);
        }
        self.sync_sprite_states(host)?;
        Ok(true)
    }
    pub(super) fn execute_resource(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        if !matches!(
            i.opcode,
            0x37 | 0x3c | 0x3d | 0x3f | 0x40 | 0x41 | 0x49 | 0x4a
        ) {
            return Ok(None);
        }
        let retry = Some(CommandResult::Retry);
        let mut cg_index = None;
        if matches!(i.opcode,0x49|0x4a) && !(text.starts_with(":n")||text.starts_with(":r")) {
            let name=if text.starts_with("--") {
                let slot=if i.opcode==0x49 {1}else{3};
                self.state.saves.rollback.as_ref()
                    .context("Circus resource rollback requires independent432AB0 mirror")?
                    .observed_body().native_string(19496+48*slot,48)?
            }else{text.to_owned()};
            let Some(index)=self.native_cg_index(host,&name)? else{return Ok(retry)};
            cg_index=index;
        }
        match i.opcode {
            0x37 => {
                self.state.resources.centered.insert(1);
                self.state.native_live.resource_center_declared();
            }
            0x3d => {
                ensure!((0..=5).contains(&a[0]), "Circus system resource index");
                let slot = a[0] as u8 + 37;
                let name = if text == "-" {
                    self.state
                        .image_names
                        .get(&slot)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    text.to_owned()
                };
                if !self.resource_load(host, slot, &name, false)? {
                    return Ok(retry);
                }
                self.state.native_live.resource_opcode_completed(0x3d)?;
            }
            0x3c => {
                if a[0] == 0 {
                    self.state.native_live.stage_image(0, "")?;
                    return Ok(Some(CommandResult::Complete {
                        next: i.next,
                        outcome: Outcome::Advanced,
                    }));
                }
                if (33..=36).contains(&a[0]) {
                    return Ok(None);
                }
                ensure!((1..=50).contains(&a[0]), "Circus resource declaration slot");
                let slot = a[0] as u8;
                let mut name = text.to_owned();
                if let Some(index) = name.strip_prefix('*') {
                    let index = index
                        .parse::<usize>()
                        .context("Circus indirect image index")?;
                    let Some(ResourceReply::Name(resolved)) =
                        host.resource(&ResourceRequest::ImageName { index })?
                    else {
                        return Ok(retry);
                    };
                    name = resolved;
                } else if name.starts_with("--") && slot <= 29 {
                    name = self
                        .state
                        .saves
                        .rollback
                        .as_ref()
                        .context("Circus3C rollback requires432AB0 mirror")?
                        .observed_body()
                        .native_string(19496 + usize::from(slot) * 48, 48)?;
                }
                if matches!(slot, 19..=23 | 26 | 27) {
                    let name = self.resource_variant(slot, &name);
                    // Positive4284B0 records19..23 in CWnd's temporary names;
                    //its raw resource table is unchanged until dispatcher tail.
                    if (19..=23).contains(&slot) && self.state.dispatch_a2 != 0 {
                        self.state.resources.pending.insert(slot, name);
                        self.state.resources.prepared_names.insert(slot);
                        self.state
                            .native_live
                            .completed_resource_declaration(slot)?;
                        return Ok(Some(CommandResult::Complete {
                            next: i.next,
                            outcome: Outcome::Advanced,
                        }));
                    }
                    if !self.resource_load(host, slot, &name, matches!(slot, 26 | 27))? {
                        return Ok(retry);
                    }
                    self.state
                        .native_live
                        .completed_resource_declaration(slot)?;
                } else {
                    if slot == 1 {
                        if name.contains("BG\\BLACK") || name.contains("BG\\WHITE") {
                            self.state.native_flags.insert(66512, 0);
                        }
                        if self.state.native_flags.get(&66512).copied().unwrap_or(0) == 0 {
                            if !self.resource_finish(host, (10..=63).chain([0]).collect())? {
                                return Ok(retry);
                            }
                            if !self.reset_resource_stage(host)? {
                                return Ok(retry);
                            }
                            let samples = host.effect_states()?;
                            self.sync_effect_frame(&samples)?;
                            self.sync_sprite_states(host)?;
                            let mut candidate = self.state.native_live.clone();
                            candidate.reset_object(0)?;
                            let seed = candidate.sprite_seed(0)?;
                            if !host.sprite(&sprite_ops::SpriteRequest {
                                commands: vec![sprite_ops::SpriteCommand {
                                    operations: vec![SequenceOperation::Reset {
                                        values: seed.values.clone(),
                                        retained: seed.retained.clone(),
                                    }],
                                    seed,
                                }],
                            })? {
                                return Ok(retry);
                            }
                        }
                        self.state.resources.pending.clear();
                        self.state.resources.prepared_names.clear();
                        self.state.resources.centered.clear();
                    }
                    // All remaining names are dispatcher-local declarations.
                    //Only slot1's acknowledged renderer reset writes save bytes.
                    self.state.native_live.stage_image(slot, &name)?;
                    self.state.resources.pending.insert(slot, name);
                }
            }
            0x3f => {
                ensure!((0..=4).contains(&a[0]), "Circus actor resource ordinal");
                let slot = a[0] as u8 + 4;
                let object = visual_ops::resource_object(slot).unwrap();
                if text.starts_with("--") {
                    if !self.resource_rollback(host, slot, slot)? {
                        return Ok(retry);
                    }
                } else {
                    if text == "-" {
                        self.state.resources.pending.insert(slot, "-".into());
                        self.state.resources.pending.insert(slot + 5, "-".into());
                    }
                    // The original optional crossfade is enabled explicitly by66584.
                    if self.state.value(99,183)? != 0
                        && !self.state.native_skipping()
                        && self.state.dispatch_a2 == 0
                        && self.state.native_flags.get(&25104).copied().unwrap_or(0) == 0
                        && self.state.native_flags.get(&15616).copied().unwrap_or(0) == 0
                    {
                        let staged = if text.starts_with('.') {
                            self.state
                                .resources
                                .pending
                                .get(&slot)
                                .map(String::as_str)
                                .unwrap_or("")
                        } else {
                            text
                        };
                        let previous = self
                            .state
                            .image_names
                            .get(&slot)
                            .map(String::as_str)
                            .unwrap_or("");
                        let prefix=|name:&str|->Result<[u8;7]>{
                            let (encoded,_,error)=encoding_rs::SHIFT_JIS.encode(name);
                            ensure!(!error,"Circus actor filename encoding");
                            let mut prefix=[0;7];let count=encoded.len().min(7);
                            prefix[..count].copy_from_slice(&encoded[..count]);Ok(prefix)
                        };
                        if text == "-" || prefix(staged)? != prefix(previous)?
                        {
                            if !self.resource_actor_fade(host, slot, text == "-")? {
                                return Ok(retry);
                            }
                        }
                    }
                    if text != "-"
                        && !self.resource_done(host, ResourceRequest::PruneShader { object })?
                    {
                        return Ok(retry);
                    }
                }
            }
            0x40 => {
                if text.starts_with("--") {
                    if !self.resource_rollback(host, 25, 25)? {
                        return Ok(retry);
                    }
                } else if text == "-n" {
                    self.state.resources.visibility = Some(a[0]);
                } else if matches!(text, "-a" | "-m") {
                    use crysirror_scene::layer::queued::{DeltaSource,SequenceTrack};
                    let queued=text=="-m"&&!self.state.native_skipping()&&self.state.dispatch_a2==0;
                    let operation=if queued {
                        SequenceOperation::Enqueue {lane:3,bank:None,track:SequenceTrack::Linear{
                            channels:vec![2],target:vec![1.],ticks:(12*i32::from(a[0])) as u32,velocity_divisor:None,ieee_inputs:false,
                            references:vec![2],loop_origins:vec![2],delta:DeltaSource::CurrentAtZeroBank,
                            looping:false,start:None,complete_first:false,complete_zero:true,fast:true,
                            fast_channels:None,skip_zero:true,capture_zero:true,
                        }}
                    }else{SequenceOperation::Set{channels:vec![2],values:vec![if text=="-a"{f32::from(a[0])}else{1.}],retained:false}};
                    if !host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{
                        seed:self.state.native_live.sprite_seed(63)?,operations:vec![operation],
                    }]})? {return Ok(retry)}
                    self.sync_sprite_states(host)?;
                } else if text == "-" {
                    if !self.resource_finish(host, vec![63])? {
                        return Ok(retry);
                    }
                    self.state.native_live.clear_resource_selectors(63)?;
                    self.state.resources.pending.insert(25, "-".into());
                } else if !self.resource_done(host, ResourceRequest::PruneShader { object: 63 })? {
                    return Ok(retry);
                }
            }
            0x41 => {
                ensure!((0..=4).contains(&a[0]), "Circus face resource ordinal");
                let slot = a[0] as u8 + 9;
                if text.starts_with("--") {
                    if !self.resource_rollback(host, slot, slot)? {
                        return Ok(retry);
                    }
                } else if text == "-" {
                    self.state.resources.pending.insert(slot, "-".into());
                }
            }
            0x49 | 0x4a => {
                self.state.callbacks.clear();
                if text.starts_with(":n") || text.starts_with(":r") {
                    let protected = text.starts_with(":n");
                    self.state.native_flags.insert(66512, i32::from(protected));
                    self.state.native_flags.insert(23908, i32::from(protected));
                    self.state.native_live.background_protection(protected);
                } else {
                    if !self.resource_finish(
                        host,
                        (0..150)
                            .filter(|n| matches!(n, 3 | 4 | 5 | 10..=63 | 66 | 88))
                            .collect(),
                    )? {
                        return Ok(retry);
                    }
                    if !self.resource_done(host, ResourceRequest::ResetStage)? {
                        return Ok(retry);
                    }
                    self.state.native_live.reset_stage(true)?;
                    self.state.native_live.resource_opcode_completed(i.opcode)?;
                    self.state.native_flags.insert(66517, 0);
                    self.state.builder = Group::default();
                    self.state.current_button = None;
                    self.state.menu_wait = false;
                    self.state.dialogue.inline_menu = false;
                    self.state.dialogue.gallery_wait = None;
                    self.state.dialogue.panel_wait = false;
                    self.state.dialogue.auto_advance = false;
                    self.state.wait_click = false;
                    self.state.wait_text_idle = None;
                    self.state.wait_menu_motion = false;
                    self.state.transition_wait = None;
                    self.state.transition_poll_at = None;
                    self.state.wait_animations = false;
                    self.state.assign(99, 246, 0)?;
                    for slot in (4..=13).chain([25, 31, 32]) {
                        self.state.image_names.remove(&slot);
                    }
                    if text.starts_with("--") {
                        let slot = if i.opcode == 0x49 { 1 } else { 3 };
                        if !self.resource_rollback(host, slot, 1)? {
                            return Ok(retry);
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
        if let Some(index)=cg_index { self.state.assign(97,i16::try_from(index)?,1)?; }
        if matches!(i.opcode, 0x3f | 0x40 | 0x41) {
            self.state.native_live.resource_opcode_completed(i.opcode)?;
        }
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
    ///40E9C0/0F9 commits the pending primary image before capturing it.
    pub(super) fn flush_framebuffer_background(&mut self,host:&mut dyn NativeHost)->Result<bool> {
        let Some(name)=self.state.resources.pending.get(&1).cloned() else{return Ok(true)};
        if !self.resource_load(host,1,&name,true)? {return Ok(false)}
        self.state.resources.clear_pending(1);
        Ok(true)
    }
    pub(super) fn flush_falling_resources(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        for slot in [28, 29] {
            let Some(name) = self.state.resources.pending.get(&slot).cloned() else {
                continue;
            };
            if !self.resource_load(host, slot, &name, true)? {
                return Ok(false);
            }
            self.state.resources.pending.remove(&slot);
        }
        Ok(true)
    }
    pub(crate) fn publish_resources(
        &mut self,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<bool> {
        self.state.resources.publishing = true;
        while let Some((&slot, name)) = self.state.resources.pending.first_key_value() {
            let name = if self.state.resources.prepared_names.contains(&slot) {
                name.clone()
            } else {
                self.resource_variant(slot, name)
            };
            if !self.resource_load(host, slot, &name, true)? {
                return Ok(false);
            }
            if slot == 1 {
                let Some(ResourceReply::Name(overlay)) =
                    host.resource(&ResourceRequest::BackgroundOverlay { name: name.clone() })?
                else {
                    return Ok(false);
                };
                if !self.resource_done(host, ResourceRequest::Unload { slot: 2 })? {
                    return Ok(false);
                }
                self.state.native_live.unload_image(2)?;
                if self.state.native_flags.get(&66517).copied().unwrap_or(0) != 0 {
                    if !self.set_resource_alignment(host,0,1)?{return Ok(false)}
                    if let Some((_, logical)) = host.image_dimensions(1)? {
                        let sx = (12800000u32 / logical[0]) as f32;
                        let sy = (7200000u32 / logical[1]) as f32;
                        let seed = self.state.native_live.sprite_seed(0)?;
                        if !host.sprite(&sprite_ops::SpriteRequest {
                            commands: vec![sprite_ops::SpriteCommand {
                                seed,
                                operations: vec![SequenceOperation::Set {
                                    channels: vec![3, 4],
                                    values: vec![sx, sy],
                                    retained: true,
                                }],
                            }],
                        })? {
                            return Ok(false);
                        }
                        self.sync_sprite_states(host)?;
                    }
                }
                if !overlay.is_empty() {
                    if !self.resource_copy_object(host, 0, 1, 1, false, false)? {
                        return Ok(false);
                    }
                    self.state.resources.pending.insert(2, overlay);
                    if self.state.resources.centered.contains(&1) {
                        self.state.resources.centered.insert(2);
                    }
                }
            }
            if self.state.resources.centered.contains(&slot) {
                if let Some(object) = visual_ops::resource_object(slot) {
                    let raw = self.state.native_live.object_record(object)?;
                    let f = |offset: usize| {
                        f32::from_bits(u32::from_le_bytes(
                            raw[offset - 8..offset - 4].try_into().unwrap(),
                        ))
                    };
                    let transformed = f(736) != 100.
                        || f(36) != 1.
                        || u32::from_le_bytes(raw[732..736].try_into().unwrap()) != 0;
                    if !transformed || self.state.dispatch_a2 != 0 {
                        if let Some((_, logical)) = host.image_dimensions(slot)? {
                            let position = [
                                (1280 - i32::try_from(logical[0])?) / 2,
                                (720 - i32::try_from(logical[1])?) / 2,
                            ];
                            let seed = self.state.native_live.sprite_seed(object)?;
                            if !host.sprite(&sprite_ops::SpriteRequest {
                                commands: vec![sprite_ops::SpriteCommand {
                                    seed,
                                    operations: vec![SequenceOperation::Set {
                                        channels: vec![0, 1],
                                        values: position.iter().map(|v| *v as f32).collect(),
                                        retained: true,
                                    }],
                                }],
                            })? {
                                return Ok(false);
                            }
                            self.state.native_live.center_resource(object, position)?;
                        }
                    }
                }
            }
            if (4..=8).contains(&slot) {
                if let Some((_, logical)) = host.image_dimensions(slot)? {
                    let x = if logical[0] == 1024 {
                        0
                    } else {
                        (1280 - i32::try_from(logical[0])?) / 2 - 128
                    };
                    if !self.resource_done(
                        host,
                        ResourceRequest::Offset {
                            objects: vec![10, 20, 30, 40, 50],
                            offset: [x as f32, 0.],
                        },
                    )? {
                        return Ok(false);
                    }
                }
            }
            if slot == 31 {
                if let Some((_, logical)) = host.image_dimensions(slot)? {
                    if !self.resource_done(
                        host,
                        ResourceRequest::Offset {
                            objects: vec![75],
                            offset: [0., 225. - logical[1] as f32],
                        },
                    )? {
                        return Ok(false);
                    }
                }
            }
            if (9..=13).contains(&slot) {
                let target = visual_ops::resource_object(slot).unwrap();
                if !self.resource_copy_object(host, target - 2, target, slot, true, true)? {
                    return Ok(false);
                }
            } else if slot == 32 {
                if !self.resource_copy_object(host, 75, 76, 32, true, true)? {
                    return Ok(false);
                }
            }
            self.state.resources.pending.remove(&slot);
            self.state.resources.prepared_names.remove(&slot);
        }
        if let Some(visible) = self.state.resources.visibility {
            if !self.resource_done(
                host,
                ResourceRequest::Visible {
                    object: 63,
                    visible: visible != 0,
                },
            )? {
                return Ok(false);
            }
            self.state.native_live.resource_visible(63,visible!=0)?;
            self.state.resources.visibility = None;
        }
        self.state.resources.centered.clear();
        self.state.resources.publishing = false;
        Ok(true)
    }
}

///Pinned41ED90/41ED90.asm byte-string policy. Literal48C79C=',';
///48C780='*,';48C77C='@,';48C6F4='@'. Preserve the original scratch reuse.
fn retarget_inline_name(old:&str,staged:&str)->Result<String> {
    let encode=|text:&str|->Result<Vec<u8>>{let(bytes,_,error)=encoding_rs::SHIFT_JIS.encode(text);ensure!(!error&&bytes.len()<48&&!bytes.contains(&0),"Circus inline image name extent/encoding");Ok(bytes.into_owned())};
    let old=encode(old)?;let mut new=encode(staged)?;let mut scratch=old.iter().map(u8::to_ascii_uppercase).collect::<Vec<_>>();
    let compound=old.contains(&b',');
    let contains=|bytes:&[u8],needle:&[u8]|bytes.windows(needle.len()).any(|window|window==needle);
    let mut base=false;let mut mode=0;
    if compound {
        if contains(&scratch,b"_B,"){base=true}
        if contains(&scratch,b"_B*,"){base=true;mode=1}
        if contains(&scratch,b"_B@,"){base=true;mode=2}
        if contains(&scratch,b"*,"){mode=1}
        if contains(&scratch,b"@,"){mode=2}
    }else{
        if scratch.last()==Some(&b'*'){mode=1}
        if scratch.last()==Some(&b'@'){mode=2}
        base=mode==0&&scratch.ends_with(b"_B")||mode!=0&&scratch.len()>=3&&scratch[..scratch.len()-1].ends_with(b"_B");
    }
    let marker=new.iter().position(|byte|*byte==b'@').or_else(||new.iter().position(|byte|*byte==b'*'));
    if let Some(at)=marker {scratch=new[at+1..].to_vec();new.remove(at);}
    if compound {
        let comma=new.iter().position(|byte|*byte==b',').context("Circus41ED90 composite old target requires composite staged source")?;
        let tail=new[comma+1..].to_vec();scratch=new[..comma].to_vec();
        if base {scratch.extend_from_slice(b"_B")}
        if mode==1{scratch.push(b'*')}else if mode==2{scratch.push(b'@')}
        scratch.push(b',');scratch.extend_from_slice(&tail);
    }else{
        if base {scratch.extend_from_slice(b"_B")}
        if mode==1{scratch.push(b'*')}else if mode==2{scratch.push(b'@')}
    }
    ensure!(scratch.len()<48,"Circus41ED90 result exceeds native fixed name");
    let(text,error)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&scratch);ensure!(!error,"Circus41ED90 result encoding");Ok(text.into_owned())
}

impl Machine {
    pub(super) fn set_resource_alignment(&mut self,host:&mut dyn NativeHost,object:u16,mode:u32)->Result<bool> {
        let mut record=self.state.native_live.object_record(object)?;
        record[956..960].copy_from_slice(&mode.to_le_bytes());
        let maps=crate::native_control::alignment_maps(&record)?;
        let seed=self.state.native_live.sprite_seed(object)?;
        if host.resource(&ResourceRequest::Alignment{seed,maps})?.is_none(){return Ok(false)}
        self.state.native_live.render_alignment(object,mode)?;Ok(true)
    }
}
