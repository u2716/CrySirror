// SPDX-License-Identifier: MPL-2.0
//! CHMode (float4): 424460 lifecycle, 424B30/424EA0 hmlist state,
//! 425270/425200 activation, and the 4A4930 rectangle/atlas table.
//! The native interpreter remains paused during this modal UI. The caller's
//! MODE_H script, rather than a second VM/thread, dispatches the returned ID.
mod gallery;
use super::*;
const ROOT: u32 = 62000;
const FIRST: u32 = 62100;
#[derive(Clone)]
struct Entry {
    key: String,
    opened: bool,
    default_opened: bool,
    result: i32,
    variants: Vec<gallery::Photo>,
}
struct Group {
    entries: Vec<Entry>,
}
pub(super) struct Recollection {
    gallery: bool,
    viewer: Option<gallery::Viewer>,
    groups: Vec<Group>,
    metadata_next: usize,
    category: usize,
    page: usize,
    selected_last: bool,
    preparing: bool,
    active: bool,
    focused: usize,
    images: BTreeMap<String, UploadedImage>,
    tints: BTreeMap<(String, u8), UploadedImage>,
    tint_job: Option<(String, u8, ImageJob)>,
    painted: BTreeMap<usize, u8>,
    layer_ids: Vec<u32>,
    fade: Option<(u64, Option<i32>)>,
}
impl Recollection {
    pub(super) fn resource_paths(&self) -> Vec<String> {
        self.images
            .values()
            .chain(self.tints.values())
            .map(|i| i.path.relative().to_owned())
            .collect()
    }

    fn new(gallery: bool) -> Self {
        Self {
            gallery,
            viewer: None,
            groups: vec![],
            metadata_next: 0,
            category: 0,
            page: 0,
            selected_last: false,
            preparing: true,
            active: false,
            focused: 255,
            images: BTreeMap::new(),
            tints: BTreeMap::new(),
            tint_job: None,
            painted: BTreeMap::new(),
            layer_ids: vec![],
            fade: None,
        }
    }
    pub(super) fn preparing(&self) -> bool {
        self.preparing
    }
    fn prefix(&self) -> &str {
        if self.gallery { "cgm" } else { "hm" }
    }
    fn list_prefix(&self) -> &str {
        if self.gallery { "cglist" } else { "hmlist" }
    }
    fn pages(&self) -> usize {
        self.groups[self.category].entries.len().div_ceil(9)
    }
    fn available(&self, id: usize) -> bool {
        match id {
            0..=9 => self.groups.len() > 1 && id < self.groups.len(),
            10..=19 => self.pages() > 1 && id - 10 < self.pages(),
            20 => true,
            21..=29 => self.page * 9 + id - 21 < self.groups[self.category].entries.len(),
            _ => false,
        }
    }
    fn source(&self, id: usize, state: u8) -> (String, [i32; 2]) {
        let row = BUTTONS[id];
        if id >= 21 {
            let entry = &self.groups[self.category].entries[self.page * 9 + id - 21];
            if !entry.opened {
                return (format!("{}_no.abm", self.prefix()), [0, 0]);
            }
            return ("@thumbnails".into(), [row[4], row[5]]);
        }
        let selected = id == self.category || (id >= 10 && id < 20 && id - 10 == self.page);
        let cell = if selected { 2 } else { state as usize };
        (
            format!("{}_cp.abm", self.prefix()),
            [row[4 + cell * 2], row[5 + cell * 2]],
        )
    }
}
fn parse_list(bytes: &[u8]) -> Result<Group> {
    ensure!(
        bytes.len() >= 4 && bytes.len() <= 48 * 256 + 4,
        "SugarPot H list budget"
    );
    let count = bytes[bytes.len() - 4] as usize; //424B30 explicitly reads the low byte.
    ensure!(
        (1..=90).contains(&count) && bytes.len() >= count * 48 + 4,
        "SugarPot H list record count"
    );
    let mut entries = Vec::with_capacity(count);
    for row in bytes[..count * 48].chunks_exact(48) {
        let end = row[..32]
            .iter()
            .position(|b| *b == 0)
            .context("SugarPot H unlock-key terminator")?;
        let key = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&row[..end])
            .context("SugarPot H unlock-key CP932")?
            .into_owned();
        ensure!(!key.is_empty(), "SugarPot H empty unlock key");
        entries.push(Entry {
            key,
            opened: i32::from_le_bytes(row[32..36].try_into()?) == 1,
            default_opened: i32::from_le_bytes(row[32..36].try_into()?) == 1,
            result: i32::from_le_bytes(row[44..48].try_into()?),
            variants: vec![],
        });
    }
    Ok(Group { entries })
}
impl SugarPotVm {
    pub(super) fn recollection(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let mut mode = self
            .recollection
            .take()
            .unwrap_or_else(|| Recollection::new(false));
        if !mode.active {
            if !mode.selected_last {
                mode.category = 0;
                mode.page = 0;
            }
            mode.preparing = true;
            mode.active = true;
            mode.fade = None;
            mode.focused = 255;
        }
        let result = self.prepare_recollection(h, &mut mode);
        if result.as_ref().is_ok_and(|ready| *ready) {
            self.wait = Some(Wait::Recollection);
        }
        self.recollection = Some(mode);
        result
    }
    fn prepare_recollection(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut Recollection,
    ) -> Result<bool> {
        //Resource priority is the original SCR bank for raw hmlist DATs, then
        //the GRP bank for picture resources. Do not flatten the archive namespaces.
        while mode.metadata_next < 10 {
            let name = format!("scr/{}{:02}.dat", mode.list_prefix(), mode.metadata_next);
            let Some(path) = self.config.assets.get(&name).cloned() else {
                ensure!(
                    !(mode.metadata_next + 1..10).any(|index| self
                        .config
                        .assets
                        .contains_key(&format!("scr/{}{index:02}.dat", mode.list_prefix()))),
                    "SugarPot H list category gap"
                );
                break;
            };
            let Some(bytes) = self.read(&path)? else {
                return Ok(false);
            };
            mode.groups.push(if mode.gallery {
                gallery::parse(&bytes)?
            } else {
                parse_list(&bytes)?
            });
            mode.metadata_next += 1;
        }
        ensure!(
            !mode.groups.is_empty() && mode.category < mode.groups.len(),
            "SugarPot H list category"
        );
        for group in &mut mode.groups {
            for entry in &mut group.entries {
                //424EA0 tests global node existence, not its scalar value.
                if mode.gallery {
                    for photo in &mut entry.variants {
                        photo.opened = photo.default_opened
                            || self
                                .globals
                                .keys()
                                .any(|key| key.eq_ignore_ascii_case(&photo.key));
                    }
                    entry.opened = entry.variants.iter().any(|photo| photo.opened);
                    continue;
                }
                entry.opened = entry.default_opened
                    || self
                        .globals
                        .keys()
                        .any(|key| key.eq_ignore_ascii_case(&entry.key));
            }
        }
        ensure!(
            mode.page < mode.pages(),
            "SugarPot H page outside native list"
        );
        for name in [
            format!("{}_bk{}.bmp", mode.prefix(), mode.category),
            format!("{}_cp.abm", mode.prefix()),
            format!("{}{}{:02}.bmp", mode.prefix(), mode.category, mode.page),
            format!("{}_no.abm", mode.prefix()),
        ] {
            let Some(image) = self.image(h, &name)? else {
                return Ok(false);
            };
            self.declared_images.insert(image.path.relative().into());
            mode.images.insert(name, image);
        }
        if !self.album_tints(h, mode)? {
            return Ok(false);
        }
        self.paint_recollection(h, mode)?;
        //Only the current page's tint resources are kept. Retire old generated
        //leases after their layers have been replaced by this completed page.
        let atlas = format!("{}{}{:02}.bmp", mode.prefix(), mode.category, mode.page);
        let locked = format!("{}_no.abm", mode.prefix());
        let retired = mode
            .tints
            .keys()
            .filter(|(name, _)| *name != atlas && *name != locked)
            .cloned()
            .collect::<Vec<_>>();
        for key in retired {
            if let Some(image) = mode.tints.remove(&key) {
                h.release_image_resource(self.client, image.path.relative())?;
            }
        }
        mode.preparing = false;
        Ok(true)
    }
    fn album_tints(&mut self, h: &mut dyn HostServices, mode: &mut Recollection) -> Result<bool> {
        let names = [
            format!("{}{}{:02}.bmp", mode.prefix(), mode.category, mode.page),
            format!("{}_no.abm", mode.prefix()),
        ];
        for name in names {
            for state in 0..2u8 {
                if mode.tints.contains_key(&(name.clone(), state)) {
                    continue;
                }
                if let Some((pending, cell, job)) = &mut mode.tint_job {
                    ensure!(
                        *pending == name && *cell == state,
                        "SugarPot album tint continuation"
                    );
                    match job.poll()? {
                        ImageProcessState::Pending => return Ok(false),
                        ImageProcessState::Failed(e) => bail!("SugarPot album tint worker: {e}"),
                        ImageProcessState::Ready(resource) => {
                            let image =
                                h.publish_image_resource(self.client, self.next_image, resource)?;
                            self.next_image = self
                                .next_image
                                .checked_add(1)
                                .context("SugarPot image ID exhausted")?;
                            if let Some(source) = mode.images.get(&name) {
                                if let Some(depth) =
                                    self.depths.get(source.path.relative()).copied()
                                {
                                    self.depths.insert(image.path.relative().into(), depth);
                                }
                            }
                            mode.tints.insert((name.clone(), state), image);
                            mode.tint_job = None;
                            continue;
                        }
                    }
                }
                let source = &mode.images[&name];
                let resource = h
                    .published_image_resource(self.client, source.path.relative())?
                    .context("SugarPot album tint source")?;
                //426830 passes packed black with alpha128(normal),64(hover),
                //0(press). 417F50's unsigned-word delta is exactly floor(S*w/256).
                let weight = if state == 0 { 128 } else { 192 };
                let effect = crysirror_image::effects::ImageEffect::RgbLookup {
                    spec: crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed(
                        [0; 3],
                        weight,
                        256 - weight,
                        8,
                    )?,
                };
                mode.tint_job = Some((
                    name.clone(),
                    state,
                    h.image_metadata(self.client)?.begin_process(
                        ImageInput::Resource(resource),
                        serde_json::to_vec(&vec![effect])?.into(),
                    )?,
                ));
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn recollection_icon(&self, mode: &Recollection, id: usize, state: u8) -> Result<Json> {
        let (name, source) = mode.source(id, state);
        let name = if name == "@thumbnails" {
            format!("{}{}{:02}.bmp", mode.prefix(), mode.category, mode.page)
        } else {
            name
        };
        let image = if id >= 21 && state < 2 {
            mode.tints
                .get(&(name.clone(), state))
                .context("SugarPot album native tint not prepared")?
        } else {
            mode.images
                .get(&name)
                .context("SugarPot album image not prepared")?
        };
        let row = BUTTONS[id];
        ensure!(
            source[0] >= 0
                && source[1] >= 0
                && source[0] + row[2] <= image.size[0] as i32
                && source[1] + row[3] <= image.size[1] as i32,
            "SugarPot H native atlas rectangle"
        );
        Ok(
            json!({"op":"put","layer":{"id":ROOT+1+id as u32,"rect":[row[0],row[1],row[2],row[3]],"order":[10001,id],"blend":self.asset_image_blend(image),"sampling":{"linear":false},
            "uv":[source[0] as f32/image.size[0] as f32,source[1] as f32/image.size[1] as f32,
                (source[0]+row[2]) as f32/image.size[0] as f32,(source[1]+row[3]) as f32/image.size[1] as f32],
            "paint":{"kind":"image","asset":image.path.relative()}}}),
        )
    }
    fn paint_recollection(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut Recollection,
    ) -> Result<()> {
        let background = &mode.images[&format!("{}_bk{}.bmp", mode.prefix(), mode.category)];
        ensure!(
            background.size == [1024, 600],
            "SugarPot H background native dimensions"
        );
        let mut layers = mode
            .layer_ids
            .iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut ui = vec![
            json!({"op":"remove","id":ROOT}),
            json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":10000,
            "kind":"panel","color":[0,0,0,0],"modal":true,"key_event_scope":true,"keyboard_navigation":false,
            "key_events":[{"buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"event":"h_cancel"},
                {"buttons":buttons::UP,"event":"h_up"},{"buttons":buttons::DOWN,"event":"h_down"},
                {"buttons":buttons::LEFT,"event":"h_left"},{"buttons":buttons::RIGHT,"event":"h_right"},
                {"buttons":buttons::ENTER|buttons::SPACE,"event":"h_activate"}]}}),
        ];
        layers.push(json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1024,600],"order":[10000,0],"blend":self.asset_image_blend(background),"sampling":{"linear":false},"paint":{"kind":"image","asset":background.path.relative()}}}));
        mode.layer_ids = vec![ROOT];
        mode.painted.clear();
        for id in 0..30 {
            if !mode.available(id) {
                continue;
            }
            layers.push(self.recollection_icon(mode, id, 0)?);
            let row = BUTTONS[id];
            ui.push(json!({"op":"put","widget":{"id":FIRST+id as u32,"parent":ROOT,"rect":[row[0],row[1],row[2],row[3]],
                "z":id,"kind":"button","interaction_only":true,"focus_follows_pointer":true,"keyboard_navigation":false,"keyboard_activation":false}}));
            mode.layer_ids.push(ROOT + 1 + id as u32);
            if mode.gallery && id >= 21 {
                let entry = &mode.groups[mode.category].entries[mode.page * 9 + id - 21];
                if entry.opened {
                    let text = format!(
                        "{}/{}",
                        entry.variants.iter().filter(|p| p.opened).count(),
                        entry.variants.len()
                    );
                    //41DC30 places 9px ASCII cells four pixels from the right.
                    let x = row[0] + row[2] - 9 * text.len() as i32 - 4;
                    let glyphs = text
                        .chars()
                        .enumerate()
                        .map(|(i, c)| json!({"character":c,"baseline":[x+i as i32*9,row[1]+18]}))
                        .collect::<Vec<_>>();
                    let layer = ROOT + 100 + id as u32;
                    layers.push(json!({"op":"put","layer":{"id":layer,"rect":[0,0,1024,600],"order":[10002,id],"paint":{"kind":"glyph_run","font":self.config.font,"size":16,"color":[255,255,255,255],"glyphs":glyphs}}}));
                    mode.layer_ids.push(layer);
                }
            }
            mode.painted.insert(id, 0);
        }
        self.submit(h, layers, ui)?;
        Ok(())
    }
    fn h_direction(mode: &Recollection, id: usize, direction: &str) -> Option<usize> {
        //4255C0/425730/4258A0/4259E0 move between the original category,
        //page, exit and 3x3 thumbnail regions. Availability bounds each target.
        if id == 255 {
            return mode.available(21).then_some(21);
        }
        let candidate = match direction {
            "h_up" => match id {
                1..=9 => id - 1,
                10..=20 => (21..30).rev().find(|i| mode.available(*i)).unwrap_or(20),
                24..=29 => id - 3,
                _ => return None,
            },
            "h_down" => match id {
                0..=8 => id + 1,
                10..=19 => 20,
                21..=26 => id + 3,
                27..=29 => {
                    if mode.pages() > 1 {
                        10 + mode.page
                    } else {
                        20
                    }
                }
                _ => return None,
            },
            "h_left" => match id {
                11..=19 => id - 1,
                21..=29 if (id - 21) % 3 != 0 => id - 1,
                21..=29 => mode.category,
                _ => return None,
            },
            "h_right" => match id {
                0..=9 => 21,
                10..=18 => id + 1,
                21..=29 if (id - 21) % 3 != 2 => id + 1,
                _ => return None,
            },
            _ => return None,
        };
        mode.available(candidate).then_some(candidate)
    }
    pub(super) fn poll_recollection(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut mode = self
            .recollection
            .take()
            .context("SugarPot H wait without mode")?;
        let result = self.poll_recollection_inner(h, input, &mut mode);
        self.recollection = Some(mode);
        result
    }
    fn poll_recollection_inner(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut Recollection,
    ) -> Result<bool> {
        if mode.viewer.is_some() {
            return self.poll_gallery_view(h, input, mode);
        }
        if mode.preparing {
            self.prepare_recollection(h, mode)?;
            return Ok(false);
        }
        if let Some((deadline, result)) = mode.fade {
            if h.now_ms() < deadline {
                if result.is_some() {
                    //426AD0 changes the native attenuation percentage before
                    //the public dB/gain conversion on each 15ms fade sample.
                    let elapsed = 105u64.saturating_sub(deadline.saturating_sub(h.now_ms()));
                    let phase = (31 + 32 * (elapsed / 15)).min(255) as i32;
                    let base = self.settings.header[5];
                    ensure!((0..=100).contains(&base), "SugarPot H BGM attenuation");
                    let attenuation = (base + phase * (100 - base) / 255).min(100);
                    let db = if attenuation == 100 {
                        -10000
                    } else {
                        (-(attenuation * attenuation) as f64 * 0.300000011920929) as i32
                    };
                    let gain = crysirror_audio::levels::gain_from_legacy_db100(db)?;
                    if h.audio_info(self.client, 0)?.is_some() {
                        h.submit_audio(
                            self.client,
                            AudioRequest::SetGain {
                                channel: 0,
                                gain,
                                fade_ms: 0.,
                            },
                        )?;
                    }
                }
                return Ok(false);
            }
            if result.is_some() {
                self.stop_audio(h, 0, 0)?;
                self.last_music.clear();
                self.native_audio.bgm_mode = 0;
                self.native_audio.bgm.clear();
            }
            let mut layers = mode
                .layer_ids
                .iter()
                .map(|id| json!({"op":"remove","id":id}))
                .collect::<Vec<_>>();
            layers.push(json!({"op":"remove","id":ROOT+40}));
            self.submit(h, layers, vec![json!({"op":"remove","id":ROOT})])?;
            self.stop_audio(h, 2, 0)?;
            if !mode.gallery {
                self.machine.as_mut().unwrap().set_reg(result.unwrap_or(-1));
                self.history.clear();
                self.last_voice_suffix.clear();
                self.text.clear();
                self.native_text.clear();
                self.text_remainder.clear();
                self.novel_body.clear();
                self.text_glyphs = 0;
                self.text_published = false;
            }
            mode.layer_ids.clear();
            mode.painted.clear();
            mode.active = false;
            mode.selected_last = result.is_some();
            mode.fade = None;
            input.buttons_pressed = 0;
            //The native parent resumes MODE_H after float4. It owns init_var,
            //H_MODE_FLAG and the subsequent ^gsb, including return to this menu.
            return Ok(true);
        }
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let mut activation = None;
        let mut cancel = input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0;
        let mut navigation = None;
        let mut layers = Vec::new();
        for id in 0..30 {
            if !mode.available(id) {
                continue;
            }
            let state: Json = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiInteraction(FIRST + id as u32))?,
            )?;
            let active = state["hovered"] == true || state["focused"] == true;
            if active {
                mode.focused = id;
            }
            let cell = if state["pressed"] == true {
                2
            } else if active {
                1
            } else {
                0
            };
            if mode.painted.get(&id) != Some(&cell) {
                layers.push(self.recollection_icon(mode, id, cell)?);
                mode.painted.insert(id, cell);
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
                "h_cancel" => cancel = true,
                "h_activate" => {
                    if mode.available(mode.focused) {
                        activation = Some(mode.focused)
                    }
                }
                key @ ("h_up" | "h_down" | "h_left" | "h_right") => {
                    navigation = Self::h_direction(mode, mode.focused, key)
                }
                _ => {}
            }
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        if let Some(id) = navigation {
            let row = BUTTONS[id];
            mode.focused = id;
            self.submit(
                h,
                vec![],
                vec![
                    json!({"op":"focus","id":FIRST+id as u32}),
                    json!({"op":"warp_pointer","position":[row[0]+row[2]/2,row[1]+row[3]/2]}),
                ],
            )?;
        }
        if cancel || activation == Some(20) {
            return self.close_recollection(h, mode, None);
        }
        if let Some(id) = activation {
            match id {
                0..=9 if id != mode.category => {
                    mode.category = id;
                    mode.page = 0;
                    mode.preparing = true;
                }
                10..=19 if id - 10 != mode.page => {
                    mode.page = id - 10;
                    mode.preparing = true;
                }
                21..=29 => {
                    let entry = &mode.groups[mode.category].entries[mode.page * 9 + id - 21];
                    if entry.opened {
                        if mode.gallery {
                            mode.viewer = Some(gallery::Viewer::new(entry)?);
                            mode.preparing = true;
                            return Ok(false);
                        }
                        return self.close_recollection(h, mode, Some(entry.result));
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }
    fn close_recollection(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut Recollection,
        result: Option<i32>,
    ) -> Result<bool> {
        let duration = if self.settings.header[4] != 0 {
            0u64
        } else {
            105
        };
        let mut layers = vec![
            json!({"op":"put","layer":{"id":ROOT+40,"rect":[0,0,1024,600],"order":[10003,0],"paint":{"kind":"solid","color":[0,0,0,255]}}}),
        ];
        if duration > 0 {
            layers.push(json!({"op":"transition","transition":{"effect":{"kind":"crossfade"},"duration_ms":duration}}));
        }
        self.submit(h, layers, vec![])?;
        mode.fade = Some((h.now_ms().saturating_add(duration), result));
        Ok(false)
    }
}
//Native engine UI geometry/UV policy; pictures remain user-owned GRP resources.
pub(super) const BUTTONS: [[i32; 10]; 30] = [
    [0, 78, 160, 64, 0, 0, 160, 0, 320, 0],
    [0, 152, 160, 64, 0, 64, 160, 64, 320, 64],
    [0, 226, 160, 64, 0, 128, 160, 128, 320, 128],
    [0, 300, 160, 64, 0, 192, 160, 192, 320, 192],
    [0, 374, 160, 64, 0, 256, 160, 256, 320, 256],
    [6, 365, 184, 48, 0, 240, 184, 240, 368, 240],
    [6, 423, 184, 48, 0, 288, 184, 288, 368, 288],
    [577, 465, 72, 104, 504, 0, 504, 104, 504, 208],
    [655, 465, 72, 104, 576, 0, 576, 104, 576, 208],
    [733, 465, 72, 104, 648, 0, 648, 104, 648, 208],
    [344, 486, 48, 48, 0, 320, 0, 368, 0, 416],
    [400, 486, 48, 48, 48, 320, 48, 368, 48, 416],
    [460, 486, 48, 48, 96, 320, 96, 368, 96, 416],
    [523, 486, 48, 48, 144, 320, 144, 368, 144, 416],
    [580, 486, 48, 48, 192, 320, 192, 368, 192, 416],
    [821, 486, 48, 48, 448, 320, 448, 392, 448, 448],
    [885, 486, 48, 48, 512, 320, 512, 392, 512, 448],
    [949, 486, 48, 48, 576, 320, 576, 392, 576, 448],
    [1013, 486, 48, 48, 640, 320, 640, 392, 640, 448],
    [1077, 486, 48, 48, 704, 320, 704, 392, 704, 448],
    [436, 556, 160, 40, 240, 320, 240, 360, 240, 400],
    [173, 86, 192, 113, 0, 0, 0, 0, 0, 0],
    [388, 86, 192, 113, 192, 0, 192, 0, 192, 0],
    [603, 86, 192, 113, 384, 0, 384, 0, 384, 0],
    [173, 222, 192, 113, 0, 113, 0, 113, 0, 113],
    [388, 222, 192, 113, 192, 113, 192, 113, 192, 113],
    [603, 222, 192, 113, 384, 113, 384, 113, 384, 113],
    [173, 358, 192, 113, 0, 226, 0, 226, 0, 226],
    [388, 358, 192, 113, 192, 226, 192, 226, 192, 226],
    [603, 358, 192, 113, 384, 226, 384, 226, 384, 226],
];
