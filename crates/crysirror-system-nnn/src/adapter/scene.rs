//! SceneList's film sequence runs on the existing Bridge/audio owner. Title's
//! reader and effect bank remain suspended; no renderer/runtime is copied.
use super::*;
use crate::bridge::MenuVisual;
use anyhow::ensure;
pub(super) struct SceneContext {
    session: ScriptSession,
    looping: Option<bool>,
    source: Option<i32>,
    visual: MenuVisual,
    films: Vec<i32>,
    current: usize,
    returning: bool,
}
impl NativeAdapter {
    pub(crate) fn scene_active(&self) -> bool {
        self.scene_context.is_some()
    }
    pub(crate) fn start_scene_replay(
        &mut self,
        name: Vec<u8>,
        program: Arc<Program>,
        films: Vec<i32>,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        ensure!(
            self.scene_context.is_none() && !self.has_menu_background(),
            "NNN scene replay overlaps a modal reader"
        );
        ensure!(
            self.background_loop.is_some() && self.native_background_mode()? == 4,
            "NNN scene replay requires a suspended Title actor"
        );
        ensure!(
            !films.is_empty() && films.len() <= 65536,
            "NNN scene film list bounds"
        );
        // Validate every seek before cancelling the old reader or publishing
        // resources. Script offsets are data, never host code addresses.
        for film in &films {
            program.film_pc(*film)?;
        }
        if !self.poll_outer_exit(host)? {
            return Ok(false);
        }
        let mut variables = self
            .outer_exit_variables
            .as_ref()
            .context("NNN scene entry variables absent")?
            .clone();
        variables.finish_native_instruction();
        let title = self
            .session
            .as_ref()
            .unwrap()
            .reloaded_background(variables.clone())?;
        variables.bind_calculated_value(b"mode", 10)?;
        let session =
            ScriptSession::scene(name, program, Machine::with_variables(variables), films[0])?;
        let visual = self.bridge.freeze_menu_visual(host)?;
        self.scene_context = Some(Box::new(SceneContext {
            session: title,
            looping: self.background_loop,
            source: self.bridge.menu_source_reader(),
            visual,
            films,
            current: 0,
            returning: false,
        }));
        self.session = Some(session);
        self.pending = None;
        self.worker = None;
        self.halted = false;
        self.background_loop = None;
        self.outer_exit_variables = None;
        self.instruction_base = self.executed;
        self.pc = self.session.as_ref().unwrap().machine().pc();
        self.observed_script = self.session.as_ref().unwrap().script_name().to_vec();
        self.observed_depth = 0;
        self.bridge.begin_scene_replay(host)?;
        Ok(true)
    }
    /// Native ENDFILM acknowledges exactly its own receipt, then selects the
    /// next film in the same loaded script. A final film returns to SelectScene.
    pub(super) fn try_scene_boundary(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(scene) = self.scene_context.as_mut() else {
            return Ok(false);
        };
        let Some(call) = self.pending.as_ref() else {
            return Ok(false);
        };
        if call.identify != 0x66660002 || !matches!(call.code, 0x5556000c | 0x55560006) {
            return Ok(false);
        }
        let session = self.session.as_mut().context("NNN scene reader absent")?;
        let next = if call.code == 0x5556000c {
            scene.films.get(scene.current + 1).copied()
        } else {
            None
        };
        if let Some(film) = next {
            session.program().film_pc(film)?;
        }
        self.bridge.hide_scene_skip(host)?;
        session.acknowledge(call)?;
        self.executed = self.instruction_base.saturating_add(session.executed());
        if let Some(film) = next {
            let program = session.program().clone();
            session.machine_mut().seek_film(&program, film)?;
            scene.current += 1;
        } else {
            scene.returning = true;
        }
        self.pending = None;
        self.pc = session.machine().pc();
        Ok(true)
    }
    pub(crate) fn poll_scene_replay(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        cancel: bool,
    ) -> Result<bool> {
        let Some(scene) = self.scene_context.as_mut() else {
            return Ok(true);
        };
        scene.returning |= cancel;
        if !scene.returning {
            self.advance(host, input, 1024, admit)?;
            if !self.scene_context.as_ref().unwrap().returning {
                return Ok(false);
            }
        }
        // Abandoning a scene input wait is cancellation, not a chosen CASE or
        // successful PRINT. The existing shutdown boundary drains only admitted
        // work; the original pending instruction is never forged as complete.
        if self.worker.is_some() || self.bridge.dependency_pending() {
            self.advance(host, InputFrame::default(), 1024, false)?;
            if self.worker.is_some() || self.bridge.dependency_pending() {
                return Ok(false);
            }
        }
        if !self.poll_native_exit(host, true)? {
            return Ok(false);
        }
        let mut variables = self
            .outer_exit_variables
            .take()
            .context("NNN scene return variables absent")?;
        variables.finish_native_instruction();
        variables.bind_calculated_value(b"mode", 4)?;
        let mut scene = self.scene_context.take().unwrap();
        scene.session.machine_mut().replace_variables(variables)?;
        if let Err(error) = scene.visual.swap(&mut self.bridge, host) {
            self.scene_context = Some(scene);
            return Err(error);
        }
        scene.visual.retire(self.bridge.client(), host)?;
        self.session = Some(scene.session);
        self.pending = None;
        self.worker = None;
        self.halted = false;
        self.background_loop = scene.looping;
        self.bridge.set_menu_source_reader(scene.source);
        let allow = self
            .configuration
            .as_ref()
            .context("NNN scene return configuration absent")?
            .setup
            .integer(b"backScriptSetCGOkFlag")?
            .unwrap_or(0)
            != 0;
        self.bridge.finish_scene_replay(allow)?;
        self.instruction_base = self.executed;
        self.pc = 0;
        self.observed_depth = 0;
        self.observed_script = self.session.as_ref().unwrap().script_name().to_vec();
        self.bridge.resume_after_menu(host)?;
        Ok(true)
    }
}
#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::{
        bridge::{CallbackProfile, SceneBindings},
        config::Names,
        expression::{VariableProfile, Variables},
        native_resources::ResourceNames,
    };
    use crysirror_core::{service::EngineQuery, state::ClientId};
    use serde_json::json;
    use std::time::{Duration, Instant};
    fn program(tail: Vec<u32>, films: &[u32]) -> Arc<Program> {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend(tail);
        words[8] = films.len() as u32;
        words[9] = words.len() as u32;
        words.extend(films);
        words[18] = words.len() as u32;
        Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|w| (!w).to_le_bytes())
                    .collect::<Vec<_>>(),
                Default::default(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn native_scene_replay_runs_real_film_targets_and_returns_the_single_bridge_to_title() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"film reader","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[0,0,0,255]}}}]})).unwrap()).unwrap();
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
        bridge.configure_background_read_marks(false).unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.mark_calculated(0, b"mode".to_vec());
        variables.bind_calculated_value(b"mode", 4).unwrap();
        variables
            .replace_random_state(crysirror_image::temporal_pattern::PatternRng::msvc(7))
            .unwrap();
        let title = program(
            vec![
                10, 0x66660007, 0x33330013, 6, 0, 54, 0, 255, 0, 0, 3, 0x66660002, 0x55560006,
            ],
            &[],
        );
        let mut reader = ScriptSession::background(
            b"title".to_vec(),
            title.clone(),
            Machine::with_variables(variables.clone()),
        )
        .unwrap();
        let Step::Native(fill) = reader.poll(100).unwrap() else {
            panic!()
        };
        bridge
            .commit(reader.machine_mut(), &fill, &mut rt.host)
            .unwrap();
        let Step::Native(next) = reader.poll(100).unwrap() else {
            panic!()
        };
        reader.acknowledge(&next).unwrap();
        let mut vm =
            NativeAdapter::from_prepared(title, Machine::with_variables(variables), bridge)
                .unwrap();
        vm.session = Some(reader);
        vm.background_loop = Some(false);
        vm.halted = true;
        let empty = Names::parse(b"", false).unwrap();
        vm.configuration = Some(Arc::new(NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: empty.clone(),
            main_control: None,
            chart_setup: None,
            variable_names: Names::parse(b"mode", false).unwrap(),
            project: Names::parse(b"sys title scene", false).unwrap(),
            commands: empty.clone(),
            functions: empty,
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: Default::default(),
        }));
        let scene = program(
            vec![
                10, 0x66660007, 0x33330013, 6, 0, 54, 0, 0, 255, 0, 3, 0x66660002, 0x5556000c, 10,
                0x66660007, 0x33330013, 6, 0, 54, 0, 0, 0, 255, 3, 0x66660002, 0x5556000c,
            ],
            &[32, 45],
        );
        assert!(vm
            .start_scene_replay(b"scene".to_vec(), scene.clone(), vec![0, 1], &mut rt.host)
            .unwrap());
        let mut colors = std::collections::BTreeSet::new();
        let until = Instant::now() + Duration::from_secs(5);
        while !vm
            .poll_scene_replay(&mut rt.host, InputFrame::default(), true, false)
            .unwrap()
        {
            let layer: serde_json::Value = serde_json::from_slice(
                &rt.host
                    .query_engine(ClientId(1), EngineQuery::Layer(100))
                    .unwrap(),
            )
            .unwrap();
            if let Some(c) = layer["paint"]["color"].as_array() {
                colors.insert(c.iter().map(|v| v.as_u64().unwrap()).collect::<Vec<_>>());
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(colors.contains(&vec![0, 255, 0, 255]));
        assert!(colors.contains(&vec![0, 0, 255, 255]));
        assert!(!vm.scene_active());
        assert!(!vm.bridge.scene_replay_active());
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.background_loop, Some(false));
        let layer: serde_json::Value = serde_json::from_slice(
            &rt.host
                .query_engine(ClientId(1), EngineQuery::Layer(100))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(layer["paint"]["color"], json!([255, 0, 0, 255]));
        assert_eq!(
            vm.gallery_variables().unwrap().read_calculated(0).unwrap(),
            4
        );
        assert!(vm
            .start_scene_replay(b"scene".to_vec(), scene, vec![2], &mut rt.host)
            .is_err());
        assert!(!vm.scene_active());
        assert_eq!(vm.pc, 0);
    }
}
