//! Separate DC2 native owner. Assets are streamed through the same scoped
//! storage/image workers and engine-owned UI; no DC4 state/save projection or
//! original executable is used. Native process/operation gaps stay explicit.
use super::*;
#[path = "dc2_menu_view.rs"]
mod menu;
#[path = "dc2_map_view.rs"]
mod map;
pub(super) fn read_system(
    storage: &dyn ClientStorage,
) -> Result<Option<crate::dc2_settings::SystemData>> {
    let path = VfsPath::new(Mount::Save, "system.dat")?;
    if storage.metadata(Mount::Save, path.relative())?.is_none() {
        return Ok(None);
    }
    let bytes = crysirror_resource::read_all(
        storage.open_read(&path)?.as_ref(),
        crate::dc2_settings::SYSTEM_BYTES as u64,
    )?;
    let data = crate::dc2_settings::SystemData::try_from(bytes)?;
    data.names()?;
    Ok(Some(data))
}
pub struct Dc2Vm {
    menu: menu::View,
    map: map::View,
    client: ClientId,
    config: Config,
    machine: crate::dc2::Machine,
    services: Services,
    booted: bool,
    closing: bool,
}
impl Dc2Vm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let mut services = Services::default();
        services.canvas = Some(crate::dc2::CANVAS);
        services.native_profile = Some(crate::Profile::Dc2PlusCommunication);
        services.installation_policy = config.installation_policy;
        services.native_dialogue_visible = true;
        services.native_window_visible = true;
        services.native_status_visible = true;
        Ok(Self {
            menu: Default::default(),
            map: Default::default(),
            client,
            machine: crate::dc2::Machine::new(config.script.clone()),
            config,
            services,
            booted: false,
            closing: false,
        })
    }
}
impl VmAdapter for Dc2Vm {
    fn engine_id(&self) -> &'static str {
        "circus"
    }
    fn snapshot_schema(&self) -> u32 {
        0x5769
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
            //Original4092B0 seeds MSVC rand with the process millisecond clock.
            self.services.sequence_rng.get_or_insert_with(|| {
                crysirror_vfx::falling_atlas::SequenceRng::msvc(host.now_ms() as u32)
            });
            if self.services.storage.is_none() {
                let storage = host.storage(self.client)?;
                //415480/415930 use root GRP/MES/SE/MUSIC for this installed profile.
                //The shared scoped storage owns prefix resolution and existing streams;
                //the adapter supplies only the native directory-name policy.
                storage.insert_asset_redirect("Advdata", "", "", "")?;
                self.services.storage = Some(storage);
            }
            if !self.booted {
                let Some(Prepared::ProfilePreferences(bytes)) = self.services.prepare(
                    Work::ProfilePreferences(crate::Profile::Dc2PlusCommunication),
                    self.client,
                )?
                else {
                    return Ok(VmYield::BudgetExhausted);
                };
                self.machine.state.settings = bytes
                    .map(crate::dc2_settings::SystemData::try_from)
                    .transpose()?
                    .unwrap_or_default();
                self.booted = true;
            }
            let now = host.now_ms();
            let mut bridge = Bridge {
                services: &mut self.services,
                host,
                client: self.client,
                font: &self.config.font,
                input,
            };
            if self.closing {
                self.menu.close(&mut bridge)?;
                bridge.close_dc2_audio()?;
                let expected = Work::ProfileSystemSave(
                    crate::Profile::Dc2PlusCommunication,
                    self.machine.state.settings.bytes().to_vec(),
                );
                if let Some(work) = bridge.services.loading.as_ref().map(|job| job.work.clone()) {
                    if work != expected && bridge.services.prepare(work, self.client)?.is_none() {
                        return Ok(VmYield::BudgetExhausted);
                    }
                }
                if !bridge.write_profile_system(
                    crate::Profile::Dc2PlusCommunication,
                    self.machine.state.settings.bytes(),
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
            for _ in 0..budget {
                //Deferred while a frozen 0x30 screen still owns the bitmap stage.
                if self.machine.state.resources.bitmap_discard_pending
                    && self.machine.state.resources.capture_owner.is_none()
                {
                    bridge.discard_dc2_scene_bitmap()?;
                    self.machine.state.resources.bitmap_discard_pending = false;
                }
                let Some(program) = bridge.scene(&self.machine.state.scene)? else {
                    return Ok(VmYield::BudgetExhausted);
                };
                if let Some(map) = self.machine.state.map.as_ref().filter(|map| map.result.is_none()) {
                    match self.map.tick(&mut bridge, map, &self.machine.state.settings)? {
                        map::Frame::Busy => return Ok(VmYield::BudgetExhausted),
                        map::Frame::Waiting(deadline) => {
                            bridge.finish_native_input_frame();
                            bridge.services.presentation_pending = false;
                            return Ok(VmYield::WaitUntil(deadline));
                        }
                        map::Frame::Done(result) => {
                            self.machine.state.map.as_mut().unwrap().result = Some(result);
                            continue;
                        }
                    }
                }
                if let Some(menu) = self.machine.state.menu.as_ref() {
                    match self
                        .menu
                        .tick(&mut bridge, menu, &self.machine.state.settings)?
                    {
                        menu::Frame::Busy => return Ok(VmYield::BudgetExhausted),
                        menu::Frame::Waiting => {
                            bridge.finish_native_input_frame();
                            bridge.services.presentation_pending = false;
                            return Ok(VmYield::WaitForInput);
                        }
                        menu::Frame::Selected(ordinal) => {
                            self.machine.activate_menu(&program, ordinal)?;
                            if let Some(glyphs) = self.menu.take_text() {
                                self.machine.state.text.displayed = glyphs;
                            }
                            self.menu.close(&mut bridge)?;
                            continue;
                        }
                    }
                }
                let outcome = self.machine.step(
                    &program,
                    &mut bridge,
                    now,
                    input.buttons_down & crysirror_core::service::buttons::CTRL != 0,
                )?;
                match outcome {
                    Outcome::Advanced => {}
                    Outcome::Pending => {
                        if bridge.callbacks_ready() && bridge.native_ui_waiting() {
                            bridge.finish_native_input_frame();
                            bridge.services.presentation_pending = false;
                            return Ok(VmYield::WaitForInput);
                        }
                        return Ok(VmYield::BudgetExhausted);
                    }
                    Outcome::Halted => {
                        self.closing = true;
                        return Ok(VmYield::BudgetExhausted);
                    }
                    Outcome::WaitUntil(deadline) => {
                        bridge.finish_native_input_frame();
                        bridge.services.presentation_pending = false;
                        return Ok(VmYield::WaitUntil(deadline));
                    }
                    Outcome::WaitForInput => {
                        bridge.finish_native_input_frame();
                        bridge.services.presentation_pending = false;
                        return Ok(VmYield::WaitForInput);
                    }
                }
            }
            Ok(VmYield::BudgetExhausted)
        })()
        .map_err(vm_error)
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::Unsupported(
            "DC2 native checkpoint resource reconstruction",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported(
            "DC2 native checkpoint resource reconstruction",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"profile":"dc2pc-5769","scene":self.machine.state.scene,"pc":self.machine.state.pc,"instructions":self.machine.state.instructions,"native_groups":self.machine.state.templates.len(),"engine_images":self.services.images.len(),"worker_pending":self.services.loading.is_some(),"loading":self.services.loading.as_ref().map(|l|format!("{:?}",l.work).chars().take(300).collect::<String>())})).unwrap_or_default()
    }
}
