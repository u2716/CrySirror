// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native DRAW frame waits with engine-owned frame resources. The native
//! virtual frame clock is independent of worker-poll frequency and display FPS.
use crate::{
    machine::{Machine, NativeCall, Payload},
    native_presentation::{NativeFrameEffect, NativeFrameResult, NativeFrameWork},
};
use anyhow::{bail, ensure, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use std::collections::{BTreeMap, BTreeSet};
pub struct DrawResult {
    pub frame: NativeFrameResult,
    pub effects: BTreeMap<i32, NativeFrameEffect>,
    pub display_path: Option<String>,
}
pub struct NativeDraw {
    client: ClientId,
    receipt: NativeCall,
    display: u32,
    order: u16,
    size: [u32; 2],
    frame_ms: u64,
    total: u32,
    started_ms: u64,
    rendered: u32,
    work: Option<NativeFrameWork>,
    last: Option<NativeFrameResult>,
    effects: BTreeMap<i32, NativeFrameEffect>,
    visible: BTreeSet<i32>,
    underlays: BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>,
    display_path: Option<String>,
    pub api_calls: u64,
}
impl NativeDraw {
    pub fn begin(
        client: ClientId,
        display: u32,
        order: u16,
        size: [u32; 2],
        frame_ms: u64,
        call: &NativeCall,
        work: NativeFrameWork,
        effects: BTreeMap<i32, NativeFrameEffect>,
        visible: BTreeSet<i32>,
        underlays: BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>,
        display_path: Option<String>,
        now: u64,
    ) -> Result<Self> {
        let total = match &call.payload {
            Payload::Words { count: 1, words }
                if words.len() == 1 && (0..=864000).contains(&words[0]) =>
            {
                words[0] as u32
            }
            Payload::Control {
                code: 0x5556000c | 0x5556000e | 0x55560013,
                ..
            } if call.identify == 0x66660002 => 1,
            _ => bail!("NNN native DRAW/background-frame receipt"),
        };
        ensure!(
            effects
                .values()
                .all(|effect| !matches!(effect, NativeFrameEffect::Paint(_))),
            "NNN native state map cannot contain paint DTOs"
        );
        Ok(Self {
            client,
            receipt: call.clone(),
            display,
            order,
            size,
            frame_ms,
            total,
            started_ms: now,
            rendered: 0,
            work: Some(work),
            last: None,
            effects,
            visible,
            underlays,
            display_path,
            api_calls: 0,
        })
    }
    pub(crate) fn presented_frame(
        &self,
    ) -> Option<std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>> {
        self.last.as_ref().map(|frame| frame.target.clone())
    }
    pub fn dependency_pending(&self) -> bool {
        self.work.is_some()
    }
    fn poll_admitted_frame(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(work) = self.work.as_mut() {
            let before = work.api_calls;
            let result = work.poll(host);
            self.api_calls += work.api_calls - before;
            let Some(frame) = result? else {
                return Ok(false);
            };
            let published =
                host.publish_image_resource(self.client, self.display, frame.target.clone())?;
            self.api_calls += 1;
            let [w, h] = self.size;
            if let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.display,"rect":[0,0,w,h],"order":[self.order,0],"paint":{"kind":"image","asset":published.path.relative()}}}]}))?){let _=host.release_image_resource(self.client,published.path.relative());return Err(error.into())}
            self.api_calls += 1;
            if let Some(previous) = self
                .display_path
                .replace(published.path.relative().to_owned())
            {
                host.release_image_resource(self.client, &previous)?;
            }
            if let Some(pattern) = &frame.shared_pattern {
                for effect in self.effects.values_mut() {
                    if let NativeFrameEffect::Suna(state) = effect {
                        state.resource = pattern.clone();
                    }
                }
            }
            for (slot, state) in &frame.lamp_updates {
                if let Some(NativeFrameEffect::Lamp(effect)) = self.effects.get_mut(slot) {
                    effect.state = state.clone();
                }
            }
            for (slot, state) in &frame.move_updates {
                if let Some(NativeFrameEffect::Move(effect)) = self.effects.get_mut(slot) {
                    effect.state = state.clone();
                }
            }
            if self.total > 0 {
                // CountUp is after Print. Reaching max disables picture-less
                // FADEIN; SUNA intentionally remains active at its end count.
                self.effects.retain(|_, effect| match effect {
                    NativeFrameEffect::Paint(_) => {
                        unreachable!("paint DTO is not persistent native effect state")
                    }
                    NativeFrameEffect::Move(frame) => {
                        frame.state.count_up();
                        true
                    }
                    NativeFrameEffect::Suna(state) => {
                        if state.count < state.parameters.frames {
                            state.count = (state.count + 1).min(state.parameters.frames);
                        }
                        true
                    }
                    NativeFrameEffect::Grey(state) => {
                        if state.count < state.frames {
                            state.count = state.count.saturating_add(1).min(state.frames);
                        }
                        true
                    }
                    NativeFrameEffect::Lamp(frame) => {
                        frame.state.count = frame.state.count.saturating_add(1).min(999999);
                        true
                    }
                    NativeFrameEffect::Bura(state) => {
                        if state.count < state.frames {
                            state.count = state.count.saturating_add(1).min(state.frames);
                        }
                        true
                    }
                    NativeFrameEffect::Chara(frame) => {
                        if frame.state.count < frame.state.frames {
                            frame.state.count =
                                frame.state.count.saturating_add(1).min(frame.state.frames);
                        }
                        true
                    }
                    NativeFrameEffect::Fade(state) => {
                        if state.count < state.frames {
                            state.count = (state.count + 1).min(state.frames);
                            state.fade_out || state.count != state.frames
                        } else {
                            true
                        }
                    }
                });
            }
            self.last = Some(frame);
            self.work = None;
            self.rendered += 1;
        }
        Ok(true)
    }
    /// An explicit outer-mode exit finishes at most the already admitted frame.
    /// It neither consumes the native receipt nor schedules another DRAW frame.
    pub(crate) fn cancel_at_frame(
        &mut self,
        machine: &Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<DrawResult>> {
        machine.validate_receipt(call)?;
        ensure!(self.receipt == *call, "NNN cancelled DRAW receipt changed");
        if !self.poll_admitted_frame(host)? {
            return Ok(None);
        }
        Ok(Some(DrawResult {
            frame: self
                .last
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("NNN cancelled DRAW has no completed frame"))?
                .clone(),
            effects: self.effects.clone(),
            display_path: self.display_path.take(),
        }))
    }
    pub fn poll(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<DrawResult>> {
        machine.validate_receipt(call)?;
        ensure!(self.receipt == *call, "NNN DRAW receipt changed");
        if !self.poll_admitted_frame(host)? {
            return Ok(None);
        }
        let count = self.total.max(1);
        if self.rendered >= count
            && (self.total == 0
                || host.now_ms().saturating_sub(self.started_ms)
                    >= u64::from(self.total) * self.frame_ms)
        {
            let frame = self.last.as_ref().unwrap();
            let mut vars = machine.native_variables_candidate(call)?;
            vars.replace_random_state(frame.random.clone())?;
            machine.acknowledge_with_variables(call, vars)?;
            return Ok(Some(DrawResult {
                frame: frame.clone(),
                effects: self.effects.clone(),
                display_path: self.display_path.take(),
            }));
        }
        if self.rendered < count
            && host.now_ms().saturating_sub(self.started_ms)
                >= u64::from(self.rendered) * self.frame_ms
        {
            let previous = self.last.as_ref().unwrap();
            let work = NativeFrameWork::with_base(
                self.client,
                previous.base.clone(),
                crate::native_presentation::ordered_frame_plan(
                    &self.effects,
                    &self.visible,
                    &self.underlays,
                ),
                previous.random.clone(),
                host,
            )?;
            self.api_calls += work.api_calls;
            self.work = Some(work);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod outer_exit_tests {
    use super::*;
    use crate::{
        expression::{VariableProfile, Variables},
        Program, XorProfile,
    };
    use crysirror_core::image_processing::{ImageInput, ImageProcessState};
    use crysirror_image::temporal_pattern::PatternRng;
    use std::time::{Duration, Instant};
    #[test]
    fn outer_exit_finishes_one_admitted_frame_without_ack_or_next_frame_or_clock() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"outer exit","content_revision":"1","assets":"assets","canvas":[2,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        let mut job = rt
            .host
            .image_metadata(ClientId(1))
            .unwrap()
            .begin_process(
                ImageInput::Solid {
                    size: [2, 1],
                    color: [1, 2, 3, 255],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let base = loop {
            match job.poll().unwrap() {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(e) => panic!("{e}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        };
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([5, 0x66660006, 0x22220004, 1, 300]);
        words[18] = 37;
        let program = Program::parse(
            &words
                .into_iter()
                .flat_map(|w| (!w).to_le_bytes())
                .collect::<Vec<_>>(),
            XorProfile::default(),
        )
        .unwrap();
        let random = PatternRng::msvc(7);
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.replace_random_state(random.clone()).unwrap();
        let mut machine = Machine::with_variables(variables);
        let crate::machine::Step::Native(call) = machine.poll(&program, 100).unwrap() else {
            panic!()
        };
        let pc = machine.pc();
        let executed = machine.executed();
        let work =
            NativeFrameWork::with_base(ClientId(1), base, vec![], random.clone(), &mut rt.host)
                .unwrap();
        let mut draw = NativeDraw::begin(
            ClientId(1),
            116,
            16,
            [2, 1],
            20,
            &call,
            work,
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeMap::new(),
            None,
            0,
        )
        .unwrap();
        let result = draw
            .cancel_at_frame(&machine, &call, &mut rt.host)
            .unwrap()
            .unwrap();
        assert_eq!(draw.rendered, 1);
        assert!(draw.work.is_none());
        assert_eq!(machine.pc(), pc);
        assert_eq!(machine.executed(), executed);
        machine.validate_receipt(&call).unwrap();
        assert_eq!(rt.host.now_ms(), 0);
        assert_eq!(
            result.frame.target.rgba().as_ref(),
            &[1, 2, 3, 255, 1, 2, 3, 255]
        );
        assert_eq!(
            serde_json::to_vec(&result.frame.random).unwrap(),
            serde_json::to_vec(&random).unwrap()
        );
        assert!(result.display_path.is_some());
    }
}
