// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original FES class resource admission over shared worker Image/Canvas/Layer
//! capabilities. Preparing a bank never acknowledges an original VM operation.
use crate::{
    fes_object_bank::Bank,
    native_class_state::State,
    native_image_publication::{Draw, ImageList, Plane as View},
    native_image_resource::{Outcome, Pending as FramePending},
    native_interpreter::Interpreter,
    native_values::StringVariables,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    service::{EngineQuery, HostServices, ImageMetadata, UploadedImage},
    state::ClientId,
};
use crysirror_scene::layer::{LayerCommand as L, LayerSpec};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::BTreeMap, sync::Arc};
fn rgba(color: u32) -> [u8; 4] {
    [
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
        (color >> 24) as u8,
    ]
}
fn layer_id(id: u32) -> Result<u32> {
    0x4d000000u32
        .checked_add(id)
        .context("CatSystem2 object Layer ID range")
}
fn canvas_id(id: u32) -> Result<u32> {
    0x5c000000u32
        .checked_add(id)
        .context("CatSystem2 object Canvas ID range")
}
pub(crate) fn preflight(bank: &Bank) -> Result<()> {
    ensure!(
        bank.owner <= u16::MAX.into(),
        "CatSystem2 typed draw sender range"
    );
    for object in bank.objects.values() {
        match &object.state{
   State::Base=>{},
   State::CImage(image)|State::Image(image)=>ensure!(matches!(image.mask,0|4),
    "CatSystem2 unsupported image MASK {} (channel masks 1..3 require explicit channel selection)",image.mask),
   // Affine poses and plane transforms still publish their axis-aligned base.
   State::CPlane(_)|State::Plane(_)=>{},
   State::Sound(sound)=>ensure!(sound.filename.len()<260&&!sound.filename.contains(&0)&&sound.group.is_ascii()&&sound.group.len()<32,
    "CatSystem2 SOUND filename/group field extent or encoding"),
  }
    }
    Ok(())
}
struct Metadata {
    bank: Bank,
    vm: Interpreter,
}
enum Current {
    BgTable(u32, Task<Option<crate::fes_bg_table::Table>>),
    Solid(u32, ImageJob),
    Effects(u32, ImageJob),
    Frame(u32, FramePending),
    /// SIZE2/POS2 name an HG3 frame whose geometry supplies the object's size/position.
    Reference(u32, FramePending),
}
pub(crate) struct Pending {
    pub owner: u32,
    pub native_pc: u32,
    client: ClientId,
    metadata: Option<Task<Metadata>>,
    bank: Option<Bank>,
    vm: Option<Interpreter>,
    strings: StringVariables,
    images: Arc<dyn ImageMetadata>,
    pool: TaskPool,
    ids: Vec<u32>,
    next: usize,
    current: Option<Current>,
    ready: BTreeMap<u32, Arc<dyn ImageResource>>,
    effects_done: std::collections::BTreeSet<u32>,
    frames: BTreeMap<u32, crate::native_image_resource::Prepared>,
    failure: Option<String>,
    published: Option<(u32, Vec<u8>, Receipt)>,
    existing: Option<Receipt>,
    retiring:Vec<String>,
    /// 743E10: the list's page in shared VRAM. FES VRAM words are already list-relative
    /// (meswnd's `pl` declares 0,0 under a list based at 0,4096).
    pub upload: [i32; 2],
}
impl Pending {
    pub fn begin(
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        pc: u32,
        declarations: Arc<crate::fes_declarations::Declarations>,
        mut vm: Interpreter,
        strings: StringVariables,
    ) -> Result<Option<Self>> {
        let permit = match pool.reserve() {
            Ok(p) => p,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
            Err(e) => return Err(e),
        };
        let images = host.image_metadata(client)?;
        let captured = strings.clone();
        let metadata = permit.spawn(move |cancel| {
            cancel.check()?;
            crate::fes_program::initialize_player_bank(owner, &mut vm)?;
            let bank = Bank::prepare(owner, &declarations, &mut vm, &captured)?;
            preflight(&bank)?;
            cancel.check()?;
            Ok(Metadata { bank, vm })
        })?;
        Ok(Some(Self {
            owner,
            native_pc: pc,
            client,
            metadata: Some(metadata),
            bank: None,
            vm: None,
            strings,
            images,
            pool: pool.clone(),
            ids: Vec::new(),
            next: 0,
            current: None,
            ready: BTreeMap::new(),
            effects_done: Default::default(),
            frames: BTreeMap::new(),
            failure: None,
            published: None,
            existing: None,
            retiring:Vec::new(),upload:[0,0],
        }))
    }
    ///After the original script phase, prepare only changed class resources.
    ///The already evaluated constructor/OBJECT bank is never executed again.
    pub fn begin_bank(pool:&TaskPool,host:&mut dyn HostServices,client:ClientId,
        pc:u32,bank:Bank,vm:Interpreter,strings:StringVariables,previous:Option<Receipt>)->Result<Self>{
        preflight(&bank)?;
        let mut ready=BTreeMap::new();let mut frames=BTreeMap::new();
        if let Some(old)=&previous {
            ensure!(old.owner==bank.owner,"CatSystem2 changed frame resource owner");
            for (id,image) in &old.resident {
                if let (Some(a),Some(b))=(old.bank.objects.get(id),bank.objects.get(id)) {
                    let dependency=match &b.state {State::Image(image)|State::CImage(image)=>image.object_reference,_=>None};
                    let dependency_unchanged=dependency.is_none_or(|source|match(old.bank.objects.get(&source),bank.objects.get(&source)){(Some(a),Some(b))=>resource_identity(&a.state)==resource_identity(&b.state),_=>false});
                    if dependency_unchanged&&resource_identity(&a.state)==resource_identity(&b.state) {
                        ready.insert(*id,image.clone());
                        if let Some(frame)=old.frames.get(id){frames.insert(*id,frame.clone());}
                    }
                }
            }
        }
        let mut value=Self{owner:bank.owner,native_pc:pc,client,metadata:None,
            ids:reference_order(&bank)?,bank:Some(bank),vm:Some(vm),strings,
            images:host.image_metadata(client)?,pool:pool.clone(),next:0,current:None,
            effects_done:ready.keys().copied().collect(),ready,frames,failure:None,published:None,existing:previous,retiring:Vec::new(),upload:[0,0]};
        let inherited=value.frames.clone();
        for (id,frame) in inherited{value.apply_geometry(id,&frame)?;}
        Ok(value)
    }
    fn apply_geometry(&mut self,id:u32,frame:&crate::native_image_resource::Prepared)->Result<()> {
        let State::Image(image)=&mut self.bank.as_mut().unwrap().objects.get_mut(&id).context("CatSystem2 frame geometry target missing")?.state else{anyhow::bail!("CatSystem2 frame geometry target class changed")};
        if image.reference.is_some()||image.object_reference.is_some(){return Ok(());}
        let base=frame.geometry.base.context("CatSystem2 native HG3 missing base metadata profile")?;
        if image.path_mode==0 {
            image.pose.offset=[base[0] as f32,base[1] as f32];
            image.pose.pivot=[base[0].wrapping_sub(frame.geometry.origin[0]) as f32,base[1].wrapping_sub(frame.geometry.origin[1]) as f32];
        }else{image.pose.offset=[0.;2];image.pose.pivot=base.map(|v|v as f32);}
        image.size=frame.geometry.size.map(|v|v as i32);
        Ok(())
    }
    fn retire_previous(&mut self,host:&mut dyn HostServices)->Result<()> {
        while let Some(path)=self.retiring.last(){
            //The shared image service retains any older Canvas/snapshot readers
            //until they stop referencing this generation; the VM never guesses.
            host.release_image_resource(self.client,path)?;
            self.retiring.pop();
        }Ok(())
    }
    pub fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(error) = &self.failure {
            anyhow::bail!("{error}")
        }
        match self.poll_inner(host) {
            Ok(value) => Ok(value),
            Err(error) => {
                let error = format!("{error:#}");
                self.failure = Some(error.clone());
                anyhow::bail!("{error}")
            }
        }
    }
    fn poll_inner(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(task) = &mut self.metadata {
            let Some(metadata) = task.try_take()? else {
                return Ok(false);
            };
            self.metadata = None;
            self.ids = reference_order(&metadata.bank)?;
            self.vm = Some(metadata.vm);
            self.bank = Some(metadata.bank);
        }
        if let Some(mut current) = self.current.take() {
            match &mut current {
                Current::BgTable(id, task) => {
                    let Some(table) = task.try_take()? else { self.current = Some(current); return Ok(false) };
                    let State::Image(image) = &mut self.bank.as_mut().unwrap().objects.get_mut(id).context("CatSystem2 BGTABLE object missing")?.state else { anyhow::bail!("CatSystem2 BGTABLE class changed") };
                    image.bg_table_source = None;
                    if let Some(table) = table { image.bg_table = Some(table); }
                    return self.poll_inner(host);
                }
                Current::Effects(id, job) => {
                    match job.poll()? {
                        ImageProcessState::Pending => { self.current=Some(current);return Ok(false); }
                        ImageProcessState::Failed(error) => anyhow::bail!("CatSystem2 object effects: {error}"),
                        ImageProcessState::Ready(image) => { self.ready.insert(*id,image);self.effects_done.insert(*id); }
                    }
                    return self.poll_inner(host);
                }

                Current::Solid(id, job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.current = Some(current);
                        return Ok(false);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("CatSystem2 class image worker: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client,
                            "CatSystem2 foreign class image"
                        );
                        self.ready.insert(*id, image);
                    }
                },
                Current::Reference(id, job) => {
                    let Some(outcome) = job.poll()? else {
                        self.current = Some(current);
                        return Ok(false);
                    };
                    let geometry = match outcome { Outcome::Ready(frame) => Some(frame.geometry), _ => None };
                    let object = self.bank.as_mut().unwrap().objects.get_mut(id).unwrap();
                    let (size_reference, position_reference, size, position) = match &mut object.state {
                        State::Plane(p) | State::CPlane(p) => (&mut p.size_reference, &mut p.position_reference, &mut p.size, &mut p.pose.position[..]),
                        State::Image(p) | State::CImage(p) => (&mut p.size_reference, &mut p.position_reference, &mut p.size, &mut p.pose.position[..]),
                        _ => unreachable!(),
                    };
                    let rail=geometry.as_ref().map(|g|([g.origin[0] as f32,g.origin[1] as f32],[g.size[0] as f32,g.size[1] as f32]));
                    if let Some(geometry) = geometry {
                        if size_reference.is_some() { *size = [geometry.size[0] as i32, geometry.size[1] as i32]; }
                        if position_reference.is_some() { position[0] = geometry.origin[0] as f32; position[1] = geometry.origin[1] as f32; }
                    }
                    *size_reference = None;
                    let placed=position_reference.is_some();
                    *position_reference = None;
                    if let (true,Some((origin,extent)),State::Image(image))=(placed,rail,&mut object.state){
                        if let Some(slider)=&mut image.slider{slider.place(origin,extent);}
                    }
                    self.current = None;
                    return Ok(false);
                }
                Current::Frame(id, job) => {
                    let Some(outcome) = job.poll()? else {
                        self.current = Some(current);
                        return Ok(false);
                    };
                    match outcome {
                        Outcome::MissingFile|Outcome::MissingFrame=>{
                            // 748D20 keeps the object with no surface: it draws nothing.
                            let State::Image(image)=&mut self.bank.as_mut().unwrap().objects.get_mut(id).unwrap().state else{unreachable!()};
                            let notice=format!("CatSystem2 IMAGE {:?} frame {} is unavailable",String::from_utf8_lossy(&image.filename),image.native_frame());
                            image.missing=Some((image.filename.clone(),image.native_frame()));
                            host.append_log(self.client,&notice)?;
                        }
                        Outcome::Ready(frame)=>{
                            let State::Image(image)=&mut self.bank.as_mut().unwrap().objects.get_mut(id).unwrap().state else{unreachable!()};
                            let base=frame.geometry.base.context("CatSystem2 native HG3 missing base metadata profile")?;
                            if image.path_mode==0 {
                                image.pose.offset=[base[0] as f32,base[1] as f32];
                                image.pose.pivot=[base[0].wrapping_sub(frame.geometry.origin[0]) as f32,base[1].wrapping_sub(frame.geometry.origin[1]) as f32];
                            }else{
                                image.pose.offset=[0.;2];
                                image.pose.pivot=[base[0] as f32,base[1] as f32];
                            }
                            image.size=[frame.geometry.size[0] as i32,frame.geometry.size[1] as i32];
                            let opaque=frame.geometry.flags.context("CatSystem2 native HG3 missing alpha-cache metadata")?==0;
                            let original=frame.image.clone();
                            let size=frame.geometry.size;
                            self.frames.insert(*id,frame);
                            if opaque {
                                //Original native Surface field11 is flag-derived, not
                                //an inferred scan of texels. Keep the raw immutable
                                //frame for native queries and prepare the opaque draw
                                //resource through engine-owned CopyRgb composition.
                                let job=self.images.begin_process(ImageInput::Composite{
                                    base:Box::new(ImageInput::Solid{size,color:[0,0,0,255]}),
                                    layers:vec![ImageCompositeLayer{
                                        input:Box::new(ImageInput::Resource(original)),
                                        source:[0,0,size[0] as i32,size[1] as i32],
                                        destination:[0,0],clip:None,opacity:255,math:ImageCompositeMath::CopyRgb,
                                    }],
                                },Arc::from(b"[]".as_slice()))?;
                                self.current=Some(Current::Solid(*id,job));
                                return Ok(false);
                            }
                            self.ready.insert(*id,original);
                        }
                    }
                }
            }
            self.next += 1;
        }
        while let Some(id) = self.ids.get(self.next).copied() {
            let object = &self.bank.as_ref().unwrap().objects[&id];
            if let State::Image(image) = &object.state {
                if let (Some(kind), Some(source)) = (image.bg_table_kind, image.bg_table_source.as_ref()) {
                    let source = self.strings.expand(source, 255)?;
                    let storage = host.storage(self.client)?;
                    let permit = match self.pool.reserve() {
                        Ok(permit) => permit,
                        Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                        Err(error) => return Err(error),
                    };
                    self.current = Some(Current::BgTable(id, permit.spawn(move |cancel| crate::fes_bg_table::load(storage.as_ref(), &source, kind, &cancel))?));
                    return Ok(false);
                }
            }
            if self.ready.contains_key(&id){self.next+=1;continue;}
            let reference = match &object.state {
                State::Plane(p) | State::CPlane(p) => p.size_reference.as_ref().or(p.position_reference.as_ref()),
                State::Image(p) | State::CImage(p) => p.size_reference.as_ref().or(p.position_reference.as_ref()),
                _ => None,
            };
            if let Some(reference) = reference {
                let frame = reference.frame;
                let name = self.strings.expand(&reference.name, 255)?;
                let name = String::from_utf8(name).context("CatSystem2 metadata image name encoding")?;
                let name = name.strip_suffix(".hg3").unwrap_or(&name).to_owned();
                let Some(pending) = FramePending::begin_named(&self.pool, host, self.client, self.owner, self.native_pc, &name, frame)? else {
                    return Ok(false);
                };
                self.current = Some(Current::Reference(id, pending));
                return Ok(false);
            }
            let dependency=match &object.state {State::Image(image)|State::CImage(image)=>image.object_reference,_=>None};
            if let Some(source)=dependency {
                if !self.effects_done.contains(&source) {
                    if let Some(resource)=self.ready.get(&source) {
                        let effects=object_effects(&self.bank.as_ref().unwrap().objects[&source].state,self.frames.get(&source).map(|f|&f.geometry))?;
                        if effects.is_empty(){self.effects_done.insert(source);}else{
                            self.current=Some(Current::Effects(source,self.images.begin_process(ImageInput::Resource(resource.clone()),serde_json::to_vec(&effects)?.into())?));return Ok(false);
                        }
                    }
                }
                if let (Some(frame),Some(resource))=(self.frames.get(&source).cloned(),self.ready.get(&source).cloned()) {
                    let mut frame=frame;frame.image=resource;
                    let (State::Image(image)|State::CImage(image))=&mut self.bank.as_mut().unwrap().objects.get_mut(&id).unwrap().state else{unreachable!()};
                    image.reference=Some(frame);
                }
            }
            let object=&self.bank.as_ref().unwrap().objects[&id];
            match &object.state {
                State::Image(image)|State::CImage(image) if image.reference.is_some() => {
                    let frame=image.reference.clone().unwrap();
                    let resource=frame.image.clone();
                    let opaque=frame.geometry.flags==Some(0);
                    let size=resource.size();
                    let rect=image.copy_rect.filter(|r|r[2]!=0&&r[3]!=0).unwrap_or([0,0,size[0] as i32,size[1] as i32]);
                    ensure!(rect[0]>=0&&rect[1]>=0&&rect[2]>0&&rect[3]>0,"CatSystem2 copy source rectangle");
                    let width=(rect[2] as u32).min(size[0].saturating_sub(rect[0] as u32));
                    let height=(rect[3] as u32).min(size[1].saturating_sub(rect[1] as u32));
                    ensure!(width>0&&height>0,"CatSystem2 copy outside source surface");
                    self.frames.insert(id,frame);
                    let (State::Image(image)|State::CImage(image))=&mut self.bank.as_mut().unwrap().objects.get_mut(&id).unwrap().state else{unreachable!()};
                    image.size=[width as i32,height as i32];
                    self.current=Some(Current::Solid(id,self.images.begin_process(ImageInput::Composite{
                        base:Box::new(ImageInput::Solid{size:[width,height],color:[0;4]}),
                        layers:vec![ImageCompositeLayer{input:Box::new(ImageInput::Resource(resource)),source:[rect[0],rect[1],width as i32,height as i32],destination:[0;2],clip:None,opacity:255,math:ImageCompositeMath::Copy}],
                    },if opaque{Arc::from(b"[{\"kind\":\"override_alpha\",\"alpha\":255}]".as_slice())}else{Arc::from(b"[]".as_slice())})?));
                    return Ok(false);
                }
                State::CImage(image) if image.size.iter().all(|v| *v > 0) => {
                    self.current = Some(Current::Solid(
                        id,
                        self.images.begin_process(
                            ImageInput::Solid {
                                size: [image.size[0] as u32, image.size[1] as u32],
                                color: rgba(image.color),
                            },
                            Arc::from(b"[]".as_slice()),
                        )?,
                    ));
                    return Ok(false);
                }
                // Frame -1 only caches the file in the original; nothing is drawn until setid.
                State::Image(image) if image.frame==u32::MAX => self.next+=1,
                State::Image(image) if image.missing.as_ref().is_some_and(|(file,frame)|*file==image.filename&&*frame==image.native_frame()) => self.next+=1,
                State::Image(image) if !image.filename.is_empty() => {
                    let name = self.strings.expand(&image.filename, 255)?;
                    let name = if name.len()>4&&name[name.len()-4..].eq_ignore_ascii_case(b".hg3") {name[..name.len()-4].to_vec()} else {name};
                    ensure!(
                        name.is_ascii() && !name.contains(&b' '),
                        "CatSystem2 multi-image or non-ASCII filename class policy not yet bound"
                    );
                    let Some(pending) = FramePending::begin_named(
                        &self.pool,
                        host,
                        self.client,
                        self.owner,
                        self.native_pc,
                        std::str::from_utf8(&name)?,
                        image.native_frame(),
                    )?
                    else {
                        return Ok(false);
                    };
                    self.current = Some(Current::Frame(id, pending));
                    return Ok(false);
                }
                _ => self.next += 1,
            }
        }
        // Immutable effect resources are prepared by the engine worker, after source
        // admission and before any Canvas transaction. Native queries keep raw HG3.
        for id in &self.ids {
            if self.effects_done.contains(id) {continue;}
            let Some(resource)=self.ready.get(id) else {continue};
            let state=&self.bank.as_ref().unwrap().objects[id].state;
            let effects=object_effects(state,self.frames.get(id).map(|f|&f.geometry))?;
            if effects.is_empty() {self.effects_done.insert(*id);continue;}
            self.current=Some(Current::Effects(*id,self.images.begin_process(ImageInput::Resource(resource.clone()),serde_json::to_vec(&effects)?.into())?));
            return Ok(false);
        }
        Ok(true)
    }
    pub fn prepared(&self) -> Result<(&Bank, &Interpreter)> {
        ensure!(
            self.metadata.is_none()
                && self.current.is_none()
                && self.next == self.ids.len()
                && self.failure.is_none(),
            "CatSystem2 class resources not prepared"
        );
        Ok((
            self.bank
                .as_ref()
                .context("CatSystem2 class bank missing")?,
            self.vm.as_ref().unwrap(),
        ))
    }
    pub fn publish(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        root: u32,
        list: &ImageList,
    ) -> Result<Receipt> {
        self.publish_frame(host,client,root,list,Vec::new())
    }
    pub fn publish_frame(&mut self,host:&mut dyn HostServices,client:ClientId,root:u32,list:&ImageList,final_layers:Vec<L>)->Result<Receipt>{
        ensure!(
            client == self.client,
            "CatSystem2 foreign typed resource publication"
        );
        ensure!(
            list.scope().0 == self.client,
            "CatSystem2 foreign image-list scope"
        );
        let source = serde_json::to_vec(&(list.commands()?,&final_layers))?;
        if let Some((stored_root, stored_source, receipt)) = &self.published {
            ensure!(
                *stored_root == root && *stored_source == source,
                "CatSystem2 typed publication retry changed dependency"
            );
            let receipt=receipt.clone();self.retire_previous(host)?;
            return Ok(receipt);
        }
        let (bank, vm) = self.prepared()?;
        let mut proposed = list.clone();
        proposed.clear_sender(bank.owner)?;
        let mut layers = Vec::new();
        let mut depth = Vec::new();
        let mut creation_order = bank.ordered_ids();
        let depth_of = |mut id: u32| { let mut depth = 0u32; while let Some(parent) = bank.objects.get(&id).and_then(|o| o.parent) { depth += 1; if depth > 64 || parent == id { break; } id = parent; } depth };
        creation_order.sort_by_key(|id| depth_of(*id));
        let mut text_layers = Vec::new();
        for object_id in creation_order {
            let object = &bank.objects[&object_id];
            let id = if object.id == bank.root {
                root
            } else {
                layer_id(object.id)?
            };
            let existing: serde_json::Value =
                serde_json::from_slice(&host.query_engine(client, EngineQuery::Layer(id))?)?;
            let retained = !existing.is_null();
            ensure!(!retained || object.id==bank.root || self.existing.as_ref().is_some_and(|old|old.bank.objects.contains_key(&object.id)),
                "CatSystem2 class Layer collides with another owner resource");
            let mut layer: LayerSpec = serde_json::from_value(
                serde_json::json!({"id":id,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]}}),
            )?;
            if object.id != bank.root {
                layer.parent = Some(match object.parent {
                    Some(parent) if parent != bank.root => layer_id(parent)?,
                    _ => root,
                });
            }
            match &object.state {
                State::CPlane(plane) | State::Plane(plane) => {
                    layer.rect = [
                        plane.pose.position[0],
                        plane.pose.position[1],
                        plane.size[0] as f32,
                        plane.size[1] as f32,
                    ];
                    layer.visible = object.enabled && plane.pose.visible != 0;
                    if matches!(&object.state, State::CPlane(_)) {
                        layer.paint = crysirror_scene::layer::Paint::Solid {
                            color: rgba(plane.pose.color),
                        };
                    } else if let Some(text) = plane.text.as_ref() {
                        layer.opacity = (plane.pose.color >> 24) as u8;
                        let unchanged = self.existing.as_ref().and_then(|old| old.bank.objects.get(&object.id)).is_some_and(|old| match &old.state {
                            State::Plane(old) => old.text.as_ref().is_some_and(|old| old.generation == text.generation),
                            _ => false,
                        });
                        if !unchanged {
                            let child = 0x12000000u32.checked_add(object.id).filter(|id| *id < 0x20000000).context("CatSystem2 text layer ID range")?;
                            let mut glyphs: LayerSpec = serde_json::from_value(serde_json::json!({"id":child,"parent":id,"rect":[0,0,plane.size[0],plane.size[1]],"paint":{"kind":"solid","color":[0,0,0,0]}}))?;
                            glyphs.paint = crysirror_scene::layer::Paint::GlyphRun { outline:None,
                                font: "default".to_owned(),
                                size: text.sizes[2],
                                color: [text.color[0], text.color[1], text.color[2], 255],
                                bold: false,
                                glyphs: text.glyphs[..text.visible].to_vec(),
                                reveal: None,
                                blur: None,
                            };
                            text_layers.push(L::Put { layer: glyphs });
                            text_layers.push(L::SetDepth { id: child, depth: Some(plane.pose.priority as f32) });
                        }
                    } else {
                        layer.opacity = (plane.pose.color >> 24) as u8;
                        ensure!(
                            plane.vram[..2]
                                .iter()
                                .all(|v| v.is_finite() && v.fract() == 0. && v.abs() <= 1_000_000.),
                            "CatSystem2 nonintegral Plane VRAM view profile"
                        );
                        proposed.plane(View {
                            canvas: canvas_id(object.id)?,
                            //Canvas owns >=0x20000000 temporary layer IDs.
                            //Native bank IDs occupy at most the lower28 bits;
                            //use an independent low presentation namespace.
                            presentation: 0x10000000u32
                                .checked_add(object.id)
                                .filter(|id| *id < 0x20000000)
                                .context("CatSystem2 presentation ID range")?,
                            parent: id,
                            source: [
                                plane.vram[0] as i32,
                                plane.vram[1] as i32,
                                plane.size[0],
                                plane.size[1],
                            ],
                        })?;
                    }
                    depth.push(L::SetDepth {
                        id,
                        depth: Some(plane.pose.priority as f32),
                    });
                }
                _ => {}
            }
            if retained {
                //Updating a class never Put-replaces a live external producer.
                //The implicit root belongs to the task lifecycle and retains
                //developer/native motion, opacity and ancestry untouched.
                if object.id!=bank.root {
                    layers.push(L::SetRect{id,rect:layer.rect});
                    layers.push(L::SetVisible{id,visible:layer.visible});
                    layers.push(L::SetOpacity{id,opacity:layer.opacity});
                    if let crysirror_scene::layer::Paint::Solid{color}=layer.paint {
                        layers.push(L::SetSolidColor{id,color});
                    }
                }
            }else{layers.push(L::Put { layer });}
        }
        layers.extend(depth);
        layers.extend(text_layers);
        //No original/native state commit has happened. All I/O/decode is complete;
        //owner publication mints actual engine leases and can roll back new roots.
        let mut published = BTreeMap::new();let mut minted=Vec::new();
        let result = (|| -> Result<()> {
            for (id, image) in &self.ready {
                if let Some(old)=self.existing.as_ref().filter(|old|old.resident.get(id).is_some_and(|prior|Arc::ptr_eq(prior,image))) {
                    published.insert(*id,old.resources.get(id).context("CatSystem2 retained resident lost its lease")?.clone());
                    continue;
                }
                let handle = 0x46000000u32
                    .checked_add(*id)
                    .context("CatSystem2 class image ID range")?;
                published.insert(
                    *id,
                    host.publish_image_resource(client, handle, image.clone())?,
                );
                minted.push(*id);
            }
            for object_id in bank.ordered_ids() {
                let object = &bank.objects[&object_id];
                let image = match &object.state {
                    State::CImage(image) | State::Image(image) => image,
                    _ => continue,
                };
                if image.draw_id < 0 {
                    continue;
                }
                let key = ImageList::key(bank.owner, image.draw_id as u32)?;
                let Some(resource) = published.get(&object.id) else {
                    proposed.remove(key);
                    continue;
                };
                // 65C820: a missing or non-PLANE target keeps the default whole-screen
                // rectangle with no VRAM offset; ttlflow's bg_white relies on this.
                let plane = bank.lookup(&image.plane).and_then(|target| match &bank.objects[&target].state {
                    State::Plane(plane) => Some(plane),
                    _ => None,
                });
                let offset = plane.map_or([0.; 2], |plane| [plane.vram[0], plane.vram[1]]);
                //CIMAGE65C820 sends748EF0 world coords directly. IMAGE
                //65DC00 mode0 adds its pivot back before746C40 subtracts it.
                //PATHMODE1 also retains native surface origin, which canonical
                //PNG does not carry; translate that metadata here, not pixels.
                let extra = if matches!(&object.state, State::Image(_)) && image.coordinate_mode==0 {
                    let origin = if image.path_mode == 1 {
                        self.frames
                            .get(&object.id)
                            .context("CatSystem2 native IMAGE geometry unavailable")?
                            .geometry
                            .origin
                            .map(|v| v as f32)
                    } else {
                        [0.; 2]
                    };
                    [
                        image.pose.pivot[0] + origin[0],
                        image.pose.pivot[1] + origin[1],
                    ]
                } else {
                    [0.; 2]
                };
                let padding=blur_padding(image,self.frames.get(&object.id).map(|f|&f.geometry))?;
                let position = [
                    image.pose.position[0] + image.pose.offset[0] - image.pose.pivot[0]
                        + offset[0]
                        + extra[0] - padding[0] as f32,
                    image.pose.position[1] + image.pose.offset[1] - image.pose.pivot[1]
                        + offset[1]
                        + extra[1] - padding[1] as f32,
                ];
                proposed.put(Draw {
                    key,
                    image: resource.path.relative().to_owned(),
                    size: resource.size,
                    position,
                    pivot: image.pose.pivot,
                    // Native 687370 selects 688EF0 for 32bpp MASK 4. It
                    // multiplies destination alpha by source alpha, ignores
                    // source RGB and does not apply the ordinary blend weight.
                    // The pixel kernel keeps straight destination RGB and uses
                    // ((Da+1)*(Sa+1)-1)>>8 for native byte alpha. This GPU mapping
                    // uses normalized destination-in, not a promise of identical
                    // integer rounding for every alpha pair. In particular, the
                    // mask's coloured RGB must never become a visible sprite.
                    // The shared Canvas stores premultiplied colour, so mask
                    // both colour and alpha there to retain straight RGB.
                    alpha: if image.mask==4 {255} else {u8::try_from(image.pose.alpha)
                        .context("CatSystem2 class image alpha outside native renderer range")?},
                    blend: if image.mask==4 {crysirror_scene::layer::Blend::DestinationIn} else {crysirror_scene::layer::Blend::Normal},
                    // 743600 rejects zero draw alpha before selecting the
                    // channel-mask kernel, even though that kernel otherwise
                    // ignores the ordinary blend weight.
                    visible: image.pose.visible != 0 && (image.mask!=4 || image.pose.alpha!=0),
                    clip: None,
                })?;
                let clip = plane
                    .map(|plane| -> Result<[i32; 4]> {
                        Ok([
                            offset[0] as i32,
                            offset[1] as i32,
                            (offset[0] as i32).checked_add(plane.size[0]).context("CatSystem2 Plane clip overflow")?,
                            (offset[1] as i32).checked_add(plane.size[1]).context("CatSystem2 Plane clip overflow")?,
                        ])
                    })
                    .transpose()?;
                let source_clip=image.clip.map(|b|[b[0].max(0),b[1].max(0),b[2].min(resource.size[0] as i32),b[3].min(resource.size[1] as i32)]).filter(|b|b[0]<b[2]&&b[1]<b[3]);
                let origin=[(position[0]-image.pose.pivot[0]) as i32,(position[1]-image.pose.pivot[1]) as i32];
                let clip = match (clip,source_clip) {
                    (Some(a),Some(b))=>Some([a[0].max(b[0]+origin[0]),a[1].max(b[1]+origin[1]),a[2].min(b[2]+origin[0]),a[3].min(b[3]+origin[1])]),
                    (None,Some(b))=>Some([b[0]+origin[0],b[1]+origin[1],b[2]+origin[0],b[3]+origin[1]]),
                    (a,None)=>a,
                };
                if clip.is_some_and(|r|r[0]>=r[2]||r[1]>=r[3]){proposed.remove(key);}else{proposed.clip(key, clip);}
                proposed.priority(key, image.pose.priority);
            }
            //One shared transaction creates parents before Canvas work and
            //retains existing post-Canvas order; no intermediate roots/present
            //or private VM rollback of motion state is needed.
            proposed.publish_prepared(host, client, layers.clone(), final_layers)?;
            Ok(())
        })();
        if let Err(error) = result {
            for id in &minted {
                let _ = host.release_image_resource(client, published[id].path.relative());
            }
            return Err(error);
        }
        let receipt = Receipt {
            owner: self.owner,
            bank: bank.clone(),
            vm: vm.clone(),
            list: proposed,
            resources: published,
            frames: self.frames.clone(),
            root,
            resident:self.ready.clone(),
        };
        if let Some(old)=&self.existing {
            self.retiring=old.resources.iter().filter(|(id,value)|receipt.resources.get(id).is_none_or(|next|next.path!=value.path)).map(|(_,value)|value.path.relative().to_owned()).collect();
        }
        self.published = Some((root, source, receipt.clone()));
        self.retire_previous(host)?;
        Ok(receipt)
    }
}
#[derive(Clone)]
pub(crate) struct Receipt {
    pub owner: u32,
    pub bank: Bank,
    pub vm: Interpreter,
    pub list: ImageList,
    pub resources: BTreeMap<u32, UploadedImage>,
    pub frames: BTreeMap<u32, crate::native_image_resource::Prepared>,
    pub root: u32,
    pub resident:BTreeMap<u32,Arc<dyn ImageResource>>,
}
fn reference_order(bank:&Bank)->Result<Vec<u32>>{
    fn visit(bank:&Bank,id:u32,active:&mut std::collections::BTreeSet<u32>,done:&mut std::collections::BTreeSet<u32>,out:&mut Vec<u32>)->Result<()>{
        if done.contains(&id){return Ok(());}
        ensure!(active.len()<64&&active.insert(id),"CatSystem2 cyclic image reference");
        if let Some(State::Image(image)|State::CImage(image))=bank.objects.get(&id).map(|o|&o.state){if let Some(source)=image.object_reference{visit(bank,source,active,done,out)?;}}
        active.remove(&id);done.insert(id);out.push(id);Ok(())
    }
    let mut out=Vec::new();let mut active=Default::default();let mut done=Default::default();
    for id in bank.ordered_ids(){visit(bank,id,&mut active,&mut done,&mut out)?;}
    Ok(out)
}
fn resource_identity(state:&State)->Vec<u8>{
    let image=match state {State::CImage(v)|State::Image(v)=>v,_=>return Vec::new()};
    let mut out=if matches!(state,State::CImage(_)){let mut out=vec![1];for x in image.size{out.extend(x.to_le_bytes());}out.extend(image.color.to_le_bytes());out}else{let mut out=vec![2];out.extend(image.native_frame().to_le_bytes());out.extend(image.path_mode.to_le_bytes());out.extend(&image.filename);out};
    for v in image.tone {out.extend(v.to_le_bytes());}out.extend(image.filter.to_le_bytes());out.extend(image.blur);out.extend(image.object_reference.unwrap_or(0).to_le_bytes());
    if let Some(reference)=&image.reference {out.extend(reference.source.as_bytes());out.extend(reference.geometry.frame_id.to_le_bytes());out.extend((Arc::as_ptr(&reference.image) as *const () as usize).to_le_bytes());}
    if let Some(rect)=image.copy_rect {for v in rect{out.extend(v.to_le_bytes());}}
    out
}
fn blur_padding(image:&crate::native_class_state::ImageState,g:Option<&crate::native_image_resource::Geometry>)->Result<[u32;4]>{
    if image.blur==[1;2] || (image.reference.is_some()&&image.copy_rect.is_none()){return Ok([0;4]);}
    let Some(g)=g else{return Ok([0;4])};
    let [x,y]=g.origin;
    ensure!(x>=0&&y>=0,"CatSystem2 blur surface origin");
    let right=g.canvas[0].saturating_sub(x as u32).saturating_sub(g.size[0]);
    let bottom=g.canvas[1].saturating_sub(y as u32).saturating_sub(g.size[1]);
    //68DA70 passes (x,y,x,y) to the extension helper, unlike68DF70's
    //(x,x,y,y). Retain that placement policy with the repaired RGB kernel.
    if g.color_bits==24 {return Ok([(x as u32).min(image.blur[0].into()),(y as u32).min(image.blur[0].into()),right.min(image.blur[1].into()),bottom.min(image.blur[1].into())]);}
    Ok([(x as u32).min(image.blur[0].into()),(y as u32).min(image.blur[1].into()),right.min(image.blur[0].into()),bottom.min(image.blur[1].into())])
}
/// 68D6E0 and 68C000/68C1E0/68C180: byte-exact tables over shared image effects.
fn object_effects(state:&State,geometry:Option<&crate::native_image_resource::Geometry>)->Result<Vec<crysirror_image::effects::ImageEffect>> {
    use crysirror_image::{effects::ImageEffect,rgb_lut::{RgbLookup,RgbLookupProfile},fixed_luma::FixedLumaSpec};
    let v=match state {State::Image(v)|State::CImage(v)=>v,_=>return Ok(Vec::new())};
    let mut out=Vec::new();
    if v.tone != [100;3] && (v.reference.is_none()||v.copy_rect.is_some()) {
        // The native unsigned accumulator deliberately wraps for negative rates.
        let table=|rate:u32|(0u32..256).map(|x|(x.wrapping_mul(rate)/100).min(255) as u8).collect();
        out.push(ImageEffect::RgbLookup{spec:RgbLookup{red:table(v.tone[0]),green:table(v.tone[1]),blue:table(v.tone[2]),profile:RgbLookupProfile::StraightRgba}});
    }
    if v.blur != [1;2] && (v.reference.is_none()||v.copy_rect.is_some()) {
        let opaque=geometry.is_some_and(|g|g.color_bits==24);
        // Intentional compatibility repair: native68DA70's horizontal pass
        // writes only width bytes into an uninitialized RGB scratch buffer.
        // Use the shared fully initialized, three-channel convolution for
        // both axes instead. Preserve its weights, fixed16 truncation and
        // padding, but deliberately do not reproduce undefined native pixels.
        let math=if opaque {crysirror_image::triangular_blur::TriangularBlurMath::OpaqueFixed16}else{crysirror_image::triangular_blur::TriangularBlurMath::AlphaAwareFixed24};
        out.push(ImageEffect::TriangularBlur{spec:crysirror_image::triangular_blur::TriangularBlur{radius:v.blur,padding:blur_padding(v,geometry)?,math}});
    }
    match v.filter {
        1|2=>{
            // These Q16 coefficients reproduce all256 entries of the original
            // single-precision, separately truncated coefficient tables.
            out.push(ImageEffect::FixedLuma{spec:FixedLumaSpec{coefficients:[19588,38444,7503],groups:[0,1,2],fractional_bits:16,amount256:256,tint:None}});
            if v.filter==2 {
                let table=|offset:i32|(0..256).map(|x|(x+offset).clamp(0,255) as u8).collect();
                out.push(ImageEffect::RgbLookup{spec:RgbLookup{red:table(30),green:table(5),blue:table(-35),profile:RgbLookupProfile::StraightRgba}});
            }
        }
        3=>out.push(ImageEffect::Invert),
        _=>{},
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_scene::{canvas::CanvasNode, layer::Paint};
    use std::{
        io::Write,
        time::{Duration, Instant},
    };
    struct OneSubmission<'a> {
        inner: &'a mut dyn HostServices,
        calls: usize,
    }
    impl crysirror_core::service::FileSystem for OneSubmission<'_> {
        fn read(
            &mut self,
            c: ClientId,
            p: &crysirror_core::path::VfsPath,
        ) -> crysirror_core::Result<Vec<u8>> {
            self.inner.read(c, p)
        }
        fn write_atomic(
            &mut self,
            c: ClientId,
            p: &crysirror_core::path::VfsPath,
            b: &[u8],
        ) -> crysirror_core::Result<()> {
            self.inner.write_atomic(c, p, b)
        }
    }
    impl HostServices for OneSubmission<'_> {
        fn now_ms(&self) -> u64 {
            self.inner.now_ms()
        }
        fn submit(
            &mut self,
            c: ClientId,
            commands: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            self.inner.submit(c, commands)
        }
        fn query_engine(&mut self, c: ClientId, q: EngineQuery) -> crysirror_core::Result<Vec<u8>> {
            self.inner.query_engine(c, q)
        }
        fn publish_image_resource(
            &mut self,
            c: ClientId,
            id: u32,
            image: Arc<dyn ImageResource>,
        ) -> crysirror_core::Result<UploadedImage> {
            self.inner.publish_image_resource(c, id, image)
        }
        fn release_image_resource(
            &mut self,
            c: ClientId,
            path: &str,
        ) -> crysirror_core::Result<()> {
            self.inner.release_image_resource(c, path)
        }
        fn submit_engine(
            &mut self,
            c: ClientId,
            schema: u32,
            data: &[u8],
        ) -> crysirror_core::Result<()> {
            self.calls += 1;
            if self.calls != 1 {
                return Err(crysirror_core::Error::Invalid(
                    "native publisher must use one actual atomic batch",
                ));
            }
            self.inner.submit_engine(c, schema, data)
        }
    }
    fn runtime(root: &std::path::Path) -> Result<crysirror_runtime::Runtime> {
        let project = serde_json::from_value(
            serde_json::json!({"schema":2,"name":"authored typed FES resources","content_revision":"1","assets":"game","canvas":[8,8],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"unused"}]}),
        )?;
        Ok(crysirror_runtime::Runtime::with_asset_root(
            project,
            root,
            &root.join("state"),
            Default::default(),
        )?)
    }
    fn begin(
        rt: &mut crysirror_runtime::Runtime,
        pool: &TaskPool,
        source: &[u8],
    ) -> Result<Pending> {
        let program = crate::fes_program::Program::parse(source, &|| Ok(()))?;
        let vm = Interpreter::new(128)?;
        let pending = Pending::begin(
            pool,
            &mut rt.host,
            ClientId(1),
            8,
            0x1943,
            Arc::new(program.declarations),
            vm,
            StringVariables::default(),
        )?
        .context("test worker permit")?;
        Ok(pending)
    }
    fn wait(pending: &mut Pending, host: &mut dyn HostServices) -> Result<()> {
        let start = Instant::now();
        while !pending.poll(host)? {
            ensure!(
                start.elapsed() < Duration::from_secs(10),
                "typed resource worker timeout"
            );
            std::thread::yield_now();
        }
        Ok(())
    }
    const SOLID:&[u8]=b"#DEFINE\nCIMAGE sprite[2]\nPLANE view\nCPLANE background\n#OBJECT SIZE COLOR PRI DISP\nsprite 2,2 0xff03070b 3 1\nview 4,4 0x80ffffff 1 1\nbackground 4,4 0xff204060 0 1\n#OBJECT POS PL\nsprite[0] ++$0,1 view\nsprite[1] 2,2 view\n#START\nwait 1";
    #[test]
    fn whole_bank_prepares_shared_resources_and_publishes_retained_owner_receipt() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("typed-publish", 1, 2)?;
        let mut pending = begin(&mut rt, &pool, SOLID)?;
        let list = ImageList::new(ClientId(1), 100, [8, 8])?;
        assert!(pending
            .publish(&mut rt.host, ClientId(1), 50, &list)
            .is_err());
        assert!(rt.host.engine.layers.get(ClientId(1), 50).is_none());
        wait(&mut pending, &mut rt.host)?;
        let (bank, vm) = pending.prepared()?;
        assert_eq!(
            vm.flag(0),
            1,
            "OBJECT expression evaluated once before resource polls"
        );
        assert_eq!(bank.objects.len(), 5);
        assert!(pending
            .publish(&mut rt.host, ClientId(2), 50, &list)
            .is_err());
        let receipt = {
            let mut transport = OneSubmission {
                inner: &mut rt.host,
                calls: 0,
            };
            let receipt = pending.publish(&mut transport, ClientId(1), 50, &list)?;
            let repeated = pending.publish(&mut transport, ClientId(1), 50, &list)?;
            assert_eq!(
                receipt.resources.values().next().unwrap().path,
                repeated.resources.values().next().unwrap().path
            );
            assert_eq!(transport.calls, 1);
            receipt
        };
        assert_eq!(receipt.owner, 8);
        assert_eq!(receipt.root, 50);
        assert_eq!(receipt.resources.len(), 2);
        let view = receipt
            .bank
            .lookup(&crate::fes_references::IndexedName {
                name: b"view".to_vec(),
                index: 0,
            })
            .unwrap();
        let layer = rt
            .host
            .engine
            .layers
            .get(ClientId(1), layer_id(view)?)
            .unwrap();
        assert_eq!(layer.opacity, 128);
        assert!(layer.visible);
        let canvas = rt.host.engine.canvases.surface(ClientId(1), 100).unwrap();
        assert_eq!(canvas.nodes.len(), 2);
        let CanvasNode::Draw { layer: draw } = &canvas.nodes[0] else {
            panic!()
        };
        assert_eq!(draw.rect, [1., 1., 2., 2.]);
        assert_eq!(draw.clip, Some([-1., -1., 4., 4.]));
        assert!(matches!(draw.paint, Paint::Image { .. }));
        let before = serde_json::to_vec(canvas)?;
        let again = pending.publish(&mut rt.host, ClientId(1), 50, &list)?;
        assert_eq!(
            receipt
                .resources
                .iter()
                .map(|(id, r)| (*id, r.path.clone()))
                .collect::<Vec<_>>(),
            again
                .resources
                .iter()
                .map(|(id, r)| (*id, r.path.clone()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            serde_json::to_vec(rt.host.engine.canvases.surface(ClientId(1), 100).unwrap())?,
            before
        );
        assert!(pending
            .publish(&mut rt.host, ClientId(1), 51, &list)
            .is_err());
        assert!(pending.poll(&mut rt.host)?);
        assert!(rt.host.engine.canvases.surface(ClientId(2), 100).is_none());
        Ok(())
    }
    #[test]
    fn owner_publication_failure_rolls_back_roots_and_can_retry_without_replaying_vm() -> Result<()>
    {
        let root = tempfile::tempdir()?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("typed-rollback", 1, 2)?;
        let mut pending = begin(&mut rt, &pool, SOLID)?;
        wait(&mut pending, &mut rt.host)?;
        let mut invalid = ImageList::new(ClientId(1), 100, [8, 8])?;
        invalid.plane(View {
            canvas: 999,
            presentation: 998,
            parent: 997,
            source: [0, 0, 1, 1],
        })?;
        assert!(pending
            .publish(&mut rt.host, ClientId(1), 50, &invalid)
            .is_err());
        assert!(rt.host.engine.layers.get(ClientId(1), 50).is_none());
        assert!(rt.host.engine.canvases.surface(ClientId(1), 100).is_none());
        assert_eq!(pending.prepared()?.1.flag(0), 1);
        let good = ImageList::new(ClientId(1), 100, [8, 8])?;
        let receipt = pending.publish(&mut rt.host, ClientId(1), 50, &good)?;
        assert_eq!(receipt.vm.flag(0), 1);
        assert_eq!(receipt.resources.len(), 2);
        Ok(())
    }
    #[test]
    fn native_define_fifo_controls_later_updates_of_the_same_permanent_draw_key() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("typed-define-fifo", 1, 2)?;
        let source=b"#DEFINE\nCIMAGE z_first\nCIMAGE a_later\nPLANE view\n#OBJECT SIZE PRI DISP PL\nz_first 1,1 3 1 view\na_later 1,1 3 1 view\n#OBJECT SIZE DISP\nview 1,1 1\n#START\nwait 1";
        let mut pending = begin(&mut rt, &pool, source)?;
        wait(&mut pending, &mut rt.host)?;
        let receipt = pending.publish(
            &mut rt.host,
            ClientId(1),
            50,
            &ImageList::new(ClientId(1), 100, [8, 8])?,
        )?;
        let later = receipt
            .bank
            .lookup(&crate::fes_references::IndexedName {
                name: b"a_later".to_vec(),
                index: 0,
            })
            .unwrap();
        let canvas = rt.host.engine.canvases.surface(ClientId(1), 100).unwrap();
        assert_eq!(canvas.nodes.len(), 1);
        let CanvasNode::Draw { layer } = &canvas.nodes[0] else {
            unreachable!()
        };
        let Paint::Image { asset, .. } = &layer.paint else {
            unreachable!()
        };
        assert_eq!(
            asset,
            receipt.resources[&later].path.relative(),
            "later DEFINE overwrites same draw key, regardless of name lexicographic order"
        );
        Ok(())
    }
    fn frame(flags: u32) -> Vec<u8> {
        frame_geometry(flags, [0; 2], [0; 2])
    }
    fn frame_geometry(flags: u32, origin: [i32; 2], base: [i32; 2]) -> Vec<u8> {
        let compressed = |bytes: &[u8]| {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
            e.write_all(bytes).unwrap();
            e.finish().unwrap()
        };
        let pixels = compressed(&[255; 4]);
        let ctl = compressed(&[0x09, 0x01]);
        let mut out = b"HG-3".to_vec();
        for value in [12u32, 0x300, 0, 37] {
            out.extend(value.to_le_bytes());
        }
        out.extend(b"stdinfo\0");
        for value in [
            56u32,
            40,
            1,
            1,
            32,
            origin[0] as u32,
            origin[1] as u32,
            1,
            1,
            flags,
            base[0] as u32,
            base[1] as u32,
        ] {
            out.extend(value.to_le_bytes());
        }
        out.extend(b"img0000\0");
        for value in [
            0u32,
            (24 + pixels.len() + ctl.len()) as u32,
            0,
            1,
            pixels.len() as u32,
            4,
            ctl.len() as u32,
            2,
        ] {
            out.extend(value.to_le_bytes());
        }
        out.extend(pixels);
        out.extend(ctl);
        out
    }
    #[test]
    fn loaded_image_keeps_native_raw_alpha_and_uses_shared_opaque_draw_preparation() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("config"))?;
        std::fs::write(
            root.path().join("config/startup.xml"),
            b"<root><INT><int00><name>image</name><int></int><ext>hg3</ext></int00></INT></root>",
        )?;
        std::fs::write(root.path().join("native.hg3"), frame(0))?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("typed-loaded", 1, 2)?;
        let mut pending=begin(&mut rt,&pool,b"#DEFINE\nIMAGE sprite\nPLANE view\n#OBJECT FILE IDN PRI DISP PL\nsprite native 37 1 1 view\n#OBJECT SIZE DISP\nview 1,1 1\n#START\nwait 1")?;
        wait(&mut pending, &mut rt.host)?;
        let receipt = pending.publish(
            &mut rt.host,
            ClientId(1),
            50,
            &ImageList::new(ClientId(1), 100, [8, 8])?,
        )?;
        assert_eq!(receipt.frames.len(), 1);
        assert_eq!(receipt.resources.len(), 1);
        let (&id, raw) = receipt.frames.iter().next().unwrap();
        assert_eq!(raw.geometry.frame_id, 37);
        assert_eq!(raw.image.rgba().as_ref(), [128; 4]);
        assert_eq!(pending.ready[&id].rgba().as_ref(), [128, 128, 128, 255]);
        assert!(receipt.resources[&id].opaque);
        assert!(
            !raw.image.opaque(),
            "raw native queries retain source alpha independent of draw cache flag"
        );
        Ok(())
    }
    #[test]
    fn image_path_modes_retain_distinct_origin_base_and_draw_context_placement() -> Result<()> {
        for (mode, expected) in [(0, [23., 24., 1., 1.]), (1, [15., 15., 1., 1.])] {
            let root = tempfile::tempdir()?;
            std::fs::create_dir(root.path().join("config"))?;
            std::fs::write(root.path().join("config/startup.xml"),b"<root><INT><int00><name>image</name><int></int><ext>hg3</ext></int00></INT></root>")?;
            std::fs::write(
                root.path().join("native.hg3"),
                frame_geometry(7, [3, 4], [8, 9]),
            )?;
            let mut rt = runtime(root.path())?;
            let pool = TaskPool::new("typed-image-placement", 1, 2)?;
            let source=format!("#DEFINE\nIMAGE sprite\nPLANE view\n#OBJECT FILE IDN PRI DISP PL POS PATHMODE\nsprite native 37 1 1 view 20,20 {mode}\n#OBJECT SIZE DISP\nview 32,32 1\n#START\nwait 1");
            let mut pending = begin(&mut rt, &pool, source.as_bytes())?;
            wait(&mut pending, &mut rt.host)?;
            pending.publish(
                &mut rt.host,
                ClientId(1),
                50,
                &ImageList::new(ClientId(1), 100, [32, 32])?,
            )?;
            let canvas = rt.host.engine.canvases.surface(ClientId(1), 100).unwrap();
            let CanvasNode::Draw { layer } = &canvas.nodes[0] else {
                unreachable!()
            };
            assert_eq!(layer.rect, expected, "native IMAGE PATHMODE={mode}");
        }
        Ok(())
    }
    #[test]
    fn unsupported_class_and_missing_file_retain_a_sticky_failure_without_resource_admission(
    ) -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("config"))?;
        std::fs::write(
            root.path().join("config/startup.xml"),
            b"<root><INT><int00><name>image</name><int></int><ext>hg3</ext></int00></INT></root>",
        )?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("typed-explicit-errors", 1, 2)?;
        for source in [
            b"#DEFINE\nRDRAW unsupported\n#START\nwait 1".as_slice(),
        ] {
            let mut pending = begin(&mut rt, &pool, source)?;
            let error = wait(&mut pending, &mut rt.host).unwrap_err().to_string();
            assert_eq!(pending.poll(&mut rt.host).unwrap_err().to_string(), error);
            assert!(pending
                .publish(
                    &mut rt.host,
                    ClientId(1),
                    50,
                    &ImageList::new(ClientId(1), 100, [8, 8])?
                )
                .is_err());
            assert!(rt.host.engine.layers.get(ClientId(1), 50).is_none());
        }
        Ok(())
    }
}
