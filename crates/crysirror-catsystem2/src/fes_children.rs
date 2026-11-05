// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original77A630/629850 event2 continuation. Only native resource, argument,
//! first-tick and ancestry policy lives here; I/O uses scoped workers and
//! graphics/input resources are one shared-engine admission batch.
use crate::{
    fes_loading,
    fes_program::PendingOperation,
    fes_runtime::{Services, Tick},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::service::{HostServices, InputObservation};
use std::collections::BTreeMap;
#[derive(Default)]
pub(crate) struct Children {
    pub sound_tags:crate::fes_sound_tags::Tags,
    pub(crate) save: Option<crate::fes_save::Job>,
    pub(crate) animations: crate::fes_image_methods::Animations,
    /// FES machine a1[500]: its own gallery register, opened by cgreg_init.
    pub(crate) cg: Option<crate::native_classes::cg_register::Register>,
    pub(crate) cg_load: Option<crysirror_tasks::Task<crate::native_classes::cg_register::Register>>,
    #[cfg(feature="runtime")]
    pub(crate) panels: crate::fes_save_panel::Panels,
    #[cfg(feature="runtime")]
    pub(crate) panel_render: crate::fes_panel_render::Renders,
    #[cfg(feature="runtime")]
    pub(crate) rule: Option<crate::native_image_resource::Pending>,
    by_owner: BTreeMap<u32, Pending>,
    forced: BTreeMap<u32, ForcedPhase>,
    routes_published: bool,
    #[cfg(feature="runtime")]
    pub(crate) frames:crate::fes_live_frames::Frames,
    #[cfg(feature="runtime")]
    pub(crate) motions:crate::fes_live_motions::Motions,
}
struct ForcedPhase {
    pc: u32,
    observation: InputObservation,
    tasks: Vec<u32>,
    next: usize,
}
struct Pending {
    operation: PendingOperation,
    pc: u32,
    observation: InputObservation,
    parameters: [u32; 16],
    loading: fes_loading::Pending,
    child: Option<u32>,
}
impl Children {
    pub fn validate_owner(&self, owner: u32, pc: u32, observation: InputObservation) -> Result<()> {
        if let Some(pending) = self.by_owner.get(&owner) {
            ensure!(
                pending.pc == pc && pending.observation == observation,
                "CatSystem2 execfes pending owner/input cannot change"
            );
        }
        if let Some(phase) = self.forced.get(&owner) {
            ensure!(
                phase.pc == pc && phase.observation == observation,
                "CatSystem2 forced task phase owner/input cannot change"
            );
        }
        Ok(())
    }
    pub fn routes_published(&self) -> bool {
        self.routes_published
    }
    pub fn pending_owners(&self) -> Vec<u32> {
        let mut owners=self.by_owner.keys().copied().collect::<Vec<_>>();
        #[cfg(feature="runtime")]
        owners.extend(self.frames.protected_owners());
        #[cfg(feature="runtime")]
        owners.extend(self.panel_render.pending_owners());
        #[cfg(feature="runtime")]
        owners.extend(self.motions.pending_owners());
        owners
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let loads = serde_json::json!(self
            .by_owner
            .iter()
            .map(|(owner, p)| serde_json::json!({
                "owner":owner,"origin_pc":p.pc,"revision":p.observation.revision,
                "source_byte":p.operation.byte_offset,"child":p.child,
                "phase":if p.child.is_some(){"forced_first_tick"}else{"resource_load"}
            }))
            .collect::<Vec<_>>());
        #[cfg(feature="runtime")]
        let audio=self.frames.audio_diagnostics();
        #[cfg(not(feature="runtime"))]
        let audio=serde_json::Value::Null;
        serde_json::json!({"loads":loads,"audio":audio,"forced":self.forced.iter().map(|(owner,p)|serde_json::json!({"owner":owner,"pc":p.pc,"revision":p.observation.revision,"next":p.next,"tasks":p.tasks})).collect::<Vec<_>>()})
    }
}
impl Services<'_> {
    ///333/448 execute the original descendant base phase before the target's
    ///own script. Quotas and resource dependencies retain postorder progress,
    ///rather than rerunning an already completed child on every owner poll.
    pub(crate) fn force_tree(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        pc: u32,
        observation: InputObservation,
        budget: &mut u32,
    ) -> Result<Tick> {
        ensure!(
            observation.client == self.client,
            "CatSystem2 forced task phase client mismatch"
        );
        self.fes_children.validate_owner(owner, pc, observation)?;
        #[cfg(feature="runtime")]
        self.fes_children.frames.observe_audio(host,self.client,observation)?;
        if !self.fes_children.forced.contains_key(&owner) {
            ensure!(
                self.fes_children.forced.len() < 64,
                "CatSystem2 forced task phase depth"
            );
            let mut proposed = self.tasks.clone();
            let tasks = proposed.forced_fes_phase(owner)?;
            *self.tasks = proposed;
            self.fes_children.forced.insert(
                owner,
                ForcedPhase {
                    pc,
                    observation,
                    tasks,
                    next: 0,
                },
            );
        }
        loop {
            let phase = self.fes_children.forced.get(&owner).unwrap();
            if phase.next == phase.tasks.len() {
                self.fes_children.forced.remove(&owner);
                return Ok(Tick::Waited);
            }
            if *budget == 0 {
                return Ok(Tick::Budget);
            }
            let id = phase.tasks[phase.next];
            let task = self.tasks.fes(id)?;
            let (dead, paused, first, running) = (
                task.deletion_pending,
                task.paused,
                task.first_tick,
                task.running,
            );
            if dead {
                let mut protected = self.fes_children.pending_owners();
                if let Some(p) = &self.fes_file_probe {
                    protected.push(p.owner);
                }
                if let Some(p) = &self.fes_civil_time {
                    protected.push(p.owner);
                }
                self.tasks.retire_fes_subtree(
                    id,
                    host,
                    self.client,
                    Some(&mut *self.interpreter),
                    &protected,
                )?;
                #[cfg(feature="runtime")]
                self.fes_children.frames.reap(host,self.client,self.tasks)?;
                #[cfg(feature="runtime")]
                self.fes_children.motions.reap(self.tasks);
            } else if !paused {
                if first {
                    let mut proposed = self.tasks.clone();
                    let receiver = proposed.initialize_fes_task(id)?;
                    host.submit_engine(
                        self.client,
                        1,
                        &serde_json::to_vec(
                            &serde_json::json!({"input_routes":[{"op":"put","receiver":receiver}]}),
                        )?,
                    )?;
                    *self.tasks = proposed;
                    self.fes_children.routes_published = true;
                }
                if running {
                    match self.run(host, id, pc, observation, budget)? {
                        Tick::Budget => return Ok(Tick::Budget),
                        Tick::Dependency => return Ok(Tick::Dependency),
                        Tick::Waited => {}
                    }
                }
            }
            self.fes_children.forced.get_mut(&owner).unwrap().next += 1;
            *budget = budget.saturating_sub(1);
        }
    }
    ///None means the ORIGINAL parent operation can now be acknowledged. A
    ///dependency/budget result retains the same observation, args and original
    ///source operation across all polls, including nested forced-first-ticks.
    pub(crate) fn exec_fes(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        pc: u32,
        operation: &PendingOperation,
        observation: InputObservation,
        budget: &mut u32,
    ) -> Result<Option<Tick>> {
        if !self.fes_children.by_owner.contains_key(&owner) {
            ensure!(
                self.fes_children.by_owner.len() < 64,
                "CatSystem2 nested FES continuation depth"
            );
            ensure!(
                operation.arguments.len() >= 1 && operation.arguments.len() <= 17,
                "CatSystem2 execfes filename/16 argument profile"
            );
            ensure!(
                self.tasks
                    .fes(owner)?
                    .child_tasks
                    .iter()
                    .all(Option::is_none),
                "CatSystem2 execfes while another native child is active"
            );
            let name = std::str::from_utf8(&operation.arguments[0])
                .context("CatSystem2 execfes unsupported filename encoding")?
                .to_owned();
            let mut interpreter = self.interpreter.clone();
            let mut parameters = [0; 16];
            for (destination, source) in parameters.iter_mut().zip(&operation.arguments[1..]) {
                *destination = interpreter.evaluate(source)?;
            }
            let permit = match self.pool.reserve() {
                Ok(p) => p,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {
                    return Ok(Some(Tick::Dependency))
                }
                Err(e) => return Err(e),
            };
            let storage = host.storage(self.client)?;
            let client = self.client;
            let loading = fes_loading::Pending::new(
                owner,
                pc,
                permit.spawn(move |cancel| {
                    fes_loading::load(storage.as_ref(), client, &name, cancel)
                })?,
            );
            self.fes_children.by_owner.insert(
                owner,
                Pending {
                    operation: operation.clone(),
                    pc,
                    observation,
                    parameters,
                    loading,
                    child: None,
                },
            );
            *self.interpreter = interpreter;
            return Ok(Some(Tick::Dependency));
        }
        let pending = self.fes_children.by_owner.get_mut(&owner).unwrap();
        ensure!(
            pending.pc == pc
                && pending.operation == *operation
                && pending.observation == observation,
            "CatSystem2 execfes continuation owner/source/input changed"
        );
        let child = if let Some(child) = pending.child {
            child
        } else {
            let Some(prepared) = pending.loading.poll()? else {
                return Ok(Some(Tick::Dependency));
            };
            let parameters = pending.parameters;
            let image_list = self
                .tasks
                .fes(owner)?
                .image_list
                .context("CatSystem2 execfes parent has no real image-list binding")?;
            let manager = self
                .tasks
                .image_list(image_list)?
                .manager
                .as_ref()
                .context("CatSystem2 execfes image-list manager missing")?;
            ensure!(
                manager.resource.client() == self.client,
                "CatSystem2 execfes foreign shared image-list resource"
            );
            let mut proposed = self.tasks.clone();
            let mut interpreter = self.interpreter.clone();
            let child = proposed.create_fes(owner, b"FE Script".to_vec())?;
            proposed.load_fes_player(child, &prepared.path, prepared.program, &mut interpreter,&*self.string_variables)?;
            proposed.bind_fes_image_list(child, image_list)?;
            for (index, value) in parameters.into_iter().enumerate() {
                ensure!(
                    interpreter.set_player_local(child, index as u32, value)?,
                    "CatSystem2 child raw parameter bank extent"
                );
            }
            let root = proposed.prepare_fes_object_root(child)?;
            let receiver = proposed.initialize_fes_task(child)?;
            //Layer+input registration either both exist or neither exists. No
            //native pointer, composed screen, draw algorithm or callback code.
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&serde_json::json!({
                    "layers":[{"op":"put","layer":root}],
                    "input_routes":[{"op":"put","receiver":receiver}]
                }))?,
            )?;
            *self.tasks = proposed;
            *self.interpreter = interpreter;
            self.fes_children.routes_published = true;
            self.fes_children.by_owner.get_mut(&owner).unwrap().child = Some(child);
            child
        };
        match self.force_tree(host, child, pc, observation, budget)? {
            Tick::Budget => return Ok(Some(Tick::Budget)),
            Tick::Dependency => return Ok(Some(Tick::Dependency)),
            Tick::Waited => {}
        }
        //629850 stores106 ONLY after333's actual first tick has returned.
        //Do not publish the child reference early or treat worker-ready as done.
        self.tasks.attach_fes_child_after_tick(owner, child)?;
        ensure!(
            self.interpreter.select_player_bank(owner),
            "CatSystem2 execfes parent local bank disappeared"
        );
        self.fes_children.by_owner.remove(&owner);
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_interpreter::Interpreter,
        native_settings::NativeSettings,
        task_policy::{NativeImagePool, NativePlaneImage, NativeTaskPolicy},
    };
    use crysirror_core::{
        image_processing::{ImageInput, ImageProcessState},
        state::ClientId,
    };
    use std::{
        io::Write,
        sync::Arc,
        time::{Duration, Instant},
    };
    const CLIENT: ClientId = ClientId(1);
    fn observation() -> InputObservation {
        InputObservation {
            client: CLIENT,
            revision: 1,
            time_ms: 0,
            buttons_down: 0,
            buttons_pressed: 0,
            pointer: Some([0, 0]),
            pointer_inside: Some(true),
            focused: Some(true),
        }
    }
    struct Fixture {
        runtime: crysirror_runtime::Runtime,
        tasks: NativeTaskPolicy,
        interpreter: Interpreter,
        owner: u32,
        settings: Option<NativeSettings>,
        sound: Option<crate::sound_definition::SoundDefinition>,
        input: crate::native_input::InputState,
        steam: crate::native_platform::SteamManager,
        probe: Option<crate::fes_file_probe::Pending>,
        civil: Option<crate::fes_civil_time::Pending>,
        children: Children,
        strings: crate::native_values::StringVariables,
        pool: crysirror_tasks::TaskPool,
        _root: tempfile::TempDir,
    }
    impl Fixture {
        #[cfg(feature="runtime")]
        fn motion(&self,name:&str,code:&[u8])->Result<()> {
            let startup=self._root.path().join("config/startup.xml");
            let xml=std::fs::read_to_string(&startup)?.replace("</INT>","<type><name>mot</name><int>mot.int</int><ext>kcs</ext><dir>mot</dir></type></INT>");
            std::fs::write(startup,xml)?;
            std::fs::create_dir_all(self._root.path().join("mot"))?;
            let mut encoder=flate2::write::ZlibEncoder::new(Vec::new(),flate2::Compression::default());encoder.write_all(code)?;
            let packed=encoder.finish()?;let n=u32::try_from(code.len())?;
            let mut bytes=Vec::new();
            for word in [u32::from_le_bytes(*b"KCS\0"),2,0,52,n,52+n,0,0,0,1024,52+n,0,n]{bytes.extend(word.to_le_bytes());}
            bytes.extend(packed);std::fs::write(self._root.path().join("mot").join(format!("{name}.kcs")),bytes)?;Ok(())
        }
        fn new(child: &[u8]) -> Result<Self> {
            Self::with_parent(
                b"#START\nexecfes child ++$0 5\n$3=_EXITCODE_\nwait 2",
                child,
            )
        }
        fn with_parent(parent: &[u8], child: &[u8]) -> Result<Self> {
            let root = tempfile::tempdir()?;
            std::fs::create_dir(root.path().join("config"))?;
            std::fs::create_dir(root.path().join("fes.int"))?;
            std::fs::write(root.path().join("config/startup.xml"),b"<startup><INT><type><name>fes</name><int>fes.int</int><ext>fes</ext></type></INT></startup>")?;
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(child)?;
            let packed = encoder.finish()?;
            let mut container = b"FES\0".to_vec();
            for value in [packed.len() as u32, child.len() as u32, 0] {
                container.extend(value.to_le_bytes());
            }
            container.extend(packed);
            std::fs::write(root.path().join("fes.int/child.fes"), container)?;
            let project = serde_json::from_value(
                serde_json::json!({"schema":2,"name":"native child lifecycle","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"unused"}]}),
            )?;
            let mut runtime = crysirror_runtime::Runtime::with_asset_root(
                project,
                root.path(),
                &root.path().join("state"),
                Default::default(),
            )?;
            let mut tasks = NativeTaskPolicy::bootstrap()?;
            let owner = tasks.create_fes(3, b"parent".to_vec())?;
            let program = Arc::new(crate::fes_program::Program::parse(parent, &|| Ok(()))?);
            let mut interpreter = Interpreter::new(16)?;
            tasks.load_fes_player(owner, "fes.int/parent.fes", program, &mut interpreter,&Default::default())?;
            tasks.publish_fes_object_root(owner, &mut runtime.host, CLIENT)?;
            //A real shared worker image and published resource back the original
            //image-list binding. No fake ImageResource/renderer is used here.
            let images = runtime.host.image_metadata(CLIENT)?;
            let mut job = images.begin_process(
                ImageInput::Solid {
                    size: [4, 4],
                    color: [0; 4],
                },
                Arc::from(b"[]".as_slice()),
            )?;
            let start = Instant::now();
            let image = loop {
                match job.poll()? {
                    ImageProcessState::Ready(image) => break image,
                    ImageProcessState::Failed(e) => anyhow::bail!("{e}"),
                    ImageProcessState::Pending => {
                        ensure!(
                            start.elapsed() < Duration::from_secs(5),
                            "authored shared image timeout"
                        );
                        std::thread::yield_now();
                    }
                }
            };
            let published = runtime
                .host
                .publish_image_resource(CLIENT, 100, image.clone())?;
            let list = tasks.create_image_list(3, b"images".to_vec())?;
            let base = tasks.create_plane(list.id, b"base".to_vec())?;
            let path = published.path.relative().to_owned();
            runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":base.layer_spec()?},{"op":"put","layer":{"id":base.layer+1,"parent":base.layer,"rect":[0,0,4,4],"paint":{"kind":"image","asset":path}}}]}))?)?;
            tasks.bind_image_pool(
                list.id,
                NativeImagePool {
                    resource: image.clone(),
                    path: path.clone(),
                    size: [4, 4],
                    page_size: [4, 4],
                    page_count: 1,
                    dirty_bounds: None,
                    upload_position: Some([0, 0]),
                    native_entry_keys: Default::default(),
                },
                base.id,
                NativePlaneImage {
                    resource: image,
                    path,
                },
            )?;
            tasks.bind_fes_image_list(owner, list.id)?;
            let receivers = tasks.initialize_fes_tasks([4, 4])?;
            runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"input_routes":receivers.into_iter().map(|receiver|serde_json::json!({"op":"put","receiver":receiver})).collect::<Vec<_>>()}))?)?;
            Ok(Self {
                runtime,
                tasks,
                interpreter,
                owner,
                settings: Some(NativeSettings::constructor()),
                sound: None,
                input: Default::default(),
                steam: Default::default(),
                probe: None,
                civil: None,
                children: Default::default(),
                strings: Default::default(),
                pool: crysirror_tasks::TaskPool::new("authored-native-child", 1, 2)?,
                _root: root,
            })
        }
        fn run(&mut self, input: InputObservation, budget: &mut u32) -> Result<Tick> {
            self.run_owner(self.owner, input, budget)
        }
        fn run_owner(
            &mut self,
            owner: u32,
            input: InputObservation,
            budget: &mut u32,
        ) -> Result<Tick> {
            self.drive(owner, false, input, budget)
        }
        fn force(&mut self, input: InputObservation, budget: &mut u32) -> Result<Tick> {
            self.drive(self.owner, true, input, budget)
        }
        fn drive(
            &mut self,
            owner: u32,
            forced: bool,
            input: InputObservation,
            budget: &mut u32,
        ) -> Result<Tick> {
            let mut services = Services {
                string_variables:&mut self.strings,
                client: CLIENT,
                tasks: &mut self.tasks,
                interpreter: &mut self.interpreter,
                settings: &mut self.settings,
                sound_definition: &mut self.sound,
                input_state: &mut self.input,
                steam_manager: &mut self.steam,
                fes_file_probe: &mut self.probe,
                fes_civil_time: &mut self.civil,
                fes_children: &mut self.children,handles:None,saves:&Default::default(),
                pool: &self.pool,
            };
            if forced {
                services.force_tree(&mut self.runtime.host, owner, 0x1943, input, budget)
            } else {
                services.run(&mut self.runtime.host, owner, 0x1943, input, budget)
            }
        }
    }
    #[test]
    fn parent_and_worker_loaded_child_share_the_attached_original_string_bank() -> Result<()> {
        let mut fixture=Fixture::with_parent(
            b"#START\nstrvar 10 child\nstrvar 1 7\nexecfes $str10\nstrvar 4 $str3\n$2=$str4\nwait 2",
            b"#START\nstrvar 2 %04d 8\n$1=$str1\nstrvar get 3 2\nwait 2")?;
        let start=Instant::now();
        loop {if matches!(fixture.run(observation(),&mut 100)?,Tick::Waited){break;}
            ensure!(start.elapsed()<Duration::from_secs(5),"authored string child timeout");std::thread::yield_now();}
        assert_eq!(fixture.interpreter.flag(1),7);
        assert_eq!(fixture.strings.get(2),Some(b"0008".as_slice()));
        assert_eq!(fixture.strings.get(3),Some(b"0008".as_slice()));
        assert_eq!(fixture.strings.get(4),Some(b"0008".as_slice()));
        assert_eq!(fixture.interpreter.flag(2),8);
        assert!(fixture.tasks.fes(fixture.owner)?.child_tasks[0].is_some());
        Ok(())
    }
    #[test]
    fn native_child_resource_and_forced_first_tick_precede_parent_reference_and_result_sample(
    ) -> Result<()> {
        let mut fixture = Fixture::new(b"#START\n$2=$$0+$$1\nexit 7\nwait 1")?;
        assert!(matches!(
            fixture.run(observation(), &mut 100)?,
            Tick::Dependency
        ));
        assert_eq!(fixture.interpreter.flag(0), 1);
        assert_eq!(fixture.tasks.fes(fixture.owner)?.child_tasks, [None, None]);
        let start = Instant::now();
        loop {
            let outcome = fixture.run(observation(), &mut 100)?;
            if matches!(outcome, Tick::Waited) {
                break;
            }
            ensure!(
                start.elapsed() < Duration::from_secs(5),
                "authored child continuation timeout"
            );
            std::thread::yield_now();
        }
        let child = fixture.tasks.fes(fixture.owner)?.child_tasks[0]
            .context("child reference missing after actual first tick")?;
        assert_eq!(fixture.interpreter.flag(0), 1);
        assert_eq!(fixture.interpreter.flag(2), 6);
        //Original parent code runs before its end-of-frame23 child query.
        assert_eq!(fixture.interpreter.flag(3), u32::MAX);
        assert_eq!(fixture.interpreter.evaluate(b"_EXITCODE_")?, 7);
        assert_eq!(fixture.tasks.task_result(child), 7);
        assert!(fixture.tasks.fes(child)?.running);
        assert!(fixture.children.by_owner.is_empty());
        let root = fixture
            .tasks
            .fes(child)?
            .object_root
            .context("child shared BASE missing")?;
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(CLIENT, root)
            .is_some());
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(ClientId(2), root)
            .is_none());
        let receiver = fixture
            .tasks
            .fes(child)?
            .low_button
            .context("child input resource missing")?;
        assert_eq!(
            fixture
                .runtime
                .host
                .engine
                .input_routes
                .receiver(CLIENT, receiver)
                .unwrap()
                .priority,
            3
        );
        Ok(())
    }
    #[test]
    fn unsupported_child_declarations_are_cached_without_a_fake_player_or_replayed_arguments(
    ) -> Result<()> {
        let mut fixture = Fixture::new(b"#DEFINE\nRDRAW real_object\n#START\nwait 1")?;
        let before = fixture.tasks.diagnostics()["allocated"].clone();
        assert!(matches!(
            fixture.run(observation(), &mut 100)?,
            Tick::Dependency
        ));
        let start = Instant::now();
        let error = loop {
            match fixture.run(observation(), &mut 100) {
                Err(error) => break format!("{error:#}"),
                Ok(_) => {
                    ensure!(
                        start.elapsed() < Duration::from_secs(5),
                        "authored child rejection timeout"
                    );
                    std::thread::yield_now();
                }
            }
        };
        assert!(error.contains("native class 26 still needs shared resource publication"));
        for _ in 0..2 {
            assert_eq!(
                format!("{:#}", fixture.run(observation(), &mut 100).err().unwrap()),
                error
            );
        }
        assert_eq!(fixture.interpreter.flag(0), 1);
        assert_eq!(fixture.interpreter.diagnostics()["local_banks"], 1);
        assert_eq!(fixture.tasks.diagnostics()["allocated"], before);
        assert_eq!(fixture.tasks.fes(fixture.owner)?.child_tasks, [None, None]);
        assert!(fixture
            .tasks
            .fes(fixture.owner)?
            .player
            .as_ref()
            .unwrap()
            .pending
            .as_ref()
            .is_some_and(|p| p.name == b"execfes"));
        Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn typed_child_executes_script_then_waits_for_real_class_frame_and_reuses_leases()->Result<()> {
        let mut fixture=Fixture::new(b"#DEFINE\nPLANE view\nCIMAGE sprite\n#OBJECT SIZE COLOR PRI DISP\nview 4,4 0xffffffff 1 1\nsprite 2,2 0xff03070b 3 1\n#OBJECT POS PL\nsprite 1,1 view\n#START\n$6+=1\nsprite disp 0\nsprite size 3 2\nsprite pri 7\nwait 1\nsprite disp 1\nwait 1")?;
        let list=fixture.tasks.fes(fixture.owner)?.image_list.unwrap();
        fixture.tasks.set_image_upload_position(list,[0,0])?;
        let start=Instant::now();let mut dependencies=0;
        loop{
            match fixture.run(observation(),&mut 100)? {
                Tick::Waited=>break,
                Tick::Dependency=>dependencies+=1,
                Tick::Budget=>{},
            }
            ensure!(start.elapsed()<Duration::from_secs(5),"typed original child frame timeout");
            std::thread::yield_now();
        }
        assert!(dependencies>0,"real worker dependencies must retain native execfes");
        assert_eq!(fixture.interpreter.flag(0),1,"execfes arguments execute once");
        assert_eq!(fixture.interpreter.flag(6),1,"resource polls do not replay script expressions");
        let child=fixture.tasks.fes(fixture.owner)?.child_tasks[0].context("typed child not attached after actual frame")?;
        let receipt=fixture.children.frames.ready.get(&child).context("typed frame has no real receipt")?;
        let resource=receipt.resources.values().next().context("solid class lease absent")?.path.clone();
        let canvas=receipt.list.scope().1;
        let surface=fixture.runtime.host.engine.canvases.surface(CLIENT,canvas).context("class atlas was not published")?;
        let crysirror_scene::canvas::CanvasNode::Draw{layer}=&surface.nodes[0]else{anyhow::bail!("native class is not an engine draw node")};
        assert_eq!(layer.rect,[1.,1.,3.,2.]);assert!(!layer.visible);
        assert!(fixture.tasks.fes(fixture.owner)?.player.as_ref().unwrap().pending.is_none(),"execfes ACK follows class publication");
        let mut next=observation();next.revision=2;next.time_ms=17;
        assert!(matches!(fixture.run_owner(child,next,&mut 100)?,Tick::Waited));
        let receipt=&fixture.children.frames.ready[&child];
        assert_eq!(receipt.resources.values().next().unwrap().path,resource,"pose-only frame keeps the same image lease/generation");
        let surface=fixture.runtime.host.engine.canvases.surface(CLIENT,canvas).unwrap();
        let crysirror_scene::canvas::CanvasNode::Draw{layer}=&surface.nodes[0]else{unreachable!()};
        assert!(layer.visible);assert_eq!(surface.nodes.len(),1,"old draw ID is not left in the engine atlas");
        let child_root=fixture.tasks.fes(child)?.object_root.unwrap();
        let view=fixture.tasks.fes(child)?.object_bank.as_ref().unwrap().lookup(&crate::fes_references::IndexedName{name:b"view".to_vec(),index:0}).unwrap();
        let view_canvas=0x5c000000+view;
        assert!(fixture.runtime.host.engine.canvases.surface(CLIENT,view_canvas).is_some());
        fixture.tasks.end_fes_child(fixture.owner,0,&mut fixture.interpreter)?;
        fixture.tasks.retire_fes_subtree(child,&mut fixture.runtime.host,CLIENT,Some(&mut fixture.interpreter),&[])?;
        fixture.children.frames.reap(&mut fixture.runtime.host,CLIENT,&fixture.tasks)?;
        assert!(fixture.tasks.fes(child).is_err());
        assert!(fixture.runtime.host.engine.layers.get(CLIENT,child_root).is_none());
        assert!(fixture.runtime.host.engine.canvases.surface(CLIENT,view_canvas).is_none());
        assert!(fixture.runtime.host.engine.canvases.surface(CLIENT,canvas).unwrap().nodes.is_empty());
        assert!(!fixture.children.frames.ready.contains_key(&child));
        Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn original_baseimg_bitmap_plane_and_hide_use_shared_copies_and_preserve_native_payload()->Result<()> {
        let mut fixture=Fixture::new(b"#DEFINE\nPLANE view\nCIMAGE sprite\n#OBJECT SIZE COLOR PRI DISP\nview 4,4 0xffffffff 1 1\nsprite 2,2 0xff03070b 3 1\n#OBJECT POS PL\nsprite 1,1 view\n#START\nwait 1\nbaseimg sprite\nsprite size 3 2\nwait 1\nbaseimg view\nwait 1\nbaseimg\nwait 1")?;
        let list=fixture.tasks.fes(fixture.owner)?.image_list.unwrap();fixture.tasks.set_image_upload_position(list,[0,0])?;
        let base=fixture.tasks.plane(fixture.tasks.image_list(list)?.base_plane.unwrap())?.clone();let payload=base.payload.unwrap();
        fixture.runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[
            {"op":"set_transform","id":payload,"transform":{"z":12,"rotate":0.15}},
            {"op":"begin_external_motion","id":payload,"driver":77}
        ]}))?)?;
        let before=fixture.runtime.host.engine.layers.get(CLIENT,payload).unwrap().transform.clone();
        let stamp=fixture.runtime.host.engine.layers.motion_status(CLIENT,payload)?.drivers[0].stamp;
        let started=Instant::now();
        while !matches!(fixture.run(observation(),&mut 100)?,Tick::Waited){ensure!(started.elapsed()<Duration::from_secs(5),"baseimg first class-frame timeout");std::thread::yield_now();}
        let child=fixture.tasks.fes(fixture.owner)?.child_tasks[0].unwrap();
        let old=fixture.children.frames.ready[&child].resources.values().next().unwrap().path.clone();
        let run_frame=|fixture:&mut Fixture,revision:u64,time_ms:u64|->Result<()> {
            let mut sample=observation();sample.revision=revision;sample.time_ms=time_ms;
            let start=Instant::now();loop{if matches!(fixture.run_owner(child,sample,&mut 100)?,Tick::Waited){break;}
                ensure!(start.elapsed()<Duration::from_secs(5),"baseimg resource continuation timeout");std::thread::yield_now();}Ok(())
        };
        run_frame(&mut fixture,2,17)?;
        let bitmap=0x5e000000+list;
        let frozen=fixture.runtime.host.engine.canvases.surface(CLIENT,bitmap).context("baseimg did not copy native bitmap into engine Canvas")?;
        assert_eq!([frozen.width,frozen.height],[2,2]);
        assert!(serde_json::to_string(frozen)?.contains(old.relative()),"bitmap copy must reference the actual original image lease");
        let current=fixture.children.frames.ready[&child].resources.values().next().unwrap().path.clone();
        assert_ne!(old,current,"changed CIMAGE size must publish a fresh generation");
        let retained=fixture.runtime.assets.surface(CLIENT,old.relative()).context("shared retirement lost the frozen bitmap reader")?;
        assert_eq!([retained.width,retained.height],[2,2]);
        assert_eq!(&retained.rgba[..4],&[3,7,11,255]);
        assert_eq!(fixture.tasks.image_list(list)?.field_132,1);
        assert!(fixture.runtime.host.engine.layers.get(CLIENT,base.layer).unwrap().visible);
        run_frame(&mut fixture,3,34)?;
        let copied=fixture.runtime.host.engine.canvases.surface(CLIENT,bitmap).unwrap();
        assert_eq!([copied.width,copied.height],[4,4]);
        assert!(serde_json::to_string(copied)?.contains(current.relative()),"Plane capture must include the real current ImageList draw");
        run_frame(&mut fixture,4,51)?;
        assert_eq!(fixture.tasks.image_list(list)?.field_132,0);
        assert!(!fixture.runtime.host.engine.layers.get(CLIENT,base.layer).unwrap().visible);
        assert_eq!(fixture.runtime.host.engine.canvases.surface(CLIENT,0x58000000+list).unwrap().nodes.len(),1,"baseimg without args removes bitmap0 only, preserving the native sprite");
        assert!(fixture.runtime.host.engine.canvases.surface(CLIENT,bitmap).is_some(),"native hidden copied bitmap remains allocated");
        assert_eq!(fixture.runtime.host.engine.layers.get(CLIENT,payload).unwrap().transform,before);
        assert_eq!(fixture.runtime.host.engine.layers.motion_status(CLIENT,payload)?.drivers[0].stamp,stamp,"Canvas presentation must not Put-replace the native payload producer");
        Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn original_baseimg_pri_keeps_signed_default_position_depth_and_existing_producer()->Result<()> {
        let mut fixture=Fixture::with_parent(b"#START\nbaseimg_pri -3\nwait 1\nbaseimg_pri\nwait 1\nbaseimg_pri 200\nwait 1",b"#START\nwait 1")?;
        let list=fixture.tasks.fes(fixture.owner)?.image_list.unwrap();
        let base=fixture.tasks.plane(fixture.tasks.image_list(list)?.base_plane.unwrap())?.clone();let payload=base.payload.unwrap();
        let initially_visible=fixture.runtime.host.engine.layers.get(CLIENT,base.layer).unwrap().visible;
        fixture.runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[
            {"op":"set_rect","id":payload,"rect":[3,4,4,4]},
            {"op":"set_transform","id":payload,"transform":{"rotate":0.2}},
            {"op":"begin_external_motion","id":payload,"driver":78}
        ]}))?)?;
        let stamp=fixture.runtime.host.engine.layers.motion_status(CLIENT,payload)?.drivers[0].stamp;
        for (revision,time,z,visible) in [(1,0,-3.,false),(2,17,1000.,true),(3,34,200.,true)] {
            let mut sample=observation();sample.revision=revision;sample.time_ms=time;
            let owner=fixture.owner;
            assert!(matches!(fixture.run_owner(owner,sample,&mut 100)?,Tick::Waited));
            let layer=fixture.runtime.host.engine.layers.get(CLIENT,payload).unwrap();
            assert_eq!(&layer.rect[..2],&[0.,0.]);assert_eq!(layer.transform.z,z);
            assert_eq!(fixture.runtime.host.engine.layers.depth(CLIENT,payload)?,Some(z));assert_eq!(layer.visible,visible);assert_eq!(layer.transform.rotate,0.2);
            assert_eq!(fixture.runtime.host.engine.layers.motion_status(CLIENT,payload)?.drivers[0].stamp,stamp);
        }
        assert_eq!(fixture.runtime.host.engine.layers.get(CLIENT,base.layer).unwrap().visible,initially_visible,"depth command must not invent a baseimg show/hide event");
        Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn actual_alias_kcs_class_output_root_completion_and_native_or_engine_skip_are_one_frame_receipt()->Result<()> {
        fn binary(code:&mut Vec<u8>,op:u8,left:(u8,u32),right:(u8,u32)){
            code.extend([op,left.0]);code.extend(left.1.to_le_bytes());code.push(right.0);code.extend(right.1.to_le_bytes());
        }
        fn branch(code:&mut Vec<u8>)->usize{code.extend([0x28,3]);let at=code.len();code.extend(0u32.to_le_bytes());at}
        const EX:u32=0x80000000;
        //Real original KCS data: position reads raw float parameter; alpha is
        //64 while active, and255 only when original FRAME>=2 or SKIP is set.
        let mut code=Vec::new();
        binary(&mut code,0x15,(3,EX+24),(0x13,EX+156));code.push(0x2d);
        binary(&mut code,0x15,(3,EX+112),(3,0x40ffffff));code.push(0x2d);
        binary(&mut code,0x11,(0x13,EX+144),(3,2));let natural=branch(&mut code);
        binary(&mut code,0x14,(0x13,EX+152),(3,0));let skipped=branch(&mut code);
        code.push(0x3b);let finish=u32::try_from(code.len())?;
        binary(&mut code,0x15,(3,EX+112),(3,0xffffffff));code.push(0x2d);
        binary(&mut code,0x2b,(3,38),(3,0));code.push(0x3b);
        for at in [natural,skipped]{code[at..at+4].copy_from_slice(&finish.to_le_bytes());}
        for mode in ["natural","native_skip","engine_skip","object_stop"] {
            let force=if mode=="native_skip"{"keyskip force 1\n"}else{""};
            let wait=if mode=="object_stop"{"wait 1\nsprite stop (++$9)\nwait"}else{"wait"};
            let source=format!("#SUBCOMMAND\noneshot pulse pulse p[8] : [0]\n#DEFINE\nCIMAGE sprite\nPLANE view\n#OBJECT SIZE COLOR PRI DISP\nsprite 2,2 0xff03070b 3 1\nview 4,4 0xffffffff 1 1\n#OBJECT PL\nsprite view\n#START\n{force}sprite pulse (++$7)\n{wait}\n$6=77\nwait 1");
            let mut fixture=Fixture::new(source.as_bytes())?;fixture.motion("pulse",&code)?;
            fixture.children.motions.set_explicit_seeds(Some([17,19]))?;
            let list=fixture.tasks.fes(fixture.owner)?.image_list.unwrap();fixture.tasks.set_image_upload_position(list,[0,0])?;
            let mut sample=observation();sample.time_ms=1000;
            let start=Instant::now();
            loop{if matches!(fixture.run(sample,&mut 4)?,Tick::Waited){break;}
                ensure!(start.elapsed()<Duration::from_secs(5),"original alias worker/KCS budget continuation timeout");std::thread::yield_now();}
            assert_eq!(fixture.interpreter.flag(7),1,"alias expression must not replay during seed/file/image/budget polls");
            let child=fixture.tasks.fes(fixture.owner)?.child_tasks[0].unwrap();
            let root=fixture.tasks.fes(child)?.object_root.unwrap();
            let status=fixture.runtime.host.engine.layers.motion_status(CLIENT,root)?;
            assert!(!status.complete);assert_eq!(status.drivers.len(),1);assert_eq!(status.drivers[0].stamp.revision,1);
            let first_stamp=status.drivers[0].stamp;
            let canvas=fixture.children.frames.ready[&child].list.scope().1;
            let surface=fixture.runtime.host.engine.canvases.surface(CLIENT,canvas).unwrap();
            let crysirror_scene::canvas::CanvasNode::Draw{layer}=&surface.nodes[0]else{unreachable!()};
            assert_eq!(layer.rect[0],1.);assert_eq!(layer.opacity,64,"actual KCS output must reach shared Canvas");
            let first_lease=fixture.children.frames.ready[&child].resources.values().next().unwrap().path.clone();
            if mode=="engine_skip"{
                fixture.runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"request_external_motion_skip","id":root,"driver":status.drivers[0].driver,"generation":first_stamp.generation}]}))?)?;
            }
            let completion=if mode=="natural"{2}else{1};
            for frame in 1..=completion {
                sample.revision+=1;sample.time_ms=1000+17*frame;
                assert!(matches!(fixture.run_owner(child,sample,&mut 100)?,Tick::Waited));
                let status=fixture.runtime.host.engine.layers.motion_status(CLIENT,root)?;
                assert_eq!(status.drivers[0].stamp.generation,first_stamp.generation);
                assert_eq!(status.drivers[0].stamp.revision,frame+1);
                assert_eq!(status.complete,frame==completion);
                assert_eq!(fixture.interpreter.flag(6),0,"native all-object wait must not execute following script in the completion call");
                assert_eq!(fixture.children.frames.ready[&child].resources.values().next().unwrap().path,first_lease);
            }
            let surface=fixture.runtime.host.engine.canvases.surface(CLIENT,canvas).unwrap();
            let crysirror_scene::canvas::CanvasNode::Draw{layer}=&surface.nodes[0]else{unreachable!()};
            assert_eq!(layer.opacity,255,"skip must execute original KCS endpoint, not just mark the producer complete");
            assert_eq!(fixture.tasks.fes(child)?.player.as_ref().unwrap().timing.wait,Some(crate::fes_timing::Wait::Objects));
            sample.revision+=1;sample.time_ms+=17;
            assert!(matches!(fixture.run_owner(child,sample,&mut 100)?,Tick::Waited));assert_eq!(fixture.interpreter.flag(6),0);
            sample.revision+=1;sample.time_ms+=17;
            assert!(matches!(fixture.run_owner(child,sample,&mut 100)?,Tick::Waited));assert_eq!(fixture.interpreter.flag(6),77);
            assert_eq!(fixture.interpreter.flag(7),1);
            assert_eq!(fixture.interpreter.flag(9),0,"native stop does not consume extra parameters");
        }Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn actual_fes_rand_and_table_wait_for_owned_seed_and_share_exact_native_stream()->Result<()> {
        let mut fixture=Fixture::with_parent(b"#START\nrand 8 0 6\nrandtbl 10 15 100 (++$7)\nwait 1",b"#START\nwait 1")?;
        fixture.children.motions.set_explicit_seeds(Some([17,19]))?;
        let start=Instant::now();loop{if matches!(fixture.run(observation(),&mut 100)?,Tick::Waited){break;}
            ensure!(start.elapsed()<Duration::from_secs(5),"actual FES owned seed continuation timeout");std::thread::yield_now();}
        assert_eq!(fixture.interpreter.flag(7),1,"ignored native fourth argument still evaluates exactly once");
        let mut expected=crysirror_resource::legacy_stream::Mt19937Split::new(19);
        assert_eq!(fixture.interpreter.flag(8),expected.next_u32()%7);
        for _ in 0..2002{expected.next_u32();}
        assert_eq!(fixture.children.motions.random()?.integer_word(CLIENT)?,expected.next_u32());
        let mut actual=(10..=15).map(|index|fixture.interpreter.flag(index)).collect::<Vec<_>>();actual.sort_unstable();
        assert_eq!(actual,(100..=105).collect::<Vec<_>>());Ok(())
    }
    #[test]
    fn pending_native_child_observation_is_immutable_before_any_input_or_vm_changes() -> Result<()>
    {
        let mut fixture = Fixture::new(b"#START\nwait 1")?;
        assert!(matches!(
            fixture.run(observation(), &mut 100)?,
            Tick::Dependency
        ));
        let before = fixture.interpreter.diagnostics();
        let prior_input = fixture.input.revision;
        let mut changed = observation();
        changed.revision = 2;
        changed.time_ms = 17;
        changed.pointer = Some([4, 4]);
        assert!(fixture.run(changed, &mut 100).is_err());
        assert_eq!(fixture.interpreter.diagnostics(), before);
        assert_eq!(fixture.input.revision, prior_input);
        assert_eq!(fixture.tasks.fes(fixture.owner)?.child_tasks, [None, None]);
        Ok(())
    }
    #[test]
    fn endfes_marks_then_retires_actual_input_layer_bank_and_producer_capabilities() -> Result<()> {
        let mut fixture = Fixture::with_parent(
            b"#START\nexecfes child\nwait 2\nendfes\n$5=1\nwait 1",
            b"#START\nexit 7\nwait 1",
        )?;
        let start = Instant::now();
        loop {
            if matches!(fixture.run(observation(), &mut 100)?, Tick::Waited) {
                break;
            }
            ensure!(
                start.elapsed() < Duration::from_secs(5),
                "authored child first phase timeout"
            );
            std::thread::yield_now();
        }
        let child = fixture.tasks.fes(fixture.owner)?.child_tasks[0].unwrap();
        let root = fixture.tasks.fes(child)?.object_root.unwrap();
        let button = fixture.tasks.fes(child)?.low_button.unwrap();
        let images = fixture.tasks.fes(child)?.image_list.unwrap();
        fixture.tasks.set_image_mode(images, 0)?;
        fixture.runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"begin_external_motion","id":root,"driver":99}]}))?)?;
        let stamp = fixture
            .runtime
            .host
            .engine
            .layers
            .motion_status(CLIENT, root)?
            .drivers[0]
            .stamp;
        let later = InputObservation {
            revision: 2,
            time_ms: 34,
            ..observation()
        };
        fixture.run_owner(child, later, &mut 100)?;
        assert!(matches!(fixture.run(later, &mut 100)?, Tick::Waited));
        assert!(fixture.tasks.fes(child)?.deletion_pending);
        assert_eq!(fixture.tasks.fes(fixture.owner)?.child_tasks, [None, None]);
        assert_eq!(fixture.interpreter.evaluate(b"_EXITCODE_")?, u32::MAX);
        assert_eq!(fixture.tasks.task_result(child), 7);
        assert_eq!(fixture.interpreter.flag(5), 0);
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(CLIENT, root)
            .is_some());
        assert!(fixture
            .runtime
            .host
            .engine
            .input_routes
            .receiver(CLIENT, button)
            .is_some());
        //This is the NEXT native base child phase, not the endfes ACK itself.
        fixture.tasks.retire_fes_subtree(
            child,
            &mut fixture.runtime.host,
            CLIENT,
            Some(&mut fixture.interpreter),
            &[],
        )?;
        assert!(!fixture.tasks.exists(child));
        assert_eq!(fixture.tasks.task_result(child), u32::MAX);
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(CLIENT, root)
            .is_none());
        assert!(fixture
            .runtime
            .host
            .engine
            .input_routes
            .receiver(CLIENT, button)
            .is_none());
        assert_eq!(fixture.interpreter.diagnostics()["local_banks"], 1);
        assert_eq!(fixture.tasks.image_list(images)?.field_120, 1);
        let replacement = fixture
            .tasks
            .create_fes(fixture.owner, b"replacement".to_vec())?;
        assert_eq!(replacement, child);
        let program = Arc::new(crate::fes_program::Program::parse(
            b"#START\nwait 1",
            &|| Ok(()),
        )?);
        fixture.tasks.load_fes_player(
            replacement,
            "fes.int/replacement.fes",
            program,
            &mut fixture.interpreter,&Default::default(),
        )?;
        fixture
            .tasks
            .publish_fes_object_root(replacement, &mut fixture.runtime.host, CLIENT)?;
        fixture.runtime.host.submit_engine(CLIENT,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"begin_external_motion","id":root,"driver":99}]}))?)?;
        assert_ne!(
            fixture
                .runtime
                .host
                .engine
                .layers
                .motion_status(CLIENT, root)?
                .drivers[0]
                .stamp
                .generation,
            stamp.generation
        );
        let stale = serde_json::json!({"layers":[{"op":"commit_external_motion","id":root,"driver":99,"stamp":{"generation":stamp.generation.to_string(),"revision":"1"},"complete":true}]});
        assert!(fixture
            .runtime
            .host
            .submit_engine(CLIENT, 1, &serde_json::to_vec(&stale)?)
            .is_err());
        Ok(())
    }
    #[test]
    fn protected_jobs_and_failed_engine_retirement_preserve_native_resources_until_receipt(
    ) -> Result<()> {
        let mut fixture = Fixture::new(b"#START\nwait 1")?;
        let start = Instant::now();
        loop {
            if matches!(fixture.run(observation(), &mut 100)?, Tick::Waited) {
                break;
            }
            ensure!(
                start.elapsed() < Duration::from_secs(5),
                "authored child first phase timeout"
            );
            std::thread::yield_now();
        }
        let child = fixture.tasks.fes(fixture.owner)?.child_tasks[0].unwrap();
        let root = fixture.tasks.fes(child)?.object_root.unwrap();
        let button = fixture.tasks.fes(child)?.low_button.unwrap();
        fixture
            .tasks
            .end_fes_child(fixture.owner, 0, &mut fixture.interpreter)?;
        let prior = fixture.tasks.diagnostics();
        let vars = fixture.interpreter.diagnostics();
        assert!(fixture
            .tasks
            .retire_fes_subtree(
                child,
                &mut fixture.runtime.host,
                CLIENT,
                Some(&mut fixture.interpreter),
                &[child]
            )
            .is_err());
        assert_eq!(fixture.tasks.diagnostics(), prior);
        assert_eq!(fixture.interpreter.diagnostics(), vars);
        let receiver = fixture
            .runtime
            .host
            .engine
            .input_routes
            .receiver(CLIENT, button)
            .unwrap()
            .clone();
        fixture.runtime.host.submit_engine(
            CLIENT,
            1,
            &serde_json::to_vec(
                &serde_json::json!({"input_routes":[{"op":"remove","id":button}]}),
            )?,
        )?;
        assert!(fixture
            .tasks
            .retire_fes_subtree(
                child,
                &mut fixture.runtime.host,
                CLIENT,
                Some(&mut fixture.interpreter),
                &[]
            )
            .is_err());
        assert_eq!(fixture.tasks.diagnostics(), prior);
        assert_eq!(fixture.interpreter.diagnostics(), vars);
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(CLIENT, root)
            .is_some());
        fixture.runtime.host.submit_engine(
            CLIENT,
            1,
            &serde_json::to_vec(
                &serde_json::json!({"input_routes":[{"op":"put","receiver":receiver}]}),
            )?,
        )?;
        fixture.tasks.retire_fes_subtree(
            child,
            &mut fixture.runtime.host,
            CLIENT,
            Some(&mut fixture.interpreter),
            &[],
        )?;
        assert!(!fixture.tasks.exists(child));
        assert!(fixture
            .runtime
            .host
            .engine
            .layers
            .get(CLIENT, root)
            .is_none());
        Ok(())
    }
    #[test]
    fn forced_base_phase_resumes_postorder_without_replaying_a_completed_child() -> Result<()> {
        let mut fixture = Fixture::new(b"#START\n$7+=1\nwait\n$7+=10\nwait 1")?;
        let start = Instant::now();
        loop {
            if matches!(fixture.run(observation(), &mut 100)?, Tick::Waited) {
                break;
            }
            ensure!(
                start.elapsed() < Duration::from_secs(5),
                "authored forced child setup timeout"
            );
            std::thread::yield_now();
        }
        let child = fixture.tasks.fes(fixture.owner)?.child_tasks[0].unwrap();
        assert_eq!(fixture.interpreter.flag(7), 1);
        let before = fixture.tasks.fes(child)?.player.as_ref().unwrap().executed;
        let later = InputObservation {
            revision: 2,
            time_ms: 34,
            ..observation()
        };
        assert!(matches!(fixture.force(later, &mut 1)?, Tick::Budget));
        assert_eq!(fixture.children.forced.get(&fixture.owner).unwrap().next, 1);
        assert_eq!(
            fixture
                .tasks
                .fes(child)?
                .player
                .as_ref()
                .unwrap()
                .timing
                .wait,
            None
        );
        let vars = fixture.interpreter.diagnostics();
        let changed = InputObservation {
            revision: 3,
            time_ms: 50,
            ..later
        };
        assert!(fixture.force(changed, &mut 100).is_err());
        assert_eq!(fixture.interpreter.diagnostics(), vars);
        assert!(matches!(fixture.force(later, &mut 100)?, Tick::Waited));
        assert_eq!(fixture.interpreter.flag(7), 1);
        assert_eq!(
            fixture.tasks.fes(child)?.player.as_ref().unwrap().executed,
            before
        );
        assert!(fixture.children.forced.is_empty());
        Ok(())
    }
    #[test]
    fn forced_first_tick_initializes_real_input_before_source_without_an_ordinary_phase(
    ) -> Result<()> {
        let mut fixture = Fixture::new(b"#START\nwait 1")?;
        //A newly loaded native task has not passed the ordinary base phase.
        let owner = fixture.tasks.create_fes(3, b"direct forced".to_vec())?;
        let program = Arc::new(crate::fes_program::Program::parse(
            b"#START\n$9+=1\nwait 1",
            &|| Ok(()),
        )?);
        fixture.tasks.load_fes_player(
            owner,
            "fes.int/direct.fes",
            program,
            &mut fixture.interpreter,&Default::default(),
        )?;
        fixture
            .tasks
            .publish_fes_object_root(owner, &mut fixture.runtime.host, CLIENT)?;
        fixture.owner = owner;
        assert!(fixture.tasks.fes(owner)?.first_tick);
        assert!(!fixture.tasks.fes(owner)?.running);
        assert!(matches!(
            fixture.force(observation(), &mut 100)?,
            Tick::Waited
        ));
        assert_eq!(fixture.interpreter.flag(9), 1);
        assert!(!fixture.tasks.fes(owner)?.first_tick);
        let button = fixture.tasks.fes(owner)?.low_button.unwrap();
        assert!(fixture
            .runtime
            .host
            .engine
            .input_routes
            .receiver(CLIENT, button)
            .is_some());
        assert!(fixture.children.routes_published());
        Ok(())
    }
    #[cfg(feature="runtime")]
    #[test]
    fn actual_sound_load_play_pause_loop_and_eof_use_shared_resource_and_pcm()->Result<()> {
        for grouped in [false,true] {
        let fields=if grouped{"CHANNEL STYPE GROUP\nvoice 0 2 voice"}else{"CHANNEL STYPE\nvoice 0 2"};
        let source=format!("#DEFINE\nSOUND voice\n#OBJECT {fields}\n#START\nvoice load $str9 ++$5\nwait 1\nvoice play 1 1 ++$6\nwait 1\nvoice pause\nwait 1\nvoice play 0\nwait 1\nvoice loop 1\nwait 1\nwait 1\nvoice load\nwait 1\nvoice load $str9\nvoice play 2\nwait 1\nvoice play 0\nwait 1\nvoice stop\nwait 1\nvoice load\nwait 1");
        let mut fixture=Fixture::new(source.as_bytes())?;
        let startup=fixture._root.path().join("config/startup.xml");
        let xml=std::fs::read_to_string(&startup)?.replace("</INT>","<int20><name>config</name><int>config.int</int><ext>xml</ext><dir>config</dir></int20><int21><name>voice</name><int>voice.int</int><ext>wav</ext><dir>voice</dir></int21></INT><SOUNDCONFIG>sound.xml</SOUNDCONFIG>");
        std::fs::write(startup,xml)?;
        let xml=br"<document><SOUNDGROUP><voice><id>2</id><label>voice</label><type>voice</type><min>0</min><max>1</max></voice></SOUNDGROUP><MASTER><volume>100</volume><play>1</play><mute>0</mute></MASTER><PART><voice><id>2</id><head>v_</head><int>voice</int><volume>100</volume><play>1</play><sysplay>1</sysplay><voicevol>100</voicevol><mute>0</mute></voice></PART></document>";
        std::fs::write(fixture._root.path().join("config/sound.xml"),xml)?;
        fixture.sound=Some(crate::sound_definition::SoundDefinition{source:Some("config/sound.xml".into()),document:None,policy:Some(crate::native_sound_policy::SoundPolicy::parse(&crysirror_resource::xml::XmlDocument::parse(xml)?)?)});
        fixture.strings.assign(9,b"V_01".to_vec())?;
        std::fs::create_dir(fixture._root.path().join("voice"))?;
        let mut wave=b"RIFF".to_vec();wave.extend((36u32+512*4).to_le_bytes());wave.extend(b"WAVEfmt ");wave.extend(16u32.to_le_bytes());wave.extend(1u16.to_le_bytes());wave.extend(2u16.to_le_bytes());wave.extend(48000u32.to_le_bytes());wave.extend(192000u32.to_le_bytes());wave.extend(4u16.to_le_bytes());wave.extend(16u16.to_le_bytes());wave.extend(b"data");wave.extend((512u32*4).to_le_bytes());for _ in 0..1024{wave.extend(16384i16.to_le_bytes());}
        std::fs::write(fixture._root.path().join("voice/V_01.wav"),wave)?;
        let list=fixture.tasks.fes(fixture.owner)?.image_list.unwrap();fixture.tasks.set_image_upload_position(list,[0,0])?;
        let pump=|fixture:&mut Fixture,owner,input|->Result<()>{let start=Instant::now();loop{if matches!(fixture.run_owner(owner,input,&mut 4096)?,Tick::Waited){return Ok(());}ensure!(start.elapsed()<Duration::from_secs(5),"actual SOUND frame preparation timeout");std::thread::yield_now();}};
        let mut input=observation();input.time_ms=1000;let parent=fixture.owner;pump(&mut fixture,parent,input)?;
        let child=fixture.tasks.fes(fixture.owner)?.child_tasks[0].context("actual SOUND child missing")?;
        assert_eq!(fixture.interpreter.flag(5),1,"native load parameter consumed once across workers");
        assert!(fixture.runtime.host.audio_info(CLIENT,0)?.is_none(),"load is preparation only, never fake playback");
        let object=fixture.tasks.fes(child)?.object_bank.as_ref().unwrap().objects.values().find(|object|matches!(object.state,crate::native_class_state::State::Sound(_))).unwrap().id;
        let state=|fixture:&Fixture|->crate::native_class_state::SoundState{let crate::native_class_state::State::Sound(sound)=&fixture.tasks.fes(child).unwrap().object_bank.as_ref().unwrap().objects[&object].state else{panic!()};sound.clone()};
        assert!(!state(&fixture).load_dirty);assert_eq!(state(&fixture).channel,0);
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        assert_eq!(fixture.interpreter.flag(6),1);assert!(fixture.runtime.host.audio_playing(CLIENT,0)?);
        fixture.runtime.host.channel_audio.render(1)?;
        let pcm=fixture.runtime.host.channel_audio.take_output();assert_eq!(pcm.len(),48);assert!(pcm.iter().all(|p|(p[0]-0.5).abs()<1e-6&&(p[1]-0.5).abs()<1e-6));
        let before=fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().position_frames;
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        if grouped {
            assert!(!fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().paused,"native grouped pause is a real delayed actor");
            for _ in 0..11 {
                input.revision+=1;input.time_ms+=17;
                fixture.children.frames.observe_audio(&mut fixture.runtime.host,CLIENT,input)?;
            }
        }
        assert!(fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().paused);
        fixture.runtime.host.channel_audio.render(1)?;fixture.runtime.host.channel_audio.take_output();
        assert_eq!(fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().position_frames,before);
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        assert!(fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().looping);
        fixture.runtime.host.channel_audio.render(20)?;fixture.runtime.host.channel_audio.take_output();assert!(fixture.runtime.host.audio_playing(CLIENT,0)?);
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        assert!(!fixture.runtime.host.audio_info(CLIENT,0)?.unwrap().looping);
        fixture.runtime.host.channel_audio.render(20)?;fixture.runtime.host.channel_audio.take_output();assert!(!fixture.runtime.host.audio_playing(CLIENT,0)?);
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;assert!(!state(&fixture).playing,"native6285C0 uses actual source EOF");
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;assert!(state(&fixture).filename.is_empty());
        assert!(fixture.runtime.host.audio_info(CLIENT,0)?.is_none());
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        let channel=state(&fixture).channel as u32;
        assert_eq!(fixture.runtime.host.audio_info(CLIENT,channel)?.unwrap().loop_passes,Some(2),"native total-count2 includes the first pass");
        let start=Instant::now();let mut audible=0;
        while fixture.runtime.host.audio_playing(CLIENT,channel)?{
            fixture.runtime.host.channel_audio.render(1)?;
            audible+=fixture.runtime.host.channel_audio.take_output().iter().filter(|p|p[0]>0.49).count();
            ensure!(start.elapsed()<Duration::from_secs(5),"finite native pass counter did not reach actual EOF");
            std::thread::yield_now();
        }
        assert_eq!(audible,1024,"two passes of the real512-frame source, not boolean looping");
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        assert!(fixture.runtime.host.audio_playing(CLIENT,channel)?);
        let resource=fixture.children.frames.audio_resource(channel).context("native sound resource lost before stop")?;
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        assert!(!state(&fixture).playing);
        assert_eq!(fixture.runtime.host.audio_playing(CLIENT,channel)?,grouped,"only native grouped transport has the ten-update fade");
        for _ in 0..11{input.revision+=1;input.time_ms+=17;fixture.children.frames.observe_audio(&mut fixture.runtime.host,CLIENT,input)?;}
        assert!(!fixture.runtime.host.audio_playing(CLIENT,channel)?);
        assert!(fixture.runtime.host.audio_resource_info(CLIENT,resource).is_ok(),"native stop keeps encoded resource authority");
        input.revision+=1;input.time_ms+=17;pump(&mut fixture,child,input)?;
        for _ in 0..11{input.revision+=1;input.time_ms+=17;fixture.children.frames.observe_audio(&mut fixture.runtime.host,CLIENT,input)?;}
        assert!(fixture.runtime.host.audio_resource_info(CLIENT,resource).is_err(),"release authority retires only after the original terminal actor");
        assert!(fixture.children.frames.audio_diagnostics()["slots"].as_array().unwrap().is_empty());
        }
        Ok(())
    }

}
