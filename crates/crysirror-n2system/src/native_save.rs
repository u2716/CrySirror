// SPDX-License-Identifier: MPL-2.0
//! Totono trial1.0 variable NPF:469010(global),469450(local),469890(reader).
//! Header is NPSG/NPSL, version -235855164, total length, record count.
//! Each1024-byte record stores kind, CP932 name terminated by FOUR zero
//! bytes, unaligned LE double, text+four zeros, associative label+four zeros.
//! Native buffers can overrun a record for long strings; bounded ports reject
//! that layout rather than reproducing memory corruption. This is not the
//! script/frames/backlog record format and never certifies a full slot restore.
use crate::{array::Arrays, value::Value};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
const VERSION: i32 = -235855164;
const RECORD: usize = 1024;
const COUNT: usize = 32766;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Variable {
    pub kind: u8,
    pub name: String,
    pub number: f64,
    pub text: String,
    pub label: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Variables {
    pub global: bool,
    pub records: Vec<Variable>,
}
fn encoded(text: &str, limit: usize) -> Result<Vec<u8>> {
    ensure!(!text.contains('\0'), "N2 variable NPF embedded terminator");
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(
        !errors && bytes.len() < limit,
        "N2 variable NPF CP932 capacity"
    );
    Ok(bytes.into_owned())
}
fn string(record: &[u8], offset: &mut usize, limit: usize) -> Result<String> {
    let tail = record
        .get(*offset..)
        .context("N2 variable NPF field offset")?;
    let len = tail
        .iter()
        .position(|b| *b == 0)
        .context("N2 variable NPF missing terminator")?;
    ensure!(
        len < limit && tail.get(len..len + 4) == Some(&[0, 0, 0, 0]),
        "N2 variable NPF string extent"
    );
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&tail[..len]);
    ensure!(!errors, "N2 variable NPF malformed CP932");
    *offset += len + 4;
    Ok(text.into_owned())
}
impl Variables {
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() >= 16 && bytes.len() <= 16 + COUNT * RECORD,
            "N2 variable NPF byte budget"
        );
        let global = match &bytes[..4] {
            b"NPSG" => true,
            b"NPSL" => false,
            _ => anyhow::bail!("N2 variable NPF signature"),
        };
        ensure!(
            i32::from_le_bytes(bytes[4..8].try_into()?) == VERSION,
            "N2 variable NPF version"
        );
        let count = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
        ensure!(
            count <= COUNT
                && bytes.len() == 16 + count * RECORD
                && u32::from_le_bytes(bytes[8..12].try_into()?) as usize == bytes.len(),
            "N2 variable NPF header extent"
        );
        let mut records = Vec::with_capacity(count);
        let mut names = BTreeSet::new();
        for record in bytes[16..].chunks_exact(RECORD) {
            let kind = record[0];
            ensure!(kind <= 2, "N2 variable NPF unknown scalar kind {kind}");
            let mut offset = 1;
            let name = string(record, &mut offset, 128)?;
            ensure!(
                name.starts_with(if global { '#' } else { '$' })
                    && name.len() > 1
                    && names.insert(name.clone()),
                "N2 variable NPF name/scope/duplicate"
            );
            let number = f64::from_le_bytes(
                record
                    .get(offset..offset + 8)
                    .context("N2 variable NPF number extent")?
                    .try_into()?,
            );
            ensure!(number.is_finite(), "N2 variable NPF nonfinite scalar");
            offset += 8;
            let text = string(record, &mut offset, 1024)?;
            let label = string(record, &mut offset, 128)?;
            ensure!(
                record[offset..].iter().all(|b| *b == 0),
                "N2 variable NPF unknown record tail"
            );
            records.push(Variable {
                kind,
                name,
                number,
                text,
                label,
            });
        }
        Ok(Self { global, records })
    }
    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        ensure!(self.records.len() <= COUNT, "N2 variable NPF record budget");
        let mut out = vec![0; 16 + self.records.len() * RECORD];
        out[..4].copy_from_slice(if self.global { b"NPSG" } else { b"NPSL" });
        out[4..8].copy_from_slice(&VERSION.to_le_bytes());
        let length = out.len() as u32;
        out[8..12].copy_from_slice(&length.to_le_bytes());
        out[12..16].copy_from_slice(&(self.records.len() as u32).to_le_bytes());
        let mut names = BTreeSet::new();
        for (record, v) in out[16..].chunks_exact_mut(RECORD).zip(&self.records) {
            ensure!(
                v.kind <= 2
                    && v.number.is_finite()
                    && v.name.starts_with(if self.global { '#' } else { '$' })
                    && v.name.len() > 1
                    && names.insert(&v.name),
                "N2 variable NPF record identity"
            );
            record[0] = v.kind;
            let mut offset = 1;
            for (at, text, limit) in [(0, &v.name, 128), (1, &v.text, 1024), (2, &v.label, 128)] {
                let value = encoded(text, limit)?;
                let end = offset + value.len();
                ensure!(end + 4 <= RECORD, "N2 variable NPF record overflow");
                record[offset..end].copy_from_slice(&value);
                offset = end + 4;
                if at == 0 {
                    ensure!(offset + 8 <= RECORD, "N2 variable NPF number overflow");
                    record[offset..offset + 8].copy_from_slice(&v.number.to_le_bytes());
                    offset += 8;
                }
            }
        }
        Ok(out)
    }
    pub(crate) fn values(&self) -> Result<(BTreeMap<String, Value>, Arrays)> {
        let mut values = BTreeMap::new();
        let mut arrays = Arrays::default();
        let mut paths = BTreeMap::new();
        let mut records = self.records.iter().collect::<Vec<_>>();
        records.sort_by_key(|v| v.name.bytes().filter(|b| *b == b'/').count());
        for record in &records {
            let path = &record.name[1..];
            let parts = path.split('/').collect::<Vec<_>>();
            ensure!(
                parts.len() <= 128 && parts.iter().all(|p| !p.is_empty()),
                "N2 variable NPF indexed path"
            );
            let scalar = match record.kind {
                1 => Value::String(record.text.clone()),
                0 | 2 => {
                    let n = record.number as f32;
                    ensure!(n.is_finite(), "N2 variable NPF VM numeric range");
                    Value::Number(n)
                }
                _ => anyhow::bail!("N2 variable NPF scalar kind"),
            };
            let root = parts[0];
            if parts.len() == 1 {
                values.insert(root.to_owned(), scalar);
                continue;
            }
            let root_id = if let Some(Value::Array(id, _)) = values.get(root) {
                *id
            } else {
                let id = arrays.allocate(values.get(root).unwrap_or(&Value::Number(0.)))?;
                values.insert(root.to_owned(), arrays.value(id)?);
                id
            };
            paths.insert(root.to_owned(), root_id);
            let mut parent = root_id;
            let mut full = root.to_owned();
            for part in &parts[1..] {
                ensure!(
                    part.bytes().all(|b| b.is_ascii_digit()),
                    "N2 variable NPF noncanonical indexed member"
                );
                parent = arrays.child(parent, &Value::String((*part).into()))?;
                full.push('/');
                full.push_str(part);
                paths.insert(full.clone(), parent);
            }
            arrays.nodes.get_mut(&parent).unwrap().scalar = scalar;
        }
        for record in records {
            if record.label.is_empty() {
                continue;
            }
            let (parent, child) = record.name[1..]
                .rsplit_once('/')
                .context("N2 variable NPF label without indexed parent")?;
            let parent_id = *paths.get(parent).context("N2 variable NPF label parent")?;
            let child_id = *paths
                .get(&format!("{parent}/{child}"))
                .context("N2 variable NPF label child")?;
            ensure!(
                arrays
                    .nodes
                    .get_mut(&parent_id)
                    .unwrap()
                    .names
                    .insert(record.label.clone(), child_id)
                    .is_none(),
                "N2 variable NPF duplicate associative label"
            );
        }
        // Cached root scalars follow the now-populated arena.
        for value in values.values_mut() {
            if let Value::Array(id, _) = value {
                *value = arrays.value(*id)?;
            }
        }
        arrays.validate()?;
        Ok((values, arrays))
    }
    pub(crate) fn from_values(
        global: bool,
        values: &BTreeMap<String, Value>,
        arrays: &Arrays,
        previous: Option<&Self>,
    ) -> Result<Self> {
        arrays.validate()?;
        let old = previous
            .into_iter()
            .flat_map(|v| &v.records)
            .map(|v| (v.name.as_str(), v))
            .collect::<BTreeMap<_, _>>();
        let mut todo = values
            .iter()
            .map(|(name, value)| {
                (
                    format!("{}{name}", if global { '#' } else { '$' }),
                    value.clone(),
                    String::new(),
                    0,
                )
            })
            .collect::<Vec<_>>();
        let mut records = Vec::new();
        while let Some((name, value, label, depth)) = todo.pop() {
            ensure!(
                depth < 128 && records.len() + todo.len() < COUNT,
                "N2 variable NPF tree depth/count"
            );
            let mut record = old
                .get(name.as_str())
                .map(|v| (*v).clone())
                .unwrap_or(Variable {
                    kind: 0,
                    name: name.clone(),
                    number: 0.,
                    text: String::new(),
                    label: String::new(),
                });
            record.label = label;
            let scalar=if let Value::Array(id,_)=&value {
                &arrays.nodes.get(id).context("N2 variable NPF missing scalar node")?.scalar
            }else{value.scalar()};
            match scalar {
                Value::String(s) => {
                    record.kind = 1;
                    record.text = s.clone();
                }
                Value::Number(n) => {
                    record.kind = 2;
                    record.number = f64::from(*n);
                    record.text.clear();
                }
                Value::Boolean(b) => {
                    record.kind = 2;
                    record.number = f64::from(u8::from(*b));
                    record.text.clear();
                }
                Value::Null => {
                    record.kind = 0;
                    record.number = 0.;
                    record.text.clear();
                }
                _ => anyhow::bail!("N2 variable NPF unsupported native scalar"),
            }
            if let Value::Array(id, _) = value {
                let node = arrays
                    .nodes
                    .get(&id)
                    .context("N2 variable NPF missing indexed node")?;
                for (label, child) in &node.names {
                    ensure!(
                        node.items.contains(child),
                        "N2 variable NPF associative-only member {label}"
                    );
                }
                for (index, child) in node.items.iter().enumerate() {
                    let labels = node
                        .names
                        .iter()
                        .filter(|(_, id)| *id == child)
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>();
                    ensure!(
                        labels.len() <= 1,
                        "N2 variable NPF multiple labels for one indexed value"
                    );
                    todo.push((
                        format!("{name}/{index}"),
                        arrays.value(*child)?,
                        labels.first().map_or(String::new(), |s| (*s).clone()),
                        depth + 1,
                    ));
                }
            }
            records.push(record);
        }
        // Original outer map orders CP932 byte-sum buckets, then full names.
        let mut ordered = records
            .into_iter()
            .map(|v| {
                let bytes = encoded(&v.name, 128)?;
                Ok((bytes.iter().map(|b| u32::from(*b)).sum::<u32>(), bytes, v))
            })
            .collect::<Result<Vec<_>>>()?;
        ordered.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        Ok(Self {
            global,
            records: ordered.into_iter().map(|(_, _, v)| v).collect(),
        })
    }
}
