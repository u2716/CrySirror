// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Authored original records, shared Scene/UI/reveal and real native CASEs.
use crate::{
    Program, XorProfile,
    config::Names,
    expression::{VariableProfile, Variables},
    machine::Machine,
    native_text::*,
    native_ui::*,
};
use crysirror_core::{
    image_processing::ImageResource,
    path::VfsPath,
    service::{
        EngineQuery, FileSystem, HostServices, InputFrame, TextExtent, TextMeasurement, buttons,
    },
    state::{ClientId, Command},
};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
struct Metrics(Arc<Mutex<Vec<std::thread::ThreadId>>>);
impl TextMeasurement for Metrics {
    fn measure_simple(&self, _: &str, _: f32, _: &str) -> crysirror_core::Result<TextExtent> {
        self.0.lock().unwrap().push(std::thread::current().id());
        Ok(TextExtent {
            advance: 0.,
            ascent: 4.,
            descent: 0.,
            line_height: 4.,
        })
    }
}
struct Host {
    engine: crysirror_scene::EngineSystems,
    now: u64,
    calls: usize,
    workers: Arc<Mutex<Vec<std::thread::ThreadId>>>,
}
impl Host {
    fn new() -> Self {
        let mut host = Self {
            engine: Default::default(),
            now: 0,
            calls: 0,
            workers: Default::default(),
        };
        host.submit_engine(ClientId(1),1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,64,64],"paint":{"kind":"solid","color":[0,0,0,255]}}}]})).unwrap()).unwrap();
        host.calls = 0;
        host
    }
    fn tick(&mut self, input: InputFrame) {
        self.now += 16;
        self.engine.tick(16, input, [64, 64]).unwrap();
    }
}
impl FileSystem for Host {
    fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::Unsupported("fixture I/O"))
    }
    fn write_atomic(&mut self, _: ClientId, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported("fixture write"))
    }
}
impl HostServices for Host {
    fn now_ms(&self) -> u64 {
        self.now
    }
    fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported(
            "fixture classic commands",
        ))
    }
    fn text_measurement(
        &mut self,
        client: ClientId,
    ) -> crysirror_core::Result<Arc<dyn TextMeasurement>> {
        assert_eq!(client, ClientId(1));
        Ok(Arc::new(Metrics(self.workers.clone())))
    }
    fn query_engine(
        &mut self,
        client: ClientId,
        q: EngineQuery,
    ) -> crysirror_core::Result<Vec<u8>> {
        let value = match q {
            EngineQuery::Layer(id) => {
                serde_json::to_value(self.engine.layers.get(client, id)).unwrap()
            }
            EngineQuery::UiEvents => {
                serde_json::to_value(self.engine.ui.drain_events(client.0, None)).unwrap()
            }
            EngineQuery::UiInteraction(id) => {
                serde_json::to_value(self.engine.ui.interaction(client.0, id)).unwrap()
            }
            _ => return Err(crysirror_core::Error::Unsupported("fixture query")),
        };
        Ok(serde_json::to_vec(&value).unwrap())
    }
    fn submit_engine(
        &mut self,
        client: ClientId,
        schema: u32,
        data: &[u8],
    ) -> crysirror_core::Result<()> {
        assert_eq!(schema, 1);
        let batch = serde_json::from_slice(data).unwrap();
        self.engine
            .submit(client, &batch)
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))?;
        self.calls += 1;
        Ok(())
    }
}
#[derive(Debug)]
struct Background;
impl ImageResource for Background {
    fn client(&self) -> ClientId {
        ClientId(1)
    }
    fn size(&self) -> [u32; 2] {
        [64, 64]
    }
    fn rgba(&self) -> Arc<[u8]> {
        panic!("layout may not fetch pixels")
    }
    fn opaque(&self) -> bool {
        true
    }
}
fn profile() -> MessageProfile {
    MessageProfile::from_legacy_setup(&Names::parse(b"fontSize 4 nameFontSize 4 printX 0 printY 0 nextY 4 LPrintX 0 LPrintY 0 LNextY 4 fullMessageNumber 3 windowMessageNumber 3 nameFlag 0 nameColorChangeFlag 0",false).unwrap(),
        MessageBindings{client:ClientId(1),root:1,body_layer:200,name_layer:201,order:30,canvas:[64,64],window:None},
        "fixture",2,None,None,buttons::MOUSE_LEFT).unwrap()
}
fn raw(s: &str) -> Vec<u8> {
    let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(s);
    assert!(!bad);
    bytes.into_owned()
}
fn spin(mut poll: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !poll() {
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
}
#[test]
fn saved_fixed_text_prepares_without_scene_mutation_then_arms_and_reveals_without_print_ack() {
    let mut host = Host::new();
    let live = NativeTextBridge::new(profile()).unwrap();
    let rows = vec![raw("日本"), raw("語")];
    let before = serde_json::to_value(&host.engine).unwrap();
    let prep = NativeTextBridge::new(profile())
        .unwrap()
        .restore_seed()
        .unwrap()
        .begin(
            rows.clone(),
            0x22220002,
            77,
            false,
            false,
            Arc::new(Background),
            &mut host,
        )
        .unwrap();
    assert!(
        prep.finish().is_err(),
        "a pending worker cannot be committed"
    );
    let mut prep = live
        .restore_seed()
        .unwrap()
        .begin(
            rows.clone(),
            0x22220002,
            77,
            false,
            false,
            Arc::new(Background),
            &mut host,
        )
        .unwrap();
    spin(|| prep.poll(&mut host).unwrap());
    assert_eq!(host.calls, 0);
    assert_eq!(serde_json::to_value(&host.engine).unwrap(), before);
    assert!(
        host.workers
            .lock()
            .unwrap()
            .iter()
            .all(|id| *id != std::thread::current().id())
    );
    let prepared = prep.finish().unwrap();
    assert!(prepared.release_images.is_empty());
    assert!(live.state.rows.is_empty());
    let mut restored = prepared.bridge;
    assert_eq!(restored.state.rows, rows);
    assert_eq!(restored.state.serial, 77);
    host.submit_engine(
        ClientId(1),
        1,
        &serde_json::to_vec(&prepared.batch).unwrap(),
    )
    .unwrap();
    let click = InputFrame {
        buttons_down: buttons::MOUSE_LEFT,
        buttons_pressed: buttons::MOUSE_LEFT,
        ..Default::default()
    };
    assert!(!restored.poll_restored_input(&mut host, click).unwrap());
    assert_eq!(
        host.calls, 1,
        "load's triggering click must not complete reveal"
    );
    host.tick(Default::default());
    assert!(!restored.poll_restored_input(&mut host, click).unwrap());
    assert_eq!(host.calls, 2);
    assert!(!restored.poll_restored_input(&mut host, click).unwrap());
    host.tick(Default::default());
    assert!(restored.poll_restored_input(&mut host, click).unwrap());
    assert_eq!(
        host.calls, 2,
        "advancing saved wait does not submit PRINT again"
    );
}
fn selection_program() -> Program {
    let mut w = vec![0; 32];
    w[0] = 32;
    w[1] = 0x66660001;
    w[2] = 0x55550001;
    w.extend([
        4, 0x66660002, 0xa000007b, 18, 4, 0x66660002, 0x55560008, 1, 3, 0x66660002, 0x55560005, 4,
        0x66660002, 0x55560008, 2, 3, 0x66660002, 0x55560005, 3, 0x66660002, 0xa000007d,
    ]);
    Program::parse(
        &w.into_iter()
            .flat_map(|v: u32| (!v).to_le_bytes())
            .collect::<Vec<_>>(),
        XorProfile::default(),
    )
    .unwrap()
}
fn selection_profile() -> SelectionProfile {
    SelectionProfile {
        client: ClientId(1),
        root: 1,
        widget_base: 300,
        layer_base: 400,
        order: 30,
        canvas: [64, 64],
        face: profile().body,
        rows: vec![
            SelectionRow {
                text_origin: [0, 20],
                hit_rect: [0, 20, 30, 10],
            },
            SelectionRow {
                text_origin: [0, 40],
                hit_rect: [0, 40, 30, 10],
            },
        ],
        message_origins: vec![[0, 0]],
        single_byte_map: None,
    }
}
#[test]
fn saved_selection_uses_total_minus_supplemental_and_resumes_actual_case_without_receipt() {
    let mut host = Host::new();
    let live = NativeSelectionBridge::new(selection_profile()).unwrap();
    let mut extra = vec![b'#'];
    extra.extend(raw("説明"));
    let seed = live.restore_seed().unwrap();
    assert!(seed.begin(vec![], 3, 3, &mut host).is_err());
    let mut prep = seed
        .begin(vec![extra, raw("左"), raw("右")], 3, 1, &mut host)
        .unwrap();
    spin(|| prep.poll().unwrap());
    assert_eq!(host.calls, 0);
    let prepared = prep.finish().unwrap();
    let mut restored = prepared.bridge;
    let batch = prepared.batch;
    assert_eq!(batch["ui"].as_array().unwrap().len(), 2);
    assert_eq!(
        batch["layers"][0]["layer"]["paint"]["glyphs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let p = selection_program();
    let mut vars = Variables::fresh(VariableProfile::Legacy1000);
    vars.write_stored(250, 123);
    let mut machine = Machine::from_native_saved(&p, 32, vec![], [None, None], vars).unwrap();
    host.submit_engine(ClientId(1), 1, &serde_json::to_vec(&batch).unwrap())
        .unwrap();
    assert!(
        !restored
            .poll_restored_input(&mut machine, &p, &mut host)
            .unwrap()
    );
    let click = InputFrame {
        buttons_down: buttons::MOUSE_LEFT,
        buttons_pressed: buttons::MOUSE_LEFT,
        pointer_x: 1,
        pointer_y: 41,
    };
    host.tick(click);
    assert!(
        !restored
            .poll_restored_input(&mut machine, &p, &mut host)
            .unwrap()
    );
    host.tick(InputFrame {
        pointer_x: 1,
        pointer_y: 41,
        ..Default::default()
    });
    assert!(
        restored
            .poll_restored_input(&mut machine, &p, &mut host)
            .unwrap()
    );
    assert_eq!(machine.pc(), 47);
    assert_eq!(machine.executed(), 0);
    assert_eq!(machine.variables().unwrap().read_stored(250), 123);
    assert!(host.engine.ui.get(1, 300).is_none());
    assert!(host.engine.layers.get(ClientId(1), 400).is_none());
}
#[test]
fn bad_saved_case_keeps_pc_execution_variables_and_native_state() {
    let mut p = selection_program();
    p.words[36] = 3; // A CASE record without its required operand.
    let mut vars = Variables::fresh(VariableProfile::Legacy1000);
    vars.write_stored(250, 321);
    let mut machine = Machine::from_native_saved(&p, 32, vec![], [None, None], vars).unwrap();
    let pc = machine.pc();
    let count = machine.executed();
    assert!(machine.resume_saved_case(&p, 2).is_err());
    assert_eq!(machine.pc(), pc);
    assert_eq!(machine.executed(), count);
    assert_eq!(machine.variables().unwrap().read_stored(250), 321);
}
pub(crate) fn save_file() -> (
    crate::native_save::NativeSaveFile,
    crate::native_save::LegacySaveProfile,
) {
    use crate::native_save::{LegacySaveProfile, NativeSaveFile, SaveBlock};
    let profile = LegacySaveProfile {
        header_name: b"wait-fixture".to_vec(),
        version: 1,
        extension_sizes: vec![],
        omake: false,
        cutin: false,
    };
    let mut info = vec![0; 32];
    info[..4].copy_from_slice(&8u32.to_le_bytes());
    info[4..8].copy_from_slice(&1u32.to_le_bytes());
    let mut mini = vec![0; 20];
    mini[..4].copy_from_slice(&1u32.to_le_bytes());
    mini[4..8].copy_from_slice(&1u32.to_le_bytes());
    let file = NativeSaveFile {
        blocks: vec![
            SaveBlock::new(0, b"wait-fixture", vec![]).unwrap(),
            SaveBlock::new(1, b"INFO", info).unwrap(),
            SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056]).unwrap(),
            SaveBlock::new(3, b"MINICG", mini).unwrap(),
            SaveBlock::new(4, b"GAMEVAR", vec![0; 4096]).unwrap(),
            SaveBlock::new(5, b"EFFECT", vec![0; 16384]).unwrap(),
            SaveBlock::new(6, b"EFFECTFILENAME", vec![0; 4096]).unwrap(),
            SaveBlock::new(7, b"MESSAGE", vec![0; 16384]).unwrap(),
        ],
    };
    (file, profile)
}
pub(crate) fn word(file: &mut crate::native_save::NativeSaveFile, index: usize, value: i32) {
    let at = (index - 8) * 4;
    file.blocks[2].payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
#[test]
fn original_count_is_total_and_window_number_is_not_visibility() {
    use crate::native_save_wait::SavedWait;
    let (mut file, profile) = save_file();
    word(&mut file, 33, 64);
    word(&mut file, 34, 4);
    file.validate(&profile).unwrap();
    word(&mut file, 34, 65);
    assert!(file.validate(&profile).is_err());
    word(&mut file, 34, 0);
    word(&mut file, 33, 65);
    assert!(file.validate(&profile).is_err());
    word(&mut file, 33, 0);
    word(&mut file, 20, 10);
    word(&mut file, 30, 0x22220001);
    word(&mut file, 180, 7);
    word(&mut file, 28, 123);
    word(&mut file, 476, -1);
    word(&mut file, 485, 1);
    file.validate(&profile).unwrap();
    let original = file.encode(&profile).unwrap();
    let decoded = crate::native_save::NativeSaveFile::parse(&original, &profile).unwrap();
    let Some(SavedWait::Message {
        window_number,
        serial,
        cannot_click,
        ..
    }) = decoded.saved_wait().unwrap()
    else {
        panic!()
    };
    assert_eq!(window_number, 7);
    assert_eq!(serial, u32::MAX);
    assert!(cannot_click);
    assert_eq!(decoded.encode(&profile).unwrap(), original);
}
#[test]
fn original_selection_records_feed_real_vm_port_wait_without_executing_select_again() {
    use crate::bridge::{Bridge, CallbackProfile, SceneBindings};
    use crysirror_core::vm::{VmAdapter, VmYield};
    let mut host = Host::new();
    let original = NativeSelectionBridge::new(selection_profile()).unwrap();
    let (mut file, profile) = save_file();
    word(&mut file, 20, 9);
    word(&mut file, 33, 3);
    word(&mut file, 34, 1);
    let mut extra = vec![b'#'];
    extra.extend(raw("説明"));
    for (index, row) in [extra, raw("左"), raw("右")].iter().enumerate() {
        file.blocks[7].payload[index * 256..index * 256 + row.len()].copy_from_slice(row);
    }
    // Decode an actual original envelope; no NativeCall or pending callback is
    // constructed anywhere in this restore path.
    let bytes = file.encode(&profile).unwrap();
    let file = crate::native_save::NativeSaveFile::parse(&bytes, &profile).unwrap();
    let seed = original.restore_seed().unwrap();
    let mut prep = file
        .saved_wait()
        .unwrap()
        .unwrap()
        .begin(None, Some(&seed), Arc::new(Background), &mut host)
        .unwrap();
    spin(|| prep.poll(&mut host).unwrap());
    assert_eq!(host.calls, 0);
    let done = prep.finish().unwrap();
    let mut bridge = Bridge::new(
        ClientId(1),
        CallbackProfile::Mugen2Legacy,
        SceneBindings {
            root: 1,
            overlap_canvas: 7,
            effect_base: 100,
            size: [64, 64],
            layers: 16,
            music_channel: 0,
        },
    )
    .unwrap();
    let (batch, releases) = bridge.install_saved_wait(done).unwrap();
    assert!(releases.is_empty());
    host.submit_engine(ClientId(1), 1, &serde_json::to_vec(&batch).unwrap())
        .unwrap();
    let p = Arc::new(selection_program());
    let mut vars = Variables::fresh(VariableProfile::Legacy1000);
    vars.write_stored(250, 123);
    let machine = Machine::from_native_saved(&p, 32, vec![], [None, None], vars).unwrap();
    let mut vm = crate::adapter::NativeAdapter::from_prepared(p, machine, bridge).unwrap();
    for _ in 0..3 {
        assert_eq!(
            vm.step(&mut host, Default::default(), 20).unwrap(),
            VmYield::WaitForInput
        );
    }
    let diag: serde_json::Value = serde_json::from_slice(&vm.diagnostics()).unwrap();
    assert_eq!(diag["instructions"], 0);
    assert_eq!(diag["wait_kind"], "saved_selection_input");
    host.tick(InputFrame {
        buttons_down: buttons::MOUSE_LEFT,
        buttons_pressed: buttons::MOUSE_LEFT,
        pointer_x: 1,
        pointer_y: 41,
    });
    assert_eq!(
        vm.step(&mut host, Default::default(), 20).unwrap(),
        VmYield::WaitForInput
    );
    host.tick(InputFrame {
        pointer_x: 1,
        pointer_y: 41,
        ..Default::default()
    });
    spin(|| vm.step(&mut host, Default::default(), 20).unwrap() == VmYield::Halted);
    let (machine, _) = vm.into_idle_state().unwrap();
    assert_eq!(machine.pc(), 47);
    assert_eq!(
        machine.executed(),
        1,
        "only the selected original END executes"
    );
    assert_eq!(machine.variables().unwrap().read_stored(250), 123);
    assert!(host.engine.ui.get(1, 300).is_none());
}

#[test]
fn saved_special_selection_rebuilds_count_indexed_windows_variable_cells_and_distinct_message_font()
{
    let mut host = Host::new();
    let setup=Names::parse(b"useSpecialNumber 2 specialWindowSizeX 16 specialWindowSizeY 10 specialWindow2printX1 3 specialWindow2printY1 12 specialWindow2printX2 28 specialWindow2printY2 35 specialmessagePrintX 2 specialmessagePrintY 1 specialCursorDeltaX 0 specialCursorDeltaY 0 specialCursorSizeX 14 specialCursorSizeY 8 specialFontSize 4 centeringFlag 1 centerAreaSizeX 16 specialWindowVarName1 frame",false).unwrap();
    let mut live = NativeSelectionBridge::new(selection_profile()).unwrap();
    live.configure_native_background(&setup).unwrap();
    live.configure_special(Arc::new(crate::native_ui::special::Policy {
        setup,
        variables: Names::parse(b"frame", false).unwrap(),
        window: crate::native_ui::special::Atlas {
            path: "authored-window.png".into(),
            size: [32, 20],
            image: None,
        },
        cursor: None,
    }))
    .unwrap();
    let mut variables = Variables::fresh(VariableProfile::Legacy1000);
    variables.write_stored(0, 4);
    let seed = live.restore_seed().unwrap();
    let mut extra = vec![b'#'];
    extra.extend(raw("説明"));
    let mut preparation = seed
        .begin_with_controls(
            vec![extra, raw("左"), raw("右")],
            3,
            1,
            (50, 2, 1),
            &variables,
            &mut host,
        )
        .unwrap();
    spin(|| preparation.poll_resources(&mut host).unwrap());
    let ready = preparation.finish().unwrap();
    assert_eq!(ready.bridge.original_controls(), (50, 2, 1));
    assert_eq!(ready.bridge.profile.rows[0].hit_rect, [5, 13, 14, 8]);
    assert_eq!(ready.bridge.profile.rows[1].hit_rect, [30, 36, 14, 8]);
    let window = ready.batch["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["layer"]["id"] == 656)
        .unwrap();
    assert_eq!(window["layer"]["rect"], json!([3, 12, 16, 10]));
    assert_eq!(window["layer"]["uv"], json!([0.5, 0.5, 1.0, 1.0]));
    host.submit_engine(ClientId(1), 1, &serde_json::to_vec(&ready.batch).unwrap())
        .unwrap();
    assert!(host.engine.ui.interaction(1, 300).is_some());
    let (layers, widgets) = ready.bridge.restored_node_ids();
    assert!(layers.contains(&656));
    assert!(layers.contains(&657));
    assert_eq!(widgets, vec![300, 301]);
}
