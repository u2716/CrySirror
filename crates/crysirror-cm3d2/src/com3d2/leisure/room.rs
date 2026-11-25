// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::unity_scene::UnityFiles;
use crysirror_unity::{
    classes::{Mesh, SubMesh},
    gltf::GlbBuilder,
};
const CREATION: &str = "/Canvas CreativeRoomUI Base Setting Windows/Window Room Creation";
const SLOTS: &str = "/Canvas CreativeRoomUI Save Load";
const INFO: &str = "/parent info room/";
const BUTTONS: &str = "/Canvas CreativeRoomUI Buttons";
#[derive(Default)]
pub(super) struct Room {
    file_mode: Option<bool>,
    headers: Vec<Option<Header>>,
    size: [usize; 3],
    unit: f32,
    png: Vec<u8>,
}
impl Room {
    fn model(&self) -> Result<(Vec<u8>, usize)> {
        let [nx, ny, nz] = self.size;
        let mut mesh = Mesh::default();
        let mut quad = |origin: [f32; 3], u: [f32; 3], v: [f32; 3]| {
            let i = mesh.positions.len() as u16;
            for (x, y) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
                mesh.positions.push(std::array::from_fn(|k| {
                    (origin[k] + x * u[k] + y * v[k]) * self.unit
                }));
                mesh.uv0.push([x, y]);
            }
            mesh.indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
        };
        for x in 0..nx {
            for z in 0..nz {
                let (a, b) = (x as f32 - 0.5, z as f32 - 0.5);
                quad([a, 0., b], [0., 0., 1.], [1., 0., 0.]);
                quad([a, ny as f32, b], [1., 0., 0.], [0., 0., 1.]);
                for y in 0..ny {
                    let h = y as f32;
                    if x == 0 {
                        quad([a, h, b], [0., 1., 0.], [0., 0., 1.]);
                    }
                    if x + 1 == nx {
                        quad([a + 1., h, b], [0., 0., 1.], [0., 1., 0.]);
                    }
                    if z == 0 {
                        quad([a, h, b], [1., 0., 0.], [0., 1., 0.]);
                    }
                    if z + 1 == nz {
                        quad([a, h, b + 1.], [0., 1., 0.], [1., 0., 0.]);
                    }
                }
            }
        }
        let count = mesh.indices.len() / 6;
        mesh.submeshes.push(SubMesh {
            first_byte: 0,
            index_count: mesh.indices.len() as u32,
            topology: 0,
            first_vertex: 0,
            vertex_count: mesh.positions.len() as u32,
        });
        let mut glb = GlbBuilder::new("COM3D2 CreativeRoom");
        let tex = glb.image("room panel", || Ok((self.png.clone(), "image/png")))?;
        let mat = glb.material(
            "RoomPanel",
            crysirror_asset::pbr::PbrMaterial {
                base_color_texture: Some(tex),
                ..Default::default()
            },
        );
        let node = glb.node(
            "Parent Tile Object",
            None,
            [0.; 3],
            [0., 0., 0., 1.],
            [1.; 3],
        )?;
        glb.mesh(node, &mesh, &[mat])?;
        Ok((glb.finish()?, count))
    }
}
impl Game {
    pub(super) fn open_room(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<(String, (Vec<u8>, usize))> {
        let scene = CreativeRoom.name();
        let canvas = self
            .assets
            .filtered_canvas(op, host, c, scene, "Canvas uGUI", |p| {
                p.is_empty()
                    || CREATION.starts_with(p)
                    || p.starts_with(CREATION)
                    || BUTTONS.starts_with(p)
                    || p.starts_with(BUTTONS)
                    || p.starts_with(SLOTS)
            })?;
        self.leisure.base = canvas.base;
        self.adv_camera = ([0., 1.8, 0.], 7., [45., 20.]);
        self.sound
            .play(host, c, &self.files, Kind::Bgm, "BGM020.ogg", 0.5, true)?;
        let texture = {
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let s = &mut vm.runtime.host_mut().state.retail;
            let t = s.table("my_room_texture_panel_list.nei")?;
            let mut texture = None;
            for r in 1..t.rows {
                if t.exists(0, r) && s.enabled("my_room_texture_panel_enabled_list", t.int(0, r))? {
                    let address = format!("SceneCreativeRoom/Debug/Textures/{}", t.string(2, r));
                    if let Some(key) = self.assets.unity.resource_of(
                        op,
                        &address,
                        crysirror_unity::class_id::TEXTURE_2D,
                    )? {
                        texture = Some(key);
                        break;
                    }
                }
            }
            texture.context("no enabled room texture in installed Resources")?
        };
        let (_, bytes) = self.assets.unity.bytes(op, &texture)?;
        let img = crysirror_unity::texture::decode_56(&bytes, |name| {
            ensure!(
                !name.contains(['/', '\\']) && name.ends_with(".resS"),
                "invalid room texture stream"
            );
            op.open(name)
        })?;
        self.leisure.room = Room {
            size: [1, 2, 1],
            unit: 1.,
            png: crysirror_image::encode_png(&img)?,
            ..Default::default()
        };
        self.room_widgets();
        Ok(("CreativeRoom".into(), self.leisure.room.model()?))
    }
    fn room_widgets(&mut self) {
        self.widgets = self
            .leisure
            .base
            .iter()
            .filter(|w| !w.path.starts_with(SLOTS) || self.leisure.room.file_mode.is_some())
            .cloned()
            .collect();
        let r = &self.leisure.room;
        for w in &mut self.widgets {
            if let Some(mode) = r.file_mode
                && w.path.starts_with(SLOTS)
            {
                if let Some(slot) = slot_number(&w.path) {
                    let header = r.headers.get(slot).and_then(Option::as_ref);
                    if w.path.ends_with("/Button Element") && (mode || header.is_some()) {
                        w.collider = Some(w.rect);
                    }
                    if let Some(l) = &mut w.label {
                        if w.path.ends_with("/Text Date") {
                            l.set_text(
                                &header
                                    .and_then(|h| {
                                        chrono::NaiveDateTime::parse_from_str(&h.time, "%Y%m%d%H%M")
                                            .ok()
                                    })
                                    .map(|t| t.format("%Y.%m.%d %H:%M").to_string())
                                    .unwrap_or_default(),
                            );
                        }
                        if w.path.ends_with("/InputField Room Name/Text") {
                            l.set_text(header.map_or("", |h| &h.comment));
                        }
                    }
                } else if w.path.ends_with("/Text Title")
                    && let Some(l) = &mut w.label
                {
                    l.set_text(if mode { "セーブ" } else { "ロード" });
                }
            }
            if w.path.ends_with("/Button Save")
                || w.path.ends_with("/Button Load")
                || w.path.ends_with("/Button Quit")
                || w.path.contains(INFO) && w.path.ends_with("/HitArea")
            {
                w.collider = Some(w.rect);
            }
            if let Some(l) = w.label.as_mut() {
                for (i, (axis, title)) in [("X", "横幅"), ("Y", "高さ"), ("Z", "奥行")]
                    .iter()
                    .enumerate()
                {
                    if w.path.ends_with(&format!("/Text Size{axis}")) {
                        l.set_text(&format!("{title}：{}m", r.size[i] as f32 * r.unit));
                    }
                    if w.path.ends_with(&format!("/InputField {axis}/Text")) {
                        l.set_text(&r.size[i].to_string());
                    }
                }
                if w.path.ends_with("/InputField Unit Size/Text") {
                    l.set_text(&r.unit.to_string());
                }
            }
        }
    }
    pub(super) fn room_click(
        &mut self,
        path: &str,
        input: InputFrame,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        if path.ends_with("/Button Save") || path.ends_with("/Button Load") {
            let save = path.ends_with("/Button Save");
            let r = &mut self.leisure.room;
            r.file_mode = if r.file_mode == Some(save) {
                None
            } else {
                Some(save)
            };
            r.headers = (0..10).map(|i| read_header(op, i)).collect::<Result<_>>()?;
            self.room_widgets();
            return Ok(());
        }
        if let Some(slot) = slot_number(path)
            && path.ends_with("/Button Element")
        {
            let save = self
                .leisure
                .room
                .file_mode
                .context("room slot panel closed")?;
            return self.leisure_prompt(
                op,
                host,
                c,
                now,
                if save {
                    "上書きしますか？"
                } else {
                    "ロードしますか？"
                },
                "okcancel",
                Response::RoomFile(slot, save),
            );
        }
        if path.ends_with("/Button Quit") {
            return self.leisure_prompt(
                op,
                host,
                c,
                now,
                "セーブしていないデータは失われます。\n終了しますか？",
                "yesno",
                Response::Exit,
            );
        }
        if path.ends_with("/HitArea") {
            let w = self
                .widgets
                .iter()
                .find(|w| w.path == path)
                .context("room size slider")?;
            let t = ((input.pointer_x as f32 - w.rect[0]) / w.rect[2]).clamp(0., 1.);
            let r = &mut self.leisure.room;
            if path.contains("/Slider Size Unit Scale/") {
                r.unit = ((0.25 + t * 0.75) * 20.).round() / 20.;
            } else if let Some(i) = ["X", "Y", "Z"]
                .iter()
                .position(|axis| path.contains(&format!("/Slider Size{axis}/")))
            {
                let (lo, hi) = if i == 1 { (2., 16.) } else { (1., 32.) };
                r.size[i] = (lo + t * (hi - lo)).round() as usize;
            } else {
                return Ok(());
            }
            let model = r.model()?;
            self.replace_leisure_model("CreativeRoom", model, host, c)?;
            self.room_widgets();
        }
        Ok(())
    }
}

use crate::binio::{Reader, Writer};
use crysirror_core::path::{Mount, VfsPath};
#[derive(Clone)]
struct Header {
    comment: String,
    time: String,
    guid: String,
}
fn slot_number(path: &str) -> Option<usize> {
    let suffix = path
        .split_once("/Parent Button Area/Element")?
        .1
        .split('/')
        .next()?;
    if suffix.is_empty() {
        Some(0)
    } else {
        suffix.strip_prefix(" (")?.strip_suffix(')')?.parse().ok()
    }
}
fn file_path(slot: usize) -> Result<VfsPath> {
    ensure!(slot < 10, "invalid room slot");
    Ok(VfsPath::new(
        Mount::Save,
        &format!("MyRoom/Room_{slot:03}.room"),
    )?)
}
fn header(r: &mut Reader) -> Result<Header> {
    r.expect_str("COM3D2_MY_ROOM")?;
    r.expect_str("2")?;
    Ok(Header {
        comment: r.str()?,
        time: r.str()?,
        guid: r.str()?,
    })
}
fn read_header(op: &Opener, slot: usize) -> Result<Option<Header>> {
    let path = file_path(slot)?;
    if !op.storage.exists(&path) {
        return Ok(None);
    }
    Ok(Some(header(&mut Reader::new(&op.storage.read(&path)?))?))
}
fn tile_indices([nx, ny, nz]: [usize; 3], x: usize, y: usize, z: usize) -> [i32; 6] {
    [
        x + 1 == nx,
        x == 0,
        y + 1 == ny,
        y == 0,
        z + 1 == nz,
        z == 0,
    ]
    .map(|on| if on { 0 } else { -1 })
}
impl Game {
    pub(super) fn apply_room_file(
        &mut self,
        slot: usize,
        save: bool,
        op: &Opener,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<()> {
        let path = file_path(slot)?;
        if save {
            let h = read_header(op, slot)?;
            let guid = match h.as_ref() {
                Some(h) if !h.guid.is_empty() => h.guid.clone(),
                _ => self
                    .script
                    .tjs
                    .lock()
                    .map_err(|_| anyhow::anyhow!("TJS lock"))?
                    .runtime
                    .host_mut()
                    .state
                    .guid(),
            };
            let h = Header {
                guid,
                comment: h.map(|h| h.comment).unwrap_or_default(),
                time: chrono::Local::now().format("%Y%m%d%H%M").to_string(),
            };
            let r = &self.leisure.room;
            let mut w = Writer::default();
            for s in ["COM3D2_MY_ROOM", "2", &h.comment, &h.time, &h.guid] {
                w.str(s);
            }
            for n in r.size {
                w.i32(n.try_into()?);
            }
            w.f32(r.unit);
            for x in 0..r.size[0] {
                for y in 0..r.size[1] {
                    for z in 0..r.size[2] {
                        for mat in tile_indices(r.size, x, y, z) {
                            w.str(if mat < 0 { "-1" } else { "0" });
                        }
                    }
                }
            }
            w.i32(0);
            op.storage.write_atomic(&path, &w.bytes)?;
            self.leisure.room.headers[slot] = Some(h);
        } else {
            let bytes = op.storage.read(&path)?;
            let mut r = Reader::new(&bytes);
            header(&mut r)?;
            let size = [r.count()?, r.count()?, r.count()?];
            let unit = r.f32()?;
            ensure!(
                (1..=32).contains(&size[0])
                    && (2..=16).contains(&size[1])
                    && (1..=32).contains(&size[2])
                    && (0.25..=1.).contains(&unit),
                "invalid saved room dimensions"
            );
            for x in 0..size[0] {
                for y in 0..size[1] {
                    for z in 0..size[2] {
                        for mat in tile_indices(size, x, y, z) {
                            ensure!(
                                r.str()?.parse::<i32>()? == mat,
                                "custom room panel overrides require material and topology restoration"
                            );
                        }
                    }
                }
            }
            ensure!(
                r.i32()? == 0 && r.at == bytes.len(),
                "room contains deployed objects or an unknown extension"
            );
            let room = Room {
                size,
                unit,
                png: self.leisure.room.png.clone(),
                ..Default::default()
            };
            let model = room.model()?;
            self.replace_leisure_model("CreativeRoom", model, host, c)?;
            self.leisure.room = room;
        }
        self.room_widgets();
        Ok(())
    }
}
