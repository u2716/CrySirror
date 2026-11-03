// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original5DC340/5DC9D0(23/24)/64FA00/650230 task return slots.
//! Distinct from FES player's519 result and parent's108 cached child status.
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct Results {
    slots: BTreeMap<u32, u32>,
}
impl Results {
    pub fn construct(&mut self, id: u32) -> Result<(), &'static str> {
        if id >= 65536 || self.slots.contains_key(&id) {
            return Err("native task result slot ownership");
        }
        //5DC340 writes task20=-1, independent of zero-initialized allocation.
        self.slots.insert(id, u32::MAX);
        Ok(())
    }
    pub fn query(&self, id: u32) -> u32 {
        //64FA00 initializes its out value to-1; a missing native receiver leaves
        //that value unchanged. This is not a fabricated successful result.
        self.slots.get(&id).copied().unwrap_or(u32::MAX)
    }
    pub fn set(&mut self, id: u32, result: u32) -> Result<(), &'static str> {
        *self
            .slots
            .get_mut(&id)
            .ok_or("native task result target missing")? = result;
        Ok(())
    }
    pub fn destroy(&mut self, id: u32) -> Result<(), &'static str> {
        self.slots
            .remove(&id)
            .ok_or("native task result target missing")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_constructor_return_mutation_missing_and_reused_slot() {
        let mut r = Results::default();
        assert_eq!(r.query(8), u32::MAX);
        r.construct(8).unwrap();
        assert_eq!(r.query(8), u32::MAX);
        assert!(r.construct(8).is_err());
        r.set(8, 7).unwrap();
        assert_eq!(r.query(8), 7);
        assert_eq!(r.query(9), u32::MAX);
        assert!(r.set(9, 0).is_err());
        assert_eq!(r.query(8), 7);
        r.destroy(8).unwrap();
        assert_eq!(r.query(8), u32::MAX);
        r.construct(8).unwrap();
        assert_eq!(r.query(8), u32::MAX);
    }
}
