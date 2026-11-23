//! A non-looping CBackExecClass keeps calculating/painting retained effects
//! after NEXT. This has no guest instruction, synthetic DRAW or VM ACK.
use super::*;
impl Bridge {
    pub(crate) fn request_background_repaint(&mut self) {
        self.actor_clock_at = None;
    }
    pub(crate) fn idle_background_pending(&self) -> bool {
        self.idle_background
            .as_ref()
            .is_some_and(|idle| idle.dependency_pending())
    }
    pub(crate) fn poll_background_idle(
        &mut self,
        machine: &mut Machine,
        host: &mut dyn HostServices,
        admit: bool,
    ) -> Result<()> {
        ensure!(
            self.background_read_marks.is_some(),
            "NNN retained background painter outside background context"
        );
        if self.idle_background.is_none() {
            if !admit
                || self
                    .actor_clock_at
                    .is_some_and(|at| host.now_ms().saturating_sub(at) < self.frame_time_ms)
            {
                return Ok(());
            }
            let mut clock = crate::native_draw_clock::DrawClock::new(
                self.actor_clock.clone(),
                self.actor_clock_at,
                self.last_background_steps,
                1,
            )?
            .background(self.background_maximum_steps);
            clock.admit(host.now_ms(), false)?;
            let effects = self.frame_effects();
            let visible = self.visible_effects(&effects)?;
            let underlays = self.frame_underlays(&effects, &visible)?;
            let random = machine
                .variables()
                .and_then(|vars| vars.random_state())
                .context("NNN retained background RNG absent")?;
            let canvas = self.prepare_frame_canvas(host, visible.first().copied())?;
            let work = crate::native_presentation::NativeFrameWork::begin_draw(
                self.client,
                canvas,
                crate::native_draw::ordered_draw_plan(&effects, &visible, &underlays, None),
                random,
                true,
                host,
            )?.with_buffers(self.frame_buffers());
            let mut draw = crate::native_draw::NativeDraw::begin_idle(
                self.client,
                self.bindings.effect_base + u32::from(self.bindings.layers),
                self.bindings.layers,
                self.bindings.size,
                self.frame_time_ms,
                work,
                effects,
                visible,
                underlays,
                self.display_path.take(),
                host.now_ms(),
            );
            draw.set_clock(clock)?;
            self.idle_background = Some(draw);
        }
        let idle = self.idle_background.as_mut().unwrap();
        let before = idle.api_calls;
        let result = idle.poll_idle(host);
        self.api_calls += idle.api_calls - before;
        if let Some(result) = result? {
            let mut vars = machine
                .variables()
                .context("NNN retained background variables absent")?
                .clone();
            vars.replace_random_state(result.frame.random.clone())?;
            self.adopt_draw_result(result, 0, host)?;
            machine.replace_variables(vars)?;
            self.idle_background = None;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{VariableProfile, Variables};
    use crysirror_core::service::MemoryAssetReader;
    use std::time::{Duration, Instant};
    #[test]
    fn native_background_idle_keeps_effect_counters_after_next_without_a_guest_call_or_pc_change() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"retained background","content_revision":"1","assets":"game","canvas":[1,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.set_canvas_image_presenter(true);
        rt.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,1,1],"paint":{"kind":"solid","color":[0,0,0,255]}}}]})).unwrap()).unwrap();
        let mut bridge = Bridge::new(
            ClientId(1),
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [1, 1],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge.configure_background_read_marks(false).unwrap();
        bridge.set_frame_time(50).unwrap();
        bridge.active_effects.insert(0);
        bridge.flashes.insert(
            0,
            crate::native_effects_flash::FlashState::parse(&[2, 1, 0, 200, 100, 50]),
        );
        let mut vars = Variables::fresh(VariableProfile::Legacy1000);
        let rng = crysirror_image::temporal_pattern::PatternRng::msvc(7);
        vars.replace_random_state(rng.clone()).unwrap();
        let mut machine = Machine::with_variables(vars);
        let until = Instant::now() + Duration::from_secs(5);
        for elapsed in [0, 50, 50, 50] {
            rt.tick_control(elapsed, InputFrame::default()).unwrap();
            bridge
                .poll_background_idle(&mut machine, &mut rt.host, true)
                .unwrap();
            while bridge.idle_background_pending() {
                rt.prepare_canvas_images(|_, _| {
                    Ok(Some(Arc::new(MemoryAssetReader(Arc::from([0, 0, 0, 255])))))
                })
                .unwrap();
                bridge
                    .poll_background_idle(&mut machine, &mut rt.host, false)
                    .unwrap();
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
            let before = bridge.effect_counter(0);
            for _ in 0..8 {
                bridge
                    .poll_background_idle(&mut machine, &mut rt.host, false)
                    .unwrap();
            }
            assert_eq!(bridge.effect_counter(0), before);
            assert_eq!(machine.pc(), 0);
            assert_eq!(machine.executed(), 0);
        }
        // CountIsMax changes the active command, not previousEffect, which is
        // assigned only by SetEffectScript. This fixture never called it.
        assert_eq!(bridge.previous_effect(0), None);
        assert_eq!(bridge.fill_effects.get(&0), Some(&[200, 100, 50]));
        assert_eq!(bridge.effect_counter(0), Some((2, 2)));
        assert_eq!(machine.variables().unwrap().random_state(), Some(rng));
    }
}
