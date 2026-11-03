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
    pub saves:&'a std::collections::BTreeMap<u32,crate::native_save::Container>,
    pub pool: &'a crysirror_tasks::TaskPool,
}
impl Services<'_> {
    pub fn run(&mut self,host:&mut dyn HostServices,owner:u32,origin_pc:u32,
        observation:InputObservation,budget:&mut u32)->Result<Tick> {
        #[cfg(feature="runtime")]
        if self.fes_children.frames.in_flight(owner){
            if !self.fes_children.frames.advance(host,self.client,self.pool,self.tasks,self.interpreter,self.string_variables,self.sound_definition.as_ref().and_then(|d|d.policy.as_ref()),&mut self.fes_children.motions,budget,owner,origin_pc,observation)?{return Ok(Tick::Dependency);}
            self.tasks.observe_fes_child_result(owner,self.interpreter)?;
            return Ok(Tick::Waited);
        }
        let tick=self.run_frame(host,owner,origin_pc,observation,budget)?;
        if matches!(tick,Tick::Waited){
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
        {
            use crysirror_core::service::buttons;
            let held=|bit:u64|observation.buttons_down&bit!=0||observation.buttons_pressed&bit!=0;
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
 if method==b"blend" {
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
  if crate::fes_object_natives::is_builtin(method){
   // Not connected yet (settbl, arc, page, attach, history-text queries): the object keeps its state.
   host.append_log(self.client,&format!("CatSystem2 FES object method {} has no capability yet",String::from_utf8_lossy(method)))?;
   self.tasks.acknowledge_fes_object(owner,&op)?;
   *budget=budget.saturating_sub(1);
   if *budget>0{continue;}
   return Ok(Tick::Budget);
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
// 77AB10 leaves the CG register absent when no table loads; 77ADF0/77AF20 then
// store 0 in the destination, which is what every query below reports.
(PendingKind::Native,b"cgreg_init")=>{}
// Window chrome and developer hooks with no effect on this host's scene.
(PendingKind::Native,b"deletebgbutton"|b"force_redraw"|b"wndmenu"|b"decode"|b"resetactive"|b"sendbtn")=>{}
// 77BEE0: `send target a b [wait]` posts a message the owning KCS reads from native71.
(PendingKind::Native,b"send")=>{
 let mut interpreter=self.interpreter.clone();
 let mut word=|i:usize|->Result<u32>{op.arguments.get(i).map_or(Ok(0),|source|interpreter.evaluate(source))};
 let (target,a,b)=(word(0)?,word(1)?,word(2)?);
 self.tasks.post_script_event(owner,[owner,target,a,b]);
 *self.interpreter=interpreter;
}
(PendingKind::Native,b"cgreg_t"|b"cgregnum")=>{
 let mut interpreter=self.interpreter.clone();
 if let Some(source)=op.arguments.first(){let destination=interpreter.evaluate(source)?;interpreter.set_flag(destination,0);}
 *self.interpreter=interpreter;
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
// 77B640, 77B820, 7814A0: pointer warps the host does not perform for a script.
(PendingKind::Native,b"automove"|b"restoremove"|b"automoveex")=>{}
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
// 786360, 786840: E-mote quality options are owned by the shared puppet.
(PendingKind::Native,b"m2_getconf")=>{
 let mut vm=self.interpreter.clone();
 if let Some(source)=op.arguments.first(){let flag=vm.evaluate(source)?;vm.set_flag(flag,1);}
 *self.interpreter=vm;
}
(PendingKind::Native,b"m2_setconf"|b"input_text"|b"messagebox"|b"config_open")=>{
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
            let mut driver=Services{string_variables:&mut string_variables,client:ClientId(1),tasks:&mut tasks,interpreter:&mut interpreter,
                settings:&mut settings,sound_definition:&mut sound,input_state:&mut input,
                steam_manager:&mut steam,fes_file_probe:&mut probe,fes_civil_time:&mut civil,fes_children:&mut children,saves:&Default::default(),pool:&pool};
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
            let mut driver=Services{string_variables:&mut string_variables,client:ClientId(1),tasks:&mut tasks,interpreter:&mut interpreter,
            settings:&mut settings,sound_definition:&mut sound,input_state:&mut input,
            steam_manager:&mut steam,fes_file_probe:&mut probe,fes_civil_time:&mut civil,fes_children:&mut children,saves:&Default::default(),pool:&pool};
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
        let mut driver = Services {
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
            fes_children:&mut children,saves:&Default::default(),
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
        let mut driver = Services {
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
            fes_children:&mut children,saves:&Default::default(),
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
        let mut driver = Services {
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
            fes_children:&mut children,saves:&Default::default(),
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
