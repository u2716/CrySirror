// SPDX-License-Identifier: MPL-2.0
//! Copy-on-write VM state for cheap instruction checkpoints.
//!
//! The adapter checkpoints the whole VM before each native instruction so a
//! pending capability read can roll it back. Large containers wrapped in
//! `Shared` make that checkpoint a reference-count bump; the first mutation
//! after a checkpoint copies only the touched container. Serialization is
//! identical to `T`, so framework and original saves are unchanged.
use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}
impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T: Clone> DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}
impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}
impl<T: serde::Serialize> serde::Serialize for Shared<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Shared<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::new)
    }
}

impl<'a, T> IntoIterator for &'a Shared<Vec<T>> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a, T: Clone> IntoIterator for &'a mut Shared<Vec<T>> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Arc::make_mut(&mut self.0).iter_mut()
    }
}
impl<T> std::borrow::Borrow<T> for Shared<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Shared;

    #[test]
    fn checkpoint_is_shared_until_first_mutation() {
        let mut live = Shared::new(vec![1, 2, 3]);
        let checkpoint = live.clone();
        assert!(std::ptr::eq(&*live, &*checkpoint));
        live.push(4);
        assert_eq!(*checkpoint, [1, 2, 3]);
        assert_eq!(*live, [1, 2, 3, 4]);
        assert_eq!(
            serde_json::to_string(&live).unwrap(),
            serde_json::to_string(&vec![1, 2, 3, 4]).unwrap()
        );
    }
}
