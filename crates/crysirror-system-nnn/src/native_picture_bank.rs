// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Immutable native bank identity plus per-frame semantic geometry. Image
//! generations are engine leases; native frame selection never copies pixels.
use crate::{
    image_bridge::PreparedPicture, native_geometry::NativeRects, native_move::NativeMoveState,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct BankPicture {
    pub load_ok: bool,
    pub picture: PreparedPicture,
    pub rectangles: NativeRects,
    pub assigned: bool,
}
#[derive(Clone, Default)]
pub(crate) struct FramePictureBank {
    pub pictures: Arc<BTreeMap<i32, BankPicture>>,
    pub geometry: Option<Arc<BTreeMap<i32, NativeMoveState>>>,
}
impl FramePictureBank {
    pub(crate) fn resolve(&mut self, geometry: &BTreeMap<i32, NativeMoveState>) {
        self.geometry = Some(Arc::new(geometry.clone()));
    }
    pub(crate) fn picture(&self, slot: i32) -> Option<&BankPicture> {
        self.pictures.get(&slot)
    }
    pub(crate) fn rectangles(&self, slot: i32) -> Option<&NativeRects> {
        self.geometry
            .as_ref()
            .and_then(|all| all.get(&slot))
            .map(|state| &state.rectangles)
            .or_else(|| self.pictures.get(&slot).map(|p| &p.rectangles))
    }
    pub(crate) fn animation_picture(
        &self,
        slot: i32,
    ) -> Option<crate::native_picture_bank_animation::BankAnimationPicture> {
        let p = self.picture(slot)?;
        if !p.load_ok { return None; }
        let rect = self.rectangles(slot)?;
        Some(crate::native_picture_bank_animation::BankAnimationPicture {
            raw: p.picture.raw.clone(),
            opaque: p.picture.opaque.clone(),
            keyed: p.picture.keyed.clone(),
            has_mask: p.picture.has_mask,
            source_clip: p.picture.source_clip,
            // CAllEffect::GetSrcPoint returns0,0 even when EFFECT.src contains
            // other words. GetDstSize returns dst.right/bottom as extents.
            source: [0, 0, rect.destination[2], rect.destination[3]],
            destination: [rect.destination[0], rect.destination[1]],
        })
    }
}
