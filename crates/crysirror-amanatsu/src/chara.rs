// SPDX-License-Identifier: MPL-2.0
//! `Character.Human` assembly from a `HumanData` card: the body prefab
//! (`bo_body`) with its skeleton, the head (`bo_head`, `Custom` face
//! `headId`) grafted at `cf_s_headlink`, the four hair parts (`bo_hair_b/f/
//! s/o`) at the head's `HairParent` and the clothes of the first coordinate
//! (`co_*`) on the body, their bones resolved by name onto the skeletons
//! (`AssignedAnotherWeights`). Materials get the list's main textures,
//! tinted by the card's skin/hair colours and the clothes' colour masks.
//!
//! SATD-amanatsu-port: accessories, face/body shape values
//! (`customshape`), makeup/paint/eye layers of the `LIF/*` texture creation,
//! clothing body cut-outs (`body_upper/lower/onepi` types), blend shapes,
//! physics and poses are not applied.
use crate::{card::Card, level::Level, tables::Table, unity::Unity};
use anyhow::{Context, Result};
use crysirror_asset::pbr::AlphaMode;
use crysirror_image::ImageSurface;
use crysirror_unity::{
    msgpack::Value,
    scene6::Scene6,
    world6::{MaterialOverride, SceneAssets, UnityMaterial},
    PPtr,
};
use std::collections::HashMap;

/// `ChaListDefine.KeyType` columns used here.
pub mod key {
    pub const ID: i32 = 26;
    pub const MAIN_AB: i32 = 31;
    pub const MAIN_DATA: i32 = 32;
    pub const MAIN_TEX: i32 = 34;
    pub const MAIN_TEX_AB: i32 = 37;
    pub const COLOR_MASK_AB: i32 = 5;
    pub const COLOR_MASK: i32 = 6;
}

/// Hair part lists in `HairData.parts` order.
pub const HAIR_LISTS: [&str; 4] = ["bo_hair_b", "bo_hair_f", "bo_hair_s", "bo_hair_o"];
/// Clothes part lists in `ClothesData.parts` order.
pub const CLOTHES_LISTS: [&str; 11] = ["co_top", "co_bot", "co_bra", "co_shorts", "co_gloves", "co_panst", "co_socks", "co_shoes", "co_add_arm", "co_add_leg", "co_add_other"];

/// Body objects the base body does not show: the clothing cut-out
/// variants (type 01 is the full torso), the one-piece variant, H options,
/// silhouettes and the shadow proxy.
const HIDDEN_BODY: [&str; 16] = [
    "n_dankon", "n_mnpb", "body_onepi", "n_silhouetteTop", "o_body_shadow", "o_upper_type02", "o_upper_type03", "o_upper_type04", "o_upper_type05", "o_lower_type02", "o_lower_type03", "o_lower_type04", "o_lower_type05",
    "o_lower_type06", "o_upper_type06", "n_body_silhouette",
];

/// Face texture lists: (shader, list, texture key, card id field, card
/// colour fields for the texture's R, G, B mask channels).
const FACE_TEXTURES: [(&str, &str, i32, &str, [&str; 3]); 3] = [
    ("AL/eyebrow", "mt_eyebrow", 11, "eyebrowId", ["eyebrowColor", "eyebrowColor2", ""]),
    ("AL/eyelash_up", "mt_eyeline_up", 17, "eyelineUpId", ["eyelineColor", "eyelineColor2", "eyelineColor3"]),
    ("AL/eye", "mt_eye", 23, "", ["eye01Color", "eye02Color", "eye03Color"]),
];

/// The `chara/list/characustom` tables, merged over every list bundle.
#[derive(Default)]
pub struct Lists {
    tables: HashMap<String, Table>,
}

impl Lists {
    pub fn load(unity: &mut Unity) -> Result<Self> {
        let mut out = Lists::default();
        let bundles: Vec<String> = unity
            .bundle_files()
            .into_iter()
            .filter_map(|k| k.strip_prefix("bundle:").and_then(|r| r.split_once('#')).map(|(b, _)| b.to_owned()))
            .filter(|b| b.starts_with("lib/chara/list/characustom/"))
            .collect();
        for key in unity.bundle_files().into_iter().filter(|k| bundles.iter().any(|b| k.starts_with(&format!("bundle:{b}#")))) {
            let file = unity.file(&key)?;
            let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 49).map(|o| o.path_id).collect();
            for id in ids {
                let f = unity.decode(&key, id)?;
                let name = f.string("m_Name").unwrap_or("").to_owned();
                let list = name.split('[').next().unwrap_or(&name).to_owned();
                let Ok(t) = Table::parse(&unity.text_asset(&key, id)?) else { continue };
                let e = out.tables.entry(list).or_insert_with(|| Table { keys: t.keys.clone(), rows: Vec::new() });
                if e.keys == t.keys {
                    e.rows.extend(t.rows);
                }
            }
        }
        Ok(out)
    }

    /// A row of `list` by id, as a key → value lookup.
    pub fn row(&self, list: &str, id: i64) -> Option<Row<'_>> {
        let t = self.tables.get(list)?;
        let col = t.keys.iter().position(|k| *k == key::ID)?;
        let values = t.rows.iter().find(|r| r.get(col).and_then(|v| v.parse::<i64>().ok()) == Some(id))?;
        Some(Row { keys: &t.keys, values })
    }
}

pub struct Row<'a> {
    keys: &'a [i32],
    values: &'a [String],
}

impl Row<'_> {
    pub fn get(&self, key: i32) -> Option<&str> {
        let i = self.keys.iter().position(|k| *k == key)?;
        self.values.get(i).map(String::as_str).filter(|v| !v.is_empty() && *v != "0")
    }
    /// (`lib/` bundle, asset) of an AB/data key pair.
    pub fn asset(&self, ab: i32, data: i32) -> Option<(String, String)> {
        Some((format!("lib/{}", self.get(ab)?), self.get(data)?.to_owned()))
    }
}

/// How a part's materials are drawn.
#[derive(Clone, Debug, Default)]
struct PartLook {
    /// Shaders the look applies to (prefix match); others keep theirs.
    shaders: &'static [&'static str],
    /// Face layers by shader: (texture, sRGB tint).
    layers: Vec<(&'static str, Option<(String, String)>, Vec<[f32; 4]>)>,
    /// Main texture (`_t`) and colour mask (`_mc`) of the list.
    main: Option<(String, String)>,
    mask: Option<(String, String)>,
    /// Card colours (sRGB) for the mask channels R, G, B (and the tint
    /// without a mask).
    colors: Vec<[f32; 4]>,
    /// Card outline colour (hair).
    outline: Option<[f32; 4]>,
}

pub struct Human {
    /// Body file key (the scene's primary file).
    pub key: String,
    pub female: bool,
    /// `shapeValueBody[0]` (height, 0..1).
    pub height: f32,
    pub scene: Scene6,
    /// Part looks by the file key of their prefab.
    looks: HashMap<String, PartLook>,
    pub log: Vec<String>,
}

fn color(v: Option<&Value>) -> Option<[f32; 4]> {
    let a = v?.as_array();
    (a.len() >= 3).then(|| [0, 1, 2, 3].map(|i| a.get(i).and_then(Value::as_f32).unwrap_or(1.)))
}

/// The first coordinate's blocks (`Clothes`, `Hair`, …).
fn coordinate(card: &Card) -> Result<Card> {
    let v = crysirror_unity::msgpack::decode(card.block("Coordinate").context("card Coordinate")?)?;
    let first = v.as_array().first().and_then(Value::as_bytes).context("card coordinate 0")?;
    Ok(Card::container(first)?.1)
}

impl Human {
    /// Assemble the card's character.
    pub fn from_card(unity: &mut Unity, lists: &Lists, card: &Card, female: bool) -> Result<Self> {
        let mut log = Vec::new();
        let custom = card.parts("Custom")?;
        let face = custom.first().context("card face")?;
        let body_data = custom.get(1).context("card body")?;
        let body_row = lists.row("bo_body", if female { 1 } else { 0 }).context("bo_body row")?;
        let (bb, bp) = body_row.asset(key::MAIN_AB, key::MAIN_DATA).context("bo_body prefab")?;
        let body = Level::prefab(unity, &bb, &bp)?;
        let mut scene = body.scene;
        let mut looks = HashMap::new();
        let skin = color(body_data.get("skinMainColor")).unwrap_or([1.; 4]);
        looks.insert(body.key.clone(), PartLook { shaders: &["AL/skin_body"], main: body_row.asset(key::MAIN_TEX_AB, key::MAIN_TEX), colors: vec![skin], ..Default::default() });
        for name in HIDDEN_BODY {
            for n in scene.nodes.iter_mut().filter(|n| n.name == name) {
                n.active = false;
            }
        }
        let skeleton = scene.nodes.iter().position(|n| n.name == "cf_j_root").context("body cf_j_root")?;
        // Head.
        let head_id = face.get("headId").and_then(Value::as_i64).unwrap_or(0);
        let head_row = lists.row("bo_head", head_id).with_context(|| format!("bo_head {head_id}"))?;
        let (hb, hp) = head_row.asset(key::MAIN_AB, key::MAIN_DATA).context("bo_head prefab")?;
        let head = Level::prefab(unity, &hb, &hp)?;
        let mut layers = Vec::new();
        for (shader, list, tex, id_field, color_fields) in FACE_TEXTURES {
            // The eye's iris comes from the first pupil entry.
            let pupil = face.get("pupil").map(Value::as_array).and_then(|a| a.first());
            let src = if id_field.is_empty() { pupil } else { Some(face) };
            let id = src.and_then(|s| s.get(if id_field.is_empty() { "id" } else { id_field })).and_then(Value::as_i64).unwrap_or(0);
            let colors = color_fields.iter().map(|f| src.and_then(|s| color(s.get(f))).unwrap_or([1.; 4])).collect();
            let texture = lists.row(list, id).and_then(|r| r.asset(key::MAIN_AB, tex));
            layers.push((shader, texture, colors));
        }
        looks.insert(head.key.clone(), PartLook { shaders: &["AL/skin_head"], main: head_row.asset(key::MAIN_TEX_AB, key::MAIN_TEX), colors: vec![skin], layers, ..Default::default() });
        let link = scene.nodes.iter().position(|n| n.name == "cf_s_headlink").context("body cf_s_headlink")?;
        let head_root = scene.graft(&head.scene, &head.key, link);
        let hair_parent = {
            let mut stack = vec![head_root];
            let mut found = None;
            while let Some(n) = stack.pop() {
                if scene.nodes[n].name == "HairParent" {
                    found = Some(n);
                    break;
                }
                stack.extend(&scene.nodes[n].children);
            }
            found.context("head HairParent")?
        };
        log.push(format!("body {bp}, head {hp}"));
        let coord = coordinate(card)?;
        // Hair.
        let hair = crysirror_unity::msgpack::decode(coord.block("Hair").context("coordinate Hair")?)?;
        for (i, part) in hair.get("parts").map(Value::as_array).unwrap_or(&[]).iter().enumerate() {
            let Some(list) = HAIR_LISTS.get(i) else { break };
            let id = part.get("id").and_then(Value::as_i64).unwrap_or(-1);
            let Some(row) = lists.row(list, id) else {
                log.push(format!("{list} {id}: no list row"));
                continue;
            };
            let Some((ab, data)) = row.asset(key::MAIN_AB, key::MAIN_DATA) else { continue };
            match Level::prefab(unity, &ab, &data) {
                Ok(p) => {
                    let c = color(part.get("baseColor"));
                    let outline = color(part.get("outlineColor"));
                    looks.insert(p.key.clone(), PartLook { shaders: &["AL/hair"], colors: c.into_iter().collect(), outline, ..Default::default() });
                    let r = scene.graft(&p.scene, &p.key, hair_parent);
                    scene.alias_bones(r, head_root);
                    log.push(format!("{list} {id}: {data}"));
                }
                Err(e) => log.push(format!("{list} {id}: {e:#}")),
            }
        }
        // Clothes.
        let clothes = crysirror_unity::msgpack::decode(coord.block("Clothes").context("coordinate Clothes")?)?;
        for (i, part) in clothes.get("parts").map(Value::as_array).unwrap_or(&[]).iter().enumerate() {
            let Some(list) = CLOTHES_LISTS.get(i) else { break };
            let id = part.get("id").and_then(Value::as_i64).unwrap_or(0);
            let Some(row) = lists.row(list, id) else {
                log.push(format!("{list} {id}: no list row"));
                continue;
            };
            let Some((ab, data)) = row.asset(key::MAIN_AB, key::MAIN_DATA) else { continue };
            if data == "p_dummy" {
                continue;
            }
            match Level::prefab(unity, &ab, &data) {
                Ok(p) => {
                    let colors = part.get("colorInfo").map(Value::as_array).unwrap_or(&[]).iter().filter_map(|c| color(c.get("baseColor"))).collect();
                    looks.insert(p.key.clone(), PartLook { shaders: &["AL/cloth"], main: row.asset(key::MAIN_TEX_AB, key::MAIN_TEX), mask: row.asset(key::COLOR_MASK_AB, key::COLOR_MASK), colors, ..Default::default() });
                    let r = scene.graft(&p.scene, &p.key, 0);
                    let n = scene.alias_bones(r, skeleton);
                    log.push(format!("{list} {id}: {data} ({n} bones)"));
                }
                Err(e) => log.push(format!("{list} {id}: {e:#}")),
            }
        }
        // Shadow-caster proxies (`o_body_shadow`, `cf_O_face_shadow`) only
        // feed Unity's shadow pass.
        for n in scene.nodes.iter_mut().filter(|n| n.name.ends_with("_shadow")) {
            n.active = false;
        }
        let height = body_data.get("shapeValueBody").map(Value::as_array).and_then(|a| a.first()).and_then(Value::as_f32).unwrap_or(0.5);
        Ok(Self { key: body.key, female, height, scene, looks, log })
    }
}

/// Animator controllers of the action motions (`ani/mot/<sex>`).
pub fn motion_controller(female: bool) -> (String, &'static str) {
    (format!("lib/ani/mot/{}/000_00.unity3d", u8::from(female)), if female { "f_00" } else { "m_00" })
}

impl Human {
    /// A controller state's motion as glTF tracks: the state's height
    /// blend tree (`_S`/`_M`/`_L` clips) at the card's height, nearest
    /// child (SATD-amanatsu-port: no blending between the three).
    pub fn state_tracks(&self, unity: &mut Unity, state: &str) -> Result<crysirror_unity::world6::ClipTracks> {
        use crysirror_unity::animator::{Clip, Controller};
        let (bundle, name) = motion_controller(self.female);
        let (k, id) = unity.bundle_asset(&bundle, name, 91)?.with_context(|| format!("controller {name}"))?;
        let ctl = Controller::decode(&unity.decode(&k, id)?)?;
        let si = ctl.find_state(0, state).with_context(|| format!("state {state}"))?;
        let first = ctl.layers[0].states[si].clip.context("state motion")?;
        let pick = if self.height < 1. / 3. { 0 } else if self.height < 2. / 3. { 1 } else { 2 };
        let cp = *ctl.clips.get(first + pick).or(ctl.clips.get(first)).context("state clip")?;
        let (f, cid) = unity.target(&k, cp)?;
        let clip = Clip::decode(&unity.decode(&f, cid)?)?;
        let root = 0;
        Ok(crysirror_unity::world6::clip_tracks(&self.scene, root, &clip, state, 30.))
    }
}

/// The importer's view of a character: the installation plus the parts'
/// texture creation (`CustomTextureCreate`: main texture tinted through the
/// colour mask channels).
pub struct CharaAssets<'a> {
    pub unity: &'a mut Unity,
    pub human: &'a Human,
    /// Toon colour scale for the scene's key light: Unity lights characters
    /// with their own light (`Directional Light Sun Chara`), the engine with
    /// the one sun.
    pub light: f32,
}

impl CharaAssets<'_> {
    fn texture_named(&mut self, (bundle, name): &(String, String)) -> Result<Option<ImageSurface>> {
        let Some((k, id)) = self.unity.bundle_asset(bundle, name, crysirror_unity::class_id::TEXTURE_2D)? else { return Ok(None) };
        Ok(Some(SceneAssets::texture(&mut *self.unity, &k, id)?))
    }
}

/// `main × lerp(lerp(lerp(1, c1, m.r), c2, m.g), c3, m.b)` in display
/// space (the colour mask blend of the ILLGAMES create shaders).
pub fn tint_by_mask(main: &mut ImageSurface, mask: Option<&ImageSurface>, colors: &[[f32; 4]]) {
    let c = |i: usize| colors.get(i).copied().unwrap_or([1.; 4]);
    for y in 0..main.height {
        for x in 0..main.width {
            let i = ((y * main.width + x) * 4) as usize;
            let m = match mask {
                Some(m) if m.width > 0 && m.height > 0 => {
                    let mx = (x as u64 * m.width as u64 / main.width as u64) as u32;
                    let my = (y as u64 * m.height as u64 / main.height as u64) as u32;
                    let j = ((my * m.width + mx) * 4) as usize;
                    [f32::from(m.rgba[j]) / 255., f32::from(m.rgba[j + 1]) / 255., f32::from(m.rgba[j + 2]) / 255.]
                }
                Some(_) => [0.; 3],
                None => [1., 0., 0.],
            };
            for ch in 0..3 {
                let mut t = 1.;
                for (k, w) in m.iter().enumerate() {
                    t += (c(k)[ch] - t) * w;
                }
                main.rgba[i + ch] = (f32::from(main.rgba[i + ch]) * t).clamp(0., 255.) as u8;
            }
        }
    }
}

impl SceneAssets for CharaAssets<'_> {
    fn object(&mut self, file: &str, p: PPtr) -> Result<Option<(String, i64, i32, crysirror_unity::managed::Fields)>> {
        SceneAssets::object(&mut *self.unity, file, p)
    }
    fn mesh(&mut self, file: &str, path_id: i64) -> Result<std::sync::Arc<crysirror_unity::unity6::Mesh6>> {
        SceneAssets::mesh(&mut *self.unity, file, path_id)
    }
    fn texture(&mut self, file: &str, path_id: i64) -> Result<ImageSurface> {
        SceneAssets::texture(&mut *self.unity, file, path_id)
    }
    fn texture_size(&mut self, file: &str, path_id: i64) -> Result<(u32, u32)> {
        SceneAssets::texture_size(&mut *self.unity, file, path_id)
    }
    fn material_hook(&mut self, file: &str, m: &UnityMaterial) -> Result<Option<MaterialOverride>> {
        let Some(look) = self.human.looks.get(file).cloned() else { return Ok(None) };
        let shader = m.shader.as_str();
        // Outline shells and helper passes draw nothing here; the eyelid
        // layer shades the skin in the original shader (not converted).
        if shader.ends_with("_outline") || shader.starts_with("AL/sub/") || shader == "AL/eyelid" {
            return Ok(Some(MaterialOverride { hidden: true, ..Default::default() }));
        }
        let mut o = MaterialOverride::default();
        if let Some((_, texture, colors)) = look.layers.iter().find(|l| shader == l.0) {
            // A face layer: the list texture's RGB select the card colours,
            // its alpha is the shape.
            match texture {
                Some(t) => match self.texture_named(t)? {
                    Some(mask) => {
                        let mut img = ImageSurface { width: mask.width, height: mask.height, rgba: mask.rgba.iter().enumerate().map(|(i, a)| if i % 4 == 3 { *a } else { 255 }).collect() };
                        if shader == "AL/eye" {
                            // Iris: R/G/B select the three eye colours.
                            tint_by_mask(&mut img, Some(&mask), colors);
                        } else {
                            // Lashes/brows: the shape in the main colour
                            // (gradient/highlight channels not converted).
                            tint_by_mask(&mut img, None, &colors[..1]);
                        }
                        o.base_texture = Some((format!("{}:{}", t.0, t.1), img));
                        o.base_color = Some([1.; 4]);
                        o.alpha_mode = Some(AlphaMode::Blend);
                    }
                    None => o.hidden = true,
                },
                None => o.hidden = true,
            }
            return Ok(Some(o));
        }
        if !look.shaders.iter().any(|s| shader.starts_with(s)) {
            return Ok(None);
        }
        // AL character shaders are cel shaded with outlines.
        let outline = look.outline.unwrap_or([0.25, 0.18, 0.18, 1.]);
        o.toon = Some((toon_lookup(shader), if shader.starts_with("AL/hair") || shader.starts_with("AL/cloth") || shader.starts_with("AL/skin") { 0.0015 } else { 0. }, outline));
        let k = self.light;
        if let Some(main) = &look.main {
            if let Some(mut img) = self.texture_named(main)? {
                let mask = match &look.mask {
                    Some(mk) => self.texture_named(mk)?,
                    None => None,
                };
                tint_by_mask(&mut img, mask.as_ref(), &look.colors);
                // Texture alpha cuts openings (the face's eye holes).
                if img.rgba.chunks_exact(4).any(|p| p[3] < 128) {
                    o.alpha_mode = Some(AlphaMode::Mask);
                }
                o.base_texture = Some((format!("{}:{}:{}", main.0, main.1, m.name), img));
                o.base_color = Some([k, k, k, 1.]);
            }
        } else if let Some(c) = look.colors.first() {
            // Toon shading works on display-referred colours.
            o.base_color = Some([c[0] * k, c[1] * k, c[2] * k, 1.]);
        }
        Ok(Some(o))
    }
}

/// Two-tone ramp of the ILLGAMES toon shaders: a soft terminator around
/// n·l = 0 (u = 0.5) between a warm shadow tone and full colour.
pub fn toon_lookup(shader: &str) -> ImageSurface {
    let shadow: [f32; 3] = if shader.starts_with("AL/skin") { [0.86, 0.74, 0.74] } else { [0.74, 0.72, 0.78] };
    let mut rgba = vec![0u8; 256 * 3 * 4];
    for x in 0..256 {
        let u = x as f32 / 255.;
        let t = ((u - 0.46) / 0.08).clamp(0., 1.);
        for c in 0..3 {
            rgba[x * 4 + c] = ((shadow[c] + (1. - shadow[c]) * t) * 255.).round() as u8;
        }
        rgba[x * 4 + 3] = 255;
        // Row 1: shadow-layer rate (no shadow texture), row 2: unused.
        rgba[(256 + x) * 4..(256 + x) * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
        rgba[(512 + x) * 4 + 3] = 255;
    }
    ImageSurface { width: 256, height: 3, rgba }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_tint() {
        let mut main = ImageSurface { width: 1, height: 1, rgba: vec![200, 200, 200, 255] };
        let mask = ImageSurface { width: 1, height: 1, rgba: vec![0, 255, 0, 255] };
        tint_by_mask(&mut main, Some(&mask), &[[1., 0., 0., 1.], [0.5, 0.5, 1., 1.], [0., 0., 0., 1.]]);
        assert_eq!(&main.rgba[..3], &[100, 100, 200]);
    }
}
