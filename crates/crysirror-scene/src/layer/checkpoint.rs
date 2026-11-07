//! Exact logical subtree capture; mutable device state is never part of it.
use super::*;
impl LayerSystem {
    pub(crate) fn checkpoint_ids(&self, client: u32, roots: &[u32]) -> Result<BTreeSet<u32>> {
        let mut ids = BTreeSet::new();
        let Some(owner) = self.clients.get(&client) else {
            return Ok(ids);
        };
        for root in roots {
            if !owner.contains_key(root) {
                continue;
            }
            ids.insert(*root);
        }
        for _ in 0..=64 {
            let count = ids.len();
            for (&id, layer) in owner {
                if layer.parent.is_some_and(|parent| ids.contains(&parent)) {
                    ids.insert(id);
                }
            }
            if count == ids.len() {
                return Ok(ids);
            }
        }
        bail!("checkpoint layer hierarchy depth")
    }
    pub(crate) fn checkpoint_scope(&self, client: u32, roots: &[u32]) -> Result<Self> {
        ensure!(
            self.txn.0.is_empty(),
            "checkpoint during open layer transaction"
        );
        let ids = self.checkpoint_ids(client, roots)?;
        let mut state = self.clone();
        state.clients.retain(|owner, layers| {
            if *owner != client {
                return false;
            }
            layers.retain(|id, _| ids.contains(id));
            !layers.is_empty()
        });
        if let Some(owner) = state.clients.get(&client) {
            ensure!(
                owner.values().all(|layer| layer
                    .parent
                    .is_none_or(|parent| owner.contains_key(&parent))),
                "checkpoint roots require complete parent hierarchy"
            );
        }
        state.child_index.clear();
        state.crossfades.retain(|owner, layers| {
            if *owner != client {
                return false;
            }
            layers.retain(|id, _| ids.contains(id));
            !layers.is_empty()
        });
        state.puppets.retain(|owner, layers| {
            if *owner != client {
                return false;
            }
            layers.retain(|id, _| ids.contains(id));
            !layers.is_empty()
        });
        state
            .puppet_frames
            .retain(|(owner, id), _| *owner == client && ids.contains(id));
        state
            .dynamic_puppets
            .retain(|(owner, id)| *owner == client && ids.contains(id));
        if state
            .transition
            .as_ref()
            .is_some_and(|transition| transition.owner == client)
        {
            ensure!(
                self.clients.get(&client).map_or(0, BTreeMap::len) == ids.len(),
                "whole-client transition needs all owned roots in checkpoint"
            );
        } else {
            state.transition = None;
        }
        state.motions.retain_checkpoint_scope(client, &ids);
        state.sequences.retain_checkpoint_scope(client, &ids);
        state.depth_orders.retain_checkpoint_scope(client, &ids);
        state.validate()?;
        Ok(state)
    }
    pub(crate) fn restore_checkpoint_scope(
        &mut self,
        client: u32,
        roots: &[u32],
        saved: &Self,
    ) -> Result<()> {
        ensure!(
            self.txn.0.is_empty(),
            "checkpoint restore during open layer transaction"
        );
        let mut removed = self.checkpoint_ids(client, roots)?;
        if let Some(owner) = saved.clients.get(&client) {
            removed.extend(owner.keys());
        }
        if self
            .transition
            .as_ref()
            .is_some_and(|transition| transition.owner == client)
        {
            ensure!(
                self.clients
                    .get(&client)
                    .is_none_or(|owner| owner.keys().all(|id| removed.contains(id))),
                "partial checkpoint cannot replace another scope's whole-client transition"
            );
        }
        ensure!(
            saved.transition.is_none()
                || self
                    .transition
                    .as_ref()
                    .is_none_or(|transition| transition.owner == client),
            "scene checkpoint cannot replace another client's transition"
        );
        if let Some(owner) = self.clients.get_mut(&client) {
            owner.retain(|id, _| !removed.contains(id));
        }
        if let Some(owner) = saved.clients.get(&client) {
            self.clients
                .entry(client)
                .or_default()
                .extend(owner.clone());
        }
        self.child_index.remove(&client);
        if let Some(owner) = self.crossfades.get_mut(&client) {
            owner.retain(|id, _| !removed.contains(id));
        }
        if let Some(owner) = saved.crossfades.get(&client) {
            self.crossfades
                .entry(client)
                .or_default()
                .extend(owner.clone());
        }
        if let Some(owner) = self.puppets.get_mut(&client) {
            owner.retain(|id, _| !removed.contains(id));
        }
        if let Some(owner) = saved.puppets.get(&client) {
            self.puppets
                .entry(client)
                .or_default()
                .extend(owner.clone());
        }
        self.puppet_frames
            .retain(|(owner, id), _| *owner != client || !removed.contains(id));
        self.puppet_frames
            .extend(saved.puppet_frames.iter().map(|(k, v)| (*k, v.clone())));
        self.dynamic_puppets
            .retain(|(owner, id)| *owner != client || !removed.contains(id));
        self.dynamic_puppets
            .extend(saved.dynamic_puppets.iter().copied());
        if self
            .transition
            .as_ref()
            .is_some_and(|transition| transition.owner == client)
        {
            self.transition = None;
        }
        if let Some(transition) = &saved.transition {
            self.transition = Some(transition.clone());
            self.transition_serial = self.transition_serial.max(transition.serial);
        }
        self.motions
            .restore_checkpoint_scope(client, &removed, &saved.motions)?;
        self.sequences
            .restore_checkpoint_scope(client, &removed, &saved.sequences);
        self.depth_orders
            .restore_checkpoint_scope(client, &removed, &saved.depth_orders);
        self.validate()
    }
}
