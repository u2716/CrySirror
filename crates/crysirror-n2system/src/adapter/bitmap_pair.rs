// SPDX-License-Identifier: MPL-2.0
//! CreateDifferenceTexture40C800 copies source RGBA only at nontransparent
//! pixels whose RGB differs from the reference. The shared image worker owns
//! the pixel comparison. CreateAnimation40B6A0/4915F0 only preloads a bitmap:
//! its parsed name/priority/coordinates are never installed in the native tree.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
pub(super) struct Difference {
    first: NativeBitmap,
    second: NativeBitmap,
    job: Option<ImageJob>,
}
impl N2SystemVm {
    pub(super) fn native_difference(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(
            args.len() == 6,
            "N2System CreateDifferenceTexture signature"
        );
        if state.difference.is_none() {
            let Some(first) = self.read_native_bitmap(host, current, &args.string(4)?)? else {
                return Ok(false);
            };
            let Some(second) = self.read_native_bitmap(host, current, &args.string(5)?)? else {
                return Ok(false);
            };
            state.difference = Some(Difference {
                first,
                second,
                job: None,
            });
        }
        let preparing = state.difference.as_mut().unwrap();
        if preparing.job.is_none() {
            // Both requests are admitted before yielding; live target inputs
            // therefore freeze at the same presenter fence.
            let first = preparing.first.poll(
                self.client,
                self.pool.as_ref().unwrap(),
                self.images.as_ref().unwrap(),
            )?;
            let second = preparing.second.poll(
                self.client,
                self.pool.as_ref().unwrap(),
                self.images.as_ref().unwrap(),
            )?;
            let (Some(first), Some(second)) = (first, second) else {
                return Ok(false);
            };
            if first.size() != second.size() {
                // Original40C800 releases its temporaries and returns without
                // registering the requested name for unequal bitmap extents.
                state.difference = None;
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            let input = ImageInput::Raster {
                destination: Box::new(ImageInput::Resource(first)),
                source: Box::new(ImageInput::Resource(second)),
            };
            let effects = Arc::from(
                &b"[{\"kind\":\"byte_raster\",\"spec\":{\"operation\":\"rgb_difference_cutout\"}}]"
                    [..],
            );
            match self.images.as_ref().unwrap().begin_process(input, effects) {
                Ok(job) => preparing.job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                Err(e) => return Err(e.into()),
            }
        }
        let image = match preparing.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(error) => bail!("N2System difference bitmap: {error}"),
            ImageProcessState::Ready(image) => image,
        };
        let size = image.size();
        let id = self.visual_owned(
            host,
            current,
            args,
            json!({"kind":"image","asset":""}),
            size,
            None,
            Some(image),
        )?;
        self.bitmap_formats.insert(
            id,
            bitmap_read::BitmapFormat {
                origin: None,
                alpha: Some(true),
            },
        );
        state.difference = None;
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn native_animation_preload(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 5, "N2System CreateAnimation signature");
        if state.animation_bitmap.is_none() {
            let source = args.string(4)?;
            ensure!(
                source.eq_ignore_ascii_case("SCREEN")
                    || resource::resolve(self.catalog.as_ref().unwrap(), &source).is_ok(),
                "N2System CreateAnimation requires a native image source"
            );
            let Some(read) = self.read_native_bitmap(host, current, &source)? else {
                return Ok(false);
            };
            state.animation_bitmap = Some(read);
        }
        let Some(image) = state.animation_bitmap.as_mut().unwrap().poll(
            self.client,
            self.pool.as_ref().unwrap(),
            self.images.as_ref().unwrap(),
        )?
        else {
            return Ok(false);
        };
        let budget = if cfg!(target_os = "vita") {
            24 * 1024 * 1024
        } else {
            96 * 1024 * 1024
        };
        let bytes = |image: &Arc<dyn ImageResource>| {
            let [w, h] = image.size();
            u64::from(w) * u64::from(h) * 4
        };
        ensure!(
            self.animation_preloads.len() < 1024
                && self.animation_preloads.iter().map(bytes).sum::<u64>() + bytes(&image) <= budget,
            "N2System CreateAnimation preload budget"
        );
        self.animation_preloads.push(image);
        state.animation_bitmap = None;
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
