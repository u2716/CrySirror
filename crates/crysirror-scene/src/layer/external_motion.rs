// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Engine-owned completion for external animation producers. No VM, callback,
//! interpolator or executable code is stored. Pose commands and one completion
//! sample can be admitted atomically in the ordinary Layer command batch.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};
//Ephemeral capabilities must not become valid again after deleting/reusing a
//node or restoring a prior scene. Workers cannot survive process lifetime.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
///Finite generic Layer sample, independent of the producer's VM/program. Any
///omitted component retains its current value; filters/paint are unaffected.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalMotionPose {
    /// Procedural additive channels, independent of retained base tracks.
    /// Publish a zero offset to return to the unmodified base pose.
    #[serde(default)]
    pub offset: Option<super::PoseOffset>,
    #[serde(default)]
    pub rect: Option<[f32; 4]>,
    #[serde(default)]
    pub transform: Option<super::SpriteTransform>,
    #[serde(default)]
    pub opacity: Option<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalMotionStamp {
    #[serde(with = "crate::input_buffer::exact_u64")]
    pub generation: u64,
    #[serde(with = "crate::input_buffer::exact_u64")]
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalMotionStatus {
    ///Producer identity in this client/layer scope, never a process pointer.
    pub driver: u32,
    pub stamp: ExternalMotionStamp,
    ///Requested by an owner; only the real producer can publish completion.
    pub skip_requested: bool,
    pub complete: bool,
}
#[derive(Clone, Debug, Default)]
pub(super) struct ExternalMotions {
    clients: BTreeMap<u32, BTreeMap<u32, BTreeMap<u32, ExternalMotionStatus>>>,
}
impl super::LayerSystem {
    /// Whether this client still owns external producer registrations, even
    /// completed producers not yet retired. Their host continuations cannot be
    /// reconstructed by restoring plain scene records.
    pub fn has_external_motion_producers(&self, client: crysirror_core::state::ClientId) -> bool {
        self.motions
            .external
            .clients
            .get(&client.0)
            .is_some_and(|layers| layers.values().any(|drivers| !drivers.is_empty()))
    }
}
impl ExternalMotions {
    pub fn begin(&mut self, client: u32, layer: u32, driver: u32) -> Result<()> {
        let layers = self.clients.get(&client);
        ensure!(
            !layers
                .and_then(|l| l.get(&layer))
                .is_some_and(|d| d.contains_key(&driver)),
            "external motion driver already exists"
        );
        let count = |l: &BTreeMap<u32, BTreeMap<u32, ExternalMotionStatus>>| {
            l.values().map(BTreeMap::len).sum::<usize>()
        };
        ensure!(
            layers.map_or(0, count) < 4096
                && self.clients.values().map(count).sum::<usize>() < 16384,
            "external motion driver budget"
        );
        ensure!(
            layers.and_then(|l| l.get(&layer)).map_or(0, BTreeMap::len) < 256,
            "external motion layer driver budget"
        );
        let generation = NEXT_GENERATION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| anyhow::anyhow!("external motion generation exhausted"))?;
        self.clients
            .entry(client)
            .or_default()
            .entry(layer)
            .or_default()
            .insert(
                driver,
                ExternalMotionStatus {
                    driver,
                    stamp: ExternalMotionStamp {
                        generation,
                        revision: 0,
                    },
                    skip_requested: false,
                    complete: false,
                },
            );
        Ok(())
    }
    fn entry(
        &mut self,
        client: u32,
        layer: u32,
        driver: u32,
        generation: u64,
    ) -> Result<&mut ExternalMotionStatus> {
        let state = self
            .clients
            .get_mut(&client)
            .and_then(|c| c.get_mut(&layer))
            .and_then(|l| l.get_mut(&driver))
            .context("external motion driver missing")?;
        ensure!(
            state.stamp.generation == generation,
            "external motion stale/foreign generation"
        );
        Ok(state)
    }
    pub fn commit(
        &mut self,
        client: u32,
        layer: u32,
        driver: u32,
        stamp: ExternalMotionStamp,
        complete: bool,
    ) -> Result<()> {
        let state = self.entry(client, layer, driver, stamp.generation)?;
        ensure!(
            !state.complete,
            "external motion producer already completed"
        );
        ensure!(
            state.stamp.revision.checked_add(1) == Some(stamp.revision),
            "external motion sample revision mismatch"
        );
        state.stamp = stamp;
        state.complete = complete;
        Ok(())
    }
    pub fn request_skip(
        &mut self,
        client: u32,
        layer: u32,
        driver: u32,
        generation: u64,
    ) -> Result<()> {
        let state = self.entry(client, layer, driver, generation)?;
        if !state.complete {
            state.skip_requested = true;
        }
        Ok(())
    }
    pub fn remove(&mut self, client: u32, layer: u32, driver: u32, generation: u64) -> Result<()> {
        self.entry(client, layer, driver, generation)?;
        self.clients
            .get_mut(&client)
            .unwrap()
            .get_mut(&layer)
            .unwrap()
            .remove(&driver);
        self.clean();
        Ok(())
    }
    pub fn cancel_layer(&mut self, client: u32, layer: u32) {
        if let Some(layers) = self.clients.get_mut(&client) {
            layers.remove(&layer);
        }
        self.clean();
    }
    fn clean(&mut self) {
        for layers in self.clients.values_mut() {
            layers.retain(|_, drivers| !drivers.is_empty());
        }
        self.clients.retain(|_, layers| !layers.is_empty());
    }
    pub fn status(&self, client: u32, layer: u32) -> Vec<ExternalMotionStatus> {
        self.clients
            .get(&client)
            .and_then(|c| c.get(&layer))
            .into_iter()
            .flat_map(|l| l.values().cloned())
            .collect()
    }
    pub fn validate(&self, allowed: bool, target: &impl Fn(u32, u32) -> bool) -> Result<()> {
        ensure!(
            allowed || self.clients.is_empty(),
            "UI external motion producers are not supported"
        );
        let mut total = 0;
        for (&client, layers) in &self.clients {
            let mut owned = 0;
            for (&layer, drivers) in layers {
                ensure!(
                    target(client, layer) && !drivers.is_empty() && drivers.len() <= 256,
                    "external motion target invalid"
                );
                for (&driver, state) in drivers {
                    ensure!(
                        driver == state.driver && state.stamp.generation > 0,
                        "external motion identity invalid"
                    );
                }
                owned += drivers.len();
            }
            ensure!(owned <= 4096, "external motion owner budget");
            total += owned;
        }
        ensure!(total <= 16384, "external motion global budget");
        Ok(())
    }
}
//A worker/producer continuation is a capability, not project/save data. Refuse
//a snapshot with retained producers (even completed ones) until the owner
//explicitly retires them. Restoring fake active metadata must never turn an
//unbound producer into complete=true or accept an old asynchronous sample.
impl Serialize for ExternalMotions {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        if !self.clients.is_empty() {
            return Err(serde::ser::Error::custom(
                "retire external motion producers before snapshot",
            ));
        }
        BTreeMap::<u32, u32>::new().serialize(s)
    }
}
impl<'de> Deserialize<'de> for ExternalMotions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let values = BTreeMap::<u32, serde::de::IgnoredAny>::deserialize(d)?;
        if !values.is_empty() {
            return Err(serde::de::Error::custom(
                "external motion producer continuation cannot be restored",
            ));
        }
        Ok(Self::default())
    }
}

impl ExternalMotions {
    pub(super) fn retain_checkpoint_scope(&mut self, client:u32, ids:&std::collections::BTreeSet<u32>) {
        self.clients.retain(|owner,layers| {if *owner!=client{return false} layers.retain(|id,_|ids.contains(id));!layers.is_empty()});
    }
    pub(super) fn restore_checkpoint_scope(&mut self, client:u32, removed:&std::collections::BTreeSet<u32>, saved:&Self)->Result<()> {
        if let Some(owner)=self.clients.get_mut(&client){owner.retain(|id,_|!removed.contains(id));}
        if let Some(owner)=saved.clients.get(&client) {
            for (&layer,drivers) in owner {
                for (&driver,state) in drivers {
                    self.begin(client,layer,driver)?;
                    let restored=self.clients.get_mut(&client).unwrap().get_mut(&layer).unwrap().get_mut(&driver).unwrap();
                    restored.skip_requested=state.skip_requested;
                    restored.complete=state.complete;
                    // New generation: an old asynchronous producer cannot
                    // mutate a newly restored logical owner continuation.
                    restored.stamp.revision=state.stamp.revision;
                }
            }
        }
        Ok(())
    }
}
