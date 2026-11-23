// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! 660110 PANEL composition. Native part/number/placement policy feeds shared
//! immutable image jobs and font raster jobs; this module contains no rasterizer.
use crate::{
    fes_object_bank::Bank, fes_panel_parts::{Kind,Part,TextStyle},fes_save_panel::Panels,
    native_class_state::State,native_image_resource::{Geometry,Outcome,Pending as FramePending,Prepared},
    native_values::StringVariables,task_policy::NativeTaskPolicy,
};
use anyhow::{ensure,Context,Result};
use crysirror_core::{
    image_processing::{GlyphImageBatchSpec,GlyphImageBatchQuery,GlyphImageBatchState,GlyphImageCell,GlyphRasterOptions,GlyphImageOrigin,ImageCompositeLayer,ImageCompositeMath,ImageInput,ImageJob,ImageProcessState,ImageResource},
    service::HostServices,state::ClientId,
};
use crysirror_tasks::TaskPool;
use std::{collections::BTreeMap,sync::Arc};
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
enum Asset {Named(String,u32),Saved(u32)}
#[derive(Clone)]
struct Request {asset:Asset,bytes:Option<Arc<[u8]>>}
#[derive(Clone)]
struct Row {id:u32,part:Part,source:Option<Asset>,text:Vec<u8>}
#[derive(Clone,Debug,PartialEq,Eq)]
struct Identity {revision:u64,filename:Vec<u8>,references:Vec<(u32,Vec<u8>,u32)>}
struct Plan {id:u32,identity:Identity,name:String,rows:Vec<Row>,requests:Vec<Request>,size:[i32;2]}
struct TextJob {id:u32,positions:Vec<[i32;2]>,job:Box<dyn GlyphImageBatchQuery>}
struct PanelJob {
    plan:Plan,next:usize,loading:Option<(Asset,FramePending)>,
    loaded:BTreeMap<Asset,Option<Prepared>>,texts:BTreeMap<u32,Vec<(Arc<dyn ImageResource>,[i32;2])>>,
    text_next:usize,text_job:Option<TextJob>,composition:Option<ImageJob>,geometry:Option<Geometry>,
}
struct Pending {client:ClientId,pc:u32,plans:Vec<Plan>,current:Option<PanelJob>,next:usize,complete:Vec<(u32,Identity,Prepared)>}
#[derive(Default)]
pub(crate) struct Renders {
    pending:BTreeMap<u32,Pending>,
    published:BTreeMap<(u32,u32),Identity>,
}
fn stem(name:&[u8],strings:&StringVariables)->Result<String>{
    let name=strings.expand(name,255)?;
    Ok(crysirror_resource::text_stream::decode_auto(crate::native_scene::stem(&name),1024).context("CatSystem2 panel resource name encoding")?)
}
fn add_request(requests:&mut Vec<Request>,asset:Asset,bytes:Option<Arc<[u8]>>){
    if !requests.iter().any(|r|r.asset==asset){requests.push(Request{asset,bytes});}
}
fn plan(bank:&Bank,id:u32,panels:&Panels,strings:&StringVariables)->Result<Plan>{
    let object=bank.objects.get(&id).context("CatSystem2 panel object missing")?;
    let State::Image(image)=&object.state else{anyhow::bail!("CatSystem2 panel IMAGE base missing")};
    let panel=panels.panels.get(&id).context("CatSystem2 panel metadata missing")?;
    let name=stem(&image.filename,strings)?;
    let mut requests=Vec::new();
    add_request(&mut requests,Asset::Named(name.clone(),0),None);
    let mut rows=Vec::new();let mut references=Vec::new();
    for (part_id,part) in &panel.live_parts {
        if !panel.shown.contains(part_id)||panel.hidden.contains(part_id){continue;}
        let anchor=Asset::Named(name.clone(),*part_id);
        add_request(&mut requests,anchor,None);
        let source=match &part.kind {
            Kind::ObjectImage{target}|Kind::NumberObject{target}=>{
                if let Some(target)=bank.lookup(target) {
                    if let State::Image(source)=&bank.objects[&target].state {
                        let requested=part.frame(&panel.flags,panel.date)?;
                        let frame=if part.raw_frame||!source.mapped_ids{requested}else{source.ids.get(requested as usize).copied().unwrap_or(0)};
                        references.push((target,source.filename.clone(),frame));
                        if source.filename.is_empty(){None}else{Some(Asset::Named(stem(&source.filename,strings)?,frame))}
                    }else{None}
                }else{None}
            }
            Kind::SavedImage{key,..}=>{
                if let Some(bytes)=panel.saved.as_ref().and_then(|saved|saved.first(*key)) {
                    let asset=Asset::Saved(*key);add_request(&mut requests,asset.clone(),Some(Arc::from(bytes)));Some(asset)
                }else{None}
            }
            _=>None,
        };
        if let Some(asset)=&source{if !matches!(asset,Asset::Saved(_)){add_request(&mut requests,asset.clone(),None);}}
        let text=if panel.rebuilt&&matches!(part.kind,Kind::Text{..}){part.text(&panel.flags,panel.date,panel.saved.as_ref())}else{Vec::new()};
        rows.push(Row{id:*part_id,part:part.clone(),source,text});
    }
    Ok(Plan{id,identity:Identity{revision:panel.revision,filename:name.as_bytes().to_vec(),references},name,rows,requests,size:image.size})
}
impl Renders {
    pub fn reap(&mut self,tasks:&NativeTaskPolicy){self.pending.retain(|owner,_|tasks.fes(*owner).is_ok());self.published.retain(|(owner,_),_|tasks.fes(*owner).is_ok());}
    pub fn in_flight(&self,owner:u32)->bool{self.pending.contains_key(&owner)}
    pub fn pending_owners(&self)->Vec<u32>{self.pending.keys().copied().collect()}
    #[allow(clippy::too_many_arguments)]
    pub fn advance(&mut self,host:&mut dyn HostServices,client:ClientId,pool:&TaskPool,tasks:&mut NativeTaskPolicy,owner:u32,pc:u32,panels:&mut Panels,strings:&StringVariables)->Result<bool>{
        self.published.retain(|(owner,_),_|tasks.fes(*owner).is_ok());
        if !self.pending.contains_key(&owner){
            let Some(bank)=tasks.fes_object_bank_mut(owner)? else{return Ok(true)};
            let mut plans=Vec::new();
            for id in bank.ordered_ids(){
                let object=&bank.objects[&id];
                if !matches!(object.declared_type,11|12){continue;}
                if !matches!(&object.state,State::Image(image) if !image.filename.is_empty()){continue;}
                panels.panels.entry(id).or_default();
                let candidate=plan(bank,id,panels,strings)?;
                if self.published.get(&(owner,id))!=Some(&candidate.identity){plans.push(candidate);}
            }
            if plans.is_empty(){return Ok(true);}
            self.pending.insert(owner,Pending{client,pc,plans,current:None,next:0,complete:Vec::new()});
        }
        let pending=self.pending.get_mut(&owner).unwrap();
        ensure!(pending.client==client&&pending.pc==pc,"CatSystem2 panel pending owner/PC changed");
        loop {
            if pending.current.is_none(){
                if pending.next==pending.plans.len(){break;}
                let p=std::mem::replace(&mut pending.plans[pending.next],Plan{id:0,identity:Identity{revision:0,filename:Vec::new(),references:Vec::new()},name:String::new(),rows:Vec::new(),requests:Vec::new(),size:[0;2]});
                pending.current=Some(PanelJob{plan:p,next:0,loading:None,loaded:BTreeMap::new(),texts:BTreeMap::new(),text_next:0,text_job:None,composition:None,geometry:None});
            }
            let job=pending.current.as_mut().unwrap();
            let Some(prepared)=job.poll(host,client,pool,owner,pc)? else{return Ok(false)};
            pending.complete.push((job.plan.id,job.plan.identity.clone(),prepared));pending.next+=1;pending.current=None;
        }
        let pending=self.pending.remove(&owner).unwrap();
        let bank=tasks.fes_object_bank_mut(owner)?.context("CatSystem2 panel bank retired during dependency")?;
        for (id,identity,frame) in pending.complete {
            ensure!(panels.panels.get(&id).is_some_and(|p|p.revision==identity.revision),"CatSystem2 panel changed during composition");
            let object=bank.objects.get_mut(&id).context("CatSystem2 panel object retired during dependency")?;
            let State::Image(image)=&mut object.state else{anyhow::bail!("CatSystem2 panel IMAGE base changed")};
            image.size=frame.geometry.size.map(|v|v as i32);
            image.copy_rect=Some([0,0,image.size[0],image.size[1]]);
            image.reference=Some(frame);image.object_reference=None;image.image_changed=true;
            self.published.insert((owner,id),identity);
        }
        Ok(true)
    }
}
impl PanelJob {
    fn anchor(&self,id:u32)->Option<&Prepared>{self.loaded.get(&Asset::Named(self.plan.name.clone(),id))?.as_ref()}
    fn poll(&mut self,host:&mut dyn HostServices,client:ClientId,pool:&TaskPool,owner:u32,pc:u32)->Result<Option<Prepared>>{
        if let Some((asset,load))=&mut self.loading {
            let Some(outcome)=load.poll()? else{return Ok(None)};
            let frame=match outcome{Outcome::Ready(frame)=>Some(frame),Outcome::MissingFile|Outcome::MissingFrame=>None};
            self.loaded.insert(asset.clone(),frame);self.next+=1;self.loading=None;
        }
        while let Some(request)=self.plan.requests.get(self.next){
            if self.loaded.contains_key(&request.asset){self.next+=1;continue;}
            let load=match &request.asset {
                Asset::Named(name,frame)=>FramePending::begin_named(pool,host,client,owner,pc,name,*frame)?,
                Asset::Saved(key)=>FramePending::begin_bytes(pool,host,client,owner,pc,&format!("panel saved image {key}"),request.bytes.clone().context("CatSystem2 panel saved image bytes")?,0)?,
            };
            let Some(load)=load else{return Ok(None)};
            self.loading=Some((request.asset.clone(),load));return Ok(None);
        }
        if self.geometry.is_none(){
            let geometry=self.loaded.get(&Asset::Named(self.plan.name.clone(),0)).and_then(Option::as_ref).or_else(||self.loaded.iter().find_map(|(asset,value)|matches!(asset,Asset::Named(name,_) if name==&self.plan.name).then_some(value.as_ref()).flatten())).map(|f|f.geometry.clone());
            let size=geometry.as_ref().map(|g|g.canvas).or_else(||self.plan.size.iter().all(|v|*v>0).then(||self.plan.size.map(|v|v as u32))).context("CatSystem2 panel has no declared canvas geometry")?;
            ensure!(size.iter().all(|v|(1..=8192).contains(v)),"CatSystem2 panel canvas extent");
            self.geometry=Some(Geometry{frame_id:0,size,canvas:size,origin:[0;2],base:Some([0;2]),color_bits:32,flags:Some(1),image_mode:Some(0),compression_type:Some(0),attributes:Vec::new()});
        }
        if let Some(text)=&mut self.text_job {
            match text.job.poll()? {
                GlyphImageBatchState::Cancelled=>anyhow::bail!("CatSystem2 panel glyph preparation cancelled"),
                GlyphImageBatchState::Pending=>return Ok(None),GlyphImageBatchState::Failed(error)=>anyhow::bail!("CatSystem2 panel glyph job: {error}"),
                GlyphImageBatchState::Ready(images)=>{
                    ensure!(images.client()==client&&images.images().len()==text.positions.len(),"CatSystem2 panel glyph batch ownership/extent");
                    self.texts.insert(text.id,images.images().iter().cloned().zip(text.positions.iter().copied()).collect());self.text_next+=1;self.text_job=None;
                }
            }
        }
        while let Some(row)=self.plan.rows.get(self.text_next){
            if let Kind::Text{style,..}=&row.part.kind {
                if !row.text.is_empty(){
                    if let Some(anchor)=self.anchor(row.id){
                        if let Some((spec,mut positions))=text_spec(style,&row.text,host,client)? {
                            for position in &mut positions{position[0]=position[0].wrapping_add(anchor.geometry.origin[0]);position[1]=position[1].wrapping_add(anchor.geometry.origin[1]);}
                            let image=host.image_metadata(client)?.begin_glyph_image_batch(spec)?;
                            self.text_job=Some(TextJob{id:row.id,positions,job:image});return Ok(None);
                        }
                    }
                }
            }
            self.text_next+=1;
        }
        if self.composition.is_none(){
            let images=host.image_metadata(client)?;let mut layers=Vec::new();
            for row in &self.plan.rows {
                let Some(anchor)=self.anchor(row.id)else{continue};
                let origin=anchor.geometry.origin;
                match &row.part.kind {
                    Kind::Norm=>layers.push(layer(input(anchor)?,anchor.image.size(),origin)),
                    Kind::ObjectImage{..}|Kind::NumberObject{..}=>{
                        if let Some(frame)=row.source.as_ref().and_then(|asset|self.loaded.get(asset)).and_then(Option::as_ref){
                            layers.push(layer(input(frame)?,frame.image.size(),[origin[0].wrapping_add(frame.geometry.origin[0]),origin[1].wrapping_add(frame.geometry.origin[1])]));
                        }
                    }
                    Kind::SavedImage{size,clip,..}=>{
                        if let Some(frame)=row.source.as_ref().and_then(|asset|self.loaded.get(asset)).and_then(Option::as_ref){
                            let mut source=input(frame)?;let mut extent=frame.image.size();
                            if let Some(size)=size {ensure!(size.iter().all(|v|*v>0),"CatSystem2 panel saved image target size");extent=*size;source=ImageInput::Process{input:Box::new(source),effects:serde_json::to_vec(&[crysirror_image::effects::ImageEffect::Resize{width:size[0],height:size[1]}])?.into()};}
                            let rect=clip.unwrap_or([0,0,extent[0] as i32,extent[1] as i32]);
                            ensure!(rect[2]>0&&rect[3]>0,"CatSystem2 panel saved image crop extent");
                            let mut entry=layer(source,extent,origin);entry.source=rect;layers.push(entry);
                        }
                    }
                    Kind::Text{style,..}=>{
                        if let Some(cells)=self.texts.get(&row.id){
                            for (image,position) in cells {
                                if let Some(spec)=crate::native_text_coverage::kernel(style.flags,style.font_size,rgb(style.edge_color)){
                                    let effects=serde_json::to_vec(&[crysirror_image::effects::ImageEffect::CoverageSum{spec}])?;
                                    layers.push(layer(ImageInput::Process{input:Box::new(ImageInput::Resource(image.clone())),effects:effects.into()},image.size(),*position));
                                }
                                let color=rgb(style.fill_color);
                                let effects=serde_json::to_vec(&[crysirror_image::effects::ImageEffect::ChannelScale{factors:[color[0],color[1],color[2],255]}])?;
                                layers.push(layer(ImageInput::Process{input:Box::new(ImageInput::Resource(image.clone())),effects:effects.into()},image.size(),*position));
                            }
                        }
                    }
                    // 660110 receives a null native bitmap pointer for these ptype
                    // records until an actual bitmap setter supplies one.
                    Kind::Image|Kind::NumberImage=>{},
                }
            }
            self.composition=Some(images.begin_process(ImageInput::Composite{base:Box::new(ImageInput::Solid{size:self.geometry.as_ref().unwrap().size,color:[0;4]}),layers},Arc::from(b"[]".as_slice()))?);
            return Ok(None);
        }
        match self.composition.as_mut().unwrap().poll()? {
            ImageProcessState::Pending=>Ok(None),ImageProcessState::Failed(error)=>anyhow::bail!("CatSystem2 panel composition: {error}"),
            ImageProcessState::Ready(image)=>Ok(Some(Prepared{source:format!("FES PANEL {owner}/{} revision {}",self.plan.id,self.plan.identity.revision),geometry:self.geometry.clone().unwrap(),image})),
        }
    }
}
fn input(frame:&Prepared)->Result<ImageInput>{
    let input=ImageInput::Resource(frame.image.clone());
    Ok(if frame.geometry.flags==Some(0){ImageInput::Process{input:Box::new(input),effects:Arc::from(b"[{\"kind\":\"override_alpha\",\"alpha\":255}]".as_slice())}}else{input})
}
fn layer(input:ImageInput,size:[u32;2],destination:[i32;2])->ImageCompositeLayer{
    ImageCompositeLayer{input:Box::new(input),source:[0,0,size[0] as i32,size[1] as i32],destination,clip:None,opacity:255,math:ImageCompositeMath::AlphaOnAlphaScalar256}
}
fn rgb(color:u32)->[u8;3]{[(color>>16)as u8,(color>>8)as u8,color as u8]}
fn text_spec(style:&TextStyle,bytes:&[u8],host:&mut dyn HostServices,client:ClientId)->Result<Option<(GlyphImageBatchSpec,Vec<[i32;2]>)>>{
    let size=style.font_size as i32;
    ensure!((1..=512).contains(&size),"CatSystem2 PANEL text requires a nonzero native font-size profile");
    let source=crysirror_resource::text_stream::decode_auto(bytes,65534)?;
    let mut font=if style.font.is_empty(){"default".to_owned()}else{crysirror_resource::text_stream::decode_auto(&style.font,256)?};
    let fonts=host.font_library(client)?;
    if let Some(alias)=fonts.resolve_alias(&font)?{font=alias;}else if font!="default"{host.append_log(client,&format!("CatSystem2 panel font {font} uses engine default fallback"))?;font="default".to_owned();}
    let mut cells=Vec::new();let mut positions=Vec::new();let mut pen=[0i32;2];let mut measure=0i32;let mut line=0u32;
    for character in source.chars(){
        if measure>size.wrapping_mul(style.columns as i32).wrapping_sub(size/2) {
            line=line.wrapping_add(1);if line>=style.max_lines{break;}
            pen=[0,pen[1].wrapping_add(size).wrapping_add(style.line_pitch as i32)];measure=0;
        }
        ensure!(pen.iter().all(|v|v.unsigned_abs()<=1_000_000),"CatSystem2 panel text position budget");
        cells.push(GlyphImageCell{canvas:[size as u32+8;2],glyphs:vec![crysirror_core::service::Glyph{glyph_id:fonts.glyph_id(&font,character)?,x_26_6:64,y_26_6:64,advance_26_6:0}]});
        positions.push(pen);
        let encoded=character.to_string();let (encoded,_,errors)=encoding_rs::SHIFT_JIS.encode(&encoded);
        let advance=if !errors&&encoded.len()==1{size/2}else{size};pen[0]=pen[0].wrapping_add(advance);measure=measure.wrapping_add(size);
        if style.flags&3!=0{pen[0]=pen[0].wrapping_sub(2);measure=measure.wrapping_sub(2);}
    }
    if cells.is_empty(){return Ok(None);}
    let mut raster=GlyphRasterOptions::default();raster.origin=GlyphImageOrigin::Top;
    Ok(Some((GlyphImageBatchSpec{font,size:size as u16,cells,raster},positions)))
}
