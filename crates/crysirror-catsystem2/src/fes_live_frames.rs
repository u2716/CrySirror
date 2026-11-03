// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!629850's post-script resource phase. Jobs remain outside cloneable original
//!VM/task state; a frame is complete only after its engine publication receipt.
use crate::{
    fes_object_resources::{Pending, Receipt},
    native_image_publication::ImageList,
    native_interpreter::Interpreter,
    native_values::StringVariables,
    task_policy::NativeTaskPolicy,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, InputObservation},
    state::ClientId,
};
use crysirror_tasks::TaskPool;
use std::collections::BTreeMap;
struct Frame {
    pc: u32,
    observation: InputObservation,
    image_list: u32,
    pending: Pending,
    motion: Option<crysirror_scene::layer::LayerCommand>,
}
struct MotionPhase {
    pc: u32,
    observation: InputObservation,
    ready: bool,
    sound_ready: bool,
}
#[derive(Default)]
pub(crate) struct Frames {
    pending: BTreeMap<u32, Frame>,
    pub(crate) ready: BTreeMap<u32, Receipt>,
    motion: BTreeMap<u32, MotionPhase>,
    sounds: crate::fes_live_sound::Sounds,
}
impl Frames {
    pub fn audio_diagnostics(&self)->serde_json::Value{self.sounds.diagnostics()}
    #[cfg(test)]pub fn audio_resource(&self,channel:u32)->Option<crysirror_core::service::AudioResourceHandle>{self.sounds.resource(channel)}
    pub fn audio_needs_updates(&self)->bool{self.sounds.needs_updates()}
    pub fn observe_audio(&mut self,host:&mut dyn HostServices,client:ClientId,observation:InputObservation)->Result<()>{self.sounds.observe(host,client,observation)}
    pub fn in_flight(&self, owner: u32) -> bool {
        self.pending.contains_key(&owner) || self.motion.contains_key(&owner)
    }
    pub fn protected_owners(&self) -> Vec<u32> {
        self.pending
            .keys()
            .chain(self.motion.keys())
            .copied()
            .collect()
    }
    pub fn reap(&mut self, host:&mut dyn HostServices, client:ClientId, tasks: &NativeTaskPolicy) -> Result<()> {
        self.ready.retain(|owner, _| tasks.fes(*owner).is_ok());
        self.sounds.reap(host,client,tasks)
    }
    pub fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        pool: &TaskPool,
        tasks: &mut NativeTaskPolicy,
        vm: &Interpreter,
        strings: &StringVariables,
        sound_policy: Option<&crate::native_sound_policy::SoundPolicy>,
        motions: &mut crate::fes_live_motions::Motions,
        budget: &mut u32,
        owner: u32,
        pc: u32,
        observation: InputObservation,
    ) -> Result<bool> {
        ensure!(
            observation.client == client,
            "CatSystem2 foreign class-frame observation"
        );
        if tasks.fes(owner)?.object_bank.is_none() {
            return Ok(true);
        }
        if !self.motion.contains_key(&owner) {
            self.motion.insert(
                owner,
                MotionPhase {
                    pc,
                    observation,
                    ready: false,
                    sound_ready: false,
                },
            );
        }
        let phase = self.motion.get_mut(&owner).unwrap();
        ensure!(
            phase.pc == pc && phase.observation == observation,
            "CatSystem2 native motion frame owner/input changed"
        );
        if !phase.ready {
            let clock = (u128::from(observation.time_ms) * 60 / 1000) as u32;
            if !motions.advance(
                host,
                client,
                owner,
                tasks.fes_object_bank_mut(owner)?.unwrap(),
                clock,
                budget,
            )? {
                return Ok(false);
            }
            phase.ready = true;
        }
        if !phase.sound_ready {
            if !self.sounds.advance(host,client,pool,owner,pc,observation,
                tasks.fes_object_bank_mut(owner)?.unwrap(),sound_policy,strings)? { return Ok(false); }
            phase.sound_ready=true;
        }
        crate::fes_text::advance(tasks.fes_object_bank_mut(owner)?.unwrap(),(u128::from(observation.time_ms)*60/1000) as u32,host,"default")?;
        let bank = tasks.fes(owner)?.object_bank.as_ref().unwrap();
        if bank
            .objects
            .values()
            .all(|object| matches!(object.state, crate::native_class_state::State::Base))
        {
            let command = motions.final_command(owner, bank)?;
            if let Some(command) = &command {
                host.submit_engine(
                    client,
                    1,
                    &serde_json::to_vec(&crysirror_scene::ServiceBatch {
                        layers: vec![command.clone()],
                        ..Default::default()
                    })?,
                )?;
            }
            motions.committed(owner, command.as_ref())?;
            self.motion.remove(&owner);
            return Ok(true);
        }
        if !self.pending.contains_key(&owner) {
            let task = tasks.fes(owner)?;
            let Some(bank) = task.object_bank.as_ref() else {
                return Ok(true);
            };
            let list = task
                .image_list
                .context("CatSystem2 class frame has no bound ImageList")?;
            let manager = tasks
                .image_list(list)?
                .manager
                .as_ref()
                .context("CatSystem2 class frame has no prepared engine atlas")?;
            ensure!(
                manager.resource.client() == client,
                "CatSystem2 class frame atlas owner changed"
            );
            let upload = task.upload_position.context("CatSystem2 class frame has no atlas base")?;
            let mut pending = Pending::begin_bank(
                pool,
                host,
                client,
                pc,
                bank.clone(),
                vm.clone(),
                strings.clone(),
                self.ready.get(&owner).cloned(),
            )?;
            pending.upload = upload;
            let motion = motions.final_command(owner, bank)?;
            self.pending.insert(
                owner,
                Frame {
                    pc,
                    observation,
                    image_list: list,
                    pending,
                    motion,
                },
            );
        }
        let frame = self.pending.get_mut(&owner).unwrap();
        ensure!(
            frame.pc == pc && frame.observation == observation,
            "CatSystem2 class-frame native PC/input changed while pending"
        );
        if !frame.pending.poll(host)? {
            return Ok(false);
        }
        let root = tasks
            .fes(owner)?
            .object_root
            .context("CatSystem2 class frame lost its engine root")?;
        let list = tasks.image_list(frame.image_list)?;
        let publication = if let Some(publication) = &list.publication {
            publication.clone()
        } else {
            ImageList::new(
                client,
                0x58000000u32
                    .checked_add(frame.image_list)
                    .context("CatSystem2 ImageList Canvas range")?,
                list.manager
                    .as_ref()
                    .context("CatSystem2 class frame lost its atlas")?
                    .size,
            )?
        };
        let receipt = frame.pending.publish_frame(
            host,
            client,
            root,
            &publication,
            frame.motion.iter().cloned().collect(),
        )?;
        tasks.commit_fes_class_frame(owner, frame.image_list, &receipt)?;
        motions.committed(owner, frame.motion.as_ref())?;
        self.ready.insert(owner, receipt);
        self.pending.remove(&owner);
        self.motion.remove(&owner);
        Ok(true)
    }
}
