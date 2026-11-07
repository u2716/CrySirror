//! Native animated-fade image imports use immutable engine Canvas paint.
//! ABMP traversal publishes ordinary Layer nodes; no composed frame is uploaded.
use super::*;
use crysirror_core::service::EngineQuery;
impl QlieVm {
    pub(super) fn animated_transition(
        &mut self,
        h: &mut dyn HostServices,
        path: &str,
        duration: u64,
        priority: i32,
        easing: crysirror_scene::layer::Easing,
        force_second_capture: bool,
    ) -> Result<bool> {
        let capture = self.capture_id(0);
        let clock = self.capture_id(1);
        let second_capture = self.capture_id(2);
        let eff = Self::builtin("eff", self.host_thread);
        let root = self.root_layer_for(self.host_thread);
        self.rule_fade = None;
        if self.objects[&eff].asset.as_deref() != Some(path) || self.objects[&eff].preparing {
            if !self.load_image(h, &eff, path)? {
                return Ok(false);
            }
        }
        let bundle = self.objects[&eff]
            .bundle
            .as_ref()
            .context("QLIE animated transition resource")?;
        let slot = bundle
            .images
            .first()
            .context("QLIE transition capture slot")?
            .slot;
        // Native6BA0F8 binds the frozen old scene to import0 and, when
        // *CaptureFadein=1 (or the explicit native flag), the current scene
        // to import1. Both remain engine Canvas paint, not uploaded screens.
        let second_slot = if bundle.animation.capture_both || force_second_capture {
            Some(
                bundle
                    .images
                    .get(1)
                    .context("QLIE second transition capture slot")?
                    .slot,
            )
        } else {
            None
        };
        let motion = bundle
            .animation
            .motions
            .first()
            .context("QLIE transition motion")?;
        // TImoGraphic uses motion0's declared duration (+10 for the sampled
        // endpoint), not padded editor keys that may lie beyond that duration.
        let end = motion.duration_ms + 10;
        if second_slot.is_some() {
            let mut commands = Vec::new();
            if !self.second_capture {
                commands.push(json!({"op":"create","id":second_capture,"width":self.config.canvas[0],"height":self.config.canvas[1]}));
            }
            commands.push(json!({"op":"clear","id":second_capture,"color":[0,0,0,255]}));
            commands.push(json!({"op":"copy_subtree","source":root,"destination":second_capture,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"position":[0,0]}));
            if !self.capture_scene(h, commands, priority)? {
                return Ok(false);
            }
            self.second_capture = true;
        }
        let mut presentations = vec![
            json!({"op":"present_target","id":capture,"layer":capture,"parent":root,"order":priority}),
        ];
        let mut visibility = vec![json!({"op":"set_visible","id":capture,"visible":false})];
        if second_slot.is_some() {
            presentations
                .push(json!({"op":"present_target","id":second_capture,"layer":second_capture,"parent":root,"order":priority}));
            visibility.push(json!({"op":"set_visible","id":second_capture,"visible":false}));
        }
        self.submit(h, json!({"canvases":presentations,"layers":visibility}))?;
        let capture: serde_json::Value =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(capture))?)?;
        let second = if let Some(slot) = second_slot {
            Some((
                slot,
                serde_json::from_slice::<serde_json::Value>(
                    &h.query_engine(self.client, EngineQuery::Layer(second_capture))?,
                )?,
            ))
        } else {
            None
        };
        let o = self.objects.get_mut(&eff).unwrap();
        o.capture_views.clear();
        o.capture_views.insert(slot, capture);
        if let Some((slot, view)) = second {
            o.capture_views.insert(slot, view);
        }
        o.position = [0, 0, priority];
        o.offset = 0;
        o.started = h.now_ms();
        o.visible = true;
        o.motion = 0;
        o.events.clear();
        o.animation_clocks.clear();
        o.properties.insert("ps".into(), vec![Value::Int(0)]);
        self.put_parent(h, &eff)?;
        let mut commands = Vec::new();
        if !self.effect_clock {
            commands.push(json!({"op":"put","layer":{"id":clock,"parent":root,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
        } else {
            commands.push(json!({"op":"end_motion","id":clock}));
        }
        commands.push(json!({"op":"begin_motion","id":clock,"track":{"property":"x","from":0,"to":end,"duration":duration,"easing":easing}}));
        self.submit(h, json!({"layers":commands}))?;
        self.effect_clock = true;
        self.animated_fade = Some(end);
        self.capture_waiting = false;
        self.effect_priority = priority;
        self.update_visuals(h)?;
        Ok(true)
    }
    pub(super) fn cancel_animated_transition(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.animated_fade.take().is_some() {
            let eff = Self::builtin("eff", self.host_thread);
            let o = self.objects.get_mut(&eff).unwrap();
            o.visible = false;
            if o.created {
                let layer = o.layer;
                self.submit(
                    h,
                    json!({"layers":[{"op":"set_visible","id":layer,"visible":false}]}),
                )?;
            }
        }
        Ok(())
    }
}
