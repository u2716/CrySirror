// SPDX-License-Identifier: MPL-2.0
//! CreateFrame40CC50 builds a colour/alpha127 rectangle padded by four pixels,
//! runs four feedback-blur passes4181F0 (clipped edge neighborhoods), then
//! copies the source at8,8. Native4464D0 subtracts8 from constructor positions.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
};

pub(super) struct Preparing {
    read: NativeBitmap,
    job: Option<ImageJob>,
}
impl N2SystemVm {
    pub(super) fn native_frame_image(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 6, "N2System CreateFrame signature");
        if state.frame_image.is_none() {
            let Some(read) = self.read_native_bitmap(host, current, &args.string(5)?)? else {
                return Ok(false);
            };
            state.frame_image = Some(Preparing { read, job: None });
        }
        let preparation = state.frame_image.as_mut().unwrap();
        if preparation.job.is_none() {
            let Some(image) = preparation.read.poll(
                self.client,
                self.pool.as_ref().unwrap(),
                self.images.as_ref().unwrap(),
            )?
            else {
                return Ok(false);
            };
            let [w, h] = image.size();
            let source = ImageInput::Resource(image);
            let padded = w;
            let mut color = args.color(4)?;
            color[3] = 127;
            let mut clear = color;
            clear[3] = 0;
            let effects = serde_json::to_vec(&json!([
                {"kind":"canvas_margins","margins":[4,4,4,4],"fill":clear},
                {"kind":"feedback_box_blur","spec":{"passes":4,"clipped_edges":true}}
            ]))?;
            let base = ImageInput::Process {
                input: Box::new(ImageInput::Solid {
                    size: [padded + 8, h + 8],
                    color,
                }),
                effects: Arc::from(effects),
            };
            let input = ImageInput::Composite {
                base: Box::new(base),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(source),
                    source: [0, 0, padded as i32, h as i32],
                    destination: [8, 8],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Copy,
                }],
            };
            match self
                .images
                .as_ref()
                .unwrap()
                .begin_process(input, Arc::from(&b"[]"[..]))
            {
                Ok(job) => preparation.job = Some(job),
                Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                Err(e) => return Err(e.into()),
            }
        }
        let image = match preparation.job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(error) => bail!("N2System CreateFrame: {error}"),
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
        let mut rect = self.layer(host, id)?["rect"].clone();
        rect[0] = json!(rect[0].as_f64().context("N2System frame x")? - 8.);
        rect[1] = json!(rect[1].as_f64().context("N2System frame y")? - 8.);
        self.submit(
            host,
            vec![json!({"op":"set_rect","id":id,"rect":rect})],
            vec![],
        )?;
        self.framed_images.insert(id);
        self.bitmap_formats.insert(
            id,
            bitmap_read::BitmapFormat {
                origin: None,
                alpha: Some(true),
            },
        );
        state.frame_image = None;
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
