// SPDX-License-Identifier: MPL-2.0
//! The 甘夏ろけーしょん VM session: the installed Unity data, the active
//! build scene and its uGUI presentation through shared layer services.
use crate::{
    ui::{Host, Ui},
    unity::Unity,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{FontLibrary, HostServices, InputFrame, TextMeasurement},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_unity::{scene6::Scene6, ugui, PPtr};
use serde::Deserialize;
use serde_json::json;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

pub const ENGINE_ID: &str = "amanatsu";
pub const WIDTH: f32 = 1920.;
pub const HEIGHT: f32 = 1080.;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Unity player data directory (`AmanatsuLocation_Data`).
    pub data_dir: String,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.data_dir.is_empty() && !self.data_dir.contains(".."), "AmanatsuLocation data directory");
        Ok(())
    }
}

/// The scene shown, by build index (`BuildSettings`): Init, Logo, Title,
/// ActionMap, CustomScene, Memory.
pub struct Level {
    pub key: String,
    pub scene: Scene6,
    pub states: HashMap<usize, ugui::NodeState>,
}

pub struct Game {
    pub unity: Unity,
    pub ui: Ui,
    pub level: Level,
    pub now: u64,
    pub log: Vec<String>,
}

struct HostAdapter<'a> {
    host: &'a mut dyn HostServices,
    client: ClientId,
}

impl Host for HostAdapter<'_> {
    fn upload(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<String> {
        Ok(self.host.upload_rgba(self.client, id, width, height, rgba)?.relative().to_owned())
    }
    fn fonts(&mut self) -> Result<Arc<dyn FontLibrary>> {
        Ok(self.host.font_library(self.client)?)
    }
    fn measurement(&mut self) -> Result<Arc<dyn TextMeasurement>> {
        Ok(self.host.text_measurement(self.client)?)
    }
}

pub struct AmanatsuVm {
    client: ClientId,
    config: Config,
    game: Option<Game>,
    error: Option<String>,
}

impl AmanatsuVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self { client, config, game: None, error: None })
    }

    fn boot(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let storage = host.storage(self.client)?;
        let mut unity = Unity::new(storage, &self.config.data_dir)?;
        // SATD-amanatsu-port: InitScene/LogoScene/TitleScene logic; the
        // title scene is shown directly.
        let key = "level2".to_owned();
        let scene = unity.scene(&key)?;
        let mut states = HashMap::new();
        if let Some(n) = scene.find("Title/Canvas/Information/languageDLG") {
            states.insert(n, ugui::NodeState { active: Some(false), ..Default::default() });
        }
        self.game = Some(Game { unity, ui: Ui::default(), level: Level { key, scene, states }, now: 0, log: Vec::new() });
        Ok(())
    }

    fn step_inner(&mut self, host: &mut dyn HostServices, _input: InputFrame) -> Result<VmYield> {
        if self.game.is_none() {
            self.boot(host)?;
        }
        let game = self.game.as_mut().context("game")?;
        game.now = host.now_ms();
        let client = self.client;
        let key = game.level.key.clone();
        let cell = RefCell::new((&mut game.unity, &mut game.ui, HostAdapter { host, client }));
        let info = |p: PPtr| {
            let mut c = cell.borrow_mut();
            c.0.sprite_meta(&key, p).map(|(b, _, ppu)| (b, ppu))
        };
        let size = |p: PPtr| {
            let mut c = cell.borrow_mut();
            c.0.sprite_meta(&key, p).map(|(_, s, ppu)| (s, ppu))
        };
        let measure = |t: &str, f: PPtr, px: f32| {
            let mut c = cell.borrow_mut();
            let (u, ui, h) = &mut *c;
            ui.measure(u, h, &key, t, f, px)
        };
        let svc = ugui::Services { sprite_info: &info, sprite_size: &size, measure: &measure };
        let layout = ugui::layout(&game.level.scene, &game.level.states, WIDTH, HEIGHT, &svc);
        drop(svc);
        let (unity, ui, mut h) = cell.into_inner();
        let ops = ui.compose(unity, &mut h, &key, &layout, 0)?;
        h.host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers": ops}))?)?;
        Ok(VmYield::WaitUntil(game.now + 16))
    }
}

impl VmAdapter for AmanatsuVm {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn preparation_pending(&self) -> bool {
        false
    }
    fn step(&mut self, host: &mut dyn HostServices, input: InputFrame, _budget: u32) -> crysirror_core::Result<VmYield> {
        match self.step_inner(host, input) {
            Ok(v) => Ok(v),
            Err(e) => {
                let message = format!("{e:#}");
                self.error = Some(message.clone());
                Err(Error::Io(message))
            }
        }
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("AmanatsuLocation framework snapshot"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("AmanatsuLocation framework snapshot"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        let Some(g) = &self.game else {
            return serde_json::to_vec(&json!({"error": self.error})).unwrap_or_default();
        };
        serde_json::to_vec(&json!({
            "scene": g.level.key,
            "error": self.error,
            "hits": g.ui.hits.iter().map(|(n, r)| json!({"path": g.level.scene.path(*n), "rect": [r.x0, r.y0, r.x1, r.y1]})).collect::<Vec<_>>(),
            "log": g.log.iter().rev().take(32).collect::<Vec<_>>(),
        }))
        .unwrap_or_default()
    }
}

#[cfg(feature = "runtime")]
pub fn register(registry: &mut crysirror_runtime::AdapterRegistry) -> Result<()> {
    registry.register(ENGINE_ID, Arc::new(Factory))
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl crysirror_runtime::runtime::VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &crysirror_runtime::model::ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "AmanatsuLocation executes native scenes, not data actions");
        Ok(Box::new(AmanatsuVm::new(client, serde_json::from_value(spec.config.clone())?)?))
    }
}
