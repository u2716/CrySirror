// SPDX-License-Identifier: MPL-2.0
//! uGUI layout of a [`Scene6`] into renderer-independent draw items:
//! `Canvas`/`CanvasScaler` (screen-space overlay, constant pixel size or
//! scale with screen size), `RectTransform` rectangles (anchors, pivot,
//! anchored position, size delta, local scale), `CanvasGroup` alpha,
//! `Image` (simple and sliced sprites, plain colour) and `TextMeshProUGUI`
//! text runs, plus raycast targets for hit testing. Layout groups are not
//! recomputed: the serialized rectangles carry the editor's last layout.
//!
//! Output coordinates are screen pixels, origin top-left, y down.
use crate::managed::{Fields, Value};
use crate::scene6::{Scene6, CANVAS, CANVAS_GROUP, RECT_TRANSFORM};
use crate::{class_id, PPtr};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }
    /// The overlap of two rectangles, if any.
    pub fn intersect(&self, o: &Rect) -> Option<Rect> {
        let r = Rect { x0: self.x0.max(o.x0), y0: self.y0.max(o.y0), x1: self.x1.min(o.x1), y1: self.y1.min(o.y1) };
        (r.x1 > r.x0 && r.y1 > r.y0).then_some(r)
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
}

/// Run-time overrides of a node (set by game logic).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeState {
    pub active: Option<bool>,
    /// `CanvasGroup.alpha`.
    pub group_alpha: Option<f32>,
    /// `Graphic.color`.
    pub color: Option<[f32; 4]>,
    /// `TMP_Text.text`.
    pub text: Option<String>,
    /// `Image.sprite` (a sprite of the scene's file).
    pub sprite: Option<PPtr>,
    /// A runtime image (consumer asset path) shown by an `Image`/`RawImage`
    /// instead of its sprite/texture (e.g. `RawImage.texture` set by code).
    pub image: Option<String>,
    pub anchored_position: Option<[f32; 2]>,
    pub size_delta: Option<[f32; 2]>,
    /// `RectTransform.anchorMin`/`anchorMax` (e.g. a `Slider` fill rect).
    pub anchor_min: Option<[f32; 2]>,
    pub anchor_max: Option<[f32; 2]>,
    pub scale: Option<[f32; 3]>,
    /// `Image.fillAmount`.
    pub fill: Option<f32>,
    /// `CanvasGroup.blocksRaycasts`.
    pub blocks_raycasts: Option<bool>,
    /// `CanvasGroup.interactable`.
    pub interactable: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    /// A sprite (or the white texture when `sprite` is null) with
    /// nine-slice borders in canvas units (left, bottom, right, top).
    /// `sliced`: borders (left, bottom, right, top) in canvas units and in
    /// sprite pixels.
    /// `image`: a runtime image replacing the sprite ([`NodeState::image`]).
    Image { sprite: PPtr, color: [f32; 4], sliced: Option<([f32; 4], [f32; 4])>, fill: Option<(i64, f32)>, image: Option<String> },
    Text { text: String, font: PPtr, size: f32, color: [f32; 4], alignment: i64, wrap: bool, margin: [f32; 4] },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawItem {
    pub node: usize,
    pub rect: Rect,
    /// Canvas units → screen pixels.
    pub scale: f32,
    pub paint: Paint,
    /// Canvas sorting order, then hierarchy order.
    pub order: (i64, u32),
    /// Screen rectangle of the enclosing `RectMask2D`/`Mask` ancestors.
    pub clip: Option<Rect>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub items: Vec<DrawItem>,
    /// Raycast targets in draw order: (node, rect). Graphics under a
    /// `CanvasGroup` that does not block raycasts are left out.
    pub hits: Vec<(usize, Rect)>,
    /// Laid-out nodes whose `CanvasGroup` chain is not interactable
    /// (`Selectable.IsInteractable` also requires its own flag).
    pub non_interactable: std::collections::HashSet<usize>,
    pub rects: HashMap<usize, Rect>,
}

fn color(f: &Fields, name: &str) -> [f32; 4] {
    match f.get(name) {
        Some(Value::Struct(s)) => ["r", "g", "b", "a"].map(|k| s.float(k).unwrap_or(1.) as f32),
        _ => [1.; 4],
    }
}

fn vec2(f: &Fields, name: &str, d: [f32; 2]) -> [f32; 2] {
    match f.get(name) {
        Some(Value::Struct(s)) => [s.float("x").map_or(d[0], |v| v as f32), s.float("y").map_or(d[1], |v| v as f32)],
        _ => d,
    }
}

fn pptr(f: &Fields, name: &str) -> PPtr {
    match f.get(name) {
        Some(Value::PPtr(p)) => *p,
        _ => PPtr::default(),
    }
}

/// Sprite border lookup for sliced images: (left, bottom, right, top) in
/// sprite pixels and the sprite's pixels per unit.
pub type SpriteInfo<'a> = &'a dyn Fn(PPtr) -> Option<([f32; 4], f32)>;

/// Sprite size in pixels (`Sprite.rect`) for `Image` preferred sizes.
pub type SpriteSize<'a> = &'a dyn Fn(PPtr) -> Option<([f32; 2], f32)>;

/// Text measurement: (text, font asset, font size) → (width, height) in
/// canvas units of the unwrapped text.
pub type MeasureText<'a> = &'a dyn Fn(&str, PPtr, f32) -> [f32; 2];

/// Layout inputs supplied by the consumer.
pub struct Services<'a> {
    pub sprite_info: SpriteInfo<'a>,
    pub sprite_size: SpriteSize<'a>,
    pub measure: MeasureText<'a>,
}

struct Walk {
    /// Parent rectangle in canvas units, y up (x0, y0 bottom-left).
    parent: Rect,
    /// Canvas units → screen pixels and the canvas origin on screen.
    scale: f32,
    origin: [f32; 2],
    alpha: f32,
    sorting: i64,
    in_canvas: bool,
    blocks: bool,
    interactable: bool,
    /// Intersected screen rectangles of the masking ancestors.
    clip: Option<Rect>,
}

/// Lay out every active canvas of the scene for a `width`×`height` screen.
pub fn layout(scene: &Scene6, states: &HashMap<usize, NodeState>, width: f32, height: f32, svc: &Services) -> Layout {
    let mut out = Layout::default();
    let mut order = 0u32;
    let mut driven: HashMap<usize, crate::scene6::RectParams> = HashMap::new();
    for &r in &scene.roots {
        let w = Walk { parent: Rect { x0: 0., y0: 0., x1: width, y1: height }, scale: 1., origin: [0., 0.], alpha: 1., sorting: 0, in_canvas: false, blocks: true, interactable: true, clip: None };
        walk(scene, states, r, w, width, height, svc, &mut driven, &mut out, &mut order);
    }
    out.items.sort_by(|a, b| a.order.cmp(&b.order));
    out
}

/// `CanvasGroup` of a node over the inherited walk state: (alpha,
/// blocksRaycasts, interactable), `ignoreParentGroups` resetting the flags.
fn canvas_group(scene: &Scene6, n: usize, st: Option<&NodeState>, w: &Walk) -> (f32, bool, bool) {
    let (mut alpha, mut blocks, mut interactable) = (w.alpha, w.blocks, w.interactable);
    if let Some(g) = scene.component(n, CANVAS_GROUP, "") {
        alpha *= st.and_then(|s| s.group_alpha).unwrap_or(g.fields.float("m_Alpha").unwrap_or(1.) as f32);
        let own_blocks = st.and_then(|s| s.blocks_raycasts).unwrap_or(g.fields.int("m_BlocksRaycasts").unwrap_or(1) != 0);
        let own_inter = st.and_then(|s| s.interactable).unwrap_or(g.fields.int("m_Interactable").unwrap_or(1) != 0);
        if g.fields.int("m_IgnoreParentGroups").unwrap_or(0) != 0 {
            (blocks, interactable) = (own_blocks, own_inter);
        } else {
            (blocks, interactable) = (blocks && own_blocks, interactable && own_inter);
        }
    }
    (alpha, blocks, interactable)
}

#[allow(clippy::too_many_arguments)]
fn walk(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize, mut w: Walk, sw: f32, sh: f32, svc: &Services, driven: &mut HashMap<usize, crate::scene6::RectParams>, out: &mut Layout, order: &mut u32) {
    let sprite_info = svc.sprite_info;
    let node = &scene.nodes[n];
    let st = states.get(&n);
    if !st.and_then(|s| s.active).unwrap_or(node.active && !dropdown_template(scene, n)) {
        return;
    }
    // A root Canvas (screen space overlay/camera) defines the canvas space.
    if let Some(c) = scene.component(n, CANVAS, "") {
        if c.fields.int("m_Enabled").unwrap_or(1) != 0 && !w.in_canvas {
            let mut scale = c.fields.float("m_ScaleFactor").unwrap_or(1.) as f32;
            if let Some(cs) = node.components.iter().find(|c| c.script.ends_with("CanvasScaler")) {
                let f = &cs.fields;
                match f.int("m_UiScaleMode").unwrap_or(0) {
                    1 => {
                        let reference = vec2(f, "m_ReferenceResolution", [800., 600.]);
                        let m = f.float("m_MatchWidthOrHeight").unwrap_or(0.) as f32;
                        scale = match f.int("m_ScreenMatchMode").unwrap_or(0) {
                            1 => (sw / reference[0]).min(sh / reference[1]),
                            2 => (sw / reference[0]).max(sh / reference[1]),
                            _ => {
                                let lw = (sw / reference[0]).log2();
                                let lh = (sh / reference[1]).log2();
                                2f32.powf(lw + (lh - lw) * m)
                            }
                        };
                    }
                    0 => scale = f.float("m_ScaleFactor").unwrap_or(1.) as f32,
                    _ => {}
                }
            }
            // A CanvasGroup on the Canvas object applies to everything in it.
            let (alpha, blocks, interactable) = canvas_group(scene, n, st, &w);
            w = Walk { parent: Rect { x0: 0., y0: 0., x1: sw / scale, y1: sh / scale }, scale, origin: [0., 0.], alpha, sorting: c.fields.int("m_SortingOrder").unwrap_or(0), in_canvas: true, blocks, interactable, clip: None };
            out.rects.insert(n, Rect { x0: 0., y0: 0., x1: sw, y1: sh });
            if !interactable {
                out.non_interactable.insert(n);
            }
            for &k in &node.children {
                walk(scene, states, k, Walk { ..w }, sw, sh, svc, driven, out, order);
            }
            return;
        }
    }
    if !w.in_canvas {
        for &k in &node.children {
            walk(scene, states, k, Walk { ..w }, sw, sh, svc, driven, out, order);
        }
        return;
    }
    // RectTransform rectangle in the parent's space (canvas units, y up).
    let Some(mut rp) = driven.get(&n).copied().or(node.xform.rect) else { return };
    if let Some(p) = st.and_then(|s| s.anchored_position) {
        rp.anchored_position = p;
    }
    if let Some(d) = st.and_then(|s| s.size_delta) {
        rp.size_delta = d;
    }
    if let Some(a) = st.and_then(|s| s.anchor_min) {
        rp.anchor_min = a;
    }
    if let Some(a) = st.and_then(|s| s.anchor_max) {
        rp.anchor_max = a;
    }
    let scale3 = st.and_then(|s| s.scale).unwrap_or(node.xform.scale);
    let pr = w.parent;
    let amin = [pr.x0 + rp.anchor_min[0] * pr.width(), pr.y0 + rp.anchor_min[1] * pr.height()];
    let amax = [pr.x0 + rp.anchor_max[0] * pr.width(), pr.y0 + rp.anchor_max[1] * pr.height()];
    let mut size = [amax[0] - amin[0] + rp.size_delta[0], amax[1] - amin[1] + rp.size_delta[1]];
    // ContentSizeFitter (min 1 / preferred 2 per axis) resizes the node
    // around its pivot.
    if let Some(csf) = script(scene, n, "ContentSizeFitter") {
        for (axis, key) in [(0, "m_HorizontalFit"), (1, "m_VerticalFit")] {
            let fit = csf.int(key).unwrap_or(0);
            if fit != 0 {
                let (min, pref, _) = sizes(scene, states, n, axis, svc);
                let want = if fit == 1 { min } else { pref };
                rp.size_delta[axis] += want - size[axis];
                size[axis] = want;
            }
        }
    }
    let pivot_pos = [
        amin[0] + (amax[0] - amin[0]) * rp.pivot[0] + rp.anchored_position[0],
        amin[1] + (amax[1] - amin[1]) * rp.pivot[1] + rp.anchored_position[1],
    ];
    // Local scale applies around the pivot (rotation is not laid out).
    let sx = scale3[0];
    let sy = scale3[1];
    let local = Rect {
        x0: pivot_pos[0] - rp.pivot[0] * size[0] * sx,
        y0: pivot_pos[1] - rp.pivot[1] * size[1] * sy,
        x1: pivot_pos[0] + (1. - rp.pivot[0]) * size[0] * sx,
        y1: pivot_pos[1] + (1. - rp.pivot[1]) * size[1] * sy,
    };
    let screen = Rect {
        x0: w.origin[0] + local.x0 * w.scale,
        x1: w.origin[0] + local.x1 * w.scale,
        y0: sh - (w.origin[1] + local.y1 * w.scale),
        y1: sh - (w.origin[1] + local.y0 * w.scale),
    };
    out.rects.insert(n, screen);
    let (alpha, blocks, interactable) = canvas_group(scene, n, st, &w);
    if !interactable {
        out.non_interactable.insert(n);
    }
    // Children see this rect in unscaled local units (size, not scaled
    // size); scale carries into their screen mapping.
    let child_parent = Rect { x0: 0., y0: 0., x1: size[0], y1: size[1] };
    let child_origin = [w.origin[0] + local.x0 * w.scale, w.origin[1] + local.y0 * w.scale];
    let child_scale = w.scale * sx;
    for c in &node.components {
        if c.class_id != class_id::MONO_BEHAVIOUR || c.fields.int("m_Enabled").unwrap_or(1) == 0 {
            continue;
        }
        let f = &c.fields;
        let tint = st.and_then(|s| s.color).unwrap_or_else(|| color(f, "m_Color"));
        let tint = [tint[0], tint[1], tint[2], tint[3] * alpha];
        // A `Mask` without `showMaskGraphic` only clips.
        let hidden_graphic = script(scene, n, "Mask").is_some_and(|m| m.int("m_ShowMaskGraphic").unwrap_or(1) == 0);
        let paint = match c.script.rsplit('.').next().unwrap_or("") {
            "Image" | "RawImage" if hidden_graphic => None,
            "Image" | "RawImage" => {
                let sprite = st.and_then(|s| s.sprite).unwrap_or_else(|| pptr(f, if c.script.ends_with("RawImage") { "m_Texture" } else { "m_Sprite" }));
                let ty = f.int("m_Type").unwrap_or(0);
                let sliced = if ty == 1 || ty == 2 {
                    sprite_info(sprite).filter(|(b, _)| b.iter().any(|v| *v > 0.)).map(|(b, ppu)| {
                        let mult = f.float("m_PixelsPerUnitMultiplier").unwrap_or(1.) as f32;
                        // Canvas reference pixels per unit 100.
                        let k = 100. / (ppu * mult.max(1e-3));
                        (b.map(|v| v * k), b)
                    })
                } else {
                    None
                };
                let fill = (ty == 3).then(|| (f.int("m_FillMethod").unwrap_or(0), st.and_then(|s| s.fill).unwrap_or(f.float("m_FillAmount").unwrap_or(1.) as f32)));
                let image = st.and_then(|s| s.image.clone());
                let sliced = if image.is_some() { None } else { sliced };
                Some(Paint::Image { sprite, color: tint, sliced, fill, image })
            }
            "TextMeshProUGUI" => {
                let text = st.and_then(|s| s.text.clone()).unwrap_or_else(|| f.string("m_text").unwrap_or("").to_owned());
                let fc = color(f, "m_fontColor");
                let fc = st.and_then(|s| s.color).unwrap_or(fc);
                let margin = match f.get("m_margin") {
                    Some(Value::Struct(s)) => ["x", "y", "z", "w"].map(|k| s.float(k).unwrap_or(0.) as f32),
                    _ => [0.; 4],
                };
                Some(Paint::Text {
                    text,
                    font: pptr(f, "m_fontAsset"),
                    size: f.float("m_fontSize").unwrap_or(36.) as f32,
                    color: [fc[0], fc[1], fc[2], fc[3] * alpha],
                    alignment: f.int("m_HorizontalAlignment").map(|h| h | (f.int("m_VerticalAlignment").unwrap_or(256))).unwrap_or(f.int("m_textAlignment").unwrap_or(257)),
                    wrap: f.int("m_TextWrappingMode").map(|m| m != 0).unwrap_or(f.int("m_enableWordWrapping").unwrap_or(1) != 0),
                    margin,
                })
            }
            _ => None,
        };
        if let Some(paint) = paint {
            *order += 1;
            // Masked graphics only take raycasts inside the mask.
            let hit = match w.clip {
                Some(c) => screen.intersect(&c),
                None => Some(screen),
            };
            if let Some(hit) = hit.filter(|_| f.int("m_RaycastTarget").unwrap_or(0) != 0 && blocks) {
                out.hits.push((n, hit));
            }
            out.items.push(DrawItem { node: n, rect: screen, scale: w.scale * sx, paint, order: (w.sorting, *order), clip: w.clip });
        }
    }
    let _ = RECT_TRANSFORM;
    // Horizontal/VerticalLayoutGroup drive their children's rectangles.
    for (axis, name) in [(0usize, "HorizontalLayoutGroup"), (1, "VerticalLayoutGroup")] {
        if let Some(g) = script(scene, n, name) {
            arrange(scene, states, n, g, axis, size, svc, driven);
        }
    }
    if let Some(g) = script(scene, n, "GridLayoutGroup") {
        arrange_grid(scene, states, n, g, size, driven);
    }
    // `RectMask2D` and `Mask` clip the children to this rectangle (a
    // `Mask` by its graphic's shape; the rectangle is used).
    let clip = if script(scene, n, "RectMask2D").is_some() || script(scene, n, "Mask").is_some() {
        Some(w.clip.map_or(screen, |c| c.intersect(&screen).unwrap_or(Rect { x0: screen.x0, y0: screen.y0, x1: screen.x0, y1: screen.y0 })))
    } else {
        w.clip
    };
    for &k in &node.children {
        walk(scene, states, k, Walk { parent: child_parent, scale: child_scale, origin: child_origin, alpha, sorting: w.sorting, in_canvas: true, blocks, interactable, clip }, sw, sh, svc, driven, out, order);
    }
}

/// A Dropdown's `m_Template` (deactivated in the dropdown's `Start`; shown
/// only while the list is open).
fn dropdown_template(scene: &Scene6, n: usize) -> bool {
    let Some(p) = scene.nodes[n].parent else { return false };
    scene.nodes[p].components.iter().any(|c| c.fields.pptr("m_Template").is_some_and(|t| t.path_id == scene.nodes[n].transform))
}

/// The node's enabled MonoBehaviour with this script class (last name part).
fn script<'a>(scene: &'a Scene6, n: usize, name: &str) -> Option<&'a Fields> {
    scene.nodes[n]
        .components
        .iter()
        .find(|c| c.class_id == class_id::MONO_BEHAVIOUR && c.fields.int("m_Enabled").unwrap_or(1) != 0 && c.script.rsplit('.').next() == Some(name))
        .map(|c| &c.fields)
}

fn active(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize) -> bool {
    states.get(&n).and_then(|s| s.active).unwrap_or(scene.nodes[n].active)
}

fn padding(g: &Fields) -> [f32; 4] {
    match g.get("m_Padding") {
        Some(Value::Struct(s)) => ["m_Left", "m_Right", "m_Top", "m_Bottom"].map(|k| s.int(k).unwrap_or(0) as f32),
        _ => [0.; 4],
    }
}

/// Layout children of a group (active, not `ignoreLayout`).
fn layout_children(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize) -> Vec<usize> {
    scene.nodes[n]
        .children
        .iter()
        .copied()
        .filter(|&c| active(scene, states, c) && script(scene, c, "LayoutElement").is_none_or(|e| e.int("m_IgnoreLayout").unwrap_or(0) == 0))
        .collect()
}

/// `LayoutUtility` min/preferred/flexible size of a node along `axis`.
fn sizes(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize, axis: usize, svc: &Services) -> (f32, f32, f32) {
    let (mut min, mut pref, mut flex) = (0f32, 0f32, 0f32);
    let group = [(0usize, "HorizontalLayoutGroup"), (1, "VerticalLayoutGroup")].into_iter().find_map(|(a, name)| script(scene, n, name).map(|g| (a, g)));
    if let Some((along, g)) = group {
        let pad = padding(g);
        let edge = if axis == 0 { pad[0] + pad[1] } else { pad[2] + pad[3] };
        let spacing = g.float("m_Spacing").unwrap_or(0.) as f32;
        let control = g.int(if axis == 0 { "m_ChildControlWidth" } else { "m_ChildControlHeight" }).unwrap_or(1) != 0;
        let kids = layout_children(scene, states, n);
        for (i, &c) in kids.iter().enumerate() {
            let (cmin, cpref, cflex) = if control {
                sizes(scene, states, c, axis, svc)
            } else {
                let d = states.get(&c).and_then(|s| s.size_delta).map(|d| d[axis]).or_else(|| scene.nodes[c].xform.rect.map(|r| r.size_delta[axis])).unwrap_or(0.);
                (d, d, 0.)
            };
            if along == axis {
                min += cmin;
                pref += cpref;
                flex += cflex;
                if i > 0 {
                    min += spacing;
                    pref += spacing;
                }
            } else {
                min = min.max(cmin);
                pref = pref.max(cpref);
                flex = flex.max(cflex);
            }
        }
        min += edge;
        pref = pref.max(min - edge) + edge;
    } else if let Some(g) = script(scene, n, "GridLayoutGroup") {
        // `GridLayoutGroup.CalculateLayoutInput*`: columns by the constraint
        // (flexible: ⌈√n⌉ preferred), rows to hold the children.
        let kids = layout_children(scene, states, n).len().max(1);
        let pad = padding(g);
        let cell = vec2(g, "m_CellSize", [100., 100.]);
        let spacing = vec2(g, "m_Spacing", [0., 0.]);
        let count = g.int("m_ConstraintCount").unwrap_or(2).max(1) as usize;
        let cols = match g.int("m_Constraint").unwrap_or(0) {
            1 => count,
            2 => kids.div_ceil(count),
            _ => (kids as f32).sqrt().ceil() as usize,
        }
        .max(1);
        let rows = match g.int("m_Constraint").unwrap_or(0) {
            2 => count,
            _ => kids.div_ceil(cols),
        };
        let (k, edge) = if axis == 0 { (cols, pad[0] + pad[1]) } else { (rows, pad[2] + pad[3]) };
        let v = edge + k as f32 * cell[axis] + k.saturating_sub(1) as f32 * spacing[axis];
        min = v;
        pref = v;
    } else {
        // ILayoutElement graphics.
        for c in &scene.nodes[n].components {
            if c.class_id != class_id::MONO_BEHAVIOUR || c.fields.int("m_Enabled").unwrap_or(1) == 0 {
                continue;
            }
            match c.script.rsplit('.').next().unwrap_or("") {
                "Image" => {
                    let sp = states.get(&n).and_then(|s| s.sprite).unwrap_or_else(|| pptr(&c.fields, "m_Sprite"));
                    if let Some((px, ppu)) = (svc.sprite_size)(sp) {
                        let k = 100. / ppu.max(1e-3);
                        if c.fields.int("m_Type").unwrap_or(0) == 0 {
                            pref = pref.max(px[axis] * k);
                        }
                    }
                }
                "TextMeshProUGUI" => {
                    let text = states.get(&n).and_then(|s| s.text.clone()).unwrap_or_else(|| c.fields.string("m_text").unwrap_or("").to_owned());
                    let m = (svc.measure)(&text, pptr(&c.fields, "m_fontAsset"), c.fields.float("m_fontSize").unwrap_or(36.) as f32);
                    pref = pref.max(m[axis]);
                }
                _ => {}
            }
        }
    }
    if let Some(e) = script(scene, n, "LayoutElement") {
        let get = |k: &str| e.float(k).map(|v| v as f32).filter(|v| *v >= 0.);
        let (kmin, kpref, kflex) = if axis == 0 { ("m_MinWidth", "m_PreferredWidth", "m_FlexibleWidth") } else { ("m_MinHeight", "m_PreferredHeight", "m_FlexibleHeight") };
        if let Some(v) = get(kmin) {
            min = v;
        }
        if let Some(v) = get(kpref) {
            pref = v;
        }
        if let Some(v) = get(kflex) {
            flex = v;
        }
    }
    (min, pref.max(min), flex)
}

/// `HorizontalOrVerticalLayoutGroup.SetChildrenAlongAxis` for both axes of
/// a group of size `size` (canvas units): children get top-left anchors.
#[allow(clippy::too_many_arguments)]
fn arrange(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize, g: &Fields, along: usize, size: [f32; 2], svc: &Services, driven: &mut HashMap<usize, crate::scene6::RectParams>) {
    let kids = layout_children(scene, states, n);
    if kids.is_empty() {
        return;
    }
    let pad = padding(g);
    let spacing = g.float("m_Spacing").unwrap_or(0.) as f32;
    // TextAnchor: 0..8 = upper/middle/lower × left/center/right.
    let anchor = g.int("m_ChildAlignment").unwrap_or(0);
    let align = [(anchor % 3) as f32 * 0.5, (anchor / 3) as f32 * 0.5];
    let reverse = g.int("m_ReverseArrangement").unwrap_or(0) != 0;
    let mut pos = [[0f32; 2]; 0].to_vec();
    let mut sz = Vec::new();
    for axis in 0..2 {
        let control = g.int(if axis == 0 { "m_ChildControlWidth" } else { "m_ChildControlHeight" }).unwrap_or(1) != 0;
        let expand = g.int(if axis == 0 { "m_ChildForceExpandWidth" } else { "m_ChildForceExpandHeight" }).unwrap_or(1) != 0;
        let inner = size[axis] - if axis == 0 { pad[0] + pad[1] } else { pad[2] + pad[3] };
        let lead = if axis == 0 { pad[0] } else { pad[2] };
        let child_sizes: Vec<(f32, f32, f32)> = kids
            .iter()
            .map(|&c| {
                if control {
                    let (a, b, f) = sizes(scene, states, c, axis, svc);
                    (a, b, if expand { f.max(1.) } else { f })
                } else {
                    let d = states.get(&c).and_then(|s| s.size_delta).map(|d| d[axis]).or_else(|| scene.nodes[c].xform.rect.map(|r| r.size_delta[axis])).unwrap_or(0.);
                    (d, d, if expand { 1. } else { 0. })
                }
            })
            .collect();
        if axis == along {
            let total_pref: f32 = child_sizes.iter().map(|c| c.1).sum::<f32>() + spacing * (kids.len() - 1) as f32;
            let total_flex: f32 = child_sizes.iter().map(|c| c.2).sum();
            let extra = (inner - total_pref).max(0.);
            let mut cursor = lead;
            if total_flex == 0. {
                cursor += (inner - total_pref).max(0.) * align[axis];
            }
            let order: Vec<usize> = if reverse { (0..kids.len()).rev().collect() } else { (0..kids.len()).collect() };
            let mut p = vec![0f32; kids.len()];
            let mut s = vec![0f32; kids.len()];
            for i in order {
                // `LayoutGroup.SetChildrenAlongAxis`: each cell takes its
                // flexible share; an uncontrolled child keeps its size and
                // sits at `(cell - size) * alignment` inside the cell.
                let (_, pref, flex) = child_sizes[i];
                let mut cell = pref;
                if total_flex > 0. {
                    cell += extra * flex / total_flex;
                }
                if control {
                    p[i] = cursor;
                    s[i] = cell;
                } else {
                    p[i] = cursor + (cell - pref) * align[axis];
                    s[i] = pref;
                }
                cursor += cell + spacing;
            }
            pos.push([0.; 2]);
            sz.push(s.clone());
            for (i, v) in p.into_iter().enumerate() {
                if pos.len() <= i + 1 {
                    pos.resize(i + 2, [0.; 2]);
                }
                pos[i + 1][axis] = v;
            }
        } else {
            let mut s = vec![0f32; kids.len()];
            for (i, &(min, pref, flex)) in child_sizes.iter().enumerate() {
                let w = if control { if flex > 0. { inner.max(min) } else { pref.min(inner).max(min) } } else { pref };
                s[i] = w;
                if pos.len() <= i + 1 {
                    pos.resize(i + 2, [0.; 2]);
                }
                pos[i + 1][axis] = lead + (inner - w) * align[axis];
            }
            sz.push(s);
        }
    }
    // sz holds the axis order pushed (0 then 1).
    for (i, &c) in kids.iter().enumerate() {
        let Some(mut rp) = scene.nodes[c].xform.rect else { continue };
        let (x, y) = (pos[i + 1][0], pos[i + 1][1]);
        let (w, h) = (sz[0][i], sz[1][i]);
        rp.anchor_min = [0., 1.];
        rp.anchor_max = [0., 1.];
        rp.size_delta = [w, h];
        rp.anchored_position = [x + rp.pivot[0] * w, -(y + (1. - rp.pivot[1]) * h)];
        driven.insert(c, rp);
    }
}

/// `GridLayoutGroup`: fixed `m_CellSize` cells with `m_Spacing`, filled from
/// `m_StartCorner` along `m_StartAxis`, columns/rows from `m_Constraint`
/// (0 flexible, 1 fixed columns, 2 fixed rows); the block is aligned by
/// `m_ChildAlignment` inside the padding.
fn arrange_grid(scene: &Scene6, states: &HashMap<usize, NodeState>, n: usize, g: &Fields, size: [f32; 2], driven: &mut HashMap<usize, crate::scene6::RectParams>) {
    let kids = layout_children(scene, states, n);
    if kids.is_empty() {
        return;
    }
    let pad = padding(g);
    let cell = vec2(g, "m_CellSize", [100., 100.]);
    let spacing = vec2(g, "m_Spacing", [0., 0.]);
    let inner = [size[0] - pad[0] - pad[1], size[1] - pad[2] - pad[3]];
    let count = g.int("m_ConstraintCount").unwrap_or(2).max(1) as usize;
    let fit = |axis: usize| (((inner[axis] + spacing[axis] + 0.001) / (cell[axis] + spacing[axis])).floor() as usize).max(1);
    let (cols, rows) = match g.int("m_Constraint").unwrap_or(0) {
        1 => (count, kids.len().div_ceil(count)),
        2 => (kids.len().div_ceil(count), count),
        _ => {
            let c = fit(0).min(kids.len());
            (c, kids.len().div_ceil(c))
        }
    };
    let corner = g.int("m_StartCorner").unwrap_or(0);
    let vertical_first = g.int("m_StartAxis").unwrap_or(0) == 1;
    let anchor = g.int("m_ChildAlignment").unwrap_or(0);
    let align = [(anchor % 3) as f32 * 0.5, (anchor / 3) as f32 * 0.5];
    let block = [cols as f32 * cell[0] + (cols - 1) as f32 * spacing[0], rows as f32 * cell[1] + (rows - 1) as f32 * spacing[1]];
    let start = [pad[0] + (inner[0] - block[0]).max(0.) * align[0], pad[2] + (inner[1] - block[1]).max(0.) * align[1]];
    for (i, &c) in kids.iter().enumerate() {
        let Some(mut rp) = scene.nodes[c].xform.rect else { continue };
        let (mut col, mut row) = if vertical_first { (i / rows, i % rows) } else { (i % cols, i / cols) };
        if corner % 2 == 1 {
            col = cols - 1 - col;
        }
        if corner >= 2 {
            row = rows - 1 - row;
        }
        let (x, y) = (start[0] + col as f32 * (cell[0] + spacing[0]), start[1] + row as f32 * (cell[1] + spacing[1]));
        rp.anchor_min = [0., 1.];
        rp.anchor_max = [0., 1.];
        rp.size_delta = cell;
        rp.anchored_position = [x + rp.pivot[0] * cell[0], -(y + (1. - rp.pivot[1]) * cell[1])];
        driven.insert(c, rp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene6::{Component, Node, RectParams, Xform};

    fn node(name: &str, parent: Option<usize>, children: Vec<usize>, components: Vec<Component>) -> Node {
        let rect = RectParams { anchor_min: [0.; 2], anchor_max: [1.; 2], anchored_position: [0.; 2], size_delta: [0.; 2], pivot: [0.5; 2] };
        Node { name: name.into(), active: true, game_object: 0, transform: 0, xform: Xform { rect: Some(rect), scale: [1.; 3], rotation: [0., 0., 0., 1.], ..Default::default() }, parent, children, components, origin: 0 }
    }

    fn component(class_id: i32, script: &str, fields: Vec<(&str, Value)>) -> Component {
        Component { path_id: 0, class_id, script: script.into(), fields: Fields(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()) }
    }

    /// A CanvasGroup on the root Canvas object fades and blocks everything
    /// under it (ActionMap `SubFadeCanvas`).
    #[test]
    fn root_canvas_group_applies() {
        let canvas = component(CANVAS, "", vec![("m_Enabled", Value::Int(1))]);
        let group = component(CANVAS_GROUP, "", vec![("m_Alpha", Value::Float(1.)), ("m_BlocksRaycasts", Value::Int(1)), ("m_Interactable", Value::Int(1))]);
        let image = component(class_id::MONO_BEHAVIOUR, "UnityEngine.UI.Image", vec![("m_RaycastTarget", Value::Int(1))]);
        let scene = Scene6 { nodes: vec![node("Fade", None, vec![1], vec![canvas, group]), node("Panel", Some(0), vec![], vec![image])], roots: vec![0], ..Default::default() };
        let svc = Services { sprite_info: &|_| None, sprite_size: &|_| None, measure: &|_, _, _| [0.; 2] };
        let alpha = |states: &HashMap<usize, NodeState>| {
            let l = layout(&scene, states, 1920., 1080., &svc);
            match &l.items[0].paint {
                Paint::Image { color, .. } => (color[3], l.hits.len()),
                _ => unreachable!(),
            }
        };
        assert_eq!(alpha(&HashMap::new()), (1., 1));
        let mut states = HashMap::new();
        states.insert(0, NodeState { group_alpha: Some(0.), blocks_raycasts: Some(false), ..Default::default() });
        assert_eq!(alpha(&states), (0., 0));
    }
}

/// `Slider.value` setter (`UpdateVisuals`): the fill rect's anchors span
/// the normalized value along the slider's direction (0 left-to-right,
/// 1 right-to-left, 2 bottom-to-top, 3 top-to-bottom); the handle rect's
/// anchors sit at the value.
pub fn slider_value(scene: &Scene6, slider: &Fields, value: f32, states: &mut HashMap<usize, NodeState>) {
    let (lo, hi) = (slider.float("m_MinValue").unwrap_or(0.) as f32, slider.float("m_MaxValue").unwrap_or(1.) as f32);
    let t = if hi > lo { ((value - lo) / (hi - lo)).clamp(0., 1.) } else { 0. };
    let dir = slider.int("m_Direction").unwrap_or(0);
    let axis = usize::from(dir >= 2);
    let at = if dir == 1 || dir == 3 { 1. - t } else { t };
    if let Some(handle) = slider.pptr("m_HandleRect").filter(|p| p.path_id != 0).and_then(|p| scene.nodes.iter().position(|n| n.transform == p.path_id || n.components.iter().any(|c| c.path_id == p.path_id))) {
        let (mut amin, mut amax) = ([0., 0.], [1., 1.]);
        amin[axis] = at;
        amax[axis] = at;
        let st = states.entry(handle).or_default();
        st.anchor_min = Some(amin);
        st.anchor_max = Some(amax);
    }
    let Some(fill) = slider.pptr("m_FillRect").and_then(|p| scene.nodes.iter().position(|n| n.transform == p.path_id || n.components.iter().any(|c| c.path_id == p.path_id))) else { return };
    let reverse = dir == 1 || dir == 3;
    let mut amin = [0., 0.];
    let mut amax = [1., 1.];
    if reverse {
        amin[axis] = 1. - t;
    } else {
        amax[axis] = t;
    }
    let st = states.entry(fill).or_default();
    st.anchor_min = Some(amin);
    st.anchor_max = Some(amax);
}

/// A `Selectable` (Button, Toggle or a subclass): it serialises
/// `m_Transition` and `m_Interactable`.
pub fn is_selectable(fields: &Fields) -> bool {
    fields.get("m_Transition").is_some() && fields.get("m_Interactable").is_some()
}

/// `Selectable.DoStateTransition` between the normal and the disabled state
/// on the target graphic: a colour tint (1) multiplies its colour by
/// `m_Colors` × `m_ColorMultiplier`, a sprite swap (2) overrides its sprite
/// with `m_SpriteState.m_DisabledSprite`. Animation transitions (3) are not
/// driven here.
pub fn selectable_transition(scene: &Scene6, selectable: &Fields, disabled: bool, states: &mut HashMap<usize, NodeState>) {
    let target = pptr(selectable, "m_TargetGraphic");
    if target.path_id == 0 {
        return;
    }
    let Some((node, graphic)) = scene.nodes.iter().enumerate().find_map(|(i, n)| n.components.iter().find(|c| c.path_id == target.path_id).map(|c| (i, c))) else { return };
    match selectable.int("m_Transition").unwrap_or(1) {
        1 => {
            let Some(Value::Struct(colors)) = selectable.get("m_Colors") else { return };
            let tint = color(colors, if disabled { "m_DisabledColor" } else { "m_NormalColor" });
            let k = colors.float("m_ColorMultiplier").unwrap_or(1.) as f32;
            let tint = tint.map(|c| (c * k).min(1.));
            let base = color(&graphic.fields, "m_Color");
            states.entry(node).or_default().color = (tint != [1.; 4]).then(|| std::array::from_fn(|i| base[i] * tint[i]));
        }
        2 => {
            let Some(Value::Struct(sprites)) = selectable.get("m_SpriteState") else { return };
            let sprite = pptr(sprites, "m_DisabledSprite");
            states.entry(node).or_default().sprite = (disabled && sprite.path_id != 0).then_some(sprite);
        }
        _ => {}
    }
}

#[cfg(test)]
mod selectable_tests {
    use super::*;
    use crate::scene6::{Component, Node};

    fn fields(v: Vec<(&str, Value)>) -> Fields {
        Fields(v.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    #[test]
    fn disabled_transition() {
        let rgba = |c: f64, a: f64| Value::Struct(fields(vec![("r", Value::Float(c)), ("g", Value::Float(c)), ("b", Value::Float(c)), ("a", Value::Float(a))]));
        let graphic = Component { path_id: 7, class_id: class_id::MONO_BEHAVIOUR, script: "UnityEngine.UI.Image".into(), fields: fields(vec![("m_Color", rgba(1., 1.))]) };
        let node = Node { name: String::new(), active: true, game_object: 1, transform: 2, xform: Default::default(), parent: None, children: Vec::new(), components: vec![graphic], origin: 0 };
        let scene = Scene6 { nodes: vec![node], ..Default::default() };
        let tint = fields(vec![
            ("m_Transition", Value::Int(1)),
            ("m_Interactable", Value::Int(1)),
            ("m_TargetGraphic", Value::PPtr(PPtr { file_id: 0, path_id: 7 })),
            ("m_Colors", Value::Struct(fields(vec![("m_NormalColor", rgba(1., 1.)), ("m_DisabledColor", rgba(0.5, 0.5)), ("m_ColorMultiplier", Value::Float(1.))]))),
        ]);
        assert!(is_selectable(&tint));
        let mut states = HashMap::new();
        selectable_transition(&scene, &tint, true, &mut states);
        assert_eq!(states[&0].color, Some([0.5; 4]));
        selectable_transition(&scene, &tint, false, &mut states);
        assert_eq!(states[&0].color, None);
        let swap = fields(vec![
            ("m_Transition", Value::Int(2)),
            ("m_TargetGraphic", Value::PPtr(PPtr { file_id: 0, path_id: 7 })),
            ("m_SpriteState", Value::Struct(fields(vec![("m_DisabledSprite", Value::PPtr(PPtr { file_id: 0, path_id: 9 }))]))),
        ]);
        selectable_transition(&scene, &swap, true, &mut states);
        assert_eq!(states[&0].sprite, Some(PPtr { file_id: 0, path_id: 9 }));
        selectable_transition(&scene, &swap, false, &mut states);
        assert_eq!(states[&0].sprite, None);
    }
}
