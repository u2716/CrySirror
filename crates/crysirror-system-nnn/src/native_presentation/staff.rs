// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Retained CPicture cache mutations become visible in original painter order.
//! Only scalar metadata and scoped image leases cross this native boundary.
use super::*;
use crate::native_picture_cache::PictureCache;
impl NativeFrameWork {
    pub(super) fn begin_staff_pages(
        &mut self,
        frame: &crate::native_picture_effect::PictureEffectFrame,
        p: [i32; 16],
    ) -> Result<crate::native_staff_pages::Pages> {
        let pictures = frame
            .bank
            .pictures
            .iter()
            .map(|(s, p)| (*s, p.picture.clone()))
            .collect();
        let bank = crate::native_staff_bank::snapshot(
            self.client,
            frame.bank_limit,
            &pictures,
            &self.buffers.staff_bank,
        )?;
        for slot in 0..frame.bank_limit {
            self.buffers.staff_geometry.insert(
                slot,
                frame
                    .bank
                    .rectangles(slot)
                    .cloned()
                    .unwrap_or_else(|| crate::native_geometry::NativeRects::natural(frame.canvas)),
            );
        }
        let context = self
            .buffers
            .staff_context
            .clone()
            .context("NNN staff resource context absent")?;
        crate::native_staff_pages::Pages::begin(
            context,
            frame.slot as u64 + 1,
            p,
            frame.slot,
            frame.bank_limit,
            frame.geometry_assigned,
            [
                frame.rectangles.destination[0],
                frame.rectangles.destination[1],
            ],
            frame.canvas,
            bank,
            self.current.as_ref().unwrap().clone(),
        )
    }
    pub(super) fn staff_underlay(
        &self,
        slot: i32,
        layer: crysirror_core::image_processing::ImageCompositeLayer,
    ) -> Result<Option<crysirror_core::image_processing::ImageCompositeLayer>> {
        if !self.buffers.staff_touched.contains(&slot) {
            return Ok(Some(layer));
        }
        let cache = self
            .buffers
            .staff_bank
            .get(&slot)
            .context("NNN changed underlay cache absent")?;
        if !cache.load_ok {
            return Ok(None);
        }
        let Some(p) = &cache.storage else {
            return Ok(None);
        };
        let r = self
            .buffers
            .staff_geometry
            .get(&slot)
            .context("NNN changed underlay geometry absent")?;
        let [x, y, w, h] = r.destination;
        let [sx, sy, _, _] = r.source;
        let Some((source, at)) = crysirror_image::coverage_bounds::intersect_source_window(
            [sx, sy, w, h],
            [x, y],
            p.source_clip,
        )?
        else {
            return Ok(None);
        };
        let image = if matches!(
            layer.math,
            crysirror_core::image_processing::ImageCompositeMath::CopyRgb
        ) {
            p.opaque.clone()
        } else if cache.mask_exists {
            p.raw.clone()
        } else {
            p.keyed.clone()
        };
        Ok(Some(
            crysirror_core::image_processing::ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(image)),
                source,
                destination: at,
                ..layer
            },
        ))
    }
    pub(super) fn refresh_staff_effect(&self, effect: &mut NativeFrameEffect) -> Result<()> {
        if self.buffers.staff_touched.is_empty() {
            return Ok(());
        }
        let cache = |slot: i32| {
            self.buffers
                .staff_touched
                .contains(&slot)
                .then(|| self.buffers.staff_bank.get(&slot))
                .flatten()
        };
        let opaque = |p: &PictureCache<crate::image_bridge::NativePictureViews>| {
            p.storage.as_ref().map(|v| v.opaque.clone())
        };
        let masked = |p: &PictureCache<crate::image_bridge::NativePictureViews>| {
            p.storage.as_ref().map(|v| {
                if p.mask_exists {
                    v.raw.clone()
                } else {
                    v.keyed.clone()
                }
            })
        };
        let move_frame = |frame: &mut MoveFrame| {
            if let Some(p) = cache(frame.slot) {
                frame.source_clip = p.storage.as_ref().map(|v| v.source_clip);
                frame.source = if !p.load_ok || !frame.geometry_assigned {
                    None
                } else if matches!(
                    frame.math,
                    crysirror_core::image_processing::ImageCompositeMath::CopyRgb
                ) {
                    opaque(p)
                } else {
                    masked(p)
                };
            }
        };
        match effect {
            NativeFrameEffect::Move(frame) => move_frame(frame),
            NativeFrameEffect::Scroll(frame) => {
                move_frame(&mut frame.normal);
                if let Some(p) = cache(frame.slot) {
                    frame.frame.source = opaque(p);
                }
            }
            NativeFrameEffect::Shake(frame) => {
                if let Some(p) = cache(frame.slot) {
                    if frame.picture.is_some() {
                        frame.picture = opaque(p);
                    }
                }
            }
            NativeFrameEffect::Lamp(frame) => {
                if let Some(p) = cache(frame.slot) {
                    if frame.source.is_some() {
                        frame.source = p.storage.as_ref().map(|v| v.keyed.clone());
                    }
                }
            }
            NativeFrameEffect::Chara(frame) => {
                if let Some(p) = self.current_slot.and_then(cache) {
                    if frame.source.is_some() {
                        frame.source = p.storage.as_ref().map(|v| {
                            if p.mask_exists {
                                v.raw.clone()
                            } else {
                                v.opaque.clone()
                            }
                        });
                    }
                }
            }
            NativeFrameEffect::Fusuma(frame) => {
                if let Some(p) = self.current_slot.and_then(cache) {
                    if frame.first.is_some() {
                        frame.first = opaque(p);
                    }
                }
                if let Some(p) = cache(frame.state.picture_slot()) {
                    frame.second = opaque(p);
                }
            }
            NativeFrameEffect::Picture(frame) => {
                let mut bank = (*frame.bank.pictures).clone();
                for slot in &self.buffers.staff_touched {
                    let Some(p) = cache(*slot) else {
                        continue;
                    };
                    let previous = bank.get(slot);
                    let assigned = previous.is_some_and(|p| p.assigned);
                    let rectangles = frame.bank.rectangles(*slot).cloned().unwrap_or_else(|| {
                        crate::native_geometry::NativeRects::natural(frame.canvas)
                    });
                    if let Some(picture) = crate::native_staff_bank::prepared(
                        *slot,
                        p,
                        previous.map(|p| &p.picture),
                        *slot < 4 || *slot == 14,
                    ) {
                        bank.insert(
                            *slot,
                            crate::native_picture_bank::BankPicture {
                                load_ok: p.load_ok,
                                picture,
                                rectangles,
                                assigned,
                            },
                        );
                    }
                }
                frame.bank.pictures = Arc::new(bank);
                if let Some(p) = cache(frame.slot) {
                    let admitted = frame.geometry_assigned
                        || matches!(
                            frame.state.command(),
                            62 | 72
                                | 77
                                | 94
                                | 96
                                | 104
                                | 106
                                | 107
                                | 110
                                | 119
                                | 121
                                | 123
                                | 125
                                | 126
                                | 131
                                | 132
                                | 133
                        );
                    frame.has_mask = admitted && p.mask_exists;
                    frame.source_clip = if admitted {
                        p.storage.as_ref().map(|v| v.source_clip)
                    } else {
                        None
                    };
                    let uses_mask = frame.source_uses_mask;
                    let force_opaque = !uses_mask
                        || ((matches!(frame.state.command(), 65 | 124 | 125 | 127 | 131 | 136)
                            || matches!(frame.state.kind,crate::native_picture_effect::PictureEffectKind::Koma{parameters} if parameters[8]==2))
                            && !p.mask_exists);
                    frame.source = if !admitted {
                        None
                    } else if force_opaque {
                        opaque(p)
                    } else {
                        masked(p)
                    };
                    if let Some(normal) = frame.normal.as_mut() {
                        move_frame(normal);
                    }
                }
                if let Some((slot, mask)) = frame.state.companion_slot(frame.bank_limit as u16) {
                    if let Some(p) = cache(slot) {
                        frame.second_clip = p.storage.as_ref().map(|v| v.source_clip);
                        frame.second = if mask && !p.mask_exists {
                            None
                        } else if mask {
                            p.storage.as_ref().map(|v| v.raw.clone())
                        } else {
                            opaque(p)
                        };
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
