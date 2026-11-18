// SPDX-License-Identifier: MPL-2.0
//! Persistent painter-ordered destinations. These are logical allocations, not
//! GPU handles: the presenter owns backing and submission lifetimes. A target
//! is sampled at its ordinary painter position, never topologically sorted.
use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderTargetSpec {
    pub size: [u32; 2],
    #[serde(default)]
    pub clear: [u8; 4],
    /// Immutable seed image, copied once before the first routed draw.
    #[serde(default)]
    pub initial: Option<String>,
    /// Treat stored alpha as opaque when presenting, before layer opacity.
    #[serde(default)]
    pub opaque: bool,
}
impl RenderTargetSpec {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size.iter().all(|v| (1..=8192).contains(v)),
            "render target dimensions"
        );
        if let Some(asset) = &self.initial {
            VfsPath::new(Mount::Asset, asset)?;
            ensure!(
                !asset.starts_with("__surface/")
                    && crysirror_core::service::frame_history_layer(asset).is_none(),
                "render target seed must be immutable"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderTargetBinding {
    pub owner: u32,
    pub id: u32,
    pub generation: u32,
    /// Owner-state revision: repeated presentation never repeats bitmap writes.
    pub revision: u64,
    pub spec: RenderTargetSpec,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Targets {
    pub revision: u64,
    pub allocations: BTreeMap<(u32, u32), RenderTargetBinding>,
    pub outputs: BTreeMap<(u32, u32), u32>,
}
// Saving logical declarations alone would silently lose mutable pixels. Until
// a presenter readback is explicitly attached, fail rather than restore empty.
impl Serialize for Targets {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if !self.allocations.is_empty() || !self.outputs.is_empty() {
            return Err(serde::ser::Error::custom(
                "persistent render targets require pixel capture before checkpoint",
            ));
        }
        serializer.serialize_unit()
    }
}
impl<'de> Deserialize<'de> for Targets {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        <()>::deserialize(deserializer)?;
        Ok(Self::default())
    }
}
impl Targets {
    pub fn advance(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
    pub fn validate(&self, clients: &BTreeMap<u32, BTreeMap<u32, LayerSpec>>) -> Result<()> {
        ensure!(self.revision < u64::MAX, "render target revision exhausted");
        for (&(owner, id), target) in &self.allocations {
            ensure!(
                target.owner == owner && target.id == id && target.generation != 0,
                "persistent target identity"
            );
            ensure!(
                clients
                    .get(&owner)
                    .is_some_and(|layers| layers.contains_key(&id)),
                "persistent target without its owned layer"
            );
            target.spec.validate()?;
        }
        for (&(owner, id), target) in &self.outputs {
            ensure!(
                clients
                    .get(&owner)
                    .is_some_and(|layers| layers.contains_key(&id))
                    && self.allocations.contains_key(&(owner, *target)),
                "dangling render output"
            );
        }
        Ok(())
    }
    pub fn create(&mut self, owner: u32, id: u32, spec: RenderTargetSpec) -> Result<()> {
        spec.validate()?;
        let generation = NEXT_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| anyhow::anyhow!("render target generation exhausted"))?;
        // Budget covers live backing; presenters separately reserve seed and
        // feedback scratch. Replacing a target keeps existing output bindings.
        let bytes = self
            .allocations
            .iter()
            .filter(|(key, _)| **key != (owner, id))
            .map(|(_, t)| u64::from(t.spec.size[0]) * u64::from(t.spec.size[1]) * 4)
            .sum::<u64>()
            + u64::from(spec.size[0]) * u64::from(spec.size[1]) * 4;
        let limit = if cfg!(target_os = "vita") { 24 } else { 128 } * 1024 * 1024;
        ensure!(bytes <= limit, "persistent render target byte budget");
        self.allocations.insert(
            (owner, id),
            RenderTargetBinding {
                owner,
                id,
                generation,
                revision: 0,
                spec,
            },
        );
        Ok(())
    }
    pub fn remove(&mut self, owner: u32, id: u32) {
        self.allocations.remove(&(owner, id));
        self.outputs
            .retain(|&(client, source), target| client != owner || (source != id && *target != id));
    }
}

impl LayerSystem {
    pub fn render_target(&self, client: ClientId, id: u32) -> Option<&RenderTargetBinding> {
        self.targets.allocations.get(&(client.0, id))
    }
    pub(super) fn route_frame(&self, frame: &mut Vec<Primitive>) -> Result<()> {
        ensure!(
            self.targets.revision < u64::MAX,
            "render target revision exhausted"
        );
        let stamp = |target: &RenderTargetBinding| {
            let mut target = target.clone();
            target.revision = self.targets.revision;
            target
        };
        for p in frame.iter_mut() {
            if let Some(id) = self.targets.outputs.get(&(p.owner, p.id)) {
                ensure!(
                    p.composite.is_none(),
                    "routed composite requires an explicit target scope"
                );
                p.output = Some(
                    self.targets
                        .allocations
                        .get(&(p.owner, *id))
                        .context("render output allocation missing")?
                        .clone(),
                );
            }
            p.output = p.output.as_ref().map(stamp);
            p.target = self.targets.allocations.get(&(p.owner, p.id)).map(stamp);
        }
        // Hidden/faded targets still own backing and receive routed writes.
        // A zero-area declaration reserves backing without presenting it.
        for target in self.targets.allocations.values() {
            if !frame
                .iter()
                .any(|p| p.owner == target.owner && p.id == target.id)
            {
                let mut declaration = rectangle(
                    target.owner,
                    target.id,
                    0,
                    [
                        0.,
                        0.,
                        target.spec.size[0] as f32,
                        target.spec.size[1] as f32,
                    ],
                    Paint::Solid { color: [0; 4] },
                    None,
                    [0; 2],
                );
                declaration.opacity = 0;
                declaration.target = Some(stamp(target));
                frame.push(declaration);
            }
        }
        Ok(())
    }
}
