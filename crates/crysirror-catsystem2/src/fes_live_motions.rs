// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!Original alias-to-KCS resource policy. Source bytes are data read through
//!shared storage workers; the first-class Controller executes the same KCS VM.
use crate::{
    fes_object_bank::Bank,
    fes_program::{Alias, PendingOperation},
    kcs::Program,
    native_interpreter::Interpreter,
    native_values::StringVariables,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{ClientStorage, HostServices},
    state::ClientId,
};
use crysirror_scene::layer::{ExternalMotionStamp, LayerCommand, MotionStatus};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::BTreeMap, sync::Arc};
const DRIVER: u32 = 0x4341544d;
struct Tree {
    clock: u32,
    ids: Vec<u32>,
    next: usize,
    current: bool,
}
struct Stop {pc:u32,operation:PendingOperation,vm:Interpreter,targets:Vec<u32>,tree:Tree}
struct Driver {
    root: u32,
    stamp: ExternalMotionStamp,
    complete: bool,
}
struct Seed {
    owner: u32,
    pc: u32,
    operation: PendingOperation,
    query: Box<dyn crysirror_core::service::SessionSeedQuery>,
}
struct Pending {
    operation: PendingOperation,
    pc: u32,
    words: Vec<(u32, Vec<u32>)>,
    states: BTreeMap<u32,crate::native_class_state::State>,
    builtin_blend: bool,
    vm: Interpreter,
    name: Vec<u8>,
    worker: Option<Task<Arc<Program>>>,
    ready: Option<Arc<Program>>,
    failure: Option<String>,
}
#[derive(Default)]
pub(crate) struct Motions {
    pending: BTreeMap<u32, Pending>,
    cache: BTreeMap<Vec<u8>, Arc<Program>>,
    trees: BTreeMap<u32, Tree>,
    stops: BTreeMap<u32,Stop>,
    drivers: BTreeMap<u32, Driver>,
    explicit_seeds: Option<[u32; 2]>,
    random: Option<crate::motion::RandomStreams>,
    seed: Option<Seed>,
}
pub(crate) fn load(
    storage: &dyn ClientStorage,
    name: &[u8],
    check: &dyn Fn() -> Result<()>,
) -> Result<Arc<Program>> {
    check()?;
    let name = std::str::from_utf8(name)?;
    let catalog = crate::native_resource_paths::Catalog::load(storage)?;
    for path in catalog.candidates(storage, "mot", name, check)? {
        check()?;
        if let Some(path) = crate::native_resource_paths::resolve(storage, &path)? {
            let bytes = crysirror_resource::read_all(
                storage.open_asset(&path)?.as_ref(),
                crate::kcs::MAX_SCRIPT_BYTES as u64,
            )?;
            check()?;
            let program = Program::parse(&bytes)?;
            check()?;
            return Ok(Arc::new(program));
        }
    }
    anyhow::bail!("CatSystem2 original motion resource {name} unavailable")
}
impl Motions {
    pub fn pending_owners(&self) -> Vec<u32> {
        self.pending
            .keys()
            .chain(self.trees.keys())
            .copied()
            .chain(self.seed.as_ref().map(|seed| seed.owner))
            .chain(self.stops.keys().copied())
            .collect()
    }
    pub fn set_explicit_seeds(&mut self, seeds: Option<[u32; 2]>) -> Result<()> {
        ensure!(
            self.random.is_none() && self.seed.is_none(),
            "CatSystem2 session seed source is already admitted"
        );
        self.explicit_seeds = seeds;
        Ok(())
    }
    pub fn random(&mut self) -> Result<&mut crate::motion::RandomStreams> {
        self.random
            .as_mut()
            .context("CatSystem2 session random stream has no seed receipt")
    }
    pub fn ensure_random(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        pc: u32,
        operation: &PendingOperation,
    ) -> Result<bool> {
        if let Some(random) = &self.random {
            ensure!(
                random.belongs_to(client),
                "CatSystem2 foreign random stream"
            );
            return Ok(true);
        }
        if self.seed.is_none() {
            let source = self.explicit_seeds.map_or(
                crysirror_core::service::SessionSeedSource::Os { words: 2 },
                |words| crysirror_core::service::SessionSeedSource::Fixed {
                    words: words.to_vec(),
                },
            );
            let query = match host.begin_session_seed(client, source) {
                Ok(query) => query,
                Err(crysirror_core::Error::Limit("session seed workers")) => return Ok(false),
                Err(error) => return Err(anyhow::anyhow!("{error}")),
            };
            ensure!(
                query.client() == client,
                "CatSystem2 seed query belongs to another client"
            );
            self.seed = Some(Seed {
                owner,
                pc,
                operation: operation.clone(),
                query,
            });
        }
        let seed = self.seed.as_mut().unwrap();
        ensure!(
            seed.owner == owner && seed.pc == pc && seed.operation == *operation,
            "CatSystem2 pending session seed original owner/PC/operation changed"
        );
        match seed.query.poll()? {
            crysirror_core::service::SessionSeedState::Pending => Ok(false),
            crysirror_core::service::SessionSeedState::Cancelled => {
                anyhow::bail!("CatSystem2 owned session seed preparation was cancelled")
            }
            crysirror_core::service::SessionSeedState::Failed(error) => {
                anyhow::bail!("CatSystem2 session seed preparation failed: {error}")
            }
            crysirror_core::service::SessionSeedState::Ready(material) => {
                ensure!(
                    material.client == client && material.words.len() == 2,
                    "CatSystem2 invalid/foreign session seed completion"
                );
                self.random = Some(crate::motion::RandomStreams::new(
                    client,
                    material.words[0],
                    material.words[1],
                ));
                self.seed = None;
                Ok(true)
            }
        }
    }
    pub fn reap(&mut self, tasks: &crate::task_policy::NativeTaskPolicy) {
        self.drivers.retain(|owner, _| tasks.fes(*owner).is_ok());
    }
    pub fn apply(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        pool: &TaskPool,
        owner: u32,
        root: u32,
        pc: u32,
        operation: &PendingOperation,
        alias: &Alias,
        bank: &mut Bank,
        vm: &mut Interpreter,
        strings: &StringVariables,
    ) -> Result<bool> {
        self.apply_common(host,client,pool,owner,root,pc,operation,alias,bank,vm,strings,false)
    }
    pub fn blend(&mut self,host:&mut dyn HostServices,client:ClientId,pool:&TaskPool,owner:u32,root:u32,pc:u32,operation:&PendingOperation,bank:&mut Bank,vm:&mut Interpreter,strings:&StringVariables)->Result<bool>{
        let alias=Alias{mode:b"oneshot".to_vec(),name:b"blend".to_vec(),resource:b"pColor".to_vec(),defaults:Vec::new(),arguments:Vec::new()};
        self.apply_common(host,client,pool,owner,root,pc,operation,&alias,bank,vm,strings,true)
    }
    fn apply_common(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        pool: &TaskPool,
        owner: u32,
        root: u32,
        pc: u32,
        operation: &PendingOperation,
        alias: &Alias,
        bank: &mut Bank,
        vm: &mut Interpreter,
        strings: &StringVariables,
        builtin_blend: bool,
    ) -> Result<bool> {
        ensure!(
            bank.owner == owner,
            "CatSystem2 alias metadata owner changed"
        );
        if !self.ensure_random(host, client, owner, pc, operation)? {
            return Ok(false);
        }
        if !self.pending.contains_key(&owner) {
            ensure!(
                self.pending.len() < 64,
                "CatSystem2 pending motion owners budget"
            );
            ensure!(
                alias.resource.is_ascii()
                    && !alias.resource.is_empty()
                    && alias.resource.len() < 32,
                "CatSystem2 alias motion filename profile"
            );
            let key = alias.resource.to_ascii_lowercase();
            let cached = self.cache.get(&key).cloned();
            let permit = if cached.is_none() {
                match pool.reserve() {
                    Ok(p) => Some(p),
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            let mut staged = vm.clone();
            let ids = bank.targets(&operation.name, &mut staged, strings)?;
            let mut words = Vec::with_capacity(ids.len());
            let mut states=BTreeMap::new();
            let blend=if builtin_blend{Some(operation.arguments.get(1).map_or(Ok(255),|s|staged.evaluate(s))?)}else{None};
            for id in ids {
                if let Some(value)=blend{
                    let mut state=bank.objects[&id].state.clone();
                    // 76B880: alpha per class, then pColor with [alpha, r, g, b].
                    use crate::native_class_state::State as S;
                    let color=match &mut state{
                        S::Sound(sound)=>{sound.pose.volume=value;0}
                        S::Image(image)|S::CImage(image)=>{image.pose.alpha=value&255;0}
                        S::Plane(plane)|S::CPlane(plane)=>{plane.pose.color=(value<<24)|(plane.pose.color&0x00ff_ffff);plane.pose.color}
                        S::Base=>continue,
                    };
                    states.insert(id,state);
                    words.push((id,vec![(value as i32 as f32).to_bits(),(((color>>16)&255) as f32).to_bits(),(((color>>8)&255) as f32).to_bits(),((color&255) as f32).to_bits()]));
                    continue;
                }
                let values = crate::fes_alias_values::Values::from_state(&bank.objects[&id].state);
                let parameters = crate::fes_alias_values::project(
                    alias,
                    &operation.arguments[1..],
                    values,
                    &mut staged,
                )?;
                words.push((id, parameters.into_iter().map(f32::to_bits).collect()));
            }
            let worker = if let Some(permit) = permit {
                let storage = host.storage(client)?;
                let name = alias.resource.clone();
                Some(
                    permit
                        .spawn(move |cancel| load(storage.as_ref(), &name, &|| cancel.check()))?,
                )
            } else {
                None
            };
            self.pending.insert(
                owner,
                Pending {
                    operation: operation.clone(),
                    pc,
                    words,
                    states,
                    builtin_blend,
                    vm: staged,
                    name: alias.resource.clone(),
                    worker,
                    ready: cached,
                    failure: None,
                },
            );
        }
        let pending = self.pending.get_mut(&owner).unwrap();
        ensure!(
            pending.operation == *operation && pending.pc == pc,
            "CatSystem2 pending alias original row/native PC changed"
        );
        if let Some(error) = &pending.failure {
            anyhow::bail!("{error}");
        }
        if pending.ready.is_none() {
            match pending
                .worker
                .as_mut()
                .context("CatSystem2 alias worker lost")?
                .try_take()
            {
                Ok(Some(program)) => {
                    pending.ready = Some(program);
                    pending.worker = None;
                }
                Ok(None) => return Ok(false),
                Err(error) => {
                    let text = format!("{error:#}");
                    pending.failure = Some(text.clone());
                    anyhow::bail!("{text}");
                }
            }
        }
        let program = pending.ready.as_ref().unwrap().clone();
        if pending.builtin_blend {
            //76B880 passes source-pointer RGB bytes for SOUND. They are not a
            //portable semantic value. The reviewed original pColor only clamps
            //and independently writes each byte, then Native38;74C590 consumes
            //alpha/pan only. Normalize those unobservable RGB inputs only under
            //this exact bounded code profile, never for arbitrary replacement
            //scripts. CRC is a layout guard, not a security/integrity API.
            let mut crc=flate2::Crc::new();crc.update(&program.code);
            ensure!(program.code.len()==767&&crc.sum()==0x975a7795&&program.globals_bytes==0&&program.stack_bytes==1024&&program.strings.is_empty(),
                "CatSystem2 builtin SOUND blend requires reviewed independent-byte pColor KCS profile");
        }
        //Stage every original controller before changing the native object bank.
        //Parameter side effects remain uncommitted while worker/validation fails.
        let mut updated = Vec::new();
        for (id, parameters) in &pending.words {
            let mut object = bank
                .objects
                .get(id)
                .context("CatSystem2 alias object disappeared")?
                .clone();
            if let Some(state)=pending.states.get(id){object.state=state.clone();}
            if !bank.active_subtree(*id)? {
                if let Some(seed) = object.state.motion_seed() {
                    object.motions.set_base(seed)?;
                }
            }
            //75AAC0 zeroes record32 for both accepted mode spellings; the
            //reviewed path passes that original group0, start0 and delay0.
            object.motions.insert(
                pending.name.iter().map(|b| u16::from(*b)).collect(),
                program.clone(),
                parameters,
                None,
                0,
                0,
                0,
            )?;
            updated.push((*id, object));
        }
        //The owner root already exists before aliases; expose a genuine
        //engine completion producer BEFORE acknowledging controller insertion.
        let root_commands = if let Some(driver) = self.drivers.get(&owner) {
            ensure!(driver.root == root, "CatSystem2 motion driver root changed");
            if driver.complete {
                vec![
                    LayerCommand::RemoveExternalMotion {
                        id: root,
                        driver: DRIVER,
                        generation: driver.stamp.generation,
                    },
                    LayerCommand::BeginExternalMotion {
                        id: root,
                        driver: DRIVER,
                    },
                ]
            } else {
                Vec::new()
            }
        } else {
            vec![LayerCommand::BeginExternalMotion {
                id: root,
                driver: DRIVER,
            }]
        };
        if !root_commands.is_empty() {
            host.submit_engine(
                client,
                1,
                &serde_json::to_vec(&crysirror_scene::ServiceBatch {
                    layers: root_commands,
                    ..Default::default()
                })?,
            )?;
            let status: MotionStatus = serde_json::from_slice(&host.query_engine(
                client,
                crysirror_core::service::EngineQuery::LayerMotion(root),
            )?)?;
            let driver = status
                .drivers
                .into_iter()
                .find(|driver| driver.driver == DRIVER)
                .context("CatSystem2 motion driver publication has no actual engine receipt")?;
            self.drivers.insert(
                owner,
                Driver {
                    root,
                    stamp: driver.stamp,
                    complete: driver.complete,
                },
            );
        }
        for (id, object) in updated {
            bank.objects.insert(id, object);
        }
        *vm = pending.vm.clone();
        if self.cache.len() < 256
            && self
                .cache
                .values()
                .map(|p| p.code.len())
                .sum::<usize>()
                .saturating_add(program.code.len())
                <= 64 * 1024 * 1024
        {
            self.cache
                .insert(pending.name.to_ascii_lowercase(), program);
        }
        self.pending.remove(&owner);
        Ok(true)
    }
    ///7563A0 LABEL115 -> root virtual+8 ->748580 child-first original
    ///motion callbacks. A budget yield keeps both the original clock and KCS
    ///instruction cursor; it never repeats the preceding FES script phase.
    pub fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        bank: &mut Bank,
        clock: u32,
        budget: &mut u32,
    ) -> Result<bool> {
        if !self.trees.contains_key(&owner) && !bank.active_subtree(bank.root)? {
            return Ok(true);
        }
        if !self.trees.contains_key(&owner) {
            if let Some(driver) = self.drivers.get(&owner).filter(|driver| !driver.complete) {
                let status: MotionStatus = serde_json::from_slice(&host.query_engine(
                    client,
                    crysirror_core::service::EngineQuery::LayerMotion(driver.root),
                )?)?;
                let actual = status
                    .drivers
                    .iter()
                    .find(|actual| actual.driver == DRIVER)
                    .context("CatSystem2 native producer was removed outside its lifetime")?;
                ensure!(
                    actual.stamp == driver.stamp && !actual.complete,
                    "CatSystem2 native producer was changed outside its ownership boundary"
                );
                if actual.skip_requested {
                    bank.skip_motions()?;
                }
            }
        }
        if !self.trees.contains_key(&owner) {
            self.trees.insert(
                owner,
                Tree {
                    clock,
                    ids: bank.motion_order()?,
                    next: 0,
                    current: false,
                },
            );
        }
        let tree = self.trees.get_mut(&owner).unwrap();
        ensure!(
            tree.clock == clock,
            "CatSystem2 native motion frame clock changed while pending"
        );
        while let Some(id) = tree.ids.get(tree.next).copied() {
            if *budget == 0 {
                return Ok(false);
            }
            let object = bank
                .objects
                .get_mut(&id)
                .context("CatSystem2 native motion frame lost object")?;
            if !tree.current {
                if !object.motions.any_active()? {
                    tree.next += 1;
                    *budget = budget.saturating_sub(1);
                    continue;
                }
                tree.current = true;
            }
            let before = object.motions.instructions();
            let state = if let Some(random) = &mut self.random {
                object
                    .motions
                    .evaluate_with_services(clock, false, *budget, client, host, random)?
            } else {
                object.motions.evaluate(clock, false, *budget)?
            };
            let used = object.motions.instructions().saturating_sub(before).max(1);
            *budget = budget.saturating_sub(u32::try_from(used).unwrap_or(u32::MAX));
            match state {
                crate::motion_set::Evaluation::Budget => {
                    *budget = 0;
                    return Ok(false);
                }
                crate::motion_set::Evaluation::Ready { visited, pose } => {
                    object.state.apply_motion(visited, pose);
                    if visited && !object.motions.any_active()? {
                        object.motions.commit_output()?;
                    }
                    tree.next += 1;
                    tree.current = false;
                }
            }
        }
        self.trees.remove(&owner);
        Ok(true)
    }
    ///76B7B0: request real KCS skip, evaluate target children then own manager
    ///at clock0, commit its real output, then set SOUND playword0. No track is
    ///deleted or declared complete on behalf of a script that ignores SKIP.
    pub fn stop(&mut self,host:&mut dyn HostServices,client:ClientId,owner:u32,pc:u32,operation:&PendingOperation,bank:&mut Bank,vm:&mut Interpreter,strings:&StringVariables,budget:&mut u32)->Result<bool>{
        ensure!(bank.owner==owner&&!self.trees.contains_key(&owner),"CatSystem2 stop overlaps native motion frame");
        if !self.stops.contains_key(&owner){
            fn subtree(bank:&Bank,id:u32,depth:usize,ids:&mut Vec<u32>)->Result<()>{
                ensure!(depth<64,"CatSystem2 stop subtree cycle/depth");
                ensure!(bank.objects.contains_key(&id),"CatSystem2 stop target missing");
                for child in bank.objects.values().filter(|object|object.parent==Some(id)){subtree(bank,child.id,depth+1,ids)?;}
                ids.push(id);Ok(())
            }
            let mut staged=vm.clone();let targets=bank.targets(&operation.name,&mut staged,strings)?;
            let mut ids=Vec::new();for target in &targets{subtree(bank,*target,0,&mut ids)?;}
            for id in &ids{bank.objects.get_mut(id).unwrap().motions.skip_all()?;}
            self.stops.insert(owner,Stop{pc,operation:operation.clone(),vm:staged,targets,tree:Tree{clock:0,ids,next:0,current:false}});
        }
        let stop=self.stops.get_mut(&owner).unwrap();ensure!(stop.pc==pc&&stop.operation==*operation,"CatSystem2 pending stop original operation changed");
        while let Some(id)=stop.tree.ids.get(stop.tree.next).copied(){
            if *budget==0{return Ok(false);}
            let object=bank.objects.get_mut(&id).context("CatSystem2 stop object disappeared")?;
            if !stop.tree.current {
                if !object.motions.any_active()?{
                    if object.motions.output()?.is_some(){object.motions.commit_output()?;}
                    stop.tree.next+=1;*budget=budget.saturating_sub(1);continue;
                }
                stop.tree.current=true;
            }
            let before=object.motions.instructions();
            let state=if let Some(random)=&mut self.random{object.motions.evaluate_with_services(0,false,*budget,client,host,random)?}else{object.motions.evaluate(0,false,*budget)?};
            let used=object.motions.instructions().saturating_sub(before).max(1);*budget=budget.saturating_sub(u32::try_from(used).unwrap_or(u32::MAX));
            match state{
                crate::motion_set::Evaluation::Budget=>{*budget=0;return Ok(false);},
                crate::motion_set::Evaluation::Ready{visited,pose}=>{object.state.apply_motion(visited,pose);object.motions.commit_output()?;stop.tree.next+=1;stop.tree.current=false;}
            }
        }
        for id in &stop.targets{
            if let crate::native_class_state::State::Sound(sound)=&mut bank.objects.get_mut(id).unwrap().state{
                sound.playing=false;sound.paused=false;sound.stop_requested=true;sound.play_dirty=true;
            }
        }
        *vm=stop.vm.clone();self.stops.remove(&owner);Ok(true)
    }
    pub fn final_command(&self, owner: u32, bank: &Bank) -> Result<Option<LayerCommand>> {
        let Some(driver) = self.drivers.get(&owner).filter(|driver| !driver.complete) else {
            return Ok(None);
        };
        let stamp = ExternalMotionStamp {
            generation: driver.stamp.generation,
            revision: driver
                .stamp
                .revision
                .checked_add(1)
                .context("CatSystem2 native motion revision exhausted")?,
        };
        Ok(Some(LayerCommand::CommitExternalMotion {
            id: driver.root,
            driver: DRIVER,
            stamp,
            complete: !bank.active_subtree(bank.root)?,
            pose: None,
        }))
    }
    pub fn committed(&mut self, owner: u32, command: Option<&LayerCommand>) -> Result<()> {
        if let Some(LayerCommand::CommitExternalMotion {
            stamp, complete, ..
        }) = command
        {
            let driver = self
                .drivers
                .get_mut(&owner)
                .context("CatSystem2 native motion driver lost after frame publication")?;
            ensure!(
                driver.stamp.generation == stamp.generation
                    && driver.stamp.revision.checked_add(1) == Some(stamp.revision),
                "CatSystem2 stale native motion publication receipt"
            );
            driver.stamp = *stamp;
            driver.complete = *complete;
        }
        Ok(())
    }
}
