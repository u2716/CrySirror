//! Affine view composition independent of any layer store or renderer.
use crate::{Error, Result};
/// Compose a screen-space scale about `pivot` after an existing local affine
/// matrix and its separately applied anchor. Column-major output keeps that
/// anchor separate, so callers retain their original object position/pivot.
pub fn anchored_view_scale(
    matrix: Option<[f32; 16]>,
    anchor: [f32; 2],
    pivot: [f32; 2],
    scale: [f32; 2],
) -> Result<[f32; 16]> {
    let mut matrix = matrix.unwrap_or([
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ]);
    if !matrix
        .iter()
        .chain(anchor.iter())
        .chain(pivot.iter())
        .chain(scale.iter())
        .all(|v| v.is_finite())
        || matrix[3] != 0.
        || matrix[7] != 0.
        || matrix[11] != 0.
        || matrix[15] != 1.
    {
        return Err(Error::Invalid("anchored affine view transform"));
    }
    for axis in 0..2 {
        for column in 0..3 {
            matrix[column * 4 + axis] *= scale[axis];
        }
        matrix[12 + axis] = ((matrix[12 + axis] + anchor[axis] - pivot[axis]) * scale[axis]
            + pivot[axis])
            - anchor[axis];
    }
    if !matrix.iter().all(|v| v.is_finite()) {
        return Err(Error::Invalid("anchored affine view transform overflow"));
    }
    Ok(matrix)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_scale_keeps_anchor_separate_and_scales_the_complete_world_position() {
        let matrix = anchored_view_scale(None, [100., 50.], [640., 360.], [1.5, 1.5]).unwrap();
        for point in [[0., 0.], [1280., 0.], [0., 720.], [1280., 720.]] {
            for axis in 0..2 {
                let actual = [100., 50.][axis]
                    + matrix[axis] * point[0]
                    + matrix[4 + axis] * point[1]
                    + matrix[12 + axis];
                let world = [100., 50.][axis] + point[axis];
                let expected = (world - [640., 360.][axis]) * 1.5 + [640., 360.][axis];
                assert_eq!(actual, expected);
            }
        }
    }
    #[test]
    fn identity_preserves_existing_affine_shear_and_invalid_inputs_reject() {
        let matrix = [
            1., 2., 0., 0., 3., 4., 0., 0., 0., 0., 1., 0., 20., 30., 0., 1.,
        ];
        assert_eq!(
            anchored_view_scale(Some(matrix), [40., 60.], [80., 90.], [1., 1.]).unwrap(),
            matrix
        );
        let mut invalid = matrix;
        invalid[3] = 1.;
        assert!(anchored_view_scale(Some(invalid), [0.; 2], [0.; 2], [1.; 2]).is_err());
        assert!(anchored_view_scale(None, [f32::NAN, 0.], [0.; 2], [1.; 2]).is_err());
    }
}
