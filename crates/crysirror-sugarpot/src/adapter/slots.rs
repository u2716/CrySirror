// SPDX-License-Identifier: MPL-2.0
//! CSaveMode429B00: ten pages of ten native slots,99=file00, plus read-only
//! quick slot100=file101. Native atlas geometry is4A40F0..4A4630. Shared UI
//! owns modal focus, keyboard, text editing and pointer capture.
use super::*;
use crate::native_storage::slot::Slot;
const ROOT: u32 = 64500;
const FIRST: u32 = 64550;
const MEMO: u32 = 64650;
#[derive(Clone)]
struct Entry {
    slot: Slot,
    modified: Option<i64>,
}
pub(super) struct SlotMenu {
    pub(super) resume: Option<Box<Wait>>,
    load: bool,
    catalog: Option<Task<Vec<Option<Entry>>>>,
    entries: Vec<Option<Entry>>,
    page: usize,
    focus: usize,
    background: Option<UploadedImage>,
    atlas: Option<UploadedImage>,
    active: bool,
    dirty: bool,
    painted: BTreeMap<usize, u32>,
    layers: Vec<u32>,
    pub(super) memo: String,
    preview_slot: Option<usize>,
    preview_read: Option<Task<Option<Vec<u8>>>>,
    preview_job: Option<ImageJob>,
    preview: Option<UploadedImage>,
}
impl SlotMenu {
    fn available(&self, index: usize) -> bool {
        match index {
            0..=9 => !self.load || self.entries[index + 10 * self.page].is_some(),
            10..=20 => true,
            23 => self.load && self.entries[100].is_some(),
            _ => false,
        }
    }
}
fn rect(index: usize) -> [i32; 4] {
    match index {
        0..=9 => [547, 62 + 49 * index as i32, 456, 48],
        10..=19 => [590 + 41 * (index - 10) as i32, 2, 40, 48],
        20 => [436, 556, 152, 40],
        23 => [37, 430, 456, 48],
        _ => unreachable!(),
    }
}
fn atlas(index: usize, cell: u32) -> [i32; 2] {
    match index {
        0..=9 | 23 => [0, 48 * cell as i32],
        10..=19 => [40 * (index - 10) as i32, 144 + 48 * cell as i32],
        20 => [400, 144 + 40 * cell as i32],
        _ => unreachable!(),
    }
}
impl SugarPotVm {
    pub(super) fn slot_menu_resources(&self) -> Vec<String> {
        self.slot_menu
            .as_ref()
            .and_then(|menu| menu.preview.as_ref())
            .map(|image| vec![image.path.relative().into()])
            .unwrap_or_default()
    }
    pub(super) fn slot_menu_source_wait(&self) -> Option<&Wait> {
        self.slot_menu
            .as_ref()
            .and_then(|menu| menu.resume.as_deref())
    }
    pub(super) fn refresh_slot_after_save(&mut self, index: usize, slot: Slot) {
        if let Some(menu) = &mut self.slot_menu {
            if let Some(entry) = menu.entries.get_mut(index) {
                *entry = Some(Entry {
                    slot,
                    modified: None,
                });
            }
            menu.dirty = true;
        }
    }
    pub(super) fn start_slot_menu(&mut self, load: bool) -> Result<()> {
        ensure!(
            self.slot_menu.is_none(),
            "SugarPot overlapping native slot menu"
        );
        let storage = self.storage.as_ref().unwrap().clone();
        let catalog = self.pool.as_ref().unwrap().spawn(move |cancel| {
            let mut entries = Vec::with_capacity(101);
            for index in 0..=100 {
                cancel.check()?;
                let path = crate::native_storage::slot_path(index)?;
                let meta = storage.metadata(Mount::Save, path.relative())?;
                let entry = if let Some(bytes) =
                    crate::native_storage::read_file(storage.clone(), &path, &cancel)?
                {
                    ensure!(bytes.len() <= 0x10000, "SugarPot slot body64KiB limit");
                    Some(Entry {
                        slot: Slot::decode(&bytes)?,
                        modified: meta.and_then(|m| m.modified_ms),
                    })
                } else {
                    None
                };
                entries.push(entry);
            }
            Ok(entries)
        })?;
        let resume = self.wait.take().map(Box::new);
        self.slot_menu = Some(SlotMenu {
            resume,
            load,
            catalog: Some(catalog),
            entries: vec![],
            page: 0,
            focus: 255,
            background: None,
            atlas: None,
            active: false,
            dirty: false,
            painted: BTreeMap::new(),
            layers: vec![],
            memo: String::new(),
            preview_slot: None,
            preview_read: None,
            preview_job: None,
            preview: None,
        });
        self.wait = Some(Wait::Slots);
        Ok(())
    }
    fn slot_preview(&mut self, h: &mut dyn HostServices, menu: &mut SlotMenu) -> Result<()> {
        let target = match menu.focus {
            0..=9 => Some(menu.page * 10 + menu.focus),
            23 => Some(100),
            _ => menu.preview_slot,
        };
        if target != menu.preview_slot {
            menu.preview_slot = target;
            menu.preview_read = None;
            menu.preview_job = None;
            if let Some(slot) = target {
                let path = crate::native_storage::slot_path(slot as i32)?;
                let path = VfsPath::new(
                    Mount::Save,
                    &format!("{}c.dat", path.relative().strip_suffix(".dat").unwrap()),
                )?;
                let storage = self.storage.as_ref().unwrap().clone();
                menu.preview_read = Some(self.pool.as_ref().unwrap().spawn(move |cancel| {
                    crate::native_storage::read_file(storage, &path, &cancel)
                })?);
            }
        }
        if let Some(read) = &mut menu.preview_read {
            let Some(bytes) = read.try_take()? else {
                return Ok(());
            };
            menu.preview_read = None;
            if let Some(bytes) = bytes {
                ensure!(
                    bytes.len() == 512 * 300 * 4,
                    "SugarPot native thumbnail raw512x300 BGRA layout"
                );
                menu.preview_job = Some(h.image_metadata(self.client)?.begin_packed_pixels(
                    crysirror_core::image_processing::PackedImageSpec {
                        size: [512, 300],
                        layout: crysirror_core::image_processing::PackedPixelLayout::Bgrx8,
                        bytes: bytes.into(),
                    },
                )?);
            } else {
                if let Some(old) = menu.preview.take() {
                    self.submit(h, vec![json!({"op":"remove","id":ROOT+30})], vec![])?;
                    h.release_image_resource(self.client, old.path.relative())?;
                    menu.layers.retain(|id| *id != ROOT + 30);
                    self.published_layers.retain(|id| *id != ROOT + 30);
                }
            }
        }
        if let Some(job) = &mut menu.preview_job {
            let resource = match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot native slot preview: {e}"),
                ImageProcessState::Ready(resource) => resource,
            };
            let image = h.publish_image_resource(self.client, self.next_image, resource)?;
            self.next_image = self
                .next_image
                .checked_add(1)
                .context("SugarPot image IDs")?;
            self.submit(h,vec![json!({"op":"put","layer":{"id":ROOT+30,"rect":[12,70,512,300],"order":[10002,0],"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
            if let Some(old) = menu.preview.replace(image) {
                h.release_image_resource(self.client, old.path.relative())?;
            }
            menu.preview_job = None;
            if !menu.layers.contains(&(ROOT + 30)) {
                menu.layers.push(ROOT + 30);
                self.published_layers.push(ROOT + 30);
            }
        }
        Ok(())
    }
    fn slot_icon(&self, menu: &SlotMenu, index: usize, state: u32) -> Result<Json> {
        let image = menu.atlas.as_ref().unwrap();
        let cell = if (10..20).contains(&index) && menu.page == index - 10 {
            2
        } else {
            state
        };
        let src = atlas(index, cell);
        let r = rect(index);
        ensure!(
            src[0] >= 0
                && src[1] >= 0
                && src[0] + r[2] <= image.size[0] as i32
                && src[1] + r[3] <= image.size[1] as i32,
            "SugarPot slot atlas bounds"
        );
        Ok(
            json!({"op":"put","layer":{"id":ROOT+1+index as u32,"rect":r,"order":[10001,index],"blend":self.asset_image_blend(image),"sampling":{"linear":false},"uv":[src[0] as f32/image.size[0] as f32,src[1] as f32/image.size[1] as f32,(src[0]+r[2]) as f32/image.size[0] as f32,(src[1]+r[3]) as f32/image.size[1] as f32],"paint":{"kind":"image","asset":image.path.relative()}}}),
        )
    }
    fn paint_slot_menu(&mut self, h: &mut dyn HostServices, menu: &mut SlotMenu) -> Result<()> {
        let background = menu.background.as_ref().unwrap();
        ensure!(
            background.size == [1024, 600],
            "SugarPot slot background dimensions"
        );
        let mut layers = menu
            .layers
            .iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut ui = if menu.active {
            vec![json!({"op":"remove","id":ROOT})]
        } else {
            vec![]
        };
        ui.push(json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":10000,"kind":"panel","color":[0,0,0,0],"modal":true}}));
        layers.push(json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1024,600],"order":[10000,0],"blend":self.asset_image_blend(background),"sampling":{"linear":false},"paint":{"kind":"image","asset":background.path.relative()}}}));
        menu.layers = vec![ROOT];
        menu.painted.clear();
        for index in (0..=20).chain([23]) {
            if !menu.available(index) {
                continue;
            }
            layers.push(self.slot_icon(menu, index, 0)?);
            menu.layers.push(ROOT + 1 + index as u32);
            let r = rect(index);
            ui.push(json!({"op":"put","widget":{"id":FIRST+index as u32,"parent":ROOT,"rect":r,"z":index,"kind":"button","interaction_only":true,"focus_follows_pointer":true}}));
            menu.painted.insert(index, 0);
            if index < 10 || index == 23 {
                let slot = if index == 23 {
                    100
                } else {
                    menu.page * 10 + index
                };
                let label = if slot == 100 {
                    "QSave:".to_owned()
                } else {
                    format!("No.{:02}", if slot == 99 { 0 } else { slot + 1 })
                };
                let text = menu.entries[slot].as_ref().map_or_else(
                    || format!("{label}　----------"),
                    |entry| format!("{label}　{}\n{}", entry.slot.title, entry.slot.memo),
                );
                ui.push(json!({"op":"put","widget":{"id":64700+index as u32,"parent":ROOT,"rect":[r[0]+6,r[1]+3,r[2]-12,r[3]-6],"z":100+index,"kind":"label","font":self.config.font,"text_size":16,"text":text,"text_color":[255,255,255,255],"color":[0,0,0,0]}}));
            }
        }
        if !menu.load {
            ui.push(json!({"op":"put","widget":{"id":MEMO,"parent":ROOT,"rect":[40,500,460,32],"z":200,"kind":"text_field","text":menu.memo,"font":self.config.font,"text_size":18,"max_length":31}}));
        }
        if let Some(image) = &menu.preview {
            layers.push(json!({"op":"put","layer":{"id":ROOT+30,"rect":[12,70,512,300],"order":[10002,0],"paint":{"kind":"image","asset":image.path.relative()}}}));
            menu.layers.push(ROOT + 30);
        }
        self.submit(h, layers, ui)?;
        self.published_layers
            .retain(|id| !(*id >= ROOT && *id <= ROOT + 30));
        self.published_layers.extend(menu.layers.iter().copied());
        if !self.published_ui.contains(&ROOT) {
            self.published_ui.push(ROOT);
        }
        menu.active = true;
        Ok(())
    }
    pub(super) fn poll_slot_menu(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut menu = self
            .slot_menu
            .take()
            .context("SugarPot missing slot menu continuation")?;
        let mut child = None;
        let done = (|| -> Result<bool> {
            if let Some(catalog) = &mut menu.catalog {
                let Some(entries) = catalog.try_take()? else {
                    return Ok(false);
                };
                menu.entries = entries;
                menu.catalog = None;
                //42A920 chooses the newest ordinary native file, excluding
                //quick101. Missing host timestamps retain the default page0.
                if let Some((index, _)) = menu.entries[..100]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, e)| e.as_ref()?.modified.map(|time| (index, time)))
                    .max_by_key(|(_, time)| *time)
                {
                    menu.page = index / 10;
                }
            }
            let prefix = if menu.load { "load" } else { "save" };
            if menu.background.is_none() {
                let Some(image) = self.image(h, &format!("{prefix}_bk.bmp"))? else {
                    return Ok(false);
                };
                self.declared_images.insert(image.path.relative().into());
                menu.background = Some(image);
            }
            if menu.atlas.is_none() {
                let Some(image) = self.image(h, &format!("{prefix}_cp.abm"))? else {
                    return Ok(false);
                };
                self.declared_images.insert(image.path.relative().into());
                menu.atlas = Some(image);
            }
            if !menu.active || menu.dirty {
                self.paint_slot_menu(h, &mut menu)?;
                menu.dirty = false;
                input.buttons_pressed = 0;
                return Ok(false);
            }
            if input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0 {
                input.buttons_pressed = 0;
                return Ok(true);
            }
            let mut action = None;
            let mut layers = Vec::new();
            for index in (0..=20).chain([23]) {
                if !menu.available(index) {
                    continue;
                }
                let state: Json = serde_json::from_slice(&h.query_engine(
                    self.client,
                    EngineQuery::UiInteraction(FIRST + index as u32),
                )?)?;
                let cell = if state["pressed"] == true {
                    2
                } else if state["hovered"] == true || state["focused"] == true {
                    1
                } else {
                    0
                };
                if menu.painted.get(&index) != Some(&cell) {
                    layers.push(self.slot_icon(&menu, index, cell)?);
                    menu.painted.insert(index, cell);
                }
                if state["hovered"] == true || state["focused"] == true {
                    menu.focus = index;
                }
                if state["activated"] == true {
                    action = Some(index);
                }
            }
            if !layers.is_empty() {
                self.submit(h, layers, vec![])?;
            }
            if !menu.load {
                let widget: Json = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::UiWidget(MEMO))?,
                )?;
                let text = widget["text"]
                    .as_str()
                    .context("SugarPot save memo widget text")?;
                let mut bounded = String::new();
                for ch in text.chars() {
                    let next = format!("{bounded}{ch}");
                    let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(&next);
                    ensure!(!invalid, "SugarPot save memo is not CP932");
                    if bytes.len() > 31 {
                        break;
                    }
                    bounded = next;
                }
                if bounded != text {
                    self.submit(h,vec![],vec![json!({"op":"set_text","id":MEMO,"text":bounded,"font":self.config.font})])?;
                }
                menu.memo = bounded;
            }
            self.slot_preview(h, &mut menu)?;
            if let Some(index) = action {
                input.buttons_pressed = 0;
                match index {
                    10..=19 => {
                        menu.page = index - 10;
                        self.paint_slot_menu(h, &mut menu)?;
                    }
                    20 => return Ok(true),
                    0..=9 | 23 => {
                        let slot = if index == 23 {
                            100
                        } else {
                            menu.page * 10 + index
                        };
                        //Keep menu/wait until cancellation or full completion.
                        //No menu activation writes reg; successful load replaces
                        //this VM through the engine-owned coordinator.
                        child = Some((slot, menu.load));
                        return Ok(false);
                    }
                    _ => unreachable!(),
                }
            }
            Ok(false)
        })();
        //Install the native menu before starting a save/load child so its
        //suspended source wait remains available to the native slot writer.
        match done {
            Ok(true) => {
                self.submit(
                    h,
                    menu.layers
                        .iter()
                        .map(|id| json!({"op":"remove","id":id}))
                        .collect(),
                    vec![json!({"op":"remove","id":ROOT})],
                )?;
                self.published_layers.retain(|id| !menu.layers.contains(id));
                self.published_ui.retain(|id| *id != ROOT);
                if let Some(image) = menu.preview.take() {
                    h.release_image_resource(self.client, image.path.relative())?;
                }
                self.wait = menu.resume.map(|wait| *wait);
                Ok(true)
            }
            Ok(false) => {
                self.slot_menu = Some(menu);
                if let Some((slot, load)) = child {
                    if load {
                        self.start_native_load(slot as i32, true, false)?;
                    } else {
                        self.start_native_save(h, slot as i32, false)?;
                    }
                }
                Ok(false)
            }
            Err(error) => {
                self.slot_menu = Some(menu);
                Err(error)
            }
        }
    }
}
