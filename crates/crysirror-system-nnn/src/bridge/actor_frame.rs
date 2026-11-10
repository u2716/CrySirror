// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! A native PRINT actor update retained across shared image dependencies.
//! Calcu precedes painter traversal; native CountUp follows publication. The
//! one pending message receipt owns RNG but is never advanced by an animation
//! frame. GlyphRuns and their reveal clocks remain retained engine objects.
use super::*;
use crate::native_presentation::{NativeFrameResult, NativeFrameWork};

pub(super) struct ActorFrame {
    admitted: u64,
    clock: crysirror_scene::animation::AdaptiveStepClock,
    steps: u32,
    effects: BTreeMap<i32, NativeFrameEffect>,
    work: NativeFrameWork,
    result: Option<NativeFrameResult>,
    path: Option<String>,
    windows: bool,
    selection: bool,
    pre_published: bool,
    selection_refreshed: bool,
    post_plan: Vec<NativeFrameEffect>,
    post_capture: Option<crysirror_core::image_processing::ImageJob>,
    post_work: Option<NativeFrameWork>,
    post_result: Option<NativeFrameResult>,
    glyph_complete: Option<bool>,
    glyphs_existing: bool,
}

/// CAllEffect::CountUp visits all assigned native effects, including Eye-off
/// slots. This is state policy, not an image operation. MOVE's terminal rect
/// copy and FADEIN's command retirement occur at the actual crossed endpoint.

impl Bridge {
    pub(super) fn has_animated_effects(&self) -> bool {
        self.actor_refresh_required
            || self.post_text_path.is_some()
            || self.active_effects.contains(&15)
            || self.picture_assignments.iter().any(|slot| {
                self.rectangles.get(slot).is_some_and(|r| {
                    let (count, frames) = self.effect_counter(*slot).unwrap_or((0, 0));
                    if count >= frames {
                        r.destination != r.destination_to
                    } else {
                        r.destination_from != r.destination_to
                    }
                })
            })
            || !(self.patterns.is_empty()
                && self.fades.is_empty()
                && self.greys.is_empty()
                && self.negas.is_empty()
                && self.chara_trans.is_empty()
                && self.old_lamps.is_empty()
                && self.buras.is_empty()
                && self.moves.is_empty()
                && self.scrolls.is_empty()
                && self.fusumas.is_empty()
                && self.shakes.is_empty()
                && self.flashes.is_empty()
                && self.picture_effects.is_empty()
                && self.mosaics.is_empty())
    }
    pub(super) fn poll_message_frame(
        &mut self,
        machine: &mut Machine,
        call: Option<&NativeCall>,
        selection: bool,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        use crysirror_core::image_processing::ImageProcessState;
        // Special-window variables and the engine's retained pointer/focus
        // state are read before the post-text capture, without draining input.
        if selection
            && self.actor_frame.is_none()
            && !self.has_animated_effects()
            && self
                .selection
                .as_ref()
                .is_some_and(|s| s.published_selection())
        {
            let variables = match call {
                Some(call) => machine.native_variables_candidate(call)?,
                None => machine
                    .variables()
                    .context("NNN saved selection variable bank absent")?
                    .clone(),
            };
            if !self
                .selection
                .as_mut()
                .unwrap()
                .refresh_special(&variables, host)?
            {
                return Ok(false);
            }
        }
        if self.actor_frame.is_none() && self.actor_ready_for_input {
            return Ok(true);
        }
        if !self.has_animated_effects() && self.actor_frame.is_none() {
            return Ok(true);
        }
        if self.actor_frame.is_none() {
            let now = host.now_ms();
            if self.actor_clock_at == Some(now) {
                return Ok(true);
            }
            let elapsed = self
                .actor_clock_at
                .map(|previous| {
                    now.checked_sub(previous)
                        .context("NNN native actor clock moved backwards")
                })
                .transpose()?
                .unwrap_or(0);
            let mut clock = self.actor_clock.clone();
            let steps = if selection {
                clock.clear()?;
                1
            } else {
                clock.begin(elapsed.try_into()?)?;
                let steps = clock.available(None)?;
                clock.consume_frames(steps)?;
                clock.end()?;
                steps
            };
            let mut effects = self.frame_effects();
            let mut random = match call {
                Some(call) => machine.native_variables_candidate(call)?,
                None => machine
                    .variables()
                    .context("NNN saved actor variable bank absent")?
                    .clone(),
            }
            .random_state()
            .context("NNN actor original random state absent")?;
            // CalcuOnly visits every enabled slot before either Print pass,
            // including the screen layer15. SELECT additionally counts before
            // Print; PRINT counts only after both passes and its glyphs.
            crate::native_presentation::calcu_effect_states(&mut effects, &mut random)?;
            if selection {
                crate::native_presentation::count_effect_states(&mut effects, 1);
            }
            let visible = self.visible_effects(&effects)?;
            let post_visible = BTreeSet::from([15]);
            let post_effects: BTreeMap<_, _> = effects
                .iter()
                .filter(|(slot, _)| **slot == 15)
                .map(|(slot, effect)| (*slot, effect.clone()))
                .collect();
            let post_underlays = self
                .frame_underlays(&effects, &post_visible)?
                .into_iter()
                .filter(|(slot, _)| *slot == 15)
                .collect();
            let post_plan = crate::native_presentation::ordered_frame_plan(
                &post_effects,
                &visible,
                &post_underlays,
            );
            let pre_effects: BTreeMap<_, _> = effects
                .iter()
                .filter(|(slot, _)| **slot < 15)
                .map(|(slot, effect)| (*slot, effect.clone()))
                .collect();
            let first = visible
                .iter()
                .copied()
                .filter(|slot| *slot < 15)
                .next()
                .unwrap_or(15);
            let underlays = self
                .frame_underlays(&effects, &BTreeSet::from([first]))?
                .into_iter()
                .filter(|(slot, _)| *slot < 15)
                .collect();
            let canvas = self.prepare_frame_canvas(host, Some(first))?;
            let work = NativeFrameWork::begin_draw(
                self.client,
                canvas,
                crate::native_presentation::ordered_frame_plan(&pre_effects, &visible, &underlays),
                random,
                false,
                host,
            )?.with_buffers(self.frame_buffers());
            self.api_calls += work.api_calls;
            self.actor_frame = Some(ActorFrame {
                admitted: now,
                clock,
                steps,
                effects,
                work,
                result: None,
                path: None,
                selection,
                windows: !selection
                    && self
                        .text
                        .as_ref()
                        .is_some_and(|text| text.published_message()),
                pre_published: false,
                selection_refreshed: false,
                post_plan,
                post_capture: None,
                post_work: None,
                post_result: None,
                glyph_complete: None,
                glyphs_existing: if selection {
                    self.selection
                        .as_ref()
                        .is_some_and(|selection| selection.published_selection())
                } else {
                    self.text
                        .as_ref()
                        .is_some_and(|text| text.published_message())
                },
            });
        }
        let frame = self.actor_frame.as_mut().unwrap();
        if frame.result.is_none() {
            let before = frame.work.api_calls;
            frame.result = frame.work.poll(host)?;
            self.api_calls += frame.work.api_calls - before;
            if frame.result.is_none() {
                return Ok(false);
            }
        }
        let background = frame.result.as_ref().unwrap().target.clone();
        if !frame.pre_published {
            if frame.windows {
                let text = self
                    .text
                    .as_mut()
                    .context("NNN admitted actor window state absent")?;
                let before = text.api_calls;
                let ready = text.prepare_background_refresh(host, background.clone())?;
                self.api_calls += text.api_calls - before;
                if !ready {
                    return Ok(false);
                }
            }
            let display = self.bindings.effect_base + u32::from(self.bindings.layers);
            if frame.path.is_none() {
                let image =
                    host.publish_image_resource(self.client, display, background.clone())?;
                self.api_calls += 1;
                frame.path = Some(image.path.relative().to_owned());
            }
            let [w, h] = self.bindings.size;
            let mut layers = vec![
                json!({"op":"put","layer":{"id":display,"parent":self.bindings.root,"rect":[0,0,w,h],"order":[self.bindings.layers,0],"paint":{"kind":"image","asset":frame.path.as_ref().unwrap()}}}),
            ];
            if frame.windows {
                layers.extend(
                    self.text
                        .as_ref()
                        .unwrap()
                        .background_refresh_operations()?,
                );
            }
            if frame.post_plan.is_empty() && self.post_text_path.is_some() {
                layers.push(json!({"op":"remove","id":display+1}));
            }
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&json!({"layers":layers}))?,
            )?;
            self.api_calls += 1;
            if let Some(previous) = self.display_path.replace(frame.path.take().unwrap()) {
                host.release_image_resource(self.client, &previous)?;
                self.api_calls += 1;
            }
            if frame.post_plan.is_empty() {
                if let Some(previous) = self.post_text_path.take() {
                    host.release_image_resource(self.client, &previous)?;
                    self.api_calls += 1;
                }
            }
            self.message_background = Some(background.clone());
            if frame.windows {
                let text = self.text.as_mut().unwrap();
                let before = text.api_calls;
                text.accept_background_refresh(host)?;
                self.api_calls += text.api_calls - before;
            } else if !frame.selection {
                if let Some(text) = self.text.as_mut() {
                    text.set_background(background.clone())?;
                }
            }
            frame.pre_published = true;
        }
        if frame.selection
            && !frame.selection_refreshed
            && self
                .selection
                .as_ref()
                .is_some_and(|s| s.published_selection())
        {
            let variables = match call {
                Some(call) => machine.native_variables_candidate(call)?,
                None => machine
                    .variables()
                    .context("NNN saved SELECT variables absent")?
                    .clone(),
            };
            if !self
                .selection
                .as_mut()
                .unwrap()
                .refresh_special(&variables, host)?
            {
                return Ok(false);
            }
            frame.selection_refreshed = true;
        }
        if !frame.post_plan.is_empty() {
            // Initial messages/menus first prepare their actual retained
            // GlyphRuns. This is a dependency phase, never an input wait or
            // invented ACK. The presenter remains on its last full frame.
            let glyphs_ready = if frame.selection {
                self.selection
                    .as_ref()
                    .is_some_and(|selection| selection.published_selection())
            } else {
                self.text
                    .as_ref()
                    .is_some_and(|text| text.published_message())
            };
            if !glyphs_ready {
                return Ok(true);
            }
            if frame.post_capture.is_none()
                && frame.post_work.is_none()
                && frame.post_result.is_none()
            {
                if !frame.selection {
                    frame.glyph_complete =
                        Some(self.text.as_ref().unwrap().shared_reveal_complete(host)?);
                }
                frame.post_capture = Some(host.begin_layer_prefix_image_over(
                    self.client,
                    self.bindings.size,
                    [i32::MAX - 1, 0],
                    [0, 0, 0, 255],
                )?);
                self.api_calls += 1;
            }
            if let Some(job) = frame.post_capture.as_mut() {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(error) => {
                        bail!("NNN shared post-text frame capture: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        let pre = frame.result.as_ref().unwrap();
                        let mut plan = frame.post_plan.clone();
                        if let Some(pattern) = &pre.shared_pattern {
                            for effect in &mut plan {
                                if let NativeFrameEffect::Suna(state) = effect {
                                    state.resource = pattern.clone();
                                }
                            }
                        }
                        let work = NativeFrameWork::with_context_base(
                            self.client,
                            image,
                            plan,
                            pre.random.clone(),
                            false,
                            pre.buffers.clone(),
                            host,
                        )?;
                        self.api_calls += work.api_calls;
                        frame.post_work = Some(work);
                        frame.post_capture = None;
                    }
                }
            }
            if let Some(work) = frame.post_work.as_mut() {
                let before = work.api_calls;
                frame.post_result = work.poll(host)?;
                self.api_calls += work.api_calls - before;
                if frame.post_result.is_none() {
                    return Ok(false);
                }
                frame.post_work = None;
            }
            let post = frame
                .post_result
                .as_ref()
                .context("NNN post-text frame result absent")?;
            let id = self.bindings.effect_base + u32::from(self.bindings.layers) + 1;
            let published = host.publish_image_resource(self.client, id, post.target.clone())?;
            self.api_calls += 1;
            let [w, h] = self.bindings.size;
            if let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":id,"parent":self.bindings.root,"rect":[0,0,w,h],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":published.path.relative()}}}]}))?) {
                let _=host.release_image_resource(self.client,published.path.relative());return Err(error.into());
            }
            self.api_calls += 1;
            if let Some(previous) = self
                .post_text_path
                .replace(published.path.relative().to_owned())
            {
                host.release_image_resource(self.client, &previous)?;
                self.api_calls += 1;
            }
        }
        let mut frame = self.actor_frame.take().unwrap();
        let mut result = frame.result.take().unwrap();
        if let Some(post) = frame.post_result {
            let mut buffers=post.buffers;
            let mut pictures=std::mem::take(&mut result.buffers.picture_updates);
            pictures.extend(buffers.picture_updates);
            buffers.picture_updates=pictures;
            result.buffers=buffers;
            result.target = post.target;
            result.random = post.random;
            result.shared_pattern = post.shared_pattern.or(result.shared_pattern);
            result.lamp_updates.extend(post.lamp_updates);
            result.move_updates.extend(post.move_updates);
            result.scroll_updates.extend(post.scroll_updates);
            result.shake_updates.extend(post.shake_updates);
        }
        self.adopt_frame_buffers(&result.buffers);
        let mut variables = match call {
            Some(call) => machine.native_variables_candidate(call)?,
            None => machine
                .variables()
                .context("NNN saved actor native bank absent")?
                .clone(),
        };
        variables.replace_random_state(result.random.clone())?;
        match call {
            Some(call) => machine.update_native_variables(call, variables)?,
            None => machine.replace_variables(variables)?,
        }
        if !frame.glyphs_existing && !frame.post_plan.is_empty() {
            if frame.selection {
                let _ =
                    host.query_engine(self.client, crysirror_core::service::EngineQuery::UiEvents)?;
                self.api_calls += 1;
            } else if let Some(text) = self.text.as_mut() {
                text.accept_first_presentation(host.now_ms());
            }
        }
        if let Some(text) = self.text.as_mut() {
            text.set_presented_reveal_complete(frame.glyph_complete);
        }
        if let Some(pattern) = &result.shared_pattern {
            for effect in frame.effects.values_mut() {
                if let NativeFrameEffect::Suna(state) = effect {
                    state.resource = pattern.clone();
                }
            }
        }
        for (slot, state) in &result.lamp_updates {
            if let Some(NativeFrameEffect::Lamp(effect)) = frame.effects.get_mut(slot) {
                effect.state = state.clone();
            }
        }
        for (slot,state) in &result.buffers.picture_updates{
            if let Some(NativeFrameEffect::Picture(effect))=frame.effects.get_mut(slot){effect.state=state.clone();}
        }
        for (slot, state) in &result.move_updates {
            if let Some(NativeFrameEffect::Picture(effect)) = frame.effects.get_mut(slot) {
                effect.rectangles = state.rectangles.clone();
            }
            if let Some(NativeFrameEffect::Move(effect)) = frame.effects.get_mut(slot) {
                effect.state = state.clone();
            }
        }
        for (slot, state) in &result.shake_updates {
            if let Some(NativeFrameEffect::Shake(effect)) = frame.effects.get_mut(slot) {
                effect.state = state.clone();
            }
        }
        for (slot, state) in &result.scroll_updates {
            if let Some(NativeFrameEffect::Scroll(effect)) = frame.effects.get_mut(slot) {
                *effect = state.clone();
            }
        }
        if !frame.selection {
            crate::native_presentation::count_effect_states(&mut frame.effects, frame.steps);
        }
        if !frame.selection {
            self.last_native_steps = frame.steps;
        }
        self.actor_clock = frame.clock;
        self.actor_clock_at = Some(frame.admitted);
        self.actor_ready_for_input = true;
        let path = self.display_path.take();
        self.adopt_draw_result(
            crate::native_draw::DrawResult {
                frame: result,
                effects: frame.effects,
                display_path: path,
                clock: None,
            },
            frame.steps.try_into()?,
            host,
        )?;
        self.message_background = Some(background);
        Ok(true)
    }
}
