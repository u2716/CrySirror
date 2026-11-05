// SPDX-License-Identifier: MPL-2.0
// Button patterns, corner boxes and WBCALL policy adopted from siglus_rs
// 5f358ef1360b0439e92aed2b749e8f25d8721954, reallive/window_buttons.rs.
//! Native window-skin policy. Images, input, text layout and modal UI are
//! retained engine resources; this module owns no pixels or hit testing.
use super::*;
use crate::vm::machine::{Frame, LongOp};
use crysirror_scene::ui::WidgetInteraction;
use std::{cell::Cell, collections::BTreeSet};

const FIRST: u32 = 650_000;
const HISTORY: u32 = 660_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Clear,
    Backlog,
    Left,
    Right,
    Extra(u16),
    Voice,
    Skip,
    Auto,
}
impl Kind {
    const ALL: [Self; 15] = [
        Self::Clear,
        Self::Backlog,
        Self::Left,
        Self::Right,
        Self::Extra(0),
        Self::Extra(1),
        Self::Extra(2),
        Self::Extra(3),
        Self::Extra(4),
        Self::Extra(5),
        Self::Extra(6),
        Self::Extra(7),
        Self::Voice,
        Self::Skip,
        Self::Auto,
    ];
    fn name(self) -> String {
        match self {
            Self::Clear => "CLEAR".into(),
            Self::Backlog => "MSGBK".into(),
            Self::Left => "MSGBKLEFT".into(),
            Self::Right => "MSGBKRIGHT".into(),
            Self::Voice => "KOEPLAY".into(),
            Self::Skip => "READJUMP".into(),
            Self::Auto => "AUTOMODE".into(),
            Self::Extra(n) => format!("EXBTN_{n:03}"),
        }
    }
    fn pattern(self) -> i32 {
        match self {
            Self::Clear => 8,
            Self::Backlog => 16,
            Self::Left => 24,
            Self::Right => 32,
            Self::Extra(n) => 40 + 8 * i32::from(n),
            Self::Voice => 120,
            Self::Skip => 104,
            Self::Auto => 112,
        }
    }
}

#[derive(Default)]
pub(super) struct Toolbar {
    pub(super) open: BTreeSet<u32>,
    drawn: BTreeMap<u32, (Value, Value)>,
    seen: BTreeMap<u32, u32>,
    hidden_by_button: bool,
    return_wait: Option<(usize, Wait, Rc<Cell<bool>>)>,
    pub(super) pause_started: Option<u64>,
    pub(super) chars_since_pause: usize,
    pages: Vec<String>,
    current: BTreeMap<u32, String>,
    history: Option<(Vec<String>, usize)>,
}
impl Toolbar {
    pub(super) fn record(&mut self, window: u32, text: &str, chars: usize) {
        self.current.entry(window).or_default().push_str(text);
        self.chars_since_pause = self.chars_since_pause.saturating_add(chars);
    }
    pub(super) fn archive(&mut self, window: u32) {
        if let Some(text) = self.current.remove(&window).filter(|s| !s.is_empty()) {
            self.pages.push(text);
            if self.pages.len() > 256 {
                self.pages.remove(0);
            }
        }
    }
    pub(super) fn archive_all(&mut self) {
        for w in self.current.keys().copied().collect::<Vec<_>>() {
            self.archive(w);
        }
    }
}

#[derive(Debug)]
struct ButtonReturnFence(Rc<Cell<bool>>);
impl LongOp for ButtonReturnFence {
    fn step(&mut self, _: &mut Machine) -> Result<bool> {
        Ok(self.0.get())
    }
    fn name(&self) -> &'static str {
        "window button return"
    }
}

fn corner_box(window: [i32; 4], values: &[i32]) -> Option<[i32; 4]> {
    let [origin, x, y, w, h] = *values else {
        return None;
    };
    if w <= 0 || h <= 0 {
        return None;
    }
    Some([
        if matches!(origin, 1 | 3) {
            window[0] + window[2] - x - w
        } else {
            window[0] + x
        },
        if matches!(origin, 2 | 3) {
            window[1] + window[3] - y - h
        } else {
            window[1] + y
        },
        w,
        h,
    ])
}

impl RealLiveVm {
    fn window_button_specs(&self, m: &Machine) -> Result<Vec<(u32, Kind, Value, Value)>> {
        let mut out = Vec::new();
        for &window in &self.toolbar.open {
            let prefix = format!("WINDOW.{window:03}.");
            let set = m.gameexe.int(&format!("{prefix}WAKU_SETNO")).unwrap_or(0);
            if set < 0 {
                continue;
            }
            let pattern = if m.gameexe.int(&format!("{prefix}WAKU_MOD")).unwrap_or(0) != 0 {
                m.gameexe.int(&format!("{prefix}WAKU_NO")).unwrap_or(0)
            } else {
                m.sys.settings.waku_all
            };
            let skin = format!("WAKU.{set:03}.{pattern:03}.");
            let Some(name) = m
                .gameexe
                .str(&format!("{skin}BTN"))
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let image = m
                .sys
                .gfx
                .loader
                .as_ref()
                .context("window button image loader")?
                .image(name)?;
            let geometry = self.frame_geometry(m, window, None)?.window;
            let visible = !self.ext.text_hidden && !self.ext.windows_off.contains(&window);
            let enabled = visible
                && matches!(self.wait, Some(Wait::Pause { .. } | Wait::PageFull { .. }))
                && self.toolbar.return_wait.is_none()
                && self.toolbar.history.is_none();
            for (slot, kind) in Kind::ALL.into_iter().enumerate() {
                let name = kind.name();
                let global = if matches!(kind, Kind::Extra(_)) {
                    "WINDOW_EXBTN_USE".into()
                } else {
                    format!("WINDOW_{name}_USE")
                };
                if m.gameexe.int(&global).unwrap_or(1) == 0
                    || m.gameexe.int(&format!("{prefix}{name}_USE")) == Some(0)
                {
                    continue;
                }
                if let Kind::Extra(n) = kind {
                    if m.sys
                        .remembered
                        .get(&(crate::vm::modules::sys_max::EXBTN_MOD + n))
                        .copied()
                        .unwrap_or(1)
                        == 0
                        || m.sys.remembered.get(&(4600 + n)).copied().unwrap_or(1) == 0
                    {
                        continue;
                    }
                }
                let Some(rect) =
                    corner_box(geometry, &m.gameexe.ints(&format!("{skin}{name}_BOX")))
                else {
                    continue;
                };
                let id = FIRST + window * 32 + slot as u32;
                let widget = json!({"id":id,"kind":"button","rect":rect,"z":1001,
                    "visible":visible,"enabled":enabled,"interaction_only":true,
                    "keyboard_focusable":false,"activate_on_press":true});
                // Retained engine interactions drive the art too; no second
                // adapter-local pointer hit tester is needed.
                let layer = json!({"id":id,"parent":SCENE_ROOT,"rect":rect,
                    "paint":{"kind":"image","asset":image.asset},"order":[999,3],"visible":visible});
                out.push((id, kind, widget, layer));
            }
        }
        Ok(out)
    }

    pub(super) fn sync_window_buttons(
        &mut self,
        m: &Machine,
        h: &mut dyn HostServices,
    ) -> Result<()> {
        let specs = self.window_button_specs(m)?;
        let mut layers = Vec::new();
        let mut ui = Vec::new();
        let mut next = BTreeMap::new();
        for (id, kind, widget, mut layer) in specs {
            let interaction = self.query(h, EngineQuery::UiInteraction(id))?;
            let hover = interaction["hovered"].as_bool().unwrap_or(false);
            let pressed = interaction["pressed"].as_bool().unwrap_or(false);
            let on = match kind {
                Kind::Skip => m.skip_mode,
                Kind::Auto => m.sys.settings.auto_mode,
                _ => false,
            };
            let state = match (on, hover) {
                (true, true) => 4,
                (true, false) => 3,
                (false, true) if pressed => 2,
                (false, true) => 1,
                _ => 0,
            };
            let window = (id - FIRST) / 32;
            let prefix = format!("WINDOW.{window:03}.");
            let set = m.gameexe.int(&format!("{prefix}WAKU_SETNO")).unwrap_or(0);
            let pattern = if m.gameexe.int(&format!("{prefix}WAKU_MOD")).unwrap_or(0) != 0 {
                m.gameexe.int(&format!("{prefix}WAKU_NO")).unwrap_or(0)
            } else {
                m.sys.settings.waku_all
            };
            let name = m
                .gameexe
                .str(&format!("WAKU.{set:03}.{pattern:03}.BTN"))
                .context("button skin")?;
            let image = m
                .sys
                .gfx
                .loader
                .as_ref()
                .context("button loader")?
                .image(name)?;
            let region = image.region(kind.pattern() + state);
            let region = if region.x2 > region.x1 {
                region
            } else {
                image.region(kind.pattern())
            };
            let (w, h) = (image.width as f32, image.height as f32);
            layer["rect"][2] = json!(region.x2 - region.x1 + 1);
            layer["rect"][3] = json!(region.y2 - region.y1 + 1);
            layer["uv"] = json!([
                region.x1 as f32 / w,
                region.y1 as f32 / h,
                (region.x2 + 1) as f32 / w,
                (region.y2 + 1) as f32 / h
            ]);
            if self
                .toolbar
                .drawn
                .get(&id)
                .is_none_or(|(a, _)| a != &widget)
            {
                ui.push(json!({"op":"put","widget":widget}));
            }
            if self.toolbar.drawn.get(&id).is_none_or(|(_, a)| a != &layer) {
                layers.push(json!({"op":"put","layer":layer}));
            }
            next.insert(id, (widget, layer));
        }
        for id in self
            .toolbar
            .drawn
            .keys()
            .filter(|id| !next.contains_key(id))
        {
            layers.push(json!({"op":"remove","id":id}));
            ui.push(json!({"op":"remove","id":id}));
        }
        if !layers.is_empty() || !ui.is_empty() {
            self.submit(h, json!({"layers":layers,"ui":ui}))?;
        }
        self.toolbar.drawn = next;
        Ok(())
    }

    pub(super) fn restore_button_wait(&mut self, m: &Machine) {
        if self
            .toolbar
            .return_wait
            .as_ref()
            .is_some_and(|(depth, _, _)| m.stack.len() <= *depth)
        {
            let (_, wait, ready) = self.toolbar.return_wait.take().unwrap();
            ready.set(true);
            self.wait = Some(wait);
            self.toolbar.pause_started = None;
        }
    }

    pub(super) fn tick_window_buttons(
        &mut self,
        m: &mut Machine,
        h: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        if self.toolbar.hidden_by_button {
            if input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::ENTER | buttons::SPACE) != 0
            {
                self.toolbar.hidden_by_button = false;
                self.set_text_hidden(h, false)?;
                self.input_used = true;
            }
            return Ok(true);
        }
        if self.toolbar.history.is_some() {
            return self.poll_button_history(h, input);
        }
        if !matches!(self.wait, Some(Wait::Pause { .. } | Wait::PageFull { .. })) {
            return Ok(false);
        }
        for (id, kind, widget, _) in self.window_button_specs(m)? {
            if widget["enabled"] != true {
                continue;
            }
            let value = self.query(h, EngineQuery::UiInteraction(id))?;
            if value.is_null() {
                continue;
            }
            let interaction: WidgetInteraction = serde_json::from_value(value)?;
            if !interaction.activated
                || self.toolbar.seen.get(&id) == Some(&interaction.activation_serial)
            {
                continue;
            }
            self.toolbar.seen.insert(id, interaction.activation_serial);
            self.input_used = true;
            match kind {
                Kind::Clear => {
                    self.toolbar.hidden_by_button = true;
                    self.set_text_hidden(h, true)?;
                }
                Kind::Backlog | Kind::Left => {
                    if !m.sys.backlog_enabled {
                        return Ok(true);
                    }
                    let mut pages = self.toolbar.pages.clone();
                    pages.extend(
                        self.toolbar
                            .current
                            .values()
                            .filter(|t| !t.is_empty())
                            .cloned(),
                    );
                    if !pages.is_empty() {
                        let index = pages.len() - 1;
                        self.toolbar.history = Some((pages, index));
                        self.draw_button_history(h)?;
                    }
                }
                // Original MSGBKRIGHT has no action outside backlog.
                Kind::Right => {}
                Kind::Voice => {
                    let (id, _) = m.sys.sound.last_koe;
                    if id >= 0 {
                        self.play_koe(m, h, id, false)?;
                    }
                }
                Kind::Skip => m.skip_mode = !m.skip_mode,
                Kind::Auto => {
                    m.sys.settings.auto_mode = !m.sys.settings.auto_mode;
                    self.toolbar.pause_started = Some(h.now_ms());
                }
                Kind::Extra(n) => {
                    let values = m.gameexe.ints(&format!("WBCALL.{n:03}"));
                    if let Some(&scene) = values.first() {
                        let scenario = m.archive.scenario(scene)?;
                        let ip = scenario
                            .entrypoint(values.get(1).copied().unwrap_or(0))
                            .unwrap_or(0);
                        let ready = Rc::new(Cell::new(false));
                        let wait = self.wait.take().context("window button pause")?;
                        self.toolbar.return_wait = Some((m.stack.len(), wait, ready.clone()));
                        m.push_long_op(Box::new(ButtonReturnFence(ready)));
                        m.stack.push(Frame::new(scenario, ip, FrameKind::Farcall));
                    }
                }
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn button_pause_ready(&mut self, m: &Machine, h: &mut dyn HostServices) -> bool {
        let since = *self.toolbar.pause_started.get_or_insert_with(|| h.now_ms());
        if m.sys.should_fast_forward() || (m.skip_mode && m.kidoku_read) {
            return true;
        }
        if !m.sys.settings.auto_mode {
            return false;
        }
        let delay = i64::from(m.sys.settings.auto_base_time)
            + i64::from(m.sys.settings.auto_char_time) * self.toolbar.chars_since_pause as i64;
        let voice_busy = h
            .audio_info(self.client, 1)
            .ok()
            .flatten()
            .is_some_and(|s| s.source_playing);
        !voice_busy && h.now_ms().saturating_sub(since) >= delay.max(0) as u64
    }

    fn draw_button_history(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let (pages, index) = self.toolbar.history.as_ref().context("window history")?;
        let (w, height) = (self.width, self.height);
        let mut ui = vec![
            json!({"op":"put","widget":{"id":HISTORY,"kind":"panel",
            "rect":[0,0,w,height],"z":2000,"modal":true,"clip_children":true,"color":[12,16,24,245]}}),
            json!({"op":"put","widget":{"id":HISTORY+1,"parent":HISTORY,"kind":"label",
                "rect":[24,24,w.saturating_sub(48),height.saturating_sub(100)],
                "font":self.config.font_alias,"text":pages[*index],"text_size":24}}),
        ];
        for (offset, text, x, enabled) in [
            (2, "<", 24, *index > 0),
            (3, ">", 110, *index + 1 < pages.len()),
            (4, "Close", 196, true),
        ] {
            ui.push(json!({"op":"put","widget":{"id":HISTORY+offset,"parent":HISTORY,"kind":"button",
                "rect":[x,height.saturating_sub(60),80,36],"text":text,"font":self.config.font_alias,"enabled":enabled}}));
        }
        self.submit(h, json!({"ui":ui}))
    }
    fn poll_button_history(&mut self, h: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        let mut action = None;
        for n in 2..=4 {
            let id = HISTORY + n;
            let value = self.query(h, EngineQuery::UiInteraction(id))?;
            if value["activated"] == true {
                let serial = value["activation_serial"].as_u64().unwrap_or(0) as u32;
                if self.toolbar.seen.get(&id) != Some(&serial) {
                    self.toolbar.seen.insert(id, serial);
                    action = Some(n);
                }
            }
        }
        if input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0 {
            action = Some(4);
        }
        if action == Some(4) {
            self.toolbar.history = None;
            self.toolbar.pause_started = Some(h.now_ms());
            self.submit(h, json!({"ui":[{"op":"remove","id":HISTORY}]}))?;
        } else if let Some(n) = action {
            let (pages, index) = self.toolbar.history.as_mut().unwrap();
            *index = if n == 2 {
                index.saturating_sub(1)
            } else {
                (*index + 1).min(pages.len() - 1)
            };
            self.draw_button_history(h)?;
        }
        self.input_used = true;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{
        archive,
        bytecode::{Opcode, asm},
        graphics::{AssetLoader, Image},
        object::TextParams,
        scenario,
    };
    use crysirror_runtime::{AdapterRegistry, Project, Runtime};
    struct Images;
    impl AssetLoader for Images {
        fn image(&self, _: &str) -> Result<Rc<Image>> {
            Ok(Rc::new(Image {
                asset: "buttons.png".into(),
                width: 256,
                height: 16,
                regions: (0..128)
                    .map(|n| crysirror_image::g00::Region {
                        x1: n * 2,
                        y1: 0,
                        x2: n * 2 + 1,
                        y2: 15,
                        origin_x: 0,
                        origin_y: 0,
                    })
                    .collect(),
            }))
        }
        fn gan(&self, _: &str) -> Result<Vec<u8>> {
            bail!("not used")
        }
        fn text_size(&self, _: &TextParams) -> (i32, i32) {
            (0, 0)
        }
    }
    fn fixture() -> (tempfile::TempDir, Runtime, RealLiveVm, Machine) {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("assets")).unwrap();
        let exe = "#SEEN_START=1\n#WAKU.000.000.BTN=\"buttons\"\n#WAKU.000.000.CLEAR_BOX=0,10,10,30,16\n#WAKU.000.000.AUTOMODE_BOX=0,50,10,30,16\n#WAKU.000.000.EXBTN_000_BOX=0,90,10,30,16\n#WBCALL.000=2,0\n";
        std::fs::write(t.path().join("assets/Gameexe.ini"), exe).unwrap();
        let code = asm::command(Opcode::new(0, 3, 15, 0), 0, &[]);
        let returning = asm::command(Opcode::new(0, 4, 300, 0), 0, &[]);
        std::fs::write(
            t.path().join("assets/Seen.txt"),
            archive::build(&[
                (1, scenario::build(&code, &[], &[])),
                (2, scenario::build(&returning, &[], &[])),
            ]),
        )
        .unwrap();
        let p:Project=serde_json::from_value(json!({"schema":2,"name":"buttons","content_revision":"1","canvas":[640,480],"assets":"assets","clients":[{"name":"test","adapter":"reallive","game":"synthetic","config":{}}]})).unwrap();
        let mut registry = AdapterRegistry::default();
        crate::register(&mut registry).unwrap();
        std::fs::write(t.path().join("project.json"), serde_json::to_vec(&p).unwrap()).unwrap();
        let r = Runtime::with_registry(
            p,
            &t.path().join("project.json"),
            &t.path().join("state"),
            registry,
        )
        .unwrap();
        let mut vm =
            RealLiveVm::new(ClientId(1), serde_json::from_value(json!({})).unwrap()).unwrap();
        let mut r = r;
        vm.boot(&mut r.host).unwrap();
        let mut m = vm.machine.take().unwrap();
        m.sys.gfx.loader = Some(Rc::new(Images));
        vm.wait = Some(Wait::Pause {
            kind: 15,
            window: 0,
        });
        vm.toolbar.open.insert(0);
        (t, r, vm, m)
    }
    fn pointer(r: &mut Runtime, x: i32, down: bool) -> InputFrame {
        let input = InputFrame {
            pointer_x: x,
            pointer_y: 12,
            buttons_pressed: if down { buttons::MOUSE_LEFT } else { 0 },
            buttons_down: if down { buttons::MOUSE_LEFT } else { 0 },
            ..Default::default()
        };
        r.host.engine.ui.tick(input, [640, 480]).unwrap();
        input
    }
    #[test]
    fn button_boxes_use_original_corner_origins_and_reject_disabled_boxes() {
        assert_eq!(
            corner_box([10, 20, 800, 121], &[0, 510, 97, 30, 24]),
            Some([520, 117, 30, 24])
        );
        assert_eq!(
            corner_box([10, 20, 800, 121], &[3, 10, 20, 30, 24]),
            Some([770, 97, 30, 24])
        );
        assert_eq!(corner_box([0, 0, 800, 121], &[0, 0, 0, 0, 0]), None);
    }
    #[test]
    fn engine_widgets_drive_atlas_states_hide_and_auto_without_advancing_text() {
        let (_t, mut r, mut vm, mut m) = fixture();
        vm.sync_window_buttons(&m, &mut r.host).unwrap();
        let id = FIRST + 14; // AutoMode, native pattern 112.
        let normal = vm.query(&mut r.host, EngineQuery::Layer(id)).unwrap();
        assert_eq!(normal["uv"][0], 112. * 2. / 256.);
        pointer(&mut r, 51, false);
        vm.sync_window_buttons(&m, &mut r.host).unwrap();
        assert_eq!(
            vm.query(&mut r.host, EngineQuery::Layer(id)).unwrap()["uv"][0],
            113. * 2. / 256.
        );
        let input = pointer(&mut r, 51, true);
        assert!(vm.tick_window_buttons(&mut m, &mut r.host, input).unwrap());
        assert!(m.sys.settings.auto_mode);
        assert!(matches!(vm.wait, Some(Wait::Pause { .. } | Wait::PageFull { .. })));
        vm.sync_window_buttons(&m, &mut r.host).unwrap();
        assert_eq!(
            vm.query(&mut r.host, EngineQuery::Layer(id)).unwrap()["uv"][0],
            116. * 2. / 256.
        );
        pointer(&mut r, 11, false);
        let input = pointer(&mut r, 11, true);
        vm.tick_window_buttons(&mut m, &mut r.host, input).unwrap();
        vm.sync_window_buttons(&m, &mut r.host).unwrap();
        assert!(vm.ext.text_hidden);
        assert_eq!(
            vm.query(&mut r.host, EngineQuery::Layer(FIRST)).unwrap()["visible"],
            false
        );
        vm.tick_window_buttons(
            &mut m,
            &mut r.host,
            InputFrame {
                buttons_pressed: buttons::SPACE,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!vm.ext.text_hidden);
        assert!(matches!(vm.wait, Some(Wait::Pause { .. } | Wait::PageFull { .. })));
    }
    #[test]
    fn extra_script_returns_to_original_pause_before_parent_can_run() {
        let (_t, mut r, mut vm, mut m) = fixture();
        vm.sync_window_buttons(&m, &mut r.host).unwrap();
        let input = pointer(&mut r, 91, true);
        assert!(vm.tick_window_buttons(&mut m, &mut r.host, input).unwrap());
        assert_eq!(m.scene_number(), 2);
        assert!(vm.wait.is_none());
        assert!(matches!(
            m.run_budget(100).unwrap(),
            RunResult::BudgetExhausted
        ));
        assert_eq!(m.scene_number(), 1);
        assert_eq!(
            m.frame().ip,
            0,
            "fence prevents parent bytecode running after rtlButton"
        );
        vm.restore_button_wait(&m);
        assert!(matches!(
            vm.wait,
            Some(Wait::Pause {
                kind: 15,
                window: 0
            })
        ));
        assert!(vm.toolbar.return_wait.is_none());
    }
}
