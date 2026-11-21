// SPDX-License-Identifier: MPL-2.0
//! Unity 2020+ scene hierarchies (levels and bundle prefabs) as plain data:
//! GameObjects with their Transform/RectTransform and decoded components,
//! for consumers that lay out uGUI or assemble 3D content.
use crate::managed::{Fields, Value};
use crate::{class_id, PPtr, SerializedFile, TreeRegistry};
use anyhow::{Context, Result};
use std::collections::HashMap;

pub const RECT_TRANSFORM: i32 = 224;
pub const CANVAS: i32 = 223;
pub const CANVAS_RENDERER: i32 = 222;
pub const CANVAS_GROUP: i32 = 225;
pub const ANIMATOR: i32 = 95;
pub const SPRITE: i32 = 213;

/// A Transform or RectTransform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Xform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    /// RectTransform: anchor min/max, anchored position, size delta, pivot.
    pub rect: Option<RectParams>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectParams {
    pub anchor_min: [f32; 2],
    pub anchor_max: [f32; 2],
    pub anchored_position: [f32; 2],
    pub size_delta: [f32; 2],
    pub pivot: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub path_id: i64,
    pub class_id: i32,
    /// Script class name of MonoBehaviours (`Ns.Class` without namespace
    /// when it has none), else empty.
    pub script: String,
    pub fields: Fields,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub active: bool,
    pub game_object: i64,
    pub transform: i64,
    pub xform: Xform,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub components: Vec<Component>,
    /// Serialized file of the node's object pointers: 0 the scene's own,
    /// `n` [`Scene6::files`]`[n − 1]` for grafted parts.
    pub origin: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene6 {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    /// Transform path id → node.
    pub by_transform: HashMap<i64, usize>,
    /// GameObject path id → node.
    pub by_game_object: HashMap<i64, usize>,
    /// File keys of grafted parts ([`Node::origin`] − 1).
    pub files: Vec<String>,
    /// Skinned-renderer bones resolved onto another node (a part's bone
    /// copy → the skeleton's bone of the same name).
    pub bone_alias: HashMap<usize, usize>,
}

fn v<'a>(f: &'a Fields, name: &str) -> Option<&'a Value> {
    f.get(name)
}

fn pptr(f: &Fields, name: &str) -> PPtr {
    match v(f, name) {
        Some(Value::PPtr(p)) => *p,
        _ => PPtr::default(),
    }
}

fn floats<const N: usize>(f: &Fields, name: &str, keys: [&str; N], default: [f32; N]) -> [f32; N] {
    match v(f, name) {
        Some(Value::Struct(s)) => {
            let mut out = default;
            for (i, k) in keys.iter().enumerate() {
                if let Some(x) = s.float(k) {
                    out[i] = x as f32;
                }
            }
            out
        }
        _ => default,
    }
}

fn xform(f: &Fields, rect: bool) -> Xform {
    let xy = ["x", "y"];
    Xform {
        position: floats(f, "m_LocalPosition", ["x", "y", "z"], [0.; 3]),
        rotation: floats(f, "m_LocalRotation", ["x", "y", "z", "w"], [0., 0., 0., 1.]),
        scale: floats(f, "m_LocalScale", ["x", "y", "z"], [1.; 3]),
        rect: rect.then(|| RectParams {
            anchor_min: floats(f, "m_AnchorMin", xy, [0.5; 2]),
            anchor_max: floats(f, "m_AnchorMax", xy, [0.5; 2]),
            anchored_position: floats(f, "m_AnchoredPosition", xy, [0.; 2]),
            size_delta: floats(f, "m_SizeDelta", xy, [100.; 2]),
            pivot: floats(f, "m_Pivot", xy, [0.5; 2]),
        }),
    }
}

impl Scene6 {
    /// Every hierarchy of `file`. `script_name` resolves a MonoBehaviour's
    /// `m_Script` pointer to its class name. Objects that do not decode are
    /// left out of their GameObject's components.
    pub fn load(file: &SerializedFile, registry: &TreeRegistry, script_name: &dyn Fn(PPtr) -> String) -> Result<Self> {
        let mut s = Scene6::default();
        let mut transforms: HashMap<i64, (Fields, bool)> = HashMap::new();
        let mut objects: Vec<(i64, i32, Fields)> = Vec::new();
        for info in &file.objects {
            let Ok(f) = file.decode_using(info, registry) else { continue };
            match info.class_id {
                class_id::GAME_OBJECT => {
                    let idx = s.nodes.len();
                    s.nodes.push(Node {
                        name: f.string("m_Name").unwrap_or("").to_owned(),
                        active: f.int("m_IsActive").unwrap_or(1) != 0,
                        game_object: info.path_id,
                        transform: 0,
                        xform: Xform::default(),
                        parent: None,
                        children: Vec::new(),
                        components: Vec::new(),
                        origin: 0,
                    });
                    s.by_game_object.insert(info.path_id, idx);
                }
                class_id::TRANSFORM | RECT_TRANSFORM => {
                    transforms.insert(info.path_id, (f, info.class_id == RECT_TRANSFORM));
                }
                c => objects.push((info.path_id, c, f)),
            }
        }
        for (&tid, (f, rect)) in &transforms {
            let go = pptr(f, "m_GameObject").path_id;
            if let Some(&n) = s.by_game_object.get(&go) {
                s.nodes[n].transform = tid;
                s.nodes[n].xform = xform(f, *rect);
                s.by_transform.insert(tid, n);
            }
        }
        for (&tid, (f, _)) in &transforms {
            let Some(&n) = s.by_transform.get(&tid) else { continue };
            if let Some(Value::Array(kids)) = v(f, "m_Children") {
                for k in kids {
                    if let Value::PPtr(p) = k {
                        if let Some(&c) = s.by_transform.get(&p.path_id) {
                            s.nodes[c].parent = Some(n);
                            s.nodes[n].children.push(c);
                        }
                    }
                }
            }
        }
        for (path_id, class_id, fields) in objects {
            let go = pptr(&fields, "m_GameObject").path_id;
            let Some(&n) = s.by_game_object.get(&go) else { continue };
            let script = if class_id == class_id::MONO_BEHAVIOUR { script_name(pptr(&fields, "m_Script")) } else { String::new() };
            s.nodes[n].components.push(Component { path_id, class_id, script, fields });
        }
        s.roots = (0..s.nodes.len()).filter(|&i| s.nodes[i].parent.is_none()).collect();
        Ok(s)
    }

    /// The hierarchy under `root` as its own scene (a prefab instance):
    /// `root` becomes the only root, path ids are kept.
    pub fn subtree(&self, root: usize) -> Scene6 {
        let mut out = Scene6::default();
        let mut map: HashMap<usize, usize> = HashMap::new();
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            let i = out.nodes.len();
            map.insert(n, i);
            let mut node = self.nodes[n].clone();
            node.parent = if n == root { None } else { node.parent.and_then(|p| map.get(&p).copied()) };
            node.children.clear();
            out.by_game_object.insert(node.game_object, i);
            out.by_transform.insert(node.transform, i);
            out.nodes.push(node);
            if let Some(p) = out.nodes[i].parent {
                out.nodes[p].children.push(i);
            }
            stack.extend(self.nodes[n].children.iter().rev());
        }
        out.roots = vec![0];
        out.files = self.files.clone();
        out.bone_alias = self.bone_alias.iter().filter_map(|(k, v)| Some((*map.get(k)?, *map.get(v)?))).collect();
        out
    }

    /// The file key of a node's pointers (`primary` for the scene's own).
    pub fn file_of<'a>(&'a self, node: usize, primary: &'a str) -> &'a str {
        match self.nodes[node].origin {
            0 => primary,
            o => self.files.get(o as usize - 1).map_or(primary, String::as_str),
        }
    }

    /// Attach `part` (a scene of `part_file`, e.g. a prefab instance) under
    /// `parent`; returns the index of the part's first root. Lookup maps
    /// keep the scene's own objects; grafted nodes are reached by index.
    pub fn graft(&mut self, part: &Scene6, part_file: &str, parent: usize) -> usize {
        let base = self.nodes.len();
        self.files.push(part_file.to_owned());
        let origin = self.files.len() as u32;
        for n in &part.nodes {
            let mut n = n.clone();
            n.parent = Some(n.parent.map_or(parent, |p| p + base));
            n.children = n.children.iter().map(|c| c + base).collect();
            n.origin = origin;
            self.nodes.push(n);
        }
        for &r in &part.roots {
            self.nodes[parent].children.push(r + base);
        }
        for (k, v) in &part.bone_alias {
            self.bone_alias.insert(k + base, v + base);
        }
        base + part.roots.first().copied().unwrap_or(0)
    }

    /// Resolve the bones under `part` onto same-named nodes under
    /// `skeleton` (ILLGAMES `AssignedAnotherWeights`); returns the count.
    pub fn alias_bones(&mut self, part: usize, skeleton: usize) -> usize {
        let mut names: HashMap<&str, usize> = HashMap::new();
        let mut stack = vec![skeleton];
        while let Some(n) = stack.pop() {
            names.entry(self.nodes[n].name.as_str()).or_insert(n);
            stack.extend(&self.nodes[n].children);
        }
        let mut alias = Vec::new();
        let mut stack = vec![part];
        while let Some(n) = stack.pop() {
            if let Some(&t) = names.get(self.nodes[n].name.as_str()) {
                if t != n {
                    alias.push((n, t));
                }
            }
            stack.extend(&self.nodes[n].children);
        }
        let count = alias.len();
        self.bone_alias.extend(alias);
        count
    }

    /// Node by slash-separated path from a root (`Canvas/Panel`).
    pub fn find(&self, path: &str) -> Option<usize> {
        let mut parts = path.split('/');
        let first = parts.next()?;
        let mut cur = *self.roots.iter().find(|&&r| self.nodes[r].name == first)?;
        for p in parts {
            cur = *self.nodes[cur].children.iter().find(|&&c| self.nodes[c].name == p)?;
        }
        Some(cur)
    }

    /// The node's component of a class (`class_id`) or script name.
    pub fn component(&self, node: usize, class_id: i32, script: &str) -> Option<&Component> {
        self.nodes[node].components.iter().find(|c| c.class_id == class_id && (script.is_empty() || c.script == script))
    }

    /// The path of a node from its root.
    pub fn path(&self, mut node: usize) -> String {
        let mut parts = vec![self.nodes[node].name.clone()];
        while let Some(p) = self.nodes[node].parent {
            parts.push(self.nodes[p].name.clone());
            node = p;
        }
        parts.reverse();
        parts.join("/")
    }
}

/// `m_ClassName` (with namespace) of a decoded MonoScript.
pub fn script_class(f: &Fields) -> String {
    let (ns, class) = (f.string("m_Namespace").unwrap_or(""), f.string("m_ClassName").unwrap_or(""));
    if ns.is_empty() { class.to_owned() } else { format!("{ns}.{class}") }
}

/// Resolve `m_Script` through the file itself or its externals (opened by
/// `open_external`).
pub fn resolve_script(file: &SerializedFile, p: PPtr, registry: &TreeRegistry, open_external: &dyn Fn(&str) -> Option<std::sync::Arc<SerializedFile>>) -> Result<String> {
    if p.is_null() {
        return Ok(String::new());
    }
    let f = if p.file_id == 0 {
        let info = file.object(p.path_id).context("local MonoScript")?;
        file.decode_using(info, registry)?
    } else {
        let name = file.external(p.file_id)?;
        let ext = open_external(name).with_context(|| format!("external {name}"))?;
        let info = *ext.object(p.path_id).context("external MonoScript")?;
        ext.decode_using(&info, registry)?
    };
    Ok(script_class(&f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtree_keeps_the_hierarchy() {
        let mut s = Scene6::default();
        for (name, parent) in [("A", None), ("B", Some(0)), ("C", Some(0)), ("D", Some(1))] {
            let i = s.nodes.len();
            s.nodes.push(Node { name: name.into(), active: true, game_object: 10 + i as i64, transform: 20 + i as i64, xform: Xform::default(), parent, children: Vec::new(), components: Vec::new(), origin: 0 });
            if let Some(p) = parent {
                s.nodes[p].children.push(i);
            }
        }
        s.roots = vec![0];
        let t = s.subtree(1);
        assert_eq!(t.nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["B", "D"]);
        assert_eq!((t.roots.clone(), t.nodes[0].parent, t.nodes[1].parent, t.nodes[0].children.clone()), (vec![0], None, Some(0), vec![1]));
        assert_eq!(t.path(1), "B/D");
        assert_eq!(t.by_game_object.get(&13), Some(&1));
        // Graft B/D under C of the original and alias D onto itself's name
        // twin in the skeleton.
        let mut g = s.clone();
        let root = g.graft(&t, "part", 2);
        assert_eq!((root, g.nodes[root].parent, g.path(root + 1), g.file_of(root + 1, "main")), (4, Some(2), "A/C/B/D".to_owned(), "part"));
        assert_eq!(g.alias_bones(root, 1), 2);
        assert_eq!((g.bone_alias[&root], g.bone_alias[&(root + 1)]), (1, 3));
    }
}
