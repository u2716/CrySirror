// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Native SACT handle/initialization policy. Canvas, layers, fonts and audio are
//! existing CrySirror services; no SDL/gfx/font/mixer implementation is embedded.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
use serde_json::json;
pub(crate) const BACK_SCENE: u32 = 0x40ff_0000;
pub(crate) const MAIN_SPRITE: u32 = 0x40ff_0001;
#[derive(Clone, Debug)]
pub struct Sact {
    owner: Option<ClientId>,
    pub(crate) initial_window: Option<String>,
    pub canvas: [u32; 2],
    pub font_aliases: [String; 2],
    pub initialized: bool,
    pub font_ready: bool,
    pub audio_ready: bool,
    pub calls: u64,
    pub sprites: Option<crate::sprites::Sprites>,
}
impl Sact {
    pub fn new(canvas: [u32; 2], font_aliases: [String; 2]) -> Result<Self> {
        ensure!(
            canvas.iter().all(|n| (1..=8192).contains(n)),
            "System4 native canvas dimensions"
        );
        ensure!(
            font_aliases.iter().all(|n| !n.is_empty() && n.len() <= 256),
            "System4 native font aliases"
        );
        Ok(Self {
            owner: None,
            initial_window: None,
            canvas,
            font_aliases,
            initialized: false,
            font_ready: false,
            audio_ready: false,
            calls: 0,
            sprites: None,
        })
    }
    pub fn init(&mut self, client: ClientId, host: &mut dyn HostServices) -> Result<i32> {
        ensure!(
            self.owner.is_none_or(|owner| owner == client),
            "foreign System4 SACT owner"
        );
        if self.initialized {
            self.calls += 1;
            return Ok(1);
        }
        // Font initialization in the reference actually loads two faces. A
        // provider's mere presence must not stand in for published font data.
        let library = host.font_library(client)?;
        for alias in &self.font_aliases {
            let resolved = library
                .resolve_alias(alias)?
                .with_context(|| format!("System4 font `{alias}` is not published"))?;
            ensure!(
                library.is_loaded(&resolved)?,
                "System4 resolved font `{alias}` is unavailable"
            );
        }
        host.text_measurement(client)?;
        // Runtime owns the already constructed device-free channel mixer. No
        // device or prepared voice is fabricated to acknowledge initialization.
        host.audio_playing(client, 0)?;
        let mut sprites = crate::sprites::Sprites::new(client, self.canvas)?;
        if let Some(path) = &self.initial_window {
            sprites.install_window_data(self.canvas, path.clone());
        }
        let [w, h] = self.canvas;
        let existing_window = host.query_engine(client, EngineQuery::Layer(16))?;
        ensure!(
            existing_window.len() <= 1024 * 1024,
            "native WindowSurface query budget"
        );
        let existing_window: serde_json::Value = serde_json::from_slice(&existing_window)?;
        let mut layers = vec![
            json!({"op":"put","layer":{"id":BACK_SCENE,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":false}}),
            json!({"op":"put","layer":{"id":1,"parent":BACK_SCENE,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[-2147483648,-2147483648]}}),
        ];
        // Logo and other native libraries share the original WindowSurface.
        // SACT initialization must not erase their already presented pixels.
        if existing_window.is_null() {
            layers.push(json!({"op":"put","layer":{"id":16,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":false}}));
        }
        let batch = json!({"canvases":[
            {"op":"create","id":1,"width":w,"height":h},
            {"op":"clear","id":1,"color":[0,0,0,255]}
        ],"layers":layers});
        host.submit_engine(client, 1, &serde_json::to_vec(&batch)?)?;
        self.owner = Some(client);
        self.sprites = Some(sprites);
        self.font_ready = true;
        self.audio_ready = true;
        self.initialized = true;
        self.calls += 1;
        Ok(1)
    }
}

// Pixel/semantic tests deliberately bypass font and device initialization. They
// must still install the real scene hierarchy that Sact::init owns, rather than
// setting initialized=true and submitting children of a nonexistent parent.
#[cfg(all(test, feature = "runtime"))]
pub(crate) fn install_scene_fixture(host: &mut dyn HostServices, canvas: [u32; 2]) {
    let [w, h] = canvas;
    host.submit_engine(ClientId(1), 1, &serde_json::to_vec(&json!({"layers":[
        {"op":"put","layer":{"id":BACK_SCENE,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":false}},
        {"op":"put","layer":{"id":1,"parent":BACK_SCENE,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[-2147483648,-2147483648]}}
    ]})).unwrap()).unwrap();
}
