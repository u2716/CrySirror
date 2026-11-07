//! 6D648C selects svar=1, gvar=2, var=3. 6C5B28 resolves the shared
//! calculator bank (+12/+8); 6C5A08 resolves the per-thread bank (+20/+16).
//! 6E633C/6C5A00 links only System's shared bank to Game's. In particular,
//! identically named gvar image/button arrays must never alias across readers.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct ValueBank {
    pub values: BTreeMap<String, Value>,
    pub arrays: BTreeMap<String, usize>,
    pub array_types: BTreeMap<String, String>,
    pub array_shapes: BTreeMap<String, Vec<usize>>,
}
impl Machine {
    pub(crate) fn reset_system_globals(&mut self) {
        // System startup calls 6E4204 again: clear thread declarations and
        // register fresh Result/builtin cells while retaining shared svar.
        self.system_bank = self.initial_bank.clone();
    }
    pub(crate) fn global_bindings(&self) -> impl Iterator<Item = (String, &Value)> {
        self.globals
            .iter()
            .map(|(name, value)| (name.clone(), value))
            .chain(
                self.system_bank
                    .values
                    .iter()
                    .map(|(name, value)| (format!("system/{name}"), value)),
            )
            .chain(
                self.shared_bank
                    .values
                    .iter()
                    .map(|(name, value)| (format!("shared/{name}"), value)),
            )
    }
    pub(crate) fn thread_globals(&self) -> &BTreeMap<String, Value> {
        self.globals_for(self.thread_kind())
    }
    fn globals_for(&self, thread: ThreadKind) -> &BTreeMap<String, Value> {
        match thread {
            ThreadKind::Game => &self.globals,
            ThreadKind::System => &self.system_bank.values,
        }
    }
    pub(crate) fn thread_globals_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self.thread_kind() {
            ThreadKind::Game => &mut self.globals,
            ThreadKind::System => &mut self.system_bank.values,
        }
    }
    pub(crate) fn global_value(&self, name: &str) -> Option<&Value> {
        self.global_value_for(self.thread_kind(), name)
    }
    pub(super) fn global_value_for(&self, thread: ThreadKind, name: &str) -> Option<&Value> {
        self.globals_for(thread)
            .get(name)
            .or_else(|| self.shared_bank.values.get(name))
    }
    pub(crate) fn global_contains(&self, name: &str) -> bool {
        self.global_value(name).is_some()
    }
    pub(crate) fn set_global_value(&mut self, name: String, value: Value) {
        if !self.thread_globals().contains_key(&name) && self.shared_bank.values.contains_key(&name)
        {
            self.shared_bank.values.insert(name, value);
        } else {
            self.thread_globals_mut().insert(name, value);
        }
    }
    pub(crate) fn global_arrays(&self) -> &BTreeMap<String, usize> {
        match self.thread_kind() {
            ThreadKind::Game => &self.arrays,
            ThreadKind::System => &self.system_bank.arrays,
        }
    }
    pub(crate) fn global_array_extent(&self, name: &str) -> Option<usize> {
        self.global_arrays()
            .get(name)
            .or_else(|| self.shared_bank.arrays.get(name))
            .copied()
    }
    pub(crate) fn global_array_kind(&self, name: &str) -> Option<&String> {
        let types = match self.thread_kind() {
            ThreadKind::Game => &self.array_types,
            ThreadKind::System => &self.system_bank.array_types,
        };
        types
            .get(name)
            .or_else(|| self.shared_bank.array_types.get(name))
    }
    pub(crate) fn set_global_array_extent(&mut self, name: String, count: usize) {
        if !self.global_arrays().contains_key(&name) && self.shared_bank.arrays.contains_key(&name)
        {
            self.shared_bank.arrays.insert(name, count);
        } else {
            match self.thread_kind() {
                ThreadKind::Game => {
                    self.arrays.insert(name, count);
                }
                ThreadKind::System => {
                    self.system_bank.arrays.insert(name, count);
                }
            }
        }
    }
    pub(crate) fn set_global_array_value(&mut self, base: &str, index: usize, value: Value) {
        let key = format!("{base}[{index}]");
        if !self.global_arrays().contains_key(base) && self.shared_bank.arrays.contains_key(base) {
            self.shared_bank.values.insert(key, value);
        } else {
            self.thread_globals_mut().insert(key, value);
        }
    }
    pub(crate) fn remove_global_value(&mut self, name: &str) {
        if self.thread_globals().contains_key(name) {
            self.thread_globals_mut().remove(name);
        } else {
            self.shared_bank.values.remove(name);
        }
    }
    pub(crate) fn remove_global_array(&mut self, name: &str) {
        if self.global_arrays().contains_key(name) {
            match self.thread_kind() {
                ThreadKind::Game => {
                    self.arrays.remove(name);
                    self.array_types.remove(name);
                    self.array_shapes.remove(name);
                }
                ThreadKind::System => {
                    self.system_bank.arrays.remove(name);
                    self.system_bank.array_types.remove(name);
                    self.system_bank.array_shapes.remove(name);
                }
            }
        } else {
            self.shared_bank.arrays.remove(name);
            self.shared_bank.array_types.remove(name);
            self.shared_bank.array_shapes.remove(name);
        }
    }
    pub(crate) fn global_object_roots(&self) -> impl Iterator<Item = &Value> {
        self.globals
            .iter()
            .chain(self.system_bank.values.iter())
            .chain(self.shared_bank.values.iter())
            .filter(|(key, _)| !key.starts_with("resultobj["))
            .map(|(_, value)| value)
    }
}
