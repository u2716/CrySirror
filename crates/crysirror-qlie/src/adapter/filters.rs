//! Translate native resource-filter syntax into the shared image worker graph.
//! Original immutable inputs stay leased so removing/replacing a filter is
//! reversible; repeated filter edits do not accumulate decoded resources.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageProcessState};
fn constant_mode(value: &str) -> Result<crysirror_image::rgb_lut::ConstantByteBlend> {
    use crysirror_image::rgb_lut::ConstantByteBlend as B;
    // Native557BD8 ->55E8EC ->538138. Numeric script IDs differ from
    // internal MMX kernel IDs; native names are accepted at the same boundary.
    let id = crate::expression::parse_integer(value).unwrap_or(-1);
    Ok(match (id, value.to_ascii_lowercase().as_str()) {
        (0 | 1, _) | (_, "pass" | "normal") => B::Normal,
        (2, _) | (_, "screen") => B::Screen,
        (3, _) | (_, "multiple" | "multiply" | "modulate") => B::Multiply,
        (4, _) | (_, "difference") => B::Difference,
        (5, _) | (_, "lighten") => B::Lighten,
        (6, _) | (_, "darken") => B::Darken,
        (7, _) | (_, "add" | "lineardodge") => B::Add,
        (8, _) | (_, "sub") => B::Subtract,
        (9, _) | (_, "subnot" | "linearburn") => B::LinearBurn,
        (10, _) | (_, "exclusion") => B::Exclusion,
        (11, _) | (_, "overlay") => B::Overlay,
        (12, _) | (_, "hardlight") => B::HardLight,
        (13, _) | (_, "linearlight") => B::LinearLight,
        (14, _) | (_, "pinlight") => B::PinLight,
        _ => bail!("unsupported QLIE constant blend {value}"),
    })
}
impl QlieVm {
    pub(super) fn image_filter(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        index: usize,
        source: &str,
    ) -> Result<bool> {
        let o = &self.objects[id];
        let slot = if let Some(bundle) = &o.bundle {
            bundle.images.get(index).map(|v| v.slot)
        } else if index == 0 {
            Some(0)
        } else {
            None
        };
        let Some(slot) = slot.filter(|i| o.images.get(*i).is_some_and(Option::is_some)) else {
            ensure!(source.is_empty(), "QLIE filter input slot absent: {index}");
            return Ok(true);
        };
        if source.is_empty() {
            if let Some(original) = self.objects.get_mut(id).unwrap().originals.remove(&slot) {
                let old = self.objects.get_mut(id).unwrap().images[slot].replace(original);
                if let Some(old) = old {
                    h.release_image_resource(self.client, old.path.relative())?;
                }
            }
            return Ok(true);
        }
        let mut effects = Vec::new();
        for command in source.split('\\').filter(|v| !v.is_empty()) {
            let a = crate::program::arguments(command)?;
            let n = |i: usize, default: i32| -> Result<i32> {
                a.get(i)
                    .map(|v| {
                        crate::expression::parse_integer(v).context("QLIE resource filter integer")
                    })
                    .transpose()
                    .map(|v| v.unwrap_or(default))
            };
            match a[0].as_str() {
                "chroma" => effects.push(
                    json!({"kind":"hsl_modulate","hue":0,"saturation":n(1,0)?,"luminance":0}),
                ),
                "blur" => {
                    let x = n(1, 0)?;
                    let y = n(2, x)?;
                    ensure!(
                        (0..=128).contains(&x) && (0..=128).contains(&y),
                        "QLIE blur radius"
                    );
                    effects
                        .push(json!({"kind":"box_blur","area":[-x,-y,x,y],"alpha_weighted":false}));
                }
                "colorfill" => {
                    let color = n(1, 0)? as u32;
                    let mode = constant_mode(a.get(2).map(String::as_str).unwrap_or("1"))?;
                    let spec = crysirror_image::rgb_lut::RgbLookup::constant_byte256(
                        [
                            (color >> 16) as u8,
                            (color >> 8) as u8,
                            color as u8,
                            (color >> 24) as u8,
                        ],
                        mode,
                    );
                    effects.push(json!({"kind":"rgb_lookup","spec":spec}));
                }
                "flip" => effects
                    .push(json!({"kind":"flip","horizontal":n(1,0)?!=0,"vertical":n(2,0)?!=0})),
                "mirror" => effects
                    .push(json!({"kind":"flip","horizontal":n(1,0)?==1,"vertical":n(2,0)?==1})),
                "mosaic" | "mosic" => {
                    //55CBB4 averages all four stored bytes independently. It
                    //does nothing unless both cell axes are at least two.
                    let x=n(1,8)?;
                    let y=n(2,x)?;
                    if x>=2 && y>=2 { effects.push(json!({"kind":"block_average","cell":[x.min(1024),y.min(1024)]})); }
                }
                "canvas" => effects.push(json!({"kind":"canvas_margins","margins":[n(1,0)?,n(2,0)?,n(3,0)?,n(4,0)?],"fill":[136,136,136,0]})),
                _ => bail!("unsupported QLIE resource filter {command}"),
            }
        }
        if self.image_job.is_none() {
            let o = self.objects.get_mut(id).unwrap();
            let original = o
                .originals
                .entry(slot)
                .or_insert_with(|| o.images[slot].as_ref().unwrap().clone());
            let resource = h
                .published_image_resource(self.client, original.path.relative())?
                .context("QLIE immutable filter input lease")?;
            self.image_job = Some(h.image_metadata(self.client)?.begin_process(
                ImageInput::Resource(resource),
                serde_json::to_vec(&effects)?.into(),
            )?);
        }
        match self.image_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => bail!("QLIE resource filter worker: {error}"),
            ImageProcessState::Ready(image) => {
                self.image_job = None;
                self.next_image = self
                    .next_image
                    .checked_add(1)
                    .context("QLIE filter resource id")?;
                let published = h.publish_image_resource(self.client, self.next_image, image)?;
                let o = self.objects.get_mut(id).unwrap();
                let old = o.images[slot].replace(published).unwrap();
                if old.path != o.originals[&slot].path {
                    h.release_image_resource(self.client, old.path.relative())?;
                }
                Ok(true)
            }
        }
    }
}
