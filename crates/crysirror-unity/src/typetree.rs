// SPDX-License-Identifier: MPL-2.0
//! Unity type trees (serialized format ≥ 12 blob layout) and a generic
//! decoder of objects under them, producing [`managed::Fields`].
//!
//! Blob: i32 node count, i32 string buffer size, nodes `{u16 version, u8
//! level, u8 type flags (1 = array), u32 type string offset, u32 name string
//! offset, i32 byte size, i32 index, u32 meta flags[, u64 ref type hash
//! (format ≥ 19)]}`, the local string buffer. Offsets with bit 31 set name
//! Unity's common string table. Meta flag 0x4000 aligns the stream to 4
//! bytes after the field.
use crate::managed::{Fields, Value};
use crate::{PPtr, Reader};
use anyhow::{bail, ensure, Context, Result};

/// Unity's `CommonString` buffer (offsets with bit 31 set).
const COMMON: &str = "AABB\0AnimationClip\0AnimationCurve\0AnimationState\0Array\0Base\0BitField\0bitset\0bool\0char\0ColorRGBA\0Component\0data\0deque\0double\0dynamic_array\0FastPropertyName\0first\0float\0Font\0GameObject\0Generic Mono\0GradientNEW\0GUID\0GUIStyle\0int\0list\0long long\0map\0Matrix4x4f\0MdFour\0MonoBehaviour\0MonoScript\0m_ByteSize\0m_Curve\0m_EditorClassIdentifier\0m_EditorHideFlags\0m_Enabled\0m_ExtensionPtr\0m_GameObject\0m_Index\0m_IsArray\0m_IsStatic\0m_MetaFlag\0m_Name\0m_ObjectHideFlags\0m_PrefabInternal\0m_PrefabParentObject\0m_Script\0m_StaticEditorFlags\0m_Type\0m_Version\0Object\0pair\0PPtr<Component>\0PPtr<GameObject>\0PPtr<Material>\0PPtr<MonoBehaviour>\0PPtr<MonoScript>\0PPtr<Object>\0PPtr<Prefab>\0PPtr<Sprite>\0PPtr<TextAsset>\0PPtr<Texture>\0PPtr<Texture2D>\0PPtr<Transform>\0Prefab\0Quaternionf\0Rectf\0RectInt\0RectOffset\0second\0set\0short\0size\0SInt16\0SInt32\0SInt64\0SInt8\0staticvector\0string\0TextAsset\0TextMesh\0Texture\0Texture2D\0Transform\0TypelessData\0UInt16\0UInt32\0UInt64\0UInt8\0unsigned int\0unsigned long long\0unsigned short\0vector\0Vector2f\0Vector3f\0Vector4f\0m_ScriptingClassIdentifier\0Gradient\0Type*\0int2_storage\0int3_storage\0BoundsInt\0m_CorrespondingSourceObject\0m_PrefabInstance\0m_PrefabAsset\0FileSize\0Hash128\0RenderingLayerMask\0";

/// Bound of nodes in one type tree.
const MAX_NODES: usize = 1 << 20;
/// Bound of decoded array elements (per array).
const MAX_ELEMENTS: usize = 1 << 28;

#[derive(Clone, Debug, PartialEq)]
pub struct TypeNode {
    pub type_name: String,
    pub name: String,
    pub byte_size: i32,
    pub level: u8,
    pub is_array: bool,
    pub meta_flag: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TypeTree {
    pub nodes: Vec<TypeNode>,
}

fn string_at(local: &[u8], offset: u32) -> Result<String> {
    let (buf, at) = if offset & 0x8000_0000 != 0 { (COMMON.as_bytes(), (offset & 0x7fff_ffff) as usize) } else { (local, offset as usize) };
    ensure!(at < buf.len(), "type tree string offset {offset:#x} outside the table");
    // Common offsets must start a string.
    ensure!(offset & 0x8000_0000 == 0 || at == 0 || buf[at - 1] == 0, "type tree common string offset {at} is not a string start");
    let end = buf[at..].iter().position(|c| *c == 0).context("unterminated type tree string")?;
    Ok(String::from_utf8_lossy(&buf[at..at + end]).into_owned())
}

impl TypeTree {
    /// The blob layout at the reader (format ≥ 12).
    pub fn read_blob(r: &mut Reader, format: u32) -> Result<Self> {
        let count = r.count(24)?;
        ensure!(count <= MAX_NODES, "type tree node limit");
        let strings = r.count(1)?;
        let node_size = if format >= 19 { 32 } else { 24 };
        let raw = r.take(count * node_size)?;
        let local = r.take(strings)?;
        let mut nodes = Vec::with_capacity(count);
        for c in raw.chunks_exact(node_size) {
            let u32_at = |i: usize| u32::from_le_bytes(c[i..i + 4].try_into().unwrap());
            nodes.push(TypeNode {
                level: c[2],
                is_array: c[3] & 1 != 0,
                type_name: string_at(local, u32_at(4))?,
                name: string_at(local, u32_at(8))?,
                byte_size: u32_at(12) as i32,
                meta_flag: u32_at(20),
            });
        }
        ensure!(nodes.first().is_some_and(|n| n.level == 0), "type tree without a root");
        Ok(Self { nodes })
    }

    /// Index after the subtree of `i`.
    fn end(&self, i: usize) -> usize {
        let level = self.nodes[i].level;
        let mut j = i + 1;
        while j < self.nodes.len() && self.nodes[j].level > level {
            j += 1;
        }
        j
    }

    fn children(&self, i: usize) -> Vec<usize> {
        let (level, end) = (self.nodes[i].level, self.end(i));
        (i + 1..end).filter(|&j| self.nodes[j].level == level + 1).collect()
    }

    /// Decode a whole object.
    pub fn decode(&self, bytes: &[u8]) -> Result<Fields> {
        self.decode_with(bytes, &[])
    }

    /// Decode a whole object; `refs` are the file's reference types
    /// (`[SerializeReference]` data of a `ManagedReferencesRegistry`).
    pub fn decode_with(&self, bytes: &[u8], refs: &[RefType]) -> Result<Fields> {
        let mut r = Reader::new(bytes);
        let v = self.value(&mut r, 0, refs)?;
        ensure!(r.remaining() == 0, "type tree {} left {} of {} bytes", self.nodes[0].type_name, r.remaining(), bytes.len());
        match v {
            Value::Struct(f) => Ok(f),
            other => Ok(Fields(vec![(self.nodes[0].name.clone(), other)])),
        }
    }

    fn value(&self, r: &mut Reader, i: usize, refs: &[RefType]) -> Result<Value> {
        let n = &self.nodes[i];
        let v = if n.is_array {
            self.array(r, i, refs)?
        } else {
            match n.type_name.as_str() {
                "bool" => Value::Bool(r.u8()? != 0),
                "SInt8" => Value::Int(i64::from(r.u8()? as i8)),
                "UInt8" | "char" => Value::Int(i64::from(r.u8()?)),
                "SInt16" | "short" => Value::Int(i64::from(r.i16()?)),
                "UInt16" | "unsigned short" => Value::Int(i64::from(r.u16()?)),
                "SInt32" | "int" | "Type*" => Value::Int(i64::from(r.i32()?)),
                "UInt32" | "unsigned int" => Value::Int(i64::from(r.u32()?)),
                "SInt64" | "long long" | "FileSize" => Value::Int(r.i64()?),
                "UInt64" | "unsigned long long" => Value::Int(r.i64()?),
                "float" => Value::Float(f64::from(r.f32()?)),
                "double" => Value::Float(f64::from_le_bytes(r.take(8)?.try_into()?)),
                "string" => {
                    let len = r.count(1)?;
                    let s = String::from_utf8_lossy(r.take(len)?).into_owned();
                    r.align()?;
                    Value::String(s)
                }
                "TypelessData" => {
                    let len = r.count(1)?;
                    Value::Bytes(r.take(len)?.to_vec())
                }
                "ManagedReferencesRegistry" => registry(r, refs)?,
                t if t.starts_with("PPtr<") => {
                    let file_id = r.i32()?;
                    let path_id = r.i64()?;
                    Value::PPtr(PPtr { file_id, path_id })
                }
                _ => {
                    let kids = self.children(i);
                    // vector/map/set/staticvector wrap one array child.
                    if kids.len() == 1 && self.nodes[kids[0]].is_array {
                        self.value(r, kids[0], refs)?
                    } else {
                        let mut f = Vec::with_capacity(kids.len());
                        for k in kids {
                            f.push((self.nodes[k].name.clone(), self.value(r, k, refs)?));
                        }
                        Value::Struct(Fields(f))
                    }
                }
            }
        };
        if n.meta_flag & 0x4000 != 0 {
            r.align()?;
        }
        Ok(v)
    }

    fn array(&self, r: &mut Reader, i: usize, refs: &[RefType]) -> Result<Value> {
        let kids = self.children(i);
        ensure!(kids.len() == 2, "type tree array {} without size and data", self.nodes[i].name);
        let n = r.i32()?;
        ensure!(n >= 0 && (n as usize) <= MAX_ELEMENTS, "type tree array size {n}");
        let n = n as usize;
        let elem = &self.nodes[kids[1]];
        let simple = self.end(kids[1]) == kids[1] + 1 && elem.meta_flag & 0x4000 == 0;
        if simple {
            match elem.type_name.as_str() {
                "UInt8" | "SInt8" | "char" => return Ok(Value::Bytes(r.take(n)?.to_vec())),
                "float" => {
                    ensure!(n * 4 <= r.remaining(), "type tree float array exceeds the object");
                    let mut out = Vec::with_capacity(n);
                    for _ in 0..n {
                        out.push(r.f32()?);
                    }
                    return Ok(Value::Floats(out));
                }
                _ => {}
            }
        }
        if elem.byte_size > 0 {
            ensure!(n.saturating_mul(elem.byte_size as usize) <= r.remaining(), "type tree array {} exceeds the object", self.nodes[i].name);
        }
        let mut out = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n {
            out.push(self.value(r, kids[1], refs)?);
        }
        Ok(Value::Array(out))
    }
}

/// A format ≥ 20 reference type: the managed class and its type tree.
#[derive(Clone, Debug, PartialEq)]
pub struct RefType {
    pub class: String,
    pub namespace: String,
    pub assembly: String,
    pub tree: std::sync::Arc<TypeTree>,
}

fn aligned_string(r: &mut Reader) -> Result<String> {
    let len = r.count(1)?;
    let s = String::from_utf8_lossy(r.take(len)?).into_owned();
    r.align()?;
    Ok(s)
}

/// `ManagedReferencesRegistry` (version 2): i32 version, i32 count, then
/// per object `{i64 rid, aligned strings class/namespace/assembly, the data
/// under that reference type's tree}`. Version 1 lists objects until the
/// `Terminus` sentinel type.
fn registry(r: &mut Reader, refs: &[RefType]) -> Result<Value> {
    let version = r.i32()?;
    let mut out = Vec::new();
    let mut entry = |r: &mut Reader, rid: Option<i64>| -> Result<bool> {
        let class = aligned_string(r)?;
        let namespace = aligned_string(r)?;
        let assembly = aligned_string(r)?;
        if class == "Terminus" && namespace == "UnityEngine.DMAT" && assembly == "FAKE_ASM" {
            return Ok(false);
        }
        let data = if class.is_empty() && namespace.is_empty() && assembly.is_empty() {
            Value::Struct(Fields::default())
        } else {
            let t = refs
                .iter()
                .find(|t| t.class == class && t.namespace == namespace && t.assembly == assembly)
                .with_context(|| format!("SerializeReference type {namespace}.{class} ({assembly}) has no type tree"))?;
            let mut fields = Vec::new();
            for k in t.tree.children(0) {
                fields.push((t.tree.nodes[k].name.clone(), t.tree.value(r, k, refs)?));
            }
            Value::Struct(Fields(fields))
        };
        out.push(Value::Struct(Fields(vec![
            ("rid".into(), Value::Int(rid.unwrap_or(out.len() as i64))),
            ("class".into(), Value::String(format!("{namespace}.{class}"))),
            ("data".into(), data),
        ])));
        Ok(true)
    };
    if version == 1 {
        while entry(r, None)? {}
    } else {
        let n = r.count(8)?;
        for _ in 0..n {
            let rid = r.i64()?;
            entry(r, Some(rid))?;
        }
    }
    Ok(Value::Struct(Fields(vec![("version".into(), Value::Int(i64::from(version))), ("RefIds".into(), Value::Array(out))])))
}

/// Unity value-type names whose components read as floats.
pub fn is_float_struct(type_name: &str) -> bool {
    matches!(type_name, "Vector2f" | "Vector3f" | "Vector4f" | "Quaternionf" | "ColorRGBA" | "Rectf" | "Matrix4x4f")
}

#[allow(dead_code)]
fn unsupported(t: &str) -> Result<Value> {
    bail!("type tree type {t} is not decodable")
}
