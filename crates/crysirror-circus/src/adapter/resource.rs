//! Resource leases and retained drawable mutations for native dispatcher staging.
use super::*;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
use crate::machine::text_ops::TextRequest;
pub(super) fn load_resource_metadata(storage: &dyn ClientStorage) -> Result<Vec<(String, String)>> {
    let mut budget = crysirror_resource::pe::MAX_PE_BYTES as u64;
    let names = storage
        .list(Mount::Asset, "")?
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .collect::<Vec<_>>();
    ensure!(
        names.len() <= 32,
        "Circus resource metadata candidate budget"
    );
    for name in names {
        let path = VfsPath::new(Mount::Asset, &name)?;
        let metadata = storage
            .metadata(Mount::Asset, path.relative())?
            .context("Circus resource metadata source")?;
        if metadata.is_directory {
            continue;
        }
        let size = metadata.size.context("Circus resource metadata extent")?;
        ensure!(size <= budget, "Circus resource metadata budget");
        budget -= size;
        let bytes = crysirror_resource::read_all(storage.open_read(&path)?.as_ref(), size)?;
        let Ok(pe) = crysirror_resource::pe::PeData::parse(&bytes) else {
            continue;
        };
        let digest = pe
            .sha256()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>();
        if digest != "178ee9a5e34356b11942213e2e32a34afe4933c90d563bf5a3d6402250f54ab7" {
            continue;
        }
        let string = |address: u64| -> Result<String> {
            let mut bytes = Vec::new();
            for i in 0..260 {
                let b = pe.va_slice(address + i, 1)?[0];
                if b == 0 {
                    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes);
                    ensure!(!error, "Circus BG overlay encoding");
                    return Ok(text.into_owned());
                }
                bytes.push(b);
            }
            bail!("Circus BG overlay name terminator")
        };
        let mut mappings = Vec::new();
        for i in 0..128u64 {
            let row = pe.va_slice(0x4b4830 + 8 * i, 8)?;
            let prefix = u32::from_le_bytes(row[..4].try_into()?);
            if prefix == 0 {
                return Ok(mappings);
            }
            let overlay = u32::from_le_bytes(row[4..].try_into()?);
            ensure!(overlay != 0, "Circus BG overlay mapping");
            mappings.push((string(u64::from(prefix))?, string(u64::from(overlay))?));
        }
        bail!("Circus BG overlay mapping terminator")
    }
    bail!("Circus resource publication requires pinned native BG metadata")
}
impl Bridge<'_> {
    pub(super) fn native_resource(
        &mut self,
        request: &ResourceRequest,
    ) -> Result<Option<ResourceReply>> {
        match request {
            ResourceRequest::CgIndex { name } => {
                if self.services.names.is_none() {
                    let Some(Prepared::Names(names))=self.services.prepare(Work::Names,self.client)? else{return Ok(None)};
                    self.services.names=Some(names);
                }
                let prefix=name.split_once('.').map_or(name.as_str(),|(prefix,_)|prefix);
                let index=(!prefix.is_empty()).then(||self.services.names.as_ref().unwrap().images.iter()
                    .position(|candidate|candidate.eq_ignore_ascii_case(prefix))).flatten();
                return Ok(Some(ResourceReply::Index(index)));
            }
            ResourceRequest::ImageName { index } => {
                if self.services.names.is_none() {
                    let Some(Prepared::Names(names)) =
                        self.services.prepare(Work::Names, self.client)?
                    else {
                        return Ok(None);
                    };
                    self.services.names = Some(names);
                }
                return Ok(Some(ResourceReply::Name(
                    self.services
                        .names
                        .as_ref()
                        .unwrap()
                        .images
                        .get(*index)
                        .context("Circus indirect image name index")?
                        .clone(),
                )));
            }
            ResourceRequest::PrepareLoad { slot, name } => {
                if self.services.images.contains_key(slot)
                    && self
                        .services
                        .image_names
                        .get(slot)
                        .is_none_or(|previous| previous != name)
                {
                    if self
                        .native_resource(&ResourceRequest::Unload { slot: *slot })?
                        .is_none()
                    {
                        return Ok(None);
                    }
                }
            }
            ResourceRequest::BackgroundOverlay { name } => {
                if self.services.resource_metadata.is_none() {
                    let Some(Prepared::ResourceMetadata(table)) =
                        self.services.prepare(Work::ResourceMetadata, self.client)?
                    else {
                        return Ok(None);
                    };
                    self.services.resource_metadata = Some(table);
                }
                let name = name.to_ascii_lowercase();
                let overlay = self
                    .services
                    .resource_metadata
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find_map(|(prefix, overlay)| {
                        name.starts_with(&prefix.to_ascii_lowercase())
                            .then(|| overlay.clone())
                    })
                    .unwrap_or_default();
                return Ok(Some(ResourceReply::Name(overlay)));
            }
            ResourceRequest::RestoreAux { records } => self.restore_stage_objects(records)?,
            ResourceRequest::Alignment{seed,maps}=>{
                if !self.native_sprite(&crate::machine::sprite_ops::SpriteRequest{commands:vec![crate::machine::sprite_ops::SpriteCommand{seed:seed.clone(),operations:vec![]}]})?{return Ok(None)}
                self.set_primary_alignment_maps(seed.object,maps.clone())?;
            },
            ResourceRequest::PreserveTextureGeometry {object,crop,allocation,reference} => {
                //447D20 preserves the target's actual texture pointer. Looking
                //up the declared slot here would replace transferred aliases
                //or discard an already resident PCK frame.
                let mut id=super::sprite_resources::alias(*object);
                let mut layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
                if layer["paint"]["kind"]!="image" {
                    id=0x3e00_0000+u32::from(*object);
                    layer=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
                }
                if layer["paint"]["kind"]=="image" {
                    let [l,t,r,b]=*crop;
                    //447D20 copies allocation/logical geometry but preserves
                    //only renderer188's actual texture. Use the copied extent
                    //for UV normalization, not that texture's current slot.
                    let quad=crysirror_scene::layer::crop::texel_center_quad(*allocation,*reference,[l as f32,t as f32,r as f32,b as f32],[0.,0.])?;
                    let size=quad.map(|quad|quad.size).unwrap_or([1.,1.]);
                    layer["rect"]=json!([0,0,size[0],size[1]]);
                    layer["uv"]=json!(quad.map(|quad|quad.uv));
                    layer["visible"]=json!(quad.is_some());
                    layer["sampling"]=json!({"address_u":"repeat","address_v":"repeat"});
                    self.submit(vec![json!({"op":"put","layer":layer})],vec![])?;
                    if let Some((_,current))=self.services.primary.resources.get_mut(object) {*current=*crop;}
                    self.set_primary_source_extent(*object,[r as f32-l as f32,b as f32-t as f32])?;
                    if id==0x3e00_0000+u32::from(*object){self.set_packed_source_crop(*object,*crop);}
                }
            },
            ResourceRequest::AdvanceObjects { objects, fast } => {
                let states = self
                    .primary_sprite_states()?
                    .into_iter()
                    .collect::<BTreeMap<_, _>>();
                let mut commands = Vec::new();
                for object in objects {
                    if let Some(state) = states.get(object) {
                        commands.push(json!({"op":"sequence","id":0x3a04_0000+u32::from(*object),"operation":{"kind":"advance","generation":state.generation,"revision":state.revision+1,"rate":state.values[16],"fast":fast}}));
                    }
                }
                self.submit(commands, vec![])?;
                self.advance_stage_objects(objects, *fast)?;
            }
            ResourceRequest::PruneShader { object } => {
                if !self.prune_sprite_shader(*object)? {
                    return Ok(None);
                }
            }
            ResourceRequest::Unload { slot } => {
                let objects = self
                    .services
                    .primary
                    .resources
                    .iter()
                    .filter_map(|(object, (bound, _))| (*bound == Some(*slot)).then_some(*object))
                    .collect::<Vec<_>>();
                let held = self.services.images.contains_key(slot);
                if let Some(image) = self.services.images.remove(slot) {
                    // A copied/transferred native slot can still own the same
                    // texture even when this particular slot is unloaded.
                    if !self.services.images.values().any(|held|held.path==image.path){
                        self.retire_ui_image(image.path.relative())?;
                        self.host.release_image_resource(self.client,image.path.relative())?;
                    }
                    self.services.image_names.remove(slot);
                    self.services.image_extents.remove(slot);
                }
                let alias=0x3700_0000+u32::from(*slot);
                let transient_alias=self.services.primary.resources.iter()
                    .any(|(object,(owner,_))|owner.is_none()&&sprite_resources::alias(*object)==alias);
                if !transient_alias {self.services.displayed.remove(slot);}
                // Native release also accepts an already empty texture slot.
                // A retained actor may exist without a drawable image leaf.
                let current:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(alias))?)?;
                let mut commands=Vec::new();
                if !current.is_null()&&!transient_alias{commands.push(json!({"op":"remove","id":alias}));}
                for object in objects {
                    commands.push(json!({"op":"put","layer":{"id":sprite_resources::alias(object),"parent":visual::content_layer(object),"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]}}}));
                    self.services.primary.resources.remove(&object);
                }
                self.submit(commands, vec![])?;
                if held && (4..=8).contains(slot) {
                    if self
                        .native_resource(&ResourceRequest::Unload { slot: *slot + 5 })?
                        .is_none()
                    {
                        return Ok(None);
                    }
                }
            }
            ResourceRequest::Offset { objects, offset } => {
                self.set_stage_texture_offset(objects,*offset)?;
            }
            ResourceRequest::Visible { object, visible } => {
                let id = self.visual_root(*object)?;
                self.submit(
                    vec![json!({"op":"set_visible","id":id,"visible":self.native_primary_visible(*object,*visible)})],
                    vec![],
                )?;
            }
            ResourceRequest::ResetStage => {
                if !self
                    .native_effect_control(&crate::machine::effect_ops::EffectRequest::Finish)?
                {
                    return Ok(None);
                }
                if !self.falling_effect(0, 0)? {
                    return Ok(None);
                }
                if self.native_text(&TextRequest::ClearUi)?.is_none() {
                    return Ok(None);
                }
                if self.native_text(&TextRequest::ClearDialogue)?.is_none() {
                    return Ok(None);
                }
                for slot in (4..=13).chain([25, 31, 32]) {
                    if self
                        .native_resource(&ResourceRequest::Unload { slot })?
                        .is_none()
                    {
                        return Ok(None);
                    }
                }
                //425C20 detaches these private GUI/transition carriers, not the background.
                let mut commands = Vec::new();
                for object in [3u16, 4, 5, 88] {
                    if self.services.primary.resources.remove(&object).is_some() {
                        commands.push(json!({"op":"remove","id":sprite_resources::alias(object)}));
                    }
                }
                self.submit(commands, vec![])?;
                if self.primary_sprite_states()?.iter().any(|(object,_)|*object==0){self.set_primary_alignment_maps(0,[crysirror_scene::layer::queued::ValueMap::Identity,crysirror_scene::layer::queued::ValueMap::Identity])?;}
            }
        }
        Ok(Some(ResourceReply::Done))
    }
}
