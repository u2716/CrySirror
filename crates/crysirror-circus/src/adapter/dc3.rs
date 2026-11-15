//! Source-specific DC3WY session owner. Shared Bridge owns I/O workers,
//! immutable uploads, engine UI/layers and the host's PCM output services.
//! This owner never constructs or audits a DC4PH NativeLiveState.
use super::*;
pub(super) fn read_system(
    storage: &dyn ClientStorage,
) -> Result<Option<crate::dc3_settings::SystemData>> {
    let path = VfsPath::new(Mount::Save, "system.dat")?;
    if storage.metadata(Mount::Save, path.relative())?.is_none() {
        return Ok(None);
    }
    let bytes = crysirror_resource::read_all(
        storage.open_read(&path)?.as_ref(),
        crate::dc3_settings::SYSTEM_BYTES as u64,
    )?;
    let data = crate::dc3_settings::SystemData::try_from(bytes)?;
    data.names()?;
    Ok(Some(data))
}
pub struct Dc3Vm {
    client: ClientId,
    config: Config,
    machine: crate::dc3::Machine,
    services: Services,
    booted: bool,
    closing: bool,
    pending_frame: Option<Outcome>,
}
impl Dc3Vm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let mut services = Services::default();
        services.canvas = Some(crate::dc3::CANVAS);
        services.native_profile = Some(crate::Profile::Dc3WithYou);
        services.installation_policy = config.installation_policy;
        //49D828/432E90: first prefix selects native6, subsequent ones5.
        //The shared pool maps these to7/6; these are source resource names,
        //not a second decoder or metadata from the DC4PH executable.
        services.sound_prefixes = Some(vec!["se\\223".into(), "se\\224".into(), "se\\225".into()]);
        services.native_dialogue_visible = true;
        services.native_window_visible = true;
        services.native_status_visible = true;
        services.voice_rate_percent = 100;
        Ok(Self {
            client,
            machine: crate::dc3::Machine::new(config.script.clone()),
            config,
            services,
            booted: false,
            closing: false,
            pending_frame: None,
        })
    }
}
impl Dc3Vm {
    ///D.C.III (0x8a71): the same native interpreter over translated scripts.
    pub fn new_original(client: ClientId, config: Config) -> Result<Self> {
        let mut vm = Self::new(client, config)?;
        vm.machine.state.original = true;
        Ok(vm)
    }
}
impl VmAdapter for Dc3Vm {
    fn engine_id(&self) -> &'static str {
        "circus"
    }
    fn snapshot_schema(&self) -> u32 {
        0xa09f
    }
    fn presentation_pending(&self) -> bool {
        self.services.presentation_pending
    }
    fn request_close(&mut self) -> bool {
        self.closing = true;
        true
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        (|| -> Result<VmYield> {
            self.services.presentation_pending = true;
            //Original40E810 seeds MSVC rand with the process millisecond clock.
            self.services.sequence_rng.get_or_insert_with(|| {
                crysirror_vfx::falling_atlas::SequenceRng::msvc(host.now_ms() as u32)
            });
            if self.services.storage.is_none() {
                self.services.storage = Some(host.storage(self.client)?);
            }
            if !self.booted {
                let Some(Prepared::Dc3System(data)) =
                    self.services.prepare(Work::Dc3SystemLoad, self.client)?
                else {
                    return Ok(VmYield::BudgetExhausted);
                };
                self.machine.state.system_data = data;
                //417E70 calls40CED0(empty,100) after loading preferences.
                //This writes only fallback50's gain; its enable flag remains
                //the installed/persisted source value until native5E resets it.
                self.machine
                    .state
                    .system_data
                    .set_source_value(94, 50, 100, 0)?;
                self.booted = true;
            }
            let now = host.now_ms();
            if self.services.native_clock.is_none() {
                self.services.native_clock =
                    Some(crysirror_core::frame_admission::FrameAdmissionClock::new(
                        now as u32, 1000, 60,
                    )?);
            }
            let mut bridge = Bridge {
                services: &mut self.services,
                host,
                client: self.client,
                font: &self.config.font,
                input,
            };
            if self.closing {
                bridge.close_dc3_music()?;
                let expected = Work::ProfileSystemSave(
                    crate::Profile::Dc3WithYou,
                    self.machine.state.system_data.bytes().to_vec(),
                );
                if let Some(work) = bridge.services.loading.as_ref().map(|job| job.work.clone()) {
                    if work != expected && bridge.services.prepare(work, self.client)?.is_none() {
                        return Ok(VmYield::BudgetExhausted);
                    }
                }
                if !bridge.write_profile_system(
                    crate::Profile::Dc3WithYou,
                    self.machine.state.system_data.bytes(),
                )? {
                    return Ok(VmYield::BudgetExhausted);
                }
                bridge.services.presentation_pending = false;
                return Ok(VmYield::Halted);
            }
            bridge.services.system.observe_window_geometry(
                bridge.host.window_geometry(self.client)?,
                bridge.host.window_fullscreen(self.client)?,
            );
            let cursor_deadline = bridge.update_native_cursor(false)?;
            if !bridge.services.native_iteration_open {
                let clock = bridge.services.native_clock.as_mut().unwrap();
                if !clock.poll(now as u32, false) {
                    bridge.services.presentation_pending = false;
                    return Ok(VmYield::WaitUntil(clock.deadline(now)));
                }
                bridge.services.native_iteration_open = true;
            }
            if bridge.services.audio_update_due {
                bridge.services.audio_update_due = false;
                bridge.advance_native_release_queues()?;
                bridge.advance_native_duck()?;
                bridge.advance_dc3_music()?;
            }
            if self.pending_frame.is_some() {
                return finish_frame(
                    &mut self.machine,
                    &mut bridge,
                    &mut self.pending_frame,
                    now,
                    cursor_deadline,
                );
            }
            //D.C.III: freeze the presented screen before the first frame's
            //script changes when a presented-screen 0x0E effect is ahead.
            let epoch = bridge.services.dc3_original.presented_epoch;
            if self.machine.state.original && bridge.services.dc3_original.lookahead_epoch != Some(epoch) {
                let Some(program) = bridge.scene(&self.machine.state.scene)? else {
                    return Ok(VmYield::BudgetExhausted);
                };
                bridge.services.dc3_original.lookahead_epoch = Some(epoch);
                if self.machine.original_effect_ahead(&program) {
                    bridge.original_screen(&crate::machine::effect_ops::OriginalScreen::Freeze)?;
                }
            }
            for index in 0..budget {
                let Some(program) = bridge.scene(&self.machine.state.scene)? else {
                    return Ok(VmYield::BudgetExhausted);
                };
                let outcome = self.machine.step(
                    &program,
                    &mut bridge,
                    now,
                    index == 0 && bridge_native_press(input),
                    input.buttons_down & crysirror_core::service::buttons::CTRL != 0,
                    index == 0 && input.buttons_pressed & crysirror_core::service::buttons::MOUSE_RIGHT != 0,
                )?;
                match outcome {
                    Outcome::Advanced => continue,
                    Outcome::Pending => {
                        //An engine UI awaiting input is presentable; resource
                        //workers and partial callbacks keep the frame fence.
                        if bridge.callbacks_ready() && bridge.native_ui_waiting() {
                            self.pending_frame = Some(Outcome::WaitForInput);
                            return finish_frame(
                                &mut self.machine,
                                &mut bridge,
                                &mut self.pending_frame,
                                now,
                                cursor_deadline,
                            );
                        }
                        return Ok(VmYield::BudgetExhausted);
                    }
                    Outcome::Halted => {
                        self.closing = true;
                        return Ok(VmYield::BudgetExhausted);
                    }
                    outcome => {
                        self.pending_frame = Some(outcome);
                        return finish_frame(
                            &mut self.machine,
                            &mut bridge,
                            &mut self.pending_frame,
                            now,
                            cursor_deadline,
                        );
                    }
                }
            }
            Ok(VmYield::BudgetExhausted)
        })()
        .map_err(vm_error)
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::Unsupported(
            "DC3WY host snapshot resource reconstruction",
        ))
    }
    fn restore(&mut self, _bytes: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported(
            "DC3WY host snapshot resource reconstruction",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"profile":"dc3wy-a09f","scene":self.machine.state.scene,"pc":self.machine.state.pc,"instructions":self.machine.state.instructions,"native_groups":self.machine.state.templates.len(),"engine_images":self.services.images.len(),"worker_pending":self.services.loading.is_some(),"loading":self.services.loading.as_ref().map(|l|format!("{:?}",l.work).chars().take(300).collect::<String>())})).unwrap_or_default()
    }
}
fn finish_frame(
    machine: &mut crate::dc3::Machine,
    bridge: &mut Bridge<'_>,
    pending: &mut Option<Outcome>,
    now: u64,
    cursor_deadline: Option<u64>,
) -> Result<VmYield> {
    if !machine.publish_resources(bridge)? {
        return Ok(VmYield::BudgetExhausted);
    }
    //4233A0's final loop publishes retained audio declarations only when
    //the native dispatcher yields. A worker retry keeps this exact fence;
    //it must not execute the next script instruction or consume another PC.
    if !machine.publish_audio(bridge).with_context(|| {
        format!(
            "Circus DC3WY {} dispatch fence pc={:#x} opcode={:#04x}",
            machine.state.scene,
            machine.state.last_offset,
            machine.state.last_opcode.unwrap_or(0)
        )
    })? {
        return Ok(VmYield::BudgetExhausted);
    }
    if bridge.services.loading.is_some() || bridge.services.menu_image_job.is_some() {
        return Ok(VmYield::BudgetExhausted);
    }
    if !machine.dispatch_tail(bridge)? {
        return Ok(VmYield::BudgetExhausted);
    }
    if !bridge.prepare_sprite_mask_frame()? {
        return Ok(VmYield::BudgetExhausted);
    }
    bridge.native_frame()?;
    bridge.services.dc3_original.presented_epoch += 1;
    bridge.finish_native_input_frame();
    bridge.services.native_iteration_open = false;
    bridge.services.presentation_pending = false;
    let outcome = pending
        .take()
        .context("DC3WY native frame continuation owner")?;
    let moving = bridge.menu_animating()?
        || !bridge.visual_complete()?
        || bridge.services.text.animating()
        || !bridge.services.music_releases.is_empty();
    let moving = moving || bridge.dc3_music_moving() || bridge.native_duck_active()?;
    if moving {
        return Ok(VmYield::WaitUntil(now.saturating_add(16)));
    }
    Ok(match outcome {
        Outcome::WaitUntil(deadline) => {
            VmYield::WaitUntil(cursor_deadline.map_or(deadline, |cursor| cursor.min(deadline)))
        }
        _ => cursor_deadline.map_or(VmYield::WaitForInput, VmYield::WaitUntil),
    })
}
fn bridge_native_press(input: InputFrame) -> bool {
    input.buttons_pressed
        & (crysirror_core::service::buttons::MOUSE_LEFT
            | crysirror_core::service::buttons::ENTER
            | crysirror_core::service::buttons::SPACE)
        != 0
}
