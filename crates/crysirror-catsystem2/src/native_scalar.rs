// SPDX-License-Identifier: MPL-2.0
//! KCS natives with no task or resource state, from the dispatch table at A2DD38.
//! Float arguments and results travel as raw f32 bits in the VM's dword slots.

/// `Some(result)` when `id` is one of these natives; `result` is its return slot.
pub(crate) fn apply(id: u32, arguments: &[u32]) -> Option<Option<u32>> {
    let float = |index: usize| arguments.get(index).copied().map(f32::from_bits);
    let unary = |op: fn(f32) -> f32| float(0).map(|value| Some(op(value).to_bits()));
    match id {
        // 659940: entry points this build compiled out.
        439 | 543..=548 | 550 | 552 | 556 | 559..=563 | 573 | 578 | 579 | 581 | 642..=644 | 652
        | 670..=673 | 686 | 687 | 974..=976 => Some(None),
        // 659950 and 654B60 report nothing available.
        549 | 551 | 554 | 555 | 575 | 580 => Some(Some(0)),
        // 64CA20
        925 => Some(Some(1)),
        // 64F560
        6 => arguments.first().map(|value| Some(*value)),
        // 659200
        874 => match arguments {
            [low, high, ..] => Some(Some((low & 0xffff) | (high << 16))),
            _ => None,
        },
        50 => unary(f32::sin),
        51 => unary(f32::cos),
        54 => unary(f32::acos),
        57 => unary(f32::abs),
        62 => unary(|value| value * std::f32::consts::PI / 180.),
        63 => unary(|value| value * 180. / std::f32::consts::PI),
        461 => unary(f32::log10),
        61 => match (float(0), float(1)) {
            (Some(base), Some(power)) => Some(Some(base.powf(power).to_bits())),
            _ => None,
        },
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::apply;
    #[test]
    fn scalar_natives_keep_their_return_slot_shape() {
        assert_eq!(apply(543, &[]), Some(None));
        assert_eq!(apply(551, &[]), Some(Some(0)));
        assert_eq!(apply(925, &[]), Some(Some(1)));
        assert_eq!(apply(874, &[0x1_2345, 2]), Some(Some(0x0002_2345)));
        assert_eq!(apply(62, &[180f32.to_bits()]), Some(Some(std::f32::consts::PI.to_bits())));
        assert_eq!(apply(61, &[2f32.to_bits(), 3f32.to_bits()]), Some(Some(8f32.to_bits())));
        assert_eq!(apply(61, &[2f32.to_bits()]), None);
        assert_eq!(apply(100_000, &[]), None);
    }
}
