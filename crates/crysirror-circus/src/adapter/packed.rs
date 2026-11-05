//! Native pack slots map to shared seekable frame resources and retained queue
//! cursors. Frames decode on a worker before their owner tick can commit.
use super::*;
use crate::machine::{packed_ops::PackedRequest,sprite_ops::{SpriteRequest,SpriteCommand}};
use crysirror_scene::layer::queued::{SequenceOperation,SequenceStatus,SequenceTrack};
#[derive(Default)]
pub(super) struct PackedImages {
    pub(super) archives:BTreeMap<String,Arc<dyn Archive>>,
    slots:BTreeMap<u8,String>,
    //Client-scoped identities refer to immutable resource readers, not slots
    //or native pool/device addresses. Queue/snapshot owners retain old names.
    lineages:BTreeMap<u32,String>,
    generations:BTreeMap<String,u32>,
    next_lineage:u32,
    context:Option<DecodeContext>,
    active:BTreeMap<u16,Playback>,
    queue_names:BTreeMap<u16,BTreeMap<u8,String>>,
    activation_only:BTreeSet<u16>,
    stepped:BTreeSet<u16>,
    dimensions:BTreeMap<u16,([u32;2],[u32;2])>,
    images:BTreeMap<u16,UploadedImage>,
    decoding:Option<FrameDecode>,
    pub(super) preparing:bool,
}
//448CE0 releases the preceding188 texture and444710 allocates the next.
// Decoded frame pixels are retained only while a real drawable/snapshot owns
// them; a looping queue does not retain an unbounded cache of every frame.
struct FrameDecode {object:u16,name:String,index:i32,logical:[u32;2],job:crysirror_core::image_processing::ImageJob}
struct Playback {name:String,shown:i32,pending:Option<(i32,String,UploadedImage,[u32;2])>,crop:Option<[i32;4]>}
pub(super) fn open_pack(storage:&dyn ClientStorage,client:ClientId,name:&str)->Result<Arc<dyn Archive>>{
    let mut context=DecodeContext::new(client);
    context.limits.max_entry_size=context.limits.max_resident_bytes;
    open_pack_with_context(storage,&context,name)
}
pub(super) fn open_pack_with_context(storage:&dyn ClientStorage,context:&DecodeContext,name:&str)->Result<Arc<dyn Archive>>{
    let mut name=name.replace('\\',"/");if !name.to_ascii_lowercase().ends_with(".pck"){name.push_str(".PCK");}
    let path=VfsPath::new(Mount::Asset,&format!("Advdata/GRP/{name}"))?;
    //44F600 reads the entire pack into an immutable allocation. A retained
    //open file/range reader would incorrectly observe later SMB/HTTP/file edits.
    let bytes=crysirror_resource::read_all(storage.open_read(&path)?.as_ref(),context.limits.max_resident_bytes)?;
    let reader=context.memory(bytes)?;
    Ok(Arc::from(crysirror_resource::circus_frames::CircusFramePack.open(reader,context)?))
}
pub(super) fn decode_frame(archive:&dyn Archive,client:ClientId,index:u32)->Result<(ImageSurface,[u32;2])>{
    let bytes=crysirror_resource::read_all(archive.open(index,&DecodeContext::new(client))?.as_ref(),64*1024*1024)?;
    let image=crysirror_image::crx::decode(&bytes)?.surface;
    let logical=[image.width,image.height];
    let extent=[image.width.max(2).next_power_of_two(),image.height.max(2).next_power_of_two()];
    Ok((crysirror_image::padding::pad_to_extent(&image,extent,[0;4])?,logical))
}
fn layer(object:u16)->u32{0x3e00_0000+u32::from(object)}
fn actor(object:u16)->u32{0x3a04_0000+u32::from(object)}
impl Bridge<'_> {
    pub(super) fn clear_packed_presentation(&mut self)->Result<()> {
        let mut layers=Vec::new();
        for object in self.services.packed.active.keys().copied() {
            let id=layer(object);
            let current:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
            if !current.is_null(){layers.push(json!({"op":"remove","id":id}));}
        }
        self.submit(layers,vec![])?;
        let objects=self.services.packed.images.keys().copied().collect::<Vec<_>>();
        self.release_packed_images(&objects)?;
        self.services.packed.decoding=None;
        self.services.packed.active.clear();self.services.packed.dimensions.clear();self.services.packed.activation_only.clear();self.services.packed.stepped.clear();self.services.packed.preparing=false;Ok(())
    }
    pub(super) fn detach_reset_packed(&mut self,objects:&[u16])->Result<()> {
        let mut commands=Vec::new();
        for object in objects {
            if let Some(playback)=self.services.packed.active.get_mut(object) {
                if playback.shown>=0 {commands.push(json!({"op":"remove","id":layer(*object)}));}
                playback.shown=-1;playback.pending=None;
            }
            self.services.packed.dimensions.remove(object);
        }
        self.submit(commands,vec![])?;
        self.release_packed_images(objects)
    }
    pub(super) fn detach_completed_packed(&mut self)->Result<()> {
        for object in self.services.packed.active.keys().copied().collect::<Vec<_>>() {
            let current:Option<SequenceStatus>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(actor(object)))?)?;
            if current.is_some_and(|state|state.values[6]<0.) && self.services.packed.active[&object].shown>=0 {
                self.submit(vec![json!({"op":"remove","id":layer(object)})],vec![])?;
                self.release_packed_images(&[object])?;
                if let Some(playback)=self.services.packed.active.get_mut(&object){playback.shown=-1;}
            }
        }Ok(())
    }

}
impl PackedImages {
    pub(super) fn release_slot(&mut self,slot:u8){self.slots.remove(&slot);}
    pub(super) fn decoder_pending(&self)->bool{self.decoding.is_some()}
    pub(super) fn decode_context(&mut self,client:ClientId)->DecodeContext {
        self.context.get_or_insert_with(||{let mut context=DecodeContext::new(client);context.limits.max_entry_size=context.limits.max_resident_bytes;context}).clone()
    }
}
impl Bridge<'_> {
    fn release_packed_images(&mut self,objects:&[u16])->Result<()> {
        for object in objects {
            if let Some(image)=self.services.packed.images.remove(object){self.host.release_image_resource(self.client,image.path.relative())?;}
        }Ok(())
    }
    fn prepare_packed_image(&mut self,object:u16,name:&str,index:i32)->Result<Option<(UploadedImage,[u32;2])>>{
        use crysirror_core::image_processing::ImageProcessState;
        if self.services.packed.decoding.is_none(){
            let Some(Prepared::PackedFrame(spec,logical))=self.services.prepare(Work::PackedFrame(name.to_owned(),u32::try_from(index)?),self.client)?else{return Ok(None)};
            let job=self.host.image_metadata(self.client)?.begin_packed_pixels(spec)?;
            self.services.packed.decoding=Some(FrameDecode{object,name:name.to_owned(),index,logical,job});
        }
        let pending=self.services.packed.decoding.as_mut().unwrap();
        ensure!(pending.object==object&&pending.name==name&&pending.index==index,"Circus frame decode owner changed");
        let resource=match pending.job.poll()? {
            ImageProcessState::Pending=>return Ok(None),
            ImageProcessState::Failed(error)=>bail!("Circus frame image preparation failed: {error}"),
            ImageProcessState::Ready(resource)=>resource,
        };
        let logical=pending.logical;
        let image=self.host.publish_image_resource(self.client,0x3502_0000+u32::from(object),resource)?;
        self.services.packed.decoding=None;
        Ok(Some((image,logical)))
    }
    fn show_packed_image(&mut self,object:u16,image:&UploadedImage,logical:[u32;2])->Result<()> {
        self.visual_root(object)?;
        let root=visual::content_layer(object);
        let filter=self.services.visual.filters.get(&object).cloned().unwrap_or_else(||json!({"encoded_sampling":true}));
        let alias=sprite_resources::alias(object);
        let previous:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(alias))?)?;
        if !previous.is_null(){self.submit(vec![json!({"op":"remove","id":alias})],vec![])?;}
        let [l,t,r,b]=self.services.primary.resources.get(&object).map(|(_,crop)|*crop).or_else(||self.services.packed.active.get(&object).and_then(|playback|playback.crop)).unwrap_or([0,0,logical[0] as i32,logical[1] as i32]);
        let quad=crysirror_scene::layer::crop::texel_center_quad(image.size,logical.map(|value|value as f32),[l as f32,t as f32,r as f32,b as f32],[0.,0.])?;
        let size=quad.map(|quad|quad.size).unwrap_or([1.,1.]);let uv=quad.map(|quad|quad.uv);
        self.submit(vec![json!({"op":"set_visible","id":visual::object_layer(object),"visible":self.native_primary_visible(object,true)}),json!({"op":"put","layer":{"id":layer(object),"parent":root,"rect":[0,0,size[0],size[1]],"uv":uv,"visible":quad.is_some(),"sampling":{"address_u":"repeat","address_v":"repeat"},"filter":filter,"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
        if let Some(old)=self.services.packed.images.insert(object,image.clone()) {
            if old.path!=image.path {self.host.release_image_resource(self.client,old.path.relative())?;}
        }
        self.services.packed.dimensions.insert(object,(image.size,logical));Ok(())
    }
    pub(super) fn set_packed_source_crop(&mut self,object:u16,crop:[i32;4]) {if let Some(playback)=self.services.packed.active.get_mut(&object){playback.crop=Some(crop);}}
    pub(super) fn rebind_packed_image(&mut self,source:&str,image:&UploadedImage)->Result<()> {
        for current in self.services.packed.images.values_mut().filter(|current|current.path.relative()==source){*current=image.clone();}
        for playback in self.services.packed.active.values_mut(){
            if let Some((_,_,pending,_))=playback.pending.as_mut(){if pending.path.relative()==source{*pending=image.clone();}}
        }Ok(())
    }
    pub(super) fn native_packed_dimensions(&self,object:u16)->Option<([u32;2],[u32;2])>{self.services.packed.dimensions.get(&object).copied()}
    pub(super) fn native_packed_lineage(&self,object:u16)->Result<Option<u32>> {
        self.services.packed.active.get(&object).map(|playback|self.services.packed.generations.get(&playback.name).copied().context("Circus packed immutable lineage missing")).transpose()
    }
    pub(super) fn native_packed_frame_count(&self,slot:u8)->Result<Option<u32>>{
        self.services.packed.slots.get(&slot).map(|name|u32::try_from(self.services.packed.archives[name].entries().len()).map_err(Into::into)).transpose()
    }
    fn remember_packed_record(&mut self,object:u16,name:&str)->Result<()> {
        let state:Option<SequenceStatus>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(actor(object)))?)?;
        let lane=state.as_ref().and_then(|state|state.lanes.get(&9));
        let bank=u8::try_from(lane.map_or(0,|lane|lane.records.len()))?;
        let names=self.services.packed.queue_names.entry(object).or_default();
        names.retain(|bank,_|lane.is_some_and(|lane|lane.records.contains_key(bank)));
        names.entry(bank).or_insert_with(||name.to_owned());Ok(())
    }
    fn packed_cursor_name(&self,object:u16,state:&SequenceStatus)->Result<String>{
        if let Some(lane)=state.lanes.get(&9).filter(|lane|lane.active.is_some()) {
            return self.services.packed.queue_names.get(&object).and_then(|names|names.get(&(lane.cursor as u8))).cloned().context("Circus packed queue resource owner missing");
        }
        self.services.packed.active.get(&object).map(|playback|playback.name.clone()).context("Circus packed presentation owner missing")
    }
    pub(super) fn restore_packed_image(&mut self,slot:u8,object:u16,record:&[u8])->Result<bool>{
        let word=|offset:usize|->Result<u32>{Ok(u32::from_le_bytes(record.get(offset-8..offset-4).context("Circus packed restore record extent")?.try_into()?))};
        if word(956)?==0 {
            let current:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(layer(object)))?)?;
            if !current.is_null(){self.submit(vec![json!({"op":"remove","id":layer(object)})],vec![])?;}
            self.release_packed_images(&[object])?;
            if let Some(playback)=self.services.packed.active.get_mut(&object){playback.shown=-1;}return Ok(true)
        }
        let name=self.services.packed.slots.get(&slot).cloned().context("Circus restored pack slot missing")?;
        self.restore_packed_name(object,&name,record)
    }
    fn restore_packed_name(&mut self,object:u16,name:&str,record:&[u8])->Result<bool>{
        let name=name.to_owned();
        let word=|offset:usize|->Result<u32>{Ok(u32::from_le_bytes(record.get(offset-8..offset-4).context("Circus packed restore record extent")?.try_into()?))};
        let frames=u32::try_from(self.services.packed.archives[&name].entries().len())?;
        let Some((image,logical))=self.prepare_packed_image(object,&name,0)?else{return Ok(false)};
        let current:SequenceStatus=serde_json::from_slice::<Option<SequenceStatus>>(&self.host.query_engine(self.client,EngineQuery::LayerSequence(actor(object)))?)?.context("Circus packed restore primary controls missing")?;
        let mut crop=[0;4];for (index,offset) in [96,100,104,108].into_iter().enumerate(){crop[index]=crate::machine::menu_ops::native_ftol(f64::from(f32::from_bits(word(offset)?)));}
        if crop[2]==crop[0]||crop[3]==crop[1]{crop=[0,0,logical[0] as i32,logical[1] as i32];}
        self.remember_packed_record(object,&name)?;
        self.native_sprite(&SpriteRequest{commands:vec![SpriteCommand{seed:crate::machine::sprite_ops::SpriteSeed{object,values:current.values,retained:current.retained},operations:vec![
            SequenceOperation::Set{channels:vec![6,14,15],values:vec![0.,(crop[2]-crop[0]) as f32,(crop[3]-crop[1]) as f32],retained:false},
            SequenceOperation::Enqueue{lane:9,bank:None,track:SequenceTrack::Frames{channel:6,frames,hold_ticks:word(924)? as i32,looping:true,capture:false}},
        ]}]})?;
        self.services.packed.active.insert(object,Playback{name,shown:0,pending:None,crop:Some(crop)});
        self.show_packed_image(object,&image,logical)?;Ok(true)
    }
    pub(super) fn native_packed(&mut self,request:&PackedRequest)->Result<bool>{
        match request {
            PackedRequest::Load{slot,name}=>{
                if !self.services.packed.archives.contains_key(name){
                    let Some(Prepared::Packed(archive))=self.services.prepare(Work::PackedOpen(name.clone()),self.client)?else{return Ok(false)};
                    let generation=self.services.packed.next_lineage.checked_add(1).context("Circus pack lineage exhausted")?;
                    self.services.packed.next_lineage=generation;
                    self.services.packed.lineages.insert(generation,name.clone());
                    self.services.packed.generations.insert(name.clone(),generation);
                    self.services.packed.archives.insert(name.clone(),archive);
                }
                self.services.packed.slots.insert(*slot,name.clone());Ok(true)
            },
            PackedRequest::Play{slot,seed,position,hold_ticks,looping}=>{
                let Some(name)=self.services.packed.slots.get(slot).cloned()else{return Ok(true)};
                let frames=u32::try_from(self.services.packed.archives[&name].entries().len())?;
                let Some((image,logical))=self.prepare_packed_image(seed.object,&name,0)?else{return Ok(false)};
                let reset=crate::native_live::NativeLiveState::sprite_reset_seed(seed.object)?;
                let operations=vec![
                    SequenceOperation::Reset{values:reset.values,retained:reset.retained},
                    SequenceOperation::Set{channels:vec![0,1],values:vec![position[0] as f32,position[1] as f32],retained:true},
                    SequenceOperation::Set{channels:vec![6,14,15],values:vec![0.,logical[0] as f32,logical[1] as f32],retained:false},
                    SequenceOperation::Enqueue{lane:9,bank:None,track:SequenceTrack::Frames{channel:6,frames,hold_ticks:i32::from(*hold_ticks),looping:*looping,capture:false}},
                ];
                self.remember_packed_record(seed.object,&name)?;
                self.native_sprite(&SpriteRequest{commands:vec![SpriteCommand{seed:seed.clone(),operations}]})?;
                self.services.packed.active.insert(seed.object,Playback{name,shown:0,pending:None,crop:Some([0,0,logical[0] as i32,logical[1] as i32])});
                self.show_packed_image(seed.object,&image,logical)?;Ok(true)
            },
        }
    }
    pub(super) fn prepare_packed_tick(&mut self,fast:bool)->Result<bool>{
        self.services.packed.preparing=true;
        for object in self.services.packed.active.keys().copied().collect::<Vec<_>>() {
            if self.services.primary.paused.contains(&object)||self.services.effect_control.particle_wrapper(object){continue}
            let current:Option<SequenceStatus>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(actor(object)))?)?;
            let Some(current)=current else{continue};
            let next:Option<SequenceStatus>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequencePreview{id:actor(object),rate:current.values[16],fast})?)?;
            let next_state=next.context("Circus packed cursor owner missing")?;
            let activating=current.lanes.get(&9).is_none_or(|lane|lane.active.is_none())&&next_state.lanes.get(&9).is_some_and(|lane|lane.active.is_some());
            if activating {self.services.packed.activation_only.insert(object);continue}
            let next=next_state.values[6] as i32;
            if next<0 || next_state.lanes.get(&9).is_none_or(|lane|lane.active.is_none()) {continue}
            let stepped=current.lanes.get(&9).is_some_and(|lane|{
                let record=lane.control.as_ref().or_else(||lane.records.get(&(lane.cursor as u8)));
                matches!((lane.active.as_ref(),record.map(|record|&record.track)),
                    (Some(crysirror_scene::layer::queued::Active::Frames{counter,..}),Some(SequenceTrack::Frames{hold_ticks,..})) if i64::from(*counter)>i64::from(*hold_ticks))
            });
            if !stepped {continue}
            self.services.packed.stepped.insert(object);
            let name=self.packed_cursor_name(object,&next_state)?;
            let playback=&self.services.packed.active[&object];
            if next==playback.shown&&name==playback.name||playback.pending.as_ref().is_some_and(|pending|pending.0==next&&pending.1==name){continue}
            let Some((image,logical))=self.prepare_packed_image(object,&name,next)?else{return Ok(false)};
            self.services.packed.active.get_mut(&object).unwrap().pending=Some((next,name,image,logical));
        }self.services.packed.preparing=false;Ok(true)
    }
    pub(super) fn publish_packed_tick(&mut self)->Result<()> {
        for object in self.services.packed.active.keys().copied().collect::<Vec<_>>() {
            if self.services.primary.paused.contains(&object)||self.services.effect_control.particle_wrapper(object){continue}
            if self.services.packed.activation_only.remove(&object){continue}
            let state:Option<SequenceStatus>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(actor(object)))?)?;
            let Some(state)=state else{self.services.packed.active.remove(&object);continue};
            let index=state.values[6] as i32;
            if index<0 {
                if self.services.packed.active[&object].shown>=0 {self.submit(vec![json!({"op":"remove","id":layer(object)})],vec![])?;}
                self.release_packed_images(&[object])?;
                self.services.packed.active.get_mut(&object).unwrap().shown=-1;self.services.packed.stepped.remove(&object);continue
            }
            if !self.services.packed.stepped.remove(&object)||state.lanes.get(&9).is_none_or(|lane|lane.active.is_none()){continue}
            let name=self.packed_cursor_name(object,&state)?;
            if index==self.services.packed.active[&object].shown&&name==self.services.packed.active[&object].name{continue}
            let (expected,expected_name,image,logical)=self.services.packed.active.get_mut(&object).unwrap().pending.take().context("Circus frame dependency was not prepared before tick")?;
            ensure!(expected==index&&expected_name==name,"Circus frame dependency cursor changed");
            self.show_packed_image(object,&image,logical)?;
            let playback=self.services.packed.active.get_mut(&object).unwrap();playback.shown=index;playback.name=name;
        }Ok(())
    }
}
#[cfg(all(test,unix))]
mod tests {
    use super::*;
    use crysirror_core::service::AssetReader;
    use std::os::unix::fs::FileExt;
    struct ReadAt(std::fs::File);
    impl AssetReader for ReadAt {
        fn len(&self)->crysirror_core::Result<u64>{self.0.metadata().map(|m|m.len()).map_err(|e|crysirror_core::Error::Io(e.to_string()))}
        fn read_at(&self,offset:u64,out:&mut [u8])->crysirror_core::Result<usize>{self.0.read_at(out,offset).map_err(|e|crysirror_core::Error::Io(e.to_string()))}
    }
    #[test]
    #[ignore="requires licensed original frame packs via CIRCUS_FRAME_PACK_DIR"]
    fn installed_frames_decode_by_ordinal_without_reading_the_complete_pack(){
        let root=std::path::PathBuf::from(std::env::var_os("CIRCUS_FRAME_PACK_DIR").expect("CIRCUS_FRAME_PACK_DIR"));
        let context=DecodeContext::new(ClientId(77));let mut files=0;
        for entry in std::fs::read_dir(root).unwrap(){
            let path=entry.unwrap().path();if !path.extension().is_some_and(|s|s.eq_ignore_ascii_case("pck")){continue}
            let archive=crysirror_resource::circus_frames::CircusFramePack.open(Arc::new(ReadAt(std::fs::File::open(&path).unwrap())),&context).unwrap();
            let count=archive.entries().len();assert!(count>0);
            for ordinal in [0,count/2,count-1] {
                let (image,logical)=decode_frame(archive.as_ref(),ClientId(77),ordinal as u32).unwrap_or_else(|error|panic!("{} frame{ordinal}: {error:#}",path.display()));
                image.validate().unwrap();assert!(logical[0]>0&&logical[1]>0);
                assert!(image.width.is_power_of_two()&&image.height.is_power_of_two());
            }
            files+=1;eprintln!("{}: {count} indexed frames",path.file_name().unwrap().to_string_lossy());
        }assert!(files>0);
    }
}
