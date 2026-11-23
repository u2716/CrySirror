// SPDX-License-Identifier: MPL-2.0
use crate::kcs::{Event, Machine, Program};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{EngineQuery, HostServices, InputFrame},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_resource::xml::XmlDocument;
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Capability-relative original KCS entry; no substituted CST boot script.
    pub script: String,
    /// Licensed font alias belonging to this engine project.
    pub font: String,
    /// Original startup.xml ETC/heap byte count.
    #[serde(default = "heap")]
    pub heap_bytes: u32,
    /// Caller-owned PE module read only as data to verify native syscall ABI.
    #[serde(default)]
    pub executable: Option<String>,
    /// Original virtual canvas from startup.xml, independent of host monitor.
    #[serde(default = "canvas")]
    pub canvas: [u32; 2],
    ///Explicit float/integer native stream seeds for deterministic sessions.
    ///Absent seeds use one bounded shared OS seed worker, never host time.
    #[serde(default)]
    pub random_seeds:Option<[u32;2]>,
}
fn canvas() -> [u32; 2] {
    [1280, 720]
}
fn heap() -> u32 {
    1024 * 1024
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            VfsPath::new(Mount::Asset, &self.script)?.relative() == self.script
                && self.script.to_ascii_lowercase().ends_with(".kcs"),
            "CatSystem2 original KCS entry path"
        );
        ensure!(
            !self.font.is_empty() && self.font.len() <= 256,
            "CatSystem2 font alias"
        );
        ensure!(
            (4..=16 * 1024 * 1024).contains(&self.heap_bytes),
            "CatSystem2 heap budget"
        );
        if let Some(path) = &self.executable {
            ensure!(
                VfsPath::new(Mount::Asset, path)?.relative() == path,
                "CatSystem2 module capability path"
            );
        }
        ensure!(
            self.canvas.iter().all(|n| (1..=8192).contains(n)),
            "CatSystem2 canvas budget"
        );
        Ok(())
    }
}
pub struct CatSystem2Vm {
    client: ClientId,
    config: Config,
    pool: Option<TaskPool>,
    loading: Option<Task<Prepared>>,
    program: Option<Program>,
    machine: Option<Machine>,
    error: Option<String>,
    native_abi: bool,
    null_view_constructor: bool,
    abi_warned: bool,
    input_routes_bound:bool,
    event_wait:Option<NativeEventWait>,
    kcs_children: Vec<KcsChild>,
    kcs_starts: std::collections::VecDeque<(u32,Vec<u8>)>,
    kcs_loading_task: Option<u32>,
    string_maps:crate::native_string_map::StringMaps,
    kcs_loading:Option<(u32,String,Task<Program>)>,
    image_pool_loading:Option<crate::image_list::PendingImages>,
    fes_loading:Option<crate::fes_loading::Pending>,
    texture_cache:Option<crate::texture_cache::NativeTextureCache>,
    texture_clear:Option<crate::texture_cache::PendingClear>,
    debug_key_absent: bool,
    menu_installed: bool,
    menu_state: crate::native_menu::State,
    tasks: crate::task_policy::NativeTaskPolicy,
    xml_loading: Option<Task<Option<Arc<XmlDocument>>>>,
    loop_loading: Option<Task<PreparedLoops>>,
    settings_loading: Option<Task<crate::native_settings::NativeSettings>>,
    settings: Option<crate::native_settings::NativeSettings>,
    loop_catalog_revision: u64,
    loop_entries: u32,
    documents: BTreeMap<u32, Arc<XmlDocument>>,
    next_document: u32,
    document_bytes: usize,
    native_calls: BTreeMap<u32, u64>,
    interpreter: Option<crate::native_interpreter::Interpreter>,
    handles: crate::native_scene::Handles,
    #[cfg(feature="runtime")]
    m2: crate::native_m2::Planes,
    #[cfg(feature="runtime")]
    kcs_sound: crate::native_kcs_sound::KcsSound,
    motion_loading: Option<(Vec<u8>,crysirror_tasks::Task<std::sync::Arc<Program>>)>,
    #[cfg(feature="runtime")]
    image_loading: std::collections::BTreeMap<u32,crate::native_image_resource::Pending>,
    /// Native 111: the remaining "name[:frame]" tokens, the frames loaded so far,
    /// the current load and the final composition job.
    #[cfg(feature="runtime")]
    layered_loading: Option<LayeredLoad>,
    /// 365's refcounted file blocks {bytes, size, refcount} and the pending read,
    /// decode (364/368) or recolour (369) of the call at that PC.
    #[cfg(feature="runtime")]
    blobs: BTreeMap<u32,(String,std::sync::Arc<[u8]>,i32)>,
    #[cfg(feature="runtime")]
    blob_pending: Option<(u32,BlobPending)>,
    scene_loading: Option<(u32,crysirror_tasks::Task<Option<(Vec<u8>,std::sync::Arc<crate::scene::Scene>)>>)>,
    split_loading: Option<Task<crysirror_text::byte_split::ByteSplitOutput>>,
    save_loading: Option<Task<Option<crate::native_save::Container>>>,
    saves: BTreeMap<u32,crate::native_save::Container>,
    stores: crate::native_store::Stores,
    ticks: u64,
    /// Scripts waiting in native 69, and the script they called.
    script_calls: BTreeMap<u32,u32>,
    script_memory: Vec<(u32,u32,Vec<u8>)>,
    main_task: u32,
    list_shadows: BTreeMap<u32,u64>,
    open_files: BTreeMap<u32,(Vec<u8>,usize)>,
    next_file: u32,
    image_tags: BTreeMap<u32,u32>,
    #[cfg(feature="runtime")]
    surface: crate::native_surface::PendingCall,
    #[cfg(feature="runtime")]
    movies:crate::native_movie::Movies,
    save_writing: Option<Task<Option<String>>>,
    next_save:u32,
    string_variables:crate::native_values::StringVariables,
    sound_loading: Option<Task<crate::sound_definition::SoundDefinition>>,
    sound_definition: Option<crate::sound_definition::SoundDefinition>,
    input_state: crate::native_input::InputState,
    /// Original B9BD9C manager is absent before its initialization command.
    /// The engine owns any SDK; this option preserves native creation policy.
    steam_manager: crate::native_platform::SteamManager,
    fes_file_probe: Option<crate::fes_file_probe::Pending>,
    fes_civil_time: Option<crate::fes_civil_time::Pending>,
    fes_children:crate::fes_children::Children,
    fes_tick_input: Option<(u32,u32,crysirror_core::service::InputObservation)>,
    fes_owner_tick:Option<FesOwnerTick>,
    last_fes_owner_revision:Option<u64>,
    classes:crate::native_classes::State,
}
struct FesOwnerTick {
    tasks:Vec<u32>,
    next:usize,
    observation:crysirror_core::service::InputObservation,
    origin_pc:u32,
}
#[derive(Clone,Debug)]
struct NativeEventWait{slot:u32,pc:u32}
/// A running "KCS Script" task. Its fields trade places with the adapter's while it steps.
struct KcsChild{id:u32,script:String,program:Option<Program>,machine:Option<Machine>,event_wait:Option<NativeEventWait>,finished:bool}
struct PreparedLoops { entries: Option<Vec<crysirror_core::service::AudioLoopMetadata>>, source: String }
struct Prepared {
    program: Program,
    menu: Option<crysirror_scene::ui::menu::MenuSpec>,
    native_abi: bool,
    null_view_constructor: bool,
    debug_key_absent: bool,
    sound: Option<crate::sound_definition::SoundDefinition>,
}
impl CatSystem2Vm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let mut fes_children=crate::fes_children::Children::default();
        #[cfg(feature="runtime")]
        fes_children.motions.set_explicit_seeds(config.random_seeds)?;
        Ok(Self {
            client,
            config,
            pool: None,
            loading: None,
            program: None,
            machine: None,
            error: None,
            native_abi: false,
            null_view_constructor: false,
            abi_warned: false,
            input_routes_bound:false,
            event_wait:None,
            kcs_children: Vec::new(),
            kcs_starts: Default::default(),
            kcs_loading_task: None,
            string_maps:Default::default(),
            kcs_loading:None,
            image_pool_loading:None,
            fes_loading:None,
            texture_cache:None,texture_clear:None,
            debug_key_absent: false,
            menu_installed: false,
            menu_state: Default::default(),
            tasks: crate::task_policy::NativeTaskPolicy::bootstrap()?,
            xml_loading: None,
            loop_loading: None,
            settings_loading: None,
            settings: None,
            loop_catalog_revision: 0,
            loop_entries: 0,
            documents: BTreeMap::new(),
            next_document: 1,
            document_bytes: 0,
            native_calls: BTreeMap::new(),
            interpreter: None,
            handles: Default::default(),
            #[cfg(feature="runtime")]
            m2: Default::default(),
            classes: Default::default(),
            #[cfg(feature="runtime")]
            kcs_sound: Default::default(),
            motion_loading: None,
            #[cfg(feature="runtime")]
            image_loading: Default::default(),
            #[cfg(feature="runtime")]
            layered_loading: None,
            #[cfg(feature="runtime")]
            blobs: BTreeMap::new(),
            #[cfg(feature="runtime")]
            blob_pending: None,
            scene_loading: None,
            split_loading: None,
            save_loading:None,
            saves:BTreeMap::new(),
            stores:Default::default(),
            ticks:0,
            script_calls:BTreeMap::new(),
            script_memory:Vec::new(),
            main_task:0,
            list_shadows:BTreeMap::new(),
            open_files:BTreeMap::new(),
            next_file:0,
            image_tags:BTreeMap::new(),
            #[cfg(feature="runtime")]
            surface:Default::default(),
            #[cfg(feature="runtime")]
            movies:Default::default(),
            save_writing:None,
            next_save:1,
            string_variables:Default::default(),
            sound_loading: None,
            sound_definition: None,
            input_state: Default::default(),
            steam_manager: Default::default(),
            fes_file_probe: None,
            fes_civil_time: None,
            fes_children,
            fes_tick_input: None,
            fes_owner_tick:None,
            last_fes_owner_revision:None,
        })
    }
    fn advance_fes_tasks(&mut self,host:&mut dyn HostServices,budget:&mut u32)->Result<bool>{
        //A synchronous forced333/448 dependency cannot reenter the ordinary
        //child phase while its original owner/input observation is retained.
        if self.fes_tick_input.is_some(){return Ok(true);}
        if self.fes_owner_tick.is_none(){
            let scripts=self.kcs_children.iter().filter(|child|!child.finished).map(|child|child.id).collect::<Vec<_>>();
            let tasks=self.tasks.running_fes_tasks_under(&scripts)?;
            if tasks.is_empty(){
                #[cfg(feature="runtime")]
                if self.fes_children.frames.audio_needs_updates(){
                    let observation=host.input_observation(self.client)?;
                    self.fes_children.frames.observe_audio(host,self.client,observation)?;
                }
                return Ok(true);
            }
            let observation=host.input_observation(self.client)?;
            ensure!(observation.client==self.client,"CatSystem2 foreign task-phase input observation");
            if self.last_fes_owner_revision==Some(observation.revision){return Ok(true);}
            let origin_pc=self.machine.as_ref().context("CatSystem2 main task missing")?.pc;
            self.fes_owner_tick=Some(FesOwnerTick{tasks,next:0,observation,origin_pc});
        }
        loop{
            let phase=self.fes_owner_tick.as_ref().unwrap();
            #[cfg(feature="runtime")]
            self.fes_children.frames.observe_audio(host,self.client,phase.observation)?;
            if phase.next==phase.tasks.len(){
                self.last_fes_owner_revision=Some(phase.observation.revision);self.fes_owner_tick=None;return Ok(true);
            }
            if *budget==0{return Ok(false);}
            let (id,observation,origin_pc)=(phase.tasks[phase.next],phase.observation,phase.origin_pc);
            // A KCS child's FES runs with that task current, as when the child forces it.
            let main=self.tasks.current();
            let script=self.tasks.fes_script_owner(id);
            let result=self.advance_fes_task(host,id,observation,origin_pc,script,budget);
            self.tasks.set_current(main);
            match result?{Some(false)=>return Ok(false),_=>{self.fes_owner_tick.as_mut().unwrap().next+=1;}}
        }
    }
    /// `Some(false)` when the phase must resume this task next tick.
    fn advance_fes_task(&mut self,host:&mut dyn HostServices,id:u32,observation:crysirror_core::service::InputObservation,origin_pc:u32,script:u32,budget:&mut u32)->Result<Option<bool>>{
        if self.tasks.exists(script){self.tasks.set_current(script);}
            if self.tasks.fes(id)?.deletion_pending {
                let mut protected=self.fes_children.pending_owners();
                if let Some(p)=&self.fes_loading{protected.push(p.task_id);}
                if let Some(p)=&self.fes_file_probe{protected.push(p.owner);}
                if let Some(p)=&self.fes_civil_time{protected.push(p.owner);}
                self.tasks.retire_fes_subtree(id,host,self.client,self.interpreter.as_mut(),&protected)?;
                #[cfg(feature="runtime")]
                self.fes_children.frames.reap(host,self.client,&self.tasks)?;
                #[cfg(feature="runtime")]
                self.fes_children.panels.reap(&self.tasks);
                #[cfg(feature="runtime")]
                self.fes_children.panel_render.reap(&self.tasks);
                #[cfg(feature="runtime")]
                self.fes_children.motions.reap(&self.tasks);
                return Ok(Some(true));
            }
            let tick=crate::fes_runtime::Services{pointer_glide:Some(&mut self.classes.input_task),
                string_variables:&mut self.string_variables,
                client:self.client,tasks:&mut self.tasks,
                interpreter:self.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?,
                settings:&mut self.settings,sound_definition:&mut self.sound_definition,
                input_state:&mut self.input_state,steam_manager:&mut self.steam_manager,
                fes_file_probe:&mut self.fes_file_probe,fes_civil_time:&mut self.fes_civil_time,fes_children:&mut self.fes_children,handles:Some(&self.handles),saves:&self.saves,
                pool:self.pool.as_ref().context("CatSystem2 worker pool missing")?,
            }.run(host,id,origin_pc,observation,budget)?;
            if self.fes_children.routes_published(){self.input_routes_bound=true;}
            match tick{
                crate::fes_runtime::Tick::Waited=>Ok(Some(true)),
                crate::fes_runtime::Tick::Budget|crate::fes_runtime::Tick::Dependency=>Ok(Some(false)),
            }
    }
    fn run(&mut self, host: &mut dyn HostServices, budget: u32) -> Result<VmYield> {
        self.tasks.flush_retired_images(host,self.client)?;
        if self.program.is_none() {
            if self.loading.is_none() {
                let storage = host.storage(self.client)?;
                let path = VfsPath::new(Mount::Asset, &self.config.script)?;
                let config = self.config.clone();
                let pool = TaskPool::new("catsystem2-source", 1, 1)?;
                self.loading = Some(pool.spawn(move |cancel| {
                    cancel.check()?;
                    let reader = storage.open_asset(&path)?;
                    let bytes = crysirror_resource::read_all(
                        reader.as_ref(),
                        crate::kcs::MAX_SCRIPT_BYTES as u64,
                    )?;
                    cancel.check()?;
                    let program=Program::parse(&bytes)?;
                    let mut menu=None;
                    let mut native_abi=false;
                    let mut null_view_constructor=false;
                    let mut debug_key_absent=false;
                    if let Some(module)=config.executable.as_ref() {
                        let module=storage.open_asset(&VfsPath::new(Mount::Asset,module)?)?;
                        let module=crysirror_resource::read_all(module.as_ref(),crysirror_resource::pe::MAX_PE_BYTES as u64)?;
                        cancel.check()?;
                        let pe=crysirror_resource::pe::PeData::parse(&module)?;
                        null_view_constructor=crate::native_classes::view3d::has_null_constructor(&pe);
                        // This is a verified native ABI fingerprint, never a game
                        // script/PC shortcut. Generic KCS parsing remains unpinned.
                        native_abi=pe.sha256()==[0xf8,0xa1,0xfb,0x4f,0xed,0x8a,0x0c,0x64,0xc3,0xfb,0xc0,0xbc,0xf0,0x09,0xf6,0x3e,0x8f,0x31,0x81,0xcf,0xbc,0x1a,0x0d,0xbd,0x60,0xf9,0xdd,0xef,0xc8,0xe2,0x5b,0x3d];
                        debug_key_absent=storage.metadata(Mount::Asset,"cs2_debug_key.dat").context("CatSystem2 optional debug-key metadata")?.is_none();
                        ensure!(debug_key_absent,"CatSystem2 present debug key requires unsupported native validation");
                        if native_abi {
                            let resources=pe.menu_templates()?;
                            menu=Some(crate::native_menu::build(crate::native_menu::production(&resources)?,config.canvas,&config.font)?);
                        }
                    }
                    // Older scripts (Grisaia) have no native 938: FES sounds still need their groups.
                    let sound=crate::sound_definition::load(storage.as_ref(),&||cancel.check()).ok();
                    Ok(Prepared{program,menu,native_abi,null_view_constructor,debug_key_absent,sound})
                })?);
                self.pool = Some(pool);
                return Ok(VmYield::BudgetExhausted);
            }
            let Some(prepared) = self.loading.as_mut().unwrap().try_take()? else {
                return Ok(VmYield::BudgetExhausted);
            };
            if let Some(menu) = prepared.menu {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(&json!({"ui":[{"op":"set_menu","menu":menu}]}))?,
                )?;
                self.menu_installed = true;
            }
            self.native_abi = prepared.native_abi;
            self.null_view_constructor = prepared.null_view_constructor;
            self.sound_definition = prepared.sound;
            self.debug_key_absent = prepared.debug_key_absent;
            self.machine = Some(Machine::new(&prepared.program, self.config.heap_bytes)?);
            self.program = Some(prepared.program);
            self.loading = None;
            // Runtime's startup fence may poll only dormant adapters. Source
            // publication must not also execute the first KCS instructions;
            // they belong to the first ordinary owner tick and its input.
            return Ok(VmYield::BudgetExhausted);
        }
        if self.menu_installed {
            let events: Vec<crysirror_scene::ui::UiEvent> =
                serde_json::from_slice(&host.query_engine(self.client, EngineQuery::MenuEvents)?)?;
            for event in events {
                if event.menu_bar == Some(crate::native_menu::PRODUCTION_MENU) {
                    self.menu_state.enqueue(event.widget.id)?;
                }
            }
            if self.menu_state.tick(host, self.client, self.pool.as_ref().context("CatSystem2 menu worker pool missing")?, self.config.canvas, &self.config.font, self.config.executable.as_deref())? {
                return Ok(VmYield::BudgetExhausted);
            }
        }
        self.ticks+=1;
        self.main_task=self.tasks.current();
        //Original synchronous native117 cannot reenter InputTask callbacks.
        //Its asynchronous engine dependency retains input in the shared journal.
        if self.input_routes_bound && self.texture_clear.is_none() && self.fes_file_probe.is_none() && self.fes_owner_tick.is_none() {
            // Drain even while a KCS task owns a forced FES tick: unarmed deliveries only
            // acknowledge, and a queued sample would otherwise block that tick's route puts.
            let mut drained=false;
            let mut completed_delivery=false;
            for _ in 0..16 {
            let delivery:Option<crysirror_scene::input_routes::InputDelivery>=serde_json::from_slice(&host.query_engine(self.client,EngineQuery::InputRouteDelivery)?)?;
            if let Some(delivery)=delivery {
                // The script destroyed this click controller after the sample was routed.
                if self.tasks.button(delivery.receiver).is_err(){
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":false}]}))?)?;
                    continue;
                }
                let hover=matches!(delivery.event.phase,crysirror_scene::input_routes::InputRoutePhase::HoverEnter|crysirror_scene::input_routes::InputRoutePhase::HoverLeave);
                // 5E7A30 records the offering source in INPUT this[1238] (native 109 reads it).
                if matches!(delivery.event.phase,crysirror_scene::input_routes::InputRoutePhase::Press|crysirror_scene::input_routes::InputRoutePhase::Release){
                    use crysirror_scene::input_routes::InputRouteSource as S;
                    self.input_state.last_source=match delivery.event.source{S::Keyboard=>1,S::Pointer=>2,S::Controller=>4};
                }
                let fes_hover=if hover&&self.tasks.button(delivery.receiver)?.binding_target!=Some(self.tasks.current()) {
                    let needs_input=self.tasks.button(delivery.receiver)?.binding_target.is_some_and(|id|self.tasks.fes(id).is_ok_and(|task|task.player_loaded&&!task.paused&&!task.deletion_pending));
                    if needs_input{let observation=host.input_observation(self.client)?;ensure!(observation.client==self.client,"CatSystem2 foreign hover input observation");self.input_state.observe(&observation)?;}
                    self.tasks.empty_fes_hover(delivery.receiver,&self.input_state,self.interpreter.as_mut().context("CatSystem2 FES hover interpreter unavailable")?)?
                }else{false};
                if fes_hover {
                    //Native64's no-object path returns its unchanged0 result;
                    //it neither fabricates a script callback nor consumes a
                    //lower-priority owner's physical event as handled.
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":false}]}))?)?;
                }else if self.event_wait.is_none()&&hover&&self.tasks.button(delivery.receiver)?.binding_target==Some(self.tasks.current()) {
                    //5DE8E0 7/8→Main64→877310: no VM callback while
                    //native event-wait flag16 is zero. Only71/65A180 calls
                    //877390 to arm it;the concrete gate is checked above.
                    //An unarmed hover does not execute a script callback.
                    // This concrete native gate is unarmed, including after a
                    // no-event owner tick. No script callback can be resumed.
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":true}]}))?)?;
                }else if self.tasks.button(delivery.receiver)?.binding_target.is_some_and(|id|self.tasks.fes(id).is_ok()) {
                    //A FES-bound low button: 629850 polls its object buttons from
                    //the shared pointer state, so the physical event stays unhandled here.
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":false}]}))?)?;
                }else if let Some(target)=self.tasks.button(delivery.receiver)?.binding_target.filter(|id|self.kcs_children.iter().any(|child|child.id==*id)) {
                    //A button a KCS task bound reaches that task's own native71.
                    let bindings=&self.settings.as_ref().context("CatSystem2 key bindings unavailable")?.input;
                    for message in crate::native_input::script_messages(delivery.receiver,&delivery.event,bindings){self.tasks.queue_script_event(target,message);}
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":true}]}))?)?;
                }else if let (Some(wait),Some(message))=(self.event_wait.as_ref().map(|w|w.slot),crate::native_input::main_event_message(delivery.receiver,&delivery.event)) {
                    //5DE8E0 → 5E8B36 → 877310: the 16-byte message reaches the
                    //armed native71 slot and the script resumes with true.
                    self.machine.as_mut().context("CatSystem2 machine missing")?.resume_native_event(Some((wait,message)))?;
                    self.event_wait=None;
                    // 5DE8E0: the button answers its last native108 result; 0 passes the
                    // event on to lower buttons, such as the running KCS task's own.
                    let handled=self.tasks.button(delivery.receiver)?.result!=0;
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":handled}]}))?)?;
                    completed_delivery=handled;
                    // A handled non-drag event retires its route plan at ACK;
                    // the resumed script can now safely update receivers.
                    if handled{drained=true;break;}
                }else{
                    //877310 returns 0 when no native71 is armed; events the original
                    //InputTask has no message for are not VM callbacks either.
                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"acknowledge","receipt":delivery.receipt,"handled":false}]}))?)?;
                }
            }else{drained=true;break;}
            }
            // Resuming native71 does not finish an unhandled event: lower
            // receivers still own the remaining dispatch plan. If the bounded
            // drain runs out, retain that resumed VM until the next drain rather
            // than letting native448 mutate the registry inside the old plan.
            if !drained{return Ok(VmYield::BudgetExhausted);}
            let stats:crysirror_scene::input_routes::InputRouteStats=serde_json::from_slice(&host.query_engine(self.client,EngineQuery::InputRouteStats)?)?;
            // A resumed but unhandled native71 still belongs to the old
            // sample. poll(None) can also mean bounded work or UI suspension;
            // its retained journal must drain before registry mutation.
            if stats.leased||(!completed_delivery&&(stats.queued_samples>0||stats.queued_events>0)){return Ok(VmYield::BudgetExhausted);}
        }
        if self.event_wait.is_some()&&self.fes_owner_tick.is_none(){
            let mut proposed=self.tasks.clone();
            let receivers=proposed.initialize_fes_tasks(self.config.canvas)?;
            if !receivers.is_empty(){
                let commands=receivers.into_iter().map(|receiver|json!({"op":"put","receiver":receiver})).collect::<Vec<_>>();
                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":commands}))?)?;
                self.input_routes_bound=true;
            }
            self.tasks=proposed;
        }
        let mut remaining = budget;
        if !self.advance_fes_tasks(host,&mut remaining)?{return Ok(VmYield::BudgetExhausted);}
        self.pump_kcs_children(host)?;
        #[cfg(feature="runtime")]
        {
            let pc=self.machine.as_ref().map_or(0,|machine|machine.pc);
            if !self.tick_m2(host,pc)? { return Ok(VmYield::BudgetExhausted); }
        }
        {let now=host.now_ms();crate::native_classes::tick(host,self.client,&mut self.tasks,&mut self.classes,now)?;}
        #[cfg(feature="runtime")]
        {let now=host.now_ms();crate::native_classes::particles::tick(host,self.client,&self.tasks,self.config.canvas,&mut self.classes.particles,self.fes_children.motions.random().ok(),now)?;}
        let main=self.run_script(host,remaining)?;
        self.run_kcs_child(host,budget)?;
        #[cfg(feature="runtime")]
        {
            // Publish poses and commands from this script slice before the host
            // renders it. The model clock has already advanced at this time;
            // unchanged players are skipped by their owner update stamp.
            let pc=self.machine.as_ref().map_or(0,|machine|machine.pc);
            if !self.tick_m2(host,pc)? { return Ok(VmYield::BudgetExhausted); }
        }
        Ok(main)
    }
    #[cfg(feature="runtime")]
    fn tick_m2(&mut self,host:&mut dyn HostServices,pc:u32)->Result<bool>{
        if self.m2.needs_random()&&!self.fes_children.motions.ensure_random_for_kcs(host,self.client,self.tasks.current(),pc)? { return Ok(false); }
        let now=host.now_ms();
        let config=self.settings.as_ref().map(|s|s.m2.clone()).unwrap_or_default();
        self.m2.tick(host,self.client,&self.tasks,&self.kcs_sound,&config,self.config.canvas,now,self.fes_children.motions.random().ok())?;
        Ok(true)
    }
    fn swap_kcs_child(&mut self,child:&mut KcsChild){
        std::mem::swap(&mut self.config.script,&mut child.script);
        std::mem::swap(&mut self.program,&mut child.program);
        // 5DD580 class12: every KCS task shares the owner's one script heap (this+112/116).
        if let (Some(owner),Some(other))=(self.machine.as_mut(),child.machine.as_mut()){std::mem::swap(&mut owner.heap,&mut other.heap);}
        std::mem::swap(&mut self.machine,&mut child.machine);
        std::mem::swap(&mut self.event_wait,&mut child.event_wait);
    }
    /// 629850 cases 4 and 5: start a requested KCS task, drop an ended one.
    fn pump_kcs_children(&mut self,host:&mut dyn HostServices)->Result<()>{
        for retired in self.tasks.take_retired_kcs(){
            #[cfg(feature="runtime")]
            {
                self.image_loading.remove(&retired);
                self.surface.cancel(host,self.client,retired)?;
            }
            self.kcs_children.retain(|child|child.id!=retired);
            self.tasks.destroy_task_slot(retired)?;
        }
        if self.kcs_loading.is_none(){
            // Requests stay queued until a worker is free.
            let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(()),Err(e)=>return Err(e)};
            let request=self.tasks.take_kcs_request().map(|(owner,name)|(owner,name,None)).or_else(||self.kcs_starts.pop_front().map(|(id,name)|(0,name,Some(id))));
            if let Some((owner,name,existing))=request{
                self.kcs_loading_task=existing;
                let name=String::from_utf8(name).context("CatSystem2 execkcs name encoding")?;
                let dir=self.config.script.rsplit_once('/').map_or("",|(dir,_)|dir);
                let path=if dir.is_empty(){format!("{name}.kcs")}else{format!("{dir}/{name}.kcs")};
                let storage=host.storage(self.client)?;
                let vfs=VfsPath::new(Mount::Asset,&path)?;
                let task=permit.spawn(move|cancel|{
                    cancel.check()?;
                    let reader=storage.open_asset(&vfs)?;
                    let bytes=crysirror_resource::read_all(reader.as_ref(),crate::kcs::MAX_SCRIPT_BYTES as u64)?;
                    cancel.check()?;
                    Program::parse(&bytes)
                })?;
                self.kcs_loading=Some((owner,path,task));
            }
        }
        if let Some((owner,path,task))=self.kcs_loading.as_mut(){
            if let Some(program)=task.try_take()?{
                let owner=*owner;let script=path.clone();
                self.kcs_loading=None;
                let machine=Machine::new(&program,self.config.heap_bytes)?;
                let id=match self.kcs_loading_task.take(){Some(id)=>id,None=>self.tasks.create_kcs_child(owner)?};
                self.kcs_children.push(KcsChild{id,script,program:Some(program),machine:Some(machine),event_wait:None,finished:false});
            }
        }
        Ok(())
    }
    fn run_kcs_child(&mut self,host:&mut dyn HostServices,budget:u32)->Result<()>{
        let mut index=0;
        while index<self.kcs_children.len(){
        let mut child=self.kcs_children.remove(index);
        if child.finished{self.kcs_children.insert(index,child);index+=1;continue;}
        let main=self.tasks.current();
        self.tasks.set_current(child.id);
        self.swap_kcs_child(&mut child);
        let result=self.run_script(host,budget.max(4_000_000));
        let result=match result{Ok(v)=>Ok(v),Err(e)=>{let pending=self.machine.as_ref().and_then(|m|m.pending.as_ref()).map(|c|format!("KCS child native {} args {:x?} PC {:#x}",c.id,c.arguments,c.pc)).unwrap_or_else(||"KCS child".to_owned());Err(e.context(pending))}};
        self.swap_kcs_child(&mut child);
        self.tasks.set_current(main);
        child.finished=matches!(result?,VmYield::Halted);
        self.kcs_children.insert(index,child);
        index+=1;
        }
        Ok(())
    }
    fn run_script(&mut self,host:&mut dyn HostServices,mut remaining:u32)->Result<VmYield>{
        if let Some(wait)=self.event_wait.as_ref().map(|w|w.slot){
            //Original child phase precedes Main's native71 continuation.
            let event=self.tasks.take_script_event(self.tasks.current()).map(|message|(wait,message));
            self.machine.as_mut().context("CatSystem2 machine missing")?.resume_native_event(event)?;
            self.event_wait=None;
        }
        let machine = self
            .machine
            .as_mut()
            .context("CatSystem2 machine missing")?;
        loop {
            let before = machine.instructions;
            let event = machine.step(self.program.as_ref().unwrap(), remaining)?;
            remaining = remaining
                .saturating_sub((machine.instructions - before).min(u64::from(u32::MAX)) as u32);
            match event {
                Event::Halted => return Ok(VmYield::Halted),
                Event::BudgetExhausted => return Ok(VmYield::BudgetExhausted),
                Event::Native(call) => {
                    // A module whose fingerprint was not reviewed runs the same table on trust;
                    // anything it does differently shows up as an unsupported call below.
                    if !self.native_abi&&!self.abi_warned{
                        self.abi_warned=true;
                        host.append_log(self.client,"CatSystem2 module ABI is not a reviewed one; natives follow the reviewed table")?;
                    }
                    // SendMessage to an absent task has no receiver (5DCC80).
                    if let Some((_,value))=crate::native_messages::forwarding(call.id){
                        if call.arguments.first().is_none_or(|id|!self.tasks.exists(*id)){
                            machine.acknowledge(value.then_some(0))?;
                            *self.native_calls.entry(call.id).or_default()+=1;
                            continue;
                        }
                    }
                    // The same messages sent to a task of another class, or to none, change nothing.
                    let misdirected=match call.id{
                        104|105|108|848|849=>self.tasks.button(call.arguments[0]).is_err(),
                        256|260|262=>self.tasks.image_list(call.arguments[0]).is_err(),
                        _=>false,
                    };
                    if misdirected{machine.acknowledge(None)?;continue;}
                    if matches!(call.id,447|449|450|451)&&self.tasks.fes(call.arguments[0]).is_err(){
                        // Messages 158/159/157: FES-only cases; every other class ignores them.
                        machine.acknowledge((call.id==450).then_some(0))?;
                        continue;
                    }
                    if let Some(result)=crate::native_misc::null_handle(call.id,&call.arguments){
                        machine.preflight_native(result.is_some(),&[])?;
                        machine.acknowledge(result)?;
                        *self.native_calls.entry(call.id).or_default()+=1;
                        continue;
                    }
                    match call.id {
                        id if crate::native_classes::claims(id) => {
                            // Particle scripts draw from the session random streams (B93150).
                            if matches!(id,271|693|965)&&!self.fes_children.motions.ensure_random_for_kcs(host,self.client,self.tasks.current(),call.pc)?{return Ok(VmYield::BudgetExhausted);}
                            let mut cx=crate::native_classes::Ctx{strings:&mut self.string_variables,machine:&mut *machine,host:&mut *host,client:self.client,tasks:&mut self.tasks,state:&mut self.classes,canvas:self.config.canvas,call:&call,pool:self.pool.as_ref(),settings:self.settings.as_mut(),handles:&mut self.handles,interpreter:self.interpreter.as_mut(),saves:&mut self.saves,next_save:&mut self.next_save,font:&self.config.font,input:&mut self.input_state,ticks:self.ticks,
                                #[cfg(feature="runtime")]m2:&mut self.m2,#[cfg(feature="runtime")]movies:&mut self.movies,#[cfg(feature="runtime")]kcs_sound:&mut self.kcs_sound};
                            if !crate::native_classes::dispatch(&mut cx)?{return Ok(VmYield::BudgetExhausted);}
                            if crate::native_classes::take_halt(&mut self.classes){return Ok(VmYield::Halted);}
                        }
                        1 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 debug log arguments");
                            machine.preflight_native(false,&[])?;
                            // 650400 formats VM substitutions, then escapes its
                            // percent bytes for 5CAC00's500-byte printf sink.
                            // The engine log accepts literal text, never printf.
                            let mut bytes=crate::native_values::format_bytes(machine,call.arguments[0])?;
                            ensure!(!bytes.windows(2).any(|w|w==b"%%"),"CatSystem2 unsupported consecutive-percent debug format");
                            ensure!(bytes.len()+bytes.iter().filter(|b|**b==b'%').count()<2047,"CatSystem2 debug format buffer extent");
                            bytes.truncate(500);
                            let text=crysirror_resource::text_stream::decode_auto(&bytes,500)?;
                            host.append_log(self.client,&text)?;
                            machine.acknowledge(None)?;
                        }
                        108 => {
                            // 648950 → 5DE8E0 case 94: the button's pending
                            // callback result returned for its next key message.
                            ensure!(call.arguments.len()==2,"CatSystem2 button result arguments");
                            machine.preflight_native(false,&[])?;
                            self.tasks.set_button_result(call.arguments[0],call.arguments[1])?;
                            machine.acknowledge(None)?;
                        }
                        318 => {
                            // 659170: clamped linear interpolation (from, to, start, end, now).
                            ensure!(call.arguments.len()==5,"CatSystem2 interpolation arguments");
                            machine.preflight_native(true,&[])?;
                            let [from,to,start,end,now]=[0,1,2,3,4].map(|i|f32::from_bits(call.arguments[i]));
                            let value=if start>=now{from}else if now<end{(now-start)/(end-start)*(to-from)+from}else{to};
                            machine.acknowledge(Some(value.to_bits()))?;
                        }
                        319 => {
                            // 65B280: floor(x + 0.5) as a truncating integer conversion.
                            ensure!(call.arguments.len()==1,"CatSystem2 round arguments");
                            machine.preflight_native(true,&[])?;
                            machine.acknowledge(Some((f32::from_bits(call.arguments[0])+0.5).floor() as i32 as u32))?;
                        }
                        323 => {
                            ensure!(call.arguments.is_empty(),"CatSystem2 scene constructor arguments");
                            machine.preflight_native(true,&[])?;
                            let id=self.handles.allocate()?;
                            self.handles.scenes.insert(id,Default::default());
                            machine.acknowledge(Some(id))?;
                        }
                        324 => {
                            machine.preflight_native(false,&[])?;
                            self.handles.scenes.remove(&call.arguments[0]);
                            machine.acknowledge(None)?;
                        }
                        325 => {
                            // 642100 → 72B1B0: read and decode a scene; a missing file answers 0.
                            ensure!(call.arguments.len()==2,"CatSystem2 scene load arguments");
                            machine.preflight_native(true,&[])?;
                            let id=call.arguments[0];
                            if !self.handles.scenes.contains_key(&id){machine.acknowledge(Some(0))?;continue;}
                            if self.scene_loading.is_none(){
                                let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                                let stem=crate::native_scene::stem(&name).to_vec();
                                let path=format!("scene.int/{}.cst",String::from_utf8_lossy(&stem));
                                let storage=host.storage(self.client)?;
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let task=permit.spawn(move|cancel|{
                                    cancel.check()?;
                                    let vfs=VfsPath::new(Mount::Asset,&path)?;
                                    let Some(vfs)=crate::native_resource_paths::resolve(storage.as_ref(),&vfs)? else{return Ok(None);};
                                    let reader=storage.open_asset(&vfs)?;
                                    let bytes=crysirror_resource::read_all(reader.as_ref(),crate::kcs::MAX_SCRIPT_BYTES as u64)?;
                                    let scene=crate::scene::Scene::parse(&bytes).with_context(||format!("CatSystem2 scene {path}"))?;
                                    Ok(Some((stem,std::sync::Arc::new(scene))))
                                })?;
                                self.scene_loading=Some((id,task));
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(loaded)=self.scene_loading.as_mut().unwrap().1.try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            self.scene_loading=None;
                            let scene=self.handles.scenes.get_mut(&id).unwrap();
                            let found=loaded.is_some();
                            if let Some((name,decoded))=loaded{scene.name=name;scene.scene=Some(decoded);}
                            machine.acknowledge(Some(u32::from(found)))?;
                        }
                        326 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 scene query arguments");
                            machine.preflight_native(true,&[])?;
                            let value=self.handles.scenes.get(&call.arguments[0]).map_or(0,|s|s.query(call.arguments[1] as i32,call.arguments[2] as i32));
                            machine.acknowledge(Some(value as u32))?;
                        }
                        327|1037 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 scene text arguments");
                            let text=self.handles.scenes.get(&call.arguments[0]).and_then(|s|s.text(call.arguments[2] as i32,call.arguments[3] as i32));
                            match text{
                                Some(mut text) if call.arguments[1]!=0=>{let len=text.len() as u32;text.push(0);machine.commit_native(Some(len),&[(call.arguments[1],text)])?;}
                                _=>machine.acknowledge(Some(0))?,
                            }
                        }
                        338 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 scene name arguments");
                            let mut name=self.handles.scenes.get(&call.arguments[0]).map_or(Vec::new(),|s|s.name.clone());name.push(0);
                            machine.commit_native(Some(0),&[(call.arguments[1],name)])?;
                        }
                        328|329|332|934 => {
                            machine.preflight_native(!matches!(call.id,934),&[])?;
                            let a=call.arguments.clone();
                            let first=crate::native_values::format_bytes(machine,a[1])?;
                            let first=crate::native_scene::stem(&first).to_vec();
                            let second=if call.id==934{crate::native_values::format_bytes(machine,a[2])?}else{Vec::new()};
                            let result=match self.handles.scenes.get_mut(&a[0]){
                                None=>if call.id==329{u32::MAX}else{0},
                                Some(scene)=>match call.id{
                                    328=>{if !first.is_empty(){scene.insert(&first,a[2]);}1}
                                    329=>if first.is_empty(){u32::MAX}else{scene.visit(&first,a[2])},
                                    332=>scene.value(&first),
                                    _=>{scene.alias(&first,crate::native_scene::stem(&second));0}
                                }
                            };
                            machine.acknowledge((call.id!=934).then_some(result))?;
                        }
                        // 642460: every "$str<n>" naming an existing string variable attaches that
                        // variable's text to name entry a1 under (a2, n) (72D7C0); the scan
                        // resumes one byte past the number's digits.
                        336 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 scene attribute arguments");
                            machine.preflight_native(false,&[])?;
                            let a=call.arguments.clone();
                            if self.handles.scenes.contains_key(&a[0]){
                                let source=machine.read_cstring(a[3],65534)?;
                                let mut at=0;
                                while at<source.len(){
                                    let found=source[at..].starts_with(b"$str").then(||crate::native_classes::parse_number(&source[at+4..])).flatten();
                                    match found.and_then(|(n,used)|self.string_variables.get(n as u32).map(|text|(n,used,text.to_vec()))){
                                        Some((n,used,text))=>{self.handles.scenes.get_mut(&a[0]).unwrap().set_attribute(a[1],a[2],n as u32,&text);at+=used+1;}
                                        None=>at+=1,
                                    }
                                }
                            }
                            machine.acknowledge(None)?;
                        }
                        // 72CF50 empties the first list, 72CFD0 the second.
                        330|331 => {
                            machine.preflight_native(false,&[])?;
                            if let Some(scene)=self.handles.scenes.get_mut(&call.arguments[0]){if call.id==330{scene.names.clear();}else{scene.history.clear();}}
                            machine.acknowledge(None)?;
                        }
                        333 => {
                            // 72D080: copy the entry at an index, clamping past the end to the last.
                            ensure!(call.arguments.len()==4,"CatSystem2 scene name-list arguments");
                            let names=self.handles.scenes.get(&call.arguments[0]).map_or(Vec::new(),|s|s.history.clone());
                            let index=(call.arguments[1] as usize).min(names.len().saturating_sub(1));
                            let (mut name,value)=names.get(index).map_or((Vec::new(),u32::MAX),|e|(e.name.clone(),e.value));
                            name.push(0);
                            machine.commit_native(Some(if names.is_empty(){u32::MAX}else{index as u32}),&[(call.arguments[2],name),(call.arguments[3],value.to_le_bytes().to_vec())])?;
                        }
                        982 => {
                            machine.preflight_native(true,&[])?;
                            let value=self.handles.scenes.get(&call.arguments[0]).map_or(u32::MAX,|s|s.history.len() as u32);
                            machine.acknowledge(Some(value))?;
                        }
                        863 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 stack arguments");
                            machine.preflight_native(true,&[])?;
                            ensure!(call.arguments[0]<=65536,"CatSystem2 stack element extent");
                            let id=self.handles.allocate()?;
                            self.handles.stacks.insert(id,crate::native_scene::NativeStack{element:call.arguments[0] as usize,bytes:Vec::new()});
                            machine.acknowledge(Some(id))?;
                        }
                        864 => {
                            machine.preflight_native(false,&[])?;
                            self.handles.stacks.remove(&call.arguments[0]);
                            machine.acknowledge(None)?;
                        }
                        865 => {
                            // 644B70: push the element at a VM pointer and answer success.
                            machine.preflight_native(true,&[])?;
                            let result=if let Some(stack)=self.handles.stacks.get_mut(&call.arguments[0]){
                                let bytes=machine.read_bytes(call.arguments[1],stack.element)?;
                                ensure!(stack.bytes.len()+bytes.len()<=16*1024*1024,"CatSystem2 stack byte budget");
                                stack.bytes.extend(bytes);1
                            }else{0};
                            machine.acknowledge(Some(result))?;
                        }
                        866 => {
                            // 644B10: pop into a VM pointer; empty answers 0 and writes nothing.
                            let popped=self.handles.stacks.get_mut(&call.arguments[0]).filter(|s|s.count()>0).map(|stack|{let at=stack.bytes.len()-stack.element;stack.bytes.split_off(at)});
                            match popped{
                                Some(bytes) if call.arguments[1]!=0=>machine.commit_native(Some(1),&[(call.arguments[1],bytes)])?,
                                _=>machine.acknowledge(Some(0))?,
                            }
                        }
                        867 => {
                            machine.preflight_native(true,&[])?;
                            let value=self.handles.stacks.get(&call.arguments[0]).map_or(0,|s|s.count() as u32);
                            machine.acknowledge(Some(value))?;
                        }
                        868 => {
                            machine.preflight_native(false,&[])?;
                            if let Some(source)=self.handles.stacks.get(&call.arguments[1]).cloned(){
                                if let Some(target)=self.handles.stacks.get_mut(&call.arguments[0]){*target=source;}
                            }
                            machine.acknowledge(None)?;
                        }
                        278 => {
                            // 657D10 -> 73FF40: expand $strN references in place.
                            ensure!(call.arguments.len()==1,"CatSystem2 string expansion arguments");
                            let mut bytes=self.string_variables.expand(&machine.read_cstring(call.arguments[0],65534)?,65534)?;
                            bytes.push(0);
                            machine.commit_native(None,&[(call.arguments[0],bytes)])?;
                        }
                        143 => {
                            // 6562E0: messages 160/171 start a named script on a class-12 task.
                            ensure!(call.arguments.len()==3,"CatSystem2 script start arguments");
                            machine.preflight_native(false,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            if self.tasks.exists(call.arguments[0]){self.kcs_starts.push_back((call.arguments[0],name));}
                            machine.acknowledge(None)?;
                        }
                        // 641CC0 -> 72BC80(4): input count, -1 before a scene loaded.
                        608 => {
                            machine.preflight_native(true,&[])?;
                            let value=self.handles.scenes.get(&call.arguments[0]).map_or(0,|s|s.scene.as_ref().map_or(u32::MAX,|d|d.input_ranges.len() as u32));
                            machine.acknowledge(Some(value))?;
                        }
                        // 641AE0 -> 72BD20.
                        980 => {
                            machine.preflight_native(true,&[])?;
                            let value=self.handles.scenes.get(&call.arguments[0]).map_or(0,|s|s.count_kind_20());
                            machine.acknowledge(Some(value))?;
                        }
                        // 641E00 -> 72B660(language): only a loaded .cstl table (+0x130) changes the
                        // text, which this reader, like 1038 below, does not load.
                        1033|1038 => {
                            // 641EB0 -> 72B930: the line in one language; other languages live in .cstl.
                            ensure!(call.arguments.len()==5,"CatSystem2 scene language text arguments");
                            let text=self.handles.scenes.get(&call.arguments[0]).and_then(|s|s.text(call.arguments[2] as i32,call.arguments[3] as i32));
                            match text{
                                Some(mut text) if call.arguments[1]!=0=>{let len=text.len() as u32;text.push(0);machine.commit_native(Some(len),&[(call.arguments[1],text)])?;}
                                _=>machine.acknowledge(Some(0))?,
                            }
                        }
                        392|1002 => {
                            // 655CF0 and 647500 copy through a table this build leaves empty.
                            ensure!(call.arguments.len()==2,"CatSystem2 string copy arguments");
                            let mut bytes=crate::native_values::format_bytes(machine,call.arguments[1])?;bytes.push(0);
                            machine.commit_native((call.id==392).then_some(1),&[(call.arguments[0],bytes)])?;
                        }
                        // 64BB80: space-separated "name[:frame]" images (6A5AB0/6A53B0). The first
                        // that loads is the base; each later one is drawn over it (687370) at
                        // its own canvas place (bitmap +0x24/+0x28), clipped to both images.
                        // The result joins VM+152's owned images; nothing loaded answers 0.
                        #[cfg(feature="runtime")]
                        111 => {
                            use crysirror_core::image_processing::ImageProcessState as State;
                            ensure!(call.arguments.len()==1,"CatSystem2 layered image arguments");
                            machine.preflight_native(true,&[])?;
                            if self.layered_loading.as_ref().is_none_or(|l|l.pc!=call.pc){
                                let list=crate::native_values::format_bytes(machine,call.arguments[0])?;
                                let mut names=std::collections::VecDeque::new();
                                for token in list.split(|b|*b==b' ').filter(|t|!t.is_empty()){
                                    let (name,frame)=match token.iter().position(|b|*b==b':'){
                                        Some(at)=>(&token[..at],crate::native_values::integer(&String::from_utf8_lossy(&token[at+1..])) as u32),
                                        None=>(token,0),
                                    };
                                    names.push_back((String::from_utf8(crate::native_scene::stem(name).to_vec()).context("CatSystem2 image name encoding")?,frame));
                                }
                                self.layered_loading=Some(LayeredLoad{pc:call.pc,names,frames:Vec::new(),loading:None,job:None});
                            }
                            let load=self.layered_loading.as_mut().unwrap();
                            if let Some(job)=load.job.as_mut(){
                                match job.poll().map_err(anyhow::Error::from)?{
                                    State::Pending=>return Ok(VmYield::BudgetExhausted),
                                    State::Failed(e)=>anyhow::bail!("CatSystem2 layered image composition: {e}"),
                                    State::Ready(image)=>{
                                        let mut frame=load.frames[0].clone();frame.image=image;
                                        self.layered_loading=None;
                                        let id=self.handles.allocate()?;self.handles.images.insert(id,frame);
                                        machine.acknowledge(Some(id))?;continue;
                                    }
                                }
                            }
                            if let Some(pending)=load.loading.as_mut(){
                                let Some(outcome)=pending.poll()? else{return Ok(VmYield::BudgetExhausted);};
                                load.loading=None;
                                if let crate::native_image_resource::Outcome::Ready(frame)=outcome{load.frames.push(frame);}
                            }
                            if let Some((name,frame))=load.names.pop_front(){
                                let Some(pending)=crate::native_image_resource::Pending::begin_named(self.pool.as_ref().context("CatSystem2 worker pool missing")?,host,self.client,self.tasks.current(),call.pc,&name,frame)? else{load.names.push_front((name,frame));return Ok(VmYield::BudgetExhausted);};
                                load.loading=Some(pending);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            if load.frames.is_empty(){self.layered_loading=None;machine.acknowledge(Some(0))?;continue;}
                            if load.frames.len()==1{
                                let frame=load.frames.pop().unwrap();self.layered_loading=None;
                                let id=self.handles.allocate()?;self.handles.images.insert(id,frame);
                                machine.acknowledge(Some(id))?;continue;
                            }
                            let base=&load.frames[0];
                            let mut layers=Vec::new();
                            for layer in &load.frames[1..]{
                                let size=layer.image.size();
                                let blit=crate::native_surface::Blit{destination:0,at:[0,0],size:[size[0] as i32,size[1] as i32],source_at:layer.geometry.origin,math:crate::native_surface::Math::AlphaOver};
                                let Some((src,dst,size))=crate::native_surface::clip(blit,layer,base) else{continue;};
                                layers.push(crysirror_core::image_processing::ImageCompositeLayer{input:Box::new(crysirror_core::image_processing::ImageInput::Resource(layer.image.clone())),source:[src[0],src[1],size[0],size[1]],destination:dst,clip:None,opacity:255,math:crysirror_core::image_processing::ImageCompositeMath::AlphaOnAlphaScalar256});
                            }
                            let input=crysirror_core::image_processing::ImageInput::Composite{base:Box::new(crysirror_core::image_processing::ImageInput::Resource(base.image.clone())),layers};
                            load.job=Some(host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?);
                            return Ok(VmYield::BudgetExhausted);
                        }
                        // 64B010: read an "image" file into a refcounted block {data, size, size, 1, 0}.
                        #[cfg(feature="runtime")]
                        365 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 image block arguments");
                            machine.preflight_native(true,&[])?;
                            if self.blob_pending.as_ref().is_none_or(|(pc,_)|*pc!=call.pc){
                                let name=crate::native_values::format_bytes(machine,call.arguments[0])?;
                                let name=String::from_utf8(name).context("CatSystem2 image block name encoding")?;
                                let storage=host.storage(self.client)?;
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let task=permit.spawn(move|cancel|crate::native_image_resource::read_named(storage.as_ref(),&name,&cancel))?;
                                self.blob_pending=Some((call.pc,BlobPending::Read(task)));
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some((_,BlobPending::Read(task)))=self.blob_pending.as_mut() else{anyhow::bail!("CatSystem2 image block pending state")};
                            let Some(read)=task.try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            self.blob_pending=None;
                            let id=match read{
                                Some((path,bytes)) if !bytes.is_empty()=>{let id=self.handles.allocate()?;self.blobs.insert(id,(path,bytes.into(),1));id}
                                _=>0,
                            };
                            machine.acknowledge(Some(id))?;
                        }
                        // 64B3A0: release one reference; the block goes at zero (its flag is never set).
                        #[cfg(feature="runtime")]
                        366 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 image block release arguments");
                            machine.preflight_native(false,&[])?;
                            if let Some(entry)=self.blobs.get_mut(&call.arguments[0]){
                                entry.2-=1;
                                if entry.2<=0{self.blobs.remove(&call.arguments[0]);}
                            }
                            machine.acknowledge(None)?;
                        }
                        // 64C190 → 6750A0/68F940: decode frame a2 of a block into a new image (VM+152).
                        // 64B9C0 → 68F480: frame a1's 36-byte stdinfo into a VM buffer (6A1E70 layout).
                        #[cfg(feature="runtime")]
                        364|368 => {
                            let a=call.arguments.clone();
                            ensure!(a.len()==if call.id==364{2}else{3},"CatSystem2 image block decode arguments");
                            let out=if call.id==368{a[2]}else{0};
                            machine.preflight_native(true,&if out!=0{vec![(out,vec![0;36])]}else{vec![]})?;
                            let Some((path,bytes,_))=self.blobs.get(&a[0]).cloned() else{machine.acknowledge(Some(0))?;continue;};
                            if call.id==368&&out==0{machine.acknowledge(Some(0))?;continue;}
                            if self.blob_pending.as_ref().is_none_or(|(pc,_)|*pc!=call.pc){
                                let Some(pending)=crate::native_image_resource::Pending::begin_bytes(self.pool.as_ref().context("CatSystem2 worker pool missing")?,host,self.client,self.tasks.current(),call.pc,&path,bytes,a[1])? else{return Ok(VmYield::BudgetExhausted);};
                                self.blob_pending=Some((call.pc,BlobPending::Decode(pending)));
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some((_,BlobPending::Decode(pending)))=self.blob_pending.as_mut() else{anyhow::bail!("CatSystem2 image block pending state")};
                            let Some(outcome)=pending.poll()? else{return Ok(VmYield::BudgetExhausted);};
                            self.blob_pending=None;
                            let frame=match outcome{crate::native_image_resource::Outcome::Ready(frame)=>Some(frame),_=>None};
                            match (call.id,frame){
                                (364,Some(frame))=>{let id=self.handles.allocate()?;self.handles.images.insert(id,frame);machine.acknowledge(Some(id))?;}
                                (368,Some(frame))=>{
                                    let g=&frame.geometry;let base=g.base.unwrap_or_default();
                                    let words=[g.origin[0],g.origin[1],g.size[0] as i32,g.size[1] as i32,g.canvas[0] as i32,g.canvas[1] as i32,(g.color_bits/8) as i32,base[0],base[1]];
                                    machine.commit_native(Some(1),&[(out,words.iter().flat_map(|w|w.to_le_bytes()).collect())])?;
                                }
                                (368,None)=>machine.commit_native(Some(0),&[(out,vec![0;36])])?,
                                _=>machine.acknowledge(Some(0))?,
                            }
                        }
                        // 64B410 → 68BEC0: a 32-bit image's four stored bytes times ARGB a1 / 255.
                        #[cfg(feature="runtime")]
                        369 => {
                            use crysirror_core::image_processing::ImageProcessState as State;
                            ensure!(call.arguments.len()==2,"CatSystem2 image colour multiply arguments");
                            machine.preflight_native(false,&[])?;
                            let a=call.arguments.clone();
                            let Some(frame)=self.handles.images.get(&a[0]).cloned() else{machine.acknowledge(None)?;continue;};
                            if frame.geometry.color_bits/8<=3{machine.acknowledge(None)?;continue;}
                            if self.blob_pending.as_ref().is_none_or(|(pc,_)|*pc!=call.pc){
                                let [b,g,r,alpha]=a[1].to_le_bytes();
                                let effects=crate::native_surface_ops::effects_bytes(&[json!({"kind":"channel_scale","factors":[r,g,b,alpha]})]);
                                let job=host.image_metadata(self.client)?.begin_process(crysirror_core::image_processing::ImageInput::Resource(frame.image.clone()),effects).map_err(anyhow::Error::from)?;
                                self.blob_pending=Some((call.pc,BlobPending::Recolour(job)));
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some((_,BlobPending::Recolour(job)))=self.blob_pending.as_mut() else{anyhow::bail!("CatSystem2 image colour pending state")};
                            match job.poll().map_err(anyhow::Error::from)?{
                                State::Pending=>return Ok(VmYield::BudgetExhausted),
                                State::Failed(e)=>anyhow::bail!("CatSystem2 image colour multiply: {e}"),
                                State::Ready(image)=>{
                                    self.blob_pending=None;
                                    if let Some(entry)=self.handles.images.get_mut(&a[0]){entry.image=image;}
                                    if let Some(upload)=self.handles.uploads.remove(&a[0]){let _=host.release_image_resource(self.client,upload.path.relative());}
                                    machine.acknowledge(None)?;
                                }
                            }
                        }
                        #[cfg(feature="runtime")]
                        710 => {
                            // 653B10 -> 741600: decode a named image; a missing file answers 0.
                            ensure!(call.arguments.len()==1,"CatSystem2 image load arguments");
                            machine.preflight_native(true,&[])?;
                            let owner = self.tasks.current();
                            if !self.image_loading.contains_key(&owner){
                                let name=crate::native_values::format_bytes(machine,call.arguments[0])?;
                                let name=String::from_utf8(crate::native_scene::stem(&name).to_vec()).context("CatSystem2 image name encoding")?;
                                let Some(pending)=crate::native_image_resource::Pending::begin_named(self.pool.as_ref().context("CatSystem2 worker pool missing")?,host,self.client,self.tasks.current(),call.pc,&name,0)? else{return Ok(VmYield::BudgetExhausted);};
                                self.image_loading.insert(owner,pending);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let pending = self.image_loading.get_mut(&owner).unwrap();
                            ensure!(pending.owner == owner && pending.native_pc == call.pc, "CatSystem2 image completion belongs to another native call");
                            let Some(outcome)=pending.poll()? else{return Ok(VmYield::BudgetExhausted);};
                            self.image_loading.remove(&owner);
                            let id=match outcome{
                                crate::native_image_resource::Outcome::Ready(frame)=>{let id=self.handles.allocate()?;self.handles.images.insert(id,frame);id}
                                _=>0,
                            };
                            machine.acknowledge(Some(id))?;
                        }
                        #[cfg(feature="runtime")]
                        257|732 => {
                            // 643910/6444C0 → messages 280/282 → 742A70: one image-list draw record.
                            ensure!(call.arguments.len()==13,"CatSystem2 image-list draw arguments");
                            machine.preflight_native(false,&[])?;
                            let a=call.arguments.clone();
                            
                            if self.tasks.image_list(a[0]).is_ok_and(|list|list.manager.is_some()){
                                let Some(frame)=self.handles.images.get(&a[2]).cloned() else{machine.acknowledge(None)?;continue;};
                                if !self.handles.uploads.contains_key(&a[2]){
                                    let upload=host.publish_image_resource(self.client,0x47000000u32+(a[2]&0xffffff),frame.image.clone())?;
                                    self.handles.uploads.insert(a[2],upload);
                                }
                                let upload=&self.handles.uploads[&a[2]];
                                let image=upload.path.relative().to_owned();let size=upload.size;
                                let key=crate::native_image_publication::ImageList::key(self.tasks.current(),a[1])?;
                                let publication=self.tasks.list_publication(a[0],self.client)?;
                                // 743230 -> 746330 replaces the bitmap's default
                                // hot spot with arguments5/6. Using stdinfo.base
                                // here loses the pivot of captured screen images.
                                // 746C40 truncates position and pivot separately.
                                let origin=frame.geometry.origin;
                                let position=[f32::from_bits(a[3]).trunc()+origin[0] as f32,f32::from_bits(a[4]).trunc()+origin[1] as f32];
                                let pivot=[f32::from_bits(a[5]),f32::from_bits(a[6])];
                                publication.put(crate::native_image_publication::Draw{key,image,size,position,pivot,alpha:a[10].min(255) as u8,blend:crysirror_scene::layer::Blend::Normal,visible:a[11]!=0,clip:None})?;
                                publication.publish(host,self.client,Vec::new())?;
                            }
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        #[cfg(feature="runtime")]
                        112|113|115|116|118|367|733|736|723|726|727|730|713|737|538|735 => {
                            use crate::native_surface::{self as surface,Pending,Math,Blit,LIST_BITMAP};
                            use crysirror_core::image_processing::ImageProcessState as State;
                            let returns=matches!(call.id,112|113|118|733);
                            machine.preflight_native(returns,&[])?;
                            let owner = (self.tasks.current(),call.pc,call.id);
                            // A pending screen capture/bitmap edit belongs to one
                            // exact native call. Another KCS task may yield here,
                            // but cannot consume that job or its geometry receipt.
                            if self.surface.job.is_some() {
                                if self.surface.owner != Some(owner) { return Ok(VmYield::BudgetExhausted); }
                            } else {
                                self.surface.owner = Some(owner);
                                self.surface.geometry = None;
                            }
                            if let Some(pending)=self.surface.job.take(){
                                match pending{
                                    Pending::Create(mut job)=>match job.poll().map_err(anyhow::Error::from)?{
                                        State::Pending=>{self.surface.job=Some(Pending::Create(job));return Ok(VmYield::BudgetExhausted);}
                                        State::Failed(e)=>anyhow::bail!("CatSystem2 native image creation: {e}"),
                                        State::Ready(image)=>{
                                            let id=self.handles.allocate()?;
                                            self.handles.images.insert(id,surface::prepared(image));
                                            machine.acknowledge(Some(id))?;
                                        }
                                    },
                                    Pending::Capture{mut job,blit,region}=>match job.poll().map_err(anyhow::Error::from)?{
                                        State::Pending=>{self.surface.job=Some(Pending::Capture{job,blit,region});return Ok(VmYield::BudgetExhausted);}
                                        State::Failed(e)=>anyhow::bail!("CatSystem2 native list capture: {e}"),
                                        State::Ready(image)=>{
                                            let batch=crysirror_scene::ServiceBatch{canvases:vec![crysirror_scene::canvas::CanvasCommand::Destroy{id:region}],..Default::default()};
                                            host.submit_engine(self.client,1,&serde_json::to_vec(&batch)?)?;
                                            let source=surface::prepared(image);
                                            let Some(destination)=self.handles.images.get(&blit.destination).cloned() else{machine.acknowledge(None)?;continue;};
                                            let Some((src,dst,size))=surface::clip(blit,&source,&destination) else{machine.acknowledge(None)?;continue;};
                                            let input=surface::composite(destination.image.clone(),source.image,src,dst,size,blit.math);
                                            let job=host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?;
                                            self.surface.job=Some(Pending::Compose{job,destination:blit.destination});
                                            return Ok(VmYield::BudgetExhausted);
                                        }
                                    },
                                    Pending::Compose{mut job,destination}=>match job.poll().map_err(anyhow::Error::from)?{
                                        State::Pending=>{self.surface.job=Some(Pending::Compose{job,destination});return Ok(VmYield::BudgetExhausted);}
                                        State::Failed(e)=>anyhow::bail!("CatSystem2 native image blit: {e}"),
                                        State::Ready(image)=>{
                                            if let Some(frame)=self.handles.images.get_mut(&destination){
                                                frame.geometry.size=image.size();
                                                if let Some((canvas,origin))=self.surface.geometry.take(){frame.geometry.canvas=canvas;frame.geometry.origin=origin;}
                                                frame.image=image;
                                            }
                                            if let Some(upload)=self.handles.uploads.remove(&destination){let _=host.release_image_resource(self.client,upload.path.relative());}
                                            machine.acknowledge(None)?;
                                        }
                                    },
                                }
                                if self.surface.job.is_none() {
                                    self.surface.owner=None;
                                    self.surface.geometry=None;
                                }
                                continue;
                            }
                            let a=call.arguments.clone();
                            match call.id{
                                // 653650: an empty image group of the given size.
                                733=>{
                                    if a[0]==0||a[1]==0{machine.acknowledge(Some(0))?;continue;}
                                    let input=surface::empty_input([a[0],a[1]])?;
                                    let job=host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?;
                                    self.surface.job=Some(Pending::Create(job));
                                }
                                // 653C70, 653090, 653820, 653C90, 653DF0, 6530B0, 6537C0: edits of one image group.
                                // 653390: image a0's rectangle a6..a9 onto image a5's rectangle a1..a4,
                                // stretched unless the sizes match (6874B0 / 6876A0). A destination
                                // without an image goes to VRAM, which the retained scene composes.
                                735=>{
                                    let (Some(source),Some(destination))=(self.handles.images.get(&a[0]).cloned(),self.handles.images.get(&a[5]).cloned()) else{machine.acknowledge(None)?;continue;};
                                    let (from,to)=([a[6] as i32,a[7] as i32,a[8] as i32,a[9] as i32],[a[1] as i32,a[2] as i32,a[3] as i32,a[4] as i32]);
                                    if from[2]<=0||from[3]<=0||to[2]<=0||to[3]<=0{machine.acknowledge(None)?;continue;}
                                    let input=surface::stretch(destination.image.clone(),source.image,from,to)?;
                                    let job=host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?;
                                    self.surface.job=Some(Pending::Compose{job,destination:a[5]});
                                }
                                723|726|727|730|713|737|736|538=>{
                                    use crate::native_surface_ops as ops;
                                    let Some(frame)=self.handles.images.get(&a[0]) else{machine.acknowledge(None)?;continue;};
                                    let edit=match call.id{
                                        538=>Some(ops::grow(frame,[a[1],a[2]])),
                                        723=>ops::rotate(frame,a[1]),
                                        726=>Some(ops::blur(frame,[a[1],a[2]])),
                                        727=>Some(ops::flip(frame,a[1]!=0)),
                                        730=>ops::scale(frame,[a[1],a[2]]),
                                        713=>(a[1]!=100||a[2]!=100||a[3]!=100).then(||ops::tone(frame,[a[1],a[2],a[3]])),
                                        737=>ops::colour_mode(frame,a[1]),
                                        _=>Some(ops::fill(frame,[a[1] as i32,a[2] as i32,a[3] as i32,a[4] as i32],a[5])),
                                    };
                                    let Some(edit)=edit else{machine.acknowledge(None)?;continue;};
                                    self.surface.geometry=Some((edit.canvas,edit.origin));
                                    let job=host.image_metadata(self.client)?.begin_process(edit.input,ops::effects_bytes(&edit.effects)).map_err(anyhow::Error::from)?;
                                    self.surface.job=Some(Pending::Compose{job,destination:a[0]});
                                }
                                // 674E90 a transparent bitmap; 6752F0 the current screen.
                                112=>{
                                    // 68E970 refuses an empty bitmap and 674E90 returns null.
                                    if a[0]==0||a[1]==0{machine.acknowledge(Some(0))?;continue;}
                                    let input=surface::empty_input([a[0],a[1]])?;
                                    let job=host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?;
                                    self.surface.job=Some(Pending::Create(job));
                                }
                                113=>{
                                    // Model commands/poses issued earlier in this
                                    // KCS slice must reach the shared layers before
                                    // capturing them; otherwise the freeze contains
                                    // the preceding owner tick's actor placement.
                                    if self.m2.needs_random() && !self.fes_children.motions.ensure_random_for_kcs(host,self.client,self.tasks.current(),call.pc)? { return Ok(VmYield::BudgetExhausted); }
                                    let config=self.settings.as_ref().map(|settings|settings.m2.clone()).unwrap_or_default();
                                    let now=host.now_ms();
                                    self.m2.tick(host,self.client,&self.tasks,&self.kcs_sound,&config,self.config.canvas,now,self.fes_children.motions.random().ok())?;
                                    self.surface.job=Some(Pending::Create(host.begin_frame_capture(self.client).map_err(anyhow::Error::from)?));
                                }
                                118=>{
                                    let copy=self.handles.images.get(&a[0]).cloned();
                                    let id=match copy{Some(copy)=>{let id=self.handles.allocate()?;self.handles.images.insert(id,copy);id}None=>0};
                                    machine.acknowledge(Some(id))?;continue;
                                }
                                _=>{
                                    // 0 as destination draws to the screen (677090), which the retained
                                    // scene already composes.
                                    if a[0]==0||a[5]==0{machine.acknowledge(None)?;continue;}
                                    let blit=Blit{destination:a[0],at:[a[1] as i32,a[2] as i32],size:[(a[3] as i32).min(a[8] as i32),(a[4] as i32).min(a[9] as i32)],source_at:[a[6] as i32,a[7] as i32],math:if call.id==116{Math::Copy}else{Math::AlphaOver}};
                                    if a[5]&0xff00_0000==LIST_BITMAP{
                                        let list=a[5]&0xff_ffff;
                                        let publication=self.tasks.list_publication(list,self.client)?;
                                        publication.publish(host,self.client,Vec::new())?;
                                        let (mut canvas,bounds)=(publication.scope().1,publication.scope().2);
                                        if self.list_shadows.get(&list).is_some_and(|tick|self.ticks.saturating_sub(*tick)<=2){canvas=0x5a00_0000|list;}
                                        // 68B620: the rectangle clipped to the work bitmap.
                                        let (x,y)=(blit.source_at[0].max(0),blit.source_at[1].max(0));
                                        let (w,h)=((blit.source_at[0]+blit.size[0]).min(bounds[0] as i32)-x,(blit.source_at[1]+blit.size[1]).min(bounds[1] as i32)-y);
                                        if w<=0||h<=0{machine.acknowledge(None)?;continue;}
                                        let region=0x5900_0000|list;
                                        let batch=crysirror_scene::ServiceBatch{canvases:surface::region_commands(canvas,region,[x as f32,y as f32,w as f32,h as f32]),..Default::default()};
                                        host.submit_engine(self.client,1,&serde_json::to_vec(&batch)?)?;
                                        let blit=Blit{source_at:[0;2],at:[blit.at[0]+x-blit.source_at[0],blit.at[1]+y-blit.source_at[1]],size:[w,h],..blit};
                                        self.surface.job=Some(Pending::Capture{job:host.begin_canvas_image(self.client,region).map_err(anyhow::Error::from)?,blit,region});
                                    }else{
                                        let (Some(source),Some(destination))=(self.handles.images.get(&a[5]).cloned(),self.handles.images.get(&a[0]).cloned()) else{machine.acknowledge(None)?;continue;};
                                        let Some((src,dst,size))=surface::clip(blit,&source,&destination) else{machine.acknowledge(None)?;continue;};
                                        let input=surface::composite(destination.image.clone(),source.image,src,dst,size,blit.math);
                                        let job=host.image_metadata(self.client)?.begin_process(input,std::sync::Arc::from(&b"[]"[..])).map_err(anyhow::Error::from)?;
                                        self.surface.job=Some(Pending::Compose{job,destination:a[0]});
                                    }
                                }
                            }
                            return Ok(VmYield::BudgetExhausted);
                        }
                        #[cfg(feature="runtime")]
                        // 653730 releases what 710/733 made; 674E90's images go through 114.
                        114|711 => {
                            machine.preflight_native(false,&[])?;
                            self.handles.images.remove(&call.arguments[0]);
                            if let Some(upload)=self.handles.uploads.remove(&call.arguments[0]){
                                let _=host.release_image_resource(self.client,upload.path.relative());
                            }
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        362|363 => {
                            // 68C9A0 canvas size (falling back to the bitmap); 68C980 origin.
                            let values=self.handles.images.get(&call.arguments[0]).map_or([0i32;2],|f|{
                                let g=&f.geometry;
                                if call.id==363{g.origin}else{
                                    let size=if g.canvas[0]!=0&&g.canvas[1]!=0{g.canvas}else{g.size};
                                    [size[0] as i32,size[1] as i32]
                                }
                            });
                            let writes:Vec<(u32,Vec<u8>)>=call.arguments[1..].iter().zip(values).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_le_bytes().to_vec())).collect();
                            machine.commit_native(None,&writes)?;
                        }
                        #[cfg(feature="runtime")]
                        372|373|718|719|720 => {
                            // 68D6C0 canvas, 68D6A0 origin, 68D620 base words.
                            let returns=true;machine.preflight_native(returns,&[])?;
                            let a=&call.arguments;
                            let ok=match self.handles.images.get_mut(&a[0]){
                                Some(frame)=>{
                                    let g=&mut frame.geometry;
                                    match call.id{
                                        372|718=>g.canvas=[a[1],a[2]],
                                        373|719=>g.origin=[a[1] as i32,a[2] as i32],
                                        _=>g.base=Some([a[1] as i32,a[2] as i32]),
                                    }
                                    1
                                }
                                None=>0,
                            };
                            machine.acknowledge(Some(ok))?;
                        }
                        #[cfg(feature="runtime")]
                        1026 => {
                            // 68C9D0: a stored pixel at origin-relative coordinates, as a BGRA word.
                            machine.preflight_native(true,&[])?;
                            let a=&call.arguments;
                            let value=self.handles.images.get(&a[0]).and_then(|f|{
                                let o=f.geometry.origin;
                                let (x,y)=((a[1] as i32).wrapping_sub(o[0]),(a[2] as i32).wrapping_sub(o[1]));
                                if x<0||y<0{return None;}
                                f.image.pixel_at([x as u32,y as u32]).ok().flatten()
                            }).map_or(0,|[r,g,b,alpha]|u32::from_le_bytes([b,g,r,alpha]));
                            machine.acknowledge(Some(value))?;
                        }
                        // Messages 285..313 on an image list (5F3AE0). A task of another class has no case.
                        261|263|340|341|455|862|880 if self.tasks.image_list(call.arguments[0]).is_err() => {
                            machine.acknowledge((call.id==261).then_some(0))?;
                        }
                        261 => {
                            // Message285 hands out the list's work bitmap (manager word 2).
                            machine.preflight_native(true,&[])?;
                            machine.acknowledge(Some(0x4800_0000|call.arguments[0]))?;
                        }
                        // 290/310 → 743600 composes draw records; the shared Canvas already
                        // presents them. 312 → 7468A0 only moves a draw that has a motion.
                        340|862|880 => {
                            machine.preflight_native(false,&[])?;
                            machine.acknowledge(None)?;
                        }
                        263|341 => {
                            // 287 → 7435A0 draw visibility; 291 → 7430C0 clears the draws.
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            if self.tasks.image_list(id)?.manager.is_some(){
                                let key=if call.id==263{Some(crate::native_image_publication::ImageList::key(self.tasks.current(),call.arguments[1])?)}else{None};
                                let publication=self.tasks.list_publication(id,self.client)?;
                                match key{
                                    Some(key)=>{if publication.set_visible(key,call.arguments[2]!=0){publication.publish(host,self.client,Vec::new())?;}}
                                    None=>{publication.clear();publication.publish(host,self.client,Vec::new())?;}
                                }
                            }
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        455 => {
                            // 644290 → message296: the base image in draw 0, and the base plane's visibility.
                            machine.preflight_native(false,&[])?;
                            let (id,image)=(call.arguments[0],call.arguments.get(1).copied().unwrap_or(0));
                            let Some(plane)=self.tasks.image_list(id)?.base_plane else{machine.acknowledge(None)?;continue;};
                            let layer=self.tasks.plane(plane)?.layer;
                            let frame=self.handles.images.get(&image).cloned();
                            if self.tasks.image_list(id)?.manager.is_some(){
                                match frame{
                                    Some(frame)=>{
                                        if !self.handles.uploads.contains_key(&image){
                                            let upload=host.publish_image_resource(self.client,0x47000000u32+(image&0xffffff),frame.image.clone())?;
                                            self.handles.uploads.insert(image,upload);
                                        }
                                        let upload=&self.handles.uploads[&image];
                                        let (path,size)=(upload.path.relative().to_owned(),upload.size);
                                        let publication=self.tasks.list_publication(id,self.client)?;
                                        publication.put(crate::native_image_publication::Draw{key:0,image:path,size,position:[0.;2],pivot:[0.;2],alpha:255,blend:crysirror_scene::layer::Blend::Normal,visible:true,clip:None})?;
                                        publication.publish(host,self.client,Vec::new())?;
                                    }
                                    None=>{
                                        let publication=self.tasks.list_publication(id,self.client)?;
                                        if publication.remove(0){publication.publish(host,self.client,Vec::new())?;}
                                    }
                                }
                            }
                            // The base plane presents the work bitmap with the 1280x720 screen at
                            // its centre, as the 80,2F source rectangle of the freeze shows.
                            let canvas=self.tasks.list_publication(id,self.client)?.scope().1;
                            let present=if image!=0{
                                vec![crysirror_scene::canvas::CanvasCommand::Present{id:canvas,layer:0x0700_0000|(id&0xffff),parent:Some(layer),position:[-128.,-47.],order:1}]
                            }else{
                                vec![crysirror_scene::canvas::CanvasCommand::Hide{layer:0x0700_0000|(id&0xffff)}]
                            };
                            let batch=crysirror_scene::ServiceBatch{canvases:present,..Default::default()};
                            host.submit_engine(self.client,1,&serde_json::to_vec(&batch)?)?;
                            host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"set_visible","id":layer,"visible":image!=0}]}))?)?;
                            machine.acknowledge(None)?;
                        }
                        258 => {
                            // 643D40 → message281 → 742FE0: remove one draw record.
                            ensure!(call.arguments.len()==2,"CatSystem2 image-list remove arguments");
                            machine.preflight_native(false,&[])?;
                            if self.tasks.image_list(call.arguments[0]).is_ok_and(|list|list.manager.is_some()){
                                let key=crate::native_image_publication::ImageList::key(self.tasks.current(),call.arguments[1])?;
                                let list=call.arguments[0];
                                let publication=self.tasks.list_publication(list,self.client)?;
                                // The work bitmap keeps its pixels until the next composition, and a
                                // script may read them after this removal: keep a Canvas copy for that.
                                let (canvas,bounds)=(publication.scope().1,publication.scope().2);
                                let shadow=0x5a00_0000|list;
                                use crysirror_scene::canvas::CanvasCommand as C;
                                let mut commands=Vec::new();
                                if self.list_shadows.insert(list,self.ticks).is_none(){commands.push(C::Create{id:shadow,width:bounds[0],height:bounds[1]});}
                                commands.push(C::Copy{source:canvas,destination:shadow,rect:[0.,0.,bounds[0] as f32,bounds[1] as f32],position:[0.;2],opacity:255,replace:true,clip:None,atop:None});
                                host.submit_engine(self.client,1,&serde_json::to_vec(&crysirror_scene::ServiceBatch{canvases:commands,..Default::default()})?)?;
                                let publication=self.tasks.list_publication(list,self.client)?;
                                if publication.remove(key){publication.publish(host,self.client,Vec::new())?;}
                            }
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        714|715|717 => {
                            // 653A10/6539A0/653930: an image's size, origin and base words.
                            ensure!(call.arguments.len()==3,"CatSystem2 image metrics arguments");
                            let geometry=self.handles.images.get(&call.arguments[0]).map(|frame|frame.geometry.clone());
                            let values=geometry.as_ref().map_or([0i32;2],|g|match call.id{714=>[g.size[0] as i32,g.size[1] as i32],715=>g.origin,_=>g.base.unwrap_or_default()});
                            
                            let writes:Vec<(u32,Vec<u8>)>=call.arguments[1..].iter().zip(values).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_le_bytes().to_vec())).collect();
                            machine.commit_native((call.id==717).then_some(1),&writes)?;
                        }
                        285 => {
                            // 656610 → INPUT message144: whether a configured key binding is held.
                            ensure!(call.arguments.len()==1,"CatSystem2 key binding arguments");
                            machine.preflight_native(true,&[])?;
                            let observation=host.input_observation(self.client)?;
                            let held=self.settings.as_ref().map_or(Ok(0),|settings|settings.input.held_sources(call.arguments[0] as usize,&observation)).unwrap_or(0);
                            machine.acknowledge(Some(held))?;
                        }
                        294 => {
                            machine.preflight_native(true,&[])?;
                            let id=self.handles.allocate()?;
                            self.handles.motions.insert(id,Default::default());
                            machine.acknowledge(Some(id))?;
                        }
                        295 => {
                            machine.preflight_native(false,&[])?;
                            self.handles.motions.remove(&call.arguments[0]);
                            machine.acknowledge(None)?;
                        }
                        296|298|313|314 => {
                            machine.preflight_native(false,&[])?;
                            if let Some(set)=self.handles.motions.get_mut(&call.arguments[0]){
                                match call.id{
                                    296=>set.clear()?,
                                    298=>set.remove_group(call.arguments[1])?,
                                    313=>set.skip_group(call.arguments[1])?,
                                    _=>set.skip_all()?,
                                }
                            }
                            machine.acknowledge(None)?;
                        }
                        299|301 => {
                            // 788D60/788DC0: base and output pose blocks, read through VM pointers.
                            ensure!(call.arguments.len()==10,"CatSystem2 motion pose arguments");
                            machine.preflight_native(false,&[])?;
                            if self.handles.motions.contains_key(&call.arguments[0]){
                                let pose=crate::native_motion::read_pose(machine,&call.arguments)?;
                                let set=self.handles.motions.get_mut(&call.arguments[0]).unwrap();
                                if call.id==299{set.set_base(pose)?;}else{set.set_output(pose)?;}
                            }
                            machine.acknowledge(None)?;
                        }
                        302 => {
                            // 788DF0: the evaluated output block.
                            ensure!(call.arguments.len()==10,"CatSystem2 motion pose arguments");
                            let Some(set)=self.handles.motions.get(&call.arguments[0]) else{machine.acknowledge(None)?;continue;};
                            let pose=match set.output()?{Some(pose)=>pose,None=>set.base()};
                            let writes=crate::native_motion::pose_writes(&call.arguments,pose);
                            machine.commit_native(None,&writes)?;
                        }
                        304|535 => {
                            // 658E00/658D10 → 788E50: load a motion program and add a controller.
                            ensure!(call.arguments.len()>=7,"CatSystem2 motion insert arguments");
                            machine.preflight_native(true,&[])?;
                            let a=call.arguments.clone();
                            if !self.handles.motions.contains_key(&a[0]){machine.acknowledge(Some(0))?;continue;}
                            let name=crate::native_values::format_bytes(machine,a[1])?;
                            let name=crate::native_scene::stem(&name).to_vec();
                            if name.is_empty(){machine.acknowledge(Some(0))?;continue;}
                            if !self.handles.motion_programs.contains_key(&name){
                                if self.motion_loading.is_none(){
                                    let text=String::from_utf8(name.clone()).context("CatSystem2 motion name encoding")?;
                                    let storage=host.storage(self.client)?;
                                    let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let task=permit.spawn(move|cancel|crate::fes_live_motions::load(storage.as_ref(),text.as_bytes(),&||cancel.check()))?;
                                    self.motion_loading=Some((name.clone(),task));
                                    return Ok(VmYield::BudgetExhausted);
                                }
                                let Some(program)=self.motion_loading.as_mut().unwrap().1.try_take()? else{return Ok(VmYield::BudgetExhausted);};
                                let (loaded,_)=self.motion_loading.take().unwrap();
                                self.handles.motion_programs.insert(loaded,program);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let program=self.handles.motion_programs[&name].clone();
                            let count=a[3] as usize;
                            ensure!(count<=64,"CatSystem2 motion parameter count");
                            let parameters=(0..count).map(|i|machine.read_word(a[2]+i as u32*4)).collect::<Result<Vec<_>>>()?;
                            let custom=if a[5]!=0&&a[4]!=0{Some(machine.read_bytes(a[4],a[5] as usize)?)}else{None};
                            let delay=if call.id==535{a.get(7).copied().unwrap_or(0)}else{0};
                            let wide:Vec<u16>=name.iter().map(|b|u16::from(*b)).collect();
                            let id=self.handles.motions.get_mut(&a[0]).unwrap().insert(wide,program,&parameters,custom.as_deref(),a[6],0,delay)?;
                            machine.acknowledge(Some(id))?;
                        }
                        307|440 => {
                            // 7893D0 one frame at a clock; 7894E0 to each controller's end.
                            machine.preflight_native(true,&[])?;
                            let Some(set)=self.handles.motions.get_mut(&call.arguments[0]) else{machine.acknowledge(Some(0))?;continue;};
                            let evaluation=if call.id==307{set.evaluate(call.arguments[1],call.arguments[2]!=0,remaining.max(1))?}else{set.terminal(remaining.max(1))?};
                            match evaluation{
                                crate::motion_set::Evaluation::Budget=>return Ok(VmYield::BudgetExhausted),
                                crate::motion_set::Evaluation::Ready{visited,pose}=>{
                                    set.set_output(pose)?;
                                    machine.acknowledge(Some(u32::from(visited)))?;
                                }
                            }
                        }
                        308 => {
                            machine.preflight_native(false,&[])?;
                            if let Some(set)=self.handles.motions.get_mut(&call.arguments[0]){set.set_speed(f32::from_bits(call.arguments[1]))?;}
                            machine.acknowledge(None)?;
                        }
                        310|311 => {
                            machine.preflight_native(true,&[])?;
                            let active=match self.handles.motions.get(&call.arguments[0]){
                                None=>false,
                                Some(set)=>if call.id==310{set.group_active(call.arguments[1])?}else{set.any_active()?},
                            };
                            machine.acknowledge(Some(u32::from(active)))?;
                        }
                        586 => {
                            // 789B40: a word copied to controllers made later; it carries no pose.
                            machine.preflight_native(false,&[])?;
                            machine.acknowledge(None)?;
                        }
                        605|606|607|674|844|939|942|943|944|946|947|948|950|956|957|960|963|964|995|1000|1001 => {
                            // 71xxxx SoundConfig queries by PART label. A name without a row answers
                            // each handler's own default, so these defaults are also real answers.
                            machine.preflight_native(true,&[])?;
                            let name=call.arguments.first().map(|a|crate::native_values::format_bytes(machine,*a)).transpose()?.unwrap_or_default();
                            let policy=self.sound_definition.as_ref().and_then(|d|d.policy.as_ref());
                            let mut interpreter=self.interpreter.as_mut();
                            let mut query=|selector:&[u8],default:u32|->u32{
                                match (policy,interpreter.as_deref_mut()){
                                    (Some(policy),Some(vm))=>policy.get(selector,&[name.clone()],vm).unwrap_or(default),
                                    _=>default,
                                }
                            };
                            let value=match call.id{
                                946=>query(b"volume",100),
                                944=>query(b"play",1),
                                948=>query(b"sysplay",u32::MAX),
                                950|957=>query(b"volume_type",100),
                                960=>query(b"play_type",1),
                                956|963=>query(b"sysplay_type",100),
                                605|943|947|964|995=>100,
                                844|674|1001=>u32::MAX,
                                _=>0,
                            };
                            machine.acknowledge(Some(value))?;
                        }
                        #[cfg(feature="runtime")]
                        604|936 => {
                            ensure!(call.arguments.len()>=5,"CatSystem2 KCS sound load arguments");
                            machine.preflight_native(true,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[2])?;
                            let name=crate::native_scene::stem(&name).to_vec();
                            let policy=self.sound_definition.as_ref().and_then(|d|d.policy.as_ref());
                            match self.kcs_sound.load(host,self.client,self.pool.as_ref().context("CatSystem2 worker pool missing")?,policy,call.arguments[0],&name)?{
                                crate::native_kcs_sound::Progress::Pending=>return Ok(VmYield::BudgetExhausted),
                                crate::native_kcs_sound::Progress::Done(channel)=>machine.acknowledge(Some(channel))?,
                            }
                        }
                        #[cfg(feature="runtime")]
                        156 => {
                            machine.preflight_native(false,&[])?;
                            let policy=self.sound_definition.as_ref().and_then(|d|d.policy.as_ref());
                            match self.kcs_sound.play(host,self.client,policy,call.arguments[0])?{
                                crate::native_kcs_sound::Progress::Pending=>return Ok(VmYield::BudgetExhausted),
                                crate::native_kcs_sound::Progress::Done(())=>machine.acknowledge(None)?,
                            }
                        }
                        #[cfg(feature="runtime")]
                        157|158|159 => {
                            // 182 fades out over a3[1] ms; 183 and 184 stop now.
                            machine.preflight_native(false,&[])?;
                            let fade=if call.id==157{call.arguments.get(1).copied().unwrap_or(0) as f32}else{0.};
                            self.kcs_sound.stop(host,self.client,call.arguments[0],fade)?;
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        160 => {
                            machine.preflight_native(true,&[])?;
                            let playing=self.kcs_sound.playing(host,self.client,call.arguments[0])?;
                            machine.acknowledge(Some(u32::from(playing)))?;
                        }
                        #[cfg(feature="runtime")]
                        161|163 => {
                            // 186 sets a percent; 188 fades to the low word over the high word in ms.
                            machine.preflight_native(false,&[])?;
                            let policy=self.sound_definition.as_ref().and_then(|d|d.policy.as_ref());
                            let (percent,fade)=if call.id==161{(call.arguments[1],0.)}else{(call.arguments[1]&0xffff,call.arguments.get(2).copied().unwrap_or(0) as f32)};
                            self.kcs_sound.volume(host,self.client,policy,call.arguments[0],percent.min(100),fade)?;
                            machine.acknowledge(None)?;
                        }
                        437 => {
                            // 654080 -> 5D4940: preload generation reached. Resources load on
                            // demand here, so nothing is ever behind.
                            machine.preflight_native(true,&[])?;
                            machine.acknowledge(Some(1))?;
                        }
                        435 => {
                            // 653F90: queue a file for the preload thread; a cache hint only.
                            machine.preflight_native(false,&[])?;
                            machine.acknowledge(None)?;
                        }
                        #[cfg(feature="runtime")]
                        906 => {
                            // 640?? → message524: "name,body,wait,..." selects an E-mote body.
                            machine.preflight_native(false,&[])?;
                            let spec=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            let spec=String::from_utf8(spec).context("CatSystem2 model name encoding")?;
                            if self.tasks.plane(call.arguments[0]).is_ok()&&!spec.is_empty(){
                                if !self.m2.begin(host,self.client,self.pool.as_ref().context("CatSystem2 worker pool missing")?,call.arguments[0],spec)?{return Ok(VmYield::BudgetExhausted);}
                            }
                            machine.acknowledge(None)?;
                        }
                        // 64C?? messages 530..542: the plane's E-mote controller.
                        #[cfg(feature="runtime")]
                        917|918|919|911|913|914|916|921|926|927|928|966 => {
                            use crate::native_m2::Command;
                            let a=call.arguments.clone();
                            let plane=a[0];
                            let map=|maps:&crate::native_string_map::StringMaps,handle:u32|maps.entries(handle);
                            match call.id{
                                917=>{
                                    machine.preflight_native(false,&[])?;
                                    let words=crate::native_values::format_bytes(machine,a[1])?;
                                    self.m2.command(plane,Command::Body(String::from_utf8_lossy(&words).into_owned()));
                                    machine.acknowledge(None)?;
                                }
                                926=>{
                                    let mut digits=self.m2.body_digits(plane).into_bytes();digits.push(0);
                                    if a[1]!=0{machine.commit_native(None,&[(a[1],digits)])?;}else{machine.acknowledge(None)?;}
                                }
                                914=>{machine.preflight_native(false,&[])?;self.m2.command(plane,Command::ParameterScale(None,f32::from_bits(a[1])));machine.acknowledge(None)?;}
                                916=>{
                                    machine.preflight_native(false,&[])?;
                                    for (name,value) in map(&self.string_maps,a[1]){self.m2.command(plane,Command::ParameterScale(Some(String::from_utf8_lossy(&name).into_owned()),f32::from_bits(value)));}
                                    machine.acknowledge(None)?;
                                }
                                966=>{machine.preflight_native(false,&[])?;self.m2.command(plane,Command::ClearParameterScale);machine.acknowledge(None)?;}
                                919=>{machine.preflight_native(false,&[])?;self.m2.command(plane,Command::Tone([a[1],a[2],a[3]]));machine.acknowledge(None)?;}
                                921=>{
                                    machine.preflight_native(false,&[])?;
                                    for (name,value) in map(&self.string_maps,a[1]){self.m2.command(plane,Command::Parameter(String::from_utf8_lossy(&name).into_owned(),value as f32));}
                                    machine.acknowledge(None)?;
                                }
                                927=>{machine.preflight_native(false,&[])?;self.m2.command(plane,Command::Blink(a[1]!=0));machine.acknowledge(None)?;}
                                // 911/913 time scale of the variable interpolators and 918, 928 (lip motion)
                                // have nothing to change in the shared puppet's evaluation.
                                _=>{machine.preflight_native(false,&[])?;machine.acknowledge(None)?;}
                            }
                        }
                        #[cfg(feature="runtime")]
                        265 => {
                            // 654C90: a movie the catalog can name, or 0 for the script to skip it.
                            machine.preflight_native(true,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[0])?;
                            let name=String::from_utf8_lossy(crate::native_scene::stem(&name)).into_owned();
                            let storage=host.storage(self.client)?;
                            let catalog=crate::native_resource_paths::Catalog::load(storage.as_ref())?;
                            let mut found=None;
                            for candidate in catalog.candidates(storage.as_ref(),"movie",&name,&||Ok(()))?{
                                if let Some(path)=crate::native_resource_paths::resolve(storage.as_ref(),&candidate)?{found=Some(path);break;}
                            }
                            let id=match found{Some(path)=>self.movies.open(path.relative().to_owned(),self.config.canvas)?,None=>{host.append_log(self.client,&format!("CatSystem2 movie {name} not found"))?;0}};
                            machine.acknowledge(Some(id))?;
                        }
                        #[cfg(feature="runtime")]
                        266 => {machine.preflight_native(true,&[])?;self.movies.play(host,self.client,call.arguments[0])?;machine.acknowledge(Some(1))?;}
                        #[cfg(feature="runtime")]
                        // 655000 → 6FD6D0; 654C10 → 6FCA60.
                        267 => {machine.preflight_native(true,&[])?;let stopped=self.movies.stop(host,self.client,call.arguments[0])?;machine.acknowledge(Some(u32::from(stopped)))?;}
                        998 => {machine.preflight_native(true,&[])?;let at=self.movies.position_ms(host,self.client,call.arguments[0]);machine.acknowledge(Some(at))?;}
                        268 => {machine.preflight_native(true,&[])?;self.movies.close(host,self.client,call.arguments[0])?;machine.acknowledge(Some(1))?;}
                        #[cfg(feature="runtime")]
                        269 => {machine.preflight_native(true,&[])?;let playing=self.movies.playing(host,self.client,call.arguments[0]);machine.acknowledge(Some(playing))?;}
                        // 6FD330 pumps the original's decoder graph; the shared service keeps its own clock.
                        270 => {machine.preflight_native(false,&[])?;machine.acknowledge(None)?;}
                        #[cfg(feature="runtime")]
                        417 => {machine.preflight_native(false,&[])?;self.movies.set_gain(host,self.client,call.arguments[0],call.arguments[1])?;machine.acknowledge(None)?;}
                        // 64F970: the screen rectangle in script coordinates.
                        787 => {
                            let [w,h]=self.config.canvas;
                            let writes=call.arguments.iter().zip([0,0,w,h]).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_le_bytes().to_vec())).collect::<Vec<_>>();
                            machine.commit_native(None,&writes)?;
                        }
                        #[cfg(feature="runtime")]
                        788 => {machine.preflight_native(false,&[])?;let a=&call.arguments;self.movies.place(host,self.client,a[0],[a[1] as i32,a[2] as i32,a[3] as i32,a[4] as i32])?;machine.acknowledge(None)?;}
                        // 65A0F0 -> 12 case 172 -> 877310: a message into a script parked in its
                        // native71. The caller waits until the target has taken it and run on.
                        69 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 script call arguments");
                            machine.preflight_native(true,&[])?;
                            let (me,target)=(self.tasks.current(),call.arguments[0]);
                            if self.script_calls.contains_key(&me){
                                if self.tasks.has_script_event(target){return Ok(VmYield::BudgetExhausted);}
                                self.script_calls.remove(&me);
                                // What the called script wrote into this caller's memory (73).
                                let mut writes=Vec::new();
                                self.script_memory.retain(|(owner,address,bytes)|{if *owner==me{writes.push((*address,bytes.clone()));false}else{true}});
                                machine.commit_native(Some(1),&writes)?;
                            }else if target!=me&&(target==self.main_task||self.kcs_children.iter().any(|child|child.id==target)){
                                self.tasks.queue_script_event(target,[me,call.arguments[1],call.arguments[2],call.arguments[3]]);
                                self.script_calls.insert(me,target);
                                return Ok(VmYield::BudgetExhausted);
                            }else{machine.acknowledge(Some(0))?;}
                        }
                        // 65A010 -> 12 case 173 -> 5E93B0: the same message, queued without waiting.
                        70 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 script post arguments");
                            machine.preflight_native(true,&[])?;
                            let (me,target)=(self.tasks.current(),call.arguments[0]);
                            let known=target==self.main_task||self.kcs_children.iter().any(|child|child.id==target);
                            if known{self.tasks.queue_script_event(target,[me,call.arguments[1],call.arguments[2],call.arguments[3]]);}
                            machine.acknowledge(Some(u32::from(known)))?;
                        }
                        // 65A090/65A1B0: bytes between two scripts' memory; all tasks share one heap.
                        72|73 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 script memory arguments");
                            let (target,theirs,len,mine)=(call.arguments[0],call.arguments[1],call.arguments[2] as usize,call.arguments[3]);
                            let (from,to)=if call.id==72{(theirs,mine)}else{(mine,theirs)};
                            let remote=target!=self.tasks.current();
                            if call.id==73&&remote{
                                // A script's locals are its own: they reach it when its call returns.
                                let bytes=machine.read_bytes(from,len)?;
                                self.script_memory.push((target,to,bytes));
                                machine.preflight_native(false,&[])?;
                                machine.acknowledge(None)?;
                            }else{
                                let bytes=machine.read_bytes(from,len).unwrap_or_else(|_|vec![0;len]);
                                machine.commit_native(None,&[(to,bytes)])?;
                            }
                        }
                        // 6474C0: `enable_sceneanm`, as the config screen left it.
                        1050 => {
                            machine.preflight_native(true,&[])?;
                            let value=self.settings.as_ref().map_or(0,|settings|settings.extras.get(&b"enable_sceneanm"[..]).copied().unwrap_or(0));
                            machine.acknowledge(Some(value))?;
                        }
                        // 643060, 6430A0, 643150: matrices in VM memory; a null address is the original's scratch.
                        207|204|206 => {
                            use crate::native_math as math;
                            let a=call.arguments.clone();
                            machine.preflight_native(false,&[])?;
                            if a[0]==0{machine.acknowledge(None)?;}else{
                                let matrix=if call.id==207{math::IDENTITY}else{
                                    let current=math::from_bytes(&machine.read_bytes(a[0],64)?);
                                    let word=|i:usize|f32::from_bits(a[i]);
                                    if call.id==204{
                                        let turned=math::multiply(&current,&math::rotation(0,word(1)));
                                        let turned=math::multiply(&turned,&math::rotation(1,word(2)));
                                        math::multiply(&turned,&math::rotation(2,word(3)))
                                    }else{math::multiply(&current,&math::translation(word(1),word(2),word(3)))}
                                };
                                machine.commit_native(None,&[(a[0],math::to_bytes(&matrix))])?;
                            }
                        }
                        // 646E20 -> 67EC90: D3DXVec4Transform in place, row vector times matrix.
                        358 => {
                            let vector=machine.read_bytes(call.arguments[0],16)?;
                            let matrix=crate::native_math::from_bytes(&machine.read_bytes(call.arguments[1],64)?);
                            let out=crate::native_math::transform(&vector,&matrix);
                            machine.commit_native(None,&[(call.arguments[0],out)])?;
                        }
                        // 65AF80: pow.
                        397 => {
                            machine.preflight_native(true,&[])?;
                            let value=f32::from_bits(call.arguments[0]).powf(f32::from_bits(call.arguments[1]));
                            machine.acknowledge(Some(value.to_bits()))?;
                        }
                        // 646390: a bracket list into a word array; the count answers.
                        539 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 bracket list arguments");
                            let text=machine.read_cstring(call.arguments[0],2048)?;
                            let interpreter=self.interpreter.as_mut();
                            let mut scratch=interpreter;
                            let words=crate::native_math::bracket_words(&text,call.arguments[2] as usize,&mut |word|match &mut scratch{Some(vm)=>vm.evaluate(word).unwrap_or(0),None=>std::str::from_utf8(word).ok().and_then(|w|w.parse().ok()).unwrap_or(0)});
                            let bytes=words.iter().flat_map(|w|w.to_le_bytes()).collect::<Vec<_>>();
                            if bytes.is_empty()||call.arguments[1]==0{machine.preflight_native(true,&[])?;machine.acknowledge(Some(words.len() as u32))?;}
                            else{machine.commit_native(Some(words.len() as u32),&[(call.arguments[1],bytes)])?;}
                        }
                        // 656130.
                        1051 => {
                            machine.preflight_native(true,&[])?;
                            let (text,needle)=(machine.read_cstring(call.arguments[0],65534)?,crate::native_values::format_bytes(machine,call.arguments[1])?);
                            machine.acknowledge(Some(crate::native_math::find_characters(&text,&needle)))?;
                        }
                        // 648530: the value of the first map key that prefixes the text, ignoring case.
                        1052 => {
                            machine.preflight_native(true,&[])?;
                            let text=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            let found=self.string_maps.entries(call.arguments[0]).into_iter().find(|(key,_)|text.len()>=key.len()&&text[..key.len()].eq_ignore_ascii_case(key)).map_or(0,|(_,value)|value);
                            machine.acknowledge(Some(found))?;
                        }
                        // 648210: a serialized map added to a map without replacing keys.
                        892 => {
                            machine.preflight_native(false,&[])?;
                            if call.arguments[0]!=0&&call.arguments[1]!=0{
                                let bytes=machine.read_block(call.arguments[1],16*1024*1024)?;
                                self.string_maps.deserialize_new(call.arguments[0],&bytes)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        // 6504C0, 6543B0: debug output.
                        1040|1042 => {
                            machine.preflight_native(false,&[])?;
                            let text=machine.read_cstring(call.arguments[0],4096)?;
                            host.append_log(self.client,&String::from_utf8_lossy(&text))?;
                            machine.acknowledge(None)?;
                        }
                        // 5D3EC0, 5D6400, 5D4650: the original's archive read-ahead; the engine has its own.
                        434|436|438 => {machine.preflight_native(false,&[])?;machine.acknowledge(None)?;}
                        // 64FBB0: this task's argument string (+712, message 26 from 659DC0); the
                        // tasks here take none, so the buffer gets an empty one.
                        454 => {machine.commit_native(None,&[(call.arguments[0],vec![0])])?;}
                        // 645090: whether the named sound can be opened.
                        284 => {
                            machine.preflight_native(true,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            let storage=host.storage(self.client)?;
                            #[cfg(feature="runtime")]
                            let found=u32::from(crate::fes_live_sound::resolve(storage.as_ref(),crate::native_scene::stem(&name),&||Ok(())).is_ok());
                            #[cfg(not(feature="runtime"))]
                            let found={let _=(&storage,&name);0};
                            machine.acknowledge(Some(found))?;
                        }
                        // 64BB00, 653A80: a bitmap's size.
                        #[cfg(feature="runtime")]
                        119|712 => {
                            let size=self.handles.images.get(&call.arguments[0]).map_or([0;2],|frame|frame.image.size());
                            let writes=call.arguments[1..].iter().zip(size).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_le_bytes().to_vec())).collect::<Vec<_>>();
                            machine.commit_native(None,&writes)?;
                        }
                        // 64B4C0 / 653840 -> 68C6F0: five raw ats words from the shared HG3 metadata.
                        #[cfg(feature="runtime")]
                        536|724 => {
                            let a=&call.arguments;
                            ensure!(a.len()==7,"CatSystem2 bitmap attribute arguments");
                            let values=self.handles.images.get(&a[0]).and_then(|frame|frame.geometry.attribute(a[1]));
                            let writes=values.map_or(Vec::new(),|values|a[2..].iter().zip(values).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_le_bytes().to_vec())).collect());
                            // Native724 treats a null wrapper as success, unlike536.
                            machine.commit_native(Some(u32::from(values.is_some()||(call.id==724&&a[0]==0))),&writes)?;
                        }
                        // 650F30 and family: the message log the backlog reads.
                        696 => {machine.preflight_native(true,&[])?;let id=self.tasks.logs.create();machine.acknowledge(Some(id))?;}
                        697 => {machine.preflight_native(false,&[])?;self.tasks.logs.destroy(call.arguments[0]);machine.acknowledge(None)?;}
                        876 => {machine.preflight_native(false,&[])?;self.tasks.logs.clear(call.arguments[0]);machine.acknowledge(None)?;}
                        785 => {machine.preflight_native(false,&[])?;self.tasks.logs.set_limit(call.arguments[0],call.arguments[1]);machine.acknowledge(None)?;}
                        // 650DB0 / 650E60 -> 72E8E0: live message insertion;
                        // 698 supplies history_index=-1, while 977 passes it explicitly.
                        698 | 977 => {
                            let a=&call.arguments;
                            ensure!(a.len()==if call.id==698 {7} else {8},"CatSystem2 message log arguments");
                            machine.preflight_native(false,&[])?;
                            // The original null-handle wrapper does not even format
                            // the three strings, which may be invalid VM addresses.
                            if a[0]!=0 {
                                ensure!(self.tasks.logs.contains(a[0]),"CatSystem2 message log handle {:#x} missing",a[0]);
                                let scene=crate::native_values::format_bytes(machine,a[1])?;
                                let name=crate::native_values::format_bytes(machine,a[3])?;
                                let body=crate::native_values::format_bytes(machine,a[4])?;
                                ensure!(scene.len()<64&&name.len()<256&&body.len()<4096,"CatSystem2 formatted message log extent");
                                let scene=if let Ok(name)=std::str::from_utf8(&scene) {name.to_owned()} else {
                                    encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(&scene).context("CatSystem2 message log scene CP932")?.into_owned()
                                };
                                let row=crate::native_log::Row{
                                    scene,line:a[2],name,text:body,voice:String::new(),
                                    flags:u32::from(a[5]!=0)*4,page_start:a[6]!=0,history_index:if call.id==698 {-1} else {a[7] as i32},
                                };
                                self.tasks.logs.add_message(a[0],row)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        // 651040 -> 72EF40: the row's speaker, which the backlog turns into its name image.
                        700 => {
                            let mut bytes=self.tasks.logs.rows(call.arguments[0]).get(call.arguments[2] as usize).map_or(Vec::new(),|row|row.name.clone());
                            bytes.push(0);
                            machine.commit_native(None,&[(call.arguments[1],bytes)])?;
                        }
                        // 64A0F0, 6495D0: the log into and out of a save container.
                        850 => {
                            machine.preflight_native(true,&[])?;
                            let bytes=self.tasks.logs.serialize(call.arguments[2]).ok();
                            let written=match (self.saves.get_mut(&call.arguments[0]),bytes){(Some(save),Some(bytes))=>{save.remove(call.arguments[1]);save.append(call.arguments[1],&bytes)?;1}_=>0};
                            machine.acknowledge(Some(written))?;
                        }
                        851 => {
                            machine.preflight_native(true,&[])?;
                            let record=self.saves.get(&call.arguments[0]).and_then(|save|save.first(call.arguments[1])).map(<[u8]>::to_vec);
                            let read=match record{Some(bytes)=>{self.tasks.logs.restore(call.arguments[2],&bytes)?;1}None=>0};
                            machine.acknowledge(Some(read))?;
                        }
                        // 6442E0 -> 743E40: one draw record's clip; an empty rectangle removes it.
                        #[cfg(feature="runtime")]
                        339 => {
                            ensure!(call.arguments.len()==6,"CatSystem2 draw clip arguments");
                            machine.preflight_native(false,&[])?;
                            let a=call.arguments.clone();
                            if self.tasks.image_list(a[0]).is_ok_and(|list|list.manager.is_some()){
                                let key=crate::native_image_publication::ImageList::key(self.tasks.current(),a[1])?;
                                let clip=(a[4] as i32>0&&a[5] as i32>0).then(||[a[2] as i32,a[3] as i32,a[2] as i32+a[4] as i32,a[3] as i32+a[5] as i32]);
                                let publication=self.tasks.list_publication(a[0],self.client)?;
                                if publication.set_clip(key,clip){publication.publish(host,self.client,Vec::new())?;}
                            }
                            machine.acknowledge(None)?;
                        }
                        882 => {
                            // 654970 -> 62BED0 case 352 -> 758380: a named value in the task's player.
                            ensure!(call.arguments.len()==3,"CatSystem2 FES named value arguments");
                            machine.preflight_native(false,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            if self.tasks.fes(call.arguments[0]).is_ok_and(|task|task.player_loaded)&&!name.is_empty(){
                                self.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?.set_player_named(call.arguments[0],&name,call.arguments[2])?;
                            }
                            machine.acknowledge(None)?;
                        }
                        645 => {
                            // 654AA0 -> 62BED0 case 340 -> 757650: an argument in the player's private bank.
                            ensure!(call.arguments.len()==3,"CatSystem2 FES argument arguments");
                            machine.preflight_native(false,&[])?;
                            if self.tasks.fes(call.arguments[0]).is_ok_and(|task|task.player_loaded){
                                self.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?.set_player_local(call.arguments[0],call.arguments[1],call.arguments[2])?;
                            }
                            machine.acknowledge(None)?;
                        }
                        646 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 FES argument query arguments");
                            machine.preflight_native(true,&[])?;
                            let value=self.interpreter.as_ref().map_or(0,|vm|vm.player_local(call.arguments[0],call.arguments[1]));
                            machine.acknowledge(Some(value))?;
                        }
                        647 => {
                            // 654B00 -> 62BED0 case 342: pauses the task's own phase.
                            ensure!(call.arguments.len()==2,"CatSystem2 FES pause arguments");
                            machine.preflight_native(false,&[])?;
                            self.tasks.set_fes_paused(call.arguments[0],call.arguments[1]!=0);
                            machine.acknowledge(None)?;
                        }
                        630 => {
                            // 6544B0 -> 62BED0 case 337 -> 757850: run a section synchronously
                            // up to its end or its first wait, then present the frame.
                            ensure!(call.arguments.len()==4,"CatSystem2 FES section call arguments");
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            if self.tasks.fes(id).is_ok_and(|task|task.player_loaded) {
                                let observation=if let Some((owner,pc,observation))=self.fes_tick_input {
                                    ensure!(owner==id&&pc==call.pc,"CatSystem2 FES section call owner mismatch");
                                    observation
                                }else{
                                    let observation=host.input_observation(self.client)?;
                                    let section=crate::native_values::format_bytes(machine,call.arguments[1])?;
                                    let object=crate::native_values::format_bytes(machine,call.arguments[2])?;
                                    let (qualified,name,index)=if object.is_empty(){(section,Vec::new(),0)}else{
                                        let mut qualified=object.clone();qualified.push(b'.');qualified.extend(section);(qualified,object,call.arguments[3])
                                    };
                                    let started=self.tasks.fes_player_mut(id)?.begin_external(&qualified,&name,index)?;
                                    if !started{machine.acknowledge(None)?;*self.native_calls.entry(call.id).or_default()+=1;continue;}
                                    self.fes_tick_input=Some((id,call.pc,observation));
                                    observation
                                };
                                let tick=crate::fes_runtime::Services{pointer_glide:Some(&mut self.classes.input_task),
                                    string_variables:&mut self.string_variables,
                                    client:self.client,tasks:&mut self.tasks,
                                    interpreter:self.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?,
                                    settings:&mut self.settings,sound_definition:&mut self.sound_definition,
                                    input_state:&mut self.input_state,steam_manager:&mut self.steam_manager,
                                    fes_file_probe:&mut self.fes_file_probe,fes_civil_time:&mut self.fes_civil_time,fes_children:&mut self.fes_children,handles:Some(&self.handles),saves:&self.saves,
                                    pool:self.pool.as_ref().context("CatSystem2 worker pool missing")?,
                                }.run(host,id,call.pc,observation,&mut remaining)?;
                                match tick{
                                    crate::fes_runtime::Tick::Dependency|crate::fes_runtime::Tick::Budget=>return Ok(VmYield::BudgetExhausted),
                                    crate::fes_runtime::Tick::Waited=>{
                                        self.tasks.fes_player_mut(id)?.end_external();
                                        self.fes_tick_input=None;
                                    }
                                }
                            }
                            machine.acknowledge(None)?;
                        }
                        19..=22 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 string compare arguments");
                            let formatted=matches!(call.id,20|22);
                            let a=if formatted {crate::native_values::format_bytes(machine,call.arguments[0])?} else {machine.read_cstring(call.arguments[1],4096)?};
                            let b=if formatted {crate::native_values::format_bytes(machine,call.arguments[1])?} else {machine.read_cstring(call.arguments[0],4096)?};
                            machine.acknowledge(Some(crate::native_values::compare(&a,&b,call.id>=21)))?;
                        }
                        36 => {
                            // 6556C0: native memset(dst, low-byte value, byte
                            // count), void return; this is owned VM memory.
                            ensure!(call.arguments.len()==3,"CatSystem2 memory fill arguments");
                            let count=call.arguments[2] as usize;
                            ensure!(count<=crate::kcs::MAX_SCRIPT_BYTES,"CatSystem2 native memory fill budget");
                            if count==0 {machine.acknowledge(None)?;}else{
                                machine.commit_native(None,&[(call.arguments[0],vec![call.arguments[1] as u8;count])])?;
                            }
                        }
                        41 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 byte token split arguments");
                            machine.preflight_native(false,&[])?;
                            if self.split_loading.is_none() {
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve() {Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let source=machine.read_cstring(call.arguments[1],65534)?;
                                let delimiters=machine.read_cstring(call.arguments[2],256)?;
                                self.split_loading=Some(permit.spawn(move |cancel|crysirror_text::byte_split::split_delimited_bytes(&source,&delimiters,crysirror_text::byte_split::ByteSplitBoundary::ShiftJis,Default::default(),&||cancel.check()))?);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(output)=self.split_loading.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            // Native41 returns void. Only the native buffer's
                            // logical address/layout belongs to the adapter.
                            machine.commit_native(None,&[(call.arguments[0],output.nul_bytes)])?;
                            self.split_loading=None;
                        }
                        42 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 NUL-list item arguments");
                            let mut bytes=crate::native_values::nul_list_item(machine,call.arguments[1],call.arguments[2])?;
                            let present=!bytes.is_empty();bytes.extend([0,0]);
                            machine.commit_native(Some(u32::from(present)),&[(call.arguments[0],bytes)])?;
                        }
                        66 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 task constructor arguments");
                            if matches!(call.arguments[0],14|43) {
                                ensure!(self.null_view_constructor,"CatSystem2 task class {} has no implemented constructor for this native module profile",call.arguments[0]);
                                machine.preflight_native(true,&[])?;
                                machine.acknowledge(Some(0))?;
                                continue;
                            }
                            // 5DD580 has no constructor for these and answers a null task.
                            if matches!(call.arguments[0],3|4|5|7|9|10|55|56){
                                machine.preflight_native(true,&[])?;
                                machine.acknowledge(Some(0))?;
                                continue;
                            }
                            if !matches!(call.arguments[0],2|6|8|15|34|54){
                                // Task classes whose work has no CrySirror capability yet still own a real
                                // slot, so scripts can address them; 5DC9D0 ignores messages they lack.
                                let name=crate::native_values::format_bytes(machine,call.arguments[2])?;
                                let id=self.tasks.create_opaque(call.arguments[0],call.arguments[1],name)?;
                                machine.acknowledge(Some(id))?;
                                continue;
                            }
                            ensure!(matches!(call.arguments[0],2|6|8|15|34|54),"CatSystem2 unsupported native gcall 66 task class {} at PC {:#x}; original call remains unacknowledged",call.arguments[0],call.pc);
                            machine.preflight_native(true,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[2])?;
                            let mut proposed=self.tasks.clone();
                            let id=if matches!(call.arguments[0],2|8|54) {
                                // 5E38C0 (class 8) builds on the Plane constructor 5DF070.
                                let plane=proposed.create_plane(call.arguments[1],name)?;
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":plane.layer_spec()?}]}))?)?;
                                if call.arguments[0]==8{crate::native_classes::text_window::created(&mut self.classes.text_window,plane.id);}
                                plane.id
                            }else if call.arguments[0]==6{
                                //5DD580 class6 is5DE590 alone: unbound button,
                                //not Plane. No geometry/input record exists
                                //until separately reviewed message89 binding.
                                proposed.create_button(call.arguments[1],name)?.id
                            }else if call.arguments[0]==15{
                                //5F3790/68E6F0 create an empty bitmap wrapper
                                //and native task fields, with manager/baseplane
                                //absent. Full5F3AE0/5DC9D0 ignore26. No file IO
                                //or image resource exists before message279.
                                proposed.create_image_list(call.arguments[1],name)?.id
                            }else{
                                //627E50→62DAA0→26. The constructor125 query
                                //has no parent/children to reach INPUT through;
                                //saved cursor remains Unknown, never zeroed.
                                proposed.create_fes(call.arguments[1],name)?
                            };
                            // Original1/2 attach the native task.26 is unhandled
                            // for Plane/Button/ImageList; FES copies the current
                            // main's source-empty parameter buffer after attach.
                            self.tasks=proposed;machine.acknowledge(Some(id))?;
                        }
                        67 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 native task destroy arguments");
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            //659E40 sends3; no receiver is the original no-op.
                            //Supported FES receivers are only MARKED here.
                            //5DC510/5DC790 release at the next child phase.
                            if !self.tasks.is_fes(id)&&self.tasks.exists(id){
                                // 5DC510 releases the task; its Plane layers go with it.
                                self.kcs_children.retain(|child|child.id!=id);
                                #[cfg(feature="runtime")]
                                {
                                    self.image_loading.remove(&id);
                                    self.surface.cancel(host,self.client,id)?;
                                }
                                #[cfg(feature="runtime")]
                                self.m2.remove(host,self.client,id)?;
                                crate::native_classes::retire(host,self.client,&mut self.classes,id)?;
                                #[cfg(feature="runtime")]
                                {
                                    let canvases=self.tasks.release_plane_publications(id)?;
                                    if !canvases.is_empty(){host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":canvases}))?)?;}
                                }
                                let commands=self.tasks.retire_plain_task(id)?;
                                if !commands.is_empty(){host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":commands}))?)?;}
                                machine.acknowledge(None)?;
                            }else if self.tasks.exists(id){
                                let mut proposed=self.tasks.clone();let mut interpreter=self.interpreter.clone();
                                proposed.request_fes_destroy(id,interpreter.as_mut())?;
                                machine.acknowledge(None)?;self.tasks=proposed;self.interpreter=interpreter;
                            }else{machine.acknowledge(None)?;}
                        }
                        79 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 solid plane arguments");
                            // 6758B0 drops the old object first; 676CB0 then builds one only for render type 1.
                            if self.tasks.plane(call.arguments[0]).is_ok_and(|p|p.render_type!=1){
                                machine.preflight_native(false,&[])?;
                                if let Some(plane)=self.tasks.plane_mut(call.arguments[0]){crate::native_plane::remove_payload(host,self.client,plane)?;}
                                machine.acknowledge(None)?;*self.native_calls.entry(call.id).or_default()+=1;continue;
                            }
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            if self.tasks.plane(id).is_ok() {
                                let w=f32::from_bits(call.arguments[1]);let h=f32::from_bits(call.arguments[2]);
                                ensure!(w.is_finite()&&h.is_finite()&&w<=8192.&&h<=8192.,"CatSystem2 unsupported native solid plane extent");
                                // 679140 builds an empty quad: nothing to draw until a real size arrives.
                                if w<=0.||h<=0.{
                                    if let Some(plane)=self.tasks.plane_mut(id){crate::native_plane::remove_payload(host,self.client,plane)?;}
                                    machine.acknowledge(None)?;
                                    *self.native_calls.entry(call.id).or_default()+=1;
                                    continue;
                                }
                                let plane=self.tasks.plane(id)?;
                                ensure!(plane.image.is_none(),"CatSystem2 unsupported native solid replacement of an image plane");
                                let c=call.arguments[3];
                                // 63FDE0→message42→679140 builds two triangles
                                // with XYZ plus packed diffuse ARGB per vertex.
                                // A normal engine Solid layer owns that quad.
                                let color=[(c>>16) as u8,(c>>8) as u8,c as u8,(c>>24) as u8];
                                let payload=plane.layer+1;
                                let mut commands=Vec::new();
                                if plane.payload.is_some(){commands.push(json!({"op":"remove","id":payload}));}
                                commands.push(json!({"op":"put","layer":{"id":payload,"parent":plane.layer,"rect":[0.,0.,w,h],"paint":{"kind":"solid","color":color}}}));
                                commands.push(json!({"op":"set_depth","id":payload,"depth":0.}));
                                // 679140's new native record has Src[1]=0:
                                // resource creation itself does not show it.
                                commands.push(json!({"op":"set_visible","id":plane.layer,"visible":false}));
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":commands}))?)?;
                                self.tasks.set_solid_payload(id,c)?;
                                // Later alpha/pose messages must retain this
                                // quad's extent instead of rebuilding a 1x1 rect.
                                self.tasks.plane_mut(id).unwrap().pose.size = [w,h];
                            }
                            machine.acknowledge(None)?;
                        }
                        100 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 solid diffuse arguments at PC {:#x}",call.pc);
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            if self.tasks.plane(id).is_ok() {
                                let mut proposed=self.tasks.clone();
                                if let Some((payload,color))=proposed.set_solid_diffuse(id,call.arguments[1],call.arguments[2]).with_context(||format!("CatSystem2 native100 at {} PC {:#x}",self.config.script,call.pc))? {
                                    host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"set_solid_color","id":payload,"color":color}]}))?)?;
                                }
                                self.tasks=proposed;
                            }
                            machine.acknowledge(None)?;
                        }
                        77 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 native task query arguments");
                            machine.acknowledge(Some(u32::from(self.tasks.exists(call.arguments[0]))))?;
                        }
                        78 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 Plane target arguments");
                            // 6758B0 drops the old object first; 676CB0 then builds one only for render type 1.
                            if self.tasks.plane(call.arguments[0]).is_ok_and(|p|p.render_type!=1){
                                machine.preflight_native(false,&[])?;
                                if let Some(plane)=self.tasks.plane_mut(call.arguments[0]){crate::native_plane::remove_payload(host,self.client,plane)?;}
                                machine.acknowledge(None)?;*self.native_calls.entry(call.id).or_default()+=1;continue;
                            }
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            let (width,height)=(f32::from_bits(call.arguments[1]),f32::from_bits(call.arguments[2]));
                            let mut payload=None;
                            if let Some(plane)=self.tasks.plane_mut(id){
                                crate::native_plane::create_target(host,self.client,plane,width,height)?;
                                payload=plane.payload;
                            }
                            #[cfg(feature="runtime")]
                            if let (Some(payload),Some(list),true)=(payload,self.tasks.main_image_list(),width>=1.&&height>=1.){
                                let publication=self.tasks.list_publication(list,self.client)?;
                                publication.plane(crate::native_image_publication::Plane{canvas:0x5c000000u32.checked_add(id).context("CatSystem2 Plane Canvas range")?,presentation:0x10000000u32.checked_add(id).context("CatSystem2 presentation ID range")?,parent:payload,source:[0,0,width as i32,height as i32]})?;
                                publication.publish(host,self.client,Vec::new())?;
                            }
                            #[cfg(not(feature="runtime"))]
                            let _=(payload,id,width,height);
                            machine.acknowledge(None)?;
                        }
                        80|83 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 Plane clear arguments");
                            machine.preflight_native(false,&[])?;
                            if let Some(plane)=self.tasks.plane_mut(call.arguments[0]){crate::native_plane::remove_payload(host,self.client,plane)?;}
                            machine.acknowledge(None)?;
                        }
                        84|85|88|93|94|101 => {
                            machine.preflight_native(false,&[])?;
                            let a=&call.arguments;
                            let word=|i:usize|a.get(i).map(|v|f32::from_bits(*v));
                            let selector=match call.id {85=>a.get(3),84|93|101=>a.get(4),_=>None}.copied().unwrap_or(0);
                            let mut source_changed=false;
                            if let Some(plane)=self.tasks.plane_mut(a[0]){
                                source_changed=selector!=0&&plane.source_pose.is_some();
                                let pose=if source_changed {plane.source_pose.as_mut().unwrap()} else {&mut plane.pose};
                                let previous=*pose;
                                match call.id{
                                    84=>for i in 0..3{if let Some(v)=word(i+1){pose.base[i]=v;}},
                                    85=>for i in 0..2{if let Some(v)=word(i+1){pose.position[i]=v;}},
                                    88=>{if let Some(v)=word(1){pose.position[2]=v;}}
                                    // 640EC0 → message55: Euler angles; the 2D layer turns about Z.
                                    93=>{if let Some(v)=word(3){pose.rotation=v;}}
                                    // 640F80 → message66: per-axis scale.
                                    101=>for i in 0..3{if let Some(v)=word(i+1){pose.scale[i]=v;}},
                                    _=>pose.alpha=(a.get(1).copied().unwrap_or(255) as i32).clamp(0,255) as u8,
                                }
                                source_changed &= *pose!=previous;
                                crate::native_plane::publish(host,self.client,plane)?;
                            }
                            #[cfg(feature="runtime")]
                            if source_changed {crate::native_plane::publish_source(host,self.client,&mut self.tasks,a[0])?;}
                            machine.acknowledge(None)?;
                        }
                        86|87|343|345 => {
                            // Messages 61, 62, 75, 76: read Plane words back through VM pointers.
                            let a=call.arguments.clone();
                            let selector=match call.id {86=>a.get(3),87=>a.get(4),_=>None}.copied().unwrap_or(0);
                            let pose=self.tasks.plane_mut(a[0]).map(|p|if selector!=0 {p.source_pose.unwrap_or(p.pose)}else{p.pose}).unwrap_or_default();
                            let values:Vec<f32>=match call.id{
                                86=>pose.position[..2].to_vec(),
                                87=>pose.base.to_vec(),
                                343=>pose.size.to_vec(),
                                _=>vec![0.,0.,pose.size[0],pose.size[1]],
                            };
                            let writes:Vec<(u32,Vec<u8>)>=a[1..].iter().zip(&values).filter(|(p,_)|**p!=0).map(|(p,v)|(*p,v.to_bits().to_le_bytes().to_vec())).collect();
                            machine.commit_native(None,&writes)?;
                        }
                        466 => {
                            // 640E10 → message82 → 67DE10: render-state word without layer geometry.
                            machine.preflight_native(false,&[])?;
                            machine.acknowledge(None)?;
                        }
                        92 => {
                            ensure!(call.arguments.len()==2&&call.arguments[1]<=1,"CatSystem2 plane visibility arguments/profile");
                            machine.preflight_native(false,&[])?;
                            if self.tasks.plane(call.arguments[0]).is_ok() {
                                let mut proposed=self.tasks.clone();
                                let messages=proposed.visibility_messages(call.arguments[0],call.arguments[1]!=0)?;
                                let commands=messages.into_iter().map(|(id,visible)|json!({"op":"set_visible","id":id,"visible":visible})).collect::<Vec<_>>();
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":commands}))?)?;
                                self.tasks=proposed;
                            }
                            machine.acknowledge(None)?;
                        }
                        71 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 event wait arguments");
                            let slot=call.arguments[0];
                            //Only validates the future16-byte message and
                            //delayed boolean slot; native71 does not clear it.
                            machine.preflight_native(true,&[(slot,vec![0;16])])?;
                            machine.acknowledge(None)?;
                            self.event_wait=Some(NativeEventWait{slot,pc:call.pc});
                            *self.native_calls.entry(call.id).or_default()+=1;
                            return Ok(VmYield::BudgetExhausted);
                        }
                        104 => {
                            ensure!(call.arguments.len()==7,"CatSystem2 click controller bind arguments");
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            let button=self.tasks.button(id)?;
                            if button.binding_target.is_none(){
                                let a=&call.arguments;let x=a[1] as i32;let y=a[2] as i32;
                                let right=x.wrapping_add(a[3] as i32);let bottom=y.wrapping_add(a[4] as i32);
                                let full_canvas=x<=0&&y<=0&&right>=self.config.canvas[0] as i32&&bottom>=self.config.canvas[1] as i32;
                                let empty=x>=right||y>=bottom;
                                let native_rect=if full_canvas{[-100000,-100000,100000,100000]}else{[x,y,right,bottom]};
                                let [left,top,right,bottom]=native_rect;
                                let rect=if empty&&!full_canvas{None}else{Some([i64::from(left),i64::from(top),i64::from(right)-i64::from(left),i64::from(bottom)-i64::from(top)])};
                                let receiver=crysirror_scene::input_routes::InputReceiverSpec{id,priority:a[5] as i32,sources:7,rect,coverage:None,enabled:true,hover_enabled:true,drag_enabled:true};
                                let mut proposed=self.tasks.clone();proposed.bind_button(id,self.tasks.current(),native_rect,a[6],full_canvas)?;
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"put","receiver":receiver}]}))?)?;
                                self.tasks=proposed;self.input_routes_bound=true;
                            }
                            machine.acknowledge(None)?;
                        }
                        105 => {
                            ensure!(call.arguments.len()==2&&call.arguments[1]<=1,"CatSystem2 click controller enabled arguments/profile");
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            if self.tasks.button(id)?.binding_target.is_some(){
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"set_enabled","id":id,"enabled":call.arguments[1]!=0}]}))?)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        848 => {
                            ensure!(call.arguments.len()==2&&call.arguments[1]<=1,"CatSystem2 hover enabled arguments/profile");
                            machine.preflight_native(false,&[])?;let id=call.arguments[0];
                            if self.tasks.button(id)?.binding_target.is_some(){
                                let mut receiver:crysirror_scene::input_routes::InputReceiverSpec=serde_json::from_slice::<Option<crysirror_scene::input_routes::InputReceiverSpec>>(&host.query_engine(self.client,EngineQuery::InputRouteReceiver(id))?)?.context("CatSystem2 engine input receiver missing")?;
                                receiver.hover_enabled=call.arguments[1]!=0;
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"put","receiver":receiver}]}))?)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        849 => {
                            ensure!(call.arguments.len()==2&&call.arguments[1]<=1,"CatSystem2 drag enabled arguments/profile");
                            machine.preflight_native(false,&[])?;let id=call.arguments[0];
                            if self.tasks.button(id)?.binding_target.is_some(){
                                let mut receiver:crysirror_scene::input_routes::InputReceiverSpec=serde_json::from_slice::<Option<crysirror_scene::input_routes::InputReceiverSpec>>(&host.query_engine(self.client,EngineQuery::InputRouteReceiver(id))?)?.context("CatSystem2 engine input receiver missing")?;
                                receiver.drag_enabled=call.arguments[1]!=0;
                                host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"input_routes":[{"op":"put","receiver":receiver}]}))?)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        117 => {
                            ensure!(call.arguments.len()==6&&call.arguments[0]==0,"CatSystem2 unsupported native117 bitmap target/profile");
                            machine.preflight_native(false,&[])?;
                            if self.texture_clear.is_none(){
                                self.texture_clear=crate::texture_cache::PendingClear::begin(self.pool.as_ref().context("CatSystem2 worker pool missing")?,host,self.client,call.pc,&call.arguments,self.texture_cache.clone())?;
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let clear=self.texture_clear.as_mut().unwrap();ensure!(clear.native_pc==call.pc,"CatSystem2 texture clear native ownership mismatch");
                            let Some(prepared)=clear.poll()? else{return Ok(VmYield::BudgetExhausted);};
                            let prepared=clear.publish(host,self.client,prepared)?;
                            machine.acknowledge(None)?;self.texture_cache=Some(prepared);self.texture_clear=None;
                        }
                        256 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 image-list initialization arguments");
                            machine.preflight_native(false,&[])?;let id=call.arguments[0];
                            ensure!(self.tasks.image_list(id)?.manager.is_none(),"CatSystem2 unsupported image-list reinitialization");
                            if self.image_pool_loading.is_none(){
                                self.image_pool_loading=crate::image_list::PendingImages::begin(self.pool.as_ref().context("CatSystem2 worker pool missing")?,host,self.client,id,call.pc,[call.arguments[1],call.arguments[2]],self.config.canvas)?;
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let pending=self.image_pool_loading.as_mut().unwrap();
                            ensure!(pending.task_id==id&&pending.native_pc==call.pc,"CatSystem2 image-list worker ownership mismatch");
                            let Some(images)=pending.poll()? else{return Ok(VmYield::BudgetExhausted);};
                            let mut proposed=self.tasks.clone();let plane=proposed.create_plane(id,b"base plane".to_vec())?;
                            let mut group=plane.layer_spec()?;group.visible=false;
                            let mut payload:crysirror_scene::layer::LayerSpec=serde_json::from_value(json!({"id":plane.layer+1,"parent":plane.layer,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"transform":{"z":1000.},"paint":{"kind":"solid","color":[0,0,0,0]}}))?;
                            let atlas=host.publish_image_resource(self.client,0x43000000+id*2,images.atlas.clone())?;
                            let base=match host.publish_image_resource(self.client,plane.layer+1,images.base.clone()){
                                Ok(base)=>base,Err(e)=>{host.release_image_resource(self.client,atlas.path.relative())?;return Err(e.into());}
                            };
                            payload.paint=crysirror_scene::layer::Paint::Image{asset:base.path.relative().to_owned(),tint:[255;4]};
                            let submit=host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":group},{"op":"put","layer":payload},{"op":"set_depth","id":plane.layer+1,"depth":1000.}]}))?);
                            if let Err(e)=submit{host.release_image_resource(self.client,base.path.relative())?;host.release_image_resource(self.client,atlas.path.relative())?;return Err(e.into());}
                            let size=images.atlas.size();let page=images.page_size;
                            proposed.bind_image_pool(id,crate::task_policy::NativeImagePool{resource:images.atlas,path:atlas.path.relative().to_owned(),size,page_size:page,page_count:size[0].div_ceil(page[0])*size[1].div_ceil(page[1]),dirty_bounds:None,upload_position:None,native_entry_keys:Default::default()},plane.id,crate::task_policy::NativePlaneImage{resource:images.base,path:base.path.relative().to_owned()})?;
                            self.tasks=proposed;self.image_pool_loading=None;machine.acknowledge(None)?;
                        }
                        260 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 image-list upload position arguments");
                            machine.preflight_native(false,&[])?;
                            self.tasks.set_image_upload_position(call.arguments[0],[call.arguments[1] as i32,call.arguments[2] as i32])?;
                            machine.acknowledge(None)?;
                        }
                        262 => {
                            ensure!(call.arguments.len()==2&&call.arguments[1]<=1,"CatSystem2 image-list presentation mode arguments/profile");
                            machine.preflight_native(false,&[])?;
                            self.tasks.set_image_mode(call.arguments[0],call.arguments[1])?;
                            machine.acknowledge(None)?;
                        }
                        448 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 FES start arguments");
                            machine.preflight_native(false,&[])?;
                            let observation=if let Some((owner,pc,observation))=self.fes_tick_input {
                                ensure!(owner==call.arguments[0]&&pc==call.pc,"CatSystem2 FES input continuation owner mismatch");
                                observation
                            }else{
                                let observation=host.input_observation(self.client)?;
                                self.fes_tick_input=Some((call.arguments[0],call.pc,observation));
                                observation
                            };
                            let tick=crate::fes_runtime::Services{pointer_glide:Some(&mut self.classes.input_task),
                string_variables:&mut self.string_variables,
                                client:self.client,tasks:&mut self.tasks,
                                interpreter:self.interpreter.as_mut().context("CatSystem2 FES interpreter unavailable")?,
                                settings:&mut self.settings,sound_definition:&mut self.sound_definition,
                                input_state:&mut self.input_state,steam_manager:&mut self.steam_manager,
                                fes_file_probe:&mut self.fes_file_probe,fes_civil_time:&mut self.fes_civil_time,fes_children:&mut self.fes_children,handles:Some(&self.handles),saves:&self.saves,
                                pool:self.pool.as_ref().context("CatSystem2 worker pool missing")?,
                            }.force_tree(host,call.arguments[0],call.pc,observation,&mut remaining)?;
                            if self.fes_children.routes_published(){self.input_routes_bound=true;}
                            match tick{
                                crate::fes_runtime::Tick::Dependency|crate::fes_runtime::Tick::Budget=>return Ok(VmYield::BudgetExhausted),
                                crate::fes_runtime::Tick::Waited=>{
                                    //654B30 completes only after the actual forced
                                    //native FES tick reaches its own wait boundary.
                                    machine.acknowledge(None)?;self.fes_tick_input=None;
                                    self.last_fes_owner_revision=Some(observation.revision);
                                }
                            }
                        }
                        450 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 FES section arguments");
                            machine.preflight_native(true,&[])?;
                            let section=crate::native_values::format_bytes(machine,call.arguments[1])?;
                            let mut proposed=self.tasks.clone();
                            let changed=proposed.set_fes_section(call.arguments[0],&section)?;
                            machine.acknowledge(Some(u32::from(changed)))?;
                            self.tasks=proposed;
                        }
                        451 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 FES save-object arguments");
                            machine.preflight_native(false,&[])?;
                            let mut interpreter=self.interpreter.as_ref().context("CatSystem2 FES Main interpreter missing")?.clone();
                            self.tasks.set_fes_save_object(call.arguments[0],call.arguments[1],&mut interpreter)?;
                            machine.acknowledge(None)?;
                            self.interpreter=Some(interpreter);
                        }
                        452 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 native task result query arguments");
                            //64FA00 -> base5DC9D0 message23 observes actual
                            //task20; missing receiver retains native -1.
                            machine.acknowledge(Some(self.tasks.task_result(call.arguments[0])))?;
                        }
                        453 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 native task result setter arguments");
                            machine.preflight_native(false,&[])?;
                            self.tasks.set_task_result(self.tasks.current(),call.arguments[0])?;
                            machine.acknowledge(None)?;
                        }
                        447 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 FES load arguments");
                            machine.preflight_native(false,&[])?;
                            let id=call.arguments[0];
                            ensure!(!self.tasks.fes(id)?.player_loaded,"CatSystem2 unsupported FES player reinitialization");
                            if self.fes_loading.is_none(){
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                                let name=std::str::from_utf8(&name).context("CatSystem2 unsupported FES filename encoding")?.to_owned();
                                let storage=host.storage(self.client)?;let client=self.client;
                                self.fes_loading=Some(crate::fes_loading::Pending::new(id,call.pc,permit.spawn(move|cancel|crate::fes_loading::load(storage.as_ref(),client,&name,cancel))?));
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let loading=self.fes_loading.as_mut().unwrap();
                            ensure!(loading.task_id==id&&loading.native_pc==call.pc,"CatSystem2 FES worker ownership mismatch");
                            let polled=match loading.poll(){
                                Ok(polled)=>polled,
                                // 756E40 keeps the task without a player when the script file is absent.
                                Err(error) if format!("{error:#}").contains("missing-load")=>{
                                    let name=crate::native_values::format_bytes(machine,call.arguments[1])?;
                                    host.append_log(self.client,&format!("CatSystem2 FES script {} is absent",String::from_utf8_lossy(&name)))?;
                                    self.fes_loading=None;
                                    machine.acknowledge(None)?;
                                    *self.native_calls.entry(call.id).or_default()+=1;
                                    continue;
                                }
                                Err(error)=>return Err(error),
                            };
                            let Some(prepared)=polled else{return Ok(VmYield::BudgetExhausted);};
                            let mut proposed=self.tasks.clone();
                            let mut interpreter=self.interpreter.as_ref().context("CatSystem2 FES Main interpreter missing")?.clone();
                            proposed.load_fes_player(id,&prepared.path,prepared.program,&mut interpreter,&self.string_variables)?;
                            proposed.publish_fes_object_root(id,host,self.client)?;
                            machine.acknowledge(None)?;
                            self.tasks=proposed;self.interpreter=Some(interpreter);self.fes_loading=None;
                        }
                        449 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 FES image-list binding arguments");
                            machine.preflight_native(false,&[])?;
                            self.tasks.bind_fes_image_list(call.arguments[0],call.arguments[1])?;
                            machine.acknowledge(None)?;
                        }
                        247 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 CSS load arguments");
                            machine.preflight_native(true,&[])?;
                            ensure!(self.saves.len()<256,"CatSystem2 CSS handle budget");
                            if self.save_loading.is_none(){
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                let name=crate::native_values::format_bytes(machine,call.arguments[0])?;
                                let name=std::str::from_utf8(&name).context("CatSystem2 unsupported save filename encoding")?.to_owned();
                                ensure!(name.len()<260,"CatSystem2 save filename extent");
                                let storage=host.storage(self.client)?;let client=self.client;
                                self.save_loading=Some(permit.spawn(move |cancel|crate::native_save::load(storage.as_ref(),client,&name,cancel))?);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(saved)=self.save_loading.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            let id=if let Some(saved)=saved{let id=self.next_save;self.next_save=id.checked_add(1).context("CatSystem2 CSS handle overflow")?;self.saves.insert(id,saved);id}else{0};
                            machine.acknowledge(Some(id))?;self.save_loading=None;
                        }
                        // 655330/6553E0/655310/6556F0 over shared storage, read-only: the module
                        // only opens game data here, and a missing file answers null.
                        29 => {
                            machine.preflight_native(true,&[])?;
                            let name=crate::native_values::format_bytes(machine,call.arguments[0])?;
                            let name=String::from_utf8_lossy(&name).replace('\\',"/");
                            let storage=host.storage(self.client)?;
                            let read=|mount|crysirror_core::path::VfsPath::new(mount,&name).ok().and_then(|path|storage.read(&path).ok());
                            let handle=match read(crysirror_core::path::Mount::Asset).or_else(||read(crysirror_core::path::Mount::Save)){
                                Some(bytes)=>{self.next_file=self.next_file.wrapping_add(1).max(1);self.open_files.insert(self.next_file,(bytes.to_vec(),0));self.next_file}
                                None=>0,
                            };
                            machine.acknowledge(Some(handle))?;
                        }
                        30 => {
                            let a=&call.arguments;
                            let count=(a[1] as usize).saturating_mul(a[2] as usize);
                            let (result,writes)=match self.open_files.get_mut(&a[3]){
                                Some((bytes,at))=>{
                                    let end=(*at+count).min(bytes.len());
                                    let chunk=bytes[*at..end].to_vec();
                                    *at=end;
                                    (chunk.len() as u32,vec![(a[0],chunk)])
                                }
                                None=>(0,vec![]),
                            };
                            machine.commit_native(Some(result),&writes)?;
                        }
                        32 => {
                            machine.preflight_native(false,&[])?;
                            self.open_files.remove(&call.arguments[0]);
                            machine.acknowledge(None)?;
                        }
                        33 => {
                            machine.preflight_native(true,&[])?;
                            machine.acknowledge(Some(0))?;
                        }
                        id if crate::native_misc::IDS.contains(&id) => {
                            let (result,writes)=crate::native_misc::apply(id,&call.arguments,machine)?;
                            machine.commit_native(result,&writes)?;
                        }
                        // 68D690/68C970: a word on the image object.
                        728 => {
                            machine.preflight_native(false,&[])?;
                            self.image_tags.insert(call.arguments[0],call.arguments[1]);
                            machine.acknowledge(None)?;
                        }
                        877 => {
                            machine.preflight_native(true,&[])?;
                            machine.acknowledge(Some(self.image_tags.get(&call.arguments[0]).copied().unwrap_or(0)))?;
                        }
                        id if crate::native_store::IDS.contains(&id) => {
                            let (result,writes)=crate::native_store::apply(id,&call.arguments,machine,&mut self.saves,&mut self.next_save,&mut self.stores)?;
                            machine.commit_native(result,&writes)?;
                        }
                        245|1031|990 => {
                            // 78B890 writes the container; 78BF20 deletes the named file.
                            machine.preflight_native(call.id!=990,&[])?;
                            if self.save_writing.is_none(){
                                let image=if call.id==990{None}else{
                                    let Some(saved)=self.saves.get(&call.arguments[0]) else{machine.acknowledge(Some(0))?;continue;};
                                    Some(saved.file_image()?)
                                };
                                let at=if call.id==990{0}else{1};
                                let name=crate::native_values::format_bytes(machine,call.arguments[at])?;
                                let name=std::str::from_utf8(&name).context("CatSystem2 unsupported save filename encoding")?.to_owned();
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve(){Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e.into())};
                                let storage=host.storage(self.client)?;
                                self.save_writing=Some(permit.spawn(move |cancel|{
                                    cancel.check()?;
                                    let write=||->Result<()>{
                                        let path=crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save,&name)?;
                                        match image{
                                            Some(image)=>storage.write_atomic(&path,&image)?,
                                            None=>match storage.remove(&path){Err(crysirror_core::Error::NotFound)=>(),other=>other?},
                                        }
                                        Ok(())
                                    };
                                    Ok(write().err().map(|e|format!("{e:#}")))
                                })?);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(result)=self.save_writing.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            self.save_writing=None;
                            if let Some(error)=&result{host.append_log(self.client,&format!("CatSystem2 save file: {error}"))?;}
                            let ok=result.is_none();
                            machine.acknowledge((call.id!=990).then_some(u32::from(ok)))?;
                        }
                        // 64A210: the string bank (VM+180, 740070) as one record of container a0 under key a1.
                        243 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 string bank save arguments");
                            machine.preflight_native(true,&[])?;
                            let saved=match (call.arguments[0],self.string_variables.serialize()){
                                (0,_)|(_,None)=>0,
                                (container,Some(bytes))=>{
                                    let save=self.saves.get_mut(&container).with_context(||format!("CatSystem2 string bank save into unknown container {container}"))?;
                                    save.append(call.arguments[1],&bytes)?;1
                                }
                            };
                            machine.acknowledge(Some(saved))?;
                        }
                        // 6496E0: the first record under key a1 replaces the whole bank (740290).
                        253 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 string bank load arguments");
                            machine.preflight_native(true,&[])?;
                            let record=(call.arguments[0]!=0).then(||self.saves.get(&call.arguments[0]).and_then(|s|s.first(call.arguments[1])).map(<[u8]>::to_vec)).flatten();
                            let loaded=match record{Some(bytes)=>{self.string_variables.deserialize(&bytes)?;1}None=>0};
                            machine.acknowledge(Some(loaded))?;
                        }
                        276 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 string variable assignment arguments");
                            machine.preflight_native(false,&[])?;
                            let bytes=machine.read_cstring(call.arguments[1],65534)?;
                            self.string_variables.assign(call.arguments[0],bytes)?;
                            machine.acknowledge(None)?;
                        }
                        277 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 string variable query arguments");
                            let value=self.string_variables.get(call.arguments[0]);
                            let mut bytes=value.unwrap_or(b"").to_vec();bytes.push(0);
                            machine.commit_native(Some(u32::from(value.is_some())),&[(call.arguments[1],bytes)])?;
                        }
                        938 => {
                            ensure!(call.arguments.is_empty(),"CatSystem2 sound config found arguments");
                            machine.preflight_native(true,&[])?;
                            if self.sound_definition.is_none() {
                                if self.sound_loading.is_none() {
                                    let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve() {Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                    let storage=host.storage(self.client)?;
                                    self.sound_loading=Some(permit.spawn(move |cancel|crate::sound_definition::load(storage.as_ref(),&||cancel.check()))?);
                                    return Ok(VmYield::BudgetExhausted);
                                }
                                let Some(definition)=self.sound_loading.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                                self.sound_definition=Some(definition);self.sound_loading=None;
                            }
                            machine.acknowledge(Some(self.sound_definition.as_ref().unwrap().found()))?;
                        }
                        209 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 interpreter constructor arguments");
                            machine.preflight_native(false,&[])?;
                            if self.interpreter.is_none() {
                                self.interpreter=Some(crate::native_interpreter::Interpreter::new(call.arguments[0])?);
                            }
                            machine.acknowledge(None)?;
                        }
                        210 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 flag query arguments");
                            machine.acknowledge(Some(self.interpreter.as_ref().map(|i|i.flag(call.arguments[0])).unwrap_or(0)))?;
                        }
                        211|212 => {
                            ensure!(call.arguments.len()==if call.id==211{2}else{1},"CatSystem2 interpreter mutation arguments");
                            machine.preflight_native(false,&[])?;
                            if let Some(interpreter)=self.interpreter.as_mut() {
                                if call.id==211 {interpreter.set_flag(call.arguments[0],call.arguments[1]);}
                                else {interpreter.set_default(call.arguments[0]);}
                            }
                            machine.acknowledge(None)?;
                        }
                        213 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 expression arguments");
                            machine.preflight_native(true,&[])?;
                            let value=if let Some(interpreter)=self.interpreter.as_mut() {
                                let mut source=machine.read_cstring(call.arguments[0],2047)?;
                                if source.last()==Some(&b'%'){source.pop();}
                                interpreter.evaluate(&source)?
                            }else{0};
                            machine.acknowledge(Some(value))?;
                        }
                        359|789 => {
                            ensure!(if call.id==359 {call.arguments.len()==4}else{call.arguments.is_empty()},"CatSystem2 native settings arguments");
                            machine.preflight_native(call.id==789,&[])?;
                            if self.settings.is_none() {
                                if self.settings_loading.is_none() {
                                    let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve() {Ok(p)=>p,Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),Err(e)=>return Err(e)};
                                    let storage=host.storage(self.client)?;
                                    self.settings_loading=Some(permit.spawn(move |cancel|crate::native_settings::load(storage.as_ref(),&||cancel.check()))?);
                                    return Ok(VmYield::BudgetExhausted);
                                }
                                let Some(settings)=self.settings_loading.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                                if call.id==359 {machine.commit_native(None,&settings.writes(&call.arguments)?)?;}else{machine.acknowledge(Some(settings.flag_count))?;}
                                self.string_variables.language_prefix=settings.language_prefix.clone();
                                self.settings=Some(settings);self.settings_loading=None;
                            } else {
                                let settings=self.settings.as_ref().unwrap();
                                if call.id==359 {machine.commit_native(None,&settings.writes(&call.arguments)?)?;}else{machine.acknowledge(Some(settings.flag_count))?;}
                            }
                        }
                        400 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 LoopPoint import arguments");
                            machine.preflight_native(false,&[])?;
                            if self.loop_loading.is_none() {
                                let permit=match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve() {
                                    Ok(p)=>p,
                                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),
                                    Err(e)=>return Err(e),
                                };
                                let name=crate::native_values::format(machine,call.arguments[1])?.replace('\\',"/");
                                let path=VfsPath::new(Mount::Asset,&name)?;ensure!(path.relative()==name,"CatSystem2 LoopPoint capability path");
                                let storage=host.storage(self.client)?;
                                self.loop_loading=Some(permit.spawn(move |cancel| {
                                    cancel.check()?;
                                    let reader=match storage.open_asset(&path) {Ok(r)=>r,Err(Error::NotFound)=>return Ok(PreparedLoops{entries:None,source:name}),Err(e)=>return Err(e.into())};
                                    let bytes=crysirror_resource::read_all(reader.as_ref(),crate::loop_policy::MAX_LOOP_FILE as u64)?;
                                    cancel.check()?;
                                    let startup=crysirror_resource::read_all(storage.open_asset(&VfsPath::new(Mount::Asset,"config/startup.xml")?)?.as_ref(),1024*1024)?;
                                    let paths=if crate::loop_policy::SoundPaths::has_config(&startup)?{
                                        let sound_path=crate::loop_policy::SoundPaths::config_name(&startup)?;
                                        let sound=crysirror_resource::read_all(storage.open_asset(&VfsPath::new(Mount::Asset,&sound_path)?)?.as_ref(),1024*1024)?;
                                        cancel.check()?;
                                        crate::loop_policy::SoundPaths::parse(&startup,&sound)?
                                    }else{crate::loop_policy::SoundPaths::legacy(&startup)?};
                                    let entries=crate::loop_policy::parse_checked(&bytes,&paths,&||cancel.check())?;
                                    cancel.check()?;Ok(PreparedLoops{entries:Some(entries),source:name})
                                })?);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(prepared)=self.loop_loading.as_mut().unwrap().try_take()? else{return Ok(VmYield::BudgetExhausted);};
                            if let Some(mut entries)=prepared.entries {
                                for entry in &mut entries {
                                    if let Some(previous)=host.audio_loop_metadata(self.client,&entry.asset)? {
                                        ensure!(previous.range.timebase==entry.range.timebase,"CatSystem2 loop timebase changed");
                                        entry.range.end_frame=previous.range.end_frame;
                                    }
                                }
                                let receipt=host.publish_audio_loop_metadata(self.client,&entries)?;
                                ensure!(receipt.client==self.client&&receipt.entries as usize>=entries.len(),"CatSystem2 loop catalog owner receipt");
                                self.loop_catalog_revision=receipt.revision;self.loop_entries=receipt.entries;
                            } else {
                                host.append_log(self.client,&format!("Can't find LoopPointFile : {}",prepared.source))?;
                            }
                            machine.acknowledge(None)?;self.loop_loading=None;
                        }
                        590 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 XML load arguments");
                            machine.preflight_native(true,&[])?;
                            if self.xml_loading.is_none() {
                                ensure!(self.documents.len()<64,"CatSystem2 XML handle budget");
                                let permit = match self.pool.as_ref().context("CatSystem2 worker pool missing")?.reserve() {
                                    Ok(p)=>p,
                                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(VmYield::BudgetExhausted),
                                    Err(e)=>return Err(e),
                                };
                                // Scripts write Windows separators; VfsPath still refuses escapes.
                                let name=crate::native_values::format(machine,call.arguments[0])?.replace('\\',"/");
                                // A directory names no document: the native open fails and answers 0.
                                if name.is_empty()||name.ends_with('/'){
                                    host.append_log(self.client,&format!("CatSystem2 XML open of directory {name:?}"))?;
                                    machine.acknowledge(Some(0))?;
                                    *self.native_calls.entry(call.id).or_default()+=1;
                                    continue;
                                }
                                let path=VfsPath::new(Mount::Asset,&name).with_context(||format!("CatSystem2 XML path {name:?}"))?;
                                let storage=host.storage(self.client)?;
                                self.xml_loading=Some(permit.spawn(move |cancel| {
                                    cancel.check()?;
                                    let reader=match storage.open_asset(&path) {
                                        Ok(r)=>r,
                                        Err(Error::NotFound)=>return Ok(None),
                                        Err(e)=>return Err(e.into()),
                                    };
                                    let bytes=crysirror_resource::read_all(reader.as_ref(),crysirror_resource::xml::MAX_XML_BYTES as u64)?;
                                    cancel.check()?;
                                    let document=XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS).with_context(||format!("CatSystem2 XML {} starting {:02x?}",path.relative(),&bytes[..bytes.len().min(32)]))?;
                                    cancel.check()?;
                                    Ok(Some(Arc::new(document)))
                                })?);
                                return Ok(VmYield::BudgetExhausted);
                            }
                            let Some(document)=self.xml_loading.as_mut().unwrap().try_take()? else {return Ok(VmYield::BudgetExhausted)};
                            self.xml_loading=None;
                            if let Some(document)=document {
                                let bytes=self.document_bytes.checked_add(document.resident_bytes()).context("CatSystem2 XML resident overflow")?;
                                ensure!(bytes<=32*1024*1024,"CatSystem2 XML resident budget");
                                let id=self.next_document;
                                let next=id.checked_add(1).context("CatSystem2 XML handle exhaustion")?;
                                // Owner publication precedes returning the real logical handle.
                                self.documents.insert(id,document);
                                self.document_bytes=bytes;
                                self.next_document=next;
                                machine.acknowledge(Some(id))?;
                            } else { machine.acknowledge(Some(0))?; }
                        }
                        591 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 XML close arguments");
                            let id=call.arguments[0];
                            if id!=0 {ensure!(self.documents.contains_key(&id),"CatSystem2 invalid XML handle {id}");}
                            machine.preflight_native(false,&[])?;
                            if let Some(doc)=self.documents.remove(&id) {self.document_bytes-=doc.resident_bytes();}
                            machine.acknowledge(None)?;
                        }
                        592|593 => {
                            ensure!(call.arguments.len()==3,"CatSystem2 XML query arguments");
                            let id=call.arguments[0];
                            let node=if id==0 {None} else {
                                let document=self.documents.get(&id).with_context(||format!("CatSystem2 invalid XML handle {id}"))?;
                                let path=crate::native_values::format(machine,call.arguments[2])?;
                                xml_node(document,&path)?
                            };
                            if call.id==592 {
                                let value=node.map(|n|crate::native_values::integer(&n.text)).unwrap_or(call.arguments[1]);
                                machine.acknowledge(Some(value))?;
                            } else if let Some(node)=node {
                                ensure!(node.text.len()<65536,"CatSystem2 XML string result budget");
                                let mut bytes=node.text.as_bytes().to_vec(); bytes.push(0);
                                machine.commit_native(None,&[(call.arguments[1],bytes)])?;
                            } else {machine.acknowledge(None)?;}
                        }
                        68 => {
                            ensure!(call.arguments.is_empty(),"CatSystem2 current-task query arguments");
                            machine.acknowledge(Some(self.tasks.current()))?;
                        }
                        8 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 virtual screen-size arguments");
                            // The reviewed native draw interface returns its
                            // configured virtual width/height, not monitor size.
                            machine.commit_native(None,&[
                                (call.arguments[0],self.config.canvas[0].to_le_bytes().to_vec()),
                                (call.arguments[1],self.config.canvas[1].to_le_bytes().to_vec())
                            ])?;
                        }
                        13|14|15|16 => {
                            ensure!(call.arguments.len()==2,"CatSystem2 native string arguments");
                            let mut bytes=if call.id==13||call.id==15 {machine.read_cstring(call.arguments[1],65534)?} else {crate::native_values::format_bytes(machine,call.arguments[1])?};
                            if call.id==13||call.id==15 {
                                let destination=u64::from(call.arguments[0]);
                                let source=u64::from(call.arguments[1]);
                                let old=if call.id==15 {machine.read_cstring(call.arguments[0],65534)?.len()}else{0};
                                let overlaps=destination<source+bytes.len() as u64+1&&source<destination+old as u64+bytes.len() as u64+2;
                                ensure!(!overlaps||(call.id==13&&destination==source),"unsupported overlapping native CatSystem2 string operation");
                            }
                            if call.id==15||call.id==16 {
                                let mut prefix=machine.read_cstring(call.arguments[0],65534)?;
                                ensure!(prefix.len()+bytes.len()<=65534,"CatSystem2 native append length budget");
                                prefix.extend(bytes);bytes=prefix;
                            }
                            // Original native copy/append terminate with a WORD zero.
                            bytes.extend([0,0]);
                            machine.commit_native(Some(call.arguments[0]),&[(call.arguments[0],bytes)])?;
                        }
                        17|18 => {
                            ensure!(call.arguments.len()==1,"CatSystem2 native string length arguments");
                            let bytes=if call.id==17 {machine.read_cstring(call.arguments[0],65534)?}else{crate::native_values::format_bytes(machine,call.arguments[0])?};
                            machine.acknowledge(Some(bytes.len() as u32))?;
                        }
                        393=>{
                            ensure!(call.arguments.len()==1,"CatSystem2 GMenuReturnTitle arguments");
                            machine.preflight_native(false,&[])?;
                            // A module whose menu resources were not reviewed has no menu installed.
                            if !self.menu_installed{machine.acknowledge(None)?;*self.native_calls.entry(call.id).or_default()+=1;continue;}
                            host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"ui":[{"op":"set_menu_enabled","id":crate::native_menu::PRODUCTION_MENU,"item":crate::native_menu::RETURN_TITLE,"enabled":call.arguments[0]!=0}]}))?)?;
                            machine.acknowledge(None)?;
                        }
                        465=>{
                            ensure!(call.arguments.is_empty()&&self.debug_key_absent,"CatSystem2 debug query requires verified absent native debug key");
                            machine.acknowledge(Some(0))?;
                        }
                        515|885|894 => {
                            machine.preflight_native(true,&[])?;
                            let handle=self.string_maps.create()?;
                            machine.acknowledge(Some(handle))?;
                        }
                        516|886|895 => {
                            machine.preflight_native(false,&[])?;
                            self.string_maps.destroy(*call.arguments.first().context("CatSystem2 string map handle")?);
                            machine.acknowledge(None)?;
                        }
                        517|887|896 => {
                            ensure!(call.arguments.len()>=3,"CatSystem2 string map set arguments");
                            machine.preflight_native(false,&[])?;
                            // 659AA0 reads the key directly; 648390/648790 format it first.
                            let key=if call.id==517{machine.read_cstring(call.arguments[1],4096)?}else{crate::native_values::format_bytes(machine,call.arguments[1])?};
                            if call.id==517{self.string_maps.insert_new(call.arguments[0],key,call.arguments[2]);}else{self.string_maps.set(call.arguments[0],key,call.arguments[2]);}
                            machine.acknowledge(None)?;
                        }
                        518 => {
                            ensure!(call.arguments.len()>=2,"CatSystem2 string map get arguments");
                            machine.preflight_native(true,&[])?;
                            let key=machine.read_cstring(call.arguments[1],4096)?;
                            machine.acknowledge(Some(self.string_maps.get(call.arguments[0],&key)))?;
                        }
                        889|898 => {
                            machine.preflight_native(false,&[])?;
                            self.string_maps.clear(*call.arguments.first().context("CatSystem2 string map handle")?);
                            machine.acknowledge(None)?;
                        }
                        890|899 => {
                            machine.preflight_native(true,&[])?;
                            let size=self.string_maps.serialized_len(*call.arguments.first().context("CatSystem2 string map handle")?);
                            machine.acknowledge(Some(size))?;
                        }
                        891|900 => {
                            ensure!(call.arguments.len()>=2,"CatSystem2 string map serialize arguments");
                            if call.arguments[1]==0{machine.acknowledge(None)?;}else{
                                let bytes=self.string_maps.serialize(call.arguments[0]).unwrap_or_default();
                                if bytes.is_empty(){machine.acknowledge(None)?;}else{machine.commit_native(None,&[(call.arguments[1],bytes)])?;}
                            }
                        }
                        901 => {
                            ensure!(call.arguments.len()>=2,"CatSystem2 string map restore arguments");
                            machine.preflight_native(false,&[])?;
                            if call.arguments[1]!=0&&call.arguments[0]!=0{
                                let bytes=machine.read_block(call.arguments[1],16*1024*1024)?;
                                self.string_maps.deserialize(call.arguments[0],&bytes)?;
                            }
                            machine.acknowledge(None)?;
                        }
                        902 => {
                            ensure!(call.arguments.len()>=3,"CatSystem2 string map key arguments");
                            match self.string_maps.key_at(call.arguments[0],call.arguments[1]).map(<[u8]>::to_vec){
                                Some(mut key)=>{key.push(0);machine.commit_native(Some(1),&[(call.arguments[2],key)])?;}
                                None=>machine.acknowledge(Some(0))?,
                            }
                        }
                        // 647110: GAME/flag local (a1 == 0) or global range a2 into two VM words;
                        // an index of 8 or more answers 0 without writing.
                        790 => {
                            ensure!(call.arguments.len()==4,"CatSystem2 flag range arguments");
                            let range=self.settings.as_ref().and_then(|s|s.flag_ranges[usize::from(call.arguments[0]!=0)].get(call.arguments[1] as usize).copied());
                            match range{
                                Some([start,count])=>machine.commit_native(Some(1),&[(call.arguments[2],start.to_le_bytes().to_vec()),(call.arguments[3],count.to_le_bytes().to_vec())])?,
                                None=>{machine.preflight_native(true,&[])?;machine.acknowledge(Some(0))?;}
                            }
                        }
                        // 999 reads a zeroed setting.
                        999 => {machine.preflight_native(true,&[])?;machine.acknowledge(Some(0))?;}
                        739 => {
                            // 64FBF0: the virtual pointer position into two VM dwords.
                            ensure!(call.arguments.len()==2,"CatSystem2 pointer query arguments");
                            let observation=host.input_observation(self.client)?;
                            self.input_state.observe(&observation)?;
                            let [x,y]=self.input_state.fes_position();
                            machine.commit_native(None,&[(call.arguments[0],x.to_le_bytes().to_vec()),(call.arguments[1],y.to_le_bytes().to_vec())])?;
                        }
                        571|572 => {machine.preflight_native(false,&[])?;machine.acknowledge(None)?;}
                        // Destructors of objects never created along this path.
                        246 if call.arguments.first()==Some(&0) => {
                            let returns=call.id==268;
                            machine.preflight_native(returns,&[])?;
                            machine.acknowledge(returns.then_some(0))?;
                        }
                        4|5 => {
                            // 64FD70 timeGetTime and 64FA40 the task clock (5DCCE0; pausing
                            // a KCS task is not modeled, so the two agree).
                            machine.preflight_native(true,&[])?;
                            let now=host.input_observation(self.client)?.time_ms as u32;
                            machine.acknowledge(Some(now))?;
                        }
                        35 => {
                            // 655680: memcpy(destination, source, count) in VM memory.
                            ensure!(call.arguments.len()==3,"CatSystem2 memory copy arguments");
                            let count=call.arguments[2] as usize;
                            ensure!(count<=crate::kcs::MAX_SCRIPT_BYTES,"CatSystem2 native memory copy budget");
                            if count==0{machine.acknowledge(None)?;}else{
                                let bytes=machine.read_bytes(call.arguments[1],count)?;
                                machine.commit_native(None,&[(call.arguments[0],bytes)])?;
                            }
                        }
                        25|38|43|44|45|46|47|49|519|540 => {
                            let (result,writes)=crate::native_strings::apply(call.id,&call.arguments,machine)?.context("CatSystem2 string native table")?;
                            machine.commit_native(result,&writes)?;
                        }
                        id if crate::native_scalar::apply(id,&call.arguments).is_some() => {
                            let result=crate::native_scalar::apply(id,&call.arguments).flatten();
                            machine.preflight_native(result.is_some(),&[])?;
                            machine.acknowledge(result)?;
                        }
                        _=>anyhow::bail!("CatSystem2 unsupported native gcall {} at {} PC {:#x} ({} argument bytes); original call remains unacknowledged",call.id,self.config.script,call.pc,call.arguments.len()*4),
                    }
                    *self.native_calls.entry(call.id).or_default() += 1;
                    if remaining == 0 {
                        return Ok(VmYield::BudgetExhausted);
                    }
                }
            }
        }
    }
}
// Original XML treats a single <document> root as an invisible wrapper.
fn xml_node<'a>(
    document: &'a XmlDocument,
    path: &str,
) -> Result<Option<&'a crysirror_resource::xml::XmlNode>> {
    let parts = path
        .split('/')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return Ok(None);
    }
    if document.root().name == "document" {
        return document.child_path(path);
    }
    if parts[0] != document.root().name {
        return Ok(None);
    }
    document.child_path(&parts[1..].join("/"))
}
impl VmAdapter for CatSystem2Vm {
    fn presentation_pending(&self) -> bool {
        // A native modal needs a newly presented UI even if a game draw was
        // interrupted. Ordinary event/timer waits do not hold this fence.
        if self.menu_state.modal_active() { return false; }
        #[cfg(feature="runtime")]
        {
            // These are synchronous native drawing/resource callbacks split
            // over shared jobs. Keep the previous completed picture while a
            // script has only changed part of its scene (e.g. scale, then
            // capture/rebind, then position). Internal captures still consume
            // the current logical scene and can finish behind this fence.
            self.surface.job.is_some()
                || self.blob_pending.is_some()
                || !self.image_loading.is_empty()
                || self.image_pool_loading.is_some()
                || self.fes_children.frames.presentation_pending()
                || !self.fes_children.panel_render.pending_owners().is_empty()
        }
        #[cfg(not(feature="runtime"))]
        { false }
    }
    fn preparation_pending(&self) -> bool {
        self.program.is_none()
    }
    fn engine_id(&self) -> &'static str {
        "catsystem2"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        _input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        let result = self.run(host, budget);
        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                let pending=self.machine.as_ref().and_then(|m|m.pending.as_ref()).map(|c|format!(" [native {} args {:x?} PC {:#x}]",c.id,c.arguments,c.pc)).unwrap_or_default();
                let message = format!("{e:#}{pending}");
                self.error = Some(message.clone());
                Err(Error::Io(message))
            }
        }
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported(
            "CatSystem2 framework snapshots are not implemented",
        ))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported(
            "CatSystem2 framework snapshots are not implemented",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        //Keep each json! expansion bounded; diagnostic growth must not
        //raise the entire crate's macro recursion limit or alter native state.
        let mut value=json!({"profile":"kcs-v2","script":self.config.script,"loaded":self.program.is_some(),"pc":self.machine.as_ref().map(|m|m.pc),"instructions":self.machine.as_ref().map(|m|m.instructions),"pending_native":self.machine.as_ref().and_then(|m|m.pending.as_ref()),"native_calls":self.native_calls,"message_log":self.tasks.logs.diagnostics(),"tasks":self.tasks.diagnostics(),"image_pool_pending":self.image_pool_loading.is_some(),"fes_loading_pending":self.fes_loading.is_some(),"texture_cache":self.texture_cache.as_ref().map(crate::texture_cache::NativeTextureCache::diagnostics),"texture_clear_pending":self.texture_clear.is_some(),"fes_file_probe_pending":self.fes_file_probe.is_some(),"fes_civil_time_pending":self.fes_civil_time.is_some(),"fes_tick_input_revision":self.fes_tick_input.map(|(_,_,o)|o.revision),"fes_owner_tick":self.fes_owner_tick.as_ref().map(|p|json!({"revision":p.observation.revision,"next":p.next,"tasks":p.tasks,"origin_pc":p.origin_pc})),"last_fes_owner_revision":self.last_fes_owner_revision,"input_observation":{"available":self.input_state.available,"revision":self.input_state.revision,"position":self.input_state.position},"steam_manager":self.steam_manager.diagnostics(),"native_event_wait":self.event_wait.as_ref().map(|w|json!({"slot":w.slot,"native_pc":w.pc,"resume":"next_owner_tick_zero_or_message_one"})),"input_routes_bound":self.input_routes_bound,"string_variables":self.string_variables.diagnostics(),"save_containers":self.saves.iter().map(|(id,c)|json!({"id":id,"container":c.diagnostics()})).collect::<Vec<_>>(),"save_container_pending":self.save_loading.is_some()});
        if std::env::var_os("CRYSIRROR_CATSYSTEM2_OBJECTS").is_some() {
            value["native_images"] = json!(self.handles.images.iter().map(|(id, frame)| json!({"id":id,"source":frame.source,"size":frame.geometry.size,"origin":frame.geometry.origin,"base":frame.geometry.base,"alpha_pixels":frame.image.rgba().chunks_exact(4).filter(|pixel|pixel[3]!=0).count(),"upload":self.handles.uploads.get(id).map(|u|u.path.relative())})).collect::<Vec<_>>());
        }
        let services=json!({"sound_definition":self.sound_definition.as_ref().map(|s|s.diagnostics()),"sound_definition_pending":self.sound_loading.is_some(),"interpreter":self.interpreter.as_ref().map(|i|i.diagnostics()),"pending_native_text":self.machine.as_ref().and_then(|m|m.pending.as_ref().and_then(|p|match p.id{66=>p.arguments.get(2),447=>p.arguments.get(1),_=>None}).and_then(|a|crate::native_values::format(m,*a).ok())),"pending_resource":self.machine.as_ref().and_then(|m|m.pending.as_ref().filter(|p|p.id==400).and_then(|p|p.arguments.get(1)).and_then(|address|crate::native_values::format(m,*address).ok())),"xml_documents":self.documents.len(),"xml_resident_bytes":self.document_bytes,"loop_catalog_revision":self.loop_catalog_revision,"loop_catalog_entries":self.loop_entries,"loop_import_pending":self.loop_loading.is_some(),"settings_source":self.settings.as_ref().map(|s|json!({"policy":"client_save_setup_xml","windows_imported":false,"gamedef_loaded":s.gamedef,"setup_loaded":s.setup,"flag_count":s.flag_count,"dirty":s.dirty,"stillread":s.configuration(b"stillread",0,0).ok()})),"settings_pending":self.settings_loading.is_some(),"error":self.error});
        value["fes_children"]=self.fes_children.diagnostics();
        #[cfg(feature="runtime")]
        { value["m2_planes"] = self.m2.diagnostics(); }
        if let serde_json::Value::Object(services)=services {
            value.as_object_mut().unwrap().extend(services);
        }
        serde_json::to_vec(&value).unwrap_or_default()
    }
}
#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    use crysirror_runtime::{model::ClientSpec, runtime::VmFactory};
    struct Factory;
    impl VmFactory for Factory {
        fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
            ensure!(
                spec.program.is_empty(),
                "CatSystem2 executes original KCS, not data VM actions"
            );
            Ok(Box::new(CatSystem2Vm::new(
                client,
                serde_json::from_value(spec.config.clone())?,
            )?))
        }
    }
    registry.register("catsystem2", std::sync::Arc::new(Factory))
}

#[cfg(test)]
mod config_tests {
    use super::*;
    #[test]
    fn native_random_seed_config_matches_strict_optional_u32_pair_contract()->Result<()> {
        let base=serde_json::json!({"script":"main.kcs","font":"default"});
        let omitted:Config=serde_json::from_value(base.clone())?;omitted.validate()?;
        assert_eq!(omitted.random_seeds,None);
        for words in [serde_json::Value::Null,serde_json::json!([0,u32::MAX]),serde_json::json!([17,19])]{
            let mut value=base.clone();value["random_seeds"]=words;
            let config:Config=serde_json::from_value(value)?;config.validate()?;
        }
        for words in [serde_json::json!([]),serde_json::json!([1]),serde_json::json!([1,2,3]),serde_json::json!([true,1]),serde_json::json!([1.0,2]),serde_json::json!([-1,0]),serde_json::json!([4294967296u64,0])]{
            let mut value=base.clone();value["random_seeds"]=words;
            assert!(serde_json::from_value::<Config>(value).is_err());
        }
        let mut unknown=base;unknown["random_source"]=serde_json::json!("time");
        assert!(serde_json::from_value::<Config>(unknown).is_err());Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_xml_document_wrapper_and_defaults_use_real_nodes() {
        let d = crysirror_resource::xml::XmlDocument::parse(
            b"<document><x><z/></x><x><y>23</y></x></document>",
        )
        .unwrap();
        assert_eq!(super::xml_node(&d, "/x//y/").unwrap().unwrap().text, "23");
        assert!(super::xml_node(&d, "document/x/y").unwrap().is_none());
        assert!(super::xml_node(&d, "").unwrap().is_none());
        let d = crysirror_resource::xml::XmlDocument::parse(b"<r><x>7</x></r>").unwrap();
        assert_eq!(super::xml_node(&d, "r/x").unwrap().unwrap().text, "7");
        assert!(super::xml_node(&d, "x").unwrap().is_none());
    }
}

#[cfg(feature="runtime")]
struct LayeredLoad {
    pc: u32,
    names: std::collections::VecDeque<(String,u32)>,
    frames: Vec<crate::native_image_resource::Prepared>,
    loading: Option<crate::native_image_resource::Pending>,
    job: Option<crysirror_core::image_processing::ImageJob>,
}

#[cfg(feature="runtime")]
enum BlobPending {
    Read(Task<Option<(String,Vec<u8>)>>),
    Decode(crate::native_image_resource::Pending),
    Recolour(crysirror_core::image_processing::ImageJob),
}
