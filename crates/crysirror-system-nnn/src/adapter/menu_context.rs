//! One active SPT reader and one suspended foreground reader. Both use the
//! same native globals/Bridge, hence one audio owner and one variable bank at
//! each boundary. Switching never acknowledges the suspended PRINT/SELECT.
use super::*;
use crate::{bridge::MenuVisual, expression::Variables};
use anyhow::ensure;
mod wipe_scope;
struct Reader {
    session: ScriptSession,
    pending: Option<NativeCall>,
    looping: Option<bool>,
    halted: bool,
    source: Option<i32>,
    paused_at: u64,
    mode: i32,
    read_marks: Option<bool>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bridge::{CallbackProfile, SceneBindings},
        config::Names,
        expression::VariableProfile,
        native_resources::ResourceNames,
        XorProfile,
    };
    use crysirror_core::{service::EngineQuery, state::ClientId};
    use serde_json::{json, Value};
    use std::time::{Duration, Instant};
    fn program(tail: Vec<u32>) -> Arc<Program> {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend(tail);
        words[18] = words.len() as u32;
        Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|word| (!word).to_le_bytes())
                    .collect::<Vec<_>>(),
                XorProfile::default(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn native_menu_background_swaps_real_readers_and_effects_preserving_foreground_receipt_globals_and_rng(
    ) {
        check_modal_readers(false);
    }
    #[test]
    fn native_menu_background_returns_to_reinitialized_title_instead_of_resuming_cancelled_script()
    {
        check_modal_readers(true);
    }
    fn check_modal_readers(from_title: bool) {
        for mode in [5, 6, 12] {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir(root.path().join("game")).unwrap();
            let project=serde_json::from_value(json!({"schema":2,"name":"modal script","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
            let mut rt = crysirror_runtime::Runtime::with_asset_root(
                project,
                root.path(),
                &root.path().join("state"),
                Default::default(),
            )
            .unwrap();
            rt.set_canvas_image_presenter(true);
            rt.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[0,0,0,255]},"clock_paused":true}}]})).unwrap()).unwrap();
            let mut bridge = Bridge::new(
                ClientId(1),
                CallbackProfile::Mugen2Legacy,
                SceneBindings {
                    root: 1,
                    overlap_canvas: 7,
                    effect_base: 100,
                    size: [4, 4],
                    layers: 16,
                    music_channel: 0,
                },
            )
            .unwrap();
            bridge
                .configure_resources(
                    crate::resource_bridge::NativeResources::blackcyc(
                        ClientId(1),
                        Arc::new(|_, _| Ok(None)),
                    )
                    .unwrap(),
                    ResourceNames::new("").unwrap(),
                    0,
                )
                .unwrap();
            let mut words = vec![0u32; 32];
            words[0] = 32;
            words[1] = 0x66660001;
            words[2] = 0x55550001;
            words.extend([10, 0x66660007, 0x33330013, 6, 0, 54, 0, 255, 0, 0]);
            words.extend([6, 0x66660006, 0x22220001, 0, 7, 0]);
            let pool = words.len() as u32;
            words[4] = 1;
            words[5] = pool;
            words.push(pool + 1);
            words.push(u32::from_le_bytes(*b"abc\0"));
            words[18] = words.len() as u32;
            let foreground = Arc::new(
                Program::parse(
                    &words
                        .into_iter()
                        .flat_map(|word| (!word).to_le_bytes())
                        .collect::<Vec<_>>(),
                    XorProfile::default(),
                )
                .unwrap(),
            );
            let mut variables = Variables::fresh(VariableProfile::Legacy1000);
            variables.mark_calculated(6, b"mode".to_vec());
            variables.mark_calculated(7, b"rnd".to_vec());
            variables.bind_calculated_value(b"mode", 10).unwrap();
            variables
                .replace_random_state(crysirror_image::temporal_pattern::PatternRng::msvc(7))
                .unwrap();
            let original_rng = variables.random_state().unwrap();
            let mut machine = Machine::with_variables(variables);
            let Step::Native(fill) = machine.poll(&foreground, 100).unwrap() else {
                panic!()
            };
            bridge.commit(&mut machine, &fill, &mut rt.host).unwrap();
            let Step::Native(wait) = machine.poll(&foreground, 100).unwrap() else {
                panic!()
            };
            let mut vm = NativeAdapter::from_prepared(foreground, machine, bridge).unwrap();
            vm.pending = Some(wait.clone());
            let names = Names::parse(b"", false).unwrap();
            vm.configuration = Some(Arc::new(NativeConfiguration {
                resources: ResourceNames::new("").unwrap(),
                setup: names.clone(),
                main_control: None,
                chart_setup: None,
                variable_names: Names::parse(b"a b c d e f mode rnd listenbgmmode listenbgmnumber", false).unwrap(),
                project: names.clone(),
                commands: names.clone(),
                functions: names,
                initial: None,
                module_sha256: "authored".into(),
                profile: VariableProfile::Legacy1000,
                scripts: XorProfile::default(),
            }));
            if from_title {
                let mut variables = vm.modal_variables().unwrap();
                variables.bind_calculated_value(b"mode", 4).unwrap();
                let title_program = program(vec![
                    10, 0x66660007, 0x33330013, 6, 0, 54, 0, 255, 0, 0, 3, 0x66660002, 0x55560006,
                ]);
                let mut session = ScriptSession::background(
                    b"title".to_vec(),
                    title_program,
                    Machine::with_variables(variables),
                )
                .unwrap();
                let Step::Native(fill) = session.poll(100).unwrap() else {
                    panic!()
                };
                vm.bridge
                    .commit(session.machine_mut(), &fill, &mut rt.host)
                    .unwrap();
                let Step::Native(next) = session.poll(100).unwrap() else {
                    panic!()
                };
                assert_eq!(next.code, 0x55560006);
                session.acknowledge(&next).unwrap();
                vm.pc = session.machine().pc();
                vm.session = Some(session);
                vm.pending = None;
                vm.background_loop = Some(false);
                vm.halted = true;
                vm.bridge.configure_background_read_marks(false).unwrap();
                assert!(vm.pause_title_for_menu(&mut rt.host).unwrap());
                assert_eq!(vm.pc, 0);
                assert!(vm.pending.is_none());
                assert!(!vm.halted);
            }
            let background = program(vec![
                8, 0x66660005, 0, 0xa0000000, 3, 0x80000064, 0x90000009, 0xa000003d, 8, 0x66660005,
                0, 0xa0000000, 3, 0x80000065, 0x80000007, 0xa000003d, 10, 0x66660007, 0x33330013,
                6, 0, 54, 0, 0, 0, 255, 3, 0x66660002, 0x55560006,
            ]);
            assert!(vm
                .start_menu_background(b"back".to_vec(), background, false, mode, &mut rt.host)
                .unwrap());
            assert_eq!(
                vm.modal_variables().unwrap().read_calculated(6).unwrap(),
                mode
            );
            let until = Instant::now() + Duration::from_secs(5);
            while !vm.halted || vm.dependency_pending() {
                vm.tick_menu_background(&mut rt.host, InputFrame::default())
                    .unwrap();
                rt.prepare_canvas_images(|_, _| {
                    Ok(Some(Arc::new(crysirror_core::service::MemoryAssetReader(
                        vec![0u8; 16 * 4]
                            .chunks_exact(4)
                            .flat_map(|_| [0, 0, 0, 255])
                            .collect::<Vec<_>>()
                            .into(),
                    ))))
                })
                .unwrap();
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
            let color = |rt: &mut crysirror_runtime::Runtime| -> Value {
                let layer: Value = serde_json::from_slice(
                    &rt.host
                        .query_engine(ClientId(1), EngineQuery::Layer(100))
                        .unwrap(),
                )
                .unwrap();
                layer["paint"]["color"].clone()
            };
            assert_eq!(color(&mut rt), json!([0, 0, 255, 255]));
            assert!(vm.switch_menu_background(false, &mut rt.host).unwrap());
            assert_eq!(color(&mut rt), json!([255, 0, 0, 255]));
            check_restored_reader(&vm, &wait, from_title);
            let mut variables = vm.modal_variables().unwrap();
            assert_eq!(variables.read_stored(100), 9);
            let random = variables.random_state().unwrap();
            assert_ne!(random, original_rng);
            variables.write_stored(100, 11);
            if from_title {
                vm.session
                    .as_mut()
                    .unwrap()
                    .machine_mut()
                    .replace_variables(variables)
                    .unwrap();
            } else {
                vm.session
                    .as_mut()
                    .unwrap()
                    .machine_mut()
                    .update_native_variables(&wait, variables)
                    .unwrap();
            }
            vm.set_listen_state(1, 7).unwrap();
            assert!(vm.switch_menu_background(true, &mut rt.host).unwrap());
            assert_eq!(color(&mut rt), json!([0, 0, 255, 255]));
            assert_eq!(vm.modal_variables().unwrap().read_stored(100), 11);
            assert_eq!(vm.modal_variables().unwrap().read_stored(8), 1);
            assert_eq!(vm.modal_variables().unwrap().read_stored(9), 7);
            assert_eq!(
                vm.modal_variables().unwrap().random_state(),
                Some(random.clone())
            );
            assert!(vm.end_menu_background(&mut rt.host).unwrap());
            assert!(!vm.has_menu_background());
            assert_eq!(color(&mut rt), json!([255, 0, 0, 255]));
            check_restored_reader(&vm, &wait, from_title);
            assert_eq!(vm.modal_variables().unwrap().read_stored(9), 7);
            assert_eq!(vm.modal_variables().unwrap().random_state(), Some(random));
        }
    }
    fn check_restored_reader(vm: &NativeAdapter, wait: &NativeCall, title: bool) {
        if title {
            assert_eq!(vm.pc, 0);
            assert_eq!(vm.session.as_ref().unwrap().script_name(), b"title");
            assert_eq!(vm.background_loop, Some(false));
            assert!(vm.pending.is_none());
            assert!(!vm.halted);
            assert_eq!(vm.modal_variables().unwrap().read_calculated(6).unwrap(), 4);
        } else {
            assert_eq!(vm.pc, wait.pc);
            vm.session
                .as_ref()
                .unwrap()
                .machine()
                .validate_receipt(wait)
                .unwrap();
            assert_eq!(
                vm.modal_variables().unwrap().read_calculated(6).unwrap(),
                10
            );
        }
    }
}
pub(super) struct MenuContext {
    reader: Reader,
    visual: MenuVisual,
    pub background: bool,
    mode: i32,
    read_marks: bool,
}
impl NativeAdapter {
    pub(crate) fn set_modal_mode(&mut self,mode:i32)->Result<()> {
        // Background readers bind their own mode when admitted/swapped. A
        // Title reader may still be finishing its suspended frame here.
        if self.worker.is_some()||self.background_loop.is_some(){return Ok(())}
        let mut variables=self.modal_variables()?;
        variables.bind_calculated_value(b"mode",mode)?;
        self.publish_general_variables(variables)
    }
    pub(crate) fn pause_title_for_menu(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        ensure!(
            self.menu_context.is_none()
                && self.background_loop.is_some()
                && self.native_background_mode()? == 4,
            "NNN modal Title suspension boundary"
        );
        if !self.poll_outer_exit(host)? {
            return Ok(false);
        }
        let variables = self
            .outer_exit_variables
            .take()
            .context("NNN suspended Title variables absent")?;
        let session = self
            .session
            .as_ref()
            .context("NNN suspended Title reader absent")?
            .reloaded_background(variables)?;
        self.session = Some(session);
        self.pending = None;
        self.halted = false;
        self.pc = 0;
        self.observed_depth = 0;
        self.instruction_base = self.executed;
        self.observed_script = self.session.as_ref().unwrap().script_name().to_vec();
        Ok(true)
    }
    pub(crate) fn gallery_variables(&self) -> Result<Variables> {
        self.modal_variables()
    }
    pub(crate) fn set_listen_state(&mut self, mode: i32, number: i32) -> Result<()> {
        ensure!(self.worker.is_none(), "NNN ListenBGM variables require the owned reader");
        let configuration = self.configuration.as_ref().context("NNN ListenBGM configuration absent")?;
        let mut variables = self.modal_variables()?;
        for (name, value) in [(b"listenbgmmode".as_slice(), mode), (b"listenbgmnumber".as_slice(), number)] {
            if let Some(id) = configuration.variable_names.index(name) {
                variables.write_stored(id, value);
            }
        }
        let machine = self.session.as_mut().unwrap().machine_mut();
        if let Some(call) = self.pending.as_ref() {
            machine.update_native_variables(call, variables)?;
        } else {
            machine.replace_variables(variables)?;
        }
        Ok(())
    }
    pub(super) fn native_background_mode(&self) -> Result<i32> {
        let index = self
            .configuration
            .as_ref()
            .context("NNN native mode configuration absent")?
            .variable_names
            .index(b"mode")
            .context("NNN calculated mode name absent")?;
        self.modal_variables()?.read_calculated(index)
    }
    pub(crate) fn retained_general_enter_frame(&self)->Option<Arc<dyn crysirror_core::image_processing::ImageResource>>{self.bridge.retained_general_enter_frame()}
    pub(crate) fn retain_general_overlap_buffers(&mut self,old:Arc<dyn crysirror_core::image_processing::ImageResource>,new:Arc<dyn crysirror_core::image_processing::ImageResource>)->Result<()> {
        self.bridge.retain_general_overlap_buffers(old,new)
    }
    pub(crate) fn bind_entry_wipe_context(&self,fade:&mut crate::native_general_entry::EntryFadePlayer)->Result<()> {
        if !fade.needs_random_context(){return Ok(());}
        let variables=self.modal_variables()?;
        let random=variables.random_state().context("NNN General entry random state absent")?;
        fade.bind_random(random,self.bridge.simple_wipe_tables());Ok(())
    }
    pub(crate) fn commit_entry_wipe_context(&mut self,fade:&mut crate::native_general_entry::EntryFadePlayer)->Result<()> {
        if let Some((random,tables))=fade.take_random_state(){
            let mut variables=self.modal_variables()?;variables.replace_random_state(random)?;
            self.publish_general_variables(variables)?;self.bridge.set_simple_wipe_tables(tables);
        }Ok(())
    }
    pub(crate) fn bind_wipe_context(&self,fade:&mut crate::native_general::ExitFadePlayer)->Result<()> {
        if !fade.needs_random_context() {return Ok(());}
        let variables=self.modal_variables()?;
        let random=variables.random_state().context("NNN General native random state absent")?;
        fade.bind_random(random,self.bridge.simple_wipe_tables());
        Ok(())
    }
    pub(crate) fn commit_wipe_context(&mut self,fade:&mut crate::native_general::ExitFadePlayer)->Result<()> {
        if let Some(image)=fade.retained_enter_frame(){self.bridge.retain_general_enter_frame(image)?;}
        if let Some((random,tables))=fade.take_random_state() {
            let mut variables=self.modal_variables()?;
            variables.replace_random_state(random.clone())?;
            self.publish_general_variables(variables)?;
            self.bridge.set_simple_wipe_tables(tables.clone());
        }
        Ok(())
    }
    fn modal_variables(&self) -> Result<Variables> {
        let session = self
            .session
            .as_ref()
            .context("NNN modal reader not owned by session")?;
        match &self.pending {
            Some(call) => session.machine().native_variables_candidate(call),
            None => session
                .machine()
                .variables()
                .cloned()
                .context("NNN modal variable bank absent"),
        }
    }
    pub(crate) fn menu_background_active(&self) -> bool {
        self.menu_context.as_ref().is_some_and(|ctx| ctx.background)
    }
    pub(crate) fn has_menu_background(&self) -> bool {
        self.menu_context.is_some()
    }
    pub(crate) fn start_menu_background(
        &mut self,
        name: Vec<u8>,
        program: Arc<Program>,
        looping: bool,
        mode: i32,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        ensure!(
            matches!(mode, 5 | 6 | 7 | 8 | 12 | 13 | 15..=18 | 20 | 26 | 34),
            "NNN modal background native mode"
        );
        ensure!(
            self.menu_context.is_none(),
            "NNN modal background already active"
        );
        let config = self
            .configuration
            .as_ref()
            .context("NNN modal configuration absent")?
            .clone();
        config.resources.script(&name)?;
        let read_marks = config.setup.integer(b"backScriptSetCGOkFlag")?.unwrap_or(0) != 0;
        let foreground_mode = self.original_input_mode();
        let (mut variables, restart_title) = if foreground_mode.is_some() {
            (self.modal_variables()?, false)
        } else {
            // commonTitle::Calcu saves the effect bank before Load/Config;
            // GeneralInit reloads its background script when Title returns.
            // Cancel only at the admitted frame boundary, without ACKing the
            // old DRAW countdown or resuming its stale return stack later.
            ensure!(
                self.background_loop.is_some() && self.native_background_mode()? == 4,
                "NNN menu background requires PRINT/SELECT or a Title actor"
            );
            if !self.poll_outer_exit(host)? {
                return Ok(false);
            }
            (
                self.outer_exit_variables
                    .as_ref()
                    .context("NNN Title cancellation variables absent")?
                    .clone(),
                true,
            )
        };
        let restarted = if restart_title {
            let session = self.session.as_ref().context("NNN Title reader absent")?;
            Some(session.reloaded_background(variables.clone())?)
        } else {
            None
        };
        ensure!(
            variables.bind_calculated_value(b"mode", mode)?,
            "NNN menu native mode binding absent"
        );
        let background =
            ScriptSession::background(name, program, Machine::with_variables(variables))?;
        let visual = self.bridge.freeze_menu_visual(host)?;
        let foreground = Reader {
            session: restarted.unwrap_or_else(|| self.session.take().unwrap()),
            pending: if restart_title {
                None
            } else {
                self.pending.take()
            },
            looping: self.background_loop,
            halted: !restart_title && self.halted,
            source: self.bridge.menu_source_reader(),
            paused_at: host.now_ms(),
            mode: foreground_mode.unwrap_or(4),
            read_marks: restart_title.then_some(read_marks),
        };
        self.pending = None;
        self.outer_exit_variables = None;
        self.instruction_base = self.executed;
        self.session = Some(background);
        self.background_loop = Some(looping);
        self.halted = false;
        self.pc = 0;
        self.observed_depth = 0;
        self.observed_script = self.session.as_ref().unwrap().script_name().to_vec();
        self.bridge.enter_foreground_context();
        self.bridge.configure_background_read_marks(read_marks)?;
        self.menu_context = Some(Box::new(MenuContext {
            reader: foreground,
            visual,
            background: true,
            mode,
            read_marks,
        }));
        Ok(true)
    }
    pub(crate) fn switch_menu_background(
        &mut self,
        background: bool,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        let Some(ctx) = &self.menu_context else {
            return Ok(true);
        };
        if ctx.background == background {
            return Ok(true);
        }
        if self.worker.is_some() || self.bridge.dependency_pending() {
            ensure!(
                ctx.background,
                "NNN suspended foreground acquired new VM work during a menu action"
            );
            self.advance(host, InputFrame::default(), 1024, false)?;
            if self.worker.is_some() || self.bridge.dependency_pending() {
                return Ok(false);
            }
        }
        let mut variables = self.modal_variables()?;
        // Temporary instruction write bindings belong to their old receipt;
        // persistent writes and the common RNG bank cross the reader boundary.
        variables.finish_native_instruction();
        let mut ctx = self.menu_context.take().unwrap();
        let old_halted = self.halted;
        let old_looping = self.background_loop;
        let old_source = self.bridge.menu_source_reader();
        let old_mode = if ctx.background {
            ctx.mode
        } else {
            self.original_input_mode().unwrap_or(4)
        };
        let target_mode = ctx.reader.mode;
        let target_read_marks = ctx.reader.read_marks;
        if let Err(error) = ctx.visual.swap(&mut self.bridge, host) {
            self.menu_context = Some(ctx);
            return Err(error);
        }
        self.bridge
            .reanchor_menu_waits(host.now_ms(), ctx.reader.paused_at)?;
        let active = self.session.take().unwrap();
        let pending = self.pending.take();
        self.session = Some(ctx.reader.session);
        self.pending = ctx.reader.pending;
        self.halted = ctx.reader.halted;
        self.background_loop = ctx.reader.looping;
        self.bridge.set_menu_source_reader(ctx.reader.source);
        ctx.reader = Reader {
            session: active,
            pending,
            looping: old_looping,
            halted: old_halted,
            source: old_source,
            paused_at: host.now_ms(),
            mode: old_mode,
            read_marks: if ctx.background || old_looping.is_some() {
                Some(ctx.read_marks)
            } else {
                None
            },
        };
        ctx.background = background;
        self.bridge.enter_foreground_context();
        if let Some(read_marks) = target_read_marks {
            self.bridge.configure_background_read_marks(read_marks)?;
        }
        let session = self.session.as_mut().unwrap();
        let _ = variables.bind_calculated_value(b"mode", target_mode)?;
        if let Some(call) = &self.pending {
            session
                .machine_mut()
                .update_native_variables(call, variables)?;
        } else {
            session.machine_mut().replace_variables(variables)?;
        }
        self.instruction_base = self.executed.saturating_sub(session.executed());
        self.pc = session.machine().pc();
        self.observed_script = session.script_name().to_vec();
        self.observed_depth = session.depth();
        self.menu_context = Some(ctx);
        Ok(true)
    }
    pub(crate) fn end_menu_background(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self.switch_menu_background(false, host)? {
            return Ok(false);
        }
        if let Some(ctx) = self.menu_context.take() {
            ctx.visual.retire(self.bridge.client(), host)?;
        }
        Ok(true)
    }
    pub(crate) fn inactive_menu_images(&self) -> std::collections::BTreeSet<String> {
        self.menu_context
            .as_ref()
            .map(|ctx| ctx.visual.resource_paths())
            .unwrap_or_default()
    }
    pub(crate) fn tick_menu_background(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<()> {
        ensure!(
            self.menu_background_active(),
            "NNN inactive modal background reader"
        );
        self.advance(host, input, 1024, true)?;
        Ok(())
    }
}
