// SPDX-License-Identifier: MPL-2.0
//! CCGMode (float3), 41C180/41C580/41E030..41EFE0. Album navigation
//! shares CHMode's identical native rectangle/atlas table; variants and viewer
//! waits are separate native policy. Shared services own every decoded frame.
use super::*;
#[derive(Clone)]
pub(super) struct Photo {
    pub key: String,
    pub opened: bool,
    pub default_opened: bool,
}
pub(super) struct Viewer {
    photos: Vec<Photo>,
    index: usize,
    up: bool,
    image: Option<UploadedImage>,
    animation: Option<animation::AnimationState>,
    origin: [i32; 2],
    extent: [i32; 2],
    started: u64,
    pan_complete: bool,
    forced_pan_end: bool,
    pinned: Option<String>,
    retired: Vec<String>,
    painted: bool,
}
impl Viewer {
    pub(super) fn new(entry: &Entry) -> Result<Self> {
        let photos = entry
            .variants
            .iter()
            .filter(|photo| photo.opened)
            .cloned()
            .collect::<Vec<_>>();
        ensure!(
            !photos.is_empty(),
            "SugarPot CG group has no unlocked variant"
        );
        Ok(Self {
            photos,
            index: 0,
            up: PAN_UP.contains(&entry.result),
            image: None,
            animation: None,
            origin: [0; 2],
            extent: [0; 2],
            started: 0,
            pan_complete: false,
            forced_pan_end: false,
            pinned: None,
            retired: vec![],
            painted: false,
        })
    }
}
pub(super) fn parse(bytes: &[u8]) -> Result<Group> {
    ensure!(
        bytes.len() >= 56 && bytes.len() <= 48 * 8193 + 8,
        "SugarPot CG list byte budget"
    );
    let count = u32::from_le_bytes(bytes[bytes.len() - 8..bytes.len() - 4].try_into()?) as usize;
    let groups = u32::from_le_bytes(bytes[bytes.len() - 4..].try_into()?) as usize;
    ensure!(
        (1..=8192).contains(&count)
            && (1..=90).contains(&groups)
            && bytes.len() == (count + 1) * 48 + 8,
        "SugarPot CG list record/ordinal count"
    );
    ensure!(
        &bytes[count * 48..count * 48 + 3] == b"-1\0",
        "SugarPot CG list sentinel"
    );
    let mut entries: Vec<Entry> = vec![];
    for row in bytes[..count * 48].chunks_exact(48) {
        let end = row[..32]
            .iter()
            .position(|b| *b == 0)
            .context("SugarPot CG key terminator")?;
        let key = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&row[..end])
            .context("SugarPot CG key CP932")?
            .into_owned();
        ensure!(
            !key.is_empty() && !key.contains(['/', '\\', ':']),
            "SugarPot CG resource basename"
        );
        let group = i32::from_le_bytes(row[36..40].try_into()?);
        let opened = i32::from_le_bytes(row[32..36].try_into()?) == 1;
        if entries.last().is_none_or(|entry| entry.result != group) {
            entries.push(Entry {
                key: key.clone(),
                opened,
                default_opened: false,
                result: group,
                variants: vec![],
            });
        }
        entries.last_mut().unwrap().variants.push(Photo {
            key,
            opened,
            default_opened: opened,
        });
    }
    ensure!(
        entries.len() == groups,
        "SugarPot CG consecutive group count"
    );
    Ok(Group { entries })
}
impl SugarPotVm {
    pub(in crate::adapter) fn gallery(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let mut mode = self
            .gallery
            .take()
            .unwrap_or_else(|| Recollection::new(true));
        if !mode.active {
            //41C410 resets category, 41BFC0 resets the first page on every open.
            mode.category = 0;
            mode.page = 0;
            mode.active = true;
            mode.preparing = true;
            mode.focused = 255;
            mode.fade = None;
        }
        let result = self.prepare_recollection(h, &mut mode);
        if result.as_ref().is_ok_and(|ready| *ready) {
            self.wait = Some(Wait::Gallery);
        }
        self.gallery = Some(mode);
        result
    }
    pub(in crate::adapter) fn poll_gallery(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut mode = self
            .gallery
            .take()
            .context("SugarPot CG wait without album")?;
        let result = self.poll_recollection_inner(h, input, &mut mode);
        self.gallery = Some(mode);
        result
    }
    pub(super) fn poll_gallery_view(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut Recollection,
    ) -> Result<bool> {
        let mut view = mode.viewer.take().context("SugarPot CG viewer state")?;
        let finish = self.gallery_view(h, input, &mut view)?;
        if view.image.is_some() && view.painted {
            mode.preparing = false;
        }
        if finish {
            if let Some(path) = view.pinned.take() {
                self.declared_images.remove(&path);
            }
            self.submit(
                h,
                vec![
                    json!({"op":"remove","id":ROOT+60}),
                    json!({"op":"remove","id":ROOT+61}),
                ],
                vec![],
            )?;
            self.paint_recollection(h, mode)?;
            mode.preparing = false;
        } else {
            mode.viewer = Some(view);
        }
        Ok(false)
    }
    fn gallery_view(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        view: &mut Viewer,
    ) -> Result<bool> {
        if view.image.is_none() {
            let key = &view.photos[view.index].key;
            //41E630 explicitly prefers BMP over ABM; ordinary sprite loading
            //has a different native priority, so pass the chosen extension.
            let name = if self
                .config
                .assets
                .contains_key(&format!("grp/{}.bmp", key.to_ascii_lowercase()))
            {
                format!("{key}.bmp")
            } else {
                format!("{key}.abm")
            };
            let Some(image) = self.image(h, &name)? else {
                return Ok(false);
            };
            view.animation = self.animation_state(&name, h.now_ms())?;
            view.extent = [
                (image.size[0] as i32 - 1024).max(0),
                (image.size[1] as i32 - 600).max(0),
            ];
            view.origin = if view.up { [0, view.extent[1]] } else { [0, 0] };
            view.started = h.now_ms();
            view.pan_complete = view.extent[1] == 0;
            view.forced_pan_end = false;
            view.painted = false;
            let path = image.path.relative().to_owned();
            self.declared_images.insert(path.clone());
            if let Some(old) = view.pinned.replace(path) {
                view.retired.push(old);
            }
            view.image = Some(image);
        }
        let mut image = view.image.as_ref().unwrap().clone();
        if let Some(animation) = &view.animation {
            let cursor = animation.clock.frame(h.now_ms())? as usize;
            image = animation.asset.frames
                [animation.asset.programs[animation.selected][cursor] as usize]
                .clone();
        }
        let distance = if view.forced_pan_end || self.settings.header[4] != 0 {
            view.extent[1]
        } else {
            ((h.now_ms().saturating_sub(view.started) / 33).saturating_mul(4))
                .min(view.extent[1] as u64) as i32
        };
        let pan = if view.up {
            view.extent[1] - distance
        } else {
            distance
        };
        view.origin[1] = pan;
        view.pan_complete |= distance == view.extent[1];
        let height = image.size[1];
        let mut ui = vec![];
        if !view.painted {
            ui = vec![
                json!({"op":"remove","id":ROOT}),
                json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":10000,
                "kind":"panel","modal":true,"color":[0,0,0,0],"key_event_scope":true,"key_events":[
                    {"buttons":buttons::ENTER|buttons::SPACE|buttons::MOUSE_LEFT,"event":"cg_next"},
                    {"buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"event":"cg_back"}]}}),
            ];
        }
        self.submit(h,vec![json!({"op":"put","layer":{"id":ROOT+60,"rect":[0,0,1024,600],"order":[10004,0],"paint":{"kind":"solid","color":[0,0,0,255]}}}),
            json!({"op":"put","layer":{"id":ROOT+61,"rect":[-view.origin[0],-pan,image.size[0],height],"order":[10005,0],"paint":{"kind":"image","asset":image.path.relative()}}})],ui)?;
        view.painted = true;
        for path in view.retired.drain(..) {
            if view.pinned.as_ref() != Some(&path) {
                self.declared_images.remove(&path);
            }
        }
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let mut next = false;
        let mut back = false;
        for event in events {
            if event["widget"]["owner"] != self.client.0 {
                continue;
            }
            next |= event["kind"] == "cg_next";
            back |= event["kind"] == "cg_back";
        }
        input.buttons_pressed = 0;
        if back {
            return Ok(true);
        }
        if next {
            if !view.pan_complete {
                //41ED40 consumes the advance which skips an active pan;
                //the next independent click selects the next unlocked variant.
                view.forced_pan_end = true;
                view.pan_complete = true;
            } else {
                view.index += 1;
                if view.index == view.photos.len() {
                    return Ok(true);
                }
                view.image = None;
                view.animation = None;
                view.painted = false;
            }
        }
        Ok(false)
    }
}
//4A6308's 82 native group-ID/direction pairs all select bottom-to-top (1).
//Other groups retain direction0. Resource dimensions bound the portable pan.
const PAN_UP: &[i32] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29,
    30, 31, 32, 34, 35, 36, 87, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53,
    54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 78, 79,
    80, 81, 82, 24, 83, 84, 85, 86,
];
