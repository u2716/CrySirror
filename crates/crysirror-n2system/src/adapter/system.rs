// SPDX-License-Identifier: MPL-2.0
//! Native system requests retain a completion receipt across owner ticks.
//! DateTime/String/Random argument policy follows NitroSharp 77a93db4 (MIT).
//! Windows Message button values follow the shipped NSS comparisons, rather
//! than Win32's unrelated IDYES/IDNO values.
use super::*;
use crysirror_core::image_processing::ImageProcessState;
use crysirror_core::service::{CivilTimeState, SessionSeedSource, SessionSeedState};
use crysirror_scene::ui::dialog::{DialogButton, DialogSpec};

impl N2SystemVm {
    pub(super) fn update_native_system(
        &mut self,
        host: &mut dyn HostServices,
        input: &InputFrame,
    ) -> Result<()> {
        if let Some((id, _)) = self.native_dialog {
            let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::DialogEvents)?,
            )?;
            for event in events {
                ensure!(
                    event.widget.owner == self.client.0
                        && event.widget.id == id
                        && event.kind == "dialog_result",
                    "N2System unexpected message completion"
                );
                ensure!(
                    matches!(event.value, 1. | 2. | 3. | 4.),
                    "N2System message result"
                );
                self.native_dialog_results.insert(id, event.value as u32);
                self.native_dialog = None;
            }
        }
        // Native keydown variables are press edges, not held-key states
        // (reference InputSnapshot.IsKeyDown / InputContext.Update).
        for (name, mask) in [
            ("SYSTEM_r_button_down", buttons::MOUSE_RIGHT),
            ("SYSTEM_l_button_down", buttons::MOUSE_LEFT),
            ("SYSTEM_keydown_enter", buttons::ENTER),
            ("SYSTEM_keydown_esc", buttons::ESCAPE),
        ] {
            self.machine.variables.insert(
                name.into(),
                Value::Boolean(input.buttons_pressed & mask != 0),
            );
        }
        for &(name, mask) in buttons::NAMES {
            self.machine.variables.insert(
                format!("SYSTEM_keydown_{name}"),
                Value::Boolean(input.buttons_pressed & mask != 0),
            );
        }
        // Hideable backgrounds and dialogue pages share the native text-erase
        // switch. Retain which visible nodes this switch hid so unrelated
        // hidden layers are not revealed when text returns.
        let erased = self
            .machine
            .variables
            .get("SYSTEM_text_erase")
            .map(Value::truth)
            .transpose()?
            .unwrap_or(false);
        let mut visibility = Vec::new();
        if erased {
            let ids = self
                .entities
                .nodes
                .values()
                .filter(|e| e.hideable || self.pages.contains_key(&e.id))
                .map(|e| e.id)
                .collect::<Vec<_>>();
            for id in ids {
                if !self.hidden_for_text.contains(&id) && self.layer(host, id)?["visible"] == true {
                    visibility.push(json!({"op":"set_visible","id":id,"visible":false}));
                    self.hidden_for_text.insert(id);
                }
            }
        } else {
            for id in std::mem::take(&mut self.hidden_for_text) {
                if self.entities.nodes.contains_key(&id) {
                    visibility.push(json!({"op":"set_visible","id":id,"visible":true}));
                }
            }
        }
        self.submit(host, visibility, vec![])?;
        if let Some(value) = self.machine.flags.get("SYSTEM_window_full") {
            let full = value.truth()?;
            if self.fullscreen != Some(full) {
                self.submit(
                    host,
                    vec![],
                    vec![json!({"op":"set_fullscreen","enabled":full})],
                )?;
                self.fullscreen = Some(full);
            }
        }
        for (kind, suffix) in [("BGM", "bgm"), ("SE", "se"), ("Voice", "voice")] {
            let enabled = self
                .machine
                .flags
                .get(&format!("SYSTEM_sound_{suffix}"))
                .map(Value::truth)
                .transpose()?
                .unwrap_or(true);
            let gain = self
                .machine
                .flags
                .get(&format!("SYSTEM_sound_volume_{suffix}"))
                .map(Value::number)
                .transpose()?
                .unwrap_or(1000.)
                / 1000.;
            let settings = crysirror_core::service::AudioMixGroupSettings {
                gain: gain.clamp(0., 4.),
                muted: !enabled,
                ..Default::default()
            };
            let group = format!("n2system-{kind}");
            if host
                .audio_mix_group(self.client, &group)?
                .is_none_or(|g| g.settings != settings)
            {
                host.submit_audio(
                    self.client,
                    AudioRequest::SetMixGroup {
                        name: group,
                        settings,
                        fade_ms: 0.,
                    },
                )?;
            }
        }
        if let Some(id) = self.interrupt {
            if self
                .threads
                .get(&id)
                .is_none_or(|state| state.thread.halted)
            {
                if self.entities.nodes.contains_key(&id) {
                    self.destroy(host, id)?;
                }
                self.interrupt = None;
            }
        }
        if let Some((path, task)) = &mut self.shortcut_job {
            if let Some(modules) = task.try_take()? {
                let path = path.clone();
                for (name, module) in modules {
                    self.machine.insert(&name, module)?;
                }
                self.shortcut_job = None;
                self.start_shortcut(host, &path)?;
            }
        }
        let menu_locked = self
            .machine
            .variables
            .get("SYSTEM_menu_lock")
            .map(Value::truth)
            .transpose()?
            .unwrap_or(false);
        if !menu_locked
            && self.interrupt.is_none()
            && self.shortcut_job.is_none()
            && self.native_dialog.is_none()
        {
            if let Some(path) = self
                .shortcuts
                .iter()
                .find(|(mask, _)| input.buttons_pressed & **mask != 0)
                .map(|(_, path)| path.clone())
            {
                let module = vm::module_name(path.split("->").next().unwrap())?;
                if self.machine.modules.contains_key(&module) {
                    self.start_shortcut(host, &path)?;
                } else {
                    let storage = self.storage.as_ref().unwrap().clone();
                    let catalog = self.catalog.as_ref().unwrap().clone();
                    let config = self.config.clone();
                    let existing = self.machine.modules.keys().cloned().collect();
                    let task = self.pool.as_ref().unwrap().spawn(move |cancel| {
                        resource::modules(
                            storage,
                            catalog,
                            &config.script_root,
                            &module,
                            &config.encoding,
                            existing,
                            &cancel,
                        )
                    })?;
                    self.shortcut_job = Some((path, task));
                }
            }
        }
        Ok(())
    }

    fn start_shortcut(&mut self, host: &mut dyn HostServices, path: &str) -> Result<()> {
        ensure!(
            self.threads.len() < 255,
            "N2System interrupt thread capacity"
        );
        let (module, routine) = path
            .split_once("->")
            .map_or((path, None), |(m, r)| (m, Some(r)));
        let thread = self.machine.start(module, routine, "shortcut".into())?;
        let id = self.entities.root("shortcut".into())?;
        self.submit(host, vec![self.entity_group(id)?], vec![])?;
        let mut execution = Execution::new(thread);
        execution.interrupt = true;
        self.threads.insert(id, execution);
        self.interrupt = Some(id);
        Ok(())
    }

    pub(super) fn native_system(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let result = match call.function {
            Builtin::System => {
                use crysirror_core::service::{
                    DesktopRequest, DesktopRequestState, DesktopResponse,
                };
                // Original trial 415E70: only exact uppercase OPEN: is active;
                // other commands explicitly do nothing. Optional operands are
                // ignored for URLs. Arbitrary executable launch is not an
                // authority provided by the portable host's OpenUrl capability.
                if state.desktop.is_none() {
                    let command = args.string(0)?;
                    let Some(url) = command.strip_prefix("OPEN:") else {
                        self.machine.complete(&mut state.thread, Value::Null)?;
                        return Ok(true);
                    };
                    let request = DesktopRequest::OpenUrl {
                        url: url.to_owned(),
                    };
                    request.validate()?;
                    state.desktop = Some(host.begin_desktop_request(self.client, request)?);
                }
                match state.desktop.as_mut().unwrap().poll()? {
                    DesktopRequestState::Pending => return Ok(false),
                    DesktopRequestState::Ready(DesktopResponse::Opened) => state.desktop = None,
                    DesktopRequestState::Ready(_) => {
                        bail!("N2System System OPEN returned an unrelated host response")
                    }
                }
                Value::Null
            }
            Builtin::SetShortcut => {
                let key = args.string(0)?.to_ascii_lowercase();
                let mask = buttons::NAMES
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, mask)| *mask)
                    .with_context(|| format!("N2System shortcut key unavailable: {key}"))?;
                let path = args.string(1)?;
                if path.is_empty() {
                    self.shortcuts.remove(&mask);
                } else {
                    vm::module_name(path.split("->").next().unwrap())?;
                    self.shortcuts.insert(mask, path);
                }
                Value::Null
            }
            Builtin::IsInterrupt => Value::Boolean(state.interrupt),
            // This asks for the original Direct3D HAL, not whether an unrelated
            // graphics API is accelerated. Portable hosts expose no D3D HAL.
            Builtin::EnableDirect3DHAL => Value::Boolean(false),
            Builtin::LockVideo => {
                const HOLD: u32 = 0x7fff_0001;
                if args.truth(0)? {
                    if self.video_hold.is_none() {
                        if self.video_capture.is_none() {
                            self.video_capture = Some(host.begin_layer_prefix_image_over(
                                self.client,
                                self.config.canvas,
                                [i32::MAX, i32::MAX],
                                [0, 0, 0, 255],
                            )?);
                        }
                        match self.video_capture.as_mut().unwrap().poll()? {
                            ImageProcessState::Pending => return Ok(false),
                            ImageProcessState::Failed(error) => {
                                bail!("N2System display hold: {error}")
                            }
                            ImageProcessState::Ready(image) => {
                                let uploaded =
                                    host.publish_image_resource(self.client, HOLD, image)?;
                                let path = uploaded.path.relative().to_owned();
                                self.submit(host,vec![json!({"op":"put","layer":{"id":HOLD,"order":[i32::MAX,i32::MAX],"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"image","asset":path}}})],vec![])?;
                                self.video_hold = Some(path);
                                self.video_capture = None;
                            }
                        }
                    }
                } else if let Some(path) = self.video_hold.take() {
                    self.submit(host, vec![json!({"op":"remove","id":HOLD})], vec![])?;
                    host.release_image_resource(self.client, &path)?;
                }
                Value::Null
            }
            Builtin::Message => {
                if let Some(id) = state.dialog {
                    let Some(result) = self.native_dialog_results.remove(&id) else {
                        return Ok(false);
                    };
                    state.dialog = None;
                    Value::Number(result as f32)
                } else {
                    if self.native_dialog.is_some() {
                        return Ok(false);
                    }
                    let kind = args.string(2)?.to_ascii_uppercase();
                    let buttons: &[(&str, u32)] = match kind.as_str() {
                        "" | "OK" => &[("OK", 1)],
                        "YESNO" => &[("Yes", 2), ("No", 3)],
                        other => bail!("N2System Message button profile {other}"),
                    };
                    let dialog = DialogSpec {
                        id: 1,
                        title: args.string(0)?,
                        message: args.string(1)?,
                        buttons: buttons
                            .iter()
                            .map(|(label, id)| DialogButton {
                                id: *id,
                                label: (*label).into(),
                            })
                            .collect(),
                        default: buttons[0].1,
                        cancel: Some(buttons.last().unwrap().1),
                        font: self.style.font.clone(),
                        text_size: self.style.size.clamp(8, 128),
                    };
                    dialog.validate()?;
                    self.submit(
                        host,
                        vec![],
                        vec![json!({"op":"open_dialog","dialog":dialog})],
                    )?;
                    self.native_dialog = Some((1, current));
                    state.dialog = Some(1);
                    return Ok(false);
                }
            }
            Builtin::DateTime => {
                let references = (0..6)
                    .map(|i| args.reference(i).cloned())
                    .collect::<Result<Vec<_>>>()?;
                if state.civil.is_none() {
                    state.civil = Some(host.begin_civil_time(self.client)?);
                }
                match state.civil.as_mut().unwrap().poll()? {
                    CivilTimeState::Pending => return Ok(false),
                    CivilTimeState::Failed(error) => bail!("N2System civil clock: {error}"),
                    CivilTimeState::Ready(time) => {
                        ensure!(time.client == self.client, "N2System civil clock owner");
                        let values = [
                            time.year,
                            time.month.into(),
                            time.day.into(),
                            time.hour.into(),
                            time.minute.into(),
                            time.second.into(),
                        ];
                        for (reference, value) in references.iter().zip(values) {
                            self.machine
                                .assign(reference, Value::Number(value as f32))?;
                        }
                        state.civil = None;
                        Value::Null
                    }
                }
            }
            Builtin::Random => {
                let max = args.integer(0)?;
                ensure!(max >= 0, "N2System negative random upper bound");
                if max == 0 {
                    Value::Number(0.)
                } else {
                    if self.rng.is_none() {
                        if self.seed.is_none() {
                            self.seed = Some(host.begin_session_seed(
                                self.client,
                                SessionSeedSource::Os { words: 1 },
                            )?);
                        }
                        match self.seed.as_mut().unwrap().poll()? {
                            SessionSeedState::Pending => return Ok(false),
                            SessionSeedState::Failed(error) => {
                                bail!("N2System random seed: {error}")
                            }
                            SessionSeedState::Cancelled => bail!("N2System random seed cancelled"),
                            SessionSeedState::Ready(material) => {
                                ensure!(
                                    material.client == self.client && material.words.len() == 1,
                                    "N2System random seed owner/extent"
                                );
                                self.rng =
                                    Some(crysirror_core::random::Mt19937::new(material.words[0]));
                                self.seed = None;
                            }
                        }
                    }
                    // Rejection sampling preserves [0,max) without modulo bias.
                    // This is an owned portable stream, not the Windows CRT RNG.
                    let bound = max as u32;
                    let threshold = bound.wrapping_neg() % bound;
                    let value = loop {
                        let n = self.rng.as_mut().unwrap().next_u32();
                        if n >= threshold {
                            break n % bound;
                        }
                    };
                    Value::Number(value as f32)
                }
            }
            Builtin::String => {
                let format = args.string(0)?;
                let strings = (1..args.len())
                    .map(|i| args.string(i))
                    .collect::<Result<Vec<_>>>()?;
                let arguments = (1..args.len())
                    .map(|i| {
                        if matches!(args.value(i), Value::Number(_)) {
                            Ok(crysirror_text::printf_values::Value::Number(f64::from(
                                args.value(i).number()?,
                            )))
                        } else {
                            Ok(crysirror_text::printf_values::Value::Text(&strings[i - 1]))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                let output =
                    crysirror_text::printf_values::format(&format, &arguments, 1024 * 1024)?;
                if output.invalid {
                    host.append_log(
                        self.client,
                        "N2System String: invalid format argument retained in output",
                    )?;
                }
                Value::String(output.text)
            }
            Builtin::MilliTime => {
                Value::Number(host.now_ms().saturating_sub(self.start_ms.unwrap_or(0)) as f32)
            }
            Builtin::EndScene => {
                self.machine.complete(&mut state.thread, Value::Null)?;
                state.thread.frames.clear();
                state.thread.stack.clear();
                state.thread.halted = true;
                return Ok(true);
            }
            function => bail!(
                "N2System native system service {} is not connected",
                function.name()
            ),
        };
        self.machine.complete(&mut state.thread, result)?;
        Ok(true)
    }
}
