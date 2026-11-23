// SPDX-License-Identifier: MPL-2.0
// Indexed scalar-node semantics: AetherN2System/NsArray.cs at 77a93db4 (MIT).
// Nodes are bounded logical identities, never shared host pointers.
use crate::{
    value::Value,
    vm::{Argument, Machine, Thread},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub scalar: Value,
    pub items: Vec<u32>,
    pub names: BTreeMap<String, u32>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Arrays {
    pub nodes: BTreeMap<u32, Node>,
    next: u32,
}
impl Arrays {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.nodes.len() <= 262144 && self.nodes.keys().all(|id| *id > 0 && *id <= self.next),
            "N2System indexed arena extent"
        );
        for node in self.nodes.values() {
            ensure!(
                !matches!(node.scalar, Value::Array(..))
                    && node.items.len() <= 262144
                    && node.names.len() <= 262144,
                "N2System indexed node extent"
            );
            ensure!(
                node.items
                    .iter()
                    .chain(node.names.values())
                    .all(|id| self.nodes.contains_key(id))
                    && node.names.keys().all(|key| key.len() <= 4096),
                "N2System indexed child identity"
            );
        }
        Ok(())
    }
    pub fn reachable<'a>(&self, roots: impl Iterator<Item = &'a Value>) -> Result<Self> {
        let mut todo = roots
            .filter_map(|v| {
                if let Value::Array(id, _) = v {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut nodes = BTreeMap::new();
        while let Some(id) = todo.pop() {
            if nodes.contains_key(&id) {
                continue;
            }
            let node = self
                .nodes
                .get(&id)
                .context("N2System persistent indexed root")?;
            todo.extend(node.items.iter().chain(node.names.values()).copied());
            nodes.insert(id, node.clone());
        }
        Ok(Self {
            nodes,
            next: self.next,
        })
    }
    pub fn allocate(&mut self, value: &Value) -> Result<u32> {
        ensure!(self.nodes.len() < 262144, "N2System indexed value capacity");
        self.next = self
            .next
            .checked_add(1)
            .context("N2System indexed identity exhausted")?;
        self.nodes.insert(
            self.next,
            Node {
                scalar: value.scalar().clone(),
                items: Vec::new(),
                names: BTreeMap::new(),
            },
        );
        Ok(self.next)
    }
    pub fn value(&self, id: u32) -> Result<Value> {
        Ok(Value::Array(
            id,
            Box::new(
                self.nodes
                    .get(&id)
                    .context("N2System stale indexed value")?
                    .scalar
                    .clone(),
            ),
        ))
    }
    pub fn child(&mut self, id: u32, key: &Value) -> Result<u32> {
        let number = match key.scalar() {
            Value::Number(n) => Some(*n),
            Value::String(s) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
                s.parse::<u32>().ok().map(|v| v as f32)
            }
            _ => None,
        };
        if let Some(index) =
            number.filter(|n| n.is_finite() && *n >= 0. && *n == n.trunc() && *n < 1_000_000.)
        {
            let index = index as usize;
            let len = self
                .nodes
                .get(&id)
                .context("N2System missing indexed parent")?
                .items
                .len();
            ensure!(
                index.saturating_add(1).saturating_sub(len)
                    <= 262144usize.saturating_sub(self.nodes.len()),
                "N2System indexed growth capacity"
            );
            for _ in len..=index {
                let child = self.allocate(&Value::Number(0.))?;
                self.nodes.get_mut(&id).unwrap().items.push(child);
            }
            return Ok(self.nodes[&id].items[index]);
        }
        let name = key.string()?;
        ensure!(name.len() <= 4096, "N2System indexed name capacity");
        if let Some(child) = self
            .nodes
            .get(&id)
            .context("N2System indexed parent missing")?
            .names
            .get(&name)
        {
            return Ok(*child);
        }
        let child = self.allocate(&Value::Number(0.))?;
        self.nodes.get_mut(&id).unwrap().names.insert(name, child);
        Ok(child)
    }
    pub fn items(&mut self, id: u32, values: &[Argument]) -> Result<()> {
        ensure!(
            self.nodes.contains_key(&id),
            "N2System indexed target missing"
        );
        ensure!(
            values.len() <= 262144usize.saturating_sub(self.nodes.len()),
            "N2System indexed initializer capacity"
        );
        let mut items = Vec::with_capacity(values.len());
        for value in values {
            items.push(self.allocate(&value.value)?);
        }
        let node = self.nodes.get_mut(&id).unwrap();
        node.items = items;
        node.names.clear();
        Ok(())
    }
    pub fn names(&mut self, id: u32, names: &[Argument]) -> Result<()> {
        ensure!(
            self.nodes.contains_key(&id),
            "N2System associative target missing"
        );
        let mut labels = Vec::with_capacity(names.len());
        for name in names {
            let label = name.value.string()?;
            ensure!(label.len() <= 4096, "N2System associative name capacity");
            labels.push(label);
        }
        let mut map = BTreeMap::new();
        for (index, name) in labels.into_iter().enumerate() {
            let child = self.child(id, &Value::Number(index as f32))?;
            map.insert(name, child);
        }
        self.nodes.get_mut(&id).unwrap().names = map;
        Ok(())
    }
}
impl Machine {
    pub(crate) fn merge_common(
        &mut self,
        flags: &mut BTreeMap<String, Value>,
        arrays: &Arrays,
        replace: bool,
    ) -> Result<()> {
        arrays.validate()?;
        ensure!(
            self.arrays
                .next
                .checked_add(u32::try_from(arrays.nodes.len())?)
                .is_some(),
            "N2System common indexed identity exhaustion"
        );
        ensure!(
            self.arrays.nodes.len().saturating_add(arrays.nodes.len()) <= 262144,
            "N2System common-data merge capacity"
        );
        ensure!(
            (if replace {
                flags.len()
            } else {
                self.flags
                    .keys()
                    .chain(flags.keys())
                    .collect::<BTreeSet<_>>()
                    .len()
            }) <= 32766,
            "N2System common flag capacity"
        );
        let mut ids = BTreeMap::new();
        for (id, node) in &arrays.nodes {
            ids.insert(*id, self.arrays.allocate(&node.scalar)?);
        }
        for (id, node) in &arrays.nodes {
            let target = self.arrays.nodes.get_mut(&ids[id]).unwrap();
            target.items = node.items.iter().map(|id| ids[id]).collect();
            target.names = node
                .names
                .iter()
                .map(|(name, id)| (name.clone(), ids[id]))
                .collect();
        }
        // Native469890 clears the # table before installing its saved values.
        // Array admission/remapping above is fallible and must precede that
        // replacement. Local thread references retain their existing arena IDs.
        if replace {
            self.flags.clear();
        }
        for (name, value) in flags.iter() {
            let value = if let Value::Array(id, _) = value {
                self.arrays.value(ids[id])?
            } else {
                value.clone()
            };
            self.flags.insert(name.clone(), value);
        }
        Ok(())
    }
    pub fn indexed_builtin(&mut self, call: &crate::vm::Call) -> Result<Option<Value>> {
        use crate::catalog::Builtin;
        let args = &call.arguments;
        match call.function {
            Builtin::Array | Builtin::AssocArray => {
                if let Some(first) = args.first() {
                    let id = self.ensure_array(first)?;
                    if call.function == Builtin::Array {
                        self.arrays.items(id, &args[1..])?;
                    } else {
                        self.arrays.names(id, &args[1..])?;
                    }
                }
                Ok(Some(Value::Null))
            }
            Builtin::Count => {
                let count = if let Some(Argument {
                    value: Value::Array(id, _),
                    ..
                }) = args.first()
                {
                    self.arrays
                        .nodes
                        .get(id)
                        .context("N2System Count indexed identity")?
                        .items
                        .len()
                } else {
                    0
                };
                Ok(Some(Value::Number(count as f32)))
            }
            Builtin::VariableValue => {
                ensure!(
                    (2..=3).contains(&args.len()),
                    "N2System VariableValue argument count"
                );
                let sigil = args[0].value.string()?;
                ensure!(
                    sigil == "#" || sigil == "$",
                    "N2System VariableValue global sigil"
                );
                let path = args[1].value.string()?;
                let split = path.find('[').unwrap_or(path.len());
                let name = crate::source::Name {
                    text: path[..split].into(),
                    flag: sigil == "#",
                    explicit: true,
                };
                ensure!(
                    !name.text.is_empty() && !name.text.contains(['$', '#', ']']),
                    "N2System VariableValue path"
                );
                if split == path.len() {
                    let value = if let Some(value) = args.get(2) {
                        let root = self.read(&name);
                        let value = if let Value::Array(id, _) = root {
                            self.arrays
                                .nodes
                                .get_mut(&id)
                                .context("N2System dynamic indexed root")?
                                .scalar = value.value.scalar().clone();
                            self.arrays.value(id)?
                        } else {
                            value.value.clone()
                        };
                        self.assign(&name, value.clone())?;
                        value
                    } else {
                        self.read(&name)
                    };
                    return Ok(Some(value));
                }
                let value = Argument {
                    value: self.read(&name),
                    reference: Some(name),
                };
                let mut id = self.ensure_array(&value)?;
                let mut tail = &path[split..];
                let mut depth = 0;
                while !tail.is_empty() {
                    depth += 1;
                    ensure!(
                        depth <= 64 && tail.starts_with('['),
                        "N2System dynamic indexed depth/path"
                    );
                    let close = tail.find(']').context("N2System dynamic indexed close")?;
                    let key = &tail[1..close];
                    ensure!(
                        !key.is_empty() && !key.contains('['),
                        "N2System dynamic indexed key"
                    );
                    id = self.arrays.child(id, &Value::String(key.into()))?;
                    tail = &tail[close + 1..];
                }
                if let Some(value) = args.get(2) {
                    self.arrays.nodes.get_mut(&id).unwrap().scalar = value.value.scalar().clone();
                }
                Ok(Some(self.arrays.value(id)?))
            }
            Builtin::DeleteArray => {
                if let Some(first) = args.first() {
                    if let Value::Array(id, _) = first.value {
                        let node = self
                            .arrays
                            .nodes
                            .get_mut(&id)
                            .context("N2System DeleteArray target missing")?;
                        node.items.clear();
                        node.names.clear();
                    }
                }
                Ok(Some(Value::Null))
            }
            Builtin::Strlen => {
                let text = args
                    .first()
                    .map(|v| v.value.string())
                    .transpose()?
                    .unwrap_or_default();
                Ok(Some(Value::Number(text.encode_utf16().count() as f32)))
            }
            Builtin::Substr => {
                let text = args
                    .first()
                    .map(|v| v.value.string())
                    .transpose()?
                    .unwrap_or_default();
                let start = args
                    .get(1)
                    .context("N2System Substr start missing")?
                    .value
                    .number()?;
                let length = args
                    .get(2)
                    .map(|v| v.value.number())
                    .transpose()?
                    .unwrap_or(text.encode_utf16().count() as f32);
                ensure!(
                    start.is_finite() && length.is_finite() && start >= 0. && length >= 0.,
                    "N2System Substr range"
                );
                let text: Vec<u16> = text.encode_utf16().collect();
                let start = (start as usize).min(text.len());
                let end = start.saturating_add(length as usize).min(text.len());
                Ok(Some(Value::String(String::from_utf16(&text[start..end])?)))
            }
            Builtin::Double => {
                let value = args
                    .first()
                    .context("N2System Double value missing")?
                    .value
                    .scalar();
                let number = if let Value::String(s) = value {
                    s.parse::<f32>()?
                } else {
                    value.number()?
                };
                ensure!(number.is_finite(), "N2System floating conversion range");
                Ok(Some(Value::Number(number)))
            }
            Builtin::Strstr => {
                let text = args
                    .first()
                    .map(|v| v.value.string())
                    .transpose()?
                    .unwrap_or_default();
                let pattern = args
                    .get(1)
                    .map(|v| v.value.string())
                    .transpose()?
                    .unwrap_or_default();
                // Windows NSS scripts subtract one from this result before
                // zero-based Substr calls. Preserve the1-based UTF-16 index;
                // the fork's boolean ContainsString is not this ABI.
                Ok(Some(Value::Number(text.find(&pattern).map_or(0., |at| {
                    (text[..at].encode_utf16().count() + 1) as f32
                }))))
            }
            _ => Ok(None),
        }
    }
    pub fn ensure_array(&mut self, value: &Argument) -> Result<u32> {
        if let Value::Array(id, _) = value.value {
            ensure!(
                self.arrays.nodes.contains_key(&id),
                "N2System indexed identity missing"
            );
            return Ok(id);
        }
        let id = self.arrays.allocate(&value.value)?;
        if let Some(reference) = &value.reference {
            self.assign(reference, self.arrays.value(id)?)?;
        }
        Ok(id)
    }
    /// Sweep only at an owner safepoint, after every execution has returned its
    /// pending call/stack to the scheduler. Named and numeric aliases mark the
    /// same node. Worker parsing/layout never owns mutable VM indexed values.
    pub fn collect_arrays<'a>(&mut self, threads: impl Iterator<Item = &'a Thread>) {
        if self.arrays.nodes.len() < 8192 {
            return;
        }
        let mut todo = Vec::new();
        let mut root = |value: &Value| {
            if let Value::Array(id, _) = value {
                todo.push(*id);
            }
        };
        for value in self.variables.values().chain(self.flags.values()) {
            root(value);
        }
        for thread in threads {
            for arg in &thread.stack {
                root(&arg.value);
            }
            if let Some(call) = &thread.pending {
                for arg in &call.arguments {
                    root(&arg.value);
                }
            }
        }
        let mut live = BTreeSet::new();
        while let Some(id) = todo.pop() {
            if live.insert(id) {
                if let Some(node) = self.arrays.nodes.get(&id) {
                    todo.extend(node.items.iter().copied());
                    todo.extend(node.names.values().copied());
                }
            }
        }
        self.arrays.nodes.retain(|id, _| live.contains(id));
    }
}
