// SPDX-License-Identifier: MPL-2.0
//! Byte-keyed queue maps retain their wire shape through internally tagged
//! commands. Serde's tag buffering turns JSON object keys into string Content,
//! losing serde_json's numeric map-key deserializer. Accept that decimal form
//! explicitly, while preserving numeric keys in binary formats and rejecting
//! malformed/out-of-range keys instead of choosing a fallback lane or bank.
use serde::{Deserialize, Deserializer, de::{self, MapAccess, Visitor}};
use std::{collections::BTreeMap, fmt, marker::PhantomData};

struct Key(u8);
impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if !deserializer.is_human_readable() {
            return u8::deserialize(deserializer).map(Self);
        }
        struct KeyVisitor;
        impl Visitor<'_> for KeyVisitor {
            type Value = Key;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a byte map key (integer or decimal string in 0..=255)")
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Key, E> {
                u8::try_from(value).map(Key).map_err(E::custom)
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Key, E> {
                u8::try_from(value).map(Key).map_err(E::custom)
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Key, E> {
                if value.is_empty() || value.len() > 3 || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(E::custom("byte map key must contain 1..=3 decimal digits"));
                }
                value.parse::<u8>().map(Key).map_err(E::custom)
            }
        }
        deserializer.deserialize_any(KeyVisitor)
    }
}

pub(super) fn deserialize<'de, D, V>(deserializer: D) -> Result<BTreeMap<u8, V>, D::Error>
where D: Deserializer<'de>, V: Deserialize<'de> {
    struct MapVisitor<V>(PhantomData<V>);
    impl<'de, V: Deserialize<'de>> Visitor<'de> for MapVisitor<V> {
        type Value = BTreeMap<u8, V>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a map with at most 256 distinct byte keys")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
            if input.size_hint().is_some_and(|n| n > 256) {
                return Err(de::Error::custom("byte map exceeds 256 entries"));
            }
            let mut values = BTreeMap::new();
            while let Some(Key(key)) = input.next_key::<Key>()? {
                if values.contains_key(&key) {
                    return Err(de::Error::custom("duplicate byte map key"));
                }
                values.insert(key, input.next_value()?);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(MapVisitor(PhantomData))
}

struct Map<V>(BTreeMap<u8, V>);
impl<'de, V: Deserialize<'de>> Deserialize<'de> for Map<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize(deserializer).map(Self)
    }
}

pub(super) fn nested<'de, D, V>(deserializer: D) -> Result<BTreeMap<u8, BTreeMap<u8, V>>, D::Error>
where D: Deserializer<'de>, V: Deserialize<'de> {
    let values: BTreeMap<u8, Map<V>> = deserialize(deserializer)?;
    Ok(values.into_iter().map(|(key, value)| (key, value.0)).collect())
}
