//! Render-target clipping preserved when a subtree becomes a local snapshot.
use super::*;

pub(super) fn inverse_to_root(root: u32, owner: &BTreeMap<u32, &LayerSpec>) -> Result<Mat3> {
    let mut chain = Vec::new();
    let mut next = Some(root);
    while let Some(id) = next {
        ensure!(
            chain.len() < owner.len(),
            "viewport clip snapshot parent cycle"
        );
        let layer = owner
            .get(&id)
            .context("viewport clip snapshot parent missing")?;
        let (t,_) = layer.transform.resolved();
        ensure!(
            !t.camera_enabled
                && !t.billboard
                && t.rotate_x == 0.
                && t.rotate_y == 0.
                && t.matrix_is_2d(),
            "viewport clip snapshot requires a 2D root hierarchy"
        );
        chain.push(*layer);
        next = layer.parent;
    }
    let mut matrix = Mat3::IDENTITY;
    for layer in chain.into_iter().rev() {
        matrix = parent_basis(matrix, layer) * affine(layer);
    }
    let x = matrix.x_axis.truncate();
    let y = matrix.y_axis.truncate();
    let tolerance = x.length().max(y.length()).max(1.) * 1e-6;
    ensure!(
        (x.y.abs() <= tolerance && y.x.abs() <= tolerance)
            || (x.x.abs() <= tolerance && y.y.abs() <= tolerance),
        "rotated/sheared viewport clip snapshot requires polygon clipping"
    );
    ensure!(
        matrix.determinant().abs() > 1e-12,
        "viewport clip snapshot singular root"
    );
    Ok(matrix.inverse())
}
pub(super) fn rebase(layers: &mut [LayerSpec], inverse: Mat3) -> Result<()> {
    for layer in layers {
        if let Some(clip) = layer.viewport_clip {
            let clip = transformed_bounds(inverse, clip);
            ensure!(
                viewport_clip_valid(clip),
                "viewport clip snapshot transformed bounds"
            );
            layer.viewport_clip = Some(clip);
        }
    }
    Ok(())
}
