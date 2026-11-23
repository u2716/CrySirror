// SPDX-License-Identifier: MPL-2.0
//! `Character.Human` assembly from a `HumanData` card: the body prefab
//! (`bo_body`) with its skeleton, the head (`bo_head`, `Custom` face
//! `headId`) grafted at `cf_s_headlink`, the four hair parts (`bo_hair_b/f/
//! s/o`) at the head's `HairParent` and the clothes of the first coordinate
//! (`co_*`) on the body, their bones resolved by name onto the skeletons
//! (`AssignedAnotherWeights`). Materials get the list's main textures,
//! tinted by the card's skin/hair colours and the clothes' colour masks.
//! The body bone prefab adds the chest/hip/arm/leg/belly/waist MagicaCloth
//! springs and `cf_hit_*` colliders, tuned by the card's softness values.
//!
//! SATD-amanatsu-port: accessory swing/hide states, face/body shape values
//! (`customshape`), makeup/paint/eye layers of the `LIF/*` texture creation,
//! clothing body cut-outs (`body_upper/lower/onepi` types), blend shapes
//! and poses are not applied.
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
    pub const EYE_HI_UP_TEX: i32 = 14;
    pub const EYE_PIPIL_TEX: i32 = 18;
    /// Accessory default parent node.
    pub const ACCESSORY_PARENT: i32 = 54;
    /// Display name.
    pub const NAME: i32 = 39;
}

/// The face mesh node (`AL/skin_head`), target of the flush blend.
pub const FACE_MESH: &str = "cf_O_face";

/// Hair part lists in `HairData.parts` order.
pub const HAIR_LISTS: [&str; 4] = ["bo_hair_b", "bo_hair_f", "bo_hair_s", "bo_hair_o"];
/// Clothes part lists in `ClothesData.parts` order.
/// Accessory lists by `Define.CategoryNo` 121 (`ao_hair`) to 130.
pub const ACCESSORY_LISTS: [&str; 10] = ["ao_hair", "ao_head", "ao_face", "ao_neck", "ao_body", "ao_waist", "ao_leg", "ao_arm", "ao_hand", "ao_kokan"];

/// `HumanAccessory.Define.AccessoryParentKey` node names (0 none, 1 the
/// root bone).
const ACCESSORY_PARENTS: [&str; 56] = [
    "", "cf_j_root", "a_n_hair_pony", "a_n_hair_twin_L", "a_n_hair_twin_R", "a_n_hair_pin", "a_n_hair_pin_R", "a_n_headtop", "a_n_head", "a_n_headflont", "a_n_headside", "a_n_earrings_L", "a_n_earrings_R", "a_n_megane", "a_n_nose", "a_n_mouth", "a_n_neck", "a_n_bust_f", "a_n_bust", "a_n_nip_L", "a_n_nip_R", "a_n_back", "a_n_back_L", "a_n_back_R", "a_n_waist", "a_n_waist_f", "a_n_waist_b", "a_n_waist_L", "a_n_waist_R", "a_n_leg_L", "a_n_knee_L", "a_n_ankle_L", "a_n_heel_L", "a_n_leg_R",
    "a_n_knee_R", "a_n_ankle_R", "a_n_heel_R", "a_n_shoulder_L", "a_n_arm_L", "a_n_elbo_L", "a_n_wrist_L", "a_n_shoulder_R", "a_n_arm_R", "a_n_elbo_R", "a_n_wrist_R", "a_n_hand_L", "a_n_ind_L", "a_n_mid_L", "a_n_ring_L", "a_n_hand_R", "a_n_ind_R", "a_n_mid_R", "a_n_ring_R", "a_n_dan", "a_n_kokan", "a_n_ana",
];

pub const CLOTHES_LISTS: [&str; 11] = ["co_top", "co_bot", "co_bra", "co_shorts", "co_gloves", "co_panst", "co_socks", "co_shoes", "co_add_arm", "co_add_leg", "co_add_other"];

/// Body objects the base body does not show: the clothing cut-out
/// variants (type 01 is the full torso), the one-piece variant, H options,
/// silhouettes and the shadow proxy.
/// `HumanBody.UpdateVisible` objects shown by the body state (`Table` refs
/// 96 and 102): exported for runtime visibility on undressed characters.
pub const SON: &str = "n_dankon";
pub const MOZ: &str = "n_mnpb";

const HIDDEN_BODY: [&str; 16] = [
    "n_dankon", "n_mnpb", "body_onepi", "n_silhouetteTop", "o_body_shadow", "o_upper_type02", "o_upper_type03", "o_upper_type04", "o_upper_type05", "o_lower_type02", "o_lower_type03", "o_lower_type04", "o_lower_type05",
    "o_lower_type06", "o_upper_type06", "n_body_silhouette",
];

/// The body bone prefab carrying the body MagicaCloth springs and the
/// `cf_hit_*` colliders (`HumanBody` loads it beside `bo_body`).
const BODY_BONE: (&str, &str) = ("lib/chara/body/bone/000.unity3d", "p_cf_al_body_bone_00");

/// `HumanMagicaSpringSetting` groups: (field, `ShakeCorrectData` part,
/// card body softness field).
const SPRING_GROUPS: [(&str, &str, &str); 10] = [
    ("_magicaBustLBones", "_bustData", "bustSoftness"),
    ("_magicaBustRBones", "_bustData", "bustSoftness"),
    ("_magicaHipLBones", "_hipData", "hipSoftness"),
    ("_magicaHipRBones", "_hipData", "hipSoftness"),
    ("_magicaArmLBones", "_upperArmData", "upperArmSoftness"),
    ("_magicaArmRBones", "_upperArmData", "upperArmSoftness"),
    ("_magicaLegLBones", "_thighData", "thighSoftness"),
    ("_magicaLegRBones", "_thighData", "thighSoftness"),
    ("_magicaBellyBones", "_bellyData", "bellySoftness"),
    ("_magicaWaistBones", "_waistData", "waistSoftness"),
];

/// `Table.sKeyNameTable` state objects of the clothes (`S_*_NUGE`,
/// `S_UWB_B_NUGE2`, `S_PANST_NUGE`): shown only while a part is in its
/// half state; the `*_DEF` objects (`n_top_a`, `n_bot_a`, `n_panst_a`) are
/// the dressed state.
const UNDRESSED: [&str; 4] = ["n_top_b", "n_bot_b", "n_bot_c", "n_panst_b"];

/// `HumanCloth.UpdateVisible`/`DrawOption` for the dressed state: hide the
/// half-undressed objects under the part and the `CmpClothes.objOpt01/02`
/// options the coordinate's `hideOpt` flags hide.
fn dress(scene: &mut Scene6, part: usize, hide_opt: &[bool]) {
    let origin = scene.nodes[part].origin;
    let mut stack = vec![part];
    let mut sub = Vec::new();
    while let Some(n) = stack.pop() {
        sub.push(n);
        stack.extend(&scene.nodes[n].children);
    }
    let mut hidden: Vec<i64> = Vec::new();
    for &n in &sub {
        for c in scene.nodes[n].components.iter().filter(|c| c.script.ends_with("CmpClothes")) {
            for (i, field) in ["objOpt01", "objOpt02"].iter().enumerate() {
                if hide_opt.get(i).copied().unwrap_or(false) {
                    hidden.extend(c.fields.array(field).unwrap_or(&[]).iter().filter_map(|v| if let crysirror_unity::managed::Value::PPtr(p) = v { Some(p.path_id) } else { None }));
                }
            }
        }
    }
    for n in sub {
        let node = &mut scene.nodes[n];
        if UNDRESSED.contains(&node.name.as_str()) || (node.origin == origin && hidden.contains(&node.game_object)) {
            node.active = false;
        }
    }
}

/// The `ShakeCorrectData` asset (`chara/list/shakecorrect`).
fn shake_correct(unity: &mut Unity) -> Result<crysirror_unity::managed::Fields> {
    for key in unity.bundle_files().into_iter().filter(|k| k.starts_with("bundle:lib/chara/list/shakecorrect/")) {
        let ids: Vec<i64> = unity.file(&key)?.objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let f = unity.decode(&key, id)?;
            if f.get("_bustData").is_some() {
                return Ok(f);
            }
        }
    }
    anyhow::bail!("no ShakeCorrectData")
}

/// `HumanBody.SetSpringSetting`/`ChangeBustGravity`: each group's
/// MagicaCloths get `PartsData.GetData(softness)` (min + (max − min) ×
/// clamp01(v)) as spring power, spring limit distance, angle-restoration
/// stiffness and velocity attenuation; the chest gravity is
/// lerp(`_bustGravity`, `bustWeight`). Returns the components changed.
fn apply_shake(scene: &mut Scene6, bone_root: usize, shake: &crysirror_unity::managed::Fields, body: &Value) -> usize {
    use crysirror_unity::managed::{Fields, Value as F};
    let origin = scene.nodes[bone_root].origin;
    let range = |f: Option<&Fields>, v: f32| {
        let (a, b) = f.map_or((0., 0.), |f| (f.float("_min").unwrap_or(0.) as f32, f.float("_max").unwrap_or(0.) as f32));
        a + (b - a) * v.clamp(0., 1.)
    };
    let soft = |k: &str| body.get(k).and_then(Value::as_f32).unwrap_or(0.5);
    let Some(setting) = scene.nodes.iter().filter(|n| n.origin == origin).flat_map(|n| &n.components).find(|c| c.script.ends_with("HumanMagicaSpringSetting")).map(|c| c.fields.clone()) else { return 0 };
    let mut changed = 0;
    for (group, part, softness) in SPRING_GROUPS {
        let v = soft(softness);
        let part = shake.fields(part);
        let data = part.and_then(|p| p.fields("_data"));
        let values = [
            ("serializeData.springConstraint.springPower", range(part.and_then(|p| p.fields("_power")), v)),
            ("serializeData.springConstraint.limitDistance", range(data.and_then(|d| d.fields("_limitDistance")), v)),
            ("serializeData.angleRestorationConstraint.stiffness.value", range(data.and_then(|d| d.fields("_stiffness")), v)),
            ("serializeData.angleRestorationConstraint.velocityAttenuation", range(data.and_then(|d| d.fields("_velocityAttenuation")), v)),
        ];
        let gravity = part.is_some_and(|_| group.starts_with("_magicaBust")).then(|| range(shake.fields("_bustGravity"), soft("bustWeight")));
        let springs: Vec<i64> = match setting.get(group) {
            Some(F::Array(a)) => a.iter().filter_map(|e| if let F::Struct(f) = e { f.pptr("Spring") } else { None }).map(|p| p.path_id).collect(),
            Some(F::Struct(f)) => f.pptr("Spring").map(|p| p.path_id).into_iter().collect(),
            _ => Vec::new(),
        };
        for c in scene.nodes.iter_mut().filter(|n| n.origin == origin).flat_map(|n| &mut n.components).filter(|c| springs.contains(&c.path_id)) {
            for (path, x) in values {
                c.fields.set(path, F::Float(f64::from(x)));
            }
            if let Some(g) = gravity {
                c.fields.set("serializeData.gravity", F::Float(f64::from(g)));
            }
            changed += 1;
        }
    }
    changed
}

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

    /// The loaded list names.
    pub fn names(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
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
    /// Card `skinShadowColor` (`_Shadow_color` of the skin shaders).
    shadow: Option<[f32; 4]>,
    /// Pupil and highlight layers of the left and right eye (card
    /// `pupil[0]`, `pupil[1]`).
    eyes: Vec<EyeLayers>,
    /// Face makeup layers drawn over the main texture (head).
    makeup: Option<std::sync::Arc<Makeup>>,
    /// The colour mask is the material's own `_Main_color_mask`
    /// (accessories, which have no main texture).
    material_mask: bool,
    /// Hair: the card's start/end colours for the `_Hair_color_mask`.
    hair: Option<std::sync::Arc<HairLook>>,
}

/// The `AL/skin_head` makeup layers of a face mesh: each layer's mask
/// lives in one of the mesh's UV sets (uv1 cheek space, mirrored with
/// u = 1 on the centre line; uv2 nose; uv3 mouth), outside [0, 1] where
/// the layer does not reach.
#[derive(Debug)]
pub struct Makeup {
    /// The meshes sharing the base texture (uv0) the layers bake into.
    pub meshes: Vec<std::sync::Arc<crysirror_unity::unity6::Mesh6>>,
    pub layers: Vec<MakeupLayer>,
}

#[derive(Clone, Debug)]
pub struct MakeupLayer {
    pub texture: (String, String),
    /// UV set of the layer.
    pub uv: usize,
    /// Mask channel colour (sRGB, alpha the strength).
    pub color: [f32; 4],
    /// Texture channel of the mask.
    pub channel: usize,
    /// Added (highlights) instead of mixed.
    pub additive: bool,
    /// Two colours: `color` → `color2` by the texture's red ×
    /// `color2.a`, mixed in by the texture's alpha × `color.a` (the body's
    /// `_Nip` layer).
    pub color2: Option<[f32; 4]>,
    /// The texture's own colour × `color`, mixed in by its alpha ×
    /// `color.a` (the body's `_Under_hair_texture`).
    pub tinted: bool,
    /// Placement of the texture in its UV set.
    pub place: Option<Place>,
}

/// Hair colour inputs of one part (`HumanHair.ChangeSettingHairColor`:
/// `baseColor` → `_Main_color`, `startColor` → `_Sub_color_01`, `endColor`
/// → `_Sub_color_02`).
#[derive(Debug)]
pub struct HairLook {
    pub start: [f32; 4],
    pub end: [f32; 4],
    /// `mt_hairgloss` texture of the card's `glossId`
    /// (`HumanHair.LoadHairGlossMask`) and `glossColor`/`glossColor2`
    /// (`_Highlight_color01/02`).
    pub gloss: Option<(String, String)>,
    pub gloss_colors: [[f32; 4]; 2],
    /// The part's meshes (the gloss band lies in their uv1).
    pub meshes: Vec<std::sync::Arc<crysirror_unity::unity6::Mesh6>>,
}

/// `AL/skin_head` placements of a makeup texture (decompiled forward
/// program).
#[derive(Clone, Copy, Debug)]
pub enum Place {
    /// Cheek (or its highlight) in uv1: card position/size/rotation
    /// (`_Cheek_position_x/_y`, `_Cheek_size`, `_Cheek_rotation`) with the
    /// material's `_Cheek_position` vector.
    Cheek { pos: [f32; 2], size: f32, rotation: f32, highlight: bool },
    /// Nose in uv2 at `_NoseScale`.
    Nose { scale: f32 },
    /// Body detail layer in uv2 (`AL/skin_body`): away from the centre the
    /// coordinate moves toward `uv · (2 − 2z) + z − 0.5` (`z = size · 0.27 +
    /// 0.54`) by `sat((|uv − 0.5| + 0.05) · 8.69565 − 1)`.
    Nip { size: f32 },
}

impl Place {
    /// The texture coordinate of a layer UV.
    fn apply(self, uv: [f32; 2], m: Option<&UnityMaterial>) -> [f32; 2] {
        match self {
            Place::Cheek { pos, size, rotation, highlight } => {
                let p = m.and_then(|m| m.colors.get(if highlight { "_CheekHighlight_position" } else { "_Cheek_position" }).copied()).unwrap_or(if highlight { [-0.07, 0., 0., 0.75] } else { [-0.05, 0., 0., 0.75] });
                let (k, o) = if highlight { ([0.4, -0.27], [-0.2, 0.12]) } else { ([0.37, -0.23], [-0.15, 0.08]) };
                let s = 2. - 2. * (size * 0.2 + p[3] - 0.1);
                let d = [uv[0] + pos[0] * k[0] + p[0] + o[0], uv[1] + pos[1] * k[1] + p[1] + o[1]].map(|v| 2. * s * v - s);
                let (sn, cs) = ((p[2] - rotation + 0.5) * std::f32::consts::TAU).sin_cos();
                [d[0] * cs + d[1] * sn + 0.5, -d[0] * sn + d[1] * cs + 0.5]
            }
            Place::Nose { scale } => [uv[0] * (scale * 0.4 + 0.8) + scale * -0.2 + 0.1, uv[1] * (scale * 0.4 + 0.8) + scale * -0.2 + 0.1 + scale * 0.08 - 0.04],
            Place::Nip { size } => {
                let z = size * 0.27 + 0.54;
                let w = size * 0.27 + 0.04;
                let d = ((uv[0] - 0.5).powi(2) + (uv[1] - 0.5).powi(2)).sqrt();
                let k = ((d + 0.05) * 8.69565 - 1.).clamp(0., 1.);
                uv.map(|v| v + k * (v * (2. - 2. * z) + w - v))
            }
        }
    }
}

/// `HumanFace` makeup of the card: cheek (`mt_cheek`, uv1: R blush, B
/// highlight at its own placement), nose (`mt_nose`, uv2: R), lip and lip
/// line (`mt_lip` R with the B highlight, `mt_lipline`, uv3), as the
/// `AL/skin_head` forward program mixes them (`HumanFace.ChangeSettingCheek*`
/// write the card values to the material unchanged). SATD-amanatsu-port:
/// the nose gloss (A) and line (B), eye shadow (eyelid layer), face detail
/// and paint and moles are not drawn; highlights are added to the base
/// texture instead of lit. The expression flush is `flush_layer`.
fn face_makeup(lists: &Lists, face: &Value, makeup: Option<&Value>) -> Vec<MakeupLayer> {
    let mut out = Vec::new();
    let tex = |list: &str, id: i64, key: i32| lists.row(list, id).and_then(|r| r.asset(key::MAIN_AB, key));
    let id = |v: Option<&Value>, k: &str| v.and_then(|v| v.get(k)).and_then(Value::as_i64).unwrap_or(0);
    let num = |v: Option<&Value>, k: &str, d: f32| v.and_then(|v| v.get(k)).and_then(Value::as_f32).unwrap_or(d);
    if let Some(t) = tex("mt_cheek", id(makeup, "cheekId"), 2) {
        let pos = makeup.and_then(|m| m.get("cheekPos")).map(Value::as_array).filter(|a| a.len() >= 2).map_or([0.5; 2], |a| [a[0].as_f32().unwrap_or(0.5), a[1].as_f32().unwrap_or(0.5)]);
        let (size, rotation) = (num(makeup, "cheekSize", 1.), num(makeup, "cheekRotation", 0.5));
        out.push(MakeupLayer {
            texture: t.clone(),
            uv: 1,
            color: makeup.and_then(|m| color(m.get("cheekColor"))).unwrap_or([1., 0.5, 0.5, 0.5]),
            channel: 0,
            additive: false, color2: None, tinted: false,
            place: Some(Place::Cheek { pos, size, rotation, highlight: false }),
        });
        if let Some(c) = makeup.and_then(|m| color(m.get("cheekHighlightColor"))) {
            // The highlight placement is the material's (no card values).
            out.push(MakeupLayer { texture: t, uv: 1, color: c, channel: 2, additive: true, color2: None, tinted: false, place: Some(Place::Cheek { pos: [f32::NAN; 2], size: f32::NAN, rotation: f32::NAN, highlight: true }) });
        }
    }
    if let Some(t) = tex("mt_nose", id(Some(face), "noseId"), 43) {
        out.push(MakeupLayer { texture: t, uv: 2, color: color(face.get("noseColor")).unwrap_or([1.; 4]), channel: 0, additive: false, color2: None, tinted: false, place: Some(Place::Nose { scale: f32::NAN }) });
    }
    if let Some(t) = tex("mt_lip", id(makeup, "lipId"), 30) {
        out.push(MakeupLayer { texture: t.clone(), uv: 3, color: makeup.and_then(|m| color(m.get("lipColor"))).unwrap_or([1., 0.6, 0.6, 0.5]), channel: 0, additive: false, color2: None, tinted: false, place: None });
        if let Some(c) = makeup.and_then(|m| color(m.get("lipHighlightColor"))) {
            out.push(MakeupLayer { texture: t, uv: 3, color: c, channel: 2, additive: true, color2: None, tinted: false, place: None });
        }
    }
    if let Some(t) = tex("mt_lipline", id(Some(face), "lipLineId"), 29) {
        out.push(MakeupLayer { texture: t, uv: 3, color: color(face.get("lipLineColor")).unwrap_or([1., 0.6, 0.6, 1.]), channel: 0, additive: false, color2: None, tinted: false, place: None });
    }
    out
}

/// The `AL/hair` colour (decompiled forward program; the mask in uv0,
/// TEXCOORD0 → `v6.zw`): `_Main_color`, then `_Sub_color_02` by the mask's
/// G and `_Sub_color_01` by its B (each × its alpha), mixed in linear
/// space. SATD-amanatsu-port: `_Add_skin_color` (md_02 A), the `Ex` hair
/// colours and the shadow tint are not applied.
pub fn hair_colors(mask: &ImageSurface, main: [f32; 4], start: [f32; 4], end: [f32; 4]) -> ImageSurface {
    let lin = |c: [f32; 4]| [srgb_decode(c[0]), srgb_decode(c[1]), srgb_decode(c[2])];
    let (m, s1, s2) = (lin(main), lin(start), lin(end));
    let rgba = mask
        .rgba
        .chunks_exact(4)
        .flat_map(|p| {
            let (g, b) = (f32::from(p[1]) / 255., f32::from(p[2]) / 255.);
            let c: [f32; 3] = std::array::from_fn(|k| {
                let c = m[k] + (s2[k] - m[k]) * g * end[3];
                c + (s1[k] - c) * b * start[3]
            });
            let c = c.map(|v| (srgb_encode(v) * 255.).round() as u8);
            [c[0], c[1], c[2], 255]
        })
        .collect();
    ImageSurface { width: mask.width, height: mask.height, rgba }
}

/// The `AL/hair` gloss band baked into the uv0 colour texture: the gloss
/// texture is read at `(uv1.x, uv1.y · k − 0.56 · (1.5 − 1.2 · LL) + k ·
/// _hilightmove3)` with `k = 2.5 − 1.2 · _LightLength`; its R strokes mix
/// `glossColor` by `R · min(a, 0.9) · sat(r + g + b)`; its B band lifts
/// the saturation (`(c − lum) · 2 + lum`) by `B · _HairHigh2Multiply`
/// (linear space). SATD-amanatsu-port: the
/// view-dependent shift (normal × `_hilightmove`/`_hilightmove2`, view
/// parallax), the fresnel weight and card `glossSize` are not applied: the
/// band is fixed on the hair.
pub fn bake_hair_gloss(img: &mut ImageSurface, mesh: &crysirror_unity::unity6::Mesh6, gloss: &ImageSurface, colors: [[f32; 4]; 2], ll: f32, shift: f32, high2: f32) {
    let (Some(uv0), Some(uv1)) = (mesh.uv.first(), mesh.uv.get(1)) else { return };
    if uv0.len() != mesh.positions.len() || uv1.len() != uv0.len() || gloss.width == 0 || gloss.height == 0 {
        return;
    }
    let k = 2.5 - 1.2 * ll;
    let off = -0.56 * (1.5 - 1.2 * ll) + k * shift;
    let h1 = linear_color(colors[0]);
    let a1 = h1[3].clamp(0., 0.9) * (h1[0] + h1[1] + h1[2]).clamp(0., 1.);
    let (w, h) = (img.width as usize, img.height as usize);
    for t in mesh.indices.chunks_exact(3) {
        let p = [0, 1, 2].map(|i| uv0[t[i] as usize]);
        let q = [0, 1, 2].map(|i| uv1[t[i] as usize]);
        let d = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if d.abs() < 1e-12 {
            continue;
        }
        let lo = |k: usize| p.iter().map(|v| v[k]).fold(f32::MAX, f32::min);
        let hi = |k: usize| p.iter().map(|v| v[k]).fold(f32::MIN, f32::max);
        let (x0, x1) = (((lo(0) * w as f32) as i64).max(0), ((hi(0) * w as f32) as i64 + 1).min(w as i64 - 1));
        let (y0, y1) = (((lo(1) * h as f32) as i64).max(0), ((hi(1) * h as f32) as i64 + 1).min(h as i64 - 1));
        for py in y0..=y1 {
            for px in x0..=x1 {
                let (x, y) = ((px as f32 + 0.5) / w as f32, (py as f32 + 0.5) / h as f32);
                let b1 = ((x - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (y - p[0][1])) / d;
                let b2 = ((p[1][0] - p[0][0]) * (y - p[0][1]) - (x - p[0][0]) * (p[1][1] - p[0][1])) / d;
                let b0 = 1. - b1 - b2;
                if b0 < -1e-3 || b1 < -1e-3 || b2 < -1e-3 {
                    continue;
                }
                let v = [0, 1].map(|i| q[0][i] * b0 + q[1][i] * b1 + q[2][i] * b2);
                let (gu, gv) = (v[0], v[1] * k + off);
                if !(0. ..1.).contains(&gu) || !(0. ..1.).contains(&gv) {
                    continue;
                }
                let sx = ((gu * gloss.width as f32) as u32).min(gloss.width - 1);
                let sy = (((1. - gv) * gloss.height as f32) as u32).min(gloss.height - 1);
                let j = ((sy * gloss.width + sx) * 4) as usize;
                let (r, b) = (f32::from(gloss.rgba[j]) / 255., f32::from(gloss.rgba[j + 2]) / 255.);
                let o = ((h - 1 - py as usize) * w + px as usize) * 4;
                let lin: [f32; 3] = std::array::from_fn(|c| srgb_decode(f32::from(img.rgba[o + c]) / 255.));
                let lum = 0.212673 * lin[0] + 0.715152 * lin[1] + 0.072175 * lin[2];
                for c in 0..3 {
                    let mut v = lin[c];
                    v += ((v - lum) * 2. + lum - v).max(-v) * b * high2;
                    v += (h1[c] - v) * r * a1;
                    img.rgba[o + c] = (srgb_encode(v) * 255.).round() as u8;
                }
            }
        }
    }
}

/// Draw makeup layers into a face texture: every triangle is rasterised in
/// UV0 space and each layer sampled through its own UV set.
pub fn bake_makeup(img: &mut ImageSurface, mesh: &crysirror_unity::unity6::Mesh6, layers: &[(MakeupLayer, ImageSurface)], m: Option<&UnityMaterial>) {
    let (w, h) = (img.width as usize, img.height as usize);
    let uv0 = &mesh.uv[0];
    if w == 0 || h == 0 || uv0.len() != mesh.positions.len() {
        return;
    }
    for (layer, tex) in layers {
        let Some(uvk) = mesh.uv.get(layer.uv).filter(|u| u.len() == uv0.len()) else { continue };
        if tex.width == 0 || tex.height == 0 {
            continue;
        }
        let inside = |p: [f32; 2]| (0. ..=1.).contains(&p[0]) && (0. ..=1.).contains(&p[1]);
        // Material values stand in where the card has none (NaN).
        let get = |n: &str, d: f32| m.and_then(|m| m.floats.get(n).copied()).unwrap_or(d);
        let place = layer.place.map(|p| match p {
            Place::Cheek { highlight: true, .. } => Place::Cheek {
                pos: [get("_CheekHighlight_position_x", 0.666), get("_CheekHighlight_position_y", 0.743)],
                size: get("_CheekHighlight_size", 0.6),
                rotation: get("_CheekHighlight_rotation", 0.5),
                highlight: true,
            },
            Place::Nose { .. } => Place::Nose { scale: get("_NoseScale", 0.) },
            p => p,
        });
        for t in mesh.indices.chunks_exact(3) {
            let q = [0, 1, 2].map(|i| uvk[t[i] as usize]);
            if !q.iter().any(|p| inside(*p)) {
                continue;
            }
            let p = [0, 1, 2].map(|i| uv0[t[i] as usize]);
            let d = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
            if d.abs() < 1e-12 {
                continue;
            }
            let lo = |k: usize| p.iter().map(|v| v[k]).fold(f32::MAX, f32::min);
            let hi = |k: usize| p.iter().map(|v| v[k]).fold(f32::MIN, f32::max);
            let (x0, x1) = (((lo(0) * w as f32) as i64).max(0), ((hi(0) * w as f32) as i64).min(w as i64 - 1));
            let (y0, y1) = (((lo(1) * h as f32) as i64).max(0), ((hi(1) * h as f32) as i64).min(h as i64 - 1));
            for py in y0..=y1 {
                for px in x0..=x1 {
                    let (x, y) = ((px as f32 + 0.5) / w as f32, (py as f32 + 0.5) / h as f32);
                    let b1 = ((x - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (y - p[0][1])) / d;
                    let b2 = ((p[1][0] - p[0][0]) * (y - p[0][1]) - (x - p[0][0]) * (p[1][1] - p[0][1])) / d;
                    let b0 = 1. - b1 - b2;
                    if b0 < -1e-4 || b1 < -1e-4 || b2 < -1e-4 {
                        continue;
                    }
                    let mut v = [0, 1].map(|i| q[0][i] * b0 + q[1][i] * b1 + q[2][i] * b2);
                    if !inside(v) {
                        continue;
                    }
                    if let Some(p) = place {
                        v = p.apply(v, m);
                        if !inside(v) {
                            continue;
                        }
                    }
                    let sx = ((v[0] * tex.width as f32) as u32).min(tex.width - 1);
                    let sy = (((1. - v[1]) * tex.height as f32) as u32).min(tex.height - 1);
                    let j = ((sy * tex.width + sx) * 4) as usize;
                    let texel = [0, 1, 2, 3].map(|c| f32::from(tex.rgba[j + c]) / 255.);
                    let (mask, target) = match (layer.color2, layer.tinted) {
                        (Some(c2), _) => {
                            let t = texel[0] * c2[3].clamp(0., 1.);
                            (texel[3] * layer.color[3].clamp(0., 1.), [0, 1, 2].map(|c| srgb_decode(layer.color[c]) + (srgb_decode(c2[c]) - srgb_decode(layer.color[c])) * t))
                        }
                        (None, true) => (texel[3] * layer.color[3].clamp(0., 1.), [0, 1, 2].map(|c| texel[c] * srgb_decode(layer.color[c]))),
                        (None, false) => (texel[layer.channel.min(3)] * layer.color[3].clamp(0., 1.), [0, 1, 2].map(|c| srgb_decode(layer.color[c]))),
                    };
                    // Image rows run top-down; UV v runs bottom-up.
                    let o = ((h - 1 - py as usize) * w + px as usize) * 4;
                    // Mixed in linear space (URP linear project).
                    for c in 0..3 {
                        let mut out = srgb_decode(f32::from(img.rgba[o + c]) / 255.);
                        let col = target[c];
                        if layer.additive {
                            out += col * mask;
                        } else {
                            out += (col - out) * mask;
                        }
                        img.rgba[o + c] = (srgb_encode(out) * 255.).round() as u8;
                    }
                }
            }
        }
    }
}

/// `HumanFace.ChangeSettingEyes`/`ChangeSettingEyeHL*` values of one eye.
#[derive(Clone, Debug, Default)]
struct EyeLayers {
    /// `mt_eyepipil` texture (`overId`), its R/G/B colours
    /// (`pupil01..03Color`) and `_Pipil_size` (`_eyePupilSizeRange`).
    pupil: Option<(String, String)>,
    pupil_colors: Vec<[f32; 4]>,
    pupil_size: [f32; 2],
    /// `_Eye_position`: iris size (`_eyeSizeRange` of `eyeWidth`/
    /// `eyeHeight`) and offset (`_eyeLookRange` of `eyeX`/`eyeY`, mirrored
    /// for the right eye).
    iris: [f32; 4],
    /// `mt_eye_hi_up` highlights: texture, `_Highlight_0N_position`
    /// (size x/y from `_eyeHLSizeRange`, offset x − 0.5, y − 0.5),
    /// rotation, `color` (R core) and `color2` (G glow).
    highlights: Vec<(Option<(String, String)>, [f32; 4], f32, [f32; 4], [f32; 4])>,
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
    /// Clothing parts with their state variants (`HumanCloth`).
    pub clothes: Vec<ClothPart>,
    /// Accessory part root names (exported unique, `acs{slot}#root`).
    pub accessories: Vec<String>,
    /// Every accessory slot of the coordinate: the list name of its part
    /// (`ListInfoBase` `Name`) when one is equipped.
    pub accessory_slots: Vec<Option<String>>,
    /// The renderer nodes of the grafted head with its hair
    /// (`CharaVisible` `Head`; the head root is merged into the body's
    /// bones on export).
    pub head: Vec<String>,
    /// `HumanGraphic` (card `Graphic`): the `mt_ramp` texture every
    /// character material takes as `_Lamp` (`ChangeRampTexture`), and
    /// `ShadowDepth`/`LineWidth` written to `_Shadow_power`/`_Line_width`.
    pub ramp: Option<(String, String)>,
    pub shadow_depth: f32,
    pub line_width: f32,
    pub log: Vec<String>,
}

/// A clothing part (`ChaFileClothes` slot) for `HumanCloth.SetClothesState`:
/// the node names of its state variants `n_*_a`/`_b`/`_c` in order and of
/// the part root (made unique per part: `{list}#{name}`).
#[derive(Clone, Debug)]
pub struct ClothPart {
    pub list: &'static str,
    pub root: String,
    pub variants: Vec<String>,
}

impl ClothPart {
    /// The number of states: each variant, then off.
    pub fn states(&self) -> usize {
        self.variants.len().max(1) + 1
    }

    /// The node names hidden in `state` (0 dressed, the last off).
    pub fn hidden(&self, state: usize) -> Vec<String> {
        if state + 1 >= self.states() {
            return vec![self.root.clone()];
        }
        self.variants.iter().enumerate().filter(|(i, _)| *i != state).map(|(_, v)| v.clone()).collect()
    }
}

impl Human {
    /// Export every clothing state variant (for runtime states through
    /// `AnimationState.hidden` instead of the import-time dress).
    pub fn undress_all(&mut self) {
        let mut names: std::collections::HashSet<String> = self.clothes.iter().flat_map(|c| c.variants.iter().cloned()).collect();
        // Tears, shown by the face state; the body state objects.
        names.extend(crate::face::TEARS.iter().map(|n| (*n).to_owned()));
        names.insert(SON.to_owned());
        names.insert(MOZ.to_owned());
        for n in self.scene.nodes.iter_mut().filter(|n| names.contains(&n.name)) {
            n.active = true;
        }
    }
}

fn color(v: Option<&Value>) -> Option<[f32; 4]> {
    let a = v?.as_array();
    (a.len() >= 3).then(|| [0, 1, 2, 3].map(|i| a.get(i).and_then(Value::as_f32).unwrap_or(1.)))
}

/// The first coordinate's blocks (`Clothes`, `Hair`, …).
/// `HumanCoordinate.Define.CoordinateType`: `Swimsuit` 0, `AfterBath` 1
/// (the card's `Coordinate` entries in that order).
pub const COORDINATE_TYPES: [&str; 2] = ["Swimsuit", "AfterBath"];

/// The card's coordinate `index` (the first when the card has fewer).
fn coordinate_at(card: &Card, index: usize) -> Result<Card> {
    let v = crysirror_unity::msgpack::decode(card.block("Coordinate").context("card Coordinate")?)?;
    let list = v.as_array();
    let entry = list.get(index).or(list.first()).and_then(Value::as_bytes).context("card coordinate")?;
    Ok(Card::container(entry)?.1)
}

impl Human {
    /// Assemble the card's character in its first coordinate (`Swimsuit`).
    pub fn from_card(unity: &mut Unity, lists: &Lists, card: &Card, female: bool) -> Result<Self> {
        Self::from_card_coordinate(unity, lists, card, female, 0)
    }

    /// Assemble the card's character in coordinate `coordinate`
    /// (`COORDINATE_TYPES`).
    pub fn from_card_coordinate(unity: &mut Unity, lists: &Lists, card: &Card, female: bool, coordinate: usize) -> Result<Self> {
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
        let shadow = color(body_data.get("skinShadowColor"));
        // `AL/skin_body` body detail layers of the card: `mt_nip` (uv2,
        // `areolaSize`, `nipColor` → `nipColor2` by R, alpha A) and
        // `mt_underhair` (uv3, its colour × `underhairColor`).
        let mut detail = Vec::new();
        let bid = |k: &str| body_data.get(k).and_then(Value::as_i64).unwrap_or(0);
        if let Some(t) = lists.row("mt_nip", bid("nipId")).and_then(|r| r.asset(key::MAIN_AB, 40)) {
            let size = body_data.get("areolaSize").and_then(Value::as_f32).unwrap_or(0.5);
            detail.push(MakeupLayer { texture: t, uv: 2, color: color(body_data.get("nipColor")).unwrap_or([1., 0.7, 0.7, 1.]), channel: 3, additive: false, color2: Some(color(body_data.get("nipColor2")).unwrap_or([1., 0.6, 0.6, 1.])), tinted: false, place: Some(Place::Nip { size }) });
        }
        if let Some(t) = lists.row("mt_underhair", bid("underhairId")).and_then(|r| r.asset(key::MAIN_AB, 61)) {
            detail.push(MakeupLayer { texture: t, uv: 3, color: color(body_data.get("underhairColor")).unwrap_or([0.2, 0.15, 0.1, 1.]), channel: 3, additive: false, color2: None, tinted: true, place: None });
        }
        let mut body_meshes = Vec::new();
        if !detail.is_empty() {
            let renderers: Vec<PPtr> = scene.nodes.iter().filter(|n| n.name.starts_with("o_") && !HIDDEN_BODY.contains(&n.name.as_str())).flat_map(|n| n.components.iter().filter(|c| c.class_id == 137).filter_map(|c| c.fields.pptr("m_Mesh"))).collect();
            for p in renderers {
                match unity.target(&body.key, p).and_then(|(f, id)| SceneAssets::mesh(&mut *unity, &f, id)) {
                    Ok(m) => body_meshes.push(m),
                    Err(e) => log.push(format!("body detail mesh: {e:#}")),
                }
            }
        }
        let body_makeup = (!detail.is_empty() && !body_meshes.is_empty()).then(|| std::sync::Arc::new(Makeup { meshes: body_meshes, layers: detail }));
        looks.insert(body.key.clone(), PartLook { shaders: &["AL/skin_body"], main: body_row.asset(key::MAIN_TEX_AB, key::MAIN_TEX), colors: vec![skin], shadow, makeup: body_makeup, ..Default::default() });
        for name in HIDDEN_BODY {
            for n in scene.nodes.iter_mut().filter(|n| n.name == name) {
                n.active = false;
            }
        }
        let skeleton = scene.nodes.iter().position(|n| n.name == "cf_j_root").context("body cf_j_root")?;
        // Body physics: the bone prefab's MagicaCloth springs and colliders
        // on the body skeleton, tuned by the card's softness values.
        // Only the female body has a bone prefab.
        if female {
            let bone = Level::prefab(unity, BODY_BONE.0, BODY_BONE.1)?;
            let bone_root = scene.graft(&bone.scene, &bone.key, 0);
            scene.alias_bones(bone_root, skeleton);
            match shake_correct(unity) {
                Ok(shake) => log.push(format!("body springs: {} components tuned", apply_shake(&mut scene, bone_root, &shake, body_data))),
                Err(e) => log.push(format!("body springs untuned: {e:#}")),
            }
        }
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
        let lerp = |a: f32, b: f32, v: f32| a + (b - a) * v.clamp(0., 1.);
        let num = |v: Option<&Value>, d: f32| v.and_then(Value::as_f32).unwrap_or(d);
        let mut eyes = Vec::new();
        for (lr, p) in face.get("pupil").map(Value::as_array).unwrap_or(&[]).iter().take(2).enumerate() {
            // `CmpEyeLookMaterial` ranges: size 4.0→0.9 (x) / 4.0→1.3 (y);
            // look x −0.2→0.11 (left) / −0.11→0.2 (right, value mirrored),
            // y −0.07→0.15.
            let ex = num(face.get("eyeX"), 0.5);
            let iris = [
                lerp(4.0, 0.9, num(face.get("eyeWidth"), 0.5)),
                lerp(4.0, 1.3, num(face.get("eyeHeight"), 0.5)),
                if lr == 0 { lerp(-0.2, 0.11, ex) } else { lerp(-0.11, 0.2, 1. - ex) },
                lerp(-0.07, 0.15, num(face.get("eyeY"), 0.5)),
            ];
            // `CmpEyeLookMaterial` ranges of `p_cf_al_head_00`: pupil
            // 2.0→0.2 (width, height), highlight 3.0→0.5.
            let pupil_size = [lerp(2.0, 0.2, num(face.get("pupilWidth"), 0.5)), lerp(2.0, 0.2, num(face.get("pupilHeight"), 0.5))];
            let highlights = p
                .get("highlightInfos")
                .map(Value::as_array)
                .unwrap_or(&[])
                .iter()
                .take(3)
                .map(|h| {
                    let id = h.get("id").and_then(Value::as_i64).unwrap_or(-1);
                    let tex = lists.row("mt_eye_hi_up", id).and_then(|r| r.asset(key::MAIN_AB, key::EYE_HI_UP_TEX));
                    let pos = [lerp(3.0, 0.5, num(h.get("width"), 0.5)), lerp(3.0, 0.5, num(h.get("height"), 0.5)), num(h.get("x"), 0.5).clamp(0., 1.) - 0.5, num(h.get("y"), 0.5).clamp(0., 1.) - 0.5];
                    (tex, pos, num(h.get("rotation"), 0.5), color(h.get("color")).unwrap_or([1.; 4]), color(h.get("color2")).unwrap_or([1.; 4]))
                })
                .collect();
            let over = p.get("overId").and_then(Value::as_i64).unwrap_or(-1);
            eyes.push(EyeLayers {
                pupil: lists.row("mt_eyepipil", over).and_then(|r| r.asset(key::MAIN_AB, key::EYE_PIPIL_TEX)),
                pupil_colors: ["pupil01Color", "pupil02Color", "pupil03Color"].iter().map(|k| color(p.get(k)).unwrap_or([0., 0., 0., 1.])).collect(),
                pupil_size,
                iris,
                highlights,
            });
        }
        let makeup_block = coordinate_at(card, coordinate).ok().and_then(|c| c.block("FaceMakeup").and_then(|b| crysirror_unity::msgpack::decode(b).ok()));
        let makeup_layers = face_makeup(lists, face, makeup_block.as_ref());
        let face_mesh = head.scene.nodes.iter().find(|n| n.name == "cf_O_face").and_then(|n| n.components.iter().find(|c| c.class_id == 137)).and_then(|c| c.fields.pptr("m_Mesh"));
        let makeup = match face_mesh {
            Some(p) if !makeup_layers.is_empty() => {
                let (f, id) = unity.target(&head.key, p)?;
                Some(std::sync::Arc::new(Makeup { meshes: vec![SceneAssets::mesh(&mut *unity, &f, id)?], layers: makeup_layers }))
            }
            _ => None,
        };
        log.push(format!("face makeup: {:?}", makeup.as_ref().map(|m| m.layers.iter().map(|l| l.texture.1.clone()).collect::<Vec<_>>())));
        looks.insert(head.key.clone(), PartLook { shaders: &["AL/skin_head"], main: head_row.asset(key::MAIN_TEX_AB, key::MAIN_TEX), colors: vec![skin], layers, shadow, eyes, makeup, ..Default::default() });
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
        let coord = coordinate_at(card, coordinate)?;
        // Hair.
        let hair = crysirror_unity::msgpack::decode(coord.block("Hair").context("coordinate Hair")?)?;
        let gloss = hair.get("glossId").and_then(Value::as_i64).and_then(|id| lists.row("mt_hairgloss", id)).and_then(|r| r.asset(key::MAIN_TEX_AB, key::MAIN_TEX));
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
                    // The part's own renderers (outline shells excepted).
                    let mut meshes = Vec::new();
                    for n in p.scene.nodes.iter().filter(|n| !n.name.ends_with("_line")) {
                        for mp in n.components.iter().filter(|c| c.class_id == 137 || c.class_id == 33).filter_map(|c| c.fields.pptr("m_Mesh")) {
                            if let Ok((f, id)) = unity.target(&p.key, mp) {
                                if let Ok(m) = SceneAssets::mesh(&mut *unity, &f, id) {
                                    meshes.push(m);
                                }
                            }
                        }
                    }
                    let gloss_colors = [color(part.get("glossColor")).unwrap_or([1.; 4]), color(part.get("glossColor2")).unwrap_or([1., 1., 1., 0.])];
                    let hair_look = HairLook { gloss: gloss.clone(), gloss_colors, meshes, start: color(part.get("startColor")).unwrap_or([1.; 4]), end: color(part.get("endColor")).unwrap_or([1.; 4]) };
                    looks.insert(p.key.clone(), PartLook { shaders: &["AL/hair"], colors: c.into_iter().collect(), outline, hair: Some(std::sync::Arc::new(hair_look)), ..Default::default() });
                    let r = scene.graft(&p.scene, &p.key, hair_parent);
                    scene.alias_bones(r, head_root);
                    log.push(format!("{list} {id}: {data}"));
                }
                Err(e) => log.push(format!("{list} {id}: {e:#}")),
            }
        }
        // Clothes.
        let mut clothes_parts: Vec<ClothPart> = Vec::new();
        let clothes = &mut clothes_parts;
        let clothes_block = crysirror_unity::msgpack::decode(coord.block("Clothes").context("coordinate Clothes")?)?;
        for (i, part) in clothes_block.get("parts").map(Value::as_array).unwrap_or(&[]).iter().enumerate() {
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
                    let hide: Vec<bool> = part.get("hideOpt").map(Value::as_array).unwrap_or(&[]).iter().map(|v| v.as_bool().unwrap_or(false)).collect();
                    dress(&mut scene, r, &hide);
                    // Unique names for the part's state nodes.
                    let mut stack = vec![r];
                    let mut variants: Vec<(String, usize)> = Vec::new();
                    while let Some(n) = stack.pop() {
                        stack.extend(&scene.nodes[n].children);
                        let name = scene.nodes[n].name.clone();
                        let b = name.as_bytes();
                        if name.starts_with("n_") && b.len() >= 4 && b[b.len() - 2] == b'_' && matches!(b[b.len() - 1], b'a' | b'b' | b'c') {
                            let unique = format!("{list}#{name}");
                            scene.nodes[n].name = unique.clone();
                            variants.push((unique, n));
                        }
                    }
                    variants.sort();
                    // One variant set per part: the first letter group.
                    let root = format!("{list}#root");
                    scene.nodes[r].name = root.clone();
                    clothes.push(ClothPart { list, root, variants: variants.into_iter().map(|v| v.0).collect() });
                    log.push(format!("{list} {id}: {ab} {data} ({n} bones)"));
                }
                Err(e) => log.push(format!("{list} {id}: {e:#}")),
            }
        }
        // Accessories (`HumanAccessory.ChangeAccessory`): the slot's list
        // prefab under its parent node (the card's `parentKeyType`, else the
        // list's), the card's two corrections on `N_move`/`N_move2`
        // (`UpdateAccessoryMoveFromInfo`: position × 0.1, Euler degrees,
        // scale) and the card colours through the material's colour mask.
        // SATD-amanatsu-port: `DynamicBone` swing, hide categories, visible
        // timings, patterns, gloss/metallic and hair-coloured accessories
        // are not applied.
        let acs = crysirror_unity::msgpack::decode(coord.block("Accessory").context("coordinate Accessory")?)?;
        let mut accessories = Vec::new();
        let mut accessory_slots = vec![None; acs.get("parts").map(Value::as_array).map_or(0, |a| a.len())];
        let head_origin = scene.nodes[head_root].origin;
        for (slot, part) in acs.get("parts").map(Value::as_array).unwrap_or(&[]).iter().enumerate() {
            let ty = part.get("type").and_then(Value::as_i64).unwrap_or(120);
            let Some(list) = usize::try_from(ty - 121).ok().and_then(|i| ACCESSORY_LISTS.get(i)) else { continue };
            let id = part.get("id").and_then(Value::as_i64).unwrap_or(0);
            let Some(row) = lists.row(list, id) else {
                log.push(format!("accessory {slot} {list} {id}: no list row"));
                continue;
            };
            let Some((ab, data)) = row.asset(key::MAIN_AB, key::MAIN_DATA) else { continue };
            let key_index = part.get("parentKeyType").and_then(Value::as_i64).unwrap_or(0);
            let parent_name = usize::try_from(key_index).ok().and_then(|i| ACCESSORY_PARENTS.get(i)).filter(|n| !n.is_empty()).map(|n| (*n).to_owned()).or_else(|| row.get(key::ACCESSORY_PARENT).map(str::to_owned));
            let Some(parent) = parent_name.as_deref().and_then(|name| scene.nodes.iter().position(|n| n.name == name && (n.origin == 0 || n.origin == head_origin))) else {
                log.push(format!("accessory {slot} {list} {id}: parent {parent_name:?} not found"));
                continue;
            };
            match Level::prefab(unity, &ab, &data) {
                Ok(p) => {
                    let colors = part.get("color").map(Value::as_array).unwrap_or(&[]).iter().filter_map(|c| color(Some(c))).collect();
                    looks.insert(p.key.clone(), PartLook { shaders: &["AL/acs"], colors, material_mask: true, ..Default::default() });
                    let r = scene.graft(&p.scene, &p.key, parent);
                    let origin = scene.nodes[r].origin;
                    let root_name = format!("acs{slot}#root");
                    scene.nodes[r].name = root_name.clone();
                    accessories.push(root_name);
                    accessory_slots[slot] = Some(row.get(key::NAME).unwrap_or("").to_owned());
                    let moves = part.get("addMove").map(Value::as_array).and_then(|a| a.get(2)).map(Value::as_array).unwrap_or(&[]);
                    let v3 = |i: usize, d: f32| moves.get(i).map(Value::as_array).filter(|a| a.len() >= 3).map_or([d; 3], |a| [0, 1, 2].map(|k| a[k].as_f32().unwrap_or(d)));
                    for (k, name) in ["N_move", "N_move2"].iter().enumerate() {
                        let Some(n) = scene.nodes.iter().position(|n| n.origin == origin && n.name == *name) else { continue };
                        let (pos, rot, scale) = (v3(k * 3, 0.), v3(k * 3 + 1, 0.), v3(k * 3 + 2, 1.));
                        let q = glam::Quat::from_euler(glam::EulerRot::YXZ, rot[1].to_radians(), rot[0].to_radians(), rot[2].to_radians());
                        let x = &mut scene.nodes[n].xform;
                        x.position = pos.map(|v| v * 0.1);
                        x.rotation = q.to_array();
                        x.scale = scale;
                    }
                    log.push(format!("accessory {slot} {list} {id}: {data} on {}", parent_name.unwrap_or_default()));
                }
                Err(e) => log.push(format!("accessory {slot} {list} {id}: {e:#}")),
            }
        }
        // Shadow-caster proxies (`o_body_shadow`, `cf_O_face_shadow`) only
        // feed Unity's shadow pass.
        for n in scene.nodes.iter_mut().filter(|n| n.name.ends_with("_shadow")) {
            n.active = false;
        }
        let height = body_data.get("shapeValueBody").map(Value::as_array).and_then(|a| a.first()).and_then(Value::as_f32).unwrap_or(0.5);
        let mut head = Vec::new();
        let mut stack = vec![head_root];
        while let Some(n) = stack.pop() {
            if scene.nodes[n].components.iter().any(|c| matches!(c.class_id, 23 | 137)) {
                head.push(scene.nodes[n].name.clone());
            }
            stack.extend(&scene.nodes[n].children);
        }
        let (ramp_id, shadow_depth, line_width) = card.graphic();
        let ramp = lists.row("mt_ramp", ramp_id).and_then(|r| r.asset(key::MAIN_AB, key::MAIN_TEX_AB));
        if ramp.is_none() {
            log.push(format!("mt_ramp {ramp_id}: no texture"));
        }
        Ok(Self { key: body.key, female, height, scene, looks, clothes: clothes_parts, accessories, accessory_slots, head, ramp, shadow_depth, line_width, log })
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
        let (bundle, name) = motion_controller(self.female);
        self.controller_tracks(unity, &bundle, name, state)
    }

    /// A state's own (first) clip of a controller, named `name` in the GLB
    /// (H postures keep their `_L`/`_M`/`_S` variants as separate states).
    pub fn exact_state_tracks(&self, unity: &mut Unity, bundle: &str, controller: &str, state: &str, name: &str) -> Result<crysirror_unity::world6::ClipTracks> {
        use crysirror_unity::animator::{Clip, Controller};
        let (k, id) = unity.bundle_asset(bundle, controller, 91)?.with_context(|| format!("controller {controller}"))?;
        let ctl = Controller::decode(&unity.decode(&k, id)?)?;
        let si = ctl.find_state(0, state).with_context(|| format!("state {state}"))?;
        let cp = *ctl.clips.get(ctl.layers[0].states[si].clip.context("state motion")?).context("state clip")?;
        let (f, cid) = unity.target(&k, cp)?;
        let clip = Clip::decode(&unity.decode(&f, cid)?)?;
        Ok(crysirror_unity::world6::clip_tracks(&self.scene, 0, &clip, name, 30.))
    }

    /// The H height suffix of the card (`_S`, `_M`, `_L`).
    pub fn height_suffix(&self) -> &'static str {
        if self.height < 1. / 3. {
            "S"
        } else if self.height < 2. / 3. {
            "M"
        } else {
            "L"
        }
    }

    /// A state's motion of any controller (`bundle`, controller asset) with
    /// the `_S`/`_M`/`_L` height pick (oil poses, H postures).
    pub fn controller_tracks(&self, unity: &mut Unity, bundle: &str, name: &str, state: &str) -> Result<crysirror_unity::world6::ClipTracks> {
        use crysirror_unity::animator::{Clip, Controller};
        let (k, id) = unity.bundle_asset(bundle, name, 91)?.with_context(|| format!("controller {name}"))?;
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
    /// The scene's ambient (sky) intensity: hair and clothes take their
    /// shadow tone from the ambient light.
    pub ambient: f32,
}

impl CharaAssets<'_> {
    /// The base texture/colour of a character material.
    fn base_look(&mut self, file: &str, m: &UnityMaterial, look: &PartLook, shader: &str, k: f32, mut o: MaterialOverride) -> Result<MaterialOverride> {
        if let Some(main) = &look.main {
            if let Some(mut img) = self.texture_named(main)? {
                let mask = match &look.mask {
                    Some(mk) => self.mask_named(mk)?,
                    None => None,
                };
                tint_by_mask(&mut img, mask.as_ref(), &look.colors);
                if let Some(makeup) = look.makeup.as_ref().filter(|_| shader.starts_with("AL/skin_head") || shader.starts_with("AL/skin_body")) {
                    let mut layers = Vec::new();
                    for l in &makeup.layers {
                        if let Some(t) = self.mask_named(&l.texture)? {
                            layers.push((l.clone(), t));
                        }
                    }
                    for mesh in &makeup.meshes {
                        bake_makeup(&mut img, mesh, &layers, Some(m));
                    }
                }
                // Texture alpha cuts openings (the face's eye holes).
                if img.rgba.chunks_exact(4).any(|p| p[3] < 128) {
                    o.alpha_mode = Some(AlphaMode::Mask);
                }
                o.base_texture = Some((format!("{}:{}:{}", main.0, main.1, m.name), img));
                o.base_color = Some([k, k, k, 1.]);
            }
        } else if let Some(mask) = m.textures.get("_Main_color_mask").filter(|t| look.material_mask && t.texture.path_id != 0) {
            // White tinted through the material's colour mask.
            let (f, id) = self.unity.target(if m.file.is_empty() { file } else { &m.file }, mask.texture)?;
            let mask = self.mask(&f, id)?;
            let mut img = ImageSurface { width: mask.width, height: mask.height, rgba: vec![255; mask.rgba.len()] };
            tint_by_mask(&mut img, Some(&mask), &look.colors);
            o.base_texture = Some((format!("{file}:{}:acs", m.name), img));
            o.base_color = Some([k, k, k, 1.]);
        } else if let (Some(hair), Some(mask)) = (look.hair.as_ref(), m.textures.get("_Hair_color_mask").filter(|t| t.texture.path_id != 0)) {
            let (f, id) = self.unity.target(if m.file.is_empty() { file } else { &m.file }, mask.texture)?;
            let mask = self.mask(&f, id)?;
            let main = look.colors.first().copied().unwrap_or([1.; 4]);
            let mut img = hair_colors(&mask, main, hair.start, hair.end);
            if let Some(g) = hair.gloss.as_ref().map(|g| self.mask_named(g)).transpose()?.flatten() {
                let ll = m.floats.get("_LightLength").copied().unwrap_or(0.03);
                let shift = m.floats.get("_hilightmove3").copied().unwrap_or(0.);
                for mesh in &hair.meshes {
                    bake_hair_gloss(&mut img, mesh, &g, hair.gloss_colors, ll, shift, m.floats.get("_HairHigh2Multiply").copied().unwrap_or(0.25));
                }
            }
            // Soft gradients: 512² is ample (prepared-asset budget).
            o.base_texture = Some((format!("{file}:{}:hair", m.name), halve_to(img, 512)));
            o.base_color = Some([k, k, k, 1.]);
        } else if let Some(c) = look.colors.first() {
            // Toon shading works on display-referred colours.
            o.base_color = Some([c[0] * k, c[1] * k, c[2] * k, 1.]);
        }
        Ok(o)
    }

    /// A mask texture as the shaders sample it: RGB linearised when the
    /// texture is sRGB (`m_ColorSpace` 1; alpha is always linear).
    fn mask(&mut self, file: &str, id: i64) -> Result<ImageSurface> {
        let mut img = SceneAssets::texture(&mut *self.unity, file, id)?;
        if self.unity.decode(file, id)?.int("m_ColorSpace") == Some(1) {
            let lut: [u8; 256] = std::array::from_fn(|v| (srgb_decode(v as f32 / 255.) * 255.).round() as u8);
            for p in img.rgba.chunks_exact_mut(4) {
                for c in &mut p[..3] {
                    *c = lut[*c as usize];
                }
            }
        }
        Ok(img)
    }

    fn mask_named(&mut self, (bundle, name): &(String, String)) -> Result<Option<ImageSurface>> {
        let Some((k, id)) = self.unity.bundle_asset(bundle, name, crysirror_unity::class_id::TEXTURE_2D)? else { return Ok(None) };
        Ok(Some(self.mask(&k, id)?))
    }

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
        // `AL/sub/mnpb`: the censor plane, unlit screen-space block noise
        // (`_SP1` scale, `_TimeParameters.x · 3` steps, `_Color`/`_Color2`,
        // `_WhiteOff`).
        if shader == "AL/sub/mnpb" {
            let lin = |k: &str, d: [f32; 4]| { let c = linear_color(m.colors.get(k).copied().unwrap_or(d)); [c[0], c[1], c[2]] };
            let f = |k: &str, d: f32| m.floats.get(k).copied().unwrap_or(d);
            return Ok(Some(MaterialOverride {
                unlit_effect: Some(crysirror_unity::world6::ShaderEffect::BlockNoise {
                    scale: f("_SP1", 6.).clamp(0.001, 1000.),
                    rate: 3.,
                    color_a: lin("_Color", [0.9, 0.66, 0.63, 1.]),
                    color_b: lin("_Color2", [0.9, 0.45, 0.49, 1.]),
                    white: f("_WhiteOff", 0.).clamp(0., 1.),
                }),
                ..Default::default()
            }));
        }
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
                Some(t) => match self.mask_named(t)? {
                    Some(mask) => {
                        let mut img = ImageSurface { width: mask.width, height: mask.height, rgba: mask.rgba.iter().enumerate().map(|(i, a)| if i % 4 == 3 { *a } else { 255 }).collect() };
                        if shader == "AL/eye" {
                            // Iris (`AL/eye` forward program).
                            iris_colors(&mut img, &mask, colors, m.colors.get("_Line_color").copied().unwrap_or([0., 0., 0., 1.]));
                            // Materials `cf_m_hitomi_00_L`/`_R` take the card's
                            // first/second pupil entry.
                            let eye = look.eyes.get(usize::from(m.name.ends_with("_R"))).or(look.eyes.first()).cloned();
                            let mut highlights = Vec::new();
                            if let Some(eye) = eye {
                                // `AL/eye` forward program: the iris samples
                                // `(uv + off − 0.5) · S + 0.5` with
                                // `S = (_Eye_position.xy + 0.2) · _Eye_position_add.xy`,
                                // `off = _Eye_position.zw − (0.05, 0.1) + 0.1 · rebers · x̂ + _Eye_position_add.zw`
                                // (card values in `_Eye_position`); the pupil
                                // `(uv + off − 0.5) · _Pipil_size + 0.5`; highlight N
                                // `(uv + off + 0.5 · H.zw − 0.5) · H.xy · _Eye_position_add.xy + 0.5`.
                                // SATD-amanatsu-port: `_rotation`, parallax,
                                // `_Eye_open`, the gradation and the highlights'
                                // unfollow terms are not applied.
                                let add = m.colors.get("_Eye_position_add").copied().unwrap_or([0.5, 0.5, 0., 0.]);
                                let rebers = m.floats.get("_Eye_postion_rebers").copied().unwrap_or(0.);
                                let off = [eye.iris[2] - 0.05 + 0.1 * rebers + add[2], eye.iris[3] - 0.1 + add[3]];
                                let iris = [(eye.iris[0] + 0.2) * add[0], (eye.iris[1] + 0.2) * add[1], -off[0], -off[1]];
                                img = resample_layer(&img, img.width, img.height, iris);
                                if let Some(t) = &eye.pupil {
                                    if let Some(p) = self.mask_named(t)? {
                                        pupil_over(&mut img, &p, [eye.pupil_size[0], eye.pupil_size[1], -off[0], -off[1]], &eye.pupil_colors);
                                    }
                                }
                                for (t, pos, rot, c1, c2) in &eye.highlights {
                                    if let Some(h) = t.as_ref().map(|t| self.mask_named(t)).transpose()?.flatten() {
                                        highlights.push((h, [pos[0] * add[0], pos[1] * add[1], -off[0] - 0.5 * pos[2], -off[1] - 0.5 * pos[3]], *rot, *c1, *c2));
                                    }
                                }
                            }
                            // The iris covers the sclera at its alpha
                            // (`AL/eye` forward program): opaque.
                            if let Some(mask) = m.textures.get("_Shadow_mask").filter(|t| t.texture.path_id != 0) {
                                let (f, id) = self.unity.target(if m.file.is_empty() { file } else { &m.file }, mask.texture)?;
                                let mask = self.mask(&f, id)?;
                                sclera_under(&mut img, &mask, m);
                                for (h, pos, rot, c1, c2) in &highlights {
                                    highlight_over(&mut img, h, *pos, *rot, *c1, *c2);
                                }
                                // A 512² eye is ample on screen (budget).
                                let img = halve_to(img, 512);
                                o.base_texture = Some((format!("{}:{}:{}", t.0, t.1, m.name), img));
                                // Shaded like the other AL materials (no
                                // `_Lamp`: a soft terminator).
                                let k = self.light;
                                let floor = 1. - (m.floats.get("_Shadow_power").copied().unwrap_or(0.5) * 0.4 + 0.5).clamp(0., 1.);
                                o.toon_shade_from_base = true;
                                o.toon_light_limit = Some(1.);
                                o.toon = Some((toon_ramp_floor(&ramp_curve(None), [0.; 3], floor, false), 0., [0., 0., 0., 1.]));
                                o.base_color = Some([k, k, k, 1.]);
                                return Ok(Some(o));
                            }
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
        // The material's `_Lamp` ramp (G: shadow amount over the half
        // Lambert term) between the full colour and a shadow tone. The AL
        // shaders' forward programs (DXBC, see the validation note) shade
        // `base · mix(tone, 1, max(lit, 1 − sat(_Shadow_power · 0.4 + 0.5)))`
        // with `tone` the base colour's HSV shade, or `_Shade_color` with
        // `_UseShadowColor`, under the key light clamped to 1. The skin
        // programs (checked against their DXBC on a CPU interpreter) use the
        // `_Lamp` blue channel at `sat((n·l + 0.6) / 1.6)` gated by
        // `sat((n·l − 0.05) · 5)`, mix the base shade with the ambient tone
        // and light it with a fixed bluish unlit-side colour
        // (`ToonShading::shadow_light`).
        // SATD-amanatsu-port: the skin base pre-grade (an overlay layer that
        // warms and lifts mid tones), SSS, shine and rim terms are not drawn.
        // `HumanGraphic.ChangeRampTexture`: the card's ramp replaces the
        // material's `_Lamp`.
        let lamp = match (&self.human.ramp, m.textures.get("_Lamp").filter(|t| t.texture.path_id != 0)) {
            (Some(r), _) => self.mask_named(&r.clone())?,
            (None, Some(t)) => {
                let (f, id) = self.unity.target(if m.file.is_empty() { file } else { &m.file }, t.texture)?;
                Some(self.mask(&f, id)?)
            }
            (None, None) => None,
        };
        let skin = shader.starts_with("AL/skin");
        let curve = if m.floats.contains_key("_Shadow_power") { skin_ramp_curve(lamp.as_ref()) } else { ramp_curve(lamp.as_ref()) };
        // `ChangeShadowDepth`: the card's depth replaces `_Shadow_power`.
        if let Some(power) = m.floats.get("_Shadow_power").map(|_| &self.human.shadow_depth) {
            let floor = 1. - (power * 0.4 + 0.5).clamp(0., 1.);
            let shade = m.colors.get("_Shade_color").filter(|_| m.floats.get("_UseShadowColor").is_some_and(|u| *u > 0.5));
            // Skin derives the tone from its base colour; hair and clothes
            // from the ambient light (`HSV(SH · 0.7 + 0.3)` with the
            // saturation × 1.5: grey for the engine's scalar ambient).
            // SATD-amanatsu-port: the ambient's hue is not available here.
            // (A linear factor; the ramp multiplies display-referred colour.)
            let ambient = (!skin).then(|| [srgb_encode(self.ambient * 0.7 + 0.3); 3]);
            // The programs (checked on their DXBC) mix a base shade with the
            // SH ambient tone and light the result themselves: a bluish
            // light on the unlit side and no ambient term, so the base needs
            // no compensation for the engine's ambient. The base shade is the
            // base colour's HSV shade on skin, a blue-leaning shade of it on
            // clothes, the material's `_Shadow_color` on hair, and
            // `_Shade_color` with `_UseShadowColor` (which replaces the whole
            // tone in the original).
            use crysirror_unity::world6::ToonShadeKind;
            let lin3 = |c: &[f32; 4]| { let l = linear_color(*c); [l[0], l[1], l[2]] };
            o.toon_shade = Some(match shade {
                Some(c) => (ToonShadeKind::Fixed, lin3(c)),
                None if skin => (ToonShadeKind::BaseHsv, [0.5; 3]),
                None if shader.starts_with("AL/hair") => (ToonShadeKind::Fixed, m.colors.get("_Shadow_color").map_or([0.5; 3], lin3)),
                None => (ToonShadeKind::Cloth, [0.5; 3]),
            });
            o.toon_shade_from_base = true;
            // The card `skinShadowColor` (`_Shadow_color`) at grazing angles.
            if skin && shade.is_none() {
                o.toon_shadow_rim = look.shadow.map(|c| { let l = linear_color(c); [l[0], l[1], l[2], c[3]] });
            }
            o.toon_light_limit = Some(1.);
            o.toon_shadow_light = Some([0.539542, 0.546249, 0.737911, 1.]);
            o.toon_ambient = Some(0.);
            // The md layers keep their own shadow tone.
            let tone = shade.map(|c| [c[0], c[1], c[2]]).or(ambient);
            let k = 1.;
            let mut o = self.base_look(file, m, &look, shader, k, o)?;
            // The body's md layers (`_Detail_mask_01` slice 0) shade painted
            // creases and contours as a shadow layer.
            let mf = if m.file.is_empty() { file.to_owned() } else { m.file.clone() };
            if let (Some(md), Some((key, img))) = (m.textures.get("_Detail_mask_01").filter(|t| t.texture.path_id != 0 && shader.starts_with("AL/skin_body")), o.base_texture.as_ref()) {
                let (f, id) = self.unity.target(&mf, md.texture)?;
                let (slices, srgb) = self.unity.texture_array(&f, id)?;
                if let Some(md) = slices.first() {
                    let alpha = look.shadow.map_or(0., |c| c[3]);
                    let layer = md_shadow_layer(img, md, srgb, k, tone, floor, alpha, m);
                    o.toon_shadow = Some((format!("{key}:md"), layer));
                }
            }
            // `_expressionmap` × `_HohoakaIntensity`: the flushed face the
            // face mesh blends toward by the expression's rate.
            if let (Some(t), Some(c), Some((key, img))) = (m.textures.get("_expressionmap").filter(|t| t.texture.path_id != 0 && shader.starts_with("AL/skin_head")), m.colors.get("_hohoakacolor").copied(), o.base_texture.clone()) {
                let (f, id) = self.unity.target(&mf, t.texture)?;
                let map = self.mask(&f, id)?;
                o.toon_blend = Some((format!("{key}:flush"), flush_layer(&img, &map, [t.scale[0], t.scale[1], t.offset[0], t.offset[1]], c)));
            }
            // The face's md layers (two Texture2D masks).
            if let (Some(md), Some((key, img))) = (m.textures.get("_Detail_mask_01").filter(|t| t.texture.path_id != 0 && shader.starts_with("AL/skin_head")), o.base_texture.clone()) {
                let (f, id) = self.unity.target(&mf, md.texture)?;
                let md = self.mask(&f, id)?;
                let md2 = match m.textures.get("_Detail_mask_02").filter(|t| t.texture.path_id != 0) {
                    Some(t) => {
                        let (f, id) = self.unity.target(&mf, t.texture)?;
                        Some(self.mask(&f, id)?)
                    }
                    None => None,
                };
                o.toon_shadow = Some((format!("{key}:md"), face_md_layer(&img, &md, md2.as_ref(), k, floor, m)));
            }
            // Clothes (`_Detail_mask_01`) and hair (`_md_01`) md layers.
            let md_name = if shader.starts_with("AL/cloth") { Some("_Detail_mask_01") } else if shader.starts_with("AL/hair") { Some("_md_01") } else { None };
            if let (Some(md), Some((key, img))) = (md_name.and_then(|n| m.textures.get(n)).filter(|t| t.texture.path_id != 0), o.base_texture.clone()) {
                let (f, id) = self.unity.target(&mf, md.texture)?;
                let md = self.mask(&f, id)?;
                o.toon_shadow = Some((format!("{key}:md"), green_md_layer(&img, &md, k, tone, floor, m)));
            }
            let ramp = toon_ramp_floor(&curve, [0.; 3], floor, o.toon_shadow.is_some());
            let outline = [outline[0] * k, outline[1] * k, outline[2] * k, outline[3]];
            o.toon = Some((ramp, if shader.starts_with("AL/hair") || shader.starts_with("AL/cloth") || shader.starts_with("AL/skin") { 0.0015 } else { 0. }, outline));
            return Ok(Some(o));
        }
        let shadow = match look.shadow.filter(|_| shader.starts_with("AL/skin")) {
            Some(c) => {
                let power = m.floats.get("_Shadow_power").copied().unwrap_or(0.5).clamp(0., 1.);
                [0, 1, 2].map(|i| 1. - (1. - c[i].clamp(0., 1.)) * c[3].clamp(0., 1.) * power)
            }
            None if shader.starts_with("AL/skin") => [0.86, 0.74, 0.74],
            None => [0.74, 0.72, 0.78],
        };
        let ramp = toon_ramp(&curve, shadow);
        o.toon_ambient = m.floats.get("_AmbientIntensity").or(m.floats.get("_AmbientIntensity1")).copied();
        let k = self.light;
        // The outline colour is shaded like the surface colour.
        let outline = [outline[0] * k, outline[1] * k, outline[2] * k, outline[3]];
        o.toon = Some((ramp, if shader.starts_with("AL/hair") || shader.starts_with("AL/cloth") || shader.starts_with("AL/skin") { 0.0015 } else { 0. }, outline));
        self.base_look(file, m, &look, shader, k, o).map(Some)
    }

}

/// Box-halve an image until both sides are at most `max`.
fn halve_to(mut img: ImageSurface, max: u32) -> ImageSurface {
    while img.width > max.max(1) || img.height > max.max(1) {
        let (w, h) = ((img.width / 2).max(1), (img.height / 2).max(1));
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                for c in 0..4 {
                    let s: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)].iter().map(|(dx, dy)| {
                        let (sx, sy) = ((x * 2 + dx).min(img.width - 1), (y * 2 + dy).min(img.height - 1));
                        u32::from(img.rgba[((sy * img.width + sx) * 4 + c) as usize])
                    }).sum();
                    out[((y * w + x) * 4 + c) as usize] = ((s + 2) / 4) as u8;
                }
            }
        }
        img = ImageSurface { width: w, height: h, rgba: out };
    }
    img
}

/// The `AL/eye` iris colour from its `_Eye_mask` (`cf_eye_*`):
/// `lerp(eye01, eye02, R · eye02.a)`, then linear light `c + 2 · eye03 − 1`
/// by `G · eye03.a`, then `_Line_color` by B; alpha the mask's A.
pub fn iris_colors(img: &mut ImageSurface, mask: &ImageSurface, colors: &[[f32; 4]], line: [f32; 4]) {
    let get = |i: usize| linear_color(colors.get(i).copied().unwrap_or([1.; 4]));
    let (e1, e2, e3, line) = (get(0), get(1), get(2), linear_color(line));
    for (o, p) in img.rgba.chunks_exact_mut(4).zip(mask.rgba.chunks_exact(4)) {
        let [r, g, b, a] = [p[0], p[1], p[2], p[3]].map(|v| f32::from(v) / 255.);
        for c in 0..3 {
            let mut v = e1[c] + (e2[c] - e1[c]) * r * e2[3];
            v = (v + (v + 2. * e3[c] - 1. - v) * g * e3[3]).clamp(0., 1.);
            v += (line[c] - v) * b;
            o[c] = (srgb_encode(v) * 255.).round() as u8;
        }
        o[3] = (a * 255.).round() as u8;
    }
}

/// The `AL/eye` pupil (`_Pipil_texture`) over the iris colour, sampled at
/// `(uv − 0.5 − offset) × size + 0.5`: `lerp(c, P1, A · P1.a)`, then
/// `lerp(c, P2, R · A · P2.a)` and `lerp(c, P3, G · A · P3.a)`; the iris
/// alpha is kept.
pub fn pupil_over(img: &mut ImageSurface, p: &ImageSurface, pos: [f32; 4], colors: &[[f32; 4]]) {
    if p.width == 0 || p.height == 0 {
        return;
    }
    let get = |i: usize| linear_color(colors.get(i).copied().unwrap_or([0., 0., 0., 0.]));
    let (p1, p2, p3) = (get(0), get(1), get(2));
    for y in 0..img.height {
        for x in 0..img.width {
            let lu = ((x as f32 + 0.5) / img.width as f32 - 0.5 - pos[2]) * pos[0] + 0.5;
            let lv = (1. - (y as f32 + 0.5) / img.height as f32 - 0.5 - pos[3]) * pos[1] + 0.5;
            if !(0. ..1.).contains(&lu) || !(0. ..1.).contains(&lv) {
                continue;
            }
            let sx = ((lu * p.width as f32) as u32).min(p.width - 1);
            let sy = (((1. - lv) * p.height as f32) as u32).min(p.height - 1);
            let j = ((sy * p.width + sx) * 4) as usize;
            let [r, g, _, a] = [0, 1, 2, 3].map(|k| f32::from(p.rgba[j + k]) / 255.);
            let i = ((y * img.width + x) * 4) as usize;
            for c in 0..3 {
                let mut v = srgb_decode(f32::from(img.rgba[i + c]) / 255.);
                v += (p1[c] - v) * a * p1[3];
                v += (p2[c] - v) * r * a * p2[3];
                v += (p3[c] - v) * g * a * p3[3];
                img.rgba[i + c] = (srgb_encode(v) * 255.).round() as u8;
            }
        }
    }
}

/// An `AL/eye` highlight over the eye colour, sampled at
/// `rot((uv − 0.5 − offset) × size) + 0.5` (`rotation` 0.5 neutral):
/// `lerp(c, color2, G · A · color2.a)`, then `lerp(c, color, R · A · color.a)`.
pub fn highlight_over(img: &mut ImageSurface, h: &ImageSurface, pos: [f32; 4], rotation: f32, c1: [f32; 4], c2: [f32; 4]) {
    if h.width == 0 || h.height == 0 {
        return;
    }
    let (s, c) = ((rotation - 0.5) * std::f32::consts::TAU).sin_cos();
    for y in 0..img.height {
        for x in 0..img.width {
            let u = ((x as f32 + 0.5) / img.width as f32 - 0.5 - pos[2]) * pos[0];
            let v = (1. - (y as f32 + 0.5) / img.height as f32 - 0.5 - pos[3]) * pos[1];
            let (lu, lv) = (u * c + v * s + 0.5, -u * s + v * c + 0.5);
            if !(0. ..1.).contains(&lu) || !(0. ..1.).contains(&lv) {
                continue;
            }
            let sx = ((lu * h.width as f32) as u32).min(h.width - 1);
            let sy = (((1. - lv) * h.height as f32) as u32).min(h.height - 1);
            let j = ((sy * h.width + sx) * 4) as usize;
            let [r, g, _, a] = [0, 1, 2, 3].map(|k| f32::from(h.rgba[j + k]) / 255.);
            let i = ((y * img.width + x) * 4) as usize;
            let (c1, c2) = (linear_color(c1), linear_color(c2));
            for k in 0..3 {
                let mut o = srgb_decode(f32::from(img.rgba[i + k]) / 255.);
                o += (c2[k] - o) * g * a * c2[3];
                o += (c1[k] - o) * r * a * c1[3];
                img.rgba[i + k] = (srgb_encode(o) * 255.).round() as u8;
            }
        }
    }
}

/// The `AL/eye` sclera under an iris image (eye UV space, top-down): the
/// `_Shadow_mask` sampled at `(0.7u + 0.15, 0.7v + 0.65 − 0.2 · _Eye_white_ty)`,
/// `lerp(lerp(_Eye_white_color_01, _02, R), _03, G · _03.a)`, then the iris
/// over it at its alpha; the result is opaque.
pub fn sclera_under(img: &mut ImageSurface, mask: &ImageSurface, m: &UnityMaterial) {
    if mask.width == 0 || mask.height == 0 {
        return;
    }
    let col = |n: &str, d: [f32; 4]| m.colors.get(n).copied().unwrap_or(d);
    let (w1, w2, w3) = (col("_Eye_white_color_01", [1.; 4]), col("_Eye_white_color_02", [0.69, 0.72, 0.82, 1.]), col("_Eye_white_color_03", [0.76, 0.88, 0.99, 1.]));
    let ty = m.floats.get("_Eye_white_ty").copied().unwrap_or(0.5);
    for y in 0..img.height {
        for x in 0..img.width {
            let u = (x as f32 + 0.5) / img.width as f32;
            let v = 1. - (y as f32 + 0.5) / img.height as f32;
            let (su, sv) = (0.7 * u + 0.15, 0.7 * v + 0.65 - 0.2 * ty);
            let mx = ((su.clamp(0., 1.) * mask.width as f32) as u32).min(mask.width - 1);
            let my = (((1. - sv.clamp(0., 1.)) * mask.height as f32) as u32).min(mask.height - 1);
            let j = ((my * mask.width + mx) * 4) as usize;
            let (r, g) = (f32::from(mask.rgba[j]) / 255., f32::from(mask.rgba[j + 1]) / 255.);
            let i = ((y * img.width + x) * 4) as usize;
            let a = f32::from(img.rgba[i + 3]) / 255.;
            for c in 0..3 {
                let white = srgb_decode(w1[c]) + (srgb_decode(w2[c]) - srgb_decode(w1[c])) * r;
                let white = white + (srgb_decode(w3[c]) - white) * g * w3[3];
                let iris = srgb_decode(f32::from(img.rgba[i + c]) / 255.);
                img.rgba[i + c] = (srgb_encode(white + (iris - white) * a) * 255.).round() as u8;
            }
            img.rgba[i + 3] = 255;
        }
    }
}

/// `src` resampled into a `width`×`height` eye UV space at
/// `(uv − 0.5 − offset) × size + 0.5` (transparent outside).
pub fn resample_layer(src: &ImageSurface, width: u32, height: u32, pos: [f32; 4]) -> ImageSurface {
    let mut out = ImageSurface { width, height, rgba: vec![0; (width * height * 4) as usize] };
    if src.width == 0 || src.height == 0 {
        return out;
    }
    for y in 0..height {
        for x in 0..width {
            let lu = ((x as f32 + 0.5) / width as f32 - 0.5 - pos[2]) * pos[0] + 0.5;
            let lv = (1. - (y as f32 + 0.5) / height as f32 - 0.5 - pos[3]) * pos[1] + 0.5;
            if !(0. ..1.).contains(&lu) || !(0. ..1.).contains(&lv) {
                continue;
            }
            let sx = ((lu * src.width as f32) as u32).min(src.width - 1);
            let sy = (((1. - lv) * src.height as f32) as u32).min(src.height - 1);
            let j = ((sy * src.width + sx) * 4) as usize;
            let i = ((y * width + x) * 4) as usize;
            out.rgba[i..i + 4].copy_from_slice(&src.rgba[j..j + 4]);
        }
    }
    out
}

/// Shadow amount of the ILLGAMES `_Lamp` ramp textures at 256 steps of
/// the half Lambert term (their G channel, 1 facing away from the light);
/// without one, a soft terminator around n·l = 0.
pub fn ramp_curve(lamp: Option<&ImageSurface>) -> [f32; 256] {
    std::array::from_fn(|x| {
        let u = x as f32 / 255.;
        match lamp {
            Some(l) if l.width > 0 && l.height > 0 => {
                let lx = ((u * l.width as f32) as u32).min(l.width - 1);
                f32::from(l.rgba[(lx * 4 + 1) as usize]) / 255.
            }
            _ => 1. - ((u - 0.46) / 0.08).clamp(0., 1.),
        }
    })
}

/// The face's flushed base (`AL/skin_head` before its makeup): the base
/// moves toward `_hohoakacolor · clamp(√lum(base), 0.1, 1)` by
/// `_expressionmap.r · _hohoakacolor.a` (linear; `st` the map's scale and
/// offset on uv0). The runtime rate blends base → this layer.
pub fn flush_layer(base: &ImageSurface, map: &ImageSurface, st: [f32; 4], color: [f32; 4]) -> ImageSurface {
    let mut out = base.clone();
    let c = linear_color(color);
    let (w, h) = (base.width as usize, base.height as usize);
    for y in 0..h {
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32 * st[0] + st[2];
            let v = (1. - (y as f32 + 0.5) / h as f32) * st[1] + st[3];
            let mx = ((u.rem_euclid(1.) * map.width as f32) as usize).min(map.width as usize - 1);
            let my = (((1. - v.rem_euclid(1.)) * map.height as f32) as usize).min(map.height as usize - 1);
            let k = f32::from(map.rgba[(my * map.width as usize + mx) * 4]) / 255. * c[3];
            if k <= 0. {
                continue;
            }
            let i = (y * w + x) * 4;
            let lin = [0, 1, 2].map(|j| srgb_decode(f32::from(base.rgba[i + j]) / 255.));
            let l = (lin[0] * 0.212673 + lin[1] * 0.715152 + lin[2] * 0.072175).sqrt().clamp(0.1, 1.);
            for j in 0..3 {
                let t = lin[j] + (c[j] * l - lin[j]) * k.min(1.);
                out.rgba[i + j] = (srgb_encode(t) * 255.).round().clamp(0., 255.) as u8;
            }
        }
    }
    out
}

/// Shadow amount of the AL skin programs at 256 steps of the engine's half
/// Lambert coordinate `u = n·l · 0.4951 + 0.5`: `_Lamp` blue at
/// `sat((n·l + 0.6) / 1.6)`, the lit part gated by `sat((n·l − 0.05) · 5)`.
/// SATD-amanatsu-port: the md layers' shift of the lamp coordinate.
pub fn skin_ramp_curve(lamp: Option<&ImageSurface>) -> [f32; 256] {
    std::array::from_fn(|x| {
        let nl = (x as f32 / 255. - 0.5) / 0.4951;
        let lx = ((nl + 0.6) / 1.6).clamp(0., 1.);
        let b = match lamp {
            Some(l) if l.width > 0 && l.height > 0 => {
                let px = ((lx * l.width as f32) as u32).min(l.width - 1);
                f32::from(l.rgba[(px * 4 + 2) as usize]) / 255.
            }
            None | Some(_) => 1. - ((lx - 0.46) / 0.08).clamp(0., 1.),
        };
        let gate = ((nl - 0.05) * 5.).clamp(0., 1.);
        1. - (1. - b) * gate
    })
}

/// The engine toon lookup of the AL shaders: row 0 `mix(tone, 1, lit)` with
/// `lit = max(1 − shadow amount, floor)` (a black `tone` leaves the lit
/// amount for `ToonShading::shade_from_base`), row 1 rate 0 with a shadow
/// layer (applied at its alpha everywhere), else 1.
pub fn toon_ramp_floor(curve: &[f32; 256], tone: [f32; 3], floor: f32, layer: bool) -> ImageSurface {
    let amount: [f32; 256] = std::array::from_fn(|x| 1. - (1. - curve[x].clamp(0., 1.)).max(floor));
    let mut ramp = toon_ramp(&amount, tone);
    if layer {
        for x in 0..256 {
            ramp.rgba[(256 + x) * 4..(256 + x) * 4 + 3].fill(0);
        }
    }
    ramp
}

/// The AL shaders' shadow tone of a linear colour: hue kept, saturation
/// `√S · min(10√V, 1)`, value `0.5√V + 0.5` (HSV).
pub fn hsv_shade(c: [f32; 3]) -> [f32; 3] {
    let v = c[0].max(c[1]).max(c[2]);
    let d = v - c[0].min(c[1]).min(c[2]);
    let h = if d <= 1e-6 {
        0.
    } else if v == c[0] {
        ((c[1] - c[2]) / d / 6.).rem_euclid(1.)
    } else if v == c[1] {
        (2. + (c[2] - c[0]) / d) / 6.
    } else {
        (4. + (c[0] - c[1]) / d) / 6.
    };
    let s = d / (v + 1e-10);
    let s2 = s.sqrt().min(1.) * (v.sqrt() * 10.).min(1.);
    let v2 = v.sqrt() * 0.5 + 0.5;
    [1., 2. / 3., 1. / 3.].map(|o: f32| {
        let hue = (((h + o).fract() * 6. - 3.).abs() - 1.).clamp(0., 1.);
        ((hue - 1.) * s2 + 1.) * v2
    })
}

/// The skin body's md shading (`_Detail_mask_01` slice 0, decompiled
/// forward program): G painted shading outside the chest region (A ≤ 0.66
/// marks it; its own term is off by `_Bust_shadowonoff`), scaled by
/// `0.3 + 0.7 · max(0, _Shadow_color.a)`, divided by
/// `clamp(1 − 0.5 · _mdGPower · (1 − B), 0.5, 1)` and weighted by
/// `lerp(_mdGIntensity, 0.5, B)`. As a toon shadow layer: colour
/// `base · k · mix(tone, 1, floor)` in linear space (tone: `_Shade_color` or
/// the HSV shade),
/// amount the md value. SATD-amanatsu-port: the original shifts the `_Lamp`
/// coordinate by this amount and light terms (not decoded); the layer
/// applies it at a fixed 1.5× strength.
/// A material/card colour (sRGB, alpha linear) in linear space.
fn linear_color(c: [f32; 4]) -> [f32; 4] {
    [srgb_decode(c[0]), srgb_decode(c[1]), srgb_decode(c[2]), c[3]]
}

fn srgb_decode(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn srgb_encode(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1. / 2.4) - 0.055 }
}

#[allow(clippy::too_many_arguments)]
fn md_shadow_layer(base: &ImageSurface, md: &ImageSurface, srgb: bool, k: f32, tone: Option<[f32; 3]>, floor: f32, shadow_alpha: f32, m: &UnityMaterial) -> ImageSurface {
    let lin = |b: u8| if srgb { srgb_decode(f32::from(b) / 255.) } else { f32::from(b) / 255. };
    let get = |n: &str, d: f32| m.floats.get(n).copied().unwrap_or(d);
    let (intensity, power, bust) = (get("_mdGIntensity", 0.8), get("_mdGPower", 0.5), get("_Bust_shadowonoff", 0.));
    shadow_layer(base, k, tone, floor, 512, &|u, v| {
        let j = texel(md, u, v);
        let (g, b, a) = (lin(md.rgba[j + 1]), lin(md.rgba[j + 2]), lin(md.rgba[j + 3]));
        let chest = ((a - 0.66) * -4.16667 + 1.).clamp(0., 1.);
        let shade = ((chest * 0.75).max(shadow_alpha) * 0.7 + 0.3).min(1.) * g;
        let s = (shade * (1. - chest) + shade * chest * bust).max(0.);
        let div = (1. - 0.5 * power * (1. - b)).clamp(0.5, 1.);
        ((s / div).clamp(0., 1.) * (intensity + (0.5 - intensity) * b)).max(0.)
    })
}

/// The face's md shading (`AL/skin_head` forward program,
/// `_Detail_mask_01`/`_02`, both linearised): `r = sat(sat((1 − G) · 1.23 −
/// 0.31) − (R − md2.B))`, amount `(1 − r) / clamp(1 − 0.5 · _mdGPower ·
/// (1 − B), 0.5, 1) · lerp(_mdGIntensity, 0.5, B)` (G marks the outer face
/// and neck, R and the md2 oval cancel in the face centre).
/// SATD-amanatsu-port: the nose/lip `_spmap` maxima and the `_Lamp`
/// coordinate shift are not applied (fixed strength as for the body).
fn face_md_layer(base: &ImageSurface, md: &ImageSurface, md2: Option<&ImageSurface>, k: f32, floor: f32, m: &UnityMaterial) -> ImageSurface {
    let get = |n: &str, d: f32| m.floats.get(n).copied().unwrap_or(d);
    let (intensity, power) = (get("_mdGIntensity", 1.), get("_mdGPower", 0.));
    let ch = |img: &ImageSurface, u: f32, v: f32, c: usize| f32::from(img.rgba[texel(img, u, v) + c]) / 255.;
    shadow_layer(base, k, None, floor, 512, &|u, v| {
        let (r, g, b) = (ch(md, u, v, 0), ch(md, u, v, 1), ch(md, u, v, 2));
        let oval = md2.map_or(0., |t| ch(t, u, v, 2));
        let lit = (((1. - g) * 1.23 - 0.31).clamp(0., 1.) - (r - oval)).clamp(0., 1.);
        let div = (1. - 0.5 * power * (1. - b)).clamp(0.5, 1.);
        ((1. - lit) / div).min(1.) * (intensity + (0.5 - intensity) * b)
    })
}

/// The clothes' and hair's md shading (`AL/cloth`, `AL/hair` forward
/// programs): `sat(G / clamp(1 − 0.5 · _mdGPower, 0.5, 1)) · _mdGIntensity`
/// of the (linearised) md texture in uv0, as a 256² shadow layer (every
/// clothing variant carries one).
/// SATD-amanatsu-port: the `_Lamp` coordinate shift is not applied (fixed
/// strength as for the body).
fn green_md_layer(base: &ImageSurface, md: &ImageSurface, k: f32, tone: Option<[f32; 3]>, floor: f32, m: &UnityMaterial) -> ImageSurface {
    let get = |n: &str, d: f32| m.floats.get(n).copied().unwrap_or(d);
    let (intensity, power) = (get("_mdGIntensity", 0.5), get("_mdGPower", 0.5));
    let div = (1. - 0.5 * power).clamp(0.5, 1.);
    shadow_layer(base, k, tone, floor, 256, &|u, v| (f32::from(md.rgba[texel(md, u, v) + 1]) / 255. / div).clamp(0., 1.) * intensity)
}

/// Byte offset of the texel at top-down image coordinates `u`, `v` in [0, 1).
fn texel(img: &ImageSurface, u: f32, v: f32) -> usize {
    let x = ((u * img.width as f32) as u32).min(img.width.saturating_sub(1));
    let y = ((v * img.height as f32) as u32).min(img.height.saturating_sub(1));
    ((y * img.width + x) * 4) as usize
}

/// A toon shadow layer over a display-referred base texture: colour
/// `base · k · mix(tone, 1, floor)` in linear space (tone: the given one or
/// the HSV shade of the texel), alpha `amount(u, v) · 1.5`; at most `max`
/// texels a side (prepared-asset budget).
fn shadow_layer(base: &ImageSurface, k: f32, tone: Option<[f32; 3]>, floor: f32, max: u32, amount: &dyn Fn(f32, f32) -> f32) -> ImageSurface {
    let (ow, oh) = (base.width.min(max).max(1), base.height.min(max).max(1));
    let mut out = ImageSurface { width: ow, height: oh, rgba: vec![0; (ow * oh * 4) as usize] };
    for y in 0..oh {
        for x in 0..ow {
            let (u, v) = ((x as f32 + 0.5) / ow as f32, (y as f32 + 0.5) / oh as f32);
            let bi = texel(base, u, v);
            let c = [0, 1, 2].map(|n| srgb_decode(f32::from(base.rgba[bi + n]) / 255.));
            let t = tone.unwrap_or_else(|| hsv_shade(c));
            let i = ((y * ow + x) * 4) as usize;
            for n in 0..3 {
                let v = srgb_encode(c[n] * (t[n] + (1. - t[n]) * floor)) * k;
                out.rgba[i + n] = (v.clamp(0., 1.) * 255.).round() as u8;
            }
            out.rgba[i + 3] = ((amount(u, v) * 1.5).clamp(0., 1.) * 255.).round() as u8;
        }
    }
    out
}

/// The engine toon lookup (256×3): row 0 the colour between `shadow` and
/// full by the ramp's shadow amount, row 1 the shadow-layer rate (no shadow
/// texture), row 2 unused.
pub fn toon_ramp(curve: &[f32; 256], shadow: [f32; 3]) -> ImageSurface {
    let mut ramp = ImageSurface { width: 256, height: 3, rgba: vec![0u8; 256 * 3 * 4] };
    let rgba = &mut ramp.rgba;
    for (x, a) in curve.iter().enumerate() {
        for c in 0..3 {
            rgba[x * 4 + c] = ((1. - (1. - shadow[c]) * a.clamp(0., 1.)) * 255.).round() as u8;
        }
        rgba[x * 4 + 3] = 255;
        rgba[(256 + x) * 4..(256 + x) * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
        rgba[(512 + x) * 4 + 3] = 255;
    }
    ramp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_layer_mix() {
        // A grey base under a full mask with an opaque red flush colour
        // becomes red × √lum; an empty mask leaves it.
        let base = ImageSurface { width: 2, height: 1, rgba: vec![188, 188, 188, 255, 188, 188, 188, 255] };
        let map = ImageSurface { width: 2, height: 1, rgba: vec![255, 0, 0, 255, 0, 0, 0, 255] };
        let out = flush_layer(&base, &map, [1., 1., 0., 0.], [1., 0., 0., 1.]);
        let l = srgb_decode(188. / 255.).sqrt();
        assert_eq!(out.rgba[0], (srgb_encode(l) * 255.).round() as u8);
        assert_eq!(&out.rgba[1..3], &[0, 0]);
        assert_eq!(&out.rgba[4..8], &[188, 188, 188, 255]);
    }

    #[test]
    fn skin_lamp_curve() {
        // A lamp whose blue falls linearly from 1 to 0.
        let mut lamp = ImageSurface { width: 256, height: 1, rgba: vec![0; 256 * 4] };
        for x in 0..256 {
            lamp.rgba[x * 4 + 2] = 255 - x as u8;
        }
        let c = skin_ramp_curve(Some(&lamp));
        let at = |nl: f32| c[((nl * 0.4951 + 0.5) * 255.).round() as usize];
        // Below the gate (n·l ≤ 0.05) everything is shadow.
        assert_eq!(at(-0.5), 1.);
        assert_eq!(at(0.), 1.);
        // Fully lit side: lamp x = (n·l + 0.6) / 1.6 ≈ 1 → blue ≈ 0.
        assert!(at(1.) < 0.02, "{}", at(1.));
        // Half the gate at n·l = 0.15, lamp x ≈ 0.47.
        let mid = at(0.15);
        assert!((mid - (1. - 0.5 * 0.47)).abs() < 0.03, "{mid}");
    }

    #[test]
    fn al_shade_tone() {
        // Grey keeps its hue-less tone at 0.5·sqrt(V) + 0.5.
        let g = hsv_shade([0.25, 0.25, 0.25]);
        assert!(g.iter().all(|c| (c - 0.75).abs() < 1e-5), "{g:?}");
        // A skin colour gets a more saturated tone of the same hue.
        let s = hsv_shade([0.91, 0.64, 0.55]);
        assert!(s[0] > s[1] && s[1] > s[2] && s[2] < 0.55, "{s:?}");
        // The floor keeps the lit amount at or above it.
        let ramp = toon_ramp_floor(&[1.; 256], [0.; 3], 0.3, true);
        assert_eq!(ramp.rgba[0], (0.3f32 * 255.).round() as u8);
        assert_eq!(ramp.rgba[256 * 4], 0);
    }

    #[test]
    fn eye_layers() {
        // Pupil colours apply in sequence by A, then R and G.
        let mut img = ImageSurface { width: 1, height: 1, rgba: vec![255, 255, 255, 128] };
        let p = ImageSurface { width: 1, height: 1, rgba: vec![255, 0, 0, 255] };
        pupil_over(&mut img, &p, [1., 1., 0., 0.], &[[0., 0., 0., 0.5], [1., 0., 0., 1.], [0., 1., 0., 1.]]);
        assert_eq!(&img.rgba[..], &[255, 0, 0, 128]);
        // The iris covers the sclera at its alpha and the result is opaque.
        let mut eye = ImageSurface { width: 1, height: 1, rgba: vec![0, 0, 0, 0] };
        let mask = ImageSurface { width: 1, height: 1, rgba: vec![0, 0, 0, 255] };
        sclera_under(&mut eye, &mask, &UnityMaterial::default());
        assert_eq!(&eye.rgba[..], &[255, 255, 255, 255]);
    }

    #[test]
    fn ramp_from_lamp() {
        // G 255 → 0 across the texture: full shadow facing away.
        let lamp = ImageSurface { width: 2, height: 1, rgba: vec![0, 255, 0, 255, 0, 0, 0, 255] };
        let curve = ramp_curve(Some(&lamp));
        assert_eq!((curve[0], curve[255]), (1., 0.));
        let r = toon_ramp(&curve, [0.5, 0.5, 0.5]);
        assert_eq!(&r.rgba[..3], &[128, 128, 128]);
        assert_eq!(&r.rgba[255 * 4..255 * 4 + 3], &[255, 255, 255]);
    }

    #[test]
    fn mask_tint() {
        let mut main = ImageSurface { width: 1, height: 1, rgba: vec![200, 200, 200, 255] };
        let mask = ImageSurface { width: 1, height: 1, rgba: vec![0, 255, 0, 255] };
        tint_by_mask(&mut main, Some(&mask), &[[1., 0., 0., 1.], [0.5, 0.5, 1., 1.], [0., 0., 0., 1.]]);
        assert_eq!(&main.rgba[..3], &[100, 100, 200]);
    }
}
