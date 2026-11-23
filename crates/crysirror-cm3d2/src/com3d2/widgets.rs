// SPDX-License-Identifier: MPL-2.0
//! Serialized NGUI widget references of the Unity 5.6 player.
use super::*;
use crate::unity_scene::{ObjectKey, UnityData, UnityFiles};
use crysirror_unity::{PPtr, class_id as c, reader::Reader};
use std::collections::HashMap;

fn ptr(r: &mut Reader<'_>) -> Result<PPtr> {
    r.pptr64()
}

#[derive(Clone)]
pub(super) struct Widget {
    pub path: String,
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub asset: String,
    pub depth: i32,
    pub tint: [u8; 4],
}

pub(super) struct Assets {
    pub(super) unity: UnityData,
    levels: Vec<String>,
    uploaded: HashMap<ObjectKey, (String, u32, u32)>,
}

impl Assets {
    pub fn load(op: &mut Opener) -> Result<Self> {
        let mut unity = UnityData::new()?;
        let main = unity.file(op, "globalgamemanagers")?;
        ensure!(
            main.format == 17 && main.unity_version == "5.6.4p2",
            "unsupported COM3D2 serialized ABI"
        );
        let settings = main
            .objects
            .iter()
            .find(|o| o.class_id == c::BUILD_SETTINGS)
            .context("BuildSettings")?;
        let levels = crysirror_unity::classes::build_levels(&main.bytes(settings)?)?;
        ensure!(
            levels
                .first()
                .is_some_and(|n| n.ends_with("/_cm3d21.unity")),
            "unexpected COM3D2 boot scene"
        );
        Ok(Self {
            unity,
            levels,
            uploaded: HashMap::new(),
        })
    }

    fn reference(&mut self, op: &mut Opener, file: &str, r: &mut Reader<'_>) -> Result<ObjectKey> {
        self.unity
            .resolve(op, file, ptr(r)?)?
            .context("null UI asset")
    }

    fn texture(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        key: ObjectKey,
    ) -> Result<(String, u32, u32)> {
        if let Some(texture) = self.uploaded.get(&key) {
            return Ok(texture.clone());
        }
        let (class, bytes) = self.unity.bytes(op, &key)?;
        ensure!(class == c::TEXTURE_2D, "widget texture class {class}");
        let image = crysirror_unity::texture::decode_56(&bytes, |path| {
            ensure!(
                !path.contains(['/', '\\']) && path.ends_with(".resS"),
                "invalid sibling texture stream"
            );
            op.open(path)
        })?;
        let id = u32::try_from(self.uploaded.len())? + 1;
        let asset = host
            .upload_rgba(client, id, image.width, image.height, &image.rgba)?
            .relative()
            .to_owned();
        let texture = (asset, image.width, image.height);
        self.uploaded.insert(key, texture.clone());
        Ok(texture)
    }

    fn sprite(&mut self, op: &mut Opener, key: ObjectKey) -> Result<(ObjectKey, [f32; 4])> {
        let (class, data) = self.unity.bytes(op, &key)?;
        ensure!(class == 213, "expected a Unity Sprite");
        let mut r = Reader::new(&data);
        r.string()?;
        let rect = r.f32s::<4>()?;
        r.take(44)?;
        Ok((self.reference(op, &key.file, &mut r)?, rect))
    }

    fn atlas(
        &mut self,
        op: &mut Opener,
        key: ObjectKey,
        name: &str,
    ) -> Result<(ObjectKey, [f32; 4])> {
        let (class, bytes) = self.unity.bytes(op, &key)?;
        ensure!(class == c::MONO_BEHAVIOUR, "expected UIAtlas");
        let mut r = Reader::new(&bytes);
        r.seek(32)?;
        let material = self.reference(op, &key.file, &mut r)?;
        let mut rect = None;
        for _ in 0..r.count(52)? {
            let entry = r.string()?;
            let pos = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
            r.take(32)?;
            if entry == name {
                rect = Some(pos.map(|v| v as f32));
                break;
            }
        }
        let rect = rect.with_context(|| format!("missing UIAtlas sprite {name}"))?;
        let (class, mat) = self.unity.bytes(op, &material)?;
        ensure!(class == c::MATERIAL, "expected atlas material");
        let mat = crysirror_unity::classes::Material::read_56(&mat)?;
        let main = mat
            .texture("_MainTex")
            .context("atlas material main texture")?;
        Ok((
            self.unity
                .resolve(op, &material.file, main.texture)?
                .context("null atlas texture")?,
            rect,
        ))
    }

    pub fn scene_file(&self, scene: &str) -> Result<String> {
        let suffix = format!("/{scene}.unity");
        let index = self
            .levels
            .iter()
            .position(|p| p.ends_with(&suffix))
            .with_context(|| format!("missing {scene}"))?;
        Ok(format!("level{index}"))
    }

    pub fn scene(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        scene: &str,
    ) -> Result<Vec<Widget>> {
        let file = self.scene_file(scene)?;
        let sf = self.unity.file(op, &file)?;
        // GameObject component pointers and Transform parent links are 64-bit in v17.
        let mut nodes = HashMap::new();
        for o in sf.objects.iter().filter(|o| o.class_id == c::GAME_OBJECT) {
            let data = sf.bytes(o)?;
            let mut r = Reader::new(&data);
            let mut comps = Vec::new();
            for _ in 0..r.count(12)? {
                comps.push(ptr(&mut r)?.path_id);
            }
            r.i32()?;
            let name = r.string()?;
            let tr = *comps.first().context("GameObject has no transform")?;
            let tr_data = sf.bytes_of(tr)?;
            let mut t = Reader::new(&tr_data);
            t.seek(28)?;
            let pos = t.f32s::<3>()?;
            t.take(12)?;
            let children = t.count(12)?;
            t.take(12 * children)?;
            nodes.insert(tr, (name, ptr(&mut t)?.path_id, pos, comps));
        }
        let mut widgets = Vec::new();
        for (name, parent, pos, comps) in nodes.values() {
            if !matches!(
                name.as_str(),
                "Logo"
                    | "Back"
                    | "Warning"
                    | "BackGround"
                    | "TitleLogo"
                    | "Start"
                    | "Continue"
                    | "Guest"
                    | "Config"
                    | "End"
                    | "Shop"
                    | "SelectCursor"
            ) {
                continue;
            }
            let mut path = name.clone();
            let mut offset = *pos;
            let mut p = *parent;
            for _ in 0..128 {
                let Some((ancestor, up, xyz, _)) = nodes.get(&p) else {
                    break;
                };
                path = format!("{ancestor}/{path}");
                if ancestor == "UI Root" {
                    break;
                }
                offset[0] += xyz[0];
                offset[1] += xyz[1];
                p = *up;
            }
            let wanted = match scene {
                "SceneLogo" => path == "UI Root/Logo" || path == "UI Root/Back",
                "SceneWarning" => path == "UI Root/Warning",
                "SceneTitle" => path.starts_with("UI Root/TitlePanel/"),
                _ => false,
            };
            if !wanted {
                continue;
            }
            for &k in comps {
                if sf.object(k).is_none_or(|o| o.class_id != c::MONO_BEHAVIOUR) {
                    continue;
                }
                let body = sf.bytes_of(k)?;
                let mut r = Reader::new(&body);
                r.seek(16)?;
                let script = self.reference(op, &file, &mut r)?;
                let (_, bytes) = self.unity.bytes(op, &script)?;
                let class = Reader::new(&bytes).string()?;
                if class != "UI2DSprite" && class != "UISprite" {
                    continue;
                }
                r.seek(116)?;
                let tint = r
                    .f32s::<4>()?
                    .map(|v| (255. * v.clamp(0., 1.)).round() as u8);
                let pivot = r.i32()?;
                ensure!((0..=8).contains(&pivot), "invalid UIWidget pivot");
                let (w, h, depth) = (r.i32()? as f32, r.i32()? as f32, r.i32()?);
                r.seek(204)?;
                let key = self.reference(op, &file, &mut r)?;
                let (texture, rect) = if class == "UISprite" {
                    self.atlas(op, key, &r.string()?)?
                } else {
                    self.sprite(op, key)?
                };
                let (asset, tw, th) = self.texture(op, host, client, texture)?;
                let [x, y, rw, rh] = rect;
                let y = if class == "UI2DSprite" {
                    th as f32 - y - rh
                } else {
                    y
                };
                let uv = [
                    x / tw as f32,
                    y / th as f32,
                    (x + rw) / tw as f32,
                    (y + rh) / th as f32,
                ];
                let left = 960. + offset[0] - (pivot % 3) as f32 * 0.5 * w;
                let top = 540. - offset[1] - (pivot / 3) as f32 * 0.5 * h;
                widgets.push(Widget {
                    path: path.clone(),
                    rect: [left, top, w, h],
                    depth,
                    uv,
                    asset,
                    tint,
                });
            }
        }
        ensure!(!widgets.is_empty(), "no confirmed widgets in {scene}");
        widgets.sort_by(|a, b| (a.depth, &a.path).cmp(&(b.depth, &b.path)));
        Ok(widgets)
    }
}
