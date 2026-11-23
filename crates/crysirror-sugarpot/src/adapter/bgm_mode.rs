// SPDX-License-Identifier: MPL-2.0
//! CBGMMode: 418C60/418DC0 lifecycle, 419DB0 atlas state, 419FA0
//! transport and 419290..4195A0 pointer navigation. The menu uses the native
//! eighteen fixed tracks, not the CG/H unlock tables. Raster/palette/audio
//! execution belongs to the shared services; this module retains menu policy.
use super::*;
use crysirror_core::image_processing::ImageResource;
const ROOT: u32 = 61000;
const FIRST: u32 = 61100;
const OVERLAY: u32 = ROOT + 50;

pub(super) struct BgmMode {
    images: Vec<UploadedImage>,
    old: Option<Arc<dyn ImageResource>>,
    menu: Option<Arc<dyn ImageResource>>,
    capture: Option<ImageJob>,
    palette: Option<ImageJob>,
    overlay: Option<UploadedImage>,
    // 0 old->black, 1 black->menu, 2 active, 3 menu->black,
    // 4 black->old. Each palette sample owns a worker and its image lease.
    stage: u8,
    weight: u32,
    deadline: u64,
    focused: usize,
    selected: Option<usize>,
    play_mode: u8,
    playing: bool,
    pending_track: Option<usize>,
    resume_music: String,
    restoring_music: bool,
    painted: [u8; 22],
}
impl BgmMode {
    pub(super) fn preparing(&self) -> bool {
        self.images.len() != 3 || self.capture.is_some() || self.palette.is_some()
    }
    pub(super) fn resource_paths(&self) -> Vec<String> {
        self.overlay
            .iter()
            .map(|i| i.path.relative().to_owned())
            .collect()
    }
}
// 4A6CC8, fourteen DWORDs per record. Last four fields are the selected
// control's fourth atlas state and two zero native text-decoration fields.
fn row(id: usize) -> [i32; 12] {
    match id {
        0 => [299, 471, 112, 48, 224, 0, 224, 48, 224, 96, 224, 96],
        1 => [456, 471, 112, 48, 112, 0, 112, 48, 112, 96, 112, 96],
        2 => [613, 471, 112, 48, 0, 0, 0, 48, 0, 96, 0, 96],
        3 => [436, 556, 160, 40, 336, 0, 336, 40, 336, 80, 336, 0],
        _ => {
            let index = id - 4;
            let x = if index < 9 { 0 } else { 296 };
            let y = (index % 9) as i32 * 32;
            [
                if index < 9 { 160 } else { 568 },
                83 + (index % 9) as i32 * 42,
                296,
                32,
                x,
                y,
                x,
                y + 288,
                x,
                y + 576,
                x,
                y,
            ]
        }
    }
}
// Preserve the native table's deterministic (occasionally surprising)
// eight-row navigation constants. Its down path from the last record would
// dereference the adjacent string table as a rectangle: bound that native
// out-of-range bug to a stationary pointer rather than reading code/data.
fn direction(id: usize, key: &str) -> Option<usize> {
    let result = match key {
        "bgm_up" => match id {
            0..=2 => 11,
            3 => 0,
            4 | 12 => return None,
            5..=21 => id - 1,
            _ => 4,
        },
        "bgm_down" => match id {
            0..=2 => 3,
            3 => return None,
            11 | 19 => 3,
            4..=21 => id + 1,
            _ => 4,
        },
        "bgm_left" => match id {
            0 | 3 => return None,
            1 => 0,
            2 => 1,
            4..=11 => return None,
            12..=21 => id - 8,
            _ => 4,
        },
        "bgm_right" => match id {
            0 => 1,
            1 => 2,
            2 | 3 => return None,
            4..=11 => id + 8,
            12..=21 => return None,
            _ => 4,
        },
        _ => return None,
    };
    (result < 22).then_some(result)
}
impl SugarPotVm {
    pub(super) fn bgm_mode(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if self.bgm_mode.is_none() {
            //453BD0 saves VAR.bgm, rather than the currently playing menu
            //transport or the last displayed text. The return call uses mode0
            //(looping) even if that native variable was originally one-shot.
            let resume_music = match self.machine.as_ref().unwrap().state.variables.get("bgm") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Integer(i)) if *i > 1 => i.to_string(),
                _ => String::new(),
            };
            self.bgm_mode = Some(BgmMode {
                images: vec![],
                old: None,
                menu: None,
                capture: None,
                palette: None,
                overlay: None,
                stage: 0,
                weight: 224,
                deadline: 0,
                focused: 255,
                selected: None,
                play_mode: 0,
                playing: false,
                pending_track: None,
                resume_music,
                restoring_music: false,
                painted: [255; 22],
            });
        }
        let mut mode = self.bgm_mode.take().unwrap();
        let result = self.prepare_bgm_mode(h, &mut mode);
        if result.as_ref().is_ok_and(|ready| *ready) {
            self.wait = Some(Wait::BgmMode);
        }
        self.bgm_mode = Some(mode);
        result
    }
    fn prepare_bgm_mode(&mut self, h: &mut dyn HostServices, mode: &mut BgmMode) -> Result<bool> {
        const NAMES: [&str; 3] = ["bgmm_bk.bmp", "bgmm_cp.abm", "bgmm000.abm"];
        while mode.images.len() < NAMES.len() {
            let Some(image) = self.image(h, NAMES[mode.images.len()])? else {
                return Ok(false);
            };
            self.declared_images
                .insert(image.path.relative().to_owned());
            mode.images.push(image);
        }
        ensure!(
            mode.images[0].size == [1024, 600]
                && mode.images[1].size == [496, 144]
                && mode.images[2].size == [592, 864],
            "SugarPot BGM native sheet dimensions"
        );
        if mode.old.is_none() {
            if mode.capture.is_none() {
                mode.capture = Some(h.begin_layer_prefix_image_over(
                    self.client,
                    [1024, 600],
                    [9999, 0],
                    [0, 0, 0, 255],
                )?);
            }
            match mode.capture.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot BGM caller capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    mode.old = Some(resource);
                    mode.capture = None;
                }
            }
        }
        self.submit(h,vec![],vec![json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],
            "z":10000,"kind":"panel","color":[0,0,0,0],"modal":true,"key_event_scope":true,
            "keyboard_navigation":false,"key_events":[
                {"buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"event":"bgm_close"},
                {"buttons":buttons::UP,"event":"bgm_up"},{"buttons":buttons::DOWN,"event":"bgm_down"},
                {"buttons":buttons::LEFT,"event":"bgm_left"},{"buttons":buttons::RIGHT,"event":"bgm_right"},
                {"buttons":buttons::ENTER|buttons::SPACE,"event":"bgm_activate"}]}})])?;
        Ok(true)
    }
    fn bgm_icon(&self, mode: &BgmMode, id: usize, cell: u8) -> Result<Json> {
        let r = row(id);
        let image = &mode.images[if id < 4 { 1 } else { 2 }];
        let sx = r[4 + usize::from(cell) * 2];
        let sy = r[5 + usize::from(cell) * 2];
        ensure!(
            sx >= 0
                && sy >= 0
                && sx + r[2] <= image.size[0] as i32
                && sy + r[3] <= image.size[1] as i32,
            "SugarPot BGM atlas bounds"
        );
        Ok(
            json!({"op":"put","layer":{"id":ROOT+1+id as u32,"rect":[r[0],r[1],r[2],r[3]],
            "order":[9001,id],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
            "uv":[sx as f32/image.size[0]as f32,sy as f32/image.size[1]as f32,
                (sx+r[2])as f32/image.size[0]as f32,(sy+r[3])as f32/image.size[1]as f32],
            "paint":{"kind":"image","asset":image.path.relative()}}}),
        )
    }
    fn paint_bgm_mode(&mut self, h: &mut dyn HostServices, mode: &mut BgmMode) -> Result<()> {
        let image = &mode.images[0];
        let mut layers = vec![json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1024,600],
            "order":[9000,0],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
            "paint":{"kind":"image","asset":image.path.relative()}}})];
        let mut ui = vec![];
        for id in 0..22 {
            layers.push(self.bgm_icon(mode, id, 0)?);
            mode.painted[id] = 0;
            let r = row(id);
            ui.push(json!({"op":"put","widget":{"id":FIRST+id as u32,"parent":ROOT,
                "rect":[r[0],r[1],r[2],r[3]],"z":id,"kind":"button","interaction_only":true,
                "enabled":false,"focus_follows_pointer":true,"keyboard_navigation":false,"keyboard_activation":false}}));
        }
        self.submit(h, layers, ui)
    }
    fn bgm_overlay(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut BgmMode,
        resource: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let image = h.publish_image_resource(self.client, self.next_image, resource)?;
        self.next_image = self
            .next_image
            .checked_add(1)
            .context("SugarPot BGM image ID exhausted")?;
        self.submit(
            h,
            vec![
                json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],
            "order":[10000,0],"paint":{"kind":"image","asset":image.path.relative()}}}),
            ],
            vec![],
        )?;
        if let Some(old) = mode.overlay.replace(image) {
            h.release_image_resource(self.client, old.path.relative())?;
        }
        Ok(())
    }
    fn bgm_attenuate(&mut self, h: &mut dyn HostServices, phase: i32) -> Result<()> {
        if h.audio_info(self.client, 0)?.is_some() {
            //41A020 computes the percentage before the native nonlinear dB
            //conversion. A shared linear gain fade would change this curve.
            let base = self.settings.header[5];
            ensure!(
                (0..=100).contains(&base),
                "SugarPot BGM attenuation setting"
            );
            let p = (base + phase * (100 - base) / 255).min(100);
            let db = if p == 100 {
                -10000
            } else {
                (-(p * p) as f64 * 0.300000011920929) as i32
            };
            h.submit_audio(
                self.client,
                AudioRequest::SetGain {
                    channel: 0,
                    gain: crysirror_audio::levels::gain_from_legacy_db100(db)?,
                    fade_ms: 0.,
                },
            )?;
        }
        Ok(())
    }
    pub(super) fn poll_bgm_mode(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut mode = self
            .bgm_mode
            .take()
            .context("SugarPot BGM wait without native mode")?;
        let result = self.bgm_mode_step(h, input, &mut mode);
        if !result.as_ref().is_ok_and(|done| *done) {
            self.bgm_mode = Some(mode);
        }
        result
    }
    fn bgm_mode_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut BgmMode,
    ) -> Result<bool> {
        if mode.stage != 2 {
            return self.bgm_palette(h, input, mode);
        }
        if let Some(track) = mode.pending_track {
            let name = format!("bgm{:02}", track + 1);
            if !self.play_audio(h, &name, 0, PcmBus::Music, mode.play_mode == 2)? {
                return Ok(false);
            }
            mode.pending_track = None;
            mode.playing = self.settings.header[1] == 1;
            self.last_music = name.clone();
            self.last_music_loop = mode.play_mode == 2;
            self.native_audio.bgm = name;
            self.native_audio.bgm_mode = if mode.play_mode == 2 { 2 } else { 1 };
        }
        if self.settings.header[1] != 1 && mode.selected.is_some() {
            self.stop_audio(h, 0, 0)?;
            mode.selected = None;
            mode.playing = false;
            mode.play_mode = 0;
            mode.painted = [255; 22];
        }
        if mode.playing
            && !h
                .audio_info(self.client, 0)?
                .is_some_and(|info| info.source_playing)
        {
            if mode.play_mode == 1 {
                let next = (mode.selected.unwrap() + 1) % 18;
                mode.selected = Some(next);
                mode.pending_track = Some(next);
                mode.playing = false;
            } else if mode.play_mode == 0 {
                self.stop_audio(h, 0, 0)?;
                mode.selected = None;
                mode.playing = false;
            }
            mode.painted = [255; 22];
        }
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let mut activation = None;
        let mut close = input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0;
        let mut navigation = None;
        let mut layers = vec![];
        for id in 0..22 {
            let state: Json = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiInteraction(FIRST + id as u32))?,
            )?;
            let active = state["hovered"] == true || state["focused"] == true;
            if active {
                mode.focused = id;
            }
            let selected = (id < 2 && mode.play_mode == id as u8 + 1)
                || (id >= 4 && mode.selected == Some(id - 4));
            let cell = if selected || state["pressed"] == true {
                2
            } else if active {
                1
            } else {
                0
            };
            if mode.painted[id] != cell {
                layers.push(self.bgm_icon(mode, id, cell)?);
                mode.painted[id] = cell;
            }
            if state["activated"] == true {
                activation = Some(id);
            }
        }
        for event in events {
            if event["widget"]["owner"] != self.client.0 {
                continue;
            }
            match event["kind"].as_str().unwrap_or("") {
                "bgm_close" => close = true,
                "bgm_activate" if mode.focused < 22 => activation = Some(mode.focused),
                key @ ("bgm_up" | "bgm_down" | "bgm_left" | "bgm_right") => {
                    navigation = direction(mode.focused, key)
                }
                _ => {}
            }
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        if let Some(id) = navigation {
            let r = row(id);
            mode.focused = id;
            self.submit(
                h,
                vec![],
                vec![
                    json!({"op":"focus","id":FIRST+id as u32}),
                    json!({"op":"warp_pointer","position":[r[0]+r[2]/2,r[1]+r[3]/2]}),
                ],
            )?;
        }
        if close || activation == Some(3) {
            //419A00 snapshots the current menu, attenuates the selected track,
            //then restores VAR.bgm at black before fading the caller back in.
            mode.capture = Some(h.begin_layer_prefix_image_over(
                self.client,
                [1024, 600],
                [9999, 0],
                [0, 0, 0, 255],
            )?);
            mode.stage = 3;
            mode.weight = 224;
            mode.deadline = 0;
            input.buttons_pressed = 0;
            self.submit(
                h,
                vec![],
                (0..22)
                    .map(|id| json!({"op":"set_enabled","id":FIRST+id,"enabled":false}))
                    .collect(),
            )?;
        } else if let Some(id) = activation {
            match id {
                0 | 1 => {
                    let requested = id as u8 + 1;
                    mode.play_mode = if mode.play_mode == requested {
                        0
                    } else {
                        requested
                    };
                    mode.painted = [255; 22];
                    if mode.playing {
                        h.submit_audio(
                            self.client,
                            AudioRequest::SetLooped {
                                channel: 0,
                                looping: mode.play_mode == 2,
                                loop_start: None,
                            },
                        )?;
                        self.last_music_loop = mode.play_mode == 2;
                        self.native_audio.bgm_mode = if mode.play_mode == 2 { 2 } else { 1 };
                    }
                }
                2 => {
                    self.stop_audio(h, 0, 0)?;
                    mode.selected = None;
                    mode.playing = false;
                    mode.painted = [255; 22];
                }
                4..=21 => {
                    mode.selected = Some(id - 4);
                    mode.pending_track = Some(id - 4);
                    mode.playing = false;
                    mode.painted = [255; 22];
                }
                _ => {}
            }
        }
        Ok(false)
    }
    fn bgm_palette(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut BgmMode,
    ) -> Result<bool> {
        //429030's normal control dispatcher is suspended by the native fade
        //loops. Consume modal events here so a click used to skip a palette
        //cannot later activate a track after the menu becomes interactive.
        let _: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        //Capture the final menu before the closing palette starts. It is not
        //a hosted legacy framebuffer: Layer owns both immutable snapshots.
        if mode.stage == 3 && mode.capture.is_some() {
            match mode.capture.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot BGM menu capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    mode.menu = Some(resource);
                    mode.capture = None;
                }
            }
        }
        if let Some(job) = &mut mode.palette {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot BGM palette worker: {e}"),
                ImageProcessState::Ready(resource) => {
                    mode.palette = None;
                    self.bgm_overlay(h, mode, resource)?;
                    mode.deadline = h.now_ms().saturating_add(15);
                    mode.weight = if mode.stage == 0 || mode.stage == 3 {
                        mode.weight.saturating_sub(32)
                    } else {
                        mode.weight + 32
                    };
                    return Ok(false);
                }
            }
        }
        if self.settings.header[4] != 0 || input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            mode.weight = if mode.stage == 0 || mode.stage == 3 {
                0
            } else {
                256
            };
            input.buttons_pressed = 0;
        }
        if h.now_ms() < mode.deadline {
            return Ok(false);
        }
        if (mode.stage == 0 || mode.stage == 3) && mode.weight == 0 {
            self.submit(
                h,
                vec![
                    json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],
                "order":[10000,0],"paint":{"kind":"solid","color":[0,0,0,255]}}}),
                ],
                vec![],
            )?;
            if mode.stage == 0 {
                self.stop_audio(h, 0, 0)?;
                self.paint_bgm_mode(h, mode)?;
                if mode.capture.is_none() {
                    mode.capture = Some(h.begin_layer_prefix_image_over(
                        self.client,
                        [1024, 600],
                        [9999, 0],
                        [0, 0, 0, 255],
                    )?);
                }
                match mode.capture.as_mut().unwrap().poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot BGM initial menu capture: {e}"),
                    ImageProcessState::Ready(resource) => {
                        mode.menu = Some(resource);
                        mode.capture = None;
                    }
                }
            } else {
                if !mode.restoring_music {
                    self.stop_audio(h, 0, 0)?;
                    mode.restoring_music = true;
                }
                let source = mode.resume_music.clone();
                if !source.is_empty() && !self.play_audio(h, &source, 0, PcmBus::Music, true)? {
                    return Ok(false);
                }
                self.last_music = source.clone();
                self.last_music_loop = true;
                self.native_audio.bgm = source;
                self.native_audio.bgm_mode = if self.native_audio.bgm.is_empty() {
                    0
                } else {
                    2
                };
                self.submit(
                    h,
                    (ROOT..=ROOT + 22)
                        .map(|id| json!({"op":"remove","id":id}))
                        .collect(),
                    vec![],
                )?;
            }
            mode.stage += 1;
            mode.weight = 32;
        }
        if (mode.stage == 1 || mode.stage == 4) && mode.weight >= 256 {
            if mode.stage == 1 {
                self.submit(
                    h,
                    vec![json!({"op":"remove","id":OVERLAY})],
                    (0..22)
                        .map(|id| json!({"op":"set_enabled","id":FIRST+id,"enabled":true}))
                        .collect(),
                )?;
                mode.stage = 2;
                if let Some(image) = mode.overlay.take() {
                    h.release_image_resource(self.client, image.path.relative())?;
                }
                return Ok(false);
            }
            self.submit(
                h,
                vec![json!({"op":"remove","id":OVERLAY})],
                vec![json!({"op":"remove","id":ROOT})],
            )?;
            if let Some(image) = mode.overlay.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            self.stop_audio(h, 2, 0)?;
            input.buttons_pressed = 0;
            return Ok(true);
        }
        if mode.stage == 0 || mode.stage == 3 {
            self.bgm_attenuate(h, (255 - mode.weight) as i32)?;
        }
        let resource = if mode.stage == 0 || mode.stage == 4 {
            mode.old.as_ref()
        } else {
            mode.menu.as_ref()
        }
        .context("SugarPot BGM fade source")?
        .clone();
        let lut =
            crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed([0; 3], mode.weight, 0, 8)?;
        mode.palette = Some(
            h.image_metadata(self.client)?.begin_process(
                ImageInput::Resource(resource),
                serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RgbLookup {
                    spec: lut,
                }])?
                .into(),
            )?,
        );
        Ok(false)
    }
}
