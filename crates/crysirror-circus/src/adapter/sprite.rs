//! Renderer-independent native declarations use the shared incremental actor.
use super::*;
#[path="sprite_masks.rs"]
mod masks;
use crate::machine::sprite_ops::{CHANNELS, SpriteRequest};
use crysirror_scene::layer::{
    motion::MotionProperty as Property,
    queued::{
        BindingTarget, SequenceBinding, SequenceOperation, SequenceSpec, SequenceStatus, ValueMap,
    },
};

#[derive(Default)]
pub(super) struct PrimarySprites {
    objects: BTreeSet<u16>,
    pub(super) paused:BTreeSet<u16>,
    ///None is a real transient image lease, not a native resource-table slot.
    pub(super) resources: BTreeMap<u16, (Option<u8>, [i32; 4])>,
    blur_seen: BTreeMap<u16, (Vec<u8>, bool)>,
    masks:BTreeMap<u16,masks::MaskQueues>,
}
fn actor(object: u16) -> u32 {
    0x3a04_0000 + u32::from(object)
}
impl Bridge<'_> {
    pub(super) fn native_sprite(&mut self, request: &SpriteRequest) -> Result<bool> {
        // Resource generations are prepared before acknowledging any sequence
        //mutation. A worker retry must not repeat Enqueue, Reset or Finish.
        if !self.sprite_mask_resources_ready()?||!self.prepare_forced_sprite_masks(request)?{return Ok(false)}
        // Validate the entire declaration before creating any owner.
        for command in &request.commands {
            ensure!(
                command.seed.object <= 150
                    && command.seed.values.len() == CHANNELS
                    && command.seed.retained.len() == CHANNELS,
                "Circus primary sprite declaration"
            );
        }
        let mut commands=Vec::new();
        let mut created=Vec::new();
        let mut reset_objects=Vec::new();
        for command in &request.commands {
            let object = command.seed.object;
            let root = if object == 150 {
                actor(object)
            } else {
                self.visual_root(object)?
            };
            let content = visual::content_layer(object);
            if !self.services.primary.objects.contains(&object) {
                let id = actor(object);
                let mut bindings = Vec::new();
                for (channel, layer, property, map) in [
                    (0, root, Property::X, ValueMap::Identity),
                    (1, root, Property::Y, ValueMap::Identity),
                    (
                        2,
                        content,
                        Property::Opacity,
                        ValueMap::Clamp {
                            low: 0.,
                            high: 1.,
                            scale: 255.,
                        },
                    ),
                    (
                        3,
                        root,
                        Property::ScaleX,
                        ValueMap::Ratio {
                            test_channels: vec![3, 4],
                            threshold: 1000.,
                            below: 100.,
                            above: 10000.,
                        },
                    ),
                    (
                        4,
                        root,
                        Property::ScaleY,
                        ValueMap::Ratio {
                            test_channels: vec![3, 4],
                            threshold: 1000.,
                            below: 100.,
                            above: 10000.,
                        },
                    ),
                    (
                        5,
                        root,
                        Property::RotationZ,
                        ValueMap::Affine {
                            scale: std::f32::consts::PI / 180.,
                            offset: 0.,
                        },
                    ),
                    (
                        12,
                        root,
                        Property::PivotX,
                        ValueMap::ZeroFallback {
                            channel: 14,
                            scale: 0.5,
                        },
                    ),
                    (
                        13,
                        root,
                        Property::PivotY,
                        ValueMap::ZeroFallback {
                            channel: 15,
                            scale: 0.5,
                        },
                    ),
                ] {
                    bindings.push(SequenceBinding {
                        layer,
                        channel,
                        target: BindingTarget::Motion { property },
                        map,
                    });
                }
                if object == 150 {
                    bindings.clear();
                }else{
                    //The bitmap filter derives UV coordinates from these
                    //channels. Undefined reciprocal inputs must suppress its
                    //drawable instead of reusing an earlier finite filter.
                    for channel in [10,11]{bindings.push(SequenceBinding{layer:root,channel,target:BindingTarget::FiniteGate,map:ValueMap::Identity});}
                }
                // A new owner starts with finite transport properties only.
                // RestoreControl in this same atomic batch installs the exact
                // IEEE CPU state and its proven source track; shared projection
                // suppression prevents the finite seed from becoming a draw.
                let restored=command.operations.iter().find_map(|operation|match operation {SequenceOperation::RestoreControl{state,..}=>Some(state),_=>None});
                let finite_seed=|values:&[f32]|->Result<Vec<f32>>{values.iter().enumerate().map(|(channel,value)| {
                    if value.is_finite(){Ok(*value)}else{ensure!(restored.is_some_and(|state|state.allows_nonfinite_channel(channel as u8)),"Circus new IEEE actor lacks restored track provenance");Ok(0.)}
                }).collect()};
                let spec = SequenceSpec {
                    values: finite_seed(&command.seed.values)?,
                    retained: Some(finite_seed(&command.seed.retained)?),
                    rate_channel: Some(16),
                    bindings,
                };
                commands.extend([json!({"op":"put","layer":{"id":id,"parent":SCENE_ROOT,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]}}}),json!({"op":"begin_sequence","id":id,"spec":spec})]);
                created.push(object);
            }
            if object<150&&command.operations.iter().any(|operation|matches!(operation,SequenceOperation::Reset{..})) {
                reset_objects.push(object);
                self.services.visual.filters.insert(object,json!({"encoded_sampling":true}));
                self.services.primary.resources.remove(&object);
                commands.push(json!({"op":"set_order","id":root,"order":visual::painter_order(object,50)}));
                commands.push(json!({"op":"set_viewport_clip","id":root,"clip":[0,0,1280,720]}));
                let image=sprite_resources::alias(object);
                let previous:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(image))?)?;
                if !previous.is_null(){commands.push(json!({"op":"remove","id":image}));}
            }
            for operation in &command.operations {
                commands.push(json!({"op":"sequence","id":actor(object),"operation":operation}));
                //445010 resets renderer964 independently of retained banks.
                if object<150&&matches!(operation,SequenceOperation::Reset{..}) {for channel in [0,1]{commands.push(json!({"op":"sequence","id":actor(object),"operation":SequenceOperation::SetBindingMap{layer:root,channel,map:ValueMap::Identity}}));}}
            }
        }
        self.submit(commands,vec![])?;
        self.services.primary.objects.extend(created);
        if !reset_objects.is_empty(){ensure!(self.native_effect_control(&crate::machine::effect_ops::EffectRequest::ResetObjects{objects:reset_objects.clone()})?,"Circus synchronous auxiliary reset receipt");self.detach_reset_packed(&reset_objects)?;}
        self.mirror_sprite_mask_operations(request)?;
        self.detach_completed_packed()?;
        for (object, status) in self.primary_sprite_states()? {
            if object == 150 {
                self.services.text.extra_sprite_rate = status.values[16];
            }
        }
        self.refresh_primary_filters()?;
        Ok(true)
    }
    fn refresh_primary_filters(&mut self) -> Result<()> {
        let mut commands = Vec::new();
        for (object, status) in self.primary_sprite_states()? {
            let Some(lane) = status.lanes.get(&10) else {
                continue;
            };
            let Some(record) = &lane.last_record else {
                continue;
            };
            let Some(_active) = &lane.last_active else {
                continue;
            };
            let signature = serde_json::to_vec(record)?;
            let busy = lane.active.is_some();
            let previous = self.services.primary.blur_seen.get(&object);
            let changed =
                previous.is_none_or(|(seen, was_busy)| *seen != signature || *was_busy != busy);
            if busy || changed {
                let target = match &record.track {
                    crysirror_scene::layer::queued::SequenceTrack::Linear { target, .. } => {
                        target[0]
                    }
                    _ => bail!("Circus blur actor track kind"),
                };
                let reset = !busy && lane.records.is_empty() && target == 1. / 1000.;
                let new_activation = busy
                    && previous.is_none_or(|(seen, was_busy)| *seen != signature || !*was_busy);
                let nine_tap = self
                    .services
                    .visual
                    .filters
                    .get(&object)
                    .is_some_and(|filter| filter["distortion"]["kind"] == "nine_tap");
                if !reset && !new_activation && !nine_tap {
                    self.services
                        .primary
                        .blur_seen
                        .insert(object, (signature, busy));
                    continue;
                }
                let offset = [status.values[10], status.values[11]];
                if !reset && !offset.iter().all(|value|value.is_finite()) {
                    //The native actor retains reciprocal/FDIV IEEE results.
                    //Never serialize them as JSON null or send invalid UVs to
                    //the GPU; keep the last completed finite filter binding.
                    self.services.primary.blur_seen.insert(object,(signature,busy));
                    continue;
                }
                let filter = if reset {
                    json!({"encoded_sampling":true})
                } else {
                    json!({"distortion":{"kind":"nine_tap","offset":offset,"center_weight":0.2,"neighbor_weight":0.1},"ignore_tint_alpha":true,"encoded_sampling":true})
                };
                self.services.visual.filters.insert(object, filter.clone());
                if let Some(slot) = crate::machine::visual_ops::object_resource(object) {
                    if self.services.displayed.contains(&slot) {
                        commands.push(json!({"op":"set_filter","id":0x3700_0000+u32::from(slot),"filter":filter}));
                    }
                }
                self.services
                    .primary
                    .blur_seen
                    .insert(object, (signature, busy));
            }
        }
        self.submit(commands, vec![])?;
        self.refresh_sprite_masks()?;
        Ok(())
    }
    pub(super) fn primary_sprite_states(&mut self) -> Result<Vec<(u16, SequenceStatus)>> {
        let mut states = Vec::new();
        for object in self.services.primary.objects.iter().copied() {
            let state = serde_json::from_slice::<Option<SequenceStatus>>(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(actor(object)))?,
            )?
            .context("Circus primary sprite owner disappeared")?;
            states.push((object, state));
        }
        Ok(states)
    }
    pub(super) fn advance_primary_sprites(&mut self, fast: bool) -> Result<()> {
        let states = self.primary_sprite_states()?;
        let commands = states
            .into_iter()
            .filter(|(object,_)|!self.services.primary.paused.contains(object)&&!self.services.effect_control.particle_wrapper(*object))
            .map(|(object, state)| {
                let operation = SequenceOperation::Advance {
                    generation: state.generation,
                    revision: state.revision + 1,
                    rate: state.values[16],
                    fast,
                };
                json!({"op":"sequence","id":actor(object),"operation":operation})
            })
            .collect();
        self.submit(commands, vec![])?;
        self.refresh_primary_filters()?;
        Ok(())
    }
}

impl Bridge<'_> {
    pub(in crate::adapter) fn set_primary_alignment_maps(&mut self,object:u16,maps:[ValueMap;2])->Result<()> {
        let root=self.visual_root(object)?;
        self.submit(maps.into_iter().enumerate().map(|(channel,map)|json!({"op":"sequence","id":actor(object),"operation":SequenceOperation::SetBindingMap{layer:root,channel:channel as u8,map}})).collect(),vec![])
    }
}

impl Bridge<'_> {
    ///Native zero pivots continue to refer to the CPU source extent, not the
    ///half-texel-adjusted draw size. Existing queue/pivot ownership is retained.
    pub(in crate::adapter) fn set_primary_source_extent(&mut self,object:u16,extent:[f32;2])->Result<()> {
        if self.services.primary.objects.contains(&object) {
            self.submit(vec![json!({"op":"sequence","id":actor(object),"operation":SequenceOperation::Set{channels:vec![14,15],values:extent.to_vec(),retained:false}})],vec![])?;
        }Ok(())
    }
}
