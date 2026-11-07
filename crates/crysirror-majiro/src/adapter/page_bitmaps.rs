// Copyright (c) 2026 CrySirror contributors. MIT.
//! 419500/4196E0/41A630/41AC30: native page and mask ownership over immutable
//! engine bitmaps. Shared workers copy stored bytes, not rendered screenshots.
//! An indexed allocation has440610's identity palette; palette mutation is a
//! separate, currently unadmitted native operation. A linked alpha page stays
//! live: writes to its indices atomically refresh every dependent projection.
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState, ImageResource,
};
use std::sync::{Arc, Weak};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Alpha {
    None,
    Inline,
    Linked(u32),
}
#[derive(Clone)]
pub(super) struct Bitmap {
    pub image: Arc<dyn ImageResource>,
    pub indexed: bool,
    pub alpha: Alpha,
}
struct Output {
    id: Option<u32>,
    indexed: bool,
    alpha: Alpha,
    job: ImageJob,
    ready: Option<Arc<dyn ImageResource>>,
}
pub(super) struct PendingBitmap {
    event: Event,
    outputs: Vec<Output>,
    _locks: Vec<Arc<()>>,
}
fn channel(destination: ImageInput, source: ImageInput, from: u8, to: u8) -> Result<ImageInput> {
    let effect = crysirror_image::effects::ImageEffect::CopyChannel {
        source_channel: from,
        destination_channels: to,
        invert: true,
    };
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(destination),
            source: Box::new(source),
        }),
        effects: serde_json::to_vec(&[effect])?.into(),
    })
}
impl MajiroVm {
    pub(super) fn page_available(&mut self, id: u32) -> Result<()> {
        if self.page_busy.get(&id).and_then(Weak::upgrade).is_some() {
            bail!(
                "Majiro page {id} has an unfinished native bitmap operation; overlapping frame mutation requires ordered admission"
            )
        }
        self.page_busy.remove(&id);
        Ok(())
    }
    fn bitmap_input(
        &self,
        id: u32,
        overrides: &BTreeMap<u32, ImageInput>,
        link_override: Option<(u32, u32)>,
    ) -> Result<ImageInput> {
        let bitmap = self
            .page_bitmaps
            .get(&id)
            .context("Majiro missing or foreign bitmap")?;
        let input = overrides
            .get(&id)
            .cloned()
            .unwrap_or_else(|| ImageInput::Resource(bitmap.image.clone()));
        let alpha = if let Some((_, mask)) = link_override.filter(|(target, _)| *target == id) {
            Alpha::Linked(mask)
        } else {
            bitmap.alpha
        };
        if let Alpha::Linked(mask) = alpha {
            let mask_bitmap = self
                .page_bitmaps
                .get(&mask)
                .context("Majiro missing linked alpha page")?;
            ensure!(
                mask_bitmap.indexed && mask_bitmap.alpha == Alpha::None,
                "Majiro linked alpha page must be an unmasked identity-index plane"
            );
            let mask = overrides
                .get(&mask)
                .cloned()
                .unwrap_or_else(|| ImageInput::Resource(mask_bitmap.image.clone()));
            channel(input, mask, 0, 8)
        } else {
            Ok(input)
        }
    }
    pub(super) fn native_bitmap(
        &mut self,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        let Event::Native {
            hash,
            arguments: a,
            returns,
            ..
        } = event
        else {
            bail!("Majiro bitmap event")
        };
        if let Some(pending) = &self.bitmap_work {
            ensure!(&pending.event == event, "Majiro stale bitmap completion");
        } else {
            let metadata = host.image_metadata(self.client).map_err(core_error)?;
            let mut specs = Vec::new();
            let mut dependencies = Vec::new();
            if matches!(*hash, 0x83d81f59 | 0xf94b3586) {
                ensure!(matches!(a.len(), 2 | 3), "Majiro page_create ABI");
                let size = [a[0].integer()?.max(1) as u32, a[1].integer()?.max(1) as u32];
                ensure!(
                    size.iter().all(|v| *v <= 8192),
                    "Majiro page geometry profile"
                );
                ensure!(
                    self.pages.len() < 256 && self.next_page <= i32::MAX as u32,
                    "Majiro page handle budget"
                );
                let indexed = a.len() == 3 && a[2].integer()? == 0;
                //Native4196E0 allocates a zero-index mask: coverage=255-index.
                //Nested indexed+mask pages are not used as native mask inputs.
                let alpha = if *hash == 0xf94b3586 {
                    Alpha::Inline
                } else {
                    Alpha::None
                };
                let input = ImageInput::Solid {
                    size,
                    color: [0, 0, 0, 255],
                };
                specs.push((None, indexed, alpha, input));
            } else {
                let mut overrides = BTreeMap::new();
                let mut changed = Vec::new();
                let mut link_override = None;
                match *hash {
                    0xd01be374 => {
                        ensure!(a.len() == 6, "Majiro grp_fill ABI");
                        let values = a.iter().map(Value::integer).collect::<Result<Vec<_>>>()?;
                        let id = u32::try_from(values[0]).context("Majiro fill page handle")?;
                        let bitmap = self
                            .page_bitmaps
                            .get(&id)
                            .context("Majiro fill page missing or foreign")?;
                        ensure!(
                            values[1..5].iter().all(|v| v.unsigned_abs() <= 1_000_000),
                            "Majiro fill rectangle profile"
                        );
                        if values[3] <= 0 || values[4] <= 0 {
                            return Ok(Some(if *returns {
                                vec![Value::Integer(0)]
                            } else {
                                vec![]
                            }));
                        }
                        let word = values[5] as u32;
                        let color = if bitmap.indexed {
                            [word as u8, word as u8, word as u8, 255]
                        } else {
                            [word as u8, (word >> 8) as u8, (word >> 16) as u8, 255]
                        };
                        //419C40/419AE0 clips to the page and fills colour only.
                        //Indexed pages use the low byte, true-colour uses
                        //COLORREF RGB. Even an alpha-bearing destination keeps
                        //its separate mask. Borrowers of an index mask refresh
                        //through the same shared graph below.
                        let input = ImageInput::Composite {
                            base: Box::new(self.bitmap_input(id, &overrides, None)?),
                            layers: vec![ImageCompositeLayer {
                                input: Box::new(ImageInput::Solid {
                                    size: bitmap.image.size(),
                                    color,
                                }),
                                source: [values[1], values[2], values[3], values[4]],
                                destination: [values[1], values[2]],
                                clip: None,
                                opacity: 255,
                                math: ImageCompositeMath::CopyRgb,
                            }],
                        };
                        overrides.insert(id, input);
                        changed.push(id);
                        dependencies.push(id);
                    }
                    0x56bbba3a => {
                        ensure!(a.len() == 8, "Majiro grp_copy ABI");
                        let v = a.iter().map(Value::integer).collect::<Result<Vec<_>>>()?;
                        let src = u32::try_from(v[0]).context("Majiro copy source handle")?;
                        let dst = u32::try_from(v[5]).context("Majiro copy destination handle")?;
                        let source = self
                            .page_bitmaps
                            .get(&src)
                            .context("Majiro copy source missing or foreign")?;
                        let destination = self
                            .page_bitmaps
                            .get(&dst)
                            .context("Majiro copy destination missing or foreign")?;
                        ensure!(
                            !destination.indexed || source.indexed,
                            "Majiro true-colour to indexed copy has no native conversion"
                        );
                        ensure!(
                            v[1..5]
                                .iter()
                                .chain(v[6..8].iter())
                                .all(|v| v.unsigned_abs() <= 1_000_000),
                            "Majiro bitmap coordinate profile"
                        );
                        //440800 returns without writing if the clipped rectangle
                        //is empty; shared composition preserves pixel mapping.
                        if v[3] <= 0 || v[4] <= 0 {
                            return Ok(Some(if *returns {
                                vec![Value::Integer(0)]
                            } else {
                                vec![]
                            }));
                        }
                        dependencies.extend([src, dst]);
                        for bitmap in [source, destination] {
                            if let Alpha::Linked(mask) = bitmap.alpha {
                                dependencies.push(mask);
                            }
                        }
                        let input = ImageInput::Composite {
                            //447AE0 uses memcpy for same-row aliases, including
                            //overlap where C leaves the result undefined. Freeze
                            //the original source so self-copy is deterministic.
                            base: Box::new(self.bitmap_input(dst, &overrides, None)?),
                            layers: vec![ImageCompositeLayer {
                                input: Box::new(self.bitmap_input(src, &overrides, None)?),
                                source: [v[1], v[2], v[3], v[4]],
                                destination: [v[6], v[7]],
                                clip: None,
                                opacity: 255,
                                //447AE0 copies colour, then the source mask only
                                //when both pictures own/borrow one. Never blend.
                                math: if source.alpha != Alpha::None
                                    && destination.alpha != Alpha::None
                                {
                                    ImageCompositeMath::Copy
                                } else {
                                    ImageCompositeMath::CopyRgb
                                },
                            }],
                        };
                        if source.alpha != Alpha::None {
                            if let Alpha::Linked(mask) = destination.alpha {
                                let original = self
                                    .page_bitmaps
                                    .get(&mask)
                                    .context("Majiro copy destination mask absent")?;
                                overrides.insert(
                                    mask,
                                    channel(
                                        ImageInput::Resource(original.image.clone()),
                                        input.clone(),
                                        3,
                                        7,
                                    )?,
                                );
                                changed.push(mask);
                            }
                        }
                        overrides.insert(dst, input);
                        changed.push(dst);
                    }
                    0xf9705332 => {
                        ensure!(a.len() == 2, "Majiro SetAlphaPage ABI");
                        let target =
                            u32::try_from(a[0].integer()?).context("Majiro alpha target handle")?;
                        let mask =
                            u32::try_from(a[1].integer()?).context("Majiro alpha plane handle")?;
                        let dst = self
                            .page_bitmaps
                            .get(&target)
                            .context("Majiro alpha target missing or foreign")?;
                        let src = self
                            .page_bitmaps
                            .get(&mask)
                            .context("Majiro alpha plane missing or foreign")?;
                        ensure!(
                            !dst.indexed && src.indexed && src.alpha == Alpha::None,
                            "Majiro SetAlphaPage requires true-colour target and unmasked identity-index plane"
                        );
                        ensure!(
                            dst.image.size() == src.image.size(),
                            "Majiro alpha plane dimensions differ"
                        );
                        link_override = Some((target, mask));
                        changed.push(target);
                        dependencies.extend([target, mask]);
                    }
                    _ => bail!("Majiro unsupported bitmap operation"),
                }
                //Index writes affect every borrower. Include its current mask
                //and colour dependencies in the retained operation's admission.
                for (id, bitmap) in &self.page_bitmaps {
                    if let Alpha::Linked(mask) = bitmap.alpha {
                        if changed.contains(&mask) && !changed.contains(id) {
                            changed.push(*id);
                        }
                    }
                }
                changed.sort_unstable();
                changed.dedup();
                for id in changed {
                    let bitmap = &self.page_bitmaps[&id];
                    let alpha = link_override
                        .filter(|(target, _)| *target == id)
                        .map_or(bitmap.alpha, |(_, mask)| Alpha::Linked(mask));
                    dependencies.push(id);
                    if let Alpha::Linked(mask) = alpha {
                        dependencies.push(mask);
                    }
                    specs.push((
                        Some(id),
                        bitmap.indexed,
                        alpha,
                        self.bitmap_input(id, &overrides, link_override)?,
                    ));
                }
            }
            dependencies.sort_unstable();
            dependencies.dedup();
            for id in &dependencies {
                self.page_available(*id)?;
            }
            let mut outputs = Vec::new();
            for (id, indexed, alpha, input) in specs {
                let job = metadata
                    .begin_process(input, b"[]".as_slice().into())
                    .map_err(core_error)?;
                outputs.push(Output {
                    id,
                    indexed,
                    alpha,
                    job,
                    ready: None,
                });
            }
            let mut locks = Vec::new();
            for id in dependencies {
                let lock = Arc::new(());
                self.page_busy.insert(id, Arc::downgrade(&lock));
                locks.push(lock);
            }
            self.bitmap_work = Some(PendingBitmap {
                event: event.clone(),
                outputs,
                _locks: locks,
            });
            return Ok(None);
        }
        let pending = self.bitmap_work.as_mut().unwrap();
        let mut waiting = false;
        for output in &mut pending.outputs {
            if output.ready.is_some() {
                continue;
            }
            match output.job.poll().map_err(core_error)? {
                ImageProcessState::Pending => waiting = true,
                ImageProcessState::Failed(error) => bail!("Majiro shared bitmap worker: {error}"),
                ImageProcessState::Ready(image) => output.ready = Some(image),
            }
        }
        if waiting {
            return Ok(None);
        }
        let mut canvases = Vec::new();
        let mut layers = Vec::new();
        let mut uploads: Vec<(u32, String, Bitmap)> = Vec::new();
        for output in &pending.outputs {
            let image = output.ready.as_ref().unwrap().clone();
            let id = output.id.unwrap_or(self.next_page);
            if output.id.is_none() {
                ensure!(
                    self.pages.len() < 256 && id <= i32::MAX as u32,
                    "Majiro bitmap completion page budget"
                );
                let size = image.size();
                canvases.push(json!({"op":"create","id":id,"width":size[0],"height":size[1]}));
            }
            let uploaded = match host.publish_image_resource(self.client, id, image.clone()) {
                Ok(upload) => upload,
                Err(error) => {
                    for (_, path, _) in &uploads {
                        let _ = host.release_image_resource(self.client, path);
                    }
                    return Err(core_error(error));
                }
            };
            let path = uploaded.path.relative().to_owned();
            let size = image.size();
            canvases.push(json!({"op":"set","id":id,"layer":{"id":0,"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":path}}}));
            for (sprite_id, sprite) in &self.sprites {
                if sprite.page == id {
                    layers.push(json!({"op":"set_paint","id":sprite_id,"paint":{"kind":"image","asset":path}}));
                }
            }
            uploads.push((
                id,
                path,
                Bitmap {
                    image,
                    indexed: output.indexed,
                    alpha: output.alpha,
                },
            ));
        }
        let batch = json!({"canvases":canvases,"layers":layers});
        if let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?) {
            for (_, path, _) in &uploads {
                let _ = host.release_image_resource(self.client, path);
            }
            return Err(core_error(error));
        }
        let creating = pending.outputs.iter().any(|output| output.id.is_none());
        let result = if creating { self.next_page as i32 } else { 0 };
        for (id, path, bitmap) in uploads {
            self.pages.insert(id, bitmap.image.size());
            self.page_bitmaps.insert(id, bitmap);
            if let Some(previous) = self.page_resources.insert(id, path) {
                host.release_image_resource(self.client, &previous)
                    .map_err(core_error)?;
            }
        }
        if creating {
            self.next_page += 1;
        }
        self.api_calls += 1;
        self.bitmap_work = None;
        self.page_busy.retain(|_, lock| lock.strong_count() != 0);
        Ok(Some(if *returns {
            vec![Value::Integer(result)]
        } else {
            vec![]
        }))
    }
}
