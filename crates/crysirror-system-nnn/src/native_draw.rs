// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native DRAW frame waits with engine-owned frame resources. The native
//! virtual frame clock is independent of worker-poll frequency and display FPS.
use crate::{
    machine::{Machine, NativeCall, Payload},
    native_presentation::{NativeFrameEffect, NativeFrameResult, NativeFrameWork},
};
use anyhow::{Result, bail, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn ordered_draw_plan(
    effects: &BTreeMap<i32, NativeFrameEffect>,
    visible: &BTreeSet<i32>,
    underlays: &BTreeMap<i32, crysirror_core::image_processing::ImageCompositeLayer>,
    view: Option<&crate::native_text::DrawView>,
) -> Vec<NativeFrameEffect> {
    let Some(view) = view else {
        return crate::native_presentation::ordered_frame_plan(effects, visible, underlays);
    };
    let before = effects
        .iter()
        .filter(|(slot, _)| **slot < 15)
        .map(|(slot, effect)| (*slot, effect.clone()))
        .collect();
    let under = underlays
        .iter()
        .filter(|(slot, _)| **slot < 15)
        .map(|(slot, layer)| (*slot, layer.clone()))
        .collect();
    let mut plan = crate::native_presentation::ordered_frame_plan(&before, visible, &under);
    plan.push(NativeFrameEffect::MessageBackground);
    plan.extend(view.paints.iter().cloned().map(NativeFrameEffect::Paint));
    let after = effects
        .iter()
        .filter(|(slot, _)| **slot >= 15)
        .map(|(slot, effect)| (*slot, effect.clone()))
        .collect();
    let under = underlays
        .iter()
        .filter(|(slot, _)| **slot >= 15)
        .map(|(slot, layer)| (*slot, layer.clone()))
        .collect();
    plan.extend(crate::native_presentation::ordered_frame_plan(
        &after, visible, &under,
    ));
    plan
}
pub struct DrawResult {
    pub frame: NativeFrameResult,
    pub effects: BTreeMap<i32, NativeFrameEffect>,
    pub display_path: Option<String>,
    pub(crate) clock: Option<crate::native_draw_clock::DrawClock>,
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
    message_view: Option<crate::native_text::DrawView>,
    adaptive: Option<crate::native_draw_clock::DrawClock>,
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
            effects.values().all(|effect| !matches!(
                effect,
                NativeFrameEffect::Paint(_) | NativeFrameEffect::MessageBackground
            )),
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
            message_view: None,
            adaptive: None,
            api_calls: 0,
        })
    }
    pub(crate) fn set_clock(&mut self, clock: crate::native_draw_clock::DrawClock) -> Result<()> {
        ensure!(
            self.rendered == 0 && self.last.is_none(),
            "NNN DRAW clock changed after admission"
        );
        self.adaptive = Some(clock);
        Ok(())
    }
    pub(crate) fn set_message_view(&mut self, view: crate::native_text::DrawView) -> Result<()> {
        ensure!(
            self.rendered == 0 && self.last.is_none(),
            "NNN DRAW view changed after publication"
        );
        self.message_view = Some(view);
        Ok(())
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
            let mut node = serde_json::json!({"op":"put","layer":{"id":self.display,"rect":[0,0,w,h],"order":[self.order,0],"paint":{"kind":"image","asset":published.path.relative()}}});
            let mut layers = Vec::new();
            if let Some(view) = &self.message_view {
                node["layer"]["parent"] = serde_json::json!(view.root);
                layers.extend(
                    view.hide
                        .iter()
                        .map(|id| serde_json::json!({"op":"set_visible","id":id,"visible":false})),
                );
            }
            layers.push(node);
            if let Err(error) = host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&serde_json::json!({"layers":layers}))?,
            ) {
                let _ = host.release_image_resource(self.client, published.path.relative());
                return Err(error.into());
            }
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
                if let Some(NativeFrameEffect::Picture(effect)) = self.effects.get_mut(slot) {
                    effect.rectangles = state.rectangles.clone();
                }
                if let Some(NativeFrameEffect::Move(effect)) = self.effects.get_mut(slot) {
                    effect.state = state.clone();
                }
            }
            for (slot, state) in &frame.shake_updates {
                if let Some(NativeFrameEffect::Shake(effect)) = self.effects.get_mut(slot) {
                    effect.state = state.clone();
                }
            }
            for (slot, state) in &frame.scroll_updates {
                if let Some(NativeFrameEffect::Scroll(effect)) = self.effects.get_mut(slot) {
                    *effect = state.clone();
                }
            }
            let steps = self
                .adaptive
                .as_ref()
                .map_or(u32::from(self.total > 0), |clock| clock.steps);
            crate::native_presentation::count_effect_states(&mut self.effects, steps);
            if let Some(clock) = self.adaptive.as_mut() {
                clock.complete_frame()?;
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
            clock: self.adaptive.clone(),
        }))
    }
    pub fn poll(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<DrawResult>> {
        self.poll_with_admission(machine, call, host, true, false)
    }
    pub(crate) fn poll_with_admission(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
        admit: bool,
        skip: bool,
    ) -> Result<Option<DrawResult>> {
        machine.validate_receipt(call)?;
        ensure!(self.receipt == *call, "NNN DRAW receipt changed");
        if !self.poll_admitted_frame(host)? {
            return Ok(None);
        }
        if self.adaptive.is_some() {
            if self.adaptive.as_ref().unwrap().finished() {
                let frame = self.last.as_ref().unwrap();
                let mut vars = machine.native_variables_candidate(call)?;
                vars.replace_random_state(frame.random.clone())?;
                machine.acknowledge_with_variables(call, vars)?;
                return Ok(Some(DrawResult {
                    frame: frame.clone(),
                    effects: self.effects.clone(),
                    display_path: self.display_path.take(),
                    clock: self.adaptive.clone(),
                }));
            }
            if !admit || self.adaptive.as_ref().unwrap().at == Some(host.now_ms()) {
                return Ok(None);
            }
            self.adaptive.as_mut().unwrap().admit(host.now_ms(), skip)?;
            let previous = self.last.as_ref().unwrap();
            let work = NativeFrameWork::with_base(
                self.client,
                previous.base.clone(),
                ordered_draw_plan(
                    &self.effects,
                    &self.visible,
                    &self.underlays,
                    self.message_view.as_ref(),
                ),
                previous.random.clone(),
                host,
            )?;
            self.api_calls += work.api_calls;
            self.work = Some(work);
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
                clock: self.adaptive.clone(),
            }));
        }
        if admit
            && self.rendered < count
            && host.now_ms().saturating_sub(self.started_ms)
                >= u64::from(self.rendered) * self.frame_ms
        {
            let previous = self.last.as_ref().unwrap();
            let work = NativeFrameWork::with_base(
                self.client,
                previous.base.clone(),
                ordered_draw_plan(
                    &self.effects,
                    &self.visible,
                    &self.underlays,
                    self.message_view.as_ref(),
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
        Program, XorProfile,
        expression::{VariableProfile, Variables},
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
