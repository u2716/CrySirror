// SPDX-License-Identifier: MPL-2.0
//! Schema-driven decoding of MonoBehaviour script fields (Unity 4 managed
//! serialization). Player data has no type trees, so the consumer supplies
//! the schema of its scripts (see `tools/unity_managed_schema.py`): field
//! order, primitive widths, alignment of sub-4-byte values, counted arrays,
//! inline `[Serializable]` structs and object references.
use crate::{PPtr, Reader};
use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Kind {
    /// A primitive, `pptr`, or a Unity value type name.
    Name(String),
    Array { array: Box<Kind> },
    Struct { r#struct: String },
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Schema {
    pub schema: u32,
    /// Script class name → fields (base classes first).
    pub scripts: BTreeMap<String, Vec<(String, Kind)>>,
    pub structs: BTreeMap<String, Vec<(String, Kind)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    PPtr(PPtr),
    /// Vector/quaternion/color/rect/bounds/matrix components in field order.
    Floats(Vec<f32>),
    Array(Vec<Value>),
    Struct(Fields),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields(pub Vec<(String, Value)>);

impl Fields {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
    pub fn int(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            Value::Int(v) => Some(*v),
            Value::Bool(b) => Some(i64::from(*b)),
            _ => None,
        }
    }
    pub fn float(&self, name: &str) -> Option<f64> {
        match self.get(name)? {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
    pub fn bool(&self, name: &str) -> Option<bool> {
        match self.get(name)? {
            Value::Bool(b) => Some(*b),
            Value::Int(v) => Some(*v != 0),
            _ => None,
        }
    }
    pub fn string(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn pptr(&self, name: &str) -> Option<PPtr> {
        match self.get(name)? {
            Value::PPtr(p) => Some(*p),
            _ => None,
        }
    }
    pub fn floats(&self, name: &str) -> Option<&[f32]> {
        match self.get(name)? {
            Value::Floats(v) => Some(v),
            _ => None,
        }
    }
    pub fn array(&self, name: &str) -> Option<&[Value]> {
        match self.get(name)? {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }
    pub fn fields(&self, name: &str) -> Option<&Fields> {
        match self.get(name)? {
            Value::Struct(f) => Some(f),
            _ => None,
        }
    }
}

/// Depth limit of nested serializable structs (Unity stops at 7).
const MAX_DEPTH: usize = 8;

impl Schema {
    pub fn parse(json: &str) -> Result<Self> {
        let schema: Self = serde_json::from_str(json).context("Unity managed schema")?;
        ensure!(schema.schema == 1, "unsupported Unity managed schema {}", schema.schema);
        Ok(schema)
    }

    /// Decode the script fields of `class` from a MonoBehaviour body. The
    /// whole remainder must be consumed, so a schema mismatch is an error.
    pub fn decode(&self, class: &str, body: &[u8]) -> Result<Fields> {
        let fields = self.scripts.get(class).with_context(|| format!("no managed schema for script {class}"))?;
        let mut r = Reader::new(body);
        let out = self.fields(fields, &mut r, 0)?;
        ensure!(
            r.remaining() == 0,
            "script {class}: {} bytes after the schema's fields",
            r.remaining()
        );
        Ok(out)
    }

    fn fields(&self, fields: &[(String, Kind)], r: &mut Reader, depth: usize) -> Result<Fields> {
        let mut out = Vec::with_capacity(fields.len());
        for (name, kind) in fields {
            let value = self.value(kind, r, depth, false).with_context(|| format!("field {name}"))?;
            out.push((name.clone(), value));
        }
        Ok(Fields(out))
    }

    fn value(&self, kind: &Kind, r: &mut Reader, depth: usize, element: bool) -> Result<Value> {
        ensure!(depth <= MAX_DEPTH, "Unity managed nesting limit");
        let small = |r: &mut Reader, v: Value| -> Result<Value> {
            if !element {
                r.align()?;
            }
            Ok(v)
        };
        Ok(match kind {
            Kind::Name(name) => match name.as_str() {
                "bool" => {
                    let v = r.bool()?;
                    small(r, Value::Bool(v))?
                }
                "u8" => {
                    let v = r.u8()?;
                    small(r, Value::Int(i64::from(v)))?
                }
                "i8" => {
                    let v = r.u8()? as i8;
                    small(r, Value::Int(i64::from(v)))?
                }
                "i16" => {
                    let v = r.i16()?;
                    small(r, Value::Int(i64::from(v)))?
                }
                "u16" => {
                    let v = r.u16()?;
                    small(r, Value::Int(i64::from(v)))?
                }
                "i32" => Value::Int(i64::from(r.i32()?)),
                "u32" | "Color32" | "LayerMask" => Value::Int(i64::from(r.u32()?)),
                "i64" | "u64" => Value::Int(r.i64()?),
                "f32" => Value::Float(f64::from(r.f32()?)),
                "f64" => Value::Float(f64::from_le_bytes(r.take(8)?.try_into()?)),
                "string" => Value::String(r.string()?),
                "pptr" => Value::PPtr(r.pptr()?),
                "Vector2" => Value::Floats(r.f32s::<2>()?.to_vec()),
                "Vector3" => Value::Floats(r.f32s::<3>()?.to_vec()),
                "Vector4" | "Quaternion" | "Color" | "Rect" => Value::Floats(r.f32s::<4>()?.to_vec()),
                "Bounds" => Value::Floats(r.f32s::<6>()?.to_vec()),
                "Matrix4x4" => Value::Floats(r.f32s::<16>()?.to_vec()),
                "RectOffset" => Value::Floats((0..4).map(|_| r.i32().map(|v| v as f32)).collect::<Result<_>>()?),
                "AnimationCurve" => {
                    let n = r.count(16)?;
                    let mut keys = Vec::with_capacity(n * 4 + 2);
                    for _ in 0..n {
                        keys.extend(r.f32s::<4>()?);
                    }
                    let pre = r.i32()?;
                    let post = r.i32()?;
                    Value::Struct(Fields(vec![
                        ("m_Curve".into(), Value::Floats(keys)),
                        ("m_PreInfinity".into(), Value::Int(i64::from(pre))),
                        ("m_PostInfinity".into(), Value::Int(i64::from(post))),
                    ]))
                }
                other => bail!("unsupported Unity managed type {other}"),
            },
            Kind::Array { array } => {
                let n = r.count(1)?;
                let mut items = Vec::with_capacity(n.min(1 << 16));
                for _ in 0..n {
                    items.push(self.value(array, r, depth, true)?);
                }
                r.align()?;
                Value::Array(items)
            }
            Kind::Struct { r#struct } => {
                let fields = self.structs.get(r#struct).with_context(|| format!("no managed schema for struct {}", r#struct))?;
                Value::Struct(self.fields(fields, r, depth + 1)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_fields_align_and_arrays_count() {
        let schema = Schema::parse(
            r#"{"schema":1,"scripts":{"S":[["a","bool"],["b","i32"],["c",{"array":"u8"}],["d",{"struct":"P"}]]},
                "structs":{"P":[["x","string"],["y","pptr"]]}}"#,
        )
        .unwrap();
        let mut body = vec![1, 0, 0, 0];
        body.extend(7i32.to_le_bytes());
        body.extend(3i32.to_le_bytes());
        body.extend([9, 8, 7, 0]);
        body.extend(2i32.to_le_bytes());
        body.extend(b"hi\0\0");
        body.extend(1i32.to_le_bytes());
        body.extend(42i32.to_le_bytes());
        let f = schema.decode("S", &body).unwrap();
        assert_eq!(f.bool("a"), Some(true));
        assert_eq!(f.int("b"), Some(7));
        assert_eq!(f.array("c").unwrap().len(), 3);
        let d = f.fields("d").unwrap();
        assert_eq!(d.string("x"), Some("hi"));
        assert_eq!(d.pptr("y"), Some(PPtr { file_id: 1, path_id: 42 }));
        assert!(schema.decode("S", &body[..body.len() - 1]).is_err());
    }
}
