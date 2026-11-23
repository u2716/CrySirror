// SPDX-License-Identifier: MPL-2.0
//! HumanCustom's title-entry category selector and native preset lookup.
use crate::{
    card::Card,
    chara::{Lists, key},
    level::Level,
    unity::Unity,
};
use anyhow::{Context, Result};
use crysirror_unity::{
    class_id,
    managed::{Fields, Value},
    scene6,
};

pub(crate) fn preset(unity: &mut Unity, lists: &Lists, female: bool) -> Result<(String, Card)> {
    let list = if female {
        "cha_sample_f"
    } else {
        "cha_sample_m"
    };
    let (bundle, name) = lists
        .row(list, 0)
        .and_then(|row| row.asset(key::MAIN_AB, key::MAIN_DATA))
        .with_context(|| format!("HumanData.GetPreset: {list} id 0"))?;
    let (file, id) = unity
        .bundle_asset(&bundle, &name, class_id::TEXT_ASSET)?
        .with_context(|| format!("editor preset {name} in {bundle}"))?;
    Ok((name, Card::parse(&unity.text_asset(&file, id)?)?))
}

fn node(level: &Level, fields: &Fields, key: &str) -> Option<usize> {
    fields
        .pptr(key)
        .filter(|p| !p.is_null())
        .and_then(|p| level.owner(p.path_id))
}

/// The installed build packs the editor prefabs into lib/custom/ui. Each
/// cached prefab keeps its own local pointers under its build-scene Canvas.
fn panel(unity: &mut Unity, scene: &Level, field: &str) -> Result<Level> {
    let custom = scene
        .scene
        .nodes
        .iter()
        .flat_map(|n| &n.components)
        .find(|c| c.script == "CharacterCreation.HumanCustom")
        .context("HumanCustom")?;
    let cache = custom
        .fields
        .fields(field)
        .context("editor UI definition")?;
    let cache = cache.fields("_cacheUI").unwrap_or(cache);
    let asset = cache.string("_asset").context("editor UI asset")?;
    let parent = node(scene, cache, "_parent").context("editor UI canvas")?;
    let mut level = Level::prefab(unity, "lib/custom/ui.unity3d", asset)?;
    let mut canvas = scene.scene.nodes[parent].clone();
    canvas.parent = None;
    canvas.components.retain(|c| {
        matches!(c.class_id, scene6::CANVAS | scene6::CANVAS_GROUP)
            || c.script.ends_with("CanvasScaler")
    });
    anyhow::ensure!(
        canvas
            .components
            .iter()
            .any(|c| c.class_id == scene6::CANVAS),
        "editor UI parent has no Canvas"
    );
    canvas.children = std::mem::take(&mut level.scene.roots);
    let canvas_index = level.scene.nodes.len();
    for &root in &canvas.children {
        level.scene.nodes[root].parent = Some(canvas_index);
    }
    level.scene.nodes.push(canvas);
    level.scene.roots.push(canvas_index);
    Ok(level)
}

pub struct Categories {
    pages: Vec<(usize, usize)>,
    config: Option<usize>,
    selected: Option<usize>,
}

impl Categories {
    /// CacheUI instantiates the bundle object beneath a build-scene canvas.
    /// Keep the prefab's local PPtr namespace, with the authored Canvas and
    /// CanvasScaler as its layout parent.
    pub fn load(
        unity: &mut Unity,
        scene: &Level,
        female: bool,
        mature: bool,
    ) -> Result<(Level, Self)> {
        let mut level = panel(unity, scene, "_categorySelectionTopUI")?;
        let top = level
            .scene
            .nodes
            .iter()
            .flat_map(|n| &n.components)
            .find(|c| c.script == "CharacterCreation.UI.CategorySelectionTop")
            .context("CategorySelectionTop fields")?
            .fields
            .clone();
        for (name, active) in [
            ("_newMenu", true),
            ("_editMenu", false),
            (if female { "_femaleMenu" } else { "_maleMenu" }, true),
        ] {
            if let Some(group) = top.fields(name) {
                for (field, visible) in [
                    ("_actives", active),
                    ("_nonActives", false),
                    ("_porns", active && mature),
                ] {
                    for item in group.array(field).unwrap_or_default() {
                        if let Value::PPtr(p) = item {
                            if let Some(n) = level.owner(p.path_id).filter(|_| !p.is_null()) {
                                level.set_active(n, visible);
                            }
                        }
                    }
                }
            }
        }
        let pages: Vec<_> = level
            .scene
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(page, n)| {
                let c = n
                    .components
                    .iter()
                    .find(|c| c.script == "CharacterCreation.UI.CategorySelection")?;
                Some((node(&level, &c.fields, "_toggle")?, page))
            })
            .collect();
        let config = node(&level, &top, "_btnConfig");
        let selected = pages.iter().position(|&(toggle, _)| {
            level.scene.nodes[toggle]
                .components
                .iter()
                .any(|c| c.script == "UnityEngine.UI.Toggle" && c.fields.int("m_IsOn") == Some(1))
        });
        let mut result = Self {
            pages,
            config,
            selected: None,
        };
        result.select(&mut level, selected);
        Ok((level, result))
    }

    /// HumanDataParameter.fullname and HumanCustom.UpdateFullNameUI.
    pub(crate) fn show_name(level: &mut Level, card: &Card, english: bool) {
        let top = level
            .scene
            .nodes
            .iter()
            .flat_map(|n| &n.components)
            .find(|c| c.script == "CharacterCreation.UI.CategorySelectionTop");
        let label = top.and_then(|c| node(level, &c.fields, "_txtFullName"));
        if let (Some(label), Some((_, first, last))) = (label, card.parameter()) {
            level.states.entry(label).or_default().text = Some(if english {
                format!("{first} {last}")
            } else {
                format!("{last} {first}")
            });
        }
    }

    fn select(&mut self, level: &mut Level, next: Option<usize>) {
        self.selected = next;
        for (i, &(toggle, page)) in self.pages.iter().enumerate() {
            let on = next == Some(i);
            level.set_active(page, on);
            let graphic = level.scene.nodes[toggle]
                .components
                .iter()
                .find(|c| c.script == "UnityEngine.UI.Toggle")
                .and_then(|c| node(level, &c.fields, "graphic"));
            if let Some(graphic) = graphic {
                level.set_active(graphic, on);
            }
        }
    }

    pub fn click(&mut self, level: &mut Level, button: usize) -> crate::scenes::Request {
        if Some(button) == self.config {
            return crate::scenes::Request::Config;
        }
        if let Some(i) = self.pages.iter().position(|&(t, _)| t == button) {
            let toggle = level.scene.nodes[button]
                .components
                .iter()
                .find(|c| c.script == "UnityEngine.UI.Toggle");
            let group = toggle.and_then(|c| node(level, &c.fields, "m_Group"));
            let allow_off = group.is_none_or(|n| {
                level.scene.nodes[n].components.iter().any(|c| {
                    c.script == "UnityEngine.UI.ToggleGroup"
                        && c.fields.int("m_AllowSwitchOff") == Some(1)
                })
            });
            self.select(
                level,
                if self.selected == Some(i) && allow_off {
                    None
                } else {
                    Some(i)
                },
            );
        }
        crate::scenes::Request::None
    }
}

fn vector(fields: &Fields, name: &str) -> Result<glam::Vec3> {
    let v = fields
        .fields(name)
        .with_context(|| format!("editor camera {name}"))?;
    Ok(glam::Vec3::new(
        v.float("x").context("camera vector x")? as f32,
        v.float("y").context("camera vector y")? as f32,
        v.float("z").context("camera vector z")? as f32,
    ))
}

struct CameraPreset {
    pos: glam::Vec3,
    dir: glam::Vec3,
    rot: glam::Quat,
}

impl CameraPreset {
    fn read(fields: &Fields) -> Result<Self> {
        let r = vector(fields, "Rot")?;
        Ok(Self {
            pos: vector(fields, "Pos")?,
            dir: vector(fields, "Dir")?,
            rot: glam::Quat::from_euler(
                glam::EulerRot::YXZ,
                r.y.to_radians(),
                r.x.to_radians(),
                r.z.to_radians(),
            ),
        })
    }

    fn apply(&self, origin: glam::Vec3, mut lens: crate::map::View) -> crate::map::View {
        let mirror = |v: glam::Vec3| crysirror_unity::gltf::mirror_point(v.to_array());
        let eye = self.pos + origin + self.rot * self.dir;
        lens.eye = mirror(eye);
        lens.target = mirror(eye + self.rot * glam::Vec3::Z);
        lens.up = mirror(self.rot * glam::Vec3::Y);
        lens
    }
}

/// StateMiniSelection pairs its sibling toggles and page-root children in
/// Transform order. The original visibility property starts false.
pub struct QuickPages {
    root: usize,
    tabs: Vec<(usize, usize, usize)>,
    decorations: Vec<(usize, bool)>,
    selected: Option<usize>,
}

impl QuickPages {
    pub fn load(level: &mut Level) -> Result<Self> {
        let (owner, state) = level
            .scene
            .nodes
            .iter()
            .enumerate()
            .find_map(|(i, n)| {
                Some((
                    i,
                    &n.components
                        .iter()
                        .find(|c| {
                            c.script == "CharacterCreation.UI.ControlPanel.StateMiniSelection"
                        })?
                        .fields,
                ))
            })
            .context("editor quick controls")?;
        let root = node(level, state, "_root").context("quick window")?;
        let pages = node(level, state, "_pageRoot").context("quick pages")?;
        let parent = level.scene.nodes[owner]
            .parent
            .context("quick tabs parent")?;
        let toggles: Vec<_> = level.scene.nodes[parent]
            .children
            .iter()
            .filter_map(|&i| {
                let toggle = level.scene.nodes[i]
                    .components
                    .iter()
                    .find(|c| c.script == "UnityEngine.UI.Toggle")?;
                Some((i, node(level, &toggle.fields, "graphic")))
            })
            .collect();
        anyhow::ensure!(
            toggles.len() == level.scene.nodes[pages].children.len(),
            "editor quick tab/page count mismatch"
        );
        let tabs = toggles
            .into_iter()
            .zip(&level.scene.nodes[pages].children)
            .map(|((button, graphic), &page)| {
                Ok((button, graphic.context("quick toggle graphic")?, page))
            })
            .collect::<Result<_>>()?;
        let design = node(level, state, "_openDesign").context("quick window design")?;
        let design = &level.scene.nodes[design]
            .components
            .iter()
            .find(|c| c.script == "CharacterCreation.UI.ControlPanel.OpenDesign")
            .context("quick window decoration")?
            .fields;
        let mut decorations = Vec::new();
        for (key, open) in [("_ons", true), ("_offs", false)] {
            for item in design.array(key).context("quick decoration list")? {
                let Value::PPtr(p) = item else {
                    anyhow::bail!("invalid quick decoration")
                };
                decorations.push((
                    level.owner(p.path_id).context("quick decoration node")?,
                    open,
                ));
            }
        }
        let mut result = Self {
            root,
            tabs,
            decorations,
            selected: None,
        };
        result.select(level, None);
        Ok(result)
    }

    fn select(&mut self, level: &mut Level, selected: Option<usize>) {
        self.selected = selected;
        level.set_active(self.root, selected.is_some());
        for &(i, on_open) in &self.decorations {
            level.set_active(i, on_open == selected.is_some());
        }
        for (index, &(_, graphic, page)) in self.tabs.iter().enumerate() {
            let on = Some(index) == selected;
            level.set_active(graphic, on);
            if selected.is_some() {
                level.set_active(page, on);
            }
        }
    }

    pub fn click(&mut self, level: &mut Level, button: usize) {
        if let Some(index) = self.tabs.iter().position(|&(t, _, _)| t == button) {
            self.select(level, (self.selected != Some(index)).then_some(index));
        }
    }
}

/// ControlPanelUI.SetCameraFace/Body. Keep only nodes and animation for
/// CPU sampling of the moving head when the face button is pressed.
pub(crate) struct CameraPanel {
    pub(crate) exit_button: usize,
    hints: Vec<(usize, usize)>,
    face_button: usize,
    body_button: usize,
    face: CameraPreset,
    body: CameraPreset,
    rig: crysirror_asset::pbr::PbrModel,
    head: usize,
    head_offset: glam::Vec3,
    correction: glam::Vec3,
    entered: u64,
}

impl CameraPanel {
    pub(crate) fn load(
        unity: &mut Unity,
        scene: &Level,
        card: &Card,
        model: &[u8],
        entered: u64,
    ) -> Result<(Level, Self, QuickPages)> {
        let mut level = panel(unity, scene, "_controlPanelUI")?;
        let quick = QuickPages::load(&mut level)?;
        let fields = &level
            .scene
            .nodes
            .iter()
            .flat_map(|n| &n.components)
            .find(|c| c.script == "CharacterCreation.UI.ControlPanel.ControlPanelUI")
            .context("ControlPanelUI")?
            .fields;
        let face = fields
            .fields("_faceCameraData")
            .context("face camera data")?;
        let height = card
            .parts("Custom")?
            .get(1)
            .and_then(|b| b.get("shapeValueBody"))
            .and_then(|v| v.as_array().first())
            .and_then(|v| v.as_f32())
            .context("editor height")?
            .clamp(0., 1.);
        let correction = if height < 0.5 {
            vector(face, "_min")? * (1. - 2. * height)
        } else {
            vector(face, "_max")? * (2. * height - 1.)
        };
        // HumanFace.LoadHeadBone parents objHeadBone to RefObjKey 1 (cf_s_head).
        let bone = Level::prefab(
            unity,
            "lib/chara/head/bone/000.unity3d",
            "p_cf_al_head_bone_00",
        )?;
        let head_offset = glam::Vec3::from_array(crysirror_unity::gltf::mirror_point(
            bone.scene.nodes[bone.scene.roots[0]].xform.position,
        ));
        let model = crysirror_asset::pbr::load_geometry(model, |_| {
            anyhow::bail!("external editor character asset")
        })?;
        let head = model
            .nodes
            .iter()
            .position(|n| n.name == "cf_s_head")
            .context("editor head transform")?;
        let hints = level
            .scene
            .nodes
            .iter()
            .flat_map(|n| &n.components)
            .filter(|c| c.script == "CharacterCreation.UI.ControlPanel.HintUI")
            .filter_map(|c| {
                Some((
                    node(&level, &c.fields, "_target")?,
                    node(&level, &c.fields, "_fadeCanvas")?,
                ))
            })
            .collect();
        let result = Self {
            exit_button: node(&level, fields, "_btnExit").context("editor exit button")?,
            hints,
            face_button: node(&level, fields, "_btnCameraFace").context("face camera button")?,
            body_button: node(&level, fields, "_btnCameraBody").context("body camera button")?,
            body: CameraPreset::read(
                fields
                    .fields("_bodyCameraData")
                    .context("body camera data")?,
            )?,
            face: CameraPreset::read(face)?,
            rig: crysirror_asset::pbr::PbrModel {
                nodes: model.nodes,
                roots: model.roots,
                animations: model.animations,
                ..Default::default()
            },
            head,
            head_offset,
            correction,
            entered,
        };
        result.hover(&mut level, None);
        Ok((level, result, quick))
    }

    pub(crate) fn hover(&self, level: &mut Level, target: Option<usize>) {
        for &(source, hint) in &self.hints {
            level.set_active(hint, Some(source) == target);
        }
    }

    pub(crate) fn click(
        &self,
        button: usize,
        now: u64,
        lens: &crate::map::View,
    ) -> Option<crate::map::View> {
        if button == self.body_button {
            return Some(self.body.apply(glam::Vec3::ZERO, lens.clone()));
        }
        if button != self.face_button {
            return None;
        }
        let mut local: Vec<_> = self.rig.nodes.iter().map(|n| n.rest).collect();
        if let Some(clip) = self.rig.animations.first().filter(|c| c.duration > 0.) {
            let t = (now.saturating_sub(self.entered) as f32 / 1000.) % clip.duration;
            crysirror_asset::anim::sample_clip(&self.rig, clip, t, &mut local);
        }
        let pose = crysirror_asset::anim::global_pose(&self.rig, &local);
        let point = pose[self.head].transform_point3(self.head_offset);
        let origin = glam::Vec3::from_array(crysirror_unity::gltf::mirror_point(point.to_array()));
        let mut preset = CameraPreset {
            pos: self.face.pos,
            dir: self.face.dir,
            rot: self.face.rot,
        };
        preset.dir.z += self.correction.z;
        Some(preset.apply(origin + glam::Vec3::Y * self.correction.y, lens.clone()))
    }
}

/// HumanCustom's clean exit on title entry. The installed build has no
/// editor UI translation bundle and uses its original Japanese strings.
pub fn exit_dialog(unity: &mut Unity, scene: &Level) -> Result<Level> {
    let mut level = panel(unity, scene, "_confirmDialogUI")?;
    let dialog = level
        .scene
        .nodes
        .iter()
        .flat_map(|n| &n.components)
        .find(|c| c.script == "CharacterCreation.UI.ConfirmDialog.CustomConfirmDialog")
        .context("editor confirm")?;
    let choices: Vec<_> = dialog
        .fields
        .array("_infos")
        .context("editor confirm types")?
        .iter()
        .map(|v| match v {
            Value::PPtr(p) => level.owner(p.path_id).context("editor confirm type"),
            _ => anyhow::bail!("invalid dialog reference"),
        })
        .collect::<Result<_>>()?;
    let choice = *choices.get(1).context("TwoChoices dialog")?;
    for item in choices {
        level.set_active(item, item == choice);
    }
    crate::talk::show(&mut level, choice);
    let info = level.scene.nodes[choice]
        .components
        .iter()
        .find(|c| c.script.ends_with(".BaseInfoComponent"))
        .context("editor TwoChoices")?;
    let theme = node(&level, &info.fields, "_theme").context("editor dialog theme")?;
    let fields = &level.scene.nodes[theme]
        .components
        .iter()
        .find(|c| c.script == "AL.Dialog.ConfirmDialogTheme")
        .context("editor dialog theme fields")?
        .fields;
    let title = node(&level, fields, "_txtTitle").context("dialog title")?;
    let body = node(&level, fields, "_txtSentence").context("dialog message")?;
    let warning = node(&level, fields, "_txtSentenceWarning");
    let buttons: Vec<_> = fields
        .array("_btnContents")
        .context("dialog actions")?
        .iter()
        .map(|v| match v {
            Value::PPtr(p) => level.owner(p.path_id).context("dialog action"),
            _ => anyhow::bail!("invalid dialog button"),
        })
        .collect::<Result<_>>()?;
    let [yes, no] = buttons.as_slice() else {
        anyhow::bail!("unsupported editor exit choices")
    };
    if let Some(warn) = warning {
        level.set_active(warn, false);
    }
    for (owner, label) in [
        (title, "終了確認"),
        (body, "キャラクリエイトを終了しますか？"),
        (*yes, "はい"),
        (*no, "いいえ"),
    ] {
        let binding = &level.scene.nodes[owner]
            .components
            .iter()
            .find(|c| c.script == "ILLGAMES.Unity.UI.UIText")
            .context("editor dialog UIText")?
            .fields;
        // UIText.SetText writes to each live reference. The title holder in
        // the original TwoChoices prefab has both references deliberately null.
        let targets = ["_unityText", "_tmpText"].map(|key| node(&level, binding, key));
        for target in targets.into_iter().flatten() {
            level.states.entry(target).or_default().text = Some(label.to_owned());
        }
    }
    Ok(level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_asset::pbr::{
        Channel, ChannelPath, Clip, Interpolation, PbrModel, SceneNode, Trs,
    };
    use glam::{Quat, Vec3};

    #[test]
    fn editor_camera_tracks_animated_head_on_click_and_reframes_body() {
        let preset = |y, distance| CameraPreset {
            pos: Vec3::Y * y,
            dir: Vec3::NEG_Z * distance,
            rot: Quat::from_rotation_y(std::f32::consts::PI),
        };
        let panel = CameraPanel {
            exit_button: 0,
            hints: vec![],
            face_button: 1,
            body_button: 2,
            face: preset(0.62, 5.5),
            body: preset(9., 46.),
            head: 0,
            head_offset: Vec3::ZERO,
            correction: Vec3::new(0., 0.25, -0.25),
            entered: 500,
            rig: PbrModel {
                roots: vec![0],
                nodes: vec![SceneNode {
                    name: "cf_s_head".into(),
                    children: vec![],
                    mesh: None,
                    skin: None,
                    rest: Trs {
                        translation: Vec3::Y * 12.,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::ONE,
                    },
                }],
                animations: vec![Clip {
                    name: "pose".into(),
                    duration: 1.,
                    channels: vec![Channel {
                        node: 0,
                        path: ChannelPath::Translation,
                        interpolation: Interpolation::Linear,
                        times: vec![0., 1.],
                        values: vec![[0., 12., 0., 0.], [0., 16., 0., 0.]],
                    }],
                }],
                ..Default::default()
            },
        };
        let lens = crate::map::View {
            eye: [0.; 3],
            target: [0.; 3],
            up: [0., 1., 0.],
            near: 0.1,
            far: 150.,
            fov: 23.,
        };
        let check = |button, now, expected| {
            let view = panel.click(button, now, &lens).unwrap();
            assert!(Vec3::from_array(view.eye).abs_diff_eq(expected, 0.0001));
            assert!(Vec3::from_array(view.target).abs_diff_eq(expected - Vec3::Z, 0.0001));
            assert_eq!(view.fov, lens.fov);
        };
        check(1, 500, Vec3::new(0., 12.87, 5.75));
        check(1, 1000, Vec3::new(0., 14.87, 5.75));
        check(2, 1000, Vec3::new(0., 9., 46.));
        assert!(panel.click(3, 1000, &lens).is_none());
    }
}
