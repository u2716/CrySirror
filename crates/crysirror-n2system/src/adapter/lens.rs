// SPDX-License-Identifier: MPL-2.0
//! CreateEffect "Lens": setup 4368F0, frame callback 443DE0. The native effect
//! samples the screen below its own priority through a precomputed disc table
//! (`$SYSTEM_effect_lens_curvature`/1000, `$SYSTEM_effect_lens_distance`) and
//! draws an n*n bitmap at the entity position. Table construction and sampling
//! are shared image operations; this module owns variables, order and timing.
use super::*;
use crysirror_core::image_processing::{
    FieldWarpSpec, ImageInput, ImageJob, ImageProcessState, LensTable,
};

pub(super) struct LensEffect {
    table: Arc<LensTable>,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
}
impl N2SystemVm {
    pub(super) fn native_create_lens(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        let (w, h) = (args.integer(4)?, args.integer(5)?);
        // 4368F0 fails the whole effect unless the extent is square.
        ensure!(
            w == h && (1..=2048).contains(&w),
            "N2System Lens requires a square extent"
        );
        let n = w as u32;
        if state.lens_table.is_none() {
            let variable = |name: &str, fallback: f64| -> Result<f64> {
                Ok(self
                    .machine
                    .variables
                    .get(name)
                    .map(Value::number)
                    .transpose()?
                    .map_or(fallback, f64::from))
            };
            let curvature = variable("SYSTEM_effect_lens_curvature", 1000.)? / 1000.;
            let distance = variable("SYSTEM_effect_lens_distance", 1.)?;
            let client = self.client;
            match self.pool.as_ref().unwrap().spawn(move |cancel| {
                crysirror_image::lens::prepare(client, n, curvature, distance, &|| cancel.check())
            }) {
                Ok(task) => state.lens_table = Some(task),
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        let Some(table) = state.lens_table.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        state.lens_table = None;
        let id = self.visual(
            host,
            current,
            args,
            json!({"kind":"group"}),
            [n, n],
            None,
        )?;
        self.bitmap_formats.insert(
            id,
            bitmap_read::BitmapFormat {
                origin: None,
                alpha: Some(false),
            },
        );
        self.lenses.insert(
            id,
            LensEffect {
                table,
                capture: None,
                process: None,
            },
        );
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn update_lenses(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids = self.lenses.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let layer = self.layer(host, id)?;
            let lens = self.lenses.get_mut(&id).unwrap();
            if let Some(job) = &mut lens.process {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(e) => bail!("N2System Lens: {e}"),
                    ImageProcessState::Ready(image) => {
                        lens.process = None;
                        self.publish_capture(host, id, image)?;
                    }
                }
            }
            let lens = self.lenses.get_mut(&id).unwrap();
            let canvas = self.config.canvas;
            let order = [
                layer["order"][0].as_i64().context("lens order")? as i32,
                layer["order"][1].as_i64().context("lens order")? as i32,
            ];
            if lens.capture.is_none() {
                lens.capture = Some(host.begin_layer_prefix_image(self.client, canvas, order)?);
            }
            match lens.capture.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => continue,
                ImageProcessState::Failed(e) => bail!("N2System Lens capture: {e}"),
                ImageProcessState::Ready(frame) => {
                    lens.capture = None;
                    let n = lens.table.size;
                    let origin = [
                        layer["rect"][0].as_f64().context("lens x")?.trunc() as i32,
                        layer["rect"][1].as_f64().context("lens y")?.trunc() as i32,
                    ];
                    // Native 443DE0 takes its checked-edge variant unless the
                    // lens covers the whole 1024x576 frame.
                    let full_cover = origin[0] <= 0
                        && origin[1] <= 0
                        && i64::from(origin[0]) + i64::from(n) >= i64::from(canvas[0])
                        && i64::from(origin[1]) + i64::from(n) >= i64::from(canvas[1]);
                    let input = ImageInput::FieldWarp {
                        source: Box::new(ImageInput::Resource(frame)),
                        size: [n, n],
                        spec: FieldWarpSpec::Lens {
                            table: lens.table.clone(),
                            origin,
                            full_cover,
                        },
                    };
                    match self
                        .images
                        .as_ref()
                        .unwrap()
                        .begin_process(input, Arc::from(&b"[]"[..]))
                    {
                        Ok(job) => self.lenses.get_mut(&id).unwrap().process = Some(job),
                        Err(Error::Limit("image preparation task capacity")) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
        Ok(())
    }
}
