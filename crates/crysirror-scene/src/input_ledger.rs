//! Device-free descriptor counters and duplicate-preserving priority scopes.
//! A host supplies physical transitions; consumers own independent drain policy.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Default)]
pub struct PriorityScopes {
    counts: BTreeMap<u32, u32>,
}
impl PriorityScopes {
    pub fn register(&mut self, scope: u32) {
        let count = self.counts.entry(scope).or_default();
        *count = count.saturating_add(1);
    }
    pub fn unregister(&mut self, scope: u32) -> bool {
        let Some(count) = self.counts.get_mut(&scope) else {
            return false;
        };
        *count -= 1;
        if *count == 0 {
            self.counts.remove(&scope);
        }
        true
    }
    pub fn top(&self) -> Option<u32> {
        self.counts.last_key_value().map(|(&scope, _)| scope)
    }
    pub fn entries(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.counts.iter().map(|(&scope, &count)| (scope, count))
    }
    /// Ordered covering nodes may come from plain scopes, hit regions or menus.
    pub fn accepts_pointer(&self, scope: u32, extra: &[(u32, bool)]) -> bool {
        let mut nodes = extra.to_vec();
        nodes.extend(self.entries().map(|(scope, _)| (scope, true)));
        nodes.sort_by(|a, b| b.0.cmp(&a.0));
        for (node, covers) in nodes {
            if node <= scope {
                if node != scope {
                    return false;
                }
                if covers {
                    return true;
                }
            } else if covers {
                return false;
            }
        }
        false
    }
}
#[derive(Debug, Default)]
pub struct DescriptorLedger {
    held: BTreeSet<i32>,
    down: BTreeSet<i32>,
    reported: BTreeSet<i32>,
    counts: BTreeMap<i32, u32>,
    accumulators: BTreeMap<i32, u32>,
    toggles: BTreeSet<i32>,
}
impl DescriptorLedger {
    pub fn held(&self, key: i32) -> bool {
        self.held.contains(&key)
    }
    pub fn down(&self, key: i32) -> bool {
        self.down.contains(&key)
    }
    pub fn toggle(&self, key: i32) -> bool {
        self.toggles.contains(&key)
    }
    pub fn held_keys(&self) -> impl Iterator<Item = i32> + '_ {
        self.held.iter().copied()
    }
    pub fn press(&mut self, key: i32, count_repeat: bool) -> bool {
        if self.held.insert(key) {
            if !self.toggles.insert(key) {
                self.toggles.remove(&key);
            }
        }
        let was_up = self.down.insert(key);
        if count_repeat {
            self.reported.remove(&key);
        }
        if count_repeat || was_up {
            let count = self.counts.entry(key).or_default();
            *count = count.wrapping_add(1);
            let total = self.accumulators.entry(key).or_default();
            *total = total.wrapping_add(1);
        }
        was_up
    }
    pub fn release(&mut self, key: i32) {
        self.held.remove(&key);
        self.down.remove(&key);
        self.reported.remove(&key);
    }
    pub fn accumulate(&mut self, key: i32) {
        let total = self.accumulators.entry(key).or_default();
        *total = total.wrapping_add(1);
    }
    pub fn drain(&mut self, key: i32) -> u32 {
        if self.down.contains(&key) && !self.held.contains(&key) {
            self.down.remove(&key);
            self.reported.remove(&key);
        }
        let count = self.counts.remove(&key).unwrap_or(0);
        count
            | if self.down.contains(&key) && self.reported.insert(key) {
                0x80000000
            } else {
                0
            }
    }
    pub fn activation_count(&self, key: i32) -> u32 {
        self.accumulators.get(&key).copied().unwrap_or(0)
    }
    /// Clear event/down latches without pretending a physically held key went up.
    pub fn clear_transient(&mut self) {
        self.counts.clear();
        self.down.clear();
        self.reported.clear();
    }
}
