// SPDX-License-Identifier: MPL-2.0
//! Destination-reading byte kernels share the image worker's colour contract.
use super::*;
pub(super) fn uses_stored(p: &Primitive) -> bool {
    matches!(p.blend, Blend::StoredByte { .. })
        || match &p.paint {
            Paint::ProjectedMesh { frame, .. } => frame
                .parts
                .iter()
                .any(|part| matches!(part.blend, Some(Blend::StoredByte { .. }))),
            _ => false,
        }
}
pub(super) fn configure(record: &mut FilterGpu, blend: Blend, origin: [f32;2]) {
    if let Blend::StoredByte {
        mode,
        force_source_opaque,
        force_output_opaque,
    } = blend
    {
        use crysirror_image::rgb_lut::ConstantByteBlend as M;
        let id = match mode {
            M::Normal => 1,
            M::Screen => 2,
            M::Multiply => 3,
            M::Difference => 4,
            M::Lighten => 5,
            M::Darken => 6,
            M::Add => 7,
            M::Subtract => 8,
            M::LinearBurn => 9,
            M::Overlay => 10,
            M::HardLight => 11,
            M::Exclusion => 12,
            M::LinearLight => 13,
            M::PinLight => 14,
            M::Scalar256 => 15,
            M::OpaqueScalar256 => 16,
        };
        record.i = [
            id as f32,
            origin[0],origin[1],
            (u32::from(force_source_opaque)|(u32::from(force_output_opaque)<<1)) as f32,
        ];
    }
}
