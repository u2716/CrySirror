// SPDX-License-Identifier: MPL-2.0
//! JSON cannot represent IEEE nonfinite values. Preserve their original bits
//! explicitly; ordinary finite queue data keeps its existing numeric ABI.
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeSeq};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bits {
    ieee754: u32,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Scalar {
    Number(f32),
    Bits(Bits),
}
pub(super) fn serialize<S: Serializer>(values: &[f32], serializer: S) -> Result<S::Ok, S::Error> {
    let mut sequence = serializer.serialize_seq(Some(values.len()))?;
    for value in values {
        if value.is_finite() {
            sequence.serialize_element(value)?;
        } else {
            sequence.serialize_element(&Bits {
                ieee754: value.to_bits(),
            })?;
        }
    }
    sequence.end()
}
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<f32>, D::Error> {
    Ok(Vec::<Scalar>::deserialize(deserializer)?
        .into_iter()
        .map(|value| match value {
            Scalar::Number(value) => value,
            Scalar::Bits(value) => f32::from_bits(value.ieee754),
        })
        .collect())
}
pub(super) mod optional {
    use super::*;
    struct Values<'a>(&'a [f32]);
    impl Serialize for Values<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::serialize(self.0, serializer)
        }
    }
    pub(in crate::layer::queued) fn serialize<S: Serializer>(
        values: &Option<Vec<f32>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match values {
            Some(values) => serializer.serialize_some(&Values(values)),
            None => serializer.serialize_none(),
        }
    }
    pub(in crate::layer::queued) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<f32>>, D::Error> {
        Ok(
            Option::<Vec<Scalar>>::deserialize(deserializer)?.map(|values| {
                values
                    .into_iter()
                    .map(|value| match value {
                        Scalar::Number(value) => value,
                        Scalar::Bits(value) => f32::from_bits(value.ieee754),
                    })
                    .collect()
            }),
        )
    }
}
