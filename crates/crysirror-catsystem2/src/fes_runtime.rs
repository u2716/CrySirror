// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native FES dispatch against shared engine services; no composed-screen host.
use crate::{
    fes_program::PendingKind, native_interpreter::Interpreter, native_settings::NativeSettings,
    sound_definition::SoundDefinition, task_policy::NativeTaskPolicy,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, InputObservation},
    state::ClientId,
};
pub(crate) enum Tick {
    Budget,
    Dependency,
    Waited,
}
pub(crate) struct Services<'a> {
    pub client: ClientId,
    pub pointer_glide:Option<&'a mut crate::native_classes::input_task::State>,
    pub string_variables:&'a mut crate::native_values::StringVariables,
    pub tasks: &'a mut NativeTaskPolicy,
    pub interpreter: &'a mut Interpreter,
    pub settings: &'a mut Option<NativeSettings>,
    pub sound_definition: &'a mut Option<SoundDefinition>,
    pub input_state: &'a mut crate::native_input::InputState,
    pub steam_manager: &'a mut crate::native_platform::SteamManager,
    pub fes_file_probe: &'a mut Option<crate::fes_file_probe::Pending>,
    pub fes_civil_time: &'a mut Option<crate::fes_civil_time::Pending>,
    pub fes_children:&'a mut crate::fes_children::Children,
    pub handles: Option<&'a crate::native_scene::Handles>,
    pub saves:&'a std::collections::BTreeMap<u32,crate::native_save::Container>,
    pub pool: &'a crysirror_tasks::TaskPool,
}
impl Services<'_> {
    pub fn run(&mut self,host:&mut dyn HostServices,owner:u32,origin_pc:u32,
        observation:InputObservation,budget:&mut u32)->Result<Tick> {
        #[cfg(feature="runtime")]
        if self.fes_children.panel_render.in_flight(owner)||self.fes_children.frames.in_flight(owner){
            if !self.fes_children.panel_render.advance(host,self.client,self.pool,self.tasks,owner,origin_pc,&mut self.fes_children.panels,self.string_variables)?{return Ok(Tick::Dependency);}
            if !self.fes_children.frames.advance(host,self.client,self.pool,self.tasks,self.interpreter,self.string_variables,self.sound_definition.as_ref().and_then(|d|d.policy.as_ref()),&mut self.fes_children.motions,budget,owner,origin_pc,observation)?{return Ok(Tick::Dependency);}
            self.tasks.observe_fes_child_result(owner,self.interpreter)?;
            return Ok(Tick::Waited);
        }
        let tick=self.run_frame(host,owner,origin_pc,observation,budget)?;
        if matches!(tick,Tick::Waited){
            #[cfg(feature="runtime")]
            if !self.fes_children.panel_render.advance(host,self.client,self.pool,self.tasks,owner,origin_pc,&mut self.fes_children.panels,self.string_variables)?{return Ok(Tick::Dependency);}
            #[cfg(feature="runtime")]
            if !self.fes_children.frames.advance(host,self.client,self.pool,self.tasks,self.interpreter,self.string_variables,self.sound_definition.as_ref().and_then(|d|d.policy.as_ref()),&mut self.fes_children.motions,budget,owner,origin_pc,observation)?{return Ok(Tick::Dependency);}
            self.tasks.observe_fes_child_result(owner,self.interpreter)?;
        }
        Ok(tick)
    }
    fn run_frame(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        origin_pc: u32,
        observation: InputObservation,
        budget: &mut u32,
    ) -> Result<Tick> {
        ensure!(
            observation.client == self.client,
            "CatSystem2 foreign FES input observation"
        );
        self.fes_children.validate_owner(owner,origin_pc,observation)?;
        let skip = self
            .settings
            .as_ref()
            .context("CatSystem2 FES input settings unavailable")?
            .input
            .held_sources(12, &observation)?;
        self.input_state.observe(&observation)?;
        //5DBDA0's native task counter advances at60Hz, independent of present FPS.
        let clock = (u128::from(observation.time_ms) * 60 / 1000) as u32;
        if let Some(bank)=self.tasks.fes_object_bank_mut(owner)?{self.fes_children.animations.tick(bank,clock);}
        {
            use crysirror_core::service::buttons;
            let held=|bit:u64|observation.buttons_down&bit!=0||observation.buttons_pressed&bit!=0;
            self.tasks.dispatch_fes_directions(owner,observation.revision,observation.buttons_pressed)?;
            self.tasks.dispatch_fes_buttons(owner,self.input_state.fes_position(),held(buttons::MOUSE_LEFT),held(buttons::MOUSE_RIGHT))?;
        }
        loop {
            let before = self
                .tasks
                .fes(owner)?
                .player
                .as_ref()
                .context("CatSystem2 FES player missing")?
                .executed;
            let operation = self.tasks.force_fes_tick_at_with_strings(
                owner,
                self.input_state,
                skip,
                self.interpreter,
                *budget,
                clock,
                Some(self.string_variables),
            )?;
            let after = self.tasks.fes(owner)?.player.as_ref().unwrap().executed;
            *budget = budget.saturating_sub(u32::try_from(after - before).unwrap_or(u32::MAX));
            if let Some(op) = operation {
                match (op.kind.clone(),op.name.as_slice()){

(PendingKind::Object,_)=>{
 let method=op.arguments.first().context("CatSystem2 native object method missing")?;
 #[cfg(feature="runtime")]
 let panel_method=crate::fes_save_panel::supports(method);
 #[cfg(not(feature="runtime"))]
 let panel_method=false;
 let bg_table_method = {
  let mut probe=self.interpreter.clone();
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 BGTABLE object bank")?;
  crate::fes_bg_table::applies(bank,&op.name,method,&mut probe,self.string_variables)?
 };
 if bg_table_method {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 BGTABLE object bank")?;
  crate::fes_bg_table::apply(bank,&op.name,&op.arguments,self.interpreter,self.string_variables)?;
 }else if panel_method {
  #[cfg(feature="runtime")]
  if !crate::fes_save_panel::apply(host,self.client,self.pool,owner,origin_pc,&op,&mut self.fes_children.panels,self.tasks,self.interpreter,self.string_variables)?{return Ok(Tick::Dependency);}
 }else if method==b"setanm" {
  let storage=host.storage(self.client)?;
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 setanm has no native object bank")?;
  let name=op.arguments.get(1).context("CatSystem2 setanm name")?;
  if !self.fes_children.animations.set(storage,self.pool,origin_pc,bank,&op.name,name,self.interpreter,self.string_variables)?{return Ok(Tick::Dependency);}
 }else if method==b"anm" {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 anm has no native object bank")?;
  self.fes_children.animations.run(bank,&op.name,op.arguments.get(1).map(Vec::as_slice),self.interpreter,self.string_variables)?;
 }else if method==b"loadlist2" {
  // 76D080: `loadlist2 _ group row frame`: the row-th seen string names the bitmap.
  use crate::native_classes::cg_register::Group;
  let mut vm=self.interpreter.clone();
  let group_source=op.arguments.get(2).context("CatSystem2 loadlist2 group")?;
  let row=op.arguments.get(3).map_or(Ok(0),|s|vm.evaluate(s))?;
  let frame=op.arguments.get(4).map_or(Ok(0),|s|vm.evaluate(s))?;
  let group=if matches!(group_source.first(),Some(b'$'|b'0'..=b'9')){Group::Number(vm.evaluate(group_source)?)}else{Group::Name(frame,group_source.clone())};
  let name=self.fes_children.cg.as_ref().and_then(|register|register.nth_seen(&group,row));
  if let Some(name)=name{
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 loadlist2 has no native object bank")?;
   for id in bank.targets(&op.name,&mut vm,self.string_variables)?{
    if let Some(crate::native_class_state::State::Image(image))=bank.objects.get_mut(&id).map(|o|&mut o.state){
     image.filename=name.clone();
     image.image_changed=true;
     image.select_frame(frame);
    }
   }
  }
  *self.interpreter=vm;
 }else if crate::fes_image_methods::supports(method) {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 image method has no native object bank")?;
  crate::fes_image_methods::apply(bank,&op.name,&op.arguments,self.interpreter,self.string_variables)?;
 }else if matches!(method.as_slice(),b"ref"|b"refex"|b"copy") {
  #[cfg(feature="runtime")]
  {
   let mut vm=self.interpreter.clone();
   let args=&op.arguments[1..];
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 reference has no object bank")?;
   let mut source_id=None;
   let source=if method==b"refex" {
    let handle=args.first().map_or(Ok(0),|v|vm.evaluate(v))?;
    self.handles.and_then(|h|h.images.get(&handle)).cloned()
   }else{
    let raw=args.first().context("CatSystem2 image reference argument")?;
    let reference=crate::fes_references::evaluate(raw,&mut vm,self.string_variables,&|name|bank.count(name))?;
    let id=bank.lookup(&reference);source_id=id;
    id.and_then(|id|{let receipt=self.fes_children.frames.ready.get(&owner)?;let mut frame=receipt.frames.get(&id)?.clone();if let Some(resource)=receipt.resident.get(&id){frame.image=resource.clone();}Some(frame)})
   };
   let rect=if method==b"copy" {
    let mut rect=[0i32;4];
    for (at,value) in rect.iter_mut().enumerate(){if let Some(raw)=args.get(at+1){*value=vm.evaluate(raw)? as i32;}}
    Some(rect)
   }else{None};
   for id in bank.targets(&op.name,&mut vm,self.string_variables)? {
    let object=bank.objects.get_mut(&id).unwrap();
    if (method==b"refex"&&matches!(object.declared_type,1|2|25))||(method!=b"refex"&&matches!(object.declared_type,2|6|11|12|22)) {
     if let crate::native_class_state::State::Image(image)|crate::native_class_state::State::CImage(image)=&mut object.state {
      if method==b"refex"||source_id.is_some(){image.reference=source.clone();image.object_reference=source_id.filter(|source|*source!=id);image.copy_rect=rect;}
     }
    }
   }
   *self.interpreter=vm;
  }
  #[cfg(not(feature="runtime"))]
  anyhow::bail!("CatSystem2 image references need shared resources");
 }else if method==b"attr" {
  let mut vm=self.interpreter.clone();
  let number=op.arguments.get(1).map(|a|vm.evaluate(a)).transpose()?;
  let destinations=op.arguments.iter().skip(2).take(5).map(|a|vm.evaluate(a)).collect::<Result<Vec<_>>>()?;
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 attr has no native object bank")?;
  let ids=bank.targets(&op.name,&mut vm,self.string_variables)?;
  #[cfg(feature="runtime")]
  for id in ids {
   if let Some(values)=number.and_then(|number|self.fes_children.frames.ready.get(&owner)?.frames.get(&id)?.geometry.attribute(number)) {
    for (destination,value) in destinations.iter().zip(values){if *destination!=u32::MAX{vm.set_flag(*destination,value);}}
   }
  }
  #[cfg(not(feature="runtime"))]
  {let _=(number,destinations,ids);anyhow::bail!("CatSystem2 attr needs shared image resources");}
  *self.interpreter=vm;
 }else if method==b"sndtag" {
  let mut vm=self.interpreter.clone();
  let args=&op.arguments[1..];
  let command=args.first().map(Vec::as_slice).unwrap_or_default();
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 sndtag has no object bank")?;
  let reference=crate::fes_references::evaluate(&op.name,&mut vm,self.string_variables,&|name|bank.count(name))?;
  let ids=bank.targets(&op.name,&mut vm,self.string_variables)?;
  if command==b"init" {
   for id in ids {
    if let crate::native_class_state::State::Sound(sound)=&bank.objects[&id].state {
     let name=self.string_variables.expand(&sound.filename,259)?;
     if !self.fes_children.sound_tags.init(owner,origin_pc,id,&name,host.storage(self.client)?,self.pool)?{return Ok(Tick::Dependency);}
    }
   }
   self.fes_children.sound_tags.finish(owner,origin_pc);
  }else if matches!(command,b"wait"|b"remain")&&reference.index!=-1 {
   let name=args.get(1).map(|name|self.string_variables.expand(name,255)).transpose()?.unwrap_or_default();
   let frames=ids.first().and_then(|id|match &bank.objects[id].state {crate::native_class_state::State::Sound(sound)=>Some(self.fes_children.sound_tags.remaining(*id,&name,sound.first_play_ms,observation.time_ms as u32)),_=>None}).unwrap_or(0);
   if command==b"remain" {
    if let Some(dest)=args.get(2){let dest=vm.evaluate(dest)?;vm.set_flag(dest,frames);}
   }else if frames>0 {
    *self.interpreter=vm;
    self.tasks.begin_fes_wait(owner,&op,None,frames)?;
    *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
   }
  }else if !matches!(command,b"init"|b"wait"|b"remain") {anyhow::bail!("CatSystem2 unsupported sndtag command {}",String::from_utf8_lossy(command));}
  *self.interpreter=vm;
 }else if method==b"time" {
  let mut vm=self.interpreter.clone();
  let destinations=op.arguments.iter().skip(1).take(3).map(|v|vm.evaluate(v)).collect::<Result<Vec<_>>>()?;
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 time has no object bank")?;
  for id in bank.targets(&op.name,&mut vm,self.string_variables)? {
   if let crate::native_class_state::State::Sound(sound)=&bank.objects[&id].state {
    let ms=if sound.channel>=0 {host.audio_info(self.client,sound.channel as u32)?.map_or(0,|info|info.position_frames.saturating_mul(1000)/u64::from(info.source_rate.max(1)))} else {0};
    for (dest,value) in destinations.iter().zip([ms/60000,(ms%60000)/1000,ms%1000]) {if *dest!=u32::MAX {vm.set_flag(*dest,value as u32);}}
   }
  }
  *self.interpreter=vm;
 }else if method==b"ismot" {
  let mut vm=self.interpreter.clone();
  if let Some(source)=op.arguments.get(1){
   let destination=vm.evaluate(source)?;
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 ismot has no native object bank")?;
   let mut active=false;
   for id in bank.targets(&op.name,&mut vm,self.string_variables)?{active|=bank.active_subtree(id)?;}
   vm.set_flag(destination,u32::from(active));
  }
  *self.interpreter=vm;
 }else if matches!(method.as_slice(),b"arc"|b"mcarc"|b"pend"|b"mcpend") {
  #[cfg(feature="runtime")]
  {
   let root=self.tasks.fes(owner)?.object_root.context("CatSystem2 arc has no engine root")?;
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 arc has no native object bank")?;
   if !self.fes_children.motions.builtin_path(host,self.client,self.pool,owner,root,origin_pc,&op,bank,self.interpreter,self.string_variables)?{return Ok(Tick::Dependency);}
  }
  #[cfg(not(feature="runtime"))]
  anyhow::bail!("CatSystem2 original arc motion capability is not linked");
 }else if method==b"blend" {
  #[cfg(feature="runtime")]
  {
   let root=self.tasks.fes(owner)?.object_root.context("CatSystem2 blend has no engine root")?;
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 blend has no native object bank")?;
   if !self.fes_children.motions.blend(host,self.client,self.pool,owner,root,origin_pc,&op,bank,self.interpreter,self.string_variables)?{return Ok(Tick::Dependency);}
  }
  #[cfg(not(feature="runtime"))]
  anyhow::bail!("CatSystem2 original builtin motion capability is not linked");
 }else if method==b"stop" {
  #[cfg(feature="runtime")]
  {
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 stop has no native object bank")?;
   if !self.fes_children.motions.stop(host,self.client,owner,origin_pc,&op,bank,self.interpreter,self.string_variables,budget)?{return Ok(Tick::Dependency);}
  }
  #[cfg(not(feature="runtime"))]
  anyhow::bail!("CatSystem2 original stop motion capability is not linked");
 }else if crate::fes_text::is_log_method(method)&&{
  let logs=self.tasks.logs.clone();
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?;
  crate::fes_text::log_command(bank,&op.name,&op.arguments,self.interpreter,self.string_variables,&logs)?
 } {
 }else if {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?;
  crate::fes_text::applies(bank,&op.name,method,self.interpreter,self.string_variables)?
 } {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 text has no native object bank")?;
  crate::fes_text::apply(bank,&op.name,&op.arguments,&op.raw,self.interpreter,self.string_variables)?;
 }else if crate::fes_button::supports(method)&&{
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?;
  crate::fes_button::applies(bank,&op.name,method,self.interpreter,self.string_variables)?
 } {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 BUTTON has no native object bank")?;
  crate::fes_button::apply(bank,&op.name,&op.arguments,self.interpreter,self.string_variables)?;
 }else if crate::fes_sound_table::supports(method)&&{
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object command has no native object bank")?;
  crate::fes_sound_table::is_table(bank,&op.name,self.interpreter,self.string_variables)?
 } {
  #[cfg(feature="runtime")]
  if method==b"shuffle"&&!self.fes_children.motions.ensure_random(host,self.client,owner,origin_pc,&op)?{return Ok(Tick::Dependency);}
  #[cfg(feature="runtime")]
  let random=if method==b"shuffle"{Some(self.fes_children.motions.random()?)}else{None};
  #[cfg(not(feature="runtime"))]
  let random=None;
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SNDTABLE has no native object bank")?;
  crate::fes_sound_table::apply(bank,&op.name,&op.arguments,self.interpreter,self.string_variables,random,self.client)?;
 }else if crate::fes_sound_commands::supports(method) {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SOUND command has no native object bank")?;
  crate::fes_sound_commands::apply(bank,&op.name,&op.arguments,self.interpreter,self.string_variables)?;
 }else if matches!(method.as_slice(),b"disp"|b"size"|b"pri") {
  let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 object operation has no admitted native metadata bank")?;
  bank.direct(&op.name,&op.arguments,self.interpreter,self.string_variables)?;
 }else{
  // 778050..778230: these E-mote methods only count the array and return in this engine build.
  if matches!(method.as_slice(),b"m2eff"|b"m2att"|b"m2fspeed"|b"m2prmscale"|b"m2iner"|b"m2enable_blink"|b"m2enable_lip"|b"m2splay"|b"m2draw"){
   self.tasks.acknowledge_fes_object(owner,&op)?;
   *budget=budget.saturating_sub(1);
   if *budget>0{continue;}
   return Ok(Tick::Budget);
  }
  if method==b"delete"{
   // 76C350 → 754E20 destroys the instance. Members here stay declared for the bank's
   // lookups, so a deleted one goes dark and stops taking input.
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 delete has no native object bank")?;
   let ids=bank.targets(&op.name,self.interpreter,self.string_variables)?;
   bank.direct(&op.name,&[b"disp".to_vec(),b"0".to_vec()],self.interpreter,self.string_variables)?;
   for id in ids{if let Some(object)=bank.objects.get_mut(&id){object.enabled=false;}}
   self.tasks.acknowledge_fes_object(owner,&op)?;
   *budget=budget.saturating_sub(1);
   if *budget>0{continue;}
   return Ok(Tick::Budget);
  }
  if crate::fes_object_natives::is_builtin(method){
   anyhow::bail!("CatSystem2 FES object method {} has no implemented capability; original operation remains pending",String::from_utf8_lossy(method));
  }
  #[cfg(feature="runtime")]
  {
   let task=self.tasks.fes(owner)?;
   let root=task.object_root.context("CatSystem2 alias object root has no engine resource")?;
   let alias=task.player.as_ref().unwrap().program.aliases.get(method).cloned().context("CatSystem2 unknown object method/alias")?;
   let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 alias has no native object bank")?;
   if !self.fes_children.motions.apply(host,self.client,self.pool,owner,root,origin_pc,&op,&alias,bank,self.interpreter,self.string_variables)?{return Ok(Tick::Dependency);}
  }
  #[cfg(not(feature="runtime"))]
  anyhow::bail!("CatSystem2 original motion resource capability is not linked");
 }
}

(PendingKind::BlankWait|PendingKind::SectionRestart,_)=>{
 self.tasks.begin_fes_wait(owner,&op,None,1)?;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
(PendingKind::Native,b"wait")=>{
 if op.arguments.is_empty(){
  self.tasks.begin_fes_object_wait(owner,&op)?;
  *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
 }
 let mut interpreter=self.interpreter.clone();
 let (relative,duration)=if op.arguments[0]==b"frame"{
  ensure!(op.arguments.len()<=2,"CatSystem2 FES frame wait arguments");
  (Some(if let Some(source)=op.arguments.get(1){interpreter.evaluate(source)?}else{0}),0)
 }else{
  ensure!(op.arguments.len()==1&&!matches!(op.arguments[0].as_slice(),b"string"|b"privateflag"),"CatSystem2 FES resource wait requires actual shared object observation");
  // 77A070 -> 758E00/754F90: a declared object name waits on that object.
  let named=match self.tasks.fes_object_bank_mut(owner)?{
   Some(bank) if bank.count(&op.arguments[0]).is_some()=>bank.targets(&op.arguments[0],&mut interpreter,self.string_variables)?,
   _=>Vec::new(),
  };
  if !named.is_empty(){
   self.tasks.begin_fes_wait(owner,&op,None,1)?;
   let player=self.tasks.fes_player_mut(owner)?;
   player.timing.wait=Some(crate::fes_timing::Wait::Object);
   player.object_wait=named;
   *self.interpreter=interpreter;
   *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
  }
  (None,interpreter.evaluate(&op.arguments[0])?)
 };
 self.tasks.begin_fes_wait(owner,&op,relative,duration)?;
 *self.interpreter=interpreter;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
// Developer output and Steam overlays: nothing for a player to see on this host.
(PendingKind::Native,b"dbgstr"|b"dbgstru8"|b"debug"|b"steam_enablescreenshot"|b"steam_achievement"|b"xchgcursor")=>{}
// 77AB10: `cgreg_init file key list`: the record under key in that save file, plus the
// group list, on a worker.
(PendingKind::Native,b"cgreg_init")=>{
 use crate::native_classes::cg_register::Register;
 if self.fes_children.cg_load.is_none(){
  let mut vm=self.interpreter.clone();
  let file=String::from_utf8_lossy(&self.string_variables.expand(op.arguments.first().context("CatSystem2 cgreg_init file")?,259)?).into_owned();
  let key=op.arguments.get(1).map_or(Ok(0),|s|vm.evaluate(s))?;
  let list=String::from_utf8_lossy(&self.string_variables.expand(op.arguments.get(2).context("CatSystem2 cgreg_init list")?,259)?).into_owned();
  let storage=host.storage(self.client)?;let client=self.client;
  let permit=match self.pool.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(Tick::Dependency),Err(e)=>return Err(e)};
  self.fes_children.cg_load=Some(permit.spawn(move|cancel|{
   let saved=crate::native_save::load(storage.as_ref(),client,&file,cancel)?;
   let startup=crate::native_settings::optional_asset(storage.as_ref(),"config/startup.xml")?.context("CatSystem2 startup missing for gallery list")?;
   let startup=crysirror_resource::xml::XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?;
   let bytes=crate::native_settings::optional_native_asset(storage.as_ref(),&startup,&list)?;
   Ok(Register::from_parts(saved.as_ref().and_then(|s|s.first(key)),bytes.as_deref()))
  })?);
  return Ok(Tick::Dependency);
 }
 let Some(register)=self.fes_children.cg_load.as_mut().unwrap().try_take()? else{return Ok(Tick::Dependency);};
 self.fes_children.cg_load=None;
 self.fes_children.cg=Some(register);
}
// Window chrome and developer hooks with no effect on this host's scene.
(PendingKind::Native,b"deletebgbutton"|b"force_redraw"|b"wndmenu"|b"decode"|b"resetactive"|b"sendbtn")=>{}
// 77BEE0: `send target a b [wait]` posts a message the owning KCS reads from native71.
(PendingKind::Native,b"maketask"|b"deletetask")=>{
 //77FB90/77FD00 enqueue record16. 629850 forwards it to the owning
 //KCS input task, not the native66 factory: the game script creates/deletes
 //its named task in response to message103/104 from native71.
 let create=op.name==b"maketask";
 ensure!(op.arguments.len()==if create{3}else{1},"CatSystem2 FES task-message arguments");
 let mut interpreter=self.interpreter.clone();
 let number=interpreter.evaluate(&op.arguments[0])?;
 if create{
  let mut strings=self.string_variables.clone();
  let mut name=strings.expand(&op.arguments[1],2559)?;
  //6A6A10 replaces only the extension after the last path separator.
  if let Some(dot)=name.iter().rposition(|b|matches!(b,b'.'|b'/'|b'\\')){
   if name[dot]==b'.'{name.truncate(dot);}
  }
  name.extend_from_slice(b".xml");
  let parameters=strings.expand(&op.arguments[2],2559)?;
  strings.assign(700,name)?;
  strings.assign(701,parameters)?;
  *self.string_variables=strings;
 }
 self.tasks.post_script_event(owner,[owner,if create{103}else{104},number,if create{700}else{0}]);
 *self.interpreter=interpreter;
 self.tasks.acknowledge_fes_native(owner,&op)?;
 *budget=budget.saturating_sub(1);
 //The original -3 yields to message dispatch without adding a frame wait.
 return Ok(Tick::Waited);
}
(PendingKind::Native,b"send")=>{
 let mut interpreter=self.interpreter.clone();
 let mut word=|i:usize|->Result<u32>{op.arguments.get(i).map_or(Ok(0),|source|interpreter.evaluate(source))};
 let (target,a,b)=(word(0)?,word(1)?,word(2)?);
 self.tasks.post_script_event(owner,[owner,target,a,b]);
 *self.interpreter=interpreter;
}
// 77ADF0, 77AF20, 77B050: `dest [list] group`; a group starting with $, \ or a digit is a
// record number, anything else a name in that list. Without a register they store 0.
(PendingKind::Native,b"cgreg_t"|b"cgregnum"|b"cgregmax")=>{
 use crate::native_classes::cg_register::Group;
 let mut vm=self.interpreter.clone();
 let dest=vm.evaluate(op.arguments.first().context("CatSystem2 cgreg destination")?)?;
 let (list,source)=match op.arguments.len(){3=>(vm.evaluate(&op.arguments[1])?,Some(&op.arguments[2])),_=>(0,op.arguments.get(1))};
 let group=match source{
  Some(s) if matches!(s.first(),Some(b'$'|b'\\'|b'0'..=b'9'))=>Some(Group::Number(vm.evaluate(s)?)),
  Some(s)=>Some(Group::Name(list,s.clone())),
  None=>None,
 };
 let value=match (&self.fes_children.cg,group){
  (Some(register),Some(group))=>match op.name.as_slice(){
   b"cgreg_t"=>u32::from(register.any_seen(&group)),
   b"cgregnum"=>register.seen_count(&group),
   _=>register.total(&group),
  },
  _=>0,
 };
 vm.set_flag(dest,value);
 *self.interpreter=vm;
}
// 77AD00: `dest name,variant,flags`.
(PendingKind::Native,b"cgreg")=>{
 let mut vm=self.interpreter.clone();
 let dest=vm.evaluate(op.arguments.first().context("CatSystem2 cgreg destination")?)?;
 let text=self.string_variables.expand(op.arguments.get(1).context("CatSystem2 cgreg key")?,2560)?;
 let value=self.fes_children.cg.as_ref().is_some_and(|register|register.query(&text));
 vm.set_flag(dest,u32::from(value));
 *self.interpreter=vm;
}
// 77B180: `dest list name`.
(PendingKind::Native,b"cgregtbl")=>{
 let mut vm=self.interpreter.clone();
 let dest=vm.evaluate(op.arguments.first().context("CatSystem2 cgregtbl destination")?)?;
 let list=op.arguments.get(1).map_or(Ok(0),|s|vm.evaluate(s))?;
 let value=match (&self.fes_children.cg,op.arguments.get(2)){(Some(register),Some(name))=>register.ordinal(list,name) as u32,_=>0};
 vm.set_flag(dest,value);
 *self.interpreter=vm;
}
(PendingKind::Native,b"steam_isbroadcast")=>{
 let mut interpreter=self.interpreter.clone();
 if let Some(source)=op.arguments.first(){let destination=interpreter.evaluate(source)?;interpreter.set_flag(destination,0);}
 *self.interpreter=interpreter;
}
// 780290/780490: `rdraw frames [zlo zhi]`, `rwipe frames [rule zlo zhi]` make a
// screen transition object; an all-object wait lasts until it completes. A second
// command while one runs is ignored. The depth range always spans the whole screen here.
#[cfg(feature="runtime")]
(PendingKind::Native,b"rdraw"|b"rwipe")=>{
 let running=self.tasks.fes(owner)?.player.as_ref().unwrap().transition_until.is_some();
 if !running{
  let wipe=op.name==b"rwipe";
  let mut interpreter=self.interpreter.clone();
  let frames=if let Some(source)=op.arguments.first(){interpreter.evaluate(source)?}else{30};
  let frames=if wipe{frames}else{frames.max(1)};
  let rule=if wipe{op.arguments.get(1).map(|source|self.string_variables.expand(source,259)).transpose()?.filter(|name|!name.is_empty())}else{None};
  let effect=match rule{
   Some(name)=>{
    let name=String::from_utf8(crate::native_scene::stem(&name).to_vec()).context("CatSystem2 rwipe rule name encoding")?;
    let pending=match self.fes_children.rule.take(){
     Some(pending)=>pending,
     None=>{
      let Some(pending)=crate::native_image_resource::Pending::begin_named(self.pool,host,self.client,owner,origin_pc,&name,0)? else{return Ok(Tick::Dependency);};
      pending
     }
    };
    let mut pending=pending;
    let Some(outcome)=pending.poll()? else{self.fes_children.rule=Some(pending);return Ok(Tick::Dependency);};
    match outcome{
     crate::native_image_resource::Outcome::Ready(frame)=>{
      let upload=host.publish_image_resource(self.client,0x4b00_0000+(owner&0xffff),frame.image.clone())?;
      serde_json::json!({"kind":"mask","asset":upload.path.relative(),"fade":4})
     }
     _=>serde_json::json!({"kind":"crossfade"}),
    }
   }
   None=>serde_json::json!({"kind":"crossfade"}),
  };
  if frames as i32>0{
   let duration=(u64::from(frames)*1000/60).clamp(1,600_000);
   host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"transition","transition":{"effect":effect,"duration_ms":duration}}]}))?)?;
   let player=self.tasks.fes_player_mut(owner)?;
   player.transition_until=Some(player.timing.now.wrapping_add(frames));
  }
  *self.interpreter=interpreter;
 }
}
// 77B270, 77EB10, 77ED00, 77EBE0, 77EF20, 77B520, 77F530, 77F6B0, 785340: slot files.
(PendingKind::Native,b"saveexist"|b"savedelete"|b"savecopy"|b"saveexchg"|b"newsave"|b"datasave"|b"saveapend_str"|b"saveget_str"|b"saveget_date")=>{
 if self.fes_children.save.is_none(){
  use crate::fes_save::{self as save,Answer,Job,slot_name};
  let mut vm=self.interpreter.clone();
  let mut word=|at:usize|->Result<u32>{op.arguments.get(at).map_or(Ok(0),|source|vm.evaluate(source))};
  let storage=host.storage(self.client)?;let client=self.client;
  let pool=self.pool;
  let begun=match op.name.as_slice(){
   b"saveexist"=>{
    let numeric=op.arguments.first().is_some_and(|a|!a.first().is_some_and(|c|c.is_ascii_alphabetic()));
    let name=if numeric{slot_name(word(0)?)}else{String::from_utf8_lossy(&self.string_variables.expand(&op.arguments[0],259)?).into_owned()};
    let dest=word(1)?;
    let range=op.arguments.get(2).map(|_|->Result<[u32;2]>{let low=word(2)?;let high=if op.arguments.len()>3{word(3)?}else{low};Ok([low,high])}).transpose()?;
    Job::begin(pool,owner,origin_pc,move|cancel|Ok(Answer{flags:vec![(dest,save::exists(storage.as_ref(),client,&name,range,cancel)?)],string:None}))?
   }
   b"savedelete"=>{let name=slot_name(word(0)?);Job::begin(pool,owner,origin_pc,move|_|{save::delete(storage.as_ref(),&name)?;Ok(Answer::default())})?}
   b"savecopy"=>{let (a,b)=(slot_name(word(0)?),slot_name(word(1)?));Job::begin(pool,owner,origin_pc,move|_|{save::copy(storage.as_ref(),&a,&b)?;Ok(Answer::default())})?}
   b"saveexchg"=>{let (a,b)=(slot_name(word(0)?),slot_name(word(1)?));Job::begin(pool,owner,origin_pc,move|_|{save::exchange(storage.as_ref(),&a,&b)?;Ok(Answer::default())})?}
   b"newsave"=>{let (dest,low,high)=(word(0)?,word(1)?,word(2)?);Job::begin(pool,owner,origin_pc,move|_|Ok(Answer{flags:vec![(dest,save::newest(storage.as_ref(),[low,high])?)],string:None}))?}
   b"datasave"=>{
    let (slot,handle)=(word(0)?,word(1)?);
    let saved=self.saves.get(&handle).cloned();
    let name=slot_name(slot);
    Job::begin(pool,owner,origin_pc,move|_|{if let Some(saved)=saved{save::write(storage.as_ref(),&name,&saved)?;}Ok(Answer::default())})?
   }
   b"saveapend_str"=>{
    let (slot,key)=(word(0)?,word(1)?);
    let text=op.arguments.get(2).map(|source|self.string_variables.expand(source,2048)).transpose()?.unwrap_or_default();
    let name=slot_name(slot);
    Job::begin(pool,owner,origin_pc,move|cancel|{save::append_string(storage.as_ref(),client,&name,key,&text,cancel)?;Ok(Answer::default())})?
   }
   b"saveget_str"=>{
    let (slot,key,target)=(word(0)?,word(1)?,word(2)?);
    let name=slot_name(slot);
    Job::begin(pool,owner,origin_pc,move|cancel|Ok(Answer{flags:Vec::new(),string:save::get_string(storage.as_ref(),client,&name,key,cancel)?.map(|text|(target,text))}))?
   }
   _=>{
    let slot=word(0)?;
    let targets=(1..7).map(|at|if op.arguments.len()>at{word(at)}else{Ok(u32::MAX)}).collect::<Result<Vec<_>>>()?;
    Job::begin(pool,owner,origin_pc,move|_|{
     let Some(date)=save::date(storage.as_ref(),slot)? else{return Ok(Answer::default());};
     Ok(Answer{flags:targets.iter().zip(date).filter(|(flag,_)|**flag!=u32::MAX).map(|(flag,value)|(*flag,value)).collect(),string:None})
    })?
   }
  };
  let Some(job)=begun else{return Ok(Tick::Dependency);};
  self.fes_children.save=Some(job);
  *self.interpreter=vm;
 }
 let job=self.fes_children.save.as_mut().unwrap();
 ensure!(job.owner==owner&&job.pc==origin_pc,"CatSystem2 FES save job ownership mismatch");
 let Some(answer)=job.poll()? else{return Ok(Tick::Dependency);};
 self.fes_children.save=None;
 for (flag,value) in answer.flags{self.interpreter.set_flag(flag,value);}
 if let Some((target,text))=answer.string{self.string_variables.assign(target,text)?;}
}
(PendingKind::Native,b"_redirect"|b"_redirect_erase"|b"_redirect_clear")=>{
 let storage=host.storage(self.client)?;
 let words=op.arguments.iter().map(|word|{
  let bytes=self.string_variables.expand(word,1024)?;
  let name=std::str::from_utf8(&bytes).context("CatSystem2 redirect resource encoding")?.replace('\\',"/");
  Ok(name)
 }).collect::<Result<Vec<String>>>()?;
 match op.name.as_slice(){
  b"_redirect"=>{
   ensure!(words.len()==4,"CatSystem2 redirect arguments");
   //6A5610 cannot advance on an empty find string; reject that native
   //infinite-loop input rather than acknowledging a different substitution.
   ensure!(!words[2].is_empty(),"CatSystem2 redirect empty search would not terminate in the original engine");
   storage.insert_asset_redirect(&words[0],&words[1],&words[2],&words[3])?;
  }
  b"_redirect_erase"=>{ensure!(words.len()==1,"CatSystem2 redirect erase arguments");storage.erase_asset_redirect(&words[0])?;}
  _=>{ensure!(words.is_empty(),"CatSystem2 redirect clear arguments");storage.clear_asset_redirects()?;}
 }
}
//77B640/77B820/7814A0 ->629850 messages9/10/30 ->INPUT137.
(PendingKind::Native,b"automove"|b"restoremove"|b"automoveex")=>{
 let enabled=self.settings.as_ref().context("CatSystem2 pointer settings")?.configuration(b"autocur",0,0)?!=0;
  let mut vm=self.interpreter.clone();
  let target=match op.name.as_slice(){
   b"restoremove"=>self.tasks.fes(owner)?.saved_cursor.map(|p|(p.map(|v|i32::from(v as u16)),4)),
   b"automoveex"=>{
    ensure!((2..=3).contains(&op.arguments.len()),"CatSystem2 automoveex arguments");
    let x=i32::from(vm.evaluate(&op.arguments[0])? as u16);
    let y=i32::from(vm.evaluate(&op.arguments[1])? as u16);
    let duration=op.arguments.get(2).map_or(Ok(8),|v|vm.evaluate(v))? as i32;
    Some(([x,y],duration))
   }
   _=>{
    ensure!(op.arguments.len()==1,"CatSystem2 automove arguments");
    let bank=self.tasks.fes_object_bank_mut(owner)?.context("CatSystem2 automove object bank")?;
    let reference=crate::fes_references::evaluate(&op.arguments[0],&mut vm,self.string_variables,&|name|bank.count(name))?;
    bank.lookup(&reference).and_then(|id|crate::fes_button::key_position(bank,id)).map(|p|(p.map(|v|i32::from(v as u16)),4))
   }
  };
  if let Some((target,duration))=target.filter(|_|enabled){
   let rect=self.tasks.fes(owner)?.native_rect;
   if op.name!=b"restoremove" {self.tasks.save_fes_cursor(owner,self.input_state.fes_position())?;}
   if target[0]<rect[2]&&target[1]<rect[3]&&host.window_focused(self.client)?!=Some(false){
    self.pointer_glide.as_deref_mut().context("CatSystem2 FES pointer input owner missing")?.begin_glide(host.now_ms(),target,duration);
   }
  }
  *self.interpreter=vm;
}
// 77B880, 77BE30, 7813A0: button-pass requests that take effect on the next frame.
(PendingKind::Native,b"react"|b"basebtnpri"|b"btnsimulate")=>{
 self.tasks.begin_fes_wait(owner,&op,None,1)?;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
// 7846E0, 77C110: the configured system voice: none is registered, so -1.
(PendingKind::Native,b"sysvoice"|b"sysvoiceex")=>{
 let mut vm=self.interpreter.clone();
 if let Some(source)=op.arguments.first(){let flag=vm.evaluate(source)?;vm.set_flag(flag,u32::MAX);}
 *self.interpreter=vm;
}
// 786360, 786840: E-mote options, keyed by word; the labelled ones take a last word.
(PendingKind::Native,b"m2_getconf")=>{
 let key=op.arguments.first().context("CatSystem2 m2_getconf key")?;
 let label=op.arguments.get(2).map(|l|String::from_utf8_lossy(l).into_owned()).unwrap_or_default();
 let settings=self.settings.as_ref().context("CatSystem2 m2_getconf settings")?;
 if let (Some(value),Some(source))=(settings.m2.get(key,&label),op.arguments.get(1)){
  let mut vm=self.interpreter.clone();
  let flag=vm.evaluate(source)?;vm.set_flag(flag,value);
  *self.interpreter=vm;
 }
}
(PendingKind::Native,b"m2_setconf")=>{
 let key=op.arguments.first().context("CatSystem2 m2_setconf key")?;
 let mut vm=self.interpreter.clone();
 let value=op.arguments.get(1).map(|source|vm.evaluate(source)).transpose()?;
 *self.interpreter=vm;
 let label=op.arguments.get(2).map(|l|String::from_utf8_lossy(l).into_owned()).unwrap_or_default();
 let settings=self.settings.as_mut().context("CatSystem2 m2_setconf settings")?;
 settings.m2.set(key,value,&label);
 let config=settings.m2.clone();
 crate::native_m2::save_m2_config(host.storage(self.client)?.as_ref(),&config)?;
}
// 780B00..781180: the read-marks database of KCS native 869, by the handle in the first word.
(PendingKind::Native,b"dic_find"|b"dic_appear"|b"dic_title"|b"dic_ind"|b"dic_max_appear"|b"dic_update"|b"dic_update_clear"|b"dic_id")=>{
 let mut vm=self.interpreter.clone();
 let number=|at:usize,vm:&mut crate::native_interpreter::Interpreter|->Result<u32>{vm.evaluate(op.arguments.get(at).context("CatSystem2 dic argument")?)};
 let handle=number(0,&mut vm)?;
 let text=|at:usize,strings:&crate::native_values::StringVariables|->Result<Vec<u8>>{strings.expand(op.arguments.get(at).context("CatSystem2 dic text")?,259)};
 let settings=self.settings.as_mut().context("CatSystem2 dic settings")?;
 let database=settings.databases.get_mut(&handle);
 match op.name.as_slice(){
  b"dic_find"=>{
   let (flag,variable,nth)=(number(1,&mut vm)?,number(2,&mut vm)?,number(3,&mut vm)?);
   let (from,to)=(text(4,self.string_variables)?,text(5,self.string_variables)?);
   let found=database.and_then(|d|d.find(&from,&to,nth).map(<[u8]>::to_vec));
   vm.set_flag(flag,u32::from(found.is_some()));
   self.string_variables.assign(variable,found.unwrap_or_default())?;
  }
  b"dic_appear"|b"dic_max_appear"|b"dic_id"=>{
   let flag=number(1,&mut vm)?;
   let name=text(2,self.string_variables)?;
   let value=database.map_or(0,|d|match op.name.as_slice(){b"dic_appear"=>d.appear(&name),b"dic_max_appear"=>d.max_appear(&name),_=>d.id(&name)});
   vm.set_flag(flag,value);
  }
  b"dic_title"=>{
   let (title,short)=(number(1,&mut vm)?,number(2,&mut vm)?);
   let name=text(3,self.string_variables)?;
   let found=database.and_then(|d|d.titles(&name).map(|(a,b)|(a.to_vec(),b.to_vec()))).unwrap_or_default();
   self.string_variables.assign(title,found.0)?;
   self.string_variables.assign(short,found.1)?;
  }
  b"dic_ind"=>{
   let variable=number(1,&mut vm)?;
   let name=text(2,self.string_variables)?;
   self.string_variables.assign(variable,database.and_then(|d|d.ind(&name)).unwrap_or_default())?;
  }
  b"dic_update"=>{
   let (first,second)=(number(1,&mut vm)?,number(2,&mut vm)?);
   let name=text(3,self.string_variables)?;
   let (a,b)=database.map_or((0,0),|d|d.update_flags(&name));
   vm.set_flag(first,a);vm.set_flag(second,b);
  }
  _=>{
   let name=op.arguments.get(1).map(|_|text(1,self.string_variables)).transpose()?;
   if let Some(database)=database{database.clear_updates(name.as_deref());}
  }
 }
 *self.interpreter=vm;
}
// 781AC0, 781D80: slots ordered newest file first into consecutive flags; a slot with no
// file has the zero time. Equal times keep their order.
(PendingKind::Native,b"sortsave"|b"sortsave2")=>{
 let mut vm=self.interpreter.clone();
 let first=vm.evaluate(op.arguments.first().context("CatSystem2 sortsave flag")?)?;
 let slots:Vec<u32>=if op.name==b"sortsave"{
  let from=vm.evaluate(op.arguments.get(1).context("CatSystem2 sortsave from")?)?;
  let to=vm.evaluate(op.arguments.get(2).context("CatSystem2 sortsave to")?)?;
  if to>from{(from..=to).collect()}else{Vec::new()}
 }else{
  op.arguments[1..].iter().map(|source|vm.evaluate(source)).collect::<Result<_>>()?
 };
 let storage=host.storage(self.client)?;
 let mut timed=Vec::with_capacity(slots.len());
 for slot in slots{
  let name=format!("save{slot:04}.dat");
  let time=storage.metadata(crysirror_core::path::Mount::Save,&name).ok().flatten().and_then(|m|m.modified_ms).unwrap_or(0);
  timed.push((time,slot));
 }
 timed.sort_by(|a,b|b.0.cmp(&a.0));
 for (at,(_,slot)) in timed.iter().enumerate(){vm.set_flag(first+at as u32,*slot);}
 *self.interpreter=vm;
}
// 77FE60: the system font list. This player does not enumerate host fonts, so the list is
// empty: count answers 0 and a search finds nothing.
(PendingKind::Native,b"fontcom")=>{
 let mut vm=self.interpreter.clone();
 match op.arguments.first().map(Vec::as_slice){
  Some(b"count")=>{
   if let Some(source)=op.arguments.get(1){let flag=vm.evaluate(source)?;vm.set_flag(flag,0);}
  }
  Some(b"search")=>{
   if let Some(source)=op.arguments.get(2){let variable=vm.evaluate(source)?;self.string_variables.assign(variable,Vec::new())?;}
  }
  _=>{}
 }
 *self.interpreter=vm;
}
// 785A10: with no list entry found the string variable is emptied.
(PendingKind::Native,b"cgreg_getlist")=>{
 let mut vm=self.interpreter.clone();
 if let Some(source)=op.arguments.first(){
  let variable=vm.evaluate(source)?;
  if variable!=u32::MAX{self.string_variables.assign(variable,Vec::new())?;}
 }
 *self.interpreter=vm;
}
// 781310: the set bits of a value into a flag.
(PendingKind::Native,b"bitcount")=>{
 let mut vm=self.interpreter.clone();
 if let (Some(target),Some(source))=(op.arguments.first(),op.arguments.get(1)){
  let flag=vm.evaluate(target)?;let value=vm.evaluate(source)?;vm.set_flag(flag,value.count_ones());
 }
 *self.interpreter=vm;
}
// 7823A0 reads the debug switch (off); 784B50 the pixel shader version, where this renderer
// meets the 3.0 profile the scripts test for; 785610 mouse and keyboard; 782B40 finds no
// installed companion title; 7822B0 has no video memory figures to report.
(PendingKind::Native,b"isdebug"|b"psversion"|b"inputdevice"|b"check_install"|b"vram_get")=>{
 let value=if op.name==b"psversion"{0x300}else{0};
 let mut vm=self.interpreter.clone();
 for source in op.arguments.iter().take(if op.name==b"vram_get"{2}else{1}){
  let flag=vm.evaluate(source)?;
  if flag!=u32::MAX{vm.set_flag(flag,value);}
 }
 *self.interpreter=vm;
}
// 782160, 7821E0: the frame counters report the configured rate.
(PendingKind::Native,b"fps_enable")=>{}
(PendingKind::Native,b"fps_get")=>{
 let rate=self.settings.as_ref().map_or(60,|settings|settings.fps);
 let mut vm=self.interpreter.clone();
 for source in op.arguments.iter().take(2){
  let flag=vm.evaluate(source)?;
  if flag!=u32::MAX{vm.set_flag(flag,rate);}
 }
 *self.interpreter=vm;
}
// 779E20, 779EA0: values kept by name for the session.
(PendingKind::Native,b"setexvar")=>{
 let name=op.arguments.first().context("CatSystem2 setexvar name")?;
 let mut vm=self.interpreter.clone();
 let value=op.arguments.get(1).map(|source|vm.evaluate(source)).transpose()?.unwrap_or(0);
 *self.interpreter=vm;
 let key=[b"exvar:",name.as_slice()].concat();
 self.settings.as_mut().context("CatSystem2 setexvar settings")?.extras.insert(key,value);
}
(PendingKind::Native,b"getexvar")=>{
 if let (Some(name),Some(source))=(op.arguments.first(),op.arguments.get(1)){
  let key=[b"exvar:",name.as_slice()].concat();
  let value=self.settings.as_ref().and_then(|settings|settings.extras.get(&key).copied()).unwrap_or(0);
  let mut vm=self.interpreter.clone();
  let flag=vm.evaluate(source)?;vm.set_flag(flag,value);
  *self.interpreter=vm;
 }
}
// The rest are requests to the window or debug output with no effect on script state:
// 784BC0 warps the cursor, 782A30/782A90 touch input, 7820B0 the background button, 77F2C0
// a debug timer, 782DB0 a relaunch path, 786D40 a view reset, 786D60 waits on E-mote effects
// that finish at once here, 77B8E0 retitles the window, 785880 resets Steam achievements.
(PendingKind::Native,b"cursormove"|b"touch_kill"|b"touch_enable"|b"enablebgbutton"|b"dbgtime"|b"exit_boot"|b"m2_resetview"|b"m2_waiteff"|b"checktitle"|b"steam_achievement_reset")=>{
 host.append_log(self.client,&format!("CatSystem2 FES {} ignored: no host capability",String::from_utf8_lossy(&op.name)))?;
}
// The fw_* disc and activation commands do nothing, by owner decision: the scripts set
// their own status flag ($700=2) straight after fw_status.
(PendingKind::Native,b"fw_init"|b"fw_hash"|b"fw_limitpos"|b"fw_auth"|b"fw_activate"|b"fw_status"|b"fw_actcode"|b"fw_uniquecode"|b"fw_destroy")=>{}
// 77BAC0: a volume into a flag; other names leave it.
(PendingKind::Native,b"config")=>{
 let name=op.arguments.get(1).context("CatSystem2 config name")?.to_ascii_lowercase();
 let source=op.arguments.first().context("CatSystem2 config flag")?;
 let field:Option<&[u8]>=match name.as_slice(){b"vol_bgm"=>Some(b"vol_bgm"),b"vol_se"=>Some(b"vol_se"),b"vol_pcm"=>Some(b"vol_voice"),_=>None};
 if let Some(field)=field{
  let value=self.settings.as_ref().context("CatSystem2 config settings")?.configuration(field,0,0)?;
  let mut vm=self.interpreter.clone();
  let flag=vm.evaluate(source)?;vm.set_flag(flag,value);
  *self.interpreter=vm;
 }
}
// 77E8E0, 784C90, 77EA20: a browser and the system clipboard are outside this player.
(PendingKind::Native,b"opensite"|b"opensiteenc"|b"clipboard")=>{
 host.append_log(self.client,&format!("CatSystem2 FES {} ignored: no host capability",String::from_utf8_lossy(&op.name)))?;
}
// 77F150: posts message 22 about the window's active lock; the host owns focus.
(PendingKind::Native,b"lockactive")=>{}
(PendingKind::Native,b"input_text"|b"messagebox"|b"config_open")=>{
 host.append_log(self.client,&format!("CatSystem2 FES {} has no host dialog yet",String::from_utf8_lossy(&op.name)))?;
}
// 77E890 event 19: the owner's script returns to its title.
(PendingKind::Native,b"rettitle")=>{
 self.tasks.post_script_event(owner,[owner,19,0,0]);
 self.tasks.begin_fes_wait(owner,&op,None,1)?;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
(PendingKind::Native,b"break")=>{
 self.tasks.fes_player_mut(owner)?.break_section(&op)?;
 *budget=budget.saturating_sub(1);
 if *budget>0{continue;}
 return Ok(Tick::Budget);
}
(PendingKind::Native,b"strvar")=>{
 crate::fes_strings::assign(&op.arguments,self.interpreter,self.string_variables)?;
}
#[cfg(feature="runtime")]
(PendingKind::Native,b"rand"|b"randtbl")=>{
 let table=op.name==b"randtbl";let need=if table{4}else{3};
 if op.arguments.len()>=need {
  if !self.fes_children.motions.ensure_random(host,self.client,owner,origin_pc,&op)?{return Ok(Tick::Dependency);}
  crate::fes_random::apply(table,&op.arguments,self.interpreter,Some(self.fes_children.motions.random()?),self.client)?;
 }else{crate::fes_random::apply(table,&op.arguments,self.interpreter,None,self.client)?;}
}
#[cfg(feature="runtime")]
(PendingKind::Native,b"baseimg")=>{
 self.base_image(host,owner,&op)?;
}
#[cfg(feature="runtime")]
(PendingKind::Native,b"baseimg_pri")=>{
 self.base_image_priority(host,owner,&op)?;
}
(PendingKind::Native,b"strcmp")=>{
 crate::fes_strings::compare(&op.arguments,self.interpreter)?;
}
(PendingKind::Native,b"setstbl"|b"getstbl")=>{
 crate::fes_strings::table_command(op.name==b"setstbl",&op.arguments,self.interpreter,
  self.string_variables,self.tasks.fes_string_table_mut(owner)?)?;
}
(PendingKind::Native,b"replacespace")=>{
 crate::fes_strings::replace_space(&op,self.interpreter,self.string_variables)?;
}
(PendingKind::Native,b"exit"|b"result")=>{
 ensure!(op.arguments.len()<=1,"CatSystem2 FES exit/result arguments");
 let mut interpreter=self.interpreter.clone();
 let value=if let Some(source)=op.arguments.first(){interpreter.evaluate(source)?}else{0};
 if op.name==b"exit"{
  //77A580 queues event6;629850 writes task20 and then resumes
  //the SAME FES frame. Only the parent endfes destroys the child.
  self.tasks.set_task_result(owner,value)?;
 }else{
  //77FDF0 writes player519, never task20 or parent's108.
  self.tasks.set_fes_player_result(owner,value)?;
 }
 *self.interpreter=interpreter;
}
(PendingKind::Native,b"getweek")=>{
 ensure!(op.arguments.len()==4,"CatSystem2 FES getweek arguments");
 let mut interpreter=self.interpreter.clone();
 let destination=interpreter.evaluate(&op.arguments[0])?;
 let year=interpreter.evaluate(&op.arguments[1])? as i32;
 let month=u8::try_from(interpreter.evaluate(&op.arguments[2])?).context("CatSystem2 getweek month outside native date profile")?;
 let day=u8::try_from(interpreter.evaluate(&op.arguments[3])?).context("CatSystem2 getweek day outside native date profile")?;
 //77EE20->6A7450 has the same Sunday0 Gregorian result for
 //valid civil dates. Invalid native arithmetic dates are not normalized.
 let weekday=crysirror_core::service::civil_weekday(year,month,day).context("CatSystem2 getweek invalid/out-of-profile civil date")?;
 interpreter.set_flag(destination,u32::from(weekday));
 *self.interpreter=interpreter;
}
(PendingKind::Native,b"gettime")=>{
 ensure!(op.arguments.len()<=7,"CatSystem2 FES gettime arguments");
 if self.fes_civil_time.is_none(){
  *self.fes_civil_time=Some(crate::fes_civil_time::Pending::begin(host,self.client,owner,origin_pc,op.clone())?);
  return Ok(Tick::Dependency);
 }
 let pending=self.fes_civil_time.as_mut().unwrap();
 ensure!(pending.owner==owner&&pending.pc==origin_pc&&pending.operation==op,"CatSystem2 FES civil clock ownership mismatch");
 let Some(value)=pending.poll()? else{return Ok(Tick::Dependency);};
 let values=[value.year as u32,u32::from(value.month),u32::from(value.day),
  u32::from(value.weekday().context("CatSystem2 civil weekday unavailable")?),
  u32::from(value.hour),u32::from(value.minute),u32::from(value.second)];
 let mut interpreter=self.interpreter.clone();
 for (destination,value) in op.arguments.iter().zip(values){
  let flag=interpreter.evaluate(destination)?;interpreter.set_flag(flag,value);
 }
 *self.interpreter=interpreter;*self.fes_civil_time=None;
}
(PendingKind::Native,b"execfes")=>{
 if let Some(wait)=self.exec_fes(host,owner,origin_pc,&op,observation,budget)
  .with_context(||format!("CatSystem2 execfes at source byte {:#x}; owner native PC {:#x} remains unacknowledged",op.byte_offset,origin_pc))?{return Ok(wait);}
}
(PendingKind::Native,b"execkcs")=>{
 ensure!(op.arguments.len()==1,"CatSystem2 execkcs arguments");
 let name=self.string_variables.expand(&op.arguments[0],259)?;
 self.tasks.request_kcs_child(owner,name);
 self.tasks.begin_fes_wait(owner,&op,None,1)?;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
(PendingKind::Native,b"endfes"|b"endkcs")=>{
 ensure!(op.arguments.is_empty(),"CatSystem2 FES end-child arguments");
 let index=usize::from(op.name==b"endkcs");
 self.tasks.end_fes_child(owner,index,self.interpreter)?;
 self.tasks.begin_fes_wait(owner,&op,None,1)?;
 *budget=budget.saturating_sub(1);return Ok(Tick::Waited);
}
                                        (crate::fes_program::PendingKind::Native,b"getsndconfex"|b"setsndconfex") => {
                                            ensure!((2..=34).contains(&op.arguments.len()),"CatSystem2 FES sound configuration arguments");
                                            let mut interpreter=self.interpreter.clone();
                                            let value=interpreter.evaluate(&op.arguments[0])?;
                                            let sound=self.sound_definition.as_ref().and_then(|s|s.policy.as_ref()).context("CatSystem2 original extended sound configuration unavailable")?;
                                            if op.name==b"getsndconfex" {
                                                let result=sound.get(&op.arguments[1],&op.arguments[2..],&mut interpreter)?;
                                                interpreter.set_flag(value,result);
                                            }else{
                                                let mut proposed=sound.clone();
                                                proposed.set(&op.arguments[1],&op.arguments[2..],value,&mut interpreter)?;
                                                let requests=proposed.controls()?;
                                                for request in requests{host.submit_audio(self.client,request)?;}
                                                self.sound_definition.as_mut().unwrap().policy=Some(proposed);
                                            }
                                            *self.interpreter=interpreter;
                                        }
                                        (crate::fes_program::PendingKind::Native,b"getconf"|b"setconf") => {
                                            ensure!((2..=4).contains(&op.arguments.len()),"CatSystem2 FES configuration argument count");
                                            //766090 exposes word2 as type while native
                                            //expression reads word1 and optional words3/4.
                                            let mut interpreter=self.interpreter.clone();
                                            let value=interpreter.evaluate(&op.arguments[0])?;
                                            let first=if let Some(v)=op.arguments.get(2){interpreter.evaluate(v)?}else{0};
                                            let second=if let Some(v)=op.arguments.get(3){interpreter.evaluate(v)?}else{0};
                                            let name=&op.arguments[1];
                                            let settings=self.settings.as_ref().context("CatSystem2 configuration not loaded")?;
                                            if op.name==b"getconf" {
                                                let current=settings.configuration(name,first,second)?;
                                                interpreter.set_flag(value,current);
                                            }else{
                                                let mut proposed=settings.clone();
                                                proposed.set_configuration(name,value,first,second)?;
                                                if name.starts_with(b"vol_")||name.starts_with(b"mute_") {
                                                    let requests=proposed.audio_configuration()?;
                                                    for request in requests {host.submit_audio(self.client,request)?;}
                                                }
                                                *self.settings=Some(proposed);
                                            }
                                            *self.interpreter=interpreter;
                                        }
                                        (crate::fes_program::PendingKind::Native,b"fileexist") => {
                                            ensure!(op.arguments.len()==2,"CatSystem2 FES fileexist argument count");
                                            if self.fes_file_probe.is_none() {
                                                let flag=self.interpreter.evaluate(&op.arguments[1])?;
                                                *self.fes_file_probe=crate::fes_file_probe::Pending::begin(self.pool,host.storage(self.client)?,owner,origin_pc,op.clone(),flag)?;
                                                return Ok(Tick::Dependency);
                                            }
                                            let pending=self.fes_file_probe.as_mut().unwrap();
                                            ensure!(pending.owner==owner&&pending.pc==origin_pc&&pending.operation==op,"CatSystem2 FES file probe ownership mismatch");
                                            let Some(present)=pending.poll()? else{return Ok(Tick::Dependency);};
                                            self.interpreter.set_flag(pending.flag,u32::from(present));
                                            *self.fes_file_probe=None;
                                        }
                                        (crate::fes_program::PendingKind::Native,b"steam_isenable") => {
                                            ensure!(op.arguments.len()==1,"CatSystem2 FES steam_isenable argument count");
                                            let interpreter=&mut *self.interpreter;
                                            let index=interpreter.evaluate(&op.arguments[0])?;
                                            //785920: absent native manager is0, not an
                                            //SDK call or a forced global success flag.
                                            interpreter.set_flag(index,u32::from(self.steam_manager.enabled()));
                                        }
                                        (crate::fes_program::PendingKind::Native,b"steam_init") => {
                                            ensure!(op.arguments.is_empty(),"CatSystem2 FES steam_init argument count");
                                            self.steam_manager.initialize(host,self.client)?;
                                        }

     _=>anyhow::bail!("CatSystem2 unsupported FES operation {} ({:?}) at {} section {} row {} physical line {} byte {:#x}; owner native PC {:#x} remains unacknowledged",String::from_utf8_lossy(&op.name),op.kind,String::from_utf8_lossy(&self.tasks.fes(owner)?.script_name),String::from_utf8_lossy(&op.section),op.ordinal,op.line+1,op.byte_offset,origin_pc),
    }
                if op.kind==PendingKind::Object{self.tasks.acknowledge_fes_object(owner,&op)?;}
                else{self.tasks.acknowledge_fes_native(owner, &op)?;}
                *budget = budget.saturating_sub(1);
                if *budget > 0 {
                    continue;
                }
            }
            if self.tasks.fes(owner)?.player.as_ref().unwrap().timing.wait==Some(crate::fes_timing::Wait::Object) {
                let ids=self.tasks.fes(owner)?.player.as_ref().unwrap().object_wait.clone();
                let busy=self.tasks.fes_object_bank_mut(owner)?.is_some_and(|bank|ids.iter().any(|id|bank.objects.get(id).is_some_and(|object|matches!(&object.state,crate::native_class_state::State::Sound(sound) if sound.playing))));
                if !busy{self.tasks.fes_player_mut(owner)?.timing.wait=None;}
                return Ok(Tick::Waited);
            }
            if self.tasks.fes(owner)?.player.as_ref().unwrap().timing.wait==Some(crate::fes_timing::Wait::Objects) {
                let task=self.tasks.fes(owner)?;
                //The admitted profile has only its implicit BASE object;
                //DEFINE/OBJECT publication remains separately gated. This
                //root is genuinely engine-owned, never an empty VM counter.
                let root=task.object_root.context("CatSystem2 object wait lost its shared resource root")?;
                let status:crysirror_scene::layer::MotionStatus=serde_json::from_slice(
                    &host.query_engine(self.client,crysirror_core::service::EngineQuery::LayerMotion(root))?)?;
                let player=task.player.as_ref().unwrap();
                let skip=player.skip_enabled&&(player.force_skip||player.click_down);
                let transition=player.transition_until;
                let transition_running=transition.is_some_and(|until|player.timing.now<until);
                if transition.is_some()&&(!transition_running||skip){
                    if transition_running{host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"end_transition"}]}))?)?;}
                    self.tasks.fes_player_mut(owner)?.transition_until=None;
                }
                if transition_running&&!skip{return Ok(Tick::Waited);}
                if status.complete {
                    //7563A0/-1 sets native wait0 but STILL yields this call.
                    //A large clock jump cannot collapse the extra owner tick.
                    self.tasks.clear_fes_object_wait(owner)?;
                }else if skip {
                    #[cfg(feature="runtime")]
                    if let Some(bank)=self.tasks.fes_object_bank_mut(owner)? {
                        if bank.active_subtree(bank.root)?{bank.skip_motions()?;return Ok(Tick::Waited);}
                    }
                    //748610->789870->78A5C0 sets the original motion
                    //program's global40 flag. It does not authorize generic
                    //track cancellation or an invented endpoint completion.
                    anyhow::bail!("CatSystem2 object skip requires original motion-program flag execution; shared tracks remain active");
                }
                return Ok(Tick::Waited);
            }
            return Ok(
                if self
                    .tasks
                    .fes(owner)?
                    .player
                    .as_ref()
                    .unwrap()
                    .timing
                    .wait
                    .is_some()
                {
                    Tick::Waited
                } else {
                    Tick::Budget
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{path::VfsPath, service::FileSystem, state::Command};
    use std::sync::Arc;
    #[test]
    fn all_object_wait_observes_shared_motion_and_preserves_native_completion_yield() -> Result<()> {
        use crysirror_core::service::EngineQuery;
        //Each case uses actual shared Layer resources, including the case
        //where a native force-skip must fail pending original motion VM support.
        for forced in [false,true] {
            let root=tempfile::tempdir()?;
            let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({
                "schema":2,"name":"authored FES engine object","content_revision":"1","assets":"game","canvas":[4,4],
                "clients":[{"name":"native","adapter":"crysirror-script-v1","game":"unused"}]
            }))?;
            let mut runtime=crysirror_runtime::Runtime::with_asset_root(project,root.path(),&root.path().join("state"),Default::default())?;
            let mut tasks=NativeTaskPolicy::bootstrap()?;
            let owner=tasks.create_fes(3,b"motion".to_vec())?;
            let source=if forced { b"#START\n$0+=1\nkeyskip force 1\nwait\n$0+=10\nwait 1".as_slice() }
                else { b"#START\n$0+=1\nwait\n$0+=10\nwait 1".as_slice() };
            let program=Arc::new(crate::fes_program::Program::parse(source,&||Ok(()))?);
            let mut interpreter=Interpreter::new(8)?;
            tasks.load_fes_player(owner,"fes.int/motion.fes",program,&mut interpreter,&Default::default())?;
            tasks.publish_fes_object_root(owner,&mut runtime.host,ClientId(1))?;
            tasks.initialize_fes_tasks([4,4])?;
            let layer=tasks.fes(owner)?.object_root.context("published FES BASE root missing")?;
            let owned:Option<crysirror_scene::layer::LayerSpec>=serde_json::from_slice(&runtime.host.query_engine(ClientId(1),EngineQuery::Layer(layer))?)?;
            assert!(owned.is_some());
            assert!(runtime.host.query_engine(ClientId(2),EngineQuery::LayerMotion(layer)).is_err());
            runtime.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&serde_json::json!({"layers":[{
                "op":"begin_motion","id":layer,"track":{"property":"opacity","to":0,"duration":6,"external":true}
            }]}))?)?;
            let mut settings=Some(NativeSettings::constructor());let mut sound=None;
            let mut input=crate::native_input::InputState::default();let mut steam=Default::default();
            let mut probe=None;let mut civil=None;let pool=crysirror_tasks::TaskPool::new("authored-shared-fes-motion",1,1)?;
            let mut children=crate::fes_children::Children::default();
            let mut string_variables=crate::native_values::StringVariables::default();
            let mut driver=Services{pointer_glide:None,string_variables:&mut string_variables,client:ClientId(1),tasks:&mut tasks,interpreter:&mut interpreter,
                settings:&mut settings,sound_definition:&mut sound,input_state:&mut input,
                steam_manager:&mut steam,fes_file_probe:&mut probe,fes_civil_time:&mut civil,fes_children:&mut children,handles:None,saves:&Default::default(),pool:&pool};
            assert!(matches!(driver.run(&mut runtime.host,owner,0x1943,observation(1,0),&mut 100)?,Tick::Waited));
            assert_eq!(driver.interpreter.flag(0),1);
            assert_eq!(driver.tasks.fes(owner)?.player.as_ref().unwrap().timing.wait,Some(crate::fes_timing::Wait::Objects));
            //Even a huge simulation clock jump does not clear a busy object.
            if forced {
                let error=driver.run(&mut runtime.host,owner,0x1943,observation(2,10000),&mut 100).err().context("unsupported original skip was acknowledged")?;
                assert!(error.to_string().contains("original motion-program flag execution"));
                let status:crysirror_scene::layer::MotionStatus=serde_json::from_slice(&runtime.host.query_engine(ClientId(1),EngineQuery::LayerMotion(layer))?)?;
                assert!(!status.complete && status.tracks.len()==1);
                assert_eq!(driver.interpreter.flag(0),1);
                assert_eq!(driver.tasks.fes(owner)?.player.as_ref().unwrap().timing.wait,Some(crate::fes_timing::Wait::Objects));
                continue;
            }
            assert!(matches!(driver.run(&mut runtime.host,owner,0x1943,observation(2,10000),&mut 100)?,Tick::Waited));
            assert_eq!(driver.interpreter.flag(0),1);
            runtime.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&serde_json::json!({"layers":[{
                "op":"set_motion_elapsed","id":layer,"property":"opacity","elapsed":6
            }]}))?)?;
            //Native-1 detects completion but still returns from this tick.
            assert!(matches!(driver.run(&mut runtime.host,owner,0x1943,observation(3,10016),&mut 100)?,Tick::Waited));
            assert_eq!(driver.interpreter.flag(0),1);
            assert_eq!(driver.tasks.fes(owner)?.player.as_ref().unwrap().timing.wait,None);
            assert!(matches!(driver.run(&mut runtime.host,owner,0x1943,observation(4,10032),&mut 100)?,Tick::Waited));
            assert_eq!(driver.interpreter.flag(0),11);
        }
        Ok(())
    }
    #[test]
    fn civil_clock_is_a_single_shared_dependency_and_does_not_replay_source_or_use_simulation_time() {
        use crysirror_core::service::{CivilTime, CivilTimeQuery, CivilTimeState};
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Query(Arc<AtomicUsize>);
        impl CivilTimeQuery for Query {
            fn client(&self) -> ClientId { ClientId(1) }
            fn poll(&mut self) -> crysirror_core::Result<CivilTimeState> {
                Ok(if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                    CivilTimeState::Pending
                } else { CivilTimeState::Ready(CivilTime { client: ClientId(1), unix_ms: 0,
                    utc_offset_seconds: 32400, year: 2026, month: 11, day: 1,
                    hour: 5, minute: 50, second: 7, millisecond: 10 }) })
            }
        }
        struct ClockHost { begins: usize, polls: Arc<AtomicUsize> }
        impl FileSystem for ClockHost {
            fn read(&mut self, c: ClientId, p: &VfsPath) -> crysirror_core::Result<Vec<u8>> { Host.read(c,p) }
            fn write_atomic(&mut self, c: ClientId, p: &VfsPath, b: &[u8]) -> crysirror_core::Result<()> { Host.write_atomic(c,p,b) }
        }
        impl HostServices for ClockHost {
            fn now_ms(&self) -> u64 { 0 }
            fn submit(&mut self, c: ClientId, commands: &[Command]) -> crysirror_core::Result<()> { Host.submit(c,commands) }
            fn begin_civil_time(&mut self, c: ClientId) -> crysirror_core::Result<Box<dyn CivilTimeQuery>> {
                assert_eq!(c,ClientId(1)); self.begins+=1; Ok(Box::new(Query(self.polls.clone())))
            }
        }
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let owner=tasks.create_fes(3,b"clock".to_vec()).unwrap();
        let mut interpreter=Interpreter::new(16).unwrap();
        let program=Arc::new(crate::fes_program::Program::parse(
            b"#START\n$0+=1\ngettime 1 2 3 4 5 6 7\ngetweek 8 2000 2 29\nwait 2",&||Ok(())).unwrap());
        tasks.load_fes_player(owner,"fes.int/clock.fes",program,&mut interpreter,&Default::default()).unwrap();
        tasks.initialize_fes_tasks([1280,720]).unwrap();
        let mut settings=Some(NativeSettings::constructor());let mut sound=None;
        let mut input=crate::native_input::InputState::default();let mut steam=Default::default();
        let mut probe=None;let mut civil=None;
        let pool=crysirror_tasks::TaskPool::new("authored-fes-civil",1,1).unwrap();
        let mut children=crate::fes_children::Children::default();
            let mut string_variables=crate::native_values::StringVariables::default();
            let mut driver=Services{pointer_glide:None,string_variables:&mut string_variables,client:ClientId(1),tasks:&mut tasks,interpreter:&mut interpreter,
            settings:&mut settings,sound_definition:&mut sound,input_state:&mut input,
            steam_manager:&mut steam,fes_file_probe:&mut probe,fes_civil_time:&mut civil,fes_children:&mut children,handles:None,saves:&Default::default(),pool:&pool};
        let mut host=ClockHost{begins:0,polls:Arc::new(AtomicUsize::new(0))};
        for _ in 0..2 {
            assert!(matches!(driver.run(&mut host,owner,0x1943,observation(1,0),&mut 100).unwrap(),Tick::Dependency));
            assert_eq!(driver.interpreter.flag(0),1);
            assert_eq!(driver.interpreter.flag(1),0);
            assert_eq!(driver.tasks.fes(owner).unwrap().player.as_ref().unwrap().pending.as_ref().unwrap().name,b"gettime");
        }
        assert!(matches!(driver.run(&mut host,owner,0x1943,observation(1,0),&mut 100).unwrap(),Tick::Waited));
        assert_eq!((0..=8).map(|i|driver.interpreter.flag(i)).collect::<Vec<_>>(),[1,2026,11,1,0,5,50,7,2]);
        assert_eq!(host.begins,1);assert_eq!(host.polls.load(Ordering::SeqCst),2);
        assert!(matches!(driver.run(&mut host,owner,0x1943,observation(1,0),&mut 100).unwrap(),Tick::Waited));
        assert_eq!(host.begins,1);
    }
    #[test]
    fn exit_publishes_task_status_without_halting_and_result_is_a_separate_player_slot() {
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let owner = tasks.create_fes(3, b"main".to_vec()).unwrap();
        let mut interpreter = Interpreter::new(8).unwrap();
        let source = Arc::new(
            crate::fes_program::Program::parse(
                b"#START\nresult 5\n$0=1\nexit 7\n$0=2\nwait 3",
                &|| Ok(()),
            )
            .unwrap(),
        );
        tasks
            .load_fes_player(owner, "fes.int/authored.fes", source, &mut interpreter,&Default::default())
            .unwrap();
        tasks.initialize_fes_tasks([1280, 720]).unwrap();
        let mut settings = Some(NativeSettings::constructor());
        let mut sound = None;
        let mut input = crate::native_input::InputState::default();
        let mut steam = crate::native_platform::SteamManager::default();
        let mut probe = None;
        let mut civil = None;
        let pool = crysirror_tasks::TaskPool::new("authored-fes-result", 1, 1).unwrap();
        let mut children=crate::fes_children::Children::default();
            let mut string_variables=crate::native_values::StringVariables::default();
        let mut driver = Services {pointer_glide:None,
            string_variables:&mut string_variables,
            client: ClientId(1),
            tasks: &mut tasks,
            interpreter: &mut interpreter,
            settings: &mut settings,
            sound_definition: &mut sound,
            input_state: &mut input,
            steam_manager: &mut steam,
            fes_file_probe: &mut probe,
            fes_civil_time: &mut civil,
            fes_children:&mut children,handles:None,saves:&Default::default(),
            pool: &pool,
        };
        let mut budget = 100;
        assert!(matches!(
            driver
                .run(&mut Host, owner, 0x1943, observation(1, 0), &mut budget)
                .unwrap(),
            Tick::Waited
        ));
        assert_eq!(driver.interpreter.flag(0), 2);
        assert_eq!(driver.tasks.task_result(owner), 7);
        assert_eq!(driver.tasks.task_result(3), u32::MAX);
        assert_eq!(driver.tasks.task_result(65535), u32::MAX);
        let task = driver.tasks.fes(owner).unwrap();
        assert!(task.running);
        assert_eq!(task.exit_code, -1);
        assert_eq!(task.player.as_ref().unwrap().result_code, 5);
        driver.tasks.set_task_result(3, 42).unwrap();
        assert_eq!(driver.tasks.task_result(3), 42);
        assert_eq!(driver.tasks.task_result(owner), 7);
    }
    struct Host;
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::NotFound)
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
    }
    impl HostServices for Host {
        fn now_ms(&self) -> u64 {
            0
        }
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported("authored command path"))
        }
    }
    fn observation(revision: u64, time_ms: u64) -> InputObservation {
        InputObservation {
            client: ClientId(1),
            revision,
            time_ms,
            buttons_down: 0,
            buttons_pressed: 0,
            pointer: None,
            pointer_inside: None,
            focused: None,
        }
    }
    #[test]
    fn forced_and_ordinary_tick_share_real_waits_and_do_not_repeat_original_commands() {
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let owner = tasks.create_fes(3, b"main".to_vec()).unwrap();
        let mut interpreter = Interpreter::new(8).unwrap();
        let source=Arc::new(crate::fes_program::Program::parse(b"#START\n$0+=1\nendfes\n$0+=10\nendkcs\n$0+=100\nwait 2\nsteam_isenable 2\n$0+=1000",&||Ok(())).unwrap());
        tasks
            .load_fes_player(owner, "fes.int/authored.fes", source, &mut interpreter,&Default::default())
            .unwrap();
        tasks.initialize_fes_tasks([1280, 720]).unwrap();
        let mut settings = Some(NativeSettings::constructor());
        let mut sound = None;
        let mut input = crate::native_input::InputState::default();
        let mut steam = crate::native_platform::SteamManager::default();
        let mut probe = None;
        let mut civil = None;
        let pool = crysirror_tasks::TaskPool::new("authored-fes", 1, 1).unwrap();
        let mut children=crate::fes_children::Children::default();
            let mut string_variables=crate::native_values::StringVariables::default();
        let mut driver = Services {pointer_glide:None,
            string_variables:&mut string_variables,
            client: ClientId(1),
            tasks: &mut tasks,
            interpreter: &mut interpreter,
            settings: &mut settings,
            sound_definition: &mut sound,
            input_state: &mut input,
            steam_manager: &mut steam,
            fes_file_probe: &mut probe,
            fes_civil_time: &mut civil,
            fes_children:&mut children,handles:None,saves:&Default::default(),
            pool: &pool,
        };
        for (revision, time, expected) in [
            (1, 0, 1),
            (1, 0, 1),
            (2, 17, 11),
            (3, 34, 111),
            (4, 50, 111),
            (5, 67, 1111),
        ] {
            let mut budget = 100;
            assert!(matches!(
                driver
                    .run(
                        &mut Host,
                        owner,
                        0x1943,
                        observation(revision, time),
                        &mut budget
                    )
                    .unwrap(),
                Tick::Waited
            ));
            assert_eq!(driver.interpreter.flag(0), expected);
        }
        assert_eq!(driver.interpreter.flag(2), 0);
        assert_eq!(driver.steam_manager.diagnostics()["created"], false);
    }
    #[test]
    fn unsupported_fes_operation_keeps_exact_source_after_repeated_owner_polls() {
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let owner = tasks.create_fes(3, b"main".to_vec()).unwrap();
        let mut interpreter = Interpreter::new(8).unwrap();
        let source = Arc::new(
            crate::fes_program::Program::parse(b"#START\n$0=9\nexecfes child", &|| Ok(())).unwrap(),
        );
        tasks
            .load_fes_player(owner, "fes.int/authored.fes", source, &mut interpreter,&Default::default())
            .unwrap();
        tasks.initialize_fes_tasks([1280, 720]).unwrap();
        let mut settings = Some(NativeSettings::constructor());
        let mut sound = None;
        let mut input = crate::native_input::InputState::default();
        let mut steam = crate::native_platform::SteamManager::default();
        let mut probe = None;
        let mut civil = None;
        let pool = crysirror_tasks::TaskPool::new("authored-fes-retain", 1, 1).unwrap();
        let mut children=crate::fes_children::Children::default();
            let mut string_variables=crate::native_values::StringVariables::default();
        let mut driver = Services {pointer_glide:None,
            string_variables:&mut string_variables,
            client: ClientId(1),
            tasks: &mut tasks,
            interpreter: &mut interpreter,
            settings: &mut settings,
            sound_definition: &mut sound,
            input_state: &mut input,
            steam_manager: &mut steam,
            fes_file_probe: &mut probe,
            fes_civil_time: &mut civil,
            fes_children:&mut children,handles:None,saves:&Default::default(),
            pool: &pool,
        };
        let mut budget = 100;
        let first = driver
            .run(&mut Host, owner, 0x1943, observation(1, 0), &mut budget)
            .err()
            .unwrap()
            .to_string();
        let pending = driver
            .tasks
            .fes(owner)
            .unwrap()
            .player
            .as_ref()
            .unwrap()
            .pending
            .clone();
        let mut budget = 100;
        let second = driver
            .run(&mut Host, owner, 0x1943, observation(1, 0), &mut budget)
            .err()
            .unwrap()
            .to_string();
        assert_eq!(first, second);
        assert!(first.contains("execfes") && first.contains("0x1943"));
        assert_eq!(
            pending,
            driver
                .tasks
                .fes(owner)
                .unwrap()
                .player
                .as_ref()
                .unwrap()
                .pending
        );
        assert_eq!(driver.interpreter.flag(0), 9);
    }
}
