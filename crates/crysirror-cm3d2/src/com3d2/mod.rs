// SPDX-License-Identifier: MPL-2.0
//! Standalone Custom Order Maid 3D 2, Unity 5.6.4p2 / serialized format 17.
mod club;
mod widgets;
use crate::{
    adapter::{Config, Opener},
    files::GameFiles,
    kag::{AdvKag, KagStep},
    sound::{Kind, Sound},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    service::{HostServices, InputFrame, buttons},
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use serde_json::json;
use widgets::{Assets, Widget};

const BUTTONS: [&str; 6] = ["Start", "Continue", "Guest", "Config", "End", "Shop"];
const CLICK: u64 =
    buttons::ENTER | buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::MOUSE_MIDDLE;

#[derive(PartialEq, Clone, Copy, Debug)]
enum Scene {
    Logo,
    Warning,
    Script,
    Title,
}
impl Scene {
    fn name(self) -> &'static str {
        match self {
            Self::Logo => "SceneLogo",
            Self::Warning => "SceneWarning",
            Self::Script => "SceneADV",
            Self::Title => "SceneTitle",
        }
    }
}

struct Game {
    assets: Assets,
    files: GameFiles,
    script: AdvKag,
    sound: Sound,
    scene: Scene,
    widgets: Vec<Widget>,
    previous_layers: usize,
    club: Option<String>,
    club_meshes: usize,
    entered: u64,
    leaving: Option<u64>,
    down: u64,
    pointer: Option<[i32; 2]>,
    pressed: Option<usize>,
    focus: usize,
}

impl Game {
    fn enter(
        &mut self,
        scene: Scene,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        now: u64,
    ) -> Result<()> {
        self.widgets = if scene == Scene::Script {
            Vec::new()
        } else {
            self.assets.scene(op, host, client, scene.name())?
        };
        if self.scene == Scene::Title {
            if let Some(asset) = self.club.take() {
                host.submit_engine(
                    client,
                    1,
                    &serde_json::to_vec(&json!({"world":[{"op":"despawn","id":1}]}))?,
                )?;
                host.release_model(
                    client,
                    &crysirror_core::path::VfsPath::new(
                        crysirror_core::path::Mount::Asset,
                        &asset,
                    )?,
                )?;
                self.club_meshes = 0;
            }
        }
        if scene == Scene::Title {
            let file = self.assets.scene_file(scene.name())?;
            let (glb, count) = club::model(&mut self.assets.unity, op, &file)?;
            let asset = host
                .upload_model(client, 0x50000000, &glb)?
                .relative()
                .to_owned();
            let mut camera = crate::bg::orbit_camera(
                [-9.30211, 1.579518, 17.12966],
                [-221.6162, -9.254665],
                0.5999951,
                35.,
            );
            camera["op"] = json!("set_camera");
            camera["camera"]["near"] = json!(0.01);
            let world = json!({"world": [
                {"op": "set_environment", "environment": {}},
                {"op": "spawn", "entity": {"id": 1, "name": "EmpireClub_Rotary_Night",
                    "geometry": {"kind": "scene", "asset": asset}}},
                camera
            ]});
            host.submit_engine(client, 1, &serde_json::to_vec(&world)?)?;
            self.club = Some(asset);
            self.club_meshes = count;
        }
        self.scene = scene;
        self.entered = now;
        self.leaving = None;
        if scene == Scene::Title {
            self.sound
                .play(host, client, &self.files, Kind::Bgm, "BGM004.ogg", 0., true)?;
        }
        Ok(())
    }

    fn button(w: &Widget) -> Option<usize> {
        let mut parts = w.path.split('/');
        parts.find(|s| *s == "ButtonGroup")?;
        let group = parts.next()?.strip_suffix("Group")?;
        BUTTONS.iter().position(|s| *s == group)
    }

    fn rect(&self, w: &Widget, elapsed: u64) -> [f32; 4] {
        let mut rect = w.rect;
        if self.scene == Scene::Title {
            if let Some(i) = Self::button(w) {
                let t = ((elapsed as f32 - i as f32 * 300.) / 700.).clamp(0., 1.);
                rect[0] -= 600. * (1. - crate::scenes::ease_out_expo(t));
            }
        }
        rect
    }

    fn update(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        op: &mut Opener,
        input: InputFrame,
    ) -> Result<()> {
        let now = host.now_ms();
        let elapsed = now.saturating_sub(self.entered);
        let released = self.down & !input.buttons_down;
        self.down = input.buttons_down;
        self.sound.poll(host, client)?;
        match self.scene {
            Scene::Logo | Scene::Warning => {
                if let Some(since) = self.leaving {
                    if now.saturating_sub(since) >= 1000 {
                        if self.scene == Scene::Logo {
                            self.enter(Scene::Warning, op, host, client, now)?;
                        } else {
                            self.script
                                .load(&self.files, op.storage.as_ref(), "AllMain.ks", "")?;
                            self.script.exec();
                            self.enter(Scene::Script, op, host, client, now)?;
                        }
                    }
                } else if elapsed >= 1000 && released & CLICK != 0 {
                    self.sound.play(
                        host,
                        client,
                        &self.files,
                        Kind::System,
                        "SE002.ogg",
                        0.,
                        false,
                    )?;
                    self.leaving = Some(now);
                }
            }
            Scene::Script => {
                if let KagStep::SceneCall(next) =
                    self.script
                        .update(now, false, false, &self.files, op.storage.as_ref())?
                {
                    match next.as_str() {
                        "SceneADV" => {
                            self.script.enabled = true;
                            self.script.exec();
                        }
                        "SceneTitle" => self.enter(Scene::Title, op, host, client, now)?,
                        _ => bail!(
                            "COM3D2 scene {next} is not implemented (KAG {})",
                            self.script.position()
                        ),
                    }
                }
                // During reset_scene.ks these refer only to the empty boot ADV stage.
                for effect in self.script.ui.drain(..) {
                    use crate::kag::AdvUi;
                    match effect {
                        AdvUi::StopMotionScripts | AdvUi::Window(false) | AdvUi::MotionBlend(_) => {
                        }
                        other => bail!("unsupported COM3D2 ADV effect {other:?}"),
                    }
                }
            }
            Scene::Title => {
                if let Some(since) = self.leaving {
                    if now.saturating_sub(since) >= 3000 {
                        let label = self
                            .script
                            .tag_backup
                            .get("label")
                            .context("SceneTitle return label")?
                            .clone();
                        self.script
                            .jump_label(&self.files, op.storage.as_ref(), &label)?;
                        self.script.enabled = true;
                        self.script.exec();
                        self.enter(Scene::Script, op, host, client, now)?;
                    }
                } else if elapsed >= 3000 {
                    let p = [input.pointer_x, input.pointer_y];
                    let current = self.widgets.iter().find_map(|w| {
                        let index = Self::button(w)?;
                        if !w.path.ends_with(BUTTONS[index]) {
                            return None;
                        }
                        let [x, y, width, height] = self.rect(w, elapsed);
                        ((p[0] as f32) >= x
                            && (p[0] as f32) < x + width
                            && (p[1] as f32) >= y
                            && (p[1] as f32) < y + height)
                            .then_some(index)
                    });
                    if self.pointer != Some(p) {
                        if let Some(i) = current {
                            self.focus = i;
                        }
                        self.pointer = Some(p);
                    }
                    if input.buttons_pressed & buttons::DOWN != 0 {
                        self.focus = (self.focus + 1) % BUTTONS.len();
                    }
                    if input.buttons_pressed & buttons::UP != 0 {
                        self.focus = (self.focus + BUTTONS.len() - 1) % BUTTONS.len();
                    }
                    if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                        self.pressed = current;
                    }
                    let click = released & buttons::MOUSE_LEFT != 0
                        && self.pressed.take() == current
                        && current.is_some();
                    if click || (input.buttons_pressed & buttons::ENTER) != 0 {
                        if click {
                            self.focus = current.unwrap();
                        }
                        let name = BUTTONS[self.focus];
                        if name == "Start" {
                            self.leaving = Some(now);
                        } else {
                            bail!("COM3D2 TitleCtrl.On{name} is not implemented");
                        }
                    }
                }
            }
        }
        let elapsed = now.saturating_sub(self.entered);
        let fade_in = if self.scene == Scene::Title {
            3000.
        } else {
            1000.
        };
        let brightness = self
            .leaving
            .map_or((elapsed as f32 / fade_in).min(1.), |since| {
                (1. - now.saturating_sub(since) as f32 / fade_in).clamp(0., 1.)
            });
        let mut layers = Vec::new();
        for (i, w) in self.widgets.iter().enumerate() {
            let id = i + 1;
            let rect = self.rect(w, elapsed);
            let visible = !w.path.ends_with("/SelectCursor") || Self::button(w) == Some(self.focus);
            layers.push(json!({"op":"put","layer":{"id":id,"rect":rect,"uv":w.uv,"visible":visible,"paint":{"kind":"image","asset":w.asset,"tint":w.tint},"order":[w.depth,id]}}));
        }
        for id in self.widgets.len() + 1..=self.previous_layers {
            layers.push(json!({"op":"remove","id":id}));
        }
        self.previous_layers = self.widgets.len();
        layers.push(json!({"op":"put","layer":{"id":65535,"rect":[0,0,1920,1080],"paint":{"kind":"solid","color":[0,0,0,255]},"opacity":((1.-brightness)*255.) as u8,"order":[100000,0]}}));
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
        Ok(())
    }
}

struct Vm {
    client: ClientId,
    config: Config,
    game: Option<Game>,
}
impl Vm {
    fn advance(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<()> {
        let mut op = Opener {
            storage: host.storage(self.client)?,
            dir: self.config.data_dir.clone(),
        };
        if self.game.is_none() {
            let files = GameFiles::index(op.storage.as_ref(), &self.config.archives)?;
            let script = AdvKag::new(&files, op.storage.as_ref())?;
            let assets = Assets::load(&mut op)?;
            let now = host.now_ms();
            let mut g = Game {
                assets,
                files,
                script,
                sound: Sound::new(),
                scene: Scene::Logo,
                widgets: vec![],
                previous_layers: 0,
                club: None,
                club_meshes: 0,
                entered: now,
                leaving: None,
                down: 0,
                pointer: None,
                pressed: None,
                focus: 0,
            };
            g.enter(Scene::Logo, &mut op, host, self.client, now)?;
            self.game = Some(g);
        }
        self.game
            .as_mut()
            .unwrap()
            .update(host, self.client, &mut op, input)
    }
}
impl VmAdapter for Vm {
    fn engine_id(&self) -> &'static str {
        "com3d2"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("COM3D2 snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("COM3D2 snapshot"))
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        _: u32,
    ) -> crysirror_core::Result<VmYield> {
        self.advance(host, input)
            .map_err(|e| Error::Io(format!("{e:#}")))?;
        Ok(VmYield::WaitUntil(host.now_ms() + 1))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let state=self.game.as_ref().map(|g|json!({"scene":g.scene.name(),"widgets":g.widgets.len(),"club_meshes":g.club_meshes,"focus":BUTTONS[g.focus],"script":g.script.position(),"bgm":g.sound.current_bgm}));
        serde_json::to_vec(&state).unwrap_or_default()
    }
}
pub(crate) struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(
        &self,
        client: ClientId,
        spec: &crysirror_runtime::model::ClientSpec,
    ) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "COM3D2 uses native scripts");
        let config: Config = serde_json::from_value(spec.config.clone())?;
        config.validate()?;
        Ok(Box::new(Vm {
            client,
            config,
            game: None,
        }))
    }
}
