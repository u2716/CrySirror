// SPDX-License-Identifier: MPL-2.0
//! Type trees of MonoBehaviour scripts derived from managed metadata, for
//! player files built without trees whose scripts never occur in a bundle.
//! The assemblies are ECMA-335 images (for IL2CPP players, metadata-only
//! assemblies regenerated from `global-metadata.dat`); no code runs.
//!
//! Unity's serializer rules applied: instance fields that are public or
//! carry `[SerializeField]`/`[SerializeReference]`, are not literal,
//! read-only or `[NonSerialized]`; primitives (1- and 2-byte ones aligned),
//! strings, enums by their underlying type, `UnityEngine.Object` references
//! as PPtrs, `T[]`/`List<T>` as aligned arrays, `[Serializable]` classes and
//! structs inline (generic arguments substituted), `[SerializeReference]`
//! fields as reference ids with the object's `references` registry
//! appended, a depth limit of 10. Unity value types (vectors, curves,
//! gradients, events, …) reuse subtrees found in the [`TreeRegistry`].
use crate::typetree::{TypeNode, TypeTree};
use crate::TreeRegistry;
use anyhow::{bail, Context, Result};
use crysirror_dotnet::{table, Assembly, Token};
use std::collections::HashMap;

const MAX_DEPTH: usize = 10;
const ALIGN: u32 = 0x4000;

/// A resolved field type.
#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Prim(&'static str, bool),
    String,
    Named { asm: usize, row: u32, args: Vec<Ty> },
    /// A type the assemblies do not define (BCL or missing reference).
    External(String),
    Array(Box<Ty>),
    Var(u32),
    Unsupported,
}

pub struct Generator<'a> {
    asms: Vec<Assembly>,
    by_name: HashMap<String, (usize, u32)>,
    /// Attribute type names per (assembly, field row).
    field_attrs: Vec<HashMap<u32, Vec<String>>>,
    harvested: HashMap<String, Vec<TypeNode>>,
    _registry: &'a TreeRegistry,
}

fn prim(t: &str) -> Option<(&'static str, bool)> {
    Some(match t {
        "System.Boolean" => ("bool", true),
        "System.Char" => ("UInt16", true),
        "System.SByte" => ("SInt8", true),
        "System.Byte" => ("UInt8", true),
        "System.Int16" => ("SInt16", true),
        "System.UInt16" => ("UInt16", true),
        "System.Int32" => ("int", false),
        "System.UInt32" => ("unsigned int", false),
        "System.Int64" => ("SInt64", false),
        "System.UInt64" => ("UInt64", false),
        "System.Single" => ("float", false),
        "System.Double" => ("double", false),
        _ => return None,
    })
}

/// Unity tree type names of engine value types by C# name.
fn unity_value(t: &str) -> Option<&'static str> {
    Some(match t {
        "UnityEngine.Vector2" => "Vector2f",
        "UnityEngine.Vector3" => "Vector3f",
        "UnityEngine.Vector4" => "Vector4f",
        "UnityEngine.Quaternion" => "Quaternionf",
        "UnityEngine.Color" => "ColorRGBA",
        "UnityEngine.Rect" => "Rectf",
        "UnityEngine.Bounds" => "AABB",
        "UnityEngine.Matrix4x4" => "Matrix4x4f",
        "UnityEngine.LayerMask" => "BitField",
        "UnityEngine.Vector2Int" => "int2_storage",
        "UnityEngine.Vector3Int" => "int3_storage",
        "UnityEngine.BoundsInt" => "BoundsInt",
        "UnityEngine.RectInt" => "RectInt",
        "UnityEngine.Hash128" => "Hash128",
        "UnityEngine.AnimationCurve" => "AnimationCurve",
        "UnityEngine.Gradient" => "Gradient",
        "UnityEngine.RectOffset" => "RectOffset",
        "UnityEngine.GUIStyle" => "GUIStyle",
        "UnityEngine.RenderingLayerMask" => "RenderingLayerMask",
        _ => return None,
    })
}

impl<'a> Generator<'a> {
    pub fn new(asms: Vec<Assembly>, registry: &'a TreeRegistry) -> Result<Self> {
        let mut by_name = HashMap::new();
        let mut field_attrs = Vec::new();
        for (i, a) in asms.iter().enumerate() {
            for r in 1..=a.rows(table::TYPE_DEF) {
                by_name.entry(a.type_name(Token { table: table::TYPE_DEF, row: r })?).or_insert((i, r));
            }
            let mut attrs: HashMap<u32, Vec<String>> = HashMap::new();
            for r in 1..=a.rows(0x0C) {
                let parent = Token::from_u32(a.cell(0x0C, r, 0)?);
                if parent.table != table::FIELD {
                    continue;
                }
                let ctor = Token::from_u32(a.cell(0x0C, r, 1)?);
                let owner = match ctor.table {
                    table::MEMBER_REF => Token::from_u32(a.cell(table::MEMBER_REF, ctor.row, 0)?),
                    table::METHOD_DEF => Token { table: table::TYPE_DEF, row: a.method_owner(ctor.row)? },
                    _ => continue,
                };
                if let Ok(name) = a.type_name(owner) {
                    attrs.entry(parent.row).or_default().push(name);
                }
            }
            field_attrs.push(attrs);
        }
        let mut harvested = HashMap::new();
        for tree in registry.trees() {
            for (i, n) in tree.nodes.iter().enumerate() {
                if i == 0 || harvested.contains_key(&n.type_name) {
                    continue;
                }
                let end = (i + 1..tree.nodes.len()).find(|&j| tree.nodes[j].level <= n.level).unwrap_or(tree.nodes.len());
                // Containers (a single array child) are not the type itself.
                if n.is_array || (end == i + 2 + (end - i - 2) && tree.nodes.get(i + 1).is_some_and(|c| c.is_array)) {
                    continue;
                }
                harvested.insert(n.type_name.clone(), tree.nodes[i..end].to_vec());
            }
        }
        Ok(Self { asms, by_name, field_attrs, harvested, _registry: registry })
    }

    fn name_of(&self, asm: usize, row: u32) -> String {
        self.asms[asm].type_name(Token { table: table::TYPE_DEF, row }).unwrap_or_default()
    }

    fn base_of(&self, asm: usize, row: u32) -> Result<Option<Ty>> {
        let t = Token::from_u32(self.asms[asm].cell(table::TYPE_DEF, row, 3)?);
        if t.row == 0 {
            return Ok(None);
        }
        Ok(Some(self.token_ty(asm, t, &[])?))
    }

    fn token_ty(&self, asm: usize, t: Token, args: &[Ty]) -> Result<Ty> {
        let a = &self.asms[asm];
        match t.table {
            table::TYPE_DEF => Ok(Ty::Named { asm, row: t.row, args: Vec::new() }),
            table::TYPE_REF => {
                let name = a.type_name(t)?;
                if let Some(p) = prim(&name) {
                    return Ok(Ty::Prim(p.0, p.1));
                }
                if name == "System.String" {
                    return Ok(Ty::String);
                }
                Ok(match self.by_name.get(&name) {
                    Some(&(asm, row)) => Ty::Named { asm, row, args: Vec::new() },
                    None => Ty::External(name),
                })
            }
            table::TYPE_SPEC => {
                let sig = a.blob(a.cell(table::TYPE_SPEC, t.row, 0)?)?;
                let mut o = 0;
                self.sig_ty(asm, sig, &mut o, args)
            }
            _ => bail!("type token table {:#x}", t.table),
        }
    }

    fn sig_ty(&self, asm: usize, s: &[u8], o: &mut usize, args: &[Ty]) -> Result<Ty> {
        let e = *s.get(*o).context("signature")?;
        *o += 1;
        let coded = |o: &mut usize| -> Result<Token> {
            let v = crysirror_dotnet::compressed(s, o)?;
            let table = [table::TYPE_DEF, table::TYPE_REF, table::TYPE_SPEC][(v & 3) as usize % 3];
            Ok(Token { table, row: v >> 2 })
        };
        Ok(match e {
            0x02 => Ty::Prim("bool", true),
            0x03 => Ty::Prim("UInt16", true),
            0x04 => Ty::Prim("SInt8", true),
            0x05 => Ty::Prim("UInt8", true),
            0x06 => Ty::Prim("SInt16", true),
            0x07 => Ty::Prim("UInt16", true),
            0x08 => Ty::Prim("int", false),
            0x09 => Ty::Prim("unsigned int", false),
            0x0a => Ty::Prim("SInt64", false),
            0x0b => Ty::Prim("UInt64", false),
            0x0c => Ty::Prim("float", false),
            0x0d => Ty::Prim("double", false),
            0x0e => Ty::String,
            0x11 | 0x12 => {
                let t = coded(o)?;
                self.token_ty(asm, t, args)?
            }
            0x1d => Ty::Array(Box::new(self.sig_ty(asm, s, o, args)?)),
            0x13 => {
                let n = crysirror_dotnet::compressed(s, o)?;
                args.get(n as usize).cloned().unwrap_or(Ty::Var(n))
            }
            0x15 => {
                *o += 1; // CLASS / VALUETYPE
                let t = coded(o)?;
                let base = self.token_ty(asm, t, args)?;
                let n = crysirror_dotnet::compressed(s, o)?;
                let mut targs = Vec::new();
                for _ in 0..n {
                    targs.push(self.sig_ty(asm, s, o, args)?);
                }
                if self.full_name(&base) == "System.Collections.Generic.List`1" && targs.len() == 1 {
                    return Ok(Ty::Array(Box::new(targs.remove(0))));
                }
                match base {
                    Ty::Named { asm, row, .. } => Ty::Named { asm, row, args: targs },
                    Ty::External(name) => Ty::External(format!("{name}<…>")),
                    other => other,
                }
            }
            0x1f | 0x20 => {
                coded(o)?;
                self.sig_ty(asm, s, o, args)?
            }
            _ => Ty::Unsupported,
        })
    }

    fn full_name(&self, t: &Ty) -> String {
        match t {
            Ty::Named { asm, row, .. } => self.name_of(*asm, *row),
            Ty::External(n) => n.clone(),
            _ => String::new(),
        }
    }

    fn is_unity_object(&self, t: &Ty) -> Result<bool> {
        let mut cur = t.clone();
        for _ in 0..64 {
            let name = self.full_name(&cur);
            if name == "UnityEngine.Object" {
                return Ok(true);
            }
            match cur {
                Ty::Named { asm, row, .. } => match self.base_of(asm, row)? {
                    Some(b) => cur = b,
                    None => return Ok(false),
                },
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    /// Nodes of one field of type `t` at `level`.
    fn field(&self, name: &str, t: &Ty, level: u8, depth: usize, out: &mut Vec<TypeNode>, refs: &mut bool, serialize_ref: bool) -> Result<bool> {
        let node = |type_name: &str, meta: u32, is_array: bool| TypeNode { type_name: type_name.into(), name: name.into(), byte_size: -1, level, is_array, meta_flag: meta };
        if serialize_ref {
            *refs = true;
            out.push(node("managedReference", 0, false));
            out.push(TypeNode { type_name: "SInt64".into(), name: "rid".into(), byte_size: 8, level: level + 1, is_array: false, meta_flag: 0 });
            return Ok(true);
        }
        match t {
            Ty::Prim(p, align) => out.push(node(p, if *align { ALIGN } else { 0 }, false)),
            Ty::String => out.push(node("string", ALIGN, false)),
            Ty::Array(e) => {
                if matches!(**e, Ty::Array(_)) {
                    return Ok(false);
                }
                let mut elem = Vec::new();
                if !self.field("data", e, level + 2, depth + 1, &mut elem, refs, false)? {
                    return Ok(false);
                }
                out.push(node("vector", ALIGN, false));
                out.push(TypeNode { type_name: "Array".into(), name: "Array".into(), byte_size: -1, level: level + 1, is_array: true, meta_flag: ALIGN });
                out.push(TypeNode { type_name: "int".into(), name: "size".into(), byte_size: 4, level: level + 2, is_array: false, meta_flag: 0 });
                out.extend(elem);
            }
            Ty::Named { asm, row, args } => {
                let full = self.name_of(*asm, *row);
                if let Some(p) = prim(&full) {
                    out.push(node(p.0, if p.1 { ALIGN } else { 0 }, false));
                    return Ok(true);
                }
                if let Some(base) = self.base_of(*asm, *row)? {
                    if self.full_name(&base) == "System.Enum" {
                        let a = &self.asms[*asm];
                        let (fields, _) = a.members(*row)?;
                        for f in fields {
                            if a.string(a.cell(table::FIELD, f, 1)?)? == "value__" {
                                let sig = a.blob(a.cell(table::FIELD, f, 2)?)?;
                                let mut o = 1;
                                return self.field(name, &self.sig_ty(*asm, sig, &mut o, &[])?, level, depth, out, refs, false);
                            }
                        }
                        out.push(node("int", 0, false));
                        return Ok(true);
                    }
                }
                // Color32 is ColorRGBA { unsigned int rgba } (the float
                // ColorRGBA belongs to Color).
                if full == "UnityEngine.Color32" {
                    out.push(node("ColorRGBA", 0, false));
                    out.push(TypeNode { type_name: "unsigned int".into(), name: "rgba".into(), byte_size: 4, level: level + 1, is_array: false, meta_flag: 0 });
                    return Ok(true);
                }
                if let Some(h) = unity_value(&full).and_then(|u| self.harvested.get(u)) {
                    self.splice(name, h, level, out);
                    return Ok(true);
                }
                if self.is_unity_object(t)? {
                    out.push(node(&format!("PPtr<{}>", full.rsplit(['.', '/']).next().unwrap_or(&full)), 0, false));
                    return Ok(true);
                }
                return self.inline(name, *asm, *row, args, level, depth, out, refs);
            }
            Ty::External(n) => {
                let unity = unity_value(n).map(str::to_owned).unwrap_or_else(|| n.rsplit(['.', '/']).next().unwrap_or(n).to_owned());
                match self.harvested.get(&unity) {
                    Some(h) => self.splice(name, h, level, out),
                    None => return Ok(false),
                }
            }
            Ty::Var(_) | Ty::Unsupported => return Ok(false),
        }
        Ok(true)
    }

    fn splice(&self, name: &str, h: &[TypeNode], level: u8, out: &mut Vec<TypeNode>) {
        let base = h[0].level;
        for (i, n) in h.iter().enumerate() {
            let mut n = n.clone();
            n.level = n.level - base + level;
            if i == 0 {
                n.name = name.into();
            }
            out.push(n);
        }
    }

    /// A `[Serializable]` class/struct written inline.
    #[allow(clippy::too_many_arguments)]
    fn inline(&self, name: &str, asm: usize, row: u32, args: &[Ty], level: u8, depth: usize, out: &mut Vec<TypeNode>, refs: &mut bool) -> Result<bool> {
        if depth >= MAX_DEPTH {
            return Ok(false);
        }
        let a = &self.asms[asm];
        let flags = a.cell(table::TYPE_DEF, row, 0)?;
        // Interface 0x20, abstract 0x80, serializable 0x2000.
        if flags & 0x20 != 0 || flags & 0x80 != 0 || flags & 0x2000 == 0 {
            return Ok(false);
        }
        out.push(TypeNode { type_name: self.name_of(asm, row).rsplit(['.', '/']).next().unwrap_or("").to_owned(), name: name.into(), byte_size: -1, level, is_array: false, meta_flag: 0 });
        self.members(asm, row, args, level + 1, depth + 1, out, refs)?;
        Ok(true)
    }

    /// Serialized fields of a type and its bases (bases first), up to the
    /// Unity base classes.
    #[allow(clippy::too_many_arguments)]
    fn members(&self, asm: usize, row: u32, args: &[Ty], level: u8, depth: usize, out: &mut Vec<TypeNode>, refs: &mut bool) -> Result<()> {
        if let Some(base) = self.base_of(asm, row)? {
            if let Ty::Named { asm: ba, row: br, args: bargs } = &base {
                let bname = self.name_of(*ba, *br);
                if !matches!(bname.as_str(), "UnityEngine.MonoBehaviour" | "UnityEngine.ScriptableObject" | "UnityEngine.Behaviour" | "UnityEngine.Component" | "UnityEngine.Object" | "System.Object" | "System.ValueType") {
                    let bargs: Vec<Ty> = bargs.iter().map(|t| match t {
                        Ty::Var(n) => args.get(*n as usize).cloned().unwrap_or(Ty::Var(*n)),
                        o => o.clone(),
                    }).collect();
                    self.members(*ba, *br, &bargs, level, depth, out, refs)?;
                }
            }
        }
        let a = &self.asms[asm];
        let (fields, _) = a.members(row)?;
        for f in fields {
            let flags = a.cell(table::FIELD, f, 0)?;
            // Static 0x10, init-only 0x20, literal 0x40, not-serialized 0x80.
            if flags & (0x10 | 0x20 | 0x40 | 0x80) != 0 {
                continue;
            }
            let attrs = self.field_attrs[asm].get(&f).cloned().unwrap_or_default();
            let public = flags & 7 == 6;
            let serialize_ref = attrs.iter().any(|n| n == "UnityEngine.SerializeReference");
            if !(public || serialize_ref || attrs.iter().any(|n| n == "UnityEngine.SerializeField")) {
                continue;
            }
            let fname = a.string(a.cell(table::FIELD, f, 1)?)?.to_owned();
            let sig = a.blob(a.cell(table::FIELD, f, 2)?)?;
            let mut o = 1;
            let ty = self.sig_ty(asm, sig, &mut o, args)?;
            let mark = out.len();
            if !self.field(&fname, &ty, level, depth, out, refs, serialize_ref && !matches!(ty, Ty::Array(_)))? {
                out.truncate(mark);
            }
        }
        Ok(())
    }

    /// The MonoBehaviour tree of a script class (`Ns.Name`, nested
    /// `Outer/Inner`).
    pub fn mono_behaviour(&self, class: &str) -> Result<TypeTree> {
        let &(asm, row) = self.by_name.get(class).with_context(|| format!("script class {class} not in the assemblies"))?;
        let header = self.harvested.get("MonoBehaviour");
        let mut nodes = vec![TypeNode { type_name: "MonoBehaviour".into(), name: "Base".into(), byte_size: -1, level: 0, is_array: false, meta_flag: 0 }];
        let leaf = |t: &str, n: &str, l: u8, m: u32| TypeNode { type_name: t.into(), name: n.into(), byte_size: -1, level: l, is_array: false, meta_flag: m };
        let _ = header;
        nodes.push(leaf("PPtr<GameObject>", "m_GameObject", 1, 0));
        nodes.push(leaf("UInt8", "m_Enabled", 1, ALIGN));
        nodes.push(leaf("PPtr<MonoScript>", "m_Script", 1, 0));
        nodes.push(leaf("string", "m_Name", 1, ALIGN));
        let mut refs = false;
        self.members(asm, row, &[], 1, 0, &mut nodes, &mut refs)?;
        if refs {
            nodes.push(leaf("ManagedReferencesRegistry", "references", 1, 0));
        }
        Ok(TypeTree { nodes })
    }
}
