// SPDX-License-Identifier: MPL-2.0
//! Level 5 facility uniform selection (CostumePartsEnabledMgr/Ctrl).
use super::*;
use crate::tjs::retail::wardrobe;
use crysirror_unity::reader::Reader;

const ROOT: &str = "UI Root/CostumePartsEnabledPanel/";
const ITEMS: &str = "UI Root/CostumePartsEnabledPanel/Contents/Buttons/";

pub(in crate::com3d2) fn item_path(path: &str) -> Option<(&str, &str)> {
    let path = path.strip_prefix(ITEMS)?;
    Some(path.split_once('/').unwrap_or((path, "")))
}
pub(in crate::com3d2) fn wanted(path: &str) -> bool {
    item_path(path).is_some()
        || path.strip_prefix(ROOT).is_some_and(|p| {
            matches!(
                p,
                "BG" | "Contents"
                    | "Contents/Buttons"
                    | "Buttons/SelectAll"
                    | "Buttons/DeselectAll"
            )
        })
}

#[derive(Default, Clone)]
pub(in crate::com3d2) struct Layout {
    pub(super) clip: [f32; 4],
    columns: usize,
    left: f32,
    pad: [f32; 2],
}
impl Layout {
    pub(in crate::com3d2) fn read(
        &mut self,
        class: &str,
        path: &str,
        pos: [f32; 3],
        body: &[u8],
    ) -> Result<()> {
        let mut r = Reader::new(body);
        match (class, path.strip_prefix(ROOT)) {
            ("UITable", Some("Contents/Buttons")) => {
                r.seek(32)?;
                self.columns = usize::try_from(r.i32()?)?;
                ensure!(
                    (1..=64).contains(&self.columns),
                    "invalid facility uniform grid"
                );
                r.seek(60)?;
                self.pad = r.f32s()?;
                self.left = 960. + pos[0];
            }
            ("UIPanel", Some("Contents")) => {
                r.seek(172)?;
                let [x, y, w, h] = r.f32s::<4>()?;
                r.seek(204)?;
                let [dx, dy] = r.f32s::<2>()?;
                self.clip = [
                    960. + pos[0] + x + dx - w / 2.,
                    540. - pos[1] - y - dy - h / 2.,
                    w,
                    h,
                ];
            }
            _ => {}
        }
        Ok(())
    }
}
pub(super) struct PartButton {
    pub(super) mpn: String,
    button: String,
    title: String,
    pub(super) active: bool,
    icons: [String; 2],
}
#[derive(Default)]
pub(super) struct Panel {
    pub(super) layout: Layout,
    pub(super) scroll: f32,
    pub(super) parts: Vec<PartButton>,
    widgets: Vec<Widget>,
}
impl Panel {
    pub(super) fn configure(&mut self, state: &mut TjsState, enabled: &[&str]) -> Result<()> {
        let mappings = state.retail.table("random_preset_mpn_map.nei")?;
        let categories = state.retail.table("edit_category_define.nei")?;
        for row in 1..categories.rows {
            let mpn = categories.string(0, row);
            if !enabled.contains(&mpn.as_str())
                || mpn == "acckamisub"
                || categories.string(2, row) != "Item"
            {
                continue;
            }
            let Some(m) = (1..mappings.rows).find(|&r| mappings.string(0, r) == mpn) else {
                continue;
            };
            let mut title = categories.string(4, row);
            if mpn == "hairt" {
                title.push_str(" &\nリボン");
            }
            self.parts.push(PartButton {
                mpn,
                button: mappings.string(1, m),
                title,
                active: false,
                icons: Default::default(),
            });
        }
        Ok(())
    }
    pub(super) fn take_widgets(&mut self, widgets: &mut Vec<Widget>, assets: &Assets) {
        self.widgets = widgets.extract_if(.., |w| wanted(&w.path)).collect();
        self.layout = assets.costume_layout.clone();
        self.parts.retain(|p| {
            self.widgets
                .iter()
                .any(|w| item_path(&w.path) == Some((&p.button, "")) && w.collider.is_some())
        });
    }
    pub(super) fn open(
        &mut self,
        state: &mut TjsState,
        id: usize,
        op: &mut Opener,
        assets: &mut Assets,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        let slot = state
            .retail
            .edit_facility
            .context("missing edited facility")?;
        let (files, storage) = state.retail.editor_files()?;
        let model = state.retail.stock_models[&id].clone();
        let room = wardrobe::settings_mut(&mut state.retail, slot)?;
        let empty = assets.costume_empty_icon(op, host, client)?;
        for p in &mut self.parts {
            p.active = room.forced(&p.mpn);
            room.set_button(&p.mpn, p.active);
            for (i, name) in [p.mpn.as_str(), "acckamisub"].into_iter().enumerate() {
                if i == 1 && p.mpn != "hairt" {
                    break;
                }
                let prop = &model.props[model
                    .prop_names
                    .iter()
                    .position(|&n| n == name)
                    .context("missing uniform model part")?];
                p.icons[i] = empty.clone();
                if !prop.temp_file.is_empty() && files.exists(&prop.temp_file) {
                    let menu = crate::menu::MenuFile::parse(
                        &files.read(storage.as_ref(), &prop.temp_file)?,
                    )?;
                    if let Some(icon) = menu.first("icons").or_else(|| menu.first("icon"))
                        && files.exists(icon)
                    {
                        p.icons[i] = assets.schedule_icon(
                            op,
                            host,
                            client,
                            &files,
                            icon.trim_end_matches(".tex"),
                        )?;
                    }
                }
            }
        }
        self.scroll = 0.;
        Ok(())
    }
    pub(super) fn toggle(
        &mut self,
        state: &mut TjsState,
        target: Option<usize>,
        active: bool,
    ) -> Result<()> {
        let slot = state
            .retail
            .edit_facility
            .context("missing edited facility")?;
        let room = wardrobe::settings_mut(&mut state.retail, slot)?;
        for (i, p) in self.parts.iter_mut().enumerate() {
            if target.is_none_or(|t| i == t) {
                room.set_button(&p.mpn, active);
                p.active = active;
            }
        }
        Ok(())
    }
    pub(super) fn draw(
        &mut self,
        out: &mut Vec<Widget>,
        actions: &mut Vec<(Action, [f32; 4])>,
    ) -> Result<()> {
        let mut cells = Vec::new();
        for (i, p) in self.parts.iter().enumerate() {
            let root = self
                .widgets
                .iter()
                .find(|w| item_path(&w.path) == Some((&p.button, "")))
                .context("uniform control")?;
            cells.push((i, root.rect));
        }
        ensure!(
            self.layout.columns > 0 && !cells.is_empty(),
            "missing facility uniform table"
        );
        // UITable preserves sibling order here. The baked rows use the same four-column order.
        cells.sort_by_key(|(_, r)| (r[1].round() as i32, r[0].round() as i32));
        let step = [
            cells[0].1[2] + 2. * self.layout.pad[0],
            cells[0].1[3] + 2. * self.layout.pad[1],
        ];
        let height = cells.len().div_ceil(self.layout.columns) as f32 * step[1];
        self.scroll = self
            .scroll
            .clamp(0., (height - self.layout.clip[3]).max(0.));
        for mut w in self.widgets.iter().cloned() {
            if let Some((button, child)) = item_path(&w.path) {
                let Some((index, (i, r))) = cells
                    .iter()
                    .enumerate()
                    .find(|(_, (i, _))| self.parts[*i].button == button)
                else {
                    continue;
                };
                let part = &self.parts[*i];
                let dx = self.layout.left
                    + self.layout.pad[0]
                    + (index % self.layout.columns) as f32 * step[0]
                    - r[0];
                let dy = self.layout.clip[1]
                    + self.layout.pad[1]
                    + (index / self.layout.columns) as f32 * step[1]
                    - self.scroll
                    - r[1];
                w.rect[0] += dx;
                w.rect[1] += dy;
                let visible = clip(w.rect, self.layout.clip);
                if visible[2] <= 0. || visible[3] <= 0. {
                    continue;
                }
                match child {
                    "" => {
                        if part.active {
                            w.tint[3] = 255;
                        }
                        if let Some(mut hit) = w.collider {
                            hit[0] += dx;
                            hit[1] += dy;
                            actions.push((Action::Force(*i), clip(hit, self.layout.clip)));
                        }
                    }
                    "Title" => {
                        if let Some(label) = &mut w.label {
                            label.set_text(&part.title);
                        }
                    }
                    "Item" => w.asset = part.icons[0].clone(),
                    "Item2" => w.asset = part.icons[1].clone(),
                    _ => continue,
                }
            } else if let Some(hit) = w.collider {
                if w.path.ends_with("/SelectAll") {
                    actions.push((Action::ForceAll(true), hit));
                }
                if w.path.ends_with("/DeselectAll") {
                    actions.push((Action::ForceAll(false), hit));
                }
            }
            w.depth += 200;
            out.push(w);
        }
        Ok(())
    }
}
