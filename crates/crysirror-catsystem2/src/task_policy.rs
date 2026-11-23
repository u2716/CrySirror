// SPDX-License-Identifier: MPL-2.0
//! Original native task-number policy. Reserved roots denote CrySirror-owned
//! input/audio services, not private implementations of either subsystem.
use anyhow::{ensure, Context, Result};
use crysirror_core::image_processing::ImageResource;
mod retirement;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Debug)]
pub struct NativeTaskPolicy {
    slots: BTreeSet<u32>,
    current: u32,
    kcs_requests: Vec<(u32, Vec<u8>)>,
    kcs_retired: Vec<u32>,
    script_events: std::collections::VecDeque<(u32,[u32;4])>,
    planes: BTreeMap<u32, NativePlane>,
    buttons: BTreeMap<u32, NativeButton>,
    image_lists: BTreeMap<u32, NativeImageList>,
    pub(crate) logs: crate::native_log::Logs,
    image_retirements:Vec<String>,
    fes_tasks: BTreeMap<u32, crate::fes_task::FesTask>,
    results:crate::native_task_result::Results,
    opaque: BTreeMap<u32, OpaqueTask>,
}
/// Native task of a class modeled by a `native_classes` family.
#[derive(Clone, Debug)]
pub(crate) struct OpaqueTask {
    pub class: u32,
    pub parent: u32,
    pub name: Vec<u8>,
}
/// Native task ancestry is distinct from render graph ancestry. An empty plane
/// owns an engine group; geometry/paint/transforms belong to that shared Layer.
#[derive(Clone, Debug)]
pub struct NativePlane {
    pub id: u32,
    pub parent: u32,
    pub name: Vec<u8>,
    pub layer: u32,
    pub graph_parent: Option<u32>,
    pub native_rect: [i32; 4],
    pub payload: Option<u32>,
    pub image: Option<NativePlaneImage>,
    /// Original uniform triangle vertexARGB and682370/6840C0 override fields.
    pub solid_vertex_argb: Option<u32>,
    pub diffuse_enable: u32,
    pub diffuse_argb: u32,
    /// Native this[35], distinct from the last backend visibility written by
    /// parent message48. A direct child show can override a prior parent hide.
    pub own_enabled: bool,
    pub pose: PlanePose,
    /// 67C450 selects the source mesh for nonzero selector, and the divided
    /// screen mesh for zero. Only atlas-backed quads have both objects.
    pub(crate) source_pose: Option<PlanePose>,
    /// Original this[170] (5DF070 sets 1; message 84 replaces it). The render
    /// manager only builds an object for type 1 (676CB0 then `type==1` in
    /// 675240/675710/675780/674E50/676350); any other type leaves the Plane empty.
    pub render_type: u32,
    /// this[47]/this[48]: message 89 bound the Plane's own click area (5E01F0).
    pub own_button: bool,
}
/// 67DE50 base, 67D780 position, 67DE90 scale and 67D7C0 colour words of one Plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanePose {
    pub position: [f32; 3],
    pub base: [f32; 3],
    pub scale: [f32; 3],
    pub rotation: f32,
    pub alpha: u8,
    pub size: [f32; 2],
    pub(crate) matrix: Option<[f32; 16]>,
}
impl Default for PlanePose {
    fn default() -> Self {
        Self { position: [0.; 3], base: [0.; 3], scale: [1.; 3], rotation: 0., alpha: 255, size: [0.; 2], matrix: None }
    }
}
/// Original5DE590 type6 controller. The source creates no draw geometry and
/// no input registration until message89 binds a native input task/rectangle.
#[derive(Clone, Debug)]
pub struct NativeButton {
    pub id: u32,
    pub parent: u32,
    pub name: Vec<u8>,
    pub binding_target: Option<u32>,
    pub rect: [i32; 4],
    pub own_enabled: bool,
    pub relative: bool,
    pub result: u32,
    pub event_mode: u32,
    pub full_canvas_rect: bool,
}
#[derive(Clone, Debug)]
pub struct NativePlaneImage {
    pub resource: Arc<dyn ImageResource>,
    pub path: String,
}
#[derive(Clone, Debug)]
pub struct NativeImagePool {
    pub resource: Arc<dyn ImageResource>,
    pub path: String,
    pub size: [u32; 2],
    pub page_size: [u32; 2],
    pub page_count: u32,
    pub dirty_bounds: Option<[i32; 4]>,
    pub upload_position: Option<[i32; 2]>,
    pub native_entry_keys: BTreeSet<u32>,
}
///5F3790 type15.68E6F0 is an empty bitmap wrapper (68E970/68BB00),
/// not a filesystem scanner. No pixels/manager exist before message279.
#[derive(Clone, Debug)]
pub struct NativeImageList {
    pub id: u32,
    pub parent: u32,
    pub name: Vec<u8>,
    pub manager: Option<NativeImagePool>,
    pub field_120: u32,
    pub field_124: u32,
    pub field_132: u32,
    pub upload_position: Option<[i32; 2]>,
    pub base_plane: Option<u32>,
    pub field_140: u32,
    pub bitmap: Option<Arc<dyn ImageResource>>,
    pub bitmap_field4: u32,
    pub bitmap_field8: u32,
    pub bitmap_field532: u32,
    pub bitmap_name: Vec<u16>,
    pub native_entry_keys: BTreeSet<u32>,
    pub(crate) publication:Option<crate::native_image_publication::ImageList>,
}
impl NativePlane {
    pub fn layer_spec(&self) -> Result<crysirror_scene::layer::LayerSpec> {
        // A logical group has no native pixels. The shared Layer API requires
        // a positive extent; its1x1 transparent extent never substitutes the
        // original empty rectangle in native size/rectangle queries.
        Ok(serde_json::from_value(
            serde_json::json!({"id":self.layer,"parent":null,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":true,"clip":null}),
        )?)
    }
}
impl NativeTaskPolicy {
    pub fn bootstrap() -> Result<Self> {
        // Reviewed ABI: 5DBE40 constructs BASE=0; 5DD3C0 creates INPUT then
        // SOUND; 5DD580 allocates the first unused number, searching root+tree.
        let mut policy = Self {
            slots: BTreeSet::new(),
            current: 0,
            kcs_requests: Vec::new(),
            kcs_retired: Vec::new(),
            script_events: Default::default(),
            planes: BTreeMap::new(),
            buttons: BTreeMap::new(),
            image_lists: BTreeMap::new(),
            logs: Default::default(),
            image_retirements:Vec::new(),
            fes_tasks: BTreeMap::new(),
            results:Default::default(),
            opaque: BTreeMap::new(),
        };
        let root = policy.allocate()?;
        debug_assert_eq!(root, 0);
        policy.allocate()?;
        policy.allocate()?;
        policy.current = policy.allocate()?;
        Ok(policy)
    }
    fn allocate(&mut self) -> Result<u32> {
        let id = (0..65536)
            .find(|id| !self.slots.contains(id))
            .context("CatSystem2 native task number budget")?;
        self.slots.insert(id);
        self.results.construct(id).map_err(anyhow::Error::msg)?;
        Ok(id)
    }
    pub fn current(&self) -> u32 {
        self.current
    }
    /// The script task whose natives run next: 5E84E0 ticks each KCS task itself.
    pub(crate) fn set_current(&mut self, id: u32) {
        self.current = id;
    }
    /// 77A9B0's queued message 4: `execkcs` runs at the owner's next idle point.
    pub(crate) fn request_kcs_child(&mut self, owner: u32, name: Vec<u8>) {
        self.kcs_requests.push((owner, name));
    }
    pub(crate) fn take_kcs_request(&mut self) -> Option<(u32, Vec<u8>)> {
        (!self.kcs_requests.is_empty()).then(|| self.kcs_requests.remove(0))
    }
    /// 629850 case 4: a "KCS Script" task in the owner's second child slot.
    pub(crate) fn create_kcs_child(&mut self, owner: u32) -> Result<u32> {
        self.fes(owner)?;
        let id = self.allocate()?;
        self.fes_tasks.get_mut(&owner).unwrap().child_tasks[1] = Some(id);
        Ok(id)
    }
    pub(crate) fn destroy_task_slot(&mut self, id: u32) -> Result<()> {
        self.slots.remove(&id);
        self.results.destroy(id).map_err(anyhow::Error::msg)
    }
    pub(crate) fn take_retired_kcs(&mut self) -> Vec<u32> {
        core::mem::take(&mut self.kcs_retired)
    }
    pub(crate) fn fes_player_mut(&mut self,id:u32)->Result<&mut crate::fes_program::Machine>{
        self.fes_tasks.get_mut(&id).and_then(|task|task.player.as_mut()).context("CatSystem2 FES player missing")
    }
    pub(crate) fn set_fes_paused(&mut self,id:u32,paused:bool){
        if let Some(task)=self.fes_tasks.get_mut(&id){task.paused=paused;}
    }
    pub(crate) fn task_result(&self,id:u32)->u32{self.results.query(id)}
    pub(crate) fn set_task_result(&mut self,id:u32,value:u32)->Result<()>{self.results.set(id,value).map_err(anyhow::Error::msg)}
    pub(crate) fn set_fes_player_result(&mut self,id:u32,value:u32)->Result<()>{
        self.fes_tasks.get_mut(&id).context("CatSystem2 FES result target missing")?.player.as_mut().context("CatSystem2 FES player missing")?.result_code=value;Ok(())
    }
    pub(crate) fn set_fes_section(&mut self,id:u32,section:&[u8])->Result<bool>{
        //Generic message dispatch leaves the caller's initialized0 unchanged
        //when no receiver handles335. No task or resource is fabricated.
        let Some(task)=self.fes_tasks.get_mut(&id)else{return Ok(false);};
        task.player.as_mut().context("CatSystem2 FES section target has no loaded player")?.set_section(section)
    }
    pub(crate) fn set_fes_save_object(&mut self,id:u32,value:u32,interpreter:&mut crate::native_interpreter::Interpreter)->Result<()> {
        let Some(task)=self.fes_tasks.get(&id)else{return Ok(());};
        ensure!(task.player_loaded&&task.player.is_some()&&interpreter.select_player_bank(id),"CatSystem2 FES named-value target is not loaded");
        //62BED0/336->757620 first selects the real player's local bank.
        interpreter.set_named_value(b"_SAVEOBJ_",0,value)
    }
    pub(crate) fn running_fes_tasks(&self)->Result<Vec<u32>> {self.running_fes_tasks_under(&[])}
    /// The KCS script whose task owns this FES (5DC510 walks one task tree).
    pub(crate) fn fes_script_owner(&self,mut id:u32)->u32{
        while let Some(task)=self.fes_tasks.get(&id){id=task.parent;}
        id
    }
    /// `scripts` are the running KCS child tasks: their FES tasks tick in the same pass.
    pub(crate) fn running_fes_tasks_under(&self,scripts:&[u32])->Result<Vec<u32>> {
        fn visit(policy:&NativeTaskPolicy,parent:u32,depth:usize,out:&mut Vec<u32>)->Result<()> {
            ensure!(depth<64,"CatSystem2 FES task ancestry depth");
            //5DC510 visits each native child before its parent script, in
            //the original task-number order, not a flat depth sort.
            for task in policy.fes_tasks.values().filter(|t|t.parent==parent){
                visit(policy,task.id,depth+1,out)?;
                if task.deletion_pending||(task.running&&task.player_loaded&&!task.first_tick&&!task.paused){out.push(task.id);}
            }
            Ok(())
        }
        let mut out=Vec::new();visit(self,self.current,0,&mut out)?;
        for script in scripts{visit(self,*script,0,&mut out)?;}
        Ok(out)
    }
    pub(crate) fn empty_fes_hover(&mut self,receiver:u32,input:&crate::native_input::InputState,interpreter:&mut crate::native_interpreter::Interpreter)->Result<bool>{
        let target=self.button(receiver)?.binding_target;
        let Some(task)=target.and_then(|id|self.fes_tasks.get_mut(&id))else{return Ok(false);};
        if task.paused||task.deletion_pending{return Ok(true);}
        if task.low_button!=Some(receiver){return Ok(false);}
        //62BED0/64 ->62D260 has no matching object-button for this fallback
        //receiver in the admitted no-DEFINE/OBJECT program profile. Both
        //enter and leave clear selection;62D4A0 has an empty binding tree.
        ensure!(task.auxiliary_records.is_empty(),"CatSystem2 object hover requires actual shared object bindings");
        //62D3F0 performs NO INPUT query before a player exists. Its empty
        //binding tree still clears selection and returns the unchanged0.
        if task.player_loaded {
            ensure!(interpreter.select_player_bank(task.id),"CatSystem2 FES hover bank unavailable");
            let [x,y]=input.fes_position();interpreter.set_named_value(b"_MOUSE_X_",0,x as u32)?;interpreter.set_named_value(b"_MOUSE_Y_",0,y as u32)?;
        }
        task.next_object.fill(0);task.selected_object=-1;Ok(true)
    }
    pub fn create_fes(&mut self,parent:u32,name:Vec<u8>)->Result<u32> {
        ensure!(name.len()<32&&!name.contains(&0),"CatSystem2 native task name extent");
        //62DAA0 attach resolves parent177 immediately, before331/player load.
        //Nested FES inherits the actual raw-string owner, not its parent's
        //player-local bank or a fabricated new string interpreter.
        let (parameters,string_owner)=if parent==self.current{(Vec::new(),Some(self.current))}
        else{let parent=self.fes(parent)?;(parent.parameters.clone(),parent.string_bank_owner)};
        let id=self.allocate()?;
        //Main's712 startsNUL; nested26 copies its actual parent's448 buffer.
        //627E50's earlier125 cannot leave the unattached empty task, so the
        //cursor remains unknown until the real input owner supplies it.
        let mut task=crate::fes_task::FesTask::new(id,parent,name,parameters)?;
        task.string_bank_owner=string_owner;
        self.fes_tasks.insert(id,task);
        Ok(id)
    }
    pub(crate) fn fes(&self,id:u32)->Result<&crate::fes_task::FesTask> {
        self.fes_tasks.get(&id).context("CatSystem2 target is not a native FES task")
    }
    pub(crate) fn save_fes_cursor(&mut self,id:u32,point:[i32;2])->Result<()> {
        self.fes_tasks.get_mut(&id).context("CatSystem2 FES cursor owner missing")?.saved_cursor=Some(point);Ok(())
    }
    pub(crate) fn dispatch_fes_directions(&mut self,id:u32,revision:u64,pressed:u64)->Result<()> {
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 FES direction owner missing")?;
        if task.direction_revision==Some(revision){return Ok(());}
        task.direction_revision=Some(revision);
        if let Some(bank)=task.object_bank.as_mut(){
            use crysirror_core::service::buttons;
            for (direction,button) in [buttons::UP,buttons::DOWN,buttons::LEFT,buttons::RIGHT].into_iter().enumerate(){
                if pressed&button!=0{
                    let events=crate::fes_button::attached_direction(bank,direction);
                    for event in &events {task.click_values.push((&b"_SCRL_P_"[..],event.object));}
                    task.button_queue.extend(events);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn fes_string_table_mut(&mut self,id:u32)->Result<&mut crate::native_values::StringVariables> {
        Ok(&mut self.fes_tasks.get_mut(&id).context("CatSystem2 FES string-table owner missing")?.player.as_mut()
            .context("CatSystem2 FES string-table player missing")?.string_table)
    }
    pub(crate) fn forced_fes_phase(&mut self,owner:u32)->Result<Vec<u32>> {
        let root=self.fes_tasks.get_mut(&owner).context("CatSystem2 forced phase target is not FES")?;
        ensure!(root.player_loaded,"CatSystem2 forced FES requires an attached player");
        //62BED0/333 stores97 BEFORE its virtual base/child phase.
        root.running=true;
        fn visit(policy:&NativeTaskPolicy,id:u32,depth:usize,out:&mut Vec<u32>)->Result<()> {
            ensure!(depth<64,"CatSystem2 forced FES ancestry depth");
            for child in policy.fes_tasks.values().filter(|t|t.parent==id){visit(policy,child.id,depth+1,out)?;}
            out.push(id);Ok(())
        }
        let mut out=Vec::new();visit(self,owner,0,&mut out)?;Ok(out)
    }
    pub(crate) fn acknowledge_fes_native(&mut self,id:u32,operation:&crate::fes_program::PendingOperation)->Result<()> {
        self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?.player.as_mut()
            .context("CatSystem2 FES player missing")?.acknowledge_native(operation)
    }
    pub(crate) fn acknowledge_fes_object(&mut self,id:u32,operation:&crate::fes_program::PendingOperation)->Result<()> {
        self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?.player.as_mut()
            .context("CatSystem2 FES player missing")?.acknowledge_object(operation)
    }
    pub(crate) fn force_fes_tick(&mut self,id:u32,input:&crate::native_input::InputState,skip:u32,interpreter:&mut crate::native_interpreter::Interpreter,budget:u32)->Result<Option<crate::fes_program::PendingOperation>> {
        let clock=self.fes(id)?.player.as_ref().context("CatSystem2 FES player missing")?.timing.now;
        self.force_fes_tick_at(id,input,skip,interpreter,budget,clock)
    }
    pub(crate) fn begin_fes_wait(&mut self,id:u32,operation:&crate::fes_program::PendingOperation,relative:Option<u32>,duration:u32)->Result<()> {
        self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?.player.as_mut().context("CatSystem2 FES player missing")?.begin_wait(operation,relative,duration)
    }
    pub(crate) fn begin_fes_object_wait(&mut self,id:u32,operation:&crate::fes_program::PendingOperation)->Result<()> {
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?;
        ensure!(task.object_root.is_some(),"CatSystem2 all-object wait has no shared object resource root");
        task.player.as_mut().context("CatSystem2 FES player missing")?.begin_object_wait(operation)
    }
    pub(crate) fn clear_fes_object_wait(&mut self,id:u32)->Result<()> {
        let player=self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?.player.as_mut().context("CatSystem2 FES player missing")?;
        ensure!(player.timing.wait==Some(crate::fes_timing::Wait::Objects),"CatSystem2 stale all-object completion");
        player.timing.wait=None;Ok(())
    }
    pub(crate) fn prepare_fes_object_root(&mut self,id:u32)->Result<crysirror_scene::layer::LayerSpec> {
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 FES task missing")?;
        ensure!(task.player_loaded&&task.player.is_some()&&task.object_root.is_none(),"CatSystem2 FES object root admission state");
        let layer=0x4a000000u32.checked_add(id).context("CatSystem2 FES layer handle overflow")?;
        let spec=serde_json::from_value(serde_json::json!({"id":layer,"parent":null,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":true}))?;
        task.object_root=Some(layer);Ok(spec)
    }
    pub(crate) fn publish_fes_object_root(&mut self,id:u32,host:&mut dyn crysirror_core::service::HostServices,client:crysirror_core::state::ClientId)->Result<()> {
        //Prepare on a clone so a failed engine publication cannot leave a
        //logical root whose actual shared resource never existed.
        let mut proposed=self.clone();let layer=proposed.prepare_fes_object_root(id)?;
        host.submit_engine(client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":layer}]}))?)?;
        *self=proposed;Ok(())
    }
    pub(crate) fn attach_fes_child_after_tick(&mut self,owner:u32,child:u32)->Result<()> {
        let task=self.fes(child)?;
        ensure!(task.parent==owner&&task.player_loaded&&!task.first_tick&&task.running,"CatSystem2 child first tick is not complete");
        let parent=self.fes_tasks.get_mut(&owner).context("CatSystem2 FES parent missing")?;
        ensure!(parent.child_tasks.iter().all(Option::is_none),"CatSystem2 FES already has an active child");
        parent.child_tasks[0]=Some(child);Ok(())
    }
    pub(crate) fn observe_fes_child_result(&self,owner:u32,interpreter:&mut crate::native_interpreter::Interpreter)->Result<()> {
        let task=self.fes(owner)?;
        if let Some(child)=task.child_tasks[0].or(task.child_tasks[1]) {
            ensure!(self.exists(child),"CatSystem2 stale original child result reference");
            let value=self.task_result(child);
            //629850 samples23 AFTER the parent's own script phase. It does
            //not replace108 here; keep cached-child status distinct from20.
            if value!=task.exit_code as u32 {
                ensure!(interpreter.select_player_bank(owner),"CatSystem2 child-result parent bank missing");
                interpreter.set_named_value(b"_EXITCODE_",0,value)?;
            }
        }
        Ok(())
    }
    pub(crate) fn force_fes_tick_at(&mut self,id:u32,input:&crate::native_input::InputState,skip:u32,interpreter:&mut crate::native_interpreter::Interpreter,budget:u32,clock:u32)->Result<Option<crate::fes_program::PendingOperation>> {
        self.force_fes_tick_at_with_strings(id,input,skip,interpreter,budget,clock,None)
    }
    pub(crate) fn force_fes_tick_at_with_strings(&mut self,id:u32,input:&crate::native_input::InputState,skip:u32,interpreter:&mut crate::native_interpreter::Interpreter,budget:u32,clock:u32,strings:Option<&crate::native_values::StringVariables>)->Result<Option<crate::fes_program::PendingOperation>> {
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 target is not a native FES task")?;
        ensure!(task.player_loaded&&!task.first_tick&&!task.paused&&!task.deletion_pending&&task.input_task==Some(1),"CatSystem2 native FES forced-tick task state");
        ensure!(interpreter.select_player_bank(id),"CatSystem2 FES input prelude local bank missing");
        let [x,y]=input.fes_position();
        interpreter.set_named_value(b"_MOUSE_X_",0,x as u32)?;
        interpreter.set_named_value(b"_MOUSE_Y_",0,y as u32)?;
        for (name,value) in task.click_values.drain(..){interpreter.set_named_value(name,0,value)?;}
        let player=task.player.as_mut().context("CatSystem2 native FES player missing")?;
        if skip!=task.click_state {
            //629850→756B10 events19/20 have no object target: native v9=-1.
            interpreter.set_named_value(b"_CLICK_L_",0,if skip!=0 {u32::MAX}else{0})?;
            player.click_down=skip!=0;player.force_skip=skip!=0;
            task.click_state=skip;
        }
        task.running=true;
        player.step_at_clock_with_strings(interpreter,budget,clock,strings)
    }
    ///629850's button pass, run once per owner frame before the player steps.
    pub(crate) fn dispatch_fes_buttons(&mut self,id:u32,pointer:[i32;2],left:bool,right:bool)->Result<()> {
        use crate::fes_program::PendingKind;
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 target is not a native FES task")?;
        let previous=task.button_down;
        task.button_down=[left,right];
        if let Some(bank)=task.object_bank.as_mut() {
            let events=crate::fes_button::step(bank,pointer,[previous[0],left],[previous[1],right]);
            // 756B10 events 2/3: the pressed object; events 4/5 clear it on any release.
            for event in &events{
                use crate::fes_button::Event as E;
                match event.event{
                    E::PushLeft=>task.click_values.push((&b"_CLICK_L_"[..],event.object)),
                    E::PushRight=>task.click_values.push((&b"_CLICK_R_"[..],event.object)),
                    E::ScrollPosition=>task.click_values.push((&b"_SCRL_P_"[..],event.object)),
                    _=>{}
                }
            }
            task.button_queue.extend(events);
        }
        if previous[0]&&!left{task.click_values.push((&b"_CLICK_L_"[..],0));}
        if previous[1]&&!right{task.click_values.push((&b"_CLICK_R_"[..],0));
        }
        let Some(player)=task.player.as_mut() else{return Ok(())};
        // A dependency owned by the current row must finish before an event runs.
        let idle=player.pending.as_ref().is_none_or(|p|matches!(p.kind,PendingKind::SectionRestart|PendingKind::BlankWait));
        if idle {
            while let Some(event)=task.button_queue.pop_front() {
                if player.begin_event(&event.section,&event.name,event.index)? {break;}
            }
        }
        Ok(())
    }
    pub(crate) fn load_fes_player(&mut self,id:u32,path:&str,program:Arc<crate::fes_program::Program>,interpreter:&mut crate::native_interpreter::Interpreter,strings:&crate::native_values::StringVariables)->Result<()> {
        let current=self.current;
        let parent=self.fes(id)?.parent;
        if parent!=current {
            let parent=self.fes(parent)?;
            ensure!(parent.player_loaded&&parent.interpreter_owner==Some(current),"CatSystem2 nested FES parent interpreter is not attached");
        }
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 target is not a native FES task")?;
        ensure!(task.string_bank_owner==Some(current)&&task.player.is_none()&&!task.player_loaded,"CatSystem2 unsupported FES player recreation/owner profile");
        ensure!(path.len()<260&&path.is_ascii(),"CatSystem2 native FES script-name extent/encoding");
        //331 creates the original local bank and connects Main's Interpreter,
        //string bank and actual INPUT/SOUND logical roots. No object/pixel
        //resources exist in this bounded no-OBJECT program profile.
        let player=if program.declarations.is_empty(){
            let player=crate::fes_program::Machine::new(program,id,interpreter)?;
            task.object_bank=Some(crate::fes_object_bank::Bank::prepare(id,&player.program.declarations,interpreter,strings)?);
            player
        }else{
            #[cfg(feature="runtime")]
            {
                let mut staged=interpreter.clone();
                crate::fes_program::initialize_player_bank(id,&mut staged)?;
                let bank=crate::fes_object_bank::Bank::prepare(id,&program.declarations,&mut staged,strings)?;
                crate::fes_object_resources::preflight(&bank)?;
                //331/447 constructs only metadata.629850 executes the original
                //script first, then6287B0/62D4A0 initializes/publishes resources.
                let player=crate::fes_program::Machine::from_initialized(program,id);
                task.object_bank=Some(bank);*interpreter=staged;player
            }
            #[cfg(not(feature="runtime"))]
            {anyhow::bail!("CatSystem2 FES declarations require shared object/input resource publication; runtime capability is not linked");}
        };
        interpreter.set_named_value(b"_EXITCODE_",0,task.exit_code as u32)?;
        interpreter.set_named_value(b"_SAVEOBJ_",0,0)?;
        task.player=Some(player);task.player_loaded=true;
        task.interpreter_owner=Some(current);task.input_task=Some(1);task.sound_task=Some(2);
        task.script_name=path.as_bytes().to_vec();
        Ok(())
    }
    pub(crate) fn fes_object_bank_mut(&mut self,id:u32)->Result<Option<&mut crate::fes_object_bank::Bank>> {
        Ok(self.fes_tasks.get_mut(&id).context("CatSystem2 FES object-bank owner missing")?.object_bank.as_mut())
    }
    #[cfg(feature="runtime")]
    pub(crate) fn commit_fes_class_frame(&mut self,id:u32,list:u32,receipt:&crate::fes_object_resources::Receipt)->Result<()> {
        ensure!(self.fes(id)?.image_list==Some(list)&&receipt.owner==id,"CatSystem2 class receipt/task binding changed");
        self.image_lists.get_mut(&list).context("CatSystem2 class ImageList missing after publication")?.publication=Some(receipt.list.clone());
        let task=self.fes_tasks.get_mut(&id).unwrap();
        task.object_bank=Some(receipt.bank.clone());
        task.class_image_leases=receipt.resources.values().map(|v|v.path.relative().to_owned()).collect();Ok(())
    }
    pub fn bind_fes_image_list(&mut self,id:u32,image_list:u32)->Result<()> {
        let list=self.image_list(image_list)?;
        let pool=list.manager.as_ref().context("CatSystem2 native FES image-list manager missing")?;
        let [w,h]=pool.resource.size();
        let position=list.upload_position;
        let task=self.fes_tasks.get_mut(&id).context("CatSystem2 target is not a native FES task")?;
        task.image_list=Some(image_list);task.native_rect=[0,0,w as i32,h as i32];
        task.upload_position=position;task.message_parameter=0;
        Ok(())
    }
    ///629850 first tick occurs before the parent KCS resumes from71. Merely
    ///exhausting a host quota or polling a resource does not tick native tasks.
    pub(crate) fn initialize_fes_task(&mut self,id:u32)->Result<crysirror_scene::input_routes::InputReceiverSpec> {
        let task=self.fes(id)?;
        ensure!(task.first_tick&&!task.paused,"CatSystem2 unsupported native FES first-tick profile");
        let mut parent=task.parent;let mut priority=1;
        while parent!=0 {
            ensure!(priority<64,"CatSystem2 FES native ancestry depth");priority+=1;
            parent=match self.fes_tasks.get(&parent){Some(task)=>task.parent,None=>0};
        }
        let button=self.create_button(id,b"low button".to_vec())?;
        self.bind_button(button.id,id,[-100000,-100000,100000,100000],button.id,true)?;
        let receiver=serde_json::from_value(serde_json::json!({"id":button.id,"priority":priority,"sources":7,"rect":[-100000,-100000,200000,200000],"coverage":null,"enabled":true,"hover_enabled":true,"drag_enabled":true}))?;
        let task=self.fes_tasks.get_mut(&id).unwrap();task.low_button=Some(button.id);task.first_tick=false;
        Ok(receiver)
    }
    ///Only ordinary base child phases reset all existing button results. A
    ///nested synchronous333 first tick uses initialize_fes_task directly.
    pub fn initialize_fes_tasks(&mut self,canvas:[u32;2])->Result<Vec<crysirror_scene::input_routes::InputReceiverSpec>> {
        ensure!(canvas.iter().all(|v|*v<=100000),"CatSystem2 unsupported native FES full-screen rectangle");
        for button in self.buttons.values_mut(){button.result=0;}
        let ids=self.fes_tasks.values().filter(|f|f.first_tick&&!f.deletion_pending).map(|f|f.id).collect::<Vec<_>>();
        ids.into_iter().map(|id|self.initialize_fes_task(id)).collect()
    }
    pub fn exists(&self, id: u32) -> bool {
        self.slots.contains(&id)
    }
    pub fn create_plane(&mut self, parent: u32, name: Vec<u8>) -> Result<NativePlane> {
        ensure!(
            self.slots.contains(&parent),
            "CatSystem2 task parent missing"
        );
        ensure!(
            name.len() < 32 && !name.contains(&0),
            "CatSystem2 native task name extent"
        );
        let id = self.allocate()?;
        let plane = NativePlane {
            id,
            parent,
            name,
            layer: 0x43000000 + id * 2,
            graph_parent: None,
            native_rect: [0; 4],
            payload: None,
            image: None,
            solid_vertex_argb: None,diffuse_enable:0,diffuse_argb:u32::MAX,
            own_enabled: true,
            pose: Default::default(),
            source_pose: None,
            render_type: 1,
            own_button: false,
        };
        self.planes.insert(id, plane.clone());
        Ok(plane)
    }
    pub(crate) fn set_button_result(&mut self, id: u32, value: u32) -> Result<()> {
        self.buttons.get_mut(&id).with_context(|| format!("CatSystem2 button {id} missing"))?.result = value;
        Ok(())
    }
    pub fn button(&self, id: u32) -> Result<&NativeButton> {
        self.buttons
            .get(&id)
            .context("CatSystem2 target is not a native click controller")
    }
    pub fn bind_button(
        &mut self,
        id: u32,
        target: u32,
        rect: [i32; 4],
        event_mode: u32,
        full_canvas: bool,
    ) -> Result<()> {
        let b = self
            .buttons
            .get_mut(&id)
            .context("CatSystem2 target is not a native click controller")?;
        ensure!(
            b.binding_target.is_none(),
            "CatSystem2 click controller already bound"
        );
        b.binding_target = Some(target);
        b.rect = rect;
        b.event_mode = event_mode;
        b.full_canvas_rect = full_canvas;
        b.result = 1;
        Ok(())
    }
    /// 5E01F0 message 89: a Plane is also a 5DE590 click controller and binds
    /// its own geometry. Returns false when the controller was already bound
    /// (5DE8E0 case 89 leaves an existing binding unchanged).
    pub(crate) fn bind_plane_button(&mut self, id: u32, target: u32, rect: [i32; 4], event_mode: u32, full_canvas: bool) -> Result<bool> {
        let plane = self.planes.get_mut(&id).context("CatSystem2 target is not a native Plane task")?;
        plane.own_button = true;
        if self.buttons.get(&id).is_some_and(|b| b.binding_target.is_some()) {
            return Ok(false);
        }
        let (parent, name, own_enabled) = (plane.parent, plane.name.clone(), plane.own_enabled);
        self.buttons.insert(id, NativeButton { id, parent, name, binding_target: Some(target), rect, own_enabled, relative: false, result: 1, event_mode, full_canvas_rect: full_canvas });
        Ok(true)
    }
    pub fn create_button(&mut self, parent: u32, name: Vec<u8>) -> Result<NativeButton> {
        ensure!(
            self.slots.contains(&parent),
            "CatSystem2 task parent missing"
        );
        ensure!(
            name.len() < 32 && !name.contains(&0),
            "CatSystem2 native task name extent"
        );
        let id = self.allocate()?;
        let button = NativeButton {
            id,
            parent,
            name,
            binding_target: None,
            rect: [0; 4],
            own_enabled: true,
            relative: false,
            result: 0,
            event_mode: 0,
            full_canvas_rect: false,
        };
        self.buttons.insert(id, button.clone());
        Ok(button)
    }
    pub fn create_image_list(&mut self, parent: u32, name: Vec<u8>) -> Result<NativeImageList> {
        ensure!(
            self.slots.contains(&parent),
            "CatSystem2 task parent missing"
        );
        ensure!(
            name.len() < 32 && !name.contains(&0),
            "CatSystem2 native task name extent"
        );
        let task = NativeImageList {
            id: self.allocate()?,
            parent,
            name,
            manager: None,
            field_120: 1,
            field_124: 0,
            field_132: 0,
            upload_position: None,
            base_plane: None,
            field_140: 1,
            bitmap: None,
            bitmap_field4: 0,
            bitmap_field8: 0,
            bitmap_field532: 1,
            bitmap_name: Vec::new(),
            native_entry_keys: BTreeSet::new(),
            publication:None,
        };
        self.image_lists.insert(task.id, task.clone());
        Ok(task)
    }
    ///644370→283→743E10 only stores upload offsets; it does not move a
    /// plane, compose pixels, mark damage or force presentation. Uninitialised
    /// source fields stay None until this setter is actually called.
    pub fn set_image_upload_position(&mut self, id: u32, position: [i32; 2]) -> Result<()> {
        let task = self
            .image_lists
            .get_mut(&id)
            .context("CatSystem2 target is not native image-list task")?;
        task.upload_position = Some(position);
        if let Some(pool) = &mut task.manager {
            pool.upload_position = Some(position);
        }
        Ok(())
    }
    ///643E10→286 selects original CPU/presentation publication policy. It
    /// does not compose or show pixels; the later native draw operation does.
    pub fn set_image_mode(&mut self, id: u32, value: u32) -> Result<()> {
        let task = self
            .image_lists
            .get_mut(&id)
            .context("CatSystem2 target is not native image-list task")?;
        task.field_120 = value;
        task.field_124 = value;
        if value != 0 {
            task.field_140 = 1;
        }
        Ok(())
    }
    pub fn image_list_mut(&mut self, id: u32) -> Result<&mut NativeImageList> {
        self.image_lists
            .get_mut(&id)
            .context("CatSystem2 target is not native image-list task")
    }
    pub fn image_list(&self, id: u32) -> Result<&NativeImageList> {
        self.image_lists
            .get(&id)
            .context("CatSystem2 target is not native image-list task")
    }
    #[cfg(feature="runtime")]
    pub(crate) fn set_base_image_publication(&mut self,id:u32,publication:crate::native_image_publication::ImageList,visible:bool)->Result<()> {
        let list=self.image_lists.get_mut(&id).context("CatSystem2 baseimg ImageList missing")?;
        list.publication=Some(publication);list.field_132=u32::from(visible);Ok(())
    }
    pub fn bind_image_pool(
        &mut self,
        id: u32,
        pool: NativeImagePool,
        base: u32,
        image: NativePlaneImage,
    ) -> Result<()> {
        let task = self
            .image_lists
            .get_mut(&id)
            .context("CatSystem2 target is not native image-list task")?;
        ensure!(
            task.manager.is_none() && task.base_plane.is_none(),
            "CatSystem2 unsupported image-list reinitialization"
        );
        task.manager = Some(pool);
        task.base_plane = Some(base);
        let plane = self
            .planes
            .get_mut(&base)
            .context("CatSystem2 base image plane missing")?;
        plane.payload = Some(plane.layer + 1);
        plane.pose.size = image.resource.size().map(|value| value as f32);
        plane.image = Some(image);
        plane.own_enabled = false;
        Ok(())
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"fes_tasks":self.fes_tasks.values().map(crate::fes_task::FesTask::diagnostics).collect::<Vec<_>>(),"current":self.current,"task_results":self.slots.iter().map(|id|(*id,self.results.query(*id))).collect::<Vec<_>>(),"allocated":self.slots.len(),"image_lists":self.image_lists.values().map(|t|serde_json::json!({"id":t.id,"task_parent":t.parent,"name":String::from_utf8_lossy(&t.name),"manager":t.manager.as_ref().map(|p|serde_json::json!({"path":p.path,"size":p.size,"owner":p.resource.client().0,"bytes":p.resource.rgba().len(),"page_size":p.page_size,"page_count":p.page_count,"dirty_bounds":p.dirty_bounds,"upload_position":p.upload_position,"native_entries":p.native_entry_keys.len()})),"publication":t.publication.as_ref().map(crate::native_image_publication::ImageList::diagnostics),"field_120":t.field_120,"field_124":t.field_124,"field_132":t.field_132,"upload_position":t.upload_position,"base_plane":t.base_plane,"field_140":t.field_140,"empty_bitmap":{"resource":t.bitmap.is_some(),"field4":t.bitmap_field4,"field8":t.bitmap_field8,"field532":t.bitmap_field532,"name_utf16":t.bitmap_name},"native_entry_keys":t.native_entry_keys.len()})).collect::<Vec<_>>(),"buttons":self.buttons.values().map(|b|serde_json::json!({"id":b.id,"task_parent":b.parent,"name":String::from_utf8_lossy(&b.name),"binding_target":b.binding_target,"native_rect":b.rect,"own_enabled":b.own_enabled,"relative":b.relative,"result":b.result,"event_mode":b.event_mode,"full_canvas_rect":b.full_canvas_rect})).collect::<Vec<_>>(),"planes":self.planes.values().map(|p|serde_json::json!({"id":p.id,"task_parent":p.parent,"layer":p.layer,"payload":p.payload,"image":p.image.as_ref().map(|i|serde_json::json!({"path":i.path,"size":i.resource.size(),"owner":i.resource.client().0})),"name":String::from_utf8_lossy(&p.name),"graph_parent":p.graph_parent,"own_enabled":p.own_enabled,"source_pose":p.source_pose.map(|pose|serde_json::json!({"position":pose.position,"base":pose.base,"scale":pose.scale,"matrix":pose.matrix})),"pose":{"position":p.pose.position,"base":p.pose.base,"size":p.pose.size,"scale":p.pose.scale,"alpha":p.pose.alpha},"native_rect":p.native_rect,"solid_vertex_argb":p.solid_vertex_argb,"diffuse_enable":p.diffuse_enable,"diffuse_argb":p.diffuse_argb})).collect::<Vec<_>>()})
    }
    pub fn plane(&self, id: u32) -> Result<&NativePlane> {
        self.planes
            .get(&id)
            .context("CatSystem2 target is not a native Plane task")
    }
    #[cfg(feature="runtime")]
    pub(crate) fn list_publication(&mut self,id:u32,client:crysirror_core::state::ClientId)->Result<&mut crate::native_image_publication::ImageList>{
        let list=self.image_lists.get_mut(&id).context("CatSystem2 target is not native image-list task")?;
        if list.publication.is_none(){
            let size=list.manager.as_ref().context("CatSystem2 ImageList has no atlas")?.size;
            list.publication=Some(crate::native_image_publication::ImageList::new(client,0x58000000u32.checked_add(id).context("CatSystem2 ImageList Canvas range")?,size)?);
        }
        Ok(list.publication.as_mut().unwrap())
    }
    /// The list whose atlas backs Plane VRAM origin (0,0): 41 leaves this[42..43] at zero.
    #[cfg(feature="runtime")]
    pub(crate) fn main_image_list(&self)->Option<u32>{
        self.image_lists.iter().find(|(_,list)|list.manager.is_some()&&list.upload_position.is_none_or(|p|p==[0,0])).map(|(id,_)|*id)
    }
    /// A message from a FES task to the KCS task that owns it.
    pub(crate) fn post_script_event(&mut self,mut from:u32,message:[u32;4]){
        while let Some(task)=self.fes_tasks.get(&from){from=task.parent;}
        self.script_events.push_back((from,message));
    }
    pub(crate) fn queue_script_event(&mut self,script:u32,message:[u32;4]){
        self.script_events.push_back((script,message));
    }
    pub(crate) fn has_script_event(&self,script:u32)->bool{self.script_events.iter().any(|(owner,_)|*owner==script)}
    pub(crate) fn take_script_event(&mut self,script:u32)->Option<[u32;4]>{
        let at=self.script_events.iter().position(|(owner,_)|*owner==script)?;
        self.script_events.remove(at).map(|(_,message)|message)
    }
    pub(crate) fn is_fes(&self, id: u32) -> bool {
        self.fes_tasks.contains_key(&id)
    }
    /// A task of a class whose state lives in a `native_classes` family.
    pub(crate) fn create_opaque(&mut self, class: u32, parent: u32, name: Vec<u8>) -> Result<u32> {
        let id = self.allocate()?;
        self.opaque.insert(id, OpaqueTask { class, parent, name });
        Ok(id)
    }
    /// Task +0xC: the task this one was created under (0 at the root).
    pub(crate) fn parent_of(&self, id: u32) -> Result<u32> {
        if let Some(task) = self.opaque.get(&id) {
            return Ok(task.parent);
        }
        if let Some(plane) = self.planes.get(&id) {
            return Ok(plane.parent);
        }
        if let Some(task) = self.fes_tasks.get(&id) {
            return Ok(task.parent);
        }
        if let Some((owner, _)) = self.fes_tasks.iter().find(|(_, task)| task.child_tasks.contains(&Some(id))) {
            return Ok(*owner);
        }
        Ok(0)
    }
    /// Class number of a task created through `create_opaque`.
    pub(crate) fn opaque_class(&self, id: u32) -> Option<u32> {
        self.opaque.get(&id).map(|task| task.class)
    }
    pub(crate) fn opaque_task(&self, id: u32) -> Option<&OpaqueTask> {
        self.opaque.get(&id)
    }
    /// Ids of every live task of `class`, in task-number order.
    pub(crate) fn opaque_of_class(&self, class: u32) -> Vec<u32> {
        self.opaque.iter().filter(|(_, task)| task.class == class).map(|(id, _)| *id).collect()
    }
    /// A destroyed Plane leaves every list that presents its payload.
    #[cfg(feature="runtime")]
    pub(crate) fn release_plane_publications(&mut self,id:u32)->Result<Vec<crysirror_scene::canvas::CanvasCommand>>{
        let Some(plane)=self.planes.get(&id) else{return Ok(Vec::new());};
        let parents=std::collections::BTreeSet::from([plane.layer+1]);
        let mut out=Vec::new();
        for list in self.image_lists.values_mut(){
            let Some(publication)=&mut list.publication else{continue;};
            let planes=publication.take_planes(&parents);
            if planes.is_empty(){continue;}
            for plane in planes{
                out.push(crysirror_scene::canvas::CanvasCommand::Hide{layer:plane.presentation});
                out.push(crysirror_scene::canvas::CanvasCommand::Destroy{id:plane.canvas});
            }
            out.extend(publication.commands()?);
        }
        Ok(out)
    }
    /// Drop a non-FES task and answer the layer commands releasing what it published.
    pub(crate) fn retire_plain_task(&mut self, id: u32) -> Result<Vec<serde_json::Value>> {
        let mut commands = Vec::new();
        if let Some(plane) = self.planes.remove(&id) {
            if plane.payload.is_some() {
                commands.push(serde_json::json!({"op":"remove","id":plane.layer + 1}));
            }
            commands.push(serde_json::json!({"op":"remove","id":plane.layer}));
        }
        self.buttons.remove(&id);
        self.image_lists.remove(&id);
        self.opaque.remove(&id);
        self.destroy_task_slot(id)?;
        Ok(commands)
    }
    pub(crate) fn plane_mut(&mut self, id: u32) -> Option<&mut NativePlane> {
        self.planes.get_mut(&id)
    }
    pub fn set_solid_payload(&mut self, id: u32, argb:u32) -> Result<()> {
        let plane = self
            .planes
            .get_mut(&id)
            .context("CatSystem2 target is not a native Plane task")?;
        plane.payload = Some(plane.layer + 1);
        plane.solid_vertex_argb=Some(argb);plane.diffuse_enable=0;plane.diffuse_argb=u32::MAX;
        plane.source_pose=None;
        Ok(())
    }
    pub fn set_solid_diffuse(&mut self,id:u32,argb:u32,enabled:u32)->Result<Option<(u32,[u8;4])>> {
        let plane=self.planes.get_mut(&id).context("CatSystem2 diffuse target is not a native Plane")?;
        let Some(payload)=plane.payload else{return Ok(None);};
        ensure!(plane.image.is_none(),"CatSystem2 unsupported native image diffuse override");
        let Some(base)=plane.solid_vertex_argb else{plane.diffuse_enable=enabled;plane.diffuse_argb=argb;return Ok(None);};
        plane.diffuse_enable=enabled;plane.diffuse_argb=argb;
        let c=if enabled!=0{argb}else{base};
        Ok(Some((payload,[(c>>16) as u8,(c>>8) as u8,c as u8,(c>>24) as u8])))
    }
    /// Original task messages47/48 carry the same visibility argument through
    /// the task tree, rather than inheriting transform/graph-parent state.
    /// Each descendant combines that event with its own native this[35].
    pub fn visibility_messages(&mut self, id: u32, enabled: bool) -> Result<Vec<(u32, bool)>> {
        self.planes
            .get_mut(&id)
            .context("CatSystem2 visibility target is not a native Plane")?
            .own_enabled = enabled;
        let mut affected = BTreeSet::from([id]);
        let mut pending = vec![id];
        let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for plane in self.planes.values() {
            children.entry(plane.parent).or_default().push(plane.id);
        }
        while let Some(parent) = pending.pop() {
            for child in children.get(&parent).into_iter().flatten() {
                if affected.insert(*child) {
                    pending.push(*child);
                }
            }
        }
        Ok(affected
            .into_iter()
            .map(|id| {
                let plane = &self.planes[&id];
                (plane.layer, plane.own_enabled && enabled)
            })
            .collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_external_section_and_save_object_preserve_wait_and_local_bank_ownership() {
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let first=tasks.create_fes(3,b"first".to_vec()).unwrap();
        let second=tasks.create_fes(3,b"second".to_vec()).unwrap();
        let program=Arc::new(crate::fes_program::Program::parse(
            b"#START\n$0=1\nwait 4\n#other\n$0+=10\nwait 1",&||Ok(())).unwrap());
        let mut interpreter=crate::native_interpreter::Interpreter::new(16).unwrap();
        tasks.load_fes_player(first,"fes.int/first.fes",program.clone(),&mut interpreter,&Default::default()).unwrap();
        tasks.load_fes_player(second,"fes.int/second.fes",program,&mut interpreter,&Default::default()).unwrap();
        tasks.initialize_fes_tasks([1280,720]).unwrap();
        tasks.set_fes_save_object(first,99,&mut interpreter).unwrap();
        assert_eq!(interpreter.evaluate(b"_SAVEOBJ_").unwrap(),99);
        assert!(interpreter.select_player_bank(second));
        assert_eq!(interpreter.evaluate(b"_SAVEOBJ_").unwrap(),0);
        let input=crate::native_input::InputState::default();
        let operation=tasks.force_fes_tick_at(first,&input,0,&mut interpreter,32,5).unwrap().unwrap();
        tasks.begin_fes_wait(first,&operation,None,4).unwrap();
        let base=tasks.fes(first).unwrap().player.as_ref().unwrap().timing.base;
        assert!(!tasks.set_fes_section(first,b"absent").unwrap());
        assert!(!tasks.set_fes_section(65535,b"other").unwrap());
        assert!(tasks.set_fes_section(first,b"other").unwrap());
        assert_eq!(tasks.fes(first).unwrap().player.as_ref().unwrap().timing.base,base);
        assert!(tasks.force_fes_tick_at(first,&input,0,&mut interpreter,32,8).unwrap().is_none());
        assert_eq!(interpreter.flag(0),1);
        let operation=tasks.force_fes_tick_at(first,&input,0,&mut interpreter,32,9).unwrap().unwrap();
        assert_eq!(operation.section,b"other");assert_eq!(interpreter.flag(0),11);
        assert!(tasks.set_fes_section(first,b"START").is_err());
        assert_eq!(tasks.fes(first).unwrap().player.as_ref().unwrap().pending.as_ref(),Some(&operation));
    }
    #[test]
    fn running_fes_keeps_native_wait_until_clock_and_empty_hover_preserves_no_callback() {
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let id=tasks.create_fes(3,b"main".to_vec()).unwrap();
        let source=Arc::new(crate::fes_program::Program::parse(b"#START\n$0+=1\nwait 2\n$0+=10\nsteam_isenable 4",&||Ok(())).unwrap());
        let mut interpreter=crate::native_interpreter::Interpreter::new(8).unwrap();
        tasks.load_fes_player(id,"fes.int/authored.fes",source,&mut interpreter,&Default::default()).unwrap();
        let receivers=tasks.initialize_fes_tasks([1280,720]).unwrap();let receiver=receivers[0].id;
        let input=crate::native_input::InputState::default();
        let pending=tasks.force_fes_tick_at(id,&input,0,&mut interpreter,100,10).unwrap().unwrap();assert_eq!(pending.name,b"wait");assert_eq!(interpreter.flag(0),1);
        tasks.begin_fes_wait(id,&pending,None,2).unwrap();assert_eq!(tasks.running_fes_tasks().unwrap(),[id]);
        assert!(tasks.force_fes_tick_at(id,&input,0,&mut interpreter,100,11).unwrap().is_none());assert_eq!(interpreter.flag(0),1);
        let pending=tasks.force_fes_tick_at(id,&input,0,&mut interpreter,100,12).unwrap().unwrap();assert_eq!(pending.name,b"steam_isenable");assert_eq!(interpreter.flag(0),11);assert_eq!(interpreter.evaluate(b"_FRAME_").unwrap(),3);
        let before=tasks.fes(id).unwrap().player.as_ref().unwrap().executed;
        tasks.fes_tasks.get_mut(&id).unwrap().selected_object=99;
        assert!(tasks.empty_fes_hover(receiver,&input,&mut interpreter).unwrap());
        assert_eq!(tasks.fes(id).unwrap().selected_object,-1);assert_eq!(tasks.fes(id).unwrap().player.as_ref().unwrap().executed,before);assert_eq!(interpreter.evaluate(b"_MOUSE_X_").unwrap(),i32::MIN as u32);assert_eq!(interpreter.flag(0),11);
    }
    #[test]
    fn root_input_audio_and_script_have_native_logical_numbers() {
        let tasks = NativeTaskPolicy::bootstrap().unwrap();
        assert_eq!(tasks.current(), 3);
        assert_eq!(tasks.slots.len(), 4);
    }
    #[test]
    fn original_uniform_vertex_diffuse_enables_and_restores_without_losing_base_color() {
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let id=tasks.create_plane(3,b"solid".to_vec()).unwrap().id;
        assert!(tasks.set_solid_diffuse(id,0xFF112233,1).unwrap().is_none());
        assert_eq!(tasks.plane(id).unwrap().diffuse_enable,0);
        tasks.set_solid_payload(id,0x80224466).unwrap();
        assert_eq!(tasks.set_solid_diffuse(id,0xFF112233,7).unwrap().unwrap().1,[0x11,0x22,0x33,0xFF]);
        assert_eq!(tasks.plane(id).unwrap().diffuse_enable,7);
        assert_eq!(tasks.set_solid_diffuse(id,0xDEADBEEF,0).unwrap().unwrap().1,[0x22,0x44,0x66,0x80]);
        assert_eq!(tasks.plane(id).unwrap().diffuse_argb,0xDEADBEEF);
        tasks.set_solid_payload(id,0x7F010203).unwrap();
        let plane=tasks.plane(id).unwrap();
        assert_eq!((plane.solid_vertex_argb,plane.diffuse_enable,plane.diffuse_argb),(Some(0x7F010203),0,u32::MAX));
        assert!(tasks.set_solid_diffuse(3,0,1).is_err());
    }
    #[test]
    fn fes_player_admission_owns_local_bank_and_preserves_original_globals() {
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let id=tasks.create_fes(3,b"main".to_vec()).unwrap();
        let program=Arc::new(crate::fes_program::Program::parse(b"#START\n$0=9",&||Ok(())).unwrap());
        let mut interpreter=crate::native_interpreter::Interpreter::new(16).unwrap();
        interpreter.set_flag(0,7);
        tasks.load_fes_player(id,"fes.int/flow.fes",program.clone(),&mut interpreter,&Default::default()).unwrap();
        let fes=tasks.fes(id).unwrap();
        assert!(fes.first_tick&&!fes.running&&fes.low_button.is_none());
        assert_eq!((fes.interpreter_owner,fes.input_task,fes.sound_task),(Some(3),Some(1),Some(2)));
        assert_eq!(fes.player.as_ref().unwrap().executed,0);
        assert_eq!(interpreter.flag(0),7);
        assert_eq!(interpreter.evaluate(b"_EXITCODE_").unwrap(),u32::MAX);
        assert_eq!(interpreter.evaluate(b"_SAVEOBJ_").unwrap(),0);
        assert!(tasks.load_fes_player(id,"fes.int/flow.fes",program,&mut interpreter,&Default::default()).is_err());
        assert_eq!(tasks.fes(id).unwrap().player.as_ref().unwrap().executed,0);
        let receiver=tasks.initialize_fes_tasks([1280,720]).unwrap();
        assert_eq!(receiver.len(),1);
        let fes=tasks.fes(id).unwrap();
        assert!(!fes.first_tick&&!fes.running&&fes.player.as_ref().unwrap().executed==0);
    }
    #[test]
    fn fes_attach_waits_for_native_child_tick_and_registers_real_shared_low_button() {
        use crysirror_core::state::ClientId;
        use crysirror_scene::input_routes::{InputRouteCommand,InputRouteEvent,InputRoutePhase,InputRouteSource,InputRouting};
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let old=tasks.create_button(3,b"existing".to_vec()).unwrap();
        tasks.bind_button(old.id,3,[-100000,-100000,100000,100000],5,true).unwrap();
        let id=tasks.create_fes(3,b"main".to_vec()).unwrap();
        let fes=tasks.fes_tasks.get(&id).unwrap();
        assert_eq!(fes.string_bank_owner,Some(3));
        assert!(fes.parameters.is_empty()&&fes.script_name.is_empty());
        assert!(fes.saved_cursor.is_none()&&fes.input_task.is_none()&&fes.sound_task.is_none());
        assert!(fes.first_tick&&fes.low_button.is_none()&&!fes.player_loaded);
        assert_eq!(tasks.buttons.len(),1);
        let mut routes=InputRouting::default();
        let ready=tasks.initialize_fes_tasks([1280,720]).unwrap();
        assert_eq!(ready.len(),1);
        assert_eq!(tasks.button(old.id).unwrap().result,0);
        let low=ready[0].id;
        let button=tasks.button(low).unwrap();
        assert_eq!((button.parent,button.binding_target,button.event_mode,button.result),(id,Some(id),low,1));
        assert_eq!(ready[0].priority,2);
        routes.apply_batch(ClientId(1),&[InputRouteCommand::Put{receiver:ready[0].clone()}]).unwrap();
        routes.dispatch(ClientId(1),InputRouteEvent{source:InputRouteSource::Pointer,code:16,phase:InputRoutePhase::Press,point:[640,360],origin:None}).unwrap();
        assert_eq!(routes.poll(ClientId(1)).unwrap().unwrap().receiver,low);
        assert!(tasks.initialize_fes_tasks([1280,720]).unwrap().is_empty());
        assert_eq!(tasks.button(low).unwrap().result,0);
        assert_eq!(tasks.buttons.len(),2);
        assert!(tasks.create_fes(old.id,Vec::new()).is_err());
    }
    #[test]
    fn image_list_constructs_source_empty_bitmap_wrapper_and_no_fake_image_manager() {
        let mut p = NativeTaskPolicy::bootstrap().unwrap();
        p.create_plane(3, Vec::new()).unwrap();
        p.create_button(3, Vec::new()).unwrap();
        let t = p.create_image_list(3, b"imglist".to_vec()).unwrap();
        assert_eq!(t.id, 6);
        assert_eq!(
            (t.field_120, t.field_124, t.field_132, t.field_140),
            (1, 0, 0, 1)
        );
        assert!(t.manager.is_none());
        assert_eq!(t.base_plane, None);
        assert_eq!(t.upload_position, None);
        p.set_image_upload_position(t.id, [-2, 3]).unwrap();
        assert_eq!(p.image_list(t.id).unwrap().upload_position, Some([-2, 3]));
        assert!(p.image_list(t.id).unwrap().manager.is_none());
        p.image_lists.get_mut(&t.id).unwrap().field_140 = 0;
        p.set_image_mode(t.id, 0).unwrap();
        let z = p.image_list(t.id).unwrap();
        assert_eq!((z.field_120, z.field_124, z.field_140), (0, 0, 0));
        p.set_image_mode(t.id, 1).unwrap();
        let z = p.image_list(t.id).unwrap();
        assert_eq!((z.field_120, z.field_124, z.field_140), (1, 1, 1));
        assert!(t.bitmap.is_none() && t.bitmap_name.is_empty());
        assert_eq!(
            (t.bitmap_field4, t.bitmap_field8, t.bitmap_field532),
            (0, 0, 1)
        );
        assert!(t.native_entry_keys.is_empty());
        assert!(p.plane(t.id).is_err() && p.button(t.id).is_err());
    }
    #[test]
    fn native_type6_is_an_unbound_input_controller_and_cannot_be_used_as_a_plane() {
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let button = tasks.create_button(3, b"bg button".to_vec()).unwrap();
        assert_eq!(button.id, 4);
        assert_eq!(button.binding_target, None);
        assert_eq!(button.rect, [0; 4]);
        assert!(button.own_enabled);
        assert!(!button.relative);
        assert_eq!(button.result, 0);
        assert!(tasks.exists(4));
        assert!(tasks.plane(4).is_err());
        assert!(tasks.planes.is_empty());
        assert!(tasks.create_button(65535, Vec::new()).is_err());
        assert_eq!(tasks.slots.len(), 5);
    }
    #[test]
    fn plane_group_keeps_native_empty_rect_and_graph_ancestry_separate() {
        use crysirror_core::state::ClientId;
        use crysirror_scene::layer::{LayerCommand, LayerSystem, SpriteTransform};
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let plane = tasks.create_plane(3, b"plane".to_vec()).unwrap();
        assert_eq!(plane.id, 4);
        assert_eq!(plane.native_rect, [0; 4]);
        assert_eq!(plane.graph_parent, None);
        let group = plane.layer_spec().unwrap();
        assert_eq!(group.parent, None);
        assert!(group.visible);
        assert_eq!(group.rect, [0., 0., 1., 1.]);
        let mut layers = LayerSystem::default();
        let mut group = group;
        group.rect[0] = 100.;
        group.rect[1] = 50.;
        let child:crysirror_scene::layer::LayerSpec=serde_json::from_value(serde_json::json!({"id":9,"parent":plane.layer,"rect":[3,4,2,2],"paint":{"kind":"solid","color":[255,255,255,255]}})).unwrap();
        layers
            .apply_batch(
                ClientId(1),
                &[
                    LayerCommand::Put { layer: group },
                    LayerCommand::Put { layer: child },
                ],
            )
            .unwrap();
        let frame = layers.frame([1280, 720]).unwrap();
        let child = frame.iter().find(|p| p.id == 9).unwrap();
        assert!(
            (child.quad[0][0] - 103.).abs() < 0.0001 && (child.quad[0][1] - 54.).abs() < 0.0001
        );
        let transform = SpriteTransform {
            rotate: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        layers
            .apply_batch(
                ClientId(1),
                &[LayerCommand::SetTransform {
                    id: plane.layer,
                    transform,
                }],
            )
            .unwrap();
        let frame = layers.frame([1280, 720]).unwrap();
        let child = frame.iter().find(|p| p.id == 9).unwrap();
        assert!((child.quad[0][0] - 96.).abs() < 0.0001 && (child.quad[0][1] - 53.).abs() < 0.0001);
        assert!(tasks.create_plane(65535, b"missing".to_vec()).is_err());
    }
    #[test]
    fn native_visibility_events_are_not_persistent_graph_parent_inheritance() {
        let mut tasks = NativeTaskPolicy::bootstrap().unwrap();
        let a = tasks.create_plane(3, b"a".to_vec()).unwrap();
        let b = tasks.create_plane(a.id, b"b".to_vec()).unwrap();
        let c = tasks.create_plane(b.id, b"c".to_vec()).unwrap();
        assert_eq!(
            tasks.visibility_messages(a.id, false).unwrap(),
            vec![(a.layer, false), (b.layer, false), (c.layer, false)]
        );
        assert_eq!(
            tasks.visibility_messages(b.id, true).unwrap(),
            vec![(b.layer, true), (c.layer, true)]
        );
        tasks.visibility_messages(b.id, false).unwrap();
        assert_eq!(
            tasks.visibility_messages(a.id, true).unwrap(),
            vec![(a.layer, true), (b.layer, false), (c.layer, true)]
        );
        assert_eq!(tasks.plane(b.id).unwrap().graph_parent, None);
    }
}

#[cfg(test)]
mod nested_task_tests {
    use super::*;
    #[test]
    fn attach_inherits_real_string_and_interpreter_roots_with_native_descendant_priority(){
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();
        let parent=tasks.create_fes(3,b"parent".to_vec()).unwrap();
        let child=tasks.create_fes(parent,b"child".to_vec()).unwrap();
        assert_eq!(tasks.fes(child).unwrap().string_bank_owner,Some(3));
        assert_eq!(tasks.fes(child).unwrap().interpreter_owner,None);
        let program=Arc::new(crate::fes_program::Program::parse(b"#START\nwait 1",&||Ok(())).unwrap());
        let mut interpreter=crate::native_interpreter::Interpreter::new(8).unwrap();
        assert!(tasks.load_fes_player(child,"fes.int/child.fes",program.clone(),&mut interpreter,&Default::default()).is_err());
        assert_eq!(interpreter.diagnostics()["local_banks"],0);
        tasks.load_fes_player(parent,"fes.int/parent.fes",program.clone(),&mut interpreter,&Default::default()).unwrap();
        tasks.load_fes_player(child,"fes.int/child.fes",program,&mut interpreter,&Default::default()).unwrap();
        assert_eq!(tasks.fes(child).unwrap().interpreter_owner,Some(3));
        interpreter.set_player_local(parent,0,77).unwrap();interpreter.set_player_local(child,0,88).unwrap();
        assert_eq!(interpreter.evaluate(b"$$0").unwrap(),88);interpreter.select_player_bank(parent);
        assert_eq!(interpreter.evaluate(b"$$0").unwrap(),77);
        let receivers=tasks.initialize_fes_tasks([64,64]).unwrap();
        assert_eq!(receivers.iter().map(|r|r.priority).collect::<Vec<_>>(),[2,3]);
        let input=crate::native_input::InputState::default();
        tasks.force_fes_tick(parent,&input,0,&mut interpreter,100).unwrap();
        tasks.force_fes_tick(child,&input,0,&mut interpreter,100).unwrap();
        assert_eq!(tasks.running_fes_tasks().unwrap(),[child,parent]);
        assert_eq!(tasks.fes(child).unwrap().string_bank_owner,Some(3));
        assert!(tasks.create_fes(999,b"missing".to_vec()).is_err());
    }
}

#[cfg(test)]
mod unloaded_hover_tests {
    use super::*;
    #[test]
    fn original_empty_hover_before_player_and_while_paused_requires_no_guessed_input(){
        let mut tasks=NativeTaskPolicy::bootstrap().unwrap();let owner=tasks.create_fes(3,b"unloaded".to_vec()).unwrap();
        tasks.initialize_fes_tasks([64,64]).unwrap();let button=tasks.fes(owner).unwrap().low_button.unwrap();
        let mut interpreter=crate::native_interpreter::Interpreter::new(8).unwrap();
        assert!(tasks.empty_fes_hover(button,&Default::default(),&mut interpreter).unwrap());
        assert_eq!(interpreter.diagnostics()["local_banks"],0);
        let program=Arc::new(crate::fes_program::Program::parse(b"#START\nwait 1",&||Ok(())).unwrap());
        tasks.load_fes_player(owner,"fes.int/hover.fes",program,&mut interpreter,&Default::default()).unwrap();
        interpreter.set_named_value(b"_MOUSE_X_",0,123).unwrap();
        tasks.fes_tasks.get_mut(&owner).unwrap().paused=true;
        assert!(tasks.empty_fes_hover(button,&Default::default(),&mut interpreter).unwrap());
        assert_eq!(interpreter.evaluate(b"_MOUSE_X_").unwrap(),123);
    }
}
