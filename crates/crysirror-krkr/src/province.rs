// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! LayerIntf LoadProvinceImage policy over shared integer-label resources.
//! Palette indices are interaction IDs, unrelated to visible image colours.
use crate::natives::{Rt, string, this_of, unsupported};
use crysirror_core::{
    path::{Mount, VfsPath},
    region_map::{RegionMap, RegionMapQuery, RegionMapState},
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};
pub(crate) struct Pending {
    target: ObjectHandle,
    size: [u32; 2],
    name: String,
    query: Box<dyn RegionMapQuery>,
}
fn clear(rt: &mut Rt, target: ObjectHandle) -> Result<()> {
    let owner = rt.host().client;
    let layer = rt.host_mut().layer_mut(target)?;
    layer.regions = RegionMap::new(owner, [layer.image_width as u32, layer.image_height as u32])
        .map_err(|e| TjsError::runtime(e.to_string()))?;
    layer.province_allocated = false;
    Ok(())
}
pub(crate) fn load(rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.loadProvinceImage";
    let target = this_of(rt, this, M)?;
    let result = begin(rt, target, args);
    if result.is_err() {
        clear(rt, target)?;
    }
    result
}
fn begin(rt: &mut Rt, target: ObjectHandle, args: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.loadProvinceImage";
    if !rt.host().can_suspend {
        return Err(unsupported(M, "a resumable label-image boundary"));
    }
    if rt.host().province_load.is_some() {
        return Err(TjsError::runtime("province load already pending"));
    }
    let name = string(&args, 0, M)?;
    let path = rt
        .host()
        .storage()?
        .resolve_with(&name, &[".png", ".bmp"])
        .ok_or_else(|| TjsError::runtime(format!("province storage {name:?} not found")))?;
    let layer = rt.host().layer(target)?;
    if !layer.has_image {
        return Err(TjsError::runtime(
            "province image requires a drawable Layer",
        ));
    }
    let size = [
        u32::try_from(layer.image_width).map_err(|_| TjsError::runtime("province width"))?,
        u32::try_from(layer.image_height).map_err(|_| TjsError::runtime("province height"))?,
    ];
    let images = rt
        .host()
        .images
        .as_ref()
        .ok_or_else(|| TjsError::runtime("image service absent"))?;
    let path = VfsPath::new(Mount::Asset, &path).map_err(|e| TjsError::runtime(e.to_string()))?;
    let query = images
        .begin_region_map(&path, Some(size))
        .map_err(|e| TjsError::runtime(e.to_string()))?;
    rt.host_mut().province_load = Some(Pending {
        target,
        size,
        name,
        query,
    });
    rt.request_suspend();
    Ok(Variant::Void)
}
pub(crate) fn poll(rt: &mut Rt) -> anyhow::Result<bool> {
    let Some(mut pending) = rt.host_mut().province_load.take() else {
        return Ok(true);
    };
    anyhow::ensure!(
        pending.query.client() == rt.host().client,
        "foreign province query"
    );
    match pending.query.poll()? {
        RegionMapState::Pending => {
            rt.host_mut().province_load = Some(pending);
            Ok(false)
        }
        RegionMapState::Failed(error) => {
            clear(rt, pending.target).map_err(|e| anyhow::anyhow!("{e}"))?;
            anyhow::bail!("Layer.loadProvinceImage({:?}): {error}", pending.name)
        }
        RegionMapState::Ready(map) => {
            if map.client() != rt.host().client || map.size() != pending.size {
                clear(rt, pending.target).map_err(|e| anyhow::anyhow!("{e}"))?;
                anyhow::bail!(
                    "Layer.loadProvinceImage({:?}): province size mismatch",
                    pending.name
                );
            }
            let layer = rt
                .host_mut()
                .layer_mut(pending.target)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            anyhow::ensure!(
                [layer.image_width as u32, layer.image_height as u32] == pending.size,
                "Layer changed during province load"
            );
            layer.regions = map;
            layer.province_allocated = true;
            layer.image_modified = true;
            Ok(true)
        }
    }
}
pub(crate) fn target(pending: &Pending) -> ObjectHandle {
    pending.target
}
