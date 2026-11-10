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
pub(super) fn configure(record: &mut FilterGpu, blend: Blend, origin: [f32; 2], opacity: u8) {
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
            M::OpaqueScreen255 => 17,
            M::OpaqueMultiply255 => 18,
            M::MaskSeparate256 => 19,
            M::MaskPiecewise256{..} => 20,
        };
        // Exact binary fraction; do not overfill the packed flag/opacity word.
        let kind=match mode {M::MaskPiecewise256{weight}=>id as f32+f32::from(weight)/512.,_=>id as f32};
        record.i = [
            kind,
            origin[0],
            origin[1],
            (u32::from(force_source_opaque)
                | (u32::from(force_output_opaque) << 1)
                | (u32::from(opacity) << 8)) as f32,
        ];
    }
}
