// SPDX-License-Identifier: MPL-2.0
// Native names, aliases and query scopes follow NitroSharp Entity{Path,Query,
// Pattern,Alias}.cs and Rebuild/World.cs at e3869842 (MIT). Rendering stays in
// CrySirror's retained layer tree; these are script identities, not GPU objects.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    Name,
    Thread,
    Image {
        paint: Json,
        size: [u32; 2],
    },
    Visual,
    Sound {
        source: String,
        kind: String,
        gain: f32,
        looping: bool,
        playing: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: u32,
    pub name: String,
    pub parent: Option<u32>,
    pub kind: Kind,
    pub size: [f32; 2],
    pub locked: bool,
    pub hideable: bool,
    /// ConstraintBox's native animation propagation switch. Ordinary render
    /// items use None; a non-container box owns children without animating them.
    pub container: Option<bool>,
    pub pivot_auto: bool,
}
impl Entity {
    pub fn group(&self) -> Json {
        json!({"op":"put","layer":{"id":self.id,"parent":self.parent,
            "rect":[0,0,self.size[0].max(1.),self.size[1].max(1.)],"paint":{"kind":"group"}}})
    }
}
pub struct Entities {
    pub nodes: BTreeMap<u32, Entity>,
    aliases: BTreeMap<String, u32>,
    next: u32,
}
impl Default for Entities {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            aliases: BTreeMap::new(),
            next: 1,
        }
    }
}
impl Entities {
    pub fn root(&mut self, name: String) -> Result<u32> {
        self.add(name, None, Kind::Thread, [0.; 2])
    }
    pub fn add(
        &mut self,
        name: String,
        parent: Option<u32>,
        kind: Kind,
        size: [f32; 2],
    ) -> Result<u32> {
        ensure!(self.nodes.len() < 8192, "N2System live entity capacity");
        let id = self.next;
        self.next = self
            .next
            .checked_add(1)
            .context("N2System entity identity exhausted")?;
        self.nodes.insert(
            id,
            Entity {
                id,
                name,
                parent,
                kind,
                size,
                locked: false,
                hideable: false,
                container: None,
                pivot_auto: true,
            },
        );
        Ok(id)
    }
    /// Resolve every parent before allocating/replacing the final name.
    pub fn destination(&self, current: u32, path: &str) -> Result<(u32, String)> {
        let parts: Vec<_> = path.split('/').collect();
        ensure!(
            !parts.is_empty() && parts.len() <= 32 && path.len() <= 1024,
            "N2System entity path capacity"
        );
        let mut parent = current;
        for (at, part) in parts.iter().enumerate() {
            ensure!(
                !part.is_empty() && !part.contains(['*', '<', '>']),
                "N2System invalid entity path {path}"
            );
            if at + 1 == parts.len() {
                ensure!(
                    !part.starts_with('@'),
                    "N2System alias cannot be an entity's declared name"
                );
                return Ok((parent, (*part).into()));
            }
            parent = if let Some(alias) = part.strip_prefix('@') {
                self.aliases.get(alias).copied()
            } else {
                self.child(parent, part)
            }
            .with_context(|| format!("N2System missing entity parent in {path}"))?;
        }
        unreachable!()
    }
    pub fn child(&self, parent: u32, name: &str) -> Option<u32> {
        self.nodes
            .values()
            .find(|e| e.parent == Some(parent) && e.name == name)
            .map(|e| e.id)
    }
    /// Native render-item positions and opacity are absolute. Keep ownership
    /// in this graph while the shared render tree retains only logical state
    /// groups (including choice visibility), avoiding a second inheritance.
    pub fn render_parent(&self, id: u32) -> Result<Option<u32>> {
        let mut parent = self.nodes[&id].parent;
        for _ in 0..128 {
            let Some(id) = parent else { return Ok(None); };
            let entity = &self.nodes[&id];
            if !matches!(entity.kind, Kind::Visual) { return Ok(Some(id)); }
            parent = entity.parent;
        }
        anyhow::bail!("N2System render ownership depth")
    }
    /// RenderItem's animation propagation follows direct render-item children;
    /// BasicEntity/name nodes break it, as do non-container constraint boxes.
    pub fn animation_targets(&self, root: u32) -> Vec<u32> {
        let mut result = Vec::new();
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            let entity = &self.nodes[&id];
            if !matches!(entity.kind, Kind::Visual) { continue; }
            result.push(id);
            if entity.container != Some(false) {
                todo.extend(self.nodes.values().filter(|e| e.parent == Some(id)).map(|e| e.id));
            }
        }
        result
    }
    /// Native4511F0 applies bitmap callbacks to the matched node, then
    /// propagates through enabled drawable containers. Resource nodes receive
    /// a callback too; ordinary names do not propagate it to their children.
    pub fn bitmap_targets(&self, root: u32) -> Vec<u32> {
        let mut result = Vec::new();
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            let Some(entity) = self.nodes.get(&id) else { continue; };
            result.push(id);
            if matches!(entity.kind, Kind::Visual) && entity.container != Some(false) {
                todo.extend(self.nodes.values().filter(|e| e.parent == Some(id)).map(|e| e.id));
            }
        }
        result
    }
    pub fn query(&self, current: u32, main: u32, query: &str) -> Result<Vec<u32>> {
        ensure!(
            !query.is_empty() && query.len() <= 1024,
            "N2System malformed entity query"
        );
        let mut selected = vec![current];
        for (depth, part) in query.split('/').enumerate() {
            ensure!(
                depth < 32 && !part.is_empty() && !part.contains('>'),
                "N2System unsupported entity query scope: {query}"
            );
            let (all_aliases, alias, main_scope, pattern) = if let Some(p) = part.strip_prefix("<@")
            {
                (true, true, false, p)
            } else if let Some(p) = part.strip_prefix('@') {
                (false, true, false, p)
            } else if let Some(p) = part.strip_prefix('<') {
                (false, false, true, p)
            } else {
                (false, false, false, part)
            };
            ensure!(
                !pattern.is_empty()
                    // Alias keys keep a literal leading '@' (see `alias`), so
                    // `@@NAME` addresses the alias "@NAME".
                    && !pattern.contains('<')
                    && (alias || !pattern.contains('@')),
                "N2System malformed entity query part {part:?} in {query:?}"
            );
            let mut next = BTreeSet::new();
            if alias {
                // One process is owned by this adapter. Cross-process aliases
                // cannot grant access to a different CrySirror client.
                let _ = all_aliases;
                for (name, id) in &self.aliases {
                    if matches(pattern, name) {
                        next.insert(*id);
                    }
                }
            } else {
                let parents = if main_scope { vec![main] } else { selected };
                for parent in parents {
                    for e in self.nodes.values().filter(|e| e.parent == Some(parent)) {
                        if matches(pattern, &e.name) {
                            next.insert(e.id);
                        }
                    }
                }
            }
            selected = next.into_iter().collect();
        }
        Ok(selected)
    }
    /// Detached copy of the identity tables for a slot checkpoint.
    pub fn export(&self) -> (Vec<Entity>, BTreeMap<String, u32>, u32) {
        (
            self.nodes.values().cloned().collect(),
            self.aliases.clone(),
            self.next,
        )
    }
    /// Replace the identity tables from a validated slot checkpoint. The next
    /// id never moves backwards, so ids released by the live scene stay unique.
    pub fn import(
        &mut self,
        nodes: Vec<Entity>,
        aliases: BTreeMap<String, u32>,
        next: u32,
    ) -> Result<()> {
        ensure!(nodes.len() <= 8192, "N2System live entity capacity");
        let max = nodes.iter().map(|e| e.id).max().unwrap_or(0);
        ensure!(max < u32::MAX && next > max, "N2System checkpoint entity ids");
        self.nodes = nodes.into_iter().map(|e| (e.id, e)).collect();
        self.aliases = aliases;
        self.next = self.next.max(next);
        Ok(())
    }
    pub fn alias(&mut self, id: u32, alias: String) -> Result<()> {
        ensure!(
            !alias.is_empty() && alias.len() <= 1024 && !alias.contains(['*', '<']),
            "N2System malformed alias: {alias:?}"
        );
        // Totono's CreateSE helper forwards its entire qualified entity path
        // (e.g. @EFWIN/SEEVENING) as the alias too. Retain that literal key for
        // alias enumeration; it is an opaque native name, never a VFS path.
        ensure!(
            self.nodes.contains_key(&id),
            "N2System alias target missing"
        );
        ensure!(
            self.aliases.len() < 8192 || self.aliases.contains_key(&alias),
            "N2System alias capacity"
        );
        self.aliases.insert(alias, id);
        Ok(())
    }
    pub fn subtree(&self, id: u32) -> Vec<u32> {
        let mut out = vec![id];
        let mut at = 0;
        while at < out.len() {
            let parent = out[at];
            out.extend(
                self.nodes
                    .values()
                    .filter(|e| e.parent == Some(parent))
                    .map(|e| e.id),
            );
            at += 1;
        }
        out
    }
    pub fn remove(&mut self, id: u32) -> Vec<Entity> {
        let ids = self.subtree(id);
        let mut removed = Vec::new();
        for id in ids.into_iter().rev() {
            self.aliases.retain(|_, target| *target != id);
            if let Some(entity) = self.nodes.remove(&id) {
                removed.push(entity);
            }
        }
        removed
    }
    pub fn parent_size(&self, id: u32, canvas: [u32; 2]) -> [f32; 2] {
        let mut parent = self.nodes.get(&id).and_then(|e| e.parent);
        while let Some(id) = parent {
            let Some(e) = self.nodes.get(&id) else { break };
            if matches!(e.kind, Kind::Visual) {
                return e.size;
            }
            parent = e.parent;
        }
        canvas.map(|n| n as f32)
    }
}

/// N2 uses '*' only; '?' is an ordinary character. Match a name segment,
/// never a filesystem path, without recursive expansion or per-character DP.
fn matches(pattern: &str, name: &str) -> bool {
    let mut rest = name;
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(tail) = rest.strip_prefix(first) else {
        return false;
    };
    rest = tail;
    let chunks: Vec<_> = parts.collect();
    if chunks.is_empty() {
        return rest.is_empty();
    }
    for (at, chunk) in chunks.iter().enumerate() {
        if at + 1 == chunks.len() {
            return rest.ends_with(chunk);
        }
        let Some(found) = rest.find(chunk) else {
            return false;
        };
        rest = &rest[found + chunk.len()..];
    }
    true
}
