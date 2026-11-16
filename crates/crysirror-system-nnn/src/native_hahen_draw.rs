// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native polygon requests become one shared bounded edge-walker batch. No
//! screen pixels, alpha or source storage are read by the HAHEN class itself.
use anyhow::Result;
use crysirror_image::{
    floating_polygon_fragments::FloatingPolygonFragments, integer_wipe::fragments::ImageFragment,
};
pub(crate) fn spec(paint: crate::native_hahen::Paint) -> Result<FloatingPolygonFragments> {
    let spec = FloatingPolygonFragments {
        fragments: paint
            .polygons
            .into_iter()
            .map(|p| ImageFragment {
                vertices: p.vertices,
                translation: p.destination_offset,
            })
            .collect(),
        weight256: paint.weight,
    };
    spec.validate()?;
    Ok(spec)
}
