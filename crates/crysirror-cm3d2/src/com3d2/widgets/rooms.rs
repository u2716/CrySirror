// SPDX-License-Identifier: MPL-2.0
//! ScheduleFacillityMainPanel's serialized uGUI room grid (Unity 5.6).
use super::*;
use crate::tjs::retail::operations::paid::Room;

const GRID: &str = "/Window Facility Details/Parent Facility Button";
const DETAIL: &str = "/Window Facility Details/Facility Info";
const CARD: &str = "/temp Image Facility Info";

pub(super) fn anchored(
    [x, y, w, h]: [f32; 4],
    [ax, ay, bx, by, dx, dy, dw, dh, px, py]: [f32; 10],
) -> [f32; 4] {
    let width = w * (bx - ax) + dw;
    let height = h * (by - ay) + dh;
    [
        x + w * (ax + (bx - ax) * px) + dx - width * px,
        y + h * (1. - ay - (by - ay) * py) - dy - height * (1. - py),
        width,
        height,
    ]
}

impl Assets {
    pub(super) fn room_sprite(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        w: &mut Widget,
        sprite: ObjectKey,
    ) -> Result<()> {
        let (key, [x, y, sw, sh]) = self.sprite(op, sprite)?;
        let (asset, tw, th) = self.texture(op, host, client, key)?;
        w.asset = asset;
        w.uv = [
            x / tw as f32,
            1. - (y + sh) / th as f32,
            (x + sw) / tw as f32,
            1. - y / th as f32,
        ];
        Ok(())
    }

    pub(in crate::com3d2) fn facility_thumbnail(
        &mut self,
        op: &Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        w: &mut Widget,
        name: &str,
    ) -> Result<()> {
        let mut op = Opener {
            storage: op.storage.clone(),
            dir: op.dir.clone(),
        };
        let address = format!("SceneFacilityPowerUp/Sprites/{name}");
        let sprite = self
            .unity
            .resource_of(&mut op, &address, c::SPRITE)?
            .with_context(|| format!("missing facility thumbnail {name}"))?;
        self.room_sprite(&mut op, host, client, w, sprite)
    }

    pub(in crate::com3d2) fn room_list(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        style: &Label,
        rooms: &[Room],
        portrait: Option<&str>,
    ) -> Result<Vec<Widget>> {
        let file = self.scene_file("SceneDaily")?;
        let sf = self.unity.file(op, &file)?;
        let mut root = None;
        for obj in sf.objects.iter().filter(|o| o.class_id == c::GAME_OBJECT) {
            let bytes = sf.bytes(obj)?;
            let mut r = Reader::new(&bytes);
            let n = r.count(12)?;
            if n == 0 {
                continue;
            }
            let tr = r.pptr64()?.path_id;
            r.take((n - 1) * 12 + 4)?;
            if r.string()? == "ScheduleFacillityMainPanel" {
                root = Some(tr);
                break;
            }
        }
        let root = root.context("missing schedule facility canvas")?;
        let area = [0., 0., 1920., 1080.];
        let mut stack = vec![(root, String::new(), area)];
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        let mut card = Vec::new();
        let mut grid = None;
        let mut star_grids = std::collections::BTreeMap::new();
        let selected = rooms.iter().find(|room| room.selected_staff);
        while let Some((tr, parent, parent_rect)) = stack.pop() {
            ensure!(seen.insert(tr), "cycle in facility canvas");
            if sf.object(tr).is_none_or(|o| o.class_id != 224) {
                continue;
            }
            let bytes = sf.bytes_of(tr)?;
            let mut r = Reader::new(&bytes);
            let go = r.pptr64()?.path_id;
            r.seek(52)?;
            let n = r.count(12)?;
            let children = (0..n)
                .map(|_| r.pptr64().map(|p| p.path_id))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            r.pptr64()?;
            let mut rect = anchored(parent_rect, r.f32s::<10>()?);
            let bytes = sf.bytes_of(go)?;
            let mut r = Reader::new(&bytes);
            let n = r.count(12)?;
            let comps = (0..n)
                .map(|_| r.pptr64().map(|p| p.path_id))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            r.i32()?;
            let name = r.string()?;
            let path = format!("{parent}/{name}");
            if tr == root {
                rect = area;
            }
            if path.ends_with(CARD) {
                let (_, [width, height], _) = grid.context("facility card precedes its grid")?;
                rect = [0., 0., width, height];
            }
            if tr != root
                && !path.ends_with("/Window Facility Details")
                && !path.ends_with("/Window Facility Details/image background")
                && !path.ends_with(GRID)
                && !path.contains(CARD)
                && !(selected.is_some() && path.contains(DETAIL))
                && !path.contains("/Button OK")
            {
                continue;
            }
            if [
                "Button Change Name",
                "Image Check Mark",
                "Image Change",
                "Image Delete",
            ]
            .contains(&name.as_str())
            {
                continue;
            }
            for child in children.into_iter().rev() {
                stack.push((child, path.clone(), rect));
            }
            for k in comps.into_iter().filter(|&k| {
                sf.object(k)
                    .is_some_and(|o| o.class_id == c::MONO_BEHAVIOUR)
            }) {
                let b = sf.bytes_of(k)?;
                let mut r = Reader::new(&b);
                r.seek(16)?;
                let reference = self.reference(op, &file, &mut r)?;
                let (_, src) = self.unity.bytes(op, &reference)?;
                let class = Reader::new(&src).string()?;
                if class == "GridLayoutGroup" && path.ends_with(GRID) {
                    r.seek(32)?;
                    ensure!(
                        r.take(16)? == [0; 16] && r.i32()? == 4 && r.i32()? == 0 && r.i32()? == 0,
                        "unsupported facility grid placement"
                    );
                    let cell = r.f32s::<2>()?;
                    let gap = r.f32s::<2>()?;
                    ensure!(
                        r.i32()? == 0
                            && cell.iter().all(|v| *v > 0.)
                            && gap.iter().all(|v| *v >= 0.),
                        "invalid facility grid"
                    );
                    grid = Some((rect, cell, gap));
                    continue;
                }
                if class == "GridLayoutGroup" && name.starts_with("parent stars ") {
                    star_grids.insert(path.clone(), super::power::Grid::read(&mut r, rect, &path)?);
                    continue;
                }
                let star = name == "RawImage" && parent.contains("/parent stars ");
                let badge = path.contains("/frame maid icon/");
                if !(class == "Text"
                    || class == "Image"
                    || class == "RawImage" && (name == "Image Inactive mask" || star || badge))
                {
                    continue;
                }
                let mut w = Widget {
                    path: path.clone(),
                    rect,
                    depth: 0,
                    asset: String::new(),
                    uv: [0., 0., 1., 1.],
                    label: None,
                    tint: [255; 4],
                    component: k,
                    toggle: None,
                    button: None,
                    collider: None,
                    slices: None,
                };
                r.seek(44)?;
                w.tint = r
                    .f32s::<4>()?
                    .map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
                if class == "Text" {
                    r.seek(208)?;
                    let size = r.i32()?;
                    r.seek(228)?;
                    let anchor = r.i32()?;
                    r.seek(252)?;
                    let mut label = style.clone();
                    label.size = size.try_into()?;
                    label.align = (anchor % 3) as f32 / 2.;
                    label.vertical = (anchor / 3) as f32 / 2.;
                    label.set_text(&r.string()?);
                    w.label = Some(label);
                } else if class == "RawImage" && star {
                    r.seek(196)?;
                    let texture = self.reference(op, &file, &mut r)?;
                    w.asset = self.texture(op, host, client, texture)?.0;
                    let [x, y, width, height] = r.f32s::<4>()?;
                    w.uv = [x, 1. - y - height, x + width, 1. - y];
                } else if class == "RawImage" && name == "maid icon" {
                    if let Some(asset) = portrait {
                        w.asset = asset.to_owned();
                    }
                } else if class == "Image" && !path.ends_with(CARD) {
                    r.seek(196)?;
                    if let Some(sprite) = self.unity.resolve(op, &file, r.pptr64()?)? {
                        self.room_sprite(op, host, client, &mut w, sprite)?;
                    }
                }
                if path.contains(CARD) {
                    card.push(w);
                } else if path.contains(DETAIL) {
                    let room = selected.context("missing selected room info")?;
                    if star {
                        let grid = star_grids
                            .get(&parent)
                            .context("missing selected room rating grid")?;
                        let count = if parent.ends_with("Rank") {
                            room.rank
                        } else {
                            room.staff_rank
                        };
                        for index in 0..count {
                            let mut item = w.clone();
                            item.rect = grid.position(index, count);
                            out.push(item);
                        }
                        continue;
                    }
                    if path.ends_with("/Image Thumbnail") {
                        self.facility_thumbnail(op, host, client, &mut w, &room.thumbnail)?;
                    }
                    if let Some(label) = &mut w.label {
                        if path.ends_with("/Text Facility Name") {
                            label.set_text(&room.name);
                        }
                        if path.ends_with("/Text Number of people") {
                            label.set_text(&room.count.to_string());
                        }
                        if path.ends_with("/Text Level of facility") {
                            label.set_text(&room.level.to_string());
                        }
                        if path.ends_with("/Text Status") {
                            label.set_text(room.status);
                        }
                    }
                    out.push(w);
                } else {
                    if path.ends_with("/Button OK") {
                        w.collider = Some(rect);
                    }
                    out.push(w);
                }
            }
        }
        let (g, [w, h], [sx, sy]) = grid.context("missing FacilityInfoUI grid")?;
        // FacilityManager.FacilityCountMax is twelve for this profile. The
        // flexible, upper-left, horizontal Unity GridLayoutGroup centers it.
        let cols = ((g[2] + sx + 0.001) / (w + sx)).floor().clamp(1., 12.) as usize;
        let rows = 12usize.div_ceil(cols);
        let left = g[0] + (g[2] - (w + sx) * cols as f32 + sx) / 2.;
        let top = g[1] + (g[3] - (h + sy) * rows as f32 + sy) / 2.;
        for index in 0..12 {
            let room = rooms.iter().find(|r| r.physical_slot == index);
            for mut cell in card.iter().cloned() {
                if cell.path.ends_with("/Image Inactive mask") && room.is_some_and(|r| r.matches) {
                    continue;
                }
                if cell.path.contains("/frame maid icon") && !room.is_some_and(|r| r.selected_staff)
                {
                    continue;
                }
                if let Some((parent, "RawImage")) = cell.path.rsplit_once('/') {
                    let count = room.map_or(0, |room| {
                        if parent.ends_with("Rank") {
                            room.rank
                        } else {
                            room.staff_rank
                        }
                    });
                    let grid = star_grids
                        .get(parent)
                        .context("missing room card rating grid")?;
                    for i in 0..count {
                        let mut item = cell.clone();
                        item.rect = grid.position(i, count);
                        item.rect[0] += left + (index % cols) as f32 * (w + sx);
                        item.rect[1] += top + (index / cols) as f32 * (h + sy);
                        out.push(item);
                    }
                    continue;
                }
                cell.rect[0] += left + (index % cols) as f32 * (w + sx);
                cell.rect[1] += top + (index / cols) as f32 * (h + sy);
                if cell.path.ends_with(CARD) {
                    if let Some(room) = room {
                        let address = format!("SceneFacilityPowerUp/Sprites/{}", room.thumbnail);
                        if let Some(sprite) = self.unity.resource_of(op, &address, c::SPRITE)? {
                            self.room_sprite(op, host, client, &mut cell, sprite)?;
                        }
                        if room.matches {
                            cell.collider = Some(cell.rect);
                        }
                    } else {
                        cell.tint = [0, 0, 0, 64];
                    }
                    cell.path = room.map_or_else(||format!("room/empty-{index}"),|r|format!("room/{}",r.index));
                } else if let Some(label) = &mut cell.label {
                    if cell.path.ends_with("/InputField Facility Name/Text") {
                        label.set_text(room.map_or("空き部屋", |r| r.name.as_str()));
                    } else if cell.path.contains("/Text Number of people") {
                        label.set_text(&room.map_or("---".into(), |r| r.count.to_string()));
                    } else if cell.path.contains("/Text Number of min people") {
                        label.set_text(&room.map_or("---".into(), |r| r.minimum.to_string()));
                    } else if cell.path.contains("/Text Level of facility") {
                        label.set_text(&room.map_or("--".into(), |r| r.level.to_string()));
                    }
                }
                out.push(cell);
            }
        }
        for (i, w) in out.iter_mut().enumerate() {
            w.depth = 4000 + i as i32;
        }
        ensure!(
            out.iter()
                .any(|w| w.path.ends_with("/Button OK") && w.collider.is_some()),
            "missing facility panel close button"
        );
        Ok(out)
    }
}
